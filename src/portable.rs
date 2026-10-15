//! exp, sin and cos with the same bits on every system.
//!
//! `f64::exp`, `sin` and `cos` call the system's maths library: glibc on Linux, the Microsoft C
//! runtime on Windows, Apple's libm on macOS. Each is within an ulp or so of the true value, but
//! they round differently, and where a result hinges on the last bit it moves between systems
//! (issue #280): with them, an optimizer on a circuit stopped after 78 evaluations on Linux, 80
//! on Windows and 81 on macOS, and a matrix pencil on an FDTD run's probes turned those ulps into
//! 1e-11 of a frequency. The functions here come from the [`libm`](https://docs.rs/libm) crate, a
//! pure-Rust port of musl's libm (itself FreeBSD's msun, from Sun's fdlibm): basic
//! floating-point operations only, which IEEE 754 makes exact, so they give the same bits
//! everywhere, within an ulp of the true value.
//!
//! They are used where a printed result depends on the last bit: the circuit's closed-form
//! S-matrices (what its optimizations run on) and FDTD's source waveforms. Elsewhere the
//! system's functions stay, and no output shows their difference (docs/methods/conventions.md,
//! "The same numbers on every system").

use num_complex::Complex64 as c64;

/// e^x.
pub(crate) fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// sin x and cos x.
pub(crate) fn sin_cos(x: f64) -> (f64, f64) {
    libm::sincos(x)
}

/// e^z, as `Complex64::exp`: e^re (cos im + i sin im), real for a real z.
pub(crate) fn exp_c(z: c64) -> c64 {
    let r = exp(z.re);
    if z.im == 0.0 {
        return c64::new(r, 0.0);
    }
    let (s, c) = sin_cos(z.im);
    c64::new(r * c, r * s)
}

/// cos z = cos a cosh b − i sin a sinh b, as `Complex64::cos`.
pub(crate) fn cos_c(z: c64) -> c64 {
    let (s, c) = sin_cos(z.re);
    c64::new(c * libm::cosh(z.im), -s * libm::sinh(z.im))
}

/// sin z = sin a cosh b + i cos a sinh b, as `Complex64::sin`.
pub(crate) fn sin_c(z: c64) -> c64 {
    let (s, c) = sin_cos(z.re);
    c64::new(s * libm::cosh(z.im), c * libm::sinh(z.im))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4000 arguments over the ranges the circuit's phases and the waveforms' envelopes take.
    fn arguments() -> impl Iterator<Item = f64> {
        (0..4000).map(|k| {
            let t = f64::from(k) / 4000.0;
            (t - 0.5) * 2000.0 * (1.0 + 0.37 * (17.0 * t).fract())
        })
    }

    #[test]
    fn they_are_within_an_ulp_or_two_of_the_systems() {
        let close = |a: f64, b: f64| (a - b).abs() <= 4.0 * f64::EPSILON * b.abs().max(1e-300);
        for x in arguments() {
            let (s, c) = sin_cos(x);
            assert!(close(s, x.sin()) || (s - x.sin()).abs() < 1e-15, "sin {x}");
            assert!(close(c, x.cos()) || (c - x.cos()).abs() < 1e-15, "cos {x}");
            let e = x / 2000.0 * 700.0;
            assert!(close(exp(e), e.exp()), "exp {e}");
            let z = c64::new(x / 2000.0, x / 300.0);
            for (ours, theirs) in [
                (exp_c(z), z.exp()),
                (cos_c(z), z.cos()),
                (sin_c(z), z.sin()),
            ] {
                assert!(
                    (ours - theirs).norm() <= 1e-15 * theirs.norm().max(1.0),
                    "{z}"
                );
            }
        }
        assert_eq!(exp_c(c64::new(0.5, 0.0)), c64::new(exp(0.5), 0.0));
    }

    /// Their bits, folded into one word: the same on Linux, Windows and macOS (CI tests all
    /// three), where the system's functions' are not.
    #[test]
    fn they_give_the_same_bits_on_every_system() {
        let mut h = 0u64;
        let mut fold = |v: f64| h = h.rotate_left(7) ^ v.to_bits();
        for x in arguments() {
            let (s, c) = sin_cos(x);
            fold(s);
            fold(c);
            fold(exp(x / 2000.0 * 700.0));
            let z = c64::new(x / 2000.0, x / 300.0);
            for w in [exp_c(z), cos_c(z), sin_c(z)] {
                fold(w.re);
                fold(w.im);
            }
        }
        assert_eq!(h, 0xbac4_8fd0_05df_2b48, "{h:#018x}");
    }
}
