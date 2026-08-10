# ADR 0001: Normative profile location

Status: Accepted

Date: 2026-08-10 UTC

## Context

The M0 research corpus records evidence and decisions but was intentionally
non-normative. OQ-012 required a stable location and change process before later
implementation work defined observable behavior.

## Decision

The normative project profile lives in `spec/PROFILE.md` and is the sole authority
for observable behavior. ADRs in `spec/adr/` record decisions and rationale and
may establish governance, but they do not independently define observable
behavior. Research decisions about observable behavior remain candidates until a
reviewed pull request promotes them into the profile. Observable contract changes
require a reviewed pull request that records compatibility impact, updates the
profile version, and updates enforcing tests. CI enforcement is required before
the first behavior-bearing profile. The process is defined in `spec/README.md`.

## Consequences

- `_research/` remains evidence and rationale rather than a second specification.
- Policy requires implementation changes that affect diagnostics or compatibility
  boundaries to update them explicitly; automated enforcement starts before
  profile 0.1.0.
- Later work must promote behavior into the profile instead of relying on
  research, roadmap, or meeting context.
