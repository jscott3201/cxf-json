use std::{
    collections::BTreeMap,
    env,
    io::{self, Write},
    path::Path,
    process::{self, Command, ExitCode},
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
    if let Err(error) = verify_instrumentation_revision() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
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

fn verify_instrumentation_revision() -> Result<(), String> {
    let revision = option_env!("CXF_VERIFIED_BENCHMARK_REVISION")
        .ok_or_else(|| "the artifact was not built in benchmark mode".to_owned())?;
    if !is_commit_id(revision) {
        return Err("CXF_BENCHMARK_REVISION must be a 40-digit commit ID".to_owned());
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "probe crate must be inside the repository workspace".to_owned())?;
    let head = git_output(repository, &["rev-parse", "HEAD"])?;
    if head.trim() != revision {
        return Err(format!(
            "instrumentation revision mismatch: expected {revision}, got {}",
            head.trim()
        ));
    }
    let status = git_output(
        repository,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )?;
    if !status.is_empty() {
        return Err("resource-stress measurements require a clean worktree".to_owned());
    }
    Ok(())
}

fn is_commit_id(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn git_output(repository: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = git_command(repository)
        .args(arguments)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|_| "git returned non-UTF-8 output".to_owned())
}

fn git_command(repository: &Path) -> Command {
    let null_config = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut command = Command::new("git");
    command
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_GRAFT_FILE")
        .env_remove("GIT_REPLACE_REF_BASE")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_GLOBAL")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_config)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-c")
        .arg(format!("core.hooksPath={null_config}"))
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "core.preloadIndex=false",
        ])
        .arg("-C")
        .arg(repository);
    command
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
        instrumentation_revision: option_env!("CXF_VERIFIED_BENCHMARK_REVISION"),
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

    #[test]
    fn instrumentation_revision_requires_full_lower_hex() {
        assert!(is_commit_id(&"a".repeat(40)));
        assert!(!is_commit_id(&"a".repeat(39)));
        assert!(!is_commit_id(&"A".repeat(40)));
        assert!(!is_commit_id(&"g".repeat(40)));
    }

    #[test]
    fn git_commands_ignore_repository_overrides() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let command = git_command(repository);

        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_CONFIG_SYSTEM",
        ] {
            assert!(
                command
                    .get_envs()
                    .any(|(key, value)| key == name && value.is_none()),
                "{name} must be removed"
            );
        }
        assert!(command.get_envs().any(|(key, value)| {
            key == "GIT_NO_REPLACE_OBJECTS" && value.is_some_and(|value| value == "1")
        }));
    }
}
