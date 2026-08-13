//! Crate-private W-014-C1 validator over the typed projection.
//!
//! Rules are spec-decided only: connection endpoints are connectors (spec
//! Table 8.2), connected connectors carrying statically known datatypes must
//! agree, `isOfDataType` is authored only on connectors, parameters, and
//! constants, and grouping predicates are authored only on nodes whose
//! registered class can be a block. The C-008/C-009 register posture holds:
//! absence is surfaced at informational severity and is never a rejection.
//! Findings are ordered by authored evidence position (source token start);
//! the validated document is never discarded. All behavior remains
//! crate-private; profile 0.1.6 public exports are unchanged.

use std::ops::Range;

use crate::projection::{DataTypeKind, EdgeKind, NodeClass, Projection};

/// Severity of one validation finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationSeverity {
    /// The document violates a spec-decided rule.
    Error,
    /// Presence information the profile distinguishes but never rejects
    /// (C-009 posture).
    Informational,
}

/// Stable private code allocation for validator rules.
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
}

impl ValidationCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionEndpointNotConnector => "CXF-V-001",
            Self::ConnectionDataTypeMismatch => "CXF-V-002",
            Self::DataTypeOutsideDomain => "CXF-V-003",
            Self::GroupingOutsideBlock => "CXF-V-004",
            Self::ParameterValueAbsent => "CXF-V-005",
        }
    }
}

/// One validator finding with source-token evidence and authored ordering.
#[derive(Debug)]
pub(crate) struct ValidationFinding {
    code: ValidationCode,
    severity: ValidationSeverity,
    node: usize,
    token: Range<usize>,
}

impl ValidationFinding {
    pub(crate) const fn code(&self) -> ValidationCode {
        self.code
    }

    pub(crate) const fn severity(&self) -> ValidationSeverity {
        self.severity
    }

    pub(crate) const fn node(&self) -> usize {
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

/// Validates a projection with the C1 rule set, returning findings ordered
/// by authored evidence position. The projection is borrowed, not consumed.
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
                                severity: ValidationSeverity::Error,
                                node: endpoint,
                                token: edge.token().clone(),
                            });
                        }
                    }
                    let subject_type = effective_data_type(projection, edge.subject());
                    let target_type = effective_data_type(projection, target);
                    if let (Some(subject_type), Some(target_type)) = (subject_type, target_type)
                        && subject_type != target_type
                    {
                        findings.push(ValidationFinding {
                            code: ValidationCode::ConnectionDataTypeMismatch,
                            severity: ValidationSeverity::Error,
                            node: edge.subject(),
                            token: edge.token().clone(),
                        });
                    }
                }
            }
            EdgeKind::DataType => {
                let subject = edge.subject();
                let in_domain = matches!(
                    nodes[subject].class(),
                    NodeClass::Connector(_) | NodeClass::Parameter | NodeClass::Constant
                );
                if !in_domain {
                    findings.push(ValidationFinding {
                        code: ValidationCode::DataTypeOutsideDomain,
                        severity: ValidationSeverity::Error,
                        node: subject,
                        token: edge.token().clone(),
                    });
                }
            }
            EdgeKind::Input | EdgeKind::Output | EdgeKind::Parameter | EdgeKind::Constant => {
                let subject = edge.subject();
                if class_cannot_be_block(nodes[subject].class()) {
                    findings.push(ValidationFinding {
                        code: ValidationCode::GroupingOutsideBlock,
                        severity: ValidationSeverity::Error,
                        node: subject,
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
            findings.push(ValidationFinding {
                code: ValidationCode::ParameterValueAbsent,
                severity: ValidationSeverity::Informational,
                node: index,
                token: node.token().clone(),
            });
        }
    }
    // Deterministic output: authored evidence position, then node index.
    findings.sort_by(|left, right| {
        left.token
            .start
            .cmp(&right.token.start)
            .then(left.node.cmp(&right.node))
    });
    findings
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
                .all(|finding| finding.severity() == ValidationSeverity::Error)
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
    fn v005_absent_values_surface_informationally() {
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
        assert_eq!(findings[0].severity(), ValidationSeverity::Informational);
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
            // Informational presence findings (V-005) are legal on real
            // emitter output (C-009); error findings are the control.
            let errors: Vec<&ValidationFinding> = findings
                .iter()
                .filter(|finding| finding.severity() == ValidationSeverity::Error)
                .collect();
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
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
