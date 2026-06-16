// Read a GeoTIFF / COG straight from an HTTP URL.
//
// The wasm module is sandboxed and does no networking itself; the fetch happens
// here in JS and the bytes are handed to wasm. Run with: node examples/read-from-url.mjs
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import init, { geotiff_info, GeoTiffReader } from "../crates/whitebox-wasm/pkg/whitebox_wasm.js";

const wasmPath = fileURLToPath(new URL("../crates/whitebox-wasm/pkg/whitebox_wasm_bg.wasm", import.meta.url));
await init({ module_or_path: readFileSync(wasmPath) });

const url = process.argv[2] || "https://data.source.coop/giswqs/opengeos/dem.tif";
console.log("fetching", url);

const res = await fetch(url);
if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`);
const bytes = new Uint8Array(await res.arrayBuffer());
console.log("downloaded", bytes.length, "bytes");

// Header only (cheap even for large files):
console.log("info: ", geotiff_info(bytes));

// Full reader (downloads/parses the whole file):
const tif = new GeoTiffReader(bytes);
console.log("size: ", `${tif.width}x${tif.height}`, "epsg:", tif.epsg, "nodata:", tif.nodata);
console.log("bbox: ", Array.from(tif.bounding_box()));
console.log("stats:", tif.stats_json());
