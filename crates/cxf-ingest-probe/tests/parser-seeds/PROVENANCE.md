# Parser seed provenance

Status: project-authored test data.

These small inputs cover valid JSON, malformed JSON, decoded-name equality,
escapes, nesting, and object width. They were written for this repository and do
not contain external corpus bytes.

`tests/support/parser_seeds.rs` defines the invalid-UTF-8 seed as a byte array so
native and WASM replay use the same reviewed bytes without a binary fixture.
