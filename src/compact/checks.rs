//! Reference cases for the compact models, shared by the tests and the validation report.

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use std::sync::OnceLock;

use faer::Mat;

use super::fit::{self, Options, Rational, Symmetry};
use super::model::CompactModel;
use super::param::{Crossing, Interpolation};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

/// Gustavsen and Semlyen's Table 1 (their Section 4.1): the poles and residues of an 18th-order
/// response, in Hz (to be multiplied by 2π), each complex one standing for its conjugate pair too;
/// d = 0.2 and h = 2e-5.
pub(crate) const PAPER_POLES: [(f64, f64); 10] = [
    (-4500.0, 0.0),
    (-41000.0, 0.0),
    (-100.0, 5000.0),
    (-120.0, 15000.0),
    (-3000.0, 35000.0),
    (-200.0, 45000.0),
    (-1500.0, 45000.0),
    (-500.0, 70000.0),
    (-1000.0, 73000.0),
    (-2000.0, 90000.0),
];

/// The residues of Table 1, in the poles' order.
pub(crate) const PAPER_RESIDUES: [(f64, f64); 10] = [
    (-3000.0, 0.0),
    (-83000.0, 0.0),
    (-5.0, 7000.0),
    (-20.0, 18000.0),
    (6000.0, 45000.0),
    (40.0, 60000.0),
    (90.0, 10000.0),
    (50000.0, 80000.0),
    (1000.0, 45000.0),
    (-5000.0, 92000.0),
];

/// Table 1's poles and residues in s (rad/s), every conjugate pair expanded.
pub(crate) fn paper_poles_residues() -> Vec<(c64, c64)> {
    PAPER_POLES
        .iter()
        .zip(&PAPER_RESIDUES)
        .flat_map(|(&(pr, pi), &(rr, ri))| {
            let (a, c) = (c64::new(pr, pi) * TAU, c64::new(rr, ri) * TAU);
            if pi == 0.0 {
                vec![(a, c)]
            } else {
                vec![(a, c), (a.conj(), c.conj())]
            }
        })
        .collect()
}

/// The paper's frequency response f(s) = Σ c/(s − a) + d + s h at s = j2πf, f from 1 Hz to
/// 100 kHz in 100 steps (their Section 4 and "Efficiency": N_ω = 100).
pub(crate) fn paper_samples() -> (Vec<c64>, Vec<c64>) {
    let pr = paper_poles_residues();
    let s: Vec<c64> = (0..100)
        .map(|k| c64::new(0.0, TAU * (1.0 + (1e5 - 1.0) * k as f64 / 99.0)))
        .collect();
    let f = s
        .iter()
        .map(|&z| pr.iter().map(|&(a, c)| c / (z - a)).sum::<c64>() + 0.2 + z * 2e-5)
        .collect();
    (s, f)
}

/// The paper's Table 2: 10 complex pairs, β from 1 Hz to 100 kHz, α = β/100 (Eqs. 9–10), in s.
pub(crate) fn paper_starting_poles() -> Vec<c64> {
    (0..10)
        .flat_map(|k| {
            let beta = 1.0 + (1e5 - 1.0) * k as f64 / 9.0;
            let a = c64::new(-beta / 100.0, beta) * TAU;
            [a, a.conj()]
        })
        .collect()
}

/// The paper's Section 4.2: Table 1's response fitted with Table 2's 20 starting poles, one
/// relocation, a real model with d and h.
pub(crate) fn paper_fit() -> Rational {
    let (s, f) = paper_samples();
    let options = Options {
        iterations: 1,
        symmetry: Symmetry::Real,
        proportional: true,
        starting_poles: Some(paper_starting_poles()),
        ..Options::new(20)
    };
    fit::vector_fit(&s, &[f], &options).expect("the paper's fit")
}

/// The largest error of the fit's poles and residues against Table 1's, each relative to the
/// true pole's or residue's magnitude, each true pole matched to the nearest fitted one; and the
/// largest magnitude of a surplus pole's residue.
pub(crate) fn paper_recovery(model: &Rational) -> (f64, f64, f64) {
    let mut used = vec![false; model.poles.len()];
    let (mut pole_err, mut residue_err) = (0.0f64, 0.0f64);
    for (a, c) in paper_poles_residues() {
        let (j, _) = model
            .poles
            .iter()
            .enumerate()
            .filter(|(j, _)| !used[*j])
            .min_by(|x, y| (x.1 - a).norm().total_cmp(&(y.1 - a).norm()))
            .expect("enough poles");
        used[j] = true;
        pole_err = pole_err.max((model.poles[j] - a).norm() / a.norm());
        residue_err = residue_err.max((model.residues[0][j] - c).norm() / c.norm());
    }
    let surplus = (0..model.poles.len())
        .filter(|&j| !used[j])
        .map(|j| model.residues[0][j].norm())
        .fold(0.0, f64::max);
    (pole_err, residue_err, surplus)
}

/// An all-pass ring (W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012),
/// doi:10.1002/lpor.201100017, Eq. 1): through transmission
/// E_pass/E_in = e^(i(π+φ)) (a − r e^(−iφ))/(1 − r a e^(iφ)) = (r − a e^(iφ))/(1 − r a e^(iφ)),
/// φ = β L the round trip's phase, r the self-coupling and a the round trip's amplitude.
///
/// The effective index is first order in wavelength around λ₀, n_eff(λ) = n₀ − (n_g − n₀)(λ −
/// λ₀)/λ₀ (their Eq. 10 with n_g constant), which makes φ exactly linear in k = 2π/λ:
/// φ = L (n_g k − (n_g − n₀) k₀). Its poles are then exactly where r a e^(iφ) = 1.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ring {
    pub r: f64,
    pub a: f64,
    pub length_um: f64,
    pub n0: f64,
    pub ng: f64,
    pub lambda0_um: f64,
}

impl Ring {
    /// The ring of the validation cases: 10 µm radius, n₀ = 2.4, n_g = 4.2 at 1.55 µm, r = 0.95,
    /// a = 0.98: resonances 9.1 nm apart, 0.21 nm wide.
    pub(crate) fn example() -> Ring {
        Ring {
            r: 0.95,
            a: 0.98,
            length_um: TAU * 10.0,
            n0: 2.4,
            ng: 4.2,
            lambda0_um: 1.55,
        }
    }

    fn phase(&self, k: c64) -> c64 {
        self.length_um * (self.ng * k - (self.ng - self.n0) * TAU / self.lambda0_um)
    }

    /// The through transmission at wavenumber k = 2π/λ (rad/µm), any complex k.
    pub(crate) fn through_k(&self, k: c64) -> c64 {
        let z = (c64::new(0.0, 1.0) * self.phase(k)).exp();
        (self.r - self.a * z) / (1.0 - self.r * self.a * z)
    }

    /// The through transmission at `wavelength_um`.
    pub(crate) fn through(&self, wavelength_um: f64) -> c64 {
        self.through_k(c64::new(TAU / wavelength_um, 0.0))
    }

    /// The exact poles in s = −ik whose resonances lie between k_lo and k_hi, with their
    /// residues: k_m = (2πm + C + i ln(ra))/(L n_g), C = L (n_g − n₀) k₀, and the residue in s
    /// is −(1/r − r)/(L n_g), the same for every resonance.
    pub(crate) fn poles(&self, k_lo: f64, k_hi: f64) -> Vec<(c64, c64)> {
        let scale = self.length_um * self.ng;
        let c = self.length_um * (self.ng - self.n0) * TAU / self.lambda0_um;
        let m0 = ((k_lo * scale - c) / TAU).ceil() as i64;
        let m1 = ((k_hi * scale - c) / TAU).floor() as i64;
        let residue = c64::new(-(1.0 / self.r - self.r) / scale, 0.0);
        (m0..=m1)
            .map(|m| {
                let k = c64::new(TAU * m as f64 + c, (self.r * self.a).ln()) / scale;
                (c64::new(0.0, -1.0) * k, residue)
            })
            .collect()
    }
}

/// The ring-fdfd job (jobs/ring-fdfd.toml: an all-pass ring of 2 µm radius on 220 nm SOI by
/// 2D FDFD with the effective index method) at a coarse 40 nm grid and 51 wavelengths from 1.50
/// to 1.60 µm, run once per process: its 2-port spectrum.
pub(crate) fn fdfd_ring() -> &'static crate::circuit::Spectrum {
    use std::sync::OnceLock;
    static SPECTRUM: OnceLock<crate::circuit::Spectrum> = OnceLock::new();
    SPECTRUM.get_or_init(|| fdfd_ring_spectrum("40.0", 51).expect("the ring-fdfd job runs"))
}

/// The ring-fdfd job at `step_nm` and `points` wavelengths, its record read back.
pub(crate) fn fdfd_ring_spectrum(
    step_nm: &str,
    points: usize,
) -> crate::Result<crate::circuit::Spectrum> {
    use crate::circuit::{SMatrix, Spectrum};
    use crate::job::{Event, execute};
    use crate::run::{Job, Run, Stop, replay};
    use crate::units::Wavelength;
    let text = include_str!("../../jobs/ring-fdfd.toml")
        .replace("step_nm = 25.0", &format!("step_nm = {step_nm}"))
        .replace("points = 201", &format!("points = {points}"));
    let job = Job::parse(&text)?;
    let root = std::env::temp_dir().join(format!(
        "photonoxide-compact-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let mut run = Run::create(&root, &job)?;
    execute(&job, &mut run, &Stop::new(None))?;
    let dir = run.dir().to_path_buf();
    drop(run);
    let events: Vec<Event> = replay(&dir)?;
    let _ = std::fs::remove_dir_all(&root);
    let (mut names, mut wavelengths, mut matrices) = (Vec::new(), Vec::new(), Vec::new());
    for e in events {
        if let Event::SParameters {
            wavelength_um,
            ports,
            s,
            ..
        } = e
        {
            names = (1..=ports.len()).map(|k| format!("o{k}")).collect();
            wavelengths.push(Wavelength::um(wavelength_um)?);
            matrices.push(SMatrix::from_rows(
                s.iter()
                    .map(|row| row.iter().map(|v| c64::new(v[0], v[1])).collect())
                    .collect(),
            )?);
        }
    }
    Spectrum::new(names, wavelengths, matrices)
}

/// The FDFD ring's spectrum fitted with 12 poles on its 26 even-numbered wavelengths: the
/// largest |ΔS_qp| there, and at the 25 wavelengths between them, which it wasn't fitted to.
pub(crate) fn fdfd_ring_held_out() -> (f64, f64) {
    use super::model::CompactModel;
    use crate::circuit::Spectrum;
    let sp = fdfd_ring();
    let pick = |start: usize| {
        let idx: Vec<usize> = (start..sp.wavelengths().len()).step_by(2).collect();
        Spectrum::new(
            sp.ports().to_vec(),
            idx.iter().map(|&k| sp.wavelengths()[k]).collect(),
            idx.iter().map(|&k| sp.matrices()[k].clone()).collect(),
        )
        .expect("a part of a spectrum")
    };
    let (even, odd) = (pick(0), pick(1));
    let model = CompactModel::fit(&even, &Options::new(12)).expect("the FDFD ring's fit");
    let held = odd
        .wavelengths()
        .iter()
        .zip(odd.matrices())
        .map(|(&w, m)| {
            model
                .at(w)
                .expect("in band")
                .max_difference(m)
                .expect("same size")
        })
        .fold(0.0, f64::max);
    (model.error().max, held)
}

/// The example ring's 1-port spectrum over 1.53 to 1.57 µm at `count` wavelengths.
pub(crate) fn ring_spectrum(ring: &Ring, count: usize) -> crate::circuit::Spectrum {
    use crate::circuit::{SMatrix, Spectrum};
    use crate::units::Wavelength;
    let ws: Vec<Wavelength> = (0..count)
        .map(|k| Wavelength::from_um_unchecked(1.53 + 0.04 * k as f64 / (count - 1) as f64))
        .collect();
    let ms = ws
        .iter()
        .map(|w| SMatrix::from_fn(1, |_, _| ring.through(w.to_um())))
        .collect();
    Spectrum::new(vec!["o1".into()], ws, ms).expect("one matrix per wavelength")
}

/// What a parametric fit of the example ring measures.
pub(crate) struct ParametricCheck {
    /// The model's largest |ΔS| at its samples.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fitted: f64,
    /// At the values halfway between samples, against Eq. 1 at 401 wavelengths.
    pub held_out: f64,
    /// The common poles' own fit, every sample with the same poles.
    #[cfg_attr(not(test), allow(dead_code))]
    pub shared: f64,
    /// The stable pole–residue models' largest |ΔS| against Eq. 1 at 21 values.
    pub stable: f64,
    /// The largest Re a/|a| of the model's poles at 1001 values.
    pub sampled: f64,
    /// For a piecewise-linear model, where its poles cross the axis.
    pub crossings: Option<Vec<Crossing>>,
    /// For a piecewise-linear model, how many crossings the nearest pole's sign doesn't confirm.
    pub unconfirmed: Option<usize>,
    /// For a piecewise-linear model, how many intervals of 2001 values change their count of
    /// unstable poles without a crossing reported.
    pub missed: Option<usize>,
}

/// The example ring as a parametric model of one of its parameters, `name` ("r" or "n0"),
/// from `lo` to `hi` at `count` samples of 151 wavelengths, `poles` poles, interpolated by
/// `interpolation`.
pub(crate) fn ring_parametric(
    name: &str,
    (lo, hi): (f64, f64),
    count: usize,
    poles: usize,
    interpolation: Interpolation,
) -> ParametricCheck {
    use super::param::{ParametricModel, Sample};
    use crate::circuit::Parameter;
    let set = |v: f64| {
        let mut r = Ring::example();
        match name {
            "r" => r.r = v,
            _ => r.n0 = v,
        }
        r
    };
    let at = |t: f64| lo + (hi - lo) * t;
    let samples: Vec<Sample> = (0..count)
        .map(|k| {
            let v = at(k as f64 / (count - 1) as f64);
            Sample {
                values: vec![v],
                spectrum: ring_spectrum(&set(v), 151),
            }
        })
        .collect();
    let held: Vec<Sample> = (0..count - 1)
        .map(|k| {
            let v = at((k as f64 + 0.5) / (count - 1) as f64);
            Sample {
                values: vec![v],
                spectrum: ring_spectrum(&set(v), 401),
            }
        })
        .collect();
    let model = ParametricModel::fit(
        vec![Parameter::new(name, "", lo, lo, hi)],
        &samples,
        &Options::new(poles),
        interpolation,
    )
    .expect("the ring's parametric fit");
    let mut stable: f64 = 0.0;
    for j in 0..=20 {
        let v = at(j as f64 / 20.0);
        let r = model.stable_rational(&[v], 801).expect("a stable model");
        let ring = set(v);
        for k in 0..=400 {
            let w = 1.53 + 0.04 * k as f64 / 400.0;
            let s = c64::new(0.0, -TAU / w);
            stable = stable.max((r.evaluate(0, s) - ring.through(w)).norm());
        }
    }
    ParametricCheck {
        fitted: model.error().max,
        held_out: model
            .error_against(&held)
            .expect("held-out samples in range")
            .max,
        shared: model.basis_error().max,
        stable,
        sampled: model.stability(1001).expect("the poles"),
        unconfirmed: model.uniform_stability().ok().map(|c| {
            c.iter()
                .filter(|x| crossing_confirmed(&model, x, &c, (lo, hi)) == Some(false))
                .count()
        }),
        missed: model
            .uniform_stability()
            .ok()
            .map(|c| crossings_missed(&model, &c, (lo, hi), 2001)),
        crossings: model.uniform_stability().ok(),
    }
}

/// A pole whose real part is within this of zero, relative to its magnitude, is marginal: its
/// side of the axis is the round-off of the eigenvalues of the non-normal matrices behind it.
pub(crate) const MARGINAL: f64 = 1e-4;

/// Over `points` values across the range, the number of steps where the count of poles in the
/// right half plane changes with no crossing reported in or next to the step, and no pole
/// [`MARGINAL`] at either end of it: crossings the test missed.
pub(crate) fn crossings_missed(
    model: &super::param::ParametricModel,
    crossings: &[Crossing],
    (lo, hi): (f64, f64),
    points: usize,
) -> usize {
    // the count of unstable poles, and whether a pole is within round-off of the axis
    let unstable = |v: f64| {
        let poles = model.rational(&[v]).expect("in range").poles;
        (
            poles.iter().filter(|a| a.re > 0.0).count(),
            poles.iter().any(|a| a.re.abs() <= MARGINAL * a.norm()),
        )
    };
    let at = |k: usize| lo + (hi - lo) * k as f64 / (points - 1) as f64;
    let mut missed = 0;
    let mut last = unstable(at(0));
    for k in 1..points {
        let now = unstable(at(k));
        let (from, to) = (at(k.saturating_sub(2)), at((k + 1).min(points - 1)));
        let near = crossings.iter().any(|c| c.value >= from && c.value <= to);
        if now.0 != last.0 && !near && !now.1 && !last.1 {
            missed += 1;
        }
        last = now;
    }
    missed
}

/// Whether the pole nearest a reported crossing changes the sign of its real part across it:
/// `Some(true)` if it does, `None` if it is [`MARGINAL`] on both sides, `Some(false)` if not.
pub(crate) fn crossing_confirmed(
    model: &super::param::ParametricModel,
    crossing: &Crossing,
    others: &[Crossing],
    (lo, hi): (f64, f64),
) -> Option<bool> {
    let target = c64::new(0.0, -crossing.omega);
    let side = |v: f64| {
        let poles = model.rational(&[v.clamp(lo, hi)]).expect("in range").poles;
        poles
            .into_iter()
            .min_by(|a, b| (a - target).norm().total_cmp(&(b - target).norm()))
            .expect("poles")
    };
    // within the crossing's own stretch: half the way to the next one
    let eps = others
        .iter()
        .map(|c| (c.value - crossing.value).abs())
        .filter(|&d| d > 0.0)
        .fold(2e-4 * (hi - lo), f64::min)
        * 0.5;
    let (before, after) = (side(crossing.value - eps), side(crossing.value + eps));
    if before.re * after.re <= 0.0 {
        return Some((before - target).norm() < 1e-3 && (after - target).norm() < 1e-3);
    }
    let marginal = |a: c64| a.re.abs() <= MARGINAL * a.norm();
    if marginal(before) && marginal(after) {
        None
    } else {
        Some(false)
    }
}

// The validation cases (see crate::validation).

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

fn paper() -> &'static (Rational, (f64, f64, f64)) {
    static FIT: OnceLock<(Rational, (f64, f64, f64))> = OnceLock::new();
    FIT.get_or_init(|| {
        let model = paper_fit();
        let recovery = paper_recovery(&model);
        (model, recovery)
    })
}

fn vf_paper_rms() -> Outcome {
    zero(paper().0.error.rms, 1e-10)
}

fn vf_paper_poles() -> Outcome {
    zero(paper().1.0, 1e-10)
}

fn vf_paper_residues() -> Outcome {
    zero(paper().1.1, 1e-9)
}

fn vf_paper_real_start() -> Outcome {
    let (s, f) = paper_samples();
    let start: Vec<c64> = (0..20)
        .map(|k| c64::new(-TAU * (1.0 + (1e5 - 1.0) * k as f64 / 19.0), 0.0))
        .collect();
    let options = Options {
        iterations: 3,
        tolerance: 0.0,
        symmetry: Symmetry::Real,
        proportional: true,
        starting_poles: Some(start),
        ..Options::new(20)
    };
    let model = fit::vector_fit(&s, &[f], &options).expect("the paper's fit");
    zero(model.error.rms, 1e-10)
}

/// The example ring fitted with 12 complex poles at 401 wavelengths: its largest error at 1001
/// wavelengths, and its poles' largest error relative to their real parts.
fn ring_fit() -> &'static (f64, f64) {
    static FIT: OnceLock<(f64, f64)> = OnceLock::new();
    FIT.get_or_init(|| {
        let ring = Ring::example();
        let model = CompactModel::fit(&ring_spectrum(&ring, 401), &Options::new(12))
            .expect("the ring's fit");
        let between = (0..=1000)
            .map(|k| {
                let w = 1.53 + 0.04 * k as f64 / 1000.0;
                let s = model.at(Wavelength::from_um_unchecked(w)).expect("in band");
                (s[(0, 0)] - ring.through(w)).norm()
            })
            .fold(0.0, f64::max);
        let poles = &model.rational().poles;
        let worst = ring
            .poles(TAU / 1.57, TAU / 1.53)
            .iter()
            .map(|(a, _)| {
                poles
                    .iter()
                    .map(|b| (b - a).norm())
                    .fold(f64::INFINITY, f64::min)
                    / a.re.abs()
            })
            .fold(0.0, f64::max);
        (between, worst)
    })
}

fn vf_ring_allpass() -> Outcome {
    zero(ring_fit().0, 1e-8)
}

fn vf_ring_poles() -> Outcome {
    zero(ring_fit().1, 1e-8)
}

fn fdfd_ring_held_out_case() -> Outcome {
    static RUN: OnceLock<(f64, f64)> = OnceLock::new();
    zero(RUN.get_or_init(fdfd_ring_held_out).1, 2e-4)
}

fn fdfd_ring_passive() -> Outcome {
    let model = CompactModel::fit(fdfd_ring(), &Options::new(12)).expect("the FDFD ring's fit");
    let largest = model.passivity(1001).expect("singular values").largest.1;
    Outcome {
        measured: largest,
        expected: 1.0,
        tolerance: 0.0,
        error: (largest - 1.0).max(0.0),
    }
}

fn coupling() -> &'static ParametricCheck {
    static CHECK: OnceLock<ParametricCheck> = OnceLock::new();
    CHECK.get_or_init(|| {
        ring_parametric(
            "r",
            (0.90, 0.97),
            5,
            12,
            Interpolation::Polynomial { degree: 2 },
        )
    })
}

fn shift() -> &'static ParametricCheck {
    static CHECK: OnceLock<ParametricCheck> = OnceLock::new();
    CHECK.get_or_init(|| {
        ring_parametric(
            "n0",
            (2.39, 2.41),
            13,
            16,
            Interpolation::Polynomial { degree: 6 },
        )
    })
}

fn param_ring_coupling() -> Outcome {
    zero(coupling().held_out, 1e-8)
}

fn param_ring_shift() -> Outcome {
    zero(shift().held_out, 5e-6)
}

fn param_ring_stable() -> Outcome {
    zero(shift().stable, 5e-6)
}

fn piecewise_coupling() -> &'static ParametricCheck {
    static CHECK: OnceLock<ParametricCheck> = OnceLock::new();
    CHECK.get_or_init(|| ring_parametric("r", (0.90, 0.97), 5, 12, Interpolation::PiecewiseLinear))
}

fn piecewise_shift() -> &'static ParametricCheck {
    static CHECK: OnceLock<ParametricCheck> = OnceLock::new();
    CHECK
        .get_or_init(|| ring_parametric("n0", (2.39, 2.41), 13, 16, Interpolation::PiecewiseLinear))
}

fn param_piecewise_coupling() -> Outcome {
    zero(piecewise_coupling().held_out, 2e-3)
}

fn param_piecewise_stable() -> Outcome {
    let c = piecewise_coupling();
    // crossings found, plus one if the poles sampled at 1001 values disagree
    let found = c.crossings.as_ref().map_or(f64::NAN, |v| v.len() as f64);
    let measured = found + f64::from(u8::from(c.sampled >= 0.0));
    Outcome {
        measured,
        expected: 0.0,
        tolerance: 0.0,
        error: measured,
    }
}

fn param_piecewise_crossings() -> Outcome {
    let c = piecewise_shift();
    let disagreements = match (c.unconfirmed, c.missed) {
        (Some(u), Some(m)) => (u + m) as f64,
        _ => f64::NAN,
    };
    Outcome {
        measured: disagreements,
        expected: 0.0,
        tolerance: 0.0,
        error: disagreements,
    }
}

fn vf_fast_equivalence() -> Outcome {
    zero(fast_vf_difference(), 1e-8)
}

/// The compact models' validation cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "compact/vf-paper-rms",
            title: r"Vector fitting, the paper's test: an 18th-order response (2 real poles and 8 conjugate pairs, $d = 0.2$, $h = 2 \times 10^{-5}$) at 100 frequencies from 1 Hz to 100 kHz, fitted from the 20 complex starting poles of their Table 2 in one relocation, real model (RMS error shown)",
            tier: Tier::Published,
            source: "B. Gustavsen, A. Semlyen, IEEE Trans. Power Deliv. 14, 1052 (1999), doi:10.1109/61.772353, Section 4.2 and Table 1: an RMS error of 3.8e-12, round-off on a response that peaks near 200; 1.4e-11 here",
            run: vf_paper_rms,
        },
        Case {
            id: "compact/vf-paper-poles",
            title: "The same fit's 18 poles against Table 1 (largest error relative to the pole's magnitude shown)",
            tier: Tier::Published,
            source: "Gustavsen and Semlyen (1999), Table 3: errors up to 4e-10 Hz on poles of 4.5 to 90 kHz; the two surplus poles' residues are below 4.3e-7 Hz there, 3e-6 rad/s here",
            run: vf_paper_poles,
        },
        Case {
            id: "compact/vf-paper-residues",
            title: "The same fit's 18 residues against Table 1 (largest error relative to the residue's magnitude shown)",
            tier: Tier::Published,
            source: "Gustavsen and Semlyen (1999), Table 3: errors up to 1e-7 Hz on residues of 3 to 94 kHz",
            run: vf_paper_residues,
        },
        Case {
            id: "compact/vf-paper-real-start",
            title: "The same response fitted from 20 real starting poles spread over the band, three relocations (RMS error shown)",
            tier: Tier::Published,
            source: "Gustavsen and Semlyen (1999), Section 4.3 and Table 4: 7.1, 1.0e-11 and 4.2e-13 after one, two and three relocations; 20, 6e-12 and 2e-12 here",
            run: vf_paper_real_start,
        },
        Case {
            id: "compact/vf-fast-equivalence",
            title: "Vector fitting's pole relocation with each response's own unknowns eliminated by QR, against the whole block least squares: 3 responses of an all-pass ring, 12 starting poles, 151 wavelengths (largest difference in the scaling function's coefficients, relative to the largest, shown)",
            tier: Tier::Analytic,
            source: "D. Deschrijver, M. Mrozowski, T. Dhaene, D. De Zutter, IEEE Microw. Wireless Compon. Lett. 18, 383 (2008), doi:10.1109/LMWC.2008.922585, Eqs. 8, 10 and 11: the same least squares, reduced; 1.1e-11 here",
            run: vf_fast_equivalence,
        },
        Case {
            id: "compact/vf-ring-allpass",
            title: r"An all-pass ring's through transmission ($r = 0.95$, $a = 0.98$, 10 µm radius, $n_g = 4.2$), 4 resonances from 1.53 to 1.57 µm at 401 wavelengths, fitted with 12 complex poles: largest $\lvert\Delta S\rvert$ at 1001 wavelengths (shown)",
            tier: Tier::Analytic,
            source: r"W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017, Eq. 1, with $n_\text{eff}$ first order in $\lambda$ (their Eq. 10), which makes the round trip's phase linear in $k = 2\pi/\lambda$",
            run: vf_ring_allpass,
        },
        Case {
            id: "compact/vf-ring-poles",
            title: r"The same fit's poles against the ring's exact poles, where $r a e^{i\phi} = 1$ (largest error relative to the pole's real part, the resonance's half-width, shown)",
            tier: Tier::Analytic,
            source: r"Bogaerts et al. (2012), Eq. 1: $k_m = (2\pi m + C + i \ln ra)/(L n_g)$ with $C = L (n_g - n_0) k_0$, each with the residue $-(1/r - r)/(L n_g)$ in $s = -ik$",
            run: vf_ring_poles,
        },
        Case {
            id: "compact/fdfd-ring-held-out",
            title: r"A 2D FDFD spectrum (jobs/ring-fdfd.toml: an all-pass ring of 2 µm radius on 220 nm SOI by the effective index method, $H$ along $z$, at a 40 nm grid, 51 wavelengths from 1.50 to 1.60 µm), its 2-port S fitted with 12 poles on the 26 even-numbered wavelengths: largest $\lvert\Delta S_{qp}\rvert$ at the 25 between them (shown)",
            tier: Tier::Analytic,
            source: "the solver's own S at the wavelengths left out, 4.5e-5 here; at its own wavelengths the fit is within 8.5e-7, and fitted to all 51 within 6.6e-7: on 25, 40 and 50 nm grids alike the solver's spectrum fits no closer than about 2e-7",
            run: fdfd_ring_held_out_case,
        },
        Case {
            id: "compact/fdfd-ring-passive",
            title: "The same ring's 12-pole model fitted to all 51 wavelengths: its largest singular value over 1001 wavelengths (shown; a passive device's is at most 1)",
            tier: Tier::Analytic,
            source: "passivity: a device without gain gives out at most the power it takes in, so no singular value of its S exceeds 1",
            run: fdfd_ring_passive,
        },
        Case {
            id: "compact/param-ring-coupling",
            title: r"A parametric model of the all-pass ring over its self-coupling, $r$ from 0.90 to 0.97 (5 samples of 151 wavelengths, 12 basis poles, numerator and denominator polynomial of degree 2, all samples fitted at once): largest $\lvert\Delta S\rvert$ at the 4 values halfway between samples, 401 wavelengths each (shown)",
            tier: Tier::Analytic,
            source: r"Bogaerts et al. (2012), Eq. 1: $(r - a z)/(1 - r a z)$ is a ratio of functions linear in $r$, which a numerator and denominator of degree 1 represent exactly; the form is P. Triverio et al.'s, IEEE Trans. Adv. Packag. 32, 205 (2009), doi:10.1109/TADVP.2008.2007913, Eq. 6, solved as C. K. Sanathanan and J. Koerner do, IEEE Trans. Autom. Control 8, 56 (1963), doi:10.1109/TAC.1963.1105517, Eqs. 5-7; 2.3e-10 here",
            run: param_ring_coupling,
        },
        Case {
            id: "compact/param-ring-shift",
            title: r"A parametric model of the ring over $n_\text{eff}$ at 1.55 µm from 2.39 to 2.41, which shifts its resonances by 0.8 of a free spectral range (13 samples, 16 basis poles, degree 6): largest $\lvert\Delta S\rvert$ at the 12 values halfway between samples (shown)",
            tier: Tier::Analytic,
            source: "Bogaerts et al. (2012), Eq. 1; 1.9e-7 here; the 16 shared poles alone fit the samples only to 1.3, Triverio et al.'s piecewise-linear model fails between them (below), and fitting each sample alone and interpolating its poles missed by 0.2 to 0.8, measured while choosing the method",
            run: param_ring_shift,
        },
        Case {
            id: "compact/param-ring-stable",
            title: r"The same model made stable at 21 values (poles in the right half plane flipped, residues identified again over 801 wavelengths): largest $\lvert\Delta S\rvert$ against Eq. 1 (shown)",
            tier: Tier::Analytic,
            source: "Bogaerts et al. (2012), Eq. 1; 1.2e-7 here; the flip is Gustavsen and Semlyen's (1999), Section 2; the polynomial model's own poles aren't constrained, and some fall in the right half plane, out of the band",
            run: param_ring_stable,
        },
        Case {
            id: "compact/param-ring-piecewise",
            title: r"Triverio et al.'s piecewise-linear model of the ring over its self-coupling, $r$ from 0.90 to 0.97 (5 samples, each fitted alone with 12 poles and rewritten on 12 basis poles spread over the band): largest $\lvert\Delta S\rvert$ at the 4 values halfway between samples (shown)",
            tier: Tier::Published,
            source: "P. Triverio, S. Grivet-Talocia, M. S. Nakhla, IEEE Trans. Adv. Packag. 32, 205 (2009), doi:10.1109/TADVP.2008.2007913, Eqs. 8-9 and Theorem 1 (Eqs. 24-26), basis poles as their Section III-C; 8.6e-4 here, a straight line's error between samples; the same over the 0.8-free-spectral-range shift misses by 7, its poles crossing the axis",
            run: param_piecewise_coupling,
        },
        Case {
            id: "compact/param-ring-piecewise-stable",
            title: "The same model's uniform stability: the crossings of the imaginary axis found over the whole range, exactly, plus one if its poles sampled at 1001 values disagree (shown)",
            tier: Tier::Analytic,
            source: "stable at the samples by construction (Triverio et al., Section III-D); between them, a rank-one change of the poles' matrix along each segment, whose crossings are the real zeros of a rational function, found as the generalized eigenvalues of its system pencil; Triverio et al.'s own test, linear matrix inequalities (their Theorem 2), needs a semidefinite solver",
            run: param_piecewise_stable,
        },
        Case {
            id: "compact/param-ring-piecewise-crossings",
            title: "The test on the piecewise-linear model of the 0.8-free-spectral-range shift, 13 samples: of the crossings it finds (64 here), those the nearest pole's sign doesn't confirm, plus the steps of 2001 sampled values whose count of unstable poles changes with no crossing reported near (shown)",
            tier: Tier::Analytic,
            source: "the poles at sampled values, from their own eigenvalues; poles within 1e-4 of the axis, relative to their size, are at the round-off of these eigenproblems (the rewritten coefficients reach 1e5 to 1e6, the ill-conditioning of Triverio et al.'s Table I) and not counted either way",
            run: param_piecewise_crossings,
        },
    ]
}

/// Deschrijver et al.'s fast vector fitting against the full least squares it reduces: σ's
/// coefficients for three responses of the example ring (r = 0.90, 0.95, 0.97) on 12 starting
/// poles, from the eliminated blocks (their Eqs. 10–11) and from the whole block system (their
/// Eq. 8); the largest difference relative to the largest coefficient.
pub(crate) fn fast_vf_difference() -> f64 {
    use super::fit::{
        classify, eliminate, least_squares, partial_fractions, polynomial_terms, real_rows,
        solve_stacked, starting_poles,
    };
    let responses: Vec<Vec<c64>> = [0.90, 0.95, 0.97]
        .iter()
        .map(|&r| {
            let mut ring = Ring::example();
            ring.r = r;
            ring_spectrum(&ring, 151).element(0, 0)
        })
        .collect();
    let ws = ring_spectrum(&Ring::example(), 151);
    let s: Vec<c64> = ws
        .wavelengths()
        .iter()
        .map(|w| c64::new(0.0, -TAU / w.to_um()))
        .collect();
    let options = Options::new(12);
    let poles = classify(
        &starting_poles(&s, 12, Symmetry::Complex),
        Symmetry::Complex,
    )
    .expect("poles");
    let block = |f: &[c64]| -> Vec<Vec<c64>> {
        s.iter()
            .zip(f)
            .map(|(&z, &v)| {
                let mut phi = Vec::new();
                partial_fractions(&poles, z, &mut phi);
                let mut row = phi.clone();
                polynomial_terms(&options, z, &mut row);
                row.extend(phi.iter().map(|&p| -v * p));
                row.push(v);
                row
            })
            .collect()
    };
    let n_sigma = 24;
    // Eqs. 10-11: each block eliminated, the rest stacked
    let mut stacked = Vec::new();
    let mut own = 0;
    for f in &responses {
        let m = real_rows(&block(f));
        own = m.ncols() - n_sigma - 1;
        eliminate(m, own, n_sigma, &mut stacked);
    }
    let fast = solve_stacked(&stacked, n_sigma);
    // Eq. 8: the whole block system
    let blocks: Vec<Mat<f64>> = responses.iter().map(|f| real_rows(&block(f))).collect();
    let rows: usize = blocks.iter().map(|b| b.nrows()).sum();
    let cols = responses.len() * own + n_sigma;
    let mut a = Mat::<f64>::zeros(rows, cols);
    let mut rhs = Mat::<f64>::zeros(rows, 1);
    let mut at = 0;
    for (e, b) in blocks.iter().enumerate() {
        for i in 0..b.nrows() {
            for j in 0..own {
                a[(at + i, e * own + j)] = b[(i, j)];
            }
            for j in 0..n_sigma {
                a[(at + i, responses.len() * own + j)] = b[(i, own + j)];
            }
            rhs[(at + i, 0)] = b[(i, own + n_sigma)];
        }
        at += b.nrows();
    }
    let full = least_squares(a, &rhs);
    let full = &full[responses.len() * own..];
    let scale = full.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    fast.iter()
        .zip(full)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
        / scale
}
