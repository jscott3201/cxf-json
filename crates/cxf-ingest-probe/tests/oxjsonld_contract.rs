#![cfg(feature = "oxigraph")]

use cxf_ingest_probe::{DiagnosticStage, RdfNodeKind, RdfObjectSummary, parse_json_ld};

const EMBEDDED_CONTEXT: &[u8] = include_bytes!("fixtures/embedded-context.jsonld");
const NAMED_GRAPH: &[u8] = include_bytes!("fixtures/named-graph.jsonld");
const REMOTE_CONTEXT: &[u8] = include_bytes!("fixtures/remote-context.jsonld");

#[test]
fn converts_embedded_context_to_owned_rdf_summaries() {
    let report = parse_json_ld(EMBEDDED_CONTEXT).expect("embedded context should parse");

    assert_eq!(report.source.as_bytes(), EMBEDDED_CONTEXT);
    assert_eq!(report.quads.len(), 5);
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
}

#[test]
fn json_syntax_error_retains_oxjsonld_byte_position() {
    let input = b"{\n  @\n}";
    let failure = parse_json_ld(input).expect_err("input is malformed");
    let range = failure
        .diagnostic
        .range
        .expect("JSON syntax error should carry a position");

    assert_eq!(failure.source.as_bytes(), input);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::JsonLd);
    assert_eq!(range.start.offset, 4);
    assert_eq!(range.start.line, 1);
    assert_eq!(range.start.column, 2);
}
