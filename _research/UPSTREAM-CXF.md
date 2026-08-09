# Upstream and CXF baseline

## What `modelica-json` is

`lbl-srg/modelica-json` is primarily a Modelica/CDL exporter. At pinned commit
[`85721b8`](https://github.com/lbl-srg/modelica-json/commit/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb),
the CXF path is:

```text
.mo source
  -> ANTLR Modelica grammar
  -> JavaScript object/reference extraction
  -> RDF graph construction
  -> rdflib JSON-LD serialization
```

Relevant source:

- [`app.js`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/app.js): CLI and output modes.
- [`lib/parser.js`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/lib/parser.js): file traversal and pipeline dispatch.
- [`lib/modelicaToJSON.js`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/lib/modelicaToJSON.js): Modelica parse stage.
- [`lib/cxfExtractor.js`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/lib/cxfExtractor.js): RDF/CXF mapping behavior.
- [`lib/s231ClassesProperties.ttl`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/lib/s231ClassesProperties.ttl): vocabulary and partial SHACL shapes.
- [`test/test_parser.js`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/test_parser.js): regression comparison behavior.

A Rust CXF reader starts at the final JSON-LD output. It does not need the
upstream Modelica grammar, JavaScript AST, or `MODELICAPATH`.

## Observed graph content

The emitter writes RDF terms for blocks, elementary blocks, extension blocks,
connectors, parameters, constants, enumerations, instances, contained blocks,
connections, arrays, units, quantity kinds, bounds, starts, nominal values,
replacement constraints, conditional expressions, and graphics.

Focused examples:

- [arrays and dimensions](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica/BlockWithArray2.jsonld)
- [enumerations](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica/Enumeration1.jsonld)
- [connections and nested instances](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica/CustomPWithLimiter.jsonld)
- [`extends` and `final`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica/ExtendsClause_4.jsonld)
- [replaceable components](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica/ReplaceableBlock.jsonld)
- [units and numeric attributes](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica/ParameterWithAttributes.jsonld)

## Reader obligations from the OBC specification

The pinned source is
[`cxf.rst`](https://raw.githubusercontent.com/lbl-srg/obc/e1c74224778b12297ee49455719c6e58ec71f810/specification/source/cxf.rst).
The [published OBC CXF section](https://obc.lbl.gov/specification/cxf.html) is the
rendered guidance; the pinned source remains the reproducible evidence revision.

A reader or validator needs to account for these rules:

- CXF uses JSON-LD and `application/ld+json`.
- A valid graph may contain elementary, composite, or extension blocks.
- Block instances use the inputs, outputs, parameters, and constants defined by
  their block definitions.
- Connections join input and output connectors of the same datatype.
- Instance identifiers contain a full package path, `#`, and instance name;
  child identifiers append `.` plus the child name.
- Array references may be preserved or flattened. Flattened indices use
  underscore-separated row-major notation.
- Expressions may be preserved or evaluated. A reader must accept both
  symbolic and evaluated forms.
- Translation software provenance is optional.
- Elementary block CXF omits the source `equation` implementation.
- Extension blocks require an FMU path under strict validation.
- Enumeration semantics are integer-valued from 1 and map each element to a
  unique string, even though current exporter output does not reliably preserve
  ordinal information.

These are graph/profile concerns. Source annotations, expression evaluation,
array-flattening choices, and source-name collision checks belong to the
exporter.

## No authoritative closed schema

The repository's `schema-cdl.json` and `schema-modelica.json` validate
intermediate parser output, not CXF graphs. Upstream issue
[#305](https://github.com/lbl-srg/modelica-json/issues/305) records that a
proposed CXF JSON Schema was removed after the committee selected JSON-LD. A
maintainer identifies JSON Schema, SHACL, or both as possible future validation
mechanisms.

The Turtle vocabulary has partial SHACL material, but it conflicts with current
emitter and specification terms. The exporter reads it when generating
`CXF-Core.jsonld`, so it is a build input and compatibility fixture, not the sole
conformance authority.

## Known discrepancies and loss

| ID | Area | Evidence | Reader posture |
|---|---|---|---|
| C-001 | Connection predicate is `connectedTo` in the OBC table and `isConnectedTo` in emitter/Turtle | [pinned OBC CXF source](https://raw.githubusercontent.com/lbl-srg/obc/e1c74224778b12297ee49455719c6e58ec71f810/specification/source/cxf.rst), [`cxfExtractor.js`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/lib/cxfExtractor.js) | Preserve both IRIs; map only under an explicit profile |
| C-002 | Historical `http`/`https` and `S231P` namespace variants | [issue #300](https://github.com/lbl-srg/modelica-json/issues/300) | Recognize through a versioned legacy policy, never silently merge identity |
| C-003 | Complex expressions can become JavaScript object text | [issue #302](https://github.com/lbl-srg/modelica-json/issues/302) | Preserve source value and diagnose unsupported typed projection |
| C-004 | Enumeration order is absent | [issue #303](https://github.com/lbl-srg/modelica-json/issues/303) | Do not infer order from JSON-LD graph order |
| C-005 | Connection routing graphics are dropped | [issue #304](https://github.com/lbl-srg/modelica-json/issues/304) | Do not promise source round-trip |
| C-006 | Conditional output includes values such as `not undefined` | [PR #321](https://github.com/lbl-srg/modelica-json/pull/321) | Add compatibility fixtures and typed diagnostics |
| C-007 | `DataType`/`Datatype`, `constrainedby`/`constrainedBy`, and QUDT forms differ | [Turtle vocabulary](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/lib/s231ClassesProperties.ttl) | Keep full IRI identity and profile-specific aliases |

## Compatibility corpus

The pinned tree contains approximately 216 committed CXF `.jsonld` references:

- 48 focused `test/FromModelica` expected outputs;
- 7 unique Modelica-mode expected outputs;
- about 160 generated Buildings library outputs;
- one 722,726-byte `CXF-Core.jsonld` graph.

Sources:

- [focused references](https://github.com/lbl-srg/modelica-json/tree/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf/test/FromModelica)
- [full reference tree](https://github.com/lbl-srg/modelica-json/tree/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/test/reference/cxf)
- [`CXF-Core.jsonld`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/CXF-Core.jsonld)
- [recursive tree used for counts](https://api.github.com/repos/lbl-srg/modelica-json/git/trees/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb?recursive=1)

Upstream compares the focused CDL fixtures and `CXF-Core.jsonld` as RDF triples;
Modelica-mode CXF fixtures still use serialized JSON comparison. Graphs
containing blank nodes need RDF dataset canonicalization for stable
cross-implementation equality; sorting serializer-assigned blank-node labels is
not sufficient.

## Release and license baseline

- CXF export arrived in
  [`v1.2.0`](https://github.com/lbl-srg/modelica-json/releases/tag/v1.2.0)
  against a public-review S231P draft.
- [`v1.3.0`](https://github.com/lbl-srg/modelica-json/releases/tag/v1.3.0)
  expanded arrays, annotations, graphics, inheritance, conditionals, QUDT data,
  and `CXF-Core.jsonld`.
- [`v2.0.0`](https://github.com/lbl-srg/modelica-json/releases/tag/v2.0.0)
  removed Java from normal operation and added Modelica-mode CXF export.
- The pinned `master` is newer than `v2.0.0`; compatibility records must use a
  commit or release asset rather than the stale `package.json` version `1.3.1`.

`package.json` declares BSD-3-Clause, but
[`LICENSE.md`](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/LICENSE.md)
contains an additional paragraph granting LBNL rights to publicly or directly
supplied enhancements. GitHub reports `NOASSERTION`. Legal review should use the
license text, not the SPDX field alone, before copying upstream source or
fixtures into a distributable package.
