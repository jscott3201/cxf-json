# ADR 0003: Public source before package release

Status: Accepted

Date: 2026-08-10 UTC

## Context

The repository was originally private pending a product-stability gate. The
owner made the repository public to develop the project as open source before a
stable API or distributable package exists.

## Decision

The source repository is public under `MIT OR Apache-2.0`. Source visibility is
independent of package publication, API stability, and CXF conformance. The
earlier private-development decision is superseded.

The `cxf-json` package remains unpublished. Package release still requires an
explicit support policy, dependency and license review, security and resource
limits, tested release artifacts, and an adopted behavior profile.

## Consequences

- Public documentation must identify the current probe and its missing
  production guarantees.
- Repository history is already public and still requires the planned history
  audit; public visibility does not count as audit evidence.
- This decision changes distribution state, not observable parser behavior, so
  profile version 0.0.0 does not advance.
