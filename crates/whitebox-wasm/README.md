# whitebox-wasm

**Pure-Rust GeoTIFF decoding compiled to WebAssembly.** No GDAL, no PROJ, no
native libraries, no server. Decode GeoTIFF / BigTIFF / COG entirely in the
browser, Node, Deno, or any Wasm host.

This wraps `wbgeotiff`, the shared GeoTIFF engine from the original
[**whitebox_next_gen**](https://github.com/jblindsay/whitebox_next_gen) project
by John Lindsay (Whitebox Geospatial Inc.) - the next-generation, pure-Rust
rewrite of WhiteboxTools - and exposes a tiny WebAssembly API. The entire codec
stack (Deflate, LZW, PackBits, JPEG, WebP, JPEG-XL, PNG predictors, BigTIFF,
tiling) is pure Rust with zero C dependencies, so the published module imports
nothing from the host beyond its own linear memory.

## Install

```bash
npm install whitebox-wasm
```

## Usage (browser / Deno / Node >= 20, ESM)

```js
import init, { geotiff_info, GeoTiffReader, CogBuilder } from "whitebox-wasm";

await init();                       // in Node, pass the .wasm bytes to init()
const bytes = new Uint8Array(await (await fetch("dem.tif")).arrayBuffer());

// Metadata only - O(header) memory, works on multi-GB rasters:
console.log(JSON.parse(geotiff_info(bytes)));
// { ok:true, width, height, bands, epsg, nodata, bits_per_sample,
//   sample_format, compression, tiled, bigtiff }

// Parse once, read many times:
const tif = new GeoTiffReader(bytes);
console.log(tif.width, tif.height, tif.bands, tif.epsg, tif.nodata);
console.log(Array.from(tif.geo_transform()));   // [ox, px, 0, oy, 0, -py]
console.log(Array.from(tif.bounding_box()));    // [minx, miny, maxx, maxy]
const band0 = tif.read_band_f64(0);             // Float64Array
console.log(JSON.parse(tif.stats_json()));

// Encode a Cloud Optimized GeoTIFF (also a valid plain GeoTIFF):
const cb = new CogBuilder(width, height, 1);
cb.set_epsg(32610);
cb.set_origin(500000, 4000000, 30);             // x_min, y_max, pixel size
cb.set_compression("deflate");
cb.set_nodata(-9999);
const cogBytes = cb.write_f64(new Float64Array(pixels));   // Uint8Array
```

In Node the `web` target needs the wasm bytes handed to `init()`:

```js
import { readFileSync } from "node:fs";
import init, { geotiff_info } from "whitebox-wasm";
await init({ module_or_path: readFileSync("node_modules/whitebox-wasm/whitebox_wasm_bg.wasm") });
```

## API

### Convenience functions

| Function | Returns |
|---|---|
| `geotiff_info(bytes)` | JSON metadata, header-only (works on multi-GB files) |
| `geotiff_stats(bytes)` | JSON band-0 stats `{valid, min, max, mean, ...}` |
| `geotiff_read_band_f64(bytes, band)` | `Float64Array` of band pixels |
| `version()` | crate version string |

### `GeoTiffReader` (parse once, read many)

`new GeoTiffReader(bytes)`, then:

- Properties: `width`, `height`, `bands`, `bits_per_sample`, `sample_format`, `compression`, `is_bigtiff`, `epsg`, `nodata`
- `geo_transform()` -> `[x_origin, pixel_width, row_rot, y_origin, col_rot, pixel_height]` (empty if none)
- `bounding_box()` -> `[min_x, min_y, max_x, max_y]` (empty if not georeferenced)
- `value_transform()` -> `[scale, offset]` (GDAL scale/offset; empty if none)
- `info_json()`, `stats_json()` -> JSON strings
- `read_band_f64(band)` / `read_all_f64()` -> `Float64Array` (any on-disk type converted)
- `read_band_bytes(band)` -> raw `Uint8Array`
- Native typed reads (require matching on-disk type): `read_band_u8` / `i8` / `u16` / `i16` / `u32` / `i32` / `f32`

### `CogBuilder` (write Cloud Optimized GeoTIFFs)

`new CogBuilder(width, height, bands)`, configure, then `write_u8` / `write_f32` / `write_f64(data)` -> `Uint8Array`:

- `set_epsg(code)`, `set_nodata(v)`, `set_compression("none|lzw|deflate|packbits|webp|jpeg|jpegxl")`
- `set_geo_transform([6 values])` or `set_origin(x_min, y_max, pixel_size)`
- `set_tile_size(px)`, `set_bigtiff(bool)`, `set_overview_levels([2,4,8])`

Output is a tiled COG with overviews and GDAL ghost metadata - readable by GDAL, rasterio, QGIS, and `GeoTiffReader`.

JSON-returning functions report failures as `{"ok":false,"error":"..."}`; class methods throw on error.

## Limits

WebAssembly is 32-bit, so linear memory is capped at ~4 GiB. `geotiff_info` is
header-only and unaffected, but reads/writes that materialize a full raster are
bounded by that ceiling (a national 1-billion-pixel raster cannot be fully
decoded in-browser). For such data, read metadata only or process server-side.

## Links

- Source, live demo, and issues: https://github.com/opengeos/whitebox-wasm
- Live browser demo: https://opengeos.org/whitebox-wasm/

## License

Dual-licensed under MIT or Apache-2.0, at your option. Includes the vendored
`wbgeotiff` crate (Copyright John Lindsay, Whitebox Geospatial Inc.), used under
the same dual license.
