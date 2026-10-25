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
//! a strip's integral of it is closed. A top held at the ambient in place of the insulated one
//! (Harris et al.'s model, the oxide's top room temperature: [`Slab`]'s `held`) takes
//! k_m = (m + 1)π/H. Heat at several heights (the in-guide heater's ridge and slab) is a sheet
//! at each, its share of the power in its strips, the rises summed:
//!
//! - **steady,** p_m = k_m: the rise at (x, y), Σ_m φ_m(y) φ_m(s) Σ_j s_j J(k_m)/(2k k_m w_j);
//! - **at a frequency ω,** p_m² = k_m² + iω/D: its response over the steady rise, the steady
//!   series plus the terms' differences, which fall as 1/m⁴ ([`Slab::response`], its −3 dB
//!   frequency [`Slab::cutoff`]);
//! - **in time,** the step response u_m(t) = u_m(∞) − (1/ρc) ∫_t^∞ e^(−Dk_m²τ) W(x, τ) dτ, W the
//!   strips' heat kernel e^(−x²/4Dτ)/√(4πDτ) integrated across them (a difference of erfs), the
//!   integral by Simpson's rule in ln τ ([`Slab::one_over_e`]).
//!
//! **The in-guide heater** ([`Fin`]): its silicon a fin across the guide, conducting along it
//! to the held contacts and giving its heat to the oxide straight across, exact in each stretch
//! of the ridge, the slab and the channels; the oxide's spreading, which straight across leaves
//! out, closed by the slab above: the factor on the oxide's steady flow at which the fin's rise
//! at the centre is the slab's from the heat the fin gives it, less what the held contacts take
//! back from the oxide. In time by its transfer at s, inverted by fixed Talbot ([`talbot`]).
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
/// m; the shares of all the sheets that heat it sum to 1.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Sheets {
    pub(super) height: f64,
    pub(super) strips: Vec<(f64, f64, f64)>,
}

/// The oxide between an isothermal plane at y = 0 and a top at y = `height` (m), insulated, or
/// held at the ambient when `held`, of conductivity `conductivity` (W/(m·K)) and diffusivity
/// `diffusivity` (m²/s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Slab {
    pub(super) height: f64,
    pub(super) conductivity: f64,
    pub(super) diffusivity: f64,
    pub(super) held: bool,
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
    /// The m-th eigenvalue, (m + ½)π/H, or (m + 1)π/H with the top held.
    fn k(&self, m: usize) -> f64 {
        let shift = if self.held { 1.0 } else { 0.5 };
        (m as f64 + shift) * PI / self.height
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

    /// The steady rise (K per W/m) at (x, y), m, of all the sheets.
    pub(super) fn steady(&self, sheets: &[Sheets], x: f64, y: f64) -> f64 {
        sheets.iter().map(|s| self.steady_of(s, x, y)).sum()
    }

    /// The steady rise of one sheet.
    fn steady_of(&self, sheets: &Sheets, x: f64, y: f64) -> f64 {
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

    /// The steady rise of all the sheets at (x, y), as [`Slab::steady`] but its series
    /// accelerated (Kummer's transformation): inside a strip the terms tend to
    /// (2/H) sin(k_m y) sin(k_m s) · 2/k_m², whose sum is the slab's Green's function across its
    /// height, y₍<₎(H − y₍>₎)/H with the top held (y₍<₎ insulated), so that is summed in closed
    /// form and the rest falls as e^(−k_m d), d the distance from x to the strip's nearer edge:
    /// summed until k_m d exceeds 40. A point on a strip's edge isn't taken.
    pub(super) fn steady_accelerated(&self, sheets: &[Sheets], x: f64, y: f64) -> f64 {
        let h = self.height;
        let mut rise = 0.0;
        for sheet in sheets {
            let s = sheet.height;
            let (low, high) = (y.min(s), y.max(s));
            let across_height = if self.held { low * (h - high) / h } else { low };
            for &(c, w, share) in &sheet.strips {
                let (u, half) = ((x - c).abs(), 0.5 * w);
                let inside = u < half;
                let d = (u - half).abs();
                let terms = if d > 0.0 {
                    ((40.0 * h / (PI * d)).ceil() as usize).clamp(8, 400_000)
                } else {
                    400_000
                };
                let mut sum = if inside { 2.0 * across_height } else { 0.0 };
                let single = Sheets {
                    height: s,
                    strips: Vec::new(),
                };
                for m in 0..terms {
                    let k = self.k(m);
                    let near = if inside { 2.0 / k } else { 0.0 };
                    let a = across(c64::new(k, 0.0), x - c, w).re - near;
                    sum += self.weight(m, &single, y) * a / k;
                }
                rise += share / w / (2.0 * self.conductivity) * sum;
            }
        }
        rise
    }

    /// The rise's response at (x, y) to one sheet's drive at `frequency_hz`, the sheet's
    /// `steady` rise times its response: the steady series plus Σ_m [term(p_m) − term(k_m)],
    /// p_m = √(k_m² + iω/D), whose terms fall as 1/m⁴.
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
        sum
    }

    /// The −3 dB frequency (Hz) of the rise at (x, y): where its response, all the sheets' over
    /// their steady rise, falls to 1/√2, by bisection in log f to 1e-9.
    pub(super) fn cutoff(&self, sheets: &[Sheets], x: f64, y: f64) -> f64 {
        let each: Vec<f64> = sheets.iter().map(|s| self.steady_of(s, x, y)).collect();
        let steady: f64 = each.iter().sum();
        let response = |f: f64| -> c64 {
            let sum: c64 = sheets
                .iter()
                .zip(&each)
                .map(|(s, &u)| self.response(s, x, y, u, f))
                .sum();
            sum / steady
        };
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let (mut lo, mut hi) = (1e-3f64, 1e12f64);
        while hi / lo > 1.0 + 1e-9 {
            let mid = (lo * hi).sqrt();
            if norm(response(mid)) < half {
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

    /// One sheet's rise at (x, y), from 0 to `steady`, a time `t` (s) after the drive is turned
    /// on: the steady rise less each mode's ∫_t^∞ e^(−Dk_m²τ) W dτ / ρc, by Simpson's rule in
    /// ln τ to 40 of the slowest mode's time constants.
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

    /// The 1/e time (s) of the rise at (x, y), all the sheets', when the drive is turned on:
    /// when it is 1 − 1/e of the way, by bisection in log t between a hundredth and five of the
    /// slowest mode's time constants, to 1e-7.
    pub(super) fn one_over_e(&self, sheets: &[Sheets], x: f64, y: f64) -> f64 {
        let each: Vec<f64> = sheets.iter().map(|s| self.steady_of(s, x, y)).collect();
        let steady: f64 = each.iter().sum();
        let rise = |t: f64| -> f64 {
            sheets
                .iter()
                .zip(&each)
                .map(|(s, &u)| self.step(s, x, y, u, t))
                .sum()
        };
        let target = (1.0 - libm::exp(-1.0)) * steady;
        let (mut lo, mut hi) = (1e-2 * self.slowest(), 5.0 * self.slowest());
        while hi / lo > 1.0 + 1e-7 {
            let mid = (lo * hi).sqrt();
            if rise(mid) < target {
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

// ---------------------------------------------------------------------------------------------
// The in-guide heater: the silicon as a fin
// ---------------------------------------------------------------------------------------------

/// √z on the principal branch (its real part ≥ 0), by square roots alone: exact in IEEE
/// arithmetic, the same bits everywhere. For z on the negative real axis, i√|z|.
pub(super) fn sqrt_c(z: c64) -> c64 {
    let r = norm(z);
    if z.re >= 0.0 {
        let re = (0.5 * (r + z.re)).sqrt();
        if re == 0.0 {
            return c64::new(0.0, 0.0);
        }
        c64::new(re, z.im / (2.0 * re))
    } else {
        let im = (0.5 * (r - z.re)).sqrt().copysign(z.im);
        c64::new(z.im / (2.0 * im), im)
    }
}

/// cosh z and sinh z.
fn cosh_sinh(z: c64) -> (c64, c64) {
    let (a, b) = (exp_c(z), exp_c(-z));
    (0.5 * (a + b), 0.5 * (a - b))
}

/// One stretch of a [`Fin`], outward from the one before: its `length` (m), the silicon's lateral
/// conductance k t (W/K) and heat capacity per area ρc t (J/(m²·K)), the heat it takes per area
/// for each W/m of the drive (1/m), and the oxide below it and above it to the held planes (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stretch {
    pub(super) length: f64,
    pub(super) conductance: f64,
    pub(super) capacity: f64,
    pub(super) heat: f64,
    pub(super) below: f64,
    pub(super) above: f64,
}

/// The silicon of an in-guide heater as a fin across the guide, x = 0 at its centre (symmetric):
/// in each stretch the temperature T(x) uniform through the silicon's thickness obeys
/// K T″ − (Y + sC) T + q = 0 in Laplace's variable s, the heat it gives the oxide Y T, the oxide
/// a layer of conductivity k and diffusivity D between the silicon and a held plane at a distance
/// d each side, carrying it straight across: Y = Σ [k γ coth(γ d) + (a − 1) k/d], γ = √(s/D)
/// (a k/d steady), a the `spreading`, what the oxide beyond the silicon adds to the steady flow
/// (a fast drive's heat stays within its depth of the silicon, where straight across is right).
/// The fin's end, the contact, held at the ambient. Exact in each stretch (cosh and sinh of λx,
/// λ² = (Y + sC)/K), T and the heat flow −K T′ continuous between them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Fin {
    pub(super) stretches: Vec<Stretch>,
    /// The oxide's conductivity (W/(m·K)) and diffusivity (m²/s).
    pub(super) oxide: (f64, f64),
    /// The oxide's heat taken as this many times the layers' straight across: its spreading
    /// beyond the silicon, which the layers leave out.
    pub(super) spreading: f64,
}

/// Fixed Talbot's nodes (Abate & Valkó 2004): 16, about eight digits in double precision.
const TALBOT: usize = 16;

impl Fin {
    /// The heat a layer of oxide `d` m thick, held on its far side, takes per area and kelvin at
    /// s: k γ coth(γ d), or k/d at s = 0, with the spreading's (a − 1) k/d.
    fn layer(&self, s: c64, d: f64) -> c64 {
        let (k, diffusivity) = self.oxide;
        let spread = (self.spreading - 1.0) * k / d;
        if s == c64::new(0.0, 0.0) {
            return c64::new(k / d + spread, 0.0);
        }
        let g = sqrt_c(s / diffusivity);
        let e = exp_c(-2.0 * g * d);
        k * g * (1.0 + e) / (1.0 - e) + spread
    }

    /// Each stretch's λ, its particular solution q/(Y + sC), and K λ, at s.
    fn stretch(&self, st: &Stretch, s: c64) -> (c64, c64, c64) {
        let y = self.layer(s, st.below) + self.layer(s, st.above) + s * st.capacity;
        let lambda = sqrt_c(y / st.conductance);
        (lambda, st.heat / y, st.conductance * lambda)
    }

    /// The temperature and heat flow at each stretch's start, at s (per W/m of the drive), the
    /// centre's temperature first: shot from the centre (no flow) to the contact (held).
    fn starts(&self, s: c64) -> Vec<(c64, c64)> {
        let zero = c64::new(0.0, 0.0);
        // T = a + b T₀ and Φ = c + d T₀ along the fin
        let (mut a, mut b, mut c, mut d) = (zero, c64::new(1.0, 0.0), zero, zero);
        let mut each = Vec::with_capacity(self.stretches.len());
        for st in &self.stretches {
            each.push((a, b, c, d));
            let (lambda, particular, kl) = self.stretch(st, s);
            let (ch, sh) = cosh_sinh(lambda * st.length);
            let u = a - particular;
            (a, b, c, d) = (
                particular + u * ch - c * sh / kl,
                b * ch - d * sh / kl,
                c * ch - kl * u * sh,
                d * ch - kl * b * sh,
            );
        }
        let t0 = -a / b;
        each.into_iter()
            .map(|(a, b, c, d)| (a + b * t0, c + d * t0))
            .collect()
    }

    /// The centre's rise (K per W/m) at s, its Laplace transform's transfer.
    fn centre(&self, s: c64) -> c64 {
        self.starts(s)[0].0
    }

    /// The steady rise at the centre, K per W/m.
    pub(super) fn steady(&self) -> f64 {
        self.centre(c64::new(0.0, 0.0)).re
    }

    /// The heat each stretch gives the oxide, steady, as a share of the drive's: Y ∫ T dx over
    /// it (one side).
    pub(super) fn leaks(&self) -> Vec<f64> {
        let s = c64::new(0.0, 0.0);
        self.stretches
            .iter()
            .zip(self.starts(s))
            .map(|(st, (t, flow))| {
                let (lambda, particular, kl) = self.stretch(st, s);
                let (ch, sh) = cosh_sinh(lambda * st.length);
                let u = t - particular;
                let integral = particular * st.length + (u * sh - flow / kl * (ch - 1.0)) / lambda;
                let y = self.layer(s, st.below) + self.layer(s, st.above);
                (y * integral).re
            })
            .collect()
    }

    /// The −3 dB frequency (Hz) of the centre's rise: where its response falls to 1/√2, by
    /// bisection in log f to 1e-9, from 1 mHz to 1 GHz.
    pub(super) fn cutoff(&self) -> f64 {
        let steady = self.steady();
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let (mut lo, mut hi) = (1e-3f64, 1e9f64);
        while hi / lo > 1.0 + 1e-9 {
            let mid = (lo * hi).sqrt();
            if norm(self.centre(c64::new(0.0, 2.0 * PI * mid))) / steady < half {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        (lo * hi).sqrt()
    }

    /// The centre's rise a time `t` (s) after the drive is turned on: the inverse Laplace
    /// transform of its transfer over s, by fixed Talbot.
    fn step(&self, t: f64) -> f64 {
        talbot(|s| self.centre(s) / s, t)
    }

    /// The 1/e time (s) of the centre's rise when the drive is turned on, by bisection in log t
    /// from 1 ns to 1 ms, to 1e-7.
    pub(super) fn one_over_e(&self) -> f64 {
        let target = (1.0 - libm::exp(-1.0)) * self.steady();
        let (mut lo, mut hi) = (1e-9f64, 1e-3f64);
        while hi / lo > 1.0 + 1e-7 {
            let mid = (lo * hi).sqrt();
            if self.step(mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo * hi).sqrt()
    }
}

/// f(t) from its Laplace transform F(s) by the fixed Talbot method (J. Abate, P. P. Valkó, Int.
/// J. Numer. Methods Eng. 60, 979 (2004), doi:10.1002/nme.995): the Bromwich contour deformed
/// to s(θ) = rθ(cot θ + i), r = 2M/(5t), and the trapezoidal rule in θ over M nodes,
/// f(t) ≈ (r/M)[½F(r)e^(rt) + Σ_k Re(e^(t s(θ_k)) F(s(θ_k)) (1 + iσ(θ_k)))], θ_k = kπ/M,
/// σ(θ) = θ + (θ cot θ − 1) cot θ. For an F analytic off the negative real axis.
pub(super) fn talbot(f: impl Fn(c64) -> c64, t: f64) -> f64 {
    let m = TALBOT as f64;
    let r = 2.0 * m / (5.0 * t);
    let mut sum = 0.5 * (f(c64::new(r, 0.0)) * libm::exp(r * t)).re;
    for k in 1..TALBOT {
        let theta = k as f64 * PI / m;
        let (sin, cos) = libm::sincos(theta);
        let cot = cos / sin;
        let s = c64::new(r * theta * cot, r * theta);
        let sigma = theta + (theta * cot - 1.0) * cot;
        sum += (exp_c(t * s) * f(s) * c64::new(1.0, sigma)).re;
    }
    r / m * sum
}
