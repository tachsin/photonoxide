//! Graded media: a refractive index n(x, y, z), or a relative permittivity ε(x, y, z), that
//! changes from point to point, given as an expression ([`crate::expr`]) of the coordinates (in
//! µm, with their units: `1.5 + 0.01*exp(-(x/2um)^2)`) and of named parameters: a diffused
//! guide's erf or Gaussian profile, a GRIN lens's parabola (#227, §6).
//!
//! A graded medium has no dispersion: it is the same at every wavelength. It is real, so never
//! active; [`Graded::check`] refuses it where ε isn't positive and finite, and every solver that
//! samples it refuses what it finds there so.
//!
//! **How the solvers read it.** Each samples it at its own points, as it samples any
//! permittivity: FDTD's and 3D FDFD's [`crate::fdtd::Simulation::new`] and
//! [`crate::fdfd::Solver3d`] average ε over each value's cell from 8³ points (harmonic along the
//! component, arithmetic across it); FDTD's smoothing ([`crate::fdtd::Structure::with_graded`])
//! takes it at the point each value's cell is about (the nodes, for the triplets), and inside a
//! body's surface the tangent plane's share of it at the cell's centre; the mode solvers' planar
//! profiles and cross-sections ([`crate::mode::slab_fd::Profile::uniform`],
//! [`crate::mode::vector::CrossSection`]) at their cells' centres. For a medium smooth over a
//! cell each is second order, the order of the schemes. docs/methods/bodies.md has the method
//! and its validation.

use crate::expr::{COORDINATES, Dim, Expr, Parameters};
use crate::geometry::Bounds3d;
use crate::{Error, Result};

/// What a graded medium's expression gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GradedKind {
    /// The refractive index n: ε = n².
    Index,
    /// The relative permittivity ε.
    Permittivity,
}

/// A graded medium: n(x, y, z) or ε(x, y, z), an expression of the coordinates (lengths, µm)
/// and of parameters, a plain number. See the module.
#[derive(Clone, Debug, PartialEq)]
pub struct Graded {
    expr: Expr,
    /// The parameters' values, one per slot of the expression's scope, the coordinates' zero.
    values: Vec<f64>,
    kind: GradedKind,
}

/// The slots of x, y and z.
const XYZ: [usize; 3] = [0, 1, 2];

impl Graded {
    /// The medium of index n = `index`(x, y, z), parsed with `parameters`' scope and evaluated
    /// at their values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the expression reads no coordinate but x, y and z and is a
    /// plain number (`1.5 + 0.1*x/um`, not `1.5 + x`).
    pub fn index(index: &Expr, parameters: &Parameters) -> Result<Graded> {
        Graded::new(index, parameters, GradedKind::Index)
    }

    /// The medium of relative permittivity ε = `permittivity`(x, y, z), as [`Graded::index`].
    ///
    /// # Errors
    ///
    /// As [`Graded::index`].
    pub fn permittivity(permittivity: &Expr, parameters: &Parameters) -> Result<Graded> {
        Graded::new(permittivity, parameters, GradedKind::Permittivity)
    }

    /// The medium of index `text`, an expression of x, y and z with no parameters.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if it doesn't parse, and as [`Graded::index`].
    pub fn parse_index(text: &str) -> Result<Graded> {
        let parameters = Parameters::none();
        let e = Expr::parse(text, parameters.scope())
            .map_err(|e| Error::invalid("graded medium", format!("\"{text}\": {e}")))?;
        Graded::index(&e, &parameters)
    }

    fn new(e: &Expr, parameters: &Parameters, kind: GradedKind) -> Result<Graded> {
        if let Some(&slot) = e
            .slots()
            .iter()
            .find(|&&s| s < COORDINATES.len() && !XYZ.contains(&s))
        {
            return Err(Error::invalid(
                "graded medium",
                format!(
                    "\"{}\" reads {}: a graded medium is a function of x, y and z only",
                    e.text(),
                    COORDINATES[slot].0
                ),
            ));
        }
        if let Some(d) = e.dim()
            && d != Dim::NONE
        {
            return Err(Error::invalid(
                "graded medium",
                format!(
                    "\"{}\" is {}: an index or a permittivity is a plain number (divide a \
                     coordinate by its unit, x/um)",
                    e.text(),
                    d.name()
                ),
            ));
        }
        Ok(Graded {
            expr: e.clone(),
            values: parameters.values().to_vec(),
            kind,
        })
    }

    /// What its expression gives.
    pub fn kind(&self) -> GradedKind {
        self.kind
    }

    /// Its expression.
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    /// The expression's value at `p` (µm).
    fn value(&self, p: [f64; 3]) -> f64 {
        let mut buf = [0.0f64; 64];
        let mut long;
        let values: &mut [f64] = if self.values.len() <= 64 {
            &mut buf[..self.values.len()]
        } else {
            long = vec![0.0; self.values.len()];
            &mut long
        };
        values.copy_from_slice(&self.values);
        values[..3].copy_from_slice(&p);
        self.expr.eval(values)
    }

    /// The relative permittivity at `p` (µm): n² for an index.
    pub fn permittivity_at(&self, p: [f64; 3]) -> f64 {
        let v = self.value(p);
        match self.kind {
            GradedKind::Index => v * v,
            GradedKind::Permittivity => v,
        }
    }

    /// The refractive index at `p` (µm): √ε for a permittivity.
    pub fn index_at(&self, p: [f64; 3]) -> f64 {
        let v = self.value(p);
        match self.kind {
            GradedKind::Index => v,
            GradedKind::Permittivity => v.sqrt(),
        }
    }

    /// Whether the medium is physical at every point of `points`: a positive and finite index
    /// (an index of zero or below has no meaning here), a permittivity positive and finite.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] naming the first point where it isn't, and its value.
    pub fn check(&self, points: impl IntoIterator<Item = [f64; 3]>) -> Result<()> {
        for p in points {
            let v = self.value(p);
            if !(v.is_finite() && v > 0.0) {
                let what = match self.kind {
                    GradedKind::Index => "index",
                    GradedKind::Permittivity => "permittivity",
                };
                return Err(Error::invalid(
                    "graded medium",
                    format!(
                        "\"{}\" gives an {what} of {v} at ({}, {}, {}) um: it must be positive \
                         and finite",
                        self.expr.text(),
                        p[0],
                        p[1],
                        p[2]
                    ),
                ));
            }
        }
        Ok(())
    }

    /// [`Graded::check`] at the nodes of a grid of `n` cells along each side of `bounds` (a side
    /// of no length, or a half-infinite one, at its finite end; an infinite one at zero).
    ///
    /// # Errors
    ///
    /// As [`Graded::check`].
    pub fn check_over(&self, bounds: Bounds3d, n: usize) -> Result<()> {
        let n = n.max(1);
        let axis = |k: usize| -> Vec<f64> {
            let (lo, hi) = (bounds.min[k], bounds.max[k]);
            if lo.is_finite() && hi.is_finite() && hi > lo {
                (0..=n)
                    .map(|i| lo + (hi - lo) * i as f64 / n as f64)
                    .collect()
            } else if lo.is_finite() {
                vec![lo]
            } else if hi.is_finite() {
                vec![hi]
            } else {
                vec![0.0]
            }
        };
        let (xs, ys, zs) = (axis(0), axis(1), axis(2));
        let mut points = Vec::with_capacity(xs.len() * ys.len() * zs.len());
        for &x in &xs {
            for &y in &ys {
                for &z in &zs {
                    points.push([x, y, z]);
                }
            }
        }
        self.check(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_index_is_read_with_its_units() {
        let g = Graded::parse_index("1.5 + 0.1*exp(-(x/2um)^2 - (y/1um)^2) + 0.0*z/um").unwrap();
        assert_eq!(g.kind(), GradedKind::Index);
        let n = g.index_at([0.0, 0.0, 3.0]);
        assert!((n - 1.6).abs() < 1e-15);
        assert!((g.permittivity_at([0.0, 0.0, 0.0]) - 2.56).abs() < 1e-14);
        let far = g.index_at([2.0, 0.0, 0.0]);
        assert!((far - (1.5 + 0.1 * (-1f64).exp())).abs() < 1e-15);
    }

    #[test]
    fn what_isnt_a_medium_is_refused() {
        // a length, a coordinate it has no value of, a parse error
        assert!(Graded::parse_index("1.5 + x").is_err());
        assert!(Graded::parse_index("1.5 + 0.1*t").is_err());
        assert!(Graded::parse_index("1.5 +").is_err());
        // a permittivity that falls to zero within the region asked about
        let parameters = Parameters::none();
        let e = Expr::parse("2.25*(1 - (x/3um)^2)", parameters.scope()).unwrap();
        let g = Graded::permittivity(&e, &parameters).unwrap();
        let inside = Bounds3d::new([-2.0, -1.0, 0.0], [2.0, 1.0, 0.0]);
        assert!(g.check_over(inside, 8).is_ok());
        let wider = Bounds3d::new([-4.0, -1.0, 0.0], [4.0, 1.0, 0.0]);
        let e = g.check_over(wider, 8).unwrap_err();
        assert!(e.to_string().contains("positive"), "{e}");
    }

    #[test]
    fn parameters_set_its_values() {
        let defs = [
            ("n0".to_owned(), "1.5".to_owned()),
            ("w".to_owned(), "2 um".to_owned()),
        ];
        let parameters = Parameters::new(&defs).unwrap();
        let e = Expr::parse("n0*sqrt(1 - 0.02*(x/w)^2)", parameters.scope()).unwrap();
        let g = Graded::index(&e, &parameters).unwrap();
        let n = g.index_at([2.0, 0.0, 0.0]);
        assert!((n - 1.5 * 0.98f64.sqrt()).abs() < 1e-15);
    }
}
