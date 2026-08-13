# ADR 0010: Private validator core

Status: Accepted

Date: 2026-08-12 UTC

Compatibility impact: Additive

## Context

W-014 owns the versioned profile validator: structural and semantic rules
emit stable codes without discarding the parsed CXF document. Profile 0.1.5
provides the typed projection surface to validate. Producer validation gaps
(upstream issues #248, #209, #141, #133, #45) confirm that consumer-side
rules cannot rely on producer checks, while register rows C-008/C-009/C-015
show that weakly typed and absent-valued nodes are the norm.

## Decision

Profile 0.1.6 adds a private validator module consuming `&Projection`. C1
rules are spec-decided only (Table 8.2): connection endpoints resolving to
provably non-connector classes (`CXF-V-001`), connected connectors with known
disagreeing datatypes (`CXF-V-002`), `isOfDataType` outside its
connector/parameter/constant domain (`CXF-V-003`), grouping predicates on
provably non-block classes (`CXF-V-004`), and informational
parameter/constant value absence (`CXF-V-005`). Codes are a distinct,
validator-owned family (`CXF-V-*`) rather than a promotion of projection
codes, so rule coverage stays versionable through W-016's corpus work.
Findings carry severity, node index, and source token, and order by authored
evidence position.

Rules apply the register's benefit-of-the-doubt posture throughout:
knowledge that a node's class is provably wrong is required before any error
fires; absence is surfaced informationally and never rejects.

## Consequences

- No public surface change: public exports, parse entry points, option
  surface, and observation-module discipline are identical to 0.1.5.
- W-016's negative corpus gains five rule rows to cover; W-015 remains the
  owner of namespace acceptance policy and stays unblocked.
- The `CXF-V-*` allocation is the stability target W-016 and any future
  public validator surface pin against.
