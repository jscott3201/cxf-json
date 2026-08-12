import argparse
import hashlib
import html
import os
import platform
import re
import selectors
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unicodedata
from pathlib import Path
from urllib.parse import quote_from_bytes, unquote_to_bytes, urlsplit


TOOL_VERSION = 2
CANONICAL_REMOTE_URL = "https://github.com/jscott3201/cxf-json.git"
CANONICAL_GIT_PROGRAMS = {
    (
        "9048038886ac36210fbb616b49b0707465f63683cb04e33a2013baf95f746938",
        "d5aeb8954c72119600d48fc62fdf5bb9295afa85fe6523f70e03828e22b4bee9",
    ),
}
DEFAULT_LARGE_BLOB_BYTES = 1_048_576
DEFAULT_MAX_SCANNED_OBJECT_BYTES = 4_194_304
MAX_SECRET_CANDIDATES_PER_OBJECT = 1_000
MAX_TOTAL_SECRET_CANDIDATES = 10_000
GIT_TIMEOUT_SECONDS = 120
MAX_REMOTE_ADVERTISEMENT_BYTES = 8_388_608
MAX_REMOTE_REFS = 10_000
MAX_REF_NAME_BYTES = 4_096
MAX_REACHABLE_COMMITS = 10_000
MAX_TREE_OUTPUT_BYTES = 33_554_432
MAX_HISTORICAL_TREE_ENTRIES = 500_000
MAX_HISTORICAL_PATH_BYTES = 33_554_432
MAX_PATH_BYTES = 4_096
MAX_UNIQUE_BLOBS = 100_000
MAX_TOTAL_SCANNED_OBJECT_BYTES = 268_435_456
MAX_CANDIDATE_PATHS = 20
MAX_GIT_TEXT_OUTPUT_BYTES = 4_194_304
MAX_GIT_PROGRAM_BYTES = 67_108_864
OID_PATTERN = re.compile(r"[0-9a-f]{40}")
SECRET_PATTERNS = [
    (
        "private-key-header",
        re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
    ),
    ("aws-access-key", re.compile(rb"(?<![A-Z0-9])(?:AKIA|ASIA)[A-Z0-9]{16}(?![A-Z0-9])")),
    ("github-token", re.compile(rb"(?<![A-Za-z0-9])gh[pousr]_[A-Za-z0-9]{36,255}")),
    ("github-fine-grained-token", re.compile(rb"github_pat_[A-Za-z0-9_]{20,255}")),
    ("slack-token", re.compile(rb"xox[baprs]-[A-Za-z0-9-]{10,}")),
    (
        "assigned-secret",
        re.compile(
            rb"(?i)(?:api[_-]?key|client[_-]?secret|access[_-]?token|auth[_-]?token|password|passwd)"
            rb"\s*[:=]\s*['\"]?[A-Za-z0-9_./+=-]{8,}"
        ),
    ),
]
GENERATED_SUFFIXES = {
    ".7z",
    ".a",
    ".class",
    ".dll",
    ".dylib",
    ".exe",
    ".gif",
    ".gz",
    ".ico",
    ".jar",
    ".jpeg",
    ".jpg",
    ".lock",
    ".map",
    ".o",
    ".pdf",
    ".png",
    ".so",
    ".tar",
    ".tgz",
    ".wasm",
    ".xz",
    ".zip",
}


class InventoryError(Exception):
    pass


def git_environment():
    return {
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_CONFIG_COUNT": "0",
        "GIT_GRAFT_FILE": os.devnull,
        "GIT_NO_LAZY_FETCH": "1",
        "GIT_NO_REPLACE_OBJECTS": "1",
        "GIT_TERMINAL_PROMPT": "0",
        "GCM_INTERACTIVE": "never",
        "LC_ALL": "C",
        "PATH": os.defpath,
    }


def safe_log_text(value):
    return "".join(
        character
        if not unicodedata.category(character).startswith("C")
        else f"\\u{ord(character):04x}"
        for character in str(value)
    )


def run_limited_process(command, environment, max_stdout_bytes, input_data=None):
    if os.name != "posix":
        raise InventoryError("history inventory requires POSIX process-group controls")
    selector = selectors.DefaultSelector()
    process = None
    return_code = None
    input_file = None
    stdout = bytearray()
    stderr = bytearray()

    def kill_process_group():
        if process is None:
            return
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except (PermissionError, ProcessLookupError):
            try:
                process.kill()
            except ProcessLookupError:
                pass

    try:
        if input_data is not None:
            input_file = tempfile.TemporaryFile()
            input_file.write(input_data)
            input_file.seek(0)
        process = subprocess.Popen(
            command,
            stdin=input_file if input_file is not None else subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            start_new_session=True,
        )
        assert process.stdout is not None
        assert process.stderr is not None
        streams = {
            process.stdout.fileno(): (
                process.stdout,
                stdout,
                max_stdout_bytes,
                "output",
            ),
            process.stderr.fileno(): (process.stderr, stderr, 65_536, "stderr"),
        }
        for file_descriptor, stream in streams.items():
            os.set_blocking(file_descriptor, False)
            selector.register(file_descriptor, selectors.EVENT_READ, stream)
        deadline = time.monotonic() + GIT_TIMEOUT_SECONDS
        while selector.get_map():
            remaining_time = deadline - time.monotonic()
            if remaining_time <= 0:
                raise InventoryError("Git command timed out")
            for key, _ in selector.select(remaining_time):
                stream, destination, limit, label = key.data
                try:
                    chunk = os.read(key.fd, 65_536)
                except BlockingIOError:
                    continue
                if not chunk:
                    selector.unregister(key.fd)
                    continue
                remaining_bytes = limit - len(destination)
                if remaining_bytes > 0:
                    destination.extend(chunk[:remaining_bytes])
                if len(chunk) > remaining_bytes:
                    raise InventoryError(
                        f"Git command {label} exceeded {limit} bytes"
                    )
        remaining_time = deadline - time.monotonic()
        if remaining_time <= 0:
            raise InventoryError("Git command timed out")
        while True:
            result = os.waitid(
                os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT
            )
            if result is not None:
                break
            if time.monotonic() >= deadline:
                raise InventoryError("Git command timed out")
            time.sleep(0.001)
    except subprocess.TimeoutExpired as error:
        raise InventoryError("Git command timed out") from error
    finally:
        selector.close()
        if process is not None:
            kill_process_group()
            try:
                return_code = process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                return_code = process.wait(timeout=5)
            if process.stdout is not None:
                process.stdout.close()
            if process.stderr is not None:
                process.stderr.close()
        if input_file is not None:
            input_file.close()
    if return_code is None:
        raise InventoryError("Git command did not report an exit status")
    if return_code != 0:
        message = redact_secret_text(
            stderr.decode("utf-8", "backslashreplace")
        ).strip()
        message = safe_log_text(message) or "Git command failed"
        raise InventoryError(message)
    return bytes(stdout)


def program_sha256(path):
    size = path.stat().st_size
    if size > MAX_GIT_PROGRAM_BYTES:
        raise InventoryError(
            f"Git program exceeds {MAX_GIT_PROGRAM_BYTES} bytes: {path.name}"
        )
    digest = hashlib.sha256()
    with path.open("rb") as program:
        for chunk in iter(lambda: program.read(65_536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_program(path, name):
    if not path.is_file() or not os.access(path, os.X_OK):
        raise InventoryError(f"{name} must be an executable file")
    if path.stat().st_size > MAX_GIT_PROGRAM_BYTES:
        raise InventoryError(f"{name} exceeds {MAX_GIT_PROGRAM_BYTES} bytes")
    with path.open("rb") as program:
        magic = program.read(4)
    native_magics = {
        b"\x7fELF",
        b"\xca\xfe\xba\xbe",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
    }
    if magic not in native_magics:
        raise InventoryError(f"{name} must be a native executable")


class GitRunner:
    def __init__(self, executable, expected_executable_sha256, expected_helper_sha256):
        executable = Path(executable)
        if not executable.is_absolute():
            raise InventoryError("--git-executable must be an absolute path")
        try:
            executable = executable.resolve(strict=True)
        except OSError as error:
            raise InventoryError("--git-executable does not exist") from error
        validate_program(executable, "--git-executable")
        if not isinstance(expected_executable_sha256, str) or not re.fullmatch(
            r"[0-9a-f]{64}", expected_executable_sha256
        ):
            raise InventoryError("expected Git executable SHA-256 is invalid")
        if not isinstance(expected_helper_sha256, str) or not re.fullmatch(
            r"[0-9a-f]{64}", expected_helper_sha256
        ):
            raise InventoryError("expected Git HTTPS helper SHA-256 is invalid")

        self.executable = str(executable)
        snapshot_path = Path(tempfile.mkdtemp(prefix="cxf-json-git-"))
        self.snapshot_path = snapshot_path
        snapshot_executable = snapshot_path / "git"
        snapshot_https_helper = snapshot_path / "git-remote-https"
        try:
            shutil.copyfile(executable, snapshot_executable)
            snapshot_executable.chmod(0o500)
            self.executable_sha256 = program_sha256(snapshot_executable)
            if self.executable_sha256 != expected_executable_sha256:
                raise InventoryError(
                    "Git executable snapshot does not match the expected SHA-256"
                )
            environment = git_environment()
            output = run_limited_process(
                [str(snapshot_executable), "--exec-path"],
                environment,
                MAX_PATH_BYTES,
            )
            try:
                exec_path = Path(output.decode("utf-8", "strict").strip())
            except UnicodeDecodeError as error:
                raise InventoryError("git --exec-path returned invalid UTF-8") from error
            if not exec_path.is_absolute() or not exec_path.is_dir():
                raise InventoryError(
                    "git --exec-path did not return an absolute directory"
                )
            installation_root = Path(os.path.commonpath([executable, exec_path]))
            if installation_root == Path(installation_root.anchor):
                raise InventoryError(
                    "Git executable and helper directory must share an installation root"
                )
            self.exec_path = str(exec_path.resolve())
            helper = Path(self.exec_path, "git-remote-https")
            try:
                helper = helper.resolve(strict=True)
                helper.relative_to(self.exec_path)
            except (OSError, ValueError) as error:
                raise InventoryError(
                    "Git HTTPS helper must resolve inside the helper directory"
                ) from error
            validate_program(helper, "Git HTTPS helper")
            self.https_helper = str(helper)
            shutil.copyfile(helper, snapshot_https_helper)
            snapshot_https_helper.chmod(0o500)
            self.https_helper_sha256 = program_sha256(snapshot_https_helper)
            if self.https_helper_sha256 != expected_helper_sha256:
                raise InventoryError(
                    "Git HTTPS helper snapshot does not match the expected SHA-256"
                )
            (snapshot_path / "git-upload-pack").symlink_to("git")
            snapshot_path.chmod(0o500)
        except (OSError, InventoryError):
            snapshot_path.chmod(0o700)
            shutil.rmtree(snapshot_path, ignore_errors=True)
            self.snapshot_path = None
            raise
        self.command = str(snapshot_executable)
        self.command_exec_path = str(snapshot_path)
        environment["GIT_EXEC_PATH"] = self.command_exec_path
        environment["PATH"] = os.pathsep.join(
            dict.fromkeys([self.command_exec_path])
        )
        self.environment = environment

    def verify_snapshot(self):
        if program_sha256(Path(self.command)) != self.executable_sha256:
            raise InventoryError("Git executable snapshot changed during inventory")
        if (
            program_sha256(Path(self.command_exec_path, "git-remote-https"))
            != self.https_helper_sha256
        ):
            raise InventoryError("Git HTTPS helper snapshot changed during inventory")

    def close(self):
        snapshot_path = getattr(self, "snapshot_path", None)
        if snapshot_path is not None:
            error = None
            try:
                self.verify_snapshot()
            except InventoryError as snapshot_error:
                error = snapshot_error
            finally:
                snapshot_path.chmod(0o700)
                shutil.rmtree(snapshot_path)
                self.snapshot_path = None
            if error is not None:
                raise error

    def __del__(self):
        try:
            self.close()
        except (InventoryError, OSError):
            pass

    def limited(self, repository, max_stdout_bytes, *arguments, input_data=None):
        if os.name != "posix":
            raise InventoryError("history inventory requires POSIX process-group controls")
        return run_limited_process(
            [
                self.command,
                f"--exec-path={self.command_exec_path}",
                "--no-replace-objects",
                "-c",
                "http.followRedirects=false",
                "-C",
                str(repository),
                *arguments,
            ],
            self.environment,
            max_stdout_bytes,
            input_data=input_data,
        )


def default_git_runner():
    raise InventoryError("an absolute Git executable is required")


def git_limited(
    repository, max_stdout_bytes, *arguments, runner=None, input_data=None
):
    return (runner or default_git_runner()).limited(
        repository, max_stdout_bytes, *arguments, input_data=input_data
    )


def git_text(repository, *arguments, runner=None):
    return git_limited(
        repository, MAX_GIT_TEXT_OUTPUT_BYTES, *arguments, runner=runner
    ).decode("utf-8", "backslashreplace")


def read_git_object(repository, object_type, object_id, size, runner=None):
    data = git_limited(
        repository,
        size,
        "cat-file",
        object_type,
        object_id,
        runner=runner,
    )
    if len(data) != size:
        raise InventoryError(f"short read for {object_type} {object_id}")
    header = f"{object_type} {size}\0".encode()
    if hashlib.sha1(header + data).hexdigest() != object_id:
        raise InventoryError(f"object ID mismatch for {object_type} {object_id}")
    return data


def validate_object_database(repository, runner=None):
    git_limited(
        repository,
        MAX_GIT_TEXT_OUTPUT_BYTES,
        "fsck",
        "--strict",
        "--no-dangling",
        "--no-reflogs",
        runner=runner,
    )


def parse_tag_object(data):
    header, separator, message = data.partition(b"\n\n")
    if not separator:
        raise InventoryError("annotated tag object has no message separator")
    headers = {}
    for line in header.splitlines():
        name, separator, value = line.partition(b" ")
        if separator:
            headers[name] = value
    target_type = headers.get(b"type", b"").decode("ascii", "replace")
    tagger = headers.get(b"tagger")
    if not target_type or tagger is None:
        raise InventoryError("annotated tag object has malformed headers")
    tagger_match = re.fullmatch(rb"(.*) <([^<>]*)> (\d+) ([+-]\d{4})", tagger)
    if tagger_match is None:
        raise InventoryError("annotated tag object has malformed tagger metadata")
    subject = message.splitlines()[0] if message.splitlines() else b""
    return target_type, {
        "tagger_name": tagger_match.group(1).decode("utf-8", "backslashreplace"),
        "tagger_email": tagger_match.group(2).decode("utf-8", "backslashreplace"),
        "tagger_date": (
            tagger_match.group(3) + b" " + tagger_match.group(4)
        ).decode("ascii"),
        "subject": subject.decode("utf-8", "backslashreplace"),
    }


def collect_refs(repository, runner=None):
    output = git_limited(
        repository,
        MAX_REMOTE_ADVERTISEMENT_BYTES,
        "for-each-ref",
        "--format=%(refname)%00%(objecttype)%00%(objectname)%00",
        runner=runner,
    )
    if output and not output.endswith(b"\0\n"):
        raise InventoryError("local ref inventory is incomplete")
    refs = []
    errors = []
    fields = output.replace(b"\0\n", b"\0").split(b"\0")
    if fields and fields[-1] == b"":
        fields.pop()
    if len(fields) % 3 != 0:
        raise InventoryError("malformed local ref inventory")
    for index in range(0, len(fields), 3):
        try:
            ref_name = encode_ref(fields[index])
            object_type = fields[index + 1].decode("ascii", "strict")
            object_id = fields[index + 2].decode("ascii", "strict")
        except UnicodeDecodeError as error:
            raise InventoryError("malformed local ref inventory") from error
        if len(ref_name.encode("utf-8", "backslashreplace")) > MAX_REF_NAME_BYTES:
            raise InventoryError(f"local ref name exceeds {MAX_REF_NAME_BYTES} bytes")
        if len(refs) == MAX_REMOTE_REFS:
            raise InventoryError(f"local ref count exceeds {MAX_REMOTE_REFS}")
        refs.append(
            {
                "name": ref_name,
                "display_name": encode_ref_display(fields[index]),
                "object_type": object_type,
                "object_id": object_id,
            }
        )
    return sorted(refs, key=lambda ref: ref["name"]), errors


def hydrate_refs(repository, refs, max_scanned_object_bytes, scan_budget, runner=None):
    hydrated = []
    errors = []
    for ref in refs:
        ref = dict(ref)
        object_type = ref["object_type"]
        object_id = ref["object_id"]
        display_name = ref["display_name"]
        tag_metadata = None
        tag_size = None
        ref_error_count = len(errors)
        if object_type == "tag":
            tag_size = int(
                git_text(
                    repository, "cat-file", "-s", object_id, runner=runner
                ).strip()
            )
            if tag_size > max_scanned_object_bytes:
                errors.append(f"{display_name} tag metadata exceeds the content scan cap")
                commit = None
                tag_metadata = {
                    "tagger_name": "(not scanned)",
                    "tagger_email": "(not scanned)",
                    "tagger_date": "(not scanned)",
                    "subject": "(not scanned)",
                }
            else:
                scan_budget["bytes"] += tag_size
                if scan_budget["bytes"] > MAX_TOTAL_SCANNED_OBJECT_BYTES:
                    raise InventoryError(
                        "aggregate scanned object bytes exceed "
                        f"{MAX_TOTAL_SCANNED_OBJECT_BYTES}"
                    )
                tag_data = read_git_object(
                    repository, "tag", object_id, tag_size, runner=runner
                )
                target_type, tag_metadata = parse_tag_object(tag_data)
                if target_type == "tag":
                    errors.append(
                        f"{display_name} uses an unsupported nested annotated tag"
                    )
                try:
                    commit = git_text(
                        repository,
                        "rev-parse",
                        "--verify",
                        f"{object_id}^{{commit}}",
                        runner=runner,
                    ).strip()
                except InventoryError:
                    commit = None
        elif object_type == "commit":
            commit = object_id
        else:
            commit = None
        if commit is None and len(errors) == ref_error_count:
            errors.append(f"{display_name} does not resolve to a commit")
        ref.update(
            {
                "commit": commit,
                "tag_metadata": tag_metadata,
                "tag_size": tag_size,
            }
        )
        hydrated.append(ref)
    return hydrated, errors


def collect_remote_refs(repository, remote_url, runner=None):
    output = git_limited(
        repository,
        MAX_REMOTE_ADVERTISEMENT_BYTES,
        "ls-remote",
        "--refs",
        remote_url,
        runner=runner,
    )
    if output and not output.endswith(b"\n"):
        raise InventoryError("remote ref advertisement is incomplete")
    refs = {}
    for line in output.splitlines():
        fields = line.split(b"\t", 1)
        if len(fields) != 2:
            raise InventoryError("malformed remote ref advertisement")
        object_id_bytes, ref_name_bytes = fields
        try:
            object_id = object_id_bytes.decode("ascii", "strict")
        except UnicodeDecodeError as error:
            raise InventoryError("malformed remote ref advertisement") from error
        if not OID_PATTERN.fullmatch(object_id) or not ref_name_bytes.startswith(b"refs/"):
            raise InventoryError("malformed remote ref advertisement")
        if len(ref_name_bytes) > MAX_REF_NAME_BYTES:
            raise InventoryError(f"remote ref name exceeds {MAX_REF_NAME_BYTES} bytes")
        ref_name = encode_ref(ref_name_bytes)
        if ref_name in refs:
            raise InventoryError(
                f"duplicate remote ref advertisement: {encode_ref_display(ref_name_bytes)}"
            )
        if len(refs) == MAX_REMOTE_REFS:
            raise InventoryError(f"remote ref count exceeds {MAX_REMOTE_REFS}")
        refs[ref_name] = object_id
    if not refs:
        raise InventoryError("the remote advertised no refs")
    return refs


def validate_remote_url(remote_url, allow_noncanonical_remote):
    if not remote_url or any(character.isspace() for character in remote_url):
        raise InventoryError("--remote-url must not be empty or contain whitespace")
    if "://" not in remote_url:
        if re.match(r"^[^/]+@[^:]+:", remote_url):
            raise InventoryError("--remote-url must not contain user information")
        if not Path(remote_url).is_absolute():
            raise InventoryError("local --remote-url values must be absolute paths")
        if not allow_noncanonical_remote:
            raise InventoryError(
                f"W-025 evidence requires the canonical remote {CANONICAL_REMOTE_URL}"
            )
        return
    try:
        parsed = urlsplit(remote_url)
    except ValueError as error:
        raise InventoryError(f"invalid --remote-url: {error}") from error
    if parsed.scheme != "https":
        raise InventoryError("network --remote-url values must use HTTPS")
    if parsed.username is not None or parsed.password is not None:
        raise InventoryError("--remote-url must not contain user information")
    if parsed.query or parsed.fragment:
        raise InventoryError("--remote-url must not contain a query or fragment")
    if not allow_noncanonical_remote and remote_url != CANONICAL_REMOTE_URL:
        raise InventoryError(
            f"W-025 evidence requires the canonical remote {CANONICAL_REMOTE_URL}"
        )


def validate_repository(repository, remote_url, runner=None):
    git_text(repository, "rev-parse", "--git-dir", runner=runner)
    version_output = git_text(repository, "--version", runner=runner).strip()
    version_match = re.search(r"\b(\d+)\.(\d+)(?:\.\d+)?\b", version_output)
    if version_match is None:
        raise InventoryError("git --version returned an unrecognized version")
    version = tuple(int(component) for component in version_match.groups())
    if version < (2, 45):
        raise InventoryError("history inventory requires Git 2.45 or newer")
    object_format = git_text(
        repository, "rev-parse", "--show-object-format", runner=runner
    ).strip()
    if object_format != "sha1":
        raise InventoryError("history inventory currently requires Git SHA-1 object IDs")
    if (
        git_text(
            repository, "rev-parse", "--is-shallow-repository", runner=runner
        ).strip()
        != "false"
    ):
        raise InventoryError("shallow repositories cannot establish complete history")
    object_directory = Path(
        git_text(
            repository, "rev-parse", "--git-path", "objects", runner=runner
        ).strip()
    )
    if not object_directory.is_absolute():
        object_directory = (repository / object_directory).resolve()
    if (object_directory / "info" / "alternates").exists():
        raise InventoryError("repository object alternates are not allowed")
    transport_overrides = git_limited(
        repository,
        MAX_GIT_TEXT_OUTPUT_BYTES,
        "config",
        "--local",
        "--null",
        "--name-only",
        "--list",
        runner=runner,
    )
    override_pattern = re.compile(
        rb"^(extensions\.worktreeconfig|include(if)?\..*|url\..*\.insteadof|http\..*|credential\..*|remote\..*\.proxy|core\.(askpass|gitproxy)|fsck\..*)$"
    )
    if any(
        override_pattern.fullmatch(name.lower())
        for name in transport_overrides.split(b"\0")
        if name
    ):
        raise InventoryError("repository-local transport overrides are not allowed")
    if any(
        name.lower().startswith(b"remote.")
        and not name.lower().startswith(b"remote.origin.")
        for name in transport_overrides.split(b"\0")
        if name
    ):
        raise InventoryError("repository-local alternate remotes are not allowed")
    try:
        origin = git_text(
            repository, "remote", "get-url", "origin", runner=runner
        ).strip()
    except InventoryError as error:
        raise InventoryError("failed to resolve repository origin") from error
    if origin != remote_url:
        raise InventoryError("the repository origin must exactly match --remote-url")
    return {"git_version": version_output, "object_format": object_format}


def collect_commits(repository, refs, runner=None):
    tips = sorted({ref["commit"] for ref in refs if ref["commit"] is not None})
    if not tips:
        raise InventoryError("the repository contains no refs that resolve to commits")
    output_limit = (MAX_REACHABLE_COMMITS + 1) * 41
    output = git_limited(
        repository,
        output_limit,
        "rev-list",
        "--stdin",
        runner=runner,
        input_data=("".join(f"{tip}\n" for tip in tips)).encode(),
    )
    if not output.endswith(b"\n"):
        raise InventoryError("reachable commit enumeration is incomplete")
    commits = output.decode("ascii").splitlines()
    if not commits or any(not OID_PATTERN.fullmatch(commit) for commit in commits):
        raise InventoryError("reachable commit enumeration failed")
    commits = sorted(set(commits))
    if len(commits) > MAX_REACHABLE_COMMITS:
        raise InventoryError(f"reachable commit count exceeds {MAX_REACHABLE_COMMITS}")
    return commits


def commit_metadata(repository, commit, runner=None):
    fields = git_text(
        repository,
        "show",
        "-s",
        "--format=%H%x00%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI%x00%s",
        commit,
        runner=runner,
    ).rstrip("\n").split("\0", 7)
    if len(fields) != 8 or fields[0] != commit:
        raise InventoryError(f"malformed metadata for commit {commit}")
    return {
        "commit": fields[0],
        "author_name": fields[1],
        "author_email": fields[2],
        "author_date": fields[3],
        "committer_name": fields[4],
        "committer_email": fields[5],
        "committer_date": fields[6],
        "subject": fields[7],
    }


def encode_path(path):
    redacted = redact_secret_bytes(path)
    encoded = quote_from_bytes(redacted, safe="/._-")
    if redacted != path:
        digest = hashlib.sha256(path).hexdigest()
        return f"{encoded} [path-sha256:{digest}]"
    return encoded


def path_record(path):
    return {
        "identity": path,
        "display": encode_path(path),
        "generated_candidate": any(
            path.lower().endswith(suffix.encode()) for suffix in GENERATED_SUFFIXES
        ),
        "review_document": Path(path.decode("utf-8", "surrogateescape"))
        .name.upper()
        .startswith(("LICENSE", "NOTICE", "COPYING", "PROVENANCE")),
    }


def encode_ref(ref_name):
    return quote_from_bytes(ref_name, safe="/._-")


def encode_ref_display(ref_name):
    redacted = redact_secret_bytes(ref_name)
    encoded = quote_from_bytes(redacted, safe="/._-")
    if redacted != ref_name:
        digest = hashlib.sha256(ref_name).hexdigest()
        return f"{encoded} [ref-sha256:{digest}]"
    return encoded


def display_ref_name(ref_name):
    return encode_ref_display(unquote_to_bytes(ref_name))


def tree_entries(repository, commit, runner=None):
    output = git_limited(
        repository,
        MAX_TREE_OUTPUT_BYTES,
        "ls-tree",
        "-r",
        "-z",
        "--full-tree",
        commit,
        runner=runner,
    )
    if output and not output.endswith(b"\0"):
        raise InventoryError("historical tree output is incomplete")
    entries = []
    path_bytes = 0
    for record in output.split(b"\0"):
        if not record:
            continue
        if b"\t" not in record:
            raise InventoryError("malformed historical tree record")
        metadata, path = record.split(b"\t", 1)
        if len(path) > MAX_PATH_BYTES:
            raise InventoryError(f"historical path exceeds {MAX_PATH_BYTES} bytes")
        path_bytes += len(path)
        try:
            metadata_fields = metadata.decode("ascii", "strict").split(" ")
        except UnicodeDecodeError as error:
            raise InventoryError("malformed historical tree metadata") from error
        if len(metadata_fields) != 3:
            raise InventoryError("malformed historical tree metadata")
        mode, object_type, object_id = metadata_fields
        if not OID_PATTERN.fullmatch(object_id):
            raise InventoryError("malformed historical tree object ID")
        entries.append(
            {
                "mode": mode,
                "type": object_type,
                "object_id": object_id,
                "path": path,
            }
        )
    return entries, path_bytes


def safe_text(value):
    return "".join(
        character
        if character == "\n" or not unicodedata.category(character).startswith("C")
        else f"\\u{ord(character):04x}"
        for character in str(value)
    )


def secret_candidates(data, object_id, object_kind, paths):
    matches = []
    truncated = False
    for name, pattern in SECRET_PATTERNS:
        for match in pattern.finditer(data):
            if len(matches) == MAX_SECRET_CANDIDATES_PER_OBJECT:
                truncated = True
                break
            matches.append((match.start(), name))
        if truncated:
            break
    sorted_paths = sorted(paths)
    candidate_paths = sorted_paths[:MAX_CANDIDATE_PATHS]
    omitted_path_count = len(sorted_paths) - len(candidate_paths)
    candidates = []
    line = 1
    previous_offset = 0
    for offset, name in sorted(matches):
        line += data.count(b"\n", previous_offset, offset)
        previous_offset = offset
        candidates.append(
            {
                "pattern": name,
                "object_id": object_id,
                "object_kind": object_kind,
                "paths": candidate_paths,
                "omitted_path_count": omitted_path_count,
                "line": line,
            }
        )
    return candidates, truncated


def redact_secret_text(value):
    data = str(value).encode("utf-8", "backslashreplace")
    for name, pattern in SECRET_PATTERNS:
        replacement = f"[REDACTED:{name}]".encode()
        data = pattern.sub(replacement, data)
    return data.decode("utf-8", "backslashreplace")


def redact_secret_bytes(data):
    redacted = data
    for name, pattern in SECRET_PATTERNS:
        redacted = pattern.sub(f"REDACTED_{name}".encode(), redacted)
    return redacted


def is_binary(data):
    if b"\0" in data[:8192]:
        return True
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return True
    return False


def collect_inventory(
    repository,
    final_commit,
    remote_url,
    max_scanned_object_bytes=DEFAULT_MAX_SCANNED_OBJECT_BYTES,
    large_blob_bytes=DEFAULT_LARGE_BLOB_BYTES,
    allow_noncanonical_remote=False,
    git_executable=None,
    git_executable_sha256=None,
    git_https_helper_sha256=None,
):
    repository = Path(repository).resolve()
    tool_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    if not OID_PATTERN.fullmatch(final_commit):
        raise InventoryError("--final-commit must be a full lowercase commit ID")
    validate_remote_url(remote_url, allow_noncanonical_remote)
    if git_executable is None:
        raise InventoryError("an absolute Git executable is required")
    if (
        remote_url == CANONICAL_REMOTE_URL
        and (git_executable_sha256, git_https_helper_sha256)
        not in CANONICAL_GIT_PROGRAMS
    ):
        raise InventoryError(
            "canonical W-025 evidence requires a source-reviewed Git program pair"
        )
    runner = GitRunner(
        git_executable, git_executable_sha256, git_https_helper_sha256
    )
    try:
        return collect_inventory_with_runner(
            repository,
            final_commit,
            remote_url,
            max_scanned_object_bytes,
            large_blob_bytes,
            runner,
            tool_sha256,
        )
    finally:
        runner.close()


def collect_inventory_with_runner(
    repository,
    final_commit,
    remote_url,
    max_scanned_object_bytes,
    large_blob_bytes,
    runner,
    tool_sha256,
):
    repository_identity = validate_repository(repository, remote_url, runner=runner)

    scan_budget = {"bytes": 0}
    refs, coverage_errors = collect_refs(repository, runner=runner)
    if not refs:
        raise InventoryError("the repository contains no commit refs")
    remote_refs = collect_remote_refs(repository, remote_url, runner=runner)
    local_ref_objects = {ref["name"]: ref["object_id"] for ref in refs}
    ref_displays = {ref["name"]: ref["display_name"] for ref in refs}
    for ref_name, object_id in sorted(remote_refs.items()):
        if ref_name not in local_ref_objects:
            coverage_errors.append(
                f"remote ref is missing locally: {display_ref_name(ref_name)}"
            )
        elif local_ref_objects[ref_name] != object_id:
            coverage_errors.append(f"remote ref differs locally: {ref_displays[ref_name]}")
    for ref_name in sorted(set(local_ref_objects) - set(remote_refs)):
        coverage_errors.append(
            f"local ref is not advertised remotely: {ref_displays[ref_name]}"
        )
    if coverage_errors:
        raise InventoryError("local refs do not exactly match the remote advertisement")
    validate_object_database(repository, runner=runner)
    refs, ref_errors = hydrate_refs(
        repository, refs, max_scanned_object_bytes, scan_budget, runner=runner
    )
    coverage_errors.extend(ref_errors)

    commits = collect_commits(repository, refs, runner=runner)
    if final_commit not in commits:
        raise InventoryError("--final-commit is not reachable from the inventoried refs")
    metadata = []
    commit_sizes = {}
    unscanned_metadata_objects = []
    for commit in commits:
        size = int(
            git_text(repository, "cat-file", "-s", commit, runner=runner).strip()
        )
        commit_sizes[commit] = size
        if size > max_scanned_object_bytes:
            coverage_errors.append(f"commit metadata exceeds the content scan cap: {commit}")
            unscanned_metadata_objects.append(
                {
                    "object_id": commit,
                    "object_kind": "commit",
                    "paths": [f"commit:{commit}"],
                    "size": size,
                }
            )
            metadata.append(
                {
                    "commit": commit,
                    "author_name": "(not scanned)",
                    "author_email": "(not scanned)",
                    "author_date": "(not scanned)",
                    "committer_name": "(not scanned)",
                    "committer_email": "(not scanned)",
                    "committer_date": "(not scanned)",
                    "subject": "(not scanned)",
                }
            )
        else:
            scan_budget["bytes"] += size
            if scan_budget["bytes"] > MAX_TOTAL_SCANNED_OBJECT_BYTES:
                raise InventoryError(
                    "aggregate scanned object bytes exceed "
                    f"{MAX_TOTAL_SCANNED_OBJECT_BYTES}"
                )
            metadata.append(commit_metadata(repository, commit, runner=runner))

    blob_paths = {}
    submodules = set()
    final_paths = set()
    historical_tree_entries = 0
    historical_path_bytes = 0
    for commit in commits:
        entries, path_bytes = tree_entries(repository, commit, runner=runner)
        historical_tree_entries += len(entries)
        historical_path_bytes += path_bytes
        if historical_tree_entries > MAX_HISTORICAL_TREE_ENTRIES:
            raise InventoryError(
                f"historical tree entry count exceeds {MAX_HISTORICAL_TREE_ENTRIES}"
            )
        if historical_path_bytes > MAX_HISTORICAL_PATH_BYTES:
            raise InventoryError(
                f"historical path bytes exceed {MAX_HISTORICAL_PATH_BYTES}"
            )
        for entry in entries:
            if commit == final_commit:
                final_paths.add(entry["path"])
            if entry["type"] == "blob":
                blob_paths.setdefault(entry["object_id"], set()).add(entry["path"])
            elif entry["type"] == "commit":
                submodules.add((entry["path"], entry["object_id"]))

    blobs = []
    candidates = []
    truncated_secret_scans = []
    total_scanned_object_bytes = scan_budget["bytes"]
    if len(blob_paths) > MAX_UNIQUE_BLOBS:
        raise InventoryError(f"unique blob count exceeds {MAX_UNIQUE_BLOBS}")
    for object_id, paths in sorted(blob_paths.items()):
        path_records = [path_record(path) for path in sorted(paths)]
        size = int(
            git_text(repository, "cat-file", "-s", object_id, runner=runner).strip()
        )
        scanned = size <= max_scanned_object_bytes
        binary = None
        if scanned:
            total_scanned_object_bytes += size
            if total_scanned_object_bytes > MAX_TOTAL_SCANNED_OBJECT_BYTES:
                raise InventoryError(
                    "aggregate scanned object bytes exceed "
                    f"{MAX_TOTAL_SCANNED_OBJECT_BYTES}"
                )
            data = read_git_object(
                repository, "blob", object_id, size, runner=runner
            )
            binary = is_binary(data)
            display_paths = [record["display"] for record in path_records]
            found, truncated = secret_candidates(
                data, object_id, "blob", display_paths
            )
            candidates.extend(found)
            if len(candidates) > MAX_TOTAL_SECRET_CANDIDATES:
                raise InventoryError(
                    f"secret candidate count exceeds {MAX_TOTAL_SECRET_CANDIDATES}"
                )
            if truncated:
                truncated_secret_scans.append(
                    {
                        "object_id": object_id,
                        "object_kind": "blob",
                        "paths": display_paths,
                    }
                )
        blobs.append(
            {
                "object_id": object_id,
                "paths": [record["display"] for record in path_records],
                "path_records": path_records,
                "size": size,
                "scanned": scanned,
                "binary": binary,
                "large": size >= large_blob_bytes,
                "generated_candidate": any(
                    record["generated_candidate"] for record in path_records
                ),
            }
        )

    for commit in commits:
        size = commit_sizes[commit]
        paths = [f"commit:{commit}"]
        if size > max_scanned_object_bytes:
            continue
        data = read_git_object(repository, "commit", commit, size, runner=runner)
        found, truncated = secret_candidates(data, commit, "commit", paths)
        candidates.extend(found)
        if len(candidates) > MAX_TOTAL_SECRET_CANDIDATES:
            raise InventoryError(
                f"secret candidate count exceeds {MAX_TOTAL_SECRET_CANDIDATES}"
            )
        if truncated:
            truncated_secret_scans.append(
                {"object_id": commit, "object_kind": "commit", "paths": paths}
            )

    tag_paths = {}
    for ref in refs:
        if ref["object_type"] == "tag":
            tag_paths.setdefault(ref["object_id"], []).append(ref["display_name"])
    for object_id, paths in sorted(tag_paths.items()):
        size = next(
            ref["tag_size"]
            for ref in refs
            if ref["object_type"] == "tag" and ref["object_id"] == object_id
        )
        if size > max_scanned_object_bytes:
            unscanned_metadata_objects.append(
                {"object_id": object_id, "object_kind": "tag", "paths": paths, "size": size}
            )
            continue
        data = read_git_object(repository, "tag", object_id, size, runner=runner)
        found, truncated = secret_candidates(data, object_id, "tag", paths)
        candidates.extend(found)
        if len(candidates) > MAX_TOTAL_SECRET_CANDIDATES:
            raise InventoryError(
                f"secret candidate count exceeds {MAX_TOTAL_SECRET_CANDIDATES}"
            )
        if truncated:
            truncated_secret_scans.append(
                {"object_id": object_id, "object_kind": "tag", "paths": paths}
            )

    final_local_refs, _ = collect_refs(repository, runner=runner)
    final_remote_refs = collect_remote_refs(repository, remote_url, runner=runner)
    if {
        ref["name"]: ref["object_id"] for ref in final_local_refs
    } != local_ref_objects or final_remote_refs != remote_refs:
        raise InventoryError("local or remote refs changed during inventory")

    remote_manifest = "".join(
        f"{ref_name}\t{object_id}\n" for ref_name, object_id in sorted(remote_refs.items())
    )
    machine_scan_complete = (
        all(blob["scanned"] for blob in blobs)
        and not unscanned_metadata_objects
        and not truncated_secret_scans
    )
    inventory = {
        "tool_version": TOOL_VERSION,
        "tool_sha256": tool_sha256,
        "git_version": repository_identity["git_version"],
        "git_executable": runner.executable,
        "git_executable_sha256": runner.executable_sha256,
        "git_exec_path": runner.exec_path,
        "git_https_helper": runner.https_helper,
        "git_https_helper_sha256": runner.https_helper_sha256,
        "object_format": repository_identity["object_format"],
        "repository": str(repository),
        "remote_url": remote_url,
        "canonical_remote_verified": (
            remote_url == CANONICAL_REMOTE_URL and not coverage_errors
        ),
        "remote_manifest_sha256": hashlib.sha256(remote_manifest.encode()).hexdigest(),
        "remote_refs": remote_refs,
        "refs": refs,
        "coverage_errors": sorted(set(coverage_errors)),
        "machine_scan_complete": machine_scan_complete,
        "historical_tree_entries": historical_tree_entries,
        "historical_path_bytes": historical_path_bytes,
        "total_scanned_object_bytes": total_scanned_object_bytes,
        "commits": metadata,
        "final_commit": final_commit,
        "blobs": blobs,
        "submodules": [
            {
                "path": encode_path(path),
                "path_sha256": hashlib.sha256(path).hexdigest(),
                "object_id": object_id,
            }
            for path, object_id in sorted(submodules)
        ],
        "secret_candidates": sorted(
            candidates,
            key=lambda item: (
                item["object_kind"],
                item["object_id"],
                item["pattern"],
                item["line"],
            ),
        ),
        "truncated_secret_scans": sorted(
            truncated_secret_scans,
            key=lambda item: (item["object_kind"], item["object_id"]),
        ),
        "unscanned_metadata_objects": sorted(
            unscanned_metadata_objects,
            key=lambda item: (item["object_kind"], item["object_id"]),
        ),
        "max_scanned_object_bytes": max_scanned_object_bytes,
        "large_blob_bytes": large_blob_bytes,
        "required_license_files": {
            path: path.encode() in final_paths
            for path in ("LICENSE-APACHE", "LICENSE-MIT")
        },
    }
    return inventory


def markdown(value):
    value = html.escape(safe_text(redact_secret_text(value)), quote=False)
    return (
        value.replace("\\", "\\\\")
        .replace("://", ":\\/\\/")
        .replace("www.", "www\\.")
        .replace("@", "&#64;")
        .replace("*", "\\*")
        .replace("_", "\\_")
        .replace("~", "\\~")
        .replace("|", "\\|")
        .replace("!", "\\!")
        .replace("[", "\\[")
        .replace("]", "\\]")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("`", "\\`")
        .replace("\n", " ")
    )


def inline_code(value):
    value = safe_text(redact_secret_text(value)).replace("\n", " ")
    longest_run = max((len(run) for run in re.findall(r"`+", value)), default=0)
    delimiter = "`" * (longest_run + 1)
    padding = (
        " "
        if value.startswith("`")
        or value.endswith("`")
        or (value.startswith(" ") and value.endswith(" "))
        else ""
    )
    return f"{delimiter}{padding}{value}{padding}{delimiter}"


def table(lines, headers, rows):
    lines.append("| " + " | ".join(headers) + " |")
    lines.append("|" + "|".join("---" for _ in headers) + "|")
    for row in rows:
        lines.append("| " + " | ".join(markdown(value) for value in row) + " |")
    lines.append("")


def binary_label(blobs):
    if any(blob["binary"] is True for blob in blobs):
        return "yes"
    if any(not blob["scanned"] for blob in blobs):
        return "not scanned"
    return "no"


def render_report(inventory, invocation):
    refs = inventory["refs"]
    commits = inventory["commits"]
    blobs = inventory["blobs"]
    remote_refs = inventory["remote_refs"]
    coverage_errors = inventory["coverage_errors"]
    secret_candidates_found = inventory["secret_candidates"]
    authors = {}
    committers = {}
    for commit in commits:
        authors.setdefault((commit["author_name"], commit["author_email"]), []).append(
            commit["commit"]
        )
        committers.setdefault(
            (commit["committer_name"], commit["committer_email"]), []
        ).append(commit["commit"])

    path_versions = {}
    for blob in blobs:
        for record in blob["path_records"]:
            path_versions.setdefault(
                record["identity"], {"record": record, "versions": []}
            )["versions"].append(blob)

    lines = [
        "# W-025 Reachable-History Inventory",
        "",
        "Status: INCOMPLETE. This report is a machine inventory, not W-025 clearance.",
        "Dedicated secret scanning, rights/provenance review, PII review, remediation,",
        "and owner approval remain required.",
        "",
        "## Scope",
        "",
        f"- Final commit: {inline_code(inventory['final_commit'])}",
        f"- Remote: {inline_code(inventory['remote_url'])}",
        f"- Canonical W-025 remote verified: {'yes' if inventory['canonical_remote_verified'] else 'no'}",
        f"- Remote manifest SHA-256: {inline_code(inventory['remote_manifest_sha256'])}",
        f"- Git object format: {inline_code(inventory['object_format'])}",
        f"- Platform: {inline_code(sys.platform)} (POSIX required)",
        f"- Remote refs advertised: {len(remote_refs)}",
        f"- Local refs inventoried: {len(refs)}",
        f"- Unique reachable commits: {len(commits)}",
        f"- Unique reachable blobs: {len(blobs)}",
        f"- Unique reachable blob bytes: {sum(blob['size'] for blob in blobs)}",
        f"- Historical tree entries visited: {inventory['historical_tree_entries']}",
        f"- Historical path bytes visited: {inventory['historical_path_bytes']}",
        f"- Total scanned Git object bytes: {inventory['total_scanned_object_bytes']}",
        f"- Content scan cap: {inventory['max_scanned_object_bytes']} bytes per Git object",
        f"- Large-blob review threshold: {inventory['large_blob_bytes']} bytes",
        f"- Machine content scan complete: {'yes' if inventory['machine_scan_complete'] else 'no'}",
        "- Git replace objects and lazy object fetching were disabled.",
        "",
        "## Coverage",
        "",
    ]
    if coverage_errors:
        lines.extend(f"- ERROR: {markdown(error)}" for error in coverage_errors)
    else:
        lines.append("All advertised remote refs were present locally at the advertised object IDs.")
    lines.append("")

    table(
        lines,
        ["Ref", "Type", "Object", "Peeled commit"],
        [
            (
                ref["display_name"],
                ref["object_type"],
                ref["object_id"],
                ref["commit"] or "-",
            )
            for ref in refs
        ],
    )

    annotated_tags = [ref for ref in refs if ref["tag_metadata"] is not None]
    lines.extend(["### Annotated Tag Metadata", ""])
    if annotated_tags:
        table(
            lines,
            ["Ref", "Tagger", "Tagger email", "Tagger date", "Subject"],
            [
                (
                    ref["display_name"],
                    ref["tag_metadata"]["tagger_name"],
                    ref["tag_metadata"]["tagger_email"],
                    ref["tag_metadata"]["tagger_date"],
                    ref["tag_metadata"]["subject"],
                )
                for ref in annotated_tags
            ],
        )
    else:
        lines.extend(["None.", ""])

    lines.extend(["## Commit And Author Metadata", ""])
    table(
        lines,
        ["Commit", "Author", "Author email", "Author date", "Subject"],
        [
            (
                commit["commit"],
                commit["author_name"],
                commit["author_email"],
                commit["author_date"],
                commit["subject"],
            )
            for commit in commits
        ],
    )
    table(
        lines,
        ["Role", "Name", "Email", "Commit count"],
        [
            ("author", name, email, len(commit_ids))
            for (name, email), commit_ids in sorted(authors.items())
        ]
        + [
            ("committer", name, email, len(commit_ids))
            for (name, email), commit_ids in sorted(committers.items())
        ],
    )

    lines.extend(
        [
            "## Historical Path Inventory",
            "",
            "Paths are percent-encoded from raw Git bytes. Secret-pattern matches are",
            "replaced and paired with a SHA-256 of the original path bytes.",
            "",
        ]
    )
    table(
        lines,
        [
            "Path (percent-encoded bytes)",
            "Blob objects",
            "Largest bytes",
            "Binary",
            "Generated candidate",
        ],
        [
            (
                item["record"]["display"],
                ", ".join(
                    sorted(blob["object_id"] for blob in item["versions"])
                ),
                max(blob["size"] for blob in item["versions"]),
                binary_label(item["versions"]),
                "yes"
                if item["record"]["generated_candidate"]
                else "no",
            )
            for _, item in sorted(
                path_versions.items(), key=lambda pair: pair[1]["record"]["display"]
            )
        ],
    )

    lines.extend(["## Machine Findings", ""])
    lines.append("### Secret-pattern candidates")
    lines.append("")
    lines.append(
        "Matches are redacted and heuristic. This scan is not a replacement for a dedicated secret scanner."
    )
    lines.append("")
    if secret_candidates_found:
        table(
            lines,
            ["Pattern", "Object kind", "Object", "Line", "Paths", "Omitted paths"],
            [
                (
                    candidate["pattern"],
                    candidate["object_kind"],
                    candidate["object_id"],
                    candidate["line"],
                    ", ".join(candidate["paths"]),
                    candidate["omitted_path_count"],
                )
                for candidate in secret_candidates_found
            ],
        )
    else:
        lines.extend(["No heuristic secret-pattern candidates found in scanned objects.", ""])

    finding_groups = [
        ("Large blobs", [blob for blob in blobs if blob["large"]]),
        ("Binary blobs", [blob for blob in blobs if blob["binary"]]),
        (
            "Generated-artifact candidates",
            [blob for blob in blobs if blob["generated_candidate"]],
        ),
        ("Blobs above the content scan cap", [blob for blob in blobs if not blob["scanned"]]),
    ]
    for heading, findings in finding_groups:
        lines.extend([f"### {heading}", ""])
        if findings:
            table(
                lines,
                ["Object", "Bytes", "Paths"],
                [
                    (blob["object_id"], blob["size"], ", ".join(blob["paths"]))
                    for blob in findings
                ],
            )
        else:
            lines.extend(["None.", ""])

    lines.extend(["### Metadata objects above the content scan cap", ""])
    if inventory["unscanned_metadata_objects"]:
        table(
            lines,
            ["Kind", "Object", "Bytes", "Refs"],
            [
                (
                    item["object_kind"],
                    item["object_id"],
                    item["size"],
                    ", ".join(item["paths"]),
                )
                for item in inventory["unscanned_metadata_objects"]
            ],
        )
    else:
        lines.extend(["None.", ""])

    lines.extend(["### Truncated secret-candidate scans", ""])
    if inventory["truncated_secret_scans"]:
        table(
            lines,
            ["Kind", "Object", "Refs or paths"],
            [
                (item["object_kind"], item["object_id"], ", ".join(item["paths"]))
                for item in inventory["truncated_secret_scans"]
            ],
        )
    else:
        lines.extend(["None.", ""])

    lines.extend(["### License, notice, and provenance paths", ""])
    review_paths = [
        item["record"]["display"]
        for item in path_versions.values()
        if item["record"]["review_document"]
    ]
    if review_paths:
        table(lines, ["Path (percent-encoded bytes)"], [(path,) for path in sorted(review_paths)])
    else:
        lines.append("None.")
    lines.extend(
        [
            "",
            "Final-tree required license files:",
            "",
            *[
                f"- [{'x' if present else ' '}] `{path}`"
                for path, present in inventory["required_license_files"].items()
            ],
            "",
            "### Submodules",
            "",
        ]
    )
    if inventory["submodules"]:
        lines.extend(
            f"- {inline_code(entry['path'])} at {inline_code(entry['object_id'])}"
            for entry in inventory["submodules"]
        )
    else:
        lines.append("None.")

    lines.extend(
        [
            "",
            "## Required Manual Review",
            "",
            "- [ ] Run dedicated secret scanners over the exact ref and commit set above; record versions and commands.",
            "- [ ] Review every historical path and finding for personal or partner data.",
            "- [ ] Review every historical path for proprietary or upstream-derived content and provenance.",
            "- [ ] Review generated-artifact, binary, and large-blob findings for distribution suitability.",
            "- [ ] Verify license texts, notices, dependency obligations, and fixture provenance.",
            "- [ ] Record each confirmed finding and its remediation.",
            "- [ ] Decide whether dangling or server-retained unreachable objects require separate review.",
            "- [ ] Obtain owner approval for the final W-025 disposition.",
            "",
            "## Review Record",
            "",
            "- Dedicated scanner evidence: PENDING",
            "- PII and partner-data review: PENDING",
            "- Rights and provenance review: PENDING",
            "- License and notice review: PENDING",
            "- Findings and remediation: PENDING",
            "- Owner approval: PENDING",
            "",
            "## Tooling And Reproduction",
            "",
            f"- Inventory tool version: {inventory['tool_version']}",
            f"- Inventory tool SHA-256: {inline_code(inventory['tool_sha256'])}",
            f"- Git: {inline_code(inventory['git_version'])}",
            f"- Git executable: {inline_code(inventory['git_executable'])}",
            f"- Git executable SHA-256: {inline_code(inventory['git_executable_sha256'])}",
            f"- Git helper directory: {inline_code(inventory['git_exec_path'])}",
            f"- Git HTTPS helper: {inline_code(inventory['git_https_helper'])}",
            f"- Git HTTPS helper SHA-256: {inline_code(inventory['git_https_helper_sha256'])}",
            f"- Python: {inline_code(platform.python_version())}",
            f"- Invocation: {inline_code(invocation)}",
            "- Mirror acquisition: `git clone --mirror <remote> <isolated-mirror>`",
            "- Mirror refresh: `git -C <isolated-mirror> fetch --prune origin '+refs/*:refs/*'`",
            "- Ref query: `git ls-remote --refs <remote>`",
            "- Local refs: `git for-each-ref`",
            "- Commit walk: `git rev-list <all peeled ref tips>`",
            "- Historical trees: `git ls-tree -r -z --full-tree <each reachable commit>`",
            "- Object reads: `git cat-file`",
            "",
        ]
    )
    return "\n".join(lines)


def write_report(path, contents):
    path = Path(path)
    missing_directories = []
    parent = path.parent
    while not parent.exists():
        missing_directories.append(parent)
        parent = parent.parent
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
        temporary_path = None
        directories_to_sync = [path.parent]
        directories_to_sync.extend(
            directory.parent for directory in missing_directories
        )
        for directory in dict.fromkeys(directories_to_sync):
            directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
            directory_fd = os.open(directory, directory_flags)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def parse_args(arguments):
    parser = argparse.ArgumentParser(
        description="Inventory commit histories reachable from local refs and compare remote refs."
    )
    parser.add_argument("--repository", default=".")
    parser.add_argument("--git-executable", required=True)
    parser.add_argument("--git-executable-sha256", required=True)
    parser.add_argument("--git-https-helper-sha256", required=True)
    parser.add_argument("--remote-url", required=True)
    parser.add_argument("--final-commit", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument(
        "--max-scanned-object-bytes",
        type=int,
        default=DEFAULT_MAX_SCANNED_OBJECT_BYTES,
    )
    parser.add_argument(
        "--large-blob-bytes", type=int, default=DEFAULT_LARGE_BLOB_BYTES
    )
    parsed = parser.parse_args(arguments)
    if parsed.max_scanned_object_bytes <= 0 or parsed.large_blob_bytes <= 0:
        parser.error("blob byte limits must be positive")
    return parsed


def main(arguments=None):
    arguments = sys.argv[1:] if arguments is None else arguments
    parsed = parse_args(arguments)
    try:
        inventory = collect_inventory(
            parsed.repository,
            parsed.final_commit,
            parsed.remote_url,
            parsed.max_scanned_object_bytes,
            parsed.large_blob_bytes,
            git_executable=parsed.git_executable,
            git_executable_sha256=parsed.git_executable_sha256,
            git_https_helper_sha256=parsed.git_https_helper_sha256,
        )
        invocation = shlex.join(
            [
                "python3",
                "ci/inventory-history.py",
                "--repository",
                "<isolated-mirror>",
                "--git-executable",
                parsed.git_executable,
                "--git-executable-sha256",
                parsed.git_executable_sha256,
                "--git-https-helper-sha256",
                parsed.git_https_helper_sha256,
                "--remote-url",
                parsed.remote_url,
                "--final-commit",
                parsed.final_commit,
                "--report",
                "<ignored-report-path>",
                "--max-scanned-object-bytes",
                str(parsed.max_scanned_object_bytes),
                "--large-blob-bytes",
                str(parsed.large_blob_bytes),
            ]
        )
        report = Path(parsed.report)
        write_report(report, render_report(inventory, invocation))
    except (InventoryError, OSError, UnicodeError, subprocess.SubprocessError) as error:
        print(f"history inventory error: {error}", file=sys.stderr)
        return 2
    if inventory["coverage_errors"]:
        print("history inventory error: history coverage is incomplete", file=sys.stderr)
        return 1
    if not inventory["machine_scan_complete"]:
        print("history inventory error: machine content scan is incomplete", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
