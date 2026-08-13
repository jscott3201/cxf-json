//! Executable W-016 inventory for the current private failure and finding
//! allocation. These cases pin cross-stage evidence without creating a public
//! conformance API or a source-to-RDF correspondence contract.

use std::collections::{BTreeMap, BTreeSet};

use oxrdf::{GraphName, NamedOrBlankNode, Term as RdfTerm};

use crate::{
    DiagnosticSeverity, DocumentIri, ParseOptions,
    json::{JsonFailureKind, PreflightFailure},
    projection::{OpaqueValue, Projection, ProjectionCode, Term},
    semantic::{ComposedDocument, SemanticFailure, SemanticFailureKind, ingest_project_validate},
    validation::ValidationCode,
};

const REMOTE_CONTEXT: &[u8] = include_bytes!("../../tests/fixtures/remote-context.jsonld");
const COMPOSITION_BOUNDARY: &[u8] =
    include_bytes!("../../tests/projection/cxf-proj-composition.jsonld");
const ARTIFACT: &[u8] = include_bytes!("../../tests/projection/cxf-proj-artifact.jsonld");
const ENCODED_REFERENCE: &[u8] = include_bytes!("../../tests/projection/cxf-proj-encoded.jsonld");
const PROJECTION_BUNDLE: &[u8] = include_bytes!("../../tests/w016/cxf-w016-projection.jsonld");
const VALIDATION_BUNDLE: &[u8] = include_bytes!("../../tests/w016/cxf-w016-validation.jsonld");
const NAMESPACE_BUNDLE: &[u8] = include_bytes!("../../tests/w016/cxf-w016-namespaces.jsonld");
const INVALID_UTF8: &[u8] = b"{\"\x80\":0}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureKind {
    Admission,
    Json(JsonFailureKind),
    Semantic(SemanticFailureKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceExpectation {
    NotAdmitted,
    Exact,
}

struct FailureCase {
    id: &'static str,
    input: &'static [u8],
    options: fn() -> ParseOptions,
    kind: FailureKind,
    source: SourceExpectation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProjectionExpectation {
    code: &'static str,
    node: Option<&'static str>,
    node_index: Option<usize>,
    context: Option<&'static str>,
    source: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ValidationExpectation {
    code: &'static str,
    severity: DiagnosticSeverity,
    node: Option<&'static str>,
    node_index: Option<usize>,
    source: &'static str,
    source_occurrence: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExtensionExpectation {
    node_index: usize,
    predicate: &'static str,
    kind: &'static str,
    source: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RdfObjectExpectation {
    Named(&'static str),
    Literal {
        value: &'static str,
        datatype: &'static str,
        language: Option<&'static str>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RdfExpectation {
    subject: &'static str,
    predicate: &'static str,
    object: RdfObjectExpectation,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RdfNode {
    Named(String),
    Blank(String),
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RdfObject {
    Named(String),
    Blank(String),
    Literal {
        value: String,
        datatype: String,
        language: Option<String>,
    },
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RdfRecord {
    subject: RdfNode,
    predicate: String,
    object: RdfObject,
    graph: Option<RdfNode>,
}

macro_rules! rdf_named {
    ($subject:literal, $predicate:literal, $object:literal) => {
        RdfExpectation {
            subject: $subject,
            predicate: $predicate,
            object: RdfObjectExpectation::Named($object),
        }
    };
}

macro_rules! rdf_string {
    ($subject:literal, $predicate:literal, $value:literal) => {
        RdfExpectation {
            subject: $subject,
            predicate: $predicate,
            object: RdfObjectExpectation::Literal {
                value: $value,
                datatype: "http://www.w3.org/2001/XMLSchema#string",
                language: None,
            },
        }
    };
}

macro_rules! rdf_integer {
    ($subject:literal, $predicate:literal, $value:literal) => {
        RdfExpectation {
            subject: $subject,
            predicate: $predicate,
            object: RdfObjectExpectation::Literal {
                value: $value,
                datatype: "http://www.w3.org/2001/XMLSchema#integer",
                language: None,
            },
        }
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Witness {
    Extensions {
        node: &'static str,
        expected: &'static [ExtensionExpectation],
    },
    OpaqueValue {
        node: &'static str,
        occurrence: usize,
        decoded: Option<&'static str>,
        source: &'static str,
    },
    Text {
        node: &'static str,
        term: Term,
        value: &'static str,
    },
    Edge {
        index: usize,
        subject: &'static str,
        subject_index: usize,
        predicate_namespace: &'static str,
        predicate: Term,
        target: &'static str,
        target_node: Option<&'static str>,
        source: &'static str,
        count: usize,
    },
    NodeCount {
        id: &'static str,
        count: usize,
    },
}

struct ConstructedCase {
    id: &'static str,
    input: &'static [u8],
    rdf: &'static [RdfExpectation],
    projection: &'static [ProjectionExpectation],
    validation: &'static [ValidationExpectation],
    witnesses: &'static [Witness],
}

fn options() -> ParseOptions {
    ParseOptions::new().with_document_iri(
        DocumentIri::parse("https://example.test/w016/input")
            .expect("W-016 document IRI should be valid"),
    )
}

fn no_document_iri() -> ParseOptions {
    ParseOptions::new()
}

fn input_limit() -> ParseOptions {
    ParseOptions::new().with_max_input_bytes(1)
}

fn nesting_limit() -> ParseOptions {
    options().with_max_json_nesting_depth(0)
}

fn object_member_limit() -> ParseOptions {
    options().with_max_json_object_members(0)
}

fn value_limit() -> ParseOptions {
    options().with_max_json_values(0)
}

fn decoded_name_limit() -> ParseOptions {
    options().with_max_decoded_member_name_bytes(0)
}

fn rdf_quad_limit() -> ParseOptions {
    options().with_max_rdf_quads(0)
}

fn retained_term_limit() -> ParseOptions {
    options().with_max_retained_rdf_term_bytes(0)
}

macro_rules! literal_registry {
    ($function:ident, $variants:ident, $type:ty, [$($variant:path => $literal:literal),+ $(,)?]) => {
        const $variants: &[$type] = &[$($variant),+];

        const fn $function(value: $type) -> &'static str {
            match value {
                $($variant => $literal),+
            }
        }
    };
}

literal_registry!(
    json_code,
    JSON_VARIANTS,
    JsonFailureKind,
    [
        JsonFailureKind::InvalidUtf8 => "invalid-utf8",
        JsonFailureKind::Syntax => "syntax",
        JsonFailureKind::DuplicateMember => "duplicate-member",
        JsonFailureKind::NestingLimit => "nesting-limit",
        JsonFailureKind::ObjectMemberLimit => "object-member-limit",
        JsonFailureKind::ValueLimit => "value-limit",
        JsonFailureKind::DecodedMemberNameBytesLimit => "decoded-member-name-bytes-limit",
    ]
);

literal_registry!(
    semantic_code,
    SEMANTIC_VARIANTS,
    SemanticFailureKind,
    [
        SemanticFailureKind::MissingDocumentIri => "missing-document-iri",
        SemanticFailureKind::JsonLd => "json-ld",
        SemanticFailureKind::RdfQuadLimit => "rdf-quad-limit",
        SemanticFailureKind::RetainedRdfTermBytesLimit => "retained-rdf-term-bytes-limit",
    ]
);

literal_registry!(
    projection_code,
    PROJECTION_VARIANTS,
    ProjectionCode,
    [
        ProjectionCode::RootNotObject => "CXF-P-000",
        ProjectionCode::WeaklyTypedNode => "CXF-P-001",
        ProjectionCode::ConflictingTypes => "CXF-P-002",
        ProjectionCode::ValueArtifact => "CXF-P-003",
        ProjectionCode::MalformedReference => "CXF-P-004",
        ProjectionCode::UnresolvedReference => "CXF-P-005",
        ProjectionCode::DuplicateNodeId => "CXF-P-006",
    ]
);

literal_registry!(
    validation_code,
    VALIDATION_VARIANTS,
    ValidationCode,
    [
        ValidationCode::ConnectionEndpointNotConnector => "CXF-V-001",
        ValidationCode::ConnectionDataTypeMismatch => "CXF-V-002",
        ValidationCode::DataTypeOutsideDomain => "CXF-V-003",
        ValidationCode::GroupingOutsideBlock => "CXF-V-004",
        ValidationCode::ParameterValueAbsent => "CXF-V-005",
        ValidationCode::LegacyNamespaceGeneration => "CXF-C-001",
        ValidationCode::UnregisteredFamilyNamespace => "CXF-C-002",
        ValidationCode::ShadowedPrefix => "CXF-C-003",
    ]
);

#[test]
fn allocated_codes_are_literal_unique_and_exhaustive() {
    assert_unique(JSON_VARIANTS.iter().copied().map(json_code));
    assert_unique(SEMANTIC_VARIANTS.iter().copied().map(semantic_code));
    assert_unique(PROJECTION_VARIANTS.iter().copied().map(projection_code));
    for code in PROJECTION_VARIANTS {
        let literal = projection_code(*code);
        assert_eq!(code.as_str(), literal);
    }
    assert_unique(VALIDATION_VARIANTS.iter().copied().map(validation_code));
    assert_unique(
        PROJECTION_VARIANTS
            .iter()
            .copied()
            .map(projection_code)
            .chain(VALIDATION_VARIANTS.iter().copied().map(validation_code)),
    );
    for code in VALIDATION_VARIANTS {
        let literal = validation_code(*code);
        assert_eq!(code.as_str(), literal);
    }
}

#[test]
fn construction_failure_inventory() {
    let cases = [
        FailureCase {
            id: "admission-limit",
            input: b"{}",
            options: input_limit,
            kind: FailureKind::Admission,
            source: SourceExpectation::NotAdmitted,
        },
        FailureCase {
            id: "invalid-utf8",
            input: INVALID_UTF8,
            options,
            kind: FailureKind::Json(JsonFailureKind::InvalidUtf8),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "json-syntax",
            input: b"{",
            options,
            kind: FailureKind::Json(JsonFailureKind::Syntax),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "duplicate-member",
            input: br#"{"a":0,"a":1}"#,
            options,
            kind: FailureKind::Json(JsonFailureKind::DuplicateMember),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "nesting-limit",
            input: b"[]",
            options: nesting_limit,
            kind: FailureKind::Json(JsonFailureKind::NestingLimit),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "object-member-limit",
            input: br#"{"a":0}"#,
            options: object_member_limit,
            kind: FailureKind::Json(JsonFailureKind::ObjectMemberLimit),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "value-limit",
            input: b"0",
            options: value_limit,
            kind: FailureKind::Json(JsonFailureKind::ValueLimit),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "decoded-name-limit",
            input: br#"{"a":0}"#,
            options: decoded_name_limit,
            kind: FailureKind::Json(JsonFailureKind::DecodedMemberNameBytesLimit),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "missing-document-iri",
            input: b"{}",
            options: no_document_iri,
            kind: FailureKind::Semantic(SemanticFailureKind::MissingDocumentIri),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "json-ld",
            input: REMOTE_CONTEXT,
            options,
            kind: FailureKind::Semantic(SemanticFailureKind::JsonLd),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "malformed-local-context",
            input: br#"{"@context":42}"#,
            options,
            kind: FailureKind::Semantic(SemanticFailureKind::JsonLd),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "rdf-quad-limit",
            input: COMPOSITION_BOUNDARY,
            options: rdf_quad_limit,
            kind: FailureKind::Semantic(SemanticFailureKind::RdfQuadLimit),
            source: SourceExpectation::Exact,
        },
        FailureCase {
            id: "retained-term-limit",
            input: COMPOSITION_BOUNDARY,
            options: retained_term_limit,
            kind: FailureKind::Semantic(SemanticFailureKind::RetainedRdfTermBytesLimit),
            source: SourceExpectation::Exact,
        },
    ];

    for case in &cases {
        assert_failure(case);
    }
    assert_failure_coverage(&cases);
}

#[test]
fn constructed_evidence_inventory() {
    let cases = [
        ConstructedCase {
            id: "root-not-object",
            input: b"[]",
            rdf: &[],
            projection: &[ProjectionExpectation {
                code: "CXF-P-000",
                node: None,
                node_index: None,
                context: None,
                source: "[]",
            }],
            validation: &[],
            witnesses: &[],
        },
        ConstructedCase {
            id: "projection-code-bundle",
            input: PROJECTION_BUNDLE,
            rdf: &[
                rdf_string!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#containsBlock",
                    "malformed-a"
                ),
                rdf_string!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#containsBlock",
                    "malformed-b"
                ),
                rdf_named!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#hasInstance",
                    "https://example.test/w016/missing-one"
                ),
                rdf_named!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#hasInstance",
                    "https://example.test/w016/missing-two"
                ),
                rdf_named!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#hasInstance",
                    "https://example.test/w016/missing-one"
                ),
                rdf_integer!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#value",
                    "1"
                ),
                rdf_string!(
                    "https://example.test/w016/conflict",
                    "http://data.ashrae.org/S231#value",
                    "{ terms: [Array] }"
                ),
                rdf_named!(
                    "https://example.test/w016/conflict",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#Constant"
                ),
                rdf_named!(
                    "https://example.test/w016/conflict",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#Parameter"
                ),
                rdf_named!(
                    "https://example.test/w016/conflict",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#Parameter"
                ),
            ],
            projection: &[
                ProjectionExpectation {
                    code: "CXF-P-001",
                    node: Some("weak"),
                    node_index: Some(0),
                    context: None,
                    source: "{\"@id\":\"weak\"}",
                },
                ProjectionExpectation {
                    code: "CXF-P-003",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: None,
                    source: "\"{ terms: [Array] }\"",
                },
                ProjectionExpectation {
                    code: "CXF-P-004",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: Some("S231:containsBlock"),
                    source: "\"malformed-a\"",
                },
                ProjectionExpectation {
                    code: "CXF-P-004",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: Some("S231:containsBlock"),
                    source: "\"malformed-b\"",
                },
                ProjectionExpectation {
                    code: "CXF-P-002",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: None,
                    source: "{\"@id\":\"conflict\",\"@type\":[\"S231:Parameter\",\"S231:Constant\"],\"S231:value\":\"{ terms: [Array] }\",\"S231:containsBlock\":[\"malformed-a\",\"malformed-b\"],\"S231:hasInstance\":[{\"@id\":\"missing-one\"},{\"@id\":\"missing-two\"},{\"@id\":\"missing-one\"}]}",
                },
                ProjectionExpectation {
                    code: "CXF-P-006",
                    node: Some("conflict"),
                    node_index: Some(2),
                    context: None,
                    source: "{\"@id\":\"conflict\",\"@type\":\"S231:Parameter\",\"S231:value\":1}",
                },
                ProjectionExpectation {
                    code: "CXF-P-005",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: Some("missing-one"),
                    source: "{\"@id\":\"missing-one\"}",
                },
                ProjectionExpectation {
                    code: "CXF-P-005",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: Some("missing-two"),
                    source: "{\"@id\":\"missing-two\"}",
                },
                ProjectionExpectation {
                    code: "CXF-P-005",
                    node: Some("conflict"),
                    node_index: Some(1),
                    context: Some("missing-one"),
                    source: "{\"@id\":\"missing-one\"}",
                },
            ],
            validation: &[],
            witnesses: &[
                Witness::Extensions {
                    node: "conflict",
                    expected: &[
                        ExtensionExpectation {
                            node_index: 1,
                            predicate: "S231:containsBlock",
                            kind: "string",
                            source: "\"malformed-a\"",
                        },
                        ExtensionExpectation {
                            node_index: 1,
                            predicate: "S231:containsBlock",
                            kind: "string",
                            source: "\"malformed-b\"",
                        },
                    ],
                },
                Witness::OpaqueValue {
                    node: "conflict",
                    occurrence: 0,
                    decoded: Some("{ terms: [Array] }"),
                    source: "\"{ terms: [Array] }\"",
                },
                Witness::OpaqueValue {
                    node: "conflict",
                    occurrence: 1,
                    decoded: None,
                    source: "1",
                },
                Witness::Edge {
                    index: 0,
                    subject: "conflict",
                    subject_index: 1,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::HasInstance,
                    target: "missing-one",
                    target_node: None,
                    source: "{\"@id\":\"missing-one\"}",
                    count: 2,
                },
                Witness::Edge {
                    index: 1,
                    subject: "conflict",
                    subject_index: 1,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::HasInstance,
                    target: "missing-two",
                    target_node: None,
                    source: "{\"@id\":\"missing-two\"}",
                    count: 1,
                },
                Witness::Edge {
                    index: 2,
                    subject: "conflict",
                    subject_index: 1,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::HasInstance,
                    target: "missing-one",
                    target_node: None,
                    source: "{\"@id\":\"missing-one\"}",
                    count: 2,
                },
                Witness::NodeCount {
                    id: "conflict",
                    count: 2,
                },
            ],
        },
        ConstructedCase {
            id: "validation-rule-bundle",
            input: VALIDATION_BUNDLE,
            rdf: &[
                rdf_named!(
                    "https://example.test/w016/blockEndpoint",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#CompositeBlock"
                ),
                rdf_named!(
                    "https://example.test/w016/blockSource",
                    "http://data.ashrae.org/S231#connectedTo",
                    "https://example.test/w016/blockTarget"
                ),
                rdf_named!(
                    "https://example.test/w016/blockSource",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#CompositeBlock"
                ),
                rdf_named!(
                    "https://example.test/w016/blockTarget",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#CompositeBlock"
                ),
                rdf_named!(
                    "https://example.test/w016/blockType",
                    "http://data.ashrae.org/S231#isOfDataType",
                    "http://data.ashrae.org/S231#Real"
                ),
                rdf_named!(
                    "https://example.test/w016/blockType",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#CompositeBlock"
                ),
                rdf_named!(
                    "https://example.test/w016/connectorGroup",
                    "http://data.ashrae.org/S231#hasParameter",
                    "https://example.test/w016/pVal"
                ),
                rdf_named!(
                    "https://example.test/w016/connectorGroup",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#RealInput"
                ),
                rdf_named!(
                    "https://example.test/w016/integerMismatch",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#IntegerInput"
                ),
                rdf_named!(
                    "https://example.test/w016/pMissing",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#Parameter"
                ),
                rdf_integer!(
                    "https://example.test/w016/pVal",
                    "http://data.ashrae.org/S231#value",
                    "1"
                ),
                rdf_named!(
                    "https://example.test/w016/pVal",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#Parameter"
                ),
                rdf_named!(
                    "https://example.test/w016/realMismatch",
                    "http://data.ashrae.org/S231#connectedTo",
                    "https://example.test/w016/integerMismatch"
                ),
                rdf_named!(
                    "https://example.test/w016/realMismatch",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#RealOutput"
                ),
                rdf_named!(
                    "https://example.test/w016/realToBlock",
                    "http://data.ashrae.org/S231#connectedTo",
                    "https://example.test/w016/blockEndpoint"
                ),
                rdf_named!(
                    "https://example.test/w016/realToBlock",
                    "http://data.ashrae.org/S231#isConnectedTo",
                    "https://example.test/w016/blockEndpoint"
                ),
                rdf_named!(
                    "https://example.test/w016/realToBlock",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231#RealOutput"
                ),
            ],
            projection: &[],
            validation: &[
                ValidationExpectation {
                    code: "CXF-V-005",
                    severity: DiagnosticSeverity::Warning,
                    node: Some("pMissing"),
                    node_index: Some(0),
                    source: "{\"@id\":\"pMissing\",\"@type\":\"S231:Parameter\"}",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-V-003",
                    severity: DiagnosticSeverity::Error,
                    node: Some("blockType"),
                    node_index: Some(1),
                    source: "{\"@id\":\"S231:Real\"}",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-V-004",
                    severity: DiagnosticSeverity::Error,
                    node: Some("connectorGroup"),
                    node_index: Some(2),
                    source: "{\"@id\":\"pVal\"}",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-V-001",
                    severity: DiagnosticSeverity::Error,
                    node: Some("blockEndpoint"),
                    node_index: Some(5),
                    source: "{\"@id\":\"blockEndpoint\"}",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-V-001",
                    severity: DiagnosticSeverity::Error,
                    node: Some("blockEndpoint"),
                    node_index: Some(5),
                    source: "{\"@id\":\"blockEndpoint\"}",
                    source_occurrence: 1,
                },
                ValidationExpectation {
                    code: "CXF-V-002",
                    severity: DiagnosticSeverity::Error,
                    node: Some("realMismatch"),
                    node_index: Some(6),
                    source: "{\"@id\":\"integerMismatch\"}",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-V-001",
                    severity: DiagnosticSeverity::Error,
                    node: Some("blockSource"),
                    node_index: Some(8),
                    source: "{\"@id\":\"blockTarget\"}",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-V-001",
                    severity: DiagnosticSeverity::Error,
                    node: Some("blockTarget"),
                    node_index: Some(9),
                    source: "{\"@id\":\"blockTarget\"}",
                    source_occurrence: 0,
                },
            ],
            witnesses: &[
                Witness::Edge {
                    index: 0,
                    subject: "blockType",
                    subject_index: 1,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::IsOfDataType,
                    target: "S231:Real",
                    target_node: None,
                    source: "{\"@id\":\"S231:Real\"}",
                    count: 1,
                },
                Witness::Edge {
                    index: 1,
                    subject: "connectorGroup",
                    subject_index: 2,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::HasParameter,
                    target: "pVal",
                    target_node: Some("pVal"),
                    source: "{\"@id\":\"pVal\"}",
                    count: 1,
                },
                Witness::Edge {
                    index: 2,
                    subject: "realToBlock",
                    subject_index: 4,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::ConnectedTo,
                    target: "blockEndpoint",
                    target_node: Some("blockEndpoint"),
                    source: "{\"@id\":\"blockEndpoint\"}",
                    count: 1,
                },
                Witness::Edge {
                    index: 3,
                    subject: "realToBlock",
                    subject_index: 4,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::IsConnectedTo,
                    target: "blockEndpoint",
                    target_node: Some("blockEndpoint"),
                    source: "{\"@id\":\"blockEndpoint\"}",
                    count: 1,
                },
                Witness::Edge {
                    index: 4,
                    subject: "realMismatch",
                    subject_index: 6,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::ConnectedTo,
                    target: "integerMismatch",
                    target_node: Some("integerMismatch"),
                    source: "{\"@id\":\"integerMismatch\"}",
                    count: 1,
                },
                Witness::Edge {
                    index: 5,
                    subject: "blockSource",
                    subject_index: 8,
                    predicate_namespace: "http://data.ashrae.org/S231#",
                    predicate: Term::ConnectedTo,
                    target: "blockTarget",
                    target_node: Some("blockTarget"),
                    source: "{\"@id\":\"blockTarget\"}",
                    count: 1,
                },
            ],
        },
        ConstructedCase {
            id: "namespace-policy-bundle",
            input: NAMESPACE_BUNDLE,
            rdf: &[],
            projection: &[],
            validation: &[
                ValidationExpectation {
                    code: "CXF-C-001",
                    severity: DiagnosticSeverity::Warning,
                    node: None,
                    node_index: None,
                    source: "\"https://data.ashrae.org/S231P#\"",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-C-002",
                    severity: DiagnosticSeverity::Warning,
                    node: None,
                    node_index: None,
                    source: "\"http://data.ashrae.org/S231R#\"",
                    source_occurrence: 0,
                },
                ValidationExpectation {
                    code: "CXF-C-003",
                    severity: DiagnosticSeverity::Warning,
                    node: None,
                    node_index: None,
                    source: "\"https://example.test/not-cxf#\"",
                    source_occurrence: 0,
                },
            ],
            witnesses: &[],
        },
        ConstructedCase {
            id: "broken-value-artifact",
            input: ARTIFACT,
            rdf: &[
                rdf_named!(
                    "https://example.test/cxf#Scaling.gain",
                    "http://data.ashrae.org/S231P#isOfDataType",
                    "http://data.ashrae.org/S231P#Real"
                ),
                rdf_string!(
                    "https://example.test/cxf#Scaling.gain",
                    "http://data.ashrae.org/S231P#label",
                    "gain"
                ),
                rdf_string!(
                    "https://example.test/cxf#Scaling.gain",
                    "http://data.ashrae.org/S231P#value",
                    "{ terms: [Array] }"
                ),
                rdf_named!(
                    "https://example.test/cxf#Scaling.gain",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231P#Parameter"
                ),
                rdf_named!(
                    "https://example.test/cxf#Scaling.offset",
                    "http://data.ashrae.org/S231P#isOfDataType",
                    "http://data.ashrae.org/S231P#Real"
                ),
                rdf_string!(
                    "https://example.test/cxf#Scaling.offset",
                    "http://data.ashrae.org/S231P#label",
                    "offset"
                ),
                rdf_string!(
                    "https://example.test/cxf#Scaling.offset",
                    "http://data.ashrae.org/S231P#value",
                    "yMin - uMin * gain + offset"
                ),
                rdf_named!(
                    "https://example.test/cxf#Scaling.offset",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231P#Parameter"
                ),
            ],
            projection: &[ProjectionExpectation {
                code: "CXF-P-003",
                node: Some("ex:Scaling.gain"),
                node_index: Some(0),
                context: None,
                source: "\"{ terms: [Array] }\"",
            }],
            validation: &[],
            witnesses: &[
                Witness::Text {
                    node: "ex:Scaling.gain",
                    term: Term::Label,
                    value: "gain",
                },
                Witness::OpaqueValue {
                    node: "ex:Scaling.gain",
                    occurrence: 0,
                    decoded: Some("{ terms: [Array] }"),
                    source: "\"{ terms: [Array] }\"",
                },
                Witness::Text {
                    node: "ex:Scaling.offset",
                    term: Term::Label,
                    value: "offset",
                },
                Witness::OpaqueValue {
                    node: "ex:Scaling.offset",
                    occurrence: 0,
                    decoded: Some("yMin - uMin * gain + offset"),
                    source: "\"yMin - uMin * gain + offset\"",
                },
            ],
        },
        ConstructedCase {
            id: "encoded-reference-positive-control",
            input: ENCODED_REFERENCE,
            rdf: &[
                rdf_named!(
                    "https://example.test/cxf#MultiIn.mulMax.u%5B1%5D",
                    "http://data.ashrae.org/S231P#isOfDataType",
                    "http://data.ashrae.org/S231P#Real"
                ),
                rdf_string!(
                    "https://example.test/cxf#MultiIn.mulMax.u%5B1%5D",
                    "http://data.ashrae.org/S231P#label",
                    "mulMax.u[1]"
                ),
                rdf_named!(
                    "https://example.test/cxf#MultiIn.mulMax.u%5B1%5D",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231P#RealInput"
                ),
                rdf_named!(
                    "https://example.test/cxf#MultiIn.u",
                    "http://data.ashrae.org/S231P#isConnectedTo",
                    "https://example.test/cxf#MultiIn.mulMax.u%5B1%5D"
                ),
                rdf_named!(
                    "https://example.test/cxf#MultiIn.u",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://data.ashrae.org/S231P#RealInput"
                ),
            ],
            projection: &[],
            validation: &[],
            witnesses: &[Witness::Edge {
                index: 0,
                subject: "ex:MultiIn.u",
                subject_index: 0,
                predicate_namespace: "http://data.ashrae.org/S231P#",
                predicate: Term::IsConnectedTo,
                target: "ex:MultiIn.mulMax.u%5B1%5D",
                target_node: Some("ex:MultiIn.mulMax.u%5B1%5D"),
                source: "{ \"@id\": \"ex:MultiIn.mulMax.u%5B1%5D\" }",
                count: 1,
            }],
        },
    ];

    for case in &cases {
        assert_constructed(case);
    }
    assert_constructed_coverage(&cases);
}

fn assert_failure(case: &FailureCase) {
    let failure = {
        let input = case.input.to_vec();
        ingest_project_validate(&input, &(case.options)()).expect_err(case.id)
    };
    match (case.kind, case.source, failure) {
        (
            FailureKind::Admission,
            SourceExpectation::NotAdmitted,
            SemanticFailure::Preflight(PreflightFailure::Admission(_)),
        ) => {}
        (
            FailureKind::Json(expected),
            SourceExpectation::Exact,
            SemanticFailure::Preflight(PreflightFailure::Json(error)),
        ) => {
            assert_eq!(error.kind(), expected, "{} failure kind", case.id);
            assert_eq!(
                error.source_document().as_bytes(),
                case.input,
                "{} source retention",
                case.id
            );
        }
        (
            FailureKind::Semantic(expected),
            SourceExpectation::Exact,
            SemanticFailure::Semantic(error),
        ) => {
            assert_eq!(error.kind(), expected, "{} failure kind", case.id);
            assert_eq!(
                error.source_document().as_bytes(),
                case.input,
                "{} source retention",
                case.id
            );
        }
        (expected_kind, expected_source, actual) => panic!(
            "{} expected {expected_kind:?}/{expected_source:?}, got {actual:?}",
            case.id
        ),
    }
}

fn assert_constructed(case: &ConstructedCase) {
    let document = {
        let input = case.input.to_vec();
        ingest_project_validate(&input, &options())
            .unwrap_or_else(|failure| panic!("{} should construct: {failure:?}", case.id))
    };
    assert_eq!(
        document.source_document().as_bytes(),
        case.input,
        "{} source retention",
        case.id
    );
    let actual_rdf = rdf_multiset(document.quads().iter().map(rdf_record));
    let expected_rdf = rdf_multiset(case.rdf.iter().copied().map(expected_rdf_record));
    assert_eq!(actual_rdf, expected_rdf, "{} RDF multiset", case.id);

    assert_projection(case, document.projection());
    assert_validation(case, &document);
    for witness in case.witnesses {
        assert_witness(case.id, &document, *witness);
    }
}

fn assert_projection(case: &ConstructedCase, projection: &Projection) {
    let actual = projection
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code().as_str(),
                diagnostic.node(),
                diagnostic
                    .node()
                    .and_then(|index| projection.nodes()[index].id_spelling())
                    .map(str::to_owned),
                diagnostic.context().map(str::to_owned),
                projection
                    .source_slice(diagnostic.token())
                    .map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    let expected = case
        .projection
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.node_index,
                diagnostic.node.map(str::to_owned),
                diagnostic.context.map(str::to_owned),
                Some(diagnostic.source.to_owned()),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "{} projection evidence", case.id);
}

fn assert_validation(case: &ConstructedCase, document: &ComposedDocument) {
    let projection = document.projection();
    let actual = document
        .validation_findings()
        .iter()
        .map(|finding| {
            (
                finding.code().as_str(),
                finding.severity(),
                finding.node(),
                finding
                    .node()
                    .and_then(|index| projection.nodes()[index].id_spelling())
                    .map(str::to_owned),
                projection.source_slice(finding.token()).map(str::to_owned),
                source_occurrence(
                    document.source_document().as_bytes(),
                    finding.token(),
                    projection.source_slice(finding.token()).unwrap_or(""),
                ),
            )
        })
        .collect::<Vec<_>>();
    let expected = case
        .validation
        .iter()
        .map(|finding| {
            (
                finding.code,
                finding.severity,
                finding.node_index,
                finding.node.map(str::to_owned),
                Some(finding.source.to_owned()),
                finding.source_occurrence,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "{} validation evidence", case.id);
}

fn assert_witness(id: &str, document: &ComposedDocument, witness: Witness) {
    let projection = document.projection();
    match witness {
        Witness::Extensions { node, expected } => {
            let actual = first_node(projection, node)
                .extensions()
                .iter()
                .map(|extension| {
                    (
                        extension.predicate().to_owned(),
                        extension.node(),
                        extension.kind(),
                        projection
                            .source_slice(extension.token())
                            .unwrap_or("")
                            .to_owned(),
                    )
                })
                .collect::<Vec<_>>();
            let expected = expected
                .iter()
                .map(|extension| {
                    (
                        extension.predicate.to_owned(),
                        Some(extension.node_index),
                        extension.kind,
                        extension.source.to_owned(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{id} complete extension evidence");
        }
        Witness::OpaqueValue {
            node,
            occurrence,
            decoded,
            source,
        } => {
            let value = nth_node(projection, node, occurrence)
                .value()
                .unwrap_or_else(|| panic!("{id} missing opaque value on {node}"));
            let text = match value {
                OpaqueValue::Literal { text, .. } => text.as_deref(),
                OpaqueValue::TypedObject { value_text, .. } => value_text.as_deref(),
                OpaqueValue::OtherObject { .. }
                | OpaqueValue::Array { .. }
                | OpaqueValue::Null { .. } => None,
            };
            assert_eq!(text, decoded, "{id} decoded opaque value");
            assert_eq!(
                projection.source_slice(value.token()),
                Some(source),
                "{id} opaque source"
            );
        }
        Witness::Text { node, term, value } => assert_eq!(
            first_node(projection, node).text(term),
            Some(value),
            "{id} retained text property"
        ),
        Witness::Edge {
            index,
            subject,
            subject_index,
            predicate_namespace,
            predicate,
            target,
            target_node,
            source,
            count,
        } => {
            let edge = projection
                .edges()
                .get(index)
                .unwrap_or_else(|| panic!("{id} missing edge index {index}"));
            assert_eq!(edge.subject(), subject_index, "{id} edge subject index");
            assert_eq!(
                projection.nodes()[edge.subject()].id_spelling(),
                Some(subject),
                "{id} edge subject"
            );
            assert_eq!(
                edge.predicate().namespace_iri(),
                predicate_namespace,
                "{id} edge predicate namespace"
            );
            assert_eq!(edge.predicate().term(), predicate, "{id} edge predicate");
            assert_eq!(edge.target_spelling(), target, "{id} edge target spelling");
            let matches = projection
                .edges()
                .iter()
                .filter(|edge| {
                    edge.subject() == subject_index
                        && projection.nodes()[edge.subject()].id_spelling() == Some(subject)
                        && edge.predicate().namespace_iri() == predicate_namespace
                        && edge.predicate().term() == predicate
                        && edge.target_spelling() == target
                })
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), count, "{id} edge multiplicity");
            assert_eq!(
                edge.target()
                    .and_then(|index| projection.nodes()[index].id_spelling()),
                target_node,
                "{id} target node"
            );
            assert_eq!(
                projection.source_slice(edge.token()),
                Some(source),
                "{id} resolved edge source"
            );
        }
        Witness::NodeCount { id: node_id, count } => assert_eq!(
            projection
                .nodes()
                .iter()
                .filter(|node| node.id_spelling() == Some(node_id))
                .count(),
            count,
            "{id} retained node count"
        ),
    }
}

fn first_node<'a>(projection: &'a Projection, id: &str) -> &'a crate::projection::NodeProjection {
    nth_node(projection, id, 0)
}

fn nth_node<'a>(
    projection: &'a Projection,
    id: &str,
    occurrence: usize,
) -> &'a crate::projection::NodeProjection {
    projection
        .nodes()
        .iter()
        .filter(|node| node.id_spelling() == Some(id))
        .nth(occurrence)
        .unwrap_or_else(|| panic!("missing node {id} occurrence {occurrence}"))
}

fn assert_unique(codes: impl IntoIterator<Item = &'static str>) {
    let mut observed = BTreeSet::new();
    for code in codes {
        assert!(observed.insert(code), "duplicate allocated code {code}");
    }
}

fn rdf_multiset(records: impl IntoIterator<Item = RdfRecord>) -> BTreeMap<RdfRecord, usize> {
    let mut multiset = BTreeMap::new();
    for record in records {
        *multiset.entry(record).or_default() += 1;
    }
    multiset
}

fn rdf_record(quad: &oxrdf::Quad) -> RdfRecord {
    RdfRecord {
        subject: match &quad.subject {
            NamedOrBlankNode::NamedNode(node) => RdfNode::Named(node.as_str().to_owned()),
            NamedOrBlankNode::BlankNode(node) => RdfNode::Blank(node.as_str().to_owned()),
        },
        predicate: quad.predicate.as_str().to_owned(),
        object: match &quad.object {
            RdfTerm::NamedNode(node) => RdfObject::Named(node.as_str().to_owned()),
            RdfTerm::BlankNode(node) => RdfObject::Blank(node.as_str().to_owned()),
            RdfTerm::Literal(literal) => RdfObject::Literal {
                value: literal.value().to_owned(),
                datatype: literal.datatype().as_str().to_owned(),
                language: literal.language().map(str::to_owned),
            },
        },
        graph: match &quad.graph_name {
            GraphName::NamedNode(node) => Some(RdfNode::Named(node.as_str().to_owned())),
            GraphName::BlankNode(node) => Some(RdfNode::Blank(node.as_str().to_owned())),
            GraphName::DefaultGraph => None,
        },
    }
}

fn expected_rdf_record(expected: RdfExpectation) -> RdfRecord {
    RdfRecord {
        subject: RdfNode::Named(expected.subject.to_owned()),
        predicate: expected.predicate.to_owned(),
        object: match expected.object {
            RdfObjectExpectation::Named(node) => RdfObject::Named(node.to_owned()),
            RdfObjectExpectation::Literal {
                value,
                datatype,
                language,
            } => RdfObject::Literal {
                value: value.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
            },
        },
        graph: None,
    }
}

fn assert_failure_coverage(cases: &[FailureCase]) {
    let observed = cases
        .iter()
        .map(|case| match case.kind {
            FailureKind::Admission => "admission",
            FailureKind::Json(kind) => json_code(kind),
            FailureKind::Semantic(kind) => semantic_code(kind),
        })
        .collect::<BTreeSet<_>>();
    let expected = std::iter::once("admission")
        .chain(JSON_VARIANTS.iter().copied().map(json_code))
        .chain(SEMANTIC_VARIANTS.iter().copied().map(semantic_code))
        .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected, "construction inventory coverage");
}

fn assert_constructed_coverage(cases: &[ConstructedCase]) {
    let projection = cases
        .iter()
        .flat_map(|case| case.projection.iter().map(|finding| finding.code))
        .collect::<BTreeSet<_>>();
    let expected_projection = PROJECTION_VARIANTS
        .iter()
        .copied()
        .map(projection_code)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        projection, expected_projection,
        "projection inventory coverage"
    );

    let validation = cases
        .iter()
        .flat_map(|case| case.validation.iter().map(|finding| finding.code))
        .collect::<BTreeSet<_>>();
    let expected_validation = VALIDATION_VARIANTS
        .iter()
        .copied()
        .map(validation_code)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        validation, expected_validation,
        "validation inventory coverage"
    );
}

fn source_occurrence(input: &[u8], token: &std::ops::Range<usize>, source: &str) -> usize {
    let prefix = &input[..token.start];
    std::str::from_utf8(prefix)
        .expect("accepted W-016 input should be UTF-8")
        .match_indices(source)
        .count()
}
