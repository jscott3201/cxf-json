# CXF project profile

Profile version: 0.1.2

Status: Core contract with input and JSON-structure options; no public parser or
conformance profile.

Compatibility impact: Additive JSON-structure options. ADR 0006 records the
machine-readable `Additive` classification.

## Scope

Version 0.1.2 defines the owned Rust foundations, input-byte admission, and
structural JSON options used by later CXF ingestion. It does not define accepted
public JSON or CXF syntax, a parse entry point, typed CXF values, extension records,
validation rules, JSON-LD processing limits, context loading, host serialization,
or package-release stability.

The `cxf-json` package remains unpublished at package version 0.0.0. Its public
contract is governed by this profile even before package publication.

## Public boundary

The crate exports `SourceDocument`, `AdmissionError`, `DocumentIri`,
`DocumentIriError`, `SourcePosition`, `SourceRange`, `DiagnosticCode`,
`DiagnosticSeverity`, `DiagnosticStage`, `Diagnostic`, `ParseError`, and
`ParseOptions`.

Public signatures MUST NOT expose Serde, OxIRI, OxJSONLD, OxRDF, filesystem, HTTP,
Python, JavaScript, or other host-runtime values. Profile 0.1.2 defines no Serde
serialization contract for public core types.

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
diagnostic by code and structured fields, not by message text. Version 0.1.2
defines no concrete diagnostic codes or emitting behavior.

`ParseError` is an owned envelope for a future admitted source document and one
diagnostic. Version 0.1.2 defines the type and accessors but no function that
constructs or returns it.

## Compatibility

Version 0.1.0 was the first behavior-bearing profile. A breaking pre-1.0 change
MUST increment the minor version and reset the patch version to zero. Additive
behavior or a new public contract MUST advance the patch version and follow
`spec/README.md`.
