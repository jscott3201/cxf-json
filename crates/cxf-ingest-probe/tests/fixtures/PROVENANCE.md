# Owned fixture provenance

The W-024 fixtures were authored for this repository on 2026-08-09. They use
only `https://example.test/` identifiers and do not copy, translate, or derive
from `lbl-srg/modelica-json`, OBC, or another external fixture corpus.

The fixtures are covered by the repository's `MIT OR Apache-2.0` license. SHA-256
checksums are recorded in `_research/results/W-024.md` after verification.

Invalid UTF-8 cases are assembled in test code because an invalid byte sequence
cannot be represented in a JSON text fixture.

The `cxf-*` fixtures were authored for W-003 on 2026-08-09 from the operation
matrix in `_research/W003-CXF-INGESTION-QUALIFICATION.md`. They use CXF vocabulary
IRIs and repository-owned `https://example.test/` identities. Their structure and
content were not copied, translated, or derived from an external fixture.
