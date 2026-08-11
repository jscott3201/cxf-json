#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
//! Owned contract foundations for CXF JSON.
//!
//! This crate does not yet parse CXF. Profile 0.1.1 defines input-byte admission
//! and source, location, diagnostic, error, and option types for later ingestion
//! work without exposing JSON-LD, RDF, Serde, or host-runtime values.

mod contract;

pub use contract::{
    AdmissionError, Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticStage, DocumentIri,
    DocumentIriError, ParseError, ParseOptions, SourceDocument, SourcePosition, SourceRange,
};
