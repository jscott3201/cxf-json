# ADR 0002: CXF JSON project name

Status: Accepted

Date: 2026-08-10 UTC

## Context

The original `modelica-json-rust` name implied a Rust port of the upstream
`modelica-json` producer. This project instead consumes Control eXchange Format
(CXF) JSON-LD and keeps Modelica and CDL source parsing outside its boundary.
No public package or behavior-bearing profile exists yet.

## Decision

The project display name is **CXF JSON**. The repository and intended primary
Rust package use `cxf-json`; Rust code imports that package as `cxf_json`. The
existing `cxf-ingest-probe` package remains an unpublished evidence crate.

Python and npm distribution names remain open until their adapter work defines
the package boundaries.

## Consequences

- The rename does not change observable behavior or advance profile version
  0.0.0.
- Package descriptions spell out Control eXchange Format and JSON-LD because
  CXF has unrelated meanings in other ecosystems.
- References to the upstream `lbl-srg/modelica-json` project retain that name.
