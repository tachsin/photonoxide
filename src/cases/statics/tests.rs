use super::checks::{self, order_of};
use super::*;
use crate::geometry::{Point, Shape};
use crate::thermal::{Scheme, Stepping};
use crate::units::Length;
use num_complex::Complex64 as c64;

#[test]
#[ignore = "prints the cases' numbers"]
fn print_the_numbers() {
    println!("two layers {:?}", checks::two_layers());
    for n in [20, 40, 80] {
        println!("heated slab {n}: {:e}", checks::heated_slab(n));
    }
    println!("parallel plate {:e}", checks::parallel_plate());
    for n in [48, 96, 192, 384] {
        println!("coaxial {n}: {:e}", checks::coaxial(n));
    }
    for n in [100, 200, 400] {
        println!("line source {n}: {:e}", checks::line_source(n));
    }
    for (h, r) in [
        (0.008, 200.0),
        (0.004, 200.0),
        (0.002, 200.0),
        (0.001, 200.0),
        (0.001, 100.0),
        (0.001, 400.0),
    ] {
        println!("coplanar {h} {r}: {:?}", checks::coplanar(h, r));
    }
    for (h, q, l) in [
        (0.002, 1.15, 20.0),
        (0.002, 1.08, 20.0),
        (0.0005, 1.04, 5.0),
    ] {
        println!(
            "coplanar graded {h} {q} {l}: {:?}",
            checks::coplanar_graded(h, 200.0, q, l)
        );
    }
    println!("resistor {:?}", checks::resistor());
    println!("soi heater {:?}", checks::soi_heater());
}

#[test]
fn every_case_passes() {
    for case in checks::cases() {
        let o = (case.run)();
        assert!(o.passed(), "{}: {o:?}", case.id);
    }
}

#[test]
fn a_layered_slab_is_exact() {
    let (t, flux) = checks::two_layers();
    assert!(t <= 1e-12 && flux <= 1e-12, "{t:e} {flux:e}");
}

#[test]
fn the_heated_slab_converges_at_second_order() {
    let p = order_of(checks::heated_slab, 40);
    assert!((p - 2.0).abs() < 0.05, "{p}");
}

#[test]
fn a_graded_axis_holds_its_lines_and_grows_by_its_ratio() {
    let g = Grading::new(1.0, 0.01, 1.2).unwrap();
    let a = Axis::graded(-10.0, 10.0, &g, &[0.0, 0.37], &[0.0]).unwrap();
    let n = a.nodes();
    assert_eq!(n[0], -10.0);
    assert_eq!(*n.last().unwrap(), 10.0);
    assert!(n.contains(&0.0) && n.contains(&0.37));
    let (lo, hi) = a.steps();
    // a sliver before a line is shared with the step before it, so a cell can be smaller
    assert!((0.005..=0.0101).contains(&lo), "{lo}");
    assert!(hi <= 1.0 + 1e-12, "{hi}");
    for i in 1..a.cells() {
        let r = a.width(i) / a.width(i - 1);
        // the growth, or a sliver shared before a line
        assert!(
            (1.0 / (2.2 * 1.0001)..=1.2 * 1.0001 + 1.0).contains(&r),
            "{i}: {r}"
        );
    }
}

#[test]
fn a_uniform_grading_is_uniform() {
    let a = Axis::graded(0.0, 1.0, &Grading::uniform(0.1).unwrap(), &[], &[]).unwrap();
    assert_eq!(a.cells(), 10);
}

#[test]
fn a_solve_refuses_nothing_holding_the_level_and_names_a_medium_without_a_coefficient() {
    let grid = Grid::new(
        Axis::uniform(0.0, 1.0, 4).unwrap(),
        Axis::uniform(0.0, 1.0, 4).unwrap(),
    );
    // no side holds the temperature
    let mut heat = crate::thermal::Heat::new(Section::new("a"), grid.clone());
    heat.conductivity("a", Tensor::isotropic(1.0));
    let e = heat.solve().unwrap_err().to_string();
    assert!(e.contains("nothing holds"), "{e}");
    // a layer of a medium given no conductivity
    let mut section = Section::new("a");
    section.layer(0.5, 1.0, "b").unwrap();
    let mut heat = crate::thermal::Heat::new(section, grid);
    heat.conductivity("a", Tensor::isotropic(1.0))
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    let e = heat.solve().unwrap_err().to_string();
    assert!(
        e.contains("the medium b has no thermal conductivity"),
        "{e}"
    );
}

#[test]
fn the_same_bits_on_one_thread_and_many() {
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| {
            (
                checks::soi_heater(),
                checks::coaxial(96),
                checks::coplanar_graded(0.008, 50.0, 1.15, 5.0),
                checks::slab_transient(40, Scheme::Bdf2, 50, 0.5e-6),
                checks::slab_transient(40, Scheme::CrankNicolson, 50, 0.5e-6),
            )
        });
    let many = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap()
        .install(|| {
            (
                checks::soi_heater(),
                checks::coaxial(96),
                checks::coplanar_graded(0.008, 50.0, 1.15, 5.0),
                checks::slab_transient(40, Scheme::Bdf2, 50, 0.5e-6),
                checks::slab_transient(40, Scheme::CrankNicolson, 50, 0.5e-6),
            )
        });
    assert_eq!(one.0.0.to_bits(), many.0.0.to_bits());
    for (a, b) in one.3.iter().chain(&one.4).zip(many.3.iter().chain(&many.4)) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
    assert_eq!(one.0.1.to_bits(), many.0.1.to_bits());
    assert_eq!(one.1.to_bits(), many.1.to_bits());
    assert_eq!(one.2.0.to_bits(), many.2.0.to_bits());
}

/// A column of two media, 1 µm wide, 0 K at the bottom, a heater in the upper one.
fn column() -> (crate::thermal::Heat, Shape) {
    let mut section = Section::new("a");
    section.layer(1.0, 2.0, "b").unwrap();
    let grid = Grid::new(
        Axis::uniform(0.0, 1.0, 2).unwrap(),
        Axis::uniform(0.0, 2.0, 16).unwrap(),
    );
    let upper = Shape::rect(Point::um(0.5, 1.5), Length::um(1.0), Length::um(1.0)).unwrap();
    let mut heat = crate::thermal::Heat::new(section, grid);
    heat.conductivity("a", Tensor::isotropic(1.4))
        .conductivity("b", Tensor::isotropic(20.0))
        .heat_capacity("a", 1.6e6)
        .heat_capacity("b", 1.7e6)
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .heater(upper.clone(), 1.0);
    (heat, upper)
}

#[test]
fn a_turn_off_is_the_steady_state_less_a_turn_on() {
    let (heat, upper) = column();
    let steady = heat.solve().unwrap().mean(&upper).unwrap();
    for scheme in [Scheme::Bdf2, Scheme::CrankNicolson] {
        let stepping = Stepping {
            step_s: 1e-7,
            steps: 40,
            scheme,
        };
        let watch = std::slice::from_ref(&upper);
        let on = heat.transient(&stepping, 0.0, &|_| 1.0, watch).unwrap();
        let off = heat.transient(&stepping, 1.0, &|_| 0.0, watch).unwrap();
        assert_eq!(on.times_s.len(), 41);
        assert_eq!(on.watched[0][0], 0.0);
        assert!((off.watched[0][0] - steady).abs() < 1e-12 * steady);
        for (a, b) in on.watched[0].iter().zip(&off.watched[0]) {
            assert!(
                (a + b - steady).abs() < 1e-10 * steady,
                "{scheme:?}: {a} + {b}"
            );
        }
        // the rise and the fall cross 1/e at the same time
        let e = 1.0 - (-1.0f64).exp();
        let (rise, fall) = (
            on.crossing(0, 0.0, steady, e).unwrap(),
            off.crossing(0, steady, 0.0, e).unwrap(),
        );
        assert!((rise - fall).abs() < 1e-9 * rise, "{rise} {fall}");
        // held at its drive, a transient stays at its steady state
        let held = heat.transient(&stepping, 1.0, &|_| 1.0, watch).unwrap();
        for v in &held.watched[0] {
            assert!((v - steady).abs() < 1e-10 * steady);
        }
        assert!((held.last.imbalance()).abs() < 1e-9);
    }
}

#[test]
fn a_response_is_one_at_zero_frequency_and_falls() {
    let (heat, upper) = column();
    let dc = heat.response(0.0, &upper).unwrap();
    assert!((dc - c64::new(1.0, 0.0)).norm() < 1e-12);
    let cutoff = heat.cutoff(&upper).unwrap();
    let at = heat.response(cutoff, &upper).unwrap();
    assert!(
        (at.norm() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-5,
        "{at}"
    );
    // lagging: the temperature follows the power late
    assert!(at.im < 0.0, "{at}");
    assert!(heat.response(10.0 * cutoff, &upper).unwrap().norm() < 0.2);
}

#[test]
fn a_transient_names_what_it_lacks() {
    let (heat, upper) = column();
    let watch = std::slice::from_ref(&upper);
    let bad = Stepping {
        step_s: 0.0,
        steps: 4,
        scheme: Scheme::Bdf2,
    };
    let e = heat
        .transient(&bad, 0.0, &|_| 1.0, watch)
        .unwrap_err()
        .to_string();
    assert!(e.contains("positive step"), "{e}");
    let good = Stepping {
        step_s: 1e-8,
        ..bad
    };
    let e = heat
        .transient(&good, 0.0, &|_| f64::NAN, watch)
        .unwrap_err()
        .to_string();
    assert!(e.contains("drive must be finite"), "{e}");
    let outside = Shape::rect(Point::um(9.0, 9.0), Length::um(1.0), Length::um(1.0)).unwrap();
    let e = heat
        .transient(&good, 0.0, &|_| 1.0, std::slice::from_ref(&outside))
        .unwrap_err()
        .to_string();
    assert!(e.contains("covers no cell"), "{e}");
    let e = heat.response(-1.0, &upper).unwrap_err().to_string();
    assert!(e.contains("non-negative"), "{e}");
    let mut section = Section::new("a");
    section.layer(1.0, 2.0, "b").unwrap();
    let mut lacking = crate::thermal::Heat::new(section, heat.grid().clone());
    lacking
        .conductivity("a", Tensor::isotropic(1.0))
        .conductivity("b", Tensor::isotropic(1.0))
        .heat_capacity("a", 1e6)
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    let e = lacking
        .transient(&good, 0.0, &|_| 1.0, watch)
        .unwrap_err()
        .to_string();
    assert!(e.contains("the medium b has no heat capacity"), "{e}");
}

#[test]
fn a_crossing_interpolates_between_steps() {
    let history = crate::thermal::History {
        times_s: vec![0.0, 1.0, 2.0, 3.0],
        watched: vec![vec![0.0, 0.2, 0.6, 0.9]],
        last: column().0.solve().unwrap(),
    };
    assert!((history.crossing(0, 0.0, 1.0, 0.5).unwrap() - 1.75).abs() < 1e-12);
    assert_eq!(history.crossing(0, 0.0, 1.0, 0.0), Some(0.0));
    assert_eq!(history.crossing(0, 0.0, 1.0, 0.95), None);
    assert_eq!(history.crossing(1, 0.0, 1.0, 0.5), None);
    // falling
    let falling = crate::thermal::History {
        watched: vec![vec![1.0, 0.8, 0.4, 0.1]],
        ..history
    };
    assert!((falling.crossing(0, 1.0, 0.0, 0.5).unwrap() - 1.75).abs() < 1e-12);
}

#[test]
fn an_anisotropic_slab_reads_its_axis() {
    // heat down through a slab whose conductivity is 2 across and 5 up: only 5 matters
    let grid = Grid::new(
        Axis::uniform(0.0, 1.0, 3).unwrap(),
        Axis::uniform(0.0, 1.0, 10).unwrap(),
    );
    let mut heat = crate::thermal::Heat::new(Section::new("x"), grid);
    heat.conductivity("x", Tensor::diagonal(2.0, 5.0))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(1.0));
    let t = heat.solve().unwrap();
    // 1 µm wide, 1 µm thick: 5 W/m
    assert!(
        (t.outflow(Side::Bottom) - 5.0).abs() < 1e-12,
        "{}",
        t.outflow(Side::Bottom)
    );
}

#[test]
fn a_convective_side_takes_its_film() {
    // a slab between a fixed bottom at 1 and a film to 0 at the top: the series resistance
    // d/k + 1/h per unit area
    let (k, d, h) = (1.4, 2.0, 1e5);
    let grid = Grid::new(
        Axis::uniform(0.0, 1.0, 1).unwrap(),
        Axis::uniform(0.0, d, 8).unwrap(),
    );
    let mut heat = crate::thermal::Heat::new(Section::new("x"), grid);
    heat.conductivity("x", Tensor::isotropic(k))
        .boundary(Side::Bottom, Boundary::Fixed(1.0))
        .boundary(
            Side::Top,
            Boundary::Convective {
                coefficient: h,
                ambient: 0.0,
            },
        );
    let t = heat.solve().unwrap();
    // per µm of width: 1 K over (d µm·1e-6 / k + 1/h) m²K/W, times 1e-6 m
    let exact = 1e-6 / (d * 1e-6 / k + 1.0 / h);
    assert!(
        (t.outflow(Side::Top) - exact).abs() < 1e-12 * exact,
        "{} {exact}",
        t.outflow(Side::Top)
    );
}

#[test]
fn a_cut_reads_the_stack_and_its_strip() {
    let mut structure = crate::stack::Structure::new(crate::stack::LayerStack::soi_220());
    structure
        .draw(
            "Si",
            Shape::rect(Point::um(0.0, 0.0), Length::um(10.0), Length::um(0.5)).unwrap(),
        )
        .unwrap();
    let section = Section::cut(&structure, Cut::X(0.0)).unwrap();
    assert_eq!(section.medium_at(0.0, 2.1), "Si");
    assert_eq!(section.medium_at(0.4, 2.1), "SiO2");
    assert_eq!(section.medium_at(0.0, -1.0), "Si");
    assert_eq!(section.medium_at(0.0, 1.0), "SiO2");
    let f = section.features();
    assert!(f.xs.iter().any(|&x| (x - 0.25).abs() < 1e-9), "{:?}", f.xs);
    assert!(f.ys.iter().any(|&y| (y - 2.22).abs() < 1e-12), "{:?}", f.ys);
}

#[test]
fn elliptic_k_matches_its_values() {
    // K(0) = π/2, K(1/√2) = Γ(1/4)² / (4√π) = 1.854074677301372
    assert!((elliptic_k(0.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
    assert!((elliptic_k(std::f64::consts::FRAC_1_SQRT_2) - 1.854_074_677_301_372).abs() < 1e-14);
}

#[test]
fn a_layered_capacitors_field_is_each_layers_own() {
    use crate::electro::{Conductor, Electrostatic};
    // plates at y = -1 and 1 µm, silica below y = 0 and a crystal of 28 above, 2 V across
    let mut section = Section::new("oxide");
    section.layer(0.0, 1.0, "crystal").unwrap();
    let features = section.features().clone();
    let grid = Grid::fitted(
        [-1.0, 1.0],
        [-1.0, 1.0],
        &Grading::uniform(0.1).unwrap(),
        &features,
    )
    .unwrap();
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("oxide", Tensor::isotropic(3.9))
        .permittivity("crystal", Tensor::diagonal(5.0, 28.0))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(2.0));
    let phi = problem.solve().unwrap();
    let e = problem.electric_field(&phi).unwrap();
    // D = V / Σ t/ε, each layer's E = −D/ε (the potential rising upward)
    let d = 2.0 / ((1.0 / 3.9 + 1.0 / 28.0) * 1e-6);
    for (y, eps) in [(-0.95, 3.9), (-0.05, 3.9), (0.05, 28.0), (0.95, 28.0)] {
        let [ex, ey] = e.at(0.3, y);
        assert!((ey / (-d / eps) - 1.0).abs() < 1e-12, "{y}: {ey}");
        assert!(ex.abs() < 1e-9 * d / eps);
    }
    // a central difference across the interface mixes the two layers
    let j = e.grid().y().cells() / 2;
    let mixed = phi.gradient(5, j)[1];
    assert!((mixed / (d / 28.0) - 1.0).abs() > 0.1);
    // a conductor's inside has no field, and a field from another grid is refused
    let mut held = problem.clone();
    let plate = Shape::rect(Point::um(0.0, -0.5), Length::um(4.0), Length::um(0.2)).unwrap();
    held.conductor(Conductor::shape(plate, 1.0));
    let phi_held = held.solve().unwrap();
    let inside = held.electric_field(&phi_held).unwrap().at(0.0, -0.5);
    assert_eq!(inside, [0.0, 0.0]);
    let other = Grid::new(
        Axis::uniform(-1.0, 1.0, 3).unwrap(),
        Axis::uniform(-1.0, 1.0, 3).unwrap(),
    );
    let mut elsewhere = Electrostatic::new(Section::new("oxide"), other);
    elsewhere
        .permittivity("oxide", Tensor::isotropic(3.9))
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    assert!(elsewhere.electric_field(&phi).is_err());
}
