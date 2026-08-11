use std::{
    collections::BTreeMap,
    env,
    io::{self, Write},
    process::{self, ExitCode},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use cxf_ingest_probe::{
    DiagnosticStage, ProbeMetrics, StressExpected, measure_json_ld, resource_stress_cases,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize)]
struct ResourceStressReport {
    run_id: String,
    instrumentation_revision: Option<&'static str>,
    generator_version: u32,
    cases: Vec<CaseReport>,
    case_count: usize,
    input_bytes: usize,
    unexpected_outcomes: usize,
    elapsed_micros: u128,
}

#[derive(Debug, Serialize)]
struct CaseReport {
    name: &'static str,
    family: &'static str,
    parameters: BTreeMap<&'static str, usize>,
    input_bytes: usize,
    input_sha256: String,
    expected: ExpectedReport,
    actual: ActualReport,
    metrics: Option<ProbeMetrics>,
    preflight_micros: u128,
    json_ld_micros: Option<u128>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExpectedReport {
    Success { quad_count: usize },
    Failure { stage: DiagnosticStage },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ActualReport {
    Success {
        quad_count: usize,
    },
    Failure {
        stage: DiagnosticStage,
        diagnostic_message_bytes: usize,
    },
}

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let case_name = match arguments.next() {
        None => None,
        Some(option) if option == "--case" => match arguments.next() {
            Some(name) if arguments.next().is_none() => Some(name),
            _ => {
                eprintln!("usage: report_resource_stress [--case <name>]");
                return ExitCode::FAILURE;
            }
        },
        Some(_) => {
            eprintln!("usage: report_resource_stress [--case <name>]");
            return ExitCode::FAILURE;
        }
    };
    match report(case_name.as_deref()) {
        Ok(report) => {
            let failed = report.unexpected_outcomes > 0;
            let stdout = io::stdout();
            let mut output = stdout.lock();
            if let Err(error) = serde_json::to_writer_pretty(&mut output, &report) {
                eprintln!("failed to write report: {error}");
                return ExitCode::FAILURE;
            }
            if let Err(error) = output.write_all(b"\n") {
                eprintln!("failed to finish report: {error}");
                return ExitCode::FAILURE;
            }
            if failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn report(case_name: Option<&str>) -> Result<ResourceStressReport, String> {
    let started = Instant::now();
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?;
    let run_id = format!("{:x}-{:x}", started_at.as_nanos(), process::id());
    let mut cases = Vec::new();
    let mut unexpected_outcomes = 0;
    let mut input_bytes = 0;

    let selected_cases = resource_stress_cases()
        .into_iter()
        .filter(|case| case_name.is_none_or(|name| name == case.name))
        .collect::<Vec<_>>();
    if selected_cases.is_empty() {
        return Err(format!(
            "unknown resource-stress case {}",
            case_name.unwrap_or_default()
        ));
    }

    for case in selected_cases {
        let measured = measure_json_ld(&case.input);
        let expected = match case.expected {
            StressExpected::Success { quad_count } => ExpectedReport::Success { quad_count },
            StressExpected::Failure { stage } => ExpectedReport::Failure { stage },
        };
        let (actual, metrics, matched) = match measured.result {
            Ok(result) => {
                let quad_count = result.quads.len();
                let matched = matches!(
                    case.expected,
                    StressExpected::Success {
                        quad_count: expected
                    } if expected == quad_count
                );
                (
                    ActualReport::Success { quad_count },
                    Some(result.metrics),
                    matched,
                )
            }
            Err(failure) => {
                let stage = failure.diagnostic.stage;
                let diagnostic_message_bytes = failure.diagnostic.message.len();
                let matched = matches!(
                    case.expected,
                    StressExpected::Failure { stage: expected } if expected == stage
                );
                (
                    ActualReport::Failure {
                        stage,
                        diagnostic_message_bytes,
                    },
                    failure.metrics.map(|metrics| *metrics),
                    matched,
                )
            }
        };
        if !matched {
            unexpected_outcomes += 1;
        }
        input_bytes += case.input.len();
        let parameters = case
            .parameters
            .into_iter()
            .map(|parameter| (parameter.name, parameter.value))
            .collect();
        cases.push(CaseReport {
            name: case.name,
            family: case.family,
            parameters,
            input_bytes: case.input.len(),
            input_sha256: sha256(&case.input),
            expected,
            actual,
            metrics,
            preflight_micros: measured.timing.preflight.as_micros(),
            json_ld_micros: measured.timing.json_ld.map(|elapsed| elapsed.as_micros()),
        });
    }

    Ok(ResourceStressReport {
        run_id,
        instrumentation_revision: option_env!("CXF_BENCHMARK_REVISION"),
        generator_version: 1,
        case_count: cases.len(),
        cases,
        input_bytes,
        unexpected_outcomes,
        elapsed_micros: started.elapsed().as_micros(),
    })
}

fn sha256(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(input);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_carries_stable_input_identity() {
        let report =
            report(Some("scalar-string-262144")).expect("resource-stress report should run");

        assert_eq!(report.generator_version, 1);
        assert_eq!(report.case_count, 1);
        assert_eq!(report.unexpected_outcomes, 0);
        assert!(
            report
                .cases
                .iter()
                .all(|case| case.input_sha256.len() == 64)
        );
    }
}
