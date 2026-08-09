# Open questions

These questions block an API, dependency, or release commitment. Monday tracks
their state; answers and evidence belong here or in a decision record.

## Format and conformance

### OQ-001: What does "full JSON-LD support" mean for v1? (resolved)

Resolved by D-019: the project does not offer full general-purpose JSON-LD
support. It implements and tests the JSON-LD operations needed to parse CXF and
the accepted producer corpus. W-003 records that CXF-specific operation matrix
against the [JSON-LD 1.1 API](https://www.w3.org/TR/json-ld11-api/).
W-003 completed that processor-qualification evidence; D-022 records the guarded
private-development outcome.

### OQ-002: Which CXF authority defines the first strict profile?

Choices include a pinned OBC commit, a pinned `modelica-json` release/commit,
the Turtle vocabulary, or a project-authored reconciliation. None agree in all
terms today.

Cleared by: a named profile with source precedence and a discrepancy register.

### OQ-003: Must source-preserving round-trip be public in v1? (resolved)

Semantic round-trip and byte-for-byte round-trip are different contracts. The
latter requires retention of formatting, key order, number spelling, and likely
the original bytes. JSON-LD normalization cannot provide it by itself.

Resolved by D-014: no. V1 retains exact accepted bytes and parser error
positions; it does not promise per-node spans or byte-for-byte reserialization.
W-004 owns any later source mapper.

No approved dependency currently provides a full per-node source map. The
technically suitable `json-syntax` crate fails D-011. W-024 verifies the
raw-bytes and error-position mechanics but does not reopen the D-014 contract.

### OQ-004: Which legacy namespaces are supported?

Historical `S231P`, `http`/`https`, connection predicate, datatype spelling,
constraint spelling, and QUDT differences need a matrix. Recognition must not
assert that distinct IRIs are globally equivalent.

Cleared by: `W-015` and fixtures from known producer versions.

### OQ-005: Is SHACL part of v1 validation?

Current upstream shapes are partial and inconsistent. Native rules are easier
to version with stable diagnostics. SHACL may still matter for ecosystem
interchange.

Cleared by: an interoperability requirement and a validated shape source.

## API and targets

### OQ-006: Is a graph API, typed CXF API, or both public? (resolved)

Resolved by D-019: the public API is typed CXF plus a CXF extension view. RDF
graph and JSON-LD processor types remain internal. W-013 still owns the concrete
typed API design.

### OQ-007: Are remote contexts ever enabled by library-provided loaders?

The safer initial answer is no. If required, native HTTP, filesystem, browser,
and Node capabilities have different policies and async models.

Cleared by: a concrete producer document that cannot be processed with embedded
or caller-preloaded contexts.

### OQ-008: What Python versions and wheel platforms are supported?

This controls `abi3`, version-specific wheels, CPython 3.14t coverage, `abi3t`,
MSRV, CI cost, and package size.

Cleared by: owner product policy before `W-018`.

### OQ-009: What browser, bundler, and Node versions are supported?

This controls wasm-pack targets, ESM/CommonJS behavior, test runners, exports,
and TypeScript output.

Cleared by: owner product policy before `W-019`.

### OQ-010: What are the WASM size and memory gates?

Without thresholds, `W-003` and `W-009` can measure but cannot decide. Set raw
and compressed package size, peak memory on `CXF-Core.jsonld`, and parse-time
targets before the spikes.

## Distribution and governance

### OQ-011: May upstream fixtures be vendored?

The upstream license text differs from its SPDX metadata and includes an LBNL
enhancement grant. The project needs a deliberate choice between vendoring,
fetching pinned assets during preparation, or constructing independent fixtures.

Cleared by: `W-005` license review.

### OQ-012: Where does the normative project profile live?

The current `_research/` corpus is non-normative. Before implementation defines
behavior, adopt a versioned specification or ADR location and a process for
changing diagnostic and compatibility rules.

Cleared by: owner governance decision at the M0 exit.

### OQ-013: What stability gate permits public release?

The repository is private and the intended public license is `MIT OR
Apache-2.0`. "Stable" still needs objective criteria covering CXF conformance,
API compatibility, security and dependency review, Rust/Python/WASM target
gates, documentation, and release artifacts.

Cleared by: an owner-approved checklist before W-025 changes repository
visibility.

The history strategy is settled by D-015: preserve the full private history only
after `_research/results/W-025-history-audit.md` records coverage of all reachable
commits, tags, and refs plus secret, PII, proprietary/upstream-content, license,
binary, generated-artifact, and author-metadata findings. The remaining question
is the product stability checklist.
