//! The phase shifter's unit tests: two mirror-image heaters and guides, the guide's weights
//! against its own sum over a field, turn-on against turn-off, and what is refused.

use super::shifter_checks::{NPP, jacques};
use super::*;
use crate::geometry::Point;
use crate::units::{Length, Wavelength};

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .unwrap()
}

/// Two 500 × 220 nm strips at x = ±3 µm on 2 µm of oxide over a held substrate, a heater 1 µm
/// above each: a coarse problem.
fn pair() -> PhaseShifter {
    let mut section = Section::new("SiO2");
    section.layer(-6.0, -2.0, "sub").unwrap();
    let (a, b) = (
        rect([-3.25, -2.75], [0.0, 0.22]),
        rect([2.75, 3.25], [0.0, 0.22]),
    );
    let (ha, hb) = (rect([-3.5, -2.5], [1.2, 1.3]), rect([2.5, 3.5], [1.2, 1.3]));
    section.draw(a, "Si").draw(b, "Si");
    let mut features = Features::new();
    features.extend(section.features()).shape(&ha).shape(&hb);
    let grid = Grid::fitted(
        [-15.0, 15.0],
        [-6.0, 2.0],
        &Grading::new(0.5, 0.05, 1.3).unwrap(),
        &features,
    )
    .unwrap();
    let mut heat = Heat::new(section.clone(), grid);
    heat.conductivity("SiO2", Tensor::isotropic(1.4))
        .conductivity("Si", Tensor::isotropic(100.0))
        .conductivity("sub", Tensor::isotropic(148.0))
        .heat_capacity("SiO2", 1.6e6)
        .heat_capacity("Si", 1.66e6)
        .heat_capacity("sub", 1.66e6)
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    let mut optics = Optics::new(Wavelength::um(1.55).unwrap(), 0.04).unwrap();
    optics
        .medium("Si", 3.476, 1.8e-4)
        .medium("SiO2", 1.444, 1e-5);
    let guides = [-3.0, 3.0]
        .iter()
        .map(|&c| Guide::new("g", &section, ([c - 1.0, c + 1.0], [-0.72, 0.92]), &optics).unwrap())
        .collect();
    PhaseShifter::new(
        heat,
        vec![Drive::heater("a", ha, 2.0), Drive::heater("b", hb, 2.0)],
        guides,
    )
    .unwrap()
}

#[test]
fn mirror_images_respond_alike() {
    let ps = pair();
    let e = ps.efficiency().unwrap();
    let s = &e.slopes;
    assert!(
        s[0][0] > 0.0 && s[1][0] > 0.0 && s[0][0] > 3.0 * s[1][0],
        "{s:?}"
    );
    // the fitted grid is a mirror image to round-off in its lines
    assert!((s[0][0] / s[1][1] - 1.0).abs() < 1e-5, "{s:?}");
    assert!((s[1][0] / s[0][1] - 1.0).abs() < 1e-3, "{s:?}");
    assert!((e.crosstalk(1, 0, 0) / e.crosstalk(0, 1, 1) - 1.0).abs() < 1e-3);
    // π at P_π
    assert!((e.phase(0, 0, e.p_pi(0, 0)) - std::f64::consts::PI).abs() < 1e-12);
    // the same through the guide's own sum over the rise: the weights on the grid
    let rise = ps.rise(0).unwrap();
    let direct = ps.guides()[0].index_change(&rise);
    assert!(
        (direct / s[0][0] - 1.0).abs() < 1e-10,
        "{direct} {}",
        s[0][0]
    );
}

#[test]
fn turning_on_and_off_take_the_same_time() {
    let ps = pair();
    let stepping = Stepping::new(0.1e-6, 150, Scheme::Bdf2);
    let on = ps.times(0, 0, &stepping, true).unwrap();
    let off = ps.times(0, 0, &stepping, false).unwrap();
    assert!(on.one_over_e_s > 0.0 && on.ten_ninety_s > on.one_over_e_s * 0.5);
    assert!(
        (on.one_over_e_s / off.one_over_e_s - 1.0).abs() < 1e-9,
        "{on:?} {off:?}"
    );
    let f = ps.cutoff(0, 0).unwrap();
    // a single pole's 1/(2πτ) within a factor of a few: heat's response is many poles
    let pole = 1.0 / (2.0 * std::f64::consts::PI * on.one_over_e_s);
    assert!(f > 0.3 * pole && f < 3.0 * pole, "{f} {pole}");
    // too few steps to get there
    let short = Stepping::new(0.1e-6, 2, Scheme::Bdf2);
    assert!(ps.times(0, 0, &short, true).is_err());
}

#[test]
fn what_is_refused() {
    let ps = pair();
    let heat = ps.heat().clone();
    let guide = ps.guides()[0].clone();
    let h = rect([-1.0, 1.0], [1.2, 1.3]);
    assert!(PhaseShifter::new(heat.clone(), vec![], vec![guide.clone()]).is_err());
    assert!(
        PhaseShifter::new(
            heat.clone(),
            vec![Drive::heater("a", h.clone(), 1.0)],
            vec![]
        )
        .is_err()
    );
    assert!(
        PhaseShifter::new(
            heat.clone(),
            vec![Drive::heater("a", h.clone(), 0.0)],
            vec![guide.clone()]
        )
        .is_err()
    );
    assert!(Optics::new(Wavelength::um(1.55).unwrap(), 0.0).is_err());
    let optics = Optics::new(Wavelength::um(1.55).unwrap(), 0.05).unwrap();
    // no index for the media
    assert!(Guide::new("g", heat.section(), ([-1.0, 1.0], [-0.5, 0.5]), &optics).is_err());
    assert!(Guide::new("g", heat.section(), ([1.0, -1.0], [-0.5, 0.5]), &optics).is_err());
}

#[test]
fn a_doped_heater_through_its_current_heats_as_its_shapes_do() {
    // the N++ strips' heat spread by area, against the same strips' Joule heat of a current
    // across each: the rise per watt within a few percent, the contacts not dissipating
    let ps = jacques(NPP, &Grading::new(1.0, 0.05, 1.3).unwrap(), 0.05);
    let by_area = ps.efficiency().unwrap().slopes[0][0];
    let grid = ps.heat().grid().clone();
    let mut current = crate::electro::Current::new(ps.heat().section().clone(), grid);
    for (m, s) in [
        ("SiO2", 1e-11),
        ("substrate", 4.3e-4),
        ("Si", 4.3e-4),
        ("slab", 4.3e-4),
        ("N++", 1e5),
    ] {
        current.conductivity(m, Tensor::isotropic(s));
    }
    // each strip between contacts 50 nm wide at its ends: its current across it, its heat
    // uniform but in the contacts
    for (x, v) in [(-2.05, 1.0), (-1.1, 0.0), (1.1, 0.0), (2.0, 1.0)] {
        current.contact(crate::electro::Conductor::shape(
            rect([x, x + 0.05], [0.0, 0.09]),
            v,
        ));
    }
    let flow = current.solve().unwrap();
    let joule = PhaseShifter::new(
        ps.heat().clone(),
        vec![Drive::joule("N++", &flow)],
        ps.guides().to_vec(),
    )
    .unwrap();
    let by_current = joule.efficiency().unwrap().slopes[0][0];
    assert!(
        (by_current / by_area - 1.0).abs() < 0.05,
        "{by_current} {by_area}"
    );
    // a Joule heat on another grid is refused
    let other = Grid::fitted(
        [-20.0, 20.0],
        [-14.0, 3.32],
        &Grading::uniform(1.0).unwrap(),
        &Features::new(),
    )
    .unwrap();
    let mut wrong = crate::electro::Current::new(ps.heat().section().clone(), other);
    for m in ["SiO2", "substrate", "Si", "slab", "N++"] {
        wrong.conductivity(m, Tensor::isotropic(1.0));
    }
    wrong
        .contact(crate::electro::Conductor::shape(
            rect([-20.0, -19.0], [-14.0, 3.0]),
            1.0,
        ))
        .contact(crate::electro::Conductor::shape(
            rect([19.0, 20.0], [-14.0, 3.0]),
            0.0,
        ));
    let wrong = wrong.solve().unwrap();
    assert!(
        PhaseShifter::new(
            ps.heat().clone(),
            vec![Drive::joule("x", &wrong)],
            ps.guides().to_vec()
        )
        .is_err()
    );
}
