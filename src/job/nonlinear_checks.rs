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
}
