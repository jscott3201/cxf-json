use serde_json::Value;

use crate::{
    DiagnosticStage, ProbeDiagnostic, ProbeFailure, SourceDocument, SourcePosition, SourceRange,
};

/// Serde JSON value paired with the exact accepted input bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonDocument {
    pub source: SourceDocument,
    pub value: Value,
}

/// Parses ordinary JSON while retaining the original input bytes.
pub fn parse_json(input: &[u8]) -> Result<JsonDocument, ProbeFailure> {
    serde_json::from_slice(input)
        .map(|value| JsonDocument {
            source: SourceDocument::new(input),
            value,
        })
        .map_err(|error| ProbeFailure {
            source: SourceDocument::new(input),
            diagnostic: ProbeDiagnostic {
                stage: DiagnosticStage::Json,
                message: error.to_string(),
                range: serde_error_range(input, &error),
            },
        })
}

fn serde_error_range(input: &[u8], error: &serde_json::Error) -> Option<SourceRange> {
    let (line, column, offset) = if error.classify() == serde_json::error::Category::Eof {
        let offset = input.len();
        let line_start = input
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let line = input[..line_start]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count();
        (line, offset - line_start, offset)
    } else {
        let line = error.line().checked_sub(1)?;
        let column = error.column().checked_sub(1)?;
        let line_start = input
            .split_inclusive(|byte| *byte == b'\n')
            .take(line)
            .map(<[u8]>::len)
            .sum::<usize>();
        (
            line,
            column,
            line_start.checked_add(column)?.min(input.len()),
        )
    };
    let position = SourcePosition {
        offset: offset as u64,
        line: line as u64,
        column: column as u64,
    };
    Some(SourceRange {
        start: position,
        end: position,
    })
}
