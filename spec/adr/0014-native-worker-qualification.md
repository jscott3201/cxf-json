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

The first qualifying implementation uses one worker process per admitted request. A
controller follows this order:

1. reject a request that exceeds the host request cap or the caller's input policy;
2. reserve bounded capacity and start the request's monotonic deadline;
3. retain the admitted source in the controller;
4. create the worker and install target containment before releasing it to read input;
5. send one bounded request and collect one byte-bounded response;
6. validate response identity, length, and project-owned fields;
7. exit and reap the worker; and
8. settle exactly one terminal outcome.

Deadline, cancellation, response overflow, and protocol failure terminate the worker
or its contained process group. A crash enters the same reap path without another
termination attempt. The controller must prove process exit and reap before returning.
A worker forced to terminate is never reused. If termination or reaping cannot be
established within a separately bounded cleanup interval, the controller becomes
unavailable rather than accepting more work.

The deadline starts when the controller accepts the request, so queueing, process
startup, containment setup, transfer, processing, and response collection consume the
same budget. The controller marks the request terminal before terminating a timed-out
worker and discards late output. A captured response overflow takes precedence over a
racing deadline.

### Ownership and outcomes

The controller retains exact admitted source bytes until the terminal outcome. The
worker response never repeats source bytes. It carries a project-owned request
identity and source digest so the controller can reject a stale or mismatched response
before associating it with retained source. The encoding and digest algorithm remain
private until the worker protocol is promoted.

The logical outcomes are:

- source-free admission rejection;
- constructed parse failure associated with admitted source and project-owned
  diagnostics;
- successful typed result associated with admitted source and bounded non-fatal
  validation findings; and
- host failure, separate from parse diagnostics.

Host failures cover containment setup, capacity, deadline, cancellation, crash,
worker-side request framing, response bounds, protocol validation, termination,
reaping, and transport I/O. No host failure returns a partial typed result. Worker
payloads contain no backend error text, stderr, host handles, Serde framework types,
backend values, public RDF graph, or claimed source-to-RDF correspondence.

The exact worker encoding and future public result types remain private until W-013
and W-014 stabilize the typed result and its count and payload budgets. Each transport
must enforce response bytes before deserialization and independently enforce result
bounds while constructing the response.

### Resource policy

Host containment policy remains separate from `ParseOptions`. A controller has an
immutable, finite worker count, queue capacity, request cap, response cap, operation
deadline, cleanup interval, and target memory limit. A full queue fails with a fixed
capacity outcome; it never grows without a bound.

Each target must name the memory quantity it enforces and test the implementation that
ships:

| Target | Required qualification |
|---|---|
| Linux | Install a hard process or containment-group memory limit before request reading; name whether it limits virtual address space, committed memory, or another exact metric; terminate and reap the worker on every forced-failure path |
| macOS | Install and test the selected hard process limit before request reading on every supported release; do not infer safety from RSS benchmark observations; terminate and reap the worker on every forced-failure path |
| Windows | Assign the worker to a configured Job Object before execution, apply process/job committed-memory and active-process controls, prevent breakaway, terminate the job on forced failure, and wait for process completion |

No target may fall back to in-process parsing when its containment mechanism is
unavailable. It returns a fixed containment-unavailable host failure instead. Native
bindings must use the same qualified controller rather than reimplementing containment
in the binding runtime.

Production numbers will be selected from measurements, not copied from the Linux
evidence harness. Selection must cover maximum admitted input, adversarial structure,
diagnostic and extension amplification, full typed responses, parent and worker memory,
process startup, cleanup, repeated requests, and aggregate concurrency on every
supported target. Per-worker memory and concurrency must fit one documented aggregate
host budget.

### Qualification evidence

A target suite must execute the packaged controller and worker, not a mock. It must
verify:

- containment setup precedes request reading and backend entry;
- exact request, response, semantic-output, typed-result, and finding boundaries once
  the typed result stabilizes;
- deadline and cancellation termination followed by bounded reap;
- memory enforcement against the target's named metric;
- crash, malformed protocol, stale response, and response-overflow handling;
- overflow and deadline precedence;
- bounded concurrency, bounded queueing, and aggregate resource accounting;
- fresh-worker success after deadline, crash, and overflow;
- exact source association without source bytes in the worker response;
- no backend diagnostics, stderr, RDF values, or partial result leakage; and
- no in-process fallback when containment setup fails.

## Consequences

- The Linux evidence harness remains mechanism evidence. It does not qualify Linux or
  set product defaults.
- macOS and Windows require real target implementations and CI evidence before native
  hostile-input support can ship.
- Browser and Node Worker qualification remains separate under D-029; this ADR makes no
  browser or Node memory claim.
- D-029 stays open. The package remains unpublished and exposes no parser, worker,
  protocol, or containment API.
- Profile 0.1.8 does not change because this governance decision defines no observable
  parser behavior.
