//! The phase-change phase shifter devices.

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use crate::circuit::Fidelity;
use crate::geometry::Point;
use crate::material::catalogue::State;
use crate::units::{Length, Wavelength};

use super::{Device, Goal, PcmShifter, Spec, Values};

fn devices() -> [PcmShifter; 3] {
    [PcmShifter::sb2se3(), PcmShifter::sb2s3(), PcmShifter::gst()]
}

fn figure(d: &PcmShifter, v: &Values, fidelity: Fidelity, symbol: &str) -> f64 {
    d.component(v, fidelity)
        .unwrap()
        .figure(symbol)
        .unwrap()
        .value
}

#[test]
fn the_devices_are_in_the_library_with_their_models_and_objectives() {
    for (d, kind) in devices().iter().zip([
        "Sb2Se3 phase-change phase shifter",
        "Sb2S3 phase-change phase shifter",
        "GST phase-change phase shifter",
    ]) {
        assert_eq!(d.kind(), kind);
        let fidelities: Vec<Fidelity> = d.fidelities().iter().map(|m| m.fidelity).collect();
        assert_eq!(fidelities, [Fidelity::Analytic, Fidelity::TwoD]);
        // the analytic model states its error against the 2D one
        let stated = d.fidelities()[0].error.as_ref().unwrap();
        assert_eq!(
            (stated.against, stated.quantity.as_str()),
            (Fidelity::TwoD, "dn_eff_ac")
        );
        let objectives = d.objectives();
        let symbols: Vec<&str> = objectives.iter().map(|o| o.symbol.as_str()).collect();
        assert_eq!(symbols, ["L_pi", "dn_eff_ac", "alpha_c"]);
        assert_eq!(objectives[1].goal, Goal::Maximize);
        // a spec of the kind builds the library's device
        let (built, design) = Spec::new(kind).build().unwrap();
        assert_eq!(built.parameters(), d.parameters());
        assert_eq!(design.values(), &Values::defaults(d));
    }
    let soi = crate::stack::LayerStack::soi_220;
    assert!(PcmShifter::on("gold", soi(), "Si").is_err());
    assert!(PcmShifter::on("gst", soi(), "Ge").is_err());
    assert!(PcmShifter::on("gst", soi(), "Si").is_ok());
}

#[test]
fn the_films_states_are_the_catalogues_and_between_them_the_linear_mix() {
    let wl = Wavelength::um(1.55).unwrap();
    for d in devices() {
        let layer = |f: f64| {
            let v = Values::new(&d, &[("crystallized", f)]).unwrap();
            let stack = d.stack(&v).unwrap();
            let (film, bottom, top) = stack.layer("PCM").unwrap();
            // the film sits on the core, as thick as asked
            let (_, _, core_top) = stack.layer("Si").unwrap();
            assert_eq!(bottom, core_top);
            assert!(((top - bottom).to_nm() - 25.0).abs() < 1e-9);
            film.material.permittivity(wl).unwrap()
        };
        let a = d.state(State::Amorphous).unwrap().permittivity(wl).unwrap();
        let c = d
            .state(State::Crystalline)
            .unwrap()
            .permittivity(wl)
            .unwrap();
        assert_eq!(layer(0.0), a);
        assert_eq!(layer(1.0), c);
        let half = layer(0.5);
        assert!((half - 0.5 * (a + c)).norm() < 1e-13 * half.norm());
    }
}

#[test]
fn the_index_change_and_loss_of_the_three_materials_are_ordered() {
    let [se, s, gst] = devices();
    let at = |d: &PcmShifter, f: Fidelity, symbol: &str| figure(d, &Values::defaults(d), f, symbol);
    for f in [Fidelity::Analytic, Fidelity::TwoD] {
        // crystallizing raises the index of every one of them, GST's the most
        let (a, b, c) = (
            at(&s, f, "dn_eff_ac"),
            at(&se, f, "dn_eff_ac"),
            at(&gst, f, "dn_eff_ac"),
        );
        assert!(0.0 < a && a < b && b < c, "{f}: {a} {b} {c}");
        // Sb₂S₃ absorbs nothing in either state, Sb₂Se₃ a little when crystalline, GST a lot
        assert_eq!(at(&s, f, "alpha_c"), 0.0);
        assert!(at(&se, f, "alpha_a") == 0.0 && at(&se, f, "alpha_c") > 1e-5);
        assert!(at(&gst, f, "alpha_c") > 10.0 * at(&gst, f, "alpha_a"));
        // no figure of merit for a film that absorbs nothing
        let b = s.component(&Values::defaults(&s), f).unwrap();
        assert!(b.figure("FOM").is_none());
    }
}

#[test]
fn the_phase_and_loss_follow_the_crystallized_fraction() {
    // the effective index method: instant
    let d = PcmShifter::gst();
    let at = |f: f64| {
        let v = Values::new(&d, &[("crystallized", f), ("length", 10.0), ("loss", 3.0)]).unwrap();
        d.component(&v, Fidelity::Analytic).unwrap()
    };
    let (empty, half, full) = (at(0.0), at(0.5), at(1.0));
    // against the amorphous state: no phase at 0, and growing with the fraction
    assert_eq!(empty.figure("phi").unwrap().value, 0.0);
    let (p, q) = (
        half.figure("phi").unwrap().value,
        full.figure("phi").unwrap().value,
    );
    assert!(0.0 < p && p < q);
    // the full phase is 2π Δn_eff L/λ with the amorphous to crystalline change
    let dn = full.figure("dn_eff_ac").unwrap().value;
    assert!((q - TAU * dn * 10.0 / 1.55).abs() < 1e-12 * q);
    // the loss grows with it too, the guide's own 3 dB/cm over 10 µm added
    let il = |b: &super::Built| b.figure("IL").unwrap().value;
    assert!(il(&empty) < il(&half) && il(&half) < il(&full));
    let alpha_a = empty.figure("alpha").unwrap().value;
    assert!((il(&empty) - (alpha_a * 10.0 + 3.0 * 10.0 * 1e-4)).abs() < 1e-13);
    // the ends are the states': the change between them is the two n_eff's difference
    let n = |b: &super::Built| b.figure("n_eff").unwrap().value;
    assert!((n(&empty) + dn - n(&full)).abs() < 1e-14);
}

#[test]
fn the_component_is_the_phase_and_loss_and_its_derivatives_are_exact() {
    let d = PcmShifter::sb2se3();
    let base = [("crystallized", 1.0), ("length", 14.0), ("loss", 20.0)];
    let v = Values::new(&d, &base).unwrap();
    let b = d.component(&v, Fidelity::Analytic).unwrap();
    let lambda = Wavelength::um(1.55).unwrap();
    let s = b.s_matrix(lambda).unwrap();
    let (phi, il) = (b.values[0], b.values[1]);
    assert_eq!(phi, b.figure("phi").unwrap().value);
    assert_eq!(il, b.figure("IL").unwrap().value);
    let want = c64::from_polar(10f64.powf(-il / 20.0), phi);
    assert!((s[(1, 0)] - want).norm() < 1e-14);
    // reciprocal
    assert_eq!(s[(0, 1)], s[(1, 0)]);
    // dS/dL and dS/dloss through the component's derivatives and the device's against differences
    let ds = b.component.derivatives(lambda, &b.values).unwrap().unwrap();
    for (name, h) in [("length", 1e-3), ("loss", 1e-3)] {
        let k = d.parameters().iter().position(|p| p.name == name).unwrap();
        let dp = b.derivatives[k].as_ref().unwrap();
        let exact = ds[0][(1, 0)] * dp[0] + ds[1][(1, 0)] * dp[1];
        let at = |x: f64| {
            let changes: Vec<(&str, f64)> = base
                .iter()
                .map(|&(n, v)| (n, if n == name { x } else { v }))
                .collect();
            let v = Values::new(&d, &changes).unwrap();
            d.component(&v, Fidelity::Analytic)
                .unwrap()
                .s_matrix(lambda)
                .unwrap()[(1, 0)]
        };
        let x0 = v.get(name).unwrap();
        let numeric = (at(x0 + h) - at(x0 - h)) / (2.0 * h);
        assert!(
            (exact - numeric).norm() < 1e-7 * exact.norm(),
            "{name}: {exact} {numeric}"
        );
    }
}

#[test]
fn the_2d_cross_section_is_the_geometrys() {
    // the film half the guide's width, centred: the cells' permittivity is the film's under it,
    // the oxide's beside it above the guide, and the guide's below
    let d = PcmShifter::sb2se3();
    let v = Values::new(&d, &[("coverage", 0.5), ("crystallized", 1.0)]).unwrap();
    let cs = d.cross_section(&v, 1.0).unwrap();
    let wl = Wavelength::um(1.55).unwrap();
    let eps = |m: &crate::material::Material| m.permittivity(wl).unwrap();
    let film = eps(&d.state(State::Crystalline).unwrap());
    let (silicon, oxide) = (
        eps(&crate::material::silicon()),
        eps(&crate::material::silica()),
    );
    // the cell containing the point (x, y), y from the core's mid-plane
    let cell = |x: f64, y: f64| {
        let i = cs
            .x()
            .windows(2)
            .position(|w| w[0] <= x && x < w[1])
            .unwrap();
        let j = cs
            .y()
            .windows(2)
            .position(|w| w[0] <= y && y < w[1])
            .unwrap();
        cs.cells()[i * (cs.y().len() - 1) + j].xx
    };
    // 0.11 µm is the core's top, the film 25 nm over it; x = 0.125 is the film's edge
    let in_film = 0.11 + 0.0125;
    assert!((cell(0.05, in_film) - film).norm() < 1e-12);
    assert!((cell(0.2, in_film) - oxide).norm() < 1e-12);
    assert!((cell(0.2, 0.0) - silicon).norm() < 1e-12);
    assert!((cell(0.05, 0.0) - silicon).norm() < 1e-12);
    // the structure the device draws has the same materials at the same points (x along the
    // guide, y across)
    let g = d.geometry(&v).unwrap();
    let (_, bottom, top) = g.structure.stack().layer("Si").unwrap();
    let mid = (bottom + top) / 2.0;
    let at = |across: f64, dz: f64| {
        let p = Point {
            x: Length::um(5.0),
            y: Length::um(across),
        };
        eps(g.structure.material_at(p, mid + Length::um(dz)))
    };
    assert!((at(0.05, in_film) - film).norm() < 1e-12);
    assert!((at(0.2, in_film) - oxide).norm() < 1e-12);
    assert!((at(-0.05, in_film) - film).norm() < 1e-12);
}

#[test]
fn bad_designs_and_refinements_are_refused() {
    let d = PcmShifter::gst();
    // a slab as thick as the guide
    let v = Values::new(&d, &[("slab", 0.22)]).unwrap();
    assert!(d.component(&v, Fidelity::TwoD).is_err());
    // outside the parameters' ranges, or a fidelity it hasn't
    assert!(Values::new(&d, &[("crystallized", 1.5)]).is_err());
    assert!(Values::new(&d, &[("wavelength", 1.0)]).is_err());
    assert!(
        d.component(&Values::defaults(&d), Fidelity::Measured)
            .is_err()
    );
    assert!(d.clone().with_refinement(0.5).is_err());
    assert!(d.with_refinement(f64::NAN).is_err());
}

#[test]
fn a_rib_is_drawn_and_its_slab_lowers_the_index_change() {
    // the guide etched part of the way: its slab holds more of the mode out of the film's reach,
    // and the geometry draws the etch
    let d = PcmShifter::sb2se3();
    let strip = Values::defaults(&d);
    let rib = Values::new(&d, &[("slab", 0.1)]).unwrap();
    let g = d.geometry(&rib).unwrap();
    assert_eq!(g.structure.shapes("Si").len(), 1);
    let change = |v: &Values| figure(&d, v, Fidelity::Analytic, "dn_eff_ac");
    assert!(change(&rib) < change(&strip));
}

#[test]
fn a_wide_guide_has_the_same_mode_by_either_fidelity() {
    // as a guide widens its mode is the vertical column's, which the effective index method
    // solves exactly: the two fidelities' n_eff meet
    let d = PcmShifter::sb2se3();
    let v = Values::new(&d, &[("width", 3.0), ("crystallized", 1.0)]).unwrap();
    let (a, _) = d.states(&v, Fidelity::Analytic).unwrap();
    let (b, _) = d.states(&v, Fidelity::TwoD).unwrap();
    assert!((a.crystalline.re - b.crystalline.re).abs() < 2e-3);
    assert!((a.amorphous.re - b.amorphous.re).abs() < 2e-3);
}
