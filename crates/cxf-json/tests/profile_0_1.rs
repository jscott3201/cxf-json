use std::error::Error;

use cxf_json::{
    DocumentIri, DocumentIriError, ParseError, ParseOptions, SourceDocument, SourcePosition,
    SourceRange,
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

    let iri = DocumentIri::parse("https://example.test/input").expect("IRI should parse");
    let options = options.with_document_iri(iri.clone());
    assert_eq!(options.document_iri(), Some(&iri));
}

#[test]
fn public_errors_implement_standard_error() {
    fn assert_error<T: Error>() {}

    assert_error::<DocumentIriError>();
    assert_error::<ParseError>();
}
