use std::{
    env,
    fmt::Write as _,
    path::Path,
    process::{Command, Stdio},
};

#[cfg(not(target_arch = "wasm32"))]
pub use cxf_json::test_support::{MeasuredObservation, NativeTiming};
pub use cxf_json::test_support::{Metrics, Observation, OutcomeKind};
use cxf_json::{DocumentIri, ParseOptions, test_support};

pub const WORKLOAD_VERSION: u64 = 1;
pub const RETAINED_VALUES: usize = 32_768;
pub const VERIFIED_REVISION: &str = match option_env!("CXF_VERIFIED_BENCHMARK_REVISION") {
    Some(revision) => revision,
    None => "",
};

pub fn options() -> ParseOptions {
    ParseOptions::new().with_document_iri(
        DocumentIri::parse("https://benchmark.example/input")
            .expect("fixed benchmark IRI should be valid"),
    )
}

pub fn observe(input: &[u8], options: &ParseOptions) -> Observation {
    test_support::observe(input, options)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn measure(input: &[u8], options: &ParseOptions) -> MeasuredObservation {
    test_support::measure(input, options)
}

pub fn retained_values_input(values: usize) -> Vec<u8> {
    let mut input = String::with_capacity(values * 4 + 96);
    input.push_str(r#"{"@id":"https://e.test/s","https://e.test/p":["#);
    for index in 0..values {
        if index > 0 {
            input.push(',');
        }
        input.push_str(r#""v""#);
    }
    input.push_str("]}");
    input.into_bytes()
}

pub fn revision_word(index: usize) -> u32 {
    if VERIFIED_REVISION.len() != 40 {
        return 0;
    }
    u32::from_str_radix(&VERIFIED_REVISION[index * 8..index * 8 + 8], 16).unwrap_or(0)
}

pub fn verify_instrumentation_revision() -> Result<(), String> {
    if !is_commit_id(VERIFIED_REVISION) {
        return Err("the artifact was not built in benchmark mode".to_owned());
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "probe crate must be inside the repository workspace".to_owned())?;
    let head = git_output(repository, &["rev-parse", "HEAD"])?;
    if head.trim() != VERIFIED_REVISION {
        return Err(format!(
            "instrumentation revision mismatch: expected {VERIFIED_REVISION}, got {}",
            head.trim()
        ));
    }
    let status = git_output(
        repository,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )?;
    if !status.is_empty() {
        return Err("production semantic measurements require a clean worktree".to_owned());
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if !output.status.success() {
        let mut message = String::new();
        write!(
            message,
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .expect("writing to a string cannot fail");
        return Err(message);
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
