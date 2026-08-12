# CXF project profile

Profile version: 0.1.4

Status: Core contract with input, JSON-structure, RDF output options, and a
private typed CXF projection stage; no supported public parser or conformance
profile.

Compatibility impact: Additive private typed-projection module without public
behavior. ADR 0008 records the machine-readable `Additive` classification.

## Scope

Version 0.1.4 defines the owned Rust foundations, input-byte admission, structural
JSON options, RDF output-retention options, and a private typed CXF projection
stage used by later CXF ingestion. It does not define accepted public JSON or CXF
syntax, a supported parse entry point, typed CXF values, extension records,
validation rules, public JSON-LD processing behavior, context loading, host
serialization, or package-release stability.

The `cxf-json` package remains unpublished at package version 0.0.0. Its public
contract is governed by this profile even before package publication.

## Public boundary

The crate exports `SourceDocument`, `AdmissionError`, `DocumentIri`,
`DocumentIriError`, `SourcePosition`, `SourceRange`, `DiagnosticCode`,
`DiagnosticSeverity`, `DiagnosticStage`, `Diagnostic`, `ParseError`, and
`ParseOptions`.

Public signatures MUST NOT expose Serde, OxIRI, OxJSONLD, OxRDF, filesystem, HTTP,
Python, JavaScript, or other host-runtime values. Profile 0.1.4 defines no Serde
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
the distinct `http://data.ashrae.org/S231#`, `http://data.ashrae.org/S231P#`,
and `https://data.ashrae.org/S231P#` namespace generations. Distinct IRIs,
including the `connectedTo` and `isConnectedTo` spellings, MUST keep distinct
internal identity; the projection MUST NOT normalize, percent-decode, or merge
them. Compacted `prefix:local` spellings MUST register only when the document's
own `@context` maps the prefix to a registered namespace IRI; the projection
MUST NOT implement a partial JSON-LD context expander.

Instance references MUST resolve only by exact authored-string equality.
Unknown predicates, unrecognized spellings, wrong-shaped members, and graphics
payloads MUST degrade into verbatim CXF extension records, never into silence,
guesses, or process failure. CXF values stay opaque: strings are retained
decoded, while exact number and boolean spelling remains available only in the
retained source bytes that the projection owns.

Projection findings, including weakly typed nodes, conflicting type
assertions, known broken-emitter value artifacts, malformed references,
unresolved references, and duplicate node identifiers, use the private
`CXF-P-000` through `CXF-P-006` codes. Version 0.1.4 keeps these codes private;
W-014 owns any future public stabilization.

## Compatibility

Version 0.1.0 was the first behavior-bearing profile. A breaking pre-1.0 change
MUST increment the minor version and reset the patch version to zero. Additive
behavior or a new public contract MUST advance the patch version and follow
`spec/README.md`.
