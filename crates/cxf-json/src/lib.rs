#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
//! Owned contract foundations for CXF JSON.
//!
//! This crate does not yet parse CXF. Profile 0.1.2 defines input-byte admission,
//! structural JSON options, and source, location, diagnostic, and error types for
//! later ingestion work without exposing JSON-LD, RDF, Serde, or host-runtime
//! values.

mod contract;
// W-007 consumes this production seam; M1-C5 keeps it private until then.
#[allow(dead_code)]
mod json;

pub use contract::{
    AdmissionError, Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticStage, DocumentIri,
    DocumentIriError, ParseError, ParseOptions, SourceDocument, SourcePosition, SourceRange,
};
