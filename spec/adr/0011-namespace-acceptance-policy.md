# ADR 0011: Namespace acceptance policy (observational)

Status: Accepted

Date: 2026-08-12 UTC

Compatibility impact: Additive

## Context

W-015 requires namespace variants and spec/emitter predicate differences to
be accepted or rejected only through explicit policy and diagnostics, and
OQ-004 asks which legacy namespaces are supported while requiring that
recognition never assert global IRI equivalence. Since profile 0.1.5 the
projection already registers four identities with per-identity term
allowlists, keeps every distinct IRI distinct, and degrades unregistered
terms into verbatim extension records. What was missing: a normative
statement of which declared namespaces the profile accepts, and findings
when a document's `@context` behaves against convention.

The register supplies the cases: C-002 (legacy HTTPS S231P spelling),
C-016 (the same `S231` prefix legitimately maps to different S231 IRIs
across emitter generations), C-018 (QUDT namespaces), and the identity
discipline of C-001 (emitter/spec predicate spellings as distinct terms).

## Decision

Profile 0.1.7 adds a normative namespace acceptance matrix (PROFILE.md,
"Private namespace acceptance policy") with an **observational** policy:
input admissibility stays entirely with W-011 preflight, and no context
binding is rejected. The projection retains each declared root prefix
mapping verbatim (preserved order), and the validator emits findings from
the retained (last-write-wins) binding once per prefix:

- `CXF-C-001` (Warning) for a legacy-HTTPS S231P binding (C-002);
- `CXF-C-002` (Warning) for an unregistered namespace under a known family
  host (`data.ashrae.org`, `qudt.org`) — the actionable new-generation
  signal;
- `CXF-C-003` (Error) for a registered prefix bound to an unexpected
  namespace (`S231`/`S231P`/`qudt`/`unit`/`q`), because the binding cannot
  serve its conventional purpose: compacted spellings already fail
  registration by the existing gating, so the finding names the loss
  instead of silently mis-mapping it.

Duplicate prefix bindings inside one context object fail at W-011
preflight as duplicate JSON members, so the policy layer never sees them.

## Consequences

- No public surface change: exports, entry points, options, and the
  observation module are identical to 0.1.6.
- OQ-004 gains its matrix; evidence columns remain the owned fixtures and
  register rows, not redistributed upstream bytes (D-024).
- Rejection behavior remains concentrated in preflight; a future breaking
  policy change (rejecting a currently-observed namespace) would need a
  breaking profile version and owner sign-off, which is exactly what the
  observational C1 avoids committing to.
- Data namespaces (e.g. `ex`) and full-IRI spellings without context
  bindings are intentionally undiagnosed in this slice; widening is a
  later-priority decision with W-016 coverage.
