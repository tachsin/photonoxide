//! The full-wave RF mode ([`super`]): a mirrored half against the whole line, a TEM line in a
//! homogeneous dielectric, the PML's independence of its layers, and what it refuses.

use super::*;
use crate::electro::{Axis, Boundary, Conductor, Grid, Section, Tensor};
use crate::geometry::{Point, Shape};
use crate::units::Length;

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .unwrap()
}

/// Nodes from 0 to `end` with one on each of `marks`, graded from 0.05 µm at them by 30% a
/// cell to 5 µm.
fn nodes(end: f64, marks: &[f64]) -> Vec<f64> {
    use crate::electro::Grading;
    Axis::graded(
        0.0,
        end,
        &Grading::new(5.0, 0.05, 1.3).unwrap(),
        marks,
        marks,
    )
    .unwrap()
    .nodes()
    .to_vec()
}

/// A coplanar waveguide in air over a substrate of ε 4: a 10 µm signal, 5 µm gaps, grounds 40
/// µm wide, 1 µm of metal, in a window 100 µm across; the whole line (`whole`) or its right
/// half beside the mirror x = 0, on the same nodes mirrored.
fn cpw(whole: bool) -> Electrostatic {
    let marks = [5.0, 10.0, 50.0];
    let right = nodes(100.0, &marks);
    let x = if whole {
        let mut v: Vec<f64> = right.iter().rev().map(|p| -p).collect();
        v.pop();
        v.extend_from_slice(&right);
        v
    } else {
        right
    };
    let y: Vec<f64> = {
        let up = nodes(60.0, &[0.0, 1.0]);
        let mut v: Vec<f64> = nodes(60.0, &[0.0]).iter().rev().map(|p| -p).collect();
        v.pop();
        v.extend_from_slice(&up);
        v
    };
    let grid = Grid::new(Axis::new(x).unwrap(), Axis::new(y).unwrap());
    let mut section = Section::new("air");
    section.layer(-60.0, 0.0, "substrate").unwrap();
    let mut p = Electrostatic::new(section, grid);
    p.permittivity("air", Tensor::isotropic(1.0))
        .permittivity("substrate", Tensor::isotropic(4.0))
        .conductor(Conductor::shape(rect([-5.0, 5.0], [0.0, 1.0]), 1.0))
        .conductor(Conductor::shape(rect([10.0, 50.0], [0.0, 1.0]), 0.0));
    if whole {
        p.conductor(Conductor::shape(rect([-50.0, -10.0], [0.0, 1.0]), 0.0));
    }
    p
}

#[test]
fn a_mirrored_half_is_the_whole_lines_even_mode() {
    let fw = FullWave::new(0, 4.1e7).unwrap().with_count(4);
    let half = fw.clone().mirrored().solve(&cpw(false), 50e9, 1.6).unwrap();
    let whole = fw.solve(&cpw(true), 50e9, 1.6).unwrap();
    let rel = |a: c64, b: c64| (a - b).norm() / b.norm();
    assert!(rel(half.gamma, whole.gamma) < 1e-8, "{half:?} {whole:?}");
    assert!(
        rel(half.impedance, whole.impedance) < 1e-6,
        "{half:?} {whole:?}"
    );
    // a quasi-TEM line between air and ε 4: its index near √2.5
    assert!(
        (half.index() - 2.5f64.sqrt()).abs() < 0.1,
        "{}",
        half.index()
    );
    // a closed window radiates nothing: the eigenvalue's loss is the power the metal takes
    assert!(
        half.radiation().abs() < 1e-6 * half.attenuation(),
        "{half:?}"
    );
    assert!(half.dielectric == 0.0 && half.conductor > 0.0);
}

#[test]
fn a_tem_line_in_one_dielectric_travels_at_its_speed() {
    // a square bar in a box at 0 V, all of one dielectric: TEM, whose fields are the same in
    // any filling, so γ and the impedance scale as √ε and 1/√ε to the metal's share of the
    // wave, and β² = k₀²ε exactly in the limit of a perfect conductor
    let x: Vec<f64> = (0..=80).map(|i| -10.0 + 0.25 * f64::from(i)).collect();
    let grid = Grid::new(Axis::new(x.clone()).unwrap(), Axis::new(x).unwrap());
    let line = |eps: f64, sigma: f64| {
        let mut p = Electrostatic::new(Section::new("fill"), grid.clone());
        p.permittivity("fill", Tensor::isotropic(eps))
            .conductor(Conductor::shape(rect([-2.0, 2.0], [-2.0, 2.0]), 1.0))
            .conductor(Conductor::shape(rect([-20.0, 20.0], [-20.0, -11.0]), 0.0));
        for s in Side::ALL {
            p.boundary(s, Boundary::Fixed(0.0));
        }
        FullWave::new(0, sigma)
            .unwrap()
            .solve(&p, 10e9, eps.sqrt())
            .unwrap()
    };
    let (air, fill) = (line(1.0, 4.1e7), line(4.0, 4.1e7));
    let ratio = fill.gamma / air.gamma;
    assert!((ratio - 2.0).norm() < 1e-6, "{ratio}");
    let ratio = fill.impedance / air.impedance;
    assert!((ratio - 0.5).norm() < 1e-6, "{ratio}");
    // the internal inductance raises the index above √ε, less as the metal conducts better
    let excess = |sigma: f64| line(4.0, sigma).index() - 2.0;
    let (a, b) = (excess(1e6), excess(1e8));
    assert!(a > b && b > 0.0, "{a} {b}");
}

#[test]
fn the_layers_absorb_whatever_their_thickness() {
    // the line over a substrate of ε 12 that its wave outruns: it radiates into it
    let mut p = cpw(false);
    p.permittivity("substrate", Tensor::isotropic(12.0));
    let at = |t: f64, strength: f64| {
        FullWave::new(0, 4.1e7)
            .unwrap()
            .mirrored()
            .with_count(4)
            .with_strength(strength)
            .with_open(Open::new(Side::Bottom, t).unwrap().with_cells(12))
            .with_open(Open::new(Side::Right, t).unwrap().with_cells(12))
            .solve(&p, 600e9, 2.6)
            .unwrap()
    };
    let (a, b) = (at(150.0, 4.0), at(250.0, 8.0));
    assert!(a.radiation() > 0.05 * a.attenuation(), "{a:?}");
    let rel = (a.radiation() - b.radiation()).abs() / a.radiation();
    assert!(rel < 0.02, "{a:?} {b:?}");
}

#[test]
fn what_it_refuses() {
    let p = cpw(false);
    let fails = |fw: FullWave, problem: &Electrostatic, what: &str| {
        let e = fw.solve(problem, 50e9, 1.6).unwrap_err().to_string();
        assert!(e.contains(what), "{e}");
    };
    let fw = FullWave::new(0, 4.1e7).unwrap();
    fails(
        fw.clone()
            .mirrored()
            .with_open(Open::new(Side::Left, 10.0).unwrap()),
        &p,
        "symmetry plane",
    );
    fails(
        fw.clone().with_loss_tangents(&[("glass", 0.01)]),
        &p,
        "glass isn't a medium",
    );
    fails(
        FullWave::new(5, 4.1e7).unwrap(),
        &p,
        "the signal is conductor 5",
    );
    let mut strips = p.clone();
    strips.conductors[1] = Conductor::strip([10.0, 50.0], 0.0, 0.0);
    fails(fw.clone(), &strips, "zero thickness");
    let mut charged = p.clone();
    charged.boundary(Side::Top, Boundary::Flux(1.0));
    fails(fw.clone(), &charged, "the top side");
    assert!(FullWave::new(0, 0.0).is_err());
    assert!(Open::new(Side::Bottom, 0.0).is_err());
    assert!(fw.solve(&p, -1.0, 1.6).is_err());
}
