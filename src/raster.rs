//! Rasters: a structure's permittivity sampled on a rectangular grid, for pictures and checks.
//!
//! [`Structure::top_view`] and [`Structure::side_view`] sample at the cell centres, for looking at
//! a structure. [`Structure::top_view_averaged`] and [`Structure::side_view_averaged`] take an
//! [`Averaging`] instead: samples over each cell, or the fill-weighted mean of the materials in
//! it ([`crate::averaging`]), which shows what a solver averaging from the fill sees. Solvers get
//! their own discretization, with averaging or subpixel smoothing.

use serde::{Deserialize, Serialize};

use crate::averaging::Averaging;
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
    /// [`Error::InvalidValue`] for a cut at a `y` that isn't finite, an empty range or a bad
    /// step, and [`Error::OutsideValidity`] if a material has no data at `wavelength`.
    pub fn side_view(
        &self,
        y: Length,
        x: (Length, Length),
        z: (Length, Length),
        step: Length,
        wavelength: Wavelength,
    ) -> Result<Raster> {
        if !y.to_um().is_finite() {
            return Err(Error::invalid(
                "raster",
                format!("the cut must be at a finite y, got {y}"),
            ));
        }
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

/// How far either side of a side view's cut its cells reach, as a share of the step: the cut is
/// a line, and its cells' fill is taken over this thin a strip about it.
pub const CUT_STRIP: f64 = 1e-6;

impl Structure {
    /// [`Structure::top_view`] with each cell's value averaged by `averaging`:
    /// [`Averaging::Point`] is `top_view`, to the bit; [`Averaging::Samples`] the mean of 8 × 8
    /// samples over the cell; [`Averaging::Mean`] the fill-weighted mean of the materials in it
    /// ([`crate::averaging`]), each shape's area in the cell exact for polygons and to a stated
    /// tolerance for curves.
    ///
    /// # Errors
    ///
    /// As [`Structure::top_view`], and [`Error::InvalidValue`] for the diagonal or the tensor,
    /// which are a field component's averages, not a picture's.
    pub fn top_view_averaged(
        &self,
        layer: &str,
        x: (Length, Length),
        y: (Length, Length),
        step: Length,
        wavelength: Wavelength,
        averaging: Averaging,
    ) -> Result<Raster> {
        if averaging == Averaging::Point {
            return self.top_view(layer, x, y, step, wavelength);
        }
        let (_, bottom, top) = self
            .stack()
            .layer(layer)
            .ok_or_else(|| Error::invalid("raster", format!("the stack has no layer {layer}")))?;
        let z = ((bottom + top) / 2.0).to_um();
        let xs = centres(x.0.to_um(), x.1.to_um(), step.to_um())?;
        let ys = centres(y.0.to_um(), y.1.to_um(), step.to_um())?;
        let (hx, hy) = (cell(&xs, x), cell(&ys, y));
        let values = self.averaged_cells(&xs, &ys, wavelength, averaging, [0, 1], |a, b| {
            (
                [a - hx / 2.0, b - hy / 2.0, z],
                [a + hx / 2.0, b + hy / 2.0, z],
            )
        })?;
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

    /// [`Structure::side_view`] with each cell's value averaged by `averaging`, as
    /// [`Structure::top_view_averaged`]; a cell's fill is taken over a strip [`CUT_STRIP`] of
    /// the step wide either side of the cut.
    ///
    /// # Errors
    ///
    /// As [`Structure::side_view`] and [`Structure::top_view_averaged`].
    pub fn side_view_averaged(
        &self,
        y: Length,
        x: (Length, Length),
        z: (Length, Length),
        step: Length,
        wavelength: Wavelength,
        averaging: Averaging,
    ) -> Result<Raster> {
        if averaging == Averaging::Point {
            return self.side_view(y, x, z, step, wavelength);
        }
        if !y.to_um().is_finite() {
            return Err(Error::invalid(
                "raster",
                format!("the cut must be at a finite y, got {y}"),
            ));
        }
        let xs = centres(x.0.to_um(), x.1.to_um(), step.to_um())?;
        let zs = centres(z.0.to_um(), z.1.to_um(), step.to_um())?;
        let (hx, hz) = (cell(&xs, x), cell(&zs, z));
        let (yc, half) = (y.to_um(), CUT_STRIP * step.to_um());
        let values = self.averaged_cells(&xs, &zs, wavelength, averaging, [0, 2], |a, b| {
            (
                [a - hx / 2.0, yc - half, b - hz / 2.0],
                [a + hx / 2.0, yc + half, b + hz / 2.0],
            )
        })?;
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

    /// The real permittivity of the cells about (a, b) for `a` in `along` and `b` in `across`,
    /// row by row, each the box `cell(a, b)` gives, by samples or the fill's mean.
    fn averaged_cells(
        &self,
        along: &[f64],
        across: &[f64],
        wavelength: Wavelength,
        averaging: Averaging,
        axes: [usize; 2],
        cell: impl Fn(f64, f64) -> ([f64; 3], [f64; 3]),
    ) -> Result<Vec<f32>> {
        if !matches!(averaging, Averaging::Samples | Averaging::Mean) {
            return Err(Error::invalid(
                "raster",
                format!(
                    "a picture's cells take \"point\", \"samples\" or \"mean\", not \"{}\", a \
                     field component's average",
                    averaging.name()
                ),
            ));
        }
        let fills = crate::stack::fill::Fills::new(self);
        let materials = fills.materials();
        let eps: Vec<num_complex::Complex64> = materials
            .iter()
            .map(|m| m.permittivity(wavelength))
            .collect::<Result<_>>()?;
        let at = |p: [f64; 3]| {
            let m = self.material_at(Point::um(p[0], p[1]), Length::um(p[2]));
            let k = materials
                .iter()
                .position(|n| std::ptr::eq(*n, m))
                .expect("every material of the structure is listed");
            eps[k]
        };
        // the mean of 8 × 8 samples over the cell's two sides, along `axes`; across them, at
        // the box's middle
        let samples = |lo: [f64; 3], hi: [f64; 3]| {
            let n = 8;
            let mut sum = num_complex::Complex64::new(0.0, 0.0);
            for s in 0..n {
                for t in 0..n {
                    let mut w = [0.5; 3];
                    w[axes[0]] = (s as f64 + 0.5) / n as f64;
                    w[axes[1]] = (t as f64 + 0.5) / n as f64;
                    let p = [0, 1, 2].map(|k| lo[k] + w[k] * (hi[k] - lo[k]));
                    sum += at(p);
                }
            }
            sum / (n * n) as f64
        };
        let mut values = Vec::with_capacity(along.len() * across.len());
        for &b in across {
            for &a in along {
                let (lo, hi) = cell(a, b);
                let v = if averaging == Averaging::Samples {
                    samples(lo, hi)
                } else {
                    let slices = fills.slices(lo, hi);
                    crate::stack::fill::combine(&slices, &eps, 2, Averaging::Mean, |z, _| {
                        let (mut l, mut h) = (lo, hi);
                        (l[2], h[2]) = z;
                        samples(l, h)
                    })
                };
                values.push(v.re as f32);
            }
        }
        Ok(values)
    }
}

/// The width of the cells whose centres are `centres` over `range`.
fn cell(centres: &[f64], range: (Length, Length)) -> f64 {
    (range.1.to_um() - range.0.to_um()) / centres.len() as f64
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
    fn averaged_views_keep_aligned_edges_and_integrate_the_rest_exactly() {
        let range = (Length::um(-1.0), Length::um(1.0));
        let step = Length::nm(100.0);
        // the 600 nm guide's edges are cell edges: the fill's mean is point sampling, to the bit
        let s = waveguide();
        let point = s.top_view("Si", range, range, step, lam()).unwrap();
        let point_too = s
            .top_view_averaged("Si", range, range, step, lam(), Averaging::Point)
            .unwrap();
        let mean = s
            .top_view_averaged("Si", range, range, step, lam(), Averaging::Mean)
            .unwrap();
        assert_eq!(point, point_too);
        assert_eq!(point, mean);
        // a 555 nm guide: a column of the mean integrates ε across it exactly
        let mut narrow = Structure::new(LayerStack::soi_220());
        let wg = Shape::rect(Point::um(0.0, 0.0123), Length::um(4.0), Length::nm(555.0)).unwrap();
        narrow.draw("Si", wg).unwrap();
        let r = narrow
            .top_view_averaged("Si", range, range, step, lam(), Averaging::Mean)
            .unwrap();
        let eps = |m: crate::material::Material| m.permittivity(lam()).unwrap().re;
        let (si, ox) = (
            eps(crate::material::silicon()),
            eps(crate::material::silica()),
        );
        let column: f64 = (0..r.ny).map(|j| f64::from(r.at(10, j)) * 0.1).sum();
        let exact = si * 0.555 + ox * (2.0 - 0.555);
        // f32 values: about 1e-7 of each
        assert!((column - exact).abs() < 1e-5, "{column} {exact}");
        // the side view's cut through it: the layers' faces off the cells' edges, exact too
        let side = narrow
            .side_view_averaged(
                Length::ZERO,
                range,
                (Length::um(1.95), Length::um(2.3)),
                Length::nm(70.0),
                lam(),
                Averaging::Mean,
            )
            .unwrap();
        let column: f64 = (0..side.ny).map(|j| f64::from(side.at(14, j)) * 0.07).sum();
        let exact = si * 0.22 + ox * (0.35 - 0.22);
        assert!((column - exact).abs() < 1e-5, "{column} {exact}");
        // a picture has no field component
        let e = s
            .top_view_averaged("Si", range, range, step, lam(), Averaging::Diagonal)
            .unwrap_err()
            .to_string();
        assert!(e.contains("diagonal"), "{e}");
        // samples over the cell: 8 by 8, between the two
        let sampled = narrow
            .top_view_averaged("Si", range, range, step, lam(), Averaging::Samples)
            .unwrap();
        let column: f64 = (0..sampled.ny)
            .map(|j| f64::from(sampled.at(10, j)) * 0.1)
            .sum();
        assert!((column - exact).abs() > 0.0);
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
        // a cut at y = NaN drew the cladding everywhere, as if it were a cut
        let z = (Length::um(0.0), Length::um(3.0));
        let e = s
            .side_view(Length::um(f64::NAN), range, z, step, lam())
            .unwrap_err()
            .to_string();
        assert!(e.contains("finite y"), "{e}");
    }
}
