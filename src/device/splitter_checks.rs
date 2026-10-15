//! What the validation report checks the rib, the MMIs, the Y-branch and the MZIs on: their
//! closed forms (Soldano and Pennings's self-imaging lengths, the MZI's transfer function, the
//! wide rib's slab), the library's components they replace to the bit, each lower fidelity
//! against the next one up over its stated range, the FDFD fidelities in their grids, and
//! published results through them (Hadley's standard rib).

use std::sync::OnceLock;

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::components::{Mmi, MmiKind, Planar, YBranch, mzi_closed_form};
use crate::generator::SelfImaging;
use crate::material::{self, Material, Model, Provenance};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::parallel::map_in_order;
use crate::stack::Layer;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid device";

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).expect("a valid wavelength")
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// A difference that should be at most `tolerance`.
fn small(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

/// The new devices' validation cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/mmi-library",
            title: r"Devices: the 1 × 2 and 2 × 2 MMIs' 2D models (guided modes) and the Y-branch's analytic one at their defaults against the studio library's components as they were built before (a 220 nm silicon film in oxide, 6 µm wide with 1.5 µm access guides; the ideal Y-branch), at the components' own values from 1.5 to 1.6 µm: the largest $\lvert \Delta S_{qp} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: "the same components: the devices build them from their stack and parameters, to the bit",
            run: mmi_library,
        },
        Case {
            id: "device/mmi-self-imaging",
            title: r"Devices: the MMIs' first two-fold image from their 2D model's exact $L_\pi = \pi/(\beta_0 - \beta_1)$, 3 to 12 µm wide on 220 nm SOI at 1.55 µm, against Soldano and Pennings's closed forms, $3L_\pi/8$ (1 × 2) and $L_\pi/2$ (2 × 2) with their Eq. 6 (largest relative difference shown, %)",
            tier: Tier::Analytic,
            source: "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995), doi:10.1109/50.372474, Eqs. 4, 6, 33 and 37; Eq. 6 is paraxial, about 1 % short at 3 µm",
            run: mmi_self_imaging,
        },
        Case {
            id: "device/mzi-closed-form",
            title: r"Devices: the MZI of directional couplers (analytic, arms 150 and 100 µm, a phase of 40°) solved as the netlist of its devices against the closed form $C_c \thinspace \mathrm{diag}(t_l, t_u e^{i\varphi}) \thinspace C_s$ on the couplers' and arms' own S-matrices, and the MZI of Y-branches against $s^2 (t_u e^{i\varphi} + t_l)$, from 1.5 to 1.6 µm (largest $\lvert \Delta S \rvert$ shown)",
            tier: Tier::Analytic,
            source: "the transfer matrices of a Mach-Zehnder interferometer (circuit::components::mzi_closed_form); the circuit solve is exact but for round-off",
            run: mzi_closed_form_case,
        },
        Case {
            id: "device/mzi-derivatives",
            title: r"Devices: the MZI's exact $\partial S/\partial\theta$ for its arms' lengths, its phase and its arms' loss, through the netlist's adjoint, against fourth-order central differences (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "forward mode through the device's parameters and the circuit's exact derivatives (circuit::Circuit::derivatives); the differences' truncation and round-off set the tolerance",
            run: mzi_derivatives,
        },
        Case {
            id: "device/rib-wide-slab",
            title: r"Devices: a wide rib's 3D $n_\text{eff}$ through the rib device (Hadley's structure: 3.44 on 3.40 under air, 1 µm film, a 0.5 µm slab, 1.15 µm, TE; Hadley's equations on a 50 nm grid), 8 and 16 µm wide, extrapolated in $1/W^2$, against the film's slab index, the effective index method's limit (shown)",
            tier: Tier::Analytic,
            source: "as the rib widens its mode becomes the film's: the effective index method's lateral slab tends to the vertical slab's index, the exact slab's (mode::slab); the grid's error (5e-6 at 50 nm) sets the tolerance",
            run: rib_wide_slab,
        },
        Case {
            id: "device/rib-eim",
            title: r"Devices: a rib's 2D model (the effective index method) against its 3D model (Hadley's equations on cells fitted about 20 nm) over the 2D model's stated range on 220 nm SOI: widths of 0.4 to 3 µm, etched 50 to 200 nm, at 1.55 µm (largest $\lvert \Delta n_\text{eff} \rvert$ shown)",
            tier: Tier::Analytic,
            source: "the full-vector solver on the rib (G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371); the effective index method's error (G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), doi:10.1364/AO.16.000113) is the 2D model's stated error",
            run: rib_eim,
        },
        Case {
            id: "device/rib-hadley",
            title: r"Devices: Hadley's standard rib through the rib device (3 µm wide, a 1 µm film of 3.44 on 3.40 under air, 1.15 µm, TE, slabs of 0.1 to 0.7 µm; Hadley's equations on a 50 nm grid, the half), as the rib_hadley example (largest difference from his Table I shown)",
            tier: Tier::Published,
            source: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Table I (6 decimals); the 50 nm grid's error, 1.4e-6 (stack/rib-hadley-table), sets the tolerance",
            run: rib_hadley,
        },
        Case {
            id: "device/fdfd-straight",
            title: r"Devices: a strip 2 µm long through the devices' FDFD solves, its guides led out through the PMLs and its ports' phases referred back to its ends, on 100 nm cells (55 nm up the stack in 3D), 2D and 3D: the largest of $\lvert S_{11} \rvert$, $\lvert S_{21} - S_{12} \rvert$ and $\lvert S_{21} - e^{i\beta L} \rvert$, β the port mode's (shown)",
            tier: Tier::Analytic,
            source: "a uniform guide carries its mode whole, with the phase βL between its ends; the port modes are the grid's own and the sources total-field/scattered-field (R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012), doi:10.2528/PIERB11092006); the 3D PMLs' reflection on so coarse a grid sets the tolerance",
            run: fdfd_straight,
        },
        Case {
            id: "device/mmi-2d-grid",
            title: r"Devices: a 1 × 2 MMI's 2D FDFD (3 µm wide, 0.5 µm access guides, at its first two-fold image, 8.48 µm) on grids of 20, 10 and 5 nm: the power into an output, its change from 10 to 5 nm (shown)",
            tier: Tier::Analytic,
            source: "2D FDFD by the effective index method (each cell sampled 8 times a side); the change halving the grid (0.0058, then 0.0021: order 1.5) bounds what is left",
            run: mmi_2d_grid,
        },
        Case {
            id: "device/mmi-guided-2d",
            title: r"Devices: the same MMI's guided-mode propagation analysis (exact slab modes, no grid) against its 2D FDFD on a 5 nm grid, the same plane: the difference in the power into an output (shown)",
            tier: Tier::Analytic,
            source: "the guided modes leave out the radiation the junctions shed and the faces' reflections (L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995), doi:10.1109/50.372474, Section III); 2D FDFD keeps them; the 2D FDFD's own error at 5 nm (about 1e-3) is part of the tolerance",
            run: mmi_guided_2d,
        },
        Case {
            id: "device/y-branch-2d-grid",
            title: r"Devices: Zhang et al.'s Y-branch through the Y-branch device's 2D FDFD (its spline outline, arms 3 µm long to ports 1.5 µm apart) on grids of 40, 20 and 10 nm: the power into an arm, its change from 20 to 10 nm (shown)",
            tier: Tier::Analytic,
            source: "2D FDFD by the effective index method (each cell sampled 8 times a side); the change halving the grid bounds what is left (1.3e-4 more from 10 to 5 nm, measured)",
            run: y_branch_2d_grid,
        },
        Case {
            id: "device/y-branch-ideal-2d",
            title: r"Devices: the Y-branch's ideal model (half the power into each arm, no excess loss) against its 2D model on 10 nm cells, Zhang et al.'s junction with arms 3 µm long: the difference in the power into an arm (shown)",
            tier: Tier::Analytic,
            source: "the ideal model's stated error against 2D: what the junction scatters and reflects, and what the arms' bends shed (a 2D estimate; Y. Zhang et al., Opt. Express 21, 1310 (2013), doi:10.1364/OE.21.001310, give 0.13 dB in 3D FDTD)",
            run: y_branch_ideal_2d,
        },
    ]
}

/// 1.5 to 1.6 µm in steps of 5 nm.
fn band() -> Vec<Wavelength> {
    (0..21).map(|i| um(1.5 + 0.005 * f64::from(i))).collect()
}

/// A device's component and its values.
type Built = (Box<dyn Component>, Vec<f64>);

fn mmi_library() -> Outcome {
    let film = || Planar::Film {
        core: material::silicon(),
        cladding: material::silica(),
        thickness: Length::nm(220.0),
        vertical: Polarization::Te,
    };
    let built = |d: &dyn Device, f: Fidelity| {
        let b = d.component(&Values::defaults(d), f).expect(OK);
        (b.component, b.values)
    };
    let pairs: Vec<(Built, Box<dyn Component>)> = vec![
        (
            built(&StripMmi::one_by_two(), Fidelity::TwoD),
            Box::new(Mmi::new(MmiKind::OneByTwo, film(), 6.0, 1.5, um(1.55)).expect(OK)),
        ),
        (
            built(&StripMmi::two_by_two(), Fidelity::TwoD),
            Box::new(Mmi::new(MmiKind::TwoByTwo, film(), 6.0, 1.5, um(1.55)).expect(OK)),
        ),
        (
            built(&StripYBranch::new(), Fidelity::Analytic),
            Box::new(YBranch::new()),
        ),
    ];
    let mut w: f64 = 0.0;
    for ((a, values), b) in &pairs {
        if *values != b.defaults() || a.provenance() != b.provenance() {
            w = f64::INFINITY;
            continue;
        }
        for l in band() {
            let (sa, sb) = (
                a.s_matrix(l, values).expect(OK),
                b.s_matrix(l, values).expect(OK),
            );
            w = w.max(sa.max_difference(&sb).unwrap_or(f64::INFINITY));
        }
    }
    small(w, 0.0)
}

/// The largest relative difference between the MMIs' images and Soldano and Pennings's.
pub(crate) fn self_imaging() -> f64 {
    let lam = um(1.55);
    let n = |m: &Material| m.refractive_index(lam).expect(OK).re;
    let (nf, nc) = (n(&material::silicon()), n(&material::silica()));
    let nr = Slab::new(nc, nf, nc, Length::nm(220.0))
        .expect(OK)
        .modes(Polarization::Te, lam)[0]
        .effective_index();
    let mut worst: f64 = 0.0;
    for (device, mechanism) in [
        (StripMmi::one_by_two(), SelfImaging::Symmetric),
        (StripMmi::two_by_two(), SelfImaging::Paired),
    ] {
        for (width, access) in [(3.0, 0.8), (6.0, 1.5), (12.0, 1.5)] {
            let v = Values::new(
                &device,
                &[
                    ("width", width),
                    ("access_width", access),
                    ("port_width", access),
                ],
            )
            .expect(OK);
            let b = device.component(&v, Fidelity::TwoD).expect(OK);
            let image = b.figure("L_image").expect(OK).value;
            let beat = crate::generator::Mmi::beat_length(
                Length::um(width),
                nr,
                nc,
                Length::um(1.55),
                true,
            );
            let paper = mechanism.length(2, beat).to_um();
            worst = worst.max((image / paper - 1.0).abs());
        }
    }
    worst
}

fn mmi_self_imaging() -> Outcome {
    small(100.0 * self_imaging(), 1.2)
}

/// The two MZIs' largest |ΔS| against their closed forms.
pub(crate) fn mzi_mismatch() -> f64 {
    let mut worst: f64 = 0.0;
    let phase = 40f64.to_radians();
    let shift = crate::portable::exp_c(c64::new(0.0, phase));
    // of couplers
    let mzi = StripMzi::couplers();
    let v = Values::new(&mzi, &[("phase", 40.0)]).expect(OK);
    let built = mzi.component(&v, Fidelity::Analytic).expect(OK);
    let coupler = StripCoupler::new();
    let cv = Values::defaults(&coupler);
    let c = coupler.component(&cv, Fidelity::Analytic).expect(OK);
    let arm = StripWaveguide::new();
    let arm_at = |l: f64| {
        let av = Values::new(&arm, &[("length", l)]).expect(OK);
        arm.component(&av, Fidelity::Analytic).expect(OK)
    };
    let (upper, lower) = (arm_at(150.0), arm_at(100.0));
    for w in band() {
        let s = built.s_matrix(w).expect(OK);
        let sc = c.s_matrix(w).expect(OK);
        // through (o1 to o4) and across (o1 to o3)
        let tx = (sc[(3, 0)], sc[(2, 0)]);
        let tu = upper.s_matrix(w).expect(OK)[(1, 0)] * shift;
        let tl = lower.s_matrix(w).expect(OK)[(1, 0)];
        let m = mzi_closed_form(tx, tx, tu, tl);
        // M[q][p]: p 0 = o1, 1 = o2; q 0 = o4, 1 = o3
        for (q, row) in [3, 2].iter().zip(&m) {
            for (p, &e) in row.iter().enumerate() {
                worst = worst.max((s[(*q, p)] - e).norm());
            }
        }
    }
    // of Y-branches
    let mzi = StripMzi::y_branches();
    let v = Values::new(&mzi, &[("phase", 40.0), ("excess_loss", 0.3)]).expect(OK);
    let built = mzi.component(&v, Fidelity::Analytic).expect(OK);
    let split = 10f64.powf(-0.3 / 20.0) / 2f64.sqrt();
    for w in band() {
        let s = built.s_matrix(w).expect(OK);
        let tu = upper.s_matrix(w).expect(OK)[(1, 0)] * shift;
        let tl = lower.s_matrix(w).expect(OK)[(1, 0)];
        let e = split * split * (tu + tl);
        worst = worst
            .max((s[(1, 0)] - e).norm())
            .max((s[(0, 1)] - e).norm());
    }
    worst
}

fn mzi_closed_form_case() -> Outcome {
    small(mzi_mismatch(), 1e-14)
}

/// The fourth-order central difference of `f` at x.
fn fourth(f: impl Fn(f64) -> SMatrix, x: f64, h: f64) -> SMatrix {
    let (a, b, c, d) = (f(x + 2.0 * h), f(x + h), f(x - h), f(x - 2.0 * h));
    SMatrix::from_fn(a.size(), |q, p| {
        (-a[(q, p)] + 8.0 * b[(q, p)] - 8.0 * c[(q, p)] + d[(q, p)]) / (12.0 * h)
    })
}

/// The MZIs' exact derivatives against fourth-order differences, largest relative difference.
pub(crate) fn mzi_derivatives_mismatch() -> f64 {
    let w = um(1.551);
    let mut worst: f64 = 0.0;
    for (device, name, h) in [
        (StripMzi::couplers(), "upper", 1e-3),
        (StripMzi::couplers(), "phase", 1e-2),
        (StripMzi::y_branches(), "lower", 1e-3),
        (StripMzi::y_branches(), "loss", 1e-2),
    ] {
        let design = Design::new(&device, &[("phase", "30")]).expect(OK);
        let exact = derivative(&device, &design, Fidelity::Analytic, w, name)
            .expect(OK)
            .expect("an exact derivative");
        let x = design.expressions().value(name).expect(OK);
        let numeric = fourth(
            |v| {
                s_matrix(
                    &device,
                    design.with(name, v).expect(OK).values(),
                    Fidelity::Analytic,
                    w,
                )
                .expect(OK)
            },
            x,
            h,
        );
        let scale = (0..exact.size())
            .flat_map(|q| (0..exact.size()).map(move |p| (q, p)))
            .map(|(q, p)| exact[(q, p)].norm())
            .fold(0.0, f64::max);
        worst = worst.max(exact.max_difference(&numeric).expect(OK) / scale);
    }
    worst
}

fn mzi_derivatives() -> Outcome {
    small(mzi_derivatives_mismatch(), 1e-7)
}

/// A medium of index `n` for Hadley's rib.
fn medium(name: &str, n: f64) -> Material {
    Material::new(
        name,
        Model::Constant(c64::new(n, 0.0)),
        um(1.0),
        um(1.3),
        Provenance {
            reference: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), Fig. 13".into(),
            doi: "10.1109/JLT.2002.800371".into(),
            data: format!("the paper's index, {n}"),
            temperature: None,
            notes: String::new(),
        },
    )
    .expect(OK)
}

/// Hadley's stack: a 1 µm film of 3.44 on 3.40 under air.
fn hadley_stack() -> LayerStack {
    let air = medium("air", 1.0);
    LayerStack::new(
        medium("substrate", 3.40),
        vec![Layer {
            name: "film".into(),
            thickness: Length::um(1.0),
            material: medium("film", 3.44),
            background: air.clone(),
        }],
        air,
    )
    .expect(OK)
}

/// Hadley's rib `width` µm wide with a slab `slab` thick: its TE index at 1.15 µm by his
/// equations on a 50 nm grid, the half `side` µm wide, 4 µm below and above the film's middle.
pub(crate) fn hadley_rib(width: f64, slab: f64, side: f64) -> f64 {
    let lam = um(1.15);
    let mesh = Mesh::window(
        2.0 * side,
        9.0,
        (side / 0.05).round() as usize,
        180,
        Scheme::Hadley,
    )
    .with_symmetry(Symmetry::Vertical);
    let rib = RibWaveguide::on(hadley_stack(), "film")
        .expect(OK)
        .with_mesh(mesh);
    let v = Values::new(
        &rib,
        &[
            ("width", width),
            ("thickness", 1.0),
            ("etch_depth", 1.0 - slab),
            ("wavelength", 1.15),
        ],
    )
    .expect(OK);
    let cs = rib.cross_section(&v, lam).expect(OK);
    crate::mode::hadley::modes(&cs, lam, 1, None).expect(OK)[0]
        .effective_index()
        .re
}

/// The wide rib's limit from 8 and 16 µm, and the film's slab index.
pub(crate) fn wide_slab() -> (f64, f64) {
    let lam = um(1.15);
    let exact = Slab::new(3.40, 3.44, 1.0, Length::um(1.0))
        .expect(OK)
        .modes(Polarization::Te, lam)[0]
        .effective_index();
    let widths = [8.0, 16.0];
    let n = map_in_order(&widths, |&w| hadley_rib(w, 0.5, w / 2.0 + 3.0));
    let r = (widths[1] / widths[0]).powi(2);
    ((r * n[1] - n[0]) / (r - 1.0), exact)
}

fn rib_wide_slab() -> Outcome {
    let (limit, exact) = wide_slab();
    outcome(limit, exact, 2e-5)
}

/// The rib designs the 2D model's stated range spans: width and etch depth, µm.
const RIBS: [(f64, f64); 6] = [
    (0.4, 0.05),
    (0.4, 0.2),
    (0.5, 0.13),
    (1.0, 0.13),
    (3.0, 0.05),
    (3.0, 0.2),
];

/// The largest |Δn_eff| of the rib's effective index method against its 3D model at [`RIBS`].
pub(crate) fn eim_error() -> f64 {
    static E: OnceLock<f64> = OnceLock::new();
    *E.get_or_init(|| {
        let rib = RibWaveguide::new().with_mesh(Mesh::fitted(0.02, 1.5, Scheme::Hadley));
        let lam = um(1.55);
        map_in_order(&RIBS, |&(w, e)| {
            let v = Values::new(&rib, &[("width", w), ("etch_depth", e)]).expect(OK);
            let (eim, _) = rib.effective_index(&v, lam).expect(OK);
            let cs = rib.cross_section(&v, lam).expect(OK);
            let n = crate::mode::hadley::modes(&cs, lam, 1, None).expect(OK)[0]
                .effective_index()
                .re;
            (eim - n).abs()
        })
        .into_iter()
        .fold(0.0, f64::max)
    })
}

fn rib_eim() -> Outcome {
    small(eim_error(), super::rib::EIM_ERROR)
}

/// Hadley's Table I.
const HADLEY_TABLE: [f64; 8] = [
    3.412022, 3.412126, 3.412279, 3.412492, 3.412774, 3.413132, 3.413571, 3.414100,
];

/// The largest difference from Hadley's Table I through the rib device, slabs 0.1 to 0.7 µm.
pub(crate) fn hadley_table() -> f64 {
    let slabs: Vec<usize> = (1..HADLEY_TABLE.len()).collect();
    map_in_order(&slabs, |&k| {
        (hadley_rib(3.0, k as f64 / 10.0, 5.0) - HADLEY_TABLE[k]).abs()
    })
    .into_iter()
    .fold(0.0, f64::max)
}

fn rib_hadley() -> Outcome {
    small(hadley_table(), 2e-6)
}

/// The small 1 × 2 MMI the 2D cases solve: 3 µm wide, 0.5 µm access guides (the outputs 1 µm
/// apart, edge to edge), at its first two-fold image.
pub(crate) fn small_mmi() -> (StripMmi, Values) {
    let mmi = StripMmi::one_by_two();
    let v = Values::new(
        &mmi,
        &[("width", 3.0), ("access_width", 0.5), ("port_width", 0.5)],
    )
    .expect(OK);
    let image = mmi
        .component(&v, Fidelity::TwoD)
        .expect(OK)
        .figure("L_image")
        .expect(OK)
        .value;
    let v = v.with(&mmi, "length", image).expect(OK);
    (mmi, v)
}

/// The power into `o2` from `o1` of `device` at `values` at its 2D fidelity,
/// at its centre wavelength.
fn fdfd_power(device: &dyn Device, values: &Values) -> f64 {
    device
        .component(values, Fidelity::TwoD)
        .expect(OK)
        .figure("P_o2")
        .expect(OK)
        .value
}

/// The small MMI's power into `o2` by 2D FDFD at 20, 10 and 5 nm.
pub(crate) fn mmi_2d() -> &'static [f64; 3] {
    static P: OnceLock<[f64; 3]> = OnceLock::new();
    P.get_or_init(|| {
        let (mmi, v) = small_mmi();
        let p = map_in_order(&[0.02, 0.01, 0.005], |&h| {
            let d = mmi
                .clone()
                .with_model_2d(Model2d::Fdfd)
                .with_grid_2d(FdfdGrid::new(h).with_samples(1));
            fdfd_power(&d, &v)
        });
        [p[0], p[1], p[2]]
    })
}

fn mmi_2d_grid() -> Outcome {
    let p = mmi_2d();
    small((p[2] - p[1]).abs(), 5e-3)
}

fn mmi_guided_2d() -> Outcome {
    let (mmi, v) = small_mmi();
    let guided = fdfd_power(&mmi, &v);
    small((guided - mmi_2d()[2]).abs(), super::mmi::GUIDED_ERROR)
}

/// Zhang et al.'s Y-branch with short arms, by 2D FDFD at 40, 20 and 10 nm: the power into o2.
pub(crate) fn y_branch_2d() -> &'static [f64; 3] {
    static P: OnceLock<[f64; 3]> = OnceLock::new();
    P.get_or_init(|| {
        let y = StripYBranch::new();
        let v = Values::new(&y, &[("arm_length", 3.0), ("separation", 1.5)]).expect(OK);
        let p = map_in_order(&[0.04, 0.02, 0.01], |&h| {
            let d = y.clone().with_grid_2d(FdfdGrid::new(h).with_samples(1));
            fdfd_power(&d, &v)
        });
        [p[0], p[1], p[2]]
    })
}

fn y_branch_ideal_2d() -> Outcome {
    let p = y_branch_2d();
    small((0.5 - p[2]).abs(), super::ybranch::IDEAL_ERROR)
}

fn y_branch_2d_grid() -> Outcome {
    let p = y_branch_2d();
    small((p[2] - p[1]).abs(), 1e-3)
}

/// A straight strip 2 µm long through the FDFD solves, 2D and 3D on 100 nm cells: the largest of
/// |S₁₁|, |S₂₁ − S₁₂| and |S₂₁ − e^(iβL)|.
pub(crate) fn straight_mismatch() -> f64 {
    let guide = StripWaveguide::new();
    let v = Values::new(&guide, &[("length", 2.0)]).expect(OK);
    let g = guide.geometry(&v).expect(OK);
    let lam = um(1.55);
    let mut worst: f64 = 0.0;
    for three in [false, true] {
        let grid = FdfdGrid::new(0.1)
            .with_step_z(0.055)
            .with_margin(0.4)
            .with_pml(8)
            .with_samples(1);
        let (sampled, _) =
            super::solve::sampled("strip", &g, "Si", &[lam], &grid, three).expect(OK);
        let s = sampled.s_matrix(lam, &[]).expect(OK);
        let n = sampled.ports()[0]
            .mode
            .as_ref()
            .expect("a port's mode")
            .effective_index;
        let ahead = crate::portable::exp_c(c64::new(0.0, lam.wavenumber() * n * 2.0));
        worst = worst
            .max(s[(0, 0)].norm())
            .max(s[(1, 1)].norm())
            .max((s[(1, 0)] - s[(0, 1)]).norm())
            .max((s[(1, 0)] - ahead).norm());
    }
    worst
}

fn fdfd_straight() -> Outcome {
    small(straight_mismatch(), 1e-4)
}
