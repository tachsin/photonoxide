//! The thermo-optic phase shifter's closed forms, its analytic fidelity (docs/methods/devices.md,
//! "Thermo-optic phase shifters").
//!
//! **The oxide as a slab** ([`Slab`]): thermally all oxide (the guide's silicon, the slab's and
//! the heater's own conductivity left out) between the substrate, isothermal (its own silicon
//! taken as the oxide of the same thermal resistance), and an insulated top (the air's
//! 10 W/(m²·K) carries 4e-5 of what the oxide does), laterally unbounded. The heaters are sheets
//! at their height ([`Sheets`]: a metal strip at its mid-height, or the doped strips at the
//! slab's), each its share of the power spread evenly across its width. In the slab's
//! eigenfunctions φ_m(y) = √(2/H) sin(k_m y), k_m = (m + ½)π/H, each term of the temperature
//! obeys −k(u″ − p_m² u) = q_m(x), whose Green's function across is e^(−p_m|x|)/(2k p_m), and
//! a strip's integral of it is closed:
//!
//! - **steady,** p_m = k_m: the rise at (x, y), Σ_m φ_m(y) φ_m(s) Σ_j s_j J(k_m)/(2k k_m w_j);
//! - **at a frequency ω,** p_m² = k_m² + iω/D: its response over the steady rise, the steady
//!   series plus the terms' differences, which fall as 1/m⁴ ([`Slab::response`], its −3 dB
//!   frequency [`Slab::cutoff`]);
//! - **in time,** the step response u_m(t) = u_m(∞) − (1/ρc) ∫_t^∞ e^(−Dk_m²τ) W(x, τ) dτ, W the
//!   strips' heat kernel e^(−x²/4Dτ)/√(4πDτ) integrated across them (a difference of erfs), the
//!   integral by Simpson's rule in ln τ ([`Slab::one_over_e`]).
//!
//! **The mode's dn_eff/dT** ([`thermo_optic`]) by Marcatili's rectangle (E^x₁₁, the TE-like
//! mode, solved exactly: [`crate::mode::marcatili`]), its core's and cladding's indices moved by
//! their dn/dT. The rise is taken at the guide's centre.
//!
//! The elementary functions are the `libm` crate's, the same bits on every system
//! ([`crate::portable`]).

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use crate::mode::marcatili::{Family, Rectangle};
use crate::portable::exp_c;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// What heats the slab: strips across at one height, each (centre, width, share of the power),
/// m; the shares sum to 1.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Sheets {
    pub(super) height: f64,
    pub(super) strips: Vec<(f64, f64, f64)>,
}

/// The oxide between an isothermal plane at y = 0 and an insulated top at y = `height` (m), of
/// conductivity `conductivity` (W/(m·K)) and diffusivity `diffusivity` (m²/s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Slab {
    pub(super) height: f64,
    pub(super) conductivity: f64,
    pub(super) diffusivity: f64,
}

/// √(a + ib) for a > 0, its real part positive, by square roots alone (exact in IEEE
/// arithmetic): re = √((|z| + a)/2), im = b/(2 re), without the cancellation of |z| − a.
fn root(a: f64, b: f64) -> c64 {
    let re = (0.5 * ((a * a + b * b).sqrt() + a)).sqrt();
    c64::new(re, b / (2.0 * re))
}

/// |z|, by a square root alone.
fn norm(z: c64) -> f64 {
    (z.re * z.re + z.im * z.im).sqrt()
}

/// ∫ e^(−p|u − u′|) du′ over a strip from −w/2 to w/2, at u: closed.
fn across(p: c64, u: f64, w: f64) -> c64 {
    let (u, half) = (u.abs(), 0.5 * w);
    let e = |z: f64| exp_c(-p * z);
    if u <= half {
        (2.0 - e(half - u) - e(half + u)) / p
    } else {
        (e(u - half) - e(u + half)) / p
    }
}

impl Slab {
    /// The m-th eigenvalue, (m + ½)π/H.
    fn k(&self, m: usize) -> f64 {
        (m as f64 + 0.5) * PI / self.height
    }

    /// Mode m's weight at y from a sheet at `sheets`' height: φ_m(y) φ_m(s).
    fn weight(&self, m: usize, sheets: &Sheets, y: f64) -> f64 {
        let k = self.k(m);
        2.0 / self.height * libm::sin(k * y) * libm::sin(k * sheets.height)
    }

    /// Mode m's term at (x, y) for the transverse decay `p`: the strips' Green's functions
    /// integrated across them.
    fn term(&self, m: usize, p: c64, sheets: &Sheets, x: f64, y: f64) -> c64 {
        let sum: c64 = sheets
            .strips
            .iter()
            .map(|&(c, w, share)| across(p, x - c, w) * (share / w))
            .sum();
        sum * self.weight(m, sheets, y) / (2.0 * self.conductivity * p)
    }

    /// The steady rise (K per W/m) at (x, y), m.
    pub(super) fn steady(&self, sheets: &Sheets, x: f64, y: f64) -> f64 {
        let h = self.height;
        // enough terms for the series' 1/m² tail at the distance between the heights
        let gap = (y - sheets.height).abs().max(1e-3 * h);
        let terms = ((400.0 * h / gap).ceil() as usize).clamp(400, 40_000);
        let mut sum = 0.0;
        for m in 0..terms {
            let t = self.term(m, c64::new(self.k(m), 0.0), sheets, x, y).re;
            if t == 0.0 && m > 0 {
                break;
            }
            sum += t;
        }
        sum
    }

    /// The rise's response at (x, y) to the drive at `frequency_hz`, over its steady rise:
    /// the steady series plus Σ_m [term(p_m) − term(k_m)], p_m = √(k_m² + iω/D), whose terms
    /// fall as 1/m⁴.
    fn response(&self, sheets: &Sheets, x: f64, y: f64, steady: f64, frequency_hz: f64) -> c64 {
        let omega = 2.0 * PI * frequency_hz;
        let mut sum = c64::new(steady, 0.0);
        for m in 0..4000 {
            let k = self.k(m);
            let p = root(k * k, omega / self.diffusivity);
            let d = self.term(m, p, sheets, x, y) - self.term(m, c64::new(k, 0.0), sheets, x, y);
            sum += d;
            if norm(d) < 1e-14 * steady.abs() && m > 8 {
                break;
            }
        }
        sum / steady
    }

    /// The −3 dB frequency (Hz) of the rise at (x, y): where its response falls to 1/√2, by
    /// bisection in log f to 1e-9.
    pub(super) fn cutoff(&self, sheets: &Sheets, x: f64, y: f64) -> f64 {
        let steady = self.steady(sheets, x, y);
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let (mut lo, mut hi) = (1e-3f64, 1e12f64);
        while hi / lo > 1.0 + 1e-9 {
            let mid = (lo * hi).sqrt();
            if norm(self.response(sheets, x, y, steady, mid)) < half {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        (lo * hi).sqrt()
    }

    /// The strips' heat kernel at x after a time τ: Σ_j (s_j/w_j) ½[erf((c_j − x + w_j/2)/σ) −
    /// erf((c_j − x − w_j/2)/σ)], σ = √(4Dτ).
    fn kernel(&self, sheets: &Sheets, x: f64, tau: f64) -> f64 {
        let sigma = (4.0 * self.diffusivity * tau).sqrt();
        sheets
            .strips
            .iter()
            .map(|&(c, w, share)| {
                let u = c - x;
                share / w
                    * 0.5
                    * (libm::erf((u + 0.5 * w) / sigma) - libm::erf((u - 0.5 * w) / sigma))
            })
            .sum()
    }

    /// The slowest mode's time constant, 1/(D k₀²), s.
    fn slowest(&self) -> f64 {
        1.0 / (self.diffusivity * self.k(0) * self.k(0))
    }

    /// The rise at (x, y), from 0 to `steady`, a time `t` (s) after the drive is turned on: the
    /// steady rise less each mode's ∫_t^∞ e^(−Dk_m²τ) W dτ / ρc, by Simpson's rule in ln τ to
    /// 40 of the slowest mode's time constants.
    fn step(&self, sheets: &Sheets, x: f64, y: f64, steady: f64, t: f64) -> f64 {
        let end = 40.0 * self.slowest();
        if t >= end {
            return steady;
        }
        const NODES: usize = 160;
        let (z0, z1) = (libm::log(t), libm::log(end));
        let dz = (z1 - z0) / NODES as f64;
        let nodes: Vec<(f64, f64)> = (0..=NODES)
            .map(|i| {
                let tau = libm::exp(z0 + dz * i as f64);
                let simpson = if i == 0 || i == NODES {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                (tau, simpson * tau * self.kernel(sheets, x, tau))
            })
            .collect();
        let rho_c = self.conductivity / self.diffusivity;
        let mut tail = 0.0;
        for m in 0.. {
            let k = self.k(m);
            let rate = self.diffusivity * k * k;
            if rate * t > 40.0 {
                break;
            }
            let integral: f64 = nodes
                .iter()
                .map(|&(tau, f)| libm::exp(-rate * tau) * f)
                .sum::<f64>()
                * dz
                / 3.0;
            tail += self.weight(m, sheets, y) * integral / rho_c;
        }
        steady - tail
    }

    /// The 1/e time (s) of the rise at (x, y) when the drive is turned on: when it is 1 − 1/e
    /// of the way, by bisection in log t between a hundredth and five of the slowest mode's
    /// time constants, to 1e-7.
    pub(super) fn one_over_e(&self, sheets: &Sheets, x: f64, y: f64) -> f64 {
        let steady = self.steady(sheets, x, y);
        let target = (1.0 - libm::exp(-1.0)) * steady;
        let (mut lo, mut hi) = (1e-2 * self.slowest(), 5.0 * self.slowest());
        while hi / lo > 1.0 + 1e-7 {
            let mid = (lo * hi).sqrt();
            if self.step(sheets, x, y, steady, mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo * hi).sqrt()
    }
}

/// dn_eff/dT (1/K) and n_eff of the TE-like mode (Marcatili's E^x₁₁, solved exactly) of a
/// `width` × `height` µm core of index `core` (its dn/dT `core_dn_dt`) in a cladding of index
/// `cladding` (`cladding_dn_dt`), at `wavelength`: by central differences of 1 K.
///
/// # Errors
///
/// [`Error::InvalidValue`] if the rectangle guides no such mode.
pub(super) fn thermo_optic(
    width: f64,
    height: f64,
    (core, core_dn_dt): (f64, f64),
    (cladding, cladding_dn_dt): (f64, f64),
    wavelength: Wavelength,
) -> Result<(f64, f64)> {
    let n = |dt: f64| -> Result<f64> {
        Rectangle::uniform(
            core + core_dn_dt * dt,
            cladding + cladding_dn_dt * dt,
            Length::um(width),
            Length::um(height),
        )?
        .mode(Family::Ex, 1, 1, wavelength)
        .ok_or_else(|| {
            Error::invalid(
                "device",
                format!(
                    "a {width} × {height} um core guides no TE-like mode at {wavelength} by \
                     Marcatili's equations"
                ),
            )
        })
    };
    let (up, down) = (n(1.0)?, n(-1.0)?);
    Ok((0.5 * (up - down), n(0.0)?))
}
