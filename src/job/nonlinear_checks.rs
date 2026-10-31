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
    ]
}
