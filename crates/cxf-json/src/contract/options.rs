use super::DocumentIri;

/// Host-neutral options reserved for the future CXF parse entry point.
///
/// Fields are private so W-011 can add resource limits without permitting
/// external struct literals to bypass defaults.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParseOptions {
    document_iri: Option<DocumentIri>,
}

impl ParseOptions {
    /// Creates options with no document IRI.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns options configured with an absolute document IRI.
    #[must_use]
    pub fn with_document_iri(mut self, document_iri: DocumentIri) -> Self {
        self.document_iri = Some(document_iri);
        self
    }

    /// Returns the configured document IRI, if any.
    #[must_use]
    pub fn document_iri(&self) -> Option<&DocumentIri> {
        self.document_iri.as_ref()
    }
}
