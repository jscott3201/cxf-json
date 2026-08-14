# Operations and release posture

This page separates what contributors verify locally, what CI verifies, and
what the project does not yet claim for package users.

## Local development loop

Use Rust 1.97.1 and the repository lockfile:

```console
rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
cargo +1.97.1 fmt --all --check
cargo +1.97.1 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.97.1 test --workspace --all-targets --all-features --locked
cargo +1.97.1 test --workspace --no-default-features --locked
```

The Python policy and benchmark evidence checks are:

```console
python3 ci/check-release-blockers.py --development
python3 ci/test-summarize-benchmarks.py
python3 ci/test-inventory-history.py
```

A pull request also needs the profile-change gate when it changes public
contract files, `spec/PROFILE.md`, or delegated OxIRI behavior.

## CI coverage

The GitHub CI workflow verifies:

- formatting and workspace Clippy with warnings denied;
- all-feature and no-default Rust tests;
- repository-owned benchmark corpus structure and resource-stress outcomes;
- private production semantic aggregation;
- native worker-containment mechanism evidence;
- WASM builds with and without the default semantic feature;
- Node execution of internal WASM smoke workloads;
- exact WASM dependency allowlists;
- benchmark aggregation and reachable-history inventory tooling;
- coverage-guided parser campaign completion.

The release-policy workflow rejects dependency exceptions in non-development
mode. D-021 and D-029 still block package release.

## Benchmark and fuzz evidence

[`../benchmarks.md`](../benchmarks.md) is a recording of measured local
evidence. It names workload identity, instrumentation revision, command, stage
definitions, environmental details, and caveats. It gives reviewers a way to
reproduce development evidence; it does not set release thresholds.

[`../fuzz/README.md`](../fuzz/README.md) holds a separately locked fuzz
workspace and the command-line bounds for the JSON preflight and private
semantic targets. The local commands use a 30-second total campaign limit; the
CI campaign uses the workflow's fixed run-count and per-input timeout limits.
Those bounds do not transfer to end users.

## Specification and compatibility

[`../spec/PROFILE.md`](../spec/PROFILE.md) is the only normative behavior
contract. Version 0.1.8 covers the source, document IRI, JSON-structure,
diagnostic, admission, and RDF-output foundations plus the private projection,
validation, and namespace policy. No supported public parser exists. A change
to public contract behavior must update the profile version, enforce tests, and
add or supersede one ADR in [`../spec/adr/`](../spec/adr/).

- **Breaking pre-1.0:** increment the profile minor version and reset the patch
  version to zero.
- **Additive behavior before 1.0:** increment the patch version.
- **Clarification:** increment the patch version unless CI can prove the change
  is whitespace-only and no public contract file changed.

## Publication state

`cxf-json` is package version `0.0.0` and `publish = false`. No crates.io,
PyPI, npm, browser package, or release artifact is published from this
repository. GitHub Release pages, package catalogs, and host-runtime examples
must not be cited until a supported package and governing profile exist.

## Public support boundaries

Do not process untrusted input with the current repository code. Existing
admission and project output limits do not bound OxJSONLD temporary allocation,
backend diagnostic amplification, process memory, execution time, filesystem
behavior, or network behavior beyond the private adapter not installing a
remote-context loader.

Open an issue before changing parser/API scope, resource limits, package names,
binding surfaces, wire formats, compatibility policy, or benchmark claims.

[`ADR 0014`](../spec/adr/0014-native-worker-qualification.md) records the native
qualification criteria for Linux, macOS, and Windows. No target currently satisfies
those criteria. The Linux report remains mechanism evidence, and its limits are not
package defaults.
