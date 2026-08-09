# W-003: CXF ingestion qualification

Status: active.

## Purpose

Decide whether the guarded OxJSONLD/OxRDF adapter can remain a private stage in
the purpose-built CXF parser. This work qualifies only the JSON-LD behavior used
by CXF. It does not create a general JSON-LD or RDF API.

## Evidence sources

CI uses fixtures authored in this repository. They cover the CXF wire forms and
JSON-LD operations listed below without copying W3C, OBC, modelica-json, or Open
Control Engine fixtures.

The sibling Open Control Engine repository is a read-only external corpus. Runs
record its exact commit and consume files in place. No source, fixture, golden,
or generated artifact is copied from that repository. Its own `oce-cxf` fixtures
characterize a supported consumer dialect; its vendored modelica-json files
characterize producer output. Neither corpus is a normative CXF oracle.

W-005 still owns redistribution and test-fetch policy for external fixtures.

## Operation matrix

| CXF need | JSON-LD behavior | Qualification |
|---|---|---|
| Flat CXF document | top-level `@graph` | owned compact and full-IRI fixtures |
| S231 terms and classes | compact IRI expansion | compact and full-IRI forms produce equal RDF |
| CXF references | node objects with `@id` | referenced blocks, ports, parameters, and datatypes retain identity |
| CXF scalar/list spelling | scalar and array property values | single-value forms produce equal RDF |
| Context composition | ordered inline context maps | later term definition wins |
| Parameter values | value objects and XSD datatypes | lexical value and datatype survive conversion |
| Graph-set semantics | ordinary JSON arrays are unordered | reordered `containsBlock` values produce equal RDF |
| Deterministic loading | no implicit remote retrieval | remote context fails without network capability |

The array-order case is a required negative capability result. Open Control
Engine assigns meaning to `@graph`, `containsBlock`, port, parameter, and
connection array order. RDF set equality cannot carry that order. OxJSONLD may
therefore resolve terms and relationships, but it cannot be the only CXF
representation. A retained source/DTO path must own order-sensitive projection.

## Harness

`qualify_cxf_corpus` recursively reads explicit `.jsonld`/`.cxf.json` files or
directories, canonicalizes file identities, runs the private adapter, and writes
a JSON report containing file counts, input bytes, quad counts, failures, and
elapsed time. Expected failures and their exact diagnostic messages are explicit
command arguments. Any new or changed failure, unexpected success, read failure,
symlink root or discovered symlink entry, or non-UTF-8 path makes the command
fail. Explicit roots are canonicalized after the root check. The harness performs
no discovery outside those canonical roots and follows no network references.
For external evidence, `--git-root` and `--git-commit` require the selected CXF
files to match `git ls-files`, the requested commit, and a clean scoped worktree
both before and after parsing. Git runs with `GIT_OPTIONAL_LOCKS=0` so status and
index reads do not refresh the sibling repository's index.

The harness is an example target in the unpublished probe crate. It is not a
product CLI or public API.

## Acceptance gates

1. The owned operation matrix passes in native CI.
2. The owned matrix and anonymous-node entropy path execute under
   `wasm32-unknown-unknown` in Node; unexpected WASM imports fail the run.
3. Every read-only external corpus file receives a parse or classified-failure
   result; no failure is silently skipped.
4. The evidence report records the external commit, corpus counts, total bytes,
   elapsed time, maximum resident memory, release artifact size, and failures.
5. The dependency and network-capability review remains clean.
6. The final disposition records how order-sensitive CXF projection avoids RDF
   order loss.
7. D-021 records the owner-approved `getrandom` renewal and its W-009/public
   release expiry.

## Out of scope

- Typed CXF projection and profile validation.
- Public Rust, Python, or JavaScript APIs.
- Public wasm-bindgen glue, browser execution, and deployable WASM package size;
  W-009 owns those.
- External fixture redistribution or automated fetching.
- Blank-node dataset canonicalization; W-012 owns cross-processor comparison.
- Per-node source mapping; W-004 owns that contract.
