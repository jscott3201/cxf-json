# Python and PyO3 0.29.2

## Binding shape

The Python extension wraps owned core values. Parsing accepts `bytes`, a
read-only buffer, or `str`; bytes are canonical. Long parsing and validation
detach from Python so other threads can run.

Proposed surface, subject to `W-008`:

```python
document = cxf_parser.parse_cxf(data, options=None)
document.validation.accepted
document.validation.diagnostics
document.cxf
document.extensions
```

Failures that prevent CXF document construction raise `CxfParseError`. The
exception exposes the stable diagnostic code, stage, byte range, JSON Pointer,
related locations, and profile. Rendering remains a convenience string.

## Free-threading rules

PyO3 0.29.2 supports free-threaded CPython starting with Python 3.14. The
binding should assume modules are thread-safe and prove that claim rather than
setting `gil_used = true` to avoid the work.

Rules for the adapter:

- `#[pyclass]` values must satisfy `Send + Sync` unless there is a reviewed
  reason otherwise.
- Do not use `#[pyclass(unsendable)]`; cross-thread access may panic and Python
  garbage collection may leak such values.
- Parsed documents are immutable after construction.
- Mutable wrappers use a `Mutex`, `RwLock`, or atomics. PyO3 runtime borrowing
  is not a replacement for synchronization.
- No Rust lock is held while attaching to Python or calling Python code.
- Long parse, expansion, validation, and serialization work uses
  `Python::detach`.
- Python-aware one-time initialization uses PyO3 synchronization helpers such
  as `PyOnceLock` rather than an unchecked process-global Python object.
- Concurrent tests run on GIL-enabled CPython and free-threaded CPython.

Evidence:

- [PyO3 free-threading guide](https://pyo3.rs/v0.29.2/free-threading.html)
- [PyO3 class thread safety](https://pyo3.rs/v0.29.2/class/thread-safety.html)
- [PyO3 parallelism](https://pyo3.rs/v0.29.2/parallelism.html)

## Packaging

PyO3 0.29.2 is licensed `MIT OR Apache-2.0` and its workspace manifest declares
Rust 1.83. Maturin is the leading package tool because it supports PEP 621 and
PyO3 wheel production.

The wheel plan must account for free-threaded ABI differences:

- ordinary `abi3` wheels do not load on free-threaded CPython;
- CPython 3.14t needs a version-specific free-threaded wheel;
- `abi3t` begins with Python 3.15 and supports GIL and free-threaded builds;
- the supported Python floor determines whether normal wheels are per-version
  or use `abi3`.

The observed Maturin v1.14.1 release supports `abi3t`, is licensed `MIT OR
Apache-2.0`, and declares Rust 1.89. This makes the packaging toolchain's Rust
version newer than the candidate library MSRV; CI may use a newer packaging
toolchain without raising the library MSRV.

Sources:

- [PyO3 0.29.2 manifest](https://raw.githubusercontent.com/PyO3/pyo3/v0.29.2/Cargo.toml)
- [PyO3 distribution guide](https://pyo3.rs/v0.29.2/building-and-distribution.html)
- [Maturin metadata](https://www.maturin.rs/metadata.html)
- [Maturin distribution](https://www.maturin.rs/distribution.html)
- [Maturin v1.14.1](https://github.com/PyO3/maturin/releases/tag/v1.14.1)

## Copy and ownership policy

Input buffers may be borrowed only for the duration of the Rust call. The
returned document owns all values. This avoids pinning Python memory and makes
detached execution safe.

Returning large CXF documents as nested Python dictionaries can dominate parser
time and memory. The spike should compare:

- immutable wrapper objects backed by shared Rust data;
- materialized Python dictionaries/lists;
- normalized CXF JSON text for bulk interchange.

The first option gives selective access without copying the whole document. The
third gives a stable bulk boundary. The public API may support both after
measurement.

## Python spike gate

`W-008` is a go only if:

- PyO3 0.29.2 builds at the declared library MSRV;
- a clean Maturin build installs and imports on supported platforms;
- GIL-enabled and CPython 3.14t tests return equivalent documents;
- concurrent parsing and document access show no deadlock, data race, panic, or
  unexplained PyO3 borrow failure;
- long operations release Python execution through `Python::detach`;
- malformed input produces structured exceptions;
- version-specific free-threaded wheels are produced when `abi3t` is not
  available for that interpreter.

Python versions and wheel platforms remain open in `OQ-008`.
