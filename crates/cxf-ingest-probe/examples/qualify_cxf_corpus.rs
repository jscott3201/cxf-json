use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    str,
    time::Instant,
};

use cxf_ingest_probe::{ProbeDiagnostic, parse_json_ld};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct FileReport {
    path: String,
    expected_failure: bool,
    #[serde(skip)]
    expected_failure_matched: bool,
    input_bytes: Option<usize>,
    elapsed_micros: u128,
    quad_count: Option<usize>,
    failure: Option<FileFailure>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FileFailure {
    Parse { diagnostic: ProbeDiagnostic },
    Read { message: String },
}

#[derive(Debug, Serialize)]
struct CorpusReport {
    git_root: Option<String>,
    git_origin_verified: bool,
    git_commit: Option<String>,
    files: Vec<FileReport>,
    file_count: usize,
    passed: usize,
    expected_failures: usize,
    unexpected_failures: usize,
    unexpected_passes: usize,
    read_failures: usize,
    input_bytes: usize,
    quad_count: usize,
    elapsed_micros: u128,
}

fn main() -> ExitCode {
    match qualify(env::args_os().skip(1)) {
        Ok(report) => {
            let failed = report.has_regressions();
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

impl CorpusReport {
    fn has_regressions(&self) -> bool {
        self.unexpected_failures > 0 || self.unexpected_passes > 0 || self.read_failures > 0
    }
}

fn qualify(arguments: impl IntoIterator<Item = OsString>) -> Result<CorpusReport, String> {
    let started = Instant::now();
    let mut expected_failures = Vec::new();
    let mut roots = Vec::new();
    let mut git_root = None;
    let mut git_origin = None;
    let mut git_commit = None;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == "--expect-failure" {
            let path = arguments
                .next()
                .ok_or_else(|| "--expect-failure requires a file path".to_owned())?;
            let message = arguments.next().ok_or_else(|| {
                "--expect-failure requires a file path and exact message".to_owned()
            })?;
            let message = message.to_string_lossy().into_owned();
            if message.is_empty() {
                return Err("--expect-failure message must not be empty".to_owned());
            }
            expected_failures.push((PathBuf::from(path), message));
        } else if argument == "--git-root" {
            let path = arguments
                .next()
                .ok_or_else(|| "--git-root requires a repository path".to_owned())?;
            git_root = Some(PathBuf::from(path));
        } else if argument == "--git-commit" {
            let commit = arguments
                .next()
                .ok_or_else(|| "--git-commit requires a commit ID".to_owned())?;
            git_commit = Some(commit.to_string_lossy().into_owned());
        } else if argument == "--git-origin" {
            let origin = arguments
                .next()
                .ok_or_else(|| "--git-origin requires a remote URL".to_owned())?;
            git_origin = Some(origin.to_string_lossy().into_owned());
        } else if argument.to_string_lossy().starts_with("--") {
            return Err(format!("unknown option {}", argument.to_string_lossy()));
        } else {
            roots.push(PathBuf::from(argument));
        }
    }
    if roots.is_empty() {
        return Err("usage: qualify_cxf_corpus \
             [--git-root <repo> --git-origin <url> --git-commit <commit>] \
             [--expect-failure <file> <exact-message>] <file-or-directory>..."
            .to_owned());
    }

    let mut paths = Vec::new();
    let mut canonical_roots = Vec::new();
    for root in roots {
        reject_symlink_components(&root)?;
        let root = canonicalize(&root)?;
        collect_cxf(&root, &mut paths)?;
        canonical_roots.push(root);
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Err("no .jsonld or .cxf.json files found under the supplied paths".to_owned());
    }

    let mut expected_failure_map = BTreeMap::new();
    for (path, message) in expected_failures {
        let path = canonicalize(&path)?;
        if expected_failure_map.insert(path.clone(), message).is_some() {
            return Err(format!(
                "duplicate expected-failure path: {}",
                path.display()
            ));
        }
    }
    for expected in expected_failure_map.keys() {
        if !paths.contains(expected) {
            return Err(format!(
                "expected-failure path is outside the selected corpus: {}",
                expected.display()
            ));
        }
    }

    let git = match (git_root, git_origin, git_commit) {
        (Some(root), Some(origin), Some(commit)) => {
            validate_git_origin(&origin)?;
            reject_symlink_components(&root)?;
            let root = canonicalize(&root)?;
            verify_git_corpus(&root, &origin, &commit, &canonical_roots, &paths)?;
            Some((root, origin, commit))
        }
        (None, None, None) => None,
        _ => {
            return Err(
                "--git-root, --git-origin, and --git-commit must be used together".to_owned(),
            );
        }
    };

    let mut files = Vec::with_capacity(paths.len());
    for path in &paths {
        files.push(qualify_file(
            path,
            expected_failure_map.get(path).map(String::as_str),
            git.as_ref()
                .map(|(root, _, commit)| (root.as_path(), commit.as_str())),
        )?);
    }

    let passed = files
        .iter()
        .filter(|file| !file.expected_failure && file.failure.is_none())
        .count();
    let expected_failures = files
        .iter()
        .filter(|file| file.expected_failure_matched)
        .count();
    let unexpected_failures = files
        .iter()
        .filter(|file| {
            matches!(file.failure, Some(FileFailure::Parse { .. }))
                && !file.expected_failure_matched
        })
        .count();
    let unexpected_passes = files
        .iter()
        .filter(|file| file.expected_failure && file.failure.is_none())
        .count();
    let read_failures = files
        .iter()
        .filter(|file| matches!(file.failure, Some(FileFailure::Read { .. })))
        .count();
    let input_bytes = files.iter().filter_map(|file| file.input_bytes).sum();
    let quad_count = files.iter().filter_map(|file| file.quad_count).sum();
    if let Some((root, origin, commit)) = &git {
        verify_git_corpus(root, origin, commit, &canonical_roots, &paths)?;
    }
    let report = CorpusReport {
        git_root: git
            .as_ref()
            .map(|(root, _, _)| root.to_string_lossy().into_owned()),
        git_origin_verified: git.is_some(),
        git_commit: git.as_ref().map(|(_, _, commit)| commit.clone()),
        file_count: files.len(),
        passed,
        expected_failures,
        unexpected_failures,
        unexpected_passes,
        read_failures,
        input_bytes,
        quad_count,
        elapsed_micros: started.elapsed().as_micros(),
        files,
    };
    Ok(report)
}

fn canonicalize(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|error| format!("failed to resolve {}: {error}", path.display()))
}

fn reject_symlink_components(path: &Path) -> Result<(), String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        env::current_dir()
            .map_err(|error| format!("failed to read current directory: {error}"))?
            .join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("failed to inspect {}: {error}", current.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("refusing symlink component {}", current.display()));
        }
    }
    Ok(())
}

fn verify_git_corpus(
    repository: &Path,
    expected_origin: &str,
    expected_commit: &str,
    roots: &[PathBuf],
    paths: &[PathBuf],
) -> Result<(), String> {
    verify_git_version(repository)?;
    let top_level = git_output(repository, ["rev-parse", "--show-toplevel"])?;
    let top_level = canonicalize(Path::new(top_level.trim()))?;
    if top_level != repository {
        return Err("--git-root must name the repository top level".to_owned());
    }
    let actual_origin = git_output(repository, ["remote", "get-url", "origin"])?;
    if actual_origin.trim() != expected_origin {
        return Err("Git corpus origin does not match the approved origin".to_owned());
    }
    let actual_commit = git_output(repository, ["rev-parse", "HEAD"])?;
    if actual_commit.trim() != expected_commit {
        return Err(format!(
            "Git corpus commit mismatch: expected {expected_commit}, got {}",
            actual_commit.trim()
        ));
    }

    let relative_roots = roots
        .iter()
        .map(|root| {
            let relative = root.strip_prefix(repository).map_err(|_| {
                format!(
                    "corpus root {} is outside Git repository {}",
                    root.display(),
                    repository.display()
                )
            })?;
            Ok(if relative.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                relative.to_owned()
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut tracked = git_command(repository);
    tracked
        .args([
            "--literal-pathspecs",
            "ls-tree",
            "-r",
            "-z",
            expected_commit,
            "--",
        ])
        .args(&relative_roots);
    let tracked = command_bytes(tracked, "git ls-tree")?;
    let tracked = tracked
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let separator = record
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or_else(|| "git ls-tree returned an invalid record".to_owned())?;
            let metadata = str::from_utf8(&record[..separator])
                .map_err(|_| "git ls-tree returned non-UTF-8 metadata".to_owned())?;
            let path = str::from_utf8(&record[separator + 1..])
                .map_err(|_| "Git corpus contains a non-UTF-8 tracked path".to_owned())?;
            let path = repository.join(path);
            if is_cxf_path(&path) {
                let mut fields = metadata.split(' ');
                let mode = fields.next();
                let kind = fields.next();
                let object = fields.next();
                if !matches!(mode, Some("100644" | "100755"))
                    || kind != Some("blob")
                    || object.is_none()
                    || fields.next().is_some()
                {
                    return Err(format!(
                        "Git corpus path has unsupported tree mode: {}",
                        path.display()
                    ));
                }
            }
            Ok(path)
        })
        .filter_map(|path: Result<PathBuf, String>| match path {
            Ok(path) if is_cxf_path(&path) => Some(Ok(path)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let selected = paths.iter().cloned().collect::<BTreeSet<_>>();
    if tracked != selected {
        return Err(
            "selected corpus does not match CXF files in the approved commit tree".to_owned(),
        );
    }
    Ok(())
}

fn verify_git_version(repository: &Path) -> Result<(), String> {
    let output = git_output(repository, ["--version"])?;
    let (major, minor) = parse_git_version(&output)?;
    if (major, minor) < (2, 45) {
        return Err("Git-backed corpus mode requires Git 2.45 or newer".to_owned());
    }
    Ok(())
}

fn parse_git_version(output: &str) -> Result<(u64, u64), String> {
    let version = output
        .split_whitespace()
        .nth(2)
        .ok_or_else(|| "git --version returned an unrecognized version".to_owned())?;
    let mut components = version.split('.');
    let major = components
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| "git --version returned an unrecognized major version".to_owned())?;
    let minor = components
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| "git --version returned an unrecognized minor version".to_owned())?;
    Ok((major, minor))
}

fn validate_git_origin(origin: &str) -> Result<(), String> {
    if origin.contains(['?', '#']) {
        return Err("--git-origin must not contain a URL query or fragment".to_owned());
    }
    if let Some(authority_start) = origin.find("://").map(|position| position + 3) {
        let authority_end = origin[authority_start..]
            .find('/')
            .map_or(origin.len(), |position| authority_start + position);
        if origin[authority_start..authority_end].contains('@') {
            return Err("--git-origin must not contain URL credentials".to_owned());
        }
    }
    Ok(())
}

fn git_output<const N: usize>(repository: &Path, arguments: [&str; N]) -> Result<String, String> {
    let mut command = git_command(repository);
    command.args(arguments);
    let output = command_output(command, "git")?;
    Ok(output)
}

fn git_command(repository: &Path) -> Command {
    let mut command = Command::new("git");
    let null_config = if cfg!(windows) { "NUL" } else { "/dev/null" };
    command
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_PREFIX")
        .env_remove("GIT_GRAFT_FILE")
        .env_remove("GIT_REPLACE_REF_BASE")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_GLOBAL")
        .env_remove("GIT_LITERAL_PATHSPECS")
        .env_remove("GIT_GLOB_PATHSPECS")
        .env_remove("GIT_NOGLOB_PATHSPECS")
        .env_remove("GIT_ICASE_PATHSPECS")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_config)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
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

fn command_output(command: Command, name: &str) -> Result<String, String> {
    let output = command_bytes(command, name)?;
    String::from_utf8(output).map_err(|_| format!("{name} returned non-UTF-8 output"))
}

fn command_bytes(mut command: Command, name: &str) -> Result<Vec<u8>, String> {
    let output = command
        .output()
        .map_err(|error| format!("failed to run {name}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn collect_cxf(path: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    path.to_str()
        .ok_or_else(|| format!("non-UTF-8 path is unsupported: {}", path.display()))?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("refusing symlink {}", path.display()));
    }
    if metadata.is_file() {
        if is_cxf_path(path) {
            paths.push(path.to_owned());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read directory entry: {error}"))?;
        collect_cxf(&entry.path(), paths)?;
    }
    Ok(())
}

fn is_cxf_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".jsonld") || name.ends_with(".cxf.json"))
}

fn qualify_file(
    path: &Path,
    expected_failure: Option<&str>,
    git_source: Option<(&Path, &str)>,
) -> Result<FileReport, String> {
    let input_path = path;
    let path = input_path
        .to_str()
        .ok_or_else(|| format!("non-UTF-8 path is unsupported: {}", input_path.display()))?
        .to_owned();
    let started = Instant::now();
    let input = match git_source {
        Some((repository, commit)) => read_git_blob(repository, commit, input_path),
        None => fs::read(input_path).map_err(|error| error.to_string()),
    };
    let input = match input {
        Ok(input) => input,
        Err(error) => {
            let message = if git_source.is_some() {
                "external fixture read failed (diagnostic redacted)".to_owned()
            } else {
                error
            };
            return Ok(FileReport {
                path,
                expected_failure: expected_failure.is_some(),
                expected_failure_matched: false,
                input_bytes: None,
                elapsed_micros: started.elapsed().as_micros(),
                quad_count: None,
                failure: Some(FileFailure::Read { message }),
            });
        }
    };
    let result = parse_json_ld(&input);
    let elapsed_micros = started.elapsed().as_micros();
    let (quad_count, failure, expected_failure_matched) = match result {
        Ok(report) => (Some(report.quads.len()), None, false),
        Err(failure) => {
            let mut diagnostic = *failure.diagnostic;
            let expected_failure_matched = expected_failure == Some(diagnostic.message.as_str());
            redact_external_diagnostic(&mut diagnostic, git_source.is_some());
            (
                None,
                Some(FileFailure::Parse { diagnostic }),
                expected_failure_matched,
            )
        }
    };
    Ok(FileReport {
        path,
        expected_failure: expected_failure.is_some(),
        expected_failure_matched,
        input_bytes: Some(input.len()),
        elapsed_micros,
        quad_count,
        failure,
    })
}

fn redact_external_diagnostic(diagnostic: &mut ProbeDiagnostic, external: bool) {
    if external {
        diagnostic.message = "external parse failure (diagnostic redacted)".to_owned();
        diagnostic.range = None;
        diagnostic.pointer = None;
        diagnostic.rdf_term = None;
    }
}

fn read_git_blob(repository: &Path, commit: &str, path: &Path) -> Result<Vec<u8>, String> {
    let relative = path.strip_prefix(repository).map_err(|_| {
        format!(
            "corpus path {} is outside Git repository {}",
            path.display(),
            repository.display()
        )
    })?;
    let relative = relative
        .to_str()
        .ok_or_else(|| format!("non-UTF-8 path is unsupported: {}", path.display()))?;
    let mut command = git_command(repository);
    command.args(["cat-file", "blob", &format!("{commit}:{relative}")]);
    command_bytes(command, "git cat-file")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cxf_ingest_probe::{DiagnosticStage, SourcePosition, SourceRange};

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn git_mode_supported() -> bool {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace");
        verify_git_version(repository).is_ok()
    }

    #[test]
    fn canonical_roots_are_counted_once() {
        let fixtures = fixtures();
        let alias = fixtures.join("..").join("fixtures");
        let remote = fixtures.join("remote-context.jsonld");
        let report = qualify([
            OsString::from("--expect-failure"),
            remote.into_os_string(),
            OsString::from("No LoadDocumentCallback has been set to load remote contexts"),
            fixtures.into_os_string(),
            alias.into_os_string(),
        ])
        .expect("corpus should run");

        assert_eq!(report.file_count, 8);
        assert_eq!(report.expected_failures, 1);
        assert!(!report.has_regressions());
    }

    #[test]
    fn unclassified_parse_failure_is_a_regression() {
        let report = qualify([fixtures().join("remote-context.jsonld").into_os_string()])
            .expect("corpus should run");

        assert_eq!(report.unexpected_failures, 1);
        assert!(report.has_regressions());
    }

    #[test]
    fn changed_expected_failure_is_a_regression() {
        let remote = fixtures().join("remote-context.jsonld");
        let report = qualify([
            OsString::from("--expect-failure"),
            remote.clone().into_os_string(),
            OsString::from("different failure"),
            remote.into_os_string(),
        ])
        .expect("corpus should run");

        assert_eq!(report.expected_failures, 0);
        assert_eq!(report.unexpected_failures, 1);
        assert!(report.has_regressions());
    }

    #[test]
    fn empty_expected_failure_is_rejected() {
        let remote = fixtures().join("remote-context.jsonld");
        let error = qualify([
            OsString::from("--expect-failure"),
            remote.clone().into_os_string(),
            OsString::new(),
            remote.into_os_string(),
        ])
        .expect_err("empty failure should be rejected");

        assert!(error.contains("must not be empty"));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_path_component() {
        use std::os::unix::fs::symlink;

        let link = env::temp_dir().join(format!(
            "cxf-ingest-probe-symlink-root-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&link);
        symlink(fixtures(), &link).expect("symlink should be created");
        let linked_file = link.join("remote-context.jsonld");
        let error = qualify([linked_file.into_os_string()]).expect_err("symlink should fail");
        fs::remove_file(link).expect("symlink should be removed");

        assert!(error.contains("refusing symlink component"));
    }

    #[test]
    fn recognizes_only_cxf_file_suffixes() {
        assert!(is_cxf_path(Path::new("input.jsonld")));
        assert!(is_cxf_path(Path::new("input.export.cxf.json")));
        assert!(!is_cxf_path(Path::new("input.json")));
    }

    #[test]
    fn reads_exact_blob_from_git_object_database() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace");
        let commit = git_output(repository, ["rev-parse", "HEAD"]).expect("HEAD should resolve");
        let path = fixtures().join("embedded-context.jsonld");
        let bytes = read_git_blob(repository, commit.trim(), &path).expect("blob should read");

        assert_eq!(
            bytes,
            include_bytes!("../tests/fixtures/embedded-context.jsonld")
        );
    }

    #[test]
    fn git_commands_disable_lazy_fetch() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let command = git_command(repository);

        assert!(command.get_envs().any(|(name, value)| {
            name == "GIT_NO_LAZY_FETCH" && value.is_some_and(|value| value == "1")
        }));
    }

    #[test]
    fn commit_tree_membership_rejects_staged_deletion() {
        if !git_mode_supported() {
            return;
        }
        let repository = env::temp_dir().join(format!(
            "cxf-ingest-probe-staged-deletion-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&repository);
        fs::create_dir_all(repository.join("corpus")).expect("repository should be created");
        git_output(&repository, ["init"]).expect("repository should initialize");
        git_output(
            &repository,
            ["config", "user.email", "test@example.invalid"],
        )
        .expect("email should be configured");
        git_output(&repository, ["config", "user.name", "Test"])
            .expect("name should be configured");
        git_output(
            &repository,
            ["remote", "add", "origin", "test://approved-origin"],
        )
        .expect("origin should be configured");
        fs::write(repository.join("corpus/a.jsonld"), b"{}")
            .expect("first fixture should be written");
        fs::write(repository.join("corpus/b.jsonld"), b"{}")
            .expect("second fixture should be written");
        git_output(&repository, ["add", "corpus"]).expect("fixtures should be staged");
        git_output(&repository, ["commit", "-m", "fixtures"])
            .expect("fixtures should be committed");
        let commit = git_output(&repository, ["rev-parse", "HEAD"])
            .expect("commit should resolve")
            .trim()
            .to_owned();
        git_output(&repository, ["rm", "corpus/b.jsonld"])
            .expect("fixture deletion should be staged");

        let repository = canonicalize(&repository).expect("repository should canonicalize");
        let root = canonicalize(&repository.join("corpus")).expect("root should canonicalize");
        let selected = canonicalize(&repository.join("corpus/a.jsonld"))
            .expect("selected file should canonicalize");
        let error = verify_git_corpus(
            &repository,
            "test://approved-origin",
            &commit,
            &[root],
            &[selected],
        )
        .expect_err("commit-tree member omitted from the worktree should fail");

        fs::remove_dir_all(repository).expect("repository should be removed");
        assert!(error.contains("approved commit tree"), "{error}");
    }

    #[test]
    fn git_root_must_be_repository_top_level() {
        if !git_mode_supported() {
            return;
        }
        let repository = env::temp_dir().join(format!(
            "cxf-ingest-probe-nested-git-root-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&repository);
        fs::create_dir_all(repository.join("sub/corpus")).expect("repository should be created");
        git_output(&repository, ["init"]).expect("repository should initialize");
        git_output(
            &repository,
            ["config", "user.email", "test@example.invalid"],
        )
        .expect("email should be configured");
        git_output(&repository, ["config", "user.name", "Test"])
            .expect("name should be configured");
        git_output(
            &repository,
            ["remote", "add", "origin", "test://approved-origin"],
        )
        .expect("origin should be configured");
        fs::write(repository.join("sub/corpus/a.jsonld"), b"{}")
            .expect("fixture should be written");
        git_output(&repository, ["add", "sub/corpus/a.jsonld"]).expect("fixture should be staged");
        git_output(&repository, ["commit", "-m", "fixture"]).expect("fixture should be committed");
        let commit = git_output(&repository, ["rev-parse", "HEAD"])
            .expect("commit should resolve")
            .trim()
            .to_owned();

        let nested_root = canonicalize(&repository.join("sub")).expect("root should canonicalize");
        let corpus = canonicalize(&nested_root.join("corpus")).expect("corpus should canonicalize");
        let selected =
            canonicalize(&corpus.join("a.jsonld")).expect("selected file should canonicalize");
        let error = verify_git_corpus(
            &nested_root,
            "test://approved-origin",
            &commit,
            &[corpus],
            &[selected],
        )
        .expect_err("nested Git root should fail");

        fs::remove_dir_all(repository).expect("repository should be removed");
        assert!(error.contains("repository top level"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn commit_tree_symlink_mode_is_rejected() {
        use std::os::unix::fs::symlink;

        if !git_mode_supported() {
            return;
        }
        let repository = env::temp_dir().join(format!(
            "cxf-ingest-probe-tree-symlink-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&repository);
        fs::create_dir_all(repository.join("corpus")).expect("repository should be created");
        git_output(&repository, ["init"]).expect("repository should initialize");
        git_output(
            &repository,
            ["config", "user.email", "test@example.invalid"],
        )
        .expect("email should be configured");
        git_output(&repository, ["config", "user.name", "Test"])
            .expect("name should be configured");
        git_output(
            &repository,
            ["remote", "add", "origin", "test://approved-origin"],
        )
        .expect("origin should be configured");
        symlink("target", repository.join("corpus/link.jsonld"))
            .expect("fixture symlink should be created");
        git_output(&repository, ["add", "corpus/link.jsonld"]).expect("fixture should be staged");
        git_output(&repository, ["commit", "-m", "fixture symlink"])
            .expect("fixture should be committed");
        let commit = git_output(&repository, ["rev-parse", "HEAD"])
            .expect("commit should resolve")
            .trim()
            .to_owned();
        fs::remove_file(repository.join("corpus/link.jsonld"))
            .expect("worktree symlink should be removed");
        fs::write(repository.join("corpus/link.jsonld"), b"target")
            .expect("symlink payload should be materialized as a regular file");

        let repository = canonicalize(&repository).expect("repository should canonicalize");
        let corpus = canonicalize(&repository.join("corpus")).expect("corpus should canonicalize");
        let selected =
            canonicalize(&corpus.join("link.jsonld")).expect("selected file should canonicalize");
        let error = verify_git_corpus(
            &repository,
            "test://approved-origin",
            &commit,
            &[corpus],
            &[selected],
        )
        .expect_err("commit-tree symlink should fail");

        fs::remove_dir_all(repository).expect("repository should be removed");
        assert!(error.contains("unsupported tree mode"), "{error}");
    }

    #[test]
    fn commit_tree_roots_are_literal_and_may_select_repository_root() {
        if !git_mode_supported() {
            return;
        }
        let repository = env::temp_dir().join(format!(
            "cxf-ingest-probe-literal-root-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&repository);
        fs::create_dir_all(repository.join("corpus[1]")).expect("repository should be created");
        git_output(&repository, ["init"]).expect("repository should initialize");
        git_output(
            &repository,
            ["config", "user.email", "test@example.invalid"],
        )
        .expect("email should be configured");
        git_output(&repository, ["config", "user.name", "Test"])
            .expect("name should be configured");
        git_output(
            &repository,
            ["remote", "add", "origin", "test://approved-origin"],
        )
        .expect("origin should be configured");
        fs::write(repository.join("corpus[1]/a.jsonld"), b"{}").expect("fixture should be written");
        git_output(&repository, ["add", "corpus[1]/a.jsonld"]).expect("fixture should be staged");
        git_output(&repository, ["commit", "-m", "fixture"]).expect("fixture should be committed");
        let commit = git_output(&repository, ["rev-parse", "HEAD"])
            .expect("commit should resolve")
            .trim()
            .to_owned();

        let repository = canonicalize(&repository).expect("repository should canonicalize");
        let corpus =
            canonicalize(&repository.join("corpus[1]")).expect("literal root should canonicalize");
        let selected =
            canonicalize(&corpus.join("a.jsonld")).expect("selected file should canonicalize");
        verify_git_corpus(
            &repository,
            "test://approved-origin",
            &commit,
            std::slice::from_ref(&corpus),
            std::slice::from_ref(&selected),
        )
        .expect("pathspec syntax should be treated literally");
        verify_git_corpus(
            &repository,
            "test://approved-origin",
            &commit,
            std::slice::from_ref(&repository),
            std::slice::from_ref(&selected),
        )
        .expect("repository root should select the full tree");

        fs::remove_dir_all(repository).expect("repository should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn missing_promisor_object_does_not_invoke_remote_helper() {
        use std::os::unix::fs::PermissionsExt;

        if !git_mode_supported() {
            return;
        }
        let repository = env::temp_dir().join(format!(
            "cxf-ingest-probe-missing-promisor-{}",
            std::process::id()
        ));
        let helper_directory = repository.join("bin");
        let marker = repository.join("remote-helper-invoked");
        let _ = fs::remove_dir_all(&repository);
        fs::create_dir_all(&helper_directory).expect("helper directory should be created");
        git_output(&repository, ["init"]).expect("repository should initialize");
        git_output(&repository, ["config", "core.repositoryformatversion", "1"])
            .expect("repository format should be configured");
        git_output(&repository, ["config", "extensions.partialclone", "origin"])
            .expect("partial clone should be configured");
        git_output(
            &repository,
            ["config", "remote.origin.url", "probe::unused"],
        )
        .expect("promisor URL should be configured");
        git_output(&repository, ["config", "remote.origin.promisor", "true"])
            .expect("promisor remote should be configured");
        git_output(
            &repository,
            ["config", "remote.origin.partialclonefilter", "blob:none"],
        )
        .expect("partial clone filter should be configured");

        let helper = helper_directory.join("git-remote-probe");
        fs::write(&helper, b"#!/bin/sh\ntouch \"$MARKER\"\nexit 1\n")
            .expect("remote helper should be written");
        let mut permissions = fs::metadata(&helper)
            .expect("remote helper metadata should read")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&helper, permissions).expect("remote helper should be executable");
        let path = env::join_paths(
            std::iter::once(helper_directory.clone()).chain(
                env::var_os("PATH")
                    .as_deref()
                    .map(env::split_paths)
                    .into_iter()
                    .flatten(),
            ),
        )
        .expect("test PATH should be valid");

        let mut command = git_command(&repository);
        command
            .args([
                "cat-file",
                "blob",
                "1111111111111111111111111111111111111111",
            ])
            .env("PATH", path)
            .env("MARKER", &marker);
        command_bytes(command, "git cat-file")
            .expect_err("missing promised object should not resolve");

        assert!(!marker.exists(), "remote helper must not run");
        fs::remove_dir_all(repository).expect("repository should be removed");
    }

    #[test]
    fn redacts_unexpected_external_diagnostics() {
        let mut diagnostic = ProbeDiagnostic {
            stage: DiagnosticStage::JsonLd,
            message: "source value: secret".to_owned(),
            range: Some(SourceRange {
                start: SourcePosition {
                    offset: 3,
                    line: 0,
                    column: 3,
                },
                end: SourcePosition {
                    offset: 4,
                    line: 0,
                    column: 4,
                },
            }),
            pointer: Some("/secret".to_owned()),
            rdf_term: Some("https://example.test/secret".to_owned()),
        };

        redact_external_diagnostic(&mut diagnostic, true);

        assert_eq!(
            diagnostic.message,
            "external parse failure (diagnostic redacted)"
        );
        assert_eq!(diagnostic.range, None);
        assert_eq!(diagnostic.pointer, None);
        assert_eq!(diagnostic.rdf_term, None);
    }

    #[test]
    fn redacts_external_blob_read_errors() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace");
        let path = fixtures().join("embedded-context.jsonld");
        let report = qualify_file(
            &path,
            None,
            Some((repository, "1111111111111111111111111111111111111111")),
        )
        .expect("read failure should produce a report");

        let Some(FileFailure::Read { message }) = report.failure else {
            panic!("expected a read failure");
        };
        assert_eq!(
            message,
            "external fixture read failed (diagnostic redacted)"
        );
    }

    #[test]
    fn always_redacts_external_failure_messages() {
        let mut diagnostic = ProbeDiagnostic {
            stage: DiagnosticStage::JsonLd,
            message: "expected classification".to_owned(),
            range: None,
            pointer: None,
            rdf_term: None,
        };

        redact_external_diagnostic(&mut diagnostic, true);

        assert_eq!(
            diagnostic.message,
            "external parse failure (diagnostic redacted)"
        );
    }

    #[test]
    fn parses_git_version_for_lazy_fetch_requirement() {
        assert_eq!(
            parse_git_version("git version 2.45.0\n").expect("version should parse"),
            (2, 45)
        );
        assert_eq!(
            parse_git_version("git version 2.50.1 (Apple Git-155)\n")
                .expect("vendor version should parse"),
            (2, 50)
        );
    }

    #[test]
    fn rejects_credentials_in_reported_origin() {
        assert!(
            validate_git_origin("https://user:token@example.test/owner/repository.git").is_err()
        );
        assert!(validate_git_origin("https://example.test/repository.git?token=secret").is_err());
        assert!(validate_git_origin("https://example.test/repository.git#secret").is_err());
        assert!(validate_git_origin("git@example.test:owner/repository.git").is_ok());
    }
}
