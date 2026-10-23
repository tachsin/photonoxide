//! A modes job's PML (`[task.pml]`, #373): what [`super::check`] refuses, what a count keeps in
//! an open window and how it names it, the losses recorded, a job without a PML unchanged, its
//! replay and the same bits on any number of threads; and `[task.stack]`'s fixed indices.

use super::leaky_checks::{film_job, wire};
use super::*;
use crate::run::replay;

/// A run directory under the system's temporary one, removed when dropped.
struct TempDir(std::path::PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> TempDir {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "photonoxide-leaky-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    TempDir(dir)
}

/// The events of `text`'s run, in memory, on rayon's pool of `threads` (its default with
/// `None`).
fn events_on(text: &str, threads: Option<usize>) -> (Vec<String>, Vec<Event>) {
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let mut run = Run::in_memory();
    match threads {
        Some(n) => rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build()
            .unwrap()
            .install(|| execute(&job, &mut run, &Stop::new(None)))
            .unwrap(),
        None => execute(&job, &mut run, &Stop::new(None)).unwrap(),
    }
    let lines = run.lines().to_vec();
    let events = lines
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    (lines, events)
}

fn events(text: &str) -> Vec<Event> {
    events_on(text, None).1
}

/// The modes' effective indices, in the order recorded.
fn indices(events: &[Event]) -> Vec<[f64; 2]> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Mode {
                effective_index, ..
            } => Some(*effective_index),
            _ => None,
        })
        .collect()
}

/// The modes' kinds and losses, in the order recorded.
fn losses(events: &[Event]) -> Vec<(Option<usize>, String, String, f64)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ModeLoss {
                point,
                label,
                kind,
                loss_db_per_cm,
            } => Some((*point, label.clone(), kind.clone(), *loss_db_per_cm)),
            _ => None,
        })
        .collect()
}

/// The solver's details, by name.
fn detail(events: &[Event], name: &str) -> Option<String> {
    events.iter().find_map(|e| match e {
        Event::Solver { details, .. } => details
            .iter()
            .find(|[n, _]| n == name)
            .map(|[_, v]| v.clone()),
        _ => None,
    })
}

/// `jobs/strip-modes.yaml` (as its TOML, `tests/jobs-toml`) without its sweep, two modes, in a window wide enough (4.4 by 2.6 µm)
/// that a PML 0.6 µm thick lies where its guided modes' fields have died out.
fn strip() -> String {
    include_str!("../../tests/jobs-toml/strip-modes.toml")
        .replace("\r\n", "\n")
        .split("[task.sweep]")
        .next()
        .unwrap()
        .replace(
            "y_um = [-1.2, 1.2]",
            "y_um = [-2.2, 2.2]\nz_um = [0.8, 3.4]",
        )
}

const PML: &str = "\n[task.pml]\nthickness_um = [0.6, 0.6, 0.6, 0.6]\n";

#[test]
fn a_strips_job_with_and_without_a_pml_keeps_the_same_guided_modes() {
    let closed = events(&strip());
    let open = events(&format!("{}{PML}", strip()));
    let (a, b) = (indices(&closed), indices(&open));
    // the TE-like and TM-like modes, both guided both ways
    assert_eq!(a.len(), 2, "{a:?}");
    assert_eq!(b.len(), 2, "{b:?}");
    for (a, b) in a.iter().zip(&b) {
        // the solver's tolerance on a mode's index, and the PML's small effect on its tail
        assert!((a[0] - b[0]).abs() < 1e-5, "{a:?} {b:?}");
        assert!(b[1].abs() < 1e-5, "{b:?}");
    }
    // a closed window records no loss, an open one each mode's, guided here
    assert!(losses(&closed).is_empty());
    let l = losses(&open);
    assert_eq!(l.len(), 2);
    for (k, (point, label, kind, _)) in l.iter().enumerate() {
        assert_eq!(*point, None);
        assert_eq!(label, &format!("mode {} of 2", k + 1));
        assert_eq!(kind, "guided");
    }
    assert!(detail(&closed, "PML").is_none());
    let pml = detail(&open, "PML").unwrap();
    assert!(pml.contains("0.6, 0.6, 0.6 and 0.6 um") && pml.contains("strength 3"));
    let asked = detail(&open, "modes asked for").unwrap();
    assert!(asked.contains("guided above 1.4440") && asked.contains("the PML's own"));
}

#[test]
fn a_leaky_film_records_its_loss_after_its_pictures() {
    let events = events(&film_job(20.0, 1.0, 3.0));
    let at = |k: usize| &events[k];
    let mode = events
        .iter()
        .position(|e| matches!(e, Event::Mode { .. }))
        .unwrap();
    let Event::Mode {
        label,
        effective_index,
        wavelength_um,
        ..
    } = at(mode)
    else {
        unreachable!()
    };
    assert!(matches!(at(mode + 1), Event::ModeField { .. }));
    let Event::ModeLoss {
        point,
        label: loss_label,
        kind,
        loss_db_per_cm,
    } = at(mode + 2)
    else {
        panic!("{:?}", at(mode + 2))
    };
    assert_eq!(*point, None);
    assert_eq!(loss_label, label);
    // below the substrate's 3.473, above the oxide's 1.444: leaky
    assert_eq!(kind, "leaky");
    let n = num_complex::Complex64::new(effective_index[0], effective_index[1]);
    let w = Wavelength::um(*wavelength_um).unwrap();
    assert_eq!(
        loss_db_per_cm.to_bits(),
        crate::mode::dispersion::loss_db_per_cm(n, w).to_bits()
    );
    // about 7 dB/cm through 0.5 um of oxide (7.6 on this coarse grid)
    assert!((6.0..8.5).contains(loss_db_per_cm), "{loss_db_per_cm}");
    let asked = detail(&events, "modes asked for").unwrap();
    assert!(asked.contains("guided above 3.4730") && asked.contains("down to 1.4440"));
    assert!(
        detail(&events, "eigen-solver")
            .unwrap()
            .contains("n_eff = 2.85")
    );
    assert!(detail(&events, "loss").unwrap().contains("dB/cm"));
}

#[test]
fn the_pmls_own_modes_are_left_out() {
    // the wire's substrate holds the highest index: nearest it, every mode solved is the PML's
    // own, most of its field inside it, and none is kept
    let text = include_str!("../../tests/jobs-toml/leaky-wire.toml")
        .replace("step_nm = 10.0", "step_nm = 20.0");
    let high = events(&text.replace("modes_near = 2.4 ", "# "));
    assert!(indices(&high).is_empty(), "{:?}", indices(&high));
    assert!(losses(&high).is_empty());
    // nearest 2.4, the wire's leaky mode
    let n = wire(20.0);
    assert!(
        (n.re - 2.4212).abs() < 1e-3 && (2e-8..4e-8).contains(&n.im),
        "{n}"
    );
    // a strip in oxide with a PML: below the oxide, what radiates into every medium around it is
    // the window's, not the strip's
    let open = events(&format!("{}{PML}", strip()).replace("modes = 2", "modes = 12"));
    for [re, _] in indices(&open) {
        assert!(re > 1.444, "{re}");
    }
}

#[test]
fn a_sweep_records_each_points_losses() {
    let text = format!(
        "{}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n",
        film_job(20.0, 1.0, 3.0)
    );
    let events = events(&text);
    let l = losses(&events);
    // the job's own mode, then each point's
    assert_eq!(l.len(), 3, "{l:?}");
    assert_eq!(l[0].0, None);
    for (k, point) in [(1, 0), (2, 1)] {
        assert_eq!(l[k].0, Some(point));
        assert_eq!(l[k].2, "leaky");
    }
    // each right after its point's mode, with its label
    for (k, e) in events.iter().enumerate() {
        if let Event::SweepMode { point, label, .. } = e {
            let Event::ModeLoss {
                point: p, label: l, ..
            } = &events[k + 1]
            else {
                panic!("{:?}", events[k + 1])
            };
            assert_eq!((Some(*point), label), (*p, l));
        }
    }
    // a longer wavelength reaches further through the oxide
    assert!(l[2].3 > l[1].3, "{l:?}");
}

#[test]
fn a_pml_runs_record_replays_and_repeats_on_any_threads() {
    let text = film_job(20.0, 1.0, 3.0);
    let root = temp("replay");
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let replayed: Vec<Event> = replay(run.dir()).unwrap();
    let in_memory = events(&text);
    let same = |e: &Event| !matches!(e, Event::Finished { .. });
    assert_eq!(
        replayed.iter().filter(|e| same(e)).collect::<Vec<_>>(),
        in_memory.iter().filter(|e| same(e)).collect::<Vec<_>>()
    );
    let lines = |threads| {
        events_on(&text, Some(threads))
            .0
            .into_iter()
            .filter(|l| !l.starts_with("{\"type\":\"finished\""))
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(1), lines(4));
}

#[test]
fn check_refuses_a_pml_that_doesnt_fit() {
    let base = film_job(20.0, 1.0, 3.0);
    let refused = |text: String, words: &str| {
        let Err(e) = check(&Job::parse(&text).unwrap()) else {
            panic!("not refused, for {words}:\n{text}")
        };
        assert!(e.to_string().contains(words), "{e}");
    };
    let pml = "thickness_um = [0.0, 0.0, 1, 0.0]";
    assert!(base.contains(pml));
    for (with, words) in [
        ("thickness_um = [0.0, 0.0, -1.0, 0.0]", "at least 0"),
        ("thickness_um = [0.0, 0.0, 0.0, 0.0]", "opens no edge"),
        ("thickness_um = [0.0, 0.0, 0.06, 0.0]", "fewer than 4 cells"),
        (
            "thickness_um = [0.3, 0.3, 1.0, 0.0]",
            "fill the window across",
        ),
        (
            "thickness_um = [0.0, 0.0, 2.1, 0.0]",
            "above the bottom of the layer film",
        ),
        (
            "thickness_um = [0.0, 0.0, 1.0, 1.1]",
            "below the top of the layer film",
        ),
        (
            "thickness_um = [0.0, 0.0, 1.0, 0.0]\nwidth = 1.0",
            "unknown field",
        ),
    ] {
        refused(base.replace(pml, with), words);
    }
    refused(
        base.replace("strength = 3", "strength = 0"),
        "strength must be",
    );
    refused(
        base.replace("modes = 1", "modes_above = 2.0"),
        "modes_near = 2.85 is for a count",
    );
    refused(
        base.replace("modes_near = 2.85", "modes_near = -1.0"),
        "positive",
    );
    // a guide under the PML across: the strip moved to the window's side
    let strip = format!("{}{PML}", strip());
    refused(
        strip.replace("center_um = [0.0, 0.0]", "center_um = [0.0, 1.9]"),
        "no shape of the layer Si meets the cut there",
    );
    // the strength is 3 when not given
    check(&Job::parse(&base.replace("strength = 3\n", "")).unwrap()).unwrap();
}

#[test]
fn a_task_stack_takes_fixed_indices() {
    let text = include_str!("../../tests/jobs-toml/leaky-wire.toml");
    let job = Job::parse(text).unwrap();
    let task = modes_task(&job, None).unwrap();
    let stack = named_stack(&task.stack, task.core_nm, task.bottom_oxide_um).unwrap();
    let w = Wavelength::um(1.55).unwrap();
    let n = |m: &crate::material::Material| m.refractive_index(w).unwrap().re;
    assert_eq!(n(stack.substrate()), 3.5);
    assert_eq!(n(stack.cladding()), 1.0);
    assert_eq!(n(&stack.layers()[0].material), 1.45);
    assert_eq!(stack.layers()[1].background.name(), "n = 1");
    // the same at any wavelength
    let far = Wavelength::um(10.0).unwrap();
    assert_eq!(stack.substrate().refractive_index(far).unwrap().re, 3.5);
    // below 1, or not a number, is refused; a catalogue id is read as before
    for (with, words) in [
        ("substrate = 0.5", "at least 1"),
        ("substrate = true", "catalogue id"),
    ] {
        let e = check(&Job::parse(&text.replace("substrate = 3.5", with)).unwrap())
            .unwrap_err()
            .to_string();
        assert!(e.contains(words), "{e}");
    }
    check(&Job::parse(&text.replace("substrate = 3.5", "substrate = \"si\"")).unwrap()).unwrap();
}
