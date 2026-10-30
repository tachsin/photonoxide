//! The phase-change phase shifter devices' validation cases ([`super::PcmShifter`]): the
//! analytic fidelity against the 2D one over its stated range, and in the limit of a wide guide
//! where the effective index method is exact, the 2D model's grid, the film's states as the
//! catalogue's and Frantz et al.'s effective medium, Delaney et al.'s figure of merit, and every
//! device as a job to the bit.

use std::sync::OnceLock;

use num_complex::Complex64 as c64;

use crate::circuit::Fidelity;
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

use super::{Device, PcmShifter, Spec, Values};

const OK: &str = "a valid design";

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The three devices, at their defaults.
fn devices() -> [PcmShifter; 3] {
    [PcmShifter::sb2se3(), PcmShifter::sb2s3(), PcmShifter::gst()]
}

/// The designs the analytic model is checked on: the defaults (Delaney et al.'s 25 nm film on a
/// 500 × 220 nm strip), and each parameter of its validity at its ends.
const DESIGNS: [&[(&str, f64)]; 6] = [
    &[],
    &[("width", 0.4)],
    &[("width", 0.6)],
    &[("film_thickness", 0.01)],
    &[("film_thickness", 0.05)],
    &[("slab", 0.1)],
];

/// The index change from the amorphous to the crystalline state (Re n_eff) and the crystalline
/// state's loss (dB/µm) of `d` at `v`, at `fidelity`.
fn change_and_loss(d: &PcmShifter, v: &Values, fidelity: Fidelity) -> [f64; 2] {
    let (s, _) = d.states(v, fidelity).expect(OK);
    let k0 = std::f64::consts::TAU / v.get("wavelength").expect(OK);
    [
        s.crystalline.re - s.amorphous.re,
        super::pcm::loss(s.crystalline, k0),
    ]
}

/// Each device's analytic and 2D figures over [`DESIGNS`]: [Δn_eff,ac, α_c] of the analytic
/// model, then of the 2D one, per design.
fn table() -> &'static Vec<Vec<[[f64; 2]; 2]>> {
    static CELL: OnceLock<Vec<Vec<[[f64; 2]; 2]>>> = OnceLock::new();
    CELL.get_or_init(|| {
        devices()
            .iter()
            .map(|d| {
                DESIGNS
                    .iter()
                    .map(|changes| {
                        let v = Values::new(d, changes).expect(OK);
                        [
                            change_and_loss(d, &v, Fidelity::Analytic),
                            change_and_loss(d, &v, Fidelity::TwoD),
                        ]
                    })
                    .collect()
            })
            .collect()
    })
}

/// The analytic figure over the 2D one, minus 1, farthest from 0: the index change over every
/// device and design, or the loss of the devices that absorb (Sb₂Se₃ and GST).
fn analytic_against_2d(quantity: usize) -> f64 {
    let mut worst = 0.0f64;
    for (k, device) in table().iter().enumerate() {
        if quantity == 1 && k == 1 {
            // Sb₂S₃ absorbs nothing in either state
            continue;
        }
        for [a, b] in device {
            let e = a[quantity] / b[quantity] - 1.0;
            if e.abs() > worst.abs() {
                worst = e;
            }
        }
    }
    worst
}

/// The wide guide: 2 µm, where the effective index method's separation of the field is nearly
/// exact.
const WIDE: &[(&str, f64)] = &[("width", 2.0)];

/// The analytic index change over the 2D one on cells half the size, minus 1, farthest from 0,
/// over the three devices at the wide guide; and the grid study's estimate of the 2D one's error
/// (the finer cells' share of the change from the device's own, a third of it at second order),
/// noted for the report.
fn wide_guide() -> f64 {
    let (mut worst, mut grid) = (0.0f64, 0.0f64);
    for d in devices() {
        let v = Values::new(&d, WIDE).expect(OK);
        let analytic = change_and_loss(&d, &v, Fidelity::Analytic)[0];
        let coarse = change_and_loss(&d, &v, Fidelity::TwoD)[0];
        let fine = change_and_loss(
            &d.clone().with_refinement(2.0).expect(OK),
            &v,
            Fidelity::TwoD,
        )[0];
        let e = analytic / fine - 1.0;
        if e.abs() > worst.abs() {
            worst = e;
        }
        grid = grid.max((coarse / fine - 1.0).abs() / 3.0);
    }
    crate::validation::note_numerical(grid);
    worst
}

/// The 2D model's Δn_eff,ac and α_c on its own cells and on cells half the size, at the defaults
/// and with the thinnest film (10 nm, two cells of the film's two thirds of the way): the
/// relative difference of the coarser from the finer, farthest from 0. The lossless Sb₂S₃'s loss
/// is left out.
fn grid_error() -> f64 {
    static CELL: OnceLock<f64> = OnceLock::new();
    *CELL.get_or_init(|| {
        let mut worst = 0.0f64;
        for (k, d) in devices().iter().enumerate() {
            for design in [0, 3] {
                let v = Values::new(d, DESIGNS[design]).expect(OK);
                let coarse = table()[k][design][1];
                let fine = change_and_loss(
                    &d.clone().with_refinement(2.0).expect(OK),
                    &v,
                    Fidelity::TwoD,
                );
                for q in 0..2 {
                    if q == 1 && k == 1 {
                        continue;
                    }
                    let e = coarse[q] / fine[q] - 1.0;
                    if e.abs() > worst.abs() {
                        worst = e;
                    }
                }
            }
        }
        worst
    })
}

/// The film's permittivity at the fractions 0.25, 0.5 and 0.75 of each device's material, from
/// the device's stack, against Frantz et al.'s linear mix of the two states': the relative
/// difference, the largest.
fn film_mix() -> f64 {
    use crate::material::catalogue::State;
    let mut worst = 0.0f64;
    for d in devices() {
        let lambda = 1.55;
        let wl = Wavelength::um(lambda).expect(OK);
        let (a, c) = (
            d.state(State::Amorphous)
                .expect(OK)
                .permittivity(wl)
                .expect(OK),
            d.state(State::Crystalline)
                .expect(OK)
                .permittivity(wl)
                .expect(OK),
        );
        for f in [0.25, 0.5, 0.75] {
            let v = Values::new(&d, &[("crystallized", f)]).expect(OK);
            let stack = d.stack(&v).expect(OK);
            let (film, _, _) = stack.layer("PCM").expect(OK);
            let got = film.material.permittivity(wl).expect(OK);
            let want = (1.0 - f) * a + f * c;
            worst = worst.max((got - want).norm() / want.norm());
        }
    }
    worst
}

/// The device's figure of merit against Delaney et al.'s Eq. (1) from its own index change and
/// the loss of its lossier state, the larger relative difference over the devices that absorb.
fn figure_of_merit() -> f64 {
    let mut worst = 0.0f64;
    for d in [PcmShifter::sb2se3(), PcmShifter::gst()] {
        let b = d
            .component(&Values::defaults(&d), Fidelity::Analytic)
            .expect(OK);
        let value = |s: &str| b.figure(s).expect(OK).value;
        let wl = Wavelength::um(1.55).expect(OK);
        let want = crate::material::catalogue::figure_of_merit(
            value("dn_eff_ac").abs(),
            value("alpha_a").max(value("alpha_c")),
            wl,
        );
        worst = worst.max((value("FOM") - want).abs() / want);
    }
    worst
}

/// Each device at each fidelity as a job ([`crate::job::from_device`]), run in memory: the numbers
/// of its record (figures and S-parameters at five wavelengths) whose bits differ from
/// [`Device::component`]'s.
fn jobs() -> f64 {
    use crate::job::{Compute, Event, execute, from_device};
    use crate::run::{Run, Stop};
    let mut differ = 0usize;
    for d in devices() {
        for f in [Fidelity::Analytic, Fidelity::TwoD] {
            let mut spec = Spec::new(d.kind());
            spec.fidelity = f;
            spec.parameters = vec![
                ("crystallized".into(), "0.4".into()),
                ("length".into(), "12".into()),
            ];
            let compute = Compute {
                band: Some((1.53, 1.57, 5)),
                ..Compute::model()
            };
            let job = from_device("shifter", &spec, &compute).expect(OK);
            let mut run = Run::in_memory();
            execute(&job, &mut run, &Stop::new(None)).expect(OK);
            let events: Vec<Event> = run
                .lines()
                .iter()
                .map(|l| serde_json::from_str(l).expect("a recorded event"))
                .collect();
            let (_, design) = spec.build().expect(OK);
            let built = d.component(design.values(), f).expect(OK);
            for e in &events {
                match e {
                    Event::DeviceModel { figures, .. } => {
                        differ += figures.len().abs_diff(built.figures.len());
                        for (got, want) in figures.iter().zip(&built.figures) {
                            differ += usize::from(got.value.to_bits() != want.value.to_bits());
                        }
                    }
                    Event::SParameters {
                        wavelength_um, s, ..
                    } => {
                        let want = built
                            .s_matrix(Wavelength::um(*wavelength_um).expect(OK))
                            .expect(OK);
                        for (q, row) in s.iter().enumerate() {
                            for (p, v) in row.iter().enumerate() {
                                let x: c64 = want[(q, p)];
                                differ += usize::from(v[0].to_bits() != x.re.to_bits());
                                differ += usize::from(v[1].to_bits() != x.im.to_bits());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    differ as f64
}

/// Delaney et al.'s 25 nm film following the guide: on its top, its walls and the oxide beside
/// a strip.
const CONFORMAL: &[(&str, f64)] = &[("wall_film", 0.025), ("slab_film", 0.025)];

/// The films that follow the guide the cases draw: [`CONFORMAL`], the same over a rib's 100 nm
/// slab, the film on the top and the slab alone (as Delaney et al.'s Fig. 7d–f draws it), and on
/// the top and the walls alone.
const FOLLOWING: [&[(&str, f64)]; 4] = [
    CONFORMAL,
    &[("slab", 0.1), ("wall_film", 0.025), ("slab_film", 0.025)],
    &[("slab", 0.1), ("slab_film", 0.025)],
    &[("wall_film", 0.025)],
];

/// The film drawn to follow a guide of no step against the stack's film layer drawn across the
/// window, for each material, crystalline, in either polarization: the cells and the n_eff whose
/// bits differ (count).
fn step_limit() -> f64 {
    let mut differ = 0usize;
    for d in devices() {
        for tm in [0.0, 1.0] {
            let mut changes = CONFORMAL.to_vec();
            changes.push(("tm", tm));
            let v = Values::new(&d, &changes).expect(OK);
            let [(a, na), (b, nb)] = d.zero_step(&v, 1.0).expect(OK);
            differ += a
                .cells()
                .iter()
                .zip(b.cells())
                .filter(|(p, q)| p != q)
                .count();
            differ += usize::from(na.re.to_bits() != nb.re.to_bits());
            differ += usize::from(na.im.to_bits() != nb.im.to_bits());
        }
    }
    differ as f64
}

/// The film's area in the 2D model's cross-section (the film's cells, both halves) against the
/// closed form of the rectangles it is drawn of, 2 [t_f (w/2 + t_w) + t_w h + t_s (X − w/2) −
/// t_w t_s] with h the rib's step and X the window's half-width, over [`FOLLOWING`]: the largest
/// relative difference.
fn film_area() -> f64 {
    use crate::material::catalogue::State;
    let wl = Wavelength::um(1.55).expect(OK);
    let mut worst = 0.0f64;
    for d in devices() {
        let eps = d
            .state(State::Crystalline)
            .expect(OK)
            .permittivity(wl)
            .expect(OK);
        let film = crate::mode::vector::Permittivity::isotropic(eps);
        for design in FOLLOWING {
            let v = Values::new(&d, design).expect(OK);
            let cs = d.cross_section(&v, 1.0).expect(OK);
            let (x, y) = (cs.x(), cs.y());
            let ny = y.len() - 1;
            let mut area = 0.0;
            for i in 0..x.len() - 1 {
                for j in 0..ny {
                    if cs.cells()[i * ny + j] == film {
                        area += (x[i + 1] - x[i]) * (y[j + 1] - y[j]);
                    }
                }
            }
            let at = |name: &str| v.get(name).expect(OK);
            let (w, tf, tw, ts) = (
                at("width"),
                at("film_thickness"),
                at("wall_film"),
                at("slab_film"),
            );
            let step = at("thickness") - at("slab");
            let half = x[x.len() - 1];
            let want = 2.0 * (tf * (0.5 * w + tw) + tw * step + ts * (half - 0.5 * w) - tw * ts);
            worst = worst.max((2.0 * area - want).abs() / want);
        }
    }
    worst
}

/// The designs of the 2D model's grid study for a film that follows the guide: [`CONFORMAL`] in
/// TE and TM, and over a rib's 100 nm slab in TE (the rib's TM-like mode leaks into the slab,
/// and so does every mode under crystalline GST over the slab: neither is drawn).
fn following_designs(d: &PcmShifter) -> Vec<Vec<(&'static str, f64)>> {
    let mut designs = vec![CONFORMAL.to_vec(), [CONFORMAL, &[("tm", 1.0)]].concat()];
    if d.kind() != PcmShifter::gst().kind() {
        designs.push(FOLLOWING[1].to_vec());
    }
    designs
}

/// The 2D model's Δn_eff,ac and α_c for a film that follows the guide ([`following_designs`])
/// on its own cells and on cells half the size: the relative difference of the coarser from the
/// finer, farthest from 0 (the lossless Sb₂S₃'s loss left out).
fn following_grid() -> f64 {
    let mut worst = 0.0f64;
    for (k, d) in devices().iter().enumerate() {
        for design in following_designs(d) {
            let v = Values::new(d, &design).expect(OK);
            let coarse = change_and_loss(d, &v, Fidelity::TwoD);
            let fine = change_and_loss(
                &d.clone().with_refinement(2.0).expect(OK),
                &v,
                Fidelity::TwoD,
            );
            for q in 0..2 {
                if q == 1 && k == 1 {
                    continue;
                }
                let e = coarse[q] / fine[q] - 1.0;
                if e.abs() > worst.abs() {
                    worst = e;
                }
            }
        }
    }
    worst
}

/// The wide guide (2 µm) in the TM-like mode with the film on its top, and in the TE-like mode
/// with [`CONFORMAL`]: the analytic Δn_eff,ac over the 2D one on cells half the size, minus 1,
/// farthest from 0, over the three devices; the grid study's estimate of the 2D one's error
/// noted, as [`wide_guide`] does.
fn wide_guide_tm_conformal() -> f64 {
    let (mut worst, mut grid) = (0.0f64, 0.0f64);
    for d in devices() {
        for design in [
            vec![("width", 2.0), ("tm", 1.0)],
            [&[("width", 2.0)], CONFORMAL].concat(),
        ] {
            let v = Values::new(&d, &design).expect(OK);
            let analytic = change_and_loss(&d, &v, Fidelity::Analytic)[0];
            let coarse = change_and_loss(&d, &v, Fidelity::TwoD)[0];
            let fine = change_and_loss(
                &d.clone().with_refinement(2.0).expect(OK),
                &v,
                Fidelity::TwoD,
            )[0];
            let e = analytic / fine - 1.0;
            if e.abs() > worst.abs() {
                worst = e;
            }
            grid = grid.max((coarse / fine - 1.0).abs() / 3.0);
        }
    }
    crate::validation::note_numerical(grid);
    worst
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/pcm-analytic-index",
            title: r"Phase-change phase shifters (a 25 nm film of Sb₂Se₃, Sb₂S₃ or GST on a 500 × 220 nm strip), analytic against 2D: the change of effective index from the amorphous to the crystalline state, $\Delta n_\mathrm{eff}$, of the effective index method with exact complex slabs against the full-vector solver's (cells of 5 nm in and near the guide and the film), at six designs for each device: the defaults, the guide 0.4 and 0.6 µm wide, the film 10 and 50 nm thick, and a 100 nm slab beside the guide; the ratio farthest from 1, analytic over 2D (shown), the analytic fidelity's stated error",
            tier: Tier::Analytic,
            source: r"the effective index method (G. B. Hocker, W. K. Burns, doi:10.1364/AO.16.000113; L. Chrostowski, M. Hochberg, Silicon Photonics Design, Section 3.2.5) with each slab's complex mode from the field-transfer matrix (J. Chilwell, I. Hodgkinson, doi:10.1364/JOSAA.1.000742): the field taken as a product of its vertical and lateral profiles",
            run: || outcome(1.0 + analytic_against_2d(0), 1.0, 0.15),
        },
        Case {
            id: "device/pcm-analytic-loss",
            title: r"Phase-change phase shifters, analytic against 2D: the loss of the crystalline state, $\alpha_\mathrm{c}$ (the imaginary part of the effective index), of Sb₂Se₃ (2.1e-4 in $k$) and GST (1.03), at the same six designs each; the ratio farthest from 1, analytic over 2D (shown). Sb₂S₃ absorbs nothing in either state of the fits",
            tier: Tier::Analytic,
            source: r"the same effective index method: the lossy film's complex index followed in the slabs' dispersion relations as its absorption is switched on in 50 steps from the lossless guide",
            run: || outcome(1.0 + analytic_against_2d(1), 1.0, 0.15),
        },
        Case {
            id: "device/pcm-wide-guide",
            title: r"Phase-change phase shifters, the wide guide: the same devices on a guide 2 µm wide, where the effective index method's separation of the field is nearly exact, $\Delta n_\mathrm{eff}$ of the analytic fidelity against the 2D one's on cells half the size of the device's; the ratio farthest from 1, analytic over 2D (shown)",
            tier: Tier::Analytic,
            source: r"the effective index method, exact as the lateral profile separates from the vertical (Chrostowski and Hochberg: 1.2 % in n_eff for the 500 × 220 nm strip, Fig. 3.14); at 2 µm the two models agree to well under that",
            run: || outcome(1.0 + wide_guide(), 1.0, 1e-2),
        },
        Case {
            id: "device/pcm-2d-grid",
            title: r"Phase-change phase shifters, the 2D model's grid: $\Delta n_\mathrm{eff}$ and $\alpha_\mathrm{c}$ of the three devices on the device's cells (5 nm in and near the guide and the film, growing by 0.1 per µm to 0.1 µm) against cells half the size, at the defaults and with the film 10 nm thick; the ratio farthest from 1 (shown), the devices' 2D figures' grid error",
            tier: Tier::Analytic,
            source: r"the full-vector solver's second order in the cells (thermo/slab-vector-order): halving them moves either by less than 2 %",
            run: || outcome(1.0 + grid_error(), 1.0, 2e-2),
        },
        Case {
            id: "device/pcm-frantz-mix",
            title: r"Phase-change phase shifters, a film part-way crystallized: the permittivity of the film in the device's stack at 25, 50 and 75 % crystallized, for each material, against Frantz et al.'s linear effective medium of the two states' permittivities, $\varepsilon = (1-f)\thinspace \varepsilon_\mathrm{a} + f\thinspace \varepsilon_\mathrm{c}$ (largest relative difference shown)",
            tier: Tier::Published,
            source: r"J. A. Frantz et al., Opt. Mater. Express 13, 3631 (2023), doi:10.1364/OME.506019, Section 4.2: the intermediate states as the linear effective medium of the two permittivities",
            run: || outcome(film_mix(), 0.0, 1e-12),
        },
        Case {
            id: "device/pcm-fom",
            title: r"Phase-change phase shifters, the figure of merit: $2\pi\thinspace \lvert\Delta n_\mathrm{eff}\rvert/(\lambda\alpha)$ of the device (Sb₂Se₃ and GST, analytic) against Delaney et al.'s Eq. (1) evaluated from the device's own index change and the loss of its lossier state (the larger relative difference shown)",
            tier: Tier::Published,
            source: r"M. Delaney et al., Adv. Funct. Mater. 30, 2002447 (2020), doi:10.1002/adfm.202002447, Eq. (1)",
            run: || outcome(figure_of_merit(), 0.0, 1e-12),
        },
        Case {
            id: "device/pcm-jobs",
            title: r"Phase-change phase shifters as jobs: the three devices at each fidelity (analytic and 2D, 40 % crystallized, 12 µm long) as the job `job::from_device` makes, run, its figures and S-parameters from 1.53 to 1.57 µm (5 wavelengths) against `Device::component`'s: the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same computation: a spectrum job records what the device's component gives at its design's fidelity, to the bit",
            run: || outcome(jobs(), 0.0, 0.0),
        },
        Case {
            id: "device/pcm-conformal-step-limit",
            title: r"Phase-change phase shifters, the film following the guide (25 nm on its top, its walls and beside it) in the limit of a guide of no step, its core as wide as the window: the cross-section the conformal construction draws (the film on the top, then on the walls and over the slab) against the stack's film layer drawn across the window, on the same nodes, for each material crystalline, in the TE-like and the TM-like mode; the cells and the full-vector $n_\mathrm{eff}$ whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same layered film: at no step the walls vanish and the film over the slab meets the film on the top, the layer the stack draws, to the bit",
            run: || outcome(step_limit(), 0.0, 0.0),
        },
        Case {
            id: "device/pcm-conformal-area",
            title: r"Phase-change phase shifters, the film following the guide: the area of the film's cells in the 2D model's cross-section (both halves), for a 25 nm film on the top, the walls and beside a strip, the same over a rib's 100 nm slab, on the top and the slab alone, and on the top and the walls alone, against the closed form of its rectangles, $2\thinspace [t_f (w/2 + t_w) + t_w h + t_s (X - w/2) - t_w t_s]$ ($h$ the rib's step, $X$ the window's half-width; the corners square); the largest relative difference (shown)",
            tier: Tier::Analytic,
            source: r"the film's thickness measured normal to the face it lies on: $t_f$ on the top, $t_w$ on the walls, $t_s$ over the slab, as M. Delaney et al., Adv. Funct. Mater. 30, 2002447 (2020), doi:10.1002/adfm.202002447, Fig. 7d–f draws a 25 nm film over a rib and its slab",
            run: || outcome(film_area(), 0.0, 1e-12),
        },
        Case {
            id: "device/pcm-conformal-grid",
            title: r"Phase-change phase shifters, the 2D model's grid for a film following the guide: $\Delta n_\mathrm{eff}$ and $\alpha_\mathrm{c}$ of the three devices with a 25 nm film on the top, the walls and beside a 500 × 220 nm strip, in the TE-like and the TM-like mode, and over a rib's 100 nm slab in the TE-like mode (Sb₂Se₃ and Sb₂S₃: under crystalline GST over the slab the mode leaks into it), on the device's cells against cells half the size; the ratio farthest from 1 (shown)",
            tier: Tier::Analytic,
            source: r"the full-vector solver's cells (thermo/slab-vector-order); halving them moves the figures by less than 2 %, as device/pcm-2d-grid finds for the film on the top",
            run: || outcome(1.0 + following_grid(), 1.0, 2e-2),
        },
        Case {
            id: "device/pcm-wide-guide-tm-conformal",
            title: r"Phase-change phase shifters, the wide guide in the TM-like mode and with the film following it: the three devices on a guide 2 µm wide, the 25 nm film on its top in the TM-like mode, and on its top, walls and beside it in the TE-like mode; $\Delta n_\mathrm{eff}$ of the analytic fidelity (the vertical columns in TM and the lateral stack in TE for the TM-like mode; the column through the film on the wall beside the guide's) against the 2D one's on cells half the size of the device's; the ratio farthest from 1, analytic over 2D (shown)",
            tier: Tier::Analytic,
            source: r"the effective index method, exact as the lateral profile separates from the vertical (Chrostowski and Hochberg: 1.2 % in n_eff for the 500 × 220 nm strip, Fig. 3.14); at 2 µm the two models agree to well under that",
            run: || outcome(1.0 + wide_guide_tm_conformal(), 1.0, 1e-2),
        },
    ]
}
