# Project specification

`PROFILE.md` is the location for normative behavior after M0. The local,
Git-ignored `_research/` directory records evidence and candidate decisions but
does not define conforming behavior. A research decision about observable
behavior becomes normative only when a reviewed pull request promotes it into
the profile.

`PROFILE.md` is the sole authority for observable behavior. ADRs in `adr/` record
decisions and rationale; they may establish governance, but an ADR does not make
observable behavior normative unless the same pull request adds it to the
profile.

A pull request that changes any observable contract, including accepted input,
output, diagnostics, compatibility boundaries, ordering, resource limits,
context loading, or the public API and data model, must:

1. update `PROFILE.md` and its version;
2. add or supersede an ADR describing the compatibility impact; and
3. add or update tests that enforce the changed rule.

The initial transition from the no-contract placeholder 0.0.0 to the first
behavior-bearing profile is the reserved 0.1.0 exception. After 0.1.0 and before
1.0, a breaking change increments the minor version, while an additive change or
clarification increments the patch version. Starting at 1.0, profile versions use
`major.minor.patch`: major for breaking changes, minor for additive behavior, and
patch for clarifications that do not change the compatibility surface.

Every profile version change requires one changed ADR with exactly one
machine-readable classification: `Initial`, `Breaking`, `Additive`, or
`Clarification`. Initial, breaking, and additive changes require enforcing tests;
clarification tests change only when needed to preserve the clarified
interpretation. Formatting, link, and spelling corrections with no semantic
effect require only a reviewed pull request and no version change.

CI runs `ci/check-profile-change.py`. A pull request that changes `cxf-json` public
contract files or the delegated OxIRI behavior must also increase `PROFILE.md`,
change profile integration tests, and add or update an ADR that records
compatibility impact. The check validates the version bump against that
classification, requires the first behavior-bearing version to be 0.1.0, and
rejects version regressions. A non-semantic profile edit may keep the same version
when no public contract file changed.
