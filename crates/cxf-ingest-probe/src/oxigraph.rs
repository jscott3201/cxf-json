use oxjsonld::JsonLdParser;
use oxrdf::{GraphName, NamedOrBlankNode, Quad, Term};

use crate::{
    DiagnosticStage, ProbeDiagnostic, ProbeFailure, ProbeReport, RdfNodeKind, RdfNodeSummary,
    RdfObjectSummary, RdfQuadSummary, SourceDocument, SourcePosition, SourceRange,
};

/// Parses JSON-LD into owned RDF summaries without exposing Oxigraph types.
pub fn parse_json_ld(input: &[u8]) -> Result<ProbeReport, ProbeFailure> {
    let mut quads = JsonLdParser::new()
        .for_slice(input)
        .map(|result| result.map(|quad| summarize_quad(&quad)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ProbeFailure {
            source: SourceDocument::new(input),
            diagnostic: ProbeDiagnostic {
                stage: DiagnosticStage::JsonLd,
                message: error.to_string(),
                range: error.location().map(|range| SourceRange {
                    start: SourcePosition {
                        offset: range.start.offset,
                        line: range.start.line,
                        column: range.start.column,
                    },
                    end: SourcePosition {
                        offset: range.end.offset,
                        line: range.end.line,
                        column: range.end.column,
                    },
                }),
            },
        })?;

    quads.sort();
    Ok(ProbeReport {
        source: SourceDocument::new(input),
        diagnostics: Vec::new(),
        quads,
    })
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
