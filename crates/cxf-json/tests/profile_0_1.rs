use std::error::Error;

use cxf_json::{
    AdmissionError, DocumentIri, DocumentIriError, ParseError, ParseOptions, SourceDocument,
    SourcePosition, SourceRange,
};

#[test]
fn source_document_owns_exact_bytes_without_debugging_content() {
    let bytes = b"sensitive CXF bytes".to_vec();
    let source = SourceDocument::from_bytes(bytes.clone());

    assert_eq!(source.as_bytes(), bytes);
    assert_eq!(source.len(), bytes.len());
    assert!(!source.is_empty());
    assert!(SourceDocument::from_bytes(Vec::new()).is_empty());

    let debug = format!("{source:?}");
    assert!(debug.contains(&format!("len: {}", bytes.len())));
    assert!(!debug.contains("sensitive"));
}

#[test]
fn document_iri_is_absolute_and_retains_spelling() {
    let value = "https://user:secret@example.test/CXF/%7Einput?token=value#part";
    let iri = DocumentIri::parse(value).expect("absolute IRI should parse");

    assert_eq!(iri.as_str(), value);
    assert_eq!(iri.to_string(), value);
    assert_eq!(
        DocumentIri::parse("../relative").expect_err("relative IRI must fail"),
        DocumentIriError
    );

    let debug = format!("{iri:?}");
    assert_eq!(debug, "DocumentIri(\"<redacted>\")");
    assert!(!debug.contains("secret"));
    assert!(!format!("{:?}", ParseOptions::new().with_document_iri(iri)).contains("secret"));
}

#[test]
fn source_locations_are_zero_based_byte_values() {
    let start = SourcePosition::new(7, 1, 2);
    let end = SourcePosition::new(11, 1, 6);
    let range = SourceRange::new(start, end).expect("ordered offsets should form a range");

    assert_eq!(start.offset(), 7);
    assert_eq!(start.line(), 1);
    assert_eq!(start.column(), 2);
    assert_eq!(range.start(), start);
    assert_eq!(range.end(), end);
    assert_eq!(SourceRange::new(end, start), None);
}

#[test]
fn parse_options_default_to_no_document_iri() {
    let options = ParseOptions::new();
    assert_eq!(options.document_iri(), None);
    assert_eq!(
        options.max_input_bytes(),
        ParseOptions::DEFAULT_MAX_INPUT_BYTES
    );
    assert_eq!(ParseOptions::DEFAULT_MAX_INPUT_BYTES, 1_048_576);
    assert_eq!(ParseOptions::DEFAULT_MAX_JSON_NESTING_DEPTH, 64);
    assert_eq!(ParseOptions::DEFAULT_MAX_JSON_OBJECT_MEMBERS, 4_096);
    assert_eq!(ParseOptions::DEFAULT_MAX_JSON_VALUES, 65_536);
    assert_eq!(ParseOptions::DEFAULT_MAX_DECODED_MEMBER_NAME_BYTES, 262_144);
    assert_eq!(ParseOptions::DEFAULT_MAX_RDF_QUADS, 65_536);
    assert_eq!(ParseOptions::DEFAULT_MAX_RETAINED_RDF_TERM_BYTES, 8_388_608);
    assert_eq!(options.max_json_nesting_depth(), 64);
    assert_eq!(options.max_json_object_members(), 4_096);
    assert_eq!(options.max_json_values(), 65_536);
    assert_eq!(options.max_decoded_member_name_bytes(), 262_144);
    assert_eq!(options.max_rdf_quads(), 65_536);
    assert_eq!(options.max_retained_rdf_term_bytes(), 8_388_608);
    assert_eq!(ParseOptions::default(), options);

    let iri = DocumentIri::parse("https://example.test/input").expect("IRI should parse");
    let options = options
        .with_document_iri(iri.clone())
        .with_max_input_bytes(42)
        .with_max_json_nesting_depth(1)
        .with_max_json_object_members(2)
        .with_max_json_values(3)
        .with_max_decoded_member_name_bytes(4)
        .with_max_rdf_quads(5)
        .with_max_retained_rdf_term_bytes(6);
    assert_eq!(options.document_iri(), Some(&iri));
    assert_eq!(options.max_input_bytes(), 42);
    assert_eq!(options.max_json_nesting_depth(), 1);
    assert_eq!(options.max_json_object_members(), 2);
    assert_eq!(options.max_json_values(), 3);
    assert_eq!(options.max_decoded_member_name_bytes(), 4);
    assert_eq!(options.max_rdf_quads(), 5);
    assert_eq!(options.max_retained_rdf_term_bytes(), 6);
}

#[test]
fn input_byte_admission_has_an_inclusive_boundary() {
    let options = ParseOptions::new().with_max_input_bytes(3);

    assert_eq!(
        SourceDocument::admit_bytes(b"ab", &options)
            .expect("input below the limit should be admitted")
            .as_bytes(),
        b"ab"
    );
    assert_eq!(
        SourceDocument::admit_bytes(b"abc", &options)
            .expect("input at the limit should be admitted")
            .as_bytes(),
        b"abc"
    );

    let error = SourceDocument::admit_bytes(b"abcd", &options)
        .expect_err("input above the limit should be rejected");
    assert_eq!(error.actual_bytes(), 4);
    assert_eq!(error.max_input_bytes(), 3);
}

#[test]
fn zero_limit_admits_only_empty_input() {
    let options = ParseOptions::new().with_max_input_bytes(0);

    assert!(
        SourceDocument::admit_bytes(b"", &options)
            .expect("empty input should meet a zero-byte limit")
            .is_empty()
    );
    assert_eq!(
        SourceDocument::admit_bytes(b"x", &options)
            .expect_err("nonempty input should exceed a zero-byte limit")
            .actual_bytes(),
        1
    );
}

#[test]
fn admission_checks_bytes_without_parsing() {
    let options = ParseOptions::new().with_max_input_bytes(8);

    assert_eq!(
        SourceDocument::admit_bytes(b"{", &options)
            .expect("malformed JSON is still within the admission boundary")
            .as_bytes(),
        b"{"
    );
    assert_eq!(
        SourceDocument::admit_bytes(&[0xff], &options)
            .expect("invalid UTF-8 is still within the admission boundary")
            .as_bytes(),
        &[0xff]
    );
}

#[test]
fn admission_copies_only_accepted_input() {
    let options = ParseOptions::new().with_max_input_bytes(3);
    let mut accepted = b"abc".to_vec();
    let source = SourceDocument::admit_bytes(&accepted, &options)
        .expect("input at the limit should be admitted");
    accepted[0] = b'z';
    assert_eq!(source.as_bytes(), b"abc");

    let rejected = b"sensitive".to_vec();
    let error = SourceDocument::admit_bytes(&rejected, &options)
        .expect_err("oversized input should be rejected");
    assert_eq!(rejected, b"sensitive");
    assert!(!format!("{error:?}").contains("sensitive"));
    assert!(!error.to_string().contains("sensitive"));
}

#[test]
fn raw_source_construction_does_not_apply_parse_options() {
    let bytes = vec![0; 4];
    let pointer = bytes.as_ptr();
    let source = SourceDocument::from_bytes(bytes);

    assert_eq!(source.len(), 4);
    assert_eq!(source.as_bytes().as_ptr(), pointer);
}

#[test]
fn public_errors_implement_standard_error() {
    fn assert_error<T: Error>() {}
    fn assert_copy<T: Copy>() {}

    assert_error::<DocumentIriError>();
    assert_error::<AdmissionError>();
    assert_error::<ParseError>();
    assert_copy::<AdmissionError>();
}
