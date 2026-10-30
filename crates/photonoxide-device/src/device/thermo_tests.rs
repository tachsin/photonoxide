//! The thermo-optic phase shifter devices.

use std::f64::consts::PI;

use crate::circuit::Fidelity;
use crate::thermal::shifter_checks::{Heater, NPP, TIN, jacques, jacques_grading};
use crate::units::Wavelength;

use super::{Device, Measurement, Spec, ThermoShifter, Values};

/// The metal device at one of the phase's TiN variants.
fn tin(heater: Heater) -> Values {
    let Heater::Tin {
        thickness,
        from_strip_top,
    } = heater
    else {
        unreachable!()
    };
    Values::new(
        &ThermoShifter::metal(),
        &[
            ("heater_width", 2.0),
            ("heater_thickness", thickness),
            ("height", if from_strip_top { 2.0 } else { 1.78 }),
        ],
    )
    .unwrap()
}

#[test]
fn jacques_heaters_through_the_devices_are_the_phases_to_the_bit() {
    // the phase's own cross-sections (shifter_checks::jacques), and the devices' at their
    // designs: the same heat problem and mode, so the same slope, bit for bit
    let s = |p: &crate::thermal::PhaseShifter| p.efficiency().unwrap().slopes[0][0];
    let metal = ThermoShifter::metal();
    for h in TIN {
        let phase = jacques(h, &jacques_grading(1.0), 0.01);
        let device = metal.shifter(&tin(h)).unwrap();
        assert_eq!(s(&device).to_bits(), s(&phase).to_bits(), "{h:?}");
    }
    let doped = ThermoShifter::doped();
    let phase = jacques(NPP, &jacques_grading(1.0), 0.01);
    let device = doped.shifter(&Values::defaults(&doped)).unwrap();
    assert_eq!(s(&device).to_bits(), s(&phase).to_bits());
}

#[test]
fn the_phase_follows_the_power_and_its_derivative_is_exact() {
    let d = ThermoShifter::metal();
    let v = Values::new(&d, &[("power", 10.0), ("loss", 2.0)]).unwrap();
    let b = d.component(&v, Fidelity::Analytic).unwrap();
    let p_pi = b.figure("P_pi").unwrap().value;
    // φ = π P/P_π, and at P_π the guide's phase has turned by π
    assert!((b.values[0] - PI * 10.0 / p_pi).abs() < 1e-15);
    let at_pi = Values::new(&d, &[("power", p_pi)]).unwrap();
    let s = d
        .component(&at_pi, Fidelity::Analytic)
        .unwrap()
        .s_matrix(Wavelength::um(1.55).unwrap())
        .unwrap();
    assert!((s[(1, 0)].re + 1.0).abs() < 1e-12 && s[(1, 0)].im.abs() < 1e-12);
    // 2 dB/cm over 320 µm
    let lambda = Wavelength::um(1.55).unwrap();
    let t = b.s_matrix(lambda).unwrap()[(1, 0)].norm();
    assert!((t - 10f64.powf(-2.0 * 0.032 / 20.0)).abs() < 1e-14);
    // dS/dP through the component's derivatives and the device's against a difference
    let ds = b.component.derivatives(lambda, &b.values).unwrap().unwrap();
    let k = d
        .parameters()
        .iter()
        .position(|p| p.name == "power")
        .unwrap();
    let dp = b.derivatives[k].as_ref().unwrap();
    let exact = ds[0][(1, 0)] * dp[0] + ds[1][(1, 0)] * dp[1];
    let h = 1e-4;
    let at = |p: f64| {
        let v = Values::new(&d, &[("power", p), ("loss", 2.0)]).unwrap();
        d.component(&v, Fidelity::Analytic)
            .unwrap()
            .s_matrix(lambda)
            .unwrap()[(1, 0)]
    };
    let numeric = (at(10.0 + h) - at(10.0 - h)) / (2.0 * h);
    assert!(
        (exact - numeric).norm() < 1e-9 * exact.norm(),
        "{exact} {numeric}"
    );
}

#[test]
fn the_measured_fidelity_is_its_designs_only() {
    let d = ThermoShifter::doped();
    let measured = d
        .component(&Values::defaults(&d), Fidelity::Measured)
        .unwrap();
    assert_eq!(measured.figure("P_pi").unwrap().value, 22.8);
    assert!(
        measured
            .figure("tau")
            .unwrap()
            .grid
            .starts_with("measured: ")
    );
    let other = Values::new(&d, &[("buffer", 0.6)]).unwrap();
    let e = d.component(&other, Fidelity::Measured).unwrap_err();
    assert!(e.to_string().contains("buffer = 0.8"), "{e}");
    // a measurement of one's own, of another design
    let text = r#"
        kind = "doped thermo-optic phase shifter"
        source = "a measurement of our own"
        [design]
        buffer = 0.6
        [[figure]]
        symbol = "P_pi"
        name = "power for a π shift"
        value = 20.0
        unit = "mW"
        where = "our bench"
    "#;
    let mine = d
        .with_measurement(Measurement::parse(text).unwrap())
        .unwrap();
    let b = mine.component(&other, Fidelity::Measured).unwrap();
    assert_eq!(b.figure("P_pi").unwrap().value, 20.0);
    // another kind's, or a parameter the device hasn't, is refused
    let wrong = text.replace("doped thermo-optic", "thermo-optic");
    assert!(
        ThermoShifter::doped()
            .with_measurement(Measurement::parse(&wrong).unwrap())
            .is_err()
    );
    assert!(Measurement::parse(&text.replace("buffer", "gap")).is_ok());
    assert!(
        ThermoShifter::doped()
            .with_measurement(Measurement::parse(&text.replace("buffer", "gap")).unwrap())
            .is_err()
    );
    // a file without figures, or an unknown key, doesn't parse
    assert!(Measurement::parse("kind = \"x\"\nsource = \"y\"").is_err());
    assert!(Measurement::parse(&format!("{text}\nextra = 1")).is_err());
}

#[test]
fn the_designs_are_built_from_a_job_and_refuse_what_they_cant_take() {
    for kind in [
        "thermo-optic phase shifter",
        "doped thermo-optic phase shifter",
    ] {
        let (d, design) = Spec::new(kind).build().unwrap();
        assert_eq!(d.kind(), kind);
        assert_eq!(design.values().get("length"), Some(320.0));
        assert_eq!(d.fidelities()[0].fidelity, Fidelity::Analytic);
        // a silicon nitride core isn't the heat model's silicon
        let mut nitride = Spec::new(kind);
        nitride.stack = super::StackSpec::preset("nitride");
        let e = nitride.build().unwrap_err();
        assert!(e.to_string().contains("silicon"), "{e}");
    }
    // a doped heater's slab thinner than its rib
    let d = ThermoShifter::doped();
    let v = Values::new(&d, &[("slab", 0.3)]).unwrap();
    assert!(d.component(&v, Fidelity::Analytic).is_err());
}

#[test]
fn the_geometry_draws_the_guide_and_the_heater_on_the_stack() {
    let d = ThermoShifter::metal();
    let v = Values::defaults(&d);
    let g = d.geometry(&v).unwrap();
    let names: Vec<&str> = g.ports.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["o1", "o2"]);
    let stack = g.structure.stack();
    let (_, bottom, _) = stack.layer("TiN").unwrap();
    // the heater 2 µm above the strip's top, on 2 µm of buried oxide
    assert!((bottom.to_um() - (2.0 + 0.22 + 2.0)).abs() < 1e-12);
    assert_eq!(g.structure.shapes("TiN").len(), 1);
    let doped = ThermoShifter::doped();
    let g = doped.geometry(&Values::defaults(&doped)).unwrap();
    assert!(!g.structure.is_plain("Si"));
}

#[test]
fn the_accelerated_series_is_the_slabs() {
    // Kummer's transformation of the slab's steady series against the series itself, held and
    // insulated, inside a strip and beside it, at the sheet's height and off it
    use super::thermo_closed::{Sheets, Slab};
    for held in [true, false] {
        let slab = Slab {
            height: 12.2e-6,
            conductivity: 1.38,
            diffusivity: 1.38 / (2203.0 * 709.0),
            held,
        };
        let sheets = [Sheets {
            height: 2.05e-6,
            strips: vec![
                (0.0, 0.5e-6, 0.4),
                (-1.5e-6, 1.0e-6, 0.3),
                (1.5e-6, 1.0e-6, 0.3),
            ],
        }];
        for (x, y) in [
            (0.0, 2.11e-6),
            (0.1e-6, 2.05e-6),
            (5e-6, 2.11e-6),
            (1.2e-6, 4e-6),
        ] {
            let (fast, series) = (
                slab.steady_accelerated(&sheets, x, y),
                slab.steady(&sheets, x, y),
            );
            assert!(
                (fast / series - 1.0).abs() < 1e-4,
                "{held} {x} {y}: {fast} {series}"
            );
        }
    }
}

#[test]
fn fixed_talbot_inverts_a_step_into_a_pole() {
    use super::thermo_closed::talbot;
    use num_complex::Complex64 as c64;
    // F(s) = 1/(s(s + a)): f(t) = (1 − e^(−at))/a
    let a = 3.0e5;
    for t in [1e-7, 1e-6, 5e-6, 2e-5] {
        let f = talbot(|s: c64| 1.0 / (s * (s + a)), t);
        let exact = (1.0 - (-a * t).exp()) / a;
        assert!((f / exact - 1.0).abs() < 1e-8, "{t}: {f} {exact}");
    }
}

#[test]
fn harris_heater_through_the_device_is_the_phases_to_the_bit() {
    // the phase's own averaged section (crosstalk_checks::harris, the contacts held) and the
    // device's at its defaults: the same current, heat problem and mode, the same slope and
    // bandwidth, bit for bit
    use crate::thermal::crosstalk_checks::{Section2d, harris};
    let d = ThermoShifter::in_guide();
    let device = d.shifter(&Values::defaults(&d)).unwrap();
    let phase = harris(Section2d::Averaged, 1.0, true);
    let s = |p: &crate::thermal::PhaseShifter| p.efficiency().unwrap().slopes[0][0];
    assert_eq!(s(&device).to_bits(), s(&phase).to_bits());
    assert_eq!(
        device.cutoff(0, 0).unwrap().to_bits(),
        phase.cutoff(0, 0).unwrap().to_bits()
    );
}

#[test]
fn the_fin_is_its_equation_solved() {
    // the in-guide heater's fin, steady, against a finite-difference solve of the same
    // equation (−(K T')' + Y T = q, T'(0) = 0, T(end) = 0) on 5800 and 11 600 cells
    let d = ThermoShifter::in_guide();
    let fin = d.fin(&Values::defaults(&d), 2.0).unwrap();
    let (k, _) = fin.oxide;
    let total: f64 = fin.stretches.iter().map(|s| s.length).sum();
    let solve = |n: usize| -> f64 {
        let h = total / n as f64;
        // each cell's K, Y and q, by its centre's stretch
        let at = |x: f64| {
            let mut from = 0.0;
            for s in &fin.stretches {
                if x < from + s.length {
                    return s;
                }
                from += s.length;
            }
            fin.stretches.last().unwrap()
        };
        let cells: Vec<_> = (0..n)
            .map(|i| {
                let s = at((i as f64 + 0.5) * h);
                let y = fin.spreading * (k / s.below + k / s.above);
                (s.conductance, y, s.heat)
            })
            .collect();
        // tridiagonal: the faces' conductances K/h (harmonic between cells), the end held half
        // a cell beyond the last centre
        let face = |a: f64, b: f64| 2.0 * a * b / (a + b) / h;
        let (mut lower, mut diag, mut upper, mut rhs) =
            (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        for i in 0..n {
            let (kc, y, q) = cells[i];
            diag[i] = y * h;
            rhs[i] = q * h;
            if i > 0 {
                let g = face(cells[i - 1].0, kc);
                lower[i] = -g;
                diag[i] += g;
            }
            if i + 1 < n {
                let g = face(kc, cells[i + 1].0);
                upper[i] = -g;
                diag[i] += g;
            } else {
                diag[i] += 2.0 * kc / h;
            }
        }
        for i in 1..n {
            let m = lower[i] / diag[i - 1];
            diag[i] -= m * upper[i - 1];
            rhs[i] -= m * rhs[i - 1];
        }
        let mut t = vec![0.0; n];
        t[n - 1] = rhs[n - 1] / diag[n - 1];
        for i in (0..n - 1).rev() {
            t[i] = (rhs[i] - upper[i] * t[i + 1]) / diag[i];
        }
        t[0]
    };
    // the stretches' ends on the cells' faces (each a multiple of 50 nm of the 2.9 µm), second
    // order: within 1e-8 of the closed form on both (2.3e-9 and 8.9e-9 here, the cells' ends
    // rounded at the stretches' then)
    let exact = fin.steady();
    for n in [5_800, 11_600] {
        let e = solve(n) / exact - 1.0;
        assert!(e.abs() < 2e-8, "{n}: {e}");
    }
    // and the heat it gives the oxide and the contact take it all: twice each side's
    let leaks: f64 = fin.leaks().iter().sum();
    assert!(leaks > 0.3 && leaks < 0.5, "{leaks}");
}

#[test]
fn the_in_guide_heaters_measured_loss_is_its_components() {
    let d = ThermoShifter::in_guide();
    let v = Values::new(&d, &[("power", 10.0), ("loss", 5.0)]).unwrap();
    let b = d.component(&v, Fidelity::Measured).unwrap();
    assert_eq!(b.figure("P_pi").unwrap().value, 24.77);
    assert_eq!(b.values[1], 0.23);
    assert_eq!(b.figures.iter().filter(|f| f.symbol == "IL").count(), 1);
    let t = b.s_matrix(Wavelength::um(1.55).unwrap()).unwrap()[(1, 0)].norm();
    assert!((t - 10f64.powf(-0.23 / 20.0)).abs() < 1e-15);
    // the measured loss moves with neither the loss nor the length
    for name in ["loss", "length"] {
        let k = d.parameters().iter().position(|p| p.name == name).unwrap();
        assert_eq!(b.derivatives[k], Some(vec![0.0, 0.0]), "{name}");
    }
    // the analytic model's loss is the loss over the length, 5 dB/cm over 61.6 µm
    let a = d.component(&v, Fidelity::Analytic).unwrap();
    assert!((a.values[1] - 5.0 * 61.6e-4).abs() < 1e-15);
    // its design is Harris et al.'s: the fill 0.8 of 2.44 µm
    let other = Values::new(&d, &[("fill", 0.5)]).unwrap();
    let e = d.component(&other, Fidelity::Measured).unwrap_err();
    assert!(e.to_string().contains("fill"), "{e}");
}

#[test]
fn the_in_guide_heater_refuses_what_it_cant_take() {
    let (d, design) = Spec::new("in-guide thermo-optic phase shifter")
        .build()
        .unwrap();
    assert_eq!(design.values().get("length"), Some(61.6));
    assert_eq!(d.fidelities().len(), 3);
    let mut nitride = Spec::new("in-guide thermo-optic phase shifter");
    nitride.stack = super::StackSpec::preset("nitride");
    assert!(nitride.build().unwrap_err().to_string().contains("silicon"));
    let d = ThermoShifter::in_guide();
    for bad in [
        ("doped_width", 0.4),   // narrower than the rib
        ("channel_start", 0.4), // inside the p region
        ("reach", 1.05),        // the contacts before the channels' start
        ("slab", 0.3),          // thicker than the rib
    ] {
        let v = Values::new(&d, &[bad]).unwrap();
        assert!(d.component(&v, Fidelity::Analytic).is_err(), "{bad:?}");
    }
    let g = d.geometry(&Values::defaults(&d)).unwrap();
    assert!(!g.structure.is_plain("Si"));
}
