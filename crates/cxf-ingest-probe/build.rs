use std::{env, path::Path, process::Command};

fn main() {
    println!("cargo::rustc-check-cfg=cfg(cxf_json_semantic_harness)");
    println!("cargo:rerun-if-env-changed=CXF_JSON_SEMANTIC_HARNESS");
    println!("cargo:rerun-if-env-changed=CXF_BENCHMARK_REVISION");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=examples");
    println!("cargo:rerun-if-changed=tests");
    println!("cargo:rerun-if-changed=Cargo.toml");

    match env::var("CXF_JSON_SEMANTIC_HARNESS") {
        Err(env::VarError::NotPresent) => {}
        Ok(value) if value == "1" => println!("cargo::rustc-cfg=cxf_json_semantic_harness"),
        Ok(_) | Err(env::VarError::NotUnicode(_)) => {
            panic!("CXF_JSON_SEMANTIC_HARNESS must be unset or equal to 1")
        }
    }

    let Ok(revision) = env::var("CXF_BENCHMARK_REVISION") else {
        return;
    };
    if !is_commit_id(&revision) {
        panic!("CXF_BENCHMARK_REVISION must be a 40-digit commit ID");
    }
    let manifest = env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must set CARGO_MANIFEST_DIR");
    let repository = Path::new(&manifest)
        .parent()
        .and_then(Path::parent)
        .expect("probe crate must be inside the repository workspace");
    let head = git_output(repository, &["rev-parse", "HEAD"]);
    assert_eq!(
        head.trim(),
        revision,
        "CXF_BENCHMARK_REVISION does not match HEAD"
    );
    let status = git_output(
        repository,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    );
    assert!(
        status.is_empty(),
        "benchmark artifacts require a clean worktree"
    );
    println!("cargo:rustc-env=CXF_VERIFIED_BENCHMARK_REVISION={revision}");
}

fn is_commit_id(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn git_output(repository: &Path, arguments: &[&str]) -> String {
    let output = git_command(repository)
        .args(arguments)
        .output()
        .expect("Git must run while building benchmark artifacts");
    assert!(
        output.status.success(),
        "Git failed while building benchmark artifacts: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout).expect("Git output must be UTF-8")
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
