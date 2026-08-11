import json
import subprocess
import sys
from pathlib import Path


getrandom_blocker_path = Path("ci/release-blockers/D-021-getrandom-wasm-js")
isolation_blocker_path = Path("ci/release-blockers/D-029-untrusted-backend-isolation")
development_mode = sys.argv[1:] == ["--development"]
if sys.argv[1:] not in ([], ["--development"]):
    print("usage: python3 ci/check-release-blockers.py [--development]")
    sys.exit(2)

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
workspace_members = set(metadata["workspace_members"])
direct_getrandom = [
    {
        "package": package["name"],
        "req": dependency["req"],
        "kind": dependency["kind"],
        "rename": dependency["rename"],
        "optional": dependency["optional"],
        "uses_default_features": dependency["uses_default_features"],
        "features": dependency["features"],
        "target": dependency["target"],
    }
    for package in metadata["packages"]
    if package["id"] in workspace_members
    for dependency in package["dependencies"]
    if dependency["name"] == "getrandom"
]

if development_mode:
    expected = [
        {
            "package": package,
            "req": "=0.3.4",
            "kind": None,
            "rename": None,
            "optional": True,
            "uses_default_features": True,
            "features": ["wasm_js"],
            "target": "wasm32-unknown-unknown",
        }
        for package in ("cxf-ingest-probe", "cxf-json")
    ]
    if sorted(direct_getrandom, key=lambda dependency: dependency["package"]) != expected:
        print("D-021 violation: getrandom must retain its exact target-only dependency shape")
        sys.exit(1)
    if not getrandom_blocker_path.is_file():
        print("D-021 violation: development exception is missing its release blocker")
        sys.exit(1)
    if not isolation_blocker_path.is_file():
        print("D-029 violation: backend isolation release blocker is missing")
        sys.exit(1)
    sys.exit(0)

blocked = False
if direct_getrandom:
    if not getrandom_blocker_path.is_file():
        print("D-021 violation: target-only getrandom remains but its release blocker is missing")
    else:
        print(getrandom_blocker_path.read_text().rstrip())
    blocked = True
elif getrandom_blocker_path.exists():
    print("D-021 release blocker is stale after target-only getrandom removal")
    blocked = True

if isolation_blocker_path.exists():
    print(isolation_blocker_path.read_text().rstrip())
    blocked = True

if blocked:
    sys.exit(1)
