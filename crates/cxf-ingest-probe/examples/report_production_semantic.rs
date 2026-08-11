#[cfg(all(cxf_json_semantic_harness, not(target_arch = "wasm32")))]
mod enabled {
    use std::{
        io::{self, Write},
        process::{self, ExitCode},
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    use cxf_ingest_probe::production_harness::{
        OutcomeKind, RETAINED_VALUES, VERIFIED_REVISION, WORKLOAD_VERSION, measure, options,
        retained_values_input, verify_instrumentation_revision,
    };
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct Report {
        run_id: String,
        instrumentation_revision: &'static str,
        workload_version: u64,
        retained_values: usize,
        input_bytes: usize,
        input_sha256: String,
        outcome: &'static str,
        source_matches_input: bool,
        max_nesting_depth: u64,
        max_object_members: u64,
        total_values: u64,
        decoded_member_name_bytes: u64,
        emitted_rdf_quads: u64,
        retained_rdf_term_bytes: u64,
        returned_rdf_quads: u64,
        preflight_ordered_micros: u128,
        jsonld_quad_retention_micros: u128,
        elapsed_micros: u128,
    }

    pub fn main() -> ExitCode {
        if let Err(error) = verify_instrumentation_revision() {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        match report() {
            Ok(report) => {
                let mut output = io::BufWriter::new(io::stdout().lock());
                if let Err(error) = serde_json::to_writer_pretty(&mut output, &report) {
                    eprintln!("failed to write report: {error}");
                    return ExitCode::FAILURE;
                }
                if let Err(error) = output.write_all(b"\n") {
                    eprintln!("failed to finish report: {error}");
                    return ExitCode::FAILURE;
                }
                if let Err(error) = output.flush() {
                    eprintln!("failed to flush report: {error}");
                    return ExitCode::FAILURE;
                }
                let mut time_output = io::stderr().lock();
                if let Err(error) = write_time_marker(&mut time_output, &report) {
                    eprintln!("failed to write time-report identity: {error}");
                    return ExitCode::FAILURE;
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        }
    }

    fn report() -> Result<Report, String> {
        let input = retained_values_input(RETAINED_VALUES);
        let started = Instant::now();
        let measured = measure(&input, &options());
        let elapsed_micros = started.elapsed().as_micros();
        let observation = measured.observation;
        if observation.outcome != OutcomeKind::Success
            || observation.source_matches_input != Some(true)
            || observation.returned_rdf_quads != RETAINED_VALUES as u64
        {
            return Err(format!("unexpected semantic observation: {observation:?}"));
        }
        let metrics = observation
            .metrics
            .ok_or_else(|| "successful observation is missing metrics".to_owned())?;
        if metrics.emitted_rdf_quads != RETAINED_VALUES as u64 {
            return Err(format!(
                "expected {RETAINED_VALUES} emitted quads, got {}",
                metrics.emitted_rdf_quads
            ));
        }
        let preflight_ordered_micros = measured.timing.preflight_ordered_micros;
        let jsonld_quad_retention_micros = measured
            .timing
            .jsonld_quad_retention_micros
            .ok_or_else(|| "successful observation is missing semantic timing".to_owned())?;
        let combined_stage_micros = preflight_ordered_micros
            .checked_add(jsonld_quad_retention_micros)
            .ok_or_else(|| "combined semantic stage timing overflowed".to_owned())?;
        if combined_stage_micros > elapsed_micros {
            return Err(format!(
                "combined stage time {combined_stage_micros} exceeds elapsed time {elapsed_micros}"
            ));
        }
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?;

        Ok(Report {
            run_id: format!("{:x}-{:x}", started_at.as_nanos(), process::id()),
            instrumentation_revision: VERIFIED_REVISION,
            workload_version: WORKLOAD_VERSION,
            retained_values: RETAINED_VALUES,
            input_bytes: input.len(),
            input_sha256: sha256(&input),
            outcome: "success",
            source_matches_input: true,
            max_nesting_depth: metrics.max_nesting_depth,
            max_object_members: metrics.max_object_members,
            total_values: metrics.total_values,
            decoded_member_name_bytes: metrics.decoded_member_name_bytes,
            emitted_rdf_quads: metrics.emitted_rdf_quads,
            retained_rdf_term_bytes: metrics.retained_rdf_term_bytes,
            returned_rdf_quads: observation.returned_rdf_quads,
            preflight_ordered_micros,
            jsonld_quad_retention_micros,
            elapsed_micros,
        })
    }

    fn write_time_marker(writer: &mut impl Write, report: &Report) -> io::Result<()> {
        writeln!(
            writer,
            "CXF_JSON_TIME_V1 run_id={} instrumentation_revision={} workload_version={} input_sha256={}",
            report.run_id,
            report.instrumentation_revision,
            report.workload_version,
            report.input_sha256
        )?;
        writer.flush()
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
        fn report_carries_the_verified_production_workload() {
            let report = report().expect("production semantic workload should succeed");

            assert_eq!(report.workload_version, WORKLOAD_VERSION);
            assert_eq!(report.retained_values, RETAINED_VALUES);
            assert_eq!(report.outcome, "success");
            assert!(report.source_matches_input);
            assert_eq!(report.emitted_rdf_quads, RETAINED_VALUES as u64);
            assert_eq!(report.returned_rdf_quads, RETAINED_VALUES as u64);
            assert_eq!(report.input_sha256.len(), 64);
            assert!(report.preflight_ordered_micros > 0);
            assert!(report.jsonld_quad_retention_micros > 0);
            assert!(
                report.preflight_ordered_micros + report.jsonld_quad_retention_micros
                    <= report.elapsed_micros
            );
        }

        #[test]
        fn time_marker_carries_the_report_identity() {
            let report = report().expect("production semantic workload should succeed");
            let mut marker = Vec::new();

            write_time_marker(&mut marker, &report).expect("marker should serialize");

            assert_eq!(
                String::from_utf8(marker).expect("marker should be UTF-8"),
                format!(
                    "CXF_JSON_TIME_V1 run_id={} instrumentation_revision={} workload_version={} input_sha256={}\n",
                    report.run_id,
                    report.instrumentation_revision,
                    report.workload_version,
                    report.input_sha256
                )
            );
        }
    }
}

#[cfg(all(cxf_json_semantic_harness, not(target_arch = "wasm32")))]
fn main() -> std::process::ExitCode {
    enabled::main()
}

#[cfg(any(not(cxf_json_semantic_harness), target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    eprintln!("set CXF_JSON_SEMANTIC_HARNESS=1 to build the production semantic report");
    std::process::ExitCode::FAILURE
}
