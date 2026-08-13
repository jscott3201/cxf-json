# CXF project profile

Profile version: 0.1.7

Status: Core contract with input, JSON-structure, RDF output options, a
private typed CXF projection stage, a private validator core, and a private
namespace acceptance policy; no supported public parser or conformance
profile.

Compatibility impact: Additive namespace acceptance policy without public
behavior. ADR 0011 records the machine-readable `Additive` classification.

## Scope

Version 0.1.7 defines the owned Rust foundations, input-byte admission, structural
JSON options, RDF output-retention options, a private typed CXF projection
stage, a private validator core emitting stable rule codes, and a private
namespace acceptance matrix classifying declared context mappings. It does not
define accepted public JSON or CXF syntax, a supported parse entry point, typed
CXF values, extension records, public validation rules or codes, public
JSON-LD processing behavior, context loading, host serialization, or
package-release stability.

The `cxf-json` package remains unpublished at package version 0.0.0. Its public
contract is governed by this profile even before package publication.

## Public boundary

The crate exports `SourceDocument`, `AdmissionError`, `DocumentIri`,
`DocumentIriError`, `SourcePosition`, `SourceRange`, `DiagnosticCode`,
`DiagnosticSeverity`, `DiagnosticStage`, `Diagnostic`, `ParseError`, and
`ParseOptions`.

Public signatures MUST NOT expose Serde, OxIRI, OxJSONLD, OxRDF, filesystem, HTTP,
Python, JavaScript, or other host-runtime values. Profile 0.1.7 defines no Serde
serialization contract for public core types.

Normal package and documentation builds export only the types listed above. A
project-controlled build with `cfg(fuzzing)` or `cfg(cxf_json_semantic_harness)`
also exports the doc-hidden `test_support` observation module. That module is not a
supported package API. It returns project-owned outcome and metric values without
returning source bytes, backend diagnostics, or RDF values.

## Source bytes

`SourceDocument::from_bytes` MUST take ownership of the supplied `Vec<u8>` without
changing its contents. `SourceDocument::as_bytes` MUST return those exact bytes.
Construction does not validate UTF-8, JSON, JSON-LD, or CXF.

`SourceDocument` debug output MUST report the byte length without including the
document bytes. This prevents routine debug formatting from copying input content
into logs.

## Input-byte admission

`ParseOptions::DEFAULT_MAX_INPUT_BYTES` MUST be 1,048,576 bytes.
`ParseOptions::new` and `ParseOptions::default` MUST use that limit.
`ParseOptions::with_max_input_bytes` MUST replace it with the supplied unsigned
64-bit value, including zero. The limit is inclusive.

`SourceDocument::admit_bytes` MUST compare the borrowed input byte length with the
configured limit before retaining source bytes. Input at or below the limit MUST
produce a `SourceDocument` containing one exact owned copy. Input above the limit
MUST return `AdmissionError` without retaining a source copy. Admission checks no
UTF-8, JSON, JSON-LD, or CXF syntax.

`AdmissionError` MUST expose the submitted byte count and configured limit as
unsigned 64-bit values. It MUST NOT contain a `SourceDocument`, source range, JSON
Pointer, RDF term, or input-derived message text. Its debug and display output
therefore reveal only the two byte counts and fixed explanatory text.

`SourceDocument::from_bytes` remains a raw ownership constructor and does not
apply `ParseOptions`.

## JSON-structure options

The structural limits are inclusive unsigned 64-bit values. Every option accepts
zero. Profile 0.1.2 defines the options for the future CXF parse path but no public
function that applies them.

`ParseOptions::DEFAULT_MAX_JSON_NESTING_DEPTH` MUST be 64. A root container has
depth 1; a scalar root has depth 0. A zero limit therefore permits scalar roots but
no array or object.

`ParseOptions::DEFAULT_MAX_JSON_OBJECT_MEMBERS` MUST be 4,096. Members are counted
independently in each object. A zero limit permits an empty object but rejects its
first member.

`ParseOptions::DEFAULT_MAX_JSON_VALUES` MUST be 65,536. Every scalar, array, and
object, including the root, counts as one value. A zero limit rejects every JSON
value.

`ParseOptions::DEFAULT_MAX_DECODED_MEMBER_NAME_BYTES` MUST be 262,144. The count is
the total UTF-8 byte length of decoded member names, counted once per occurrence.
A zero limit permits empty member names but rejects any nonempty decoded name.

`ParseOptions::new` and `ParseOptions::default` MUST use all four defaults. The
corresponding `with_max_*` methods MUST replace one limit without changing the
others, and each accessor MUST return the configured value.

## RDF output options

The RDF output limits are inclusive unsigned 64-bit values. Every option accepts
zero. Profile 0.1.3 defines these options for private semantic ingestion but no
public function that applies them.

`ParseOptions::DEFAULT_MAX_RDF_QUADS` MUST be 65,536. Each quad emitted by the
JSON-LD processor counts before graph deduplication, including repeated identical
quads. A zero limit permits a result that emits no quads and rejects the first
emitted quad.

`ParseOptions::DEFAULT_MAX_RETAINED_RDF_TERM_BYTES` MUST be 8,388,608. The count is
the UTF-8 byte length of each owned occurrence of the subject, predicate, object,
literal datatype, optional language tag, and non-default graph name. Repeated
strings and processor-generated blank-node identifiers count each time they occur.
A zero limit permits only a result that retains no RDF term bytes.

`ParseOptions::new` and `ParseOptions::default` MUST use both defaults.
`ParseOptions::with_max_rdf_quads` and
`ParseOptions::with_max_retained_rdf_term_bytes` MUST replace one limit without
changing any other option. Their accessors MUST return the configured values.

For a quad that would exceed both limits, the quad limit takes precedence. Counter
and byte-total overflow is the corresponding limit failure. Limits are checked
before the emitted quad is retained in project output. They do not bound OxJSONLD
temporary allocations, internal diagnostic buffers, process memory, or execution
time.

## Private semantic feature

The package's default `semantic-ingestion` feature enables optional OxJSONLD and
OxRDF dependencies plus the target-only `getrandom` `wasm_js` feature required by
anonymous-node processing on `wasm32-unknown-unknown`. Disabling default features
MUST omit those three optional dependencies and retain the source and JSON
preflight contract.

The feature alone exposes no supported parse function or backend type. Its private
semantic path requires a configured `DocumentIri`, installs no remote loader,
returns at most one fixed project failure, and discards a partial graph on failure.
The conditional instrumentation module may observe this path in project fuzz,
native-report, and Node/WASM smoke builds. It is not a supported package API. The
one returned failure does not bound OxJSONLD's internal diagnostic allocation. The
target-only entropy exception remains blocked from package release under D-021.

## Document IRI

`DocumentIri::parse` MUST accept an absolute RFC 3987 IRI and MUST reject a
relative IRI. A successful value MUST retain the submitted spelling without
normalization. Failure MUST return the project-owned `DocumentIriError`, not an
OxIRI error value. `DocumentIri` debug output MUST redact the IRI spelling;
`ParseOptions` debug output inherits that redaction. `DocumentIri::as_str` and
display formatting return the exact spelling when a caller requests it.

`ParseOptions::new` and `ParseOptions::default` MUST contain no document IRI.
`ParseOptions::with_document_iri` MUST retain the supplied validated
`DocumentIri`. Option fields remain private.

## Source locations

`SourcePosition` fields are unsigned 64-bit values. Offset and column units are
bytes. Offset, line, and column are zero-based; line counts preceding line-feed
bytes, and column counts bytes since the most recent line-feed byte.

`SourceRange` is half-open: start is inclusive and end is exclusive. Equal start
and end positions represent a detection position. Construction MUST reject an
end offset before the start offset. The value constructors do not verify that
positions belong to a particular source document or that callers supplied a
consistent line/column triple.

## Diagnostics and errors

Diagnostic severity values are `Warning` and `Error`. Diagnostic stages are
`Input`, `Json`, `JsonLd`, `Cxf`, and `Profile`. Both enums are non-exhaustive.

`Diagnostic` provides a project-owned code, severity, stage, human-readable
message, optional byte range, optional JSON Pointer, and optional RDF-term
evidence. Range, pointer, and RDF-term evidence are independent; the presence of
one does not imply either other value exists. Consumers MUST match a future
diagnostic by code and structured fields, not by message text. Version 0.1.3
defines no concrete diagnostic codes or public emitting behavior.

`ParseError` is an owned envelope for a future admitted source document and one
diagnostic. Version 0.1.3 defines the type and accessors but no public function that
constructs or returns it.

## Private typed projection

The crate's private typed CXF projection consumes the retained ordered source
view and classifies the OBC section 8.2 core vocabulary into owned structures.
It MUST NOT add a public type, parse entry point, validation rule, or Serde
contract, and therefore appears in no public signature or documentation build.

The projection registers vocabulary terms by full internal IRI identity across
four registered identities: the distinct `http://data.ashrae.org/S231#`,
`http://data.ashrae.org/S231P#`, and `https://data.ashrae.org/S231P#` namespace
generations, plus `http://qudt.org/schema/qudt#` for the two unit predicates the
emitter writes there (`hasUnit`, `hasQuantityKind`). Registration is per-identity
allowlisted, not a global term-by-namespace cross-product: the S231 generations
register the S231 surface except the QUDT unit predicates, and the QUDT identity
registers only its two predicates. Distinct IRIs, including the `connectedTo`
and `isConnectedTo` spellings, MUST keep distinct internal identity; the
projection MUST NOT normalize, percent-decode, or merge them. Compacted
`prefix:local` spellings MUST register only when the document's own `@context`
maps the prefix to a registered namespace IRI; the projection MUST NOT implement
a partial JSON-LD context expander. Unit references (`qudt:hasUnit`,
`S231:hasDisplayUnit`, `qudt:hasQuantityKind`) carry verbatim target spellings
classified as QUDT unit IRIs, QUDT quantity-kind IRIs, S231-generation emitter
fallbacks, or other; the projection MUST NOT normalize or resolve unit targets.

Instance references MUST resolve only by exact authored-string equality. The
emitter attribute and annotation surface — `start`, `nominal`, `fixed`,
`instantiate`, `min`, `max`, `defaultValue`, `generatePointlist`,
`controlledDevice`, `graphics`, and `conditionalExpression` — indexes verbatim:
`graphics` and `conditionalExpression` strings keep the opaque C-005/C-006
posture as indexed text, attribute values stay opaque CXF values, and
`generatePointlist` and `fixed` stay booleans. Unknown predicates, unrecognized
spellings, and wrong-shaped members MUST degrade into verbatim CXF extension
records, never into silence, guesses, or process failure. CXF values stay
opaque: strings are retained decoded, while exact number and boolean spelling
remains available only in the retained source bytes that the projection owns.

Projection findings, including weakly typed nodes, conflicting type
assertions, known broken-emitter value artifacts, malformed references,
unresolved references, and duplicate node identifiers, use the private
`CXF-P-000` through `CXF-P-006` codes. Version 0.1.7 keeps these codes private;
W-014 owns any future public stabilization.

## Private validator core

The crate's private validator consumes the projection without discarding the
parsed CXF document. Rules combine OBC specification Table 8.2 constraints
with the frozen register postures C-008, C-009, and C-015: an error fires
only when a node's registered class is provably out of domain; unknown
(weakly typed) or library-typed classes are never treated as disproven.
Absent `value` members are surfaced and never rejected. The private code
allocation is normative:

| Code | Rule | Severity |
|---|---|---|
| `CXF-V-001` | A connection endpoint resolves to a node whose class is provably non-connector (Package, Block, Parameter, Constant, EnumerationType, DataType, or String) | Error |
| `CXF-V-002` | Both connection endpoints are connectors whose statically known datatypes disagree (unknown never disagrees) | Error |
| `CXF-V-003` | `isOfDataType` is authored on a node whose class is provably outside the connector/parameter/constant domain | Error |
| `CXF-V-004` | A grouping predicate (`hasInput`/`hasOutput`/`hasParameter`/`hasConstant`) is authored on a node whose class is provably non-block | Error |
| `CXF-V-005` | A Parameter or Constant has no `value` property (C-009) | Warning |

Findings reuse the public `DiagnosticSeverity` type, carry the rule code,
node index (absent for root-level policy findings), and the source token of
the evidenced member, and MUST order deterministically by token start, then
node index, then rule-code ordinal. Rules that never fire include anything
whose evidence depends on weakly typed or library-typed nodes. Version
0.1.7 keeps `CXF-V-*` codes private; W-016 owns negative-corpus coverage,
and a later profile decides any public stabilization.

## Private namespace acceptance policy

For each declared root `@context` prefix mapping the projection retains one
final binding per prefix (last-write-wins across context arrays; activation
and policy observations can never disagree because activations are computed
from the retained bindings only). The validator classifies each retained
binding exactly once. The acceptance matrix is observational: bindings
classify and may diagnose, but no namespace binding is rejected at this
stage; terms under unregistered namespaces already stay verbatim extension
evidence with distinct identity (C-001/C-002/C-016). JSON-LD keyword members
(`@base`, `@vocab`, `@language`, ...) are not prefix bindings and MUST NOT
produce observations.

| Declared mapping | Acceptance | Finding |
|---|---|---|
| `http://data.ashrae.org/S231#` | Accepted, registered identity | none |
| `http://data.ashrae.org/S231P#` | Accepted, registered identity (C-016: the `S231` prefix legitimately maps here in post-v1.2 output) | none |
| `https://data.ashrae.org/S231P#` | Accepted as observed legacy; identity kept distinct (C-002) | `CXF-C-001` Warning |
| `http://qudt.org/schema/qudt#` | Accepted for the two unit predicates (ADR 0009) | none |
| `http://qudt.org/vocab/unit#`, `http://qudt.org/vocab/quantitykind#` | Accepted as target-classification buckets | none |
| Unregistered namespace under a known family host (`data.ashrae.org`, `qudt.org`) | Observed: possible new generation variant | `CXF-C-002` Warning |
| Other foreign namespaces (for example `ex` data namespaces) | Observed silently; misuse already lands in extension records | none |
| A registered prefix (`S231`, `S231P`, `qudt`, `unit`, `q`) bound to an unexpected namespace | Shadowed: the binding cannot serve its conventional purpose | `CXF-C-003` Warning |

Prefix expectations are `S231` → any registered S231 generation,
`S231P` → S231P generations, `qudt` → the QUDT schema namespace, and
`unit`/`q` → the matching QUDT vocab namespace. Rows are evaluated
shadow-first: a shadowed known-prefix binding emits `CXF-C-003` only.
All policy findings are warnings because processing continues and the
document is retained; rejection behavior remains concentrated in W-011
preflight. `CXF-C-*` codes follow the same ordering, evidence, and
severity-type rules as `CXF-V-*` and stay private in 0.1.7; full-IRI term
spellings used without a context binding do not diagnose at the term level
in this slice.

Predicate-variance policy, completing the OQ-004 matrix: distinct spec and
emitter predicate spellings — `connectedTo` versus `isConnectedTo`, both
namespace generations, and QUDT identities — are accepted as authored and
retain distinct internal identity. Recognition MUST NOT assert that
distinct IRIs are globally equivalent. That acceptance is the register's
C-001/C-002/C-016 discipline, now stated as policy rather than behavior
alone.

## Compatibility

Version 0.1.0 was the first behavior-bearing profile. A breaking pre-1.0 change
MUST increment the minor version and reset the patch version to zero. Additive
behavior or a new public contract MUST advance the patch version and follow
`spec/README.md`.
