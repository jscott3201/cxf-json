use crate::{
    ParseOptions, json,
    semantic::{self, SemanticFailure, SemanticFailureKind, SemanticMetrics},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeKind {
    Success,
    AdmissionLimit,
    InvalidUtf8,
    JsonSyntax,
    DuplicateMember,
    JsonNestingLimit,
    JsonObjectMemberLimit,
    JsonValueLimit,
    DecodedMemberNameBytesLimit,
    MissingDocumentIri,
    JsonLd,
    RdfQuadLimit,
    RetainedRdfTermBytesLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metrics {
    pub max_nesting_depth: u64,
    pub max_object_members: u64,
    pub total_values: u64,
    pub decoded_member_name_bytes: u64,
    pub emitted_rdf_quads: u64,
    pub retained_rdf_term_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    pub outcome: OutcomeKind,
    pub source_matches_input: Option<bool>,
    pub returned_rdf_quads: u64,
    pub metrics: Option<Metrics>,
}

pub fn observe(input: &[u8], options: &ParseOptions) -> Observation {
    match semantic::ingest(input, options) {
        Ok(document) => Observation {
            outcome: OutcomeKind::Success,
            source_matches_input: Some(document.source_document().as_bytes() == input),
            returned_rdf_quads: u64::try_from(document.quads().len())
                .expect("a retained vector length must fit in u64"),
            metrics: Some(metrics(document.metrics())),
        },
        Err(SemanticFailure::Preflight(json::PreflightFailure::Admission(_))) => Observation {
            outcome: OutcomeKind::AdmissionLimit,
            source_matches_input: None,
            returned_rdf_quads: 0,
            metrics: None,
        },
        Err(SemanticFailure::Preflight(json::PreflightFailure::Json(error))) => Observation {
            outcome: json_outcome(error.kind()),
            source_matches_input: Some(error.source_document().as_bytes() == input),
            returned_rdf_quads: 0,
            metrics: None,
        },
        Err(SemanticFailure::Semantic(error)) => Observation {
            outcome: semantic_outcome(error.kind()),
            source_matches_input: Some(error.source_document().as_bytes() == input),
            returned_rdf_quads: 0,
            metrics: Some(metrics(error.metrics())),
        },
    }
}

const fn metrics(metrics: SemanticMetrics) -> Metrics {
    Metrics {
        max_nesting_depth: metrics.json.max_nesting_depth,
        max_object_members: metrics.json.max_object_members,
        total_values: metrics.json.total_values,
        decoded_member_name_bytes: metrics.json.decoded_member_name_bytes,
        emitted_rdf_quads: metrics.emitted_rdf_quads,
        retained_rdf_term_bytes: metrics.retained_rdf_term_bytes,
    }
}

const fn json_outcome(kind: json::JsonFailureKind) -> OutcomeKind {
    match kind {
        json::JsonFailureKind::InvalidUtf8 => OutcomeKind::InvalidUtf8,
        json::JsonFailureKind::Syntax => OutcomeKind::JsonSyntax,
        json::JsonFailureKind::DuplicateMember => OutcomeKind::DuplicateMember,
        json::JsonFailureKind::NestingLimit => OutcomeKind::JsonNestingLimit,
        json::JsonFailureKind::ObjectMemberLimit => OutcomeKind::JsonObjectMemberLimit,
        json::JsonFailureKind::ValueLimit => OutcomeKind::JsonValueLimit,
        json::JsonFailureKind::DecodedMemberNameBytesLimit => {
            OutcomeKind::DecodedMemberNameBytesLimit
        }
    }
}

const fn semantic_outcome(kind: SemanticFailureKind) -> OutcomeKind {
    match kind {
        SemanticFailureKind::MissingDocumentIri => OutcomeKind::MissingDocumentIri,
        SemanticFailureKind::JsonLd => OutcomeKind::JsonLd,
        SemanticFailureKind::RdfQuadLimit => OutcomeKind::RdfQuadLimit,
        SemanticFailureKind::RetainedRdfTermBytesLimit => OutcomeKind::RetainedRdfTermBytesLimit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentIri;

    fn options() -> ParseOptions {
        ParseOptions::new().with_document_iri(
            DocumentIri::parse("https://example.test/input").expect("test IRI should be valid"),
        )
    }

    #[test]
    fn observes_success_without_exposing_the_document() {
        let observation = observe(
            br#"{"@id":"https://example.test/s","https://example.test/p":"v"}"#,
            &options(),
        );

        assert_eq!(observation.outcome, OutcomeKind::Success);
        assert_eq!(observation.source_matches_input, Some(true));
        assert_eq!(observation.returned_rdf_quads, 1);
        assert_eq!(observation.metrics.unwrap().emitted_rdf_quads, 1);
    }

    #[test]
    fn observes_source_free_and_post_admission_failures() {
        let admission = observe(b"{}", &options().with_max_input_bytes(1));
        assert_eq!(admission.outcome, OutcomeKind::AdmissionLimit);
        assert_eq!(admission.source_matches_input, None);
        assert_eq!(admission.metrics, None);

        let duplicate = observe(br#"{"a":0,"a":1}"#, &options());
        assert_eq!(duplicate.outcome, OutcomeKind::DuplicateMember);
        assert_eq!(duplicate.source_matches_input, Some(true));
        assert_eq!(duplicate.returned_rdf_quads, 0);

        let quad_limit = observe(
            br#"{"@id":"https://example.test/s","https://example.test/p":"v"}"#,
            &options().with_max_rdf_quads(0),
        );
        assert_eq!(quad_limit.outcome, OutcomeKind::RdfQuadLimit);
        assert_eq!(quad_limit.source_matches_input, Some(true));
        assert_eq!(quad_limit.returned_rdf_quads, 0);
        assert_eq!(quad_limit.metrics.unwrap().emitted_rdf_quads, 1);
    }
}
