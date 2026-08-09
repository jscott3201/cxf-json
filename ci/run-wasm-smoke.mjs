import { readFile } from "node:fs/promises";
import { webcrypto } from "node:crypto";

const wasmPath = process.argv[2];
if (!wasmPath) {
  throw new Error("usage: node ci/run-wasm-smoke.mjs <module.wasm>");
}

const module = await WebAssembly.compile(await readFile(wasmPath));
let wasmMemory;
const imports = {
  __wbindgen_placeholder__: {
    __wbindgen_describe() {},
    __wbindgen_object_drop_ref() {},
    __wbg_getRandomValues_a608c4436c19407a(pointer, length) {
      webcrypto.getRandomValues(new Uint8Array(wasmMemory.buffer, pointer, length));
    },
  },
  __wbindgen_externref_xform__: {
    __wbindgen_externref_table_set_null() {},
    __wbindgen_externref_table_grow() {
      return -1;
    },
  },
};

for (const entry of WebAssembly.Module.imports(module)) {
  if (typeof imports[entry.module]?.[entry.name] !== "function") {
    throw new Error(`unsupported WASM import ${entry.module}.${entry.name}`);
  }
}

const instance = await WebAssembly.instantiate(module, imports);
wasmMemory = instance.exports.memory;
if (!(wasmMemory instanceof WebAssembly.Memory)) {
  throw new Error("WASM smoke module does not export memory");
}
if (typeof instance.exports.main !== "function") {
  throw new Error("WASM smoke module does not export main");
}
instance.exports.main();
