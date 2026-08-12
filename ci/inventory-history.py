import argparse
import hashlib
import html
import os
import platform
import re
import shlex
import subprocess
import sys
from pathlib import Path
from urllib.parse import quote_from_bytes, urlsplit


TOOL_VERSION = 1
CANONICAL_REMOTE_URL = "https://github.com/jscott3201/cxf-json.git"
DEFAULT_LARGE_BLOB_BYTES = 1_048_576
DEFAULT_MAX_SCANNED_OBJECT_BYTES = 4_194_304
MAX_SECRET_CANDIDATES_PER_OBJECT = 1_000
GIT_TIMEOUT_SECONDS = 120
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


def git(repository, *arguments, input_data=None, text=True, check=True):
    environment = {
        name: value
        for name, value in os.environ.items()
        if not name.startswith("GIT_") and name not in {"GCM_INTERACTIVE"}
    }
    environment.update(
        {
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_GRAFT_FILE": os.devnull,
            "GIT_NO_LAZY_FETCH": "1",
            "GIT_NO_REPLACE_OBJECTS": "1",
            "GIT_TERMINAL_PROMPT": "0",
            "GCM_INTERACTIVE": "never",
            "LC_ALL": "C",
        }
    )
    options = {
        "check": check,
        "capture_output": True,
        "env": environment,
        "input": input_data,
        "timeout": GIT_TIMEOUT_SECONDS,
    }
    if text:
        options.update({"encoding": "utf-8", "errors": "backslashreplace"})
    return subprocess.run(
        ["git", "--no-replace-objects", "-C", str(repository), *arguments],
        **options,
    )


def git_text(repository, *arguments):
    try:
        return git(repository, *arguments).stdout
    except subprocess.CalledProcessError as error:
        message = error.stderr.strip() or "Git command failed"
        raise InventoryError(message) from error


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


def collect_refs(repository, max_scanned_object_bytes):
    output = git_text(
        repository,
        "for-each-ref",
        "--format=%(refname)%00%(objecttype)%00%(objectname)",
    )
    refs = []
    errors = []
    for line in output.splitlines():
        fields = line.split("\0", 2)
        if len(fields) != 3:
            raise InventoryError("malformed local ref inventory")
        ref_name, object_type, object_id = fields
        tag_metadata = None
        tag_size = None
        if object_type == "tag":
            tag_size = int(git_text(repository, "cat-file", "-s", object_id).strip())
            if tag_size > max_scanned_object_bytes:
                errors.append(f"{ref_name} tag metadata exceeds the content scan cap")
                commit = None
                tag_metadata = {
                    "tagger_name": "(not scanned)",
                    "tagger_email": "(not scanned)",
                    "tagger_date": "(not scanned)",
                    "subject": "(not scanned)",
                }
            else:
                tag_data = git(repository, "cat-file", "tag", object_id, text=False).stdout
                target_type, tag_metadata = parse_tag_object(tag_data)
                if target_type == "tag":
                    errors.append(f"{ref_name} uses an unsupported nested annotated tag")
                resolved = git(
                    repository,
                    "rev-parse",
                    "--verify",
                    f"{object_id}^{{commit}}",
                    check=False,
                )
                commit = resolved.stdout.strip() if resolved.returncode == 0 else None
        elif object_type == "commit":
            commit = object_id
        else:
            commit = None
        if commit is None and not any(error.startswith(ref_name) for error in errors):
            errors.append(f"{ref_name} does not resolve to a commit")
        refs.append(
            {
                "name": ref_name,
                "object_type": object_type,
                "object_id": object_id,
                "commit": commit,
                "tag_metadata": tag_metadata,
                "tag_size": tag_size,
            }
        )
    return sorted(refs, key=lambda ref: ref["name"]), errors


def collect_remote_refs(repository, remote_url):
    output = git_text(repository, "ls-remote", "--refs", remote_url)
    refs = {}
    for line in output.splitlines():
        object_id, ref_name = line.split("\t", 1)
        if not OID_PATTERN.fullmatch(object_id) or not ref_name.startswith("refs/"):
            raise InventoryError(f"malformed remote ref advertisement: {line}")
        if ref_name in refs:
            raise InventoryError(f"duplicate remote ref advertisement: {ref_name}")
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


def validate_repository(repository, remote_url):
    git_text(repository, "rev-parse", "--git-dir")
    version_output = git_text(repository, "--version").strip()
    version_match = re.search(r"\b(\d+)\.(\d+)(?:\.\d+)?\b", version_output)
    if version_match is None:
        raise InventoryError("git --version returned an unrecognized version")
    version = tuple(int(component) for component in version_match.groups())
    if version < (2, 45):
        raise InventoryError("history inventory requires Git 2.45 or newer")
    object_format = git_text(repository, "rev-parse", "--show-object-format").strip()
    if object_format != "sha1":
        raise InventoryError("history inventory currently requires Git SHA-1 object IDs")
    if git_text(repository, "rev-parse", "--is-shallow-repository").strip() != "false":
        raise InventoryError("shallow repositories cannot establish complete history")
    origin = git(repository, "remote", "get-url", "origin", check=False)
    if origin.returncode != 0 or origin.stdout.strip() != remote_url:
        raise InventoryError("the repository origin must exactly match --remote-url")
    transport_overrides = git(
        repository,
        "config",
        "--local",
        "--get-regexp",
        r"^(url\..*\.insteadof|http\..*|credential\..*|remote\..*\.proxy|core\.gitproxy)$",
        check=False,
    )
    if transport_overrides.returncode not in (0, 1):
        raise InventoryError("failed to inspect repository-local transport overrides")
    if transport_overrides.stdout.strip():
        raise InventoryError("repository-local transport overrides are not allowed")
    return {"git_version": version_output, "object_format": object_format}


def collect_commits(repository, refs):
    tips = sorted({ref["commit"] for ref in refs if ref["commit"] is not None})
    if not tips:
        raise InventoryError("the repository contains no refs that resolve to commits")
    commits = git_text(repository, "rev-list", *tips).splitlines()
    if not commits or any(not OID_PATTERN.fullmatch(commit) for commit in commits):
        raise InventoryError("reachable commit enumeration failed")
    return sorted(set(commits))


def commit_metadata(repository, commit):
    fields = git_text(
        repository,
        "show",
        "-s",
        "--format=%H%x00%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI%x00%s",
        commit,
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


def tree_entries(repository, commit):
    output = git(
        repository,
        "ls-tree",
        "-r",
        "-z",
        "--full-tree",
        commit,
        text=False,
    ).stdout
    entries = []
    for record in output.split(b"\0"):
        if not record:
            continue
        metadata, path = record.split(b"\t", 1)
        mode, object_type, object_id = metadata.decode("ascii").split(" ")
        entries.append(
            {
                "mode": mode,
                "type": object_type,
                "object_id": object_id,
                "path": encode_path(path),
            }
        )
    return entries


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
                "paths": sorted(paths),
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


def is_generated_path(path):
    lowered = path.lower()
    return any(lowered.endswith(suffix) for suffix in GENERATED_SUFFIXES)


def is_review_document(path):
    name = Path(path).name.upper()
    return name.startswith(("LICENSE", "NOTICE", "COPYING", "PROVENANCE"))


def collect_inventory(
    repository,
    final_commit,
    remote_url,
    max_scanned_object_bytes=DEFAULT_MAX_SCANNED_OBJECT_BYTES,
    large_blob_bytes=DEFAULT_LARGE_BLOB_BYTES,
    allow_noncanonical_remote=False,
):
    repository = Path(repository).resolve()
    if not OID_PATTERN.fullmatch(final_commit):
        raise InventoryError("--final-commit must be a full lowercase commit ID")
    validate_remote_url(remote_url, allow_noncanonical_remote)
    repository_identity = validate_repository(repository, remote_url)

    refs, coverage_errors = collect_refs(repository, max_scanned_object_bytes)
    if not refs:
        raise InventoryError("the repository contains no commit refs")
    remote_refs = collect_remote_refs(repository, remote_url)
    local_ref_objects = {ref["name"]: ref["object_id"] for ref in refs}
    for ref_name, object_id in sorted(remote_refs.items()):
        if ref_name not in local_ref_objects:
            coverage_errors.append(f"remote ref is missing locally: {ref_name}")
        elif local_ref_objects[ref_name] != object_id:
            coverage_errors.append(f"remote ref differs locally: {ref_name}")

    commits = collect_commits(repository, refs)
    if final_commit not in commits:
        raise InventoryError("--final-commit is not reachable from the inventoried refs")
    metadata = []
    commit_sizes = {}
    unscanned_metadata_objects = []
    for commit in commits:
        size = int(git_text(repository, "cat-file", "-s", commit).strip())
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
            metadata.append(commit_metadata(repository, commit))

    blob_paths = {}
    submodules = set()
    final_paths = set()
    for commit in commits:
        for entry in tree_entries(repository, commit):
            if commit == final_commit:
                final_paths.add(entry["path"])
            if entry["type"] == "blob":
                blob_paths.setdefault(entry["object_id"], set()).add(entry["path"])
            elif entry["type"] == "commit":
                submodules.add((entry["path"], entry["object_id"]))

    blobs = []
    candidates = []
    truncated_secret_scans = []
    for object_id, paths in sorted(blob_paths.items()):
        size = int(git_text(repository, "cat-file", "-s", object_id).strip())
        scanned = size <= max_scanned_object_bytes
        binary = None
        if scanned:
            data = git(repository, "cat-file", "blob", object_id, text=False).stdout
            if len(data) != size:
                raise InventoryError(f"short read for blob {object_id}")
            binary = is_binary(data)
            found, truncated = secret_candidates(data, object_id, "blob", paths)
            candidates.extend(found)
            if truncated:
                truncated_secret_scans.append(
                    {"object_id": object_id, "object_kind": "blob", "paths": sorted(paths)}
                )
        blobs.append(
            {
                "object_id": object_id,
                "paths": sorted(paths),
                "size": size,
                "scanned": scanned,
                "binary": binary,
                "large": size >= large_blob_bytes,
                "generated_candidate": any(is_generated_path(path) for path in paths),
            }
        )

    for commit in commits:
        size = commit_sizes[commit]
        paths = [f"commit:{commit}"]
        if size > max_scanned_object_bytes:
            continue
        data = git(repository, "cat-file", "commit", commit, text=False).stdout
        found, truncated = secret_candidates(data, commit, "commit", paths)
        candidates.extend(found)
        if truncated:
            truncated_secret_scans.append(
                {"object_id": commit, "object_kind": "commit", "paths": paths}
            )

    tag_paths = {}
    for ref in refs:
        if ref["object_type"] == "tag":
            tag_paths.setdefault(ref["object_id"], []).append(ref["name"])
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
        data = git(repository, "cat-file", "tag", object_id, text=False).stdout
        found, truncated = secret_candidates(data, object_id, "tag", paths)
        candidates.extend(found)
        if truncated:
            truncated_secret_scans.append(
                {"object_id": object_id, "object_kind": "tag", "paths": paths}
            )

    remote_manifest = "".join(
        f"{ref_name}\t{object_id}\n" for ref_name, object_id in sorted(remote_refs.items())
    )
    machine_scan_complete = (
        all(blob["scanned"] for blob in blobs)
        and not unscanned_metadata_objects
        and not truncated_secret_scans
    )
    return {
        "tool_version": TOOL_VERSION,
        "git_version": repository_identity["git_version"],
        "object_format": repository_identity["object_format"],
        "repository": str(repository),
        "remote_url": remote_url,
        "canonical_remote_verified": remote_url == CANONICAL_REMOTE_URL,
        "remote_manifest_sha256": hashlib.sha256(remote_manifest.encode()).hexdigest(),
        "remote_refs": remote_refs,
        "refs": refs,
        "coverage_errors": sorted(set(coverage_errors)),
        "machine_scan_complete": machine_scan_complete,
        "commits": metadata,
        "final_commit": final_commit,
        "blobs": blobs,
        "submodules": [
            {"path": path, "object_id": object_id}
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
            path: path in final_paths for path in ("LICENSE-APACHE", "LICENSE-MIT")
        },
    }


def markdown(value):
    value = html.escape(redact_secret_text(value), quote=False)
    return value.replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def inline_code(value):
    return f"`{markdown(value).replace('`', '&#96;')}`"


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
        for path in blob["paths"]:
            path_versions.setdefault(path, []).append(blob)

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
        f"- Remote refs advertised: {len(remote_refs)}",
        f"- Local refs inventoried: {len(refs)}",
        f"- Unique reachable commits: {len(commits)}",
        f"- Unique reachable blobs: {len(blobs)}",
        f"- Unique reachable blob bytes: {sum(blob['size'] for blob in blobs)}",
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
                ref["name"],
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
                    ref["name"],
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
            "Blob versions",
            "Largest bytes",
            "Binary",
            "Generated candidate",
        ],
        [
            (
                path,
                len(versions),
                max(blob["size"] for blob in versions),
                binary_label(versions),
                "yes" if any(blob["generated_candidate"] for blob in versions) else "no",
            )
            for path, versions in sorted(path_versions.items())
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
            ["Pattern", "Object kind", "Object", "Line", "Paths"],
            [
                (
                    candidate["pattern"],
                    candidate["object_kind"],
                    candidate["object_id"],
                    candidate["line"],
                    ", ".join(candidate["paths"]),
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
    review_paths = [path for path in path_versions if is_review_document(path)]
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
            f"- `{entry['path']}` at `{entry['object_id']}`"
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
            f"- Git: {inline_code(inventory['git_version'])}",
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


def parse_args(arguments):
    parser = argparse.ArgumentParser(
        description="Inventory commit histories reachable from local refs and compare remote refs."
    )
    parser.add_argument("--repository", default=".")
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
        )
        invocation = shlex.join(
            [
                "python3",
                "ci/inventory-history.py",
                "--repository",
                "<isolated-mirror>",
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
        report.parent.mkdir(parents=True, exist_ok=True)
        report.write_text(render_report(inventory, invocation), encoding="utf-8")
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
