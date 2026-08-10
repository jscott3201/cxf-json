# ADR 0001: Normative profile location

Status: Accepted

Date: 2026-08-10 UTC

## Context

The M0 research corpus records evidence and decisions but was intentionally
non-normative. OQ-012 required a stable location and change process before later
implementation work defined observable behavior.

## Decision

The normative project profile lives in `spec/PROFILE.md`. ADRs that carry
normative decisions live in `spec/adr/`. Research decisions remain candidates
until a reviewed pull request promotes them into the profile. Observable behavior
changes require a reviewed pull request that records compatibility impact,
updates the profile version, and updates enforcing tests. The process is defined
in `spec/README.md`.

## Consequences

- `_research/` remains evidence and rationale rather than a second specification.
- A merged implementation cannot silently change diagnostics or compatibility
  boundaries.
- Later work must promote behavior into the profile instead of relying on
  research, roadmap, or meeting context.
