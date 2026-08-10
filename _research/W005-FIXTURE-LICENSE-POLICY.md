# W-005: Fixture license and use policy

Status: complete. PR #7 merged as `fd489e9`.

## Purpose

Define which fixture bytes may enter this repository, its CI, and its release
artifacts. This is an engineering governance decision based on the available
license and provenance evidence, not legal advice or a conclusion that all
external material is unusable.

## V1 policy

Repository fixtures and all automated test, policy, packaging, and release jobs
use only fixtures independently authored for this project. No external fixture,
source example, golden, generated output, or transformed derivative may be
copied, normalized, reduced, embedded, cached, uploaded, logged, packaged, or
automatically fetched.

This restriction concerns stored, transmitted, and distributed bytes or
derivatives. The optional evidence path may parse exact pinned external bytes in
memory and retain aggregate counts and classifications under the rules below.

A project-authored fixture:

- is written independently for this repository;
- may implement interoperability facts and vocabulary identifiers;
- does not reproduce external prose, examples, graphics, distinctive
  serialization, source code, or generated output;
- records authoring date, project license, provenance statement, and SHA-256.

Published specifications and upstream repositories remain reference material.
Links and concise factual citations do not authorize copying their expression.

## External evidence

External corpus qualification is an optional operator action, separate from CI.
The existing harness may read an operator-supplied local checkout only when it:

1. verifies that the checkout origin matches a project-approved expected origin
   and that HEAD matches the approved full commit, with `--git-root` naming the
   repository top level;
2. accepts explicit selected roots and requires their discovered CXF path set to
   match the approved commit tree;
3. reads each selected payload from the approved commit's Git object database,
   then parses those same bytes;
4. rejects symlinks during discovery, non-regular commit-tree entries,
   commit-tree membership mismatches, blob-read failures, and changed
   expected-failure classifications;
5. disables replacement objects, lazy object fetches, hooks, optional locks,
   system/global config, fsmonitor, untracked cache, preloaded index, and pagers
   for its Git plumbing;
6. performs no fetch, network context load, script, generator, worktree filter,
   or other checkout-controlled execution; and
7. reports paths, counts, byte counts, classifications, and aggregate
   measurements without copying source payloads into this repository or CI
   artifacts.

The approved Git commit and object reads bind parsed bytes to the recorded
revision even if the worktree or index changes concurrently. An arbitrary
successful harness invocation is not approved project evidence unless its
expected origin/commit pair is recorded by the governing work item. Exact
expected messages are matched only in memory. Every source-derived external
diagnostic message, range, pointer, RDF term, and read error is redacted before
report serialization. Reports may retain paths, an expected-origin-match boolean,
source-free failure stages, per-file counts and timing, expected-failure
configuration booleans, and aggregate classification counts; the origin value is
not serialized and the match does not establish project approval. Git-backed
mode requires Git 2.45 or newer so
`GIT_NO_LAZY_FETCH` is enforced. Per-file SHA-256 and complete notice manifests
become mandatory if any future exception proposes storing or distributing those
bytes. Reading a local corpus does not decide that it may be redistributed.

## Rights and provenance buckets

| Material | Observed evidence | V1 disposition |
|---|---|---|
| Repository-owned fixtures | Authored here; `MIT OR Apache-2.0`; checksums in W-024 and W-003 evidence | Allowed in repository, CI, and packages subject to package policy |
| OBC specification prose, examples, and assets | No license file at pinned root; GitHub license endpoint returns 404; rendered site states all rights reserved | Link and cite facts only; do not copy, transform, fetch in tests, or package |
| `modelica-json` source, fixtures, and `CXF-Core.jsonld` | Literal `BSD-3-Clause-LBNL`-style text plus DOE notice; package metadata says `BSD-3-Clause`; GitHub reports `NOASSERTION` | Do not vendor, transform, or fetch in v1 |
| Modelica Buildings source and generated CXF | `Buildings/legal.html` contains redistribution conditions, DOE notice, no-endorsement clause, and enhancement paragraph | Do not vendor, transform, or fetch in v1 |
| Open Control-owned fixtures | Root declares `MIT OR Apache-2.0`, but fixture provenance is not uniform enough for a whole-tree conclusion | Read pinned local paths only; no copying in v1 |
| Open Control `third_party/modelica-buildings-cdl/cxf` | 44 generated derivatives from Buildings `a131864…` using `modelica-json` `85721b8…`; Open Control records the Buildings license | Read pinned local paths only; treat as generated mixed-origin material |

`modelica-json`'s package field is not the literal license authority. SPDX names
the observed variant `BSD-3-Clause-LBNL`. Its added clause grants a default
license for enhancements made available publicly or directly to LBNL without a
separate written license agreement. The Buildings legal text uses the same
family and also records U.S. Government rights. Any future review must preserve
the complete literal texts rather than substituting the package field or a
generic BSD notice.

Generated output is not classified solely by the generator's license. The 44
Buildings CXF files retain source-derived structure and documentation, and
`CXF-Core.jsonld` contains generated vocabulary and block material. A future
exception must trace both source and generator rights.

## Future exception gate

Any exception to the v1 copy, transform, generate, embed, cache, upload, package,
or automatic-fetch restrictions requires a new owner-approved review for each
rights/provenance bucket and artifact class, not one approval for an entire
repository. The review must record:

- canonical repository URL, full commit, original path, and retrieval date;
- per-file and aggregate hashes;
- literal license and notice URLs plus their hashes;
- copyright, conditions, disclaimers, government notices, and no-endorsement
  obligations;
- whether each file is verbatim, modified, generated, or independently authored;
- for generated files, source and generator commits, source-to-output mapping,
  command, runtime, options, input hashes, and output hashes;
- every source archive, crate, wheel, npm package, WASM bundle, documentation
  artifact, cache, or CI artifact that would contain the bytes;
- the placement of complete literal notices in every source, binary,
  documentation, cache, and package artifact containing the bytes; and
- an explicit ruling on public-history implications under D-015.

OBC content needs an explicit license, written permission, or separate legal
clearance before such an exception. Attribution alone is not treated as
permission.

## Downstream obligations

- W-007 and later semantic work continue with owned fixtures plus optional local
  read-only evidence.
- W-021 must inspect actual package and documentation contents for fixtures,
  embedded bytes, generated files, notices, and license texts. Cargo dependency
  scanning does not establish fixture provenance.
- W-025 must inspect all reachable private history for external or derived
  content even if it was later deleted. A history rewrite invalidates the audit.
- A missing external checkout means external evidence was not run. It must not be
  reported as a pass.

## Evidence sources

- [`modelica-json` literal license](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/LICENSE.md)
- [`modelica-json` copyright and DOE notice](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/COPYRIGHT.md)
- [`modelica-json` package metadata](https://raw.githubusercontent.com/lbl-srg/modelica-json/85721b828a6ff8d9d3c1a48ff9a59808d2fa31fb/package.json)
- [SPDX `BSD-3-Clause-LBNL`](https://spdx.org/licenses/BSD-3-Clause-LBNL.html)
- [Modelica Buildings literal legal text](https://raw.githubusercontent.com/lbl-srg/modelica-buildings/a131864e4c4df22ebcd52bb8da439de0087ac365/Buildings/legal.html)
- [pinned OBC root](https://api.github.com/repos/lbl-srg/obc/contents?ref=e1c74224778b12297ee49455719c6e58ec71f810)
- [published OBC CXF section](https://obc.lbl.gov/specification/cxf.html)
- [Open Control root license metadata](https://raw.githubusercontent.com/jscott3201/open-control-engine/aa7bbae3373abb9b1a5bbb486803d39d15011b4f/Cargo.toml)
- [Open Control Buildings/CXF provenance](https://raw.githubusercontent.com/jscott3201/open-control-engine/aa7bbae3373abb9b1a5bbb486803d39d15011b4f/third_party/modelica-buildings-cdl/README.md)
