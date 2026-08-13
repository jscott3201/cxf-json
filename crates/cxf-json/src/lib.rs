#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
//! Owned contract foundations for CXF JSON.
//!
//! Normal builds do not expose a supported CXF parser. Profile 0.1.7 defines
//! input-byte, structural JSON, and private RDF output options plus source,
//! location, diagnostic, and error types without exposing JSON-LD, RDF, Serde, or
//! host-runtime values. Explicit project instrumentation builds also expose a
//! doc-hidden observation module.

mod contract;
// These stages stay private until W-007 defines a supported parse boundary.
#[allow(dead_code)]
mod json;
#[allow(dead_code)]
mod ordered;
#[allow(dead_code)]
mod projection;
#[cfg(feature = "semantic-ingestion")]
#[allow(dead_code)]
mod semantic;
#[cfg(all(
    feature = "semantic-ingestion",
    any(test, fuzzing, cxf_json_semantic_harness)
))]
// This unsupported surface exists only for explicit project instrumentation builds.
#[doc(hidden)]
#[allow(missing_docs)]
pub mod test_support;
#[allow(dead_code)]
mod validation;

pub use contract::{
    AdmissionError, Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticStage, DocumentIri,
    DocumentIriError, ParseError, ParseOptions, SourceDocument, SourcePosition, SourceRange,
};
