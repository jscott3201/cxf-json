#![cfg(feature = "oxigraph")]

use cxf_ingest_probe::{RdfObjectSummary, parse_json_ld};

const COMPACT: &[u8] = include_bytes!("fixtures/cxf-compact.jsonld");
const FULL_IRI: &[u8] = include_bytes!("fixtures/cxf-full-iri.jsonld");
const ORDER_A: &[u8] = include_bytes!("fixtures/cxf-order-a.jsonld");
const ORDER_B: &[u8] = include_bytes!("fixtures/cxf-order-b.jsonld");
const CONTEXT_LIST: &[u8] = include_bytes!("fixtures/cxf-context-list.jsonld");

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const S231P: &str = "http://data.ashrae.org/S231P#";

#[test]
fn compact_and_full_iri_cxf_forms_produce_equal_rdf() {
    let compact = parse_json_ld(COMPACT).expect("compact CXF should parse");
    let full_iri = parse_json_ld(FULL_IRI).expect("full-IRI CXF should parse");

    assert_eq!(compact.quads, full_iri.quads);
    assert!(compact.quads.iter().any(|quad| {
        quad.predicate == RDF_TYPE
            && matches!(
                &quad.object,
                RdfObjectSummary::Node(node) if node.value == format!("{S231P}Block")
            )
    }));
    assert!(compact.quads.iter().any(|quad| {
        quad.predicate == format!("{S231P}value")
            && matches!(
                &quad.object,
                RdfObjectSummary::Literal {
                    value,
                    datatype,
                    language: None,
                } if value == "0.5"
                    && datatype == "http://www.w3.org/2001/XMLSchema#double"
            )
    }));
}

#[test]
fn rdf_conversion_does_not_preserve_cxf_array_order() {
    let first_then_second = parse_json_ld(ORDER_A).expect("ordered CXF should parse");
    let second_then_first = parse_json_ld(ORDER_B).expect("reordered CXF should parse");

    assert_eq!(first_then_second.quads, second_then_first.quads);
}

#[test]
fn later_inline_context_definition_wins() {
    let report = parse_json_ld(CONTEXT_LIST).expect("context list should parse");

    assert!(report.quads.iter().any(|quad| {
        quad.predicate == RDF_TYPE
            && matches!(
                &quad.object,
                RdfObjectSummary::Node(node) if node.value == format!("{S231P}Block")
            )
    }));
    assert!(
        report
            .quads
            .iter()
            .all(|quad| !quad.predicate.contains("obsolete.example")
                && !matches!(
                    &quad.object,
                    RdfObjectSummary::Node(node) if node.value.contains("obsolete.example")
                ))
    );
}
