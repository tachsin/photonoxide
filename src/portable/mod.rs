//! The maths functions with the same bits on every system.
//!
//! `f64::exp`, `sin`, `powf`, `powi`, `hypot` and the rest, and num-complex's `Complex::norm`,
//! `exp`, `sqrt` and the rest, call the system's maths library: glibc on Linux, the Microsoft C
//! runtime on Windows, Apple's libm on macOS. Each is within an ulp or so of the true value, but
//! they round differently, and where a result hinges on the last bit it moves between systems
//! (issues #280 and #822): an optimizer on a circuit stopped after 78 evaluations on Linux, 80 on
//! Windows and 81 on macOS, and block QMR took 2412, 2222 and 2467 iterations, because its Givens
//! rotation takes `Complex::norm`, the system's `hypot`. `f64::powi` is square and multiply on
//! Linux and the C runtime's `pow` on Windows: a third of the powers differ in their last bit.
//!
//! The functions here use only `+ − × ÷` and `sqrt`, which IEEE 754 rounds exactly, in a fixed
//! order, so they give the same bits on every system and every CPU. `exp` and `ln` are ports of
//! fdlibm (genoxide's portable maths); `powi` is square and multiply written out; `erfcx` is
//! genoxide's; the rest of the real functions come from the [`libm`](https://docs.rs/libm) crate,
//! a pure-Rust port of musl's libm (itself FreeBSD's msun, from Sun's fdlibm), whose only
//! per-architecture code is `sqrt`, `fma` and `rint`, all exact. The complex functions,
//! [`complex`], are built on these. `sqrt` itself needs no replacement: IEEE 754 requires it
//! correctly rounded, so `f64::sqrt` is the same everywhere.
//!
//! **The rule** (CONTRIBUTING.md, "The same numbers on every system"): compute with `+ − × ÷` and
//! `sqrt`, and take every other function from here. `clippy.toml` forbids the system's versions.
//!
//! **Accuracy.** Each function's doc gives its largest error in ulps (units in the last place of
//! the result) and where the number comes from. For the functions from fdlibm and musl it is the
//! largest error known for OpenLibm 0.8.7 and musl 1.2.6, the same algorithms (B. Gladman,
//! V. Innocente, J. Mather, K. Ozaki and P. Zimmermann, "Accuracy of Mathematical Functions in
//! Single, Double, Double Extended, and Quadruple Precision", August 2026,
//! <https://members.loria.fr/PZimmermann/papers/accuracy.pdf>, Table 3; the larger of the two
//! columns); for the others, a first-order count of their roundings, each call at its own bound.
//! The tests hold each function to its number against mpmath's correctly rounded values
//! (`tests/data/canary/portable.py`), and hash every function's bits over a fixed grid of arguments:
//! the same hash on Linux, Windows and macOS.
//!
//! **Speed.** About the system's: `sin`, `cos` and `exp` 1.5 to 3.5 ns slower per call on
//! Windows, `powi`, `hypot` and `cbrt` faster, `powf` about three times slower (20 ns).
//!
//! **Self-contained.** This module uses std, `libm` and `num-complex` only, never `crate::`, so it
//! can become a crate of its own with a `git mv` (and its reference values,
//! `tests/data/canary/portable.*`) and a manifest; the layering test checks it.
//!
//! The functions `exp` and `ln` are adapted from genoxide's `math` module (MIT OR Apache-2.0, as
//! photonoxide), itself a port of fdlibm's `e_exp.c` and `e_log.c`, which carry this notice:
//!
//! > Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//! >
//! > Developed at SunPro, a Sun Microsystems, Inc. business.
//! > Permission to use, copy, modify, and distribute this
//! > software is freely granted, provided that this notice
//! > is preserved.

#![forbid(unsafe_code)]

pub mod complex;

#[cfg(test)]
mod tests;

/// `e` to the power `x`, within 0.949 ulp (fdlibm's `e_exp.c`; Gladman et al., OpenLibm).
/// Overflows to infinity above about 709.78 and underflows to 0 below about −745.13; NaN for
/// NaN.
///
/// fdlibm's `__ieee754_exp`, ported in genoxide's `math`: the same bits as `libm::exp` (the tests
/// check it), without its checks for the rounding mode.
#[must_use]
pub fn exp(x: f64) -> f64 {
    // fdlibm's constants, by their exact bits
    const O_THRESHOLD: f64 = f64::from_bits(0x4086_2e42_fefa_39ef); // 7.09782712893383973096e2
    const U_THRESHOLD: f64 = f64::from_bits(0xc087_4910_d52d_3051); // -7.45133219101941108420e2
    const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fee0_0000); // 6.93147180369123816490e-1
    const LN2_LO: f64 = f64::from_bits(0x3dea_39ef_3579_3c76); // 1.90821492927058770002e-10
    const INV_LN2: f64 = f64::from_bits(0x3ff7_1547_652b_82fe); // 1.44269504088896338700
    const TWO_M1000: f64 = f64::from_bits(0x0170_0000_0000_0000); // 2^-1000
    const P1: f64 = f64::from_bits(0x3fc5_5555_5555_553e); // 1.66666666666666019037e-1
    const P2: f64 = f64::from_bits(0xbf66_c16c_16be_bd93); // -2.77777777770155933842e-3
    const P3: f64 = f64::from_bits(0x3f11_566a_af25_de2c); // 6.61375632143793436117e-5
    const P4: f64 = f64::from_bits(0xbebb_bd41_c5d2_6bf1); // -1.65339022054652515390e-6
    const P5: f64 = f64::from_bits(0x3e66_3769_72be_a4d0); // 4.13813679705723846039e-8

    let high = (x.to_bits() >> 32) as u32;
    let negative = high >> 31 == 1;
    let high = high & 0x7fff_ffff;
    if high >= 0x4086_2e42 {
        // |x| >= 709.78...
        if x.is_nan() {
            return x;
        }
        if x > O_THRESHOLD {
            return f64::INFINITY;
        }
        if x < U_THRESHOLD {
            return 0.0;
        }
    }
    // argument reduction: x = k ln2 + r, |r| <= 0.5 ln2
    let (k, hi, lo, x) = if high > 0x3fd6_2e42 {
        // |x| > 0.5 ln2
        let (k, hi, lo) = if high < 0x3ff0_a2b2 {
            // and |x| < 1.5 ln2
            if negative {
                (-1, x + LN2_HI, -LN2_LO)
            } else {
                (1, x - LN2_HI, LN2_LO)
            }
        } else {
            let k = (INV_LN2 * x + if negative { -0.5 } else { 0.5 }) as i32;
            let t = f64::from(k);
            // t * LN2_HI is exact here
            (k, x - t * LN2_HI, t * LN2_LO)
        };
        (k, hi, lo, hi - lo)
    } else if high < 0x3e30_0000 {
        // |x| < 2^-28
        return 1.0 + x;
    } else {
        (0, 0.0, 0.0, x)
    };
    // x is now in the primary range
    let t = x * x;
    let c = x - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    if k == 0 {
        return 1.0 - ((x * c) / (c - 2.0) - x);
    }
    // fdlibm's order of operations, for fdlibm's results
    let y = 1.0 - ((lo - (x * c) / (2.0 - c)) - hi);
    // multiply by 2^k through the exponent bits
    if k >= -1021 {
        f64::from_bits(y.to_bits().wrapping_add((i64::from(k) << 52) as u64))
    } else {
        f64::from_bits(y.to_bits().wrapping_add((i64::from(k + 1000) << 52) as u64)) * TWO_M1000
    }
}

/// The natural logarithm of `x`, within 0.946 ulp (fdlibm's `e_log.c`; Gladman et al., OpenLibm):
/// `-inf` for ±0, NaN for negative `x` and NaN, `inf` for `inf`.
///
/// fdlibm's `__ieee754_log` as FreeBSD and OpenLibm have it, ported in genoxide's `math`. The
/// `libm` crate's `log` is musl's shorter rewrite of it, whose bits differ at times (within one
/// binary64 number, the tests check), so this one is the port.
#[must_use]
pub fn ln(x: f64) -> f64 {
    // fdlibm's constants, by their exact bits
    const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fee0_0000); // 6.93147180369123816490e-1
    const LN2_LO: f64 = f64::from_bits(0x3dea_39ef_3579_3c76); // 1.90821492927058770002e-10
    const TWO54: f64 = f64::from_bits(0x4350_0000_0000_0000); // 1.80143985094819840000e16
    const LG1: f64 = f64::from_bits(0x3fe5_5555_5555_5593); // 6.666666666666735130e-1
    const LG2: f64 = f64::from_bits(0x3fd9_9999_9997_fa04); // 3.999999999940941908e-1
    const LG3: f64 = f64::from_bits(0x3fd2_4924_9422_9359); // 2.857142874366239149e-1
    const LG4: f64 = f64::from_bits(0x3fcc_71c5_1d8e_78af); // 2.222219843214978396e-1
    const LG5: f64 = f64::from_bits(0x3fc7_4664_96cb_03de); // 1.818357216161805012e-1
    const LG6: f64 = f64::from_bits(0x3fc3_9a09_d078_c69f); // 1.531383769920937332e-1
    const LG7: f64 = f64::from_bits(0x3fc2_f112_df3e_5244); // 1.479819860511658591e-1
    const MIN_POSITIVE_BITS: u64 = 0x0010_0000_0000_0000;
    const INFINITY_BITS: u64 = 0x7ff0_0000_0000_0000;
    let mut x = x;
    let mut k: i32 = 0;
    // one comparison for the common case, positive, normal and finite x (whose bits are from
    // those of f64::MIN_POSITIVE up to those of infinity), and fdlibm's special cases otherwise
    if x.to_bits().wrapping_sub(MIN_POSITIVE_BITS) >= INFINITY_BITS - MIN_POSITIVE_BITS {
        if x.is_nan() || x < 0.0 {
            return f64::NAN;
        }
        if x == 0.0 {
            return f64::NEG_INFINITY;
        }
        if x == f64::INFINITY {
            return x;
        }
        // subnormal: scale up
        k -= 54;
        x *= TWO54;
    }
    let mut high = (x.to_bits() >> 32) as i32;
    k += (high >> 20) - 1023;
    high &= 0x000f_ffff;
    let i = (high + 0x95f64) & 0x10_0000;
    // normalize x or x / 2 into [sqrt(2) / 2, sqrt(2))
    let normalized_high = (high | (i ^ 0x3ff0_0000)) as u32;
    x = f64::from_bits((u64::from(normalized_high) << 32) | (x.to_bits() & 0xffff_ffff));
    k += i >> 20;
    let f = x - 1.0;
    let dk = f64::from(k);
    if (0x000f_ffff & (2 + high)) < 3 {
        // |f| < 2^-20
        if f == 0.0 {
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r = f * f * (0.5 - (1.0 / 3.0) * f);
        return if k == 0 {
            f - r
        } else {
            dk * LN2_HI - ((r - dk * LN2_LO) - f)
        };
    }
    let s = f / (2.0 + f);
    let z = s * s;
    let w = z * z;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    let r = t2 + t1;
    let i = (high - 0x6147a) | (0x6b851 - high);
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

// The functions taken from the `libm` crate as they are, with their docs.
macro_rules! from_libm {
    ($($(#[$doc:meta])* $name:ident = $libm:ident($($arg:ident),+);)+) => {
        $(
            $(#[$doc])*
            #[inline]
            #[must_use]
            pub fn $name($($arg: f64),+) -> f64 {
                libm::$libm($($arg),+)
            }
        )+
    };
}

from_libm! {
    /// `2` to the power `x`, within 0.751 ulp (FreeBSD's `s_exp2.c`; Gladman et al., OpenLibm).
    exp2 = exp2(x);
    /// `e^x − 1`, accurate near 0 where `exp(x) − 1` cancels, within 0.909 ulp (fdlibm's
    /// `s_expm1.c`; Gladman et al.). −1 for `-inf`.
    exp_m1 = expm1(x);
    /// `ln(1 + x)`, accurate near 0 where `ln(1 + x)` loses `x`, within 0.900 ulp (fdlibm's
    /// `s_log1p.c`; Gladman et al., musl). `-inf` for −1, NaN below.
    ln_1p = log1p(x);
    /// The base-2 logarithm of `x`, within 0.921 ulp (FreeBSD's `e_log2.c`; Gladman et al.,
    /// OpenLibm).
    log2 = log2(x);
    /// The base-10 logarithm of `x`, within 0.814 ulp (fdlibm's `e_log10.c`; Gladman et al.).
    log10 = log10(x);
    /// The sine of `x` (radians), within 0.831 ulp (fdlibm's `s_sin.c`, with an exact argument
    /// reduction for any `x`; Gladman et al.).
    sin = sin(x);
    /// The cosine of `x` (radians), within 0.834 ulp (fdlibm's `s_cos.c`; Gladman et al.).
    cos = cos(x);
    /// The tangent of `x` (radians), within 1.02 ulps (fdlibm's `s_tan.c`; Gladman et al.).
    tan = tan(x);
    /// The arcsine of `x`, in [−π/2, π/2], within 0.981 ulp (fdlibm's `e_asin.c`; Gladman et
    /// al.); NaN outside [−1, 1].
    asin = asin(x);
    /// The arccosine of `x`, in [0, π], within 0.930 ulp (fdlibm's `e_acos.c`; Gladman et al.);
    /// NaN outside [−1, 1].
    acos = acos(x);
    /// The arctangent of `x`, in [−π/2, π/2], within 0.861 ulp (fdlibm's `s_atan.c`; Gladman et
    /// al.).
    atan = atan(x);
    /// The angle of the point (`x`, `y`), in [−π, π], as `y.atan2(x)`, within 1.55 ulps
    /// (fdlibm's `e_atan2.c`; Gladman et al.), with C's special cases for zeros and infinities.
    atan2 = atan2(y, x);
    /// `√(x² + y²)` without overflow or underflow in between, within 1.21 ulps (musl's `hypot.c`;
    /// Gladman et al., OpenLibm; musl's 1.04): `inf` if either is infinite, even with a NaN.
    hypot = hypot(x, y);
    /// The hyperbolic sine of `x`, within 1.88 ulps (musl's `sinh.c`; Gladman et al.).
    sinh = sinh(x);
    /// The hyperbolic cosine of `x`, within 1.47 ulps (musl's `cosh.c`; Gladman et al.,
    /// OpenLibm; musl's 1.04).
    cosh = cosh(x);
    /// The hyperbolic tangent of `x`, within 2.22 ulps (musl's `tanh.c`; Gladman et al.).
    tanh = tanh(x);
    /// The inverse hyperbolic sine of `x`, within 1.92 ulps (musl's `asinh.c`; Gladman et al.).
    asinh = asinh(x);
    /// The inverse hyperbolic tangent of `x`, within 1.81 ulps (musl's `atanh.c`; Gladman et
    /// al.); ±inf at ±1, NaN outside [−1, 1].
    atanh = atanh(x);
    /// The cube root of `x`, correctly rounded (the `libm` crate's port of CORE-MATH's `cbrt`,
    /// which is proved so).
    cbrt = cbrt(x);
    /// The error function `erf(x) = 2/√π ∫₀ˣ e^(−t²) dt`, within 1.02 ulps (fdlibm's `s_erf.c`;
    /// Gladman et al.).
    erf = erf(x);
    /// The complementary error function `erfc(x) = 1 − erf(x)`, accurate where `1 − erf(x)`
    /// would cancel (large `x`), within 4.08 ulps (fdlibm's `s_erf.c`; Gladman et al.,
    /// OpenLibm; musl's 3.72). It underflows to 0 above about 26.5; [`erfcx`] keeps its scale.
    erfc = erfc(x);
}

/// `base` to the power `exponent`, with the special cases of C's `pow` (as
/// `base.powf(exponent)`): fdlibm's `e_pow.c`, whose results its comments call "nearly rounded",
/// within an ulp. Not everywhere: near `base` = 1 with a huge `exponent` its log₂ of the base
/// loses digits that the exponent multiplies, and the largest error known is 636 ulps (Gladman et
/// al., OpenLibm, the same code: base 1 + 1.65e-7, exponent 3.85e9, where this gives 635 ulps;
/// the tests hold it to 636 within 2⁻¹⁰ of 1 for exponents above 10⁶, to 1 elsewhere). For an
/// integer exponent [`powi`] is faster and exact to its stated bound.
#[inline]
#[must_use]
pub fn powf(base: f64, exponent: f64) -> f64 {
    libm::pow(base, exponent)
}

/// The inverse hyperbolic cosine of `x`, within 2.25 ulps (musl's `acosh.c`; Gladman et al.,
/// OpenLibm); NaN below 1.
///
/// musl's `acosh` returns a number for some `x` below −2 (Gladman et al., musl: 8.13 for
/// −1.26e6), so this one returns NaN below 1 itself.
#[inline]
#[must_use]
pub fn acosh(x: f64) -> f64 {
    if x < 1.0 {
        return f64::NAN;
    }
    libm::acosh(x)
}

/// The sine and cosine of `x` (radians), as `x.sin_cos()`: [`sin`] and [`cos`] with one argument
/// reduction, the same bits as each.
#[inline]
#[must_use]
pub fn sin_cos(x: f64) -> (f64, f64) {
    libm::sincos(x)
}

/// `x` to the integer power `n`, by square and multiply in a fixed order.
///
/// `f64::powi` is square and multiply on Linux (compiler-builtins' `__powidf2`) but the C
/// runtime's `pow` on Windows, and a third of the powers differ in their last bit between the
/// two. This is the first, written out, so that the order of the rounded multiplications is the
/// same everywhere: `x^|n|` from the binary digits of `|n|`, then `1 / x^|n|` for `n < 0`.
///
/// Accuracy: exact for `n` in −1..=2 (a correctly rounded division or product); otherwise within
/// `|n| − 1 + [n < 0]` ulps, since any chain of products giving `x^m` has a relative error below
/// (m − 1)u to first order (N. J. Higham, *Accuracy and Stability of Numerical Algorithms*, 2nd ed.,
/// SIAM, 2002, §3.1; u = 2⁻⁵³), and the division adds u.
///
/// Special cases: `powi(x, 0) = 1` for every `x`, NaN and 0 included (so `0⁰ = 1`); `powi(±0, n)`
/// is `±inf` for odd `n < 0` and `inf` for even `n < 0`; a NaN `x` gives NaN for `n ≠ 0`.
/// Intermediate powers can overflow (to `inf`, then 0 for `n < 0`) where `x^n` itself would just
/// fit.
#[inline]
#[must_use]
pub fn powi(x: f64, n: i32) -> f64 {
    let mut base = x;
    let mut exponent = n.unsigned_abs();
    let mut result = 1.0;
    loop {
        if exponent & 1 == 1 {
            result *= base;
        }
        exponent >>= 1;
        if exponent == 0 {
            break;
        }
        base *= base;
    }
    if n < 0 { 1.0 / result } else { result }
}

/// The scaled complementary error function `erfcx(x) = e^(x²) erfc(x)`: finite where `erfc`
/// underflows, about `1 / (x √π)` for large `x`. For logarithms of normal tail probabilities,
/// e.g. `ln Φ(z) = ln(erfcx(−z/√2) / 2) − z²/2` for very negative `z`. genoxide's.
///
/// Up to 26 (where `erfc` is still a normal number) it's `e^(x²) · erfc(x)`, with `x²` split
/// exactly into two parts so that `e^(x²)` is as accurate as `exp`; above, the first 9 terms of
/// the asymptotic series `1 / (x √π) · Σ (−1)ⁿ (2n − 1)!! / (2x²)ⁿ` (Abramowitz and Stegun,
/// 1964, 7.1.23), whose remainder there is below 1e-19. Below 0, `2 e^(x²) − erfcx(−x)`, which
/// overflows to infinity below about −26.6.
///
/// Accuracy, a first-order count with u = 2⁻⁵³ and each call at its bound (an error of e ulps
/// is a relative 2e·u at most): from 0 to 26, `exp` 1.9u, the split's correction u, `erfc` 8.2u
/// and the product u, 12.1u, so within 12 ulps; below 0, `2 e^(x²)` (2.9u) is at most twice the
/// result and `erfcx(−x)` (12.1u) at most the result, and the subtraction adds u, 18.9u: within
/// 19 ulps; above 26, the series' Horner steps (2u, as t < 1/1352), `1/x`, the constant and two
/// products, 6u: within 6 ulps.
#[must_use]
pub fn erfcx(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < 0.0 {
        return 2.0 * exp_square(x) - erfcx(-x);
    }
    if x < 26.0 {
        return exp_square(x) * erfc(x);
    }
    // the asymptotic series in t = 1 / (2x²), by Horner's rule: 1 − t + 3t² − 15t³ + ...; 1 / x
    // first, so that x² can't overflow
    let inverse = 1.0 / x;
    let t = 0.5 * inverse * inverse;
    const DOUBLE_FACTORIALS: [f64; 9] = [
        1.0,
        1.0,
        3.0,
        15.0,
        105.0,
        945.0,
        10_395.0,
        135_135.0,
        2_027_025.0,
    ];
    let mut sum = 0.0;
    for (n, factorial) in DOUBLE_FACTORIALS.iter().enumerate().rev() {
        let term = if n.is_multiple_of(2) {
            *factorial
        } else {
            -factorial
        };
        sum = sum * t + term;
    }
    inverse * std::f64::consts::FRAC_2_SQRT_PI * 0.5 * sum
}

// e^(x²), with x² split exactly into hi + lo (Dekker's product), so that the rounding of x²
// doesn't cost accuracy: e^(hi + lo) = e^hi (1 + lo) to within lo², far below an ulp
fn exp_square(x: f64) -> f64 {
    const SPLIT: f64 = 134_217_729.0; // 2^27 + 1
    let c = SPLIT * x;
    let high = c - (c - x);
    let low = x - high;
    let square = x * x;
    let error = ((high * high - square) + 2.0 * high * low) + low * low;
    let e = exp(square);
    if e.is_infinite() {
        return e;
    }
    e + e * error
}
