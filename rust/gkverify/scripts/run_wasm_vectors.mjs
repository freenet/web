// Run every proof vector written by tests/wasm_vectors.rs through the wasm
// probe, and fail unless each gives the result it was written with.
//
// Usage: node run_wasm_vectors.mjs PROBE.wasm VECTOR_DIR

import { readFileSync } from "node:fs";
import { join } from "node:path";

const [wasmPath, dir] = process.argv.slice(2);
// No import object: the probe must need nothing from the host.
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const { memory, probe_alloc, probe_verify } = instance.exports;

const manifest = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
if (manifest.length === 0) throw new Error("no vectors");
let failed = 0;
for (const { name, expect } of manifest) {
  const input = readFileSync(join(dir, `${name}.bin`));
  const ptr = probe_alloc(input.length);
  new Uint8Array(memory.buffer, ptr, input.length).set(input);
  const got = probe_verify(ptr, input.length); // i64 -> BigInt
  const ok = got === BigInt(expect);
  if (!ok) failed++;
  console.log(`${ok ? "ok  " : "FAIL"} ${name}: got ${got}, expected ${expect}`);
}
if (failed) {
  console.error(`${failed} of ${manifest.length} vectors gave the wrong result under wasm`);
  process.exit(1);
}
console.log(`all ${manifest.length} vectors agree under wasm`);
