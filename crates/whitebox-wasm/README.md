# whitebox-wasm

**Pure-Rust GeoTIFF decoding compiled to WebAssembly.** No GDAL, no PROJ, no
native libraries, no server. Decode GeoTIFF / BigTIFF / COG entirely in the
browser, Node, Deno, or any Wasm host.

This wraps `wbgeotiff`, the shared GeoTIFF engine from the next-generation,
pure-Rust WhiteboxTools, and exposes a tiny WebAssembly API. The entire codec
stack (Deflate, LZW, PackBits, JPEG, WebP, JPEG-XL, PNG predictors, BigTIFF,
tiling) is pure Rust with zero C dependencies, so the published module imports
nothing from the host beyond its own linear memory.

## Install

```bash
npm install whitebox-wasm
```

## Usage (browser / Deno / Node >= 20, ESM)

```js
import init, { geotiff_stats, geotiff_info, version } from "whitebox-wasm";

await init();                       // in Node, pass the .wasm bytes to init()
const bytes = new Uint8Array(await (await fetch("dem.tif")).arrayBuffer());

console.log(version());             // "0.1.0"
console.log(JSON.parse(geotiff_info(bytes)));
// { ok:true, width:512, height:512, bands:1, epsg:32610, nodata:-9999 }

console.log(JSON.parse(geotiff_stats(bytes)));
// { ok:true, width:512, height:512, bands:1, epsg:32610,
//   valid:262144, min:..., max:..., mean:... }
```

In Node the `web` target needs the wasm bytes handed to `init()`:

```js
import { readFileSync } from "node:fs";
import init, { geotiff_info } from "whitebox-wasm";
await init({ module_or_path: readFileSync("node_modules/whitebox-wasm/whitebox_wasm_bg.wasm") });
```

## API

| Function | Returns (JSON string) |
|---|---|
| `geotiff_info(bytes)` | `{ok, width, height, bands, epsg, nodata}` |
| `geotiff_stats(bytes)` | `{ok, width, height, bands, epsg, valid, min, max, mean}` (band 0, skips NaN/nodata) |
| `version()` | crate version string |

On failure, functions return `{"ok":false,"error":"..."}`.

## Links

- Source, live demo, and issues: https://github.com/opengeos/whitebox-wasm
- Live browser demo: https://opengeos.org/whitebox-wasm/

## License

Dual-licensed under MIT or Apache-2.0, at your option. Includes the vendored
`wbgeotiff` crate (Copyright John Lindsay, Whitebox Geospatial Inc.), used under
the same dual license.
