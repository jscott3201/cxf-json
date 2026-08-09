# Dependency governance baseline

Evidence snapshot: 2026-08-09. Repository popularity counts change over time;
the values below are the observations used for W-024 planning.

## Owner policy and project threshold

The owner requires widely supported, community-based packages and rejects new,
low-adoption, or apparently unmaintained packages.

The owner approved a 1,000-star parent-repository floor on 2026-08-09. Lowering
it requires an explicit owner exception; raising it does not.

A star count cannot clear a dependency by itself. D-011 also requires project
age, recent maintenance, contributor evidence, compatible license and MSRV,
supported target builds, and a locked advisory review. Monorepo crates inherit
the parent repository's community evidence but retain crate-specific version,
license, MSRV, feature, and advisory checks.

## Candidate record

| Crate | Version | Parent repository | Created | Stars | Forks | Last push | Archived | MSRV | Result |
|---|---:|---|---|---:|---:|---|---|---:|---|
| `serde` | 1.0.229 | `serde-rs/serde` | 2013-11-13 | 10,759 | 929 | 2026-07-25 | No | 1.56 | Qualified for private development |
| `serde_json` | 1.0.151 | `serde-rs/json` | 2015-05-19 | 5,614 | 661 | 2026-08-08 | No | 1.71 | Qualified for private development |
| `oxjsonld` | 0.2.5 | `oxigraph/oxigraph` | 2018-05-16 | 1,805 | 162 | 2026-08-09 | No | 1.87 | Private development under D-022 |
| `oxrdf` | 0.3.3 | `oxigraph/oxigraph` | 2018-05-16 | 1,805 | 162 | 2026-08-09 | No | 1.87 | Private development under D-022 |
| `json-ld` | 0.21.4 | `timothee-haudebourg/json-ld` | 2020-04-15 | 153 | 25 | 2026-07-02 | No | 1.83 | Reject for production |
| `json-syntax` | 0.12.5 | `timothee-haudebourg/json-syntax` | 2022-07-01 | 6 | 5 | 2026-06-21 | No | 1.71 | Reject |
| pest | 2.8.8 | `pest-parser/pest` | 2016-04-24 | 5,375 | 301 | 2026-08-09 | No | 1.83 | Healthy, wrong layer |
| `getrandom` | 0.3.4 | `rust-random/getrandom` | 2019-01-19 | 571 | 254 | 2026-07-27 | No | 1.63 | D-016/D-021 target-only exception |

Repository evidence:

- [Serde GitHub API](https://api.github.com/repos/serde-rs/serde)
- [serde_json GitHub API](https://api.github.com/repos/serde-rs/json)
- [Oxigraph GitHub API](https://api.github.com/repos/oxigraph/oxigraph)
- [json-ld GitHub API](https://api.github.com/repos/timothee-haudebourg/json-ld)
- [json-syntax GitHub API](https://api.github.com/repos/timothee-haudebourg/json-syntax)
- [pest GitHub API](https://api.github.com/repos/pest-parser/pest)
- [getrandom GitHub API](https://api.github.com/repos/rust-random/getrandom)

Version metadata:

- [serde 1.0.229](https://crates.io/api/v1/crates/serde/1.0.229)
- [serde_json 1.0.151](https://crates.io/api/v1/crates/serde_json/1.0.151)
- [oxjsonld 0.2.5](https://crates.io/api/v1/crates/oxjsonld/0.2.5)
- [oxrdf 0.3.3](https://crates.io/api/v1/crates/oxrdf/0.3.3)
- [json-ld 0.21.4](https://crates.io/api/v1/crates/json-ld/0.21.4)
- [json-syntax 0.12.5](https://crates.io/api/v1/crates/json-syntax/0.12.5)
- [getrandom 0.3.4](https://crates.io/api/v1/crates/getrandom/0.3.4)

The API links are live sources. This file preserves the values observed from
them on the snapshot date; adoption must refresh them and record the locked
dependency graph.

## Concentration and advisory findings

Oxigraph has an organization-owned, active repository with substantial stars,
forks, history, and multiple historical contributors. Its commit history is
nevertheless concentrated around one lead maintainer. W-024 may evaluate
`oxjsonld` and `oxrdf`, but production adoption requires the slice's target,
license, feature, and locked-graph evidence. If adopted, Oxigraph types remain
behind an internal adapter and its health is rechecked on dependency updates.

`json-syntax` 0.12.5 directly depends on `smallstr` 0.3. RustSec advisory
[RUSTSEC-2026-0215](https://rustsec.org/advisories/RUSTSEC-2026-0215.html)
marks all `smallstr` versions unmaintained with no patched release. The six-star
parent already fails the community floor; the transitive advisory is a second,
independent rejection reason.

`json-ld` is active and technically relevant, but its 153-star individually
owned repository and concentrated contribution history fail the owner policy.
It remains research evidence only.

## Adoption outcomes

- `serde` and `serde_json`: qualified by D-018 for private-development use at
  ordinary JSON and owned DTO boundaries after W-024 passed lockfile, target CI,
  local policy, license, feature, and advisory gates. Production-release adoption
  remains gated by W-023's release-policy integration.
- `oxjsonld` and `oxrdf`: qualified by D-022 for private CXF ingestion behind the
  internal adapter. They are not approved for production release; W-023 retains
  that gate. Contributor concentration remains R-011.
- `json-ld`, `json-syntax`, and `sophia_jsonld`: must not enter the production
  dependency graph without an explicit, expiring owner exception.
- Full `oxigraph`: out of scope; the database and RocksDB dependency surface are
  not needed for JSON-LD-to-RDF conversion.
- pest: healthy community evidence does not change D-002; it solves the wrong
  parsing layer for CXF JSON-LD.
- `getrandom` 0.3.4: D-016 permits only the target-specific `wasm_js` feature
  needed to compile OxRDF blank-node generation for `wasm32-unknown-unknown`.
  D-021 renews the exception through W-009 completion or the first public
  release, whichever comes first. The locked-metadata D-021 check prevents the
  reusable release-policy workflow from passing while the resolved direct
  dependency or its marker remains.
