//! A raster whose CRS is only WKT (e.g. an ASCII grid's `.prj`) and matches
//! no EPSG code used to be written to GeoTIFF without any CRS. Before
//! whitebox-wasm#45 such a WKT was often given the code of a CRS with other
//! parameters instead, which moved the data; now it keeps its projection as
//! user-defined GeoKeys.

use wbraster::{CrsInfo, DataType, Raster, RasterConfig, RasterFormat};

/// NAD83 / Conus Albers (EPSG:5070) with a 5,000 km false easting.
const CONUS_ALBERS_FE: &str = r#"PROJCS["unknown",GEOGCS["GCS_unknown",DATUM["D_North_American_1983",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Albers"],PARAMETER["False_Easting",5000000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",-96.0],PARAMETER["Standard_Parallel_1",29.5],PARAMETER["Standard_Parallel_2",45.5],PARAMETER["Latitude_Of_Origin",23.0],UNIT["Meter",1.0]]"#;

#[test]
fn geotiff_writes_a_wkt_only_projection_as_user_defined_keys() {
    let cfg = RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 6_000_000.0,
        y_min: 1_500_000.0,
        cell_size: 30.0,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let mut raster = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    raster.crs = CrsInfo::from_wkt(CONUS_ALBERS_FE);
    assert_eq!(raster.crs.epsg, None, "no EPSG code has these parameters");

    let path = std::env::temp_dir().join(format!("wbraster_wkt_only_{}.tif", std::process::id()));
    let written = raster.write(&path, RasterFormat::GeoTiff);
    let read = written.and_then(|_| Raster::read(&path));
    let _ = std::fs::remove_file(&path);
    let read = read.unwrap();

    assert_eq!(read.crs.epsg, None);
    let proj = read.crs.proj4.expect("user-defined projection read back");
    for part in ["+proj=aea", "+lat_1=29.5", "+lat_2=45.5", "+lat_0=23", "+lon_0=-96", "+x_0=5000000", "+y_0=0"] {
        assert!(proj.contains(part), "{part} missing from {proj}");
    }
}
