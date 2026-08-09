# Scope and research decisions

## Product boundary

The project is a purpose-built Control eXchange Format (CXF) parser and exposes
the same CXF result to Rust, Python, browser JavaScript, and Node.js consumers.
CXF represents configured control logic after a Modelica/CDL producer has
translated the source. Parsing Modelica or CDL text is a separate frontend and
is not required for CXF ingestion.

Primary goals:

- Read valid CXF JSON-LD from bytes without network access by default.
- Preserve RDF identity, datatypes, unknown terms, and enough source location
  data to explain failures.
- Project supported CXF terms into typed Rust structures.
- Validate CXF graph shape and semantic rules independently of syntax parsing.
- Return equivalent results and diagnostic codes from Rust, Python, browser
  WASM, and Node WASM.
- Use the upstream CXF corpus for compatibility tests without treating current
  exporter defects as normative behavior.

Initial non-goals:

- General-purpose JSON, JSON-LD, RDF, Modelica, or CDL parsing APIs.
- Parsing `.mo` or CDL source.
- Executing control logic or FMUs.
- Fetching arbitrary remote JSON-LD contexts.
- Reconstructing Modelica source exactly from CXF.
- Repairing lossy upstream output such as absent enum order or connection
  graphics.
- Promising `no_std`; `wasm32-unknown-unknown` is the first portability target.

## Current research conclusions

### D-001: CXF input, not Modelica input

The first public parser accepts CXF JSON-LD bytes or UTF-8 strings. It does not
depend on ANTLR, `MODELICAPATH`, or a Modelica AST. The OBC specification treats
CXF as the configured interchange representation, while upstream
`modelica-json` owns the producer-side Modelica/CDL translation.

Evidence:

- [pinned OBC CXF source](https://raw.githubusercontent.com/lbl-srg/obc/e1c74224778b12297ee49455719c6e58ec71f810/specification/source/cxf.rst)
- [pinned OBC CDL source](https://raw.githubusercontent.com/lbl-srg/obc/e1c74224778b12297ee49455719c6e58ec71f810/specification/source/cdl.rst)
- [upstream Modelica grammar](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/jsParser/antlrFiles/modelica.g4)

### D-002: pest is not the CXF JSON parser

pest handles PEG recognition and source spans. It does not provide JSON-LD
context processing, IRI expansion, RDF conversion, or CXF semantic rules. An
established JSON or JSON-LD parser removes a custom compatibility and security
surface. pest remains relevant only to a future direct Modelica/CDL frontend.

Evidence:

- [pest PEG behavior](https://pest.rs/book/grammars/peg.html)
- [pest 2.8.8 API](https://docs.rs/pest/2.8.8/pest/)
- [JSON-LD 1.1](https://www.w3.org/TR/json-ld11/)

### D-003: CXF-required JSON-LD semantics are part of compatibility

The CXF parser must not hard-code one `@context` plus `@graph` object layout.
Prefixes are aliases, RDF properties may be multi-valued, ordinary JSON-LD
arrays do not imply order, and equivalent compacted documents may serialize
differently. Full IRIs are internal identity.

The parser implements only the JSON-LD operations required by the CXF profile
and accepted producer corpus. D-022 qualifies `oxjsonld` behind an internal
adapter for private CXF ingestion; production-release adoption remains open.

Evidence:

- [JSON-LD forms](https://www.w3.org/TR/json-ld11/#forms-of-json-ld)
- [JSON-LD processing algorithms](https://www.w3.org/TR/json-ld11-api/)
- [JSON-LD sets and lists](https://www.w3.org/TR/json-ld11/#sets-and-lists)

### D-004: source, private RDF, and typed CXF are separate layers

The parser must not force a choice between source fidelity and CXF semantics.
The public document owns:

- source bytes, metadata, and available locations;
- a typed projection for supported CXF classes and predicates;
- unrecognized terms represented as CXF extension records;
- validation diagnostics.

The private ingestion pipeline may use normalized RDF terms and relationships.
The typed layer may diagnose an invalid CXF relation without discarding the
source, typed values, or extension evidence.

### D-005: parsing and validation have different outcomes

Malformed JSON and failed CXF JSON-LD processing prevent document construction.
CXF profile violations return a document plus diagnostics. A strict policy marks
the returned validation report rejected; it does not convert an already-parsed
CXF document into a parse failure or discard its evidence. The report contains
the acceptance flag and the diagnostics that produced it.

### D-006: context loading is offline by default

Embedded and explicitly preloaded contexts are accepted. Network loading
requires a caller-supplied loader and a policy for hosts, schemes, byte limits,
redirects, recursion, timeouts, content types, and cache identity.

Evidence:

- [JSON-LD remote document retrieval](https://www.w3.org/TR/json-ld11-api/#remote-document-and-context-retrieval)
- [JSON-LD security considerations](https://www.w3.org/TR/json-ld11-api/#security)

### D-007: one owned Rust contract, thin adapters

Core crates do not expose `PyObject`, `JsValue`, filesystem handles, or HTTP
clients. Python and WASM adapters convert owned Rust documents, options, and
diagnostics at the boundary. This keeps semantic behavior testable without a
host runtime.

### D-008: bytes are the canonical boundary

`parse_cxf_bytes` is the primary entry point. A string convenience API is allowed,
but JavaScript strings cannot preserve every input byte because unpaired UTF-16
surrogates are replaced during conversion. Python and WASM both expose byte
input.

Evidence:

- [wasm-bindgen string conversion](https://wasm-bindgen.github.io/wasm-bindgen/reference/types/str.html)

### D-009: the document IRI is part of parse context

Parse options accept an optional absolute document IRI, which JSON-LD uses as
the default base. Rust, Python, browser, and Node adapters must pass the same
document IRI to produce equivalent internal term identity.

JSON-LD permits relative IRI references. If RDF conversion omits a relative
identifier because no base resolves it, the reader preserves the submitted
source and any JSON Pointer known by project traversal, then records the loss in
the returned report. This is not a JSON syntax or JSON-LD processing failure.

Evidence:

- [JSON-LD API document loading](https://www.w3.org/TR/json-ld11-api/#loading-document)
- [JSON-LD object to RDF conversion](https://www.w3.org/TR/json-ld11-api/#object-to-rdf-conversion)

### D-010: Rust 1.97.1 is the project toolchain

W-024 configured the workspace, local toolchain file, and CI to use Rust 1.97.1.
Direct dependencies must declare an MSRV no newer than 1.97.1. A later toolchain
change requires a new owner ruling and target-matrix verification.

Local verification on 2026-08-09:

```text
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
```

### D-011: production dependencies require community and maintenance evidence

Direct production dependencies must clear the adoption gate in
`FIRST-SLICE.md`: established repository age, at least 1,000 GitHub stars,
recent maintenance, compatible license and MSRV, supported targets, and no
unapproved advisory in the locked graph. Stars are a minimum signal, not the
entire review. Any exception requires an owner ruling with an expiry and removal
plan.

The owner approved the 1,000-star floor on 2026-08-09. The evidence and refresh
procedure live in `DEPENDENCY-GOVERNANCE.md`.

### D-012: Serde is the preferred ordinary JSON and DTO candidate

W-024 evaluates `serde` and `serde_json` for typed options, owned
result/diagnostic DTOs, and ordinary JSON syntax checks. They do not provide
JSON-LD expansion or a source-preserving tree with per-node byte spans.
JSON-LD-to-RDF processing stays behind the guarded Oxigraph adapter. W-024's
lockfile, CI, license, feature, and advisory gates support private-development
qualification under D-018; production-release adoption remains gated by W-023.

### D-013: private development, dual-licensed public release

The GitHub repository remains private until a stability gate is defined and
met. The public release will use the conventional dual-license expression `MIT
OR Apache-2.0` with both license texts in the repository. Changing visibility is
a separate owner-approved release action, not an automatic consequence of a
version number.

### D-014: v1 retains submitted bytes, not per-node spans

After input-size admission, the v1 source-fidelity contract retains the exact
submitted input bytes and the parser's available error positions. This applies
to successful documents and failures. W-011 must reject oversized input before
making an owned copy. The contract does not promise a byte span for every
successful JSON or RDF node. No package that fails D-011 may be added to obtain
per-node spans. D-023 records the executable W-004 contract.

### D-015: public release preserves audited private history

W-025 publishes the existing private Git history rather than creating a clean
root commit. Before visibility changes, the audit covers all reachable commits,
tags, and refs for secrets, credentials, personal or partner data, proprietary
content, upstream-derived files, license obligations, large binaries, generated
artifacts, and author metadata. The evidence report records the audited commit
set, tool versions, commands, findings, remediation, and final release commit.
Any required history rewrite invalidates the report and requires a fresh audit
plus owner approval.

### D-016: target-only getrandom exception

The owner approved `getrandom` 0.3.4 as a direct target-specific dependency for
`wasm32-unknown-unknown`, despite its 571-star repository falling below D-011.
The dependency may enable only `wasm_js` and exists to activate the same
getrandom release already pulled transitively by OxRDF through `rand`. It adds no
native dependency or network loader.

The exception expires when W-003 completes or before the first public release,
whichever comes first. Re-review must remove it if OxRDF no longer needs direct
feature unification; otherwise renewal requires another owner ruling.
D-021 records the resulting renewal and its replacement expiry.

### D-017: PR CI stays fast; heavy policy runs on releases

Pull requests run formatting, Clippy, native tests for both feature sets, WASM
builds for both feature sets, and exact versioned WASM dependency allowlists.
Dependency advisory and license tools run locally during development and in a
reusable/manual policy workflow, not on every pull request. W-023 owns release
publication automation and must call that policy against the release ref before
publishing. Until W-023 implements that dependency, automated publication is out
of scope.

### D-018: qualify Serde and permit guarded OxJSONLD evaluation

W-024 passed Rust 1.97.1 native and WASM builds, exact dependency allowlists,
local advisory/license policy, PR CI, and clean reviews. Use `serde` and
`serde_json` for ordinary JSON plus owned DTO boundaries during private
development.

W-024 permitted `oxjsonld` and `oxrdf` only for guarded W-003 evaluation. D-022
now governs their private-development CXF use. This is not final production
processor adoption; W-023 retains the production-release gate.

### D-019: the library is a purpose-built CXF parser

The public API accepts CXF files and returns typed CXF documents, extensions,
and diagnostics. JSON syntax, JSON-LD processing, and RDF graph construction are
private implementation stages. The library does not expose a general JSON-LD
processor or RDF graph API.

W-003 tests the JSON-LD behaviors required by CXF and the pinned producer corpus;
it does not seek broad JSON-LD product conformance. Unknown terms remain
available through a CXF extension view so forward compatibility does not turn
the crate into an RDF toolkit.

### D-020: ordered CXF projection comes from source, not RDF

W-003 proves that reordering an ordinary JSON-LD array leaves the converted RDF
unchanged. CXF consumers may assign meaning to `@graph`, containment, port,
parameter, and connection order. The parser therefore retains an ordered CXF
source/DTO representation for projection. The private RDF stage supplies term
identity and set relationships; RDF iteration order never determines CXF order.

### D-021: renew the target-only getrandom exception through W-009

The owner renewed D-016 on 2026-08-09 after W-003 passed its local native, WASM,
dependency, and read-only CXF corpus gates. `getrandom` remains pinned to 0.3.4,
target-specific to `wasm32-unknown-unknown`, optional with the Oxigraph adapter,
and limited to `wasm_js`.

The renewal expires when W-009 completes or before the first public release,
whichever comes first. W-009 must remove the direct feature-unification
dependency if OxRDF no longer needs it; another renewal requires a new owner
ruling. `ci/check-release-blockers.py` reads locked Cargo metadata and makes the
reusable release policy fail while either the resolved direct dependency or its
D-021 marker remains. Workspace inheritance and package aliases do not bypass
the check; deleting the marker alone does not clear it.

### D-022: qualify guarded OxJSONLD/OxRDF for private CXF ingestion

W-003 passed the owned CXF operation matrix, the Git-pinned read-only Open
Control Engine corpus, native and Node-executed WASM tests, resource observation,
dependency policy, PR CI, and independent review. Keep `oxjsonld` and `oxrdf`
behind the private CXF adapter for term identity, datatypes, language tags, and
set relationships during private development.

This decision does not expose RDF types, derive CXF array order from RDF, enable
remote loading, or approve a production release. D-020 governs ordered
projection, D-021 governs the temporary WASM entropy exception, and W-023 still
gates production-release dependency adoption.

### D-023: reject duplicate JSON names and keep source evidence independent

W-004 makes zero-based byte offsets authoritative for parser detection
locations. Line and byte-column values are derived from the retained submitted
bytes. Ranges are half-open; point diagnostics have equal start and end. Exact
number spelling remains available only in the submitted document. Invalid UTF-8
is checked first and points to its first invalid byte.

Reject duplicate decoded object member names before JSON-LD processing. Do not
expose last-wins Serde or processor behavior as CXF semantics. JSON Pointers are
optional syntax paths emitted only when project traversal knows an unambiguous
path. Optional RDF terms are independent semantic evidence. Neither field
creates pointer-to-quad, span-to-term, or blank-node provenance.

V1 does not add a source mapper. A later source-aware editor or rewrite feature
requires its own work item and dependency ruling.

## Provisional decisions

The remaining proposals require spikes before adoption. Superseded entries stay
in this list as decision history.

- `D-P01`: superseded by D-022; use the guarded adapter for private CXF
  ingestion, subject to D-020, D-021, and W-023.
- `D-P02`: superseded by D-014 and D-023; retained submitted bytes plus
  available parser positions are the v1 contract, with no per-node mapper.
- `D-P03`: ship one npm package with explicit browser, web, and Node subpaths.
- `D-P04`: target ordinary CPython plus version-specific CPython 3.14t wheels;
  consider `abi3t` when Python 3.15 is in the supported matrix.
