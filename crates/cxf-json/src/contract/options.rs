use super::DocumentIri;

/// Host-neutral options reserved for the future CXF parse entry point.
///
/// Fields are private so external struct literals cannot bypass profile defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseOptions {
    document_iri: Option<DocumentIri>,
    max_input_bytes: u64,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            document_iri: None,
            max_input_bytes: Self::DEFAULT_MAX_INPUT_BYTES,
        }
    }
}

impl ParseOptions {
    /// Default maximum admitted input size in bytes.
    pub const DEFAULT_MAX_INPUT_BYTES: u64 = 1_048_576;

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

    /// Returns options with the inclusive input-size limit set to `max_input_bytes`.
    #[must_use]
    pub fn with_max_input_bytes(mut self, max_input_bytes: u64) -> Self {
        self.max_input_bytes = max_input_bytes;
        self
    }

    /// Returns the inclusive input-size limit in bytes.
    #[must_use]
    pub const fn max_input_bytes(&self) -> u64 {
        self.max_input_bytes
    }
}
