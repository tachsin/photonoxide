//! The Fresnel integrals, C(t) = ∫₀ᵗ cos(πu²/2) du and S(t) = ∫₀ᵗ sin(πu²/2) du, to round-off,
//! and the clothoid they give: ∫₀^τ exp(i a u²/2) du.
//!
//! - For |t| ≤ 1.6, their power series (A&S 7.3.11 and 7.3.13), whose largest term is under 2,
//!   so no digit is lost to cancellation.
//! - Beyond, C + iS = (1 + i)/2 · erf(z) with z = √π (1 − i) t/2 (A&S 7.3.22), and erfc by
//!   Laplace's continued fraction, √π e^{z²} erfc z = 1/(z + ½/(z + 1/(z + 3⁄2/(z + …))))
//!   (A&S 7.1.14), valid for Re z > 0 and evaluated by the modified Lentz method.
//!
//! M. Abramowitz, I. A. Stegun, Handbook of Mathematical Functions (1964), chapter 7.

use std::f64::consts::PI;

/// Where the series hands over to the continued fraction.
const SWITCH: f64 = 1.6;

/// (C(t), S(t)).
pub(crate) fn fresnel(t: f64) -> (f64, f64) {
    if t < 0.0 {
        let (c, s) = fresnel(-t);
        return (-c, -s);
    }
    if t <= SWITCH {
        series(t)
    } else if t.is_infinite() {
        (0.5, 0.5)
    } else {
        continued(t)
    }
}

/// The power series: C = t Σ (−1)ⁿ (π/2)²ⁿ t⁴ⁿ / ((2n)! (4n + 1)), and S likewise with the odd
/// powers, summed until the terms no longer change the sums.
fn series(t: f64) -> (f64, f64) {
    let z = PI / 2.0 * t * t;
    // the terms of e^{iz} = Σ (iz)^k / k!, times t/(2k + 1)
    let (mut c, mut s) = (0.0f64, 0.0f64);
    let mut term = t; // t z^k / k!
    let mut k = 0u32;
    loop {
        let part = term / f64::from(2 * k + 1);
        let (old_c, old_s) = (c, s);
        match k % 4 {
            0 => c += part,
            1 => s += part,
            2 => c -= part,
            _ => s -= part,
        }
        k += 1;
        term *= z / f64::from(k);
        if k > 4 && c == old_c && s == old_s {
            break;
        }
        if k > 200 {
            break;
        }
    }
    (c, s)
}

/// A complex number, as (re, im).
#[derive(Clone, Copy, Debug)]
struct Z(f64, f64);

impl Z {
    fn add(self, o: Z) -> Z {
        Z(self.0 + o.0, self.1 + o.1)
    }

    fn mul(self, o: Z) -> Z {
        Z(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }

    fn div(self, o: Z) -> Z {
        // Smith's algorithm, against overflow
        if o.0.abs() >= o.1.abs() {
            let r = o.1 / o.0;
            let d = o.0 + o.1 * r;
            Z((self.0 + self.1 * r) / d, (self.1 - self.0 * r) / d)
        } else {
            let r = o.0 / o.1;
            let d = o.0 * r + o.1;
            Z((self.0 * r + self.1) / d, (self.1 * r - self.0) / d)
        }
    }

    fn scale(self, a: f64) -> Z {
        Z(self.0 * a, self.1 * a)
    }

    fn norm1(self) -> f64 {
        self.0.abs() + self.1.abs()
    }
}

/// The continued fraction: E = C + iS = (1 + i)/2 · (1 − e^{iπt²/2} K/√π) with
/// K = √π e^{z²} erfc z, z = √π (1 − i) t/2, so that e^{−z²} = e^{iπt²/2}.
fn continued(t: f64) -> (f64, f64) {
    let a = PI.sqrt() * t / 2.0;
    let z = Z(a, -a);
    // K = 1/G, G = z + a₁/(z + a₂/(z + …)) with aⱼ = j/2, by the modified Lentz method
    let tiny = 1e-300;
    let mut f = z;
    let mut c = f;
    let mut d = Z(0.0, 0.0);
    for j in 1..20_000u32 {
        let aj = f64::from(j) / 2.0;
        d = z.add(d.scale(aj));
        if d.norm1() < tiny {
            d = Z(tiny, 0.0);
        }
        c = z.add(Z(aj, 0.0).div(c));
        if c.norm1() < tiny {
            c = Z(tiny, 0.0);
        }
        d = Z(1.0, 0.0).div(d);
        let delta = c.mul(d);
        f = f.mul(delta);
        if (delta.0 - 1.0).abs() + delta.1.abs() < 1e-16 {
            break;
        }
    }
    let k = Z(1.0, 0.0).div(f);
    let phase = PI / 2.0 * t * t;
    let (sin, cos) = phase.sin_cos();
    let w = Z(cos, sin).mul(k).scale(1.0 / PI.sqrt());
    let one_minus = Z(1.0 - w.0, -w.1);
    let e = Z(0.5, 0.5).mul(one_minus);
    (e.0, e.1)
}

/// ∫₀^τ exp(i a u²/2) du, as (re, im): the clothoid from zero curvature whose curvature grows
/// at the rate `a` (1/µm², positive turning left), after the arc length τ.
pub(crate) fn clothoid(tau: f64, a: f64) -> [f64; 2] {
    if a == 0.0 {
        return [tau, 0.0];
    }
    let scale = (PI / a.abs()).sqrt();
    let (c, s) = fresnel(tau / scale);
    let im = if a > 0.0 { s } else { -s };
    [scale * c, scale * im]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ∫₀ᵗ f by 20-point Gauss–Legendre on panels of width at most 1/64 (smooth: round-off).
    fn quad(f: impl Fn(f64) -> f64, t: f64) -> f64 {
        let n = ((t.abs() * 64.0).ceil() as usize).max(1);
        let h = t / n as f64;
        (0..n)
            .map(|k| {
                let a = k as f64 * h;
                crate::geometry::boundary::gauss(&f, a, a + h)
            })
            .sum()
    }

    #[test]
    fn tabulated_values() {
        // A&S Table 7.7 and DLMF 7.3; to the digits given
        for (t, c, s) in [
            (0.5, 0.492_344_225_871_446_4, 0.064_732_432_859_999_29),
            (1.0, 0.779_893_400_376_822_8, 0.438_259_147_390_354_8),
            (2.0, 0.488_253_406_075_340_75, 0.343_415_678_363_698_24),
        ] {
            let (cc, ss) = fresnel(t);
            assert!(
                (cc - c).abs() < 3e-16 && (ss - s).abs() < 3e-16,
                "{t}: {cc} {ss}"
            );
        }
        assert_eq!(fresnel(0.0), (0.0, 0.0));
        let (c, s) = fresnel(1e8);
        assert!((c - 0.5).abs() < 1e-8 && (s - 0.5).abs() < 1e-8);
    }

    #[test]
    fn against_quadrature_on_both_sides_of_the_switch() {
        let mut worst = 0.0f64;
        for k in 1..=120 {
            let t = 0.05 * f64::from(k);
            let (c, s) = fresnel(t);
            let qc = quad(|u| (PI / 2.0 * u * u).cos(), t);
            let qs = quad(|u| (PI / 2.0 * u * u).sin(), t);
            worst = worst.max((c - qc).abs()).max((s - qs).abs());
        }
        // (the quadrature's own round-off, summed over its panels, is about 10⁻¹⁵)
        assert!(worst < 1e-14, "{worst}");
        // odd
        let (c, s) = fresnel(-1.3);
        let (c2, s2) = fresnel(1.3);
        assert_eq!((c, s), (-c2, -s2));
    }

    #[test]
    fn the_clothoid() {
        // a clothoid of rate a: its tangent angle is a s²/2
        for a in [0.3, -2.0, 1e-3] {
            let tau = 2.5;
            let z = clothoid(tau, a);
            let qx = quad(|u| (a * u * u / 2.0).cos(), tau);
            let qy = quad(|u| (a * u * u / 2.0).sin(), tau);
            assert!(
                (z[0] - qx).abs() < 2e-15 && (z[1] - qy).abs() < 2e-15,
                "{a}"
            );
        }
    }
}
