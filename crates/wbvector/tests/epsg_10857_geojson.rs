//! whitebox-wasm#43: writing GeoJSON from a layer in EPSG:10857 (SIRGAS 2000 /
//! Brazil Albers) failed with "invalid source EPSG 10857 ... is not in the
//! built-in registry", which is how raster_to_vector_* outputs reached it.

use wbvector::{Feature, Geometry, Layer, VectorFormat};

#[test]
fn geojson_write_reprojects_sirgas2000_brazil_albers_to_epsg4326() {
    let path = std::env::temp_dir().join(format!(
        "wbvector_epsg_10857_{}.geojson",
        std::process::id()
    ));

    let mut layer = Layer::new("albers").with_crs_epsg(10857);
    // Viçosa (MG), forward-projected with PROJ 9.5.1 (as in the wbprojection
    // EPSG:10857 test).
    layer.push(Feature {
        fid: 0,
        geometry: Some(Geometry::point(6_153_607.9086, 8_997_042.0831)),
        attributes: vec![],
    });

    let written = wbvector::write(&layer, &path, VectorFormat::GeoJson);
    let out = written.and_then(|_| wbvector::read(&path));
    let _ = std::fs::remove_file(&path);
    let out = out.unwrap();

    match &out[0].geometry {
        Some(Geometry::Point(c)) => {
            assert!((c.x + 42.8786).abs() < 1.0e-6, "lon {}", c.x);
            assert!((c.y + 20.7546).abs() < 1.0e-6, "lat {}", c.y);
        }
        other => panic!("expected Point geometry, got {other:?}"),
    }
}
