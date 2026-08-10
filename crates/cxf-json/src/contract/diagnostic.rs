use std::{error::Error, fmt};

use super::{SourceDocument, SourceRange};

/// Severity of a project-owned diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// Processing can continue and retain a document.
    Warning,
    /// Processing cannot satisfy the active contract.
    Error,
}

/// Processing stage that produced a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DiagnosticStage {
    /// Input admission or byte handling.
    Input,
    /// JSON syntax processing.
    Json,
    /// JSON-LD processing.
    JsonLd,
    /// CXF semantic construction.
    Cxf,
    /// Versioned profile validation.
    Profile,
}

/// Stable machine-readable diagnostic code.
///
/// Profile 0.1.0 defines the container but no concrete codes. Later profiles
/// introduce codes with the behavior that emits them.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiagnosticCode(Box<str>);

impl DiagnosticCode {
    /// Returns the diagnostic code text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Owned diagnostic data independent of parser and host-runtime types.
///
/// The code and structured fields are the machine-readable contract. Message
/// text is intended for people and is not stable matching input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    stage: DiagnosticStage,
    message: Box<str>,
    range: Option<SourceRange>,
    pointer: Option<Box<str>>,
    rdf_term: Option<Box<str>>,
}

impl Diagnostic {
    /// Returns the stable machine-readable code.
    #[must_use]
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the diagnostic severity.
    #[must_use]
    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns the processing stage.
    #[must_use]
    pub fn stage(&self) -> DiagnosticStage {
        self.stage
    }

    /// Returns the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the authoritative byte range, if one is available.
    #[must_use]
    pub fn range(&self) -> Option<SourceRange> {
        self.range
    }

    /// Returns syntax-level JSON Pointer evidence, if available.
    #[must_use]
    pub fn pointer(&self) -> Option<&str> {
        self.pointer.as_deref()
    }

    /// Returns independent RDF-term evidence, if available.
    #[must_use]
    pub fn rdf_term(&self) -> Option<&str> {
        self.rdf_term.as_deref()
    }
}

/// Future parse failure that retains admitted source bytes and one diagnostic.
///
/// Profile 0.1.0 defines this envelope but no function that returns it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    source: SourceDocument,
    diagnostic: Diagnostic,
}

impl ParseError {
    /// Returns the exact admitted source bytes associated with the failure.
    #[must_use]
    pub fn source_document(&self) -> &SourceDocument {
        &self.source
    }

    /// Returns the structured failure diagnostic.
    #[must_use]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.message.fmt(formatter)
    }
}

impl Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourcePosition, SourceRange};

    #[test]
    fn diagnostic_evidence_is_independent() {
        let range = SourceRange::new(SourcePosition::new(4, 0, 4), SourcePosition::new(4, 0, 4))
            .expect("equal positions form a detection range");
        let diagnostic = Diagnostic {
            code: DiagnosticCode("CXF0001".into()),
            severity: DiagnosticSeverity::Error,
            stage: DiagnosticStage::Json,
            message: "example".into(),
            range: Some(range),
            pointer: Some("/value".into()),
            rdf_term: Some("https://example.test/value".into()),
        };
        let error = ParseError {
            source: SourceDocument::from_bytes(b"input".to_vec()),
            diagnostic,
        };

        assert_eq!(error.to_string(), "example");
        assert_eq!(error.source_document().as_bytes(), b"input");
        assert_eq!(error.diagnostic().code().as_str(), "CXF0001");
        assert_eq!(error.diagnostic().severity(), DiagnosticSeverity::Error);
        assert_eq!(error.diagnostic().stage(), DiagnosticStage::Json);
        assert_eq!(error.diagnostic().range(), Some(range));
        assert_eq!(error.diagnostic().pointer(), Some("/value"));
        assert_eq!(
            error.diagnostic().rdf_term(),
            Some("https://example.test/value")
        );
    }
}
