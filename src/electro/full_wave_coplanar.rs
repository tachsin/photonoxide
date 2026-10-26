//! The full-wave RF mode's coplanar cases ([`super::full_wave`]): Heinrich 1993's coplanar line,
//! its resistance and inductance from the intermediate range (the current reaching through the
//! metal) to the skin effect, against his closed-form model; and Frankel et al. 1991's line on a
//! GaAs half-space, its radiation into the substrate against their Eq. (9a).

use super::full_wave::{FullWave, Open, RfMode};
use super::line::{QuasiTem, mu_0};
use super::{Conductor, Electrostatic, Features, Grading, Grid, Section, Side, Tensor};
use crate::geometry::{Point, Shape};
use crate::statics::elliptic_k;
use crate::units::{Length, SPEED_OF_LIGHT};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid model";

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect(OK)
}

/// E(k), the complete elliptic integral of the second kind of modulus `k` (0 ≤ k < 1), by the
/// arithmetic-geometric mean: E = K (1 − Σ 2^(n−1) c_n²), c₀ = k, c_(n+1) = (a_n − b_n)/2.
pub(crate) fn elliptic_e(k: f64) -> f64 {
    let (mut a, mut b) = (1.0f64, (1.0 - k * k).sqrt());
    let mut sum = 0.5 * k * k;
    let mut weight = 0.5;
    for _ in 0..64 {
        let c = 0.5 * (a - b);
        let (an, bn) = (0.5 * (a + b), (a * b).sqrt());
        weight *= 2.0;
        sum += weight * c * c;
        a = an;
        b = bn;
        if c.abs() <= f64::EPSILON * a {
            break;
        }
    }
    std::f64::consts::PI / (2.0 * a) * (1.0 - sum)
}

/// K(k)/K'(k).
fn k_ratio(k: f64) -> f64 {
    elliptic_k(k) / elliptic_k((1.0 - k * k).sqrt())
}

// ---------------------------------------------------------------------------------------------
// Heinrich 1993's coplanar line
// ---------------------------------------------------------------------------------------------

/// Heinrich's coplanar waveguide (SI lengths): signal `w`, slots `s`, metal `t` thick, grounds
/// `wg` wide, conductivity `kappa`, on a substrate of `eps` (infinite, no backside metal).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Heinrich {
    pub w: f64,
    pub s: f64,
    pub t: f64,
    pub wg: f64,
    pub kappa: f64,
    pub eps: f64,
}

/// His Figs. 4 and 5: w = 40, s = 5, t = 1.5, w_g = 200 µm, κ = 3 × 10⁷ S/m, ε_r = 12.9.
pub(crate) const HEINRICH: Heinrich = Heinrich {
    w: 40e-6,
    s: 5e-6,
    t: 1.5e-6,
    wg: 200e-6,
    kappa: 3e7,
    eps: 12.9,
};

impl Heinrich {
    /// Eq. (14): k₀, k₁, k₂.
    fn moduli(&self) -> [f64; 3] {
        let (w, s, wg) = (self.w, self.s, self.wg);
        let k0 = w / (w + 2.0 * s);
        let sq = |x: f64| x * x;
        let k1 = k0
            * ((1.0 - sq((w + 2.0 * s) / (w + 2.0 * s + 2.0 * wg)))
                / (1.0 - sq(w / (w + 2.0 * s + 2.0 * wg))))
            .sqrt();
        let k2 = k0
            * ((1.0 - sq((w + 2.0 * s) / (4.0 * w + 2.0 * s)))
                / (1.0 - sq(w / (4.0 * w + 2.0 * s))))
            .sqrt();
        [k0, k1, k2]
    }

    /// Eqs. (15)–(16): a, b and p_c0 … p_c6.
    fn p(&self) -> (f64, f64, [f64; 7]) {
        let (a, b) = (self.w / 2.0, self.w / 2.0 + self.s);
        let [k0, ..] = self.moduli();
        let kp = elliptic_k((1.0 - k0 * k0).sqrt());
        let ep = elliptic_e((1.0 - k0 * k0).sqrt());
        let pi = std::f64::consts::PI;
        let r = (b - a) / (b + a);
        let p0 = b / (2.0 * a) / (kp * kp);
        let p1 = 1.0 + (8.0 * pi * a / (a + b)).ln() + a / (a + b) * (b / a).ln();
        let p2 = p1 - 2.0 * a / b * kp * kp;
        let p3 = 2.0 * b * b / (a * (b + a)) * ep / kp;
        let p4 = r * ((8.0 * pi * a / (a + b)).ln() + a / b);
        let p5 = r * 3f64.ln();
        let p6 =
            r * (24.0 * pi * b * (a + b) / ((b - a) * (b - a))).ln() - b / (b + a) * (b / a).ln();
        (a, b, [p0, p1, p2, p3, p4, p5, p6])
    }

    /// Eq. (1): F(t'), the quarter line's capacitance over ε.
    pub(crate) fn f(&self, tp: f64) -> f64 {
        let s = self.s;
        let [_, k1, _] = self.moduli();
        let (_, _, p) = self.p();
        let base = k_ratio(k1);
        if tp <= s / 2.0 {
            let u = tp / s;
            let l = (2.0 * tp / s).ln();
            base + p[0] * (u * (p[1] - l) + u * u * (1.0 - 1.5 * p[2] + p[2] * l))
        } else {
            base + p[0] / 8.0 * (p[2] + 2.0) + tp / s
        }
    }

    /// Eq. (3): the external inductance for ideal conductors, μ₀/(4F₀), F₀ = F(t/2).
    pub(crate) fn l_ext(&self) -> f64 {
        mu_0() / (4.0 * self.f(self.t / 2.0))
    }

    /// Eqs. (17) and (18): F_L of the centre and the ground conductors, at t_H.
    pub(crate) fn f_l(&self, th: f64) -> (f64, f64) {
        let s = self.s;
        let (a, b, p) = self.p();
        let pi = std::f64::consts::PI;
        let r = (b - a) / (b + a);
        if th <= s / 2.0 {
            let u = th / s;
            let l = (2.0 * th / s).ln();
            let c = p[0] / s
                * ((pi * b + b * (8.0 * pi * a / (a + b)).ln() - (b - a) * r.ln() - b * l)
                    / (a + b)
                    + u * (p[1] * p[3] - p[2] - b / a * p[4]
                        + p[5]
                        + (p[2] - p[3] + b / a - 1.0 - p[5]) * l)
                    + u * u
                        * (p[3] * (1.0 - 1.5 * p[1]) + 1.5 * p[1] - 2.0 * p[2]
                            + 1.0
                            + 1.5 * b / a * p[4]
                            - b / a * r
                            + (2.0 * p[2] + p[1] * (p[3] - 1.0) - b / a * p[4]) * l));
            let g = p[0] / s
                * ((pi * a + a * (8.0 * pi * b / (b - a)).ln() + b * r.ln() - a * l) / (a + b)
                    + u * (a / b * p[1] * p[3] + (1.0 - a / b) * p[1] - p[2] - p[4] - p[5]
                        + (-a / b * p[3] + p[2] + a / b - 1.0 + p[5]) * l)
                    + u * u
                        * (a / b * p[3] * (1.0 - 1.5 * p[1]) + 1.5 * a / b * p[1] - 2.0 * p[2]
                            + 2.0
                            - a / b
                            + 1.5 * p[4]
                            - r
                            + (2.0 * p[2] + a / b * p[1] * (p[3] - 1.0) - p[4]) * l));
            (c, g)
        } else {
            let lead = 1.0 / (2.0 * s) + th / (s * s);
            let q = 2.0 * (a * a + b * b);
            let c = lead
                + p[0] / s
                    * (pi * b / (a + b)
                        + 0.5 * p[6]
                        + (-p[1] + p[3] * (p[1] + 2.0) - b / a * p[4] - q / (a * (a + b))) / 8.0);
            let g = lead
                + p[0] / s
                    * (pi * a / (a + b) - 0.5 * p[6]
                        + (-a / b * p[1] + a / b * p[3] * (p[1] + 2.0) - p[4] - q / (b * (a + b)))
                            / 8.0);
            (c, g)
        }
    }

    /// Eq. (20).
    fn g_l(&self, x: f64) -> f64 {
        let t = self.t;
        (t * t / 12.0 - x * x / 2.0) * (1.0 + (x / t).powi(2)).ln()
            + x.powi(4) / (12.0 * t * t) * (1.0 + (t / x).powi(2)).ln()
            - 2.0 / 3.0 * x * t * ((x / t).atan() + (x / t).powi(2) * (t / x).atan())
    }

    /// Eq. (19): the DC inductance of the line of centre strip `w1`, grounds `w2`, H/m.
    pub(crate) fn l_dc(&self, w1: f64, w2: f64) -> f64 {
        let (s, g) = (self.s, |x: f64| self.g_l(x));
        mu_0() / (8.0 * std::f64::consts::PI)
            * (4.0 / (w1 * w1) * g(w1)
                + (g(w1 + 2.0 * s) + g(w1 + 2.0 * w2 + 2.0 * s) + 2.0 * g(w2)
                    - 2.0 * g(w1 + w2 + 2.0 * s))
                    / (w2 * w2)
                - 4.0 / (w1 * w2) * (g(w1 + w2 + s) - g(w1 + s) + g(s) - g(w2 + s)))
    }

    /// The transition frequencies (rad/s) of Eq. (8), ω_c1, ω_c2, ω_g1, ω_g2, and of Eq. (11),
    /// ω_L0, ω_L1, ω_L2.
    pub(crate) fn transitions(&self) -> ([f64; 4], [f64; 3]) {
        let (w, t, wg, kappa) = (self.w, self.t, self.wg, self.kappa);
        let mk = mu_0() * kappa;
        (
            [
                2f64.sqrt() * 4.0 / (mk * t * w),
                8.0 / mk * ((w + t) / (w * t)).powi(2),
                2.0 / (mk * t * wg),
                2.0 / mk * ((2.0 * wg + t) / (wg * t)).powi(2),
            ],
            [4.0 / (mk * t * wg), 4.0 / (mk * t * w), 18.0 / (mk * t * t)],
        )
    }

    /// Eqs. (5)–(8) and (21): the line's resistance at `omega` (rad/s), Ω/m.
    pub(crate) fn resistance(&self, omega: f64) -> f64 {
        let (w, t, wg, kappa) = (self.w, self.t, self.wg, self.kappa);
        let mu = mu_0();
        let f0 = self.f(t / 2.0);
        let (fc, fg) = self.f_l(t / 2.0);
        let part = |r0: f64, o1: f64, o2: f64, fl: f64| -> f64 {
            let r1 = (o2 * mu / (2.0 * kappa)).sqrt() * fl / (4.0 * f0 * f0);
            let nu = (r0 / r1).ln() / (o1 / o2).ln();
            let gamma = (o1 / o2).powi(2);
            let q = 0.25 * (0.5 - nu) * (4.0 - nu * (1.0 - gamma * gamma));
            let a4 = (gamma * nu + q) / (4.0 - nu - q);
            let a3 = 0.25 * (0.5 - nu) * (1.0 + a4);
            let a2 = (a4 - a3) / gamma;
            let a1 = a2 + gamma * a3;
            if omega <= o1 {
                r0 * (1.0 + a1 * (omega / o1).powi(2))
            } else if omega <= o2 {
                r1 * (omega / o2).powf(nu)
                    * (1.0 + a2 * (o1 / omega).powi(2) + a3 * (omega / o2).powi(2))
            } else {
                (omega * mu / (2.0 * kappa)).sqrt() * fl / (4.0 * f0 * f0)
                    * (1.0 + a4 * (o2 / omega).powi(2))
            }
        };
        let ([oc1, oc2, og1, og2], _) = self.transitions();
        part(1.0 / (kappa * w * t), oc1, oc2, fc) + part(1.0 / (2.0 * kappa * wg * t), og1, og2, fg)
    }

    /// Eqs. (9)–(11) and (22): the line's inductance at `omega` (rad/s), H/m.
    pub(crate) fn inductance(&self, omega: f64) -> f64 {
        let (w, t, wg, kappa) = (self.w, self.t, self.wg, self.kappa);
        let mu = mu_0();
        let f0 = self.f(t / 2.0);
        let (fc, fg) = self.f_l(t / 2.0);
        let [_, k1, k2] = self.moduli();
        let f1 = f0 + k_ratio(k2) - k_ratio(k1);
        let le = self.l_ext();
        let ldc0 = self.l_dc(w, wg);
        let lz1 = self.l_dc(w, 1.5 * w) - mu / 4.0 / f1;
        let (_, [o0, o1, o2]) = self.transitions();
        let skin = |o: f64| (mu / (2.0 * o * kappa)).sqrt() * (fc + fg) / (4.0 * f0 * f0);
        let lz2 = skin(o2);
        let nu1 = ((ldc0 - le) / lz1).ln() / (o0 / o1).ln();
        let nu2 = (lz1 / lz2).ln() / (o1 / o2).ln();
        let (rg, rt) = (w / wg, 2.0 * t / (9.0 * w));
        let e1 = rg.powi(4) * nu1 / (4.0 - nu1);
        let e2 = rg.powi(2) * nu1 / (4.0 - nu1);
        let e3 = rt.powi(3) * (nu2 - 0.5) / (nu2 + 2.5);
        let e4 = rt * (nu2 + 0.5) / (nu2 + 2.5);
        let a3 = ((nu2 - nu1) * (1.0 + e1) * (1.0 - e4) + 4.0 * e2 + e4 * (1.0 - 3.0 * e1))
            / ((nu1 - nu2) * (1.0 + e1) * (1.0 - e3) + 4.0 - e3 * (1.0 - 3.0 * e1));
        let a2 = (a3 * (1.0 - e3) - e2 - e4) / (1.0 + e1);
        let a4 = -(9.0 * w / (2.0 * t)) * (e4 + a3 * e3);
        let a5 = rt * rt * a3 + a4;
        let a1 = nu1 / (4.0 - nu1) + e2 * a2;
        let a0 = (1.0 - le / ldc0) * (a1 + rg * rg * a2);
        if omega <= o0 {
            ldc0 * (1.0 + a0 * (omega / o0).powi(2))
        } else if omega <= o1 {
            le + lz1
                * (omega / o1).powf(nu1)
                * (1.0 + a1 * (o0 / omega).powi(2) + a2 * (omega / o1).powi(2))
        } else if omega <= o2 {
            // a4's term to the first power: what Eq. (22)'s a2 … a5 make continuous, with its
            // first derivative, at ω_L1 and ω_L2
            le + lz2
                * (omega / o2).powf(nu2)
                * (1.0 + a3 * (o1 / omega).powi(2) + a4 * (omega / o2))
        } else {
            le + skin(omega) * (1.0 + a5 * (o2 / omega))
        }
    }
}

/// Heinrich's line as a static problem (µm), its right half beside the mirror x = 0: the signal
/// 20 µm wide to the mirror, the slot 5 µm, the ground 200 µm, all 1.5 µm thick, on GaAs below y
/// = 0, air above; the window 300 µm beyond the ground and below the metal, 300 µm above it, on
/// a grid graded from `smallest` at the metal by 15% a cell to 10 µm.
pub(crate) fn heinrich_problem(smallest: f64) -> Electrostatic {
    let h = HEINRICH;
    let (half, slot, ground, metal) = (h.w * 5e5, h.s * 1e6, h.wg * 1e6, h.t * 1e6);
    let outer = half + slot + ground;
    let (bottom, top) = (-300.0, metal + 300.0);
    let mut section = Section::new("air");
    section.layer(bottom, 0.0, "GaAs").expect(OK);
    let signal = rect([-1.0, half], [0.0, metal]);
    let gnd = rect([half + slot, outer], [0.0, metal]);
    let mut features: Features = section.features().clone();
    features.shape(&signal).shape(&gnd).line_x(0.0);
    let grid = Grid::fitted(
        [0.0, outer + 300.0],
        [bottom, top],
        &Grading::new(10.0, smallest, 1.15).expect(OK),
        &features,
    )
    .expect(OK);
    let mut p = Electrostatic::new(section, grid);
    p.permittivity("air", Tensor::isotropic(1.0))
        .permittivity("GaAs", Tensor::isotropic(h.eps))
        .conductor(Conductor::shape(signal, 1.0))
        .conductor(Conductor::shape(gnd, 0.0));
    p
}

/// Heinrich's line full-wave: the metal at his κ, the substrate a half-space (the window open
/// below and to the right into 200 µm of PML).
pub(crate) fn heinrich_full_wave() -> FullWave {
    FullWave::new(0, HEINRICH.kappa)
        .expect(OK)
        .mirrored()
        .with_count(4)
        .with_open(Open::new(Side::Bottom, 200.0).expect(OK).with_cells(15))
        .with_open(Open::new(Side::Right, 200.0).expect(OK).with_cells(15))
}

// ---------------------------------------------------------------------------------------------
// Frankel et al. 1991's radiation into the substrate
// ---------------------------------------------------------------------------------------------

/// GaAs's relative permittivity as Frankel et al. take it: 13, from their f_te = 43 GHz for d =
/// 500 µm (Eq. 4a) and their quasi-static ε_q = (ε_r + 1)/2 ≈ 7 (Fig. 5(a)).
pub(crate) const FRANKEL_EPS: f64 = 13.0;

/// Gold's conductivity for their 0.4 µm electrodes, S/m (bulk; not given).
pub(crate) const FRANKEL_GOLD: f64 = 4.1e7;

/// Frankel et al.'s Fig. 2 line (i) on a GaAs half-space (µm): a centre strip 30 µm wide between
/// slots of 20 µm, 0.4 µm of gold, ε_r = 13 below and air above; its right half beside the
/// mirror, the window 300 µm beyond the slot and 300 µm below and above the metal, the ground
/// running on through the window's right side into the layer there (a half-plane, as their
/// Eq. (9a) takes it), on a grid graded from `smallest` at the metal by 15% a cell to 5 µm.
pub(crate) fn frankel_problem(smallest: f64) -> Electrostatic {
    let (half, slot, metal, depth) = (15.0, 20.0, 0.4, 300.0);
    let mut section = Section::new("air");
    section.layer(-depth, 0.0, "GaAs").expect(OK);
    let signal = rect([-1.0, half], [0.0, metal]);
    let outer = half + slot + 300.0;
    let gnd = rect([half + slot, outer + 1.0], [0.0, metal]);
    let mut features: Features = section.features().clone();
    features.shape(&signal).shape(&gnd).line_x(0.0);
    let grid = Grid::fitted(
        [0.0, outer],
        [-depth, metal + depth],
        &Grading::new(5.0, smallest, 1.15).expect(OK),
        &features,
    )
    .expect(OK);
    let mut p = Electrostatic::new(section, grid);
    p.permittivity("air", Tensor::isotropic(1.0))
        .permittivity("GaAs", Tensor::isotropic(FRANKEL_EPS))
        .conductor(Conductor::shape(signal, 1.0))
        .conductor(Conductor::shape(gnd, 0.0));
    p
}

/// Frankel et al.'s line full-wave: bulk gold, the substrate a half-space (the window open
/// below and to the right into 200 µm of PML).
pub(crate) fn frankel_full_wave() -> FullWave {
    FullWave::new(0, FRANKEL_GOLD)
        .expect(OK)
        .mirrored()
        .with_count(4)
        .with_open(Open::new(Side::Bottom, 200.0).expect(OK).with_cells(15))
        .with_open(Open::new(Side::Right, 200.0).expect(OK).with_cells(15))
}

/// Frankel et al.'s Eq. (9a): the radiation loss (Np/m) of a coplanar waveguide of centre `s`
/// and slots `w` (m) whose mode has ε_eff at `f` (Hz), on a substrate of ε_r.
pub(crate) fn frankel_9a(s: f64, w: f64, eps_eff: f64, eps_r: f64, f: f64) -> f64 {
    let k = s / (s + 2.0 * w);
    let kk = elliptic_k(k) * elliptic_k((1.0 - k * k).sqrt());
    let c = SPEED_OF_LIGHT;
    let ratio = eps_eff / eps_r;
    (std::f64::consts::PI / 2.0).powi(5) * 2.0 * (1.0 - ratio).powi(2) / ratio.sqrt()
        * (s + 2.0 * w).powi(2)
        * eps_r.powf(1.5)
        / (c * c * c * kk)
        * f.powi(3)
}

/// The quasi-TEM index of `problem`'s mirrored line (a guess to look near).
pub(crate) fn quasi_tem_index(problem: &Electrostatic) -> f64 {
    QuasiTem::mirrored(problem, 0).expect(OK).index()
}

/// R (Ω/m) and L (H/m) of a mode, γZ₀ = R + jωL.
pub(crate) fn r_and_l(m: &RfMode) -> (f64, f64) {
    let z = m.series();
    (z.re, z.im / (2.0 * std::f64::consts::PI * m.frequency_hz))
}

/// A number on three grids `steps` (µm, coarse to fine), gated at `tolerance` (relative): the
/// finest grid's value, its estimated relative error, and whether the gate passed.
fn study(steps: &[f64; 3], values: [f64; 3], tolerance: f64) -> (f64, f64, bool) {
    study_from(0, steps, values, tolerance)
}

/// The same from grid `first` on: from 1, the two finest grids, at least first order.
fn study_from(
    first: usize,
    steps: &[f64; 3],
    values: [f64; 3],
    tolerance: f64,
) -> (f64, f64, bool) {
    let mut s = crate::convergence::Study::new("a coplanar line's number");
    for (h, v) in steps.iter().zip(values).skip(first) {
        s = s.on(*h, v).expect(OK);
    }
    let v = values[2];
    let check = s.check(tolerance * v.abs()).expect(OK);
    let error = check.error.unwrap_or(f64::INFINITY) / v.abs();
    (v, error, check.gated().is_ok())
}

/// The full-wave grids of Heinrich's line: graded from 100, 50 and 25 nm (his metal 5 skin
/// depths thick at 100 GHz, 0.29 µm).
pub(crate) const HEINRICH_STEPS: [f64; 3] = [0.1, 0.05, 0.025];

/// The frequencies of [`heinrich_numbers`], GHz: 1 (the skin depth 2.9 µm, twice the metal),
/// 10 (0.92 µm) and 100 (0.29 µm, the skin-effect range).
pub(crate) const HEINRICH_GHZ: [f64; 3] = [1.0, 10.0, 100.0];

/// photonoxide's numbers for Heinrich's line: per frequency of [`HEINRICH_GHZ`] the full-wave R
/// (Ω/m) and L (H/m), each with its estimated relative error and gate (0.5%) on
/// [`HEINRICH_STEPS`]; and the quasi-TEM external inductance (H/m) from C₀ on the same grids.
#[allow(clippy::type_complexity)]
pub(crate) fn heinrich_numbers() -> &'static ([[(f64, f64, bool); 2]; 3], (f64, f64, bool)) {
    static NUMBERS: std::sync::OnceLock<([[(f64, f64, bool); 2]; 3], (f64, f64, bool))> =
        std::sync::OnceLock::new();
    NUMBERS.get_or_init(|| {
        let h = HEINRICH;
        let fw = heinrich_full_wave();
        let mut rl = [[[0.0; 3]; 2]; 3];
        let mut external = [0.0; 3];
        for (g, &step) in HEINRICH_STEPS.iter().enumerate() {
            let problem = heinrich_problem(step);
            let q = QuasiTem::mirrored(&problem, 0).expect(OK);
            external[g] = q.inductance();
            for (k, &ghz) in HEINRICH_GHZ.iter().enumerate() {
                let omega = 2.0 * std::f64::consts::PI * ghz * 1e9;
                // near the index Heinrich's L gives: the quasi-TEM one raised by L/L_e∞
                let near = q.index() * (h.inductance(omega) / q.inductance()).sqrt();
                let m = fw.solve(&problem, ghz * 1e9, near).expect(OK);
                let (r, l) = r_and_l(&m);
                rl[k][0][g] = r;
                rl[k][1][g] = l;
            }
        }
        (
            rl.map(|q| q.map(|v| study(&HEINRICH_STEPS, v, 5e-3))),
            study(&HEINRICH_STEPS, external, 5e-3),
        )
    })
}

/// The full-wave grids of Frankel et al.'s line: graded from 40, 20 and 10 nm (gold's skin depth
/// 0.25 µm at 100 GHz, 0.12 µm at 400 GHz).
pub(crate) const FRANKEL_STEPS: [f64; 3] = [0.04, 0.02, 0.01];

/// The frequencies of [`frankel_numbers`], GHz.
pub(crate) const FRANKEL_GHZ: [f64; 3] = [100.0, 200.0, 400.0];

/// photonoxide's numbers for Frankel et al.'s line: per frequency of [`FRANKEL_GHZ`] the
/// full-wave ε_eff and the radiation (Np/m), each with its estimated relative error and gate
/// (0.5% and 1%) on [`FRANKEL_STEPS`].
pub(crate) fn frankel_numbers() -> &'static [[(f64, f64, bool); 2]; 3] {
    static NUMBERS: std::sync::OnceLock<[[(f64, f64, bool); 2]; 3]> = std::sync::OnceLock::new();
    NUMBERS.get_or_init(|| {
        let fw = frankel_full_wave();
        let mut v = [[[0.0; 3]; 2]; 3];
        for (g, &step) in FRANKEL_STEPS.iter().enumerate() {
            let problem = frankel_problem(step);
            let near = quasi_tem_index(&problem);
            for (k, &ghz) in FRANKEL_GHZ.iter().enumerate() {
                let m = fw.solve(&problem, ghz * 1e9, near).expect(OK);
                v[k][0][g] = m.index().powi(2);
                v[k][1][g] = m.radiation();
            }
        }
        // the radiation changes by 1e-4 of itself a grid, which on graded grids that aren't
        // nested needn't shrink steadily: its study takes the two finest, at least first order
        v.map(|q| {
            [
                study(&FRANKEL_STEPS, q[0], 5e-3),
                study_from(1, &FRANKEL_STEPS, q[1], 5e-3),
            ]
        })
    })
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// A gated number, or a failing outcome for one whose grid didn't converge.
fn gated((value, _, ok): (f64, f64, bool)) -> f64 {
    if ok { value } else { f64::NAN }
}

/// A full-wave value to four significant digits. At 1 GHz β² is 1e-10 of the operator's entries
/// and the metal's ε about 5 × 10⁸ i, so the last bits of a platform's math library reach the
/// fifth digit (Heinrich's R at 1 GHz: 986.687 Ω/m on Windows, 986.675 on Linux), far inside the
/// grid study's 0.5%: the report shows what every platform computes.
fn four(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(3 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

/// Heinrich's R (Ω/m) at frequency `k` of [`HEINRICH_GHZ`], full-wave against his Eqs. (5)–(8).
fn heinrich_r(k: usize) -> Outcome {
    let omega = 2.0 * std::f64::consts::PI * HEINRICH_GHZ[k] * 1e9;
    let expected = HEINRICH.resistance(omega);
    outcome(
        four(gated(heinrich_numbers().0[k][0])),
        expected,
        0.1 * expected,
    )
}

/// His L (µH/m) at frequency `k`, full-wave against his Eqs. (9)–(11).
fn heinrich_l(k: usize) -> Outcome {
    let omega = 2.0 * std::f64::consts::PI * HEINRICH_GHZ[k] * 1e9;
    let expected = HEINRICH.inductance(omega) * 1e6;
    outcome(
        four(gated(heinrich_numbers().0[k][1]) * 1e6),
        expected,
        0.03 * expected,
    )
}

/// Frankel et al.'s line's radiation (Np/m) at frequency `k` of [`FRANKEL_GHZ`], full-wave
/// against their Eq. (9a) with the full-wave ε_eff.
fn frankel_radiation(k: usize) -> Outcome {
    let [eps, radiation] = frankel_numbers()[k];
    // ε_eff to four digits too: the tolerance, a tenth of the expected, is shown in full
    let expected = frankel_9a(
        30e-6,
        20e-6,
        four(gated(eps)),
        FRANKEL_EPS,
        FRANKEL_GHZ[k] * 1e9,
    );
    outcome(four(gated(radiation)), expected, 0.1 * expected)
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "line/heinrich-1993-external",
            title: r"Travelling-wave electrodes, Heinrich's coplanar line (a 40 µm signal, 5 µm slots, grounds 200 µm wide, 1.5 µm of metal, on a GaAs half-space of $\varepsilon_r$ = 12.9): the quasi-TEM external inductance $L_{e\infty} = 1/(c^2 C_0)$ from the static solver's $C_0$ (H/m, shown), on grids graded from 100, 50 and 25 nm and gated at 0.5%, against his closed form",
            tier: Tier::Analytic,
            source: r"W. Heinrich, IEEE Trans. Microw. Theory Tech. 41, 45 (1993), doi:10.1109/22.210228, Eqs. (1), (3) and (14)–(16): $L_{e\infty} = \mu_0/(4F_0)$, $F_0$ the quarter line's capacitance over $\varepsilon$ by conformal mapping with the metal's thickness; his approximations within 1% of $C$ and $L_{e\infty}$: 1%",
            run: || {
                let expected = HEINRICH.l_ext();
                outcome(gated(heinrich_numbers().1), expected, 0.01 * expected)
            },
        },
        Case {
            id: "line/heinrich-1993-resistance-1ghz",
            title: r"Heinrich's line full-wave at 1 GHz (the skin depth 2.9 µm, twice the metal's thickness: the current through the whole strip), his κ = 3 × 10⁷ S/m, the substrate a half-space (the window open below and to the right into 200 µm of PML), on grids graded from 100, 50 and 25 nm and gated at 0.5%: R = Re $\gamma Z_0$ (Ω/m, shown to four digits: the last bits of a platform's math library reach the fifth at 1 GHz) against his model",
            tier: Tier::Published,
            source: r"Heinrich 1993, Eqs. (5)–(8) and (21), his resistance from DC through the intermediate range to the skin effect, fitted to his mode-matching analysis (his Fig. 4); his Section VI: the resistance his model's main error, its skin-effect form an order of 10% high for narrow slots: 10%",
            run: || heinrich_r(0),
        },
        Case {
            id: "line/heinrich-1993-resistance-10ghz",
            title: r"The same at 10 GHz (the skin depth 0.92 µm): R (Ω/m, shown) against his model",
            tier: Tier::Published,
            source: r"Heinrich 1993, Eqs. (5)–(8) and (21), the intermediate range between his transition frequencies $f_{c1}$ = 0.40 and $f_{c2}$ = 16 GHz; his Section VI: 10%",
            run: || heinrich_r(1),
        },
        Case {
            id: "line/heinrich-1993-resistance-100ghz",
            title: r"The same at 100 GHz (the skin depth 0.29 µm, the metal 5 skin depths thick): R (Ω/m, shown) against his model",
            tier: Tier::Published,
            source: r"Heinrich 1993, Eqs. (4)–(8), the skin-effect range by Wheeler's incremental inductance on his $L_{e\infty}$; his Section VI: the perturbation's constant skin depth overestimates R at the strips' edges, by an order of 10% for narrow slots: 10%",
            run: || heinrich_r(2),
        },
        Case {
            id: "line/heinrich-1993-inductance-1ghz",
            title: r"Heinrich's line full-wave at 1 GHz (as line/heinrich-1993-resistance-1ghz): L = Im $\gamma Z_0/\omega$ (µH/m, shown to four digits; 1.36 times $L_{e\infty}$, the external inductance itself raised by the current spreading through the grounds) against his model",
            tier: Tier::Published,
            source: r"Heinrich 1993, Eqs. (9)–(11) and (22), his inductance from its DC value (Eq. 19, 0.50 µH/m) to $L_{e\infty}$ plus the skin effect's internal part (his Fig. 5); his Section VI: $\varepsilon_{eff}$ and Z within 3% of his full-wave analysis: 3%",
            run: || heinrich_l(0),
        },
        Case {
            id: "line/heinrich-1993-inductance-10ghz",
            title: r"The same at 10 GHz: L (µH/m, shown) against his model",
            tier: Tier::Published,
            source: r"Heinrich 1993, Eqs. (9)–(11) and (22), between his $f_{L1}$ = 0.28 and $f_{L2}$ = 34 GHz; his Section VI: 3%",
            run: || heinrich_l(1),
        },
        Case {
            id: "line/heinrich-1993-inductance-100ghz",
            title: r"The same at 100 GHz: L (µH/m, shown) against his model",
            tier: Tier::Published,
            source: r"Heinrich 1993, Eqs. (4) and (9), $L_{e\infty}$ and the skin effect's internal inductance $R/\omega$; his Section VI: 3%",
            run: || heinrich_l(2),
        },
        Case {
            id: "line/heinrich-1993-grid",
            title: r"Convergence: Heinrich's line full-wave at 1, 10 and 100 GHz on grids graded from 100, 50 and 25 nm by 15% a cell: the grid study's largest estimated relative error of R and L at 25 nm (shown)",
            tier: Tier::Analytic,
            source: r"the grid study of docs/methods/convergence.md (Richardson's estimate from the three grids), the metal's faces and corners graded; the gate the published numbers pass: 0.5%",
            run: || {
                let worst = heinrich_numbers()
                    .0
                    .iter()
                    .flatten()
                    .map(|v| v.1)
                    .fold(0.0f64, f64::max);
                outcome(worst, 0.0, 5e-3)
            },
        },
        Case {
            id: "line/frankel-1991-radiation-100ghz",
            title: r"The full-wave line's radiation into its substrate: Frankel et al.'s line (i) (a 30 µm centre strip, 20 µm slots, 0.4 µm of gold at 4.1 × 10⁷ S/m) on a GaAs half-space of $\varepsilon_r$ = 13, the grounds and the substrate running on into 200 µm of PML (half-planes, as their formula takes them), on grids graded from 40, 20 and 10 nm and gated at 0.5%: at 100 GHz, the loss neither the metal nor the dielectric takes (Np/m, shown to four digits) against their Eq. (9a) with the full-wave $\varepsilon_{eff}$",
            tier: Tier::Published,
            source: r"M. Y. Frankel et al., IEEE Trans. Microw. Theory Tech. 39, 910 (1991), doi:10.1109/22.81658, Eq. (9a): Rutledge et al.'s radiation of the slots' quasi-static field into a half-space, the angle and the impedance from the guided mode's $\varepsilon_{eff}(f)$, for wavelengths above $s + 2w$; the window's own effect on the split up to 3% (grounds 300 against 600 µm, the substrate 300 against 600 µm deep): 10%",
            run: || frankel_radiation(0),
        },
        Case {
            id: "line/frankel-1991-radiation-200ghz",
            title: r"The same at 200 GHz: the radiation (Np/m, shown) against their Eq. (9a)",
            tier: Tier::Published,
            source: r"Frankel et al. 1991, Eq. (9a) and their Fig. 2, line (i) (the radiation a sixth of the line's loss at 100 GHz and more than half at 200, rising more slowly than $f^3$ as $\varepsilon_{eff}$ rises): 10%",
            run: || frankel_radiation(1),
        },
        Case {
            id: "line/frankel-1991-radiation-400ghz",
            title: r"The same at 400 GHz: the radiation (Np/m, shown) against their Eq. (9a)",
            tier: Tier::Published,
            source: r"Frankel et al. 1991, Eq. (9a); at 400 GHz the substrate's wavelength 210 µm, three times the line's 70 µm: 10%",
            run: || frankel_radiation(2),
        },
        Case {
            id: "line/frankel-1991-grid",
            title: r"Convergence: Frankel et al.'s line full-wave at 100, 200 and 400 GHz on grids graded from 40, 20 and 10 nm by 15% a cell: the grid study's largest estimated relative error of $\varepsilon_{eff}$ and the radiation at 10 nm (shown)",
            tier: Tier::Analytic,
            source: r"the grid study of docs/methods/convergence.md; the radiation changes by 1e-4 of itself a grid, not steadily on graded grids that aren't nested, so its study takes the two finest, at least first order; the gate: 0.5%",
            run: || {
                let worst = frankel_numbers()
                    .iter()
                    .flatten()
                    .map(|v| v.1)
                    .fold(0.0f64, f64::max);
                outcome(worst, 0.0, 5e-3)
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elliptic_e_has_its_known_values() {
        assert!((elliptic_e(0.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
        // E(1/√2) = 1.3506438810476755 (DLMF 19.6.E1's lemniscatic case)
        assert!((elliptic_e(0.5f64.sqrt()) - 1.350_643_881_047_675_5).abs() < 1e-14);
        assert!((elliptic_e(0.999_999) - 1.0).abs() < 1e-4);
    }

    /// The transcription: R and L and their slopes continuous at every transition frequency,
    /// as Heinrich's smoothing coefficients (Eqs. 21 and 22) make them.
    #[test]
    fn heinrich_r_and_l_are_smooth_at_his_transitions() {
        let h = HEINRICH;
        let ([c1, c2, g1, g2], [l0, l1, l2]) = h.transitions();
        let jumps = |f: &dyn Fn(f64) -> f64, o: f64| {
            let e = 1e-7 * o;
            let (lo, hi) = (f(o - e), f(o + e));
            let slope = |a: f64| (f(a * (1.0 + 1e-4)) - f(a * (1.0 - 1e-4))) / 2e-4 / f(a);
            (
                (hi - lo).abs() / lo,
                (slope(o * (1.0 + 2e-4)) - slope(o * (1.0 - 2e-4))).abs(),
            )
        };
        for o in [c1, c2, g1, g2] {
            let (value, slope) = jumps(&|w| h.resistance(w), o);
            assert!(
                value < 1e-6 && slope < 1e-3,
                "R at {o:e}: {value:e} {slope:e}"
            );
        }
        for o in [l0, l1, l2] {
            let (value, slope) = jumps(&|w| h.inductance(w), o);
            assert!(
                value < 1e-6 && slope < 1e-3,
                "L at {o:e}: {value:e} {slope:e}"
            );
        }
        // his Fig. 5: L_DC about 0.5 µH/m, about twice L_e∞; his Fig. 4: R_DC 1/(κwt) + 1/(2κw_g t)
        assert!((h.l_dc(h.w, h.wg) - 0.5e-6).abs() < 0.01e-6);
        let r_dc = 1.0 / (h.kappa * h.w * h.t) + 1.0 / (2.0 * h.kappa * h.wg * h.t);
        assert!((h.resistance(1.0) - r_dc).abs() < 1e-9 * r_dc);
        assert!((h.inductance(1.0) - h.l_dc(h.w, h.wg)).abs() < 1e-9 * h.l_dc(h.w, h.wg));
    }
}
