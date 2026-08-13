# CXF JSON overview

CXF JSON reads [Control eXchange Format (CXF)](https://obc.lbl.gov/specification/cxf.html)
as JSON-LD after a Modelica or CDL producer emits the configured control logic.
The project goal is a center contract that keeps source evidence, parse
configuration, validation results, and language adapters aligned.

## Product shape

The repository is moving toward one shared sequence:

1. admit source bytes under explicit input limits;
2. parse and diagnose JSON in bounded project stages;
3. process private JSON-LD/RDF under project output limits;
4. project supported CXF terms into owned typed values plus diagnostics;
5. expose equivalent typed results through Rust and later language adapters.

Only the contract foundations for this shape exist today. The later product
stages that matter to users — supported parsing, typed CXF values, conformance
validation, and language bindings — are not exposed yet.

## Current capability map

| Capability | Current support | Notes |
|---|---|---|
| Own exact source bytes | Supported Rust center type | `SourceDocument::from_bytes` retains ownership exactly; `admit_bytes` first applies the input limit |
| Input-size admission | Supported Rust center behavior | Inclusive 1 MiB default; oversized input returns only byte counts |
| Absolute document IRI | Supported Rust center type | RFC 3987 validation through a private backend; exact spelling retained and debug output redacted |
| Byte positions and ranges | Supported Rust center types | Offsets and columns are bytes; positions and lines are zero-based |
| Structured diagnostics | Type contract only | Code, severity, stage, message, range, pointer, and RDF-term evidence are independent fields |
| JSON structure options | Reserved ParseOptions | Depth, object-member, total-value, and decoded-name limits are defined but no public parser applies them yet |
| RDF output options | Private semantic boundary | Quad and retained-term limits govern private project output, not backend allocation |
| Typed CXF projection | Private crate stage | Classified nodes, edges, and opaque values since profile 0.1.4 |
| Public CXF parse API | Not built | No supported parse entry point |
| Conformance validation | Private validator core | Stable private `CXF-V-*` rule codes since profile 0.1.6; no public validation surface |
| Namespace acceptance policy | Private findings | Normative matrix with `CXF-C-*` warning findings since profile 0.1.7; nothing rejected beyond preflight |
| Untrusted-input safety | Not built | Backend memory, time, diagnostics, and host policy remain open |

## What the probe demonstrates

The `cxf-ingest-probe` crate is evidence code for implementation feasibility.
It verifies exact byte retention and byte locations, malformed JSON and
decoded duplicate member-name rejection, embedded JSON-LD context processing
without remote loading, native execution, and `wasm32-unknown-unknown`
execution under Node.

The probe is not the `cxf-json` product API and must not be used for
untrusted input. It does not provide typed CXF projection, profile validation,
host resource policy, or production diagnostics.

## Design boundaries you should know

- JSON-LD and RDF are private implementation stages. There is no public RDF
  graph API.
- Public signatures must not expose Serde, OxIRI, OxJSONLD, OxRDF, filesystem,
  HTTP, Python, or JavaScript values.
- `cxf-json` is package version `0.0.0` with `publish = false`, so there is no
  package stability claim.
- The doc-hidden instrumentation module available in fuzzing and explicit
  semantic-harness builds is not a supported API.
- The fuzz campaigns in [`../fuzz/README.md`](../fuzz/README.md) bound those
  processes only; they are not parser defaults.

## Source references

The normative contract lives in [`../spec/PROFILE.md`](../spec/PROFILE.md) and
is enforced by profile tests and CI. ADRs in [`../spec/adr/`](../spec/adr/)
record why each compatibility surface chose its current shape. Benchmark
methodology and baseline evidence live in [`../benchmarks.md`](../benchmarks.md).
