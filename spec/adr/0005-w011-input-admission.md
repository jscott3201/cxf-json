# ADR 0005: W-011 input-byte admission

Status: Accepted

Date: 2026-08-10 UTC

Compatibility impact: Additive

## Context

The core contract retained exact source bytes but provided no fallible admission
boundary. W-004 requires oversized input to be rejected before the library keeps
an owned source copy. `ParseError` cannot represent that failure because it always
contains an admitted `SourceDocument`.

The measured producer corpus has a largest single file of 418,986 bytes. The
resource-stress and coverage-guided evidence in PR #14 does not establish a memory
ceiling or safety threshold. It does provide enough compatibility evidence for the
owner to select a 1 MiB initial input cap while leaving JSON structure and later
processing budgets to separate work.

## Decision

Profile 0.1.1 adds an inclusive 1,048,576-byte default to `ParseOptions` and a
borrowed `SourceDocument::admit_bytes` constructor. Accepted input is copied once
into an owned `SourceDocument`. Oversized input returns a separate
`AdmissionError` containing only the observed and configured byte counts; no source
bytes are retained.

Admission checks byte length only. It does not validate UTF-8, JSON, JSON-LD, or
CXF. The existing `SourceDocument::from_bytes(Vec<u8>)` remains a raw ownership
constructor for data that has already crossed the appropriate boundary.

## Consequences

- The change is additive because existing constructors and error contracts remain
  unchanged and no parse entry point previously accepted input.
- The 1 MiB value is a compatibility policy, not a claim about bounded downstream
  memory or execution time.
- W-011 still owns bounded nesting, members per object, total values, decoded
  member-name bytes, diagnostics, and semantic processing budgets before W-007
  exposes a parse path.
- A future parse entry point may add a broader failure enum without weakening the
  invariant that `ParseError` retains an admitted source.
