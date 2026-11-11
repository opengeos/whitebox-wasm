//! whitebox-wasm#43: a layer whose EPSG code is missing from the built-in
//! registry failed to write GeoJSON ("invalid source EPSG ... is not in the
//! built-in registry") even when it also carried the CRS as WKT. The writer
//! now falls back to that WKT.

use wbvector::{Feature, Geometry, Layer, VectorFormat};

/// EPSG:10820, WGS 84 / Agriculture Canada Albers, as GDAL writes it (WKT1).
/// The code is not in the wbprojection registry.
const EPSG_10820_WKT: &str = r#"PROJCS["WGS 84 / Agriculture Canada Albers",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4326"]],PROJECTION["Albers_Conic_Equal_Area"],PARAMETER["latitude_of_center",40],PARAMETER["longitude_of_center",-96],PARAMETER["standard_parallel_1",44.75],PARAMETER["standard_parallel_2",55.75],PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH],AUTHORITY["EPSG","10820"]]"#;

/// Ottawa, forward-projected to EPSG:10820 with PROJ 9 (pyproj).
fn ottawa_layer() -> Layer {
    let mut layer = Layer::new("albers");
    layer.push(Feature {
        fid: 0,
        geometry: Some(Geometry::point(1_568_121.6472, 813_949.7022)),
        attributes: vec![],
    });
    layer
}

fn write_and_read_geojson(layer: &Layer, tag: &str) -> wbvector::Result<Layer> {
    let path = std::env::temp_dir().join(format!(
        "wbvector_unregistered_{tag}_{}.geojson",
        std::process::id()
    ));
    let out = wbvector::write(layer, &path, VectorFormat::GeoJson).and_then(|_| wbvector::read(&path));
    let _ = std::fs::remove_file(&path);
    out
}

#[test]
fn unregistered_epsg_code_falls_back_to_the_layer_wkt() {
    assert!(wbprojection::Crs::from_epsg(10820).is_err(), "EPSG:10820 joined the registry; pick another code");

    let mut layer = ottawa_layer();
    layer.set_crs_epsg(Some(10820));
    layer.set_crs_wkt(Some(EPSG_10820_WKT.to_string()));

    let out = write_and_read_geojson(&layer, "wkt").unwrap();
    match &out[0].geometry {
        Some(Geometry::Point(c)) => {
            assert!((c.x + 75.6972).abs() < 1.0e-6, "lon {}", c.x);
            assert!((c.y - 45.4215).abs() < 1.0e-6, "lat {}", c.y);
        }
        other => panic!("expected Point geometry, got {other:?}"),
    }
}

#[test]
fn unregistered_epsg_code_without_wkt_still_reports_the_code() {
    let mut layer = ottawa_layer();
    layer.set_crs_epsg(Some(10820));

    let err = write_and_read_geojson(&layer, "nowkt").unwrap_err().to_string();
    assert!(err.contains("invalid source EPSG 10820"), "{err}");
}

/// `tests/data/gdal_epsg_10820.gpkg` is one Ottawa point in EPSG:10820,
/// written by GDAL 3.13 (`SPATIAL_INDEX=NO`, then `VACUUM` at a 1 KiB page
/// size to keep it small). GDAL declares `gpkg_spatial_ref_sys.srs_id` as
/// `INTEGER PRIMARY KEY`, so the id is the rowid and the reader used to miss
/// the row and drop its WKT definition.
#[test]
fn geopackage_from_gdal_keeps_the_srs_definition_and_writes_geojson() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/gdal_epsg_10820.gpkg");
    let layer = wbvector::read(&path).unwrap();
    assert_eq!(layer.crs_epsg(), Some(10820));
    let wkt = layer.crs_wkt().expect("WKT definition from gpkg_spatial_ref_sys");
    assert!(wkt.contains("Agriculture Canada Albers"), "{wkt}");

    let out = write_and_read_geojson(&layer, "gpkg").unwrap();
    match &out[0].geometry {
        Some(Geometry::Point(c)) => {
            assert!((c.x + 75.6972).abs() < 1.0e-6, "lon {}", c.x);
            assert!((c.y - 45.4215).abs() < 1.0e-6, "lat {}", c.y);
        }
        other => panic!("expected Point geometry, got {other:?}"),
    }
}
