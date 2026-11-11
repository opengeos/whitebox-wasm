//! Oblique Stereographic, the double stereographic of EPSG method 9809
//! (PROJ `sterea`).
//!
//! The ellipsoid is mapped conformally onto a Gauss sphere, which is then
//! projected stereographically about the origin. Formulas follow IOGP
//! Guidance Note 7-2, section 3.2.2.2.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

use super::{ProjectionImpl, ProjectionParams};

const HALFPI: f64 = std::f64::consts::FRAC_PI_2;

pub(super) struct ObliqueStereographicProj {
    lon0: f64,
    e: f64,
    /// Twice the radius of the conformal sphere times the scale factor.
    two_rk: f64,
    n: f64,
    c: f64,
    sin_chi0: f64,
    cos_chi0: f64,
    fe: f64,
    fn_: f64,
}

impl ObliqueStereographicProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        if p.scale <= 0.0 {
            return Err(ProjectionError::invalid_param("scale", "must be positive"));
        }
        let phi0 = to_radians(p.lat0);
        let a = p.ellipsoid.a;
        let e = p.ellipsoid.e;
        let es = e * e;
        let sin0 = phi0.sin();
        let w0 = 1.0 - es * sin0 * sin0;
        let rho0 = a * (1.0 - es) / w0.powf(1.5);
        let nu0 = a / w0.sqrt();
        let r = (rho0 * nu0).sqrt();
        let n = (1.0 + es * phi0.cos().powi(4) / (1.0 - es)).sqrt();

        let s1 = (1.0 + sin0) / (1.0 - sin0);
        let s2 = (1.0 - e * sin0) / (1.0 + e * sin0);
        let w1 = (s1 * s2.powf(e)).powf(n);
        let sin_chi00 = (w1 - 1.0) / (w1 + 1.0);
        let c = (n + sin0) * (1.0 - sin_chi00) / ((n - sin0) * (1.0 + sin_chi00));
        let w2 = c * w1;
        let chi0 = ((w2 - 1.0) / (w2 + 1.0)).asin();

        Ok(Self {
            lon0: to_radians(p.lon0),
            e,
            two_rk: 2.0 * r * p.scale,
            n,
            c,
            sin_chi0: chi0.sin(),
            cos_chi0: chi0.cos(),
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }

    /// Isometric latitude of geodetic latitude `phi` on the ellipsoid.
    fn isometric(&self, phi: f64) -> f64 {
        let es = self.e * phi.sin();
        ((std::f64::consts::FRAC_PI_4 + 0.5 * phi).tan()
            * ((1.0 - es) / (1.0 + es)).powf(0.5 * self.e))
        .ln()
    }
}

impl ProjectionImpl for ObliqueStereographicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let phi = to_radians(lat_deg);
        let dlam = self.n * (to_radians(lon_deg) - self.lon0);

        // Conformal latitude on the Gauss sphere: w = c * exp(2 n psi).
        let w = self.c * (2.0 * self.n * self.isometric(phi)).exp();
        let sin_chi = (w - 1.0) / (w + 1.0);
        let cos_chi = (1.0 - sin_chi * sin_chi).max(0.0).sqrt();

        let b = 1.0 + sin_chi * self.sin_chi0 + cos_chi * self.cos_chi0 * dlam.cos();
        if b.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds(
                "point is antipodal to the stereographic origin",
            ));
        }
        let x = self.two_rk * cos_chi * dlam.sin() / b;
        let y = self.two_rk * (sin_chi * self.cos_chi0 - cos_chi * self.sin_chi0 * dlam.cos()) / b;
        Ok((x + self.fe, y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = y - self.fn_;

        // Spherical stereographic inverse on the Gauss sphere.
        let rho = (x * x + y * y).sqrt();
        let (chi, dlam) = if rho < 1e-12 {
            (self.sin_chi0.asin(), 0.0)
        } else {
            let cc = 2.0 * (rho / self.two_rk).atan();
            let (sin_c, cos_c) = cc.sin_cos();
            let chi = (cos_c * self.sin_chi0 + y * sin_c * self.cos_chi0 / rho)
                .clamp(-1.0, 1.0)
                .asin();
            let dlam = (x * sin_c).atan2(rho * self.cos_chi0 * cos_c - y * self.sin_chi0 * sin_c);
            (chi, dlam)
        };

        // Back to the ellipsoid: invert w = c * exp(2 n psi).
        let sin_chi = chi.sin();
        let psi = 0.5 * ((1.0 + sin_chi) / (self.c * (1.0 - sin_chi))).ln() / self.n;
        let es = self.e * self.e;
        let mut phi = 2.0 * psi.exp().atan() - HALFPI;
        let mut converged = false;
        for _ in 0..30 {
            let s = phi.sin();
            let delta = (self.isometric(phi) - psi) * phi.cos() * (1.0 - es * s * s) / (1.0 - es);
            phi -= delta;
            if delta.abs() < 1e-14 {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(ProjectionError::out_of_bounds(
                "Oblique Stereographic inverse did not converge",
            ));
        }
        let lam = dlam / self.n + self.lon0;
        Ok((to_degrees(lam), to_degrees(phi)))
    }
}

#[cfg(test)]
mod tests {
    use crate::ellipsoid::Ellipsoid;
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    /// IOGP Guidance Note 7-2 worked example (Amersfoort / RD New) plus
    /// PROJ 9 `sterea` reference values for points far from the origin.
    #[test]
    fn matches_iogp_example_and_proj_sterea() {
        let p = Projection::new(
            ProjectionParams::new(ProjectionKind::ObliqueStereographic)
                .with_lat0(52.156_160_555_555_55)
                .with_lon0(5.387_638_888_888_89)
                .with_scale(0.999_907_9)
                .with_false_easting(155_000.0)
                .with_false_northing(463_000.0)
                .with_ellipsoid(Ellipsoid::BESSEL),
        )
        .unwrap();

        // IOGP GN 7-2: 53°N 6°E -> E 196105.283, N 557057.739.
        let (x, y) = p.forward(6.0, 53.0).unwrap();
        assert!((x - 196_105.283).abs() < 1e-3, "x {x}");
        assert!((y - 557_057.739).abs() < 1e-3, "y {y}");

        for (x, y, lon, lat) in [
            (250_000.0, 600_000.0, 6.815_394_23, 53.379_005_75),
            (20_000.0, 350_000.0, 3.458_929_71, 51.124_324_86),
        ] {
            let (lon2, lat2) = p.inverse(x, y).unwrap();
            assert!((lon2 - lon).abs() < 1e-8, "lon {lon2} vs {lon}");
            assert!((lat2 - lat).abs() < 1e-8, "lat {lat2} vs {lat}");
            let (x2, y2) = p.forward(lon, lat).unwrap();
            assert!((x2 - x).abs() < 1e-3 && (y2 - y).abs() < 1e-3, "{x2} {y2}");
        }
    }
}
