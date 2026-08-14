#[cfg(all(
    cxf_json_semantic_harness,
    any(target_os = "linux", target_os = "macos")
))]
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

    const PROTOCOL_VERSION: u64 = 2;
    #[cfg(target_os = "linux")]
    const LINUX_ADDRESS_SPACE_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
    const REQUEST_LIMIT_BYTES: usize = 1024 * 1024;
    const RESPONSE_LIMIT_BYTES: usize = 4 * 1024;
    const DEADLINE: Duration = Duration::from_secs(1);
    const POLL_INTERVAL: Duration = Duration::from_millis(5);
    const MAX_CONCURRENCY: u64 = 1;
    const OVERSIZED_RESPONSE_ATTEMPT_BYTES: usize = 1024 * 1024;

    #[cfg(target_os = "macos")]
    const MACOS_ADDRESS_SPACE_CANDIDATES: [u64; 10] = [
        256 * 1024 * 1024,
        1024 * 1024 * 1024,
        4 * 1024 * 1024 * 1024,
        16 * 1024 * 1024 * 1024,
        64 * 1024 * 1024 * 1024,
        128 * 1024 * 1024 * 1024,
        256 * 1024 * 1024 * 1024,
        512 * 1024 * 1024 * 1024,
        1024 * 1024 * 1024 * 1024,
        2 * 1024 * 1024 * 1024 * 1024,
    ];

    type IoThread<T> = JoinHandle<io::Result<T>>;

    struct RequestWriter {
        handle: IoThread<()>,
        release: mpsc::SyncSender<()>,
    }

    const SEMANTIC_WORKER_ARG: &str = "--worker";
    const DELAY_WORKER_ARG: &str = "--worker-delay";
    const MEMORY_WORKER_ARG: &str = "--worker-memory-probe";
    const OVERSIZED_RESPONSE_WORKER_ARG: &str = "--worker-oversized-response";
    #[cfg(target_os = "macos")]
    const PARENT_LIVENESS_EXECUTION_WORKER_ARG: &str = "--worker-parent-liveness-execution";
    #[cfg(target_os = "macos")]
    const PARENT_LIVENESS_RESPONSE_WORKER_ARG: &str = "--worker-parent-liveness-response";
    #[cfg(target_os = "macos")]
    const PARENT_LIVENESS_STARTUP_CONTROLLER_ARG: &str = "--parent-liveness-startup";
    #[cfg(target_os = "macos")]
    const PARENT_LIVENESS_EXECUTION_CONTROLLER_ARG: &str = "--parent-liveness-execution";
    #[cfg(target_os = "macos")]
    const PARENT_LIVENESS_RESPONSE_CONTROLLER_ARG: &str = "--parent-liveness-response";

    #[derive(Clone, Copy)]
    enum WorkerMode {
        Semantic,
        Delay,
        MemoryProbe,
        OversizedResponse,
        #[cfg(target_os = "macos")]
        ParentLivenessExecution,
        #[cfg(target_os = "macos")]
        ParentLivenessResponse,
    }

    impl WorkerMode {
        const fn argument(self) -> &'static str {
            match self {
                Self::Semantic => SEMANTIC_WORKER_ARG,
                Self::Delay => DELAY_WORKER_ARG,
                Self::MemoryProbe => MEMORY_WORKER_ARG,
                Self::OversizedResponse => OVERSIZED_RESPONSE_WORKER_ARG,
                #[cfg(target_os = "macos")]
                Self::ParentLivenessExecution => PARENT_LIVENESS_EXECUTION_WORKER_ARG,
                #[cfg(target_os = "macos")]
                Self::ParentLivenessResponse => PARENT_LIVENESS_RESPONSE_WORKER_ARG,
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[derive(Clone, Copy)]
    enum ParentLivenessStage {
        Startup,
        Execution,
        Response,
    }

    #[cfg(target_os = "macos")]
    impl ParentLivenessStage {
        const fn controller_argument(self) -> &'static str {
            match self {
                Self::Startup => PARENT_LIVENESS_STARTUP_CONTROLLER_ARG,
                Self::Execution => PARENT_LIVENESS_EXECUTION_CONTROLLER_ARG,
                Self::Response => PARENT_LIVENESS_RESPONSE_CONTROLLER_ARG,
            }
        }

        const fn name(self) -> &'static str {
            match self {
                Self::Startup => "startup",
                Self::Execution => "execution",
                Self::Response => "response",
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ParentLivenessReady {
        worker_pid: u32,
        stage: String,
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
        memory_limit_kind: String,
        memory_limit_bytes: Option<u64>,
        memory_probe_limit_bytes: Option<u64>,
        outcome: String,
        source_matches_input: Option<bool>,
        returned_rdf_quads: u64,
        metrics: Option<WorkerMetrics>,
    }

    impl WorkerResponse {
        fn observation(observation: Observation) -> Self {
            Self {
                protocol_version: PROTOCOL_VERSION,
                memory_limit_kind: memory_limit_kind().to_owned(),
                memory_limit_bytes: memory_limit_bytes(),
                memory_probe_limit_bytes: None,
                outcome: outcome_name(observation.outcome).to_owned(),
                source_matches_input: observation.source_matches_input,
                returned_rdf_quads: observation.returned_rdf_quads,
                metrics: observation.metrics.map(Into::into),
            }
        }

        fn control(outcome: &str) -> Self {
            Self {
                protocol_version: PROTOCOL_VERSION,
                memory_limit_kind: memory_limit_kind().to_owned(),
                memory_limit_bytes: memory_limit_bytes(),
                memory_probe_limit_bytes: None,
                outcome: outcome.to_owned(),
                source_matches_input: None,
                returned_rdf_quads: 0,
                metrics: None,
            }
        }

        #[cfg(target_os = "macos")]
        fn memory_probe(outcome: &str, limit_bytes: Option<u64>) -> Self {
            let mut response = Self::control(outcome);
            response.memory_probe_limit_bytes = limit_bytes;
            response
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
        architecture: &'static str,
        macos_product_version: Option<String>,
        macos_build_version: Option<String>,
        darwin_release: Option<String>,
        protocol_version: u64,
        memory_limit_kind: &'static str,
        memory_limit_bytes: Option<u64>,
        host_physical_memory_bytes: Option<u64>,
        memory_probe_limit_bytes: Option<u64>,
        memory_qualification_status: &'static str,
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
        parent_liveness_mechanism: &'static str,
        parent_liveness_startup_outcome: &'static str,
        parent_liveness_execution_outcome: &'static str,
        parent_liveness_response_outcome: &'static str,
        oversized_request_outcome: &'static str,
        oversized_request_observed_bytes: usize,
        unexpected_outcomes: u64,
    }

    pub fn main() -> ExitCode {
        let mut arguments = env::args().skip(1);
        let argument = arguments.next();
        #[cfg(target_os = "macos")]
        if let Some(stage) = match argument.as_deref() {
            Some(PARENT_LIVENESS_STARTUP_CONTROLLER_ARG) => Some(ParentLivenessStage::Startup),
            Some(PARENT_LIVENESS_EXECUTION_CONTROLLER_ARG) => Some(ParentLivenessStage::Execution),
            Some(PARENT_LIVENESS_RESPONSE_CONTROLLER_ARG) => Some(ParentLivenessStage::Response),
            _ => None,
        } {
            if arguments.next().is_some() {
                eprintln!("parent-liveness controller modes do not accept arguments");
                return ExitCode::FAILURE;
            }
            return parent_liveness_controller(stage);
        }

        let mode = match argument.as_deref() {
            None => return parent_main(),
            Some(SEMANTIC_WORKER_ARG) => WorkerMode::Semantic,
            Some(DELAY_WORKER_ARG) => WorkerMode::Delay,
            Some(MEMORY_WORKER_ARG) => WorkerMode::MemoryProbe,
            Some(OVERSIZED_RESPONSE_WORKER_ARG) => WorkerMode::OversizedResponse,
            #[cfg(target_os = "macos")]
            Some(PARENT_LIVENESS_EXECUTION_WORKER_ARG) => WorkerMode::ParentLivenessExecution,
            #[cfg(target_os = "macos")]
            Some(PARENT_LIVENESS_RESPONSE_WORKER_ARG) => WorkerMode::ParentLivenessResponse,
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
        #[cfg(target_os = "macos")]
        let (macos_product_version, macos_build_version, darwin_release) = (
            Some(system_command_output("sw_vers", &["-productVersion"])?),
            Some(system_command_output("sw_vers", &["-buildVersion"])?),
            Some(system_command_output("uname", &["-r"])?),
        );
        #[cfg(target_os = "linux")]
        let (macos_product_version, macos_build_version, darwin_release) = (None, None, None);

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
        #[cfg(target_os = "linux")]
        let (host_physical_memory_bytes, memory_probe_limit_bytes, memory_qualification_status) = {
            if memory.response != WorkerResponse::control("memory_allocation_denied") {
                return Err(format!(
                    "worker address-space probe had an unexpected result: {:?}",
                    memory.response
                ));
            }
            (None, None, "evidence_only_not_qualified")
        };
        #[cfg(target_os = "macos")]
        let (host_physical_memory_bytes, memory_probe_limit_bytes, memory_qualification_status) = {
            let physical_memory = physical_memory_bytes()
                .map_err(|error| format!("failed to read macOS physical memory: {error}"))?;
            let probe_limit = memory.response.memory_probe_limit_bytes;
            let status = match memory.response.outcome.as_str() {
                "rlimit_as_mapping_denied"
                    if probe_limit.is_some_and(|limit| limit > physical_memory) =>
                {
                    "blocked_minimum_limit_exceeds_physical_memory"
                }
                "rlimit_as_mapping_denied" => "candidate_requires_product_measurement",
                "rlimit_as_unavailable" => "blocked_rlimit_as_unavailable",
                "rlimit_as_mapping_allowed" => "blocked_rlimit_as_not_enforced",
                _ => {
                    return Err(format!(
                        "worker address-space probe had an unexpected result: {:?}",
                        memory.response
                    ));
                }
            };
            (Some(physical_memory), probe_limit, status)
        };

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

        #[cfg(target_os = "macos")]
        let (
            parent_liveness_startup_outcome,
            parent_liveness_execution_outcome,
            parent_liveness_response_outcome,
        ) = (
            run_parent_liveness_probe(&executable, ParentLivenessStage::Startup)?,
            run_parent_liveness_probe(&executable, ParentLivenessStage::Execution)?,
            run_parent_liveness_probe(&executable, ParentLivenessStage::Response)?,
        );
        #[cfg(target_os = "linux")]
        let (
            parent_liveness_startup_outcome,
            parent_liveness_execution_outcome,
            parent_liveness_response_outcome,
        ) = ("not_evaluated", "not_evaluated", "not_evaluated");

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
            platform: platform(),
            architecture: env::consts::ARCH,
            macos_product_version,
            macos_build_version,
            darwin_release,
            protocol_version: PROTOCOL_VERSION,
            memory_limit_kind: memory_limit_kind(),
            memory_limit_bytes: memory_limit_bytes(),
            host_physical_memory_bytes,
            memory_probe_limit_bytes,
            memory_qualification_status,
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
            parent_liveness_mechanism: parent_liveness_mechanism(),
            parent_liveness_startup_outcome,
            parent_liveness_execution_outcome,
            parent_liveness_response_outcome,
            oversized_request_outcome: "request_limit",
            oversized_request_observed_bytes,
            unexpected_outcomes: 0,
        })
    }

    #[cfg(target_os = "macos")]
    fn run_parent_liveness_probe(
        executable: &Path,
        stage: ParentLivenessStage,
    ) -> Result<&'static str, String> {
        let mut controller = Command::new(executable)
            .arg(stage.controller_argument())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to spawn parent-liveness controller: {error}"))?;
        let stdout = controller
            .stdout
            .take()
            .ok_or_else(|| "parent-liveness controller stdout was not captured".to_owned())?;
        let reader = thread::Builder::new()
            .name("cxf-parent-liveness-ready".to_owned())
            .spawn(move || read_parent_liveness_ready(stdout))
            .map_err(|error| format!("failed to spawn parent-liveness reader: {error}"))?;

        let started = Instant::now();
        while !reader.is_finished() && started.elapsed() < DEADLINE {
            thread::sleep(POLL_INTERVAL);
        }
        if !reader.is_finished() {
            stop_child(&mut controller);
            let _ = reader.join();
            return Err(format!(
                "parent-liveness {} controller did not become ready",
                stage.name()
            ));
        }
        let ready = reader
            .join()
            .map_err(|_| "parent-liveness reader panicked".to_owned())?
            .map_err(|error| format!("parent-liveness controller response failed: {error}"))?;
        if ready.stage != stage.name() || ready.worker_pid == controller.id() {
            stop_child(&mut controller);
            return Err(format!(
                "parent-liveness {} controller returned an invalid identity",
                stage.name()
            ));
        }

        let (kill_sent, reaped) = terminate_child(&mut controller);
        if !kill_sent || !reaped {
            unsafe {
                libc::kill(ready.worker_pid as libc::pid_t, libc::SIGKILL);
            }
            return Err(format!(
                "parent-liveness {} controller was not killed and reaped",
                stage.name()
            ));
        }
        if !wait_for_process_exit(ready.worker_pid) {
            unsafe {
                libc::kill(ready.worker_pid as libc::pid_t, libc::SIGKILL);
            }
            return Err(format!(
                "parent-liveness {} worker survived controller death",
                stage.name()
            ));
        }
        Ok("worker_exited_after_controller_death")
    }

    #[cfg(target_os = "macos")]
    fn parent_liveness_controller(stage: ParentLivenessStage) -> ExitCode {
        match parent_liveness_controller_result(stage) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn parent_liveness_controller_result(stage: ParentLivenessStage) -> Result<(), String> {
        let executable = env::current_exe()
            .map_err(|error| format!("failed to locate parent-liveness worker: {error}"))?;
        let worker_mode = match stage {
            ParentLivenessStage::Startup => WorkerMode::Delay,
            ParentLivenessStage::Execution => WorkerMode::ParentLivenessExecution,
            ParentLivenessStage::Response => WorkerMode::ParentLivenessResponse,
        };
        let mut worker = Command::new(executable)
            .arg(worker_mode.argument())
            .stdin(Stdio::piped())
            .stdout(if matches!(stage, ParentLivenessStage::Startup) {
                Stdio::null()
            } else {
                Stdio::piped()
            })
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to spawn parent-liveness worker: {error}"))?;
        let stdin = worker
            .stdin
            .take()
            .ok_or_else(|| "parent-liveness worker stdin was not captured".to_owned())?;

        let mut writer = None;
        let ready = if matches!(stage, ParentLivenessStage::Startup) {
            ParentLivenessReady {
                worker_pid: worker.id(),
                stage: stage.name().to_owned(),
            }
        } else {
            let input = if matches!(stage, ParentLivenessStage::Response) {
                retained_values_input(1_024)
            } else {
                b"null".to_vec()
            };
            writer = Some(
                spawn_writer(stdin, input)
                    .map_err(|error| format!("failed to write liveness request: {error}"))?,
            );
            let stdout = worker
                .stdout
                .take()
                .ok_or_else(|| "parent-liveness worker stdout was not captured".to_owned())?;
            read_parent_liveness_ready(stdout)
                .map_err(|error| format!("parent-liveness worker did not become ready: {error}"))?
        };

        let mut output = io::stdout().lock();
        if serde_json::to_writer(&mut output, &ready).is_err()
            || output.write_all(b"\n").is_err()
            || output.flush().is_err()
        {
            return Err("failed to report parent-liveness worker".to_owned());
        }

        thread::sleep(DEADLINE + Duration::from_secs(10));
        stop_child(&mut worker);
        if let Some(writer) = writer {
            let _ = finish_writer(writer);
        }
        Err("parent-liveness controller was not terminated by its parent".to_owned())
    }

    #[cfg(target_os = "macos")]
    fn read_parent_liveness_ready(mut input: impl Read) -> io::Result<ParentLivenessReady> {
        let mut encoded = Vec::with_capacity(128);
        while encoded.len() <= 256 {
            let mut byte = [0];
            if input.read(&mut byte)? == 0 {
                break;
            }
            encoded.push(byte[0]);
            if byte[0] == b'\n' {
                return serde_json::from_slice(&encoded)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
            }
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "parent-liveness response exceeded its limit or lacked a newline",
        ))
    }

    #[cfg(target_os = "macos")]
    fn wait_for_process_exit(pid: u32) -> bool {
        let started = Instant::now();
        while started.elapsed() < DEADLINE + Duration::from_secs(1) {
            let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
            if result == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                return true;
            }
            thread::sleep(POLL_INTERVAL);
        }
        false
    }

    fn worker_main(mode: WorkerMode) -> ExitCode {
        #[cfg(target_os = "linux")]
        if let Err(error) = set_address_space_limit() {
            eprintln!("worker resource-limit setup failed: {error}");
            return ExitCode::FAILURE;
        }
        let (input, request_stdin) = match read_request() {
            Ok(request) => request,
            Err(_) => {
                eprintln!("worker request rejected");
                return ExitCode::FAILURE;
            }
        };
        #[cfg(target_os = "macos")]
        let _memory_probe_stdin = if matches!(mode, WorkerMode::MemoryProbe) {
            Some(request_stdin)
        } else {
            if start_parent_liveness_watchdog(request_stdin).is_err() {
                eprintln!("worker parent-liveness setup failed");
                return ExitCode::FAILURE;
            }
            None
        };
        #[cfg(target_os = "linux")]
        let _request_stdin = request_stdin;

        match mode {
            WorkerMode::Semantic => {
                write_response(&WorkerResponse::observation(observe(&input, &options())))
            }
            WorkerMode::Delay => {
                thread::sleep(DEADLINE + Duration::from_secs(1));
                write_response(&WorkerResponse::control("delay_completed"))
            }
            WorkerMode::MemoryProbe => {
                #[cfg(target_os = "linux")]
                let response = if oversized_mapping_is_denied(LINUX_ADDRESS_SPACE_LIMIT_BYTES) {
                    "memory_allocation_denied"
                } else {
                    "memory_allocation_allowed"
                };
                #[cfg(target_os = "linux")]
                let response = WorkerResponse::control(response);
                #[cfg(target_os = "macos")]
                let response = {
                    let (outcome, limit_bytes) = macos_address_space_probe();
                    WorkerResponse::memory_probe(outcome, limit_bytes)
                };
                write_response(&response)
            }
            WorkerMode::OversizedResponse => {
                let output = vec![b'x'; OVERSIZED_RESPONSE_ATTEMPT_BYTES];
                let mut stdout = io::stdout().lock();
                let _ = stdout.write_all(&output);
                thread::sleep(DEADLINE + Duration::from_secs(1));
                ExitCode::FAILURE
            }
            #[cfg(target_os = "macos")]
            WorkerMode::ParentLivenessExecution => {
                write_parent_liveness_ready(ParentLivenessStage::Execution)
            }
            #[cfg(target_os = "macos")]
            WorkerMode::ParentLivenessResponse => {
                let _ = observe(&input, &options());
                write_parent_liveness_ready(ParentLivenessStage::Response)
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn set_address_space_limit() -> io::Result<()> {
        let limit = libc::rlimit {
            rlim_cur: LINUX_ADDRESS_SPACE_LIMIT_BYTES as libc::rlim_t,
            rlim_max: LINUX_ADDRESS_SPACE_LIMIT_BYTES as libc::rlim_t,
        };
        // The worker sets the limit before reading input or entering the backend.
        let result = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(target_os = "macos")]
    fn set_address_space_limit_bytes(limit_bytes: u64) -> io::Result<()> {
        let limit = libc::rlimit {
            rlim_cur: limit_bytes as libc::rlim_t,
            rlim_max: limit_bytes as libc::rlim_t,
        };
        let result = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn oversized_mapping_is_denied(limit_bytes: u64) -> bool {
        let requested = usize::try_from(limit_bytes.saturating_mul(2))
            .expect("the native evidence target must represent the probe mapping");
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

    #[cfg(target_os = "macos")]
    fn macos_address_space_probe() -> (&'static str, Option<u64>) {
        let Some(limit_bytes) = MACOS_ADDRESS_SPACE_CANDIDATES
            .into_iter()
            .find(|limit| set_address_space_limit_bytes(*limit).is_ok())
        else {
            return ("rlimit_as_unavailable", None);
        };
        let outcome = if oversized_mapping_is_denied(limit_bytes) {
            "rlimit_as_mapping_denied"
        } else {
            "rlimit_as_mapping_allowed"
        };
        (outcome, Some(limit_bytes))
    }

    fn read_request() -> io::Result<(Vec<u8>, io::Stdin)> {
        let mut stdin = io::stdin();
        let mut encoded_length = [0; 8];
        stdin.read_exact(&mut encoded_length)?;
        let length = usize::try_from(u64::from_be_bytes(encoded_length)).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "worker request length overflow")
        })?;
        if length > REQUEST_LIMIT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "worker request limit exceeded",
            ));
        }
        let mut input = vec![0; length];
        stdin.read_exact(&mut input)?;
        Ok((input, stdin))
    }

    #[cfg(target_os = "macos")]
    fn start_parent_liveness_watchdog(mut stdin: io::Stdin) -> io::Result<()> {
        thread::Builder::new()
            .name("cxf-worker-parent-liveness".to_owned())
            .spawn(move || {
                let mut unexpected = [0];
                let _ = stdin.read(&mut unexpected);
                unsafe { libc::_exit(86) }
            })?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn write_parent_liveness_ready(stage: ParentLivenessStage) -> ExitCode {
        let ready = ParentLivenessReady {
            worker_pid: std::process::id(),
            stage: stage.name().to_owned(),
        };
        let mut stdout = io::stdout().lock();
        if serde_json::to_writer(&mut stdout, &ready).is_err()
            || stdout.write_all(b"\n").is_err()
            || stdout.flush().is_err()
        {
            return ExitCode::FAILURE;
        }
        thread::sleep(DEADLINE + Duration::from_secs(10));
        ExitCode::FAILURE
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
                    let _ = finish_writer(writer);
                    return Err(deadline_or_response_limit(false, true, join_io(reader)));
                }
                Ok(None) => {
                    let remaining = DEADLINE.saturating_sub(started.elapsed());
                    thread::sleep(POLL_INTERVAL.min(remaining));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = finish_writer(writer);
                    let _ = join_io(reader);
                    return Err(ParentFailure::Io(error.to_string()));
                }
            }
        };

        let write_result = finish_writer(writer);
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
        writer: RequestWriter,
        reader: IoThread<Vec<u8>>,
    ) -> ParentFailure {
        let (kill_sent, reaped) = terminate_child(child);
        let _ = finish_writer(writer);
        deadline_or_response_limit(kill_sent, reaped, join_io(reader))
    }

    fn deadline_or_response_limit(
        kill_sent: bool,
        reaped: bool,
        response: Result<Vec<u8>, ParentFailure>,
    ) -> ParentFailure {
        let response_bytes = response.map_or(0, |bytes| bytes.len());
        if response_bytes > RESPONSE_LIMIT_BYTES {
            ParentFailure::ResponseLimit {
                observed_bytes: response_bytes,
                kill_sent,
                reaped,
            }
        } else {
            ParentFailure::DeadlineExceeded {
                kill_sent,
                reaped,
                response_bytes,
            }
        }
    }

    fn reject_oversized_response(
        child: &mut Child,
        writer: RequestWriter,
        reader: IoThread<Vec<u8>>,
        observed_bytes: usize,
    ) -> ParentFailure {
        let (kill_sent, reaped) = terminate_child(child);
        let _ = finish_writer(writer);
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

    fn spawn_writer(stdin: ChildStdin, input: Vec<u8>) -> io::Result<RequestWriter> {
        let (release, wait_for_release) = mpsc::sync_channel(0);
        let handle = thread::Builder::new()
            .name("cxf-worker-request".to_owned())
            .spawn(move || {
                let mut stdin = stdin;
                write_request(&mut stdin, &input)?;
                let _ = wait_for_release.recv();
                Ok(())
            })?;
        Ok(RequestWriter { handle, release })
    }

    fn finish_writer(writer: RequestWriter) -> Result<(), ParentFailure> {
        let _ = writer.release.send(());
        join_io(writer.handle)
    }

    fn write_request(mut output: impl Write, input: &[u8]) -> io::Result<()> {
        output.write_all(&(input.len() as u64).to_be_bytes())?;
        output.write_all(input)?;
        output.flush()
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
            || response.memory_limit_kind != memory_limit_kind()
            || response.memory_limit_bytes != memory_limit_bytes()
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

    const fn platform() -> &'static str {
        if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        }
    }

    const fn memory_limit_kind() -> &'static str {
        if cfg!(target_os = "linux") {
            "rlimit_as_virtual_address_space"
        } else {
            "none"
        }
    }

    const fn memory_limit_bytes() -> Option<u64> {
        #[cfg(target_os = "linux")]
        {
            Some(LINUX_ADDRESS_SPACE_LIMIT_BYTES)
        }
        #[cfg(target_os = "macos")]
        {
            None
        }
    }

    const fn parent_liveness_mechanism() -> &'static str {
        if cfg!(target_os = "macos") {
            "framed_stdin_eof"
        } else {
            "not_evaluated"
        }
    }

    #[cfg(target_os = "macos")]
    fn physical_memory_bytes() -> io::Result<u64> {
        let mut memory_bytes = 0_u64;
        let mut length = std::mem::size_of::<u64>();
        let result = unsafe {
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&raw mut memory_bytes).cast(),
                &raw mut length,
                std::ptr::null_mut(),
                0,
            )
        };
        if result == 0 && length == std::mem::size_of::<u64>() {
            Ok(memory_bytes)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(target_os = "macos")]
    fn system_command_output(program: &str, arguments: &[&str]) -> Result<String, String> {
        let output = Command::new(program)
            .args(arguments)
            .output()
            .map_err(|error| format!("failed to run {program}: {error}"))?;
        if !output.status.success() {
            return Err(format!("{program} exited unsuccessfully"));
        }
        let value = String::from_utf8(output.stdout)
            .map_err(|_| format!("{program} output was not UTF-8"))?;
        let value = value.trim();
        if value.is_empty() {
            return Err(format!("{program} returned an empty value"));
        }
        Ok(value.to_owned())
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
        fn request_frame_carries_exact_length_and_bytes() {
            let input = br#"{"@id":"https://example.test/s"}"#;
            let mut encoded = Vec::new();
            write_request(&mut encoded, input).expect("request should encode");

            assert_eq!(&encoded[..8], &(input.len() as u64).to_be_bytes());
            assert_eq!(&encoded[8..], input);
        }

        #[test]
        fn protocol_rejects_wrong_identity() {
            let mut response = WorkerResponse::control("success");
            response.protocol_version += 1;
            let encoded = serde_json::to_vec(&response).expect("response should serialize");
            assert_eq!(decode_response(&encoded), Err(ParentFailure::Protocol));

            let mut response = WorkerResponse::control("success");
            response.memory_limit_kind.push_str("-wrong");
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

        #[test]
        fn captured_response_overflow_precedes_deadline_classification() {
            assert_eq!(
                deadline_or_response_limit(true, true, Ok(vec![0; RESPONSE_LIMIT_BYTES + 1])),
                ParentFailure::ResponseLimit {
                    observed_bytes: RESPONSE_LIMIT_BYTES + 1,
                    kill_sent: true,
                    reaped: true,
                }
            );
        }
    }
}

#[cfg(all(
    cxf_json_semantic_harness,
    any(target_os = "linux", target_os = "macos")
))]
fn main() -> std::process::ExitCode {
    enabled::main()
}

#[cfg(not(all(
    cxf_json_semantic_harness,
    any(target_os = "linux", target_os = "macos")
)))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "the native worker-containment report requires Linux or macOS and CXF_JSON_SEMANTIC_HARNESS=1"
    );
    std::process::ExitCode::FAILURE
}
