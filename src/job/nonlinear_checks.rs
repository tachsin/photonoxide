//! What the validation report checks a nonlinear job's ridge on (#595): Zhao et al. 2020's
//! guide through a job of kind `"nonlinear"` ([`super::nonlinear`]) on the job's own 20 nm
//! cells, its designed period against their printed 2.8 µm, and its efficiency from the modes'
//! overlap against the library's own overlap of the same guide on the same cells
//! ([`crate::nonlinear::guide`]): the job's stack, orientation, d tensor and grating read as the
//! library builds them.

use super::{Event, execute};
use crate::material::{CrystalAxis, Orientation};
use crate::mode::pockels::Along;
use crate::nonlinear::guide::{
    Grid, ModeField, SecondOrder, percent_per_watt_cm2, second_harmonic_coupling,
};
use crate::nonlinear::guide_checks as checks;
use crate::nonlinear::phase::fourier_coefficient;
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid nonlinear job";

/// Zhao et al.'s guide as a nonlinear job's ridge: 300 nm of x-cut 5 mol% MgO:LN on silica,
/// etched 50 nm to a ridge 1.24 µm wide with 75° sidewalls, air above, along the crystal's y,
/// poled over 5 mm for SHG of 1.57 µm; on cells of `step_nm` behind a mirror wall, at 20 °C
/// (Malitson's silica's own temperature, where the library's cases take it), with their own
/// d₃₃ = 27 pm/V alone (the catalogue's MgO:LN tensor leaves d₂₂ out).
pub(super) fn zhao_job(step_nm: f64) -> String {
    format!(
        r#"
name = "zhao-shg"
timeout_minutes = 30

[task]
kind = "nonlinear"
process = "shg"
length_mm = 5.0
temperature_c = 20.0

[task.waveguide.ridge]
layer = "LN"
width_um = 1.24
etch_um = 0.05
sidewall_deg = 75.0
along = "y"
step_nm = {step_nm:?}
families = ["te"]
mirror = true
d_pm_per_v = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 27.0, 0.0, 0.0, 0.0]]

[task.waveguide.ridge.stack]
substrate = "sio2"
cladding = 1.0

[[task.waveguide.ridge.stack.layer]]
name = "LN"
thickness_um = 0.3
material = {{ id = "linbo3-mgo", orientation = {{ cut = "x", along_x = "z" }} }}
background = 1.0

[task.poling]

[task.pump]
wavelength_um = 1.57
power_mw = 10.0
"#
    )
}

/// The setup the job of `text` records.
pub(super) fn setup_of(text: &str) -> super::nonlinear::NonlinearSetup {
    let job = Job::parse(text).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    run.lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .find_map(|e| match e {
            Event::NonlinearSetup { setup } => Some(*setup),
            _ => None,
        })
        .expect("the job records its setup")
}

/// The library's SHG efficiency of Zhao et al.'s guide on cells of `h` µm behind a mirror wall,
/// from its TE₀ modes at 1.57 and 0.785 µm, through d₃₃ = 27 pm/V turned onto the chip, with a
/// grating of 50 % duty: %/(W·cm²).
pub(super) fn library_efficiency(h: f64) -> f64 {
    let r = checks::zhao_ridge(None);
    let grid = Grid::new(h).halved();
    let (cs, f) = checks::te0_mode(&r, &grid, 1.57);
    let (cs_s, s) = checks::te0_mode(&r, &grid, 0.785);
    let ef = ModeField::new(&f, &cs, Wavelength::um(1.57).expect(OK)).expect(OK);
    let es = ModeField::new(&s, &cs_s, Wavelength::um(0.785).expect(OK)).expect(OK);
    let mut v = [[0.0; 6]; 3];
    v[2][2] = 27.0;
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).expect(OK);
    let d = SecondOrder::from_voigt(v)
        .expect(OK)
        .rotated(&x_cut)
        .in_section(Along::Y);
    let share = r.film_share(&cs, |_, _| true);
    let kappa = second_harmonic_coupling(&es, &ef, &d, &share).expect(OK);
    percent_per_watt_cm2(kappa * fourier_coefficient(1, 0.5))
}

/// The job's cells, nm: its default.
const STEP_NM: f64 = 20.0;

/// The job's setup on [`STEP_NM`] cells, run once for both cases.
fn zhao_setup() -> &'static super::nonlinear::NonlinearSetup {
    static SETUP: std::sync::OnceLock<super::nonlinear::NonlinearSetup> =
        std::sync::OnceLock::new();
    SETUP.get_or_init(|| setup_of(&zhao_job(STEP_NM)))
}

fn period() -> Outcome {
    let s = zhao_setup();
    let measured = s.grating.as_ref().expect("a designed grating").period_um;
    Outcome {
        measured,
        expected: 2.8,
        tolerance: 0.05,
        error: (measured - 2.8).abs(),
    }
}

fn efficiency() -> Outcome {
    let s = zhao_setup();
    let measured = s
        .efficiency_percent_per_w_cm2
        .unwrap_or_else(|| panic!("no efficiency: {:?}", s.notes));
    let expected = library_efficiency(STEP_NM / 1000.0);
    Outcome {
        measured,
        expected,
        tolerance: 1e-8 * expected,
        error: (measured - expected).abs(),
    }
}

/// The cells of the temperature-acceptance case, nm: coarse, since it compares one set of
/// solves reached two ways.
const ACCEPTANCE_STEP_NM: f64 = 40.0;

/// Zhao et al.'s guide on [`ACCEPTANCE_STEP_NM`] cells with a phase map against temperature,
/// 18 to 22 °C on 5 rows (each row's modes solved again), at 1.56, 1.57 and 1.58 µm: (the
/// setup's temperature acceptance, K; the map's phases ΔkL/2 at 1.57 µm, row by row).
pub(super) fn ridge_acceptance_and_map() -> (f64, Vec<f64>) {
    let text = format!(
        "{}\n[task.map]\nwavelength_um = [1.56, 1.58]\npoints = 3\ntemperature_c = [18.0, 22.0]\nacross_points = 5\n",
        zhao_job(ACCEPTANCE_STEP_NM)
    );
    let job = Job::parse(&text).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    let events: Vec<Event> = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .collect();
    let setup = events
        .iter()
        .find_map(|e| match e {
            Event::NonlinearSetup { setup } => Some(setup),
            _ => None,
        })
        .expect("the job records its setup");
    let estimate = setup
        .temperature_acceptance_k
        .as_ref()
        .unwrap_or_else(|| panic!("no temperature acceptance: {:?}", setup.notes))
        .estimate;
    let map = events
        .iter()
        .find_map(|e| match e {
            Event::PhaseMap { map } => Some(map),
            _ => None,
        })
        .expect("the job records its map");
    // the job has no expansion: the map's Δk is 2φ/L
    let length = 5000.0;
    let phases = (0..5)
        .map(|row| map.mismatch_per_um[row * 3 + 1].expect("a mismatch at 1.57 um") * length / 2.0)
        .collect();
    (estimate, phases)
}

/// The setup's temperature acceptance against Fejer et al.'s linear estimate from the map's
/// rows at 19 and 21 °C.
fn ridge_acceptance() -> Outcome {
    use crate::nonlinear::phase::HALF_MAXIMUM;
    let (estimate, phases) = ridge_acceptance_and_map();
    let slope = (phases[3] - phases[1]) / 2.0;
    let expected = 2.0 * HALF_MAXIMUM / slope.abs();
    Outcome {
        measured: estimate,
        expected,
        tolerance: 1e-8 * expected,
        error: (estimate - expected).abs(),
    }
}

/// SHG of 1.55 µm over 10 mm of poled congruent LiNbO₃ (Edwards & Lawrence's, every wave
/// extraordinary), 1 W of pump: of `shg_type` `"i"` or `"ii"`, with `db` dB/cm (one for every
/// wave). η = 50 %/(W·cm²) for type I, P₂ = ηP²L², and for type II the same overlap's
/// 200 %/(W·cm²), P₃ = ηP₁P₂L²: the sum's coupling of two equal fields is twice the harmonic's.
pub(super) fn shg_job(shg_type: &str, db: f64) -> String {
    let eta = if shg_type == "ii" { 200.0 } else { 50.0 };
    format!(
        "name = \"shg-{shg_type}\"\n[task]\nkind = \"nonlinear\"\nprocess = \"shg\"\n\
         shg_type = \"{shg_type}\"\nlength_mm = 10.0\n[task.waveguide]\n\
         efficiency_percent_per_w_cm2 = {eta:?}\nloss_db_per_cm = {db:?}\n\
         [task.waveguide.crystal]\nid = \"linbo3\"\nmodel = \"edwards-lawrence-1984\"\n\
         axes = [\"extraordinary\"]\n[task.poling]\n[task.pump]\nwavelength_um = 1.55\n\
         power_mw = 1000.0\n"
    )
}

/// What a job of `text` gives for SHG: the designed period (µm), the wavelength acceptance
/// (nm), and at the end the depleted harmonic and the conversion (mW and %).
pub(super) fn shg_numbers(text: &str) -> [f64; 4] {
    let job = Job::parse(text).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    let events: Vec<Event> = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .collect();
    let setup = events
        .iter()
        .find_map(|e| match e {
            Event::NonlinearSetup { setup } => Some(setup),
            _ => None,
        })
        .expect("the job records its setup");
    let curve = events
        .iter()
        .find_map(|e| match e {
            Event::NonlinearEfficiency { curve } => Some(curve),
            _ => None,
        })
        .expect("the job records its efficiency");
    let last = |label: &str| -> f64 {
        curve
            .series
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("no series {label}"))
            .values
            .last()
            .copied()
            .flatten()
            .expect("a value at the end")
    };
    [
        setup
            .grating
            .as_ref()
            .expect("a designed grating")
            .period_um,
        setup
            .wavelength_acceptance_nm
            .as_ref()
            .expect("an acceptance")
            .estimate,
        last("harmonic (depleted, RK4)"),
        last("conversion"),
    ]
}

/// Type I's and type II's numbers ([`shg_numbers`]) with both fundamentals in one wave, and
/// their largest relative gap.
pub(super) fn type_ii_against_type_i(db: f64) -> ([f64; 4], [f64; 4], f64) {
    let one = shg_numbers(&shg_job("i", db));
    let two = shg_numbers(&shg_job("ii", db));
    let gap = (0..4)
        .map(|j| (two[j] / one[j] - 1.0).abs())
        .fold(0.0, f64::max);
    (one, two, gap)
}

/// The type-II job's harmonic against the type-I job's, the error the largest gap of the four.
fn type_ii_degenerate() -> Outcome {
    let (one, two, gap) = type_ii_against_type_i(TYPE_II_DB);
    Outcome {
        measured: two[2],
        expected: one[2],
        tolerance: TYPE_II_TOLERANCE * one[2],
        error: gap * one[2],
    }
}

/// The type-II case's loss, dB/cm.
const TYPE_II_DB: f64 = 0.5;
const TYPE_II_TOLERANCE: f64 = 1e-12;

/// Four-wave mixing as a job: γ = 100 /(W·m), 10 mm, the index 2.0 at every wavelength (Δk = 0),
/// 1 W of pump at 1.55 µm, 300 mW of signal at 1.56 µm, every wave losing `db` dB/cm.
pub(super) fn fwm_job(db: f64) -> String {
    format!(
        "name = \"fwm-depleted\"\n[task]\nkind = \"nonlinear\"\nprocess = \"fwm\"\n\
         length_mm = 10.0\n[task.waveguide]\ngamma_per_w_m = 100.0\nloss_db_per_cm = {db:?}\n\
         [[task.waveguide.n_eff]]\nwavelengths_um = [1.0, 2.0]\nn_eff = [2.0, 2.0]\n\
         [task.pump]\nwavelength_um = 1.55\npower_mw = 1000.0\n\
         [task.signal]\nwavelength_um = 1.56\npower_mw = 300.0\n"
    )
}

/// The job's pump, signal and idler at its end, W, from its depleted series.
pub(super) fn fwm_job_end(text: &str) -> [f64; 3] {
    let job = Job::parse(text).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    let curve = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .find_map(|e| match e {
            Event::NonlinearEfficiency { curve } => Some(*curve),
            _ => None,
        })
        .expect("the job records its efficiency");
    let last = |label: &str| -> f64 {
        let s = curve
            .series
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("no series {label}"));
        s.values
            .last()
            .copied()
            .flatten()
            .expect("a power at the end")
            * 1e-3
    };
    [
        last("pump (depleted, RK4)"),
        last("signal (depleted, RK4)"),
        last("idler (depleted, RK4)"),
    ]
}

/// [`fwm_job`]'s guide without loss over its effective length (1 − e^{−αL})/α, on `steps` RK4
/// steps, its powers times e^{−αL}: with one loss α for every wave and Δk = 0, A_j = B_j e^{−αz/2}
/// turns the lossy equations into the lossless ones in ζ = (1 − e^{−αz})/α. The pump, signal and
/// idler at the end, W.
pub(super) fn fwm_equal_loss(db: f64, steps: usize) -> [f64; 3] {
    use crate::nonlinear::fwm::{FourWave, Waves};
    use num_complex::Complex64 as c64;
    let alpha = crate::nonlinear::pulse::loss_per_m(db);
    let length = 0.01;
    let effective = if alpha > 0.0 {
        -(-alpha * length).exp_m1() / alpha
    } else {
        length
    };
    let w = FourWave {
        gamma: 100.0,
        mismatch: 0.0,
    }
    .propagate(
        Waves {
            pump: c64::new(1.0, 0.0),
            signal: c64::new(0.3f64.sqrt(), 0.0),
            idler: c64::new(0.0, 0.0),
        },
        effective,
        steps,
    )
    .expect(OK);
    let decay = (-alpha * length).exp();
    [
        w.pump.norm_sqr() * decay,
        w.signal.norm_sqr() * decay,
        w.idler.norm_sqr() * decay,
    ]
}

/// The loss of the four-wave case, dB/cm.
const FWM_DB: f64 = 3.0;

/// The largest relative gap of the pump, signal and idler between the lossy job and the lossless
/// guide over its effective length.
fn fwm_depleted() -> Outcome {
    let ours = fwm_job_end(&fwm_job(FWM_DB));
    let exact = fwm_equal_loss(FWM_DB, 20_000);
    let gap = (0..3)
        .map(|j| (ours[j] / exact[j] - 1.0).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured: ours[2],
        expected: exact[2],
        tolerance: FWM_TOLERANCE * exact[2],
        error: gap * exact[2],
    }
}

const FWM_TOLERANCE: f64 = 4e-12;

/// The ring cases' power coupling of every wave.
const RING_COUPLING: f64 = 0.1;

/// SHG of 1.55 µm in a lossless ring of poled congruent LiNbO₃ 1 mm round (the grating designed,
/// η = 10 %/(W·cm²)), 1 mW of pump, every wave coupled by [`RING_COUPLING`]; or FWM in a lossless
/// ring of one index 2.0 (Δk = 0), γ = 100 /(W·m), 1 mm round, 10 mW of pump at 1.55 µm, the
/// signal at 1.56 µm.
pub(super) fn ring_job(process: &str) -> String {
    let ring = format!("[task.ring]\ncoupling = {RING_COUPLING:?}\n");
    if process == "shg" {
        format!(
            "name = \"ring-shg\"\n[task]\nkind = \"nonlinear\"\nprocess = \"shg\"\nlength_mm = 1.0\n\
             [task.waveguide]\nefficiency_percent_per_w_cm2 = 10.0\n\
             [task.waveguide.crystal]\nid = \"linbo3\"\nmodel = \"edwards-lawrence-1984\"\n\
             axes = [\"extraordinary\"]\n[task.poling]\n[task.pump]\nwavelength_um = 1.55\n\
             power_mw = 1.0\n{ring}"
        )
    } else {
        format!(
            "name = \"ring-fwm\"\n[task]\nkind = \"nonlinear\"\nprocess = \"fwm\"\nlength_mm = 1.0\n\
             [task.waveguide]\ngamma_per_w_m = 100.0\n\
             [[task.waveguide.n_eff]]\nwavelengths_um = [1.0, 2.0]\nn_eff = [2.0, 2.0]\n\
             [task.pump]\nwavelength_um = 1.55\npower_mw = 10.0\n\
             [task.signal]\nwavelength_um = 1.56\n{ring}"
        )
    }
}

/// A ring job's (resonant, single pass) against Helt et al.'s lossless closed forms on
/// resonance: |F|² = k²/(1 − r)², r = √(1 − k²); SHG's P₂ω = |F₂ω|²|F_ω|⁴ ηP²L² sinc²(ΔkL/2) and
/// FWM's P_i/P_s = |F_i|²|F_s|²|F_p|⁴ (γP_pL)² sinc²(ΔkL/2), one pass the same with every
/// |F|² = 1. Returns the job's resonant value, the closed form's, and the largest relative gap
/// of the resonant value, the single pass and every build-up.
pub(super) fn ring_against_closed_form(process: &str) -> (f64, f64, f64) {
    let s = setup_of(&ring_job(process));
    let ring = s
        .ring
        .as_ref()
        .unwrap_or_else(|| panic!("no ring: {:?}", s.notes));
    let k2 = RING_COUPLING;
    let f2 = k2 / (1.0 - (1.0 - k2).sqrt()).powi(2);
    let length_um = s.length_mm * 1000.0;
    let (single, resonant) = if process == "shg" {
        // η in 1/(W·µm²), P in W, L in µm; mW out
        let eta = s.efficiency_percent_per_w_cm2.expect("an efficiency") / 100.0 * 1e-8;
        let phase = 0.5 * s.residual_per_um * length_um;
        let single =
            eta * 1e-6 * length_um * length_um * crate::nonlinear::phase::sinc2(phase) * 1e3;
        (single, f2.powi(3) * single)
    } else {
        let gamma = s.gamma_per_w_m.expect("a gamma");
        let phase = 0.5 * s.mismatch_per_um * length_um;
        let single =
            (gamma * 10e-3 * length_um * 1e-6).powi(2) * crate::nonlinear::phase::sinc2(phase);
        (single, f2.powi(4) * single)
    };
    let gap = |a: f64, b: f64| (a / b - 1.0).abs();
    let worst = ring.build_ups.iter().map(|&b| gap(b, f2)).fold(
        gap(ring.resonant, resonant).max(gap(ring.single_pass, single)),
        f64::max,
    );
    (ring.resonant, resonant, worst)
}

fn ring_case(process: &'static str) -> Outcome {
    let (measured, expected, worst) = ring_against_closed_form(process);
    Outcome {
        measured,
        expected,
        tolerance: RING_TOLERANCE * expected,
        error: worst * expected,
    }
}

/// The ring cases' tolerance, relative.
pub(super) const RING_TOLERANCE: f64 = 1e-12;

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "job/nonlinear-ridge-zhao-period",
            title: r"Zhao et al. 2020's guide (as `nonlinear/guide-zhao-period`) through a nonlinear job's `[task.waveguide.ridge]`: the stack and the x-cut film as a job gives them, at 20 °C, the TE₀ indices interpolated to 1e-8 on the job's default 20 nm cells behind a mirror wall; the first-order period the job designs for SHG of 1.57 µm (µm, shown)",
            tier: Tier::Published,
            source: r"J. Zhao et al., Phys. Rev. Lett. 124, 163603 (2020), doi:10.1103/PhysRevLett.124.163603, p. 1: the calculated period 2.8 µm; the tolerance half its last digit",
            run: period,
        },
        Case {
            id: "job/nonlinear-ridge-zhao-efficiency",
            title: r"The same job with their $d_{33}$ = 27 pm/V (p. 1) in `d_pm_per_v`: the SHG efficiency from the modes' overlap with the designed grating's $2/\pi$, against the library's overlap of the same guide on the same 20 nm cells, the tensor turned onto the chip by hand (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the library's `second_harmonic_coupling` of `Ridge` modes (nonlinear::guide), checked on its own by `nonlinear/guide-plane-wave` and `nonlinear/guide-slab-overlap`: one discrete overlap reached by two paths",
            run: efficiency,
        },
        Case {
            id: "job/nonlinear-fwm-depleted-loss",
            title: r"Four-wave mixing through a nonlinear job: $\gamma$ = 100 /(W·m), 10 mm, one index for every wave ($\Delta k = 0$), 1 W of pump at 1.55 µm and 300 mW of signal at 1.56 µm, every wave losing 3 dB/cm; the job's depleted pump, signal and idler at the end (2000 RK4 steps, the self- and cross-phase terms in) against the lossless guide over its effective length, the largest relative gap (the idler's W shown)",
            tier: Tier::Analytic,
            source: r"with one loss $\alpha$ for every wave and $\Delta k = 0$, $A_j = B_je^{-\alpha z/2}$ turns G. P. Agrawal, Nonlinear Fiber Optics, 6th ed. (2019), Eqs. (10.2.1)–(10.2.4) with their losses into the lossless ones in $\zeta = (1 - e^{-\alpha z})/\alpha$: the lossless guide over $L_\mathrm{eff}$ (20 000 RK4 steps), its powers times $e^{-\alpha L}$",
            run: fwm_depleted,
        },
        Case {
            id: "job/nonlinear-ridge-temperature-acceptance",
            title: r"Zhao et al. 2020's guide through a nonlinear job (as `job/nonlinear-ridge-zhao-period`, on 40 nm cells), with a phase map against temperature from 18 to 22 °C, each row's modes solved again: the setup's temperature acceptance, from the modes solved at 19, 20 and 21 °C, against Fejer et al.'s linear estimate from the map's rows at 19 and 21 °C at 1.57 µm (K shown)",
            tier: Tier::Analytic,
            source: r"M. M. Fejer et al., IEEE J. Quantum Electron. 28, 2631 (1992), doi:10.1109/3.161322, Eq. (25): $\delta T = 4 \cdot 1.39156/(L\lvert\partial\Delta k/\partial T\rvert)$, the derivative by central differences of the map's own $\Delta k$: one set of solves reached two ways",
            run: ridge_acceptance,
        },
        Case {
            id: "job/nonlinear-shg-type-ii-degenerate",
            title: r"Type-II SHG through a nonlinear job with both fundamental waves the same (1.55 µm over 10 mm of poled congruent LiNbO₃, every wave extraordinary, 1 W of pump split between the two, 0.5 dB/cm; $\eta$ = 50 %/(W·cm²) for type I, $P_2 = \eta P^2L^2$, and the same overlap's 200 %/(W·cm²) for type II, $P_3 = \eta P_1P_2L^2$) against the type-I job: the designed period, the wavelength acceptance, and at the end the depleted harmonic and the conversion (2000 RK4 steps), the largest relative gap (the type-II harmonic's mW shown)",
            tier: Tier::Analytic,
            source: r"with $A_1 = A_2 = A/\sqrt2$ and one field, the sum's coupling is twice the harmonic's ($\omega_3 = 2\omega$): $-i\kappa_\mathrm{sum}A_1A_2 = -i\kappa A^2$, and each fundamental's equation is half the type-I one's, so the three-wave equations are the two-wave ones: one process reached two ways",
            run: type_ii_degenerate,
        },
        Case {
            id: "job/nonlinear-ring-shg",
            title: r"SHG of 1.55 µm through a nonlinear job's `[task.ring]`: a lossless all-pass ring of poled congruent LiNbO₃ 1 mm round (the grating designed, $\eta$ = 10 %/(W·cm²)), 1 mW of pump, every wave's power coupling $k^2$ = 0.1 and every wave on resonance; the harmonic out, one pass's and each wave's build-up against the closed forms, the largest relative gap (the ring's harmonic in mW shown)",
            tier: Tier::Analytic,
            source: r"L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010), doi:10.1364/OL.35.003006: the field enhancement $F = ik/(1 - r)$ on resonance without loss, $r = \sqrt{1 - k^2}$, and $P_{2\omega} = \lvert F_{2\omega} \rvert^2 \lvert F_\omega \rvert^4 \eta P^2L^2 \mathrm{sinc}^2(\Delta kL/2)$, one pass with $\lvert F \rvert^2 = 1$",
            run: || ring_case("shg"),
        },
        Case {
            id: "job/nonlinear-ring-fwm",
            title: r"Four-wave mixing through a nonlinear job's `[task.ring]`: a lossless ring of one index 2.0 ($\Delta k = 0$), $\gamma$ = 100 /(W·m), 1 mm round, 10 mW of pump at 1.55 µm and the signal at 1.56 µm, every wave's $k^2$ = 0.1 and on resonance; the idler's conversion out, one pass's and each wave's build-up against the closed forms, the largest relative gap (the ring's $P_i/P_s$ shown)",
            tier: Tier::Analytic,
            source: r"Helt et al. 2010's Eq. (5): $P_i/P_s = (\gamma P_pL)^2 \lvert F_s \rvert^2 \lvert F_i \rvert^2 \lvert F_p \rvert^4$ on resonance without loss, $\lvert F \rvert^2 = k^2/(1 - r)^2$, one pass with $\lvert F \rvert^2 = 1$",
            run: || ring_case("fwm"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
        (0..3)
            .map(|j| (a[j] / b[j] - 1.0).abs())
            .fold(0.0, f64::max)
    }

    /// The temperature-acceptance case's gap and the central difference's own error, as the
    /// method page quotes them: run with `--ignored --nocapture` in release.
    #[test]
    #[ignore = "prints the ridge's temperature acceptance"]
    fn ridge_acceptance_error() {
        let (estimate, phases) = ridge_acceptance_and_map();
        let slope = (phases[3] - phases[1]) / 2.0;
        let wide = (phases[4] - phases[0]) / 4.0;
        let o = ridge_acceptance();
        eprintln!(
            "estimate {estimate} K, gap {:.2e}; slope {slope} /K, from 2 K {wide}, the 1 K difference's error {:.2e}; phases {phases:?}",
            o.error / o.expected,
            ((slope - wide) / 3.0 / slope).abs()
        );
    }

    #[test]
    #[ignore = "prints the type-II case's gaps"]
    fn type_ii_gaps() {
        for db in [0.0, 0.5] {
            let (one, two, gap) = type_ii_against_type_i(db);
            eprintln!("type II at {db} dB/cm: {gap:.2e} ({one:?} {two:?})");
        }
    }

    /// The four-wave case's gaps, as its rule quotes them: run with `--ignored --nocapture` in
    /// release.
    #[test]
    #[ignore = "prints the four-wave case's gaps"]
    fn fwm_gaps() {
        let best = fwm_equal_loss(FWM_DB, 400_000);
        for steps in [2000, 4000, 100_000, 200_000] {
            eprintln!(
                "reference on {steps}: {:.2e}",
                gap(fwm_equal_loss(FWM_DB, steps), best)
            );
        }
        let ours = fwm_job_end(&fwm_job(FWM_DB));
        eprintln!("job: {:.2e} ({ours:?} {best:?})", gap(ours, best));
        let case = fwm_depleted();
        eprintln!("case: {:.2e}", case.error / case.expected);
        let lossless = fwm_job_end(&fwm_job(0.0));
        eprintln!(
            "lossless job: {:.2e}",
            gap(lossless, fwm_equal_loss(0.0, 400_000))
        );
    }

    /// The ring cases' gaps, as their rules quote them.
    #[test]
    fn the_ring_jobs_meet_helts_closed_forms() {
        for p in ["shg", "fwm"] {
            let (ours, exact, worst) = ring_against_closed_form(p);
            eprintln!("ring {p}: {ours:e} against {exact:e}, gap {worst:.2e}");
            assert!(worst < RING_TOLERANCE, "{p}: {worst:e}");
        }
    }
}
