# Coverage-guided parser checks

This workspace exercises the ordinary JSON preflight without OxJSONLD. It is
excluded from the root Cargo workspace and has its own lockfile and dated nightly
toolchain. The commands require `cargo-fuzz` 0.13.2.

Build and run the bounded local campaign from the repository root:

```sh
mkdir -p target/parser-corpus
RUSTUP_TOOLCHAIN=nightly-2026-07-25 cargo fuzz check --fuzz-dir fuzz json_preflight
RUSTUP_TOOLCHAIN=nightly-2026-07-25 cargo fuzz run --fuzz-dir fuzz json_preflight \
  target/parser-corpus crates/cxf-ingest-probe/tests/parser-seeds -- \
  -max_total_time=30 -max_len=1048576 -timeout=10 \
  -rss_limit_mb=2048 -malloc_limit_mb=512
```

The byte, time, RSS, and single-allocation values bound this test process. They are
not parser defaults or supported-input claims.
