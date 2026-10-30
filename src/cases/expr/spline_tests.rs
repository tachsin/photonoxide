//! The expressions' spline against the geometry's: a test of `expr` (photonoxide-core) that
//! needs the geometry crate, above it. Moved here from expr's tests (#508).

use crate::expr::{Expr, Scope};

/// `spline(x, [knots], [values])` at `x` (µm).
fn spline_at(knots: &str, values: &str, x: f64) -> f64 {
    let text = format!("spline(x, [{knots}], [{values}])");
    let e = Expr::parse(&text, &Scope::new()).unwrap_or_else(|e| panic!("{text}: {e}"));
    let mut v = [0.0; 7];
    v[0] = x;
    e.try_eval(&v).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// Lengths in µm, as a list's text.
fn lengths(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:?} um"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn a_spline_is_zhangs_not_a_knot_spline() {
    // Zhang et al.'s 13 widths 1/6 µm apart: the expression against the spline the geometry's
    // check solves for itself, at 1001 points across the knots and beyond the ends
    let widths = crate::geometry::curve_checks::ZHANG_WIDTHS;
    let h = 2.0 / 12.0;
    let pieces = crate::geometry::curve_checks::spline(&widths, h);
    let knots: Vec<f64> = (0..13).map(|k| f64::from(k) * h).collect();
    let mut worst = 0.0f64;
    for i in 0..=1000 {
        let x = -0.1 + 2.2 * f64::from(i) / 1000.0;
        let k = ((x / h).floor().max(0.0) as usize).min(11);
        let t = x - k as f64 * h;
        let c = &pieces[k];
        let expected = c[0] + t * (c[1] + t * (c[2] + t * c[3]));
        let got = spline_at(&lengths(&knots), &lengths(&widths), x);
        worst = worst.max((got - expected).abs());
    }
    assert!(worst < 1e-13, "{worst:e}");
    // through every knot
    for (x, w) in knots.iter().zip(widths) {
        assert!((spline_at(&lengths(&knots), &lengths(&widths), *x) - w).abs() < 1e-14);
    }
}
