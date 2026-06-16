// Stream a window out of a remote Cloud Optimized GeoTIFF using HTTP range
// requests - fetching only the header and the tiles that overlap the window,
// never the whole file.
//
//   node examples/cog-stream.mjs [url] [x y w h]
//
// The wasm module does no network I/O; it parses the header and tells us which
// byte ranges to fetch. JS issues the range requests and feeds tiles back.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import init, { CogStream, GeoTiffReader } from "../crates/whitebox-wasm/pkg/whitebox_wasm.js";

const wasmPath = fileURLToPath(new URL("../crates/whitebox-wasm/pkg/whitebox_wasm_bg.wasm", import.meta.url));
await init({ module_or_path: readFileSync(wasmPath) });

const url = process.argv[2] || "https://data.source.coop/giswqs/opengeos/dem.tif";
const [x, y, w, h] = (process.argv.slice(3).map(Number));
const win = { x: x || 1200, y: y || 800, w: w || 256, h: h || 256 };

// Fetch a byte range [start, end] inclusive.
let bytesFetched = 0;
async function range(start, end) {
  const res = await fetch(url, { headers: { Range: `bytes=${start}-${end}` } });
  if (res.status !== 206) throw new Error(`range not supported (HTTP ${res.status})`);
  const buf = new Uint8Array(await res.arrayBuffer());
  bytesFetched += buf.length;
  return buf;
}

// 1) header: grow the prefix until the layout parses.
let stream, headerLen = 32768;
for (;;) {
  try { stream = new CogStream(await range(0, headerLen - 1)); break; }
  catch (e) { headerLen *= 2; if (headerLen > (1 << 24)) throw e; }
}
console.log(`parsed header (${headerLen >> 10} KiB), levels=${stream.num_levels}, epsg=${stream.epsg}`);

// 2) pick full-res level 0, list the tiles overlapping the window.
const level = 0;
const lv = JSON.parse(stream.levels_json())[level];
const tiles = JSON.parse(stream.tiles_for_window(level, win.x, win.y, win.w, win.h));
console.log(`window ${win.w}x${win.h}@(${win.x},${win.y}) needs ${tiles.length} tiles`);

// 3) range-fetch each tile, decode, assemble into the window.
const out = new Float64Array(win.w * win.h).fill(NaN);
for (const t of tiles) {
  const tileBytes = await range(t.offset, t.offset + t.length - 1);
  const px = stream.decode_tile_f64(level, tileBytes);
  const tx0 = t.col * lv.tile_width, ty0 = t.row * lv.tile_height;
  for (let r = 0; r < lv.tile_height; r++) for (let c = 0; c < lv.tile_width; c++) {
    const gx = tx0 + c, gy = ty0 + r;
    if (gx >= win.x && gx < win.x + win.w && gy >= win.y && gy < win.y + win.h)
      out[(gy - win.y) * win.w + (gx - win.x)] = px[r * lv.tile_width + c];
  }
}

// summary: bytes fetched vs full file size
const head = await fetch(url, { method: "HEAD" });
const total = Number(head.headers.get("content-length")) || 0;
const valid = out.filter((v) => Number.isFinite(v));
console.log(`fetched ${(bytesFetched/1024).toFixed(1)} KiB of ${(total/1024/1024).toFixed(1)} MiB total` +
  ` (${(100*bytesFetched/total).toFixed(1)}%)`);
console.log(`window samples: ${valid.length} finite, ` +
  `min=${Math.min(...valid).toFixed(2)} max=${Math.max(...valid).toFixed(2)}`);
