//! The dynamic thermal compact models' cases: a known sum of exponentials fitted back, Coenen et
//! al. 2022's Foster networks turned into their Cauer ladders and their responses, and the
//! error against Jacques et al.'s N++ heater's transient as poles are added.

use super::shifter_checks::table3::*;
use super::shifter_checks::{NPP, OK, finer, jacques, jacques_grading, outcome, rect, wavelength};
use super::{
    Boundary, Drive, Features, Fitted, Foster, Grid, Guide, Heat, Optics, PhaseShifter, RcFit,
    RcModel, Scheme, Section, Side, StepResponses, Stepping, Tensor,
};
use crate::validation::{Case, Tier};

/// The known model of `thermal-rc/known-sum`: three poles, 2 × 2, each residue matrix
/// symmetric positive definite.
fn known() -> RcModel {
    RcModel::new(
        vec![2e-6, 2e-5, 3e-4],
        vec![
            vec![vec![1.0, 0.1], vec![0.1, 0.8]],
            vec![vec![2.0, 0.5], vec![0.5, 2.5]],
            vec![vec![0.6, 0.4], vec![0.4, 0.7]],
        ],
    )
    .expect(OK)
}

/// The largest relative difference of `known`'s poles and residues from a passive fit of three
/// poles to its step responses at 601 times every 2 µs.
fn known_back() -> f64 {
    let m = known();
    let times: Vec<f64> = (0..=600).map(|k| f64::from(k) * 2e-6).collect();
    let steps: Vec<Vec<Vec<f64>>> = (0..2)
        .map(|j| {
            (0..2)
                .map(|i| times.iter().map(|&t| m.step(i, j, t)).collect())
                .collect()
        })
        .collect();
    let fit = RcModel::fit(&times, &steps, None, &RcFit::new(3).passive(true)).expect(OK);
    let poles = fit
        .model
        .time_constants_s()
        .iter()
        .zip(m.time_constants_s())
        .map(|(a, b)| (a / b - 1.0).abs());
    let residues = fit
        .model
        .residues()
        .iter()
        .flatten()
        .flatten()
        .zip(m.residues().iter().flatten().flatten())
        .map(|(a, b)| (a / b - 1.0).abs());
    poles.chain(residues).fold(0.0, f64::max)
}

/// Coenen et al. 2022's Table I: the ring filter's Foster networks with and without the
/// substrate undercut (UCUT), resistances in K/mW (their "Ω") and capacitances in mJ/K (their
/// "µF", which with K/mW make the time constants µs), and the Cauer ladders they convert them to.
pub(crate) mod table1 {
    pub const UCUT_FOSTER: ([f64; 4], [f64; 4]) =
        ([0.276, 2.01, 2.314, 0.198], [9.094, 12.74, 23.55, 626.3]);
    pub const UCUT_CAUER: ([f64; 4], [f64; 4]) =
        ([1.135, 3.093, 0.5344, 0.036], [4.297, 5.370, 84.96, 3274.0]);
    pub const NO_UCUT_FOSTER: ([f64; 2], [f64; 2]) = ([0.651, 0.872], [4.880, 14.92]);
    pub const NO_UCUT_CAUER: ([f64; 2], [f64; 2]) = ([1.061, 0.459], [3.686, 23.01]);
}

/// A Foster network of Table I, its capacitances made seconds' worth (µs per K/mW).
fn coenen(foster: (&[f64], &[f64])) -> Foster {
    Foster::of_rc(
        foster.0.to_vec(),
        foster.1.iter().map(|c| c * 1e-6).collect(),
    )
    .expect(OK)
}

/// The largest relative difference of `foster`'s Cauer ladder from `cauer`'s printed values.
fn ladder(foster: (&[f64], &[f64]), cauer: (&[f64], &[f64])) -> f64 {
    let c = coenen(foster).cauer().expect(OK);
    let r = c
        .resistances()
        .iter()
        .zip(cauer.0)
        .map(|(a, b)| a / b - 1.0);
    let k = c
        .capacitances()
        .iter()
        .zip(cauer.1)
        .map(|(a, b)| a * 1e6 / b - 1.0);
    r.chain(k).fold(0.0, |m, v| m.max(v.abs()))
}

/// The UCUT ring's Foster network as a model of one heater.
fn ucut() -> RcModel {
    let f = coenen((&table1::UCUT_FOSTER.0, &table1::UCUT_FOSTER.1));
    RcModel::new(
        f.time_constants_s().to_vec(),
        f.resistances().iter().map(|r| vec![vec![*r]]).collect(),
    )
    .expect(OK)
}

/// When a step response gets 1 − 1/e of the way to its steady value, seconds, to 1 ns.
fn one_over_e(m: &RcModel) -> f64 {
    let end = m.steady()[0][0] * (1.0 - (-1.0f64).exp());
    let (mut a, mut b) = (
        0.0,
        m.time_constants_s().iter().fold(0.0, |x: f64, t| x.max(*t)) * 10.0,
    );
    while b - a > 1e-9 {
        let c = 0.5 * (a + b);
        if m.step(0, 0, c) < end {
            a = c;
        } else {
            b = c;
        }
    }
    0.5 * (a + b)
}

/// Jacques et al.'s N++ heater (as `shifter/jacques-npp`, the heat graded from 20 nm, the mode
/// on 10 nm) turned on, BDF2 steps of 0.1 µs to 60 µs.
pub(crate) fn npp_steps() -> (PhaseShifter, StepResponses) {
    let s = jacques(NPP, &jacques_grading(1.0), 0.01);
    let r = s
        .step_responses(&Stepping::new(0.1e-6, 600, Scheme::Bdf2))
        .expect(OK);
    (s, r)
}

/// The models of 1 to 8 poles fitted to the N++ heater's phase.
fn npp_fits() -> (PhaseShifter, Vec<Fitted>) {
    let (s, r) = npp_steps();
    let fits = RcModel::fits(&r.times_s, &r.steps, Some(&r.steady), &RcFit::new(8)).expect(OK);
    (s, fits)
}

/// Two of Jacques et al.'s N++ shifters side by side, their ribs' centres `pitch` µm apart, in
/// the paper's region's depth (the substrate held 12.68 µm below the buried oxide, the oxide's
/// top 3.32 µm over it with 10 W/(m²·K) to the air) and 20 µm beyond each, the heat graded from
/// 20 nm by `r` (growing by 1.2 to 2 µm), the modes on 10 nm over `r`. Drives and guides: the
/// left shifter's, then the right's.
pub(crate) fn pair(pitch: f64, r: f64) -> PhaseShifter {
    let top = 3.32;
    let bottom = -14.68;
    let half = 0.5 * pitch + 20.0;
    let mut section = Section::new("SiO2");
    section.layer(bottom, -2.0, "substrate").expect(OK);
    let centres = [-0.5 * pitch, 0.5 * pitch];
    let mut strips = Vec::new();
    for &c in &centres {
        let left = rect([c - 2.05, c - 1.05], [0.0, 0.09]);
        let right = rect([c + 1.05, c + 2.05], [0.0, 0.09]);
        section
            .draw(rect([c - 1.05, c + 1.05], [0.0, 0.09]), "slab")
            .draw(rect([c - 0.25, c + 0.25], [0.0, 0.22]), "Si")
            .draw(left.clone(), "N++")
            .draw(right.clone(), "N++");
        strips.push((left, right));
    }
    let mut features = Features::new();
    features.extend(section.features());
    let grid = Grid::fitted(
        [-half, half],
        [bottom, top],
        &finer(2.0, 0.02, 1.2, r),
        &features,
    )
    .expect(OK);
    let mut heat = Heat::new(section.clone(), grid);
    for (m, k, c) in [
        ("SiO2", K_OX, C_OX),
        ("substrate", K_SUBSTRATE, C_SI),
        ("Si", K_WG, C_SI),
        ("slab", K_SLAB, C_SI),
        ("N++", K_NPP, C_SI),
    ] {
        heat.conductivity(m, Tensor::isotropic(k))
            .heat_capacity(m, c);
    }
    heat.boundary(Side::Bottom, Boundary::Fixed(0.0)).boundary(
        Side::Top,
        Boundary::Convective {
            coefficient: 10.0,
            ambient: 0.0,
        },
    );
    let mut optics = Optics::new(wavelength(), 0.01 / r).expect(OK);
    optics
        .medium("Si", N_SI, DN_DT)
        .medium("slab", N_SI, DN_DT)
        .medium("N++", N_NPP, DN_DT)
        .medium("SiO2", N_OX, 0.0);
    let guides = centres
        .iter()
        .zip(["left", "right"])
        .map(|(&c, name)| {
            Guide::new(name, &section, ([c - 1.5, c + 1.5], [-0.75, 0.97]), &optics).expect(OK)
        })
        .collect();
    let drives = strips
        .into_iter()
        .zip(["left", "right"])
        .map(|((a, b), name)| Drive::heater(name, a, 0.5).and(b, 0.5))
        .collect();
    PhaseShifter::new(heat, drives, guides).expect(OK)
}

/// The pair's step responses: BDF2 steps of 0.2 µs to 200 µs.
pub(crate) fn pair_steps(pitch: f64) -> (PhaseShifter, StepResponses) {
    let s = pair(pitch, 1.0);
    let r = s
        .step_responses(&Stepping::new(0.2e-6, 1000, Scheme::Bdf2))
        .expect(OK);
    (s, r)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "thermal-rc/known-sum",
            title: r"Thermal compact model, analytic: a 2 × 2 model of three poles (2, 20 and 300 µs), each residue matrix symmetric positive definite, its four step responses sampled every 2 µs to 1.2 ms and fitted back by a passive model of three poles; the largest relative error of the poles and residues (shown)",
            tier: Tier::Analytic,
            source: r"the model itself: $Z_{ij}(s) = \sum_k R_{k,ij}/(1 + s\tau_k)$",
            run: || outcome(known_back(), 0.0, 1e-6),
        },
        Case {
            id: "thermal-rc/coenen-cauer-ucut",
            title: r"Thermal compact model, published: Coenen et al.'s ring filter with the substrate undercut, its Foster network of Table I (four blocks) turned into a Cauer ladder by the continued fraction of $1/Z$; the largest relative difference of the ladder's four resistances and four capacitances from Table I's (shown)",
            tier: Tier::Published,
            source: r"D. Coenen et al., IEEE Trans. Compon. Packag. Manuf. Technol. 12, 1350 (2022), doi:10.1109/TCPMT.2022.3195699, Table I's Cauer column; 2% for its Foster values' three or four digits (the last ladder elements' sensitivity to them)",
            run: || {
                let (f, c) = (table1::UCUT_FOSTER, table1::UCUT_CAUER);
                outcome(ladder((&f.0, &f.1), (&c.0, &c.1)), 0.0, 0.02)
            },
        },
        Case {
            id: "thermal-rc/coenen-cauer",
            title: r"Thermal compact model, published: the same without the undercut, Table I's two-block Foster network into its Cauer ladder; the largest relative difference from Table I's (shown)",
            tier: Tier::Published,
            source: r"D. Coenen et al., doi:10.1109/TCPMT.2022.3195699, Table I; 1% for its values' digits",
            run: || {
                let (f, c) = (table1::NO_UCUT_FOSTER, table1::NO_UCUT_CAUER);
                outcome(ladder((&f.0, &f.1), (&c.0, &c.1)), 0.0, 0.01)
            },
        },
        Case {
            id: "thermal-rc/coenen-200khz",
            title: r"Thermal compact model, published: the undercut ring's Foster network's response to its heater's power at 200 kHz, against its steady response (dB, $20\log_{10}\lvert Z(f)/Z(0)\rvert$)",
            tier: Tier::Published,
            source: r"D. Coenen et al., doi:10.1109/TCPMT.2022.3195699, Section III.A: 'a drive frequency of 200 kHz corresponds with a response of -30 dB' (Fig. 13, the circuit's, the coupling's included); 2 dB for the reading",
            run: || {
                let m = ucut();
                let db = 20.0 * (m.response(0, 0, 2e5).norm() / m.steady()[0][0]).log10();
                outcome(db, -30.0, 2.0)
            },
        },
        Case {
            id: "thermal-rc/coenen-start-up",
            title: r"Thermal compact model, published: the undercut ring's Foster network turned on, its 1/e time (µs)",
            tier: Tier::Published,
            source: r"D. Coenen et al., doi:10.1109/TCPMT.2022.3195699, Section III.B, the circuit's start-up (Fig. 14): 'device-level dynamics have a time constant of 40 µs'; 4 µs (10%) for the reading and the neighbours' share",
            run: || outcome(one_over_e(&ucut()) * 1e6, 40.0, 4.0),
        },
        Case {
            id: "thermal-rc/jacques-poles",
            title: r"Thermal compact model, convergence: Jacques et al.'s N++ heater (as `shifter/jacques-npp`: the heat graded from 20 nm, the mode on 10 nm) turned on, its guide's phase every 0.1 µs to 60 µs by BDF2, fitted by models of 1 to 8 poles through its steady phase; the error (the largest over the record, relative to the steady phase) falls at each pole added, 0.15, 0.020, 5.2e-3, 4.5e-3, 4.2e-4, 1.4e-4, 6.4e-5 and 1.9e-5; the 8 poles' (shown)",
            tier: Tier::Analytic,
            source: r"the heat solver's own transient on the same grid (static's BDF2)",
            run: || {
                let (_, fits) = npp_fits();
                let falls = fits.windows(2).all(|w| w[1].error <= w[0].error);
                let last = fits.last().expect(OK).error;
                outcome(if falls { last } else { f64::NAN }, 0.0, 1e-4)
            },
        },
        Case {
            id: "thermal-rc/jacques-cutoff",
            title: r"Thermal compact model, analytic: the 8-pole model of `thermal-rc/jacques-poles`, its −3 dB frequency against the heat solver's small-signal response's, $(K + i\omega M)\hat T = \hat Q$ on the same grid (`shifter/harris-bandwidth`'s method) (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the heat equation in frequency on the same grid; 2e-3 for the transient's 0.1 µs steps",
            run: || {
                let (s, fits) = npp_fits();
                let model = fits.last().expect(OK).model.cutoff(0, 0).expect(OK);
                let exact = s.cutoff(0, 0).expect(OK);
                outcome(model / exact - 1.0, 0.0, 2e-3)
            },
        },
    ]
}
