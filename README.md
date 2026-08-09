# modelica-json-rust

Private Rust project for a future CXF JSON-LD reader and validator, with planned
Python and WebAssembly adapters. The current code is an internal ingestion probe
that parses ordinary JSON and converts a limited owned JSON-LD fixture set to RDF
summaries. It does not implement CXF projection or validation and must not process
untrusted input.

Development is private and pre-release. A future public release will be
available under either the MIT License or Apache License 2.0 after the project's
stability gate is met.
