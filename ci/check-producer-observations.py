import json
import os
import re
import stat
import sys
from pathlib import Path, PurePosixPath
from typing import NoReturn
from urllib.parse import urlsplit


FULL_COMMIT = re.compile(r"^[0-9a-f]{40}$")
RELEASE = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+$")
REPOSITORY = re.compile(r"^https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
FULL_COMMIT_IN_PATH = re.compile(r"(?:^|/)[0-9a-f]{40}(?:/|$)")
OBSERVATION_KEYS = {
    "id",
    "producer_repository",
    "producer_commit",
    "producer_release",
    "dialect",
    "evidence",
    "witnesses",
}
DIALECT_REQUIRED_KEYS = {"namespace_iri", "connection_predicate"}
DIALECT_OPTIONAL_KEYS = {"unit_predicates"}
EVIDENCE_KEYS = {"class", "url"}
EVIDENCE_CLASSES = {"producer-source", "operator-git-corpus"}
NAMESPACE_IRIS = {
    "http://data.ashrae.org/S231#",
    "http://data.ashrae.org/S231P#",
    "https://data.ashrae.org/S231P#",
}
CONNECTION_TERMS = {"connectedTo", "isConnectedTo"}
UNIT_PREDICATES = {
    "http://qudt.org/schema/qudt#hasQuantityKind",
    "http://qudt.org/schema/qudt#hasUnit",
}
MAX_MANIFEST_BYTES = 262_144
MAX_WITNESS_BYTES = 1_048_576
WITNESS_ROOTS = {
    ("crates", "cxf-json", "tests", "projection"),
    ("crates", "cxf-json", "tests", "w016"),
}
EXPECTED_OBSERVATIONS = {
    "modelica-json-v1.2.0": {
        "pin": (
            "https://github.com/lbl-srg/modelica-json",
            "ad84dbf479df8e7fd35708e7a15e3a0c45e41662",
            "v1.2.0",
        ),
        "dialect": {
            "namespace_iri": "https://data.ashrae.org/S231P#",
            "connection_predicate": "https://data.ashrae.org/S231P#isConnectedTo",
        },
        "evidence": {
            "class": "producer-source",
            "url": "https://raw.githubusercontent.com/lbl-srg/modelica-json/ad84dbf479df8e7fd35708e7a15e3a0c45e41662/lib/cxfExtractor.js",
        },
    },
    "modelica-json-v1.3.0": {
        "pin": (
            "https://github.com/lbl-srg/modelica-json",
            "c4aa402528187446a6a49e8639fb4b760981c9fb",
            "v1.3.0",
        ),
        "dialect": {
            "namespace_iri": "https://data.ashrae.org/S231P#",
            "connection_predicate": "https://data.ashrae.org/S231P#isConnectedTo",
        },
        "evidence": {
            "class": "producer-source",
            "url": "https://raw.githubusercontent.com/lbl-srg/modelica-json/c4aa402528187446a6a49e8639fb4b760981c9fb/lib/cxfExtractor.js",
        },
    },
    "modelica-json-http-s231p-transition": {
        "pin": (
            "https://github.com/lbl-srg/modelica-json",
            "54777488ad08251d24f65d1ab2afc44b773200a5",
            None,
        ),
        "dialect": {
            "namespace_iri": "http://data.ashrae.org/S231P#",
            "connection_predicate": "http://data.ashrae.org/S231P#isConnectedTo",
        },
        "evidence": {
            "class": "producer-source",
            "url": "https://raw.githubusercontent.com/lbl-srg/modelica-json/54777488ad08251d24f65d1ab2afc44b773200a5/lib/cxfExtractor.js",
        },
    },
    "modelica-json-pinned-operator-corpus": {
        "pin": (
            "https://github.com/lbl-srg/modelica-json",
            "85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb",
            None,
        ),
        "dialect": {
            "namespace_iri": "http://data.ashrae.org/S231#",
            "connection_predicate": "http://data.ashrae.org/S231#isConnectedTo",
            "unit_predicates": [
                "http://qudt.org/schema/qudt#hasUnit",
                "http://qudt.org/schema/qudt#hasQuantityKind",
            ],
        },
        "evidence": {
            "class": "operator-git-corpus",
            "url": "https://github.com/jscott3201/open-control-engine/tree/8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67/third_party/modelica-buildings-cdl/cxf",
        },
    },
}


def fail(message) -> NoReturn:
    raise ValueError(message)


def nofollow_flag():
    try:
        return os.O_NOFOLLOW
    except AttributeError:
        fail("platform must support no-follow file opens")


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON member: {key}")
        value[key] = item
    return value


def read_regular_file(path, trusted_root, max_bytes):
    if ".." in path.parts:
        fail(f"path must not contain parent traversal: {path}")
    path = path.absolute()
    trusted_root = trusted_root.absolute()
    try:
        relative = path.relative_to(trusted_root)
    except ValueError:
        fail(f"path escapes trusted root: {path}")
    if not relative.parts:
        fail("path must name a file")

    directory_flags = os.O_RDONLY | os.O_DIRECTORY | nofollow_flag()
    directory = None
    descriptor = None
    try:
        directory = os.open(trusted_root, directory_flags)
        for part in relative.parts[:-1]:
            next_directory = os.open(part, directory_flags, dir_fd=directory)
            os.close(directory)
            directory = next_directory
        descriptor = os.open(
            relative.parts[-1], os.O_RDONLY | nofollow_flag(), dir_fd=directory
        )
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode):
            fail(f"path is not a regular file: {path}")
        if metadata.st_size > max_bytes:
            fail(f"file exceeds {max_bytes} bytes: {path}")
        with os.fdopen(descriptor, "rb", closefd=False) as file:
            data = file.read(max_bytes + 1)
        if len(data) > max_bytes:
            fail(f"file exceeds {max_bytes} bytes: {path}")
        return data
    except OSError as error:
        fail(f"cannot open regular file without following symlinks: {path}: {error}")
    finally:
        if descriptor is not None:
            os.close(descriptor)
        if directory is not None:
            os.close(directory)


def require_object(value, label):
    if not isinstance(value, dict):
        fail(f"{label} must be an object")
    return value


def require_exact_keys(value, expected, label):
    actual = set(value)
    if actual != expected:
        fail(
            f"{label} keys must be exactly {sorted(expected)}; "
            f"missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )


def require_nonempty_string(value, label):
    if not isinstance(value, str) or not value:
        fail(f"{label} must be a non-empty string")
    return value


def verify_evidence(evidence, producer_commit, label):
    evidence = require_object(evidence, label)
    require_exact_keys(evidence, EVIDENCE_KEYS, label)
    evidence_class = require_nonempty_string(evidence["class"], f"{label}.class")
    if evidence_class not in EVIDENCE_CLASSES:
        fail(f"{label}.class is unsupported: {evidence_class}")

    url = require_nonempty_string(evidence["url"], f"{label}.url")
    parsed = urlsplit(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in {"github.com", "raw.githubusercontent.com"}
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or not FULL_COMMIT_IN_PATH.search(parsed.path)
    ):
        fail(f"{label}.url must be an immutable primary-source HTTPS URL")
    if evidence_class == "producer-source" and producer_commit not in parsed.path:
        fail(f"{label}.url must contain the producer commit")


def witness_path(value, label):
    value = require_nonempty_string(value, label)
    if "\\" in value:
        fail(f"{label} must use POSIX separators")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts:
        fail(f"{label} must be a repository-relative path without parent traversal")
    if tuple(path.parts[:4]) not in WITNESS_ROOTS or path.suffix != ".jsonld":
        fail(f"{label} must name an owned JSON-LD witness")
    return Path(*path.parts)


def verify_observation(observation, index, trusted_root, expected_observations):
    label = f"observations[{index}]"
    observation = require_object(observation, label)
    require_exact_keys(observation, OBSERVATION_KEYS, label)

    observation_id = require_nonempty_string(observation["id"], f"{label}.id")
    if not re.fullmatch(r"[a-z0-9][a-z0-9.-]*", observation_id):
        fail(f"{label}.id must be a lowercase stable identifier")
    repository = require_nonempty_string(
        observation["producer_repository"], f"{label}.producer_repository"
    )
    if REPOSITORY.fullmatch(repository) is None:
        fail(f"{label}.producer_repository must be a canonical GitHub URL")
    commit = require_nonempty_string(
        observation["producer_commit"], f"{label}.producer_commit"
    )
    if FULL_COMMIT.fullmatch(commit) is None:
        fail(f"{label}.producer_commit must be a full lowercase Git commit")
    release = observation["producer_release"]
    if release is not None and (
        not isinstance(release, str) or RELEASE.fullmatch(release) is None
    ):
        fail(f"{label}.producer_release must be null or a release tag")
    expected = expected_observations.get(observation_id)
    if expected is None:
        fail(f"{label}.id is not an approved producer observation")
    if (repository, commit, release) != expected["pin"]:
        fail(f"{label} does not match its approved repository, commit, and release pin")

    dialect = require_object(observation["dialect"], f"{label}.dialect")
    dialect_keys = set(dialect)
    if not DIALECT_REQUIRED_KEYS <= dialect_keys or not dialect_keys <= (
        DIALECT_REQUIRED_KEYS | DIALECT_OPTIONAL_KEYS
    ):
        fail(f"{label}.dialect has missing or unsupported facts")
    namespace = require_nonempty_string(
        dialect["namespace_iri"], f"{label}.dialect.namespace_iri"
    )
    if namespace not in NAMESPACE_IRIS:
        fail(f"{label}.dialect.namespace_iri is not registered")
    predicate = require_nonempty_string(
        dialect["connection_predicate"],
        f"{label}.dialect.connection_predicate",
    )
    if not any(predicate == namespace + term for term in CONNECTION_TERMS):
        fail(f"{label}.dialect.connection_predicate must retain namespace identity")
    if "unit_predicates" in dialect:
        units = dialect["unit_predicates"]
        if (
            not isinstance(units, list)
            or any(not isinstance(unit, str) for unit in units)
            or set(units) != UNIT_PREDICATES
            or len(units) != len(UNIT_PREDICATES)
        ):
            fail(f"{label}.dialect.unit_predicates must name the QUDT predicate pair")
    if dialect != expected["dialect"]:
        fail(f"{label}.dialect does not match the approved producer facts")

    verify_evidence(observation["evidence"], commit, f"{label}.evidence")
    if observation["evidence"] != expected["evidence"]:
        fail(f"{label}.evidence does not match the approved primary source")
    witnesses = require_object(observation["witnesses"], f"{label}.witnesses")
    require_exact_keys(witnesses, dialect_keys, f"{label}.witnesses")
    for fact, path_value in witnesses.items():
        relative = witness_path(path_value, f"{label}.witnesses.{fact}")
        read_regular_file(trusted_root / relative, trusted_root, MAX_WITNESS_BYTES)
    return observation_id


def verify(manifest, trusted_root=None, expected_observations=EXPECTED_OBSERVATIONS):
    trusted_root = Path.cwd() if trusted_root is None else trusted_root
    try:
        document = json.loads(
            read_regular_file(manifest, trusted_root, MAX_MANIFEST_BYTES),
            object_pairs_hook=unique_object,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"manifest must be valid UTF-8 JSON: {error}")
    document = require_object(document, "manifest")
    require_exact_keys(document, {"schema_version", "observations"}, "manifest")
    if document["schema_version"] != 1 or isinstance(document["schema_version"], bool):
        fail("schema_version must be 1")
    observations = document["observations"]
    if not isinstance(observations, list) or not observations:
        fail("observations must be a non-empty array")
    if len(observations) != len(expected_observations):
        fail("manifest must contain every approved producer observation exactly once")

    identifiers = [
        verify_observation(observation, index, trusted_root, expected_observations)
        for index, observation in enumerate(observations)
    ]
    if len(identifiers) != len(set(identifiers)):
        fail("observation identifiers must be unique")
    if set(identifiers) != set(expected_observations):
        fail("manifest must contain every approved producer observation exactly once")


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: check-producer-observations.py MANIFEST.json")
    try:
        verify(Path(sys.argv[1]))
    except (OSError, ValueError) as error:
        raise SystemExit(f"producer observation violation: {error}") from error


if __name__ == "__main__":
    main()
