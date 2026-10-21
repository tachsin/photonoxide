//! What the validation report checks the devices on: their analytic models against the studio
//! library's components and their closed forms, their exact derivatives, each lower fidelity
//! against the next one up over its stated range, the solvers' orders in their grids, the same
//! bits on any number of threads, and published results reproduced through them.

use std::f64::consts::PI;
use std::sync::OnceLock;

use faer::c64;

use super::*;
use crate::circuit::components::{
    AddDropRing, AllPassRing, Bend, DirectionalCoupler, Dispersion, Supermodes, Waveguide,
};
use crate::material::{self, LorentzPole, Material, Model, Provenance};
use crate::mode::Polarization;
use crate::mode::bend::SlabBend;
use crate::mode::slab::Slab;
use crate::mode::vector;
use crate::parallel::map_in_order;
use crate::stack::Layer;
use crate::units::{Frequency, refractive_index};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid device";
const A: Fidelity = Fidelity::Analytic;
const D3: Fidelity = Fidelity::ThreeD;

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

/// `x` to `places` decimals: a figure shown without the last bits of a platform's arithmetic.
fn round(x: f64, places: i32) -> f64 {
    let s = 10f64.powi(places);
    (x * s).round() / s
}

/// The order of convergence from three values on grids halving: log₂ of the ratio of the
/// successive differences.
fn order(coarse: f64, middle: f64, fine: f64) -> f64 {
    ((coarse - middle) / (middle - fine)).abs().log2()
}

/// The devices' validation cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/library-components",
            title: r"Devices: each analytic model at its defaults (a 500 x 220 nm silicon strip; a 10 µm bend; a 200 nm gap; the rings) against the studio library's component as it has always been built, at the component's own parameter values from 1.5 to 1.6 µm: the largest $\lvert \Delta S_{qp} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: "the same component: the library's strip (n_eff 2.44506, n_g 4.172901 at 1.55 µm, 2.7 dB/cm) and coupling (0.04228/µm), which the fits are anchored at",
            run: library_components,
        },
        Case {
            id: "device/ring-closed-forms",
            title: r"Devices: an all-pass ring's figures (12 µm radius, 0.1 dB per round trip) against Bogaerts's closed forms on its guide and coupling: the free spectral range $\lambda^2/(n_g L)$ and the loaded $Q = \pi n_g L \sqrt{ra}/(\lambda(1 - ra))$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011), doi:10.1002/lpor.201100017, Eqs. 9 and 20",
            run: ring_closed_forms,
        },
        Case {
            id: "device/coupler-closed-forms",
            title: r"Devices: a directional coupler's figures (250 nm gap, 9 µm long, lossless) against coupled-mode theory: $L_x = \pi/2C$ and the power across $\sin^2(CL)$ (largest difference shown)",
            tier: Tier::Analytic,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Eqs. 4.1, 4.3 and 4.5",
            run: coupler_closed_forms,
        },
        Case {
            id: "device/derivatives",
            title: r"Devices: the exact $\partial S/\partial\theta$ through the components' derivatives, the devices' settings and the expressions, for a waveguide's length and loss, a bend's angle, a coupler's length and a ring's radius (as $1.5 r$) and coupling length, against fourth-order central differences (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "forward mode: the components' dual numbers and the expressions' (R. E. Wengert, Commun. ACM 7, 463 (1964), doi:10.1145/355586.364791); the differences' truncation and round-off set the tolerance",
            run: derivatives,
        },
        Case {
            id: "device/waveguide-fit-neff",
            title: r"Devices: a silicon strip's analytic $n_\text{eff}$ (the fit) against its 3D model (Hadley's equations, cells fitted about 10 nm) at five designs between the fit's nodes, 400 to 600 nm wide and 200 to 240 nm thick, at 1.55 µm (largest $\lvert \Delta n_\text{eff} \rvert$ shown)",
            tier: Tier::Analytic,
            source: "the full-vector solver (G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371); the interpolation and the anchor's shift (5e-5) set the tolerance",
            run: waveguide_fit_neff,
        },
        Case {
            id: "device/waveguide-fit-ng",
            title: r"Devices: the same designs' analytic $n_g$ against their 3D model (largest $\lvert \Delta n_g \rvert$ shown)",
            tier: Tier::Analytic,
            source: "the full-vector solver, n_g from five wavelengths 25 nm apart; the tolerance is the analytic model's stated error, most of it the anchor's shift (0.0164: the book's silicon against Li's)",
            run: waveguide_fit_ng,
        },
        Case {
            id: "device/waveguide-3d-order",
            title: r"Devices: the 3D waveguide's order of convergence in its grid, the 500 x 220 nm strip's $n_\text{eff}$ at 1.55 µm on cells fitted 20, 10 and 5 nm apart by Hadley's equations (shown to two decimals)",
            tier: Tier::Analytic,
            source: "second order at the strip's corners: G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371 (the standard scheme's is about first)",
            run: waveguide_order,
        },
        Case {
            id: "device/bend-analytic",
            title: r"Devices: a bend's analytic model (the straight strip's mode along the arc) against its 3D model (the straight strip by Hadley's equations on cells fitted about 10 nm, plus the conformal map's shift on cells of about 20 nm with a PML) at 5 and 10 µm radius: the largest $\lvert \Delta n_\text{eff} \rvert$ along the arc (shown)",
            tier: Tier::Analytic,
            source: "the full-vector solver on the bent strip (M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975), doi:10.1109/JQE.1975.1068563); the tolerance is the analytic model's stated error for radii from 5 µm",
            run: bend_analytic,
        },
        Case {
            id: "device/bend-2d",
            title: r"Devices: a bend's 2D model (the exact bent slab of the strip seen from above by the effective index method) against its 3D model at 2, 5 and 10 µm radius: the largest $\lvert \Delta n_\text{eff} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: "the full-vector solver on the bent strip; the effective index method's own error (+1.8 % for this strip, docs/methods/eim.md) sets the stated error",
            run: bend_2d,
        },
        Case {
            id: "device/bend-3d-grid",
            title: r"Devices: the 3D bend's shift (the strip bent at 5 µm by the conformal map less the same window straight, on the same cells) at 1.55 µm on cells of about 28, 14 and 7 nm: its largest change from one grid to the next (shown; the shift is 2.4e-3)",
            tier: Tier::Analytic,
            source: "the corners' error cancels in the shift: on these grids the standard scheme moves the strip's own index by 1.3e-3 a halving (the strip_waveguide example), the shift by under 3e-5; the map itself converges at second order (mode/bend-slab-te, M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975), doi:10.1109/JQE.1975.1068563)",
            run: bend_grid,
        },
        Case {
            id: "device/coupler-fit",
            title: r"Devices: a directional coupler's analytic cross-over length (the fit's coupling, constant in $\lambda$) against its 3D model (the supermodes, cells fitted about 10 nm) at four designs over gaps of 150 to 450 nm, widths of 400 to 600 nm and thicknesses of 200 to 240 nm, at 1.548 and 1.552 µm (largest relative difference shown, %)",
            tier: Tier::Analytic,
            source: "the full-vector solver's supermodes with mirror walls; the tolerance is the analytic model's stated error over its band, 1.548 to 1.552 µm (its coupling is 1.55 µm's at every wavelength)",
            run: coupler_fit,
        },
        Case {
            id: "device/coupler-3d-order",
            title: r"Devices: the 3D coupler's order of convergence in its grid, two 500 x 220 nm strips 200 nm apart, $\Delta n$ at 1.55 µm on cells fitted 20, 10 and 5 nm apart (shown to two decimals)",
            tier: Tier::Analytic,
            source: "about first order: the standard full-vector scheme at the strips' convex corners (Fallahkhair et al. 2008, doi:10.1109/JLT.2008.923643)",
            run: coupler_order,
        },
        Case {
            id: "device/ring-fsr",
            title: r"Devices: an all-pass ring's analytic free spectral range against its 3D model (the strip's mode from the full-vector solver) at 10 µm radius and a 200 nm gap (relative difference shown, %)",
            tier: Tier::Analytic,
            source: "Bogaerts et al. 2011, doi:10.1002/lpor.201100017, Eq. 9 on each model's group index; the tolerance is the analytic model's stated error",
            run: ring_fsr,
        },
        Case {
            id: "device/threads",
            title: r"Devices: the 3D waveguide (five solves by Hadley's equations, cells fitted about 10 nm) on 1 and 20 threads: the largest difference in its figures and its S-matrix from 1.5 to 1.6 µm (shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md); the solves are independent and collected in order",
            run: threads,
        },
        Case {
            id: "device/bend-ring-slab",
            title: r"Devices: a bend's 2D model at 2 µm radius on 220 nm SOI, 500 nm wide, from 1.524 to 1.564 µm, against the bent slab of the FDFD ring of `components/ring-fsr-fdfd`: the difference in $n_g$ (shown)",
            tier: Tier::Analytic,
            source: "the same model: the strip's vertical slab (220 nm of Li's silicon in Malitson's silica, TE) as the core of an exact bent slab (D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x)",
            run: bend_ring_slab,
        },
        Case {
            id: "device/strip-chrostowski",
            title: r"Devices: a 500 x 220 nm strip of 3.473 in 1.444 at 1550 nm through the waveguide device, the full 2.1 x 1.5 µm window on a 5 nm grid (TE-like $n_\text{eff}$ shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Fig. 3.14: 2.443 (Lumerical MODE, 20 nm); the strip's corners converge about first order, so ± 3e-3 at 5 nm, as the strip_waveguide example",
            run: strip_chrostowski,
        },
        Case {
            id: "device/coupler-chrostowski",
            title: r"Devices: two 500 x 220 nm strips 200 nm apart (the book's silicon in 1.444 oxide) through the coupler device, a quarter of the book's 5.5 x 1.22 µm window on a 5 nm grid: the cross-over length at 1.55 µm (µm, shown)",
            tier: Tier::Published,
            source: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), doi:10.1017/CBO9781316084168, Eq. 4.14 at a 200 nm gap: 37.5 µm, as the directional_coupler example",
            run: coupler_chrostowski,
        },
        Case {
            id: "device/ring-q-bogaerts",
            title: r"Devices: an all-pass ring at critical coupling through the ring device (a stated guide: $n_\text{eff}$ 2.4, $n_g$ 4.30, 2.7 dB/cm, 0.075 dB per round trip): the round trip at which its loaded Q peaks (mm, shown)",
            tier: Tier::Published,
            source: "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011), doi:10.1002/lpor.201100017, Section 2.4 and Fig. 5: \"approximately 10 mm\", as the ring_q_factor example",
            run: ring_q_bogaerts,
        },
        Case {
            id: "device/wire-ng-dwivedi",
            title: r"Devices: a 602 x 211 nm silicon wire in silica through the waveguide device (Hadley's equations, cells fitted about 10 nm, a 60 nm span): its $n_g$ at 1550 nm (shown)",
            tier: Tier::Published,
            source: "S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Table I, measured: 4.0453 ± 0.0045; the tolerance is the mzi_dwivedi example's, their Eq. 5 with ±20 nm of width and ±5 nm of thickness",
            run: wire_ng_dwivedi,
        },
    ]
}

/// The studio library's strip.
fn strip() -> Dispersion {
    Dispersion::new(um(1.55), 2.44506, 4.172_901).with_loss(2.7)
}

/// 1.5 to 1.6 µm in steps of 5 nm.
fn band() -> Vec<Wavelength> {
    (0..21).map(|i| um(1.5 + 0.005 * f64::from(i))).collect()
}

/// The largest |ΔS| between two components over the band, each at its own defaults.
fn worst(a: &dyn Component, b: &dyn Component) -> f64 {
    let (va, vb) = (a.defaults(), b.defaults());
    if va != vb || a.provenance() != b.provenance() {
        return f64::INFINITY;
    }
    band()
        .into_iter()
        .map(|w| {
            let (sa, sb) = (a.s_matrix(w, &va).expect(OK), b.s_matrix(w, &vb).expect(OK));
            sa.max_difference(&sb).unwrap_or(f64::INFINITY)
        })
        .fold(0.0, f64::max)
}

fn library_components() -> Outcome {
    let built = |d: &dyn Device| d.component(&Values::defaults(d), A).expect(OK).component;
    let pairs: Vec<(Box<dyn Component>, Box<dyn Component>)> = vec![
        (
            built(&StripWaveguide::new()),
            Box::new(Waveguide::new(strip())),
        ),
        (
            built(&StripBend::new()),
            Box::new(Bend::new(10.0, strip()).expect(OK)),
        ),
        (
            built(&StripCoupler::new()),
            Box::new(
                DirectionalCoupler::new(Supermodes::from_coupling(strip(), 0.042_28)).expect(OK),
            ),
        ),
        (
            built(&StripRing::all_pass()),
            Box::new(AllPassRing::new(strip()).expect(OK)),
        ),
        (
            built(&StripRing::add_drop()),
            Box::new(AddDropRing::new(strip()).expect(OK)),
        ),
    ];
    let w = pairs
        .iter()
        .map(|(a, b)| worst(a.as_ref(), b.as_ref()))
        .fold(0.0, f64::max);
    Outcome {
        measured: w,
        expected: 0.0,
        tolerance: 0.0,
        error: w,
    }
}

fn ring_closed_forms() -> Outcome {
    let ring = StripRing::all_pass();
    let values = Values::new(&ring, &[("radius", 12.0), ("round_trip_loss", 0.1)]).expect(OK);
    let built = ring.component(&values, A).expect(OK);
    let f = |s: &str| built.figure(s).expect(OK).value;
    let (l, kappa2) = (built.values[0], built.values[1]);
    let (n_g, l0) = (strip().group_index, 1.55);
    let fsr = l0 * l0 / (n_g * l) * 1e3;
    let a = 10f64.powf(-(2.7 * l * 1e-4 + 0.1) / 20.0);
    let ra = (1.0 - kappa2).sqrt() * a;
    let q = PI * n_g * l * ra.sqrt() / (l0 * (1.0 - ra));
    let e = (f("FSR") / fsr - 1.0).abs().max((f("Q") / q - 1.0).abs());
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 1e-12,
        error: e,
    }
}

fn coupler_closed_forms() -> Outcome {
    let coupler = StripCoupler::new();
    let values = Values::new(&coupler, &[("gap", 0.25), ("length", 9.0), ("loss", 0.0)]).expect(OK);
    let built = coupler.component(&values, A).expect(OK);
    let f = |s: &str| built.figure(s).expect(OK).value;
    let c = f("C");
    let e = (f("L_x") / (PI / (2.0 * c)) - 1.0)
        .abs()
        .max((f("cross") - libm::sin(c * 9.0).powi(2)).abs());
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 1e-12,
        error: e,
    }
}

/// The fourth-order central difference of `f` at x.
fn fourth(f: impl Fn(f64) -> SMatrix, x: f64, h: f64) -> SMatrix {
    let (a, b, c, d) = (f(x + 2.0 * h), f(x + h), f(x - h), f(x - 2.0 * h));
    SMatrix::from_fn(a.size(), |q, p| {
        (-a[(q, p)] + 8.0 * b[(q, p)] - 8.0 * c[(q, p)] + d[(q, p)]) / (12.0 * h)
    })
}

/// A device, its design's definitions, the parameter to differentiate by, and the step.
pub(crate) type DerivativeCase = (
    Box<dyn Device>,
    Vec<(&'static str, &'static str)>,
    &'static str,
    f64,
);

/// A design's analytic and 3D (n_eff, n_g).
type Pair = ((f64, f64), (f64, f64));

/// The largest relative difference between exact derivatives and fourth-order differences.
pub(crate) fn derivatives_mismatch() -> f64 {
    let w = um(1.551);
    let cases: Vec<DerivativeCase> = vec![
        (
            Box::new(StripWaveguide::new()),
            vec![("length", "120")],
            "length",
            1e-3,
        ),
        (
            Box::new(StripWaveguide::new()),
            vec![("loss", "40")],
            "loss",
            1e-2,
        ),
        (
            Box::new(StripBend::new()),
            vec![("angle", "75")],
            "angle",
            1e-3,
        ),
        (
            Box::new(StripCoupler::new()),
            vec![("length", "12")],
            "length",
            1e-3,
        ),
        (
            Box::new(StripRing::all_pass()),
            vec![("r", "8"), ("radius", "1.5*r"), ("coupling_length", "1")],
            "r",
            1e-6,
        ),
        (
            Box::new(StripRing::add_drop()),
            vec![("coupling_length", "1.5"), ("round_trip_loss", "0.2")],
            "coupling_length",
            1e-5,
        ),
    ];
    let mut worst: f64 = 0.0;
    for (device, definitions, name, h) in cases {
        let device = device.as_ref();
        let design = Design::new(device, &definitions).expect(OK);
        let exact = derivative(device, &design, A, w, name)
            .expect(OK)
            .expect("an exact derivative");
        let x = design.expressions().value(name).expect(OK);
        let numeric = fourth(
            |v| s_matrix(device, design.with(name, v).expect(OK).values(), A, w).expect(OK),
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

fn derivatives() -> Outcome {
    let e = derivatives_mismatch();
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 1e-7,
        error: e,
    }
}

/// The designs between the fit's nodes, and at its corners: width and thickness, µm.
const DESIGNS: [(f64, f64); 5] = [
    (0.425, 0.205),
    (0.575, 0.235),
    (0.475, 0.225),
    (0.4, 0.24),
    (0.6, 0.2),
];

/// The analytic and 3D (n_eff, n_g) at [`DESIGNS`], solved once.
fn waveguide_fit() -> &'static Vec<Pair> {
    static FIT: OnceLock<Vec<Pair>> = OnceLock::new();
    FIT.get_or_init(|| {
        let guide = StripWaveguide::new();
        map_in_order(&DESIGNS, |&(w, t)| {
            let v = Values::new(&guide, &[("width", w), ("thickness", t)]).expect(OK);
            let figures = |f: Fidelity| {
                let b = guide.component(&v, f).expect(OK);
                (
                    b.figure("n_eff").expect(OK).value,
                    b.figure("n_g").expect(OK).value,
                )
            };
            (figures(A), figures(D3))
        })
    })
}

fn waveguide_fit_neff() -> Outcome {
    let e = waveguide_fit()
        .iter()
        .map(|(a, b)| (a.0 - b.0).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 1e-4,
        error: e,
    }
}

fn waveguide_fit_ng() -> Outcome {
    let e = waveguide_fit()
        .iter()
        .map(|(a, b)| (a.1 - b.1).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: stated(&StripWaveguide::new(), A),
        error: e,
    }
}

/// The stated error of `device`'s model at `fidelity`.
fn stated(device: &dyn Device, fidelity: Fidelity) -> f64 {
    device
        .fidelities()
        .iter()
        .find(|m| m.fidelity == fidelity)
        .and_then(|m| m.error.as_ref())
        .map_or(0.0, |s| s.bound)
}

fn waveguide_order() -> Outcome {
    let guide = StripWaveguide::new();
    let v = Values::defaults(&guide);
    let l = um(1.55);
    // a quarter of a 2.1 × 1.5 µm window, 12.5 × 10, 6.25 × 5 and 3.125 × 2.5 nm
    let n = map_in_order(&[1, 2, 4], |&s| {
        let g = guide
            .clone()
            .with_mesh(Mesh::window(2.1, 1.5, 84 * s, 75 * s, Scheme::Hadley));
        let cs = g.cross_section(&v, l).expect(OK);
        crate::mode::hadley::modes(&cs, l, 1, None).expect(OK)[0]
            .effective_index()
            .re
    });
    outcome(round(order(n[0], n[1], n[2]), 2), 2.0, 0.5)
}

/// The bend's n_eff along the arc at `fidelity`, radius `r` µm.
fn bend_index(bend: &StripBend, r: f64, fidelity: Fidelity) -> f64 {
    let v = Values::new(bend, &[("radius", r), ("loss", 0.0)]).expect(OK);
    bend.component(&v, fidelity)
        .expect(OK)
        .figure("n_eff")
        .expect(OK)
        .value
}

/// The 3D bend's n_eff at 2, 5 and 10 µm, solved once.
fn bends_3d() -> &'static Vec<f64> {
    static BENDS: OnceLock<Vec<f64>> = OnceLock::new();
    BENDS.get_or_init(|| {
        let bend = StripBend::new();
        map_in_order(&[2.0, 5.0, 10.0], |&r| bend_index(&bend, r, D3))
    })
}

fn bend_analytic() -> Outcome {
    let bend = StripBend::new();
    let three = bends_3d();
    let e = [(5.0, three[1]), (10.0, three[2])]
        .iter()
        .map(|&(r, n)| (bend_index(&bend, r, A) - n).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: stated(&bend, A),
        error: e,
    }
}

fn bend_2d() -> Outcome {
    let bend = StripBend::new();
    let e = [2.0, 5.0, 10.0]
        .iter()
        .zip(bends_3d())
        .map(|(&r, &n)| (bend_index(&bend, r, Fidelity::TwoD) - n).abs())
        .fold(0.0, f64::max);
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: stated(&bend, Fidelity::TwoD),
        error: e,
    }
}

/// The 3D bend's shift at 5 µm on cells of w/18, w/36 and w/72 across and t/8, t/16 and t/32
/// up (about 28, 14 and 7 nm): the mapped strip's n_eff less the same window's straight.
fn bend_shifts() -> Vec<f64> {
    let bend = StripBend::new();
    let v = Values::new(&bend, &[("radius", 5.0)]).expect(OK);
    let l = um(1.55);
    let n = |cs: &crate::mode::vector::CrossSection| {
        vector::modes(cs, l, 1, Some(2.45)).expect(OK)[0]
            .effective_index()
            .re
    };
    map_in_order(&[18.0, 36.0, 72.0], |&per| {
        let b = bend
            .clone()
            .with_mesh(Mesh::fitted(0.5 / per, 1.0, Scheme::Standard));
        n(&b.cross_section(&v, l).expect(OK)) - n(&b.straight_section(&v, l).expect(OK))
    })
}

fn bend_grid() -> Outcome {
    let s = bend_shifts();
    let e = (s[1] - s[0]).abs().max((s[2] - s[1]).abs());
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 5e-5,
        error: e,
    }
}

fn coupler_fit() -> Outcome {
    let coupler = StripCoupler::new();
    let designs = [
        (0.5, 0.22, 0.2),
        (0.45, 0.21, 0.25),
        (0.55, 0.23, 0.15),
        (0.4, 0.2, 0.45),
    ];
    let errors = map_in_order(&designs, |&(w, t, g)| {
        let v = Values::new(&coupler, &[("width", w), ("thickness", t), ("gap", g)]).expect(OK);
        let (three, analytic) = (
            coupler.coupler(&v, D3).expect(OK),
            coupler.coupler(&v, A).expect(OK),
        );
        [1.548, 1.552]
            .iter()
            .map(|&l| {
                let l = um(l);
                let x3 = three.supermodes().cross_over_length(l);
                100.0 * (analytic.supermodes().cross_over_length(l) / x3 - 1.0).abs()
            })
            .fold(0.0, f64::max)
    });
    let e = errors.into_iter().fold(0.0, f64::max);
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: stated(&coupler, A),
        error: e,
    }
}

fn coupler_order() -> Outcome {
    let coupler = StripCoupler::new();
    let v = Values::defaults(&coupler);
    let l = um(1.55);
    // a quarter of a 5.5 × 1.32 µm window, 25 × 27.5, 12.5 × 13.75 and 6.25 × 6.875 nm
    let dn = map_in_order(&[1, 2, 4], |&s| {
        let c =
            coupler
                .clone()
                .with_mesh(Mesh::window(5.5, 1.32, 110 * s, 24 * s, Scheme::Standard));
        let (even, odd) = c.cross_sections(&v, l).expect(OK);
        let n = |cs| {
            vector::modes(cs, l, 1, None).expect(OK)[0]
                .effective_index()
                .re
        };
        n(&even) - n(&odd)
    });
    outcome(round(order(dn[0], dn[1], dn[2]), 2), 1.0, 0.5)
}

fn ring_fsr() -> Outcome {
    let ring = StripRing::all_pass();
    let v = Values::defaults(&ring);
    let fsr = |f: Fidelity| {
        ring.component(&v, f)
            .expect(OK)
            .figure("FSR")
            .expect(OK)
            .value
    };
    let (three, analytic) = (fsr(D3), fsr(A));
    let e = 100.0 * (analytic / three - 1.0).abs();
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: stated(&ring, A),
        error: e,
    }
}

fn threads() -> Outcome {
    let guide = StripWaveguide::new();
    let v = Values::defaults(&guide);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a thread pool")
            .install(|| {
                let built = guide.component(&v, D3).expect(OK);
                let s: Vec<SMatrix> = band()
                    .into_iter()
                    .map(|w| built.s_matrix(w).expect(OK))
                    .collect();
                (built.figures, s)
            })
    };
    let ((f1, s1), (f20, s20)) = (run(1), run(20));
    let figures = f1
        .iter()
        .zip(&f20)
        .map(|(a, b)| {
            if a.grid == b.grid {
                (a.value - b.value).abs()
            } else {
                f64::INFINITY
            }
        })
        .fold(0.0, f64::max);
    let matrices = s1
        .iter()
        .zip(&s20)
        .map(|(a, b)| a.max_difference(b).unwrap_or(f64::INFINITY))
        .fold(0.0, f64::max);
    let e = figures.max(matrices);
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

fn bend_ring_slab() -> Outcome {
    // the bent slab of components/ring-fsr-fdfd, as its check builds it
    let ring_bend = |w: Wavelength| -> crate::Result<SlabBend> {
        let n =
            |m: &Material| -> crate::Result<f64> { Ok(refractive_index(m.permittivity(w)?).re) };
        let (nf, nc) = (n(&material::silicon())?, n(&material::silica())?);
        let ridge = Slab::new(nc, nf, nc, Length::nm(220.0))?
            .modes(Polarization::Te, w)
            .first()
            .map(crate::mode::slab::SlabMode::effective_index)
            .ok_or_else(|| crate::Error::invalid("ring", "the film guides no mode"))?;
        SlabBend::new(
            Length::um(2.0),
            nc,
            &[(ridge, Length::um(0.5))],
            nc,
            Length::um(-0.25),
        )
    };
    let (reference, _) =
        Dispersion::from_bent_slab(ring_bend, Polarization::Tm, um(1.544), 0.01).expect(OK);
    let bend = StripBend::new();
    let v = Values::new(
        &bend,
        &[
            ("radius", 2.0),
            ("loss", 0.0),
            ("wavelength", 1.544),
            ("span", 0.04),
        ],
    )
    .expect(OK);
    let d = *bend.bend(&v, Fidelity::TwoD).expect(OK).dispersion();
    let e = (d.group_index - reference.group_index)
        .abs()
        .max((d.effective_index - reference.effective_index).abs());
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

/// Chrostowski & Hochberg's oxide, constant 1.444.
fn oxide() -> Material {
    Material::new(
        "oxide (1.444)",
        Model::Constant(c64::new(1.444, 0.0)),
        um(1.0),
        um(2.0),
        books(),
    )
    .expect(OK)
}

fn books() -> Provenance {
    Provenance {
        reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Listing 3.1"
            .into(),
        doi: "10.1017/CBO9781316084168".into(),
        data: String::new(),
        temperature: Some(300.0),
        notes: String::new(),
    }
}

/// A stack of `core` 220 nm thick on 2 µm of the book's oxide, in it.
fn books_stack(core: Material) -> crate::stack::LayerStack {
    let ox = oxide();
    crate::stack::LayerStack::new(
        ox.clone(),
        vec![
            Layer {
                name: "BOX".into(),
                thickness: Length::um(2.0),
                material: ox.clone(),
                background: ox.clone(),
            },
            Layer {
                name: "Si".into(),
                thickness: Length::nm(220.0),
                material: core,
                background: ox.clone(),
            },
        ],
        ox,
    )
    .expect(OK)
}

fn strip_chrostowski() -> Outcome {
    let core = Material::new(
        "Si (3.473)",
        Model::Constant(c64::new(3.473, 0.0)),
        um(1.0),
        um(2.0),
        books(),
    )
    .expect(OK);
    let guide = StripWaveguide::on(books_stack(core), "Si")
        .expect(OK)
        .with_mesh(
            Mesh::window(2.1, 1.5, 420, 300, Scheme::Standard).with_symmetry(Symmetry::Full),
        );
    let l = um(1.55);
    let cs = guide.cross_section(&Values::defaults(&guide), l).expect(OK);
    let n = vector::modes(&cs, l, 1, None).expect(OK)[0]
        .effective_index()
        .re;
    outcome(n, 2.443, 3e-3)
}

/// The book's Lorentz fit to silicon (Eq. 3.2, Listing 3.1).
fn books_silicon() -> Material {
    let resonance = 3.932_824_66e15 / (2.0 * PI) / 1e12;
    Material::new(
        "Si (Chrostowski & Hochberg's Lorentz fit)",
        Model::Lorentz {
            eps_inf: 7.987_374_92,
            poles: vec![LorentzPole {
                strength: 3.687_991_43,
                resonance: Frequency::thz(resonance).expect(OK),
                damping: 0.0,
            }],
        },
        um(1.15),
        um(1.8),
        books(),
    )
    .expect(OK)
}

fn coupler_chrostowski() -> Outcome {
    let coupler = StripCoupler::on(books_stack(books_silicon()), "Si")
        .expect(OK)
        .with_mesh(Mesh::window(5.5, 1.22, 550, 122, Scheme::Standard));
    let l = um(1.55);
    let (even, odd) = coupler
        .cross_sections(&Values::defaults(&coupler), l)
        .expect(OK);
    let n = |cs| {
        vector::modes(cs, l, 1, None).expect(OK)[0]
            .effective_index()
            .re
    };
    let lx = 1.55 / (2.0 * (n(&even) - n(&odd)));
    outcome(lx, 37.5, 1.0)
}

/// The round trip (µm) in [lo, hi] where `f` peaks, by golden-section search.
fn peak(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut fa, mut fb) = (f(a), f(b));
    while hi - lo > 1e-6 * hi {
        if fa > fb {
            hi = b;
            (b, fb) = (a, fa);
            a = hi - g * (hi - lo);
            fa = f(a);
        } else {
            lo = a;
            (a, fa) = (b, fb);
            b = lo + g * (hi - lo);
            fb = f(b);
        }
    }
    (lo + hi) / 2.0
}

fn ring_q_bogaerts() -> Outcome {
    let l0 = um(1.55);
    let device = StripRing::all_pass().with_guide(Dispersion::new(l0, 2.4, 4.30));
    let values = Values::new(&device, &[("round_trip_loss", 0.075)]).expect(OK);
    let ring = device.all_pass_ring(&values, A).expect(OK);
    // critical coupling: r = a
    let q = |l: f64| {
        let a = 10f64.powf(-(2.7 * l * 1e-4 + 0.075) / 20.0);
        ring.q_factor(l0, &[l, 1.0 - a * a]).unwrap_or(f64::NAN)
    };
    outcome(round(peak(q, 1e3, 3e4) / 1e3, 3), 10.0, 1.0)
}

fn wire_ng_dwivedi() -> Outcome {
    let guide = StripWaveguide::new();
    let v = Values::new(
        &guide,
        &[("width", 0.602), ("thickness", 0.211), ("span", 0.06)],
    )
    .expect(OK);
    let built = guide.component(&v, D3).expect(OK);
    outcome(built.figure("n_g").expect(OK).value, 4.0453, 0.032)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cheap_cases_pass() {
        for o in [
            library_components(),
            ring_closed_forms(),
            coupler_closed_forms(),
            derivatives(),
            ring_q_bogaerts(),
        ] {
            assert!(o.passed(), "{o:?}");
        }
        // the round trip's peak, as the ring_q_factor example prints it
        assert_eq!(ring_q_bogaerts().measured, 9.353);
    }
}
