# Benchmarks

Status: Initial evidence-probe baseline measured 2026-08-10. No production parser
or performance threshold exists.

## Scope

These measurements cover the qualified `cxf-ingest-probe` process before W-007
adds semantic ingestion to `cxf-json`. They establish instrumentation and a
comparison format for later parser stages. They do not prove resource safety,
select admission limits, or represent package performance.

The corpus harness reports:

- maximum JSON nesting and members in one object;
- total JSON values and decoded member-name bytes;
- preflight and JSON-LD/RDF stage time;
- emitted quads and retained RDF summary bytes;
- total corpus time and process maximum RSS.

The Node runner reports WASM module size, compile time, instantiation time,
execution time, and linear memory before and after the smoke workload.

## Environment

| Component | Value |
|---|---|
| Machine | Apple M5, arm64 |
| Operating system | macOS 26.6 (25G70) |
| Rust | rustc 1.97.1 (8bab26f4f, 2026-07-14) |
| Cargo | 1.97.1 (c980f4866, 2026-06-30) |
| Node | 26.7.0 |
| Git | 2.55.0 |
| Build | `--release`, locked dependencies |
| Runs | Five independent process executions per corpus and WASM workload |
| Instrumentation revision | `7a69e58e821eb5ebf36a55dcc67d673ec11cd7a9` |

Repository-owned evidence was read from this repository's Git object database at
the instrumentation revision. External evidence was read from the local Open
Control Engine object database at commit
`8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67`. No external fixture bytes or reports
are stored in this repository. Every discovered file contributed complete JSON
structure metrics.

Each report carries the instrumentation revision and a per-execution ID. The
aggregator requires five unique runs from one revision. Native reports must also
share the verified corpus commit and file identity; WASM reports must share the
module SHA-256.

## Structural Baseline

| Corpus | Files | Total bytes | Largest file | Max depth | Max members | JSON values | Member-name bytes | Quads | RDF term bytes median (range) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Repository-owned fixtures | 8 | 5,738 | 2,456 | 5 | 7 | 198 | 1,266 | 58 | 6,486 (fixed) |
| OCE CXF fixtures | 164 | 2,519,687 | 418,986 | 5 | 9 | 56,905 | 324,419 | 24,795 | 4,016,996 (fixed) |
| Pinned Buildings producer corpus | 44 | 2,114,507 | 201,721 | 5 | 12 | 34,716 | 238,868 | 18,522 | 4,181,608 (fixed) |

Depth counts simultaneous open arrays and objects; a root container has depth 1.
RDF term bytes count every owned occurrence, including repeated strings and
processor-generated blank-node identifiers. The metric is not a canonical graph
size. The one expected remote-context failure contributes JSON structure and
timing but no quads or RDF term bytes.

## Native Baseline

Times are microseconds. Each cell reports the median and minimum-to-maximum range
across five runs. Stage throughput is calculated for each run as total corpus
bytes divided by that run's combined preflight and JSON-LD/RDF time, then
aggregated. Stage time includes owned per-file result construction and
the `SourceDocument` copy. Corpus time includes the measured stages, discovery,
two Git verification passes, blob reads, and inter-file aggregation. It excludes
final JSON serialization. Throughput uses decimal megabytes: 1 MB is 1,000,000
bytes.

| Corpus | Preflight | JSON-LD/RDF | Stage throughput | Corpus time | Maximum RSS |
|---|---:|---:|---:|---:|---:|
| Repository-owned fixtures | 75 (68-249) | 323 (310-895) | 14.3 MB/s (5.0-15.2) | 89,626 (83,889-99,514) | 7,929,856 bytes (7,913,472-7,929,856) |
| OCE CXF fixtures | 7,875 (7,388-8,357) | 38,730 (38,098-40,628) | 54.2 MB/s (51.4-55.4) | 917,663 (873,629-944,014) | 12,730,368 bytes (12,468,224-12,926,976) |
| Pinned Buildings producer corpus | 5,248 (5,034-5,535) | 25,057 (24,148-25,768) | 70.2 MB/s (67.5-72.4) | 292,072 (286,993-329,086) | 8,847,360 bytes (8,716,288-9,191,424) |

## WASM Baseline

The release `wasm_cxf_smoke.wasm` workload exercises compact/full-IRI equality,
order loss, inline contexts, anonymous nodes, offline remote-context failure,
duplicate rejection, large exponents, and deep nesting.

| Metric | Median | Range |
|---|---:|---:|
| Module size | 621,638 bytes | fixed |
| Module SHA-256 | `19e8e2a9cbb31476971f01e4ac869239ee3dfa0f02a196113885b15eb00a4c8f` | fixed |
| Compile | 627 us | 514-1,099 us |
| Instantiate | 84 us | 66-148 us |
| Execute | 6,660 us | 6,348-8,098 us |
| Initial linear memory | 1,179,648 bytes | fixed |
| Final linear memory | 1,310,720 bytes | fixed |

The runner samples linear-memory capacity before and after execution. WASM memory
cannot shrink, so the final value is also the observed capacity high-water mark.
It is not a count of live bytes and excludes host runtime memory.

## Reproduction

The native commands require macOS `/usr/bin/time -l`; RSS output and process
timing are comparable only on an equivalent environment. Run each workload five
times. Replace `RUN` in the commands below with the run number. Keep standard
output and `/usr/bin/time` output separate so `ci/summarize-benchmarks.py` can
aggregate them.

From the repository root, build the native harness and run the owned corpus:

```bash
CXF_JSON="$(pwd)"
REVISION=7a69e58e821eb5ebf36a55dcc67d673ec11cd7a9
CXF_BENCHMARK_REVISION="$REVISION" \
  cargo +1.97.1 build --release --example qualify_cxf_corpus --locked
/usr/bin/time -l target/release/examples/qualify_cxf_corpus \
  --git-root "$CXF_JSON" \
  --git-origin git@github.com:jscott3201/cxf-json.git \
  --git-commit "$REVISION" \
  --expect-failure "$CXF_JSON/crates/cxf-ingest-probe/tests/fixtures/remote-context.jsonld" \
  "No LoadDocumentCallback has been set to load remote contexts" \
  "$CXF_JSON/crates/cxf-ingest-probe/tests/fixtures" \
  > "owned-RUN.json" 2> "owned-RUN.time"
```

Using the revision-bound native binary built above, run the pinned external corpus
from a separately acquired checkout:

```bash
OCE=/path/to/open-control-engine
/usr/bin/time -l target/release/examples/qualify_cxf_corpus \
  --git-root "$OCE" \
  --git-origin git@github.com:jscott3201/open-control-engine.git \
  --git-commit 8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67 \
  --expect-failure "$OCE/crates/oce-cxf/tests/fixtures/node_scoped_contexts.jsonld" \
  "No LoadDocumentCallback has been set to load remote contexts" \
  "$OCE/crates/oce-cxf/tests/fixtures" \
  > "oce-RUN.json" 2> "oce-RUN.time"
```

Run the producer corpus under the same pin:

```bash
OCE=/path/to/open-control-engine
/usr/bin/time -l target/release/examples/qualify_cxf_corpus \
  --git-root "$OCE" \
  --git-origin git@github.com:jscott3201/open-control-engine.git \
  --git-commit 8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67 \
  "$OCE/third_party/modelica-buildings-cdl/cxf" \
  > "buildings-RUN.json" 2> "buildings-RUN.time"
```

Build and measure the WASM smoke workload:

```bash
REVISION=7a69e58e821eb5ebf36a55dcc67d673ec11cd7a9
cargo +1.97.1 build --release --example wasm_cxf_smoke --locked \
  --target wasm32-unknown-unknown
CXF_BENCHMARK_REVISION="$REVISION" node ci/run-wasm-smoke.mjs \
  target/wasm32-unknown-unknown/release/examples/wasm_cxf_smoke.wasm \
  --metrics > "wasm-RUN.json"
```

Aggregate the five reports for each workload:

```bash
python3 ci/summarize-benchmarks.py corpus owned-{1,2,3,4,5}.json \
  --times owned-{1,2,3,4,5}.time
python3 ci/summarize-benchmarks.py corpus oce-{1,2,3,4,5}.json \
  --times oce-{1,2,3,4,5}.time
python3 ci/summarize-benchmarks.py corpus buildings-{1,2,3,4,5}.json \
  --times buildings-{1,2,3,4,5}.time
python3 ci/summarize-benchmarks.py wasm wasm-{1,2,3,4,5}.json
```

## Gaps And Update Rules

- Allocation counts are not measured. macOS `/usr/bin/time -l` reports process
  maximum RSS, which includes allocator and runtime state rather than parser-only
  retained memory.
- The corpus lacks adversarial width, depth, JSON-LD expansion, delayed `@id`, and
  diagnostic-amplification cases. Observed maxima are not safe hard caps.
- OxJSONLD has no project-controlled hard timeout or allocation budget. Native
  subprocesses and browser/Node Workers remain the termination boundary.
- W-007 must add stage measurements for ordered DTO construction, private graph
  indexing, and semantic joins.
- W-011 must measure adversarial amplification before adopting member, value,
  diagnostic, quad, or retained-term limits.

Update this file whenever a parser stage, corpus pin, dependency version, target,
or benchmark method changes. Record the tested commit, environment, commands,
five-run median and range, structural maxima, memory, artifact size, and any
metric that remains unavailable. Compare regressions only on equivalent methods
and environments.
