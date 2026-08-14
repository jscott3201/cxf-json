# CXF JSON

[![CI](https://github.com/jscott3201/cxf-json/actions/workflows/ci.yml/badge.svg)](https://github.com/jscott3201/cxf-json/actions/workflows/ci.yml)

CXF JSON is building a careful, host-neutral core for
[Control eXchange Format (CXF)](https://obc.lbl.gov/specification/cxf.html)
JSON-LD documents. A Modelica or CDL producer can say exactly what a building
control was configured to do; this project is building the Rust types and
compatibility bands that can retain, diagnose, and eventually validate that
exchange document without losing the bytes someone handed you.

## Read this first

CXF JSON is public source, **not a published parser package**.

Today the `cxf-json` crate is an unpublished Rust contract crate. It owns exact
source bytes, byte locations, validation diagnostics, an optional absolute
document IRI, and the parse options that will govern the public parser. It does
not yet expose a public CXF parse function, typed CXF model, conformance API, or
untrusted-input resource-safety claim.

That boundary is deliberate: the useful, reusable contract is landing before
language bindings are frozen around parser internals.

Host plans and state are tracked in
[`docs/LANGUAGE-SURFACES.md`](docs/LANGUAGE-SURFACES.md). In brief,

- Rust has local contract foundations, but no package release;
- Python and browser JavaScript have planned bindings, not released ones; and
- Node is used only by internal benchmark/smoke wiring today.

## Why this project

Building-control exchange data needs consumers that are precise about identity,
diagnostics, and resource boundaries. CXF JSON keeps those concerns owned and
explicit:

- **Exact source retention** keeps submitted bytes available for diagnostics and
  evidence.
- **Byte locations** report zero-based offsets, lines, and columns rather than
  lossy source abstractions.
- **Structured diagnostics** separate stage, severity, machine code, byte range,
  JSON Pointer, and RDF-term evidence.
- **Host-neutral options** define input, JSON-structure, and RDF-output limit
  configuration without leaking Serde, JSON-LD, RDF, filesystem, HTTP, Python,
  or JavaScript values into the Rust public surface.
- **Language adapters come after the shared contract** so Rust, Python,
  browsers, and Node can eventually receive equivalent typed results instead of
  four different parsers.

## What you can do now

### Develop against the Rust contract crate locally

The crate is intentionally unpublished. For a local development build, add a
path dependency from a crate whose `Cargo.toml` can resolve this checkout's
`crates/cxf-json` directory:

```toml
[dependencies]
cxf-json = { path = "<path-to-cxf-json-checkout>/crates/cxf-json" }
```

The current public Rust types are source and diagnostic foundations, not a
parser. This example is compile-checked manually against the source crate:

```rust
use cxf_json::{DocumentIri, ParseOptions, SourceDocument};

fn admit_example() -> Result<usize, Box<dyn std::error::Error>> {
    let options = ParseOptions::new()
        .with_max_input_bytes(65_536)
        .with_max_json_nesting_depth(32)
        .with_document_iri(DocumentIri::parse("https://example.test/control")?);

    let source = SourceDocument::admit_bytes(br#"{"@context":{}}"#, &options)?;
    assert_eq!(source.as_bytes(), br#"{"@context":{}}"#);
    Ok(source.len())
}
```

Profile 0.1.3 defines defaults of **1 MiB input**, **64 levels of nesting**,
**4,096 members per object**, **65,536 JSON values**, **262,144 decoded
member-name bytes**, **65,536 emitted RDF quads**, and **8 MiB retained RDF term
bytes**. These defaults do not create a supported public parser; the private
semantic stage applies the RDF output options only under profile-controlled
project instrumentation.

### Inspect the probe evidence

The `cxf-ingest-probe` workspace member is an evidence crate for the M0
implementation boundaries. It verifies exact input-byte retention, exact byte
locations, malformed JSON and decoded duplicate-name rejection, embedded JSON-LD
context processing with no network loader, native Rust execution, and
`wasm32-unknown-unknown` execution under Node.

Do not present the probe as the product API. It does not implement typed CXF
projection, profile validation, or production resource limits; do not use it for
untrusted input.

### Track the planned language bindings

The binding plan is ambitious: the Rust, Python, browser, and Node surfaces
should return equivalent typed documents and diagnostics once the parser
boundary is supported. Current reality is narrower: only the Rust contract
foundations are present, and Node’s role in this repository is internal
benchmark and smoke execution, not a supported JavaScript adapter.

See [`docs/LANGUAGE-SURFACES.md`](docs/LANGUAGE-SURFACES.md) for the live
binding-status matrix and the constraints on future PyO3 and JavaScript
documentation. That page stays current as bindings land; this section links to
it instead of copying it.

## Operation and verification

CXF JSON uses Rust 1.97.1.

```console
rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
cargo +1.97.1 fmt --all --check
cargo +1.97.1 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.97.1 test --workspace --all-targets --all-features --locked
cargo +1.97.1 test --workspace --no-default-features --locked
```

CI also runs repository-owned corpus and benchmark aggregation checks, builds
both feature sets for `wasm32-unknown-unknown`, executes Node smoke workloads,
tests reachable-history inventory tooling, enforces exact WASM dependency
allowlists, and runs a bounded coverage-guided parser campaign.

[`benchmarks.md`](benchmarks.md) records corpus, resource-stress, private
production semantic, and WASM baselines with workload identities and
reproduction commands. It also records Linux worker-containment **mechanism
evidence** without calling it a resource-safety baseline or host API. Those
records are development evidence, not package performance promises.

## Security posture

The project is moving toward an admitted, diagnosed, resource-bounded parser,
but it is **not there yet**. Do not use `cxf-json` or `cxf-ingest-probe` as a
public parser for untrusted input. Existing input and output limits constrain
contract-owned copies and project output; they do not bound parser-backend
allocation, process memory, or execution time.

[`ADR 0014`](spec/adr/0014-native-worker-qualification.md) defines how future Linux,
macOS, and Windows worker implementations must qualify. It does not make the current
Linux evidence harness or any package surface safe for untrusted input.

If you discover a vulnerability and repository private vulnerability reporting
is enabled, use GitHub’s private security report flow; otherwise open a
less-detailed public issue and coordinate disclosure through the repository
maintainer.

## Project documentation

- User overview, capability map, and current boundaries: [`docs/OVERVIEW.md`](docs/OVERVIEW.md)
- Language surface plan: [`docs/LANGUAGE-SURFACES.md`](docs/LANGUAGE-SURFACES.md)
- Verification and release posture: [`docs/OPERATIONS.md`](docs/OPERATIONS.md)
- Normative behavior: [`spec/PROFILE.md`](spec/PROFILE.md)
- Compatibility decisions: [`spec/adr/`](spec/adr/)
- Fuzz campaign commands: [`fuzz/README.md`](fuzz/README.md)

`spec/PROFILE.md` is the sole source of truth for observable behavior. ADRs
explain decisions and compatibility impact; research notes outside `spec/` never
outrank the profile.

## Contributing

This is early public source with a narrow compatibility surface. Open an issue
before adding a parser API, typed CXF model, language binding, wire format, or
public resource policy so the behavior and tests can be agreed first.

## License

Licensed under either of the following, at your option:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)
