use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub mod tool_args_ext;
pub use tool_args_ext::{
    parse_optional_output_path,
    parse_raster_path_arg,
    parse_raster_path_value,
    parse_vector_path_arg,
    parse_vector_path_value,
    IMPLICIT_MEMORY_VECTOR_OUTPUT_PATH,
};

pub type ToolArgs = BTreeMap<String, Value>;
pub type ToolId = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseTier {
    #[serde(alias = "Open")]
    #[serde(alias = "open")]
    Open,
    #[serde(alias = "Pro")]
    #[serde(alias = "pro")]
    Pro,
    #[serde(alias = "Enterprise")]
    #[serde(alias = "enterprise")]
    Enterprise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    Raster,
    Vector,
    Lidar,
    Topology,
    Hydrology,
    Terrain,
    Conversion,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParamSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

impl Default for ToolParamSpec {
    fn default() -> Self {
        Self {
            name: "",
            description: "",
            required: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolValueCardinality {
    Single,
    Multiple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolVectorGeometry {
    Point,
    Line,
    Polygon,
    LineOrPolygon,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolScalarKind {
    Integer,
    Float,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolDatasetSchema {
    Raster,
    Vector { geometry: ToolVectorGeometry },
    Lidar,
    Table,
    Json,
    Text,
    File,
    Mixed { members: Vec<ToolDatasetSchema> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInputMode {
    Existing,
    ExistingOrNumber,
    ExistingOrString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputMode {
    New,
    Report,
    Sidecar,
    InPlace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInputSchema {
    pub mode: ToolInputMode,
    pub dataset: ToolDatasetSchema,
    pub cardinality: ToolValueCardinality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutputSchema {
    pub mode: ToolOutputMode,
    pub dataset: ToolDatasetSchema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnumOption {
    pub value: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnumSchema {
    pub options: Vec<ToolEnumOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFieldSchema {
    pub parent: String,
    pub geometry: Option<ToolVectorGeometry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolParamSchema {
    Input(ToolInputSchema),
    Output(ToolOutputSchema),
    Scalar { scalar: ToolScalarKind },
    Enum(ToolEnumSchema),
    Bool,
    String,
    Field(ToolFieldSchema),
}

impl ToolParamSchema {
    pub fn input(dataset: ToolDatasetSchema) -> Self {
        Self::Input(ToolInputSchema {
            mode: ToolInputMode::Existing,
            dataset,
            cardinality: ToolValueCardinality::Single,
        })
    }

    pub fn input_multiple(dataset: ToolDatasetSchema) -> Self {
        Self::Input(ToolInputSchema {
            mode: ToolInputMode::Existing,
            dataset,
            cardinality: ToolValueCardinality::Multiple,
        })
    }

    pub fn input_raster() -> Self {
        Self::input(ToolDatasetSchema::Raster)
    }

    pub fn input_vector_any() -> Self {
        Self::input(ToolDatasetSchema::Vector {
            geometry: ToolVectorGeometry::Any,
        })
    }

    pub fn input_vector(geometry: ToolVectorGeometry) -> Self {
        Self::input(ToolDatasetSchema::Vector { geometry })
    }

    pub fn input_lidar() -> Self {
        Self::input(ToolDatasetSchema::Lidar)
    }

    pub fn input_existing_or_number(dataset: ToolDatasetSchema) -> Self {
        Self::Input(ToolInputSchema {
            mode: ToolInputMode::ExistingOrNumber,
            dataset,
            cardinality: ToolValueCardinality::Single,
        })
    }

    pub fn output(dataset: ToolDatasetSchema) -> Self {
        Self::Output(ToolOutputSchema {
            mode: ToolOutputMode::New,
            dataset,
        })
    }

    pub fn output_raster() -> Self {
        Self::output(ToolDatasetSchema::Raster)
    }

    pub fn output_vector_any() -> Self {
        Self::output(ToolDatasetSchema::Vector {
            geometry: ToolVectorGeometry::Any,
        })
    }

    pub fn bool() -> Self {
        Self::Bool
    }

    pub fn string() -> Self {
        Self::String
    }

    pub fn scalar_integer() -> Self {
        Self::Scalar {
            scalar: ToolScalarKind::Integer,
        }
    }

    pub fn scalar_float() -> Self {
        Self::Scalar {
            scalar: ToolScalarKind::Float,
        }
    }

    pub fn field(parent: &str, geometry: Option<ToolVectorGeometry>) -> Self {
        Self::Field(ToolFieldSchema {
            parent: parent.to_string(),
            geometry,
        })
    }

    pub fn enum_values(options: &[&str]) -> Self {
        Self::Enum(ToolEnumSchema {
            options: options
                .iter()
                .map(|value| ToolEnumOption {
                    value: (*value).to_string(),
                    label: None,
                })
                .collect(),
        })
    }

    pub fn io_role(&self) -> Option<ToolIoRole> {
        match self {
            Self::Input(_) => Some(ToolIoRole::Input),
            Self::Output(_) => Some(ToolIoRole::Output),
            Self::Scalar { .. } | Self::Enum(_) | Self::Bool | Self::String | Self::Field(_) => None,
        }
    }

    pub fn coarse_data_kind(&self) -> ToolDataKind {
        match self {
            Self::Input(schema) => schema.dataset.coarse_data_kind(),
            Self::Output(schema) => schema.dataset.coarse_data_kind(),
            Self::Scalar { .. } => ToolDataKind::Number,
            Self::Enum(_) | Self::String | Self::Field(_) => ToolDataKind::String,
            Self::Bool => ToolDataKind::Bool,
        }
    }
}

impl ToolDatasetSchema {
    pub fn coarse_data_kind(&self) -> ToolDataKind {
        match self {
            Self::Raster => ToolDataKind::Raster,
            Self::Vector { .. } => ToolDataKind::Vector,
            Self::Lidar => ToolDataKind::Lidar,
            Self::Table => ToolDataKind::Table,
            Self::Json => ToolDataKind::Json,
            Self::Text => ToolDataKind::Text,
            Self::File => ToolDataKind::File,
            Self::Mixed { members } => members
                .first()
                .map(ToolDatasetSchema::coarse_data_kind)
                .unwrap_or(ToolDataKind::Unknown),
        }
    }
}

pub fn param_schema_map(entries: &[(&str, ToolParamSchema)]) -> BTreeMap<String, ToolParamSchema> {
    entries
        .iter()
        .map(|(name, schema)| ((*name).to_string(), schema.clone()))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub id: ToolId,
    pub display_name: &'static str,
    pub summary: &'static str,
    pub category: ToolCategory,
    pub license_tier: LicenseTier,
    pub params: Vec<ToolParamSpec>,
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("license denied: {0}")]
    LicenseDenied(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("execution error: {0}")]
    Execution(String),
}

pub trait CapabilityProvider: Send + Sync {
    fn has_tool_access(&self, tool_id: ToolId, tier: LicenseTier) -> bool;
}

pub trait ProgressSink: Send + Sync {
    fn info(&self, _msg: &str) {}
    fn progress(&self, _pct: f64) {}
}

/// Coalesces progress into integer-percent buckets and emits each bucket at most once.
///
/// This is designed for parallel loops: workers can call `emit_unit_fraction` frequently,
/// while actual callback emissions remain bounded and monotonic.
pub struct PercentCoalescer {
    min_bucket: usize,
    max_bucket: usize,
    next_bucket: AtomicUsize,
}

impl PercentCoalescer {
    pub fn new(min_bucket: usize, max_bucket: usize) -> Self {
        assert!(min_bucket <= max_bucket, "min_bucket must be <= max_bucket");
        assert!(max_bucket <= 100, "max_bucket must be <= 100");
        Self {
            min_bucket,
            max_bucket,
            next_bucket: AtomicUsize::new(min_bucket),
        }
    }

    pub fn emit_unit_fraction(&self, sink: &dyn ProgressSink, fraction01: f64) {
        let clamped = fraction01.clamp(0.0, 1.0);
        let span = self.max_bucket.saturating_sub(self.min_bucket);
        let target = self.min_bucket + ((clamped * span as f64).floor() as usize);
        self.emit_to_bucket(sink, target);
    }

    pub fn finish(&self, sink: &dyn ProgressSink) {
        self.emit_to_bucket(sink, self.max_bucket);
    }

    fn emit_to_bucket(&self, sink: &dyn ProgressSink, mut target: usize) {
        if target > self.max_bucket {
            target = self.max_bucket;
        }

        loop {
            let next = self.next_bucket.load(Ordering::Relaxed);
            if next > target || next > self.max_bucket {
                break;
            }

            if self
                .next_bucket
                .compare_exchange(next, next + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                sink.progress((next as f64) / 100.0);
            }
        }
    }
}

pub struct ToolContext<'a> {
    pub progress: &'a dyn ProgressSink,
    pub capabilities: &'a dyn CapabilityProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    Info(String),
    Percent(f64),
}

#[derive(Default)]
pub struct RecordingProgressSink {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingProgressSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn take_events(self) -> Vec<ProgressEvent> {
        self.events.into_inner().unwrap_or_else(|_| Vec::new())
    }
}

impl ProgressSink for RecordingProgressSink {
    fn info(&self, msg: &str) {
        if let Ok(mut events) = self.events.lock() {
            events.push(ProgressEvent::Info(msg.to_string()));
        }
    }

    fn progress(&self, pct: f64) {
        if let Ok(mut events) = self.events.lock() {
            events.push(ProgressEvent::Percent(pct.clamp(0.0, 1.0)));
        }
    }
}

struct TeeProgressSink<'a> {
    external: &'a dyn ProgressSink,
    recorder: &'a RecordingProgressSink,
}

impl ProgressSink for TeeProgressSink<'_> {
    fn info(&self, msg: &str) {
        self.external.info(msg);
        self.recorder.info(msg);
    }

    fn progress(&self, pct: f64) {
        self.external.progress(pct);
        self.recorder.progress(pct);
    }
}

struct NullProgressSink;

impl ProgressSink for NullProgressSink {}

pub struct AllowAllCapabilities;

impl CapabilityProvider for AllowAllCapabilities {
    fn has_tool_access(&self, _tool_id: ToolId, _tier: LicenseTier) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaxTierCapabilities {
    pub max_tier: LicenseTier,
}

impl CapabilityProvider for MaxTierCapabilities {
    fn has_tool_access(&self, _tool_id: ToolId, tier: LicenseTier) -> bool {
        tier <= self.max_tier
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolIoRole {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDataKind {
    Raster,
    Vector,
    Lidar,
    Table,
    Json,
    Text,
    File,
    Bool,
    Number,
    String,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParamDescriptor {
    pub name: String,
    pub description: String,
    pub required: bool,
}

impl Default for ToolParamDescriptor {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            required: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub category: ToolCategory,
    pub license_tier: LicenseTier,
    pub params: Vec<ToolParamDescriptor>,
}

impl From<ToolParamSpec> for ToolParamDescriptor {
    fn from(p: ToolParamSpec) -> Self {
        Self {
            name: p.name.to_string(),
            description: p.description.to_string(),
            required: p.required,
        }
    }
}

impl From<&ToolParamSpec> for ToolParamDescriptor {
    fn from(p: &ToolParamSpec) -> Self {
        Self {
            name: p.name.to_string(),
            description: p.description.to_string(),
            required: p.required,
        }
    }
}

fn looks_like_output_param(name: &str, description: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    let d = description.trim().to_ascii_lowercase();

    if matches!(
        n.as_str(),
        "output" | "out" | "output_file" | "output_path" | "destination" | "dst"
    ) {
        return true;
    }
    if n.starts_with("output_") || n.starts_with("out_") || n.starts_with("destination_") {
        return true;
    }

    let persist_markers = [
        "output file",
        "output path",
        "destination file",
        "destination path",
        "save to",
        "write to",
        "report file",
    ];
    persist_markers.iter().any(|m| d.contains(m))
}

/// True when `word` appears in `haystack` as a whole token, i.e. not surrounded
/// by other alphanumeric characters (an optional trailing plural `s` is allowed,
/// so "raster" also matches "rasters"). `haystack` and `word` are lowercase.
///
/// This is the crucial distinction from a plain substring `contains`: a join
/// `strategy` described as "first, last, count" must NOT be read as LiDAR just
/// because "last" contains the substring "las". Treating `_`, `.`, spaces and
/// punctuation as boundaries keeps "raster_input" and "(dem)" matchable while
/// rejecting "last"/"class"/"atlas" and "texture"/"context".
fn has_word(haystack: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let is_alnum = |b: u8| b.is_ascii_alphanumeric();
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(word) {
        let start = from + rel;
        let mut end = start + word.len();
        let before_ok = start == 0 || !is_alnum(bytes[start - 1]);
        // Accept a single plural 's' before requiring the right-hand boundary.
        if end < bytes.len() && bytes[end] == b's' {
            end += 1;
        }
        let after_ok = end == bytes.len() || !is_alnum(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Whether a parameter holds a free-text expression, statement, or formula that
/// the user types (an attribute query, a field-calculator formula, a LiDAR
/// filter clause). Matched on the parameter name, which is unambiguous here.
///
/// These must be caught before the flag and dataset heuristics: their
/// descriptions routinely mention "boolean" (the value the expression evaluates
/// to) or "feature"/"raster" (the data it runs against), which `looks_bool` and
/// `infer_data_kind` would otherwise misread as a checkbox or a dataset input.
fn looks_like_expression(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    has_word(&n, "expression") || has_word(&n, "statement")
}

/// Whether a parameter is a boolean flag, recognized from the conventional
/// phrasings whitebox uses ("If true, …", "(default false)", "true/false").
fn looks_bool(description: &str) -> bool {
    let d = description.trim().to_ascii_lowercase();
    d.starts_with("if true")
        || d.starts_with("if set")
        || d.starts_with("when true")
        || d.starts_with("whether ")
        || d.contains("(default true")
        || d.contains("(default false")
        || d.contains("true/false")
        || has_word(&d, "boolean")
}

fn has_any_word(haystack: &str, words: &[&str]) -> bool {
    words.iter().any(|w| has_word(haystack, w))
}

/// Detects a "(default <number>)" hint, the most reliable signal that a textual
/// parameter actually carries a number (e.g. "Buffer distance (default 10.0)").
/// Only the first token after "default" is considered, so a categorical
/// "(default 'mean')" with an unrelated number elsewhere is not misread.
fn has_default_number(description: &str) -> bool {
    let d = description.to_ascii_lowercase();
    let Some(idx) = d.find("default") else {
        return false;
    };
    let tail = &d[idx + "default".len()..];
    for tok in tail.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-')) {
        if tok.is_empty() {
            continue;
        }
        return tok.parse::<f64>().is_ok();
    }
    false
}

/// Whether a non-dataset parameter is best modeled as a number. Conservative: a
/// "(default <number>)" hint always qualifies; otherwise a numeric noun must be
/// present and no string-valued noun (which would mark free text like a field
/// name or an expression). Only consulted after dataset keywords are ruled out.
fn looks_numeric(text: &str) -> bool {
    if has_default_number(text) {
        return true;
    }
    const NUMERIC_NOUNS: &[&str] = &[
        "distance", "radius", "threshold", "tolerance", "size", "count", "factor", "weight",
        "iterations", "iteration", "sigma", "epsg", "zoom", "height", "width", "depth", "interval",
        "angle", "azimuth", "altitude", "percentile", "resolution", "exponent", "power", "spacing",
        "bins", "cellsize", "zfactor", "minutes", "seconds", "degrees", "percent", "number",
    ];
    const STRING_NOUNS: &[&str] = &[
        "name", "prefix", "suffix", "label", "expression", "statement", "wkt", "field", "palette",
        "format", "colour", "color",
    ];
    has_any_word(text, NUMERIC_NOUNS) && !has_any_word(text, STRING_NOUNS)
}

/// Pulls a small enumeration of allowed values out of a parameter description,
/// for whitebox tools that only spell their choices out in prose, e.g.
///   "Spatial predicate: intersects, within, contains, touches."
///   "Filter type: 'mean', 'median', or 'gaussian'."
/// Returns the option list (deduped, in order) or `None` when nothing
/// enumerable is found. Conservative on purpose, mirroring the demo UI's own
/// recovery: a wrong guess turns a free-form field into a too-narrow dropdown.
///
/// A column list ("CSV with columns: a, b, c") looks identical but names a file,
/// not a choice, so any mention of CSV/column short-circuits to `None`.
fn infer_enum_options(description: &str) -> Option<Vec<String>> {
    let d = description.to_ascii_lowercase();
    if d.is_empty() || has_word(&d, "csv") || d.contains("column") {
        return None;
    }

    let is_ident = |s: &str| {
        let mut chars = s.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    let push_unique = |acc: &mut Vec<String>, v: String| {
        if !acc.contains(&v) {
            acc.push(v);
        }
    };

    // Prefer single-quoted identifiers: 'mean', 'median', ...
    let mut quoted = Vec::new();
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if let Some(rel) = d[i + 1..].find('\'') {
                let inner = &d[i + 1..i + 1 + rel];
                if is_ident(inner) {
                    push_unique(&mut quoted, inner.to_string());
                }
                i = i + 1 + rel + 1;
                continue;
            }
        }
        i += 1;
    }
    if quoted.len() >= 2 {
        return Some(quoted);
    }

    // Else a colon-introduced comma list of >= 3 bare identifiers, so we don't
    // mistake an "x, y" coordinate hint for an enumeration.
    let after = d.split_once(':').map(|(_, rest)| rest)?;
    let mut items = Vec::new();
    for raw in after.split(',') {
        let mut tok = raw.trim().trim_end_matches('.').trim();
        for lead in ["or ", "and "] {
            if let Some(rest) = tok.strip_prefix(lead) {
                tok = rest.trim();
            }
        }
        if is_ident(tok) {
            push_unique(&mut items, tok.to_string());
        } else {
            // A non-identifier token ends the run (keeps the list contiguous and
            // avoids sweeping up trailing prose after the choices).
            break;
        }
    }
    (items.len() >= 3).then_some(items)
}

fn infer_data_kind(name: &str, description: &str, role: &ToolIoRole) -> ToolDataKind {
    let text = format!("{} {}", name.to_ascii_lowercase(), description.to_ascii_lowercase());

    if has_any_word(&text, &["raster", "dem", "geotiff", "grid"])
        || text.contains(".tif")
        || text.contains(".tiff")
    {
        return ToolDataKind::Raster;
    }
    // CSV/table before vector and lidar: a column-spec description ("CSV with
    // columns: feature, class, ...") lists field names that would otherwise be
    // mistaken for vector/lidar markers. The explicit format word wins.
    if has_any_word(&text, &["csv", "table"]) {
        return ToolDataKind::Table;
    }
    if has_any_word(
        &text,
        &[
            "vector",
            "feature",
            "features",
            "geopackage",
            "gpkg",
            "geojson",
            "topojson",
            "shapefile",
            "polygon",
            "polyline",
            "linestring",
            "multipoint",
            "layer",
        ],
    ) || text.contains(".shp")
    {
        return ToolDataKind::Vector;
    }
    if has_any_word(&text, &["lidar", "las", "laz", "copc", "e57", "ply", "zlidar"]) {
        return ToolDataKind::Lidar;
    }
    if has_word(&text, "json") {
        return ToolDataKind::Json;
    }
    if has_any_word(&text, &["txt", "text", "html", "xml"]) {
        return ToolDataKind::Text;
    }

    if matches!(role, ToolIoRole::Output) {
        return ToolDataKind::File;
    }
    if looks_numeric(&text) {
        return ToolDataKind::Number;
    }
    ToolDataKind::String
}

/// Synthesizes a parameter schema from its name and description for tools that
/// ship no explicit schema. Inputs may be enumerated choice lists (rendered as a
/// dropdown); everything else routes through dataset/scalar/string inference.
fn infer_param_schema(name: &str, description: &str) -> ToolParamSchema {
    if looks_like_output_param(name, description) {
        let kind = infer_data_kind(name, description, &ToolIoRole::Output);
        return schema_from_role_and_kind(Some(ToolIoRole::Output), kind)
            .unwrap_or(ToolParamSchema::String);
    }
    if looks_like_expression(name) {
        return ToolParamSchema::String;
    }
    if looks_bool(description) {
        return ToolParamSchema::bool();
    }
    if let Some(options) = infer_enum_options(description) {
        let refs: Vec<&str> = options.iter().map(String::as_str).collect();
        return ToolParamSchema::enum_values(&refs);
    }
    let kind = infer_data_kind(name, description, &ToolIoRole::Input);
    schema_from_role_and_kind(Some(ToolIoRole::Input), kind).unwrap_or(ToolParamSchema::String)
}

fn role_and_kind_from_schema(schema: &ToolParamSchema) -> (Option<ToolIoRole>, ToolDataKind) {
    (schema.io_role(), schema.coarse_data_kind())
}

fn schema_from_role_and_kind(role: Option<ToolIoRole>, kind: ToolDataKind) -> Option<ToolParamSchema> {
    let dataset_from_kind = |k: ToolDataKind| -> Option<ToolDatasetSchema> {
        match k {
            ToolDataKind::Raster => Some(ToolDatasetSchema::Raster),
            ToolDataKind::Vector => Some(ToolDatasetSchema::Vector {
                geometry: ToolVectorGeometry::Any,
            }),
            ToolDataKind::Lidar => Some(ToolDatasetSchema::Lidar),
            ToolDataKind::Table => Some(ToolDatasetSchema::Table),
            ToolDataKind::Json => Some(ToolDatasetSchema::Json),
            ToolDataKind::Text => Some(ToolDatasetSchema::Text),
            ToolDataKind::File => Some(ToolDatasetSchema::File),
            _ => None,
        }
    };

    match (role, kind) {
        (Some(ToolIoRole::Input), k) => {
            if let Some(ds) = dataset_from_kind(k.clone()) {
                Some(ToolParamSchema::input(ds))
            } else {
                match k {
                    ToolDataKind::Bool => Some(ToolParamSchema::bool()),
                    ToolDataKind::Number => Some(ToolParamSchema::scalar_float()),
                    ToolDataKind::String | ToolDataKind::Unknown => Some(ToolParamSchema::string()),
                    _ => None,
                }
            }
        }
        (Some(ToolIoRole::Output), k) => {
            if let Some(ds) = dataset_from_kind(k.clone()) {
                Some(ToolParamSchema::output(ds))
            } else {
                match k {
                    ToolDataKind::Bool => Some(ToolParamSchema::bool()),
                    ToolDataKind::Number => Some(ToolParamSchema::scalar_float()),
                    ToolDataKind::String | ToolDataKind::Unknown => Some(ToolParamSchema::string()),
                    _ => None,
                }
            }
        }
        (None, ToolDataKind::Bool) => Some(ToolParamSchema::bool()),
        (None, ToolDataKind::Number) => Some(ToolParamSchema::scalar_float()),
        (None, ToolDataKind::String)
        | (None, ToolDataKind::File)
        | (None, ToolDataKind::Text)
        | (None, ToolDataKind::Json)
        | (None, ToolDataKind::Unknown) => Some(ToolParamSchema::string()),
        (None, k) => dataset_from_kind(k).map(ToolParamSchema::input),
    }
}

fn fallback_param_description(name: &str) -> String {
    if name.trim().is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut capitalize_next = true;
    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            capitalize_next = true;
            continue;
        }

        if capitalize_next {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }

    out.trim().to_string()
}

pub fn manifest_with_param_schema_json(
    manifest: &ToolManifest,
    param_schemas: &BTreeMap<String, ToolParamSchema>,
) -> Value {
    let mut entry = serde_json::to_value(manifest).unwrap_or_else(|_| Value::Null);
    let Value::Object(obj) = &mut entry else {
        return serde_json::to_value(manifest).unwrap_or(Value::Null);
    };

    let params_val = obj
        .get("params")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));

    let mut params_in: Vec<Value> = match params_val {
        Value::Array(params) => params,
        _ => Vec::new(),
    };

    if params_in.is_empty() && !param_schemas.is_empty() {
        for name in param_schemas.keys() {
            params_in.push(serde_json::json!({
                "name": name,
                "description": fallback_param_description(name),
                "required": false
            }));
        }
    }

    let mut params_out = Vec::new();
    for p in params_in {
        let mut po = match p {
            Value::Object(v) => v,
            _ => continue,
        };

        let name = po
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let description = po
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        if description.trim().is_empty() {
            po.insert(
                "description".to_string(),
                Value::String(fallback_param_description(&name)),
            );
        }

        let explicit_schema = param_schemas.get(&name);
        // The effective schema is either the tool's explicit one or, for the many
        // whitebox tools that ship none, an inference from name + description.
        // Role and data_kind are then derived from that schema so they always
        // agree with it (an inferred enum/scalar carries no io_role, exactly like
        // an explicit one would).
        let inferred_schema;
        let schema = match explicit_schema {
            Some(s) => Some(s),
            None => {
                inferred_schema = infer_param_schema(&name, &description);
                Some(&inferred_schema)
            }
        };
        let (role, data_kind) = match schema {
            Some(s) => role_and_kind_from_schema(s),
            None => (Some(ToolIoRole::Input), ToolDataKind::String),
        };

        if let Some(schema) = schema {
            po.insert(
                "schema".to_string(),
                serde_json::to_value(schema).unwrap_or(Value::Null),
            );
        }

        if let Some(role) = role {
            po.insert(
                "io_role".to_string(),
                serde_json::to_value(role)
                    .unwrap_or_else(|_| Value::String("input".to_string())),
            );
        }

        po.insert(
            "data_kind".to_string(),
            serde_json::to_value(data_kind)
                .unwrap_or_else(|_| Value::String("unknown".to_string())),
        );

        params_out.push(Value::Object(po));
    }

    obj.insert("params".to_string(), Value::Array(params_out));
    entry
}

pub fn manifest_with_io_schema_json(manifest: &ToolManifest) -> Value {
    manifest_with_param_schema_json(manifest, &BTreeMap::new())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStability {
    Experimental,
    Beta,
    Stable,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub name: String,
    pub description: String,
    pub args: ToolArgs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub category: ToolCategory,
    pub license_tier: LicenseTier,
    pub params: Vec<ToolParamDescriptor>,
    pub defaults: ToolArgs,
    pub examples: Vec<ToolExample>,
    pub tags: Vec<String>,
    pub stability: ToolStability,
}

impl From<ToolMetadata> for ToolDescriptor {
    fn from(m: ToolMetadata) -> Self {
        let params = m.params.into_iter().map(ToolParamDescriptor::from).collect();

        Self {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            summary: m.summary.to_string(),
            category: m.category,
            license_tier: m.license_tier,
            params,
        }
    }
}

impl From<ToolManifest> for ToolDescriptor {
    fn from(m: ToolManifest) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name,
            summary: m.summary,
            category: m.category,
            license_tier: m.license_tier,
            params: m.params,
        }
    }
}

impl From<ToolMetadata> for ToolManifest {
    fn from(m: ToolMetadata) -> Self {
        let params = m.params.into_iter().map(ToolParamDescriptor::from).collect();

        Self {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            summary: m.summary.to_string(),
            category: m.category,
            license_tier: m.license_tier,
            params,
            defaults: ToolArgs::new(),
            examples: Vec::new(),
            tags: Vec::new(),
            stability: ToolStability::Stable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub tool_id: String,
    pub args: ToolArgs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub tool_id: String,
    pub outputs: BTreeMap<String, Value>,
    pub progress: Vec<ProgressEvent>,
}

pub trait ToolRuntimeRegistry: Send + Sync {
    fn list_tools(&self) -> Vec<ToolMetadata>;
    fn list_manifests(&self) -> Vec<ToolManifest> {
        self.list_tools().into_iter().map(ToolManifest::from).collect()
    }
    fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError>;
}

pub struct ToolRuntime<'a, R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub registry: &'a R,
    pub capabilities: &'a C,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeOptions {
    pub max_tier: LicenseTier,
    pub expose_locked_tools: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            max_tier: LicenseTier::Open,
            expose_locked_tools: false,
        }
    }
}

pub struct OwnedToolRuntime<R>
where
    R: ToolRuntimeRegistry,
{
    pub registry: R,
    pub options: RuntimeOptions,
    capabilities: MaxTierCapabilities,
}

pub struct OwnedToolRuntimeWithCapabilities<R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub registry: R,
    pub options: RuntimeOptions,
    capabilities: C,
}

impl<R> OwnedToolRuntime<R>
where
    R: ToolRuntimeRegistry,
{
    pub fn with_options(registry: R, options: RuntimeOptions) -> Self {
        let capabilities = MaxTierCapabilities {
            max_tier: options.max_tier,
        };
        Self {
            registry,
            options,
            capabilities,
        }
    }

    pub fn runtime(&self) -> ToolRuntime<'_, R, MaxTierCapabilities> {
        ToolRuntime {
            registry: &self.registry,
            capabilities: &self.capabilities,
        }
    }

    pub fn list_visible_manifests(&self) -> Vec<ToolManifest> {
        let manifests = self.runtime().list_manifests();
        if self.options.expose_locked_tools {
            return manifests;
        }

        let allowed_ids: BTreeSet<String> = self
            .registry
            .list_tools()
            .into_iter()
            .filter(|m| self.capabilities.has_tool_access(m.id, m.license_tier))
            .map(|m| m.id.to_string())
            .collect();

        manifests
            .into_iter()
            .filter(|m| allowed_ids.contains(&m.id))
            .collect()
    }

    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute(req)
    }

    pub fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute_with_progress_sink(req, progress)
    }
}

impl<R, C> OwnedToolRuntimeWithCapabilities<R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub fn new(registry: R, options: RuntimeOptions, capabilities: C) -> Self {
        Self {
            registry,
            options,
            capabilities,
        }
    }

    pub fn runtime(&self) -> ToolRuntime<'_, R, C> {
        ToolRuntime {
            registry: &self.registry,
            capabilities: &self.capabilities,
        }
    }

    pub fn list_visible_manifests(&self) -> Vec<ToolManifest> {
        let manifests = self.runtime().list_manifests();
        if self.options.expose_locked_tools {
            return manifests;
        }

        let allowed_ids: BTreeSet<String> = self
            .registry
            .list_tools()
            .into_iter()
            .filter(|m| self.capabilities.has_tool_access(m.id, m.license_tier))
            .map(|m| m.id.to_string())
            .collect();

        manifests
            .into_iter()
            .filter(|m| allowed_ids.contains(&m.id))
            .collect()
    }

    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute(req)
    }

    pub fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute_with_progress_sink(req, progress)
    }
}

pub struct ToolRuntimeBuilder<R>
where
    R: ToolRuntimeRegistry,
{
    registry: R,
    options: RuntimeOptions,
}

impl<R> ToolRuntimeBuilder<R>
where
    R: ToolRuntimeRegistry,
{
    pub fn new(registry: R) -> Self {
        Self {
            registry,
            options: RuntimeOptions::default(),
        }
    }

    pub fn max_tier(mut self, tier: LicenseTier) -> Self {
        self.options.max_tier = tier;
        self
    }

    pub fn expose_locked_tools(mut self, expose: bool) -> Self {
        self.options.expose_locked_tools = expose;
        self
    }

    pub fn build(self) -> OwnedToolRuntime<R> {
        OwnedToolRuntime::with_options(self.registry, self.options)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BindingTarget {
    Python,
    R,
}

pub fn generate_wrapper_stub(manifest: &ToolManifest, target: BindingTarget) -> String {
    let fn_name = manifest.id.replace('-', "_");
    match target {
        BindingTarget::Python => format!(
            "def {fn_name}(**kwargs):\n    \"\"\"{summary}\"\"\"\n    return run_tool_json('{tool_id}', kwargs)\n",
            summary = manifest.summary,
            tool_id = manifest.id,
        ),
        BindingTarget::R => format!(
            "{fn_name} <- function(...) {{\n  # {summary}\n  run_tool_json('{tool_id}', list(...))\n}}\n",
            summary = manifest.summary,
            tool_id = manifest.id,
        ),
    }
}

impl<'a, R, C> ToolRuntime<'a, R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub fn list_manifests(&self) -> Vec<ToolManifest> {
        self.registry.list_manifests()
    }

    pub fn list_descriptors(&self) -> Vec<ToolDescriptor> {
        self.registry
            .list_manifests()
            .into_iter()
            .map(ToolDescriptor::from)
            .collect()
    }

    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, ToolError> {
        let null_progress = NullProgressSink;
        self.execute_with_progress_sink(req, &null_progress)
    }

    pub fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<ExecuteResponse, ToolError> {
        if req.tool_id.trim().is_empty() {
            return Err(ToolError::InvalidRequest("tool_id cannot be empty".to_string()));
        }

        let recorded = RecordingProgressSink::new();
        let tee = TeeProgressSink {
            external: progress,
            recorder: &recorded,
        };
        let ctx = ToolContext {
            progress: &tee,
            capabilities: self.capabilities,
        };

        let result = self.registry.run_tool(&req.tool_id, &req.args, &ctx)?;
        Ok(ExecuteResponse {
            tool_id: req.tool_id,
            outputs: result.outputs,
            progress: recorded.take_events(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolRunResult {
    pub outputs: BTreeMap<String, Value>,
}

pub trait Tool: Send + Sync {
    fn metadata(&self) -> ToolMetadata;
    fn manifest(&self) -> ToolManifest {
        ToolManifest::from(self.metadata())
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError>;
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct DemoRegistry;

    impl ToolRuntimeRegistry for DemoRegistry {
        fn list_tools(&self) -> Vec<ToolMetadata> {
            vec![ToolMetadata {
                id: "demo_add",
                display_name: "Demo Add",
                summary: "Adds a constant to each value",
                category: ToolCategory::Raster,
                license_tier: LicenseTier::Open,
                params: vec![
                    ToolParamSpec {
                        name: "input",
                        description: "Input values",
                        required: true,
                            ..Default::default()
                    },
                    ToolParamSpec {
                        name: "constant",
                        description: "Added value",
                        required: true,
                            ..Default::default()
                    },
                ],
            }]
        }

        fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
            if id != "demo_add" {
                return Err(ToolError::NotFound(id.to_string()));
            }
            if !ctx.capabilities.has_tool_access("demo_add", LicenseTier::Open) {
                return Err(ToolError::LicenseDenied("demo_add".to_string()));
            }

            let input = args
                .get("input")
                .and_then(Value::as_array)
                .ok_or_else(|| ToolError::Validation("missing input".to_string()))?;
            let c = args
                .get("constant")
                .and_then(Value::as_f64)
                .ok_or_else(|| ToolError::Validation("missing constant".to_string()))?;

            ctx.progress.info("running demo_add");
            let mut out = Vec::with_capacity(input.len());
            for (i, v) in input.iter().enumerate() {
                let n = v
                    .as_f64()
                    .ok_or_else(|| ToolError::Validation("non-numeric input".to_string()))?;
                out.push(n + c);
                ctx.progress.progress((i + 1) as f64 / input.len().max(1) as f64);
            }

            let mut outputs = BTreeMap::new();
            outputs.insert("result".to_string(), json!(out));
            Ok(ToolRunResult { outputs })
        }
    }

    #[test]
    fn max_tier_capabilities_respect_ordering() {
        let caps = MaxTierCapabilities {
            max_tier: LicenseTier::Open,
        };
        assert!(caps.has_tool_access("x", LicenseTier::Open));
        assert!(!caps.has_tool_access("x", LicenseTier::Pro));
    }

    #[test]
    fn runtime_execute_captures_outputs_and_progress() {
        let runtime = ToolRuntime {
            registry: &DemoRegistry,
            capabilities: &AllowAllCapabilities,
        };

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!([1.0, 2.0]));
        args.insert("constant".to_string(), json!(3.0));

        let response = runtime
            .execute(ExecuteRequest {
                tool_id: "demo_add".to_string(),
                args,
            })
            .expect("execution should succeed");

        assert_eq!(response.tool_id, "demo_add");
        assert_eq!(response.outputs.get("result"), Some(&json!([4.0, 5.0])));
        assert!(response
            .progress
            .iter()
            .any(|e| matches!(e, ProgressEvent::Info(msg) if msg == "running demo_add")));
    }

    #[test]
    fn list_descriptors_returns_owned_metadata() {
        let runtime = ToolRuntime {
            registry: &DemoRegistry,
            capabilities: &AllowAllCapabilities,
        };

        let list = runtime.list_descriptors();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "demo_add");
        assert_eq!(list[0].params.len(), 2);
    }

    #[test]
    fn list_manifests_contains_default_stability() {
        let runtime = ToolRuntime {
            registry: &DemoRegistry,
            capabilities: &AllowAllCapabilities,
        };

        let manifests = runtime.list_manifests();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, "demo_add");
        assert_eq!(manifests[0].stability, ToolStability::Stable);
    }

    #[test]
    fn owned_runtime_filters_locked_tools() {
        let registry = DemoRegistry;
        let runtime = ToolRuntimeBuilder::new(registry)
            .max_tier(LicenseTier::Open)
            .build();

        let manifests = runtime.list_visible_manifests();
        assert_eq!(manifests.len(), 1);
    }

    #[test]
    fn wrapper_stub_generation_produces_expected_prefix() {
        let manifest = ToolManifest {
            id: "demo_add".to_string(),
            display_name: "Demo Add".to_string(),
            summary: "Adds values".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: Vec::new(),
            defaults: ToolArgs::new(),
            examples: Vec::new(),
            tags: Vec::new(),
            stability: ToolStability::Stable,
        };

        let py = generate_wrapper_stub(&manifest, BindingTarget::Python);
        let r = generate_wrapper_stub(&manifest, BindingTarget::R);
        assert!(py.starts_with("def demo_add"));
        assert!(r.starts_with("demo_add <- function"));
    }

    #[test]
    fn percent_coalescer_emits_each_bucket_once() {
        let sink = RecordingProgressSink::new();
        let c = PercentCoalescer::new(1, 5);

        c.emit_unit_fraction(&sink, 0.0);
        c.emit_unit_fraction(&sink, 0.4);
        c.emit_unit_fraction(&sink, 1.0);
        c.finish(&sink);

        let events = sink.take_events();
        let percents: Vec<f64> = events
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::Percent(p) => Some(p),
                _ => None,
            })
            .collect();

        assert_eq!(percents, vec![0.01, 0.02, 0.03, 0.04, 0.05]);
    }

    // ── Manifest parameter inference ─────────────────────────────────────────

    fn schema_for(name: &str, description: &str) -> ToolParamSchema {
        infer_param_schema(name, description)
    }

    #[test]
    fn has_word_respects_boundaries() {
        assert!(has_word("first, last, count", "last"));
        assert!(!has_word("first, last, count", "las")); // the original bug
        assert!(!has_word("classification", "las"));
        assert!(!has_word("texture metrics", "text"));
        assert!(has_word("(dem)", "dem"));
        assert!(!has_word("tandem mass", "dem"));
        assert!(has_word("raster_input layer", "raster"));
    }

    #[test]
    fn enum_lists_are_not_misread_as_lidar() {
        // "Transfer strategy: first, last, count, sum, mean, min, max." used to
        // become a LiDAR input because "last" contains "las".
        let s = schema_for("strategy", "Transfer strategy: first, last, count, sum, mean, min, max.");
        assert!(s.io_role().is_none(), "enums are neither input nor output");
        match &s {
            ToolParamSchema::Enum(e) => {
                let vals: Vec<_> = e.options.iter().map(|o| o.value.as_str()).collect();
                assert_eq!(vals, ["first", "last", "count", "sum", "mean", "min", "max"]);
            }
            other => panic!("expected enum, got {other:?}"),
        }
    }

    #[test]
    fn quoted_choices_become_enums() {
        let s = schema_for("filter", "Filter type: 'mean', 'median', or 'gaussian'.");
        let ToolParamSchema::Enum(e) = s else {
            panic!("expected enum");
        };
        let vals: Vec<_> = e.options.iter().map(|o| o.value.as_str()).collect();
        assert_eq!(vals, ["mean", "median", "gaussian"]);
    }

    #[test]
    fn returns_filter_choice_list_is_enum() {
        // lidar_*.returns: "Returns filter: all, first, or last." Despite "lidar"
        // appearing in the tool name, the param description is a choice list.
        let s = schema_for("returns", "Returns filter: all, first, or last.");
        assert!(matches!(s, ToolParamSchema::Enum(_)), "got {s:?}");
    }

    #[test]
    fn csv_column_lists_are_files_not_enums() {
        // These read like an enum ("a, b, c") but name a CSV file's columns.
        for desc in [
            "Optional CSV defining time-dependent edge costs (columns: edge_id, dow, start_minute, value).",
            "Rules CSV with columns: feature, op, value, class.",
        ] {
            let s = schema_for("rules", desc);
            assert!(
                matches!(
                    s,
                    ToolParamSchema::Input(ToolInputSchema { dataset: ToolDatasetSchema::Table, .. })
                ),
                "expected a table file for {desc:?}, got {s:?}"
            );
        }
    }

    #[test]
    fn overlay_layers_are_vector_inputs_not_text() {
        // clip/intersect/erase ".overlay" = "Overlay polygon layer."
        let s = schema_for("overlay", "Overlay polygon layer.");
        assert!(
            matches!(
                s,
                ToolParamSchema::Input(ToolInputSchema { dataset: ToolDatasetSchema::Vector { .. }, .. })
            ),
            "got {s:?}"
        );
    }

    #[test]
    fn numeric_params_become_scalars() {
        for (n, d) in [
            ("distance", "Buffer distance in map units."),
            ("sigma", "Standard deviation of the Gaussian smoothing kernel (default 1.0)."),
            ("n_trees", "Number of trees (default 200)."),
            ("radius", "Neighbourhood radius."),
        ] {
            assert!(
                matches!(schema_for(n, d), ToolParamSchema::Scalar { .. }),
                "{n}: {d:?} -> {:?}",
                schema_for(n, d)
            );
        }
    }

    #[test]
    fn string_and_field_params_stay_strings() {
        // Numeric-sounding but genuinely textual: must not become scalars.
        assert!(matches!(schema_for("prefix", "Prefix for joined field names."), ToolParamSchema::String));
        assert!(matches!(schema_for("statement", "Conditional expression evaluated per cell."), ToolParamSchema::String));
        // No numeric noun at all.
        assert!(matches!(schema_for("note", "An arbitrary note."), ToolParamSchema::String));
    }

    #[test]
    fn expression_params_stay_strings() {
        // A free-text expression/statement is a string the user types, even when
        // its description mentions "boolean" (the value it evaluates to) or
        // "feature"/"raster" (the data it runs over), which would otherwise be
        // read as a bool flag or a dataset input. Regression for GeoLibre #1073.
        for (n, d) in [
            // extract_by_attribute.statement -> was a bool checkbox ("Boolean …").
            (
                "statement",
                "Boolean expression evaluated against attribute fields; accepts SQL-style AND/OR/NOT/XOR aliases.",
            ),
            // field_calculator.expression -> was a vector input ("… per feature").
            (
                "expression",
                "Expression evaluated per feature. Supports SQL-style CASE, CAST(... AS type), IS NULL/IS NOT NULL.",
            ),
            // filter_lidar.statement -> was a bool checkbox.
            (
                "statement",
                "Boolean expression, e.g. '!is_noise && class == 2' or 'NOT is_noise AND class == 2'.",
            ),
            // raster_calculator.expression -> was a raster input.
            ("expression", "Math expression with quoted raster variable names."),
        ] {
            assert!(
                matches!(schema_for(n, d), ToolParamSchema::String),
                "{n}: {d:?} -> {:?}",
                schema_for(n, d)
            );
        }
    }

    #[test]
    fn plural_dataset_words_match() {
        assert!(has_word("array of input rasters", "raster"));
        assert!(has_word("overlay polygons", "polygon"));
        assert!(!has_word("rasterize", "raster")); // not a plural, not a boundary
        // svm_classification.inputs
        assert!(matches!(
            schema_for("inputs", "Array of single-band input rasters."),
            ToolParamSchema::Input(ToolInputSchema { dataset: ToolDatasetSchema::Raster, .. })
        ));
    }

    #[test]
    fn point_layers_are_vector_inputs() {
        // closest_facility_network.incidents
        assert!(matches!(
            schema_for("incidents", "Incident/demand point layer."),
            ToolParamSchema::Input(ToolInputSchema { dataset: ToolDatasetSchema::Vector { .. }, .. })
        ));
    }

    #[test]
    fn boolean_flags_are_detected() {
        for (n, d) in [
            ("auto_reproject", "If true (default), automatically reproject stack rasters."),
            ("clip", "If true, remove misclassified training samples."),
            ("hilbert_sort", "Whether to Hilbert-sort features before writing."),
        ] {
            assert!(matches!(schema_for(n, d), ToolParamSchema::Bool), "{n}: {d:?}");
        }
    }

    #[test]
    fn datasets_and_outputs_still_infer() {
        assert!(matches!(
            schema_for("input", "Input raster (DEM)."),
            ToolParamSchema::Input(ToolInputSchema { dataset: ToolDatasetSchema::Raster, .. })
        ));
        assert!(matches!(
            schema_for("output", "Output raster path."),
            ToolParamSchema::Output(_)
        ));
        assert!(matches!(
            schema_for("points", "Input LiDAR point cloud (LAS)."),
            ToolParamSchema::Input(ToolInputSchema { dataset: ToolDatasetSchema::Lidar, .. })
        ));
    }
}
