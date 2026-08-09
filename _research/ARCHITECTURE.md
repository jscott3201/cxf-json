# Proposed architecture

## Workspace shape

```text
crates/
  cxf-types/       owned CXF values, extensions, source locations, diagnostics
  cxf-reader/      bytes -> JSON -> JSON-LD -> graph -> typed projection
  cxf-validation/  versioned CXF profile rules
  cxf-python/      PyO3 extension module
  cxf-wasm/        wasm-bindgen browser and Node adapter
```

JSON-LD and RDF dependencies remain private to the reader. If they require a
crate boundary after W-003, use an internal processing crate that is not exposed
as a general JSON-LD product.

W-024 precedes this public workspace shape with one `publish = false`
`cxf-ingest-probe` crate. It may prove boundaries but does not establish public
crate names or APIs.

The workspace should use Cargo feature resolver 2. Python and WASM crates are
leaf adapters. Neither is a feature on the same library target because their
host dependencies and crate types are unrelated.

Sources:

- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo feature resolver](https://doc.rust-lang.org/cargo/reference/features.html)

## Data flow

```text
bytes
  -> input policy and UTF-8 check
  -> retained source plus ordered Serde CXF syntax/DTO boundary
  -> private OxJSONLD context processing and RDF conversion
  -> private graph indexes by full IRI
  -> typed CXF projection joining source order with RDF identity
  -> CXF extension records
  -> versioned profile diagnostics
  -> Rust / Python / JavaScript DTO conversion
```

Parsing, projection, and validation remain separately testable internal stages.
Consumers receive a CXF document plus diagnostics, including when profile
validation rejects it. A strict gate marks the validation report rejected
without discarding the parsed CXF evidence.

## Core contract sketch

This is illustrative, not a committed API.

```rust
pub fn parse_cxf_bytes(
    input: &[u8],
    options: &ParseOptions,
) -> Result<ParsedCxfDocument, ParseFailure>;

pub struct ParsedCxfDocument {
    pub source: SourceDocument,
    pub cxf: CxfProjection,
    pub extensions: Vec<CxfExtension>,
    pub validation: ValidationReport,
}
```

`ParseFailure` is limited to failures that prevent CXF document construction,
such as input limits, invalid UTF-8, malformed JSON, or failed JSON-LD processing.
`ValidationReport` contains an acceptance flag and diagnostics. Profile policy
determines the flag, so strict rejection is observable without losing the
document.

`ParseOptions` includes an optional absolute document IRI. JSON-LD uses it as
the default base for relative identifiers. Every host adapter exposes the same
option. If RDF conversion omits an unresolved relative IRI, the reader preserves
its located JSON value and records a conversion-loss diagnostic rather than
turning valid JSON-LD into a parse failure.

The public contract needs these invariants:

- no borrowed input escapes `parse_cxf_bytes`;
- full IRIs identify terms inside the private graph stage;
- source bytes are retained; reported locations use byte offsets when the
  approved parser exposes them;
- unknown terms survive in the CXF extension view;
- CXF array order comes from the ordered source/DTO layer, never RDF iteration;
- diagnostic codes are stable within a major release;
- public methods do not panic on user input.

## Identity and extensions

Do not rewrite a legacy IRI into a current IRI inside the graph. A profile may
project both to one known CXF role while retaining the original term and a
compatibility diagnostic. This prevents normalization from fabricating RDF
equivalence.

Unknown classes, predicates, and values remain queryable through CXF extension
records. A closed Rust enum may represent known CXF roles, while extension
records associate an unknown term and value with its CXF entity. W-013 owns the
concrete record schema. The schema must retain original full IRIs without
exposing a general graph API.

## Loader boundary

The core loader interface accepts preloaded documents or returns a policy
failure. Native HTTP, filesystem, browser fetch, and Node fetch belong in
separate adapters. The initial release supports embedded and caller-provided
contexts only.

Any future remote loader must receive the absolute requested IRI and return
bytes plus the final IRI and content type. The policy layer owns allowlists,
redirects, limits, caching, and cancellation.

## Concurrency

Parsed documents should be immutable after construction. Shared caches must be
bounded and synchronized, or omitted from the initial release. Immutable owned
results reduce free-threaded Python risk and let native callers share documents
with `Arc` without adapter-specific synchronization.

If validation configuration is compiled, keep the compiled form immutable.
Avoid process-global mutable loader or context state.

## Serialization

Do not promise byte-for-byte JSON-LD round-trip. Source bytes can be retained
when requested, while semantic serialization may choose a normalized output.
Any canonical RDF output is a separate operation and requires a tested blank
node canonicalization algorithm.

Python and JavaScript DTO serialization must define integer, decimal, IRI,
binary source, optional-field, and validation-report behavior explicitly. A
bulk JSON form serializes the whole CXF document envelope.
Boundary equivalence is covered by `W-010`.

## Panic and failure boundary

All input-dependent failures return typed results. Python maps them to one base
exception with structured attributes. WASM throws or returns a structured error
according to one documented convention. The project should not rely on catching
Rust panics in WASM; panic recovery has additional nightly and runtime
requirements in wasm-bindgen.

Evidence:

- [wasm-bindgen panic handling](https://wasm-bindgen.github.io/wasm-bindgen/reference/catch-unwind.html)
