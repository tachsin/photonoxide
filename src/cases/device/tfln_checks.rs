//! The TFLN Mach–Zehnder modulator device's validation cases ([`super::TflnModulator`]): the
//! analytic model against the 2D one over its stated range, the published modulators through the
//! device (Zhang et al.'s and Wang et al.'s Vπ·L, Zhang et al.'s response at 250 GHz, Kharel et
//! al.'s electrode's loss), the 2D model against the measured devices, its grid's convergence,
//! and every fidelity run as a job to the bit.

use std::sync::OnceLock;

use crate::circuit::Fidelity;
use crate::convergence::Study;
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

use super::{Built, Device, Spec, TflnModulator, Values};

const OK: &str = "a valid design";

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The device's values with `changes`.
fn values(changes: &[(&str, f64)]) -> Values {
    Values::new(&TflnModulator::new(), changes).expect(OK)
}

/// The figure `symbol` of `built`.
fn figure(built: &Built, symbol: &str) -> f64 {
    built.figure(symbol).expect(OK).value
}

/// The designs the analytic model is checked on: the defaults (Zhang et al.'s) and each
/// parameter of its validity at its ends, at 50 GHz.
const DESIGNS: [&[(&str, f64)]; 7] = [
    &[],
    &[("gap", 3.5)],
    &[("gap", 8.0)],
    &[("width", 0.9)],
    &[("width", 2.0)],
    &[("film", 0.5)],
    &[("film", 0.8)],
];

/// The analytic figures over the 2D model's at [`DESIGNS`]: Vπ·L's; the line's RF index's and
/// impedance's at 50 GHz; and the 3 and 6 dB bandwidths' (referred to 1 GHz, whatever the
/// frequency); each ratio's farthest from 1.
fn analytic_against_2d() -> &'static [f64; 3] {
    static CELL: OnceLock<[f64; 3]> = OnceLock::new();
    CELL.get_or_init(|| {
        let d = TflnModulator::new();
        let farther = |m: f64, e: f64| if e.abs() > m.abs() { e } else { m };
        let (mut vpi_l, mut line, mut bandwidth) = (0.0f64, 0.0f64, 0.0f64);
        for changes in DESIGNS {
            let v = values(changes).with(&d, "frequency", 50.0).expect(OK);
            let (a, b) = (
                d.component(&v, Fidelity::Analytic).expect(OK),
                d.component(&v, Fidelity::TwoD).expect(OK),
            );
            let ratio = |s: &str| figure(&a, s) / figure(&b, s);
            vpi_l = farther(vpi_l, ratio("VpiL") - 1.0);
            line = farther(line, ratio("n_rf") - 1.0);
            line = farther(line, ratio("Z0") - 1.0);
            bandwidth = farther(bandwidth, ratio("f_3dB") - 1.0);
            bandwidth = farther(bandwidth, ratio("f_6dB") - 1.0);
        }
        [1.0 + vpi_l, 1.0 + line, 1.0 + bandwidth]
    })
}

/// The published devices the cases take through the 2D model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Made {
    /// Zhang et al.'s 5.8 mm modulator (the device's defaults).
    ZhangShort,
    /// Their 10.8 mm one.
    ZhangLong,
    /// Wang et al.'s 2 mm one, its signal 10 µm wide and its grounds 15 (P6's case's) and its
    /// gold 41 MS/m.
    Wang,
    /// Kharel et al.'s regular electrode, gold of 41 MS/m.
    Kharel,
}

impl Made {
    fn values(self) -> Values {
        match self {
            Made::ZhangShort => values(&[]),
            Made::ZhangLong => values(&[("length", 10800.0)]),
            Made::Wang => values(&[
                ("film", 0.7),
                ("width", 0.9),
                ("gap", 3.5),
                ("signal", 10.0),
                ("ground", 15.0),
                ("metal", 0.315),
                ("conductivity", 41.0),
                ("cladding", 1.5),
                ("cladding_permittivity", 3.9),
                ("length", 2000.0),
            ]),
            Made::Kharel => values(&[
                ("slab", 0.25),
                ("width", 2.0),
                ("signal", 30.0),
                ("metal", 0.8),
                ("conductivity", 41.0),
                ("cladding", 1.0),
                ("cladding_permittivity", 3.9),
                ("length", 10000.0),
            ]),
        }
    }
}

/// The 2D model of `made` at `frequency` (GHz) on the device's own grids (r = 1) and twice as
/// fine (r = 2).
fn two_d(made: Made, frequency: f64) -> [Built; 2] {
    let v = made
        .values()
        .with(&TflnModulator::new(), "frequency", frequency)
        .expect(OK);
    [1.0, 2.0].map(|r| {
        TflnModulator::new()
            .with_refinement(r)
            .expect(OK)
            .component(&v, Fidelity::TwoD)
            .expect(OK)
    })
}

/// The 2D figures of `made` the cases take, on the device's own grids and twice as fine: Vπ·L,
/// the group index, the RF index and Z₀ at 1 GHz, the conductor loss's coefficient, the 3 dB
/// bandwidth, and the response at 250 GHz over 1 GHz.
fn numbers(made: Made) -> &'static [[f64; 7]; 2] {
    static CELLS: [OnceLock<[[f64; 7]; 2]>; 4] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    CELLS[made as usize].get_or_init(|| {
        let (at_1, at_250) = (two_d(made, 1.0), two_d(made, 250.0));
        [0, 1].map(|k| {
            let (b, c) = (&at_1[k], &at_250[k]);
            [
                figure(b, "VpiL"),
                figure(b, "n_g"),
                figure(b, "n_rf"),
                figure(b, "Z0"),
                figure(b, "alpha_c"),
                figure(b, "f_3dB"),
                10f64.powf((figure(c, "m_dB") - figure(b, "m_dB")) / 20.0),
            ]
        })
    })
}

/// Number `k` of [`numbers`] gated by the two grids' convergence to `tolerance` of the finer
/// one: the finer one's value, or NaN (the case fails) if refused.
fn gated(made: Made, k: usize, tolerance: f64) -> f64 {
    let [coarse, fine] = numbers(made);
    Study::new("a modulator's number")
        .on(0.02, coarse[k])
        .and_then(|s| s.on(0.01, fine[k]))
        .and_then(|s| s.check(tolerance * fine[k].abs()))
        .and_then(|c| c.gated())
        .unwrap_or(f64::NAN)
}

/// The measured device's figure `symbol` at `made`'s design.
fn measured(made: Made, symbol: &str) -> f64 {
    let d = TflnModulator::new();
    let v = made.values();
    d.measurements()
        .iter()
        .find(|m| m.matches(&v))
        .and_then(|m| m.figure(symbol))
        .expect("a measured figure")
        .value
}

/// The 2D Vπ·L at Zhang et al.'s and Wang et al.'s designs, gated (1%), over the measured
/// fidelity's: the ratio farthest from 1.
fn against_measured() -> f64 {
    [Made::ZhangShort, Made::Wang]
        .iter()
        .map(|&m| gated(m, 0, 0.01) / measured(m, "VpiL"))
        .fold(1.0f64, |w, r| {
            if (r - 1.0).abs() > (w - 1.0).abs() {
                r
            } else {
                w
            }
        })
}

/// The 2D response at 250 GHz over 1 GHz of Zhang et al.'s two modulators, gated (1%), against
/// their measured RF Vπ's: (Vπ·L at 1 GHz / L) / V_π(250 GHz); the larger relative difference,
/// with its sign.
fn zhang_response() -> f64 {
    [(Made::ZhangShort, 0.58), (Made::ZhangLong, 1.08)]
        .iter()
        .map(|&(m, cm)| {
            let theirs = measured(m, "VpiL_1GHz") / cm / measured(m, "Vpi_250GHz");
            gated(m, 6, 0.01) / theirs - 1.0
        })
        .fold(0.0f64, |w, e| if e.abs() > w.abs() { e } else { w })
}

/// The device's own 2D grid's error at Zhang et al.'s design: each of [`numbers`]' relative
/// difference from the grid twice as fine, the largest.
fn grid_error() -> f64 {
    let [coarse, fine] = numbers(Made::ZhangShort);
    coarse
        .iter()
        .zip(fine)
        .map(|(c, f)| (c / f - 1.0).abs())
        .fold(0.0, f64::max)
}

/// The device at each of its fidelities as a job ([`crate::job::from_device`]), run in memory,
/// at DC and (but the measured fidelity, which is at DC) at 40 GHz: the numbers of its record
/// (figures and S-parameters at five wavelengths) whose bits differ from
/// [`Device::component`]'s.
fn jobs() -> f64 {
    use crate::job::{Compute, Event, execute, from_device};
    use crate::run::{Run, Stop};
    let d = TflnModulator::new();
    let mut differ = 0usize;
    for (f, frequency) in [
        (Fidelity::Analytic, "0"),
        (Fidelity::Analytic, "40"),
        (Fidelity::TwoD, "0"),
        (Fidelity::TwoD, "40"),
        (Fidelity::Measured, "0"),
    ] {
        let mut spec = Spec::new(d.kind());
        spec.fidelity = f;
        spec.parameters = vec![
            ("voltage".into(), "2.5".into()),
            ("bias".into(), "0.3".into()),
            ("frequency".into(), frequency.into()),
        ];
        let compute = Compute {
            band: Some((1.53, 1.57, 5)),
            ..Compute::model()
        };
        let job = from_device("modulator", &spec, &compute).expect(OK);
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
                            let x = want[(q, p)];
                            differ += usize::from(v[0].to_bits() != x.re.to_bits());
                            differ += usize::from(v[1].to_bits() != x.im.to_bits());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    differ as f64
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/tfln-analytic-vpil",
            title: r"TFLN Mach-Zehnder modulator, analytic against 2D: Vπ·L push-pull of the closed forms (the field $2V/(\pi g)$ at the centre of a coplanar gap taken across the film's TE slab mode, its confinement in the film, the film's $r_{33}$) against the 2D model's (the static solver's field graded from 20 nm, the full-vector mode on 20 nm cells, by first-order perturbation), at seven designs: the defaults (Zhang et al.'s), the gap 3.5 and 8 µm, the ribs 0.9 and 2.0 µm wide, the film 0.5 and 0.8 µm thick; the ratio farthest from 1, analytic over 2D (shown), the analytic fidelity's stated error",
            tier: Tier::Analytic,
            source: r"the same cross-section with the field uniform and the mode a slab's: the potential $(V/\pi)\arg z$ between coplanar half-planes in a uniform medium, its field at the gap's centre; the slab's TE mode in closed form (its confinement exact, device::tfln's unit test)",
            run: || outcome(analytic_against_2d()[0], 1.0, 0.15),
        },
        Case {
            id: "device/tfln-analytic-line",
            title: r"TFLN Mach-Zehnder modulator's electrode, analytic against 2D: the RF index and Re $Z_0$ at 50 GHz of the closed-form coplanar line (each layer's partial capacitance by conformal mapping, lithium niobate's slab scaled to an isotropic layer, the strips' thickness as plates across the gaps, Holloway and Kuester's internal impedance) against the quasi-TEM line of the static solver (graded from 4 nm), at the same seven designs: the ratio of either farthest from 1, analytic over 2D (shown); the bandwidths, whose walk-off the index's error sets, are device/tfln-analytic-bandwidth",
            tier: Tier::Analytic,
            source: r"G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984), doi:10.1049/el:19840120, Eqs. 2-3, layer on layer (exact for one layer, device::tfln's unit test); the layers' superposition overstates the silicon under the buried oxide, which raises the index: the analytic line's stated 10%",
            run: || outcome(analytic_against_2d()[1], 1.0, 0.10),
        },
        Case {
            id: "device/tfln-analytic-bandwidth",
            title: r"TFLN Mach-Zehnder modulator's bandwidths, analytic against 2D: the 3 and 6 dB electro-optic bandwidths from 1 GHz of the closed forms (the conformal-mapping line, Holloway and Kuester's conductor loss, the slab's group index, Ghione's response; a 50 Ω source and load) against the 2D model's (the quasi-TEM line of the static solver graded from 4 nm, the mode's group index), at the same seven designs: the ratio of either farthest from 1, analytic over 2D (shown), the analytic fidelity's stated error in its bandwidths",
            tier: Tier::Analytic,
            source: r"the analytic line's RF index runs 8 to 9 % high (device/tfln-analytic-line), which about doubles its walk-off from the light, $\lvert n_{RF} - n_g \rvert$, 0.46 against 0.21 at the defaults: the analytic bandwidths are low by a factor, 0.32 to 0.41 of the 2D model's at 3 dB and 0.23 to 0.30 at 6 dB. The stated error is that spread, 77 % at worst, rounded up to the next 5 %, not a factor that corrects it",
            run: || {
                outcome(
                    analytic_against_2d()[2],
                    1.0,
                    TflnModulator::ANALYTIC_BANDWIDTH_ERROR / 100.0,
                )
            },
        },
        Case {
            id: "device/tfln-zhang-vpil",
            title: r"TFLN Mach-Zehnder modulator, published, through the device: Zhang et al.'s 5.8 mm modulator (the device's defaults: a 600 nm film etched 300 nm to ribs 1.2 µm wide, 5 µm gaps, 0.6 µm of PECVD oxide and air; the walls at 70°, not given), Vπ·L push-pull of its 2D model on its own grids (20 nm) and twice as fine, gated (1%) (V·cm)",
            tier: Tier::Published,
            source: r"Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518, Section 2: 2.3 V·cm at DC (2.67 at 1 GHz); the tolerance as pockels/zhang-2022's, 0.15 V·cm",
            run: || outcome(gated(Made::ZhangShort, 0, 0.01), 2.3, 0.15),
        },
        Case {
            id: "device/tfln-wang-vpil",
            title: r"TFLN Mach-Zehnder modulator, published, through the device: Wang et al.'s 2 mm modulator (a 700 nm film etched 400 nm to ribs 0.9 µm wide, a 3.5 µm gap, 0.315 µm of metal, 1.5 µm of silica over the slab; the walls at 70°, not given), Vπ·L push-pull of its 2D model gated the same way (V·cm)",
            tier: Tier::Published,
            source: r"C. Wang et al., Opt. Express 26, 1547 (2018), doi:10.1364/OE.26.001547, Section 3.1: Vπ 9 V over 2 mm, 1.8 V·cm; the tolerance as pockels/wang-2018's, 0.15 V·cm",
            run: || outcome(gated(Made::Wang, 0, 0.01), 1.8, 0.15),
        },
        Case {
            id: "device/tfln-measured",
            title: r"TFLN Mach-Zehnder modulator, 2D against measured: Vπ·L of the 2D model at Zhang et al.'s 5.8 mm and Wang et al.'s 2 mm designs, gated (1%), over the measured fidelity's (the papers' measurements): the ratio farthest from 1 (shown), the 2D fidelity's stated error",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, doi:10.1364/PRJ.468518, Section 2: 2.3 V·cm at DC; Wang et al. 2018, doi:10.1364/OE.26.001547, Section 3.1: 1.8 V·cm. The 2D model's $r_{33}$ is the film's clamped one (100 MHz to 330 GHz): against Zhang et al.'s 2.67 V·cm at 1 GHz it is 11% low",
            run: || outcome(against_measured(), 1.0, 0.10),
        },
        Case {
            id: "device/tfln-zhang-response",
            title: r"TFLN Mach-Zehnder modulator, published, through the device: the electro-optic response at 250 GHz over 1 GHz of Zhang et al.'s 5.8 and 10.8 mm modulators by the 2D model (the quasi-TEM line graded from 4 nm and twice as fine, Zhang et al.'s loss tangents, copper of 58 MS/m, the light's group index from the mode; a 50 Ω source and load), gated (1%), against their measured (Vπ·L at 1 GHz over L) over the RF Vπ at 250 GHz: the larger relative difference (shown)",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, doi:10.1364/PRJ.468518, Section 3.C and Figs. 5(a)-(b): 8.3 and 7.3 V at 250 GHz, 2.67 V·cm at 1 GHz; the tolerance as line/zhang-2022-rf-vpi-short's (their points' spread, about 15%, and the line's loss): 20%",
            run: || outcome(zhang_response(), 0.0, 0.2),
        },
        Case {
            id: "device/tfln-kharel-loss",
            title: r"TFLN Mach-Zehnder modulator's electrode, published, through the device: the conductor loss at 50 GHz over its square root of Kharel et al.'s regular electrode on silicon (a 30 µm signal, a 5 µm gap, 800 nm of gold of 41 MS/m on a 600 nm film etched 350 nm to a rib 2 µm wide, 1 µm of oxide over the slab; the grounds 100 µm and the walls 70°, not given) through the device's 2D line (Holloway and Kuester's method on the static solver's strips, graded from 4 nm and twice as fine), gated (1%), against their simulated one (dB cm⁻¹ GHz⁻½)",
            tier: Tier::Published,
            source: r"P. Kharel et al., Optica 8, 357 (2021), doi:10.1364/OPTICA.416155, Table 1: 0.75 dB cm⁻¹ GHz⁻½ at the 5 µm gap; the tolerance as line/kharel-2021-loss's: the gold's conductivity (±5%), the unstated geometry (±3%) and Holloway and Kuester's own 5-10% (their Section V): 12%. Their 10 mm electrode measured 0.69 (Fig. 2(a)), 8% below their simulation; the device's line is 15% above it, which isn't a case",
            run: || outcome(gated(Made::Kharel, 4, 0.01), 0.75, 0.12 * 0.75),
        },
        Case {
            id: "device/tfln-2d-grid",
            title: r"TFLN Mach-Zehnder modulator, the 2D model's grid: at Zhang et al.'s 5.8 mm design, Vπ·L, the group index, the RF index and Re $Z_0$ at 1 GHz, the conductor loss's coefficient, the 3 dB bandwidth and the response at 250 GHz on the device's own grids (the Pockels field graded from 20 nm, the mode on 20 nm cells, the line graded from 4 nm) against the grids twice as fine: the relative difference farthest from 0 (shown), the device's 2D figures' grid error",
            tier: Tier::Analytic,
            source: r"the static solver's and the mode's convergence (pockels/static-order, pockels/optical-convergence, line/kharel-2021-impedance's grid study): halving the cells moves each by less than 1%",
            run: || outcome(grid_error(), 0.0, 0.01),
        },
        Case {
            id: "device/tfln-jobs",
            title: r"TFLN Mach-Zehnder modulator as jobs: the device at each fidelity (analytic, 2D and measured; 2.5 V, a bias of 0.3 rad, at DC and, but the measured, at 40 GHz) as the job `job::from_device` makes, run, its figures and S-parameters from 1.53 to 1.57 µm (5 wavelengths) against `Device::component`'s: the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same computation: a spectrum job records what the device's component gives at its design's fidelity, to the bit",
            run: || outcome(jobs(), 0.0, 0.0),
        },
    ]
}
