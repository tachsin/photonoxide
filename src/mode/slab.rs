//! The guided modes of a three-layer slab waveguide, found exactly.
//!
//! A core of index n₂ and thickness t lies between a lower cladding n₁ (x < 0) and an upper
//! cladding n₃ (x > t); x runs across the slab, and the modes propagate along z as
//! e^(i(βz − ωt)). With k = 2π/λ, a guided mode has max(n₁, n₃)·k < β < n₂·k, and
//!
//! - h = √(n₂²k² − β²) in the core, q = √(β² − n₁²k²) and p = √(β² − n₃²k²) in the claddings;
//! - TE (E along y): tan(ht) = (p + q) / (h (1 − pq/h²));
//! - TM (H along y): tan(ht) = h (p̄ + q̄) / (h² − p̄q̄), with p̄ = (n₂/n₃)² p and q̄ = (n₂/n₁)² q.
//!
//! These are the characteristic equations (3.2-5) and (3.2-11) of A. Yariv, P. Yeh, *Photonics*,
//! 6th ed. (2007), Section 3.2, as L. Chrostowski and M. Hochberg implement them in *Silicon
//! Photonics Design* (2015), [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168),
//! Listings 3.2 and 3.3. Yariv and Yeh put the core at −t < x < 0, with n₁ at x > 0; here x is
//! their −x, with the same n₁, n₂, n₃, p and q. Since
//! tan(a + b) = (tan a + tan b)/(1 − tan a tan b), each equation is the same as
//! h t = m π + atan(q/h) + atan(p/h) (TE; with q̄, p̄ for TM), m = 0, 1, …: the left side falls
//! and the right side rises with β, so mode m has exactly one root, found here by bisection to
//! the last bit.

use std::f64::consts::PI;

use super::Polarization;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// A three-layer slab: a core between two claddings, lossless.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slab {
    below: f64,
    core: f64,
    above: f64,
    thickness: Length,
}

impl Slab {
    /// A core of index `core` and thickness `thickness`, between `below` (x < 0) and `above`
    /// (x > thickness).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the indices are finite and at least 1, the core's is the
    /// highest, and the thickness is positive and finite.
    pub fn new(below: f64, core: f64, above: f64, thickness: Length) -> Result<Slab> {
        let ok = |n: f64| n.is_finite() && n >= 1.0;
        if !ok(below) || !ok(core) || !ok(above) {
            return Err(Error::invalid(
                "slab",
                format!("indices must be finite and at least 1, got {below}, {core}, {above}"),
            ));
        }
        if core <= below || core <= above {
            return Err(Error::invalid(
                "slab",
                format!("the core ({core}) must have the highest index, not {below} and {above}"),
            ));
        }
        let t = thickness.to_um();
        if !(t.is_finite() && t > 0.0) {
            return Err(Error::invalid(
                "slab",
                format!("the thickness must be positive, got {thickness}"),
            ));
        }
        Ok(Slab {
            below,
            core,
            above,
            thickness,
        })
    }

    /// The core's thickness.
    pub fn thickness(&self) -> Length {
        self.thickness
    }

    /// h t − atan(q̄/h) − atan(p̄/h) − mπ at effective index n: positive below mode m's index,
    /// negative above it.
    fn condition(&self, pol: Polarization, k: f64, n: f64, m: usize) -> f64 {
        let (n1, n2, n3) = (self.below, self.core, self.above);
        let h = k * (n2 * n2 - n * n).max(0.0).sqrt();
        let q = k * (n * n - n1 * n1).max(0.0).sqrt();
        let p = k * (n * n - n3 * n3).max(0.0).sqrt();
        let (q, p) = match pol {
            Polarization::Te => (q, p),
            Polarization::Tm => ((n2 / n1).powi(2) * q, (n2 / n3).powi(2) * p),
        };
        h * self.thickness.to_um() - q.atan2(h) - p.atan2(h) - m as f64 * PI
    }

    /// The guided modes of one polarization at `wavelength`, fundamental first (highest
    /// effective index). Empty when the slab guides none (an asymmetric slab below cutoff).
    pub fn modes(&self, polarization: Polarization, wavelength: Wavelength) -> Vec<SlabMode> {
        let k = wavelength.wavenumber();
        let lo = self.below.max(self.above);
        let hi = self.core;
        let mut modes = Vec::new();
        for m in 0.. {
            // mode m exists when its condition is still positive at the lowest guided index
            if self.condition(polarization, k, lo, m) <= 0.0 {
                break;
            }
            let (mut a, mut b) = (lo, hi);
            while b - a > f64::EPSILON * hi {
                let mid = 0.5 * (a + b);
                if mid <= a || mid >= b {
                    break;
                }
                if self.condition(polarization, k, mid, m) > 0.0 {
                    a = mid;
                } else {
                    b = mid;
                }
            }
            let n = 0.5 * (a + b);
            modes.push(SlabMode::new(*self, polarization, m, k, n));
        }
        modes
    }
}

/// A guided mode of a [`Slab`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlabMode {
    slab: Slab,
    polarization: Polarization,
    order: usize,
    effective_index: f64,
    h: f64,
    q: f64,
    p: f64,
}

impl SlabMode {
    fn new(slab: Slab, polarization: Polarization, order: usize, k: f64, n: f64) -> SlabMode {
        SlabMode {
            slab,
            polarization,
            order,
            effective_index: n,
            h: k * (slab.core * slab.core - n * n).max(0.0).sqrt(),
            q: k * (n * n - slab.below * slab.below).max(0.0).sqrt(),
            p: k * (n * n - slab.above * slab.above).max(0.0).sqrt(),
        }
    }

    /// TE or TM.
    pub fn polarization(&self) -> Polarization {
        self.polarization
    }

    /// The mode's order m: 0 for the fundamental mode of its polarization.
    pub fn order(&self) -> usize {
        self.order
    }

    /// The effective index n_eff = β/k.
    pub fn effective_index(&self) -> f64 {
        self.effective_index
    }

    /// The transverse wavenumbers, in 1/µm: h in the core, q below, p above.
    pub fn wavenumbers(&self) -> (f64, f64, f64) {
        (self.h, self.q, self.p)
    }

    /// The field across the slab at x (x = 0 at the core's bottom), and its derivative:
    /// E_y for TE, H_y for TM, unnormalized and equal to 1 (TE) or h/q̄ (TM) at x = 0
    /// (Chrostowski and Hochberg, Listing 3.3).
    pub fn field(&self, x: Length) -> (f64, f64) {
        let x = x.to_um();
        let t = self.slab.thickness.to_um();
        let (h, q, p) = (self.h, self.q, self.p);
        // the field in the core is a cos(hx) + b sin(hx), matched to e^(q̄x) at x = 0
        let (a, b, lower) = match self.polarization {
            Polarization::Te => (1.0, q / h, q),
            Polarization::Tm => {
                let qb = (self.slab.core / self.slab.below).powi(2) * q;
                (h / qb, 1.0, q)
            }
        };
        if x < 0.0 {
            let v = a * (lower * x).exp();
            (v, lower * v)
        } else if x <= t {
            (
                a * (h * x).cos() + b * (h * x).sin(),
                h * (b * (h * x).cos() - a * (h * x).sin()),
            )
        } else {
            let top = a * (h * t).cos() + b * (h * t).sin();
            let v = top * (-p * (x - t)).exp();
            (v, -p * v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lam(um: f64) -> Wavelength {
        Wavelength::um(um).unwrap()
    }

    /// The book's example: 220 nm of silicon (3.473) in oxide (1.444) at 1550 nm.
    fn soi() -> Slab {
        Slab::new(1.444, 3.473, 1.444, Length::nm(220.0)).unwrap()
    }

    #[test]
    fn the_books_slab_gives_its_effective_indices() {
        // Chrostowski & Hochberg, Section 3.2.2: TE 2.845, TM 2.051, printed to 3 decimals
        let te = soi().modes(Polarization::Te, lam(1.55));
        let tm = soi().modes(Polarization::Tm, lam(1.55));
        assert_eq!(
            (te.len(), tm.len()),
            (1, 1),
            "a single TE and TM mode at 220 nm"
        );
        assert!(
            (te[0].effective_index() - 2.845).abs() < 5e-4,
            "{}",
            te[0].effective_index()
        );
        assert!(
            (tm[0].effective_index() - 2.051).abs() < 5e-4,
            "{}",
            tm[0].effective_index()
        );
    }

    /// Asserts the TE effective indices against values printed to 4 decimals.
    fn assert_printed(modes: &[SlabMode], printed: &[f64]) {
        let got: Vec<f64> = modes.iter().map(SlabMode::effective_index).collect();
        assert_eq!(got.len(), printed.len(), "{got:?}");
        for (n, want) in got.iter().zip(printed) {
            assert!((n - want).abs() < 5e-5, "{n} vs the printed {want}");
        }
    }

    #[test]
    fn yariv_and_yehs_asymmetric_slab_gives_their_effective_indices() {
        // Yariv & Yeh, Section 3.2, p. 123 and Figs. 3.8–3.9: n₁ = 1.0, n₂ = 2.0, n₃ = 1.7,
        // t/λ = 1: two TE modes, n_eff = 1.9594 and 1.8375, and two TM modes
        let slab = Slab::new(1.0, 2.0, 1.7, Length::um(1.0)).unwrap();
        assert_printed(&slab.modes(Polarization::Te, lam(1.0)), &[1.9594, 1.8375]);
        assert_eq!(slab.modes(Polarization::Tm, lam(1.0)).len(), 2);
    }

    #[test]
    fn yariv_and_yehs_symmetric_slab_gives_their_effective_indices() {
        // Yariv & Yeh, Section 3.1, pp. 117–118: n₁ = 1.5, n₂ = 1.6, d = 5 µm, λ = 1.55 µm
        // (V = 5.6425): four TE modes, β/(ω/c) = 1.5946, 1.5785, 1.5521 and 1.5175
        let slab = Slab::new(1.5, 1.6, 1.5, Length::um(5.0)).unwrap();
        assert_printed(
            &slab.modes(Polarization::Te, lam(1.55)),
            &[1.5946, 1.5785, 1.5521, 1.5175],
        );
    }

    #[test]
    fn the_roots_satisfy_the_books_tangent_form() {
        // tan(ht) − (p+q)/h/(1 − pq/h²) = 0 for TE, tan(ht) − h(p̄+q̄)/(h² − p̄q̄) = 0 for TM
        let slab = Slab::new(1.0, 3.473, 1.444, Length::nm(600.0)).unwrap();
        let t = 0.6;
        for pol in [Polarization::Te, Polarization::Tm] {
            let modes = slab.modes(pol, lam(1.31));
            assert!(modes.len() >= 2);
            for m in modes {
                let (h, q, p) = m.wavenumbers();
                let rhs = match pol {
                    Polarization::Te => (p + q) / h / (1.0 - p * q / (h * h)),
                    Polarization::Tm => {
                        let (qb, pb) = (3.473f64.powi(2) * q, (3.473f64 / 1.444).powi(2) * p);
                        h * (pb + qb) / (h * h - pb * qb)
                    }
                };
                let lhs = (h * t).tan();
                assert!(
                    (lhs - rhs).abs() < 1e-9 * (1.0 + rhs.abs()),
                    "{pol:?} {}: {lhs} vs {rhs}",
                    m.order()
                );
            }
        }
    }

    #[test]
    fn a_symmetric_slab_guides_one_te_mode_per_pi_of_v() {
        // symmetric slab: TE mode m is guided when V = k t √(n₂² − n₁²) > mπ
        for t_nm in [100.0, 300.0, 700.0, 1500.0] {
            let slab = Slab::new(1.5, 2.0, 1.5, Length::nm(t_nm)).unwrap();
            let v = lam(1.0).wavenumber() * t_nm / 1000.0 * (4.0f64 - 2.25).sqrt();
            let expected = (v / PI).ceil() as usize;
            assert_eq!(
                slab.modes(Polarization::Te, lam(1.0)).len(),
                expected,
                "{t_nm} nm"
            );
        }
    }

    #[test]
    fn an_asymmetric_slab_has_a_cutoff() {
        // below t_c = atan(√(n₃² − n₁²)/√(n₂² − n₃²)) / (k √(n₂² − n₃²)) no TE mode is guided
        let (n1, n2, n3): (f64, f64, f64) = (1.0, 2.0, 1.5);
        let k = lam(1.0).wavenumber();
        let root = (n2 * n2 - n3 * n3).sqrt();
        let t_c = ((n3 * n3 - n1 * n1).sqrt() / root).atan() / (k * root);
        let below = Slab::new(n3, n2, n1, Length::um(0.98 * t_c)).unwrap();
        let above = Slab::new(n3, n2, n1, Length::um(1.02 * t_c)).unwrap();
        assert!(below.modes(Polarization::Te, lam(1.0)).is_empty());
        assert_eq!(above.modes(Polarization::Te, lam(1.0)).len(), 1);
    }

    #[test]
    fn the_fields_meet_the_boundary_conditions() {
        // TE: E_y and dE_y/dx continuous; TM: H_y and (1/n²) dH_y/dx continuous
        let slab = Slab::new(1.444, 3.473, 1.0, Length::nm(400.0)).unwrap();
        let eps = 1e-9;
        for pol in [Polarization::Te, Polarization::Tm] {
            for m in slab.modes(pol, lam(1.55)) {
                for (x, n_in, n_out) in [(0.0, 1.444, 3.473), (0.4, 3.473, 1.0)] {
                    let (v0, d0) = m.field(Length::um(x - eps));
                    let (v1, d1) = m.field(Length::um(x + eps));
                    let scale = v0.abs().max(1.0);
                    assert!((v0 - v1).abs() < 1e-6 * scale, "{pol:?} value at {x}");
                    let (d0, d1) = match pol {
                        Polarization::Te => (d0, d1),
                        Polarization::Tm => (d0 / (n_in * n_in), d1 / (n_out * n_out)),
                    };
                    assert!(
                        (d0 - d1).abs() < 1e-5 * (d0.abs().max(1.0)),
                        "{pol:?} slope at {x}: {d0} vs {d1}"
                    );
                }
            }
        }
    }

    #[test]
    fn modes_come_fundamental_first_and_decay_outside() {
        let slab = Slab::new(1.444, 3.473, 1.444, Length::um(1.0)).unwrap();
        let te = slab.modes(Polarization::Te, lam(1.55));
        assert!(te.len() >= 3);
        assert!(
            te.windows(2)
                .all(|w| w[0].effective_index() > w[1].effective_index())
        );
        assert!(te.iter().enumerate().all(|(i, m)| m.order() == i));
        let (far, _) = te[0].field(Length::um(-2.0));
        let (near, _) = te[0].field(Length::um(-0.1));
        assert!(far.abs() < 1e-3 * near.abs());
        // TE is better confined than TM, so its index is higher
        let tm = slab.modes(Polarization::Tm, lam(1.55));
        assert!(te[0].effective_index() > tm[0].effective_index());
    }

    #[test]
    fn bad_slabs_are_errors() {
        assert!(Slab::new(1.444, 1.4, 1.444, Length::nm(220.0)).is_err());
        assert!(Slab::new(1.444, 3.473, 3.5, Length::nm(220.0)).is_err());
        assert!(Slab::new(0.5, 3.473, 1.444, Length::nm(220.0)).is_err());
        assert!(Slab::new(1.444, f64::NAN, 1.444, Length::nm(220.0)).is_err());
        assert!(Slab::new(1.444, 3.473, 1.444, Length::ZERO).is_err());
    }
}
