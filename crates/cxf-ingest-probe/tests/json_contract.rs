use cxf_ingest_probe::{
    DiagnosticStage, JsonStructureMetrics, ProbeDiagnostic, ProbeMetrics, ProbeReport, RdfNodeKind,
    RdfNodeSummary, RdfObjectSummary, RdfQuadSummary, SourceDocument, SourcePosition, SourceRange,
    parse_json,
};

const NUMBERS: &[u8] = include_bytes!("../../cxf-json/tests/fixtures/numbers.json");

#[test]
fn retains_exact_input_bytes() {
    let document = parse_json(NUMBERS).expect("owned fixture should parse");

    assert_eq!(document.source.as_bytes(), NUMBERS);
    assert_eq!(document.value["é"]["a/b~c"], 7);
}

#[test]
fn rejects_duplicate_decoded_member_names() {
    let cases: &[&[u8]] = &[
        br#"{"value":1,"value":2}"#,
        br#"{"nested":{"key":1,"key":2}}"#,
        br#"[{"key":1,"key":2}]"#,
        br#"{"a":1,"\u0061":2}"#,
        br#"{"value":1,"value":1e400}"#,
    ];

    for input in cases {
        let failure = parse_json(input).expect_err("duplicate members must be rejected");

        assert_eq!(failure.source.as_bytes(), *input);
        assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
        assert!(
            failure
                .diagnostic
                .message
                .contains("duplicate object member")
        );
        assert!(failure.diagnostic.range.is_some());
        assert_eq!(failure.diagnostic.pointer, None);
        assert_eq!(failure.diagnostic.rdf_term, None);
    }
}

#[test]
fn compares_member_names_without_unicode_normalization() {
    let input = "{\"é\":1,\"e\\u0301\":2}".as_bytes();
    let document = parse_json(input).expect("distinct decoded names should parse");

    assert_eq!(
        document.value.as_object().map(serde_json::Map::len),
        Some(2)
    );
    assert_eq!(document.value["é"], 1);
    assert_eq!(document.value["e\u{301}"], 2);
}

#[test]
fn preserves_serde_private_number_token_as_an_object_name() {
    let input = br#"{"$serde_json::private::Number":"not a number"}"#;
    let document = parse_json(input).expect("ordinary object should not become a Serde number");

    assert_eq!(
        document.value["$serde_json::private::Number"],
        "not a number"
    );
}

#[test]
fn reports_exact_structural_metrics() {
    let input = br#"{"a":[1,{"\u0062":true}],"c":null}"#;
    let document = parse_json(input).expect("structured input should parse");

    assert_eq!(
        document.metrics,
        JsonStructureMetrics {
            max_nesting_depth: 3,
            max_object_members: 2,
            total_values: 6,
            decoded_member_name_bytes: 3,
        }
    );
}

#[test]
fn lexical_preflight_rejects_malformed_json_forms() {
    let cases: &[&[u8]] = &[
        b"01",
        b"-",
        b"1.",
        b"1e",
        b"[1,]",
        br#"{"key":1,}"#,
        br#"{"key" 1}"#,
        br#"{"key":1 "other":2}"#,
        br#""\uDEAD""#,
        b"true false",
    ];

    for input in cases {
        let failure = parse_json(input).expect_err("malformed JSON must fail");
        let range = failure
            .diagnostic
            .range
            .expect("lexical failure should be located");

        assert_eq!(failure.source.as_bytes(), *input);
        assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
        assert!(range.start.offset <= input.len() as u64);
        assert!(range.end.offset <= input.len() as u64);
    }
}

#[test]
fn nested_duplicate_location_is_relative_to_the_submitted_document() {
    let input = b"{\n  \"nested\": {\n    \"key\": 1,\n    \"key\": 2\n  }\n}";
    let failure = parse_json(input).expect_err("nested duplicate must be rejected");
    let start = failure
        .diagnostic
        .range
        .expect("duplicate detection should be located")
        .start;
    let nested_offset = input
        .windows(b"{\n    \"key\"".len())
        .position(|window| window == b"{\n    \"key\"")
        .expect("test input should contain the nested object");

    assert!(start.offset as usize > nested_offset);
    assert_eq!(start.line, 3);
    assert_eq!(start.column, 4);
}

#[test]
fn retains_number_spelling_only_in_source_bytes() {
    let document = parse_json(NUMBERS).expect("owned fixture should parse");

    assert!(
        NUMBERS
            .windows(b"1e+02".len())
            .any(|bytes| bytes == b"1e+02")
    );
    assert!(NUMBERS.windows(2).any(|bytes| bytes == b"-0"));
    assert_eq!(document.value["exponent"].as_f64(), Some(100.0));
    assert_eq!(document.value["integer"].as_i64(), Some(1));
    assert_eq!(document.value["decimal"].as_f64(), Some(1.0));
    assert!(
        document.value["negative_zero"]
            .as_f64()
            .expect("negative zero should parse as a float")
            .is_sign_negative()
    );
    assert_eq!(
        document.value["large_integer"].to_string(),
        "1.2345678901234568e+29"
    );
    assert_eq!(
        document.value["long_fraction"].to_string(),
        "0.12345678901234568"
    );
    assert_ne!(document.value["exponent"].to_string(), "1e+02");
}

#[test]
fn malformed_json_returns_owned_diagnostic() {
    let input = b"{\n  @\n}";
    let failure = parse_json(input).expect_err("input is malformed");

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
    assert_eq!(failure.diagnostic.pointer, None);
    assert_eq!(failure.diagnostic.rdf_term, None);
    let start = failure
        .diagnostic
        .range
        .expect("syntax error should be located")
        .start;
    assert_eq!(
        start,
        SourcePosition {
            offset: 4,
            line: 1,
            column: 2,
        }
    );
}

#[test]
fn eof_errors_retain_exact_positions() {
    let empty = parse_json(b"").expect_err("empty input is incomplete JSON");
    let empty_start = empty.diagnostic.range.expect("EOF should be located").start;
    assert_eq!(
        empty_start,
        SourcePosition {
            offset: 0,
            line: 0,
            column: 0,
        }
    );

    let truncated = parse_json(b"{\n").expect_err("object is incomplete");
    let truncated_start = truncated
        .diagnostic
        .range
        .expect("EOF should be located")
        .start;
    assert_eq!(
        truncated_start,
        SourcePosition {
            offset: 2,
            line: 1,
            column: 0,
        }
    );
}

#[test]
fn invalid_utf8_returns_owned_diagnostic() {
    let input = [b'{', b'"', 0xff, b'"', b':', b'1', b'}'];
    let failure = parse_json(&input).expect_err("input is not valid UTF-8 JSON");

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
    let start = failure
        .diagnostic
        .range
        .expect("invalid UTF-8 should be located")
        .start;
    assert_eq!(
        start,
        SourcePosition {
            offset: 2,
            line: 0,
            column: 2,
        }
    );
}

#[test]
fn invalid_utf8_precedes_unrelated_number_processing() {
    let mut input = b"[1e400,\"".to_vec();
    let invalid_offset = input.len();
    input.push(0xff);
    input.extend_from_slice(b"\"]");
    let failure = parse_json(&input).expect_err("input is not valid UTF-8 JSON");
    let start = failure
        .diagnostic
        .range
        .expect("invalid UTF-8 should be located")
        .start;

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(start.offset as usize, invalid_offset);
}

#[test]
fn locations_use_zero_based_byte_offsets_and_columns() {
    let unicode = parse_json("{\"é\": @}".as_bytes()).expect_err("input is malformed");
    let unicode_start = unicode
        .diagnostic
        .range
        .expect("syntax error should be located")
        .start;
    assert_eq!(
        unicode_start,
        SourcePosition {
            offset: 7,
            line: 0,
            column: 7,
        }
    );

    let crlf = parse_json(b"{\r\n  @\r\n}").expect_err("input is malformed");
    let crlf_start = crlf
        .diagnostic
        .range
        .expect("syntax error should be located")
        .start;
    assert_eq!(
        crlf_start,
        SourcePosition {
            offset: 5,
            line: 1,
            column: 2,
        }
    );
}

#[test]
fn owned_boundary_dto_round_trips_through_serde_json() {
    let range = SourceRange {
        start: SourcePosition {
            offset: 4,
            line: 0,
            column: 4,
        },
        end: SourcePosition {
            offset: 5,
            line: 0,
            column: 5,
        },
    };
    let report = ProbeReport {
        source: SourceDocument::new(NUMBERS),
        diagnostics: vec![ProbeDiagnostic {
            stage: DiagnosticStage::Json,
            message: "example".to_owned(),
            range: Some(range),
            pointer: Some("/é/a~1b~0c".to_owned()),
            rdf_term: Some("https://example.test/predicate".to_owned()),
        }],
        quads: vec![RdfQuadSummary {
            subject: RdfNodeSummary {
                kind: RdfNodeKind::Named,
                value: "https://example.test/subject".to_owned(),
            },
            predicate: "https://example.test/label".to_owned(),
            object: RdfObjectSummary::Literal {
                value: "alpha".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                language: None,
            },
            graph_name: None,
        }],
        metrics: ProbeMetrics {
            json: JsonStructureMetrics {
                max_nesting_depth: 2,
                max_object_members: 4,
                total_values: 9,
                decoded_member_name_bytes: 42,
            },
            rdf_term_bytes: 128,
        },
    };

    let encoded = serde_json::to_vec(&report).expect("report should serialize");
    let decoded: ProbeReport = serde_json::from_slice(&encoded).expect("report should deserialize");

    assert_eq!(decoded, report);
    assert_eq!(
        decoded.diagnostics[0].pointer.as_deref(),
        Some("/é/a~1b~0c")
    );
    assert_eq!(
        decoded.diagnostics[0].rdf_term.as_deref(),
        Some("https://example.test/predicate")
    );
}

#[test]
fn evaluates_json_pointer_without_claiming_a_source_map() {
    let array = parse_json(br#"[null,true,1,"value"]"#).expect("array should parse");
    assert_eq!(array.value.as_array().map(Vec::len), Some(4));

    let document = parse_json(NUMBERS).expect("owned fixture should parse");
    assert_eq!(
        document
            .value
            .pointer("/é/a~1b~0c")
            .and_then(serde_json::Value::as_i64),
        Some(7)
    );
}
