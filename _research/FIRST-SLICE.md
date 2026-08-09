# W-024: Serde/OxJSONLD ingestion boundary

Status: in progress as M0-C1.

## Purpose

Prove the smallest dependency-gated candidate path from JSON-LD bytes to RDF while
recording what Serde can and cannot preserve. This is an internal probe, not a
CXF parser or public API.

The slice uses Rust 1.97.1. Dependencies are the latest non-prerelease releases
that clear the adoption gate when implementation begins; `Cargo.lock` records
the exact resolution used for the evidence.

## Dependency adoption gate

A direct production dependency must satisfy all of these conditions before
production adoption:

- parent repository has at least 1,000 GitHub stars;
- project is at least three years old, not archived, and has a release or
  meaningful commit within the previous 12 months;
- contributor breadth and concentration are recorded; concentrated ownership
  requires a narrow adapter and recurring health review;
- declared MSRV is no newer than Rust 1.97.1;
- required native and `wasm32-unknown-unknown` feature sets build in CI created
  by the adopting slice;
- license is compatible and the locked transitive graph has no unapproved
  vulnerability or unmaintained-package advisory.

Stars are a floor, not proof of health. The owner approved the 1,000-star value
on 2026-08-09. An exception requires an owner ruling with a time limit and removal plan.
Transitive dependencies receive the advisory and maintenance check even when
their parent package clears the star threshold.

Snapshot on 2026-08-09:

| Package | Release | Parent evidence | Disposition |
|---|---:|---|---|
| `serde` | 1.0.229 | `serde-rs/serde`: 10,759 stars, 929 forks, pushed 2026-07-25 | Approved candidate |
| `serde_json` | 1.0.151 | `serde-rs/json`: 5,614 stars, 661 forks, pushed 2026-08-08 | Approved candidate |
| `oxjsonld` | 0.2.5 | `oxigraph/oxigraph`: 1,805 stars, 162 forks, pushed 2026-08-09 | Guarded candidate |
| `oxrdf` | 0.3.3 | Same Oxigraph monorepo | Guarded candidate |
| `json-ld` | 0.21.4 | Individual repository: 153 stars, concentrated contributors | Reject for production |
| `json-syntax` | 0.12.5 | Individual repository: 6 stars; unmaintained transitive advisory | Reject |

Evidence:

- [Serde repository snapshot](https://api.github.com/repos/serde-rs/serde)
- [serde_json repository snapshot](https://api.github.com/repos/serde-rs/json)
- [Oxigraph repository snapshot](https://api.github.com/repos/oxigraph/oxigraph)
- [json-ld repository snapshot](https://api.github.com/repos/timothee-haudebourg/json-ld)
- [json-syntax repository snapshot](https://api.github.com/repos/timothee-haudebourg/json-syntax)
- [`json-syntax` transitive advisory](https://rustsec.org/advisories/RUSTSEC-2026-0215.html)
- [frozen dependency-governance record](DEPENDENCY-GOVERNANCE.md)

Oxigraph clears the age, adoption, and activity floors, but its contribution
history is concentrated around one lead maintainer. Its types stay behind an
internal adapter and do not become the CXF domain API.

The owner approved Oxigraph for this guarded probe on 2026-08-09. Production
adoption still depends on the lockfile, CI, license, feature, and advisory gates.

D-016 permits one below-floor direct dependency: target-specific `getrandom`
0.3.4 with only `wasm_js`, required to enable OxRDF's existing transitive entropy
backend on `wasm32-unknown-unknown`. It is not part of the native graph. The
exception expires at W-003 completion or before public release, whichever comes
first.

## In scope

- Root workspace with resolver 2, edition 2024, and `rust-version = "1.97.1"`.
- `rust-toolchain.toml` pinned to 1.97.1.
- Private GitHub PR CI for native tests and the WASM compile gate; dependency
  advisory/license installation runs locally and in the release workflow.
- `LICENSE-MIT`, `LICENSE-APACHE`, and Cargo metadata `MIT OR Apache-2.0`;
  repository visibility remains private.
- One `publish = false` crate: `crates/cxf-ingest-probe`.
- Serde-derived owned result and diagnostic DTOs.
- `serde_json` syntax/DTO path over retained input bytes.
- `oxjsonld` to `oxrdf` conversion behind an internal trait or module boundary.
- Embedded-context JSON-LD only; no network-capable loader or dependency.
- Native tests and a `wasm32-unknown-unknown` compile gate.
- Dependency tree, feature, license, advisory, and community evidence captured in
  `_research/results/W-024.md` when the slice runs.

## Out of scope

- CXF classes, typed projection, profile validation, and public diagnostic codes.
- Full W3C JSON-LD conformance or upstream CXF corpus tests.
- Per-node source spans or source-to-RDF provenance links. D-014 explicitly
  defers these beyond the v1 raw-source contract.
- Byte-for-byte JSON round-trip.
- Remote context loading.
- Blank-node canonicalization and performance thresholds.
- Upstream fixture copying or transformation.
- PyO3, wasm-bindgen, browser execution, Node execution, and package publishing.

## Owned fixtures

Fixtures use only `https://example.test/` identifiers and are authored in this
repository. `tests/fixtures/PROVENANCE.md` records author, license, checksum, and
that no upstream artifact was copied or transformed. Invalid UTF-8 is assembled
inside tests rather than stored as a text fixture.

Required cases:

- minimal object and array JSON;
- duplicate object member at the root and one nested level;
- `1`, `1.0`, `1e+02`, `-0`, a large integer, and a long fraction;
- UTF-8 object keys and JSON Pointer escaping cases;
- malformed JSON and invalid UTF-8;
- embedded context producing a named-node RDF triple;
- typed literal, language-tagged string, unknown term, and graph object;
- remote context that must fail without attempting network access.

## Acceptance gates

1. `rustc +1.97.1 --version` and `cargo +1.97.1 --version` report 1.97.1.
2. `cargo +1.97.1 test --workspace --all-features` passes.
3. Repository CI builds the workspace for `wasm32-unknown-unknown` with the
   Oxigraph adapter enabled and no filesystem, TLS, or HTTP client in the
   dependency tree.
4. Serde DTOs round-trip source ranges, diagnostics, and RDF summary values
   through `serde_json` without change.
5. Tests record `serde_json::Value` duplicate-member and numeric-spelling
   behavior. The probe must not claim last-wins data or normalized numbers are
   source-faithful.
6. The original byte buffer remains available for diagnostics and future
   provenance work.
7. `oxjsonld` produces the expected subject, predicate, object, datatype,
   language tag, and graph placement for owned fixtures.
8. A remote-context fixture cannot perform network access and returns a
   deterministic unsupported/policy result.
9. The evidence report records exact versions from `Cargo.lock`, feature graph,
   duplicate crates, licenses, local RustSec results, repository-health snapshot,
   successful PR native/WASM CI, build commands, and unresolved gaps. The manual
   Release workflow reruns heavy advisory and license policy against the requested
   tag and publishes only after it passes. Only then can a candidate become an
   adopted production dependency.

## Decisions this slice can make

- Whether Serde and `serde_json` are sufficient for ordinary JSON syntax,
  options, DTOs, and diagnostic envelopes.
- Whether `oxjsonld`/`oxrdf` can remain the guarded JSON-LD semantic adapter on
  native Rust and the WASM target.
- Whether retaining source bytes plus available parser error positions is enough
  for the first public release. D-014 answers yes; W-004 owns any later
  project-owned source mapper.
- Whether the proposed source/RDF boundary in D-004 is executable without
  exposing third-party RDF types.

The slice does not complete W-003, W-004, W-005, or W-006. Those items remain
queued or planned at their existing corpus, licensing, and public-API scope.
