//! The error function, to round-off: for |x| < 3 the series of positive terms
//! erf x = (2/√π) e^(−x²) Σ 2ⁿ x^(2n+1) / (1·3·…·(2n+1)) (DLMF 7.6.2), which has no
//! cancellation; beyond, erfc's continued fraction (DLMF 7.9.2) by Lentz's method.

use std::f64::consts::PI;

/// erf(x).
pub(crate) fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let a = x.abs();
    let v = if a < 3.0 {
        let x2 = a * a;
        let mut term = a;
        let mut sum = a;
        let mut n = 0.0;
        while term > 1e-17 * sum {
            n += 1.0;
            term *= 2.0 * x2 / (2.0 * n + 1.0);
            sum += term;
        }
        2.0 / PI.sqrt() * (-x2).exp() * sum
    } else if a < 6.5 {
        1.0 - erfc_fraction(a)
    } else {
        1.0
    };
    v.copysign(x)
}

/// erfc(x) for x ≥ 3: e^(−x²)/√π · 1/(x + (1/2)/(x + 1/(x + (3/2)/(x + …)))).
fn erfc_fraction(x: f64) -> f64 {
    // f = x + K(aₖ / x), aₖ = k/2, by the modified Lentz method
    let tiny = 1e-300;
    let mut f = x;
    let mut c = x;
    let mut d = 0.0;
    for k in 1..200 {
        let ak = f64::from(k) / 2.0;
        d = x + ak * d;
        if d == 0.0 {
            d = tiny;
        }
        c = x + ak / c;
        if c == 0.0 {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = c * d;
        f *= delta;
        if (delta - 1.0).abs() < 1e-16 {
            break;
        }
    }
    (-x * x).exp() / PI.sqrt() / f
}

/// d erf / dx = (2/√π) e^(−x²).
pub(crate) fn erf_derivative(x: f64) -> f64 {
    2.0 / PI.sqrt() * (-x * x).exp()
}
