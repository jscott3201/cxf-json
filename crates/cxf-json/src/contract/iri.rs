use std::{error::Error, fmt};

/// Validated absolute document IRI used as JSON-LD parse context.
///
/// Validation follows RFC 3987 through a private `oxiri` dependency. The exact
/// submitted spelling is retained without normalization.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct DocumentIri(Box<str>);

impl DocumentIri {
    /// Validates and owns an absolute IRI.
    pub fn parse(value: &str) -> Result<Self, DocumentIriError> {
        oxiri::Iri::parse(value).map_err(|_| DocumentIriError)?;
        Ok(Self(value.into()))
    }

    /// Returns the exact validated IRI spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DocumentIri {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for DocumentIri {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("DocumentIri")
            .field(&"<redacted>")
            .finish()
    }
}

/// Error returned when a document IRI is not an absolute RFC 3987 IRI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentIriError;

impl fmt::Display for DocumentIriError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("document IRI must be an absolute RFC 3987 IRI")
    }
}

impl Error for DocumentIriError {}
