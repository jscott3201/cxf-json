# Parser and validation strategy

## Recommendation

Use `serde` and `serde_json` inside the CXF parser for syntax, options, and owned
boundary DTOs. D-022 permits `oxjsonld`/`oxrdf` as a private CXF ingestion stage.
Retain original input bytes. Per-node source spans remain an open product
requirement because the packages that provide them do not clear the dependency
community gate.

Do not write a pest grammar for JSON. Implement the JSON-LD behavior CXF needs,
test it against the relevant standard cases, and make no general JSON-LD claim.

## Decision matrix

| Candidate | Strength | Gap | Current use |
|---|---|---|---|
| `serde_json` 1.0.151 | Established Serde organization, typed and streaming JSON, 5,614-star parent | No JSON-LD semantics or per-node source map; ordinary value model does not preserve duplicates | Qualified for private development by D-018 |
| `oxjsonld` 0.2.5 | Direct JSON-LD to `oxrdf`; active 1,805-star Oxigraph parent | No documented lossless source tree; contributor concentration requires isolation | Private CXF stage under D-022 |
| `json-syntax` 0.12.5 | Preserves duplicate entries, order, lexical numbers, and locations | 6-star individual repository and an unmaintained transitive advisory | Reject for production |
| `json-ld` 0.21.4 | Expansion, compaction, flattening, loaders, RDF conversion | 153-star individual repository with concentrated contributors | Reject for production |
| `sophia_jsonld` 0.10.0 | Sophia RDF integration | 325-star parent and wraps older releases of the rejected `json-ld` stack | Reject for production |
| pest 2.8.8 | PEG grammar, spans, `no_std` runtime option, rule-oriented errors | Reimplements JSON syntax only; no JSON-LD or CXF behavior | Rejected for CXF input |
| nom 8 / winnow | Incremental byte parsing control | Custom JSON and JSON-LD work remains | Consider only if true streaming is mandatory |
| chumsky 0.13 | Recovery and generic spans | No JSON-LD behavior | Possible editor-facing source parser, out of scope |

Sources:

- [`serde_json`](https://docs.rs/serde_json/1.0.151/serde_json/)
- [`json-syntax`](https://docs.rs/json-syntax/0.12.5/json_syntax/)
- [`json-ld`](https://docs.rs/json-ld/0.21.4/json_ld/)
- [`oxjsonld`](https://docs.rs/oxjsonld/0.2.5/oxjsonld/)
- [`sophia_jsonld`](https://docs.rs/sophia_jsonld/latest/sophia_jsonld/)
- [pest](https://docs.rs/pest/2.8.8/pest/)
- [nom](https://docs.rs/nom/8.0.0/nom/)
- [chumsky](https://docs.rs/chumsky/0.13.0/chumsky/)

The versions above are the latest observed releases on 2026-08-09. W-024
rechecks latest non-prerelease versions at dispatch and records exact resolution
in `Cargo.lock`.

## Dependency health rule

`FIRST-SLICE.md` defines the production adoption gate. Repository stars are
evaluated at the parent/monorepo level. Passing the star floor does not override
an archive, stale release, unmaintained advisory, incompatible license, target
failure, or concentrated ownership risk.

W-024 qualifies Serde for private-development use after passing lockfile, CI,
license, feature, and advisory gates. Production-release adoption remains gated
by W-023. Oxigraph clears the age, activity, and adoption floors; its contributor
concentration is recorded as R-011 and contained through an internal adapter.
D-022 qualifies the guarded Oxigraph adapter for private CXF ingestion. The
`json-ld` and `json-syntax` stack remains research evidence and must not enter
`Cargo.lock` without a new owner exception.

## Why pest loses this comparison

pest would cover token and nesting recognition. The project would still need
custom implementations for string and Unicode edge cases, duplicate member
policy, number fidelity, an AST, contexts, IRI expansion, RDF conversion,
references, CXF cardinality, connector compatibility, arrays, expressions, and
extensions. This creates more code in the highest-risk layer without reducing
the domain work.

pest remains credible for a later Modelica/CDL source frontend because that is
a programming-language grammar problem. Its current crate is licensed `MIT OR
Apache-2.0`, declares Rust 1.83, can operate with `alloc` when default features
are disabled, exposes byte and line/column errors, and has a parser-call limit.
None of those properties make it a JSON-LD processor.

Evidence:

- [pest manifest](https://raw.githubusercontent.com/pest-parser/pest/master/pest/Cargo.toml)
- [pest runtime](https://raw.githubusercontent.com/pest-parser/pest/master/pest/src/lib.rs)
- [pest call limit](https://docs.rs/pest/2.8.8/pest/fn.set_call_limit.html)
- [RFC 8259 interoperability notes](https://www.rfc-editor.org/rfc/rfc8259.html)

## Processing stages

### 1. Input limits and UTF-8

Check configured byte limits before allocation-heavy processing. The byte API
must reject invalid UTF-8 with a source offset. The string API starts after
host-language conversion and cannot make byte-fidelity claims.

### 2. JSON syntax and source map

Retain the original input bytes. Use Serde to characterize duplicate-member and
number behavior before choosing a public source-fidelity contract. JSON-LD
object names must be unique; the reader must not silently present last-wins data
as source-faithful.

Do not route every number through `f64`. Exact number spelling remains available
in retained source bytes; W-024 records what `serde_json` and `oxjsonld` preserve
through their semantic representations.

### 3. JSON-LD processing

Expand terms and compact IRIs against embedded or supplied contexts. Preserve
RDF datatypes and language tags. Use full IRIs in the graph. Treat RDF
properties as multi-valued unless profile validation establishes cardinality.

Remote loading is disabled unless the caller installs a bounded loader.

### 4. Typed CXF projection

Index known blocks, connectors, parameters, constants, instances, connections,
types, values, units, arrays, expressions, graphics, and FMU references. Keep
unknown terms in the private graph and project them into CXF extension records.
Do not expose processor or RDF graph types.

Project order-sensitive CXF fields from the retained ordered source/DTO layer.
W-003 proves that ordinary JSON-LD array order is absent from the RDF result;
graph iteration order must not define CXF ordering.

### 5. Profile validation

Run versioned rules after graph construction:

- required classes and predicates;
- property cardinality and RDF datatype;
- instance identifier shape;
- reference resolution;
- block membership;
- input/output endpoint class;
- connector datatype compatibility;
- extension block FMU path;
- array representation and explicit order semantics;
- expression representation;
- selected legacy namespace policy.

Native Rust rules should be the initial executable contract. SHACL integration
can be added after the profile stabilizes; current upstream shapes are partial
and inconsistent.

## Diagnostics

Each diagnostic has a stable code, severity, stage, message, primary location,
related locations, JSON Pointer when available, RDF term when available, and
profile identifier.

Proposed families:

| Range | Stage | Example |
|---|---|---|
| `CXF1xxx` | bytes and JSON | duplicate member, invalid UTF-8, nesting limit |
| `CXF2xxx` | JSON-LD | invalid context, unresolved term, disallowed remote load |
| `CXF3xxx` | graph shape | unresolved node, cardinality, RDF datatype |
| `CXF4xxx` | CXF semantics | invalid connection, block membership, FMU path |
| `CXF5xxx` | compatibility | legacy namespace, known exporter defect |

Errors returned through Python and WASM must retain the same code and location
data. Host-specific exception text is not the compatibility contract.

## Resource policy

The parser needs limits for input bytes, nesting, object members, array length,
graph nodes, triples, identifier length, string length, contexts, redirects,
diagnostic count, and rendered excerpts. Graph traversal should be iterative
where practical. Default parsing performs no filesystem or network access.

RFC 8259 permits implementations to set limits on accepted JSON size, nesting,
strings, and numeric ranges:
[RFC 8259 section 9](https://www.rfc-editor.org/rfc/rfc8259.html#section-9).

## Test strategy

- Import the pinned upstream fixture tree with provenance and expected profile.
- Compare RDF semantics, not JSON serialization order.
- Add equivalent documents using alternate prefixes, full IRIs, reordered graph
  nodes, scalar/array forms, and local contexts.
- Add negative documents for duplicates, invalid UTF-8, malformed contexts,
  unresolved references, cardinality, incompatible connectors, deep nesting,
  large values, and legacy namespaces.
- Canonicalize blank-node datasets before cross-serializer equality checks.
- Differentially compare candidate processors and upstream RDF output.
- Fuzz bytes, JSON-LD processing, projection, and validation separately.
- Run the same semantic fixtures through Rust, Python, browser WASM, and Node.
