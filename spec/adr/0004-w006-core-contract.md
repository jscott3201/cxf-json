# ADR 0004: W-006 core contract boundary

Status: Accepted

Date: 2026-08-10 UTC

Compatibility impact: Initial

Version 0.0.0 defined no prior behavior or public API contract.

## Context

M0 qualified source retention, byte locations, private JSON-LD/RDF processing,
and native/WASM feasibility in the unpublished `cxf-ingest-probe`. The project
still had no production crate or normative public API. D-028 selected a
contract-only W-006 so later parser work does not freeze backend or host types into
the core API.

## Decision

W-006 adds one unpublished `cxf-json` crate. Profile 0.1.0 defines owned source
bytes, absolute document IRI validation, byte positions and ranges, diagnostics,
the future parse-error envelope, and parse options.

The crate exposes no parse entry point, typed CXF document, extension-record
schema, or Serde wire contract. OxIRI validates document IRIs behind a
project-owned type and error. JSON-LD, RDF, filesystem, HTTP, Python, and
JavaScript values remain outside public signatures.

W-007 owns the first semantic parse path. W-011 must set admission limits before
that path accepts untrusted input. W-013 owns concrete typed CXF and extension
records.

## Consequences

- `cxf-json` remains at package version 0.0.0 with `publish = false`.
- Profile 0.1.0 is behavior-bearing but makes no parsing, conformance, safety, or
  package-stability claim.
- Public contract files live under `crates/cxf-json/src/contract/`. CI requires a
  profile-version change, compatibility ADR, and profile-test change when that
  surface changes in a pull request.
- Host adapters cannot rely on a core Serde representation; a later profile must
  adopt any shared wire format.
