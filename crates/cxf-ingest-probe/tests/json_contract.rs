use cxf_ingest_probe::{
    DiagnosticStage, ProbeDiagnostic, ProbeReport, RdfNodeKind, RdfNodeSummary, RdfObjectSummary,
    RdfQuadSummary, SourceDocument, SourcePosition, SourceRange, parse_json,
};

const NUMBERS: &[u8] = include_bytes!("fixtures/numbers.json");

#[test]
fn retains_exact_input_bytes() {
    let document = parse_json(NUMBERS).expect("owned fixture should parse");

    assert_eq!(document.source.as_bytes(), NUMBERS);
    assert_eq!(document.value["é"]["a/b~c"], 7);
}

#[test]
fn records_serde_duplicate_member_behavior() {
    let input = br#"{"value":1,"value":2,"nested":{"key":1,"key":2}}"#;
    let document = parse_json(input).expect("duplicate members are valid JSON syntax");

    assert_eq!(document.source.as_bytes(), input);
    assert_eq!(document.value["value"], 2);
    assert_eq!(document.value["nested"]["key"], 2);
}

#[test]
fn retains_number_spelling_only_in_source_bytes() {
    let document = parse_json(NUMBERS).expect("owned fixture should parse");

    assert!(
        NUMBERS
            .windows(b"1e+02".len())
            .any(|bytes| bytes == b"1e+02")
    );
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
            offset: 3,
            line: 0,
            column: 3,
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
    };

    let encoded = serde_json::to_vec(&report).expect("report should serialize");
    let decoded: ProbeReport = serde_json::from_slice(&encoded).expect("report should deserialize");

    assert_eq!(decoded, report);
}

#[test]
fn parses_minimal_arrays_and_escaped_json_pointer_tokens() {
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
