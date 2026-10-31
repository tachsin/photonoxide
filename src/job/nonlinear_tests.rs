//! The `"nonlinear"` kind: each process set up as the library sets it up, its map, its
//! efficiency against the closed forms, its pairs, its files, and what it refuses.

use super::{Event, check, execute};
use crate::material::catalogue::Axis;
use crate::nonlinear::coupled::coupling_from_efficiency;
use crate::nonlinear::phase::{Device, Expansion, Index, Process, State, Variable};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;

/// Fejer et al.'s first row: d₃₃ SHG of 1.064 µm in congruent LiNbO₃ over 1 mm, designed for
/// the pump at 25 °C, with Jundt's expansion.
const PPLN: &str = r#"
name = "ppln-shg"

[task]
kind = "nonlinear"
process = "shg"
length_mm = 1.0
temperature_c = 25.0

[task.waveguide]
efficiency_percent_per_w_cm2 = 100.0
expansion = "linbo3-jundt-1997"

[task.waveguide.crystal]
id = "linbo3"
model = "edwards-lawrence-1984"
axes = ["extraordinary"]

[task.poling]
order = 1
duty = 0.5

[task.pump]
wavelength_um = 1.064
power_mw = 100.0

[task.map]
wavelength_um = [1.06, 1.068]
points = 41
temperature_c = [15.0, 35.0]
across_points = 5
"#;

/// The events of `text`'s run, in memory.
fn events(text: &str) -> Vec<Event> {
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn setup_of(events: &[Event]) -> &super::NonlinearSetup {
    events
        .iter()
        .find_map(|e| match e {
            Event::NonlinearSetup { setup } => Some(&**setup),
            _ => None,
        })
        .unwrap()
}

fn efficiency_of(events: &[Event]) -> &super::EfficiencyCurve {
    events
        .iter()
        .find_map(|e| match e {
            Event::NonlinearEfficiency { curve } => Some(&**curve),
            _ => None,
        })
        .unwrap()
}

fn series<'a>(c: &'a super::EfficiencyCurve, starts: &str) -> &'a [Option<f64>] {
    &c.series
        .iter()
        .find(|s| s.label.starts_with(starts))
        .unwrap_or_else(|| panic!("no series {starts}"))
        .values
}

#[test]
fn a_crystals_shg_is_the_librarys_to_the_bit() {
    let e = events(PPLN);
    assert!(matches!(e[0], Event::Started { ref kind, .. } if kind == "nonlinear"));
    let s = setup_of(&e);
    // the same process from the library's own calls
    let ne =
        || Index::catalogue("linbo3", Some("edwards-lawrence-1984"), Axis::Extraordinary).unwrap();
    let shg = Process::second_harmonic(Wavelength::um(1.064).unwrap(), ne(), ne(), ne());
    let state = State::celsius(25.0);
    assert_eq!(s.mismatch_per_um, shg.mismatch(&state).unwrap());
    assert_eq!(s.coherence_length_um, shg.coherence_length(&state).unwrap());
    let device = Device::quasi_phase_matched(shg, 1000.0, state, 1, 0.5)
        .unwrap()
        .with_expansion(Expansion::LINBO3_JUNDT);
    let g = s.grating.as_ref().unwrap();
    assert!(g.designed);
    assert_eq!(g.period_um, device.grating().unwrap().period());
    let dt = device.acceptance(Variable::Temperature).unwrap();
    assert_eq!(
        s.temperature_acceptance_k.as_ref().unwrap().estimate,
        dt.linear
    );
    let dl = device.acceptance(Variable::Wavelength).unwrap();
    assert_eq!(
        s.wavelength_acceptance_nm.as_ref().unwrap().estimate,
        dl.linear * 1000.0
    );
    // Fejer et al.'s Table I prints 3.40 µm, 2.04 nm and 25.8 K for this row
    assert!((s.coherence_length_um - 3.40).abs() < 0.005);
    assert!((s.wavelength_acceptance_nm.as_ref().unwrap().estimate - 2.04).abs() < 0.005);
    assert!((s.temperature_acceptance_k.as_ref().unwrap().estimate - 25.8).abs() < 0.05);
    // matched at the pump, so the response is 1 and the residual is round-off
    assert!(s.residual_per_um.abs() < 1e-12, "{}", s.residual_per_um);
    assert!((s.response - 1.0).abs() < 1e-15);
    assert!((s.matched_um.unwrap() - 1.064).abs() < 1e-9);
    // the harmonic at 0.532 µm is inside LiNbO₃'s window, below its two-photon edge (0.64 µm)
    assert!(
        s.notes.iter().any(|n| n.contains("two-photon")),
        "{:?}",
        s.notes
    );
}

#[test]
fn the_map_is_the_devices_phase_over_wavelength_and_temperature() {
    let e = events(PPLN);
    let m = e
        .iter()
        .find_map(|e| match e {
            Event::PhaseMap { map } => Some(&**map),
            _ => None,
        })
        .unwrap();
    assert_eq!(m.across.as_deref(), Some("temperature_c"));
    assert_eq!(m.wavelengths_um.len(), 41);
    assert_eq!(m.across_values, [15.0, 20.0, 25.0, 30.0, 35.0]);
    assert_eq!(m.response.len(), 41 * 5);
    // the row at 25 °C peaks at the pump, 1.064 µm: column 20
    let row = 2 * 41;
    let peak = (0..41)
        .max_by(|&a, &b| {
            m.response[row + a]
                .unwrap()
                .total_cmp(&m.response[row + b].unwrap())
        })
        .unwrap();
    assert_eq!(peak, 20);
    assert!((m.response[row + 20].unwrap() - 1.0).abs() < 1e-12);
    // Δk and the response agree: sinc²(ΔkL/2) with L = 1 mm (no strain at 25 °C)
    let dk = m.mismatch_per_um[row + 3].unwrap();
    let x: f64 = 0.5 * dk * 1000.0;
    let sinc2 = (x.sin() / x).powi(2);
    assert!((m.response[row + 3].unwrap() - sinc2).abs() < 1e-12);
}

#[test]
fn shg_along_the_length_depletes_below_its_closed_form() {
    let e = events(PPLN);
    let c = efficiency_of(&e);
    assert_eq!(c.lengths_mm.len(), 101);
    assert_eq!(c.lengths_mm[100], 1.0);
    let depleted = series(c, "harmonic (depleted");
    let undepleted = series(c, "harmonic (undepleted");
    // 100 mW in 1 mm at 100 %/(W·cm²): η P² L² = 1e-2 · 1e-2 · 1e-2 W = 0.1 mW, barely depleted
    let eta = 100.0 * 1e-10; // /(W·µm²)
    let closed = eta * 0.1 * 0.1 * 1000.0 * 1000.0 * 1e3;
    assert!((undepleted[100].unwrap() / closed - 1.0).abs() < 1e-12);
    let (d, u) = (depleted[100].unwrap(), undepleted[100].unwrap());
    assert!(d < u && (u - d) / u < 2e-3, "{d} vs {u}");
    // the undepleted harmonic grows as L²
    assert!((undepleted[50].unwrap() / u - 0.25).abs() < 1e-12);
}

/// Effective indices given as data: one table, a linear index in wavelength.
fn table(points: &[(f64, f64)]) -> String {
    let (w, n): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
    format!("[[task.waveguide.n_eff]]\nwavelengths_um = {w:?}\nn_eff = {n:?}\n")
}

#[test]
fn the_three_wave_processes_conserve_photons_and_follow_their_closed_forms() {
    // an index linear in the wavelength over 0.5–3.5 µm: Δk₀ ≠ 0, the grating designed
    let n = table(&[(0.5, 2.3), (3.5, 2.1)]);
    let job = |process: &str, signal: f64, extra: &str| {
        format!(
            "name = \"{process}\"\n[task]\nkind = \"nonlinear\"\nprocess = \"{process}\"\n\
             length_mm = 10.0\n[task.waveguide]\nefficiency_percent_per_w_cm2 = 5.0\n{n}\
             [task.poling]\n[task.pump]\nwavelength_um = 0.775\npower_mw = 200.0\n\
             [task.signal]\nwavelength_um = {signal}\npower_mw = 1.0\n{extra}"
        )
    };
    // SFG of 0.775 and 1.55 µm: the sum's undepleted closed form, η P₁P₂L²
    let e = events(&job("sfg", 1.55, ""));
    let s = setup_of(&e);
    assert!((s.waves[2].wavelength_um - 1.0 / (1.0 / 0.775 + 1.0 / 1.55)).abs() < 1e-15);
    let c = efficiency_of(&e);
    let k = coupling_from_efficiency(5.0);
    let closed = k * k * 0.2 * 1e-3 * 1e4 * 1e4 * 1e3;
    let u = series(c, "sum (undepleted")[100].unwrap();
    assert!((u / closed - 1.0).abs() < 1e-12, "{u} vs {closed}");
    let d = series(c, "sum (depleted")[100].unwrap();
    assert!(d <= u && (u - d) / u < 0.05, "{d} vs {u}");
    // DFG and OPA: the idler of 0.775 µm less 1.3 µm, the signal amplified
    let e = events(&job("dfg", 1.3, ""));
    let s = setup_of(&e);
    assert!((s.waves[2].wavelength_um - 1.0 / (1.0 / 0.775 - 1.0 / 1.3)).abs() < 1e-12);
    let c = efficiency_of(&e);
    let (idler, signal) = (series(c, "idler (depleted"), series(c, "signal (depleted"));
    // Manley–Rowe: the signal gains a photon for each idler photon made (lossless)
    let gained = signal[100].unwrap() - 1.0;
    let made = idler[100].unwrap();
    let ratio = (gained / (1.0 / 1.3)) / (made * s.waves[2].wavelength_um);
    assert!((ratio - 1.0).abs() < 1e-9, "{ratio}");
    let e = events(&job("opa", 1.3, ""));
    let c = efficiency_of(&e);
    let gain = series(c, "signal gain (depleted")[100].unwrap();
    let closed = series(c, "signal gain (undepleted")[100].unwrap();
    assert!(
        gain > 0.0 && (gain - closed).abs() < 1e-3,
        "{gain} vs {closed}"
    );
}

#[test]
fn pairs_by_spdc_and_fwm_have_a_joint_spectrum() {
    let n = table(&[(0.5, 2.3), (3.5, 2.1)]);
    let e = events(&format!(
        "name = \"spdc\"\n[task]\nkind = \"nonlinear\"\nprocess = \"spdc\"\nlength_mm = 5.0\n\
         [task.waveguide]\nefficiency_percent_per_w_cm2 = 100.0\n{n}[task.poling]\n\
         [task.pump]\nwavelength_um = 0.775\npower_mw = 1.0\nbandwidth_nm = 0.5\n\
         [task.pairs]\npoints = 41\n"
    ));
    let s = setup_of(&e);
    // degenerate by default
    assert!((s.waves[1].wavelength_um - 1.55).abs() < 1e-15);
    let p = e
        .iter()
        .find_map(|e| match e {
            Event::PairSpectrum { spectrum } => Some(&**spectrum),
            _ => None,
        })
        .unwrap();
    assert_eq!(p.amplitude.len(), 41 * 41);
    assert!(p.purity > 0.0 && p.purity <= 1.0 + 1e-12);
    assert!((p.schmidt_number * p.purity - 1.0).abs() < 1e-12);
    assert_eq!(p.amplitude.iter().copied().fold(0.0, f64::max), 1.0);
    // the brightness grows as L² at phase matching: η P L² per Hz at degeneracy, κ the sum's
    let c = efficiency_of(&e);
    let b = series(c, "spectral brightness");
    assert!((b[50].unwrap() / b[100].unwrap() - 0.25).abs() < 1e-9);
    // FWM in a guide of given γ: the signal's gain from the closed form
    let e = events(&format!(
        "name = \"fwm\"\n[task]\nkind = \"nonlinear\"\nprocess = \"fwm\"\nlength_mm = 10.0\n\
         [task.waveguide]\ngamma_per_w_m = 100.0\n{n}[task.pump]\nwavelength_um = 1.55\n\
         power_mw = 500.0\n[task.signal]\nwavelength_um = 1.56\n[task.pairs]\npoints = 21\n"
    ));
    let s = setup_of(&e);
    assert!((s.waves[2].wavelength_um - 1.0 / (2.0 / 1.55 - 1.0 / 1.56)).abs() < 1e-12);
    let c = efficiency_of(&e);
    let dk = s.mismatch_per_um * 1e6;
    let g = crate::nonlinear::fwm::signal_gain(100.0, 0.5, dk, 0.01);
    assert!((series(c, "signal gain")[100].unwrap() - 10.0 * g.log10()).abs() < 1e-12);
    assert!(e.iter().any(|e| matches!(e, Event::PairSpectrum { .. })));
}

#[test]
fn a_run_with_a_folder_writes_its_numbers_beside_its_record() {
    let dir = std::env::temp_dir().join(format!("photonoxide-nonlinear-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let job = Job::parse(PPLN).unwrap();
    let mut run = Run::create(&dir, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let folder = run.dir().to_path_buf();
    for name in [
        "nonlinear.json",
        "phase_map.csv",
        "efficiency.csv",
        "results.json",
    ] {
        assert!(folder.join(name).is_file(), "{name}");
    }
    let csv = std::fs::read_to_string(folder.join("phase_map.csv")).unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("temperature_c,wavelength_um,mismatch_rad_per_um,response")
    );
    assert_eq!(lines.count(), 41 * 5);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join("nonlinear.json")).unwrap())
            .unwrap();
    assert_eq!(json["setup"]["process"], "shg");
    let results = crate::job::results::Results::read(&folder).unwrap();
    assert_eq!(
        (results.kind.as_str(), results.outcome.as_str()),
        ("nonlinear", "finished")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_ridges_modes_set_up_its_process() {
    // Zhao et al.'s guide (Phys. Rev. Lett. 124, 163603 (2020)) on 40 nm cells, behind a mirror
    let e = events(
        r#"
name = "zhao-shg"

[task]
kind = "nonlinear"
process = "shg"
length_mm = 5.0

[task.waveguide.ridge]
layer = "LN"
width_um = 1.24
etch_um = 0.05
sidewall_deg = 75.0
along = "y"
step_nm = 40.0
families = ["te"]
tolerance = 1e-7
mirror = true
# their own d33, 27 pm/V (p. 1), alone: the catalogue's MgO:LN tensor leaves d22 out
d_pm_per_v = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 27.0, 0.0, 0.0, 0.0]]

[task.waveguide.ridge.stack]
substrate = "sio2"
cladding = 1.0

[[task.waveguide.ridge.stack.layer]]
name = "LN"
thickness_um = 0.3
material = { id = "linbo3-mgo", orientation = { cut = "x", along_x = "z" } }
background = 1.0

[task.poling]

[task.pump]
wavelength_um = 1.57
power_mw = 10.0
"#,
    );
    let s = setup_of(&e);
    assert_eq!(s.waveguide, "ridge");
    // their period, 2.8 µm, to its printed digit on these coarse cells (2.8142 on 5 nm ones)
    let g = s.grating.as_ref().unwrap();
    assert!((g.period_um - 2.8).abs() < 0.05, "{}", g.period_um);
    // the TE₀ modes' indices and group indices, from the interpolants
    assert!(
        s.waves[0].index > 1.65 && s.waves[0].index < 1.75,
        "{}",
        s.waves[0].index
    );
    assert!(s.waves[0].group_index.unwrap() > s.waves[0].index);
    // η from the overlap through their d33, the grating's 2/π in the device's (4158.9 %/(W·cm²) on
    // 20 nm cells, docs/methods/nonlinear.md)
    let overlap = s
        .overlap_percent_per_w_cm2
        .unwrap_or_else(|| panic!("{:?}", s.notes));
    let eta = s.efficiency_percent_per_w_cm2.unwrap();
    assert!(eta > 3500.0 && eta < 5000.0, "{eta}");
    let gm = 2.0 / std::f64::consts::PI;
    assert!((eta / (overlap * gm * gm) - 1.0).abs() < 1e-12);
}

#[test]
fn what_a_nonlinear_task_refuses() {
    let base =
        |task: &str| format!("name = \"x\"\n[task]\nkind = \"nonlinear\"\nlength_mm = 1.0\n{task}");
    let crystal = "[task.waveguide.crystal]\nid = \"linbo3\"\naxes = [\"e\"]\n";
    for (task, says) in [
        (
            format!("process = \"thg\"\n{crystal}[task.pump]\nwavelength_um = 1.0\n"),
            "unknown process",
        ),
        (
            format!("process = \"sfg\"\n{crystal}[task.pump]\nwavelength_um = 1.0\n"),
            "needs [task.signal]",
        ),
        (
            format!(
                "process = \"dfg\"\n{crystal}[task.pump]\nwavelength_um = 1.0\n[task.signal]\nwavelength_um = 0.9\n"
            ),
            "longer than the pump",
        ),
        (
            "process = \"shg\"\n[task.waveguide]\n[task.pump]\nwavelength_um = 1.0\n".to_owned(),
            "one of crystal, n_eff or ridge",
        ),
        (
            format!(
                "process = \"fwm\"\n{crystal}[task.poling]\n[task.pump]\nwavelength_um = 1.55\n[task.signal]\nwavelength_um = 1.56\n"
            ),
            "third-order",
        ),
        (
            format!(
                "process = \"shg\"\n{crystal}[task.pump]\nwavelength_um = 1.064\n[task.map]\nwavelength_um = [1.06, 1.07]\nwidth_um = [1.0, 2.0]\n"
            ),
            "a ridge's",
        ),
        (
            "process = \"shg\"\n[task.waveguide.crystal]\nid = \"linbo3\"\naxes = [\"e\", \"o\", \"e\"]\n[task.pump]\nwavelength_um = 1.064\n"
                .to_owned(),
            "one per wave (2)",
        ),
        (
            format!(
                "process = \"shg\"\n{crystal}[task.pump]\nwavelength_um = 1.064\n[task.pairs]\n"
            ),
            "photon pairs",
        ),
        (
            format!(
                "process = \"shg\"\n{crystal}[task.poling]\nduty = 1.5\n[task.pump]\nwavelength_um = 1.064\n"
            ),
            "duty cycle",
        ),
    ] {
        let job = Job::parse(&base(&task)).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{says}: {e}");
    }
}
