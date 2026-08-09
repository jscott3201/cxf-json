# W-004: Source-fidelity contract

Status: implementation complete on `w004-source-fidelity`; PR creation, review,
and merge are pending.

## Purpose

Operationalize D-014 without adding a package or promising a successful-node
source map. A narrow project-owned lexical preflight validates JSON grammar and
decoded member-name uniqueness without building an AST. The v1 boundary retains
the submitted document and available error locations. Typed CXF and private RDF
remain semantic views, not lossless source representations.

## Contract

| Concern | V1 behavior |
|---|---|
| Submitted bytes | After input-size admission, retain the exact byte sequence on success and every failure path |
| Duplicate names | Reject repeated decoded member names in every JSON object before JSON-LD processing |
| Number spelling | Retain exact spelling only in the submitted bytes; `serde_json::Value`, typed CXF values, and RDF literals make no lexical-fidelity promise |
| Error locations | Use a zero-based byte offset as authority; invalid UTF-8 points to the first invalid byte, while parser syntax errors report detection positions |
| Ranges | Use half-open byte ranges; a parser detection point has equal start and end and need not identify the offending token's full span |
| JSON Pointer | Include one only when project traversal already knows an unambiguous syntax path; do not reconstruct it from an offset |
| RDF term | Record optional semantic term evidence independently of a source range or JSON Pointer |
| Source-to-RDF linkage | Do not promise pointer-to-quad, span-to-term, or stable source-to-blank-node provenance |

Duplicate comparison occurs after JSON string escape decoding. For example,
`"a"` and `"\u0061"` conflict. No Unicode normalization is applied, so
canonically equivalent but differently encoded strings remain distinct names.
JSON Pointer evaluation against duplicate object names is undefined by RFC 6901;
duplicate failures therefore do not expose a pointer.

The line model follows JSON parsing rather than platform text conventions. A
line-feed byte starts a new line. A preceding carriage return remains part of
the prior line. Exposed ranges are clamped to the submitted byte sequence.

The preflight uses an explicit stack and adds no nesting limit. Downstream Serde
and OxJSONLD stages retain their own current capabilities. W-011 must define and
enforce one production limit before allocation-heavy processing.

## Result boundary

| Outcome | Document returned | Submitted bytes retained |
|---|---:|---:|
| Successful CXF ingestion | Yes | Yes |
| Profile validation rejection | Yes | Yes |
| Invalid UTF-8 or malformed JSON | No | Yes |
| Duplicate object member | No | Yes |
| JSON-LD processing failure | No | Yes |
| Input-size policy failure | No | No owned copy; the caller still owns the submitted buffer |

W-011 must reject oversized input before retaining another owned copy. The input
limit is the sole retention exception and this work does not set that limit.

## Why no source mapper

The current approved dependencies do not provide a lossless tree with per-node
spans. `json-syntax` has the needed mechanics but fails D-011. The project-owned
preflight stops at grammar and decoded member names. It discards tokens and
successful-value positions and still delegates ordinary values and JSON-LD
semantics to Serde and OxJSONLD. The retained document remains the lexical
authority; source-aware editor or rewrite requirements need a separate work item
and dependency ruling.

JSON-LD output alone cannot provide a one-to-one source-to-RDF provenance join.
Aliases, merged node descriptions, scalar and array spellings, ordinary
unordered arrays, generated list nodes, and anonymous nodes can map source forms
to RDF many-to-one or one-to-many. Named full IRIs may join typed CXF records to
private graph indexes, but that semantic identity is not source provenance. A
future instrumented processor could define a separate many-to-many provenance
relation under a new work item.

## Acceptance gates

1. Ordinary JSON and JSON-LD reject duplicate decoded names at the root, nested
   in objects and arrays, and inside `@context`.
2. Success and failure tests compare retained bytes with the submitted slice.
3. Number tests show lexical spellings in source and their normalized Serde/RDF
   representations.
4. Invalid UTF-8, multibyte UTF-8 before a syntax error, LF, CRLF, and EOF cases
   pin byte positions and point ranges. An earlier large exponent does not mask
   the first invalid UTF-8 byte.
5. Owned diagnostics round-trip optional range, JSON Pointer, and RDF-term fields
   without implying a relationship among them.
6. The owned W-003 matrix and pinned read-only corpus still pass under duplicate
   preflight; any duplicate becomes an explicit compatibility result.
7. Large exponents and 256 nested arrays pass duplicate preflight without `f64`
   conversion or an inherited recursive visitor limit.
8. Native, Serde-only, all-feature, and Node-executed WASM gates pass without a
   package change.

## Downstream constraints

- W-006 owns concrete public type names and may expose submitted bytes, optional
  parser ranges, and optional pointers without exposing probe or RDF types.
- W-007 must feed the same duplicate-checked bytes to the ordered CXF DTO and
  private RDF stages. Named identity may join those views; RDF iteration cannot
  establish source order.
- W-010 owns host-boundary parity for bytes and diagnostic fields.
- W-011 owns input, nesting, member-count, and diagnostic limits.
- W-013 and W-014 may attach independent source and semantic evidence to typed
  CXF records and diagnostics. They must not imply per-quad provenance.

## Standards

- [RFC 8259 object-name interoperability](https://www.rfc-editor.org/rfc/rfc8259.html#section-4)
- [RFC 6901 JSON Pointer syntax and evaluation](https://www.rfc-editor.org/rfc/rfc6901.html)
- [JSON-LD 1.1 JSON object definition](https://www.w3.org/TR/json-ld11/#dfn-json-object)
- [JSON-LD 1.1 arrays](https://www.w3.org/TR/json-ld11/#sets-and-lists)
- [OBC Control eXchange Format](https://obc.lbl.gov/specification/cxf.html)
