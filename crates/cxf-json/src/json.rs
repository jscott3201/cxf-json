use std::collections::HashSet;

use crate::{AdmissionError, ParseOptions, SourceDocument, SourcePosition, SourceRange};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct JsonStructureMetrics {
    pub(crate) max_nesting_depth: u64,
    pub(crate) max_object_members: u64,
    pub(crate) total_values: u64,
    pub(crate) decoded_member_name_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JsonFailureKind {
    InvalidUtf8,
    Syntax,
    DuplicateMember,
    NestingLimit,
    ObjectMemberLimit,
    ValueLimit,
    DecodedMemberNameBytesLimit,
}

impl JsonFailureKind {
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "input is not valid UTF-8",
            Self::Syntax => "invalid JSON syntax",
            Self::DuplicateMember => "duplicate object member name",
            Self::NestingLimit => "JSON nesting depth exceeds the configured limit",
            Self::ObjectMemberLimit => "JSON object members exceed the configured limit",
            Self::ValueLimit => "JSON values exceed the configured limit",
            Self::DecodedMemberNameBytesLimit => {
                "decoded JSON member-name bytes exceed the configured limit"
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct JsonPreflightError {
    source: SourceDocument,
    kind: JsonFailureKind,
    range: SourceRange,
}

impl JsonPreflightError {
    pub(crate) fn source_document(&self) -> &SourceDocument {
        &self.source
    }

    pub(crate) const fn kind(&self) -> JsonFailureKind {
        self.kind
    }

    pub(crate) const fn range(&self) -> SourceRange {
        self.range
    }

    pub(crate) const fn message(&self) -> &'static str {
        self.kind.message()
    }

    pub(crate) fn into_source_document(self) -> SourceDocument {
        self.source
    }
}

#[derive(Debug)]
pub(crate) enum PreflightFailure {
    Admission(AdmissionError),
    Json(JsonPreflightError),
}

#[derive(Debug)]
pub(crate) struct PreflightedJson {
    source: SourceDocument,
    metrics: JsonStructureMetrics,
}

impl PreflightedJson {
    pub(crate) fn source_document(&self) -> &SourceDocument {
        &self.source
    }

    pub(crate) const fn metrics(&self) -> JsonStructureMetrics {
        self.metrics
    }

    pub(crate) fn into_source_document(self) -> SourceDocument {
        self.source
    }
}

pub(crate) fn admit_and_preflight(
    input: &[u8],
    options: &ParseOptions,
) -> Result<PreflightedJson, PreflightFailure> {
    let source =
        SourceDocument::admit_bytes(input, options).map_err(PreflightFailure::Admission)?;
    preflight_admitted(source, options).map_err(PreflightFailure::Json)
}

fn preflight_admitted(
    source: SourceDocument,
    options: &ParseOptions,
) -> Result<PreflightedJson, JsonPreflightError> {
    if let Err(error) = std::str::from_utf8(source.as_bytes()) {
        return Err(json_failure(
            source,
            JsonFailureKind::InvalidUtf8,
            error.valid_up_to(),
        ));
    }

    match scan_json(source.as_bytes(), options) {
        Ok(metrics) => Ok(PreflightedJson { source, metrics }),
        Err(error) => Err(json_failure(source, error.kind, error.offset)),
    }
}

fn json_failure(
    source: SourceDocument,
    kind: JsonFailureKind,
    offset: usize,
) -> JsonPreflightError {
    let range = source_range(source.as_bytes(), offset);
    JsonPreflightError {
        source,
        kind,
        range,
    }
}

fn source_range(input: &[u8], offset: usize) -> SourceRange {
    let offset = offset.min(input.len());
    let line_start = input[..offset]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    let position = SourcePosition::new(
        offset as u64,
        input[..line_start]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count() as u64,
        (offset - line_start) as u64,
    );
    SourceRange::new(position, position).expect("equal positions form a detection range")
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
        member_count: u64,
    },
}

struct ScanError {
    kind: JsonFailureKind,
    offset: usize,
}

fn scan_json(input: &[u8], options: &ParseOptions) -> Result<JsonStructureMetrics, ScanError> {
    let mut offset = 0;
    let mut root_complete = false;
    let mut stack = Vec::new();
    let mut metrics = JsonStructureMetrics::default();

    loop {
        skip_whitespace(input, &mut offset);
        let Some(frame) = stack.last_mut() else {
            if root_complete {
                return if offset == input.len() {
                    Ok(metrics)
                } else {
                    Err(scan_error(JsonFailureKind::Syntax, offset))
                };
            }
            parse_value(input, &mut offset, &mut stack, &mut metrics, options)?;
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
                    parse_value(input, &mut offset, &mut stack, &mut metrics, options)?;
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
                    _ => return Err(scan_error(JsonFailureKind::Syntax, offset)),
                },
            },
            Frame::Object {
                state,
                names,
                member_count,
            } => match state {
                ObjectState::FirstKeyOrEnd if input.get(offset) == Some(&b'}') => {
                    offset += 1;
                    stack.pop();
                }
                ObjectState::FirstKeyOrEnd | ObjectState::Key => {
                    let key_offset = offset;
                    if input.get(offset) != Some(&b'"') {
                        return Err(scan_error(JsonFailureKind::Syntax, offset));
                    }
                    let next_member_count = checked_increment(
                        *member_count,
                        options.max_json_object_members(),
                        JsonFailureKind::ObjectMemberLimit,
                        key_offset,
                    )?;
                    let name = parse_member_name(input, &mut offset)?;
                    let name_bytes = name.len() as u64;
                    let decoded_member_name_bytes = metrics
                        .decoded_member_name_bytes
                        .checked_add(name_bytes)
                        .ok_or_else(|| {
                            scan_error(JsonFailureKind::DecodedMemberNameBytesLimit, key_offset)
                        })?;
                    if decoded_member_name_bytes > options.max_decoded_member_name_bytes() {
                        return Err(scan_error(
                            JsonFailureKind::DecodedMemberNameBytesLimit,
                            key_offset,
                        ));
                    }
                    if !names.insert(name) {
                        return Err(scan_error(JsonFailureKind::DuplicateMember, key_offset));
                    }
                    *member_count = next_member_count;
                    metrics.max_object_members = metrics.max_object_members.max(next_member_count);
                    metrics.decoded_member_name_bytes = decoded_member_name_bytes;
                    *state = ObjectState::Colon;
                }
                ObjectState::Colon => {
                    if input.get(offset) != Some(&b':') {
                        return Err(scan_error(JsonFailureKind::Syntax, offset));
                    }
                    offset += 1;
                    *state = ObjectState::Value;
                }
                ObjectState::Value => {
                    *state = ObjectState::CommaOrEnd;
                    parse_value(input, &mut offset, &mut stack, &mut metrics, options)?;
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
                    _ => return Err(scan_error(JsonFailureKind::Syntax, offset)),
                },
            },
        }
    }
}

fn parse_value(
    input: &[u8],
    offset: &mut usize,
    stack: &mut Vec<Frame>,
    metrics: &mut JsonStructureMetrics,
    options: &ParseOptions,
) -> Result<(), ScanError> {
    skip_whitespace(input, offset);
    let value_offset = *offset;
    let Some(value_start) = input.get(value_offset) else {
        return Err(scan_error(JsonFailureKind::Syntax, value_offset));
    };
    if !matches!(
        value_start,
        b'"' | b'{' | b'[' | b't' | b'f' | b'n' | b'-' | b'0'..=b'9'
    ) {
        return Err(scan_error(JsonFailureKind::Syntax, value_offset));
    }

    metrics.total_values = checked_increment(
        metrics.total_values,
        options.max_json_values(),
        JsonFailureKind::ValueLimit,
        value_offset,
    )?;

    match value_start {
        b'{' => {
            let depth = checked_depth(stack, options, value_offset)?;
            *offset += 1;
            stack.push(Frame::Object {
                state: ObjectState::FirstKeyOrEnd,
                names: HashSet::new(),
                member_count: 0,
            });
            metrics.max_nesting_depth = metrics.max_nesting_depth.max(depth);
            Ok(())
        }
        b'[' => {
            let depth = checked_depth(stack, options, value_offset)?;
            *offset += 1;
            stack.push(Frame::Array(ArrayState::FirstValueOrEnd));
            metrics.max_nesting_depth = metrics.max_nesting_depth.max(depth);
            Ok(())
        }
        b'"' => scan_string_token(input, offset).map(drop),
        b't' => parse_keyword(input, offset, b"true"),
        b'f' => parse_keyword(input, offset, b"false"),
        b'n' => parse_keyword(input, offset, b"null"),
        b'-' | b'0'..=b'9' => parse_number(input, offset),
        _ => unreachable!("value start was checked above"),
    }
}

fn checked_depth(stack: &[Frame], options: &ParseOptions, offset: usize) -> Result<u64, ScanError> {
    let depth = (stack.len() as u64)
        .checked_add(1)
        .ok_or_else(|| scan_error(JsonFailureKind::NestingLimit, offset))?;
    if depth > options.max_json_nesting_depth() {
        return Err(scan_error(JsonFailureKind::NestingLimit, offset));
    }
    Ok(depth)
}

fn checked_increment(
    value: u64,
    maximum: u64,
    kind: JsonFailureKind,
    offset: usize,
) -> Result<u64, ScanError> {
    let next = value
        .checked_add(1)
        .ok_or_else(|| scan_error(kind, offset))?;
    if next > maximum {
        return Err(scan_error(kind, offset));
    }
    Ok(next)
}

fn parse_member_name(input: &[u8], offset: &mut usize) -> Result<String, ScanError> {
    let (start, end) = scan_string_token(input, offset)?;
    serde_json::from_slice(&input[start..end]).map_err(|error| {
        let relative = error.column().saturating_sub(1);
        scan_error(JsonFailureKind::Syntax, start + relative)
    })
}

fn scan_string_token(input: &[u8], offset: &mut usize) -> Result<(usize, usize), ScanError> {
    let start = *offset;
    if input.get(start) != Some(&b'"') {
        return Err(scan_error(JsonFailureKind::Syntax, start));
    }

    *offset += 1;
    while let Some(byte) = input.get(*offset) {
        match byte {
            b'"' => {
                *offset += 1;
                return Ok((start, *offset));
            }
            b'\\' => {
                let escape_offset = *offset;
                *offset += 1;
                match input.get(*offset) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                        *offset += 1;
                    }
                    Some(b'u') => {
                        let code = parse_unicode_escape(input, offset)?;
                        if (0xd800..=0xdbff).contains(&code) {
                            let pair_offset = *offset;
                            if input.get(pair_offset) != Some(&b'\\')
                                || input.get(pair_offset + 1) != Some(&b'u')
                            {
                                return Err(scan_error(JsonFailureKind::Syntax, escape_offset));
                            }
                            *offset += 1;
                            let low = parse_unicode_escape(input, offset)?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return Err(scan_error(JsonFailureKind::Syntax, pair_offset));
                            }
                        } else if (0xdc00..=0xdfff).contains(&code) {
                            return Err(scan_error(JsonFailureKind::Syntax, escape_offset));
                        }
                    }
                    _ => return Err(scan_error(JsonFailureKind::Syntax, *offset)),
                }
            }
            0x00..=0x1f => return Err(scan_error(JsonFailureKind::Syntax, *offset)),
            _ => *offset += 1,
        }
    }
    Err(scan_error(JsonFailureKind::Syntax, input.len()))
}

fn parse_unicode_escape(input: &[u8], offset: &mut usize) -> Result<u16, ScanError> {
    debug_assert_eq!(input.get(*offset), Some(&b'u'));
    *offset += 1;
    let mut value = 0_u16;
    for _ in 0..4 {
        let Some(byte) = input.get(*offset) else {
            return Err(scan_error(JsonFailureKind::Syntax, input.len()));
        };
        let Some(digit) = hex_digit(*byte) else {
            return Err(scan_error(JsonFailureKind::Syntax, *offset));
        };
        value = value * 16 + digit;
        *offset += 1;
    }
    Ok(value)
}

const fn hex_digit(byte: u8) -> Option<u16> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as u16),
        b'a'..=b'f' => Some((byte - b'a' + 10) as u16),
        b'A'..=b'F' => Some((byte - b'A' + 10) as u16),
        _ => None,
    }
}

fn parse_keyword(input: &[u8], offset: &mut usize, keyword: &[u8]) -> Result<(), ScanError> {
    if input.get(*offset..(*offset + keyword.len()).min(input.len())) != Some(keyword) {
        return Err(scan_error(JsonFailureKind::Syntax, *offset));
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
                return Err(scan_error(JsonFailureKind::Syntax, *offset));
            }
        }
        Some(b'1'..=b'9') => consume_digits(input, offset),
        _ => return Err(scan_error(JsonFailureKind::Syntax, *offset)),
    }

    if input.get(*offset) == Some(&b'.') {
        *offset += 1;
        if !input.get(*offset).is_some_and(u8::is_ascii_digit) {
            return Err(scan_error(JsonFailureKind::Syntax, *offset));
        }
        consume_digits(input, offset);
    }

    if matches!(input.get(*offset), Some(b'e' | b'E')) {
        *offset += 1;
        if matches!(input.get(*offset), Some(b'+' | b'-')) {
            *offset += 1;
        }
        if !input.get(*offset).is_some_and(u8::is_ascii_digit) {
            return Err(scan_error(JsonFailureKind::Syntax, *offset));
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

const fn scan_error(kind: JsonFailureKind, offset: usize) -> ScanError {
    ScanError { kind, offset }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    const INVALID_UTF8: &[u8] = &[b'{', b'"', 0x80, b'"', b':', b'0', b'}'];

    const REVIEWED_SEEDS: &[(&str, &[u8], Option<JsonFailureKind>)] = &[
        (
            "decoded-duplicate",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/decoded-duplicate.seed"),
            Some(JsonFailureKind::DuplicateMember),
        ),
        (
            "deep-array",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/deep-array.seed"),
            None,
        ),
        (
            "invalid-utf8",
            INVALID_UTF8,
            Some(JsonFailureKind::InvalidUtf8),
        ),
        (
            "malformed-number",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/malformed-number.seed"),
            Some(JsonFailureKind::Syntax),
        ),
        (
            "unicode-escapes",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/unicode-escapes.seed"),
            None,
        ),
        (
            "unique-members",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/unique-members.seed"),
            None,
        ),
        (
            "unterminated-string",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/unterminated-string.seed"),
            Some(JsonFailureKind::Syntax),
        ),
        (
            "valid-object",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/valid-object.seed"),
            None,
        ),
        (
            "wide-object",
            include_bytes!("../../cxf-ingest-probe/tests/parser-seeds/wide-object.seed"),
            None,
        ),
    ];

    #[test]
    fn private_seam_applies_admission_before_json_preflight() {
        let options = ParseOptions::new().with_max_input_bytes(0);
        match admit_and_preflight(b"{", &options) {
            Err(PreflightFailure::Admission(error)) => {
                assert_eq!(error.actual_bytes(), 1);
                assert_eq!(error.max_input_bytes(), 0);
            }
            result => panic!("expected admission failure, got {result:?}"),
        }

        let options = ParseOptions::new().with_max_input_bytes(1);
        let error = json_error(b"{", &options);
        assert_eq!(error.kind(), JsonFailureKind::Syntax);
        assert_eq!(error.source_document().as_bytes(), b"{");
    }

    #[test]
    fn preflight_preserves_the_admitted_source_allocation() {
        let success_bytes = b"{}".to_vec();
        let success_pointer = success_bytes.as_ptr();
        let success = preflight_admitted(
            SourceDocument::from_bytes(success_bytes),
            &ParseOptions::new(),
        )
        .expect("object should pass");
        assert_eq!(
            success.source_document().as_bytes().as_ptr(),
            success_pointer
        );

        let failure_bytes = br#"{"a":0,"a":1}"#.to_vec();
        let failure_pointer = failure_bytes.as_ptr();
        let failure = preflight_admitted(
            SourceDocument::from_bytes(failure_bytes),
            &ParseOptions::new(),
        )
        .expect_err("duplicate should fail");
        assert_eq!(
            failure.source_document().as_bytes().as_ptr(),
            failure_pointer
        );
    }

    #[test]
    fn reports_exact_structural_metrics() {
        let input = br#"{"a":[1,{"\u0062":true}],"c":null}"#;
        let result =
            admit_and_preflight(input, &ParseOptions::new()).expect("structured input should pass");
        assert_eq!(
            result.metrics(),
            JsonStructureMetrics {
                max_nesting_depth: 3,
                max_object_members: 2,
                total_values: 6,
                decoded_member_name_bytes: 3,
            }
        );
    }

    #[test]
    fn replays_reviewed_parser_seeds_directly() {
        for (name, input, expected_failure) in REVIEWED_SEEDS {
            match (
                admit_and_preflight(input, &ParseOptions::new()),
                expected_failure,
            ) {
                (Ok(result), None) => {
                    assert_eq!(result.source_document().as_bytes(), *input, "{name}");
                    assert!(result.metrics().total_values > 0, "{name}");
                }
                (Err(PreflightFailure::Json(error)), Some(expected)) => {
                    assert_eq!(error.kind(), *expected, "{name}");
                    assert_eq!(error.source_document().as_bytes(), *input, "{name}");
                    assert!(error.range().end().offset() <= input.len() as u64, "{name}");
                }
                (result, expected) => {
                    panic!("unexpected seed result for {name}: {result:?}, expected {expected:?}")
                }
            }
        }
    }

    #[test]
    fn rejects_qualified_malformed_json_forms() {
        let cases: &[&[u8]] = &[
            b"",
            b"01",
            b"-",
            b"1.",
            b"1e",
            b"[1,]",
            br#"{"key":1,}"#,
            br#"{"key" 1}"#,
            br#"{"key":1 "other":2}"#,
            br#""\uDEAD""#,
            br#""\uD83D\u0041""#,
            b"true false",
        ];

        for input in cases {
            let error = json_error(input, &ParseOptions::new());
            assert_eq!(error.kind(), JsonFailureKind::Syntax, "{input:?}");
            assert!(error.range().end().offset() <= input.len() as u64);
        }
    }

    #[test]
    fn duplicate_names_are_decoded_without_normalization_or_message_amplification() {
        let duplicate_cases: &[&[u8]] = &[
            br#"{"value":1,"value":2}"#,
            br#"{"a":1,"\u0061":2}"#,
            br#"{"nested":{"key":1,"key":2}}"#,
            br#"[{"key":1,"key":2}]"#,
        ];
        for input in duplicate_cases {
            let error = json_error(input, &ParseOptions::new());
            assert_eq!(error.kind(), JsonFailureKind::DuplicateMember);
            assert_eq!(error.message(), "duplicate object member name");
            assert!(!error.message().contains("value"));
        }

        admit_and_preflight("{\"é\":1,\"e\\u0301\":2}".as_bytes(), &ParseOptions::new())
            .expect("non-normalized names should remain distinct");
    }

    #[test]
    fn value_limit_precedes_nesting_at_a_recognized_container() {
        let options = ParseOptions::new()
            .with_max_json_values(0)
            .with_max_json_nesting_depth(0);
        let error = json_error(b"{}", &options);
        assert_eq!(error.kind(), JsonFailureKind::ValueLimit);
        assert_eq!(error.range().start().offset(), 0);

        let error = json_error(b"@", &options);
        assert_eq!(error.kind(), JsonFailureKind::Syntax);
    }

    #[test]
    fn zero_limits_follow_the_profile_semantics() {
        let scalar_only = ParseOptions::new().with_max_json_nesting_depth(0);
        admit_and_preflight(b"0", &scalar_only).expect("scalar has depth zero");
        assert_eq!(
            json_error(b"[]", &scalar_only).kind(),
            JsonFailureKind::NestingLimit
        );

        let empty_objects = ParseOptions::new().with_max_json_object_members(0);
        admit_and_preflight(b"{}", &empty_objects).expect("empty object has no members");
        assert_eq!(
            json_error(br#"{"":0}"#, &empty_objects).kind(),
            JsonFailureKind::ObjectMemberLimit
        );

        let empty_names = ParseOptions::new().with_max_decoded_member_name_bytes(0);
        admit_and_preflight(br#"{"":0}"#, &empty_names).expect("empty name has no bytes");
        assert_eq!(
            json_error(br#"{"a":0}"#, &empty_names).kind(),
            JsonFailureKind::DecodedMemberNameBytesLimit
        );
    }

    #[test]
    fn decoded_name_limit_points_to_the_opening_quote() {
        let input = br#"{"\u00e9":0}"#;
        let options = ParseOptions::new().with_max_decoded_member_name_bytes(1);
        let error = json_error(input, &options);
        assert_eq!(error.kind(), JsonFailureKind::DecodedMemberNameBytesLimit);
        assert_eq!(error.range().start().offset(), 1);
    }

    #[test]
    fn generated_default_boundaries_are_inclusive() {
        let options = ParseOptions::new();

        let depth_at_limit = nested_array(64);
        admit_and_preflight(&depth_at_limit, &options).expect("depth limit should be inclusive");
        assert_eq!(
            json_error(&nested_array(65), &options).kind(),
            JsonFailureKind::NestingLimit
        );

        let members_at_limit = object_with_members(4_096);
        admit_and_preflight(&members_at_limit, &options).expect("member limit should be inclusive");
        assert_eq!(
            json_error(&object_with_members(4_097), &options).kind(),
            JsonFailureKind::ObjectMemberLimit
        );

        let values_at_limit = array_with_elements(65_535);
        admit_and_preflight(&values_at_limit, &options).expect("value limit should be inclusive");
        assert_eq!(
            json_error(&array_with_elements(65_536), &options).kind(),
            JsonFailureKind::ValueLimit
        );

        let names_at_limit = object_with_name_bytes(262_144);
        admit_and_preflight(&names_at_limit, &options)
            .expect("decoded-name limit should be inclusive");
        assert_eq!(
            json_error(&object_with_name_bytes(262_145), &options).kind(),
            JsonFailureKind::DecodedMemberNameBytesLimit
        );
    }

    #[test]
    fn long_duplicate_has_a_fixed_short_failure_message() {
        let name = "a".repeat(65_536);
        let input = format!("{{\"{name}\":0,\"{name}\":1}}");
        let error = json_error(input.as_bytes(), &ParseOptions::new());
        assert_eq!(error.kind(), JsonFailureKind::DuplicateMember);
        assert_eq!(error.message(), "duplicate object member name");
        assert!(error.message().len() < 64);
        assert!(!format!("{error:?}").contains(&name));
    }

    fn json_error(input: &[u8], options: &ParseOptions) -> JsonPreflightError {
        match admit_and_preflight(input, options) {
            Err(PreflightFailure::Json(error)) => error,
            result => panic!("expected JSON failure, got {result:?}"),
        }
    }

    fn nested_array(depth: usize) -> Vec<u8> {
        let mut input = Vec::with_capacity(depth * 2 + 1);
        input.extend(std::iter::repeat_n(b'[', depth));
        input.push(b'0');
        input.extend(std::iter::repeat_n(b']', depth));
        input
    }

    fn object_with_members(members: usize) -> Vec<u8> {
        let mut input = String::with_capacity(members * 12);
        input.push('{');
        for index in 0..members {
            if index > 0 {
                input.push(',');
            }
            write!(input, "\"k{index}\":0").expect("writing to a string cannot fail");
        }
        input.push('}');
        input.into_bytes()
    }

    fn array_with_elements(elements: usize) -> Vec<u8> {
        let mut input = Vec::with_capacity(elements * 2 + 1);
        input.push(b'[');
        for index in 0..elements {
            if index > 0 {
                input.push(b',');
            }
            input.push(b'0');
        }
        input.push(b']');
        input
    }

    fn object_with_name_bytes(name_bytes: usize) -> Vec<u8> {
        let mut input = String::with_capacity(name_bytes + 6);
        input.push_str("{\"");
        input.extend(std::iter::repeat_n('a', name_bytes));
        input.push_str("\":0}");
        input.into_bytes()
    }
}
