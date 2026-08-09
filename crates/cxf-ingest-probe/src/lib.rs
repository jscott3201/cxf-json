//! Internal evidence probe for the CXF JSON-LD ingestion boundary.
//!
//! This crate intentionally omits production resource limits and must not parse
//! untrusted input.

mod dto;
mod json;

#[cfg(feature = "oxigraph")]
mod oxigraph;

pub use dto::{
    DiagnosticStage, ProbeDiagnostic, ProbeFailure, ProbeReport, RdfNodeKind, RdfNodeSummary,
    RdfObjectSummary, RdfQuadSummary, SourceDocument, SourcePosition, SourceRange,
};
pub use json::{JsonDocument, parse_json};

#[cfg(feature = "oxigraph")]
pub use oxigraph::parse_json_ld;
