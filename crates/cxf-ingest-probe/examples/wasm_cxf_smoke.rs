use cxf_ingest_probe::{DiagnosticStage, RdfNodeKind, RdfObjectSummary, parse_json_ld};

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
}
