//! Swiss Oblique Mercator (EPSG method 9815 with azimuth 90°, PROJ `somerc`).
//!
//! The ellipsoid is first mapped conformally onto a Gauss sphere tangent at the
//! projection centre, and the sphere is then projected with an oblique
//! Mercator whose central line runs east–west through the centre. This is the
//! projection of the Swiss grids CH1903 / LV03 (EPSG:21781) and
//! CH1903+ / LV95 (EPSG:2056). It follows the formulation used by PROJ.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

use super::{ProjectionImpl, ProjectionParams};

const FORTPI: f64 = std::f64::consts::FRAC_PI_4;
const HALFPI: f64 = std::f64::consts::FRAC_PI_2;

pub(super) struct SwissObliqueMercatorProj {
    lon0: f64,
    a: f64,
    e: f64,
    hlf_e: f64,
    rone_es: f64,
    c: f64,
    k: f64,
    kr: f64,
    sinp0: f64,
    cosp0: f64,
    fe: f64,
    fn_: f64,
}

fn aasin(v: f64) -> f64 {
    v.clamp(-1.0, 1.0).asin()
}

impl SwissObliqueMercatorProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        if p.scale <= 0.0 {
            return Err(ProjectionError::invalid_param("scale", "must be positive"));
        }
        let phi0 = to_radians(p.lat0);
        let e = p.ellipsoid.e;
        let es = e * e;
        let one_es = 1.0 - es;
        let rone_es = 1.0 / one_es;
        let hlf_e = 0.5 * e;

        let cp = phi0.cos() * phi0.cos();
        let c = (1.0 + es * cp * cp * rone_es).sqrt();
        let sp = phi0.sin();
        let sinp0 = sp / c;
        let phip0 = aasin(sinp0);
        let cosp0 = phip0.cos();
        let esp = e * sp;
        let k = (FORTPI + 0.5 * phip0).tan().ln()
            - c * ((FORTPI + 0.5 * phi0).tan().ln() - hlf_e * ((1.0 + esp) / (1.0 - esp)).ln());
        let kr = p.scale * one_es.sqrt() / (1.0 - esp * esp);

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            e,
            hlf_e,
            rone_es,
            c,
            k,
            kr,
            sinp0,
            cosp0,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for SwissObliqueMercatorProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let phi = to_radians(lat_deg);
        let lam = to_radians(lon_deg) - self.lon0;

        let sp = self.e * phi.sin();
        let phip = 2.0
            * (self.c
                * ((FORTPI + 0.5 * phi).tan().ln() - self.hlf_e * ((1.0 + sp) / (1.0 - sp)).ln())
                + self.k)
                .exp()
                .atan()
            - HALFPI;
        let lamp = self.c * lam;
        let cp = phip.cos();
        let phipp = aasin(self.cosp0 * phip.sin() - self.sinp0 * cp * lamp.cos());
        let cos_phipp = phipp.cos();
        if cos_phipp.abs() < 1e-12 {
            return Err(ProjectionError::out_of_bounds(
                "point reaches the Swiss Oblique Mercator singularity",
            ));
        }
        let lampp = aasin(cp * lamp.sin() / cos_phipp);

        let x = self.a * self.kr * lampp;
        let y = self.a * self.kr * (FORTPI + 0.5 * phipp).tan().ln();
        Ok((x + self.fe, y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = (x - self.fe) / self.a;
        let y = (y - self.fn_) / self.a;

        let phipp = 2.0 * ((y / self.kr).exp().atan() - FORTPI);
        let lampp = x / self.kr;
        let cp = phipp.cos();
        let phip = aasin(self.cosp0 * phipp.sin() + self.sinp0 * cp * lampp.cos());
        let lamp = aasin(cp * lampp.sin() / phip.cos());
        let con = (self.k - (FORTPI + 0.5 * phip).tan().ln()) / self.c;

        let mut phi = phip;
        let mut converged = false;
        for _ in 0..20 {
            let esp = self.e * phi.sin();
            let delp = (con + (FORTPI + 0.5 * phi).tan().ln()
                - self.hlf_e * ((1.0 + esp) / (1.0 - esp)).ln())
                * (1.0 - esp * esp)
                * phi.cos()
                * self.rone_es;
            phi -= delp;
            if delp.abs() < 1e-12 {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(ProjectionError::out_of_bounds(
                "Swiss Oblique Mercator inverse did not converge",
            ));
        }
        let lam = lamp / self.c + self.lon0;
        Ok((to_degrees(lam), to_degrees(phi)))
    }
}

#[cfg(test)]
mod tests {
    use crate::ellipsoid::Ellipsoid;
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    fn lv95() -> Projection {
        Projection::new(
            ProjectionParams::new(ProjectionKind::SwissObliqueMercator)
                .with_lat0(46.952_405_555_555_6)
                .with_lon0(7.439_583_333_333_33)
                .with_scale(1.0)
                .with_false_easting(2_600_000.0)
                .with_false_northing(1_200_000.0)
                .with_ellipsoid(Ellipsoid::BESSEL),
        )
        .unwrap()
    }

    /// Reference values from PROJ 9:
    /// `+proj=somerc +lat_0=46.9524055555556 +lon_0=7.43958333333333 +k_0=1
    /// +x_0=2600000 +y_0=1200000 +ellps=bessel` (inverse, Bessel geographic).
    #[test]
    fn matches_proj_somerc() {
        let p = lv95();
        for (x, y, lon, lat) in [
            (2_705_000.0, 1_115_000.0, 8.799_578_72, 46.179_502_24),
            (2_800_000.0, 1_250_000.0, 10.088_401_77, 47.371_803_95),
            (2_600_000.0, 1_200_000.0, 7.439_583_333, 46.952_405_556),
        ] {
            let (lon2, lat2) = p.inverse(x, y).unwrap();
            assert!((lon2 - lon).abs() < 1e-8, "lon {lon2} vs {lon}");
            assert!((lat2 - lat).abs() < 1e-8, "lat {lat2} vs {lat}");
            let (x2, y2) = p.forward(lon, lat).unwrap();
            assert!((x2 - x).abs() < 1e-3, "x {x2} vs {x}");
            assert!((y2 - y).abs() < 1e-3, "y {y2} vs {y}");
        }
    }
}
