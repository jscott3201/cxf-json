import { readFile } from "node:fs/promises";
import { createHash, randomUUID, webcrypto } from "node:crypto";
import { execFileSync } from "node:child_process";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

const wasmPath = process.argv[2];
const reportMetrics = process.argv.includes("--metrics");
const instrumentationRevision = process.env.CXF_BENCHMARK_REVISION;
if (!wasmPath) {
  throw new Error("usage: node ci/run-wasm-smoke.mjs <module.wasm> [--metrics]");
}
if (reportMetrics && !/^[0-9a-f]{40}$/.test(instrumentationRevision ?? "")) {
  throw new Error("--metrics requires CXF_BENCHMARK_REVISION as a 40-digit commit ID");
}
if (reportMetrics) {
  const repository = fileURLToPath(new URL("../", import.meta.url));
  const gitEnv = { ...process.env };
  for (const name of [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_NAMESPACE",
    "GIT_GRAFT_FILE",
    "GIT_REPLACE_REF_BASE",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_SYSTEM",
    "GIT_CONFIG_GLOBAL",
  ]) {
    delete gitEnv[name];
  }
  const nullConfig = process.platform === "win32" ? "NUL" : "/dev/null";
  Object.assign(gitEnv, {
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: nullConfig,
    GIT_NO_REPLACE_OBJECTS: "1",
    GIT_NO_LAZY_FETCH: "1",
    GIT_OPTIONAL_LOCKS: "0",
  });
  const git = (...args) =>
    execFileSync(
      "git",
      [
        "-c",
        `core.hooksPath=${nullConfig}`,
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.untrackedCache=false",
        "-c",
        "core.preloadIndex=false",
        "-C",
        repository,
        ...args,
      ],
      {
        encoding: "utf8",
        env: gitEnv,
      },
    );
  const head = git("rev-parse", "HEAD").trim();
  if (head !== instrumentationRevision) {
    throw new Error(
      `instrumentation revision mismatch: expected ${instrumentationRevision}, got ${head}`,
    );
  }
  if (git("status", "--porcelain=v1", "--untracked-files=normal") !== "") {
    throw new Error("WASM measurements require a clean worktree");
  }
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
if (reportMetrics) {
  const embeddedRevision = Array.from({ length: 5 }, (_, index) => {
    const readWord = instance.exports[`cxf_benchmark_revision_${index}`];
    if (typeof readWord !== "function") {
      throw new Error("WASM smoke module does not export its benchmark revision");
    }
    return (readWord() >>> 0).toString(16).padStart(8, "0");
  }).join("");
  if (embeddedRevision !== instrumentationRevision) {
    throw new Error(
      `WASM benchmark revision mismatch: expected ${instrumentationRevision}, got ${embeddedRevision}`,
    );
  }
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
