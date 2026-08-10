# CXF JSON

Private Rust project for a purpose-built Control eXchange Format (CXF) JSON-LD
parser and validator, with planned Python and WebAssembly adapters. JSON-LD and
RDF processing are internal CXF ingestion details, not general-purpose APIs. The
current code is an internal probe; it does not implement CXF projection or
validation and must not process untrusted input.

The intended primary Rust package is `cxf-json` (`cxf_json` in Rust code). It
does not exist yet; the workspace currently contains only the unpublished
`cxf-ingest-probe` evidence crate.

Development is private and pre-release. A future public release will be
available under either the MIT License or Apache License 2.0 after the project's
stability gate is met.
