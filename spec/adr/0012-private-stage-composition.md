# ADR 0012: Private stage composition

Status: Accepted

Date: 2026-08-13 UTC

## Context

The crate has separate private stages for bounded semantic ingestion, typed CXF
projection, and validation. No operation exercises their ownership and failure
boundaries together. D-020 requires authored order to come from the ordered
source, while D-030 forbids joining a source object to RDF output without a
backend-provided, source-correlated expanded identifier.

## Decision

W-027 adds a crate-private composition operation under the existing
`semantic-ingestion` feature. Successful composition retains RDF output and
semantic metrics beside the source-derived projection and validation findings.
These are independent evidence: the result contains no node-to-quad map and
makes no source-to-RDF correspondence claim.

Admission, JSON, missing-document-IRI, JSON-LD, and RDF-budget failures keep the
existing semantic failure taxonomy and return no partial composed result.
Projection diagnostics and validation findings remain non-fatal evidence and do
not discard the projection.

## Consequences

- Normal builds gain no public parser, data type, diagnostic code, option, or
  host-runtime value. Profile 0.1.7 does not change.
- Existing semantic instrumentation continues to call the ingestion-only path,
  so W-022 benchmark stage definitions do not change.
- D-029 still blocks a supported hostile-input parser. Composition adds no
  cancellation, memory, deadline, concurrency, or transfer guarantee.
- D-030 remains in force. Graph order, source position, matching properties,
  matching types, and generated blank-node labels remain invalid joins.
- W-016 can run document-level negative cases through one private boundary after
  its expected-result inventory is complete.
