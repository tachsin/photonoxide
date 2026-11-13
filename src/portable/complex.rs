//! The complex functions with the same bits on every system, on `Complex64`.
//!
//! num-complex computes `norm`, `exp`, `sqrt` and the rest with std's `hypot`, `exp`, `sin`,
//! `atan2` and so on, the system's maths library: these take the same formulas, or better ones,
//! from [`super`]. `Complex64`'s arithmetic, `norm_sqr`, `inv`, `powi` and `powu` are only
//! `+ − × ÷` and stay as they are.
//!
//! **Accuracy.** Each function's doc gives its error bound, a first-order count of its roundings
//! with u = 2⁻⁵³ and each real function at its own bound (an error of e ulps is a relative 2e·u
//! at most): e.g. `exp(z)`'s real part, `e^a cos b`, carries `exp`'s 1.9u, `cos`'s 1.67u and the
//! product's u, 4.57u. A relative error ρu is below ρ ulps of the true value, so within ⌊ρ + ½⌋
//! binary64 numbers of the correctly rounded one, the number each doc gives ("within 5 ulps").
//! Where a formula is a product, each part is held to its own ulps; where it is a sum that can
//! cancel (`ln`, `powf`, `powc`), the error is normwise, relative to |w|.
//!
//! **Special values** follow C99's Annex G where num-complex does, and keep the sign of a zero
//! part: `exp(x − 0i) = eˣ − 0i`; `sqrt(−4 + 0i) = 2i` and `sqrt(−4 − 0i) = −2i`, the two sides
//! of the branch cut. Each function's doc says what it does at zeros, infinities and NaN.

use num_complex::Complex64 as c64;

/// |z|, `hypot(re, im)`: within 1.21 ulps (as [`super::hypot`]), with no overflow or underflow in
/// between; `inf` if either part is infinite, even with a NaN. As `Complex64::norm`.
#[inline]
#[must_use]
pub fn abs(z: c64) -> f64 {
    super::hypot(z.re, z.im)
}

/// The argument of z, `atan2(im, re)`, in [−π, π]: within 1.55 ulps (as [`super::atan2`]); π for
/// a negative real z with `+0` imaginary part, −π with `−0`. As `Complex64::arg`.
#[inline]
#[must_use]
pub fn arg(z: c64) -> f64 {
    super::atan2(z.im, z.re)
}

/// (|z|, arg z), [`abs`] and [`arg`]. As `Complex64::to_polar`.
#[inline]
#[must_use]
pub fn to_polar(z: c64) -> (f64, f64) {
    (abs(z), arg(z))
}

/// r e^(iθ) = r cos θ + i r sin θ: each part within 3 ulps of r cos θ and r sin θ (cos or sin
/// 1.67u, the product u). No special cases (0 · inf is NaN). As `Complex64::from_polar`.
#[inline]
#[must_use]
pub fn from_polar(r: f64, theta: f64) -> c64 {
    let (s, c) = super::sin_cos(theta);
    c64::new(r * c, r * s)
}

/// e^(iθ) = cos θ + i sin θ, each part within 0.834 ulp (as [`super::cos`] and [`super::sin`]).
/// As `Complex64::cis`.
#[inline]
#[must_use]
pub fn cis(theta: f64) -> c64 {
    let (s, c) = super::sin_cos(theta);
    c64::new(c, s)
}

/// e^z = e^a (cos b + i sin b), z = a + ib: each part within 5 ulps (`exp` 1.9u, `cos` or `sin`
/// 1.67u, the product u: 4.57u).
///
/// A real z gives e^a exactly as [`super::exp`], its zero imaginary part kept with its sign.
/// Infinities and NaN as `Complex64::exp` (C99): e^(−∞ + ib) = 0 for an infinite or NaN b,
/// e^(+∞ ± 0i) = +∞ ± 0i, e^(+∞ + ib) = +∞ + i NaN for an infinite or NaN b, e^(NaN ± 0i) =
/// NaN ± 0i.
#[must_use]
pub fn exp(z: c64) -> c64 {
    let (re, mut im) = (z.re, z.im);
    if re.is_infinite() {
        if re < 0.0 {
            if !im.is_finite() {
                return c64::new(0.0, 0.0);
            }
        } else if im == 0.0 || !im.is_finite() {
            if im.is_infinite() {
                im = f64::NAN;
            }
            return c64::new(re, im);
        }
    } else if re.is_nan() && im == 0.0 {
        return z;
    }
    let r = super::exp(re);
    if im == 0.0 {
        return c64::new(r, im);
    }
    let (s, c) = super::sin_cos(im);
    c64::new(r * c, r * s)
}

/// The principal logarithm, ln|z| + i arg z, its imaginary part in [−π, π]: the branch cut along
/// the negative real axis, `+0` imaginary part above it (π), `−0` below (−π).
///
/// Near |z| = 1, where ln|z| = ln(hypot) loses its digits, the real part is
/// ½ ln_1p((a − 1)(a + 1) + b²), a and b the larger and smaller of |re| and |im| (a − 1 exact).
/// Normwise within 15u of |ln z| (u = 2⁻⁵³), a first-order count: for 0.5 ≤ |z|² ≤ 2 the sum's
/// roundings, 3u|t| + 3u b², are at most 8.7u|ln|z|| + 5.4u|arg z| there, `ln_1p` adds 1.8u and
/// `atan2` 3.1u, 14.8u in all; elsewhere `hypot`'s 2.42u over |ln|z|| ≥ 0.35 and `ln`'s 1.9u,
/// 12.6u. ln 0 = −inf + i arg 0; ln(±inf + ib) = +inf + i arg; NaN parts give NaN, but
/// ln(±inf + i NaN) = +inf + i NaN.
#[must_use]
pub fn ln(z: c64) -> c64 {
    let r = abs(z);
    let theta = arg(z);
    if (std::f64::consts::FRAC_1_SQRT_2..=std::f64::consts::SQRT_2).contains(&r) {
        let (x, y) = (z.re.abs(), z.im.abs());
        let (a, b) = if x >= y { (x, y) } else { (y, x) };
        let t = (a - 1.0) * (a + 1.0) + b * b;
        return c64::new(0.5 * super::ln_1p(t), theta);
    }
    c64::new(super::ln(r), theta)
}

/// The principal square root, its real part ≥ 0: the branch cut along the negative real axis,
/// `+0` imaginary part above it (`sqrt(−4 + 0i) = 2i`), `−0` below (`−2i`).
///
/// Kahan's formulas (W. Kahan, "Branch cuts for complex elementary functions, or much ado about
/// nothing's sign bit", in *The State of the Art in Numerical Analysis*, 1987): t = √((|a| + |z|)/2),
/// then (t, b/(2t)) for a ≥ 0 and (|b|/(2t), ±t) for a < 0, scaled by a power of 4 where |a| + |z|
/// would overflow and by 2⁵⁴ where it would lose digits to underflow; no cancellation. Each part
/// within its own ulps: the part t within 3 (`hypot` 2.42u, the sum u, the square root halves
/// that and adds u: 2.71u), the other within 4 (and the division u: 3.71u).
///
/// Special values (C99): √(a ± 0i) = √a ± 0i for a ≥ 0 (and `+0`), ±i√|a| for a < 0; √(a ± i∞) =
/// +∞ ± i∞ for every a, NaN included; √(+∞ + ib) = +∞ ± 0i and √(−∞ + ib) = 0 ± i∞ for a finite
/// b; NaN otherwise (√(NaN ± 0i) = NaN ± 0i, as num-complex).
#[must_use]
pub fn sqrt(z: c64) -> c64 {
    const BIG: f64 = f64::from_bits(0x7fc0_0000_0000_0000); // 2^1021
    const SMALL: f64 = f64::from_bits(0x0170_0000_0000_0000); // 2^-1000
    const UP: f64 = f64::from_bits(0x4350_0000_0000_0000); // 2^54
    const DOWN: f64 = f64::from_bits(0x3e40_0000_0000_0000); // 2^-27
    let (x, y) = (z.re, z.im);
    if y == 0.0 {
        if x.is_sign_positive() {
            return c64::new(x.sqrt(), y);
        }
        return c64::new(0.0, (-x).sqrt().copysign(y));
    }
    if y.is_infinite() {
        return c64::new(f64::INFINITY, y);
    }
    if x.is_nan() || y.is_nan() {
        return c64::new(f64::NAN, f64::NAN);
    }
    if x.is_infinite() {
        return if x > 0.0 {
            c64::new(x, 0.0f64.copysign(y))
        } else {
            c64::new(0.0, f64::INFINITY.copysign(y))
        };
    }
    let m = x.abs().max(y.abs());
    let (x, y, scale) = if m >= BIG {
        (x * 0.25, y * 0.25, 2.0)
    } else if m < SMALL {
        (x * UP, y * UP, DOWN)
    } else {
        (x, y, 1.0)
    };
    let t = (0.5 * (x.abs() + super::hypot(x, y))).sqrt();
    let (re, im) = if x >= 0.0 {
        (t, y / (2.0 * t))
    } else {
        (y.abs() / (2.0 * t), t.copysign(y))
    };
    c64::new(re * scale, im * scale)
}

/// z to the real power p, `from_polar(|z|^p, p arg z)`, as `Complex64::powf`: the principal
/// branch. 1 for p = 0, whatever z; 0 for z = 0 and p > 0; for z = 0 and p < 0 `inf + i NaN`, as
/// num-complex.
///
/// Normwise, relative to |w|: `powf`'s error, p times `hypot`'s 2.42u, 4.1u |p arg z| (`atan2`
/// 3.1u and the product u, an error in the angle) and `from_polar`'s 2.67u. For p arg z of a few
/// radians and |z| away from overflow, within about 30u.
#[must_use]
pub fn powf(z: c64, p: f64) -> c64 {
    if p == 0.0 {
        return c64::new(1.0, 0.0);
    }
    let (r, theta) = to_polar(z);
    from_polar(super::powf(r, p), theta * p)
}

/// z to the complex power w, `exp(w ln z)`, the principal branch, as `Complex64::powc`. 1 for
/// w = 0, whatever z; 0 for z = 0 and Re w > 0 (num-complex gives NaN there); otherwise what
/// `exp(w ln z)` gives.
///
/// Normwise, relative to |zʷ|: an absolute error δ in the exponent w ln z is a relative δ in the
/// result, and δ ≤ (15u + √5 u)|w||ln z| ([`ln`]'s 15u, and the complex product's √5 u: R. P.
/// Brent, C. Percival and P. Zimmermann, "Error bounds on complex floating-point
/// multiplication", Math. Comp. 76, 1469 (2007)); [`exp`] adds 4.57u per part.
#[must_use]
pub fn powc(z: c64, w: c64) -> c64 {
    if w.re == 0.0 && w.im == 0.0 {
        return c64::new(1.0, 0.0);
    }
    if z.re == 0.0 && z.im == 0.0 && w.re > 0.0 {
        return c64::new(0.0, 0.0);
    }
    exp(w * ln(z))
}

/// sinh z = sinh a cos b + i cosh a sin b, z = a + ib: each part within 6 ulps (`sinh` 3.76u or
/// `cosh` 2.94u, `cos` or `sin` 1.67u, the product u: 6.43u at most).
///
/// A real z (b = ±0) gives sinh a ± 0i, also where cosh a overflows (num-complex gives a NaN
/// imaginary part there); otherwise infinities and NaN as the formula gives them.
#[must_use]
pub fn sinh(z: c64) -> c64 {
    let (a, b) = (z.re, z.im);
    if b == 0.0 {
        return c64::new(super::sinh(a), b);
    }
    let (s, c) = super::sin_cos(b);
    c64::new(super::sinh(a) * c, super::cosh(a) * s)
}

/// cosh z = cosh a cos b + i sinh a sin b: each part within 6 ulps (as [`sinh`]).
///
/// A real z gives cosh a with a zero imaginary part signed as sinh a · b, also where sinh a
/// overflows; otherwise as the formula gives.
#[must_use]
pub fn cosh(z: c64) -> c64 {
    let (a, b) = (z.re, z.im);
    if b == 0.0 {
        let sign = if a.is_nan() { 1.0 } else { a.signum() };
        return c64::new(super::cosh(a), b * sign);
    }
    let (s, c) = super::sin_cos(b);
    c64::new(super::cosh(a) * c, super::sinh(a) * s)
}

/// sin z = sin a cosh b + i cos a sinh b: each part within 6 ulps (as [`sinh`]).
///
/// An imaginary z (a = ±0) gives ±0 + i sinh b, also where cosh b overflows; otherwise as the
/// formula gives.
#[must_use]
pub fn sin(z: c64) -> c64 {
    let (a, b) = (z.re, z.im);
    if a == 0.0 {
        return c64::new(a, super::sinh(b));
    }
    let (s, c) = super::sin_cos(a);
    c64::new(s * super::cosh(b), c * super::sinh(b))
}

/// cos z = cos a cosh b − i sin a sinh b: each part within 6 ulps (as [`sinh`]).
///
/// An imaginary z gives cosh b with a zero imaginary part signed as −a · sinh b, also where
/// sinh b overflows; otherwise as the formula gives.
#[must_use]
pub fn cos(z: c64) -> c64 {
    let (a, b) = (z.re, z.im);
    if a == 0.0 {
        return c64::new(super::cosh(b), -a * b.signum());
    }
    let (s, c) = super::sin_cos(a);
    c64::new(c * super::cosh(b), -s * super::sinh(b))
}

/// tanh z, by Kahan's formulas (W. Kahan, "Branch cuts for complex elementary functions", 1987,
/// as FreeBSD's `ctanh`): with t = tan b, β = 1 + t², s = sinh a and ρ = √(1 + s²),
/// tanh z = (βρs + it)/(1 + βs²), which neither overflows nor cancels; for |a| ≥ 22,
/// ±1 + i 4 sin b cos b e^(−2|a|), the rest below half an ulp.
///
/// The real part within 21 ulps, the imaginary within 16, a first-order count that follows each
/// rounding through the formula (`sinh`'s 3.76u at most twice, β's 6.08u at most once, and the
/// eight roundings of ρ, the products and the quotient); for |a| ≥ 22 the real part is exact and
/// the imaginary within 10.
///
/// Special values (C99): tanh(±∞ + ib) = ±1 ± 0i; tanh(0 + i∞) = 0 + i NaN, other finite a with
/// an infinite or NaN b give NaN; a NaN a gives NaN + ib for b = 0 and NaN otherwise.
#[must_use]
pub fn tanh(z: c64) -> c64 {
    let (a, b) = (z.re, z.im);
    if a.is_nan() {
        return c64::new(a, if b == 0.0 { b } else { f64::NAN });
    }
    if a.is_infinite() {
        let zero = if b.is_finite() {
            let (s, c) = super::sin_cos(b);
            0.0f64.copysign(s * c)
        } else {
            0.0f64.copysign(b)
        };
        return c64::new(1.0f64.copysign(a), zero);
    }
    if !b.is_finite() {
        return if a == 0.0 {
            c64::new(a, f64::NAN)
        } else {
            c64::new(f64::NAN, f64::NAN)
        };
    }
    if a.abs() >= 22.0 {
        let e = super::exp(-a.abs());
        let (s, c) = super::sin_cos(b);
        return c64::new(1.0f64.copysign(a), 4.0 * s * c * e * e);
    }
    let t = super::tan(b);
    let beta = 1.0 + t * t;
    let s = super::sinh(a);
    let ss = s * s;
    let rho = (1.0 + ss).sqrt();
    let denominator = 1.0 + beta * ss;
    c64::new((beta * rho * s) / denominator, t / denominator)
}

/// tan z = −i tanh(iz) ([`tanh`]): the real part within 16 ulps, the imaginary within 21; for
/// |Im z| ≥ 22 the imaginary part is exactly ±1. Special values follow tanh's through the
/// rotation.
#[must_use]
pub fn tan(z: c64) -> c64 {
    let w = tanh(c64::new(-z.im, z.re));
    c64::new(w.im, -w.re)
}

/// The principal arcsine, its real part in [−π/2, π/2]: branch cuts along the real axis beyond
/// ±1, `+0` imaginary part above them, `−0` below (asin(2 ± 0i) = π/2 ± 1.317i).
///
/// The algorithm of T. E. Hull, T. F. Fairgrieve and P. T. P. Tang, "Implementing the complex
/// arcsine and arccosine functions using exception handling", ACM Trans. Math. Softw. 23, 299
/// (1997), on the first quadrant (the rest by asin(−z) = −asin z and asin(z̄) = conj asin z):
/// with r = |z + 1|, s = |z − 1|, A = (r + s)/2 and B = x/A, the real part is asin B for
/// B ≤ 0.6417 and an arctangent of sums of positive terms above; the imaginary part ln_1p of
/// positive terms for A ≤ 1.5 and ln(A + √(A² − 1)) above. For |z| < 2⁻²⁷, z itself (the cube term
/// is below u); for a part beyond 2⁵⁰⁰, atan2 and ln|z| + ln 2; for an imaginary part below 2⁻⁵⁰⁰,
/// the limits asin x + iy/√(1 − x²), π/2 + i√y at x = 1 and π/2 + i acosh x beyond.
///
/// The real part within 11 ulps and the imaginary within 14: a first-order count of each branch,
/// each sum of positive terms adding u and each call at its bound (the largest, A > 1.5, has
/// A² − 1 with a condition ≤ 1.8).
///
/// Special values (C99): asin(±0 ± 0i) = ±0 ± 0i; an infinite part gives an infinite imaginary
/// part and the angle of (|x|, |y|) as the real part, π/2, π/4 or 0; a NaN part gives NaN, but
/// asin(±0 + i NaN) = ±0 + i NaN and asin(NaN ± i∞) = NaN ± i∞.
#[must_use]
pub fn asin(z: c64) -> c64 {
    const LARGE: f64 = f64::from_bits(0x5f30_0000_0000_0000); // 2^500
    const TINY: f64 = f64::from_bits(0x20b0_0000_0000_0000); // 2^-500
    const SMALL: f64 = f64::from_bits(0x3e40_0000_0000_0000); // 2^-27
    const B_CROSSOVER: f64 = 0.6417;
    const A_CROSSOVER: f64 = 1.5;
    let (x, y) = (z.re.abs(), z.im.abs());
    if x.is_nan() || y.is_nan() {
        return if y.is_infinite() {
            c64::new(f64::NAN, z.im)
        } else if x == 0.0 {
            c64::new(z.re, f64::NAN)
        } else if x.is_infinite() {
            c64::new(f64::NAN, f64::INFINITY)
        } else {
            c64::new(f64::NAN, f64::NAN)
        };
    }
    let (re, im) = if x.max(y) < SMALL {
        (x, y)
    } else if x.max(y) >= LARGE {
        (
            super::atan2(x, y),
            super::ln(super::hypot(x, y)) + std::f64::consts::LN_2,
        )
    } else if y < TINY {
        if x < 1.0 {
            (super::asin(x), y / ((1.0 - x) * (1.0 + x)).sqrt())
        } else if x == 1.0 {
            (std::f64::consts::FRAC_PI_2, y.sqrt())
        } else {
            (std::f64::consts::FRAC_PI_2, super::acosh(x))
        }
    } else {
        let xp1 = x + 1.0;
        let r = super::hypot(xp1, y);
        let s = super::hypot(x - 1.0, y);
        let a = 0.5 * (r + s);
        let b = x / a;
        let yy = y * y;
        let re = if b <= B_CROSSOVER {
            super::asin(b)
        } else {
            let apx = a + x;
            if x <= 1.0 {
                super::atan(x / (0.5 * apx * (yy / (r + xp1) + (s + (1.0 - x)))).sqrt())
            } else {
                super::atan(x / (y * (0.5 * (apx / (r + xp1) + apx / (s + (x - 1.0)))).sqrt()))
            }
        };
        let im = if a <= A_CROSSOVER {
            let am1 = if x < 1.0 {
                0.5 * (yy / (r + xp1) + yy / (s + (1.0 - x)))
            } else {
                0.5 * (yy / (r + xp1) + (s + (x - 1.0)))
            };
            super::ln_1p(am1 + (am1 * (a + 1.0)).sqrt())
        } else {
            super::ln(a + (a * a - 1.0).sqrt())
        };
        (re, im)
    };
    c64::new(re.copysign(z.re), im.copysign(z.im))
}
