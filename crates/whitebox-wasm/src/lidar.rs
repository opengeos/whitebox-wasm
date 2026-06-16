//! LiDAR point clouds: read LAS / LAZ / PLY from memory. Backed by the pure-Rust
//! `wblidar` engine (native LAS reader + LASzip decoder, no C deps).
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use wblidar::las::reader::LasReader;
use wblidar::laz::reader::LazReader;
use wblidar::ply::reader::PlyReader;
use wblidar::{PointReader, PointRecord};

fn jerr<E: std::fmt::Display>(ctx: &str) -> impl Fn(E) -> JsValue + '_ {
    move |e| JsValue::from_str(&format!("{ctx}: {e}"))
}

/// LiDAR formats this build can read from memory.
#[wasm_bindgen]
pub fn lidar_formats() -> String {
    "las,laz,ply".to_string()
}

/// Read a LiDAR file's metadata as JSON without loading all points where
/// possible (LAS/LAZ report count and bounds from the header):
/// `{"ok":true,"format","points","epsg"|null,"point_format"|null,
///   "bounds":[min_x,min_y,min_z,max_x,max_y,max_z]|null}`.
#[wasm_bindgen]
pub fn lidar_info(data: &[u8], format: &str) -> Result<String, JsValue> {
    let (points, epsg, pfmt, bounds) = match format.to_lowercase().as_str() {
        "las" => {
            let r = LasReader::new(Cursor::new(data)).map_err(jerr("las"))?;
            let h = r.header();
            let n = h.point_count_64.unwrap_or(h.legacy_point_count as u64);
            let b = Some([h.min_x, h.min_y, h.min_z, h.max_x, h.max_y, h.max_z]);
            (n, r.crs().and_then(|c| c.epsg), Some(format!("{:?}", h.point_data_format)), b)
        }
        "laz" => {
            let r = LazReader::new(Cursor::new(data)).map_err(jerr("laz"))?;
            let h = r.header();
            let n = h.point_count_64.unwrap_or(h.legacy_point_count as u64);
            let b = Some([h.min_x, h.min_y, h.min_z, h.max_x, h.max_y, h.max_z]);
            (n, r.crs().and_then(|c| c.epsg), Some(format!("{:?}", h.point_data_format)), b)
        }
        "ply" => {
            let mut r = PlyReader::new(Cursor::new(data)).map_err(jerr("ply"))?;
            let pts = r.read_all().map_err(jerr("ply"))?;
            let b = bounds_of(&pts);
            (pts.len() as u64, None, None, b)
        }
        other => return Err(JsValue::from_str(&format!("unsupported LiDAR format '{other}' (las, laz, ply)"))),
    };
    let epsg = epsg.map(|e| e.to_string()).unwrap_or_else(|| "null".into());
    let pfmt = pfmt.map(|s| format!("\"{s}\"")).unwrap_or_else(|| "null".into());
    let bounds = bounds
        .map(|b| format!("[{},{},{},{},{},{}]", b[0], b[1], b[2], b[3], b[4], b[5]))
        .unwrap_or_else(|| "null".into());
    Ok(format!(
        "{{\"ok\":true,\"format\":\"{}\",\"points\":{points},\"epsg\":{epsg},\"point_format\":{pfmt},\"bounds\":{bounds}}}",
        format.to_lowercase()
    ))
}

fn bounds_of(pts: &[PointRecord]) -> Option<[f64; 6]> {
    if pts.is_empty() { return None; }
    let mut b = [f64::INFINITY, f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in pts {
        b[0] = b[0].min(p.x); b[1] = b[1].min(p.y); b[2] = b[2].min(p.z);
        b[3] = b[3].max(p.x); b[4] = b[4].max(p.y); b[5] = b[5].max(p.z);
    }
    Some(b)
}

fn read_all_points(data: &[u8], format: &str) -> Result<Vec<PointRecord>, JsValue> {
    match format.to_lowercase().as_str() {
        "las" => LasReader::new(Cursor::new(data)).map_err(jerr("las"))?.read_all().map_err(jerr("las")),
        "laz" => LazReader::new(Cursor::new(data)).map_err(jerr("laz"))?.read_all().map_err(jerr("laz")),
        "ply" => PlyReader::new(Cursor::new(data)).map_err(jerr("ply"))?.read_all().map_err(jerr("ply")),
        other => Err(JsValue::from_str(&format!("unsupported LiDAR format '{other}'"))),
    }
}

/// Read all point coordinates as an interleaved `Float64Array`
/// `[x0,y0,z0, x1,y1,z1, ...]` (length `3 * point_count`).
///
/// Guarded against 32-bit memory blowup; very large clouds return a clean error
/// (read the header with `lidar_info`, or downsample on your side).
#[wasm_bindgen]
pub fn lidar_read_xyz(data: &[u8], format: &str) -> Result<Vec<f64>, JsValue> {
    let pts = read_all_points(data, format)?;
    // 3 f64 per point; cap well under the 4 GiB address space.
    if (pts.len() as u64).saturating_mul(24) > 1_000_000_000 {
        return Err(JsValue::from_str(&format!(
            "{} points too large to return as one array in 32-bit WASM; downsample first", pts.len())));
    }
    let mut out = Vec::with_capacity(pts.len() * 3);
    for p in &pts {
        out.push(p.x); out.push(p.y); out.push(p.z);
    }
    Ok(out)
}

/// Read per-point classification codes as a `Uint8Array` (length `point_count`).
#[wasm_bindgen]
pub fn lidar_read_classification(data: &[u8], format: &str) -> Result<Vec<u8>, JsValue> {
    let pts = read_all_points(data, format)?;
    Ok(pts.iter().map(|p| p.classification).collect())
}

/// Read per-point intensity as a `Uint16Array` (length `point_count`).
#[wasm_bindgen]
pub fn lidar_read_intensity(data: &[u8], format: &str) -> Result<Vec<u16>, JsValue> {
    let pts = read_all_points(data, format)?;
    Ok(pts.iter().map(|p| p.intensity).collect())
}
