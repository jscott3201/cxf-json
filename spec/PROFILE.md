# CXF project profile

Profile version: 0.1.1

Status: Core contract with input-byte admission; no parser or conformance profile.

Compatibility impact: Additive input-byte admission. ADR 0005 records the
machine-readable `Additive` classification.

## Scope

Version 0.1.1 defines the owned Rust foundations and input-byte admission used by
later CXF ingestion. It does not define accepted JSON or CXF syntax, a parse entry
point, typed CXF values, extension records, validation rules, JSON-structure or
processing limits, context loading, host serialization, or package-release
stability.

The `cxf-json` package remains unpublished at package version 0.0.0. Its public
contract is governed by this profile even before package publication.

## Public boundary

The crate exports `SourceDocument`, `AdmissionError`, `DocumentIri`,
`DocumentIriError`, `SourcePosition`, `SourceRange`, `DiagnosticCode`,
`DiagnosticSeverity`, `DiagnosticStage`, `Diagnostic`, `ParseError`, and
`ParseOptions`.

Public signatures MUST NOT expose Serde, OxIRI, OxJSONLD, OxRDF, filesystem, HTTP,
Python, JavaScript, or other host-runtime values. Profile 0.1.1 defines no Serde
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
diagnostic by code and structured fields, not by message text. Version 0.1.1
defines no concrete diagnostic codes or emitting behavior.

`ParseError` is an owned envelope for a future admitted source document and one
diagnostic. Version 0.1.1 defines the type and accessors but no function that
constructs or returns it.

## Compatibility

Version 0.1.0 was the first behavior-bearing profile. A breaking pre-1.0 change
MUST increment the minor version and reset the patch version to zero. Additive
behavior or a new public contract MUST advance the patch version and follow
`spec/README.md`.
