#![no_main]

use cxf_ingest_probe::parse_json;
use libfuzzer_sys::{Corpus, fuzz_target};

const MAX_INPUT_BYTES: usize = 1_048_576;

fuzz_target!(|input: &[u8]| -> Corpus {
    if input.len() > MAX_INPUT_BYTES {
        return Corpus::Reject;
    }

    match parse_json(input) {
        Ok(document) => {
            assert_eq!(document.source.as_bytes(), input);
            assert!(document.metrics.total_values > 0);
            assert!(document.metrics.max_nesting_depth <= document.metrics.total_values);
            assert!(document.metrics.max_object_members <= document.metrics.total_values);
            assert!(document.metrics.decoded_member_name_bytes <= input.len());
        }
        Err(failure) => {
            assert_eq!(failure.source.as_bytes(), input);
            if let Some(range) = failure.diagnostic.range {
                assert!(range.start.offset <= range.end.offset);
                assert!(range.end.offset <= input.len() as u64);
            }
        }
    }

    Corpus::Keep
});
