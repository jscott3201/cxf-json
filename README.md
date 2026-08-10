# CXF JSON

[![CI](https://github.com/jscott3201/cxf-json/actions/workflows/ci.yml/badge.svg)](https://github.com/jscott3201/cxf-json/actions/workflows/ci.yml)

CXF JSON is an early-stage Rust implementation for reading and validating
[Control eXchange Format (CXF)](https://obc.lbl.gov/specification/cxf.html)
JSON-LD. CXF represents configured building-control logic after a Modelica or
CDL producer has translated its source.

The project is intended to expose the same typed CXF document and diagnostics to
Rust, Python, browser JavaScript, and Node.js. JSON-LD and RDF processing remain
internal implementation stages rather than general-purpose public APIs.

## Status

The source repository is public, but no `cxf-json` package has been published.
The behavior profile remains at reserved version 0.0.0 and makes no CXF
conformance or API-stability claim.

The workspace currently contains only `cxf-ingest-probe`, an unpublished
evidence crate. It does not implement typed CXF projection, profile validation,
or production resource limits. Do not use it to process untrusted input.

## Intended scope

- Accept CXF JSON-LD as bytes, with network access disabled by default.
- Preserve submitted bytes, RDF term identity, datatypes, unknown CXF terms, and
  available source locations.
- Project supported CXF terms into owned Rust types and return versioned
  validation diagnostics without discarding the parsed document.
- Keep Rust, Python, browser, and Node results equivalent at their boundaries.

Modelica and CDL source parsing, control execution, FMU execution, and a public
RDF graph API are outside the current scope.

## Current probe

The probe and its repository-authored fixtures verify:

- exact input-byte retention and zero-based byte locations;
- malformed JSON, invalid UTF-8, and duplicate decoded member-name rejection;
- embedded JSON-LD context processing without a network loader;
- owned RDF summaries without exposing processor-specific RDF types;
- native Rust and `wasm32-unknown-unknown` builds, including a Node-executed WASM
  smoke test.

These checks qualify implementation boundaries. They are not a public parser API
or a conformance suite.

## Build and test

The repository uses Rust 1.97.1.

```console
rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
cargo +1.97.1 fmt --all --check
cargo +1.97.1 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.97.1 test --workspace --all-targets --all-features --locked
cargo +1.97.1 test --workspace --no-default-features --locked
```

CI also builds both feature sets for `wasm32-unknown-unknown`, runs the WASM
smoke test in Node, and checks exact WASM dependency allowlists.

## Specification

[`spec/PROFILE.md`](spec/PROFILE.md) is the sole source of truth for observable
behavior. Version 0.0.0 is a placeholder with no accepted-input, output,
diagnostic, compatibility, or resource-limit contract. Architecture decisions
and their compatibility impact are recorded in [`spec/adr/`](spec/adr/).

## Contributing

The public API and first behavior-bearing profile are still being designed. Open
an issue before starting a large change so its scope and compatibility impact can
be agreed before implementation.

## License

Licensed under either of the following, at your option:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)
