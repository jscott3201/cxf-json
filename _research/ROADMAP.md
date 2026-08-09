# Research and implementation roadmap

This roadmap sequences evidence before dependency lock-in. Dates and effort are
unset. Monday owns status and ranking; this file owns acceptance criteria.

## M0: research baseline

| ID | Work item | State | Exit condition |
|---|---|---|---|
| W-001 | Pin upstream CXF behavior and corpus | Done | `UPSTREAM-CXF.md` identifies the source, specification, corpus, known loss, and license concern |
| W-002 | Decide pest's role | Done | `PARSER-STRATEGY.md` records why pest is out of the CXF path and where it may fit later |
| W-024 | Serde/OxJSONLD ingestion boundary | In Progress, M0-C1 | Rust 1.97.1 probe passes `FIRST-SLICE.md`: Serde DTO/syntax evidence, guarded OxJSONLD RDF conversion, owned fixtures, offline behavior, WASM compile, and dependency-health record |
| W-003 | JSON-LD processor qualification | Queued | The dependency-approved processor runs the pinned corpus and required W3C cases on native and WASM; results include conformance, memory, time, binary size, loader control, and dependency graph |
| W-004 | Source-fidelity contract | Queued | Determine the v1 contract for duplicates, number spelling, byte locations, JSON Pointer mapping, and graph linkage without adopting packages that fail D-011 |
| W-005 | License and fixture-use review | Queued | Record whether upstream fixtures may be copied, transformed, or fetched in tests; retain required notices and provenance |

M0 exits when W-024, W-003, W-004, and W-005 are complete; `D-P01` and `D-P02`
are replaced by adopted decisions; fixture provenance is clear; and one
dependency-approved processor passes native plus WASM.

## M1: core ingestion

| ID | Work item | State | Exit condition |
|---|---|---|---|
| W-006 | Workspace and core API scaffold | Planned | Native and WASM builds expose owned input, document, source-location, graph, and diagnostic types without host dependencies |
| W-007 | JSON-LD graph ingestion | Planned | Focused and Buildings fixtures produce RDF terms with full IRI identity, datatypes, language tags, and bounded offline context behavior |
| W-010 | Boundary equivalence corpus | Planned | Rust, Python, browser, and Node agree on bytes, Unicode, numbers, nulls, maps, and error locations |
| W-011 | Resource limits and fuzz harness | Planned | Limits are enforced; fuzz targets find no panic, hang, uncontrolled allocation, or network access |
| W-012 | Blank-node canonical comparison | Planned | Full `CXF-Core.jsonld` equality is stable across processors and serializer-assigned blank-node labels |

M1 exits when malformed input cannot panic the library and the pinned corpus has
a classified result on native Rust and WASM.

## M2: CXF projection and validation

| ID | Work item | State | Exit condition |
|---|---|---|---|
| W-013 | Typed CXF projection | Planned | Known blocks, connectors, values, parameters, instances, connections, arrays, expressions, units, graphics, and FMU references are indexed; unknown triples remain accessible |
| W-014 | Versioned profile validator | Planned | Structural and semantic rules emit stable codes without discarding the graph |
| W-015 | Compatibility profile | Planned | Namespace variants and spec/emitter predicate differences are accepted or rejected only through explicit policy and diagnostics |
| W-016 | Negative conformance corpus | Planned | Duplicate members, malformed contexts, broken references, cardinality, datatype, connection, identifier, array, expression, and limit failures are covered |
| W-017 | Differential report against upstream | Planned | Every difference for the pinned fixture set is explained and versioned |

M2 exits when a caller can distinguish valid JSON-LD, valid RDF, and valid CXF,
and no compatibility alias changes graph identity silently.

## M3: language adapters

| ID | Work item | State | Exit condition |
|---|---|---|---|
| W-008 | PyO3 0.29.2 free-threading spike | Queued | GIL and CPython 3.14t builds pass concurrent tests, detach long work, and produce installable wheels with structured exceptions |
| W-009 | wasm-bindgen/wasm-pack spike | Queued | Browser, bundler, and Node packages install and pass semantic parity, malformed-input, size, and memory gates |
| W-018 | Python public API | Planned | Owned wrappers and bulk serialization have measured copy costs and stable exception fields |
| W-019 | npm public API | Planned | Export map, TypeScript declarations, byte/string behavior, and error contract pass package-consumer tests |

M3 exits when the same pinned corpus and negative cases pass through all four
public environments.

## M4: release qualification

| ID | Work item | State | Exit condition |
|---|---|---|---|
| W-020 | Version and support policy | Planned | Rust is fixed at 1.97.1; Python versions, OS/architectures, browsers, Node, and profile compatibility are documented and tested |
| W-021 | Supply-chain and license inventory | Planned | Cargo, wheel, npm, upstream fixture, generated-artifact, and `MIT OR Apache-2.0` obligations are recorded |
| W-022 | Performance baseline | Planned | Corpus latency, throughput, peak memory, allocation, adapter conversion cost, and WASM size are reproducible |
| W-023 | Release automation | Planned | Rust crates, wheels, and npm artifacts are built from tags, tested after packaging, dependency-policy checked, checksummed, and provenance-attested |
| W-025 | Public-release readiness and visibility change | Planned | OQ-013 stability gate is met, both license texts and notices are verified, `_research/results/W-025-history-audit.md` covers every reachable commit/tag/ref under D-015, and the owner explicitly approves changing the GitHub repository to public |

## Risk register

| ID | Risk | Current response |
|---|---|---|
| R-001 | No authoritative closed CXF schema | Version the Rust profile; preserve extensions; track OBC and upstream commits |
| R-002 | Specification, vocabulary, and emitter disagree | Preserve full IRI identity; diagnose profile aliases; never normalize silently |
| R-003 | Upstream output loses enum order and connection graphics | Expose the loss; do not promise reconstruction |
| R-004 | Remote contexts create SSRF, drift, and availability risk | Offline default; bounded caller-supplied loader only |
| R-005 | JSON-LD processor fails WASM or size targets | Run `W-003` before dependency adoption; split full processing only with explicit capability names |
| R-006 | Python free-threading exposes hidden mutable state | Immutable documents, explicit synchronization, CPython 3.14t concurrency tests |
| R-007 | Host DTO conversion dominates parse time and memory | Benchmark wrappers, structured objects, and JSON bulk transfer |
| R-008 | Fixture licensing is misread from package metadata | Review literal upstream license and required notices before copying corpus |
| R-009 | Blank-node labels make semantic tests flaky | Use dataset canonicalization, not raw labels or JSON order |
| R-010 | Scope expands into Modelica source parsing | Keep `.mo` input outside M0-M4; create a separate decision and frontend if requested |
| R-011 | Rust JSON-LD packages have limited community breadth | Reject packages below D-011; isolate guarded Oxigraph types and recheck health on dependency updates |
