import { readFile } from "node:fs/promises";
import { createHash, randomUUID, webcrypto } from "node:crypto";
import { performance } from "node:perf_hooks";

const wasmPath = process.argv[2];
const reportMetrics = process.argv.includes("--metrics");
const instrumentationRevision = process.env.CXF_BENCHMARK_REVISION;
if (!wasmPath) {
  throw new Error("usage: node ci/run-wasm-smoke.mjs <module.wasm> [--metrics]");
}
if (reportMetrics && !/^[0-9a-f]{40}$/.test(instrumentationRevision ?? "")) {
  throw new Error("--metrics requires CXF_BENCHMARK_REVISION as a 40-digit commit ID");
}

const bytes = await readFile(wasmPath);
const compileStarted = performance.now();
const module = await WebAssembly.compile(bytes);
const compileMicros = Math.round((performance.now() - compileStarted) * 1000);
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

const instantiateStarted = performance.now();
const instance = await WebAssembly.instantiate(module, imports);
const instantiateMicros = Math.round((performance.now() - instantiateStarted) * 1000);
wasmMemory = instance.exports.memory;
if (!(wasmMemory instanceof WebAssembly.Memory)) {
  throw new Error("WASM smoke module does not export memory");
}
if (typeof instance.exports.main !== "function") {
  throw new Error("WASM smoke module does not export main");
}
const initialMemoryBytes = wasmMemory.buffer.byteLength;
const executeStarted = performance.now();
instance.exports.main();
const executeMicros = Math.round((performance.now() - executeStarted) * 1000);

if (reportMetrics) {
  console.log(
    JSON.stringify(
      {
        run_id: randomUUID(),
        instrumentation_revision: instrumentationRevision,
        node: process.version,
        platform: process.platform,
        architecture: process.arch,
        module_bytes: bytes.byteLength,
        module_sha256: createHash("sha256").update(bytes).digest("hex"),
        compile_micros: compileMicros,
        instantiate_micros: instantiateMicros,
        execute_micros: executeMicros,
        initial_memory_bytes: initialMemoryBytes,
        final_memory_bytes: wasmMemory.buffer.byteLength,
      },
      null,
      2,
    ),
  );
}
