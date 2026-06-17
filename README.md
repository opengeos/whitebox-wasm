# whitebox-wasm

[![CI](https://github.com/opengeos/whitebox-wasm/actions/workflows/ci.yml/badge.svg)](https://github.com/opengeos/whitebox-wasm/actions/workflows/ci.yml)
[![WASI CLI](https://github.com/opengeos/whitebox-wasm/actions/workflows/wasi.yml/badge.svg)](https://github.com/opengeos/whitebox-wasm/actions/workflows/wasi.yml)
[![npm](https://img.shields.io/npm/v/whitebox-wasm.svg)](https://www.npmjs.com/package/whitebox-wasm)
[![Live demo](https://img.shields.io/badge/demo-GitHub%20Pages-blue)](https://opengeos.github.io/whitebox-wasm/)

**Pure-Rust geospatial toolkit compiled to WebAssembly.** No GDAL, no PROJ, no
native libraries, no server. Two artifacts, both pure WebAssembly:

- a **browser/Node library** (the npm package) for raster, vector, and LiDAR I/O
  and analysis, and
- a **WASI command-line runner** that runs the full WhiteboxTools algorithm suite
  on regular files.

**Library** (browser / Node / Deno / any Wasm host):

- **Raster** - GeoTIFF / BigTIFF / COG read + write, stats, HTTP range-request streaming
- **Projections** - full EPSG and user-defined CRS to WGS84 lon/lat
- **Vector** - GeoJSON / TopoJSON / GML / GPX / KML / FlatGeobuf / GeoPackage / KMZ -> GeoJSON, with reprojection
- **LiDAR** - LAS / LAZ / PLY point clouds (xyz, classification, intensity)
- **Analysis** - convex hull, Moran's I spatial autocorrelation

**Tools** (WASI): **733 WhiteboxTools** (slope, filters, hydrology,
geomorphometry, ...) that read and write rasters as regular files - see
[Running the tools on files](#wbtools_oss-tools-on-real-files-wasi).

This vendors the pure-Rust geospatial crates from
[**whitebox_next_gen**](https://github.com/jblindsay/whitebox_next_gen) (the
next-generation WhiteboxTools) and exposes them through a WebAssembly API.
Maintainers: see [MAINTAINING.md](MAINTAINING.md) for the vendored-crate patch
set and how to re-sync from upstream.

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
`geo_transform()`, `bounding_box()`, `center()`, `center_lonlat()`,
`bounds_lonlat()` (WGS84, full EPSG support via the bundled pure-Rust projection
engine, plus user-defined projections), `value_transform()`; `info_json()`,
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

That downloads the whole file. To read a window or overview out of a large
remote COG **without downloading all of it**, use `CogStream` with HTTP range
requests - the wasm reports byte ranges, your JS fetches them:

```js
import init, { CogStream } from "whitebox-wasm";
await init();
const range = (a, b) => fetch(url, { headers: { Range: `bytes=${a}-${b}` } })
  .then(r => r.arrayBuffer()).then(b => new Uint8Array(b));

const stream = new CogStream(await range(0, 65535));         // header only
const tiles = JSON.parse(stream.tiles_for_window(0, 1200, 800, 256, 256));
for (const t of tiles) {
  const px = stream.decode_tile_f64(0, await range(t.offset, t.offset + t.length - 1));
  // ... place the decoded tile into your window
}
```

See [`examples/cog-stream.mjs`](examples/cog-stream.mjs) - a 256x256 window read
that fetches ~13% of a 5.7 MiB file. `CogStream` API: `num_levels`, `epsg`,
`nodata`, `geo_transform()`, `levels_json()`, `tiles_for_window(level,x,y,w,h)`,
`tile_range(level,col,row)`, `decode_tile_f64(level, bytes)`.

## Limits

WebAssembly is 32-bit, so linear memory is capped at ~4 GiB. `geotiff_info` is
header-only and works on multi-gigabyte rasters, and `CogStream` reads remote
COGs tile-by-tile within that budget. But operations that materialize a *full*
raster (whole-band reads/writes, `geotiff_stats`) are bounded by the ceiling and
return a clean error (never a crash) when a raster is too large - a national
billion-pixel raster cannot be fully decoded in one piece in-browser. Read
metadata, stream a window/overview, or process server-side instead.

## Build from source

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build crates/whitebox-wasm --release --target web --out-dir pkg
```

## Compiling other Whitebox crates to WASM

The crates vendored here all compile cleanly to `wasm32-unknown-unknown`:
`wbgeotiff`, `wbprojection`, `wbhdf`, `wbvector`, `wblidar`, `wbtopology`, and
`wbspatialstats`. The only change any of them needed was enabling the
[`getrandom`](https://docs.rs/getrandom/#webassembly-support) `js` feature on
WASM (for `wbspatialstats`, which uses `rand`):

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }
```

### `wbtools_oss` tools on real files (WASI)

The full `wbtools_oss` algorithm suite (**733 tools** - slope, filters,
hydrology, geomorphometry, ...) reads and writes rasters by **file path**, so it
cannot run in the browser (no filesystem). It *can* run under
[**WASI**](https://wasi.dev) (`wasm32-wasip1`), which provides a real filesystem,
and the tools then work with regular files unchanged. This repo ships a WASI CLI
(`crates/whitebox-cli`) for exactly that:

```bash
rustup target add wasm32-wasip1
cargo build -p whitebox-cli --target wasm32-wasip1 --release

# list all tools
wasmtime run target/wasm32-wasip1/release/whitebox.wasm list

# run a tool on regular files (map a host dir into the sandbox with --dir)
wasmtime run --dir ./data::/work \
  target/wasm32-wasip1/release/whitebox.wasm \
  slope --input=/work/dem.tif --output=/work/slope.tif --units=degrees
```

The only change needed to compile `wbtools_oss` to WASI is patching the
`kdtree 0.8.0` dependency, which ships `criterion` as a *normal* dependency (a
packaging bug - it belongs in `dev-dependencies`) and `criterion` has a
`compile_error!` for `wasm + rayon`. The vendored `crates/kdtree-wasm` drops that
dependency and the workspace `[patch.crates-io]` applies it. (`getrandom` and
`ureq`/`rustls` - blockers on the browser `wasm32-unknown-unknown` target - are
*not* blockers on WASI, which provides randomness and a clock natively.) This fix
is best made **upstream** in the `kdtree` crate and
[`whitebox_next_gen`](https://github.com/jblindsay/whitebox_next_gen).

See [MAINTAINING.md](MAINTAINING.md) for the full WASM fix set and how to re-sync vendored crates from upstream.

These crates (`wbcore`, `wbraster`, `wbtools_oss`, `whitebox-cli`, `kdtree-wasm`)
are a **separate WASI artifact** and are not part of the browser npm package -
the npm `.wasm` does not include them.

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
option. Includes the vendored `wbgeotiff` and `wbprojection` crates (© John
Lindsay, Whitebox Geospatial Inc.) from
[whitebox_next_gen](https://github.com/jblindsay/whitebox_next_gen), used under
the same dual license.
