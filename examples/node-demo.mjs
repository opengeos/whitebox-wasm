// Usage from Node (>=20): node examples/node-demo.mjs
// The `web` target needs the wasm bytes passed to init() explicitly in Node.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import init, { geotiff_stats, geotiff_info, version } from "../crates/whitebox-wasm/pkg/whitebox_wasm.js";

const wasmPath = fileURLToPath(new URL("../crates/whitebox-wasm/pkg/whitebox_wasm_bg.wasm", import.meta.url));
await init({ module_or_path: readFileSync(wasmPath) });

const tif = readFileSync(fileURLToPath(new URL("./sample.tif", import.meta.url)));
console.log("version:", version());
console.log("info:   ", geotiff_info(tif));
const stats = JSON.parse(geotiff_stats(tif));
console.log("stats:  ", stats);

const n = 64 * 48, expMin = 3.0, expMax = (n - 1) * 0.25 + 3.0;
const ok = stats.ok && stats.width === 64 && stats.height === 48 && stats.epsg === 32610
  && Math.abs(stats.min - expMin) < 1e-3 && Math.abs(stats.max - expMax) < 1e-3 && stats.valid === n;
console.log(ok ? "PASS" : "FAIL");
process.exit(ok ? 0 : 1);
