# ADR 0014: Native worker qualification

Status: Accepted

Date: 2026-08-14 UTC

## Context

D-029 blocks a public untrusted-input release until backend work runs behind a
killable, memory-contained boundary with bounded time, concurrency, requests, and
responses. The repository has Linux mechanism evidence, but its 256 MiB address-space
limit, one-second deadline, 1 MiB request cap, 4 KiB scalar response cap, and
single-worker execution are test values rather than product policy.

There is no supported parser, public typed CXF result, host serialization contract,
or native worker controller. The private composed result and validation findings are
also not fully bounded for a public response. Promoting the evidence protocol or its
numbers would therefore stabilize the wrong result shape before amplification and
cross-target behavior have been measured.

CXF is the JSON-LD representation of configured Control Description Language (CDL)
logic. Native containment governs where and under what resource policy the reader
runs; it does not own CDL-to-CXF translation or CXF meaning. The current public
technical references are the OBC [CDL](https://obc.lbl.gov/specification/cdl.html) and
[CXF](https://obc.lbl.gov/specification/cxf.html) sections. The ASHRAE
[Standard 231 resource page](https://data.ashrae.org/standard231/) identifies the
current standard resources and elementary-block basis. Clause-level conformance claims
require a separate review of the licensed standard.

## Decision

The first native containment boundary will be qualified against one project-owned
contract for Linux, macOS, and Windows. Qualification is target-specific: evidence
from one operating system establishes no claim for another. W-020 must name supported
operating-system versions and architectures, and every advertised native target must
pass its own qualification suite before package release.

This ADR defines qualification criteria, not a parser API, host API, wire encoding, or
production resource values. A target becomes supported only when a later profile
change defines observable behavior and tests a packaged implementation against these
criteria.

### Lifecycle

The baseline qualification uses one worker process per admitted request. This is not a
public API constraint. Reusing a worker after ordinary success requires a later
decision and state-reset, leakage, memory-fragmentation, and parity evidence; a worker
forced to terminate is never reusable. A controller follows this order:

1. reject a request that exceeds the host request cap or the caller's input policy;
2. reserve host-wide count and byte capacity and start the request's monotonic
   deadline;
3. retain the admitted source in the controller;
4. create the worker and install target containment before releasing it to read input;
5. send one bounded request and collect one byte-bounded response;
6. validate response identity, length, and project-owned fields;
7. exit and reap the worker; and
8. settle exactly one terminal outcome.

Deadline, cancellation, response overflow, and protocol failure terminate the worker
or its contained process group. A crash enters the same reap path without another
termination attempt. The controller must prove process exit and reap before returning.

A queued request whose cancellation or deadline is observed before spawn settles
without creating a worker and releases its source and resource reservation. Dequeue
must atomically claim only a live request. If cancellation or deadline races a claimed
spawn, the new worker enters termination and reap without receiving a successful
outcome. When a controller becomes unavailable, it rejects new work and settles every
queued request with a fixed host failure before releasing its reservation.

The operation deadline is an execution cutoff. A separate cleanup deadline starts
when termination begins. The controller does not settle the request until reap
succeeds. If cleanup expires without reap, no ordinary library result is allowed: the
containment owner must fail-stop its process so the target's parent-death or job-close
mechanism removes the worker. A target may avoid terminating the caller only by adding
an outer supervisor that is itself terminated and reaped before the caller returns a
fixed containment-lost failure. The supported package must document which fail-stop
boundary it uses.

Controller death must not orphan a worker. Each target needs an independent
parent-death or job-close mechanism installed before backend entry. Qualification must
kill the controller during startup, execution, and response handling and prove that no
worker or descendant remains.

The deadline starts when the controller accepts the request, so queueing, process
startup, containment setup, transfer, processing, response collection, and response
validation consume the same budget. When response overflow, cancellation, or deadline
closes the success path, the controller terminates the worker, joins the bounded
reader, and then chooses the final host failure. If the final bounded reader state
contains byte `response_cap + 1`, response overflow takes precedence. Otherwise a
cancellation requested before the absolute deadline wins; deadline wins when no valid
response completed by the cutoff; abnormal exit wins next; and a bounded response from
a successful worker that fails validation is a protocol failure. Setup and transport
failures observed before those events keep their own category. Late valid output never
reopens the success path.

### Ownership and outcomes

The controller retains exact admitted source bytes until the terminal outcome. The
worker response never repeats source bytes. It carries a project-owned request
identity and source digest so the controller can reject a stale or mismatched response
before associating it with retained source. The digest must be collision-resistant;
its encoding and algorithm remain private until the worker protocol is promoted.

The controller sends the worker the exact admitted octets. No containment transport
may repair UTF-8, convert newlines, parse and re-emit JSON, expand contexts, normalize
IRIs, or otherwise rewrite the request. The source digest identifies only those exact
octets for this request. It is not a CXF identifier, semantic hash, RDF canonical form,
or equivalence key.

Containment does not flatten arrays, evaluate expressions, propagate parameters,
resolve paths or remote contexts, execute extension blocks, infer missing CDL
declarations, merge distinct IRIs, or join source nodes to RDF output. Those choices
remain with the governing CXF/CDL profile and the source-derived projection. Host
resource outcomes cannot redefine whether a document is valid CXF or equivalent to
another document.

Before dispatch, the controller and worker must agree on a project-owned semantic
contract identifier covering the profile version, typed-result schema, JSON-LD mode,
and pinned authority revisions. A mismatch is a protocol host failure. The identifier
allows the private transport to change without treating two semantic contracts as the
same protocol.

The logical outcomes are:

- source-free admission rejection;
- constructed parse failure associated with admitted source and project-owned
  diagnostics;
- source-associated project limit failure with no partial result;
- successful typed result associated with admitted source and bounded non-fatal
  validation findings; and
- host failure, separate from parse diagnostics.

Source/JSON/JSON-LD failures, project limit failures, project profile findings, and
host failures are separate namespaces and cannot be translated into one another. A
successful typed result is complete. Typed-result, extension, diagnostic, or finding
overflow returns a distinct project limit or host limit outcome with no partial result
and no valid or conformant indication.

Host failures cover containment setup, capacity, deadline, cancellation, crash,
worker-side request framing, response bounds, protocol validation, termination,
reaping, and transport I/O. No host failure returns a partial typed result. Worker
payloads contain no backend error text, stderr, host handles, Serde framework types,
backend values, public RDF graph, or claimed source-to-RDF correspondence.

The exact worker encoding and future public result types remain private until W-013
and W-014 stabilize the typed result and its count and payload budgets. Each transport
must enforce response bytes before deserialization. Its decoder must use a closed
schema and explicit nesting, field-count, string-length, and allocation bounds; a byte
cap alone is not a decoder budget. The worker independently enforces result bounds
while constructing the response.

### Resource policy

Host containment policy remains separate from `ParseOptions`. Every controller and
native binding in one application process draws from one host-wide budget. The
implementation may use a shared coordinator or another mechanism, but constructing a
new API object cannot create another budget. The budget has an immutable, finite
worker count, queue count, queue byte limit, request cap, response cap, operation
deadline, cleanup deadline, and target memory limit. Deployments with multiple
application processes need an operator-owned aggregate limit outside the package.

Admission reserves the retained source, parent and transport copies, maximum response,
worker memory allowance, and fixed controller overhead before queueing. The
reservation remains charged through response validation and reap. Every settled path
releases it exactly once; fail-stop releases all reservations with process teardown. A
full count or byte budget fails with a fixed capacity outcome; neither budget grows
without a bound.

Each target must name the memory quantity it enforces and test the implementation that
ships:

| Target | Required qualification |
|---|---|
| Linux | Install a hard containment-group or process memory limit before request reading; name its exact metric; prevent descendant escape or contain the full process tree; install parent-death termination with its startup race closed; terminate and reap on every forced-failure path |
| macOS | Install and test the selected hard process limit before request reading on every supported release; prevent or contain descendants; install a tested parent-liveness termination mechanism; do not infer safety from RSS benchmark observations; terminate and reap on every forced-failure path |
| Windows | Assign the worker to a configured Job Object before execution; apply process/job committed-memory and active-process controls; prohibit breakaway; set `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; keep the job handle non-inheritable and inaccessible to the worker; terminate the job on forced failure; wait for process completion and close process, thread, and job handles |

No target may fall back to in-process parsing after containment setup, spawn,
handshake, transfer, protocol, crash, deadline, cleanup, or capacity failure. It
returns the applicable fixed host failure instead. Native bindings must use the same
qualified coordinator rather than reimplementing containment in the binding runtime.

Production numbers will be selected from measurements, not copied from the Linux
evidence harness. Selection must cover maximum admitted input, adversarial structure,
diagnostic and extension amplification, full typed responses, parent and worker memory,
process startup, cleanup and fail-stop timing, repeated requests, and aggregate
concurrency on every supported target. Per-worker memory and concurrency must fit one
documented aggregate host budget.

The evidence for each selected value must be reproducible and revision-bound. It must
record target and architecture, operating-system and toolchain versions, workload
identity, commands, observed range, chosen margin, and the derivation from measured
values to the product limit. CI must reject a limit change that lacks updated evidence.

### Qualification evidence

A target suite must execute the packaged controller and worker, not a mock. It must
verify:

- containment setup precedes request reading and backend entry;
- exact request, response, semantic-output, typed-result, and finding boundaries once
  the typed result stabilizes;
- queued deadline and cancellation without spawn, plus running deadline and
  cancellation followed by termination and reap;
- cleanup expiry through the packaged fail-stop or outer-supervisor path without an
  ordinary result or surviving worker;
- memory enforcement against the target's named metric;
- controller death without an orphaned worker or descendant;
- attempted descendant creation without escape from memory and termination policy;
- crash, malformed or structurally excessive protocol, stale response, and
  response-overflow handling;
- overflow and deadline precedence;
- shared count and byte budgets across multiple controller and binding instances;
- exact-once reservation release for every settled path;
- fresh-worker success after deadline, crash, and overflow;
- distinct process identity, successful reap, closed handles, and no zombie or handle
  growth across repeated ordinary successes;
- exact source association without source bytes in the worker response;
- identical project-owned typed projection and finding evidence across Linux, macOS,
  and Windows for well-resourced profile fixtures, excluding backend-assigned blank-node
  labels and allowing variation only in named host failures;
- no backend diagnostics, stderr, RDF values, or partial result leakage; and
- no in-process fallback after any host failure.

## Consequences

- The Linux evidence harness remains mechanism evidence. It does not qualify Linux or
  set product defaults.
- macOS and Windows require real target implementations and CI evidence before native
  hostile-input support can ship.
- Browser and Node Worker qualification remains separate under D-029; this ADR makes no
  browser or Node memory claim.
- The evidence harness protocol and private result layouts have no compatibility
  standing. A supported design may replace them without a compatibility shim or legacy
  decode path.
- Worker qualification cannot establish CXF or CDL conformance. Before a public
  conformance claim, the profile must pin the Standard 231 edition and reviewed clauses,
  the Modelica Buildings basis, and the exact CXF-Core artifact revision, then resolve
  any disagreement as an explicit profile decision rather than transport behavior.
- D-029 stays open. The package remains unpublished and exposes no parser, worker,
  protocol, or containment API.
- Profile 0.1.8 does not change because this governance decision defines no observable
  parser behavior.
