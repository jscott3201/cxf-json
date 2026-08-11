# Benchmarks

Status: Initial corpus and resource-stress baselines measured 2026-08-10. No
production parser or performance threshold exists.

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

The resource-stress harness generates 16 deterministic inputs for structural
width, depth, value density, decoded-name size, contexts, object order, RDF lists,
compact IRIs, and failure diagnostics. Each report carries the generator
parameters, input byte count, SHA-256, expected and actual outcome, stage timing,
and available structural and RDF metrics.

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
| Runs | Five independent process executions per corpus, resource-stress, and WASM workload |
| Corpus baseline revision | `7a69e58e821eb5ebf36a55dcc67d673ec11cd7a9` |
| Resource-stress revision | `8715cc908c8a239a6f809621bb3ec26debe06534` |

Corpus baseline evidence was read from this repository's Git object database at
its instrumentation revision. External evidence was read from the local Open
Control Engine object database at commit
`8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67`. No external fixture bytes or reports
are stored in this repository. Every discovered file contributed complete JSON
structure metrics.

Each report carries the instrumentation revision and a per-execution ID. The
aggregator requires five unique runs from one revision. Corpus reports must share
the verified commit and file identity. Resource-stress reports must share generator
version, case identity, structural metrics, and outcomes. WASM reports must share
the module SHA-256.

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

## Resource-Stress Baseline

Generator version 1 emits 1,133,933 bytes across 16 cases. Times are
microseconds; each timing cell reports the five-run median and range. RDF term
bytes are fixed unless a range is shown. The two expected failures retain source
bytes and diagnostic metadata but do not produce a successful graph.

| Case | Input bytes | Depth | Max members | JSON values | Result | RDF term bytes | Preflight | JSON-LD/RDF |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 262,144-byte string | 262,192 | 1 | 2 | 3 | 1 quad | 262,215 | 158 (142-443) | 154 (142-786) |
| Semantic depth 16 | 754 | 16 | 2 | 32 | 15 quads | 791 | 7 (6-7) | 53 (45-55) |
| Semantic depth 32 | 1,538 | 32 | 2 | 64 | 31 quads | 1,655 | 10 (8-10) | 114 (94-121) |
| Semantic depth 64 | 3,106 | 64 | 2 | 128 | 63 quads | 3,383 | 16 (14-17) | 308 (306-327) |
| 4,096-member object | 118,785 | 1 | 4,096 | 4,097 | 0 quads | 0 | 440 (433-452) | 1,259 (1,203-1,412) |
| 32,768 null values | 163,887 | 2 | 2 | 32,771 | 0 quads | 0 | 119 (111-131) | 1,781 (1,769-1,786) |
| 32,768 retained values | 65,583 | 2 | 2 | 32,771 | 32,768 quads | 2,392,064 | 140 (134-143) | 9,238 (8,986-9,308) |
| 512 names of 256 bytes | 135,169 | 1 | 512 | 513 | 0 quads | 0 | 191 (189-205) | 385 (373-432) |
| 65,536-byte decoded duplicate | 137,744 | - | - | - | JSON failure | - | 196 (187-202) | - |
| 512 context terms | 33,831 | 2 | 514 | 1,027 | 512 quads | 51,712 | 111 (108-122) | 659 (634-694) |
| 256 repeated local contexts | 36,377 | 4 | 512 | 1,795 | 256 quads | 20,992 | 150 (142-170) | 6,061 (5,986-6,515) |
| Early `@id`, 128 by 16 | 59,681 | 2 | 17 | 2,305 | 2,048 quads | 172,320 | 234 (214-259) | 1,014 (977-1,056) |
| Late `@id`, 128 by 16 | 59,809 | 2 | 17 | 2,305 | 2,048 quads | 172,320 | 225 (211-228) | 980 (964-1,030) |
| RDF list of 1,024 values | 2,105 | 3 | 2 | 1,028 | 2,049 quads | 237,451 (237,415-237,481) | 5 (5-6) | 623 (609-632) |
| 2,048 compact-IRI properties | 28,782 | 3 | 2,050 | 2,054 | 2,048 quads | 208,896 | 153 (148-162) | 1,315 (1,267-1,321) |
| 512 colliding keyword aliases | 24,590 | 2 | 513 | 1,026 | JSON-LD failure | 0 | 116 (110-123) | 394 (369-405) |

The complete suite took 28,739 us (28,381-31,364 us) with process maximum RSS
of 31,621,120 bytes (30,113,792-31,932,416 bytes). RSS covers the whole suite and
cannot be assigned to one case.

The measurements support four narrow conclusions:

- Retaining 32,768 repeated values emits 32,768 repeated quads and 2,392,064 RDF
  term bytes. The larger null-valued control emits no quads.
- Reprocessing a 512-term parent context for 256 local contexts takes roughly nine
  times the JSON-LD/RDF time of one 512-term context with 512 properties.
- Early and late `@id` controls are equivalent within run variation in regular
  OxJSONLD mode; these results do not show an order-specific cost.
- The decoded-duplicate case produces a 65,562-byte diagnostic message from a
  65,536-byte decoded name. Diagnostic size therefore needs an independent bound.

The pinned coverage-guided parser workspace completed 2,164,307 bounded local
executions in 11 seconds with no invariant failure. Its process controls are test
settings, not parser defaults.

## WASM Baseline

The release `wasm_cxf_smoke.wasm` workload exercises compact/full-IRI equality,
order loss, inline contexts, anonymous nodes, offline remote-context failure,
duplicate rejection, large exponents, reviewed parser seeds, deep nesting, and all
16 resource-stress cases.

| Metric | Median | Range |
|---|---:|---:|
| Module size | 661,976 bytes | fixed |
| Module SHA-256 | `61a826137098aebb86e5758f6743f88ee7f678cd3a8ebc06eea2466007603aeb` | fixed |
| Compile | 592 us | 553-1,445 us |
| Instantiate | 68 us | 63-324 us |
| Execute | 55,599 us | 53,536-60,885 us |
| Initial linear memory | 1,179,648 bytes | fixed |
| Final linear memory | 17,760,256 bytes | fixed |

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

Build and measure the generated resource-stress suite:

```bash
REVISION=8715cc908c8a239a6f809621bb3ec26debe06534
CXF_BENCHMARK_REVISION="$REVISION" \
  cargo +1.97.1 build --release --locked -p cxf-ingest-probe \
    --example report_resource_stress
/usr/bin/time -l target/release/examples/report_resource_stress \
  > "stress-RUN.json" 2> "stress-RUN.time"
```

Build and measure the WASM smoke workload:

```bash
REVISION=8715cc908c8a239a6f809621bb3ec26debe06534
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
python3 ci/summarize-benchmarks.py resource-stress stress-{1,2,3,4,5}.json \
  --times stress-{1,2,3,4,5}.time
python3 ci/summarize-benchmarks.py wasm wasm-{1,2,3,4,5}.json
```

`fuzz/README.md` records the pinned coverage-guided parser commands and their
test-process bounds.

## Gaps And Update Rules

- Allocation counts are not measured. macOS `/usr/bin/time -l` reports process
  maximum RSS, which includes allocator and runtime state rather than parser-only
  retained memory.
- The generated suite samples selected structural and graph growth patterns. It
  does not establish safe hard caps or cover every JSON-LD expansion shape.
- Process RSS is measured for the full resource-stress suite, not per case.
- OxJSONLD has no project-controlled hard timeout or allocation budget. Native
  subprocesses and browser/Node Workers remain the termination boundary.
- W-007 must add stage measurements for ordered DTO construction, private graph
  indexing, and semantic joins.
- The next W-011 slice must use this evidence to select or defer member, value,
  diagnostic, quad, and retained-term limits.

Update this file whenever a parser stage, corpus pin, dependency version, target,
or benchmark method changes. Record the tested commit, environment, commands,
five-run median and range, structural maxima, memory, artifact size, and any
metric that remains unavailable. Compare regressions only on equivalent methods
and environments.
