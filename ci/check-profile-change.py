import json
import os
import re
import subprocess
import sys
from pathlib import Path


profile_path = Path("spec/PROFILE.md")
profile_test = Path("crates/cxf-json/tests/profile_0_1.rs")
contract_adr = Path("spec/adr/0004-w006-core-contract.md")
version_pattern = re.compile(r"^Profile version: ([0-9]+\.[0-9]+\.[0-9]+)$", re.MULTILINE)
impact_pattern = re.compile(
    r"^Compatibility impact: (Initial|Breaking|Additive|Clarification)$", re.MULTILINE
)


def fail(message):
    print(f"profile contract violation: {message}")
    sys.exit(1)


def profile_version(text):
    matches = version_pattern.findall(text)
    if len(matches) != 1:
        fail("spec/PROFILE.md must contain exactly one Profile version line")
    return matches[0]


def version_tuple(version):
    return tuple(int(part) for part in version.split("."))


def compatibility_impact(paths):
    impacts = [
        impact
        for path in paths
        for impact in impact_pattern.findall(Path(path).read_text())
    ]
    if len(impacts) != 1:
        fail("a profile version change must have exactly one compatibility impact")
    return impacts[0]


def expected_version(base_version, impact):
    major, minor, patch = base_version
    if base_version == (0, 0, 0):
        return (0, 1, 0) if impact == "Initial" else None
    if impact == "Initial":
        return None
    if impact == "Breaking":
        return (0, minor + 1, 0) if major == 0 else (major + 1, 0, 0)
    if impact == "Additive":
        return (0, minor, patch + 1) if major == 0 else (major, minor + 1, 0)
    if impact == "Clarification":
        return (major, minor, patch + 1)
    return None


def git_show(base, path):
    return subprocess.run(
        ["git", "show", f"{base}:{path}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def dependency_line(manifest_text, name):
    match = re.search(rf"^{re.escape(name)} = .+$", manifest_text, re.MULTILINE)
    return match.group(0) if match else None


def locked_package(lock_text, name):
    records = lock_text.split("[[package]]")
    matches = [
        record.strip()
        for record in records
        if re.search(rf'^name = "{re.escape(name)}"$', record, re.MULTILINE)
    ]
    if len(matches) != 1:
        fail(f"Cargo.lock must contain exactly one {name} package")
    return matches[0]


profile = profile_path.read_text()
if profile_version(profile) == "0.0.0":
    fail("the current profile must be behavior-bearing")
if not profile_test.is_file():
    fail("profile 0.1.0 integration tests are missing")
if not contract_adr.is_file() or compatibility_impact([contract_adr]) != "Initial":
    fail("ADR 0004 must record compatibility impact")
metadata = json.loads(
    subprocess.run(
        [
            "cargo",
            "+1.97.1",
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
package = next((package for package in metadata["packages"] if package["name"] == "cxf-json"), None)
if package is None or package["publish"] != []:
    fail("the cxf-json package must exist and remain unpublished")
dependencies = {dependency["name"] for dependency in package["dependencies"]}
if dependencies != {"oxiri"}:
    fail("the cxf-json package may depend only on private IRI validation in profile 0.1.0")


base = os.environ.get("PROFILE_BASE_SHA")
if not base:
    sys.exit(0)

changed = set(
    subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=ACMRT", base],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
)
changed.update(
    subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
)
contract_changed = any(
    path == "crates/cxf-json/Cargo.toml"
    or path == "crates/cxf-json/src/lib.rs"
    or path.startswith("crates/cxf-json/src/contract/")
    for path in changed
)
base_manifest = git_show(base, "Cargo.toml")
base_lock = git_show(base, "Cargo.lock")
delegated_iri_changed = dependency_line(base_manifest, "oxiri") != dependency_line(
    Path("Cargo.toml").read_text(), "oxiri"
) or locked_package(base_lock, "oxiri") != locked_package(Path("Cargo.lock").read_text(), "oxiri")
contract_changed = contract_changed or delegated_iri_changed
profile_changed = "spec/PROFILE.md" in changed
tests_changed = any(path.startswith("crates/cxf-json/tests/") for path in changed)
changed_adrs = [path for path in changed if path.startswith("spec/adr/")]

if contract_changed and not profile_changed:
    fail("a cxf-json public contract change must update spec/PROFILE.md")
if not profile_changed:
    sys.exit(0)

base_version = version_tuple(profile_version(git_show(base, "spec/PROFILE.md")))
current_version = version_tuple(profile_version(profile))
if current_version < base_version:
    fail("the profile version must not decrease")
if current_version == base_version:
    if contract_changed:
        fail("a public contract change must increase the profile version")
    sys.exit(0)
if not changed_adrs:
    fail("a profile version change must add or update an ADR")

impact = compatibility_impact(changed_adrs)
expected = expected_version(base_version, impact)
if expected is None:
    fail(f"{impact} impact is invalid from profile version {base_version}")
if current_version != expected:
    fail(f"{impact} impact requires profile version {expected}, found {current_version}")
if contract_changed and impact == "Clarification":
    fail("a public contract change cannot be classified as a clarification")
if impact != "Clarification" and not tests_changed:
    fail("an initial, breaking, or additive profile change must update profile tests")
