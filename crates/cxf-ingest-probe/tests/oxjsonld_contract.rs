#![cfg(feature = "oxigraph")]

use cxf_ingest_probe::{DiagnosticStage, RdfNodeKind, RdfObjectSummary, parse_json_ld};

const EMBEDDED_CONTEXT: &[u8] =
    include_bytes!("../../cxf-json/tests/fixtures/embedded-context.jsonld");
const NAMED_GRAPH: &[u8] = include_bytes!("../../cxf-json/tests/fixtures/named-graph.jsonld");
const REMOTE_CONTEXT: &[u8] = include_bytes!("../../cxf-json/tests/fixtures/remote-context.jsonld");

#[test]
fn repeated_parse_reports_match_within_process() {
    let first = parse_json_ld(EMBEDDED_CONTEXT).expect("embedded context should parse");
    let second = parse_json_ld(EMBEDDED_CONTEXT).expect("embedded context should parse");

    assert_eq!(first, second);
}

#[test]
fn converts_embedded_context_to_owned_rdf_summaries() {
    let report = parse_json_ld(EMBEDDED_CONTEXT).expect("embedded context should parse");

    assert_eq!(report.source.as_bytes(), EMBEDDED_CONTEXT);
    assert_eq!(report.quads.len(), 5);
    assert!(report.metrics.json.max_nesting_depth >= 2);
    assert!(report.metrics.rdf_term_bytes > 0);
    assert!(report.quads.iter().all(|quad| {
        quad.subject.kind == RdfNodeKind::Named
            && quad.subject.value == "https://example.test/subject"
            && quad.graph_name.is_none()
    }));
    assert!(report.quads.iter().any(|quad| {
        quad.predicate == "https://example.test/label"
            && matches!(
                &quad.object,
                RdfObjectSummary::Literal {
                    value,
                    datatype,
                    language: None,
                } if value == "alpha"
                    && datatype == "http://www.w3.org/2001/XMLSchema#string"
            )
    }));
    assert!(report.quads.iter().any(|quad| {
        quad.predicate == "https://example.test/count"
            && matches!(
                &quad.object,
                RdfObjectSummary::Literal {
                    value,
                    datatype,
                    language: None,
                } if value == "7"
                    && datatype == "http://www.w3.org/2001/XMLSchema#integer"
            )
    }));
    assert!(report.quads.iter().any(|quad| {
        quad.predicate == "https://example.test/link"
            && matches!(
                &quad.object,
                RdfObjectSummary::Node(node)
                    if node.kind == RdfNodeKind::Named
                        && node.value == "https://example.test/target"
            )
    }));
    assert!(report.quads.iter().any(|quad| {
        quad.predicate == "https://example.test/name"
            && matches!(
                &quad.object,
                RdfObjectSummary::Literal {
                    value,
                    language: Some(language),
                    ..
                } if value == "example" && language == "en"
            )
    }));
    assert!(
        report
            .quads
            .iter()
            .any(|quad| quad.predicate == "https://example.test/unknown")
    );
}

#[test]
fn retains_named_graph_identity() {
    let report = parse_json_ld(NAMED_GRAPH).expect("named graph should parse");

    assert_eq!(report.quads.len(), 1);
    let graph_name = report.quads[0]
        .graph_name
        .as_ref()
        .expect("quad should retain its named graph");
    assert_eq!(graph_name.kind, RdfNodeKind::Named);
    assert_eq!(graph_name.value, "https://example.test/graph");
}

#[test]
fn converts_anonymous_subject_without_exposing_oxigraph_id_type() {
    let input = br#"{
        "@context": {"label": "https://example.test/label"},
        "label": "anonymous"
    }"#;
    let report = parse_json_ld(input).expect("anonymous subject should parse");

    assert_eq!(report.quads.len(), 1);
    assert_eq!(report.quads[0].subject.kind, RdfNodeKind::Blank);
    assert!(!report.quads[0].subject.value.is_empty());
}

#[test]
fn rejects_remote_context_without_network_loader() {
    let failure = parse_json_ld(REMOTE_CONTEXT).expect_err("remote context should fail");

    assert_eq!(failure.source.as_bytes(), REMOTE_CONTEXT);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::JsonLd);
    assert!(failure.diagnostic.message.contains("remote context"));
    assert!(failure.diagnostic.range.is_none());
    assert_eq!(failure.diagnostic.pointer, None);
    assert_eq!(failure.diagnostic.rdf_term, None);
    let metrics = failure
        .metrics
        .expect("JSON-LD failure should retain completed preflight metrics");
    assert!(metrics.json.total_values > 0);
    assert_eq!(metrics.rdf_term_bytes, 0);
}

#[test]
fn json_syntax_error_is_rejected_before_json_ld_processing() {
    let input = b"{\n  @\n}";
    let failure = parse_json_ld(input).expect_err("input is malformed");
    let range = failure
        .diagnostic
        .range
        .expect("JSON syntax error should carry a position");

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
    assert_eq!(failure.metrics, None);
    assert_eq!(range.start.offset, 4);
    assert_eq!(range.start.line, 1);
    assert_eq!(range.start.column, 2);
}

#[test]
fn rejects_duplicate_members_before_json_ld_processing() {
    let input = br#"{
        "@context": {"label": "https://example.test/first", "label": "https://example.test/second"},
        "label": "value"
    }"#;
    let failure = parse_json_ld(input).expect_err("duplicate context term must be rejected");

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
    assert!(
        failure
            .diagnostic
            .message
            .contains("duplicate object member")
    );
    assert_eq!(failure.diagnostic.pointer, None);
    assert_eq!(failure.diagnostic.rdf_term, None);
}

#[test]
fn rejects_invalid_surrogate_escape_before_json_ld_processing() {
    let input = br#"{
        "@context": {"value": "https://example.test/value"},
        "value": "\uDEAD"
    }"#;
    let failure = parse_json_ld(input).expect_err("invalid surrogate must fail JSON syntax");

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
    assert!(failure.diagnostic.range.is_some());
}

#[test]
fn number_spelling_remains_only_in_submitted_bytes() {
    let input = br#"{
        "@context": {"value": "https://example.test/value"},
        "@id": "https://example.test/subject",
        "value": [1, 1.0, 1e+02, -0, 1e400]
    }"#;
    let report = parse_json_ld(input).expect("numeric JSON-LD should parse");

    assert_eq!(report.source.as_bytes(), input);
    assert_eq!(report.quads.len(), 5);
    assert!(report.quads.iter().all(|quad| {
        matches!(
            &quad.object,
            RdfObjectSummary::Literal { value, .. }
                if value == "0" || value == "1" || value == "100" || value == "1.0E400"
        )
    }));
    assert!(report.quads.iter().all(|quad| {
        !matches!(
            &quad.object,
            RdfObjectSummary::Literal { value, .. }
                if value == "1.0" || value == "1e+02" || value == "-0" || value == "1e400"
        )
    }));
}

#[test]
fn duplicate_preflight_does_not_set_a_nesting_limit() {
    let depth = 256;
    let mut input = vec![b'['; depth];
    input.extend_from_slice(b"{}");
    input.extend(std::iter::repeat_n(b']', depth));

    let report = parse_json_ld(&input).expect("nesting policy belongs to W-011");
    assert!(report.quads.is_empty());
}
