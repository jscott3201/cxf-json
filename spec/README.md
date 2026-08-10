# Project specification

`PROFILE.md` is the location for normative behavior after M0. The `_research/`
directory records evidence and candidate decisions but does not define
conforming behavior. A research decision becomes normative only when a reviewed
pull request promotes it into the profile.

ADRs that carry normative decisions live in `adr/`. A pull request that adds or
changes accepted input, observable diagnostics, compatibility boundaries,
ordering, or the public data model must:

1. update `PROFILE.md` and its version;
2. add or supersede an ADR describing the compatibility impact; and
3. add or update tests that enforce the changed rule.

Profile versions use `major.minor.patch`: major for breaking observable changes,
minor for additive behavior, and patch for clarifications that do not change the
compatibility surface. A clarification updates the profile version but needs no
ADR; tests change only when needed to preserve the clarified interpretation.
Formatting, link, and spelling corrections with no semantic effect require only
a reviewed pull request.
