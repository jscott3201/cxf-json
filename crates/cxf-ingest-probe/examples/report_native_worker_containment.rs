#[cfg(all(cxf_json_semantic_harness, target_os = "linux"))]
mod enabled {
    use std::{
        env,
        io::{self, Read, Write},
        path::Path,
        process::{Child, ChildStdin, ChildStdout, Command, ExitCode, ExitStatus, Stdio},
        sync::mpsc::{self, Receiver, TryRecvError},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    use cxf_ingest_probe::production_harness::{
        Metrics, Observation, OutcomeKind, VERIFIED_REVISION, observe, options,
        retained_values_input, verify_instrumentation_revision,
    };
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};

    const PROTOCOL_VERSION: u64 = 1;
    const ADDRESS_SPACE_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
    const REQUEST_LIMIT_BYTES: usize = 1024 * 1024;
    const RESPONSE_LIMIT_BYTES: usize = 4 * 1024;
    const DEADLINE: Duration = Duration::from_secs(1);
    const POLL_INTERVAL: Duration = Duration::from_millis(5);
    const MAX_CONCURRENCY: u64 = 1;
    const OVERSIZED_RESPONSE_ATTEMPT_BYTES: usize = 1024 * 1024;

    type IoThread<T> = JoinHandle<io::Result<T>>;

    const SEMANTIC_WORKER_ARG: &str = "--worker";
    const DELAY_WORKER_ARG: &str = "--worker-delay";
    const MEMORY_WORKER_ARG: &str = "--worker-memory-probe";
    const OVERSIZED_RESPONSE_WORKER_ARG: &str = "--worker-oversized-response";

    #[derive(Clone, Copy)]
    enum WorkerMode {
        Semantic,
        Delay,
        MemoryProbe,
        OversizedResponse,
    }

    impl WorkerMode {
        const fn argument(self) -> &'static str {
            match self {
                Self::Semantic => SEMANTIC_WORKER_ARG,
                Self::Delay => DELAY_WORKER_ARG,
                Self::MemoryProbe => MEMORY_WORKER_ARG,
                Self::OversizedResponse => OVERSIZED_RESPONSE_WORKER_ARG,
            }
        }
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(deny_unknown_fields)]
    struct WorkerMetrics {
        max_nesting_depth: u64,
        max_object_members: u64,
        total_values: u64,
        decoded_member_name_bytes: u64,
        emitted_rdf_quads: u64,
        retained_rdf_term_bytes: u64,
    }

    impl From<Metrics> for WorkerMetrics {
        fn from(metrics: Metrics) -> Self {
            Self {
                max_nesting_depth: metrics.max_nesting_depth,
                max_object_members: metrics.max_object_members,
                total_values: metrics.total_values,
                decoded_member_name_bytes: metrics.decoded_member_name_bytes,
                emitted_rdf_quads: metrics.emitted_rdf_quads,
                retained_rdf_term_bytes: metrics.retained_rdf_term_bytes,
            }
        }
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(deny_unknown_fields)]
    struct WorkerResponse {
        protocol_version: u64,
        address_space_limit_bytes: u64,
        outcome: String,
        source_matches_input: Option<bool>,
        returned_rdf_quads: u64,
        metrics: Option<WorkerMetrics>,
    }

    impl WorkerResponse {
        fn observation(observation: Observation) -> Self {
            Self {
                protocol_version: PROTOCOL_VERSION,
                address_space_limit_bytes: ADDRESS_SPACE_LIMIT_BYTES,
                outcome: outcome_name(observation.outcome).to_owned(),
                source_matches_input: observation.source_matches_input,
                returned_rdf_quads: observation.returned_rdf_quads,
                metrics: observation.metrics.map(Into::into),
            }
        }

        fn control(outcome: &str) -> Self {
            Self {
                protocol_version: PROTOCOL_VERSION,
                address_space_limit_bytes: ADDRESS_SPACE_LIMIT_BYTES,
                outcome: outcome.to_owned(),
                source_matches_input: None,
                returned_rdf_quads: 0,
                metrics: None,
            }
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct CompletedWorker {
        response: WorkerResponse,
        response_bytes: usize,
    }

    #[derive(Debug, Eq, PartialEq)]
    enum ParentFailure {
        RequestLimit {
            observed_bytes: usize,
        },
        DeadlineExceeded {
            kill_sent: bool,
            reaped: bool,
            response_bytes: usize,
        },
        ResponseLimit {
            observed_bytes: usize,
            kill_sent: bool,
            reaped: bool,
        },
        WorkerFailed {
            exit_code: Option<i32>,
        },
        Protocol,
        Io(String),
    }

    #[derive(Serialize)]
    struct Report {
        instrumentation_revision: &'static str,
        platform: &'static str,
        protocol_version: u64,
        address_space_limit_bytes: u64,
        deadline_millis: u128,
        request_limit_bytes: usize,
        response_limit_bytes: usize,
        max_concurrency: u64,
        semantic_input_bytes: usize,
        semantic_input_sha256: String,
        semantic_outcome: String,
        semantic_returned_rdf_quads: u64,
        semantic_emitted_rdf_quads: u64,
        semantic_response_bytes: usize,
        remote_context_input_bytes: usize,
        remote_context_input_sha256: String,
        remote_context_outcome: String,
        remote_context_response_bytes: usize,
        memory_probe_outcome: String,
        memory_probe_response_bytes: usize,
        deadline_outcome: &'static str,
        deadline_kill_sent: bool,
        deadline_reaped: bool,
        deadline_response_bytes: usize,
        oversized_response_outcome: &'static str,
        oversized_response_observed_bytes: usize,
        oversized_response_attempted_bytes: usize,
        oversized_response_kill_sent: bool,
        oversized_response_reaped: bool,
        post_overflow_outcome: String,
        post_overflow_response_bytes: usize,
        oversized_request_outcome: &'static str,
        oversized_request_observed_bytes: usize,
        unexpected_outcomes: u64,
    }

    pub fn main() -> ExitCode {
        let mut arguments = env::args().skip(1);
        let mode = match arguments.next().as_deref() {
            None => return parent_main(),
            Some(SEMANTIC_WORKER_ARG) => WorkerMode::Semantic,
            Some(DELAY_WORKER_ARG) => WorkerMode::Delay,
            Some(MEMORY_WORKER_ARG) => WorkerMode::MemoryProbe,
            Some(OVERSIZED_RESPONSE_WORKER_ARG) => WorkerMode::OversizedResponse,
            Some(_) => {
                eprintln!("invalid worker mode");
                return ExitCode::FAILURE;
            }
        };
        if arguments.next().is_some() {
            eprintln!("worker modes do not accept arguments");
            return ExitCode::FAILURE;
        }
        worker_main(mode)
    }

    fn parent_main() -> ExitCode {
        if let Err(error) = verify_instrumentation_revision() {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        match report() {
            Ok(report) => {
                let mut output = io::BufWriter::new(io::stdout().lock());
                if serde_json::to_writer_pretty(&mut output, &report).is_err()
                    || output.write_all(b"\n").is_err()
                    || output.flush().is_err()
                {
                    eprintln!("failed to write worker-containment report");
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
        let executable = env::current_exe()
            .map_err(|error| format!("failed to locate worker executable: {error}"))?;

        let semantic_input = retained_values_input(32_768);
        let semantic = completed(run_worker(
            &executable,
            &semantic_input,
            WorkerMode::Semantic,
        ))?;
        let semantic_metrics = semantic
            .response
            .metrics
            .as_ref()
            .ok_or_else(|| "worker semantic observation is missing metrics".to_owned())?;
        if semantic.response.outcome != "success"
            || semantic.response.source_matches_input != Some(true)
            || semantic.response.returned_rdf_quads != 32_768
            || semantic_metrics.emitted_rdf_quads != 32_768
        {
            return Err(format!(
                "worker semantic observation was unexpected: {:?}",
                semantic.response
            ));
        }
        let semantic_emitted_rdf_quads = semantic_metrics.emitted_rdf_quads;

        let remote_context_input =
            include_bytes!("../../cxf-json/tests/fixtures/remote-context.jsonld");
        let remote = completed(run_worker(
            &executable,
            remote_context_input,
            WorkerMode::Semantic,
        ))?;
        if remote.response.outcome != "json_ld"
            || remote.response.source_matches_input != Some(true)
            || remote.response.returned_rdf_quads != 0
        {
            return Err(format!(
                "worker remote-context observation was unexpected: {:?}",
                remote.response
            ));
        }

        let memory = completed(run_worker(&executable, b"", WorkerMode::MemoryProbe))?;
        if memory.response != WorkerResponse::control("memory_allocation_denied") {
            return Err(format!(
                "worker address-space probe had an unexpected result: {:?}",
                memory.response
            ));
        }

        let (deadline_kill_sent, deadline_reaped, deadline_response_bytes) =
            match run_worker(&executable, b"null", WorkerMode::Delay) {
                Err(ParentFailure::DeadlineExceeded {
                    kill_sent,
                    reaped,
                    response_bytes,
                }) if kill_sent && reaped => (kill_sent, reaped, response_bytes),
                result => return Err(format!("worker deadline probe failed: {result:?}")),
            };

        let (
            oversized_response_observed_bytes,
            oversized_response_kill_sent,
            oversized_response_reaped,
        ) = match run_worker(&executable, b"null", WorkerMode::OversizedResponse) {
            Err(ParentFailure::ResponseLimit {
                observed_bytes,
                kill_sent,
                reaped,
            }) if observed_bytes == RESPONSE_LIMIT_BYTES + 1 && kill_sent && reaped => {
                (observed_bytes, kill_sent, reaped)
            }
            result => {
                return Err(format!(
                    "worker oversized-response probe failed: {result:?}"
                ));
            }
        };

        let post_overflow = completed(run_worker(
            &executable,
            br#"{"@id":"https://example.test/post-overflow"}"#,
            WorkerMode::Semantic,
        ))?;
        if post_overflow.response.outcome != "success"
            || post_overflow.response.source_matches_input != Some(true)
        {
            return Err(format!(
                "worker host did not recover after response overflow: {:?}",
                post_overflow.response
            ));
        }

        let oversized_request = vec![0; REQUEST_LIMIT_BYTES + 1];
        let oversized_request_observed_bytes = match run_worker(
            Path::new("/worker-must-not-be-spawned"),
            &oversized_request,
            WorkerMode::Semantic,
        ) {
            Err(ParentFailure::RequestLimit { observed_bytes })
                if observed_bytes == REQUEST_LIMIT_BYTES + 1 =>
            {
                observed_bytes
            }
            result => {
                return Err(format!("worker oversized-request probe failed: {result:?}"));
            }
        };

        Ok(Report {
            instrumentation_revision: VERIFIED_REVISION,
            platform: "linux",
            protocol_version: PROTOCOL_VERSION,
            address_space_limit_bytes: ADDRESS_SPACE_LIMIT_BYTES,
            deadline_millis: DEADLINE.as_millis(),
            request_limit_bytes: REQUEST_LIMIT_BYTES,
            response_limit_bytes: RESPONSE_LIMIT_BYTES,
            max_concurrency: MAX_CONCURRENCY,
            semantic_input_bytes: semantic_input.len(),
            semantic_input_sha256: sha256(&semantic_input),
            semantic_outcome: semantic.response.outcome,
            semantic_returned_rdf_quads: semantic.response.returned_rdf_quads,
            semantic_emitted_rdf_quads,
            semantic_response_bytes: semantic.response_bytes,
            remote_context_input_bytes: remote_context_input.len(),
            remote_context_input_sha256: sha256(remote_context_input),
            remote_context_outcome: remote.response.outcome,
            remote_context_response_bytes: remote.response_bytes,
            memory_probe_outcome: memory.response.outcome,
            memory_probe_response_bytes: memory.response_bytes,
            deadline_outcome: "deadline_exceeded",
            deadline_kill_sent,
            deadline_reaped,
            deadline_response_bytes,
            oversized_response_outcome: "response_limit",
            oversized_response_observed_bytes,
            oversized_response_attempted_bytes: OVERSIZED_RESPONSE_ATTEMPT_BYTES,
            oversized_response_kill_sent,
            oversized_response_reaped,
            post_overflow_outcome: post_overflow.response.outcome,
            post_overflow_response_bytes: post_overflow.response_bytes,
            oversized_request_outcome: "request_limit",
            oversized_request_observed_bytes,
            unexpected_outcomes: 0,
        })
    }

    fn worker_main(mode: WorkerMode) -> ExitCode {
        if set_address_space_limit().is_err() {
            eprintln!("worker resource-limit setup failed");
            return ExitCode::FAILURE;
        }
        let input = match read_request() {
            Ok(input) => input,
            Err(_) => {
                eprintln!("worker request rejected");
                return ExitCode::FAILURE;
            }
        };

        match mode {
            WorkerMode::Semantic => {
                write_response(&WorkerResponse::observation(observe(&input, &options())))
            }
            WorkerMode::Delay => {
                thread::sleep(DEADLINE + Duration::from_secs(1));
                write_response(&WorkerResponse::control("delay_completed"))
            }
            WorkerMode::MemoryProbe => {
                let outcome = if oversized_mapping_is_denied() {
                    "memory_allocation_denied"
                } else {
                    "memory_allocation_allowed"
                };
                write_response(&WorkerResponse::control(outcome))
            }
            WorkerMode::OversizedResponse => {
                let output = vec![b'x'; OVERSIZED_RESPONSE_ATTEMPT_BYTES];
                let mut stdout = io::stdout().lock();
                let _ = stdout.write_all(&output);
                thread::sleep(DEADLINE + Duration::from_secs(1));
                ExitCode::FAILURE
            }
        }
    }

    fn set_address_space_limit() -> io::Result<()> {
        let limit = libc::rlimit {
            rlim_cur: ADDRESS_SPACE_LIMIT_BYTES as libc::rlim_t,
            rlim_max: ADDRESS_SPACE_LIMIT_BYTES as libc::rlim_t,
        };
        // The worker sets the limit before reading input or entering the backend.
        let result = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn oversized_mapping_is_denied() -> bool {
        let requested = usize::try_from(ADDRESS_SPACE_LIMIT_BYTES * 2)
            .expect("the Linux evidence target must represent a 512 MiB mapping");
        // PROT_NONE reserves address space without committing or touching pages.
        let mapping = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                requested,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if mapping == libc::MAP_FAILED {
            true
        } else {
            // The unexpected mapping is released before the worker reports failure.
            unsafe {
                libc::munmap(mapping, requested);
            }
            false
        }
    }

    fn read_request() -> io::Result<Vec<u8>> {
        let mut input = Vec::with_capacity(REQUEST_LIMIT_BYTES.min(64 * 1024));
        io::stdin()
            .lock()
            .take((REQUEST_LIMIT_BYTES + 1) as u64)
            .read_to_end(&mut input)?;
        if input.len() > REQUEST_LIMIT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "worker request limit exceeded",
            ));
        }
        Ok(input)
    }

    fn write_response(response: &WorkerResponse) -> ExitCode {
        let mut encoded = match serde_json::to_vec(response) {
            Ok(encoded) => encoded,
            Err(_) => return ExitCode::FAILURE,
        };
        encoded.push(b'\n');
        if encoded.len() > RESPONSE_LIMIT_BYTES {
            return ExitCode::FAILURE;
        }
        let mut output = io::stdout().lock();
        if output.write_all(&encoded).is_err() || output.flush().is_err() {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }

    fn run_worker(
        executable: &Path,
        input: &[u8],
        mode: WorkerMode,
    ) -> Result<CompletedWorker, ParentFailure> {
        if input.len() > REQUEST_LIMIT_BYTES {
            return Err(ParentFailure::RequestLimit {
                observed_bytes: input.len(),
            });
        }

        let request = input.to_vec();
        let started = Instant::now();
        let mut child = Command::new(executable)
            .arg(mode.argument())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| ParentFailure::Io(error.to_string()))?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                stop_child(&mut child);
                return Err(ParentFailure::Io(
                    "worker stdin was not captured".to_owned(),
                ));
            }
        };
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                stop_child(&mut child);
                return Err(ParentFailure::Io(
                    "worker stdout was not captured".to_owned(),
                ));
            }
        };
        let (reader, response_limit) = match spawn_reader(stdout) {
            Ok(reader) => reader,
            Err(error) => {
                stop_child(&mut child);
                return Err(ParentFailure::Io(error.to_string()));
            }
        };
        let writer = match spawn_writer(stdin, request) {
            Ok(writer) => writer,
            Err(error) => {
                stop_child(&mut child);
                let _ = join_io(reader);
                return Err(ParentFailure::Io(error.to_string()));
            }
        };

        let status = loop {
            match response_limit.try_recv() {
                Ok(observed_bytes) => {
                    return Err(reject_oversized_response(
                        &mut child,
                        writer,
                        reader,
                        observed_bytes,
                    ));
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
            }
            if started.elapsed() >= DEADLINE {
                return Err(expire_child(&mut child, writer, reader));
            }
            match child.try_wait() {
                Ok(Some(status)) if started.elapsed() < DEADLINE => break status,
                Ok(Some(_)) => {
                    let _ = join_io(writer);
                    let response_bytes = join_io(reader).map_or(0, |bytes| bytes.len());
                    return Err(ParentFailure::DeadlineExceeded {
                        kill_sent: false,
                        reaped: true,
                        response_bytes,
                    });
                }
                Ok(None) => {
                    let remaining = DEADLINE.saturating_sub(started.elapsed());
                    thread::sleep(POLL_INTERVAL.min(remaining));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = join_io(writer);
                    let _ = join_io(reader);
                    return Err(ParentFailure::Io(error.to_string()));
                }
            }
        };

        let write_result = join_io(writer);
        let response_bytes = join_io(reader)?;
        if response_bytes.len() > RESPONSE_LIMIT_BYTES {
            return Err(ParentFailure::ResponseLimit {
                observed_bytes: response_bytes.len(),
                kill_sent: false,
                reaped: true,
            });
        }
        if !status.success() || write_result.is_err() {
            return Err(worker_failed(status));
        }
        let response = decode_response(&response_bytes)?;
        Ok(CompletedWorker {
            response,
            response_bytes: response_bytes.len(),
        })
    }

    fn expire_child(
        child: &mut Child,
        writer: IoThread<()>,
        reader: IoThread<Vec<u8>>,
    ) -> ParentFailure {
        let (kill_sent, reaped) = terminate_child(child);
        let _ = join_io(writer);
        let response_bytes = join_io(reader).map_or(0, |bytes| bytes.len());
        ParentFailure::DeadlineExceeded {
            kill_sent,
            reaped,
            response_bytes,
        }
    }

    fn reject_oversized_response(
        child: &mut Child,
        writer: IoThread<()>,
        reader: IoThread<Vec<u8>>,
        observed_bytes: usize,
    ) -> ParentFailure {
        let (kill_sent, reaped) = terminate_child(child);
        let _ = join_io(writer);
        let _ = join_io(reader);
        ParentFailure::ResponseLimit {
            observed_bytes,
            kill_sent,
            reaped,
        }
    }

    fn terminate_child(child: &mut Child) -> (bool, bool) {
        match child.try_wait() {
            Ok(Some(_)) => (false, true),
            Ok(None) | Err(_) => {
                let kill_sent = child.kill().is_ok();
                (kill_sent, child.wait().is_ok())
            }
        }
    }

    fn stop_child(child: &mut Child) {
        let _ = child.kill();
        let _ = child.wait();
    }

    fn spawn_writer(stdin: ChildStdin, input: Vec<u8>) -> io::Result<IoThread<()>> {
        thread::Builder::new()
            .name("cxf-worker-request".to_owned())
            .spawn(move || {
                let mut stdin = stdin;
                stdin.write_all(&input)?;
                stdin.flush()
            })
    }

    fn spawn_reader(stdout: ChildStdout) -> io::Result<(IoThread<Vec<u8>>, Receiver<usize>)> {
        let (limit_sender, limit_receiver) = mpsc::sync_channel(1);
        let reader = thread::Builder::new()
            .name("cxf-worker-response".to_owned())
            .spawn(move || {
                let mut bytes = Vec::with_capacity(RESPONSE_LIMIT_BYTES + 1);
                let result = stdout
                    .take((RESPONSE_LIMIT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes);
                if result.is_ok() && bytes.len() > RESPONSE_LIMIT_BYTES {
                    let _ = limit_sender.send(bytes.len());
                }
                result?;
                Ok(bytes)
            })?;
        Ok((reader, limit_receiver))
    }

    fn join_io<T>(handle: IoThread<T>) -> Result<T, ParentFailure> {
        handle
            .join()
            .map_err(|_| ParentFailure::Io("worker I/O thread panicked".to_owned()))?
            .map_err(|error| ParentFailure::Io(error.to_string()))
    }

    fn worker_failed(status: ExitStatus) -> ParentFailure {
        ParentFailure::WorkerFailed {
            exit_code: status.code(),
        }
    }

    fn decode_response(bytes: &[u8]) -> Result<WorkerResponse, ParentFailure> {
        let response: WorkerResponse =
            serde_json::from_slice(bytes).map_err(|_| ParentFailure::Protocol)?;
        if response.protocol_version != PROTOCOL_VERSION
            || response.address_space_limit_bytes != ADDRESS_SPACE_LIMIT_BYTES
        {
            return Err(ParentFailure::Protocol);
        }
        Ok(response)
    }

    fn completed(
        result: Result<CompletedWorker, ParentFailure>,
    ) -> Result<CompletedWorker, String> {
        result.map_err(|error| format!("worker did not complete: {error:?}"))
    }

    const fn outcome_name(outcome: OutcomeKind) -> &'static str {
        match outcome {
            OutcomeKind::Success => "success",
            OutcomeKind::AdmissionLimit => "admission_limit",
            OutcomeKind::InvalidUtf8 => "invalid_utf8",
            OutcomeKind::JsonSyntax => "json_syntax",
            OutcomeKind::DuplicateMember => "duplicate_member",
            OutcomeKind::JsonNestingLimit => "json_nesting_limit",
            OutcomeKind::JsonObjectMemberLimit => "json_object_member_limit",
            OutcomeKind::JsonValueLimit => "json_value_limit",
            OutcomeKind::DecodedMemberNameBytesLimit => "decoded_member_name_bytes_limit",
            OutcomeKind::MissingDocumentIri => "missing_document_iri",
            OutcomeKind::JsonLd => "json_ld",
            OutcomeKind::RdfQuadLimit => "rdf_quad_limit",
            OutcomeKind::RetainedRdfTermBytesLimit => "retained_rdf_term_bytes_limit",
        }
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
        fn response_round_trips_inside_the_protocol_cap() {
            let response = WorkerResponse::observation(observe(
                br#"{"@id":"https://example.test/s"}"#,
                &options(),
            ));
            let mut encoded = serde_json::to_vec(&response).expect("response should serialize");
            encoded.push(b'\n');

            assert!(encoded.len() <= RESPONSE_LIMIT_BYTES);
            assert_eq!(
                decode_response(&encoded).expect("response should decode"),
                response
            );
        }

        #[test]
        fn protocol_rejects_wrong_identity() {
            let mut response = WorkerResponse::control("success");
            response.protocol_version += 1;
            let encoded = serde_json::to_vec(&response).expect("response should serialize");
            assert_eq!(decode_response(&encoded), Err(ParentFailure::Protocol));

            let mut response = WorkerResponse::control("success");
            response.address_space_limit_bytes -= 1;
            let encoded = serde_json::to_vec(&response).expect("response should serialize");
            assert_eq!(decode_response(&encoded), Err(ParentFailure::Protocol));

            let mut value =
                serde_json::to_value(WorkerResponse::control("success")).expect("response value");
            value
                .as_object_mut()
                .expect("response should be an object")
                .insert("unexpected".to_owned(), true.into());
            let encoded = serde_json::to_vec(&value).expect("response should serialize");
            assert_eq!(decode_response(&encoded), Err(ParentFailure::Protocol));
        }

        #[test]
        fn oversized_request_is_rejected_before_spawn() {
            let input = vec![0; REQUEST_LIMIT_BYTES + 1];
            assert_eq!(
                run_worker(
                    Path::new("/worker-must-not-be-spawned"),
                    &input,
                    WorkerMode::Semantic,
                ),
                Err(ParentFailure::RequestLimit {
                    observed_bytes: REQUEST_LIMIT_BYTES + 1,
                })
            );
        }

        #[test]
        fn response_contains_only_project_owned_scalars() {
            let encoded = serde_json::to_string(&WorkerResponse::observation(observe(
                br#"{"@id":"https://example.test/s"}"#,
                &options(),
            )))
            .expect("response should serialize");

            for forbidden in [
                "source_document",
                "source_bytes",
                "backend",
                "diagnostic",
                "ordered_source",
            ] {
                assert!(!encoded.contains(forbidden));
            }
        }
    }
}

#[cfg(all(cxf_json_semantic_harness, target_os = "linux"))]
fn main() -> std::process::ExitCode {
    enabled::main()
}

#[cfg(not(all(cxf_json_semantic_harness, target_os = "linux")))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "the native worker-containment report requires Linux and CXF_JSON_SEMANTIC_HARNESS=1"
    );
    std::process::ExitCode::FAILURE
}
