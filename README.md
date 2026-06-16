# whitebox-wasm

[![CI](https://github.com/opengeos/whitebox-wasm/actions/workflows/ci.yml/badge.svg)](https://github.com/opengeos/whitebox-wasm/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/whitebox-wasm.svg)](https://www.npmjs.com/package/whitebox-wasm)
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
npm install whitebox-wasm
```

## Usage (browser / Deno / Node ≥ 20, ESM)

```js
import init, { geotiff_info, GeoTiffReader, CogBuilder } from "whitebox-wasm";

await init();                       // in Node, pass the .wasm bytes to init()

// Read a GeoTIFF/COG straight from a URL (the fetch happens in JS; the
// sandboxed wasm has no network of its own):
const bytes = new Uint8Array(
  await (await fetch("https://example.com/dem.tif")).arrayBuffer());

console.log(JSON.parse(geotiff_info(bytes)));   // header-only, O(header) memory

const tif = new GeoTiffReader(bytes);           // parse once, read many
console.log(tif.width, tif.height, tif.epsg, tif.nodata);
console.log(Array.from(tif.bounding_box()));
const band0 = tif.read_band_f64(0);             // Float64Array
console.log(JSON.parse(tif.stats_json()));

// Write a Cloud Optimized GeoTIFF (valid plain GeoTIFF too):
const cb = new CogBuilder(width, height, 1);
cb.set_epsg(32610); cb.set_origin(500000, 4000000, 30); cb.set_compression("deflate");
const cogBytes = cb.write_f64(new Float64Array(pixels));   // Uint8Array
```

Runnable Node examples: [`examples/node-demo.mjs`](examples/node-demo.mjs) and
[`examples/read-from-url.mjs`](examples/read-from-url.mjs).

## API

**Convenience:** `geotiff_info(bytes)` (header-only JSON, works on multi-GB files),
`geotiff_stats(bytes)` (band-0 stats JSON), `geotiff_read_band_f64(bytes, band)`
(`Float64Array`), `version()`.

**`GeoTiffReader(bytes)`** - parse once, then: `width`/`height`/`bands`/`epsg`/
`nodata`/`sample_format`/`compression`/`bits_per_sample`/`is_bigtiff`;
`geo_transform()`, `bounding_box()`, `value_transform()`; `info_json()`,
`stats_json()`; `read_band_f64(band)`, `read_all_f64()`, `read_band_bytes(band)`,
and native typed reads `read_band_u8|i8|u16|i16|u32|i32|f32`.

**`CogBuilder(width, height, bands)`** - `set_epsg`, `set_nodata`,
`set_compression`, `set_geo_transform`/`set_origin`, `set_tile_size`,
`set_bigtiff`, `set_overview_levels`, then `write_u8|write_f32|write_f64(data)`
-> `Uint8Array` (tiled COG with overviews and GDAL ghost metadata).

JSON functions report errors as `{"ok":false,"error":"..."}`; class methods throw.

### Reading from an HTTP URL

The wasm module is sandboxed and does no network I/O itself (it imports nothing
from the host). HTTP happens in JavaScript, which hands the bytes to wasm:

```js
const res = await fetch(url);                       // browser, Deno, Node >= 18
const bytes = new Uint8Array(await res.arrayBuffer());
const tif = new GeoTiffReader(bytes);
```

This downloads the whole file. True COG range-request streaming (fetch only the
header + the overview/tiles you need) is a planned enhancement; see Limits.

## Limits

WebAssembly is 32-bit, so linear memory is capped at ~4 GiB. `geotiff_info` is
header-only and works on multi-gigabyte rasters, but any operation that
materializes a full raster (whole-band reads/writes, stats) is bounded by that
ceiling. A national 1-billion-pixel raster cannot be fully decoded in-browser;
read metadata only, or process server-side. Range-request COG streaming (read
just the header + needed tiles, never the whole file) would lift this for remote
COGs and is on the roadmap.

## Build from source

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build crates/whitebox-wasm --release --target web --out-dir pkg
```

## Releasing

Push a tag `vX.Y.Z`. CI then:
1. publishes `whitebox-wasm@X.Y.Z` to npm via Trusted Publishing (OIDC, with provenance, no secret required),
2. attaches the raw `.wasm` + JS loader to the GitHub Release.

Pushes to `main` redeploy the [live demo](https://opengeos.github.io/whitebox-wasm/) to GitHub Pages.

## Credits

The GeoTIFF engine comes from the original
[**whitebox_next_gen**](https://github.com/jblindsay/whitebox_next_gen) project
by John Lindsay (Whitebox Geospatial Inc.), the next-generation, pure-Rust
rewrite of WhiteboxTools. This repository vendors its `wbgeotiff` crate and adds
a thin WebAssembly binding. All credit for the underlying codec belongs to that
project.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. Includes the vendored `wbgeotiff` crate (© John Lindsay, Whitebox
Geospatial Inc.), used under the same dual license.
