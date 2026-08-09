use std::collections::HashSet;

use serde_json::Value;

use crate::{
    DiagnosticStage, ProbeDiagnostic, ProbeFailure, SourceDocument, SourcePosition, SourceRange,
};

/// Serde JSON value paired with the exact submitted input bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonDocument {
    pub source: SourceDocument,
    pub value: Value,
}

/// Parses ordinary JSON while retaining the submitted input bytes.
pub fn parse_json(input: &[u8]) -> Result<JsonDocument, ProbeFailure> {
    validate_unique_members(input)?;
    let value = serde_json::from_slice(input).map_err(|error| json_failure(input, error))?;
    Ok(JsonDocument {
        source: SourceDocument::new(input),
        value,
    })
}

pub(crate) fn validate_unique_members(input: &[u8]) -> Result<(), ProbeFailure> {
    if let Err(error) = std::str::from_utf8(input) {
        let offset = error.valid_up_to();
        return Err(ProbeFailure {
            source: SourceDocument::new(input),
            diagnostic: Box::new(ProbeDiagnostic {
                stage: DiagnosticStage::Json,
                message: format!("invalid UTF-8 at byte offset {offset}"),
                range: Some(source_range(input, offset, offset)),
                pointer: None,
                rdf_term: None,
            }),
        });
    }

    scan_json(input).map_err(|error| ProbeFailure {
        source: SourceDocument::new(input),
        diagnostic: Box::new(ProbeDiagnostic {
            stage: DiagnosticStage::Json,
            message: error.message,
            range: Some(source_range(input, error.offset, error.offset)),
            pointer: None,
            rdf_term: None,
        }),
    })
}

fn json_failure(input: &[u8], error: serde_json::Error) -> ProbeFailure {
    ProbeFailure {
        source: SourceDocument::new(input),
        diagnostic: Box::new(ProbeDiagnostic {
            stage: DiagnosticStage::Json,
            message: error.to_string(),
            range: serde_error_range(input, &error),
            pointer: None,
            rdf_term: None,
        }),
    }
}

pub(crate) fn source_range(input: &[u8], start_offset: usize, end_offset: usize) -> SourceRange {
    let start_offset = start_offset.min(input.len());
    let end_offset = end_offset.clamp(start_offset, input.len());
    SourceRange {
        start: source_position(input, start_offset),
        end: source_position(input, end_offset),
    }
}

fn source_position(input: &[u8], offset: usize) -> SourcePosition {
    let offset = offset.min(input.len());
    let line_start = input[..offset]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    SourcePosition {
        offset: offset as u64,
        line: input[..line_start]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count() as u64,
        column: (offset - line_start) as u64,
    }
}

fn serde_error_range(input: &[u8], error: &serde_json::Error) -> Option<SourceRange> {
    let offset = if error.classify() == serde_json::error::Category::Eof {
        input.len()
    } else {
        let line = error.line().checked_sub(1)?;
        let column = error.column().checked_sub(1)?;
        let line_start = input
            .split_inclusive(|byte| *byte == b'\n')
            .take(line)
            .map(<[u8]>::len)
            .sum::<usize>();
        line_start.checked_add(column)?.min(input.len())
    };
    Some(source_range(input, offset, offset))
}

#[derive(Clone, Copy)]
enum ArrayState {
    FirstValueOrEnd,
    Value,
    CommaOrEnd,
}

#[derive(Clone, Copy)]
enum ObjectState {
    FirstKeyOrEnd,
    Key,
    Colon,
    Value,
    CommaOrEnd,
}

enum Frame {
    Array(ArrayState),
    Object {
        state: ObjectState,
        names: HashSet<String>,
    },
}

struct ScanError {
    message: String,
    offset: usize,
}

fn scan_json(input: &[u8]) -> Result<(), ScanError> {
    let mut offset = 0;
    let mut root_complete = false;
    let mut stack = Vec::new();

    loop {
        skip_whitespace(input, &mut offset);
        let Some(frame) = stack.last_mut() else {
            if root_complete {
                return if offset == input.len() {
                    Ok(())
                } else {
                    Err(scan_error("trailing characters", offset))
                };
            }
            parse_value(input, &mut offset, &mut stack)?;
            root_complete = true;
            continue;
        };

        match frame {
            Frame::Array(state) => match state {
                ArrayState::FirstValueOrEnd if input.get(offset) == Some(&b']') => {
                    offset += 1;
                    stack.pop();
                }
                ArrayState::FirstValueOrEnd | ArrayState::Value => {
                    *state = ArrayState::CommaOrEnd;
                    parse_value(input, &mut offset, &mut stack)?;
                }
                ArrayState::CommaOrEnd => match input.get(offset) {
                    Some(b',') => {
                        offset += 1;
                        *state = ArrayState::Value;
                    }
                    Some(b']') => {
                        offset += 1;
                        stack.pop();
                    }
                    _ => return Err(scan_error("expected ',' or ']'", offset)),
                },
            },
            Frame::Object { state, names } => match state {
                ObjectState::FirstKeyOrEnd if input.get(offset) == Some(&b'}') => {
                    offset += 1;
                    stack.pop();
                }
                ObjectState::FirstKeyOrEnd | ObjectState::Key => {
                    let key_offset = offset;
                    let name = parse_string(input, &mut offset)?;
                    if !names.insert(name.clone()) {
                        return Err(scan_error(
                            format!("duplicate object member {name:?}"),
                            key_offset,
                        ));
                    }
                    *state = ObjectState::Colon;
                }
                ObjectState::Colon => {
                    if input.get(offset) != Some(&b':') {
                        return Err(scan_error("expected ':'", offset));
                    }
                    offset += 1;
                    *state = ObjectState::Value;
                }
                ObjectState::Value => {
                    *state = ObjectState::CommaOrEnd;
                    parse_value(input, &mut offset, &mut stack)?;
                }
                ObjectState::CommaOrEnd => match input.get(offset) {
                    Some(b',') => {
                        offset += 1;
                        *state = ObjectState::Key;
                    }
                    Some(b'}') => {
                        offset += 1;
                        stack.pop();
                    }
                    _ => return Err(scan_error("expected ',' or '}'", offset)),
                },
            },
        }
    }
}

fn parse_value(input: &[u8], offset: &mut usize, stack: &mut Vec<Frame>) -> Result<(), ScanError> {
    skip_whitespace(input, offset);
    match input.get(*offset) {
        Some(b'{') => {
            *offset += 1;
            stack.push(Frame::Object {
                state: ObjectState::FirstKeyOrEnd,
                names: HashSet::new(),
            });
            Ok(())
        }
        Some(b'[') => {
            *offset += 1;
            stack.push(Frame::Array(ArrayState::FirstValueOrEnd));
            Ok(())
        }
        Some(b'"') => parse_string(input, offset).map(drop),
        Some(b't') => parse_keyword(input, offset, b"true"),
        Some(b'f') => parse_keyword(input, offset, b"false"),
        Some(b'n') => parse_keyword(input, offset, b"null"),
        Some(b'-' | b'0'..=b'9') => parse_number(input, offset),
        _ => Err(scan_error("expected a JSON value", *offset)),
    }
}

fn parse_string(input: &[u8], offset: &mut usize) -> Result<String, ScanError> {
    let start = *offset;
    if input.get(start) != Some(&b'"') {
        return Err(scan_error("expected an object member name", start));
    }

    *offset += 1;
    while let Some(byte) = input.get(*offset) {
        match byte {
            b'"' => {
                *offset += 1;
                return serde_json::from_slice(&input[start..*offset]).map_err(|error| {
                    let relative = error.column().saturating_sub(1);
                    scan_error(strip_serde_location(error.to_string()), start + relative)
                });
            }
            b'\\' => {
                *offset += 1;
                match input.get(*offset) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                        *offset += 1;
                    }
                    Some(b'u') => {
                        *offset += 1;
                        for _ in 0..4 {
                            if !input.get(*offset).is_some_and(u8::is_ascii_hexdigit) {
                                return Err(scan_error("invalid Unicode escape", *offset));
                            }
                            *offset += 1;
                        }
                    }
                    _ => return Err(scan_error("invalid string escape", *offset)),
                }
            }
            0x00..=0x1f => return Err(scan_error("control character in string", *offset)),
            _ => *offset += 1,
        }
    }
    Err(scan_error("unterminated string", input.len()))
}

fn parse_keyword(input: &[u8], offset: &mut usize, keyword: &[u8]) -> Result<(), ScanError> {
    if input.get(*offset..(*offset + keyword.len()).min(input.len())) != Some(keyword) {
        return Err(scan_error("invalid JSON literal", *offset));
    }
    *offset += keyword.len();
    Ok(())
}

fn parse_number(input: &[u8], offset: &mut usize) -> Result<(), ScanError> {
    if input.get(*offset) == Some(&b'-') {
        *offset += 1;
    }

    match input.get(*offset) {
        Some(b'0') => {
            *offset += 1;
            if input.get(*offset).is_some_and(u8::is_ascii_digit) {
                return Err(scan_error("leading zero in number", *offset));
            }
        }
        Some(b'1'..=b'9') => consume_digits(input, offset),
        _ => return Err(scan_error("expected a digit", *offset)),
    }

    if input.get(*offset) == Some(&b'.') {
        *offset += 1;
        if !input.get(*offset).is_some_and(u8::is_ascii_digit) {
            return Err(scan_error("expected a digit after decimal point", *offset));
        }
        consume_digits(input, offset);
    }

    if matches!(input.get(*offset), Some(b'e' | b'E')) {
        *offset += 1;
        if matches!(input.get(*offset), Some(b'+' | b'-')) {
            *offset += 1;
        }
        if !input.get(*offset).is_some_and(u8::is_ascii_digit) {
            return Err(scan_error("expected an exponent digit", *offset));
        }
        consume_digits(input, offset);
    }

    Ok(())
}

fn consume_digits(input: &[u8], offset: &mut usize) {
    while input.get(*offset).is_some_and(u8::is_ascii_digit) {
        *offset += 1;
    }
}

fn skip_whitespace(input: &[u8], offset: &mut usize) {
    while matches!(input.get(*offset), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        *offset += 1;
    }
}

fn strip_serde_location(message: String) -> String {
    match message.rsplit_once(" at line ") {
        Some((message, _)) => message.to_owned(),
        None => message,
    }
}

fn scan_error(message: impl Into<String>, offset: usize) -> ScanError {
    ScanError {
        message: message.into(),
        offset,
    }
}
