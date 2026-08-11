#[path = "../tests/support/parser_seeds.rs"]
mod parser_seeds;

use cxf_ingest_probe::{
    DiagnosticStage, RdfNodeKind, RdfObjectSummary, StressExpected, parse_json, parse_json_ld,
    resource_stress_cases,
};

const COMPACT: &[u8] = include_bytes!("../tests/fixtures/cxf-compact.jsonld");
const FULL_IRI: &[u8] = include_bytes!("../tests/fixtures/cxf-full-iri.jsonld");
const ORDER_A: &[u8] = include_bytes!("../tests/fixtures/cxf-order-a.jsonld");
const ORDER_B: &[u8] = include_bytes!("../tests/fixtures/cxf-order-b.jsonld");
const CONTEXT_LIST: &[u8] = include_bytes!("../tests/fixtures/cxf-context-list.jsonld");
const REMOTE_CONTEXT: &[u8] = include_bytes!("../tests/fixtures/remote-context.jsonld");
const ANONYMOUS: &[u8] = br#"{
  "@context": {"label": "https://example.test/label"},
  "label": "anonymous"
}"#;

fn main() {
    let compact = parse_json_ld(COMPACT).expect("compact CXF should parse");
    let full_iri = parse_json_ld(FULL_IRI).expect("full-IRI CXF should parse");
    assert_eq!(compact.quads, full_iri.quads);
    assert_eq!(compact.quads.len(), 20);

    let first_then_second = parse_json_ld(ORDER_A).expect("ordered CXF should parse");
    let second_then_first = parse_json_ld(ORDER_B).expect("reordered CXF should parse");
    assert_eq!(first_then_second.quads, second_then_first.quads);

    let context = parse_json_ld(CONTEXT_LIST).expect("context list should parse");
    assert_eq!(context.quads.len(), 2);
    assert!(context.quads.iter().any(|quad| {
        quad.predicate == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
            && matches!(
                &quad.object,
                RdfObjectSummary::Node(node)
                    if node.value == "http://data.ashrae.org/S231P#Block"
            )
    }));

    let anonymous = parse_json_ld(ANONYMOUS).expect("anonymous CXF node should parse");
    assert_eq!(anonymous.quads.len(), 1);
    assert_eq!(anonymous.quads[0].subject.kind, RdfNodeKind::Blank);

    let remote = parse_json_ld(REMOTE_CONTEXT).expect_err("remote context should fail");
    assert_eq!(remote.diagnostic.stage, DiagnosticStage::JsonLd);
    assert_eq!(
        remote.diagnostic.message,
        "No LoadDocumentCallback has been set to load remote contexts"
    );

    let duplicate = br#"{"@context":{},"@id":"first","@id":"second"}"#;
    let failure = parse_json_ld(duplicate).expect_err("duplicates should fail before JSON-LD");
    assert_eq!(failure.source.as_bytes(), duplicate);
    assert_eq!(failure.diagnostic.stage, DiagnosticStage::Json);
    assert!(
        failure
            .diagnostic
            .message
            .contains("duplicate object member")
    );
    assert_eq!(failure.diagnostic.pointer, None);
    assert_eq!(failure.diagnostic.rdf_term, None);

    let large_number = br#"{
      "@context": {"value": "https://example.test/value"},
      "@id": "https://example.test/subject",
      "value": 1e400
    }"#;
    let numeric = parse_json_ld(large_number).expect("large exponent should reach JSON-LD");
    assert!(numeric.quads.iter().any(|quad| {
        matches!(
            &quad.object,
            RdfObjectSummary::Literal { value, .. } if value == "1.0E400"
        )
    }));

    let depth = 256;
    let mut nested = vec![b'['; depth];
    nested.extend_from_slice(b"{}");
    nested.extend(std::iter::repeat_n(b']', depth));
    let nested = parse_json_ld(&nested).expect("nesting policy belongs to W-011");
    assert!(nested.quads.is_empty());

    for (name, input) in parser_seeds::PARSER_SEEDS {
        match parse_json(input) {
            Ok(document) => assert_eq!(document.source.as_bytes(), *input, "{name}"),
            Err(failure) => assert_eq!(failure.source.as_bytes(), *input, "{name}"),
        }
    }

    for case in resource_stress_cases() {
        match (case.expected, parse_json_ld(&case.input)) {
            (StressExpected::Success { quad_count }, Ok(report)) => {
                assert_eq!(report.quads.len(), quad_count, "{}", case.name);
            }
            (StressExpected::Failure { stage }, Err(failure)) => {
                assert_eq!(failure.diagnostic.stage, stage, "{}", case.name);
            }
            (_, result) => panic!(
                "unexpected resource-stress result for {}: {result:?}",
                case.name
            ),
        }
    }
}
