use super::DocumentIri;

/// Host-neutral options reserved for the future CXF parse entry point.
///
/// Fields are private so external struct literals cannot bypass profile defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseOptions {
    document_iri: Option<DocumentIri>,
    max_input_bytes: u64,
    max_json_nesting_depth: u64,
    max_json_object_members: u64,
    max_json_values: u64,
    max_decoded_member_name_bytes: u64,
    max_rdf_quads: u64,
    max_retained_rdf_term_bytes: u64,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            document_iri: None,
            max_input_bytes: Self::DEFAULT_MAX_INPUT_BYTES,
            max_json_nesting_depth: Self::DEFAULT_MAX_JSON_NESTING_DEPTH,
            max_json_object_members: Self::DEFAULT_MAX_JSON_OBJECT_MEMBERS,
            max_json_values: Self::DEFAULT_MAX_JSON_VALUES,
            max_decoded_member_name_bytes: Self::DEFAULT_MAX_DECODED_MEMBER_NAME_BYTES,
            max_rdf_quads: Self::DEFAULT_MAX_RDF_QUADS,
            max_retained_rdf_term_bytes: Self::DEFAULT_MAX_RETAINED_RDF_TERM_BYTES,
        }
    }
}

impl ParseOptions {
    /// Default maximum admitted input size in bytes.
    pub const DEFAULT_MAX_INPUT_BYTES: u64 = 1_048_576;
    /// Default maximum simultaneous open JSON arrays and objects.
    pub const DEFAULT_MAX_JSON_NESTING_DEPTH: u64 = 64;
    /// Default maximum decoded member-name/value pairs in one JSON object.
    pub const DEFAULT_MAX_JSON_OBJECT_MEMBERS: u64 = 4_096;
    /// Default maximum JSON scalar and container values, including the root.
    pub const DEFAULT_MAX_JSON_VALUES: u64 = 65_536;
    /// Default maximum UTF-8 bytes across decoded JSON member names.
    pub const DEFAULT_MAX_DECODED_MEMBER_NAME_BYTES: u64 = 262_144;
    /// Default maximum RDF quads emitted before graph deduplication.
    pub const DEFAULT_MAX_RDF_QUADS: u64 = 65_536;
    /// Default maximum UTF-8 bytes retained across emitted RDF term occurrences.
    pub const DEFAULT_MAX_RETAINED_RDF_TERM_BYTES: u64 = 8_388_608;

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

    /// Returns options with the inclusive JSON nesting limit set to `max_depth`.
    #[must_use]
    pub fn with_max_json_nesting_depth(mut self, max_depth: u64) -> Self {
        self.max_json_nesting_depth = max_depth;
        self
    }

    /// Returns the inclusive JSON nesting limit.
    #[must_use]
    pub const fn max_json_nesting_depth(&self) -> u64 {
        self.max_json_nesting_depth
    }

    /// Returns options with the inclusive per-object member limit set to `max_members`.
    #[must_use]
    pub fn with_max_json_object_members(mut self, max_members: u64) -> Self {
        self.max_json_object_members = max_members;
        self
    }

    /// Returns the inclusive per-object member limit.
    #[must_use]
    pub const fn max_json_object_members(&self) -> u64 {
        self.max_json_object_members
    }

    /// Returns options with the inclusive total JSON value limit set to `max_values`.
    #[must_use]
    pub fn with_max_json_values(mut self, max_values: u64) -> Self {
        self.max_json_values = max_values;
        self
    }

    /// Returns the inclusive total JSON value limit.
    #[must_use]
    pub const fn max_json_values(&self) -> u64 {
        self.max_json_values
    }

    /// Returns options with the decoded member-name byte limit set to `max_bytes`.
    #[must_use]
    pub fn with_max_decoded_member_name_bytes(mut self, max_bytes: u64) -> Self {
        self.max_decoded_member_name_bytes = max_bytes;
        self
    }

    /// Returns the inclusive decoded member-name byte limit.
    #[must_use]
    pub const fn max_decoded_member_name_bytes(&self) -> u64 {
        self.max_decoded_member_name_bytes
    }

    /// Returns options with the inclusive emitted RDF quad limit set to `max_quads`.
    #[must_use]
    pub fn with_max_rdf_quads(mut self, max_quads: u64) -> Self {
        self.max_rdf_quads = max_quads;
        self
    }

    /// Returns the inclusive emitted RDF quad limit.
    #[must_use]
    pub const fn max_rdf_quads(&self) -> u64 {
        self.max_rdf_quads
    }

    /// Returns options with the inclusive retained RDF term-byte limit set to `max_bytes`.
    #[must_use]
    pub fn with_max_retained_rdf_term_bytes(mut self, max_bytes: u64) -> Self {
        self.max_retained_rdf_term_bytes = max_bytes;
        self
    }

    /// Returns the inclusive retained RDF term-byte limit.
    #[must_use]
    pub const fn max_retained_rdf_term_bytes(&self) -> u64 {
        self.max_retained_rdf_term_bytes
    }
}
