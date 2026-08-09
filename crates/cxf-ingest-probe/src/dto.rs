use std::fmt;

use serde::{Deserialize, Serialize};

/// Submitted input bytes retained exactly on success and failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceDocument {
    bytes: Vec<u8>,
}

impl SourceDocument {
    /// Copies submitted input into an owned source document.
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }

    /// Returns the original submitted bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Zero-based byte position in submitted input.
///
/// `column` counts bytes since the most recent line-feed byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourcePosition {
    pub offset: u64,
    pub line: u64,
    pub column: u64,
}

/// Half-open byte range in submitted input.
///
/// A parser detection position has equal start and end positions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

/// Processing layer that produced a diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticStage {
    Json,
    JsonLd,
}

/// Owned diagnostic data shared by native and future host adapters.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProbeDiagnostic {
    pub stage: DiagnosticStage,
    pub message: String,
    pub range: Option<SourceRange>,
    /// Syntax-level path when project traversal identifies one unambiguously.
    pub pointer: Option<String>,
    /// Semantic term evidence independent of `range` and `pointer`.
    pub rdf_term: Option<String>,
}

/// Input failure that retains the source bytes and structured diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProbeFailure {
    pub source: SourceDocument,
    pub diagnostic: Box<ProbeDiagnostic>,
}

impl fmt::Display for ProbeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.message.fmt(formatter)
    }
}

impl std::error::Error for ProbeFailure {}

/// RDF node category retained without exposing Oxigraph types.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RdfNodeKind {
    Named,
    Blank,
}

/// Owned RDF subject or graph-name summary.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RdfNodeSummary {
    pub kind: RdfNodeKind,
    pub value: String,
}

/// Owned RDF object summary.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RdfObjectSummary {
    Node(RdfNodeSummary),
    Literal {
        value: String,
        datatype: String,
        language: Option<String>,
    },
    Other(String),
}

/// Owned RDF quad summary used by the ingestion probe.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RdfQuadSummary {
    pub subject: RdfNodeSummary,
    pub predicate: String,
    pub object: RdfObjectSummary,
    pub graph_name: Option<RdfNodeSummary>,
}

/// Serializable probe result used to test future host-language boundaries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProbeReport {
    pub source: SourceDocument,
    pub diagnostics: Vec<ProbeDiagnostic>,
    pub quads: Vec<RdfQuadSummary>,
}
