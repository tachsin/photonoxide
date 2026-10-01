//! Rasters: a structure's permittivity sampled on a rectangular grid, for pictures and checks.
//!
//! This is point sampling at the cell centres, for looking at a structure. Solvers get their
//! own discretization, with subpixel smoothing (ROADMAP.md, 0.4).

use serde::{Deserialize, Serialize};

use crate::geometry::Point;
use crate::stack::Structure;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// Values on a grid of `nx` × `ny` cells covering [x0, x1] × [y0, y1] (µm), row by row from
/// y0: the value of cell (i, j) is `values[j * nx + i]`, sampled at its centre. For a side
/// view, "y" is the height z.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Raster {
    /// Cells along x.
    pub nx: usize,
    /// Cells along y (or z).
    pub ny: usize,
    /// The left edge, µm.
    pub x0: f64,
    /// The right edge, µm.
    pub x1: f64,
    /// The bottom edge, µm.
    pub y0: f64,
    /// The top edge, µm.
    pub y1: f64,
    /// The values, `nx * ny` of them.
    pub values: Vec<f32>,
}

impl Raster {
    /// The value of cell (i, j).
    pub fn at(&self, i: usize, j: usize) -> f32 {
        self.values[j * self.nx + i]
    }

    /// The smallest and largest value.
    pub fn range(&self) -> (f32, f32) {
        self.values
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            })
    }
}

/// The cell centres covering [lo, hi] with cells of about `step`: a whole number of equal
/// cells, the count rounded to nearest.
fn centres(lo: f64, hi: f64, step: f64) -> Result<Vec<f64>> {
    if !(lo.is_finite() && hi.is_finite() && hi > lo) {
        return Err(Error::invalid(
            "raster",
            format!("the range {lo} to {hi} um is empty"),
        ));
    }
    if !(step.is_finite() && step > 0.0) {
        return Err(Error::invalid(
            "raster",
            format!("the step must be positive, got {step} um"),
        ));
    }
    let n = ((hi - lo) / step).round().max(1.0);
    if n > 10_000.0 {
        return Err(Error::invalid(
            "raster",
            format!("{n} cells along one side is more than 10000"),
        ));
    }
    let n = n as usize;
    let h = (hi - lo) / n as f64;
    Ok((0..n).map(|i| lo + (i as f64 + 0.5) * h).collect())
}

impl Structure {
    /// The real permittivity seen from above, at the middle of the layer named `layer`, over
    /// [x0, x1] × [y0, y1] with cells of about `step`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown layer, an empty range or a bad step, and
    /// [`Error::OutsideValidity`] if a material has no data at `wavelength`.
    pub fn top_view(
        &self,
        layer: &str,
        x: (Length, Length),
        y: (Length, Length),
        step: Length,
        wavelength: Wavelength,
    ) -> Result<Raster> {
        let (_, bottom, top) = self
            .stack()
            .layer(layer)
            .ok_or_else(|| Error::invalid("raster", format!("the stack has no layer {layer}")))?;
        let z = (bottom + top) / 2.0;
        let xs = centres(x.0.to_um(), x.1.to_um(), step.to_um())?;
        let ys = centres(y.0.to_um(), y.1.to_um(), step.to_um())?;
        let mut values = Vec::with_capacity(xs.len() * ys.len());
        for &yv in &ys {
            for &xv in &xs {
                let m = self.material_at(Point::um(xv, yv), z);
                values.push(m.permittivity(wavelength)?.re as f32);
            }
        }
        Ok(Raster {
            nx: xs.len(),
            ny: ys.len(),
            x0: x.0.to_um(),
            x1: x.1.to_um(),
            y0: y.0.to_um(),
            y1: y.1.to_um(),
            values,
        })
    }

    /// The real permittivity on the vertical cut at `y`, over [x0, x1] × [z0, z1] with cells of
    /// about `step`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty range or a bad step, and
    /// [`Error::OutsideValidity`] if a material has no data at `wavelength`.
    pub fn side_view(
        &self,
        y: Length,
        x: (Length, Length),
        z: (Length, Length),
        step: Length,
        wavelength: Wavelength,
    ) -> Result<Raster> {
        let xs = centres(x.0.to_um(), x.1.to_um(), step.to_um())?;
        let zs = centres(z.0.to_um(), z.1.to_um(), step.to_um())?;
        let mut values = Vec::with_capacity(xs.len() * zs.len());
        for &zv in &zs {
            for &xv in &xs {
                let m = self.material_at(
                    Point {
                        x: Length::um(xv),
                        y,
                    },
                    Length::um(zv),
                );
                values.push(m.permittivity(wavelength)?.re as f32);
            }
        }
        Ok(Raster {
            nx: xs.len(),
            ny: zs.len(),
            x0: x.0.to_um(),
            x1: x.1.to_um(),
            y0: z.0.to_um(),
            y1: z.1.to_um(),
            values,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Shape;
    use crate::stack::LayerStack;

    fn waveguide() -> Structure {
        let mut s = Structure::new(LayerStack::soi_220());
        // 600 nm wide, so its edges (y = ±0.3 um) fall between the 100 nm cells' centres
        let wg = Shape::rect(Point::um(0.0, 0.0), Length::um(4.0), Length::nm(600.0)).unwrap();
        s.draw("Si", wg).unwrap();
        s
    }

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    #[test]
    fn the_top_view_shows_silicon_in_the_waveguide_and_oxide_around() {
        let r = waveguide()
            .top_view(
                "Si",
                (Length::um(-1.0), Length::um(1.0)),
                (Length::um(-1.0), Length::um(1.0)),
                Length::nm(100.0),
                lam(),
            )
            .unwrap();
        assert_eq!((r.nx, r.ny), (20, 20));
        let si = 3.4757f32 * 3.4757;
        let ox = 1.444f32 * 1.444;
        // the centre row is silicon, the edge rows oxide
        assert!((r.at(10, 10) - si).abs() < 1e-3, "{}", r.at(10, 10));
        assert!((r.at(10, 0) - ox).abs() < 2e-3, "{}", r.at(10, 0));
        // 600 nm of 2 um: 6 of 20 rows are silicon
        let silicon_rows = (0..r.ny).filter(|&j| r.at(0, j) > 10.0).count();
        assert_eq!(silicon_rows, 6);
        let (lo, hi) = r.range();
        assert!(lo < 2.1 && hi > 12.0);
    }

    #[test]
    fn the_side_view_shows_the_layers() {
        let r = waveguide()
            .side_view(
                Length::ZERO,
                (Length::um(-1.0), Length::um(1.0)),
                (Length::um(-0.5), Length::um(3.0)),
                Length::nm(10.0),
                lam(),
            )
            .unwrap();
        // z = -0.5..0: substrate silicon; 0..2: buried oxide; 2..2.22: the waveguide; above: oxide
        let z_index = |z: f64| ((z + 0.5) / 0.01) as usize;
        assert!(r.at(100, z_index(-0.25)) > 10.0);
        assert!(r.at(100, z_index(1.0)) < 2.2);
        assert!(r.at(100, z_index(2.11)) > 10.0);
        assert!(r.at(100, z_index(2.6)) < 2.2);
        // 22 rows of silicon in the device layer
        let rows = (z_index(1.5)..r.ny)
            .filter(|&j| r.at(100, j) > 10.0)
            .count();
        assert_eq!(rows, 22);
    }

    #[test]
    fn bad_rasters_are_errors() {
        let s = waveguide();
        let range = (Length::um(-1.0), Length::um(1.0));
        let step = Length::nm(100.0);
        assert!(s.top_view("Metal", range, range, step, lam()).is_err());
        assert!(
            s.top_view("Si", (Length::um(1.0), Length::um(1.0)), range, step, lam())
                .is_err()
        );
        assert!(s.top_view("Si", range, range, Length::ZERO, lam()).is_err());
        assert!(
            s.top_view("Si", range, range, Length::nm(0.01), lam())
                .is_err()
        );
        // silicon has no data at 1 um
        let e = s
            .top_view("Si", range, range, step, Wavelength::um(1.0).unwrap())
            .unwrap_err();
        assert!(matches!(e, Error::OutsideValidity { .. }), "{e}");
    }
}
