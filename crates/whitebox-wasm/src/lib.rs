//! WebAssembly bindings for the pure-Rust Whitebox GeoTIFF engine ([`wbgeotiff`]).
//!
//! The whole stack is pure Rust with no C dependencies (no GDAL/PROJ/HDF5),
//! so it compiles to `wasm32-unknown-unknown` and runs in any browser, Node,
//! Deno, or Wasmtime host. All functions operate on in-memory byte buffers.
//!
//! Two layers are exposed:
//! - **Convenience functions** ([`geotiff_info`], [`geotiff_stats`],
//!   [`geotiff_read_band_f64`]) for one-shot use.
//! - **Stateful classes** ([`GeoTiffReader`] parses once and serves many reads;
//!   [`CogBuilder`] encodes Cloud Optimized GeoTIFFs to bytes).
use wasm_bindgen::prelude::*;
use wbgeotiff::{CogWriter, Compression, GeoTiff, GeoTransform, SampleFormat};

/// Install a panic hook so Rust panics surface as readable `console.error`
/// messages instead of an opaque `RuntimeError: unreachable`.
#[wasm_bindgen(start)]
pub fn __start() {
    console_error_panic_hook::set_once();
}

// ───────────────────────── helpers ─────────────────────────

/// Format an `f64` as a JSON value: a finite number, or `null` for NaN /
/// infinity (which are not representable in JSON). Mirrors `JSON.stringify`.
fn json_f64(v: f64) -> String {
    if v.is_finite() { v.to_string() } else { "null".to_string() }
}

fn json_opt_f64(v: Option<f64>) -> String {
    v.map(json_f64).unwrap_or_else(|| "null".to_string())
}

fn err_json(msg: &str) -> String {
    format!("{{\"ok\":false,\"error\":\"{}\"}}", msg.replace('"', "'"))
}

fn jerr<E: std::fmt::Display>(ctx: &str) -> impl Fn(E) -> JsValue + '_ {
    move |e| JsValue::from_str(&format!("{ctx}: {e}"))
}

fn sample_format_str(sf: SampleFormat) -> String {
    format!("{:?}", sf).to_lowercase()
}

/// Compute min/max/mean/valid over band 0, returned as a JSON string.
fn stats_json_from(gt: &GeoTiff) -> String {
    let (w, h, bands) = (gt.width(), gt.height(), gt.band_count());
    let epsg = gt.epsg().map(|e| e.to_string()).unwrap_or_else(|| "null".into());
    let values = match gt.read_all_f64() {
        Ok(v) => v,
        Err(e) => return err_json(&format!("read: {e}")),
    };
    let nodata = gt.no_data();
    let (mut min, mut max, mut sum, mut count) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64, 0u64);
    for &v in values.iter().step_by(bands.max(1)) {
        if v.is_nan() { continue; }
        if let Some(nd) = nodata { if v == nd { continue; } }
        if v < min { min = v; }
        if v > max { max = v; }
        sum += v;
        count += 1;
    }
    if count == 0 {
        return format!(
            "{{\"ok\":true,\"width\":{w},\"height\":{h},\"bands\":{bands},\"epsg\":{epsg},\
\"valid\":0,\"min\":null,\"max\":null,\"mean\":null}}"
        );
    }
    let mean = sum / count as f64;
    format!(
        "{{\"ok\":true,\"width\":{w},\"height\":{h},\"bands\":{bands},\"epsg\":{epsg},\
\"valid\":{count},\"min\":{},\"max\":{},\"mean\":{}}}",
        json_f64(min), json_f64(max), json_f64(mean)
    )
}

// ───────────────────────── convenience functions ─────────────────────────

/// Decode only a GeoTIFF's header and return its metadata as JSON. O(header)
/// memory, so it works on multi-gigabyte rasters that whole-image reads cannot
/// fit in WASM's 4 GiB address space.
///
/// `{"ok":true,"width","height","bands","epsg"|null,"nodata"|null,
///   "bits_per_sample","sample_format","compression","tiled","bigtiff"}`
#[wasm_bindgen]
pub fn geotiff_info(data: &[u8]) -> String {
    let m = match GeoTiff::peek_meta(data) {
        Ok(m) => m,
        Err(e) => return err_json(&format!("decode: {e}")),
    };
    let epsg = m.epsg.map(|e| e.to_string()).unwrap_or_else(|| "null".into());
    format!(
        "{{\"ok\":true,\"width\":{},\"height\":{},\"bands\":{},\"epsg\":{},\"nodata\":{},\
\"bits_per_sample\":{},\"sample_format\":\"{}\",\"compression\":\"{:?}\",\"tiled\":{},\"bigtiff\":{}}}",
        m.width, m.height, m.bands, epsg, json_opt_f64(m.no_data),
        m.bits_per_sample, sample_format_str(m.sample_format), m.compression, m.tiled, m.is_bigtiff
    )
}

/// Decode a GeoTIFF and return band-0 summary statistics as JSON:
/// `{"ok":true,"width","height","bands","epsg","valid","min","max","mean"}`.
#[wasm_bindgen]
pub fn geotiff_stats(data: &[u8]) -> String {
    match GeoTiff::from_bytes(data) {
        Ok(gt) => stats_json_from(&gt),
        Err(e) => err_json(&format!("decode: {e}")),
    }
}

/// Read a single band of pixel values as an `f64` `Float64Array` (any on-disk
/// sample format is converted), row-major, length `width * height`.
#[wasm_bindgen]
pub fn geotiff_read_band_f64(data: &[u8], band: usize) -> Result<Vec<f64>, JsValue> {
    GeoTiffReader::new(data)?.read_band_f64(band)
}

/// Semantic version of this crate, exposed for runtime feature detection.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ───────────────────────── GeoTiffReader ─────────────────────────

/// A parsed GeoTIFF held in memory. Construct once, then call the accessor and
/// `read_*` methods many times without re-parsing the file.
#[wasm_bindgen]
pub struct GeoTiffReader {
    inner: GeoTiff,
}

#[wasm_bindgen]
impl GeoTiffReader {
    /// Parse a GeoTIFF / BigTIFF / COG from raw bytes.
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<GeoTiffReader, JsValue> {
        let inner = GeoTiff::from_bytes(data).map_err(jerr("decode"))?;
        Ok(GeoTiffReader { inner })
    }

    // ── metadata accessors ──
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 { self.inner.width() }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 { self.inner.height() }
    #[wasm_bindgen(getter)]
    pub fn bands(&self) -> usize { self.inner.band_count() }
    #[wasm_bindgen(getter)]
    pub fn bits_per_sample(&self) -> u16 { self.inner.bits_per_sample() }
    #[wasm_bindgen(getter)]
    pub fn sample_format(&self) -> String { sample_format_str(self.inner.sample_format()) }
    #[wasm_bindgen(getter)]
    pub fn compression(&self) -> String { format!("{:?}", self.inner.compression()) }
    #[wasm_bindgen(getter)]
    pub fn is_bigtiff(&self) -> bool { self.inner.is_bigtiff }
    /// EPSG code, or `undefined` if the file is not georeferenced by EPSG.
    #[wasm_bindgen(getter)]
    pub fn epsg(&self) -> Option<u32> { self.inner.epsg().map(|e| e as u32) }
    /// No-data sentinel, or `undefined` if none is declared.
    #[wasm_bindgen(getter)]
    pub fn nodata(&self) -> Option<f64> { self.inner.no_data() }

    /// Affine geo-transform as `[x_origin, pixel_width, row_rotation,
    /// y_origin, col_rotation, pixel_height]`, or an empty array if absent.
    pub fn geo_transform(&self) -> Vec<f64> {
        match self.inner.geo_transform() {
            Some(g) => vec![g.x_origin, g.pixel_width, g.row_rotation,
                            g.y_origin, g.col_rotation, g.pixel_height],
            None => Vec::new(),
        }
    }

    /// Bounding box as `[min_x, min_y, max_x, max_y]`, or empty if not georeferenced.
    pub fn bounding_box(&self) -> Vec<f64> {
        match self.inner.bounding_box() {
            Some(b) => vec![b.min_x, b.min_y, b.max_x, b.max_y],
            None => Vec::new(),
        }
    }

    /// GDAL value transform as `[scale, offset]` (physical = raw*scale+offset),
    /// or empty if none. Apply to `read_*` outputs to get physical values.
    pub fn value_transform(&self) -> Vec<f64> {
        match self.inner.value_transform() {
            Some(t) => vec![t.scale, t.offset],
            None => Vec::new(),
        }
    }

    /// Full metadata as a JSON string (same shape as [`geotiff_info`]).
    pub fn info_json(&self) -> String {
        let epsg = self.inner.epsg().map(|e| e.to_string()).unwrap_or_else(|| "null".into());
        format!(
            "{{\"ok\":true,\"width\":{},\"height\":{},\"bands\":{},\"epsg\":{},\"nodata\":{},\
\"bits_per_sample\":{},\"sample_format\":\"{}\",\"compression\":\"{:?}\"}}",
            self.inner.width(), self.inner.height(), self.inner.band_count(), epsg,
            json_opt_f64(self.inner.no_data()), self.inner.bits_per_sample(),
            sample_format_str(self.inner.sample_format()), self.inner.compression()
        )
    }

    /// Band-0 statistics as a JSON string (same shape as [`geotiff_stats`]).
    pub fn stats_json(&self) -> String { stats_json_from(&self.inner) }

    // ── data reads ──
    fn check_band(&self, band: usize) -> Result<(), JsValue> {
        let n = self.inner.band_count();
        if band >= n {
            return Err(JsValue::from_str(&format!("band {band} out of range (bands={n})")));
        }
        Ok(())
    }

    /// Read a band as `f64`, converting from any on-disk type. `Float64Array`.
    pub fn read_band_f64(&self, band: usize) -> Result<Vec<f64>, JsValue> {
        self.check_band(band)?;
        let bands = self.inner.band_count();
        let all = self.inner.read_all_f64().map_err(jerr("read"))?;
        if bands == 1 { Ok(all) } else { Ok(all.into_iter().skip(band).step_by(bands).collect()) }
    }

    /// Read every band as `f64`, interleaved per pixel (`band0,band1,...`).
    pub fn read_all_f64(&self) -> Result<Vec<f64>, JsValue> {
        self.inner.read_all_f64().map_err(jerr("read"))
    }

    /// Read a band's raw, undecoded-to-native bytes. `Uint8Array`.
    pub fn read_band_bytes(&self, band: usize) -> Result<Vec<u8>, JsValue> {
        self.check_band(band)?;
        self.inner.read_band_bytes(band).map_err(jerr("read"))
    }

    // Native typed reads: these require the band's on-disk type to match and
    // error otherwise. Use `read_band_f64` for format-agnostic access.
    /// Native `u8` band. `Uint8Array`.
    pub fn read_band_u8(&self, band: usize) -> Result<Vec<u8>, JsValue> {
        self.check_band(band)?; self.inner.read_band_u8(band).map_err(jerr("read"))
    }
    /// Native `i8` band. `Int8Array`.
    pub fn read_band_i8(&self, band: usize) -> Result<Vec<i8>, JsValue> {
        self.check_band(band)?; self.inner.read_band_i8(band).map_err(jerr("read"))
    }
    /// Native `u16` band. `Uint16Array`.
    pub fn read_band_u16(&self, band: usize) -> Result<Vec<u16>, JsValue> {
        self.check_band(band)?; self.inner.read_band_u16(band).map_err(jerr("read"))
    }
    /// Native `i16` band. `Int16Array`.
    pub fn read_band_i16(&self, band: usize) -> Result<Vec<i16>, JsValue> {
        self.check_band(band)?; self.inner.read_band_i16(band).map_err(jerr("read"))
    }
    /// Native `u32` band. `Uint32Array`.
    pub fn read_band_u32(&self, band: usize) -> Result<Vec<u32>, JsValue> {
        self.check_band(band)?; self.inner.read_band_u32(band).map_err(jerr("read"))
    }
    /// Native `i32` band. `Int32Array`.
    pub fn read_band_i32(&self, band: usize) -> Result<Vec<i32>, JsValue> {
        self.check_band(band)?; self.inner.read_band_i32(band).map_err(jerr("read"))
    }
    /// Native `f32` band. `Float32Array`.
    pub fn read_band_f32(&self, band: usize) -> Result<Vec<f32>, JsValue> {
        self.check_band(band)?; self.inner.read_band_f32(band).map_err(jerr("read"))
    }
}

// ───────────────────────── CogBuilder (writer) ─────────────────────────

/// Builder for encoding a Cloud Optimized GeoTIFF (tiled, with overviews and
/// GDAL ghost metadata) to bytes. A COG is also a valid plain GeoTIFF.
///
/// Configure with the `set_*` methods, then call one of `write_*` with the
/// pixel data to get a `Uint8Array` of the encoded file.
#[wasm_bindgen]
pub struct CogBuilder {
    width: u32,
    height: u32,
    bands: u16,
    epsg: Option<u16>,
    nodata: Option<f64>,
    geo_transform: Option<[f64; 6]>,
    compression: Compression,
    tile_size: Option<u32>,
    bigtiff: bool,
    overview_levels: Option<Vec<u32>>,
}

#[wasm_bindgen]
impl CogBuilder {
    /// New builder for a `width` x `height` raster with `bands` bands.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32, bands: u16) -> CogBuilder {
        CogBuilder {
            width, height, bands: bands.max(1),
            epsg: None, nodata: None, geo_transform: None,
            compression: Compression::Deflate, tile_size: None,
            bigtiff: false, overview_levels: None,
        }
    }

    /// Set the EPSG code (1..=65535).
    pub fn set_epsg(&mut self, epsg: u32) { if (1..=65535).contains(&epsg) { self.epsg = Some(epsg as u16); } }
    /// Set the no-data sentinel value.
    pub fn set_nodata(&mut self, v: f64) { self.nodata = Some(v); }
    /// Set the full affine geo-transform:
    /// `[x_origin, pixel_width, row_rotation, y_origin, col_rotation, pixel_height]`.
    pub fn set_geo_transform(&mut self, gt: Vec<f64>) -> Result<(), JsValue> {
        if gt.len() != 6 { return Err(JsValue::from_str("geo_transform needs 6 values")); }
        self.geo_transform = Some([gt[0], gt[1], gt[2], gt[3], gt[4], gt[5]]);
        Ok(())
    }
    /// Convenience: north-up geo-transform from upper-left origin and pixel size.
    pub fn set_origin(&mut self, x_min: f64, y_max: f64, pixel_size: f64) {
        self.geo_transform = Some([x_min, pixel_size, 0.0, y_max, 0.0, -pixel_size]);
    }
    /// Compression: `none`, `lzw`, `deflate`, `packbits`, `webp`, `jpeg`, `jpegxl`.
    pub fn set_compression(&mut self, name: &str) -> Result<(), JsValue> {
        self.compression = match name.to_lowercase().as_str() {
            "none" => Compression::None,
            "lzw" => Compression::Lzw,
            "deflate" | "zip" => Compression::Deflate,
            "packbits" => Compression::PackBits,
            "webp" => Compression::WebP,
            "jpeg" => Compression::Jpeg,
            "jpegxl" | "jxl" => Compression::JpegXl,
            other => return Err(JsValue::from_str(&format!("unknown compression: {other}"))),
        };
        Ok(())
    }
    /// Internal tile size in pixels (default 512).
    pub fn set_tile_size(&mut self, px: u32) { self.tile_size = Some(px); }
    /// Force BigTIFF (64-bit offsets) for very large outputs.
    pub fn set_bigtiff(&mut self, on: bool) { self.bigtiff = on; }
    /// Explicit overview decimation factors (e.g. `[2,4,8]`); empty disables overviews.
    pub fn set_overview_levels(&mut self, levels: Vec<u32>) { self.overview_levels = Some(levels); }

    fn build(&self) -> CogWriter {
        let mut w = CogWriter::new(self.width, self.height, self.bands)
            .compression(self.compression);
        if let Some(e) = self.epsg { w = w.epsg(e); }
        if let Some(nd) = self.nodata { w = w.no_data(nd); }
        if let Some(g) = self.geo_transform {
            w = w.geo_transform(GeoTransform::new(g[0], g[1], g[2], g[3], g[4], g[5]));
        }
        if let Some(ts) = self.tile_size { w = w.tile_size(ts); }
        if self.bigtiff { w = w.bigtiff(true); }
        if let Some(ref lv) = self.overview_levels { w = w.overview_levels(lv.clone()); }
        w
    }

    fn check_len(&self, len: usize) -> Result<(), JsValue> {
        let expected = self.width as usize * self.height as usize * self.bands as usize;
        if len != expected {
            return Err(JsValue::from_str(&format!(
                "data length {len} != width*height*bands {expected}")));
        }
        Ok(())
    }

    /// Encode `u8` pixel data to a COG. `Uint8Array`.
    pub fn write_u8(&self, data: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.check_len(data.len())?;
        self.build().write_u8_to_vec(data).map_err(jerr("write"))
    }
    /// Encode `f32` pixel data to a COG. `Uint8Array`.
    pub fn write_f32(&self, data: &[f32]) -> Result<Vec<u8>, JsValue> {
        self.check_len(data.len())?;
        self.build().write_f32_to_vec(data).map_err(jerr("write"))
    }
    /// Encode `f64` pixel data to a COG. `Uint8Array`.
    pub fn write_f64(&self, data: &[f64]) -> Result<Vec<u8>, JsValue> {
        self.check_len(data.len())?;
        self.build().write_f64_to_vec(data).map_err(jerr("write"))
    }
}
