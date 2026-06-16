//! WebAssembly bindings for the pure-Rust Whitebox GeoTIFF engine ([`wbgeotiff`]).
//!
//! The whole stack is pure Rust with no C dependencies (no GDAL/PROJ/HDF5),
//! so it compiles to `wasm32-unknown-unknown` and runs in any browser, Node,
//! Deno, or Wasmtime host. All functions operate on in-memory byte buffers.
use wasm_bindgen::prelude::*;
use wbgeotiff::GeoTiff;

/// Install a panic hook so Rust panics surface as readable `console.error`
/// messages instead of an opaque `RuntimeError: unreachable`.
#[wasm_bindgen(start)]
pub fn __start() {
    console_error_panic_hook::set_once();
}

/// Format an `f64` as a JSON value: a finite number, or `null` for NaN /
/// infinity (which are not representable in JSON). This mirrors the behaviour
/// of `JSON.stringify`, which also serialises `NaN`/`Infinity` to `null`.
fn json_f64(v: f64) -> String {
    if v.is_finite() { v.to_string() } else { "null".to_string() }
}

/// Like [`json_f64`] but for an optional value (`None` -> `null`).
fn json_opt_f64(v: Option<f64>) -> String {
    v.map(json_f64).unwrap_or_else(|| "null".to_string())
}

/// Decode a GeoTIFF from raw bytes and return summary statistics as a JSON string.
///
/// The returned JSON has the shape:
/// `{"ok":true,"width":W,"height":H,"bands":B,"epsg":E|null,"valid":N,
///   "min":..,"max":..,"mean":..}` on success, or
/// `{"ok":false,"error":"..."}` on failure.
///
/// Statistics are computed over band 0, skipping NaN and the nodata value.
#[wasm_bindgen]
pub fn geotiff_stats(data: &[u8]) -> String {
    stats_json(data)
}

/// Decode only a GeoTIFF's header and return its metadata as JSON:
/// `{"ok":true,"width":W,"height":H,"bands":B,"epsg":E|null,"nodata":V|null,
///   "bits_per_sample":N,"sample_format":"uint|int|float","compression":"...",
///   "tiled":bool,"bigtiff":bool}`.
///
/// This is O(header) memory: it never loads pixel data, so it works on
/// multi-gigabyte rasters that whole-image reads cannot fit in WASM's 4 GiB
/// address space.
#[wasm_bindgen]
pub fn geotiff_info(data: &[u8]) -> String {
    let m = match GeoTiff::peek_meta(data) {
        Ok(m) => m,
        Err(e) => return err_json(&format!("decode: {e}")),
    };
    let epsg = m.epsg.map(|e| e.to_string()).unwrap_or_else(|| "null".into());
    let nodata = json_opt_f64(m.no_data);
    let sf = format!("{:?}", m.sample_format).to_lowercase();
    let comp = format!("{:?}", m.compression);
    format!(
        "{{\"ok\":true,\"width\":{},\"height\":{},\"bands\":{},\"epsg\":{},\"nodata\":{},\
\"bits_per_sample\":{},\"sample_format\":\"{}\",\"compression\":\"{}\",\"tiled\":{},\"bigtiff\":{}}}",
        m.width, m.height, m.bands, epsg, nodata,
        m.bits_per_sample, sf, comp, m.tiled, m.is_bigtiff
    )
}

/// Read a single band of pixel values as an `f64` array (any on-disk sample
/// format is converted to `f64`). Returns a `Float64Array` in row-major order,
/// length `width * height`.
///
/// This loads the whole band into memory, so it is bounded by WASM's 4 GiB
/// address space. Use [`geotiff_read_window_f64`] for sub-regions of large
/// rasters. On error (decode failure, bad band index, out of memory) the
/// promise/return throws via `Result`.
#[wasm_bindgen]
pub fn geotiff_read_band_f64(data: &[u8], band: usize) -> Result<Vec<f64>, JsValue> {
    let gt = GeoTiff::from_bytes(data).map_err(|e| JsValue::from_str(&format!("decode: {e}")))?;
    let bands = gt.band_count();
    if band >= bands {
        return Err(JsValue::from_str(&format!("band {band} out of range (bands={bands})")));
    }
    // read_all_f64 returns chunky/interleaved samples; de-interleave the band.
    let all = gt.read_all_f64().map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    if bands == 1 {
        return Ok(all);
    }
    Ok(all.into_iter().skip(band).step_by(bands).collect())
}

/// Semantic version of this crate, exposed for runtime feature detection.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn err_json(msg: &str) -> String {
    format!("{{\"ok\":false,\"error\":\"{}\"}}", msg.replace('"', "'"))
}

fn stats_json(data: &[u8]) -> String {
    let gt = match GeoTiff::from_bytes(data) {
        Ok(g) => g,
        Err(e) => return err_json(&format!("decode: {e}")),
    };
    let (w, h, bands) = (gt.width(), gt.height(), gt.band_count());
    let epsg = gt.epsg().map(|e| e.to_string()).unwrap_or_else(|| "null".into());
    // read_all_f64 converts any sample format (u8/i16/f32/...) to f64.
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
