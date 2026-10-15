//! GeoTIFF GeoKey directory parsing and construction.
//!
//! The GeoKey directory is stored in three TIFF tags:
//! - `GeoKeyDirectoryTag` (34735): array of u16 values
//! - `GeoDoubleParamsTag` (34736): array of f64 values (referenced by keys)
//! - `GeoAsciiParamsTag` (34737): concatenated ASCII strings (referenced by keys)

#![allow(dead_code)]

use super::error::{GeoTiffError, Result};

// ── GeoKey codes ─────────────────────────────────────────────────────────────

/// Well-known GeoKey codes (GeoTIFF 1.1 specification).
#[allow(non_upper_case_globals, dead_code, missing_docs)]
pub mod key {
    // Configuration keys
    pub const GTModelTypeGeoKey: u16 = 1024;
    pub const GTRasterTypeGeoKey: u16 = 1025;
    pub const GTCitationGeoKey: u16 = 1026;

    // Geographic CS parameter keys
    pub const GeographicTypeGeoKey: u16 = 2048;
    pub const GeogCitationGeoKey: u16 = 2049;
    pub const GeogGeodeticDatumGeoKey: u16 = 2050;
    pub const GeogPrimeMeridianGeoKey: u16 = 2051;
    pub const GeogLinearUnitsGeoKey: u16 = 2052;
    pub const GeogAngularUnitsGeoKey: u16 = 2054;
    pub const GeogEllipsoidGeoKey: u16 = 2056;
    pub const GeogSemiMajorAxisGeoKey: u16 = 2057;
    pub const GeogSemiMinorAxisGeoKey: u16 = 2058;
    pub const GeogInvFlatteningGeoKey: u16 = 2059;
    pub const GeogPrimeMeridianLongGeoKey: u16 = 2061;

    // Projected CS parameter keys
    pub const ProjectedCSTypeGeoKey: u16 = 3072;
    pub const PCSCitationGeoKey: u16 = 3073;
    pub const ProjectionGeoKey: u16 = 3074;
    pub const ProjCoordTransGeoKey: u16 = 3075;
    pub const ProjLinearUnitsGeoKey: u16 = 3076;
    pub const ProjStdParallel1GeoKey: u16 = 3078;
    pub const ProjStdParallel2GeoKey: u16 = 3079;
    pub const ProjNatOriginLongGeoKey: u16 = 3080;
    pub const ProjNatOriginLatGeoKey: u16 = 3081;
    pub const ProjFalseEastingGeoKey: u16 = 3082;
    pub const ProjFalseNorthingGeoKey: u16 = 3083;
    pub const ProjFalseProjOriginLongGeoKey: u16 = 3084;
    pub const ProjFalseProjOriginLatGeoKey: u16 = 3085;
    pub const ProjFalseOriginEastingGeoKey: u16 = 3086;
    pub const ProjFalseOriginNorthingGeoKey: u16 = 3087;
    pub const ProjCenterLongGeoKey: u16 = 3088;
    pub const ProjCenterLatGeoKey: u16 = 3089;
    pub const ProjScaleAtNatOriginGeoKey: u16 = 3092;
    pub const ProjAzimuthAngleGeoKey: u16 = 3094;
    pub const ProjStraightVertPoleLongGeoKey: u16 = 3095;

    // Vertical CS keys
    pub const VerticalCSTypeGeoKey: u16 = 4096;
    pub const VerticalCitationGeoKey: u16 = 4097;
    pub const VerticalDatumGeoKey: u16 = 4098;
    pub const VerticalUnitsGeoKey: u16 = 4099;
}

// ── ModelType ─────────────────────────────────────────────────────────────────

/// GTModelTypeGeoKey values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Projection coordinate system (metres / feet).
    Projected = 1,
    /// Geographic coordinate system (degrees).
    Geographic = 2,
    /// Geocentric coordinate system (rarely used).
    Geocentric = 3,
    /// User-defined or unknown.
    UserDefined = 32767,
}

impl ModelType {
    /// Parse from a GeoKey value.
    pub fn from_value(v: u16) -> Self {
        match v {
            1 => Self::Projected,
            2 => Self::Geographic,
            3 => Self::Geocentric,
            _ => Self::UserDefined,
        }
    }
}

// ── RasterType ────────────────────────────────────────────────────────────────

/// GTRasterTypeGeoKey values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterType {
    /// Pixel-is-Area (the default; pixels represent an area centred on the tiepoint).
    PixelIsArea = 1,
    /// Pixel-is-Point (the tiepoint is at the centre of the pixel).
    PixelIsPoint = 2,
}

impl RasterType {
    /// Parse from a GeoKey value.
    pub fn from_value(v: u16) -> Self {
        if v == 2 { Self::PixelIsPoint } else { Self::PixelIsArea }
    }
}

// ── GeoKeyValue ──────────────────────────────────────────────────────────────

/// The value of a single GeoKey entry.
#[derive(Debug, Clone, PartialEq)]
pub enum GeoKeyValue {
    /// A 16-bit integer value stored inline in the key directory.
    Short(u16),
    /// One or more 64-bit double values from `GeoDoubleParamsTag`.
    Doubles(Vec<f64>),
    /// A UTF-8 string from `GeoAsciiParamsTag`.
    Ascii(String),
}

// ── GeoKeyEntry ──────────────────────────────────────────────────────────────

/// A single decoded GeoKey entry.
#[derive(Debug, Clone)]
pub struct GeoKeyEntry {
    /// GeoKey code (e.g. `GTModelTypeGeoKey` = 1024).
    pub key_id: u16,
    /// Parsed value.
    pub value: GeoKeyValue,
}

// ── GeoKeyDirectory ───────────────────────────────────────────────────────────

/// Decoded GeoKey directory from a GeoTIFF file.
#[derive(Debug, Clone, Default)]
pub struct GeoKeyDirectory {
    /// GeoTIFF specification version (usually 1).
    pub version: u16,
    /// Major revision (usually 1).
    pub key_revision: u16,
    /// Minor revision (usually 0 or 1).
    pub minor_revision: u16,
    /// All decoded key entries.
    pub entries: Vec<GeoKeyEntry>,
}

impl GeoKeyDirectory {
    /// Parse the GeoKey directory from the raw tag arrays.
    ///
    /// - `dir_words`: raw u16 words from `GeoKeyDirectoryTag` (34735)
    /// - `doubles`: values from `GeoDoubleParamsTag` (34736) — may be empty
    /// - `ascii`: concatenated string from `GeoAsciiParamsTag` (34737) — may be empty
    pub fn parse(dir_words: &[u16], doubles: &[f64], ascii: &str) -> Result<Self> {
        if dir_words.len() < 4 {
            return Err(GeoTiffError::InvalidGeoKey(
                "GeoKeyDirectory must have at least 4 header words".into(),
            ));
        }

        let version = dir_words[0];
        let key_revision = dir_words[1];
        let minor_revision = dir_words[2];
        let num_keys = dir_words[3] as usize;

        if dir_words.len() < 4 + num_keys * 4 {
            return Err(GeoTiffError::InvalidGeoKey(format!(
                "Directory claims {} keys but only {} words available",
                num_keys,
                dir_words.len() - 4
            )));
        }

        let mut entries = Vec::with_capacity(num_keys);

        for i in 0..num_keys {
            let base = 4 + i * 4;
            let key_id = dir_words[base];
            let tiff_tag_location = dir_words[base + 1]; // 0 = inline, 34736, or 34737
            let count = dir_words[base + 2] as usize;
            let value_offset = dir_words[base + 3] as usize;

            let value = match tiff_tag_location {
                0 => GeoKeyValue::Short(value_offset as u16),
                34736 => {
                    // Doubles
                    if value_offset + count > doubles.len() {
                        return Err(GeoTiffError::InvalidGeoKey(format!(
                            "GeoKey {}: double offset {} + count {} out of range ({})",
                            key_id,
                            value_offset,
                            count,
                            doubles.len()
                        )));
                    }
                    GeoKeyValue::Doubles(doubles[value_offset..value_offset + count].to_vec())
                }
                34737 => {
                    // ASCII
                    let start = value_offset;
                    let end = value_offset + count;
                    if end > ascii.len() {
                        return Err(GeoTiffError::InvalidGeoKey(format!(
                            "GeoKey {}: ASCII offset {} + count {} out of range ({})",
                            key_id, value_offset, count, ascii.len()
                        )));
                    }
                    let s = &ascii[start..end];
                    // GeoAscii strings use '|' as separator and may be NUL-terminated
                    let s = s.trim_end_matches('|').trim_end_matches('\0');
                    GeoKeyValue::Ascii(s.to_owned())
                }
                other => {
                    return Err(GeoTiffError::InvalidGeoKey(format!(
                        "GeoKey {}: unknown tiff_tag_location {}",
                        key_id, other
                    )));
                }
            };

            entries.push(GeoKeyEntry { key_id, value });
        }

        Ok(Self { version, key_revision, minor_revision, entries })
    }

    // ── Convenience accessors ────────────────────────────────────────────────

    /// Look up a key and return its value, if present.
    pub fn get(&self, key_id: u16) -> Option<&GeoKeyValue> {
        self.entries.iter().find(|e| e.key_id == key_id).map(|e| &e.value)
    }

    /// Get the `GTModelTypeGeoKey` value.
    pub fn model_type(&self) -> Option<ModelType> {
        self.get(key::GTModelTypeGeoKey).and_then(|v| {
            if let GeoKeyValue::Short(n) = v { Some(ModelType::from_value(*n)) } else { None }
        })
    }

    /// Get the `GTRasterTypeGeoKey` value.
    pub fn raster_type(&self) -> Option<RasterType> {
        self.get(key::GTRasterTypeGeoKey).and_then(|v| {
            if let GeoKeyValue::Short(n) = v { Some(RasterType::from_value(*n)) } else { None }
        })
    }

    /// Return the EPSG code for the projected CRS (`ProjectedCSTypeGeoKey`),
    /// or for the geographic CRS (`GeographicTypeGeoKey`), whichever is set.
    ///
    /// A projected model whose `ProjectedCSTypeGeoKey` is user-defined (32767)
    /// or missing has no EPSG code: its `GeographicTypeGeoKey` only names the
    /// datum the projection is based on, so returning it would mislabel a grid
    /// in metres as degrees. Use [`to_proj_string`](Self::to_proj_string) for
    /// those.
    pub fn epsg(&self) -> Option<u16> {
        if let Some(GeoKeyValue::Short(n)) = self.get(key::ProjectedCSTypeGeoKey) {
            if *n != 32767 && *n != 0 { return Some(*n); }
        }
        if self.model_type() == Some(ModelType::Projected) {
            return None;
        }
        if let Some(GeoKeyValue::Short(n)) = self.get(key::GeographicTypeGeoKey) {
            if *n != 32767 { return Some(*n); }
        }
        None
    }

    /// Build a PROJ string for a *user-defined* projected CRS (ProjectedCSType =
    /// 32767, e.g. NLCD's Albers) from the projection-parameter GeoKeys. Returns
    /// `None` for CRS that carry a real projected EPSG code (use [`epsg`]) or
    /// that are not projected. Covers the common projections.
    pub fn to_proj_string(&self) -> Option<String> {
        use key::*;
        // Only for user-defined PROJECTED CRS.
        if !matches!(self.get(GTModelTypeGeoKey), Some(GeoKeyValue::Short(1))) {
            return None;
        }
        if let Some(GeoKeyValue::Short(n)) = self.get(ProjectedCSTypeGeoKey) {
            if *n != 32767 && *n != 0 { return None; }
        }
        let ct = match self.get(ProjCoordTransGeoKey) {
            Some(GeoKeyValue::Short(n)) => *n,
            _ => return None,
        };
        let d = |k: u16| -> Option<f64> {
            match self.get(k)? {
                GeoKeyValue::Doubles(v) => v.first().copied(),
                GeoKeyValue::Short(n) => Some(*n as f64),
                _ => None,
            }
        };
        let proj = match ct {
            1 => "tmerc",
            7 => "merc",
            8 => "lcc",         // Lambert Conformal Conic (2SP)
            10 => "laea",
            11 => "aea",        // Albers Equal Area
            12 => "aeqd",
            14 | 15 | 16 => "stere",
            17 => "eqc",
            20 => "mill",
            21 => "ortho",
            24 => "sinu",
            _ => return None,
        };
        let mut s = format!("+proj={proj}");
        let lat0 = d(ProjNatOriginLatGeoKey)
            .or_else(|| d(ProjCenterLatGeoKey))
            .or_else(|| d(ProjFalseProjOriginLatGeoKey));
        let lon0 = d(ProjNatOriginLongGeoKey)
            .or_else(|| d(ProjCenterLongGeoKey))
            .or_else(|| d(ProjFalseProjOriginLongGeoKey))
            .or_else(|| d(ProjStraightVertPoleLongGeoKey));
        if let Some(v) = d(ProjStdParallel1GeoKey) { s.push_str(&format!(" +lat_1={v}")); }
        if let Some(v) = d(ProjStdParallel2GeoKey) { s.push_str(&format!(" +lat_2={v}")); }
        if let Some(v) = lat0 { s.push_str(&format!(" +lat_0={v}")); }
        if let Some(v) = lon0 { s.push_str(&format!(" +lon_0={v}")); }
        if let Some(v) = d(ProjScaleAtNatOriginGeoKey) { s.push_str(&format!(" +k_0={v}")); }
        s.push_str(&format!(" +x_0={} +y_0={}",
            d(ProjFalseEastingGeoKey).or_else(|| d(ProjFalseOriginEastingGeoKey)).unwrap_or(0.0),
            d(ProjFalseNorthingGeoKey).or_else(|| d(ProjFalseOriginNorthingGeoKey)).unwrap_or(0.0)));
        match self.get(GeographicTypeGeoKey) {
            Some(GeoKeyValue::Short(4326)) => s.push_str(" +datum=WGS84"),
            Some(GeoKeyValue::Short(4269)) => s.push_str(" +datum=NAD83"),
            Some(GeoKeyValue::Short(4267)) => s.push_str(" +datum=NAD27"),
            _ => match (d(GeogSemiMajorAxisGeoKey), d(GeogInvFlatteningGeoKey)) {
                (Some(a), Some(rf)) if rf > 0.0 => s.push_str(&format!(" +a={a} +rf={rf}")),
                _ => s.push_str(" +datum=WGS84"),
            },
        }
        s.push_str(" +units=m +no_defs");
        Some(s)
    }

    /// Build the GeoKeys of a *user-defined* projected CRS from a PROJ string,
    /// the inverse of [`to_proj_string`](Self::to_proj_string). Covers the same
    /// projections. Returns `None` for a string without a supported `+proj`
    /// or with a non-metre `+units`.
    pub fn from_proj_string(proj: &str) -> Option<Self> {
        use key::*;
        let params: Vec<(&str, &str)> = proj
            .split_whitespace()
            .filter_map(|t| t.strip_prefix('+'))
            .map(|t| t.split_once('=').unwrap_or((t, "")))
            .collect();
        let get = |k: &str| params.iter().find(|(name, _)| *name == k).map(|(_, v)| *v);
        let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
        if get("units").is_some_and(|u| u != "m") {
            return None;
        }
        // No GeoKey here carries a latitude of true scale, and a value this
        // parser cannot read (e.g. PROJ's `1r` radians) must not become 0.
        const NUMERIC: [&str; 11] =
            ["lat_0", "lon_0", "lat_1", "lat_2", "k_0", "k", "x_0", "y_0", "a", "rf", "lat_ts"];
        if get("lat_ts").is_some() || NUMERIC.iter().any(|k| get(k).is_some() && num(k).is_none()) {
            return None;
        }
        let ct: u16 = match get("proj")? {
            "tmerc" => 1,
            "merc" => 7,
            "lcc" => 8,
            "laea" => 10,
            "aea" => 11,
            "aeqd" => 12,
            // Only the oblique form; polar stereographic is code 15, whose
            // keys `to_proj_string` does not read.
            "stere" if num("lat_0").is_none_or(|lat| lat.abs() < 90.0) => 14,
            "eqc" => 17,
            "mill" => 20,
            "ortho" => 21,
            "sinu" => 24,
            _ => return None,
        };
        let mut b = GeoKeyBuilder::new()
            .model_type(ModelType::Projected)
            .raster_type(RasterType::PixelIsArea)
            .short(ProjectedCSTypeGeoKey, 32767)
            .short(ProjectionGeoKey, 32767)
            .short(ProjCoordTransGeoKey, ct)
            .short(ProjLinearUnitsGeoKey, 9001);
        // An ellipsoid the keys below cannot express is rejected rather than
        // silently written as WGS84.
        if get("b").is_some() {
            return None;
        }
        match get("datum") {
            Some("WGS84") => b = b.short(GeographicTypeGeoKey, 4326),
            Some("NAD83") => b = b.short(GeographicTypeGeoKey, 4269),
            Some("NAD27") => b = b.short(GeographicTypeGeoKey, 4267),
            Some(_) => return None,
            None => {
                let (a, rf) = match (num("a"), num("rf"), get("ellps")) {
                    (Some(a), Some(rf), _) => (a, rf),
                    (None, None, Some("GRS80")) => (6_378_137.0, 298.257_222_101),
                    (None, None, Some("WGS84") | None) => (6_378_137.0, 298.257_223_563),
                    _ => return None,
                };
                b = b
                    .short(GeographicTypeGeoKey, 32767)
                    .short(GeogGeodeticDatumGeoKey, 32767)
                    .short(GeogEllipsoidGeoKey, 32767)
                    .short(GeogAngularUnitsGeoKey, 9102)
                    .double(GeogSemiMajorAxisGeoKey, a)
                    .double(GeogInvFlatteningGeoKey, rf);
            }
        }
        // Key choice follows what GDAL writes for each method: centre keys for
        // the azimuthal ones, false-origin keys for the two-parallel conics.
        let centred = matches!(ct, 10 | 12 | 21);
        let (lat_key, lon_key) = if centred {
            (ProjCenterLatGeoKey, ProjCenterLongGeoKey)
        } else if ct == 8 || ct == 11 {
            (ProjFalseProjOriginLatGeoKey, ProjFalseProjOriginLongGeoKey)
        } else {
            (ProjNatOriginLatGeoKey, ProjNatOriginLongGeoKey)
        };
        if let Some(v) = num("lat_1") { b = b.double(ProjStdParallel1GeoKey, v); }
        if let Some(v) = num("lat_2") { b = b.double(ProjStdParallel2GeoKey, v); }
        b = b.double(lat_key, num("lat_0").unwrap_or(0.0));
        b = b.double(lon_key, num("lon_0").unwrap_or(0.0));
        if let Some(v) = num("k_0").or_else(|| num("k")) {
            b = b.double(ProjScaleAtNatOriginGeoKey, v);
        }
        let (fe, fn_) = (num("x_0").unwrap_or(0.0), num("y_0").unwrap_or(0.0));
        let (fe_key, fn_key) = if ct == 8 || ct == 11 {
            (ProjFalseOriginEastingGeoKey, ProjFalseOriginNorthingGeoKey)
        } else {
            (ProjFalseEastingGeoKey, ProjFalseNorthingGeoKey)
        };
        b = b.double(fe_key, fe).double(fn_key, fn_);
        let mut gkd = b.build();
        gkd.entries.sort_by_key(|e| e.key_id);
        Some(gkd)
    }

    // ── Serialisation ────────────────────────────────────────────────────────

    /// Encode the GeoKey directory into the three raw tag arrays suitable for
    /// writing into a TIFF file.
    ///
    /// Returns `(dir_words, doubles, ascii)`.
    pub fn encode(&self) -> (Vec<u16>, Vec<f64>, String) {
        let mut dir_words: Vec<u16> = Vec::new();
        let mut doubles: Vec<f64> = Vec::new();
        let mut ascii = String::new();

        // Reserve header slot
        dir_words.extend_from_slice(&[self.version, self.key_revision, self.minor_revision, 0]);

        let mut key_count: u16 = 0;
        for entry in &self.entries {
            key_count += 1;
            match &entry.value {
                GeoKeyValue::Short(v) => {
                    dir_words.extend_from_slice(&[entry.key_id, 0, 1, *v]);
                }
                GeoKeyValue::Doubles(vals) => {
                    let offset = doubles.len() as u16;
                    let count = vals.len() as u16;
                    doubles.extend_from_slice(vals);
                    dir_words.extend_from_slice(&[entry.key_id, 34736, count, offset]);
                }
                GeoKeyValue::Ascii(s) => {
                    let offset = ascii.len() as u16;
                    // GeoAscii: append string + '|' separator
                    let padded = format!("{}|", s);
                    let count = padded.len() as u16;
                    ascii.push_str(&padded);
                    dir_words.extend_from_slice(&[entry.key_id, 34737, count, offset]);
                }
            }
        }
        // Fill in number of keys in header
        dir_words[3] = key_count;

        (dir_words, doubles, ascii)
    }
}

/// Builder for constructing a `GeoKeyDirectory` programmatically.
#[derive(Debug, Default)]
pub struct GeoKeyBuilder {
    entries: Vec<GeoKeyEntry>,
}

impl GeoKeyBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a short (u16) key.
    pub fn short(mut self, key_id: u16, value: u16) -> Self {
        self.entries.push(GeoKeyEntry { key_id, value: GeoKeyValue::Short(value) });
        self
    }

    /// Add a double key.
    pub fn double(mut self, key_id: u16, value: f64) -> Self {
        self.entries.push(GeoKeyEntry { key_id, value: GeoKeyValue::Doubles(vec![value]) });
        self
    }

    /// Add an ASCII key.
    pub fn ascii(mut self, key_id: u16, value: impl Into<String>) -> Self {
        self.entries.push(GeoKeyEntry { key_id, value: GeoKeyValue::Ascii(value.into()) });
        self
    }

    /// Set the ModelType.
    pub fn model_type(self, t: ModelType) -> Self {
        self.short(key::GTModelTypeGeoKey, t as u16)
    }

    /// Set the RasterType.
    pub fn raster_type(self, t: RasterType) -> Self {
        self.short(key::GTRasterTypeGeoKey, t as u16)
    }

    /// Set both ProjectedCSTypeGeoKey and ModelType to Projected.
    pub fn projected_epsg(self, epsg: u16) -> Self {
        self.short(key::GTModelTypeGeoKey, ModelType::Projected as u16)
            .short(key::GTRasterTypeGeoKey, RasterType::PixelIsArea as u16)
            .short(key::ProjectedCSTypeGeoKey, epsg)
    }

    /// Set both GeographicTypeGeoKey and ModelType to Geographic.
    pub fn geographic_epsg(self, epsg: u16) -> Self {
        self.short(key::GTModelTypeGeoKey, ModelType::Geographic as u16)
            .short(key::GTRasterTypeGeoKey, RasterType::PixelIsArea as u16)
            .short(key::GeographicTypeGeoKey, epsg)
    }

    /// Build the `GeoKeyDirectory`.
    pub fn build(self) -> GeoKeyDirectory {
        GeoKeyDirectory {
            version: 1,
            key_revision: 1,
            minor_revision: 0,
            entries: self.entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GeoKeys as GDAL writes ESRI:102567 (Hartebeesthoek94 Lo29, E-N).
    fn lo29_keys() -> GeoKeyDirectory {
        use key::*;
        GeoKeyBuilder::new()
            .model_type(ModelType::Projected)
            .raster_type(RasterType::PixelIsArea)
            .short(GeographicTypeGeoKey, 4148)
            .double(GeogSemiMajorAxisGeoKey, 6_378_137.0)
            .double(GeogInvFlatteningGeoKey, 298.257_223_563)
            .short(ProjectedCSTypeGeoKey, 32767)
            .short(ProjectionGeoKey, 32767)
            .short(ProjCoordTransGeoKey, 1)
            .short(ProjLinearUnitsGeoKey, 9001)
            .double(ProjNatOriginLongGeoKey, 29.0)
            .double(ProjNatOriginLatGeoKey, 0.0)
            .double(ProjFalseEastingGeoKey, 0.0)
            .double(ProjFalseNorthingGeoKey, 0.0)
            .double(ProjScaleAtNatOriginGeoKey, 1.0)
            .build()
    }

    #[test]
    fn user_defined_projected_has_no_epsg() {
        let gk = lo29_keys();
        assert_eq!(gk.epsg(), None, "the datum code 4148 must not stand in for the projection");
        let proj = gk.to_proj_string().unwrap();
        assert!(proj.starts_with("+proj=tmerc"), "{proj}");
        assert!(proj.contains("+lon_0=29"), "{proj}");
        assert!(proj.contains("+k_0=1"), "{proj}");
    }

    #[test]
    fn epsg_codes_still_resolve() {
        assert_eq!(GeoKeyBuilder::new().projected_epsg(32735).build().epsg(), Some(32735));
        assert_eq!(GeoKeyBuilder::new().geographic_epsg(4148).build().epsg(), Some(4148));
    }

    #[test]
    fn proj_string_round_trips_through_geokeys() {
        for proj in [
            "+proj=tmerc +lat_0=0 +lon_0=29 +k_0=1 +x_0=0 +y_0=0 +a=6378137 +rf=298.257223563 +units=m +no_defs",
            "+proj=aea +lat_1=29.5 +lat_2=45.5 +lat_0=23 +lon_0=-96 +x_0=0 +y_0=0 +datum=NAD83 +units=m +no_defs",
            "+proj=lcc +lat_1=33 +lat_2=45 +lat_0=39 +lon_0=-96 +x_0=500000 +y_0=0 +datum=WGS84 +units=m +no_defs",
            "+proj=laea +lat_0=52 +lon_0=10 +x_0=4321000 +y_0=3210000 +datum=WGS84 +units=m +no_defs",
        ] {
            let gk = GeoKeyDirectory::from_proj_string(proj).unwrap();
            assert_eq!(gk.epsg(), None);
            assert_eq!(gk.to_proj_string().as_deref(), Some(proj));
            let ids: Vec<u16> = gk.entries.iter().map(|e| e.key_id).collect();
            assert!(ids.windows(2).all(|w| w[0] < w[1]), "keys must be sorted: {ids:?}");
        }
    }

    #[test]
    fn from_proj_string_rejects_unsupported() {
        assert!(GeoKeyDirectory::from_proj_string("+proj=longlat +datum=WGS84").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=tmerc +lon_0=0 +units=us-ft").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=tmerc +lon_0=0 +ellps=intl").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=tmerc +lon_0=0 +a=6378137 +b=6356752").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=tmerc +lon_0=0 +datum=ED50").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=stere +lat_0=90 +lat_ts=70 +lon_0=-45").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=stere +lat_0=90 +lon_0=-45").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=stere +lat_0=52 +lon_0=5").is_some());
        assert!(GeoKeyDirectory::from_proj_string("+proj=merc +lat_ts=56.5 +datum=WGS84").is_none());
        assert!(GeoKeyDirectory::from_proj_string("+proj=stere +lat_0=1r +datum=WGS84").is_none());
    }
}
