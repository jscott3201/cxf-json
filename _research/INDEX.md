# modelica-json-rust research index

Status: research baseline, 2026-08-09. This corpus is non-normative until the
project adopts a specification and decision process.

## Current verdict

Build a purpose-built CXF parser, not a general JSON, JSON-LD, RDF, Modelica, or
CDL parser. JSON-LD expansion and RDF identity are internal mechanisms required
to interpret CXF. The public contract returns CXF values and CXF diagnostics,
retains original bytes and available error locations, and preserves unknown CXF
extension terms without exposing a general RDF toolkit.

Do not use pest for the CXF input. A PEG grammar would replace an established
JSON parser without handling JSON-LD expansion or CXF graph semantics. Keep
pest on the candidate list only if a later project adds direct CDL or Modelica
source parsing.

Serde and `serde_json` are qualified by D-018 for private-development use at
ordinary JSON and owned DTO boundaries. `oxjsonld` and `oxrdf` passed W-024
native/WASM feasibility and remain guarded W-003 candidates behind an internal
adapter, not the selected production processor. Production-release adoption
remains gated by W-023. `json-ld` and `json-syntax` are excluded from production
because their parent repositories do not meet the owner-established community
threshold.

## Documents

| File | Purpose |
|---|---|
| [SCOPE-AND-DECISIONS.md](SCOPE-AND-DECISIONS.md) | Scope, current research conclusions, and explicit non-goals |
| [UPSTREAM-CXF.md](UPSTREAM-CXF.md) | Pinned `modelica-json` and OBC CXF behavior |
| [PARSER-STRATEGY.md](PARSER-STRATEGY.md) | Parser comparison, JSON-LD boundary, and validation layers |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Proposed crate graph, data flow, API, and diagnostic contract |
| [PYTHON-PYO3.md](PYTHON-PYO3.md) | PyO3 0.29.2, free-threaded CPython, and wheel plan |
| [WASM.md](WASM.md) | wasm-bindgen/wasm-pack constraints and package plan |
| [ROADMAP.md](ROADMAP.md) | Milestones, work items, acceptance criteria, and risks |
| [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) | Decisions that require evidence or owner input |
| [FIRST-SLICE.md](FIRST-SLICE.md) | Selected W-024 scope, dependency gate, and acceptance tests |
| [DEPENDENCY-GOVERNANCE.md](DEPENDENCY-GOVERNANCE.md) | Frozen community, maintenance, version, and advisory evidence |
| [results/W-024.md](results/W-024.md) | First-slice implementation and verification evidence |

## Evidence policy

- Upstream `modelica-json` facts are pinned to commit
  [`85721b8`](https://github.com/lbl-srg/modelica-json/commit/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb),
  dated 2026-07-08, unless a source says otherwise.
- OBC CXF facts are pinned to commit
  [`e1c7422`](https://github.com/lbl-srg/obc/commit/e1c74224778b12297ee49455719c6e58ec71f810),
  dated 2026-07-21.
- A release number, crate version, target, API, or command is recorded only with
  a source link. Versions observed in this research are not dependency pins.
- Recommendations are labeled as decisions or proposals. Upstream fixtures
  show exporter behavior; they are not by themselves a normative CXF schema.
- Monday tracks IDs and state. These files hold the technical reasoning.

## Stable IDs

- Decisions: `D-001` onward.
- Provisional decisions: `D-P01` onward.
- Research and implementation items: `W-001` onward.
- Open questions: `OQ-001` onward.
- Risks: `R-001` onward.
- Compatibility findings: `C-001` onward.

Renaming a title must not change its ID.

## Monday project

Folder: **Modelica JSON Rust** in the Aionforge Labs workspace.

| Board | Purpose |
|---|---|
| [MJR Roadmap](https://aionforgelabs.monday.com/boards/18425770672) | M0-M4 and cross-milestone workstreams |
| [MJR Cycles](https://aionforgelabs.monday.com/boards/18425770673) | Dispatched and merged implementation cycles |
| [MJR Queue](https://aionforgelabs.monday.com/boards/18425770671) | W-001-W-025 state and priority |
| [MJR Defects](https://aionforgelabs.monday.com/boards/18425770675) | Observed implementation defect classes |
| [MJR Spec](https://aionforgelabs.monday.com/boards/18425770674) | Research documents, decisions, open questions, and compatibility findings |

W-024 was the first owner-ranked slice and is complete. W-003, W-004, W-005,
W-008, and W-009 retain Queue status `Queued` while remaining in the Backlog
group; other planned work also remains in Backlog.

Private repository:
[jscott3201/modelica-json-rust](https://github.com/jscott3201/modelica-json-rust).
PR [#1](https://github.com/jscott3201/modelica-json-rust/pull/1) merged W-024 as
`8b227a1278fade28d4157eb8ef615560b54b1a0d`.
D-013 keeps the repository private until the stability gate is defined and met,
then releases it under `MIT OR Apache-2.0`.
