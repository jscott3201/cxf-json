#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use oxjsonld::JsonLdParser;
use oxrdf::{GraphName, NamedOrBlankNode, Quad, Term};

use crate::{
    DiagnosticStage, ProbeDiagnostic, ProbeFailure, ProbeMetrics, ProbeReport, RdfNodeKind,
    RdfNodeSummary, RdfObjectSummary, RdfQuadSummary, SourceDocument,
    json::{source_range, validate_unique_members},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::{MeasuredProbe, ProbeTiming};

/// Parses JSON-LD into owned RDF summaries without exposing Oxigraph types.
pub fn parse_json_ld(input: &[u8]) -> Result<ProbeReport, ProbeFailure> {
    let json = validate_unique_members(input)?;
    parse_preflighted_json_ld(input, json)
}

/// Parses JSON-LD and reports native stage timing outside the parse result.
#[cfg(not(target_arch = "wasm32"))]
pub fn measure_json_ld(input: &[u8]) -> MeasuredProbe {
    let preflight_started = Instant::now();
    let json = validate_unique_members(input);
    let preflight = preflight_started.elapsed();
    let json = match json {
        Ok(json) => json,
        Err(failure) => {
            return MeasuredProbe {
                result: Err(failure),
                timing: ProbeTiming {
                    preflight,
                    json_ld: None,
                },
            };
        }
    };
    let json_ld_started = Instant::now();
    let result = parse_preflighted_json_ld(input, json);
    let json_ld = Some(json_ld_started.elapsed());
    MeasuredProbe {
        result,
        timing: ProbeTiming { preflight, json_ld },
    }
}

fn parse_preflighted_json_ld(
    input: &[u8],
    json: crate::JsonStructureMetrics,
) -> Result<ProbeReport, ProbeFailure> {
    let quads = JsonLdParser::new()
        .for_slice(input)
        .map(|result| result.map(|quad| summarize_quad(&quad)))
        .collect::<Result<Vec<_>, _>>();
    let mut quads = match quads {
        Ok(quads) => quads,
        Err(error) => {
            return Err(ProbeFailure {
                source: SourceDocument::new(input),
                diagnostic: Box::new(ProbeDiagnostic {
                    stage: DiagnosticStage::JsonLd,
                    message: error.to_string(),
                    range: error.location().map(|range| {
                        source_range(
                            input,
                            usize::try_from(range.start.offset).unwrap_or(input.len()),
                            usize::try_from(range.end.offset).unwrap_or(input.len()),
                        )
                    }),
                    pointer: None,
                    rdf_term: None,
                }),
                metrics: Some(Box::new(ProbeMetrics {
                    json,
                    rdf_term_bytes: 0,
                })),
            });
        }
    };

    quads.sort();
    let rdf_term_bytes = quads.iter().map(retained_quad_bytes).sum();
    Ok(ProbeReport {
        source: SourceDocument::new(input),
        diagnostics: Vec::new(),
        quads,
        metrics: ProbeMetrics {
            json,
            rdf_term_bytes,
        },
    })
}

fn retained_quad_bytes(quad: &RdfQuadSummary) -> usize {
    retained_node_bytes(&quad.subject)
        + quad.predicate.len()
        + retained_object_bytes(&quad.object)
        + quad.graph_name.as_ref().map_or(0, retained_node_bytes)
}

fn retained_node_bytes(node: &RdfNodeSummary) -> usize {
    node.value.len()
}

fn retained_object_bytes(object: &RdfObjectSummary) -> usize {
    match object {
        RdfObjectSummary::Node(node) => retained_node_bytes(node),
        RdfObjectSummary::Literal {
            value,
            datatype,
            language,
        } => value.len() + datatype.len() + language.as_ref().map_or(0, String::len),
        RdfObjectSummary::Other(value) => value.len(),
    }
}

fn summarize_quad(quad: &Quad) -> RdfQuadSummary {
    RdfQuadSummary {
        subject: summarize_node(&quad.subject),
        predicate: quad.predicate.as_str().to_owned(),
        object: summarize_term(&quad.object),
        graph_name: summarize_graph_name(&quad.graph_name),
    }
}

fn summarize_node(node: &NamedOrBlankNode) -> RdfNodeSummary {
    match node {
        NamedOrBlankNode::NamedNode(node) => RdfNodeSummary {
            kind: RdfNodeKind::Named,
            value: node.as_str().to_owned(),
        },
        NamedOrBlankNode::BlankNode(node) => RdfNodeSummary {
            kind: RdfNodeKind::Blank,
            value: node.as_str().to_owned(),
        },
    }
}

fn summarize_term(term: &Term) -> RdfObjectSummary {
    match term {
        Term::NamedNode(node) => RdfObjectSummary::Node(RdfNodeSummary {
            kind: RdfNodeKind::Named,
            value: node.as_str().to_owned(),
        }),
        Term::BlankNode(node) => RdfObjectSummary::Node(RdfNodeSummary {
            kind: RdfNodeKind::Blank,
            value: node.as_str().to_owned(),
        }),
        Term::Literal(literal) => RdfObjectSummary::Literal {
            value: literal.value().to_owned(),
            datatype: literal.datatype().as_str().to_owned(),
            language: literal.language().map(str::to_owned),
        },
    }
}

fn summarize_graph_name(graph_name: &GraphName) -> Option<RdfNodeSummary> {
    match graph_name {
        GraphName::DefaultGraph => None,
        GraphName::NamedNode(node) => Some(RdfNodeSummary {
            kind: RdfNodeKind::Named,
            value: node.as_str().to_owned(),
        }),
        GraphName::BlankNode(node) => Some(RdfNodeSummary {
            kind: RdfNodeKind::Blank,
            value: node.as_str().to_owned(),
        }),
    }
}
