#![no_main]

use libfuzzer_sys::{Corpus, fuzz_target};

#[cfg(fuzzing)]
use std::sync::LazyLock;

#[cfg(fuzzing)]
use cxf_json::{
    DocumentIri, ParseOptions,
    test_support::{Observation, OutcomeKind, observe},
};

#[cfg(fuzzing)]
const MAX_INPUT_BYTES: usize = 1_048_576;

#[cfg(fuzzing)]
static DEFAULT_OPTIONS: LazyLock<ParseOptions> = LazyLock::new(options);
#[cfg(fuzzing)]
static ZERO_QUAD_OPTIONS: LazyLock<ParseOptions> =
    LazyLock::new(|| options().with_max_rdf_quads(0));
#[cfg(fuzzing)]
static ZERO_TERM_OPTIONS: LazyLock<ParseOptions> =
    LazyLock::new(|| options().with_max_retained_rdf_term_bytes(0));

#[cfg(fuzzing)]
fn options() -> ParseOptions {
    ParseOptions::new().with_document_iri(
        DocumentIri::parse("https://fuzz.example/input").expect("fixed fuzz IRI should be valid"),
    )
}

#[cfg(fuzzing)]
fn check_observation(observation: Observation, options: &ParseOptions) {
    assert_ne!(observation.source_matches_input, Some(false));
    assert_ne!(observation.outcome, OutcomeKind::AdmissionLimit);

    if let Some(metrics) = observation.metrics {
        if observation.outcome == OutcomeKind::RdfQuadLimit {
            assert_eq!(
                metrics.emitted_rdf_quads,
                options
                    .max_rdf_quads()
                    .checked_add(1)
                    .expect("fuzz limits leave room for the rejecting quad")
            );
        } else {
            assert!(metrics.emitted_rdf_quads <= options.max_rdf_quads());
        }
        assert!(metrics.retained_rdf_term_bytes <= options.max_retained_rdf_term_bytes());
        assert!(metrics.max_nesting_depth <= options.max_json_nesting_depth());
        assert!(metrics.max_object_members <= options.max_json_object_members());
        assert!(metrics.total_values <= options.max_json_values());
        assert!(
            metrics.decoded_member_name_bytes <= options.max_decoded_member_name_bytes()
        );
        if observation.outcome == OutcomeKind::Success {
            assert_eq!(observation.returned_rdf_quads, metrics.emitted_rdf_quads);
        } else {
            assert_eq!(observation.returned_rdf_quads, 0);
        }
    } else {
        assert_ne!(observation.outcome, OutcomeKind::Success);
        assert_eq!(observation.returned_rdf_quads, 0);
    }
}

#[cfg(fuzzing)]
fuzz_target!(|input: &[u8]| -> Corpus {
    if input.len() > MAX_INPUT_BYTES {
        return Corpus::Reject;
    }

    let options = match input.len() % 3 {
        0 => &*DEFAULT_OPTIONS,
        1 => &*ZERO_QUAD_OPTIONS,
        _ => &*ZERO_TERM_OPTIONS,
    };
    check_observation(observe(input, options), options);
    Corpus::Keep
});

#[cfg(not(fuzzing))]
fuzz_target!(|input: &[u8]| -> Corpus {
    let _ = input;
    Corpus::Reject
});
