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

/// Decode a GeoTIFF and return only its georeferencing/shape metadata as JSON:
/// `{"ok":true,"width":W,"height":H,"bands":B,"epsg":E|null,"nodata":V|null}`.
#[wasm_bindgen]
pub fn geotiff_info(data: &[u8]) -> String {
    match GeoTiff::from_bytes(data) {
        Ok(gt) => {
            let epsg = gt.epsg().map(|e| e.to_string()).unwrap_or_else(|| "null".into());
            let nodata = json_opt_f64(gt.no_data());
            format!(
                "{{\"ok\":true,\"width\":{},\"height\":{},\"bands\":{},\"epsg\":{},\"nodata\":{}}}",
                gt.width(), gt.height(), gt.band_count(), epsg, nodata
            )
        }
        Err(e) => err_json(&format!("decode: {e}")),
    }
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
