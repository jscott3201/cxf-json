# ADR 0008: Private core-vocabulary projection

Status: Accepted

Date: 2026-08-12 UTC

Compatibility impact: Additive

## Context

W-013 owns the typed CXF and extension model (D-028). Its first slice needs the
OBC section 8.2 core vocabulary projected from the W-007 ordered source view
while four decision constraints remain in force: D-020 takes ordering from the
source view rather than RDF, D-030 forbids source-to-RDF identity joins and
partial JSON-LD context expanders, D-014 keeps exact scalar spelling only in
retained submitted bytes, and the upstream discrepancy register (C-001 through
C-016) requires distinct IRI identity with a total extension fallback.

A public typed API is premature: namespace acceptance policy (W-015), stable
validation codes (W-014), and the negative corpus (W-016) do not exist yet.

## Decision

Profile 0.1.4 adds a private typed CXF projection module to the unpublished
`cxf-json` crate. The module consumes the ordered source view, owns the
admitted source document for token-range lifetimes, registers the three
distinct namespace generations `http://data.ashrae.org/S231#`,
`http://data.ashrae.org/S231P#`, and `https://data.ashrae.org/S231P#` without
merging them, gates compacted spellings on the document's own registered
`@context` mappings, classifies nodes and link edges with exact-string
reference resolution, keeps values opaque, and degrades every unrecognized or
wrong-shaped member into verbatim extension records.

Emitter damage recorded in the register becomes private diagnostics
`CXF-P-000` through `CXF-P-006`: non-object roots, weakly typed nodes,
conflicting type assertions, known broken-emitter value artifacts, malformed
references, unresolved references, and duplicate node identifiers. Subclass
type assertions merge to the most specific registered class; only incompatible
assertions diagnose.

Owned fixtures live under `crates/cxf-json/tests/projection/` with recorded
provenance, outside the qualified benchmark corpus directory so existing
`benchmarks.md` baselines stay revision-honest.

## Consequences

- W-013-C1 lands as PR #23 with no public surface change: profile 0.1.4's
  public export list, option surface, and observation-module discipline are
  identical to 0.1.3.
- W-014 consumes the typed model and decides which private `CXF-P-*` concepts
  stabilize as public diagnostic codes; W-015 consumes the registration table
  and decides namespace acceptance policy.
- Embedded reference-object content stays verbatim extension evidence in C1;
  promoting embedded nodes into first-class nodes is a later W-013 decision.
- The projection keeps authored grouping order as emitted order only (C-010);
  CDL declaration order reconstruction remains out of scope.
- Later W-013 slices (units, graphics, expressions, FMU references) extend the
  registration table through the normal profile process.
