# WASM target and package plan

## Target boundary

The initial WASM target is `wasm32-unknown-unknown` through wasm-bindgen. It has
no operating-system host imports. `std::fs` returns errors and
`std::thread::spawn` panics, so core parsing cannot depend on either.

Source: [Rust `wasm32-unknown-unknown` target](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html).

The first WASM release accepts bytes or strings, uses embedded or caller-supplied
contexts, and performs no network access. Browser and Node loaders can be added
later behind explicit policies.

## Binding contract

Proposed exports, subject to `W-009`:

```text
parseCxf(Uint8Array | string, options?) -> CXF Document DTO
parseCxfJson(Uint8Array | string, options?) -> serialized CXF Document DTO
```

The document DTO includes `validation.accepted` and the diagnostics that
determined it. Strict rejection remains inspectable and does not discard the
CXF document. `parseCxfJson` serializes that same envelope, including validation.

`Uint8Array` is canonical. wasm-bindgen string conversion uses JavaScript
`TextEncoder`/`TextDecoder`; unpaired UTF-16 surrogates become U+FFFD. A string
call therefore cannot promise byte-exact source locations for the original JS
string.

Source: [wasm-bindgen string types](https://wasm-bindgen.github.io/wasm-bindgen/reference/types/str.html).

`serde-wasm-bindgen` is a candidate for structured DTOs. A JSON-string result
may have a smaller and more predictable boundary for large CXF documents.
Measure both rather than assuming native object conversion is faster.

Source: [Serde and `JsValue`](https://wasm-bindgen.github.io/wasm-bindgen/reference/arbitrary-data-with-serde.html).

## Tooling baseline

Observed during research:

- wasm-bindgen 0.2.127, licensed `MIT OR Apache-2.0`; library manifest Rust
  version 1.77 and CLI/support policy Rust 1.86.
- wasm-pack v0.15.0, licensed `MIT OR Apache-2.0`; its manifest has no
  `rust-version`, while the README states Rust 1.30 or newer.

These are evidence snapshots, not selected pins.

Sources:

- [wasm-bindgen 0.2.127 release](https://github.com/wasm-bindgen/wasm-bindgen/releases/tag/0.2.127)
- [wasm-bindgen manifest](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/0.2.127/Cargo.toml)
- [wasm-bindgen MSRV and license policy](https://github.com/wasm-bindgen/wasm-bindgen/blob/main/README.md)
- [wasm-pack v0.15.0 release](https://github.com/wasm-bindgen/wasm-pack/releases/tag/v0.15.0)
- [wasm-pack manifest](https://raw.githubusercontent.com/wasm-bindgen/wasm-pack/v0.15.0/Cargo.toml)

## Package topology

wasm-pack emits different wrappers for `bundler`, `nodejs`, and `web` targets.
One generated directory should not be treated as universal.

The provisional package design is one npm package with controlled exports:

```text
@scope/cxf-parser
  .        browser/bundler entry
  ./node   Node entry
  ./web    direct browser ES module entry
```

Build each target into a separate internal directory and assemble a hand-owned
`package.json` export map. Avoid `experimental-nodejs-module` while the official
deployment documentation marks it experimental.

Sources:

- [wasm-pack build command](https://wasm-bindgen.github.io/wasm-pack/book/commands/build.html)
- [wasm-bindgen deployment targets](https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html)

Package naming and browser/Node version floors remain open.

## JSON-LD dependency risk

The selected processor must compile for `wasm32-unknown-unknown` without pulling
filesystem, native TLS, or unsupported randomness/time behavior. `json-ld`
documents normal dependencies including `futures` and optional `reqwest`; its
WASM fit is not established by its native docs. `oxjsonld` also needs a target
build and size measurement.

Only the JSON-LD behavior required by CXF belongs in the product. Any fallback
for target or size constraints remains a private CXF ingestion detail and must
not be described as a general JSON-LD processor.

## WASM spike gate

`W-009` is a go only if:

- the core and selected processing stack build for `wasm32-unknown-unknown`;
- wasm-pack produces the required `bundler`, `nodejs`, and `web` artifacts;
- Node and browser tests return semantically equivalent CXF documents and
  diagnostics;
- byte, string, large integer, null, Unicode, and error-location boundaries have
  explicit behavior;
- malformed input returns a structured failure without trapping or poisoning
  the instance;
- default parsing performs no network access;
- `npm pack` installs and runs each supported export;
- package size, parse time, and peak memory meet thresholds set before the spike.

Do not rely on WASM panic recovery for user errors. wasm-bindgen documents extra
nightly, standard-library rebuild, unwind, and runtime exception requirements
for catch-unwind support:
[panic handling](https://wasm-bindgen.github.io/wasm-bindgen/reference/catch-unwind.html).
