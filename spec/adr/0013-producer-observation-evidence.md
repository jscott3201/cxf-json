# ADR 0013: Producer observation evidence

Status: Accepted

Date: 2026-08-13 UTC

Compatibility impact: Clarification

## Context

Profile 0.1.7 described HTTP S231P as post-v1.2 producer output. The released
`modelica-json` v1.2.0 and v1.3.0 commits use HTTPS S231P. Reference output
regenerated in the later transitional commit
`54777488ad08251d24f65d1ab2afc44b773200a5` uses HTTP S231P, while the pinned
operator corpus was generated from commit
`85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb`, which uses HTTP S231.

The repository cannot store or fetch external fixture bytes under D-024.
Project-owned fixtures can demonstrate recognition of observed vocabulary
facts, but cannot prove that an external producer revision emitted those bytes.

## Decision

Profile 0.1.8 corrects the producer-generation description without changing the
namespace acceptance matrix or IRI identity rules. A source-free manifest records
the producer repository, full commit, optional release, observed dialect facts,
immutable source URL, evidence class, and independently authored witness path for
each known generation.

CI validates the closed manifest schema, approved pins, immutable evidence URLs,
and no-follow paths under repository-owned fixture roots. It performs no network
access. Private projection tests establish only that each recorded dialect fact
has local recognition coverage; they do not attribute fixture bytes to a producer
or declare a producer version compatible.

## Consequences

- Public exports, parsing behavior, diagnostics, dependencies, and namespace
  identities do not change.
- Released v1.2.0 and v1.3.0 are recorded as HTTPS S231P observations, not HTTP
  S231P observations.
- External corpus qualification remains an optional operator action against
  exact Git objects. A missing checkout is not reported as a pass.
- Advancing or adding a producer pin requires an explicit checker update and
  review; changing the manifest alone cannot silently widen the evidence set.
