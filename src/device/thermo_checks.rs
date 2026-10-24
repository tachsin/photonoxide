//! The thermo-optic phase shifter devices' validation cases ([`super::ThermoShifter`]): each
//! lower fidelity against the next one up over its stated range, Jacques et al.'s published
//! heaters through the devices (the phase's own cases, `shifter/jacques-*`, reproduced through
//! them), the 2D models against the measured devices, the 2D grid's convergence, and every
//! fidelity run as a job to the bit.

use std::sync::OnceLock;

use crate::circuit::Fidelity;
use crate::convergence::Study;
use crate::thermal::shifter_checks::{Heater, NPP, TIN};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

use super::{Device, Spec, ThermoShifter, Values};

const OK: &str = "a valid design";

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// `d`'s values with `changes`.
fn values(d: &ThermoShifter, changes: &[(&str, f64)]) -> Values {
    Values::new(d, changes).expect(OK)
}

/// The 2D model's P_π (mW) of `d` at `v`, its grids refined by `r`: the cross-section's
/// [`crate::thermal::PhaseShifter::efficiency`], as the 2D fidelity's figure takes it.
fn p_pi_2d(d: &ThermoShifter, v: &Values, r: f64) -> f64 {
    d.clone()
        .with_refinement(r)
        .expect(OK)
        .shifter(v)
        .expect(OK)
        .efficiency()
        .expect(OK)
        .p_pi(0, 0)
        * 1e3
}

/// The figure `symbol` of `d`'s model at `fidelity` and `v`.
fn figure(d: &ThermoShifter, v: &Values, fidelity: Fidelity, symbol: &str) -> f64 {
    d.component(v, fidelity)
        .expect(OK)
        .figure(symbol)
        .expect(OK)
        .value
}

/// `value(r)` on the device's own grids (r = 1) and twice as fine, gated by their convergence to
/// `tolerance` of the finer one: the finer one's value, or NaN (the case fails) if refused.
fn gated(quantity: &str, value: impl Fn(f64) -> f64, tolerance: f64) -> f64 {
    let (coarse, fine) = (value(1.0), value(2.0));
    Study::new(quantity)
        .on(0.02, coarse)
        .and_then(|s| s.on(0.01, fine))
        .and_then(|s| s.check(tolerance * fine.abs()))
        .and_then(|c| c.gated())
        .unwrap_or(f64::NAN)
}

/// The designs the analytic models are checked on: each parameter of their validity at its ends,
/// the rest at the defaults (the metal heater's at Jacques et al.'s simulated 2 µm).
fn designs(doped: bool) -> Vec<Vec<(&'static str, f64)>> {
    if doped {
        vec![
            vec![],
            vec![("buffer", 0.4)],
            vec![("buffer", 1.6)],
            vec![("heater_width", 0.5)],
            vec![("heater_width", 2.0)],
            vec![("width", 0.4)],
            vec![("width", 0.6)],
        ]
    } else {
        vec![
            vec![],
            vec![("heater_width", 1.0)],
            vec![("heater_width", 10.0)],
            vec![("heater_width", 2.0), ("height", 1.0)],
            vec![("heater_width", 2.0), ("height", 3.0)],
            vec![("heater_width", 2.0), ("width", 0.4)],
            vec![("heater_width", 2.0), ("width", 0.6)],
        ]
    }
}

/// The analytic P_π's largest relative difference from the 2D model's over [`designs`], with
/// its sign.
fn analytic_against_2d(doped: bool) -> f64 {
    let d = if doped {
        ThermoShifter::doped()
    } else {
        ThermoShifter::metal()
    };
    designs(doped)
        .iter()
        .map(|changes| {
            let v = values(&d, changes);
            figure(&d, &v, Fidelity::Analytic, "P_pi") / p_pi_2d(&d, &v, 1.0) - 1.0
        })
        .fold(0.0f64, |m, e| if e.abs() > m.abs() { e } else { m })
}

/// The analytic 1/e time's and −3 dB frequency's largest relative difference from the 2D
/// model's at the default design and the ends of the heater's width, with its sign.
fn analytic_time(doped: bool) -> f64 {
    let d = if doped {
        ThermoShifter::doped()
    } else {
        ThermoShifter::metal()
    };
    let widths: [f64; 3] = if doped {
        [1.0, 0.5, 2.0]
    } else {
        [7.5, 1.0, 10.0]
    };
    let mut worst = 0.0f64;
    for w in widths {
        let v = values(&d, &[("heater_width", w)]);
        let (a, b) = (
            d.component(&v, Fidelity::Analytic).expect(OK),
            d.component(&v, Fidelity::TwoD).expect(OK),
        );
        for s in ["tau", "f_3dB"] {
            let e = a.figure(s).expect(OK).value / b.figure(s).expect(OK).value - 1.0;
            if e.abs() > worst.abs() {
                worst = e;
            }
        }
    }
    worst
}

/// The metal device at one of the phase's TiN variants: 2.0 µm wide, its thickness, its 2 µm
/// above the strip's top (a height of 2.0) or the buried oxide's (1.78).
fn tin_design(heater: Heater) -> Values {
    let Heater::Tin {
        thickness,
        from_strip_top,
    } = heater
    else {
        unreachable!("a TiN variant")
    };
    let height = if from_strip_top { 2.0 } else { 1.78 };
    values(
        &ThermoShifter::metal(),
        &[
            ("heater_width", 2.0),
            ("heater_thickness", thickness),
            ("height", height),
            ("cover", 1.0),
        ],
    )
}

/// The doped device at the phase's N++ heater (the device's defaults).
fn npp_design() -> Values {
    let Heater::Npp { top } = NPP else {
        unreachable!("the N++ heater")
    };
    values(&ThermoShifter::doped(), &[("cladding", top)])
}

/// The midpoint of the smallest and largest of `values`.
fn midpoint(values: &[f64]) -> f64 {
    let (low, high) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    0.5 * (low + high)
}

/// The 2D model's P_π (mW) at the measured device's design, on its own grids and twice as fine.
fn fabricated(doped: bool) -> &'static [f64; 2] {
    static METAL: OnceLock<[f64; 2]> = OnceLock::new();
    static DOPED: OnceLock<[f64; 2]> = OnceLock::new();
    let cell = if doped { &DOPED } else { &METAL };
    cell.get_or_init(|| {
        let d = if doped {
            ThermoShifter::doped()
        } else {
            ThermoShifter::metal()
        };
        let v = Values::defaults(&d);
        [p_pi_2d(&d, &v, 1.0), p_pi_2d(&d, &v, 2.0)]
    })
}

/// The 2D model's P_π at the fabricated design, gated by its grids (1%), against the measured
/// fidelity's: the relative difference.
fn against_measured(doped: bool) -> f64 {
    let d = if doped {
        ThermoShifter::doped()
    } else {
        ThermoShifter::metal()
    };
    let [coarse, fine] = *fabricated(doped);
    let gated = gated("P_pi (mW)", |r| if r == 1.0 { coarse } else { fine }, 0.01);
    let measured = figure(&d, &Values::defaults(&d), Fidelity::Measured, "P_pi");
    gated / measured - 1.0
}

/// The device's own 2D grid's error in P_π at both fabricated designs: its relative difference
/// from the grid twice as fine, the larger.
fn grid_error() -> f64 {
    [false, true]
        .iter()
        .map(|&doped| {
            let [coarse, fine] = *fabricated(doped);
            (coarse / fine - 1.0).abs()
        })
        .fold(0.0, f64::max)
}

/// Each device at each of its fidelities as a job ([`crate::job::from_device`]), run in memory:
/// the numbers of its record (figures and S-parameters at five wavelengths) whose bits differ
/// from [`Device::component`]'s.
fn jobs() -> f64 {
    use crate::job::{Compute, Event, execute, from_device};
    use crate::run::{Run, Stop};
    let mut differ = 0usize;
    for d in [ThermoShifter::metal(), ThermoShifter::doped()] {
        for f in [Fidelity::Analytic, Fidelity::TwoD, Fidelity::Measured] {
            let mut spec = Spec::new(d.kind());
            spec.fidelity = f;
            spec.parameters = vec![("power".into(), "12.5".into())];
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
    }
    differ as f64
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "device/thermo-analytic-metal",
            title: r"Thermo-optic phase shifter (TiN over a 500 × 220 nm strip), analytic against 2D: $P_\pi$ of the closed forms (the heater as a sheet in the oxide between the isothermal substrate and an insulated top, its eigenfunctions in closed form across; Marcatili's $dn_\mathrm{eff}/dT$) against the 2D model's (the static solver in Jacques et al.'s 40 × 18 µm region graded from 20 nm, the mode on 10 nm), at seven designs: the defaults (Jacques et al.'s fabricated 7.5 µm), the TiN 1 and 10 µm wide, and at 2 µm of TiN its height 1 and 3 µm and the strip 0.4 and 0.6 µm wide; the ratio farthest from 1, analytic over 2D (shown), the analytic fidelity's stated error",
            tier: Tier::Analytic,
            source: r"the same heat problem thermally all oxide: the guide's silicon and the heater's conductivity left out, the substrate's 12.7 µm of silicon as 0.12 µm of oxide; Marcatili's rectangle (doi:10.1002/j.1538-7305.1969.tb01166.x) for the mode",
            run: || outcome(1.0 + analytic_against_2d(false), 1.0, 0.15),
        },
        Case {
            id: "device/thermo-analytic-doped",
            title: r"Doped thermo-optic phase shifter (two N++ strips in a 90 nm slab beside a 500 × 220 nm rib), analytic against 2D: $P_\pi$ of the closed forms (the strips as sheets in the oxide) against the 2D model's, at seven designs: the defaults (Jacques et al.'s), the buffer 0.4 and 1.6 µm, the strips 0.5 and 2.0 µm wide, the rib 0.4 and 0.6 µm wide; the ratio farthest from 1, analytic over 2D (shown), the analytic fidelity's stated error",
            tier: Tier::Analytic,
            source: r"the same heat problem thermally all oxide: the slab's silicon (55 W/(m·K), 90 nm), which carries the heat across to the rib, left out; Marcatili's rectangle of the rib alone for the mode",
            run: || outcome(1.0 + analytic_against_2d(true), 1.0, 0.15),
        },
        Case {
            id: "device/thermo-analytic-time-metal",
            title: r"Thermo-optic phase shifter in time, analytic against 2D: the 1/e time of the rise turned on and the −3 dB frequency of the closed forms (the oxide slab's eigenfunctions, Simpson's rule in $\ln\tau$ for the step, the frequency series in closed form) against the 2D model's (BDF2 steps of 0.2 µs, its frequency response), the TiN 7.5, 1 and 10 µm wide: the ratio of either farthest from 1, analytic over 2D (shown)",
            tier: Tier::Analytic,
            source: r"the same heat problem thermally all oxide, its heat capacity the oxide's (Jacques et al.'s Table 3: 2203 kg/m³, 709 J/(kg·K))",
            run: || outcome(1.0 + analytic_time(false), 1.0, 0.05),
        },
        Case {
            id: "device/thermo-analytic-time-doped",
            title: r"Doped thermo-optic phase shifter in time, analytic against 2D: the same, the N++ strips 1.0, 0.5 and 2.0 µm wide (BDF2 steps of 0.1 µs): the ratio of either farthest from 1, analytic over 2D (shown); the oxide's sheets leave out the slab's silicon, which carries the heat across to the rib at once, so the closed form is slower",
            tier: Tier::Analytic,
            source: r"the same heat problem thermally all oxide; the tolerance is what the closed form states for a doped heater's time",
            run: || outcome(1.0 + analytic_time(true), 1.0, 0.6),
        },
        Case {
            id: "device/thermo-jacques-tin",
            title: r"Thermo-optic phase shifter, published, through the device: Jacques et al.'s 2.0 µm TiN heater as the metal device (`shifter/jacques-tin`'s cross-section, to the bit), $P_\pi$ of its 2D model on its own grids and twice as fine, gated (1%); the four variants of what the paper doesn't state (TiN 100 or 200 nm; its 2 µm from the strip's top or the buried oxide's, a height of 2.0 or 1.78 µm), their midpoint shown (mW)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the simulated $P_\pi$ of the 2.0 µm TiN design, 23.9 mW (Section 2.2, Fig. 1(e)); 10% for what the paper doesn't state",
            run: || {
                let d = ThermoShifter::metal();
                let pi: Vec<f64> = TIN
                    .iter()
                    .map(|&h| {
                        let v = tin_design(h);
                        gated("P_pi (mW)", |r| p_pi_2d(&d, &v, r), 0.01)
                    })
                    .collect();
                outcome(midpoint(&pi), 23.9, 0.1 * 23.9)
            },
        },
        Case {
            id: "device/thermo-jacques-npp",
            title: r"Doped thermo-optic phase shifter, published, through the device: Jacques et al.'s N++ heater (the doped device's defaults: `shifter/jacques-npp`'s cross-section, to the bit), $P_\pi$ of its 2D model gated the same way (mW)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the simulated $P_\pi$ of the N++ design, 25.5 mW (Section 2.2, Fig. 1(e)); 10% for what the paper doesn't state (the cladding above)",
            run: || {
                let (d, v) = (ThermoShifter::doped(), npp_design());
                outcome(gated("P_pi (mW)", |r| p_pi_2d(&d, &v, r), 0.01), 25.5, 2.55)
            },
        },
        Case {
            id: "device/thermo-tin-measured",
            title: r"Thermo-optic phase shifter, 2D against measured: $P_\pi$ of the 2D model at Jacques et al.'s fabricated TiN heater (7.5 µm wide, 2 µm above the strip, 320 µm long; the metal device's defaults), gated by its grids (1%), against the measured fidelity's (the paper's measurement): their ratio (shown), the 2D fidelity's stated error",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456, measured: 29.2 mW, the MZI's arms 200 µm apart (Section 3.1, Fig. 8); their own simulation 3.0 mW above it (Section 3.1)",
            run: || outcome(1.0 + against_measured(false), 1.0, 0.15),
        },
        Case {
            id: "device/thermo-npp-measured",
            title: r"Doped thermo-optic phase shifter, 2D against measured: $P_\pi$ of the 2D model at Jacques et al.'s fabricated N++ heater (the doped device's defaults), gated the same way, against the measured fidelity's: their ratio (shown), the 2D fidelity's stated error",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456, measured: 22.8 mW, the arms 200 µm apart (Section 3.1, Fig. 8); their own simulation 2.7 mW above it (Section 3.1)",
            run: || outcome(1.0 + against_measured(true), 1.0, 0.15),
        },
        Case {
            id: "device/thermo-2d-grid",
            title: r"Thermo-optic phase shifters, the 2D model's grid: $P_\pi$ at both fabricated designs on the devices' own grids (the heat graded from 20 nm to 0.5 µm by 1.2, the mode on 10 nm) against the grids twice as fine: the ratio farther from 1 (shown), the devices' 2D figures' grid error",
            tier: Tier::Analytic,
            source: r"the static solver's second order in the grid (static/line-source-order) and the mode's (thermo/slab-vector-order): halving the cells moves $P_\pi$ by less than 1%",
            run: || outcome(1.0 + grid_error(), 1.0, 0.01),
        },
        Case {
            id: "device/thermo-jobs",
            title: r"Thermo-optic phase shifters as jobs: both devices at each fidelity (analytic, 2D and measured, 12.5 mW of heater power) as the job `job::from_device` makes, run, its figures and S-parameters from 1.53 to 1.57 µm (5 wavelengths) against `Device::component`'s: the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same computation: a spectrum job records what the device's component gives at its design's fidelity, to the bit",
            run: || outcome(jobs(), 0.0, 0.0),
        },
    ]
}
