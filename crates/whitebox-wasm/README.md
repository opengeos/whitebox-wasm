# whitebox-wasm

**Pure-Rust geospatial toolkit compiled to WebAssembly.** No GDAL, no PROJ, no
native libraries, no server. Work with raster, vector, and LiDAR data entirely
in the browser, Node, Deno, or any Wasm host:

- **Raster** - read/write GeoTIFF / BigTIFF / COG, stats, range-request streaming
- **Projections** - full EPSG + user-defined CRS to WGS84 lon/lat
- **Vector** - read GeoJSON, TopoJSON, GML, GPX, KML, FlatGeobuf, GeoPackage, KMZ -> GeoJSON, with reprojection
- **LiDAR** - read LAS / LAZ / PLY point clouds (xyz, classification, intensity)
- **Analysis** - convex hull, Moran's I spatial autocorrelation

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
| `geotiff_info(bytes)` | JSON metadata, header-only (incl. `bbox`, `center`, `center_lonlat`) |
| `geotiff_stats(bytes)` | JSON band-0 stats `{valid, min, max, mean, ...}` |
| `geotiff_read_band_f64(bytes, band)` | `Float64Array` of band pixels |
| `version()` | crate version string |

### `GeoTiffReader` (parse once, read many)

`new GeoTiffReader(bytes)`, then:

- Properties: `width`, `height`, `bands`, `bits_per_sample`, `sample_format`, `compression`, `is_bigtiff`, `epsg`, `nodata`
- `geo_transform()` -> `[x_origin, pixel_width, row_rot, y_origin, col_rot, pixel_height]` (empty if none)
- `bounding_box()` -> `[min_x, min_y, max_x, max_y]` in the dataset CRS (empty if not georeferenced)
- `center()` -> `[x, y]` center in the dataset CRS
- `center_lonlat()` -> `[lon, lat]` WGS84 degrees; `bounds_lonlat()` -> `[min_lon, min_lat, max_lon, max_lat]` WGS84 (any EPSG via the bundled pure-Rust projection engine; also handles user-defined projections like NLCD's Albers; empty if not georeferenced)
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

### `CogStream` (read remote COGs via HTTP range requests)

Read a window or overview out of a large COG **without downloading the whole
file**. The wasm parses the header and reports byte ranges; your JS does the
range requests:

```js
import init, { CogStream } from "whitebox-wasm";
await init();

const url = "https://example.com/big-cog.tif";
const range = (a, b) => fetch(url, { headers: { Range: `bytes=${a}-${b}` } })
  .then(r => r.arrayBuffer()).then(b => new Uint8Array(b));

const stream = new CogStream(await range(0, 65535));   // header prefix
const lv = JSON.parse(stream.levels_json())[0];        // level 0 = full res
const tiles = JSON.parse(stream.tiles_for_window(0, 1200, 800, 256, 256));

for (const t of tiles) {
  const bytes = await range(t.offset, t.offset + t.length - 1);  // one tile
  const px = stream.decode_tile_f64(0, bytes);                   // Float64Array
  // place px (tile_width x tile_height) into your output window...
}
```

- `new CogStream(headerBytes)` - parse the IFD chain + tile index (throws if the
  prefix is too short; fetch more and retry).
- `num_levels`, `epsg`, `nodata`, `geo_transform()`, `levels_json()`
- `bounding_box()`, `center()`, `center_lonlat()`, `bounds_lonlat()` (same semantics as `GeoTiffReader`)
- `tiles_for_window(level, x, y, w, h)` -> JSON `[{col,row,offset,length}]`
- `tile_range(level, col, row)` -> `[offset, length]`
- `decode_tile_f64(level, tileBytes)` -> `Float64Array` (one decoded tile)

Use a higher `level` (overview) for zoomed-out views. See
[`examples/cog-stream.mjs`](../../examples/cog-stream.mjs) for a full window read
that fetches only the tiles it needs (about 13% of a 5.7 MiB file for a 256x256
window). Requires a tiled COG on a server that supports HTTP range requests.

JSON-returning functions report failures as `{"ok":false,"error":"..."}`; class methods throw on error.

## Vector

```js
import init, { vector_to_geojson, vector_info, vector_to_geojson_reproject } from "whitebox-wasm";
await init();
const bytes = new Uint8Array(await (await fetch("data.fgb")).arrayBuffer());
const geojson = JSON.parse(vector_to_geojson(bytes, "flatgeobuf"));
const meta = JSON.parse(vector_info(bytes, "flatgeobuf")); // { features, geometry, epsg, fields, bbox }
const wgs84 = vector_to_geojson_reproject(bytes, "flatgeobuf", 4326, 0); // dst, src(0=auto)
```

- `vector_formats()` -> supported formats (geojson, topojson, gml, gpx, kml, flatgeobuf, geopackage, kmz)
- `vector_to_geojson(data, format)` -> GeoJSON string
- `vector_to_geojson_reproject(data, format, dst_epsg, src_epsg)` -> reprojected GeoJSON (`src_epsg=0` uses the layer CRS, or 4326)
- `vector_info(data, format)` -> JSON `{name, features, geometry, epsg, fields, bbox}`

## LiDAR

```js
import init, { lidar_info, lidar_read_xyz } from "whitebox-wasm";
await init();
const las = new Uint8Array(await (await fetch("cloud.laz")).arrayBuffer());
const meta = JSON.parse(lidar_info(las, "laz")); // { points, epsg, point_format, bounds }
const xyz = lidar_read_xyz(las, "laz");          // Float64Array [x0,y0,z0, x1,y1,z1, ...]
```

- `lidar_formats()`, `lidar_info(data, format)` (header-only count/bounds for LAS/LAZ)
- `lidar_read_xyz` -> `Float64Array`; `lidar_read_classification` -> `Uint8Array`; `lidar_read_intensity` -> `Uint16Array`

## Analysis

- `convex_hull(points_xy)` -> hull ring `Float64Array` (input `[x0,y0,x1,y1,...]`)
- `morans_i(points_xy, values, distance_threshold)` -> JSON global spatial autocorrelation `{morans_i, expected, variance, z_score, p_value, n}`

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
`wbgeotiff`, `wbprojection`, `wbvector`, `wblidar`, `wbtopology`, `wbspatialstats`, and `wbhdf` crates (Copyright John Lindsay, Whitebox
Geospatial Inc.), used under the same dual license.
