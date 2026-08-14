# Owned projection fixture provenance

The `cxf-proj-*` fixtures were authored for this repository on 2026-08-12 for
work item W-013-C1 (`_research/W013-C1-CORE-VOCABULARY-PROJECTION.md`, local
and Git-ignored). Their structure derives from specification facts in the
published OBC CXF section and from factual observations of upstream producer
output shape recorded in `_research/results/W-013-UPSTREAM-SURVEY.md`
(namespace IRIs, predicate spellings, identifier forms, known damage
classes). The documents, identifiers, labels, descriptions, and values were
authored for this repository; no prose, examples, graphics, source, goldens,
or distinctive serializations were copied, translated, or derived from
`lbl-srg/modelica-json`, OBC, or another external fixture corpus. Instance
identities are project-authored; C1 fixture identities use
`https://example.test/` or relative IRIs, and C2 fixtures declare the
emitter-factual `ex: http://example.org#` prefix plus QUDT vocab prefixes as
vocabulary facts (register rows C-015/C-016/C-018).

D-024 permits specification facts and vocabulary identifiers in owned tests
and forbids storing, transmitting, distributing, or test-fetching external
fixture bytes; these fixtures comply. They are covered by the repository's
`MIT OR Apache-2.0` license. They are excluded from the qualified
benchmark corpus under `crates/cxf-json/tests/fixtures/` so the corpus
baseline recorded in `benchmarks.md` remains revision-honest.

W-013-C2 (`_research/W013-C2-ATTRIBUTES-UNITS-ANNOTATION-SURFACE.md`,
local and Git-ignored) added `cxf-proj-units.jsonld` and
`cxf-proj-annotation.jsonld` on 2026-08-12 under the same license and
exclusion terms. Their member shapes follow factual producer-output
observations recorded in the survey (QUDT prefix declarations, the 27-entry
unit mapping and `S231:<raw>` fallback shape, `xsd:decimal` typed-literal
`nominal` values, string graphics with unbalanced-paren damage, and the
register-documented `not undefined` conditional expression); all
identifiers, labels, descriptions, values, and path strings were authored
for this repository.

W-027 added `cxf-proj-composition.jsonld` on 2026-08-13 under the same license
and exclusion terms. It combines a relative authored identifier, a registered
CXF parameter class, and one project-authored extension predicate to prove that
JSON-LD acceptance, source projection, and validation remain independent
evidence. No external fixture structure or content was copied or transformed.

W-015 reuses the owned fixtures as recognition witnesses for the source-free
producer observations in `spec/producer-observations.json`. They are not
producer output and do not establish a compatibility pass for a producer
revision. No fixture bytes changed for this use.

SHA-256 checksums recorded at authoring:

- `8a12848e3d58d0412098ebfb135f6ef7fecacda20cf511da8595c10e8282eef8` `cxf-proj-artifact.jsonld`
- `e53a270ce8796d65a2dabd64cd8e3c4b861eb7fdfa8ee08ead7d301935224275` `cxf-proj-emitter.jsonld`
- `9cbfe63f1fa3942d5fc48b70b84d70a02385881025381efc11b0b76a751bfa63` `cxf-proj-empty.jsonld`
- `20c01cd1d5b3c3d0a8d9f454a80a92639bd75b317fdfb698016221aac0aa9170` `cxf-proj-encoded.jsonld`
- `2920df34c040b6965d422fa4439b59423847f75bedcd960cac3810850f6f673e` `cxf-proj-legacy-https.jsonld`
- `35335a71ba3d52c8a6ab47df87b172092d331398a7ca9d2674215e603f938abe` `cxf-proj-specform.jsonld`
- `d69d186922e4cb394c555ba64c02c4b9e65d35ab7bcaf213b0917a2e9b43ca68` `cxf-proj-weak.jsonld`
- `09d4e2201ccc1ee8e23849f70c032dcf57b08e427c9fe8fc97d084b7a4eb4730` `cxf-proj-units.jsonld`
- `70456176810b2009cb31714628a9d24029a58d259a192d159821c96101dd8b6f` `cxf-proj-annotation.jsonld`
- `2b25c1e341d6d4faed80a08a5cd2f746b216e483df969e41f18057617d738d32` `cxf-proj-composition.jsonld`
