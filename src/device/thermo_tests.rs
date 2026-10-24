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
