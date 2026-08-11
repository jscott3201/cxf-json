use std::{error::Error, fmt};

use super::ParseOptions;

/// Exact input bytes owned by the core contract.
///
/// Construction does not validate JSON or CXF. [`SourceDocument::admit_bytes`]
/// applies the profile input-size limit before retaining a copy.
#[derive(Clone, Eq, PartialEq)]
pub struct SourceDocument {
    bytes: Vec<u8>,
}

impl SourceDocument {
    /// Takes ownership of input bytes without copying them.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Admits borrowed input under `options` and retains one exact owned copy.
    ///
    /// This checks only the byte length. UTF-8, JSON, JSON-LD, and CXF processing
    /// occur after admission.
    pub fn admit_bytes(input: &[u8], options: &ParseOptions) -> Result<Self, AdmissionError> {
        let actual_bytes = input.len() as u64;
        let max_input_bytes = options.max_input_bytes();

        if actual_bytes > max_input_bytes {
            return Err(AdmissionError {
                actual_bytes,
                max_input_bytes,
            });
        }

        Ok(Self::from_bytes(input.to_vec()))
    }

    /// Returns the exact owned bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the byte length of the source document.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` when the source document contains no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for SourceDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceDocument")
            .field("len", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

/// Input-size failure produced before source bytes are retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionError {
    actual_bytes: u64,
    max_input_bytes: u64,
}

impl AdmissionError {
    /// Returns the submitted byte count.
    #[must_use]
    pub const fn actual_bytes(self) -> u64 {
        self.actual_bytes
    }

    /// Returns the configured inclusive byte limit.
    #[must_use]
    pub const fn max_input_bytes(self) -> u64 {
        self.max_input_bytes
    }
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "input is {} bytes; maximum admitted size is {} bytes",
            self.actual_bytes, self.max_input_bytes
        )
    }
}

impl Error for AdmissionError {}

/// Zero-based byte position in a [`SourceDocument`].
///
/// `line` counts line-feed bytes before the position. `column` counts bytes
/// since the most recent line-feed byte; it is not a Unicode character count.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourcePosition {
    offset: u64,
    line: u64,
    column: u64,
}

impl SourcePosition {
    /// Creates a zero-based byte position.
    #[must_use]
    pub const fn new(offset: u64, line: u64, column: u64) -> Self {
        Self {
            offset,
            line,
            column,
        }
    }

    /// Returns the byte offset from the start of the document.
    #[must_use]
    pub const fn offset(self) -> u64 {
        self.offset
    }

    /// Returns the zero-based line number.
    #[must_use]
    pub const fn line(self) -> u64 {
        self.line
    }

    /// Returns the zero-based byte column.
    #[must_use]
    pub const fn column(self) -> u64 {
        self.column
    }
}

/// Half-open byte range in a [`SourceDocument`].
///
/// A detection position is represented by equal start and end positions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceRange {
    start: SourcePosition,
    end: SourcePosition,
}

impl SourceRange {
    /// Creates a half-open range from two byte positions.
    ///
    /// Returns `None` when the end offset precedes the start offset.
    #[must_use]
    pub const fn new(start: SourcePosition, end: SourcePosition) -> Option<Self> {
        if start.offset <= end.offset {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Returns the inclusive start position.
    #[must_use]
    pub const fn start(self) -> SourcePosition {
        self.start
    }

    /// Returns the exclusive end position.
    #[must_use]
    pub const fn end(self) -> SourcePosition {
        self.end
    }
}
