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
    expected_failure: Option<String>,
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
        } else if argument.to_string_lossy().starts_with("--") {
            return Err(format!("unknown option {}", argument.to_string_lossy()));
        } else {
            roots.push(PathBuf::from(argument));
        }
    }
    if roots.is_empty() {
        return Err("usage: qualify_cxf_corpus \
             [--git-root <repo> --git-commit <commit>] \
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

    let git = match (git_root, git_commit) {
        (Some(root), Some(commit)) => {
            reject_symlink_components(&root)?;
            let root = canonicalize(&root)?;
            verify_git_corpus(&root, &commit, &canonical_roots, &paths)?;
            Some((root, commit))
        }
        (None, None) => None,
        _ => return Err("--git-root and --git-commit must be used together".to_owned()),
    };

    let mut files = Vec::with_capacity(paths.len());
    for path in &paths {
        files.push(qualify_file(
            path,
            expected_failure_map.get(path).map(String::as_str),
        )?);
    }

    let passed = files
        .iter()
        .filter(|file| file.expected_failure.is_none() && file.failure.is_none())
        .count();
    let expected_failures = files
        .iter()
        .filter(|file| matches_expected_failure(file))
        .count();
    let unexpected_failures = files
        .iter()
        .filter(|file| {
            matches!(file.failure, Some(FileFailure::Parse { .. }))
                && !matches_expected_failure(file)
        })
        .count();
    let unexpected_passes = files
        .iter()
        .filter(|file| file.expected_failure.is_some() && file.failure.is_none())
        .count();
    let read_failures = files
        .iter()
        .filter(|file| matches!(file.failure, Some(FileFailure::Read { .. })))
        .count();
    let input_bytes = files.iter().filter_map(|file| file.input_bytes).sum();
    let quad_count = files.iter().filter_map(|file| file.quad_count).sum();
    if let Some((root, commit)) = &git {
        verify_git_corpus(root, commit, &canonical_roots, &paths)?;
    }
    let report = CorpusReport {
        git_root: git
            .as_ref()
            .map(|(root, _)| root.to_string_lossy().into_owned()),
        git_commit: git.as_ref().map(|(_, commit)| commit.clone()),
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
    expected_commit: &str,
    roots: &[PathBuf],
    paths: &[PathBuf],
) -> Result<(), String> {
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
            root.strip_prefix(repository).map_err(|_| {
                format!(
                    "corpus root {} is outside Git repository {}",
                    root.display(),
                    repository.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut status = git_command(repository);
    status
        .args(["status", "--porcelain=v1", "--untracked-files=all", "--"])
        .args(&relative_roots);
    let status = command_output(status, "git status")?;
    if !status.trim().is_empty() {
        return Err(format!("Git corpus has worktree changes:\n{status}"));
    }

    let mut tracked = git_command(repository);
    tracked.args(["ls-files", "-z", "--"]).args(&relative_roots);
    let tracked = command_bytes(tracked, "git ls-files")?;
    let tracked = tracked
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            let path = str::from_utf8(path)
                .map_err(|_| "Git corpus contains a non-UTF-8 tracked path".to_owned())?;
            Ok(repository.join(path))
        })
        .filter_map(|path: Result<PathBuf, String>| match path {
            Ok(path) if is_cxf_path(&path) => Some(Ok(path)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let selected = paths.iter().cloned().collect::<BTreeSet<_>>();
    if tracked != selected {
        return Err("selected corpus does not match tracked CXF files under Git roots".to_owned());
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
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
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

fn matches_expected_failure(file: &FileReport) -> bool {
    match (&file.expected_failure, &file.failure) {
        (Some(expected), Some(FileFailure::Parse { diagnostic })) => {
            diagnostic.message == *expected
        }
        _ => false,
    }
}

fn qualify_file(path: &Path, expected_failure: Option<&str>) -> Result<FileReport, String> {
    let path = path
        .to_str()
        .ok_or_else(|| format!("non-UTF-8 path is unsupported: {}", path.display()))?
        .to_owned();
    let started = Instant::now();
    let input = match fs::read(&path) {
        Ok(input) => input,
        Err(error) => {
            return Ok(FileReport {
                path,
                expected_failure: expected_failure.map(str::to_owned),
                input_bytes: None,
                elapsed_micros: started.elapsed().as_micros(),
                quad_count: None,
                failure: Some(FileFailure::Read {
                    message: error.to_string(),
                }),
            });
        }
    };
    let result = parse_json_ld(&input);
    let elapsed_micros = started.elapsed().as_micros();
    let (quad_count, failure) = match result {
        Ok(report) => (Some(report.quads.len()), None),
        Err(failure) => (
            None,
            Some(FileFailure::Parse {
                diagnostic: failure.diagnostic,
            }),
        ),
    };
    Ok(FileReport {
        path,
        expected_failure: expected_failure.map(str::to_owned),
        input_bytes: Some(input.len()),
        elapsed_micros,
        quad_count,
        failure,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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
}
