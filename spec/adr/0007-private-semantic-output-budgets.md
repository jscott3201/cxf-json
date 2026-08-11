# ADR 0007: Private semantic output budgets

Status: Accepted

Date: 2026-08-11 UTC

Compatibility impact: Additive

## Context

Profile 0.1.2 bounds admitted bytes and JSON structure before semantic processing.
The qualified OxJSONLD/OxRDF probe has no project-controlled quad, retained-term,
diagnostic, allocation, or time budget. Its largest successful generated case emits
32,768 quads and retains 2,392,064 RDF term bytes. The largest recorded corpus
aggregate retains 4,181,608 term bytes. These are compatibility observations, not
memory-safety thresholds.

OxJSONLD can allocate multiple internal errors before yielding one iterator item,
and one iterator step may allocate backend output before a project wrapper sees it.
Project output limits therefore cannot be presented as backend, heap, or deadline
limits.

## Decision

Profile 0.1.3 adds inclusive defaults of 65,536 emitted RDF quads and 8,388,608
retained RDF term bytes. Emitted occurrences count before deduplication. Retained
term bytes count each owned subject, predicate, object, datatype, language tag, and
non-default graph-name string occurrence. Quad-limit failure wins when one emitted
quad would exceed both limits.

The unpublished `cxf-json` crate adds a default `semantic-ingestion` feature for
optional OxJSONLD/OxRDF dependencies and the target-only D-021 entropy exception.
No-default builds retain the existing source and JSON preflight boundary. The
private adapter requires a document IRI, installs no loader, keeps backend types
out of public signatures, emits at most one fixed project failure, and returns no
partial graph after failure.

## Consequences

- M1-C6 starts private W-007 ingestion without adding a supported public parse
  function or typed CXF values. Explicit project instrumentation builds may expose
  the doc-hidden observation module described by the profile; it is not a supported
  package API.
- W-011 remains open for backend diagnostic amplification, execution deadlines,
  and process-memory policy. M1-C9 tests one Linux worker mechanism under project
  instrumentation without closing D-029 or adding parser options.
- PR #18 records the first clean-revision production baseline. M1-C8 adds native
  stage measurements; W-022 remains continuous regression work.
- M1-C6 builds a lossless ordered source view. D-030 leaves the RDF identity join
  absent until the backend provides source-correlated expanded IDs; processor
  blank-node identifiers are not source identity.
- D-021 keeps its W-009 or first-package-release expiry.
