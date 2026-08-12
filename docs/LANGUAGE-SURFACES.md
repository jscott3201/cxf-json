# Language surfaces

CXF JSON is designed so each host receives the same structured result from one
Rust contract instead of a host-specific interpretation of another parser. This
page tracks the real status of each surface. It deliberately contains no
fictional Python or JavaScript example, because no such adapter exists.

## Binding matrix

| Host | Intended user result | Adapter boundary under design | Today |
|---|---|---|---|
| Rust | Owned source, parse options, typed CXF document, structured diagnostics, compatibility profile | Direct `cxf-json` crate API | Contract foundations; unpublished package |
| Python | Python object model wrapping the same document and diagnostics | Thin adapter over a stable Rust API | Planned |
| Browser JavaScript | Package API returning typed CXF results and browser-visible diagnostics | WASM build of the supported Rust contract | Planned |
| Node.js | Explicit worker/package boundary with time, transfer, and concurrency policy | Node adapter over the same stable contract | Planned; only CI and documented local benchmark execution exist |

## Status

The first supported bindings belong after a supported Rust parse function and
typed CXF document exist. Until then, show code only for the current Rust
contract foundations, and label every planned-host behavior explicitly as
planned. No code in this repository defines a Python package, browser package,
Node package, PyO3 module, JavaScript API, or public WASM parse entry point.

## Rust contract

`crates/cxf-json` is the implementation crate. It currently exports:

- `SourceDocument` and `AdmissionError`;
- `DocumentIri` and `DocumentIriError`;
- `SourcePosition` and `SourceRange`;
- `Diagnostic`, `DiagnosticCode`, `DiagnosticSeverity`, `DiagnosticStage`, and
  `ParseError`;
- `ParseOptions`.

A local development dependency can point at this checkout:

```toml
[dependencies]
cxf-json = { path = "<path-to-cxf-json-checkout>/crates/cxf-json" }
```

The root README example shows admission, an absolute document IRI, and exact
byte retention. When a parse function is reviewed into the profile, update this
page with its real signature; do not sketch one ahead of it.

## Browser and Node direction

The verified build target is `wasm32-unknown-unknown`. Internal probe modules
build for that target and run in Node through CI and the documented local
reproduction commands. Their supported exports are the Node harness contract —
`main`, linear memory, and benchmark revision counters — not a user-facing
JavaScript API. That keeps the browser/JS path a design commitment today.

A future user-facing WASM adapter must consume CXF input through a supported
center API and return typed results. A future Node adapter must also define the
worker, deadline, request, response, and concurrency policy required by the
project's hostile-input boundary work; none of that public boundary exists yet.

## Python direction

A future Python adapter should wrap the same Rust contract rather than
reimplementing parsing. Package naming, wheel layout, PyO3 module naming, and
exact constructors are undecided. The expected result is the same structured
diagnostic information the Rust parser returns, plus Pythonic access to typed
CXF values once those values exist.

## Stability claims by surface

| Surface | Package state | Stability claim |
|---|---|---|
| Rust source | `0.0.0`, local path dependency only | Profile-governed contract, no package release |
| Python | none | None |
| Browser JavaScript | none | None |
| Node.js | none | None |
| Internal WASM probes | instrumentation only | None |
| `spec/PROFILE.md` 0.1.3 | normative project contract | Versioned project surface before package publication |

When a supported binding ships, its documentation must name the Rust profile
version, package version, host/runtime support, and resource policy separately.
Do not infer host safety from Rust defaults.
