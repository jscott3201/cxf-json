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

/// Structural JSON measurements produced by the duplicate-name preflight.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonStructureMetrics {
    /// Maximum simultaneous open arrays and objects; a root container has depth 1.
    pub max_nesting_depth: usize,
    /// Maximum decoded member-name/value pairs in one object.
    pub max_object_members: usize,
    /// Scalars, arrays, and objects encountered, including the root value.
    pub total_values: usize,
    /// UTF-8 bytes in decoded object member names, counted once per occurrence.
    pub decoded_member_name_bytes: usize,
}

/// Structural, stage-time, and retained-graph measurements for one probe run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProbeMetrics {
    pub json: JsonStructureMetrics,
    /// UTF-8 bytes retained by owned RDF summary strings, including repetition.
    pub rdf_term_bytes: usize,
}

/// Native stage timing produced only by the benchmark wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbeTiming {
    /// Duplicate-name preflight time, including failed preflights.
    pub preflight_micros: u128,
    /// JSON-LD/RDF stage time when preflight completed successfully.
    pub json_ld_micros: Option<u128>,
}

/// Deterministic parse result paired with benchmark-only native timing.
#[derive(Debug)]
pub struct MeasuredProbe {
    pub result: Result<ProbeReport, ProbeFailure>,
    pub timing: ProbeTiming,
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
    pub metrics: Option<Box<ProbeMetrics>>,
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
    pub metrics: ProbeMetrics,
}
