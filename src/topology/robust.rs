//! Robust designs: one density made three ways, eroded, nominal and dilated, by three thresholds of
//! its projection, the length scale and the edge displacement those thresholds impose, and the
//! epigraph form that turns the worst of several objectives into one an optimizer with
//! constraints takes.
//!
//! F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (2010),
//! doi:10.1007/s00158-010-0602-y, Section 5.1 and Eq. 14: the eroded, intermediate (blue-print)
//! and dilated designs, projected at thresholds 1 − η, ½ and η of the same filtered density, and
//! the robust problem min max over the three. O. Sigmund, Acta Mech. Sin. 25, 227 (2009),
//! doi:10.1007/s10409-009-0240-z, the same with morphology-based filters.
//!
//! X. Qian, O. Sigmund, Comput. Methods Appl. Mech. Engrg. 253, 237 (2013),
//! doi:10.1016/j.cma.2012.08.020, Appendix, Eq. 29: with the hat filter of radius R, the minimum
//! width b that the nominal design keeps when the eroded one (threshold η_e = 1 − η) just loses
//! a feature, b/R = 2 − 2√(1 − η_e) for η_e ≥ ¾ and 2√(η_e − ½) below. Wang et al.'s Fig. 12
//! plots the same curve with its axis labelled b/2R (their text's 1.1055 at η = 0.2 is
//! Eq. 29's b/R, 2 − 2√0.2; Qian & Sigmund's Table 1 and Fig. 19 read Wang et al.'s prediction
//! as b/R).
//!
//! The edge displacement: across a straight edge, the filtered density at a distance x outside
//! a solid half-space is the share of the filter's weight that reaches inside. For the 1D hat it
//! is ½ (1 − x/R)². For the 2D cone, R − r over a disc, the weight in a strip at distance s is
//! the cone's marginal m(s) = R a − s² arcosh(R/s), a = √(R² − s²), whose integral gives, with
//! t = x/R,
//!
//! ρ̃(t) = ½ − (2t√(1 − t²) + arcsin t − t³ arcosh(1/t)) / π, 0 ≤ t ≤ 1,
//!
//! and ρ̃(−t) = 1 − ρ̃(t) inside (worked out for this module; its derivative is −(3/π)
//! (√(1 − t²) − t² arcosh(1/t)) = −3 m/(πR²), and it is ½ at the edge and 0 at one radius).
//! A threshold η then moves the edge to where ρ̃ = η: outward below ½ (dilated), inward above
//! (eroded).

use super::{Density, Projection};
use crate::portable;
use crate::{Error, Result};

/// The three realizations of one density (Wang et al. 2010, Section 5.1): its projection at the
/// thresholds 1 − η (eroded: features shrink), ½ (nominal, the blue-print) and η (dilated:
/// features grow), at the same sharpness β.
#[derive(Clone, Debug, PartialEq)]
pub struct Realizations {
    /// Projected at 1 − η.
    pub eroded: Density,
    /// Projected at ½.
    pub nominal: Density,
    /// Projected at η.
    pub dilated: Density,
}

impl Realizations {
    /// The three, eroded first.
    pub fn all(&self) -> [&Density; 3] {
        [&self.eroded, &self.nominal, &self.dilated]
    }
}

impl Density {
    /// The eroded, nominal and dilated realizations of this density at the thresholds 1 − `eta`,
    /// ½ and `eta`, each with this density's filter and its projection's β.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a density with no projection, or an η outside (0, ½).
    pub fn robust(&self, eta: f64) -> Result<Realizations> {
        let Some(p) = self.projection() else {
            return Err(Error::invalid(
                "robust",
                "the realizations need a projection: give the density one first",
            ));
        };
        if !(eta > 0.0 && eta < 0.5) {
            return Err(Error::invalid(
                "robust",
                format!("η must be in (0, ½), the dilated threshold, got {eta}"),
            ));
        }
        let at = |threshold: f64| -> Result<Density> {
            Ok(self
                .clone()
                .with_projection(Projection::new(p.beta(), threshold)?))
        };
        Ok(Realizations {
            eroded: at(1.0 - eta)?,
            nominal: at(0.5)?,
            dilated: at(eta)?,
        })
    }
}

fn check_threshold(threshold: f64) -> Result<()> {
    if !(threshold > 0.0 && threshold < 1.0) {
        return Err(Error::invalid(
            "robust",
            format!("a threshold must be in (0, 1), got {threshold}"),
        ));
    }
    Ok(())
}

/// The minimum width b the nominal design keeps, over the filter's radius R, when the robust
/// designs' dilated threshold is `eta` (the eroded one 1 − η) and the filter is the 1D hat:
/// Qian & Sigmund 2013's Eq. 29, 2 − 2√η for η ≤ ¼ and 2√(½ − η) above. In 2D and 3D Wang et
/// al. take the same b as the diameter of the smallest disc or sphere.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an η outside (0, ½).
pub fn minimum_length(eta: f64) -> Result<f64> {
    if !(eta > 0.0 && eta < 0.5) {
        return Err(Error::invalid(
            "robust",
            format!("η must be in (0, ½), got {eta}"),
        ));
    }
    Ok(if eta <= 0.25 {
        2.0 - 2.0 * eta.sqrt()
    } else {
        2.0 * (0.5 - eta).sqrt()
    })
}

/// The 1D hat filter's density at `t` radii outside a solid half-line's edge (negative: inside),
/// ½ (1 − t)² for 0 ≤ t ≤ 1.
pub fn across_edge_1d(t: f64) -> f64 {
    if t < 0.0 {
        return 1.0 - across_edge_1d(-t);
    }
    if t >= 1.0 {
        return 0.0;
    }
    0.5 * (1.0 - t) * (1.0 - t)
}

/// The 2D conic filter's density at `t` radii outside a solid half-plane's straight edge
/// (negative: inside): ½ − (2t√(1 − t²) + arcsin t − t³ arcosh(1/t)) / π for 0 ≤ t ≤ 1.
pub fn across_edge_2d(t: f64) -> f64 {
    if t < 0.0 {
        return 1.0 - across_edge_2d(-t);
    }
    if t >= 1.0 {
        return 0.0;
    }
    if t == 0.0 {
        return 0.5;
    }
    let a = (1.0 - t * t).sqrt();
    0.5 - (2.0 * t * a + portable::asin(t) - portable::powi(t, 3) * portable::acosh(1.0 / t))
        / std::f64::consts::PI
}

/// d/dt of [`across_edge_2d`]: −(3/π)(√(1 − t²) − t² arcosh(1/t)), even in t.
fn across_edge_2d_slope(t: f64) -> f64 {
    let t = t.abs();
    if t >= 1.0 {
        return 0.0;
    }
    let tail = if t == 0.0 {
        0.0
    } else {
        t * t * portable::acosh(1.0 / t)
    };
    -3.0 / std::f64::consts::PI * ((1.0 - t * t).sqrt() - tail)
}

/// How far a straight edge of a solid region moves, in filter radii, when its filtered density
/// is projected at `threshold` by the 1D hat filter: outward (positive) below ½, inward above;
/// 1 − √(2η) for η ≤ ½.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a threshold outside (0, 1).
pub fn edge_shift_1d(threshold: f64) -> Result<f64> {
    check_threshold(threshold)?;
    Ok(if threshold <= 0.5 {
        1.0 - (2.0 * threshold).sqrt()
    } else {
        -(1.0 - (2.0 * (1.0 - threshold)).sqrt())
    })
}

/// How far a straight edge moves, in filter radii, when the 2D conic filter's density is
/// projected at `threshold`: the t at which [`across_edge_2d`] is the threshold, by Newton's
/// method from bisection's bracket (to round-off).
///
/// # Errors
///
/// [`Error::InvalidValue`] for a threshold outside (0, 1).
pub fn edge_shift_2d(threshold: f64) -> Result<f64> {
    check_threshold(threshold)?;
    // ρ̃ falls from 1 at t = −1 to 0 at t = 1: bisect, then polish by Newton
    let (mut lo, mut hi) = (-1.0f64, 1.0f64);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if across_edge_2d(mid) > threshold {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut t = 0.5 * (lo + hi);
    for _ in 0..3 {
        let slope = across_edge_2d_slope(t);
        if slope == 0.0 {
            break;
        }
        let next = t - (across_edge_2d(t) - threshold) / slope;
        if !(lo..=hi).contains(&next) {
            break;
        }
        t = next;
    }
    Ok(t)
}

/// The epigraph form of min over x of max over k of f_k(x) (Wang et al.'s Eq. 14, and every
/// worst case over wavelengths and corners): the variables y = (x, t), the objective t, and the
/// constraints g_k = f_k(x) − t ≤ 0, smooth where the max isn't. Its arrays are those an
/// optimizer with inequality constraints takes (genoxide's `Mma` and `Constrained`: the
/// objective's gradient, the constraints' values and their Jacobian row by row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Epigraph {
    variables: usize,
    functions: usize,
}

impl Epigraph {
    /// The form for `variables` design variables and `functions` objectives f_k.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no variables or no functions.
    pub fn new(variables: usize, functions: usize) -> Result<Epigraph> {
        if variables == 0 || functions == 0 {
            return Err(Error::invalid(
                "epigraph",
                format!(
                    "needs a variable and a function at least, got {variables} and {functions}"
                ),
            ));
        }
        Ok(Epigraph {
            variables,
            functions,
        })
    }

    /// The number of the optimizer's variables, the design's and t: n + 1.
    pub fn len(&self) -> usize {
        self.variables + 1
    }

    /// Never: the form has t at least.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The number of constraints, one per function.
    pub fn constraints(&self) -> usize {
        self.functions
    }

    /// A start for t: the largest of `values`, so that every constraint holds with equality at
    /// the worst.
    pub fn start(values: &[f64]) -> f64 {
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }

    /// At `y` = (x, t), with the functions' `values` f_k(x) and their `gradients` (row k the
    /// gradient of f_k, n values each, rows one after the other): writes the objective's
    /// `gradient` (n + 1), the constraints `g` (one per function) and their `jacobian` (row k
    /// the gradient of g_k in y, n + 1 values each), and returns the objective, t.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an array of the wrong length.
    pub fn evaluate(
        &self,
        y: &[f64],
        values: &[f64],
        gradients: &[f64],
        gradient: &mut [f64],
        g: &mut [f64],
        jacobian: &mut [f64],
    ) -> Result<f64> {
        let (n, k) = (self.variables, self.functions);
        for (what, got, want) in [
            ("y", y.len(), n + 1),
            ("values", values.len(), k),
            ("gradients", gradients.len(), k * n),
            ("gradient", gradient.len(), n + 1),
            ("g", g.len(), k),
            ("jacobian", jacobian.len(), k * (n + 1)),
        ] {
            if got != want {
                return Err(Error::invalid(
                    "epigraph",
                    format!("{what} must have {want} values, got {got}"),
                ));
            }
        }
        let t = y[n];
        gradient[..n].fill(0.0);
        gradient[n] = 1.0;
        for (row, ((gk, &fk), dfk)) in jacobian
            .chunks_exact_mut(n + 1)
            .zip(g.iter_mut().zip(values).zip(gradients.chunks_exact(n)))
        {
            *gk = fk - t;
            row[..n].copy_from_slice(dfk);
            row[n] = -1.0;
        }
        Ok(t)
    }
}
