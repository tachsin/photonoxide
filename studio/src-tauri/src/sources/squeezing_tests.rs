use super::*;
use crate::sources::{Arm, Filter, ModeSpec, PairProcess, PumpSpec, designed};
use photonoxide::portable;

fn bare() -> Arm {
    Arm {
        filter: Filter::None,
        coupler: None,
        coupler_loss_db: 0.0,
        detector: 1.0,
    }
}

/// Down-conversion of a 1 nm pump at 0.775 µm into 1.55 µm pairs in a 10 mm guide of group
/// indices `ng` (pump, signal, idler), no dispersion, its own sinc phase matching, on a grid
/// 15 nm either side of the signal.
fn guide(ng: [f64; 3]) -> Design {
    Design {
        medium: Medium::Guide {
            process: PairProcess::Spdc,
            group_index: ng,
            dispersion: [0.0; 3],
            length_mm: 10.0,
            matching: Shape::Sinc,
            efficiency: None,
        },
        pump: PumpSpec {
            wavelength_um: 0.775,
            bandwidth_nm: 1.0,
        },
        signal_um: None,
        half_nm: 15.0,
        points: 121,
        mean_pairs: 0.01,
        arms: [bare(), bare()],
    }
}

/// The correlated guide: the signal faster than the pump and the idler.
const CORRELATED: [f64; 3] = [2.3, 2.2, 2.3];

fn at(gain: f64, slices: usize, points: usize) -> Gain {
    Gain {
        gain,
        slices,
        points,
    }
}

/// With every wave at one group index and no dispersion, ΔkL/2 is zero at every pair of
/// frequencies: the slices are equal and commute, and their ordered product is the single
/// exponential at any gain, its squeezing g√λ_k, its modes the Schmidt modes.
#[test]
fn slices_that_commute_give_the_schmidt_model_at_any_gain() {
    let s = squeezed(&guide([2.3; 3]), &at(3.0, 20, 41)).unwrap();
    let [sliced, schmidt] = &s.squeezing;
    assert_eq!(sliced.len(), schmidt.len());
    for (k, (a, b)) in sliced.iter().zip(schmidt).enumerate() {
        assert!((a - b).abs() < 1e-9 * schmidt[0], "r_{k}: {a} against {b}");
    }
    let [p, q] = s.mean_photons;
    assert!((p - q).abs() < 1e-9 * q, "{p} against {q} photons");
    // a positive amplitude's first singular value is simple (Perron), so its mode is defined
    assert!(s.modes[0].shift < 1e-9, "{}", s.modes[0].shift);
    assert!(s.slice_change.abs() < 1e-9, "{}", s.slice_change);
    // the amplitude α(ν_s + ν_i) runs along the anti-diagonal off any grid, and the view says so
    assert!(
        s.notes.iter().all(|n| n.contains("widen the grid")),
        "{:?}",
        s.notes
    );
}

/// The Schmidt model's squeezing is g√λ_k with the designer's own weights, its device's sinc
/// on the same grid: the low-gain view and this one are one source.
#[test]
fn the_schmidt_model_squeezes_the_designers_own_modes() {
    let gain = 2.0;
    let mut d = guide(CORRELATED);
    d.points = 61;
    let weights = designed(&d).unwrap().weights;
    let s = squeezed(&d, &at(gain, 40, 61)).unwrap();
    for (k, (r, w)) in s.squeezing[1].iter().zip(&weights).enumerate() {
        let closed = gain * w.sqrt();
        assert!((r - closed).abs() < 1e-12, "r_{k} = {r} against {closed}");
    }
    // dB: 10 log₁₀ e^{2r}
    let r = s.squeezing[0][0];
    assert!((s.squeezing_db[0][0] - 10.0 * portable::log10(portable::exp(2.0 * r))).abs() < 1e-12);
}

/// Where the phase matching changes with the frequencies, the ordered product squeezes its first
/// mode beyond the Schmidt model and moves it, keeps the commutators, and converges at the
/// midpoint rule's second order in the slices (`quantum/broadband-mismatched-order`'s rule).
#[test]
fn a_mismatched_guide_squeezes_beyond_the_schmidt_model() {
    let run = |slices: usize| squeezed(&guide(CORRELATED), &at(3.0, slices, 41)).unwrap();
    let s = run(100);
    assert!(
        s.squeezing[0][0] > s.squeezing[1][0] + 0.5,
        "{:?}",
        s.squeezing
    );
    assert!(
        s.mean_photons[0] > 2.0 * s.mean_photons[1],
        "{:?}",
        s.mean_photons
    );
    assert!(s.modes[0].shift > 0.1, "{}", s.modes[0].shift);
    // the bound of quantum/broadband-symplectic
    assert!(s.symplectic_error < 1e-12, "{}", s.symplectic_error);
    let changes = [run(50).slice_change, s.slice_change, run(200).slice_change];
    let order =
        0.5 * (portable::log2(changes[0] / changes[1]) + portable::log2(changes[1] / changes[2]));
    assert!((order - 2.0).abs() < 0.1, "order {order} from {changes:?}");
    // each mode's |u|² sums to 1, one value per signal frequency
    for m in &s.modes {
        assert_eq!(m.sliced.len(), s.signal_axis.len());
        assert!((m.sliced.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((m.schmidt.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }
    assert!(s.notes.is_empty(), "{:?}", s.notes);
}

/// A crystal and four-wave mixing are sliced too; a Gaussian stand-in and four-wave mixing's
/// left-out phase modulation are said.
#[test]
fn a_crystal_and_four_wave_mixing_are_sliced_with_their_notes() {
    let crystal = Design {
        medium: Medium::Crystal {
            id: "linbo3".into(),
            model: None,
            axes: [
                "extraordinary".into(),
                "extraordinary".into(),
                "ordinary".into(),
            ],
            temperature_c: 25.0,
            length_mm: 10.0,
            matching: Shape::Gaussian,
            efficiency: Some(100.0),
        },
        half_nm: 8.0,
        ..guide(CORRELATED)
    };
    let s = squeezed(&crystal, &at(2.0, 40, 41)).unwrap();
    assert!(s.squeezing[0][0] > 0.0);
    assert!(
        s.notes.iter().any(|n| n.contains("Gaussian stand-in")),
        "{:?}",
        s.notes
    );
    let mut four = guide(CORRELATED);
    four.medium = Medium::Guide {
        process: PairProcess::Sfwm,
        group_index: [4.2, 4.2, 4.2],
        dispersion: [-1000.0; 3],
        length_mm: 5.0,
        matching: Shape::Sinc,
        efficiency: None,
    };
    four.pump.wavelength_um = 1.55;
    four.signal_um = Some(1.54);
    four.half_nm = 5.0;
    let s = squeezed(&four, &at(2.0, 40, 41)).unwrap();
    assert!(s.squeezing[0][0] > 0.0);
    assert!(
        s.notes.iter().any(|n| n.contains("cross-phase")),
        "{:?}",
        s.notes
    );
}

/// Too few slices, or a grid that cuts the first mode, are computed and said.
#[test]
fn a_coarse_slicing_or_a_cut_mode_says_so() {
    let s = squeezed(&guide(CORRELATED), &at(3.0, 2, 41)).unwrap();
    assert!(
        s.notes.iter().any(|n| n.contains("add slices")),
        "{:?}",
        s.notes
    );
    let mut narrow = guide(CORRELATED);
    narrow.half_nm = 1.0;
    let s = squeezed(&narrow, &at(3.0, 40, 41)).unwrap();
    assert!(
        s.notes.iter().any(|n| n.contains("widen the grid")),
        "{:?}",
        s.notes
    );
}

/// Each limit refuses with its numbers, and a ring or two pumps in two modes with why.
#[test]
fn squeezing_at_high_gain_refuses_what_it_cant_take() {
    let refused = |d: Design, g: Gain| squeezed(&d, &g).unwrap_err();
    let base = || guide(CORRELATED);
    assert!(refused(base(), at(3.0, 40, 121)).contains("11 to 101 points"));
    assert!(refused(base(), at(3.0, 41, 41)).contains("even number from 2 to 200"));
    assert!(refused(base(), at(3.0, 400, 41)).contains("even number from 2 to 200"));
    assert!(refused(base(), at(0.0, 40, 41)).contains("the gain g"));
    assert!(refused(base(), at(6.0, 40, 41)).contains("at most 5"));
    let mut d = base();
    d.pump.bandwidth_nm = 0.0;
    assert!(refused(d, at(3.0, 40, 41)).contains("the pump's bandwidth"));
    let mut ring = base();
    ring.medium = Medium::Ring {
        radius_um: 47.5,
        effective_index: 1.61,
        group_index: 1.75,
        dispersion: 0.0,
        coupling: 0.016,
        loss_db_per_cm: 0.0,
        pump_extra: 0.0,
        order: 1,
        span: 10.0,
    };
    assert!(refused(ring, at(3.0, 40, 41)).contains("a ring's pairs are its resonances'"));
    let mut two = base();
    two.medium = Medium::DualPump {
        modes: [
            ModeSpec {
                effective_index: 1.85,
                group_index: 3.85,
                dispersion: -10_900.0,
            },
            ModeSpec {
                effective_index: 1.75,
                group_index: 3.959,
                dispersion: -10_900.0,
            },
        ],
        length_mm: 11.0,
        delay_ps: 2.0,
    };
    assert!(refused(two, at(3.0, 40, 41)).contains("two pumps' overlap"));
    // the design's own grid is the low-gain view's: this one takes its own points
    let mut d = base();
    d.points = 301;
    assert!(squeezed(&d, &at(1.0, 10, 21)).is_ok());
}

/// The window calls the command by the name the program registers it under, with the limits
/// the command keeps.
#[test]
fn the_window_and_the_program_agree_on_the_command_and_its_limits() {
    let studio = include_str!("../studio.rs");
    let api = include_str!("../../../src/lib/api.ts");
    let sources = include_str!("../../../src/lib/sources.ts");
    assert!(studio.contains("crate::sources::squeezing::pair_squeezing,"));
    assert!(api.contains("(\"pair_squeezing\""));
    for line in [
        format!("export const MOST_GAIN = {MOST_GAIN};"),
        format!("export const FEWEST_SLICES = {FEWEST_SLICES};"),
        format!("export const MOST_SLICES = {MOST_SLICES};"),
        format!("export const MOST_GAIN_POINTS = {MOST_GAIN_POINTS};"),
    ] {
        assert!(sources.contains(&line), "sources.ts lacks {line}");
    }
}

#[test]
#[ignore = "prints the timings and the presets' figures; run by hand"]
fn print_figures() {
    for points in [41, 61, 81, 101] {
        for slices in [50, 100, 200] {
            let s = squeezed(&guide(CORRELATED), &at(3.0, slices, points)).unwrap();
            println!(
                "{points} points, {slices} slices: {:.3} s, r1 {:.4} against {:.4}, change {:.2e}, \
                 photons {:.1} against {:.1}, shift {:.3}, notes {:?}",
                s.seconds,
                s.squeezing[0][0],
                s.squeezing[1][0],
                s.slice_change,
                s.mean_photons[0],
                s.mean_photons[1],
                s.modes[0].shift,
                s.notes
            );
        }
    }
    // the window's starting points (g = 2, 100 slices, 61 points): the correlated guide, the
    // group-velocity matched one
    for (name, ng, half) in [
        ("correlated", CORRELATED, 15.0),
        ("matched", [2.3, 2.2468, 2.3532], 8.0),
    ] {
        let mut d = guide(ng);
        d.half_nm = half;
        for g in [0.5, 1.0, 2.0, 3.0] {
            let s = squeezed(&d, &at(g, 100, 61)).unwrap();
            println!(
                "{name}, g = {g}: dB {:.3?} against {:.3?}; photons {:.2} against {:.2}; modes \
                 {:.3} against {:.3}; shift {:.3e}; change {:.2e}; {:.3} s; {:?}",
                &s.squeezing_db[0][..3],
                &s.squeezing_db[1][..3],
                s.mean_photons[0],
                s.mean_photons[1],
                s.mode_number[0],
                s.mode_number[1],
                s.modes[0].shift,
                s.slice_change,
                s.seconds,
                s.notes
            );
        }
    }
}
