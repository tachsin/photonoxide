//! What the validation report checks a modes job's leaky modes on (a `[task.pml]`, #373): a
//! film leaking into its substrate against the exact leaky mode, its convergence in the grid and
//! its insensitivity to the PML, and Bienstman et al.'s leaky wire, `jobs/leaky-wire.yaml`,
//! against their published index and loss.

use num_complex::Complex64 as c64;

use super::{Event, execute};
use crate::mode::Polarization;
use crate::mode::multilayer::Multilayer;
use crate::run::{Job, Run, Stop};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid leaky job";

/// The silicon and the oxide of the film, fixed indices as the library's own leakage case has
/// them (mode/pml-soi-leakage-te).
const SI: f64 = 3.473;
const OXIDE: f64 = 1.444;

/// The buried oxide under the film, µm.
const BOX_UM: f64 = 0.5;

/// Half the film job's window across the guide, µm: the film spans it, so the window's zero
/// walls make a standing wave of one half-period across it.
const HALF: f64 = 0.3;

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "job/leaky-film-loss",
            title: r"A modes job with a PML (1 µm in the substrate, strength 3): 220 nm of silicon (3.473) in oxide (1.444) on 0.5 µm of buried oxide over a silicon substrate, across the job's whole window, its leaky TE mode on a 5 nm grid, the window's standing wave across taken out (relative error in $\operatorname{Im} n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: r"the exact leaky mode of the same stack by transfer matrices (mode::multilayer, J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742); the film is uniform across, so the job's mode is the film's times the zero walls' lowest standing wave, $n^2 = n_\text{film}^2 - \lambda_x/k_0^2$ with $\lambda_x = (4/h^2)\sin^2(\pi/(2(N+2)))$ on N cells of h (the same to 7 digits in windows of 0.6, 3 and 6 µm)",
            run: film_loss,
        },
        Case {
            id: "job/leaky-film-grid-order",
            title: r"The same film's loss from the job on grids of 10 and 5 nm: the order of convergence of $\operatorname{Im} n_\text{eff}$'s relative error (shown)",
            tier: Tier::Analytic,
            source: "the full-vector scheme's second order at interfaces (A. B. Fallahkhair et al., J. Lightwave Technol. 26, 1423 (2008), doi:10.1109/JLT.2008.923643); the errors are 2.7e-2, 6.8e-3, 1.7e-3 and 4.2e-4 at 20, 10, 5 and 2.5 nm",
            run: film_order,
        },
        Case {
            id: "job/leaky-film-pml",
            title: r"The same film's loss from the job on a 10 nm grid with PMLs of 0.5 and 1 µm at strength 3, and 1 µm at strength 10, against 1.5 µm at strength 3 (largest relative difference in $\operatorname{Im} n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: "a PML absorbs what reaches it without reflecting, at any thickness and strength (W. C. Chew et al., Microw. Opt. Technol. Lett. 15, 363 (1997), as mode/pml-soi-leakage-te cites it), up to its own discretization: 1.8e-4 at 0.5 µm (50 cells), 2.9e-6 at 1 µm, 2.8e-5 at strength 10",
            run: film_pml,
        },
        Case {
            id: "job/leaky-wire-bienstman",
            title: r"Bienstman et al.'s leaky SOI wire through a modes job, `jobs/leaky-wire.yaml` ($500 \times 220$ nm Si 3.5 on 1 µm SiO₂ 1.45 on Si, air above, 1.55 µm; a 1 µm PML of strength 3 in the substrate; a window of 2 × 3.6 µm), on a uniform 5 nm grid: $\operatorname{Re} n_\text{eff}$ (shown)",
            tier: Tier::Published,
            source: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), doi:10.1007/s11082-006-9025-9, Table 6: 2.412372; the tolerance is the 5 nm grid's error, +4.0e-3 here and on the library's graded 5 nm grid alike (mode/leaky-wire-bienstman extrapolates from finer ones): the core's corners converge slowly, +8.8e-3 and +6.4e-3 at 20 and 10 nm",
            run: wire_re,
        },
        Case {
            id: "job/leaky-wire-bienstman-loss",
            title: r"The same wire's substrate leakage from the job, $\operatorname{Im} n_\text{eff} \times 10^8$ (shown)",
            tier: Tier::Published,
            source: "P. Bienstman et al., Opt. Quantum Electron. 38, 731 (2006), doi:10.1007/s11082-006-9025-9, Table 6: 2.9135 (CAMFR); the tolerance is the 5 nm grid's error, 0.963 of it here (0.94 and 0.95 at 20 and 10 nm), 0.97 on the library's graded 5 nm grid",
            run: wire_im,
        },
    ]
}

/// The events of `text`'s run, in memory.
fn events_of(text: &str, format: crate::run::Format) -> Vec<Event> {
    let job = Job::parse_as(text, format).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).expect("a recorded event"))
        .collect()
}

/// The film's modes job at `step_nm`, its substrate's PML `pml_um` thick at `strength`: the film
/// spans the window across, and the window reaches 0.5 µm of the substrate above the PML and
/// 1 µm of the oxide above the film.
pub(super) fn film_job(step_nm: f64, pml_um: f64, strength: f64) -> String {
    format!(
        r#"
name = "leaky-film"
timeout_minutes = 30

[task]
kind = "modes"
wavelength_um = 1.55
layer = "film"
propagation = "x"
y_um = [-{HALF}, {HALF}]
z_um = [{bottom}, {top}]
step_nm = {step_nm}
modes = 1
modes_near = 2.85

[task.stack]
substrate = {SI}
cladding = {OXIDE}

[[task.stack.layer]]
name = "BOX"
thickness_um = {BOX_UM}
material = {OXIDE}
background = {OXIDE}

[[task.stack.layer]]
name = "film"
thickness_um = 0.22
material = {SI}
background = {OXIDE}

[[task.rect]]
layer = "film"
center_um = [0.0, 0.0]
size_um = [10.0, {wide}]

[task.pml]
thickness_um = [0.0, 0.0, {pml_um}, 0.0]
strength = {strength}
"#,
        bottom = -(pml_um + 0.5),
        top = BOX_UM + 0.22 + 1.0,
        wide = 4.0 * HALF,
    )
}

/// The film's effective index from its job: the job's mode with the window's standing wave
/// across taken out, n_film² = n² + λ_x/k₀², λ_x the zero walls' lowest eigenvalue on the job's
/// uniform cells across (`None` if the job found no mode).
pub(super) fn film_index(step_nm: f64, pml_um: f64, strength: f64) -> Option<c64> {
    let events = events_of(
        &film_job(step_nm, pml_um, strength),
        crate::run::Format::Toml,
    );
    let cells = events.iter().find_map(|e| match e {
        Event::Solver { cells, .. } => Some(cells[0]),
        _ => None,
    })?;
    let n = events.iter().find_map(|e| match e {
        Event::Mode {
            effective_index, ..
        } => Some(c64::new(effective_index[0], effective_index[1])),
        _ => None,
    })?;
    #[allow(clippy::cast_precision_loss)]
    let (cells, h) = (cells as f64, 2.0 * HALF / cells as f64);
    let lx = 4.0 / (h * h) * (std::f64::consts::PI / (2.0 * (cells + 2.0))).sin().powi(2);
    let k0 = Wavelength::um(1.55).ok()?.wavenumber();
    Some((n * n + lx / (k0 * k0)).sqrt())
}

/// The film's exact leaky TE mode, by transfer matrices.
pub(super) fn film_exact() -> c64 {
    Multilayer::new(
        c64::new(OXIDE, 0.0),
        &[
            (c64::new(SI, 0.0), Length::nm(220.0)),
            (c64::new(OXIDE, 0.0), Length::um(BOX_UM)),
        ],
        c64::new(SI, 0.0),
    )
    .and_then(|stack| {
        stack.mode_near(
            Polarization::Te,
            Wavelength::um(1.55)?,
            c64::new(2.85, 1e-6),
        )
    })
    .map_or(c64::new(f64::NAN, f64::NAN), |m| m.effective_index())
}

/// The relative error of the film's loss from its job at `step_nm` (1 µm PML, strength 3).
fn film_error(step_nm: f64) -> f64 {
    film_index(step_nm, 1.0, 3.0).map_or(f64::NAN, |n| (n.im / film_exact().im - 1.0).abs())
}

fn film_loss() -> Outcome {
    let error = film_error(5.0);
    Outcome {
        measured: error,
        expected: 0.0,
        // second order: 5 nm leaves 1.7e-3
        tolerance: 3e-3,
        error,
    }
}

fn film_order() -> Outcome {
    // two decimals, its error too: the report reads the same on every platform
    let order = ((film_error(10.0) / film_error(5.0)).log2() * 100.0).round() / 100.0;
    Outcome {
        measured: order,
        expected: 2.0,
        tolerance: 0.1,
        error: (order - 2.0).abs(),
    }
}

fn film_pml() -> Outcome {
    let loss = |pml: f64, strength: f64| film_index(10.0, pml, strength).map_or(f64::NAN, |n| n.im);
    let reference = loss(1.5, 3.0);
    let worst = [(0.5, 3.0), (1.0, 3.0), (1.0, 10.0)]
        .iter()
        .map(|&(pml, strength)| (loss(pml, strength) / reference - 1.0).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 5e-4,
        error: worst,
    }
}

/// `jobs/leaky-wire.yaml` on a grid of `step_nm`: its leaky mode's effective index.
pub(super) fn wire(step_nm: f64) -> c64 {
    let text = include_str!("../../jobs/leaky-wire.yaml")
        .replace("step_nm: 10.0", &format!("step_nm: {step_nm:.1}"));
    events_of(&text, crate::run::Format::Yaml)
        .iter()
        .find_map(|e| match e {
            Event::Mode {
                effective_index, ..
            } => Some(c64::new(effective_index[0], effective_index[1])),
            _ => None,
        })
        .unwrap_or(c64::new(f64::NAN, f64::NAN))
}

/// The wire on its 5 nm grid, solved once for both cases.
fn wire_5nm() -> c64 {
    static FOUND: std::sync::OnceLock<c64> = std::sync::OnceLock::new();
    *FOUND.get_or_init(|| wire(5.0))
}

fn wire_re() -> Outcome {
    let n = wire_5nm().re;
    Outcome {
        measured: n,
        expected: 2.412372,
        // the 5 nm grid's +4.0e-3
        tolerance: 5e-3,
        error: (n - 2.412372).abs(),
    }
}

fn wire_im() -> Outcome {
    let im = wire_5nm().im * 1e8;
    Outcome {
        measured: im,
        expected: 2.9135,
        // the 5 nm grid's 3.7 %
        tolerance: 0.15,
        error: (im - 2.9135).abs(),
    }
}
