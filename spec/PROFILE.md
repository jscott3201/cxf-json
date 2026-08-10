# CXF project profile

Profile version: 0.0.0

Status: Reserved; no normative behavior profile adopted.

This file is the source-of-truth location for observable behavior beginning with
M1. Version 0.0.0 defines no accepted input, output, diagnostic, compatibility,
ordering, resource-limit, context-loader, or public API contract. It does not make
the M0 ingestion probe a production API or establish CXF conformance.

M0 research conclusions are candidates for promotion, not normative rules. Before
implementation relies on one, a reviewed pull request must add the rule here,
classify its compatibility impact, add or supersede an ADR, update this version,
and add enforcing tests.

The first behavior-bearing profile will be version 0.1.0 after its change process
has CI enforcement. A breaking pre-1.0 revision after that point increments the
minor version, such as 0.1.0 to 0.2.0. The complete process is defined in
`README.md`.
