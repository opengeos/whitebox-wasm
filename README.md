# whitebox-wasm

[![CI](https://github.com/opengeos/whitebox-wasm/actions/workflows/ci.yml/badge.svg)](https://github.com/opengeos/whitebox-wasm/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/@opengeos/whitebox-wasm.svg)](https://www.npmjs.com/package/@opengeos/whitebox-wasm)
[![Live demo](https://img.shields.io/badge/demo-GitHub%20Pages-blue)](https://opengeos.github.io/whitebox-wasm/)

**Pure-Rust GeoTIFF decoding compiled to WebAssembly.** No GDAL, no PROJ, no
native libraries, no server. Decode GeoTIFF / BigTIFF / COG entirely in the
browser, Node, Deno, or any Wasm host.

This wraps [`wbgeotiff`](https://github.com/jblindsay/whitebox_next_gen) - the
shared GeoTIFF engine from the next-generation, pure-Rust WhiteboxTools - and
exposes a tiny WebAssembly API.

## Why this works

The entire codec stack (Deflate, LZW, PackBits, JPEG, WebP, JPEG-XL, PNG
predictors, BigTIFF, tiling) is implemented in pure Rust with **zero C
dependencies**, so it cross-compiles to `wasm32-unknown-unknown` cleanly. The
published module imports nothing from the host beyond its own linear memory.

## Install

```bash
npm install @opengeos/whitebox-wasm
```

## Usage (browser / Deno / Node ≥ 20, ESM)

```js
import init, { geotiff_stats, geotiff_info, version } from "@opengeos/whitebox-wasm";

await init();                       // in Node, pass the .wasm bytes to init()
const bytes = new Uint8Array(await (await fetch("dem.tif")).arrayBuffer());

console.log(version());             // "0.1.0"
console.log(JSON.parse(geotiff_info(bytes)));
// { ok:true, width:512, height:512, bands:1, epsg:32610, nodata:-9999 }

console.log(JSON.parse(geotiff_stats(bytes)));
// { ok:true, width:512, height:512, bands:1, epsg:32610,
//   valid:262144, min:..., max:..., mean:... }
```

A runnable Node example lives in [`examples/node-demo.mjs`](examples/node-demo.mjs).

## API

| Function | Returns (JSON string) |
|---|---|
| `geotiff_info(bytes)` | `{ok, width, height, bands, epsg, nodata}` |
| `geotiff_stats(bytes)` | `{ok, width, height, bands, epsg, valid, min, max, mean}` (band 0, skips NaN/nodata) |
| `version()` | crate version string |

On failure functions return `{"ok":false,"error":"..."}`.

## Build from source

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build crates/whitebox-wasm --release --target web --scope opengeos --out-dir pkg
```

## Releasing

Push a tag `vX.Y.Z`. CI then:
1. publishes `@opengeos/whitebox-wasm@X.Y.Z` to npm (requires the `NPM_TOKEN` repo secret),
2. attaches the raw `.wasm` + JS loader to the GitHub Release.

Pushes to `main` redeploy the [live demo](https://opengeos.github.io/whitebox-wasm/) to GitHub Pages.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. Includes the vendored `wbgeotiff` crate (© John Lindsay, Whitebox
Geospatial Inc.), used under the same dual license.
