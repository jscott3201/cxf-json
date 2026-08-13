//! Crate-private W-014-C1 validator over the typed projection.
//!
//! Rules are spec-decided only: connection endpoints are connectors (spec
//! Table 8.2), connected connectors carrying statically known datatypes must
//! agree, `isOfDataType` is authored only on connectors, parameters, and
//! constants, and grouping predicates are authored only on nodes whose
//! registered class can be a block. The C-008/C-009 register posture holds:
//! absence surfaces at `DiagnosticSeverity::Warning` and is never a
//! rejection. Findings order totally by token, node index, then rule-code
//! ordinal; the validated document is never discarded. All behavior remains
//! crate-private; profile 0.1.7 public exports are unchanged.

use std::ops::Range;

use std::collections::BTreeMap;

use crate::contract::DiagnosticSeverity;
use crate::projection::NamespaceClass;
use crate::projection::{DataTypeKind, EdgeKind, NamespaceObservation, NodeClass, Projection};

/// Stable private code allocation for validator rules.
///
/// Findings reuse the public `DiagnosticSeverity` type directly (no
/// parallel private taxonomy): spec-rule violations are `Error`, and the
/// C-009 presence distinction is `Warning` — processing always continues.
///
/// Codes are crate-private until a later slice or profile version promotes
/// the validator surface; W-016 owns corpus coverage of each rule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ValidationCode {
    /// A connection endpoint resolved to a node that is not a connector.
    ConnectionEndpointNotConnector,
    /// Connected connectors carry statically known but disagreeing types.
    ConnectionDataTypeMismatch,
    /// `isOfDataType` authored outside the connector/parameter/constant
    /// domain.
    DataTypeOutsideDomain,
    /// A grouping predicate (`hasInput`/`hasOutput`/`hasParameter`/
    /// `hasConstant`) authored on a node whose class cannot be a block.
    GroupingOutsideBlock,
    /// A parameter or constant has no `value` property (C-009).
    ParameterValueAbsent,
    // W-015-C1 namespace acceptance policy (matrix rows, not structural
    // rules): the document declares and uses these regions without
    // rejection; the findings make classification visible.
    /// The document binds the legacy HTTPS S231P namespace (C-002).
    LegacyNamespaceGeneration,
    /// The document binds an unregistered namespace within a known
    /// vocabulary family (`data.ashrae.org`, `qudt.org` hosts): a
    /// possible new generation variant the profile does not register.
    UnregisteredFamilyNamespace,
    /// A registered prefix (`S231`/`S231P`/`qudt`/`unit`/`q`) is bound to
    /// a namespace outside its expected set: the binding cannot serve its
    /// obvious purpose and compacted spellings do not register. Staying a
    /// warning per the observational policy: processing continues, the
    /// document is retained, and affected terms fall back to extension
    /// evidence with distinct identity.
    ShadowedPrefix,
}

impl ValidationCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionEndpointNotConnector => "CXF-V-001",
            Self::ConnectionDataTypeMismatch => "CXF-V-002",
            Self::DataTypeOutsideDomain => "CXF-V-003",
            Self::GroupingOutsideBlock => "CXF-V-004",
            Self::ParameterValueAbsent => "CXF-V-005",
            Self::LegacyNamespaceGeneration => "CXF-C-001",
            Self::UnregisteredFamilyNamespace => "CXF-C-002",
            Self::ShadowedPrefix => "CXF-C-003",
        }
    }
}

/// One validator finding with source-token evidence and authored ordering.
///
/// `node` is `None` for root-level policy findings (namespace observations
/// live in `@context`, which no graph node owns).
#[derive(Debug)]
pub(crate) struct ValidationFinding {
    code: ValidationCode,
    severity: DiagnosticSeverity,
    node: Option<usize>,
    token: Range<usize>,
}

impl ValidationFinding {
    pub(crate) const fn code(&self) -> ValidationCode {
        self.code
    }

    pub(crate) const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub(crate) const fn node(&self) -> Option<usize> {
        self.node
    }

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
    }
}

/// The effective statically known datatype of a node: connector class data
/// wins, then the first resolved `isOfDataType` reference. Absent means
/// unknown, and unknown never disagrees (C-008 posture).
fn effective_data_type(projection: &Projection, node: usize) -> Option<DataTypeKind> {
    let class = projection.nodes()[node].class();
    if let NodeClass::Connector(connector) = class
        && let Some(data_type) = connector.data_type
    {
        return Some(data_type);
    }
    projection.nodes()[node].data_type()
}

/// V-001's endpoint check fires only when the endpoint's class is KNOWN to
/// be non-connector. Weakly typed connection endpoints are the register's
/// C-008 norm (nested instances carry no type triples), and library-typed
/// endpoints may be connector classes outside the core vocabulary (C-015):
/// neither may be treated as disproven.
fn class_cannot_be_connector(class: NodeClass) -> bool {
    matches!(
        class,
        NodeClass::Package
            | NodeClass::Block(_)
            | NodeClass::Parameter
            | NodeClass::Constant
            | NodeClass::EnumerationType
            | NodeClass::DataType
            | NodeClass::Text
    )
}

/// V-003's domain check fires only when the class is PROVABLY outside the
/// connector/parameter/constant domain. Weakly typed and library-typed
/// subjects are unknown, not wrong (C-008/C-015 posture, C-320 evidence).
fn class_outside_data_type_domain(class: NodeClass) -> bool {
    matches!(
        class,
        NodeClass::Package
            | NodeClass::Block(_)
            | NodeClass::EnumerationType
            | NodeClass::DataType
            | NodeClass::Text
    )
}

/// Rules that never fire on nodes whose block role is unknown rather than
/// disproven: weakly typed and library-typed nodes get the benefit of the
/// doubt (C-008, C-015).
fn class_cannot_be_block(class: NodeClass) -> bool {
    matches!(
        class,
        NodeClass::Package
            | NodeClass::Connector(_)
            | NodeClass::Parameter
            | NodeClass::Constant
            | NodeClass::EnumerationType
            | NodeClass::DataType
            | NodeClass::Text
    )
}

/// Sort ordinal for defensive total ordering of findings: distinct
/// members carry distinct tokens in practice, but if two findings ever
/// share a token (for example both endpoints of one block-block
/// connection member), the ordinal keeps the sequence deterministic.
const fn code_order(code: ValidationCode) -> u8 {
    match code {
        ValidationCode::ConnectionEndpointNotConnector => 0,
        ValidationCode::ConnectionDataTypeMismatch => 1,
        ValidationCode::DataTypeOutsideDomain => 2,
        ValidationCode::GroupingOutsideBlock => 3,
        ValidationCode::ParameterValueAbsent => 4,
        ValidationCode::LegacyNamespaceGeneration => 5,
        ValidationCode::UnregisteredFamilyNamespace => 6,
        ValidationCode::ShadowedPrefix => 7,
    }
}

/// Validates a projection with the C1 rule set, returning findings ordered
/// by authored evidence position (token start), then node index, then rule
/// code. The projection is borrowed, not consumed.
pub(crate) fn validate(projection: &Projection) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();
    let nodes = projection.nodes();
    for edge in projection.edges() {
        match edge.kind() {
            EdgeKind::Connection => {
                // Only runs when both endpoints resolve; unresolved
                // references already carry CXF-P-005.
                if let Some(target) = edge.target() {
                    for endpoint in [edge.subject(), target] {
                        if class_cannot_be_connector(nodes[endpoint].class()) {
                            findings.push(ValidationFinding {
                                code: ValidationCode::ConnectionEndpointNotConnector,
                                severity: DiagnosticSeverity::Error,
                                node: Some(endpoint),
                                token: edge.token().clone(),
                            });
                        }
                    }
                    // Datatype comparison is connector-scoped: a
                    // non-connector endpoint already carries V-001 and is
                    // never re-judged here.
                    if matches!(nodes[edge.subject()].class(), NodeClass::Connector(_))
                        && matches!(nodes[target].class(), NodeClass::Connector(_))
                    {
                        let subject_type = effective_data_type(projection, edge.subject());
                        let target_type = effective_data_type(projection, target);
                        if let (Some(subject_type), Some(target_type)) = (subject_type, target_type)
                            && subject_type != target_type
                        {
                            findings.push(ValidationFinding {
                                code: ValidationCode::ConnectionDataTypeMismatch,
                                severity: DiagnosticSeverity::Error,
                                node: Some(edge.subject()),
                                token: edge.token().clone(),
                            });
                        }
                    }
                }
            }
            EdgeKind::DataType => {
                let subject = edge.subject();
                if class_outside_data_type_domain(nodes[subject].class()) {
                    findings.push(ValidationFinding {
                        code: ValidationCode::DataTypeOutsideDomain,
                        severity: DiagnosticSeverity::Error,
                        node: Some(subject),
                        token: edge.token().clone(),
                    });
                }
            }
            EdgeKind::Input | EdgeKind::Output | EdgeKind::Parameter | EdgeKind::Constant => {
                let subject = edge.subject();
                if class_cannot_be_block(nodes[subject].class()) {
                    findings.push(ValidationFinding {
                        code: ValidationCode::GroupingOutsideBlock,
                        severity: DiagnosticSeverity::Error,
                        node: Some(subject),
                        token: edge.token().clone(),
                    });
                }
            }
            EdgeKind::Containment => {}
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        if matches!(node.class(), NodeClass::Parameter | NodeClass::Constant)
            && node.value().is_none()
        {
            // Absence is surfaced as a warning; processing continues and
            // nothing is rejected (C-009).
            findings.push(ValidationFinding {
                code: ValidationCode::ParameterValueAbsent,
                severity: DiagnosticSeverity::Warning,
                node: Some(index),
                token: node.token().clone(),
            });
        }
    }
    // W-015 acceptance-policy findings over retained root context
    // mappings: one finding per RETAINED binding (last-write-wins, the
    // activation semantics); nothing is rejected.
    let mut retained: BTreeMap<&str, &NamespaceObservation> = BTreeMap::new();
    for observation in projection.namespace_observations() {
        retained.insert(observation.prefix(), observation);
    }
    for observation in retained.values() {
        if let Some(code) =
            policy_finding(observation.prefix(), observation.iri(), observation.class())
        {
            // All acceptance-policy findings are warnings: processing
            // continues and the document is retained (ADR 0011).
            findings.push(ValidationFinding {
                code,
                severity: DiagnosticSeverity::Warning,
                node: None,
                token: observation.token().clone(),
            });
        }
    }
    // Deterministic total order: authored evidence position, node index,
    // then rule-code ordinal.
    findings.sort_by(|left, right| {
        left.token
            .start
            .cmp(&right.token.start)
            .then(left.node.cmp(&right.node))
            .then(code_order(left.code).cmp(&code_order(right.code)))
    });
    findings
}

/// Prefix→expected-identity-set mapping for shadow detection. Prefixes are
/// document-local; these five are registered conventions the emitter and
/// ecosystem rely on, so binding them to a foreign namespace is the one
/// shape the policy diagnoses as Error.
fn prefix_shadowed(prefix: &str, class: NamespaceClass) -> bool {
    let expected = match prefix {
        "S231" => [
            NamespaceClass::S231,
            NamespaceClass::S231P,
            NamespaceClass::S231PLegacyHttps,
        ]
        .as_slice(),
        "S231P" => [NamespaceClass::S231P, NamespaceClass::S231PLegacyHttps].as_slice(),
        "qudt" => [NamespaceClass::QudtSchema].as_slice(),
        "unit" => [NamespaceClass::QudtUnitVocab].as_slice(),
        "q" => [NamespaceClass::QudtQuantityKindVocab].as_slice(),
        _ => return false,
    };
    !expected.contains(&class)
}

/// Known vocabulary-family hosts; unregistered IRIs under these are the
/// consumer-relevant signal (a possible new generation variant).
const KNOWN_FAMILY_HOSTS: [&str; 4] = [
    "http://data.ashrae.org/",
    "https://data.ashrae.org/",
    "http://qudt.org/",
    "https://qudt.org/",
];

fn known_family(iri: &str) -> bool {
    KNOWN_FAMILY_HOSTS.iter().any(|host| iri.starts_with(host))
}

/// W-015 acceptance matrix, row by row. The binding is never rejected;
/// findings classify it.
fn policy_finding(prefix: &str, iri: &str, class: NamespaceClass) -> Option<ValidationCode> {
    if prefix_shadowed(prefix, class) {
        return Some(ValidationCode::ShadowedPrefix);
    }
    match class {
        NamespaceClass::S231PLegacyHttps => Some(ValidationCode::LegacyNamespaceGeneration),
        NamespaceClass::Unregistered if known_family(iri) => {
            Some(ValidationCode::UnregisteredFamilyNamespace)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParseOptions;
    use crate::projection::project;

    fn validate_str(input: &str) -> (Projection, Vec<ValidationFinding>) {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        let projection = project(document);
        let findings = validate(&projection);
        (projection, findings)
    }

    fn codes(findings: &[ValidationFinding]) -> Vec<ValidationCode> {
        findings.iter().map(|finding| finding.code()).collect()
    }

    #[test]
    fn v001_connection_endpoint_must_be_a_connector() {
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:a", "@type": "S231:RealOutput",
                  "S231:connectedTo": { "@id": "ex:blk" } },
                { "@id": "ex:blk", "@type": "S231:Block" },
                { "@id": "ex:src", "@type": "S231:Block",
                  "S231:connectedTo": { "@id": "ex:blk" } },
                { "@id": "ex:quiet", "@type": "S231:RealOutput",
                  "S231:connectedTo": { "@id": "ex:untyped" } },
                { "@id": "ex:untyped", "S231:value": "x" }
              ]
            }"#,
        );
        // Known non-connector endpoints fire once each; the weakly typed
        // endpoint (C-008's norm) stays quiet.
        assert_eq!(
            codes(&findings),
            &[
                ValidationCode::ConnectionEndpointNotConnector,
                ValidationCode::ConnectionEndpointNotConnector,
                ValidationCode::ConnectionEndpointNotConnector
            ]
        );
        assert!(
            findings
                .iter()
                .all(|finding| finding.severity() == DiagnosticSeverity::Error)
        );
    }

    #[test]
    fn v002_connected_types_must_agree_when_both_known() {
        let (_, mismatched) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:r", "@type": "S231:RealInput",
                  "S231:isConnectedTo": { "@id": "ex:i" } },
                { "@id": "ex:i", "@type": "S231:IntegerOutput" }
              ]
            }"#,
        );
        assert_eq!(
            codes(&mismatched),
            &[ValidationCode::ConnectionDataTypeMismatch]
        );
        // Unknown types never disagree (C-008 posture).
        let (_, unknown) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:r", "@type": "S231:RealInput",
                  "S231:isConnectedTo": { "@id": "ex:u" } },
                { "@id": "ex:u", "@type": "S231:OutputConnector" }
              ]
            }"#,
        );
        assert_eq!(codes(&unknown).len(), 0, "{unknown:?}");
    }

    #[test]
    fn v002_is_connector_scoped() {
        // A datatype-carrying PARAMETER connected to a typed output fires
        // V-001 for the non-connector endpoint and is never re-judged as a
        // datatype mismatch.
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:p", "@type": "S231:Parameter",
                  "S231:isOfDataType": { "@id": "S231:Integer" },
                  "S231:value": 1,
                  "S231:connectedTo": { "@id": "ex:r" } },
                { "@id": "ex:r", "@type": "S231:RealOutput" }
              ]
            }"#,
        );
        assert_eq!(
            codes(&findings),
            &[ValidationCode::ConnectionEndpointNotConnector],
            "{findings:?}"
        );
    }

    #[test]
    fn v003_unknown_node_classes_stay_quiet() {
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:weak", "S231:isOfDataType": { "@id": "S231:Real" } },
                { "@id": "ex:lib", "@type": "ex:Vendor.CustomType",
                  "S231:isOfDataType": { "@id": "S231:Real" } }
              ]
            }"#,
        );
        assert_eq!(codes(&findings).len(), 0, "{findings:?}");
    }

    #[test]
    fn v003_datatype_reference_outside_domain() {
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:blk", "@type": "S231:Block",
                  "S231:isOfDataType": { "@id": "S231:Real" } }
              ]
            }"#,
        );
        assert_eq!(codes(&findings), &[ValidationCode::DataTypeOutsideDomain]);
    }

    #[test]
    fn v004_grouping_predicates_require_a_block() {
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:p", "@type": "S231:RealInput",
                  "S231:hasParameter": { "@id": "ex:k" } },
                { "@id": "ex:k", "@type": "S231:Parameter",
                  "S231:value": 1 },
                { "@id": "ex:inst", "@type": "ex:Library.Block",
                  "S231:hasParameter": { "@id": "ex:k" } }
              ]
            }"#,
        );
        // Connector subject fires; library-typed (unknown block role) and
        // the parameter target stay quiet (C-008/C-015 benefit of doubt).
        assert_eq!(codes(&findings), &[ValidationCode::GroupingOutsideBlock]);
    }

    #[test]
    fn v005_absent_values_surface_as_warnings() {
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:empty", "@type": "S231:Parameter" },
                { "@id": "ex:full", "@type": "S231:Parameter", "S231:value": 1 }
              ]
            }"#,
        );
        assert_eq!(codes(&findings), &[ValidationCode::ParameterValueAbsent]);
        assert_eq!(findings[0].severity(), DiagnosticSeverity::Warning);
    }

    #[test]
    fn merged_projection_fixtures_yield_no_c1_findings() {
        macro_rules! fixture_bytes {
            ($name:literal) => {
                include_bytes!($name) as &[u8]
            };
        }
        for (name, bytes) in [
            (
                "specform",
                fixture_bytes!("../tests/projection/cxf-proj-specform.jsonld"),
            ),
            (
                "units",
                fixture_bytes!("../tests/projection/cxf-proj-units.jsonld"),
            ),
            (
                "annotation",
                fixture_bytes!("../tests/projection/cxf-proj-annotation.jsonld"),
            ),
            (
                "emitter",
                fixture_bytes!("../tests/projection/cxf-proj-emitter.jsonld"),
            ),
        ] {
            let preflight = crate::json::admit_and_preflight(bytes, &ParseOptions::new())
                .expect("fixture must pass preflight");
            let (document, _) = preflight.into_ordered_document();
            let projection = project(document);
            let findings = validate(&projection);
            // Warning presence findings (V-005) are legal on real
            // emitter output (C-009); error findings are the control.
            let errors: Vec<&ValidationFinding> = findings
                .iter()
                .filter(|finding| finding.severity() == DiagnosticSeverity::Error)
                .collect();
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
    }

    // ---- W-015-C1 acceptance-matrix rows ----

    #[test]
    fn accepted_generations_produce_no_policy_findings() {
        let (_, findings) = validate_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231P#",
                "S231P": "https://data.ashrae.org/S231P#",
                "qudt": "http://qudt.org/schema/qudt#",
                "unit": "http://qudt.org/vocab/unit#",
                "q": "http://qudt.org/vocab/quantitykind#",
                "ex": "http://example.org#"
              }
            }"#,
        );
        // C-016: `S231` legitimately maps to the post-v1.2 namespace. The
        // legacy HTTPS binding diagnoses as Observation; `ex` (a data
        // namespace outside the known families) produces nothing.
        assert_eq!(
            codes(&findings),
            &[ValidationCode::LegacyNamespaceGeneration]
        );
        assert_eq!(findings[0].severity(), DiagnosticSeverity::Warning);
        assert_eq!(findings[0].node(), None);
    }

    #[test]
    fn shadowed_prefix_is_a_warning() {
        let (projection, findings) = validate_str(
            r#"{
              "@context": {
                "S231": "http://qudt.org/schema/qudt#",
                "ex": "http://example.org#"
              },
              "@graph": [
                { "@id": "ex:a", "@type": "S231:Parameter", "S231:value": 1 }
              ]
            }"#,
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.code() == ValidationCode::ShadowedPrefix
                    && finding.severity() == DiagnosticSeverity::Warning),
            "{findings:?}"
        );
        // The shadowed binding never registers its compacted spellings:
        // `@type: S231:Parameter` keeps the node library-typed, and
        // `S231:value` remains extension evidence.
        let node = &projection.nodes()[0];
        assert_eq!(node.class(), crate::projection::NodeClass::LibraryInstance);
        assert_eq!(node.extensions().len(), 1, "{:?}", node.extensions());
    }

    #[test]
    fn rebinding_to_foreign_iri_deregisters_terms() {
        // A context ARRAY rebinding `S231` to a foreign IRI must make
        // activation and policy agree: the retained binding shadows, and
        // the earlier registered binding must NOT linger in activation.
        let (projection, findings) = validate_str(
            r#"{
              "@context": [
                { "S231": "http://data.ashrae.org/S231#" },
                { "S231": "https://vocab.example.org/other#" }
              ],
              "@graph": [
                { "@id": "ex:a", "@type": "S231:Parameter" }
              ]
            }"#,
        );
        assert_eq!(
            codes(&findings),
            &[ValidationCode::ShadowedPrefix],
            "{findings:?}"
        );
        assert_eq!(
            projection.nodes()[0].class(),
            crate::projection::NodeClass::LibraryInstance,
            "the retained foreign binding must deregister `S231:` spellings"
        );
    }

    #[test]
    fn json_ld_keyword_context_members_produce_no_observations() {
        // `@base`/`@vocab`/`@language` are not prefix bindings: they must
        // never produce observations, even under known-family hosts.
        let (_, findings) = validate_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "@base": "http://data.ashrae.org/base",
                "@vocab": "http://qudt.org/schema/other#",
                "@language": "en"
              }
            }"#,
        );
        assert_eq!(findings.len(), 0, "{findings:?}");
    }

    #[test]
    fn unregistered_family_variant_is_a_warning() {
        let (_, findings) = validate_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "v2": "http://data.ashrae.org/S231R#",
                "ex": "http://example.org#"
              }
            }"#,
        );
        assert_eq!(
            codes(&findings),
            &[ValidationCode::UnregisteredFamilyNamespace]
        );
        assert_eq!(findings[0].severity(), DiagnosticSeverity::Warning);
    }

    #[test]
    fn duplicate_prefix_bindings_fail_at_admission() {
        // Duplicate object members never reach the validator: W-011
        // preflight rejects them, which is exactly why the policy rules
        // only ever see a single binding per prefix.
        let result = crate::json::admit_and_preflight(
            br#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "S231": "http://qudt.org/schema/qudt#"
              }
            }"#,
            &ParseOptions::new(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn non_family_foreign_namespaces_stay_silent() {
        let (_, findings) = validate_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "vocab": "https://vocab.example.org/points#",
                "ex": "http://example.org#"
              }
            }"#,
        );
        assert_eq!(codes(&findings).len(), 0, "{findings:?}");
    }

    #[test]
    fn findings_sort_by_authored_position() {
        // The no-value parameter is authored BEFORE the out-of-domain
        // datatype edge, so its finding must lead despite the validator
        // collecting edge findings before node findings.
        let (_, findings) = validate_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:late", "@type": "S231:Parameter" },
                { "@id": "ex:early", "@type": "S231:Block",
                  "S231:isOfDataType": { "@id": "ex:NotAType" } }
              ]
            }"#,
        );
        assert_eq!(
            codes(&findings),
            &[
                ValidationCode::ParameterValueAbsent,
                ValidationCode::DataTypeOutsideDomain
            ]
        );
        let starts: Vec<usize> = findings
            .iter()
            .map(|finding| finding.token().start)
            .collect();
        let mut sorted = starts.clone();
        sorted.sort_unstable();
        assert_eq!(starts, sorted);
    }
}
