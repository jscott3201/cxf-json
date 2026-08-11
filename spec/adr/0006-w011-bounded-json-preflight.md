# ADR 0006: W-011 bounded JSON preflight

Status: Accepted

Date: 2026-08-11 UTC

Compatibility impact: Additive

## Context

Profile 0.1.1 limits submitted bytes before source retention but leaves JSON
structure unbounded. The qualified probe scanner is iterative and rejects malformed
JSON and duplicate decoded member names, but it measures structure without
enforcing production limits.

Measured files reach depth 5, 12 members in one object, 9,014 values, and 50,930
decoded member-name bytes. Successful generated cases reach depth 64, 4,096 members,
32,771 values, and 131,072 decoded member-name bytes. These observations establish
compatibility floors, not process-memory ceilings.

## Decision

Profile 0.1.2 adds four inclusive `ParseOptions` limits: depth 64, 4,096 members per
object, 65,536 total values, and 262,144 decoded member-name bytes. Each limit may
be overridden, including with zero.

`cxf-json` adds one crate-private production seam that accepts borrowed bytes,
performs byte admission, and runs an iterative bounded JSON preflight. No public
preflight or parse function is added. The private scanner uses Serde JSON only to
decode object member names; no Serde type enters a public signature.

Preflight success and post-admission failure retain the one admitted source
allocation. Oversized input remains source-free. Failure kinds and messages remain
private, and duplicate failures do not retain or render the decoded name.

## Consequences

- W-007 can consume a private proof that byte admission and JSON preflight passed.
- Profile 0.1.2 still defines no accepted public JSON or CXF syntax and no stable
  diagnostic codes.
- The probe remains evidence rather than a production dependency. Production tests
  replay the reviewed seeds and generated structural boundaries directly.
- W-011 remains open for diagnostic-count, quad, and retained-term budgets. The
  structural limits do not bound JSON-LD expansion, process memory, or execution
  time.
