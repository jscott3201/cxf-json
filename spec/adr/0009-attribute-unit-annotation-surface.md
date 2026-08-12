# ADR 0009: Attribute, unit, and annotation surface in the private projection

Status: Accepted

Date: 2026-08-12 UTC

Compatibility impact: Additive

## Context

ADR 0008 lands the C1 core-vocabulary projection. The pinned emitter
(`modelica-json@85721b82` `lib/cxfExtractor.js`, `json2mo/graphic.js`) writes
members that C1 retained only as extension records: parameter attributes
(`start`, `nominal`, `fixed`, `instantiate`, `min`, `max`), unit members under
QUDT namespaces (`qudt:hasUnit`, `S231:hasDisplayUnit`,
`qudt:hasQuantityKind`), string `graphics` re-serialized from annotation
objects (issue #278 documents damage), string `conditionalExpression` members
(issues #321/C-006 document corruption), and metadata members (`defaultValue`,
`generatePointlist`, `controlledDevice`). Spec Table 8.2 contains none of
these except `hasFmuPath`, which C1 already indexes as text.

Review of the first C2 implementation exposed two registration defects: a
global term-by-namespace cross-product that admitted spellings like
`qudt:label` and `qudt:Real`, and a unit-target classifier that mislabeled
QUDT-schema-compacted targets as S231 fallbacks. Register rows C-017 and
C-018 record the evidence; C-014 closes — both CDL annotation spellings
(`extension`, `extenstion`) collapse upstream to the `ExtensionBlock` type
assertion, so no CXF-side misspelling exists to register.

## Decision

Profile 0.1.5 extends the private projection module: attribute values index as
opaque CXF values; `fixed` and `generatePointlist` index as booleans;
`graphics`, `conditionalExpression`, and `controlledDevice` index as verbatim
text; unit members index a new payload carrying role, verbatim target
spelling, and a classified target (QUDT unit IRI, QUDT quantity-kind IRI,
S231-generation emitter fallback, or other) that is never normalized or
resolved. Unit members accept arrays of reference objects per item.

Vocabulary registration uses per-identity allowlists: the three S231
generations register the S231 surface except the QUDT unit predicates, and
the QUDT schema namespace `http://qudt.org/schema/qudt#` registers only
`hasUnit` and `hasQuantityKind`. QUDT vocab namespaces
(`http://qudt.org/vocab/unit#`, `http://qudt.org/vocab/quantitykind#`) are
classification buckets for unit targets, not term vocabularies. No public
type, parse entry point, validation rule, or diagnostic code is added.

## Consequences

- No public surface change: profile 0.1.5's public export list, option
  surface, and observation-module discipline are identical to 0.1.4.
- This ADR supersedes ADR 0008's statements that three namespace generations
  constitute the complete registration and that graphics payloads degrade
  into extension records. ADR 0008's remaining decisions stand.
- Register rows C-017 (emitter attribute/metadata vocabulary beyond Table
  8.2) and C-018 (unit spelling instability) own fixtures and tests; C-014
  closes.
- W-014 owns validation rules over the new buckets (for example, FMU path
  domain on extension blocks); W-015 owns namespace acceptance policy for
  the QUDT identity; neither unblocks earlier.
