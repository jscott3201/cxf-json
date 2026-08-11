#[path = "support/parser_seeds.rs"]
mod parser_seeds;

use cxf_ingest_probe::{ProbeFailure, parse_json};

#[test]
fn reviewed_parser_seeds_retain_source_and_valid_ranges() {
    for (name, input) in parser_seeds::PARSER_SEEDS {
        match parse_json(input) {
            Ok(document) => {
                assert_eq!(document.source.as_bytes(), *input, "{name}");
                assert!(document.metrics.total_values > 0, "{name}");
                assert!(document.metrics.max_nesting_depth <= document.metrics.total_values);
                assert!(document.metrics.max_object_members <= document.metrics.total_values);
                assert!(document.metrics.decoded_member_name_bytes <= input.len());
            }
            Err(failure) => assert_failure(name, input, &failure),
        }
    }
}

fn assert_failure(name: &str, input: &[u8], failure: &ProbeFailure) {
    assert_eq!(failure.source.as_bytes(), input, "{name}");
    if let Some(range) = failure.diagnostic.range {
        assert!(range.start.offset <= range.end.offset, "{name}");
        assert!(range.end.offset <= input.len() as u64, "{name}");
    }
}
