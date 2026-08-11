#[cfg(cxf_json_semantic_harness)]
use cxf_ingest_probe::production_harness::{
    OutcomeKind, RETAINED_VALUES, observe, options, retained_values_input, revision_word,
};
#[cfg(cxf_json_semantic_harness)]
use cxf_json::ParseOptions;

#[cfg(cxf_json_semantic_harness)]
const COMPACT: &[u8] = include_bytes!("../../cxf-json/tests/fixtures/cxf-compact.jsonld");
#[cfg(cxf_json_semantic_harness)]
const REMOTE_CONTEXT: &[u8] = include_bytes!("../../cxf-json/tests/fixtures/remote-context.jsonld");

#[cfg(cxf_json_semantic_harness)]
#[unsafe(no_mangle)]
pub extern "C" fn cxf_benchmark_revision_0() -> u32 {
    revision_word(0)
}

#[cfg(cxf_json_semantic_harness)]
#[unsafe(no_mangle)]
pub extern "C" fn cxf_benchmark_revision_1() -> u32 {
    revision_word(1)
}

#[cfg(cxf_json_semantic_harness)]
#[unsafe(no_mangle)]
pub extern "C" fn cxf_benchmark_revision_2() -> u32 {
    revision_word(2)
}

#[cfg(cxf_json_semantic_harness)]
#[unsafe(no_mangle)]
pub extern "C" fn cxf_benchmark_revision_3() -> u32 {
    revision_word(3)
}

#[cfg(cxf_json_semantic_harness)]
#[unsafe(no_mangle)]
pub extern "C" fn cxf_benchmark_revision_4() -> u32 {
    revision_word(4)
}

#[cfg(cxf_json_semantic_harness)]
fn main() {
    let compact = observe(COMPACT, &options());
    assert_eq!(compact.outcome, OutcomeKind::Success);
    assert_eq!(compact.source_matches_input, Some(true));
    assert_eq!(compact.returned_rdf_quads, 20);

    let workload = retained_values_input(RETAINED_VALUES);
    let retained = observe(&workload, &options());
    assert_eq!(retained.outcome, OutcomeKind::Success);
    assert_eq!(retained.returned_rdf_quads, RETAINED_VALUES as u64);
    assert_eq!(
        retained
            .metrics
            .expect("success should report metrics")
            .emitted_rdf_quads,
        RETAINED_VALUES as u64
    );

    let quad_limit = observe(COMPACT, &options().with_max_rdf_quads(0));
    assert_eq!(quad_limit.outcome, OutcomeKind::RdfQuadLimit);
    assert_eq!(quad_limit.returned_rdf_quads, 0);

    let term_limit = observe(COMPACT, &options().with_max_retained_rdf_term_bytes(0));
    assert_eq!(term_limit.outcome, OutcomeKind::RetainedRdfTermBytesLimit);
    assert_eq!(term_limit.returned_rdf_quads, 0);

    let malformed = observe(b"{", &options());
    assert_eq!(malformed.outcome, OutcomeKind::JsonSyntax);
    assert_eq!(malformed.source_matches_input, Some(true));

    let remote = observe(REMOTE_CONTEXT, &options());
    assert_eq!(remote.outcome, OutcomeKind::JsonLd);
    assert_eq!(remote.source_matches_input, Some(true));

    let missing_iri = observe(b"{}", &ParseOptions::new());
    assert_eq!(missing_iri.outcome, OutcomeKind::MissingDocumentIri);

    let anonymous = observe(br#"{"https://example.test/label":"anonymous"}"#, &options());
    assert_eq!(anonymous.outcome, OutcomeKind::Success);
    assert_eq!(anonymous.returned_rdf_quads, 1);
}

#[cfg(not(cxf_json_semantic_harness))]
fn main() {
    panic!("set CXF_JSON_SEMANTIC_HARNESS=1 to run the production semantic smoke test")
}
