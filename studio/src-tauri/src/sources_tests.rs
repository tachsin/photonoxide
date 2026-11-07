use std::f64::consts::{LN_2, PI, TAU};

use photonoxide::circuit::components::{AllPassRing, Dispersion};
use photonoxide::nonlinear::pairs::{gaussian_gamma, gaussian_purity};
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};

use super::*;

fn bare() -> Arm {
    Arm {
        filter: Filter::None,
        coupler: None,
        coupler_loss_db: 0.0,
        detector: 1.0,
    }
}

/// Down-conversion of a 0.775 µm pump into 1.55 µm pairs in a 10 mm guide of group indices
/// `ng` (pump, signal, idler), no dispersion, Gaussian phase matching.
fn guide(ng: [f64; 3], bandwidth_nm: f64, half_nm: f64, points: usize) -> Design {
    Design {
        medium: Medium::Guide {
            process: PairProcess::Spdc,
            group_index: ng,
            dispersion: [0.0; 3],
            length_mm: 10.0,
            matching: Shape::Gaussian,
            efficiency: Some(100.0),
        },
        pump: PumpSpec {
            wavelength_um: 0.775,
            bandwidth_nm,
        },
        signal_um: None,
        half_nm,
        points,
        mean_pairs: 0.01,
        arms: [bare(), bare()],
    }
}

/// A Gaussian pump and Gaussian phase matching in a guide whose indices are first order make
/// the joint amplitude a two-dimensional Gaussian exp(−½ zᵀMz): its purity is √(1 − c²/(ab)),
/// and its Schmidt weights fall geometrically, λ_k = (1 − μ²) μ^(2k), μ² = (1 − P)/(1 + P).
#[test]
fn a_gaussian_amplitude_has_its_closed_form_schmidt_weights() {
    let ng = [2.30, 2.20, 2.30];
    let bandwidth_nm = 1.0;
    let d = designed(&guide(ng, bandwidth_nm, 15.0, 161)).unwrap();
    // M from the pump's α = exp(−2 ln 2 (Δs + Δi)²/Δν²) and the matching exp(−γ(ΔkL/2)²),
    // ΔkL/2 = πL((n_gp − n_gs)Δs + (n_gp − n_gi)Δi)
    let dnu = bandwidth_nm * 1e-3 / (0.775f64 * 0.775);
    let p = 4.0 * LN_2 / (dnu * dnu);
    let q = 2.0 * gaussian_gamma() * (PI * 1e4).powi(2);
    let (a_, b_) = (ng[0] - ng[1], ng[0] - ng[2]);
    let (a, b, c) = (p + q * a_ * a_, p + q * b_ * b_, p + q * a_ * b_);
    let purity = gaussian_purity(a, b, c);
    assert!(
        (d.purity - purity).abs() < 1e-9,
        "{} against {purity}",
        d.purity
    );
    assert!((d.schmidt_number - 1.0 / purity).abs() < 1e-8);
    let mu2 = (1.0 - purity) / (1.0 + purity);
    for (k, w) in d.weights.iter().take(5).enumerate() {
        let closed = (1.0 - mu2) * mu2.powi(k as i32);
        assert!((w - closed).abs() < 1e-9, "λ_{k} = {w} against {closed}");
    }
    assert!(d.notes.is_empty(), "{:?}", d.notes);
    // at low gain, either arm's g⁽²⁾ is 1 + P
    assert!((d.g2 - (1.0 + purity)).abs() < 0.01, "{}", d.g2);
    assert!((d.mean_pairs - 0.01).abs() < 1e-12);
    assert!((d.pairs.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    assert_eq!(d.unit, "nm");
    assert_eq!(d.amplitude.len(), 161 * 161);
}

/// Group-velocity matched (the pump's group index between the photons', the two terms of M's
/// corner cancelling), the pairs are separable: one Schmidt mode.
#[test]
fn group_velocity_matching_makes_the_pairs_pure() {
    // c = p + q(n_gp − n_gs)(n_gp − n_gi) = 0 for the pump's bandwidth
    let bandwidth_nm = 1.0;
    let dnu = bandwidth_nm * 1e-3 / (0.775f64 * 0.775);
    let p = 4.0 * LN_2 / (dnu * dnu);
    let q = 2.0 * gaussian_gamma() * (PI * 1e4).powi(2);
    let x = (p / q).sqrt();
    let ng = [2.3, 2.3 - x, 2.3 + x];
    let d = designed(&guide(ng, bandwidth_nm, 8.0, 161)).unwrap();
    assert!((d.purity - 1.0).abs() < 1e-9, "{}", d.purity);
    assert!(d.modes == 1, "{} modes", d.modes);
    // a single mode: thermal arms, g⁽²⁾ = 2
    assert!((d.g2 - 2.0).abs() < 1e-6, "{}", d.g2);
}

/// Down-conversion's brightness is the nonlinear job's: κ²(ν_sν_i/ν_p²)L², ¼ κ²L² at
/// degeneracy, per Hz and per W.
#[test]
fn the_brightness_is_the_libraries_pair_rate() {
    let d = designed(&guide([2.3, 2.2, 2.3], 1.0, 15.0, 101)).unwrap();
    let b = d.brightness.expect("η is given");
    let closed = 100.0 * 1e-10 / 4.0 * 1e4 * 1e4 * 1e6;
    assert!(
        (b.spectral - closed).abs() < 1e-12 * closed,
        "{} against {closed}",
        b.spectral
    );
    assert!(b.rate > 0.0);
    // without η, the view says why there is none
    let mut g = guide([2.3, 2.2, 2.3], 1.0, 15.0, 101);
    if let Medium::Guide { efficiency, .. } = &mut g.medium {
        *efficiency = None;
    }
    let d = designed(&g).unwrap();
    assert!(d.brightness.is_none() && d.brightness_note.unwrap().contains("η"));
}

/// Through a lossless coupler's cross port, a photon arrives with its share; its partner's
/// heralding is untouched, and a detector multiplies its own photon's.
#[test]
fn the_heralding_follows_the_arms_s_matrices() {
    let mut g = guide([2.3, 2.2, 2.3], 1.0, 15.0, 101);
    g.arms[0].coupler = Some(0.3);
    g.arms[0].detector = 0.8;
    let d = designed(&g).unwrap();
    let h = &d.heralding;
    assert!((h.efficiency[0] - 0.3).abs() < 1e-12, "{:?}", h.efficiency);
    assert!((h.efficiency[1] - 1.0).abs() < 1e-12);
    assert!((h.detected[0] - 0.24).abs() < 1e-12);
    assert!((h.purity - d.purity).abs() < 1e-9);
    // a ring filter far narrower than the signal cuts pairs whose idler was counted, every
    // signal through it still finds its idler, and the heralded photons are purer
    g.arms[0] = bare();
    g.arms[0].filter = Filter::Ring {
        radius_um: 20.0,
        group_index: 4.2,
        coupling: 0.05,
        loss_db_per_cm: 0.0,
    };
    let d = designed(&g).unwrap();
    let h = &d.heralding;
    assert!(h.efficiency[0] < 0.5, "{:?}", h.efficiency);
    assert!((h.efficiency[1] - 1.0).abs() < 1e-12, "{:?}", h.efficiency);
    assert!(
        h.purity > d.purity + 0.05,
        "{} against {}",
        h.purity,
        d.purity
    );
    // on resonance at the signal's centre, the lossless filter drops everything there
    assert!((h.centre[0] - 1.0).abs() < 1e-9, "{:?}", h.centre);
    assert!(h.facts.iter().any(|f| f.label.contains("filter's width")));
}

/// Hertz to 1/µm.
fn per_um(hz: f64) -> f64 {
    hz / (SPEED_OF_LIGHT * 1e6)
}

/// Helt et al.'s ring (R = 47.5 µm, v = 171 µm/ps, 1.48 GHz resonances), pumped by a
/// transform-limited pulse of `pulse` ps, as the published ring cases take it.
fn helt(pulse: f64) -> Design {
    let ng = SPEED_OF_LIGHT * 1e-6 / 171.0;
    let length = TAU * 47.5;
    let n_eff = 310.0 * 1.55 / length;
    // the coupling whose resonance at 1.55 µm is 1.48 GHz wide, by bisection on the ring's own
    let ring = AllPassRing::new(Dispersion::new(Wavelength::um(1.55).unwrap(), n_eff, ng)).unwrap();
    let target = per_um(1.48e9) * 1.55 * 1.55;
    let (mut lo, mut hi): (f64, f64) = (1e-6, 0.5);
    for _ in 0..100 {
        let mid = (lo * hi).sqrt();
        if ring
            .fwhm(Wavelength::um(1.55).unwrap(), &[length, mid])
            .unwrap()
            < target
        {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let bandwidth = per_um(4.0 * LN_2 / (TAU * pulse * 1e-12));
    Design {
        medium: Medium::Ring {
            radius_um: 47.5,
            effective_index: n_eff,
            group_index: ng,
            dispersion: 0.0,
            coupling: (lo * hi).sqrt(),
            loss_db_per_cm: 0.0,
            pump_extra: 0.0,
            order: 1,
            span: 10.0,
        },
        pump: PumpSpec {
            wavelength_um: 1.55,
            bandwidth_nm: bandwidth * 1.55 * 1.55 * 1e3,
        },
        signal_um: None,
        half_nm: 1.0,
        points: 301,
        mean_pairs: 0.01,
        arms: [bare(), bare()],
    }
}

/// Q2's published ring cases: Helt et al.'s 5 ps pump, K = 1.09 (quantum/ring-helt-short), and
/// Vernon et al.'s equal linewidths under a broad pump, K = 1.09 (quantum/ring-vernon-equal).
#[test]
fn a_rings_pairs_have_the_published_schmidt_numbers() {
    for pulse in [5.0, 0.05] {
        let d = designed(&helt(pulse)).unwrap();
        assert!(
            (d.schmidt_number - 1.09).abs() < 0.005,
            "{pulse} ps: K = {}",
            d.schmidt_number
        );
        assert_eq!(d.unit, "GHz");
        let width = d.facts.iter().find(|f| f.label == "linewidth").unwrap();
        assert!((width.value - 1.48).abs() < 1e-3, "{}", width.value);
        assert!(d.notes.is_empty(), "{:?}", d.notes);
    }
    // a pump resonance widened by a second channel makes the pairs purer (Vernon et al.)
    let mut wide = helt(0.05);
    if let Medium::Ring { pump_extra, .. } = &mut wide.medium {
        *pump_extra = 0.1;
    }
    let d = designed(&wide).unwrap();
    assert!(d.purity > 1.0 / 1.09 + 0.02, "{}", d.purity);
}

/// Two pumps in two modes of a guide: the delayed, faster pump catches up inside the guide, and
/// the pairs are purer than with the pumps together.
#[test]
fn a_delayed_pump_in_a_second_mode_purifies_the_pairs() {
    let at = |delay_ps: f64| Design {
        medium: Medium::DualPump {
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
            delay_ps,
        },
        pump: PumpSpec {
            wavelength_um: 1.55,
            bandwidth_nm: 4.5,
        },
        signal_um: Some(1.5842),
        half_nm: 15.0,
        points: 81,
        mean_pairs: 0.01,
        arms: [bare(), bare()],
    };
    let together = designed(&at(0.0)).unwrap();
    let delay = 5500.0 * 0.109 / (SPEED_OF_LIGHT * 1e-6);
    let apart = designed(&at(delay)).unwrap();
    assert!(
        apart.purity > together.purity + 0.1,
        "{} against {}",
        apart.purity,
        together.purity
    );
    let centre = apart
        .facts
        .iter()
        .find(|f| f.label.starts_with("pumps' overlap"))
        .unwrap();
    assert!((centre.value - 5.5).abs() < 1e-9, "{}", centre.value);
}

/// A crystal of the catalogue: the grating designed for the centres, its period stated.
#[test]
fn a_crystal_is_poled_for_its_centres() {
    let d = Design {
        medium: Medium::Crystal {
            id: "linbo3".into(),
            model: None,
            axes: [
                "extraordinary".into(),
                "extraordinary".into(),
                "extraordinary".into(),
            ],
            temperature_c: 25.0,
            length_mm: 10.0,
            matching: Shape::Sinc,
            efficiency: None,
        },
        ..guide([2.3, 2.2, 2.3], 0.5, 30.0, 101)
    };
    let d = designed(&d).unwrap();
    let period = d.facts.iter().find(|f| f.label == "poling period").unwrap();
    // π/|Δk₀| twice: the first-order period is twice the coherence length
    let coherence = d
        .facts
        .iter()
        .find(|f| f.label == "coherence length")
        .unwrap();
    assert!((period.value - 2.0 * coherence.value).abs() < 1e-9 * period.value);
    assert!(d.purity > 0.0 && d.purity < 1.0);
}

/// Each limit refuses with its numbers.
#[test]
fn the_designer_refuses_what_it_cant_take() {
    let base = || guide([2.3, 2.2, 2.3], 1.0, 15.0, 101);
    let refused = |d: Design| designed(&d).unwrap_err();
    let mut d = base();
    d.points = 5;
    assert!(refused(d).contains("11 to 301 points"));
    let mut d = base();
    d.mean_pairs = 2.0;
    assert!(refused(d).contains("at most 1"));
    let mut d = base();
    d.pump.bandwidth_nm = 0.0;
    assert!(refused(d).contains("the pump's bandwidth"));
    let mut d = base();
    d.half_nm = 400.0;
    assert!(refused(d).contains("at most"));
    let mut d = base();
    d.signal_um = Some(0.7);
    assert!(refused(d).contains("longer than the pump"));
    let mut d = base();
    if let Medium::Guide { process, .. } = &mut d.medium {
        *process = PairProcess::Sfwm;
    }
    d.pump.wavelength_um = 1.55;
    assert!(refused(d).contains("give the signal's wavelength"));
    let mut d = helt(5.0);
    if let Medium::Ring { coupling, .. } = &mut d.medium {
        *coupling = 1.0;
    }
    assert!(refused(d).contains("between 0 and 1"));
    let mut d = base();
    d.medium = Medium::Crystal {
        id: "linbo3".into(),
        model: None,
        axes: ["sideways".into(), "e".into(), "e".into()],
        temperature_c: 25.0,
        length_mm: 10.0,
        matching: Shape::Sinc,
        efficiency: None,
    };
    assert!(refused(d).contains("no axis \"sideways\""));
    let mut d = base();
    d.arms[1].detector = 1.5;
    assert!(refused(d).contains("the idler's detector's efficiency"));
}

/// A grid that cuts the spectrum or doesn't resolve it is computed, and the view says so.
#[test]
fn a_grid_that_cuts_the_spectrum_says_so() {
    let d = designed(&guide([2.3, 2.2, 2.3], 1.0, 2.0, 101)).unwrap();
    assert!(
        d.notes.iter().any(|n| n.contains("grid's edge")),
        "{:?}",
        d.notes
    );
    let d = designed(&guide([2.3, 2.2, 2.3], 1.0, 200.0, 21)).unwrap();
    assert!(
        d.notes.iter().any(|n| n.contains("add points")),
        "{:?}",
        d.notes
    );
}

/// The window calls the command by the name the program registers it under.
#[test]
fn the_window_and_the_program_name_the_command_alike() {
    let studio = include_str!("studio.rs");
    let api = include_str!("../../src/lib/api.ts");
    assert!(studio.contains("crate::sources::pair_source,"));
    assert!(api.contains("(\"pair_source\""));
}
