# Benchmarks

Status: Initial corpus and resource-stress baselines measured 2026-08-10. Native
production semantic end-to-end and stage baselines were measured 2026-08-11. A
Linux worker-containment evidence harness is under CI. No public parser or
performance threshold exists.

## Scope

These measurements cover the qualified `cxf-ingest-probe` process and the private
production semantic stage in `cxf-json`. Probe results establish the comparison
format and historical pre-production baseline. Production results cover the
conditional native harness, not a supported package API. They do not prove resource
safety or represent package performance.

The Linux worker-containment report tests one external process boundary around the
private semantic path. It is mechanism evidence, not a cross-platform host API or a
resource-safety result.

The corpus harness reports:

- maximum JSON nesting and members in one object;
- total JSON values and decoded member-name bytes;
- per-file preflight and JSON-LD/RDF stage time, summed for corpus reports;
- emitted quads and retained RDF summary bytes;
- whole-harness wall time and process maximum RSS.

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
| Runs | Five independent process executions per recorded baseline workload |
| Corpus baseline revision | `7a69e58e821eb5ebf36a55dcc67d673ec11cd7a9` |
| Resource-stress revision | `021b8d611fbdc488eeb09181ca9295e83aa6ab27` |
| Production semantic end-to-end baseline revision | `3b56d18c5161f00a5429e2e11f99baec30e72f00` |
| Production semantic stage baseline revision | `4994d73accdd18ec439108e069b565199e30ba6e` |

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

## Corpus Workloads And Structure

Each row is a separate workload with a fixed file set. At corpus baseline revision
`7a69e58`, repository-owned fixtures come from
`crates/cxf-ingest-probe/tests/fixtures`; the current tree stores them under
`crates/cxf-json/tests/fixtures`. OCE CXF fixtures come from
`crates/oce-cxf/tests/fixtures` in the pinned Open Control Engine checkout; the
producer corpus comes from that checkout's
`third_party/modelica-buildings-cdl/cxf` directory.

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

## Corpus Harness Timing

Each timing and memory cell reports the median and minimum-to-maximum range across
five independent process executions. A stage value is the sum of that stage's
per-file timers for one complete workload, not a per-file median. Stage time
includes owned per-file result construction and the `SourceDocument` copy.

Harness wall time starts at `qualify` function entry and ends after the
report is assembled. It includes argument processing, corpus discovery,
canonicalization, two Git verification passes, blob reads, measured stages, and
inter-file aggregation. It excludes final JSON serialization. Process maximum RSS
covers the same full process and is not parser-only memory.

Stage throughput is calculated per run as total input bytes divided by combined
preflight and JSON-LD/RDF time. MB/s uses decimal megabytes; RSS uses binary MiB.
The three rows differ in file count, byte size, and semantic shape. Compare a row
only with the same workload, method, and environment; cross-row wall and stage
times are not parser-speed comparisons.

| Workload | Files | Input bytes | Summed preflight (ms) | Summed JSON-LD/RDF (ms) | Stage throughput (MB/s) | Harness wall (ms) | Process max RSS (MiB) |
|---|---:|---:|---:|---:|---:|---:|---:|
| Repository-owned fixtures | 8 | 5,738 | 0.075 (0.068-0.249) | 0.323 (0.310-0.895) | 14.3 (5.0-15.2) | 89.626 (83.889-99.514) | 7.56 (7.55-7.56) |
| OCE CXF fixtures | 164 | 2,519,687 | 7.875 (7.388-8.357) | 38.730 (38.098-40.628) | 54.2 (51.4-55.4) | 917.663 (873.629-944.014) | 12.14 (11.89-12.33) |
| Pinned Buildings producer corpus | 44 | 2,114,507 | 5.248 (5.034-5.535) | 25.057 (24.148-25.768) | 70.2 (67.5-72.4) | 292.072 (286.993-329.086) | 8.44 (8.31-8.77) |

## Resource-Stress Baseline

Generator version 1 emits 1,133,933 bytes across 16 cases. Times are
microseconds; each timing cell reports the five-run median and range. RDF term
bytes are fixed unless a range is shown. The two expected failures retain source
bytes and diagnostic metadata but do not produce a successful graph.

| Case | Input bytes | Depth | Max members | JSON values | Result | RDF term bytes | Preflight | JSON-LD/RDF |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 262,144-byte string | 262,192 | 1 | 2 | 3 | 1 quad | 262,215 | 273 (164-365) | 262 (174-780) |
| Semantic depth 16 | 754 | 16 | 2 | 32 | 15 quads | 791 | 9 (8-16) | 74 (65-254) |
| Semantic depth 32 | 1,538 | 32 | 2 | 64 | 31 quads | 1,655 | 13 (11-24) | 133 (108-407) |
| Semantic depth 64 | 3,106 | 64 | 2 | 128 | 63 quads | 3,383 | 17 (15-53) | 460 (338-625) |
| 4,096-member object | 118,785 | 1 | 4,096 | 4,097 | 0 quads | 0 | 586 (474-981) | 1,657 (1,464-2,199) |
| 32,768 null values | 163,887 | 2 | 2 | 32,771 | 0 quads | 0 | 142 (138-159) | 2,495 (2,075-2,780) |
| 32,768 retained values | 65,583 | 2 | 2 | 32,771 | 32,768 quads | 2,392,064 | 187 (141-250) | 13,823 (11,885-15,190) |
| 512 names of 256 bytes | 135,169 | 1 | 512 | 513 | 0 quads | 0 | 287 (247-450) | 498 (447-634) |
| 65,536-byte decoded duplicate | 137,744 | - | - | - | JSON failure | - | 239 (223-496) | - |
| 512 context terms | 33,831 | 2 | 514 | 1,027 | 512 quads | 51,712 | 138 (125-263) | 865 (776-1,074) |
| 256 repeated local contexts | 36,377 | 4 | 512 | 1,795 | 256 quads | 20,992 | 203 (189-354) | 8,464 (7,223-9,386) |
| Early `@id`, 128 by 16 | 59,681 | 2 | 17 | 2,305 | 2,048 quads | 172,320 | 304 (260-370) | 1,707 (1,271-2,080) |
| Late `@id`, 128 by 16 | 59,809 | 2 | 17 | 2,305 | 2,048 quads | 172,320 | 267 (254-470) | 1,198 (1,104-1,749) |
| RDF list of 1,024 values | 2,105 | 3 | 2 | 1,028 | 2,049 quads | 237,454 (237,427-237,478) | 7 (6-49) | 828 (689-1,015) |
| 2,048 compact-IRI properties | 28,782 | 3 | 2,050 | 2,054 | 2,048 quads | 208,896 | 219 (189-310) | 1,958 (1,493-2,307) |
| 512 colliding keyword aliases | 24,590 | 2 | 513 | 1,026 | JSON-LD failure | 0 | 147 (132-153) | 506 (405-610) |

The complete suite took 41,549 us (39,347-43,334 us) with process maximum RSS
of 31,997,952 bytes (29,638,656-32,751,616 bytes). RSS covers the whole suite and
cannot be assigned to one case.

The measurements support four narrow conclusions:

- Retaining 32,768 repeated values emits 32,768 repeated quads and 2,392,064 RDF
  term bytes. The larger null-valued control emits no quads.
- Reprocessing a 512-term parent context for 256 local contexts takes roughly ten
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
| Module size | 662,814 bytes | fixed |
| Module SHA-256 | `4777f890d706b1eee9ce5e3719b9114b5ca285ad05228b345cf5f80055c9235d` | fixed |
| Compile | 817 us | 634-946 us |
| Instantiate | 96 us | 74-142 us |
| Execute | 67,112 us | 63,348-76,799 us |
| Initial linear memory | 1,179,648 bytes | fixed |
| Final linear memory | 17,760,256 bytes | fixed |

The runner samples linear-memory capacity before and after execution. WASM memory
cannot shrink, so the final value is also the observed capacity high-water mark.
It is not a count of live bytes and excludes host runtime memory.

## Production Semantic Harness

The `cxf-json` test-support shim exists only under `cfg(fuzzing)` or the
package-scoped `cxf_json_semantic_harness` cfg. It reports project-owned outcome and
metric values without returning source bytes, backend diagnostics, or RDF types.
Normal package and documentation builds contain no callable parse function.

The native and Node/WASM examples run 32,768 retained values through byte
admission, bounded JSON preflight, regular offline OxJSONLD processing, and project
RDF retention. The semantic fuzz target also selects the default, zero-quad, and
zero-term policies. These harness controls do not bound OxJSONLD work inside one
iterator step.

The first native production baseline was built from clean detached revision
`3b56d18c5161f00a5429e2e11f99baec30e72f00`. Five independent process reports
shared this workload identity:

| Input bytes | Input SHA-256 | Retained values | Max depth | Max members | Total values | Member-name bytes | Emitted/returned quads | RDF term bytes |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 131,119 | `1bffb6c3fc7998ed80ada02b562c4b553153b2589d2eed46cdbab203321930a1` | 32,768 | 2 | 2 | 32,771 | 19 | 32,768 | 2,359,296 |

Times are microseconds. Each value reports the median and minimum-to-maximum range
across five runs. Throughput uses decimal megabytes. Maximum RSS covers the whole
process; it cannot attribute resident allocations to the parser or distinguish
live retained memory.

| Elapsed | Throughput | Maximum RSS |
|---:|---:|---:|
| 18,936 (14,467-23,737) | 6.92 MB/s (5.52-9.06) | 26,738,688 bytes (26,574,848-28,426,240) |

The native release executable is 1,110,976 bytes at this revision.

The timer starts after workload generation and ends when the production observation
returns. It includes option construction, byte admission and source copying,
preflight and ordered-tree construction, JSON-LD processing, budget checks, and RDF
retention. The observation copies scalar metrics and drops the private ordered tree
and retained quads before returning, so their teardown is also timed. The timer
excludes workload generation, input hashing, and final report serialization. This
revision does not report separate production stage times.

### Native Stage Baseline

Revision `4994d73accdd18ec439108e069b565199e30ba6e` adds native-only timing around
the existing production split. `preflight_ordered` covers admission, source
copying, UTF-8 and bounded JSON checks, duplicate-name rejection, structure
metrics, and ordered-tree construction. `jsonld_quad_retention` covers
post-preflight document-IRI handling, regular offline JSON-LD processing, budget
checks, retained quads, scalar observation extraction, and private result teardown.

The five reports have the same workload, source, structure, and RDF identity shown
above. Every macOS time sidecar contains one V1 marker matching its JSON report's
run ID, instrumentation revision, workload version, and input SHA-256.

| Metric | Median | Range |
|---|---:|---:|
| Preflight and ordered construction | 1,552 us | 1,415-2,284 us |
| JSON-LD and quad retention | 8,018 us | 7,032-8,323 us |
| Combined stages | 9,543 us | 8,447-10,302 us |
| Stage throughput | 13.74 MB/s | 12.73-15.52 MB/s |
| End-to-end elapsed | 9,671 us | 8,455-10,310 us |
| End-to-end throughput | 13.56 MB/s | 12.72-15.51 MB/s |
| Maximum RSS | 26,771,456 bytes | 26,624,000-27,033,600 bytes |

Combined-stage and stage-throughput distributions sum the two timings within each
run before calculating the median and range. The two component medians therefore
need not add to the combined median.

The native release executable is 1,114,944 bytes at this revision. The two stage
timers add clock reads and change the measurement method, so these values do not
establish a performance change from the `3b56d18` end-to-end baseline. Stage and
end-to-end values remain environment-specific compatibility evidence, not parser
limits or release thresholds.

### Linux Worker-Containment Evidence

`report_native_worker_containment` re-executes one project instrumentation binary as
one child at a time. The child applies a 256 MiB `RLIMIT_AS` before reading input or
entering OxJSONLD. The parent admits at most 1 MiB, accepts at most 4 KiB of stdout,
discards stderr, and kills and reaps the child after a one-second wall-clock
deadline.

The revision-bound CI report verifies the 32,768-value production workload, the
repository remote-context failure, denial of a controlled 512 MiB address-space
reservation, kill/reap after a controlled delay, response overflow, and rejection
of an oversized request before spawn. The overflow case attempts one MiB of output
and deliberately remains alive after the parent observes byte 4,097. The parent kills
and reaps that child, then launches a semantic worker successfully. Worker replies
contain a fixed outcome, source-match boolean, counters, and the configured
address-space cap. They contain no source bytes, RDF values, ordered source tree, or
backend diagnostic text.

`RLIMIT_AS` bounds virtual address space, not RSS. The constants belong to the
evidence harness; they are not parser options, package defaults, or release
thresholds. D-029 remains open because no supported native host boundary, bounded
host-wide worker pool, macOS/Windows mechanism, or browser/Node Worker exists.

## Reproduction

The native commands require macOS `/usr/bin/time -l`; RSS output and process
timing are comparable only on an equivalent environment. Run each baselined
workload five times. Replace `RUN` in the commands below with the run number. Keep
standard output and `/usr/bin/time` output separate so
`ci/summarize-benchmarks.py` can aggregate them. Revision-bound metric builds
reject a revision that differs from `HEAD` or a worktree with tracked or untracked
changes. Corpus runs verify the requested origin, commit, and selected tree paths,
but do not independently reject unrelated dirty files. The production WASM
command remains a smoke run; no five-run production WASM baseline is recorded.

Create detached worktrees for both historical instrumentation revisions and both
production semantic revisions, plus one absolute evidence directory. The commands
below use these variables:

```bash
CXF_JSON="$(git rev-parse --show-toplevel)"
EVIDENCE_DIR="$CXF_JSON/target/benchmark-evidence"
CORPUS_REVISION=7a69e58e821eb5ebf36a55dcc67d673ec11cd7a9
STRESS_REVISION=021b8d611fbdc488eeb09181ca9295e83aa6ab27
SEMANTIC_END_TO_END_REVISION=3b56d18c5161f00a5429e2e11f99baec30e72f00
SEMANTIC_STAGE_REVISION=4994d73accdd18ec439108e069b565199e30ba6e
CORPUS_WORKTREE="/tmp/cxf-json-${CORPUS_REVISION}"
STRESS_WORKTREE="/tmp/cxf-json-${STRESS_REVISION}"
SEMANTIC_END_TO_END_WORKTREE="/tmp/cxf-json-${SEMANTIC_END_TO_END_REVISION}"
SEMANTIC_STAGE_WORKTREE="/tmp/cxf-json-${SEMANTIC_STAGE_REVISION}"
mkdir -p "$EVIDENCE_DIR"
git -C "$CXF_JSON" worktree add --detach "$CORPUS_WORKTREE" "$CORPUS_REVISION"
git -C "$CXF_JSON" worktree add --detach "$STRESS_WORKTREE" "$STRESS_REVISION"
git -C "$CXF_JSON" worktree add --detach \
  "$SEMANTIC_END_TO_END_WORKTREE" "$SEMANTIC_END_TO_END_REVISION"
git -C "$CXF_JSON" worktree add --detach \
  "$SEMANTIC_STAGE_WORKTREE" "$SEMANTIC_STAGE_REVISION"
```

Build the corpus harness from its detached worktree and run the owned corpus:

```bash
(
  cd "$CORPUS_WORKTREE"
  CXF_BENCHMARK_REVISION="$CORPUS_REVISION" \
    cargo +1.97.1 build --release --example qualify_cxf_corpus --locked
  /usr/bin/time -l target/release/examples/qualify_cxf_corpus \
    --git-root "$CORPUS_WORKTREE" \
    --git-origin git@github.com:jscott3201/cxf-json.git \
    --git-commit "$CORPUS_REVISION" \
    --expect-failure "$CORPUS_WORKTREE/crates/cxf-ingest-probe/tests/fixtures/remote-context.jsonld" \
    "No LoadDocumentCallback has been set to load remote contexts" \
    "$CORPUS_WORKTREE/crates/cxf-ingest-probe/tests/fixtures" \
    > "$EVIDENCE_DIR/owned-RUN.json" 2> "$EVIDENCE_DIR/owned-RUN.time"
)
```

Using the revision-bound native binary built above, run the pinned external corpus
from a separately acquired checkout:

```bash
OCE=/path/to/open-control-engine
/usr/bin/time -l "$CORPUS_WORKTREE/target/release/examples/qualify_cxf_corpus" \
  --git-root "$OCE" \
  --git-origin git@github.com:jscott3201/open-control-engine.git \
  --git-commit 8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67 \
  --expect-failure "$OCE/crates/oce-cxf/tests/fixtures/node_scoped_contexts.jsonld" \
  "No LoadDocumentCallback has been set to load remote contexts" \
  "$OCE/crates/oce-cxf/tests/fixtures" \
  > "$EVIDENCE_DIR/oce-RUN.json" 2> "$EVIDENCE_DIR/oce-RUN.time"
```

Run the producer corpus under the same pin:

```bash
OCE=/path/to/open-control-engine
/usr/bin/time -l "$CORPUS_WORKTREE/target/release/examples/qualify_cxf_corpus" \
  --git-root "$OCE" \
  --git-origin git@github.com:jscott3201/open-control-engine.git \
  --git-commit 8fbec096a682b3ff930dcdaa89c6f0a83bf8cd67 \
  "$OCE/third_party/modelica-buildings-cdl/cxf" \
  > "$EVIDENCE_DIR/buildings-RUN.json" 2> "$EVIDENCE_DIR/buildings-RUN.time"
```

Build and measure the generated resource-stress suite:

```bash
(
  cd "$STRESS_WORKTREE"
  CXF_BENCHMARK_REVISION="$STRESS_REVISION" \
    cargo +1.97.1 build --release --locked -p cxf-ingest-probe \
      --example report_resource_stress
  /usr/bin/time -l target/release/examples/report_resource_stress \
    > "$EVIDENCE_DIR/stress-RUN.json" 2> "$EVIDENCE_DIR/stress-RUN.time"
)
```

Build and measure the WASM smoke workload:

```bash
(
  cd "$STRESS_WORKTREE"
  CXF_BENCHMARK_REVISION="$STRESS_REVISION" \
    cargo +1.97.1 build --release --example wasm_cxf_smoke --locked \
      --target wasm32-unknown-unknown
  CXF_BENCHMARK_REVISION="$STRESS_REVISION" node ci/run-wasm-smoke.mjs \
    target/wasm32-unknown-unknown/release/examples/wasm_cxf_smoke.wasm \
    --metrics > "$EVIDENCE_DIR/wasm-RUN.json"
)
```

Aggregate the five reports for each workload:

```bash
python3 "$STRESS_WORKTREE/ci/summarize-benchmarks.py" corpus \
  "$EVIDENCE_DIR"/owned-{1,2,3,4,5}.json \
  --times "$EVIDENCE_DIR"/owned-{1,2,3,4,5}.time
python3 "$STRESS_WORKTREE/ci/summarize-benchmarks.py" corpus \
  "$EVIDENCE_DIR"/oce-{1,2,3,4,5}.json \
  --times "$EVIDENCE_DIR"/oce-{1,2,3,4,5}.time
python3 "$STRESS_WORKTREE/ci/summarize-benchmarks.py" corpus \
  "$EVIDENCE_DIR"/buildings-{1,2,3,4,5}.json \
  --times "$EVIDENCE_DIR"/buildings-{1,2,3,4,5}.time
python3 "$STRESS_WORKTREE/ci/summarize-benchmarks.py" resource-stress \
  "$EVIDENCE_DIR"/stress-{1,2,3,4,5}.json \
  --times "$EVIDENCE_DIR"/stress-{1,2,3,4,5}.time
python3 "$STRESS_WORKTREE/ci/summarize-benchmarks.py" wasm \
  "$EVIDENCE_DIR"/wasm-{1,2,3,4,5}.json
```

Build and measure the first end-to-end production semantic baseline:

```bash
(
  cd "$SEMANTIC_END_TO_END_WORKTREE"
  CXF_JSON_SEMANTIC_HARNESS=1 \
    CXF_BENCHMARK_REVISION="$SEMANTIC_END_TO_END_REVISION" \
    cargo +1.97.1 build --release --locked -p cxf-ingest-probe \
      --no-default-features --features production-semantic-harness \
      --example report_production_semantic
  stat -f %z target/release/examples/report_production_semantic \
    > "$EVIDENCE_DIR/semantic-end-to-end-artifact-size.txt"
  for run in 1 2 3 4 5; do
    LC_ALL=C /usr/bin/time -l target/release/examples/report_production_semantic \
      > "$EVIDENCE_DIR/semantic-end-to-end-${run}.json" \
      2> "$EVIDENCE_DIR/semantic-end-to-end-${run}.time"
  done
  python3 ci/summarize-benchmarks.py semantic-ingestion \
    "$EVIDENCE_DIR"/semantic-end-to-end-{1,2,3,4,5}.json \
    --times "$EVIDENCE_DIR"/semantic-end-to-end-{1,2,3,4,5}.time
)
```

Build and measure the native stage baseline from its recorded revision:

```bash
(
  cd "$SEMANTIC_STAGE_WORKTREE"
  CXF_JSON_SEMANTIC_HARNESS=1 CXF_BENCHMARK_REVISION="$SEMANTIC_STAGE_REVISION" \
    cargo +1.97.1 build --release --locked -p cxf-ingest-probe \
      --no-default-features --features production-semantic-harness \
      --example report_production_semantic
  stat -f %z target/release/examples/report_production_semantic \
    > "$EVIDENCE_DIR/semantic-artifact-size.txt"
  for run in 1 2 3 4 5; do
    LC_ALL=C /usr/bin/time -l target/release/examples/report_production_semantic \
      > "$EVIDENCE_DIR/semantic-${run}.json" \
      2> "$EVIDENCE_DIR/semantic-${run}.time"
  done
  python3 ci/summarize-benchmarks.py semantic-ingestion \
    "$EVIDENCE_DIR"/semantic-{1,2,3,4,5}.json \
    --times "$EVIDENCE_DIR"/semantic-{1,2,3,4,5}.time

  CXF_JSON_SEMANTIC_HARNESS=1 CXF_BENCHMARK_REVISION="$SEMANTIC_STAGE_REVISION" \
    cargo +1.97.1 build --release --locked -p cxf-ingest-probe \
      --no-default-features --features production-semantic-harness \
      --example wasm_production_semantic --target wasm32-unknown-unknown
  CXF_BENCHMARK_REVISION="$SEMANTIC_STAGE_REVISION" node ci/run-wasm-smoke.mjs \
    target/wasm32-unknown-unknown/release/examples/wasm_production_semantic.wasm \
    --metrics
)
```

`fuzz/README.md` records the pinned coverage-guided parser commands and their
test-process bounds.

On a clean committed Linux checkout, reproduce the worker-containment report with:

```bash
REVISION="$(git rev-parse HEAD)"
CXF_JSON_SEMANTIC_HARNESS=1 CXF_BENCHMARK_REVISION="$REVISION" \
  cargo +1.97.1 build --release --locked -p cxf-ingest-probe \
    --no-default-features --features production-semantic-harness \
    --example report_native_worker_containment
target/release/examples/report_native_worker_containment
```

## Gaps And Update Rules

- Allocation counts are not measured. macOS `/usr/bin/time -l` reports process
  maximum RSS, which includes allocator and runtime state rather than parser-only
  retained memory.
- The generated suite samples selected structural and graph growth patterns. It
  does not establish safe hard caps or cover every JSON-LD expansion shape.
- Process RSS is measured for the full resource-stress suite, not per case.
- In-process OxJSONLD has no project-controlled hard timeout or allocation budget.
  The Linux evidence harness tests an external deadline and address-space cap, but
  supported native and browser/Node worker boundaries remain absent.
- M1-C6 uses this evidence for private emitted-quad and retained-term policy. The
  limits do not bound backend allocation or diagnostic amplification.
- Native production semantic time reports carry a V1 identity marker and are also
  paired with JSON reports by filename stem. Corpus and resource-stress time
  reports retain their historical stem-only method.
- Production stage timing is native-only. The Node/WASM smoke remains an external
  whole-module measurement. Private graph indexing and semantic joins do not exist
  and therefore have no timing baseline.

Update this file whenever a parser stage, corpus pin, dependency version, target,
or benchmark method changes. Record the tested commit, environment, commands,
five-run median and range, structural maxima, memory, artifact size, and any
metric that remains unavailable. Compare regressions only on equivalent methods
and environments.
