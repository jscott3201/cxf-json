use std::fmt;

use oxjsonld::JsonLdParser;
use oxrdf::{GraphName, NamedOrBlankNode, Quad, Term};

use crate::{
    ParseOptions, SourceDocument, json,
    ordered::OrderedDocument,
    projection::{self, Projection},
    validation::{self, ValidationFinding},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SemanticMetrics {
    pub(crate) json: json::JsonStructureMetrics,
    pub(crate) emitted_rdf_quads: u64,
    pub(crate) retained_rdf_term_bytes: u64,
}

pub(crate) struct SemanticDocument {
    ordered: OrderedDocument,
    quads: Vec<Quad>,
    metrics: SemanticMetrics,
}

impl fmt::Debug for SemanticDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticDocument")
            .field("source", self.source_document())
            .field("quad_count", &self.quads.len())
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

impl SemanticDocument {
    pub(crate) fn source_document(&self) -> &SourceDocument {
        self.ordered.source_document()
    }

    pub(crate) fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub(crate) const fn metrics(&self) -> SemanticMetrics {
        self.metrics
    }

    fn into_parts(self) -> (OrderedDocument, Vec<Quad>, SemanticMetrics) {
        (self.ordered, self.quads, self.metrics)
    }

    #[cfg(test)]
    pub(crate) const fn ordered_root(&self) -> &crate::ordered::OrderedValue {
        self.ordered.root()
    }
}

/// Private composition of independently retained semantic and source-derived
/// evidence. No field claims correspondence between an RDF quad and a
/// projection node (D-030).
pub(crate) struct ComposedDocument {
    projection: Projection,
    quads: Vec<Quad>,
    semantic_metrics: SemanticMetrics,
    validation_findings: Vec<ValidationFinding>,
}

impl fmt::Debug for ComposedDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComposedDocument")
            .field("source", self.source_document())
            .field("quad_count", &self.quads.len())
            .field("semantic_metrics", &self.semantic_metrics)
            .field("projection_metrics", &self.projection.metrics())
            .field("validation_finding_count", &self.validation_findings.len())
            .finish_non_exhaustive()
    }
}

impl ComposedDocument {
    pub(crate) fn source_document(&self) -> &SourceDocument {
        self.projection.source_document()
    }

    pub(crate) fn projection(&self) -> &Projection {
        &self.projection
    }

    pub(crate) fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub(crate) const fn semantic_metrics(&self) -> SemanticMetrics {
        self.semantic_metrics
    }

    pub(crate) fn validation_findings(&self) -> &[ValidationFinding] {
        &self.validation_findings
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SemanticFailureKind {
    MissingDocumentIri,
    JsonLd,
    RdfQuadLimit,
    RetainedRdfTermBytesLimit,
}

impl SemanticFailureKind {
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::MissingDocumentIri => "private semantic ingestion requires a document IRI",
            Self::JsonLd => "JSON-LD processing failed",
            Self::RdfQuadLimit => "emitted RDF quads exceed the configured limit",
            Self::RetainedRdfTermBytesLimit => {
                "retained RDF term bytes exceed the configured limit"
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct SemanticError {
    source: SourceDocument,
    kind: SemanticFailureKind,
    metrics: SemanticMetrics,
}

impl SemanticError {
    pub(crate) fn source_document(&self) -> &SourceDocument {
        &self.source
    }

    pub(crate) const fn kind(&self) -> SemanticFailureKind {
        self.kind
    }

    pub(crate) const fn message(&self) -> &'static str {
        self.kind.message()
    }

    pub(crate) const fn metrics(&self) -> SemanticMetrics {
        self.metrics
    }
}

#[derive(Debug)]
pub(crate) enum SemanticFailure {
    Preflight(json::PreflightFailure),
    Semantic(SemanticError),
}

pub(crate) fn ingest(
    input: &[u8],
    options: &ParseOptions,
) -> Result<SemanticDocument, SemanticFailure> {
    let preflight =
        json::admit_and_preflight(input, options).map_err(SemanticFailure::Preflight)?;
    ingest_preflighted(preflight, options).map_err(SemanticFailure::Semantic)
}

/// Runs the existing private ingestion, source projection, and validation
/// stages without adding source-to-RDF correspondence or a public parse path.
pub(crate) fn ingest_project_validate(
    input: &[u8],
    options: &ParseOptions,
) -> Result<ComposedDocument, SemanticFailure> {
    let semantic = ingest(input, options)?;
    let (ordered, quads, semantic_metrics) = semantic.into_parts();
    let projection = projection::project(ordered);
    let validation_findings = validation::validate(&projection);
    Ok(ComposedDocument {
        projection,
        quads,
        semantic_metrics,
        validation_findings,
    })
}

pub(crate) fn ingest_preflighted(
    preflight: json::PreflightedJson,
    options: &ParseOptions,
) -> Result<SemanticDocument, SemanticError> {
    let json_metrics = preflight.metrics();
    let (ordered, _) = preflight.into_ordered_document();
    let Some(document_iri) = options.document_iri() else {
        return Err(semantic_error(
            ordered.into_source_document(),
            SemanticFailureKind::MissingDocumentIri,
            json_metrics,
            SemanticProgress::default(),
        ));
    };

    match parse_quads(
        ordered.source_document().as_bytes(),
        document_iri.as_str(),
        options,
    ) {
        Ok((quads, progress)) => Ok(SemanticDocument {
            ordered,
            quads,
            metrics: semantic_metrics(json_metrics, progress),
        }),
        Err((kind, progress)) => Err(semantic_error(
            ordered.into_source_document(),
            kind,
            json_metrics,
            progress,
        )),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SemanticProgress {
    emitted_rdf_quads: u64,
    retained_rdf_term_bytes: u64,
}

fn parse_quads(
    input: &[u8],
    document_iri: &str,
    options: &ParseOptions,
) -> Result<(Vec<Quad>, SemanticProgress), (SemanticFailureKind, SemanticProgress)> {
    let parser = JsonLdParser::new()
        .with_base_iri(document_iri)
        .map_err(|_| (SemanticFailureKind::JsonLd, SemanticProgress::default()))?;
    let mut progress = SemanticProgress::default();
    let mut quads = Vec::new();

    for result in parser.for_slice(input) {
        let quad = result.map_err(|_| (SemanticFailureKind::JsonLd, progress))?;
        let emitted_rdf_quads = progress
            .emitted_rdf_quads
            .checked_add(1)
            .ok_or((SemanticFailureKind::RdfQuadLimit, progress))?;
        progress.emitted_rdf_quads = emitted_rdf_quads;
        if emitted_rdf_quads > options.max_rdf_quads() {
            return Err((SemanticFailureKind::RdfQuadLimit, progress));
        }
        let quad_bytes = retained_quad_bytes(&quad)
            .ok_or((SemanticFailureKind::RetainedRdfTermBytesLimit, progress))?;
        let retained_rdf_term_bytes = checked_budget_add(
            progress.retained_rdf_term_bytes,
            quad_bytes,
            options.max_retained_rdf_term_bytes(),
            SemanticFailureKind::RetainedRdfTermBytesLimit,
        )
        .map_err(|kind| (kind, progress))?;

        progress = SemanticProgress {
            emitted_rdf_quads,
            retained_rdf_term_bytes,
        };
        quads.push(quad);
    }

    Ok((quads, progress))
}

fn semantic_error(
    source: SourceDocument,
    kind: SemanticFailureKind,
    json: json::JsonStructureMetrics,
    progress: SemanticProgress,
) -> SemanticError {
    SemanticError {
        source,
        kind,
        metrics: semantic_metrics(json, progress),
    }
}

const fn semantic_metrics(
    json: json::JsonStructureMetrics,
    progress: SemanticProgress,
) -> SemanticMetrics {
    SemanticMetrics {
        json,
        emitted_rdf_quads: progress.emitted_rdf_quads,
        retained_rdf_term_bytes: progress.retained_rdf_term_bytes,
    }
}

fn checked_budget_add(
    current: u64,
    added: u64,
    limit: u64,
    kind: SemanticFailureKind,
) -> Result<u64, SemanticFailureKind> {
    let next = current.checked_add(added).ok_or(kind)?;
    if next > limit { Err(kind) } else { Ok(next) }
}

fn retained_quad_bytes(quad: &Quad) -> Option<u64> {
    let mut total = retained_node_bytes(&quad.subject)?;
    total = total.checked_add(string_bytes(quad.predicate.as_str())?)?;
    total = total.checked_add(retained_term_bytes(&quad.object)?)?;
    total.checked_add(retained_graph_name_bytes(&quad.graph_name)?)
}

fn retained_node_bytes(node: &NamedOrBlankNode) -> Option<u64> {
    match node {
        NamedOrBlankNode::NamedNode(node) => string_bytes(node.as_str()),
        NamedOrBlankNode::BlankNode(node) => string_bytes(node.as_str()),
    }
}

fn retained_term_bytes(term: &Term) -> Option<u64> {
    match term {
        Term::NamedNode(node) => string_bytes(node.as_str()),
        Term::BlankNode(node) => string_bytes(node.as_str()),
        Term::Literal(literal) => {
            let mut total = string_bytes(literal.value())?;
            total = total.checked_add(string_bytes(literal.datatype().as_str())?)?;
            if let Some(language) = literal.language() {
                total = total.checked_add(string_bytes(language)?)?;
            }
            Some(total)
        }
    }
}

fn retained_graph_name_bytes(graph_name: &GraphName) -> Option<u64> {
    match graph_name {
        GraphName::NamedNode(node) => string_bytes(node.as_str()),
        GraphName::BlankNode(node) => string_bytes(node.as_str()),
        GraphName::DefaultGraph => Some(0),
    }
}

fn string_bytes(value: &str) -> Option<u64> {
    u64::try_from(value.len()).ok()
}

#[cfg(test)]
mod w016;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use oxrdf::{BlankNode, GraphName, Literal, NamedNode, Quad, Term};

    use super::*;
    use crate::{DocumentIri, json::PreflightFailure, ordered::OrderedValue};

    const COMPACT: &[u8] = include_bytes!("../tests/fixtures/cxf-compact.jsonld");
    const FULL_IRI: &[u8] = include_bytes!("../tests/fixtures/cxf-full-iri.jsonld");
    const ORDER_A: &[u8] = include_bytes!("../tests/fixtures/cxf-order-a.jsonld");
    const ORDER_B: &[u8] = include_bytes!("../tests/fixtures/cxf-order-b.jsonld");
    const CONTEXT_LIST: &[u8] = include_bytes!("../tests/fixtures/cxf-context-list.jsonld");
    const EMBEDDED_CONTEXT: &[u8] = include_bytes!("../tests/fixtures/embedded-context.jsonld");
    const NAMED_GRAPH: &[u8] = include_bytes!("../tests/fixtures/named-graph.jsonld");
    const REMOTE_CONTEXT: &[u8] = include_bytes!("../tests/fixtures/remote-context.jsonld");
    const COMPOSITION_BOUNDARY: &[u8] =
        include_bytes!("../tests/projection/cxf-proj-composition.jsonld");
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const S231_BLOCK: &str = "http://data.ashrae.org/S231P#Block";

    fn options() -> ParseOptions {
        ParseOptions::new().with_document_iri(
            DocumentIri::parse("https://example.test/input").expect("test IRI should be valid"),
        )
    }

    fn semantic_error(input: &[u8], options: &ParseOptions) -> SemanticError {
        match ingest(input, options).expect_err("semantic ingestion should fail") {
            SemanticFailure::Semantic(error) => error,
            SemanticFailure::Preflight(_) => panic!("expected semantic failure"),
        }
    }

    fn composition_error(input: &[u8], options: &ParseOptions) -> SemanticFailure {
        ingest_project_validate(input, options).expect_err("private composition should fail")
    }

    #[test]
    fn composition_retains_independent_rdf_projection_and_validation_evidence() {
        let document = ingest_project_validate(COMPOSITION_BOUNDARY, &options())
            .expect("owned composition fixture should construct");

        assert_eq!(document.source_document().as_bytes(), COMPOSITION_BOUNDARY);
        assert_eq!(document.quads().len(), 2);
        assert_eq!(document.semantic_metrics().emitted_rdf_quads, 2);
        assert!(document.quads().iter().all(|quad| {
            matches!(
                &quad.subject,
                NamedOrBlankNode::NamedNode(node)
                    if node.as_str() == "https://example.test/relative-parameter"
            )
        }));

        let projection = document.projection();
        assert!(projection.diagnostics().is_empty());
        assert_eq!(projection.nodes().len(), 1);
        assert_eq!(
            projection.nodes()[0].id_spelling(),
            Some("relative-parameter")
        );
        assert_eq!(projection.nodes()[0].extensions().len(), 1);
        assert_eq!(projection.nodes()[0].extensions()[0].kind(), "string");
        assert_eq!(projection.metrics().extension_members, 1);

        assert_eq!(document.validation_findings().len(), 1);
        assert_eq!(
            document.validation_findings()[0].code().as_str(),
            "CXF-V-005"
        );
    }

    #[test]
    fn projection_and_validation_findings_do_not_fail_composition() {
        let input = br#"{
            "@context": {"S231": "http://data.ashrae.org/S231#"},
            "@graph": [
                {"@id": "weak"},
                {"@id": "parameter", "@type": "S231:Parameter"}
            ]
        }"#;
        let document = ingest_project_validate(input, &options())
            .expect("projection and validation findings should be non-fatal");

        assert_eq!(document.projection().diagnostics().len(), 1);
        assert_eq!(document.validation_findings().len(), 1);
        assert_eq!(
            document.validation_findings()[0].code().as_str(),
            "CXF-V-005"
        );
    }

    #[test]
    fn composition_preserves_construction_failure_precedence() {
        let baseline = ingest_project_validate(COMPOSITION_BOUNDARY, &options())
            .expect("owned composition fixture should establish its RDF byte budget");
        let retained_rdf_term_bytes = baseline.semantic_metrics().retained_rdf_term_bytes;

        assert!(matches!(
            composition_error(b"{}", &ParseOptions::new().with_max_input_bytes(1)),
            SemanticFailure::Preflight(PreflightFailure::Admission(_))
        ));
        assert!(matches!(
            composition_error(b"{", &options()),
            SemanticFailure::Preflight(PreflightFailure::Json(error))
                if error.kind() == crate::json::JsonFailureKind::Syntax
        ));
        assert!(matches!(
            composition_error(b"{", &ParseOptions::new()),
            SemanticFailure::Preflight(PreflightFailure::Json(error))
                if error.kind() == crate::json::JsonFailureKind::Syntax
        ));
        assert!(matches!(
            composition_error(b"{}", &ParseOptions::new()),
            SemanticFailure::Semantic(error)
                if error.kind() == SemanticFailureKind::MissingDocumentIri
        ));
        assert!(matches!(
            composition_error(REMOTE_CONTEXT, &options()),
            SemanticFailure::Semantic(error) if error.kind() == SemanticFailureKind::JsonLd
        ));
        assert!(matches!(
            composition_error(COMPOSITION_BOUNDARY, &options().with_max_rdf_quads(0)),
            SemanticFailure::Semantic(error)
                if error.kind() == SemanticFailureKind::RdfQuadLimit
        ));
        assert!(matches!(
            composition_error(
                COMPOSITION_BOUNDARY,
                &options().with_max_retained_rdf_term_bytes(retained_rdf_term_bytes - 1)
            ),
            SemanticFailure::Semantic(error)
                if error.kind() == SemanticFailureKind::RetainedRdfTermBytesLimit
        ));
    }

    #[test]
    fn admission_precedes_the_private_document_iri_requirement() {
        match ingest(b"{}", &ParseOptions::new().with_max_input_bytes(1))
            .expect_err("oversized input should fail before semantic processing")
        {
            SemanticFailure::Preflight(PreflightFailure::Admission(error)) => {
                assert_eq!(error.actual_bytes(), 2);
            }
            _ => panic!("expected source-free admission failure"),
        }
    }

    #[test]
    fn missing_document_iri_retains_the_admitted_source() {
        let error = semantic_error(b"{}", &ParseOptions::new());

        assert_eq!(error.kind(), SemanticFailureKind::MissingDocumentIri);
        assert_eq!(error.source_document().as_bytes(), b"{}");
        assert_eq!(
            error.message(),
            "private semantic ingestion requires a document IRI"
        );
    }

    #[test]
    fn compact_and_full_iri_forms_produce_equal_rdf_sets() {
        let compact = ingest(COMPACT, &options()).expect("compact CXF should parse");
        let full = ingest(FULL_IRI, &options()).expect("full-IRI CXF should parse");
        let compact = compact.quads().iter().collect::<HashSet<_>>();
        let full = full.quads().iter().collect::<HashSet<_>>();

        assert_eq!(compact, full);
    }

    #[test]
    fn reordered_cxf_arrays_do_not_create_graph_order() {
        let first = ingest(ORDER_A, &options()).expect("first order should parse");
        let second = ingest(ORDER_B, &options()).expect("second order should parse");

        assert_eq!(
            first.quads().iter().collect::<HashSet<_>>(),
            second.quads().iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            contains_block_ids(&first),
            ["ex:Root.first", "ex:Root.second"]
        );
        assert_eq!(
            contains_block_ids(&second),
            ["ex:Root.second", "ex:Root.first"]
        );
    }

    #[test]
    fn qualified_context_graph_and_anonymous_forms_replay_on_the_production_path() {
        let context = ingest(CONTEXT_LIST, &options()).expect("context list should parse");
        assert!(context.quads().iter().any(|quad| {
            quad.predicate.as_str() == RDF_TYPE
                && matches!(
                    &quad.object,
                    Term::NamedNode(node) if node.as_str() == S231_BLOCK
                )
        }));
        assert!(context.quads().iter().all(|quad| {
            !quad.predicate.as_str().contains("obsolete.example")
                && !matches!(
                    &quad.object,
                    Term::NamedNode(node) if node.as_str().contains("obsolete.example")
                )
        }));

        let embedded = ingest(EMBEDDED_CONTEXT, &options()).expect("context should parse");
        assert_eq!(embedded.quads().len(), 5);
        assert!(embedded.quads().iter().any(|quad| {
            matches!(
                &quad.object,
                Term::Literal(literal)
                    if literal.value() == "example" && literal.language() == Some("en")
            )
        }));

        let named_graph = ingest(NAMED_GRAPH, &options()).expect("named graph should parse");
        assert!(matches!(
            &named_graph.quads()[0].graph_name,
            GraphName::NamedNode(node) if node.as_str() == "https://example.test/graph"
        ));

        let anonymous = ingest(br#"{"https://example.test/label":"anonymous"}"#, &options())
            .expect("anonymous subject should parse");
        assert!(matches!(
            &anonymous.quads()[0].subject,
            NamedOrBlankNode::BlankNode(_)
        ));
    }

    #[test]
    fn repeated_identical_quads_count_each_retained_occurrence() {
        let single = ingest(
            br#"{"@id":"https://example.test/s","https://example.test/p":"v"}"#,
            &options(),
        )
        .expect("single value should parse");
        let repeated = ingest(
            br#"{"@id":"https://example.test/s","https://example.test/p":["v","v"]}"#,
            &options(),
        )
        .expect("repeated values should parse");

        assert_eq!(single.metrics().emitted_rdf_quads, 1);
        assert_eq!(repeated.metrics().emitted_rdf_quads, 2);
        assert_eq!(repeated.quads()[0], repeated.quads()[1]);
        assert_eq!(
            repeated.metrics().retained_rdf_term_bytes,
            single.metrics().retained_rdf_term_bytes * 2
        );
    }

    #[test]
    fn qualified_large_exponent_reaches_json_ld_processing() {
        let input = br#"{
            "@context": {"value": "https://example.test/value"},
            "@id": "https://example.test/subject",
            "value": [1, 1.0, 1e+02, -0, 1e400]
        }"#;
        let document = ingest(input, &options()).expect("qualified numeric form should parse");

        assert_eq!(document.source_document().as_bytes(), input);
        assert_eq!(document.quads().len(), 5);
        assert!(document.quads().iter().any(|quad| {
            matches!(&quad.object, Term::Literal(literal) if literal.value() == "1.0E400")
        }));
        let OrderedValue::Array { values, .. } = ordered_member(document.ordered_root(), "value")
        else {
            panic!("value should be an ordered array")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| &input[value.token().clone()])
                .collect::<Vec<_>>(),
            [b"1".as_slice(), b"1.0", b"1e+02", b"-0", b"1e400"]
        );
    }

    #[test]
    fn document_iri_resolves_relative_identity() {
        let input = br#"{"@id":"relative","https://example.test/p":"value"}"#;
        let document = ingest(input, &options()).expect("relative ID should use the document IRI");

        assert!(document.quads().iter().all(|quad| {
            matches!(
                &quad.subject,
                NamedOrBlankNode::NamedNode(node)
                    if node.as_str() == "https://example.test/relative"
            )
        }));
    }

    #[test]
    fn rdf_budget_boundaries_are_inclusive_and_quad_limit_wins() {
        let input = br#"{"@id":"https://example.test/s","https://example.test/p":"v"}"#;
        let baseline = ingest(input, &options()).expect("one quad should parse");
        let retained_bytes = baseline.metrics().retained_rdf_term_bytes;
        assert_eq!(baseline.metrics().emitted_rdf_quads, 1);

        let quad_error = semantic_error(input, &options().with_max_rdf_quads(0));
        assert_eq!(quad_error.kind(), SemanticFailureKind::RdfQuadLimit);
        assert_eq!(quad_error.metrics().emitted_rdf_quads, 1);

        let exact = options()
            .with_max_rdf_quads(1)
            .with_max_retained_rdf_term_bytes(retained_bytes);
        assert_eq!(
            ingest(input, &exact)
                .expect("exact RDF limits should succeed")
                .metrics()
                .retained_rdf_term_bytes,
            retained_bytes
        );

        let term_error = semantic_error(
            input,
            &options().with_max_retained_rdf_term_bytes(retained_bytes - 1),
        );
        assert_eq!(
            term_error.kind(),
            SemanticFailureKind::RetainedRdfTermBytesLimit
        );

        let co_trigger = semantic_error(
            input,
            &options()
                .with_max_rdf_quads(0)
                .with_max_retained_rdf_term_bytes(0),
        );
        assert_eq!(co_trigger.kind(), SemanticFailureKind::RdfQuadLimit);
        assert!(
            ingest(
                b"{}",
                &options()
                    .with_max_rdf_quads(0)
                    .with_max_retained_rdf_term_bytes(0)
            )
            .expect("an empty graph should meet zero RDF limits")
            .quads()
            .is_empty()
        );
    }

    #[test]
    fn retained_term_bytes_cover_all_owned_term_components() {
        let predicate = "https://example.test/p";
        let datatype = "https://example.test/type";
        let graph = "https://example.test/g";
        let quad = Quad::new(
            BlankNode::new("subject").expect("blank node should be valid"),
            NamedNode::new(predicate).expect("predicate should be valid"),
            Literal::new_typed_literal(
                "value",
                NamedNode::new(datatype).expect("datatype should be valid"),
            ),
            NamedNode::new(graph).expect("graph name should be valid"),
        );
        assert_eq!(
            retained_quad_bytes(&quad),
            Some(
                ("subject".len() + predicate.len() + "value".len() + datatype.len() + graph.len())
                    as u64
            )
        );

        let language = Literal::new_language_tagged_literal("name", "en")
            .expect("language tag should be valid");
        let language_quad = Quad::new(
            NamedNode::new("https://example.test/s").expect("subject should be valid"),
            NamedNode::new(predicate).expect("predicate should be valid"),
            language,
            GraphName::DefaultGraph,
        );
        assert_eq!(
            retained_quad_bytes(&language_quad),
            Some(
                ("https://example.test/s".len()
                    + predicate.len()
                    + "name".len()
                    + "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".len()
                    + "en".len()) as u64
            )
        );

        let blank_object = Quad::new(
            NamedNode::new("https://example.test/s").expect("subject should be valid"),
            NamedNode::new(predicate).expect("predicate should be valid"),
            BlankNode::new("object").expect("blank node should be valid"),
            GraphName::DefaultGraph,
        );
        assert!(matches!(blank_object.object, Term::BlankNode(_)));
        assert!(retained_quad_bytes(&blank_object).is_some());
    }

    #[test]
    fn budget_addition_rejects_overflow_without_wrapping() {
        assert_eq!(
            checked_budget_add(u64::MAX, 1, u64::MAX, SemanticFailureKind::RdfQuadLimit),
            Err(SemanticFailureKind::RdfQuadLimit)
        );
        assert_eq!(
            checked_budget_add(
                u64::MAX,
                1,
                u64::MAX,
                SemanticFailureKind::RetainedRdfTermBytesLimit
            ),
            Err(SemanticFailureKind::RetainedRdfTermBytesLimit)
        );
    }

    #[test]
    fn remote_context_failure_uses_fixed_project_text() {
        let error = semantic_error(REMOTE_CONTEXT, &options());

        assert_eq!(error.kind(), SemanticFailureKind::JsonLd);
        assert_eq!(error.message(), "JSON-LD processing failed");
        assert_eq!(error.source_document().as_bytes(), REMOTE_CONTEXT);
        assert!(!error.message().contains("remote"));
    }

    #[test]
    fn iterator_order_defines_backend_and_budget_precedence() {
        let output_then_error = br#"[
            {"@id":"https://example.test/s","https://example.test/p":"ok"},
            {"@id":1}
        ]"#;
        let error = semantic_error(output_then_error, &options().with_max_rdf_quads(0));
        assert_eq!(error.kind(), SemanticFailureKind::RdfQuadLimit);

        let error_then_output = br#"[
            {"@id":1},
            {"@id":"https://example.test/s","https://example.test/p":"ok"}
        ]"#;
        let error = semantic_error(error_then_output, &options());
        assert_eq!(error.kind(), SemanticFailureKind::JsonLd);
        assert_eq!(error.metrics().emitted_rdf_quads, 0);
    }

    fn ordered_member<'a>(value: &'a OrderedValue, name: &str) -> &'a OrderedValue {
        let OrderedValue::Object { members, .. } = value else {
            panic!("ordered value should be an object")
        };
        &members
            .iter()
            .find(|member| member.name.as_ref() == name)
            .unwrap_or_else(|| panic!("ordered object should contain {name}"))
            .value
    }

    fn contains_block_ids(document: &SemanticDocument) -> Vec<&str> {
        let OrderedValue::Array { values: graph, .. } =
            ordered_member(document.ordered_root(), "@graph")
        else {
            panic!("@graph should be an ordered array")
        };
        let OrderedValue::Array {
            values: contains, ..
        } = ordered_member(&graph[0], "S231:containsBlock")
        else {
            panic!("containsBlock should be an ordered array")
        };
        contains
            .iter()
            .map(|value| match ordered_member(value, "@id") {
                OrderedValue::String { value, .. } => value.as_ref(),
                _ => panic!("@id should be a string"),
            })
            .collect()
    }
}
