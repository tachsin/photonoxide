use super::checks::{self, order_of};
use super::*;

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
fn nothing_holding_the_level_is_refused() {
    let grid = Grid::new(
        Axis::uniform(0.0, 1.0, 4).unwrap(),
        Axis::uniform(0.0, 1.0, 4).unwrap(),
    );
    let mut heat = crate::thermal::Heat::new(Section::new("a"), grid);
    heat.conductivity("a", Tensor::isotropic(1.0));
    let e = heat.solve().unwrap_err().to_string();
    assert!(e.contains("nothing holds"), "{e}");
}

#[test]
fn a_medium_without_a_coefficient_is_named() {
    let grid = Grid::new(
        Axis::uniform(0.0, 1.0, 4).unwrap(),
        Axis::uniform(0.0, 1.0, 4).unwrap(),
    );
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
            )
        });
    assert_eq!(one.0.0.to_bits(), many.0.0.to_bits());
    assert_eq!(one.0.1.to_bits(), many.0.1.to_bits());
    assert_eq!(one.1.to_bits(), many.1.to_bits());
    assert_eq!(one.2.0.to_bits(), many.2.0.to_bits());
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
#[ignore = "writes the static cases' rows of the report to $PHOTONOXIDE_STATIC_ROWS"]
fn write_the_rows() {
    let Some(out) = std::env::var_os("PHOTONOXIDE_STATIC_ROWS") else {
        return;
    };
    let mut rows = String::new();
    for case in checks::cases() {
        let start = std::time::Instant::now();
        let (row, passed) = crate::validation::row(&case);
        eprintln!(
            "{:6.2} s {} {passed}",
            start.elapsed().as_secs_f64(),
            case.id
        );
        rows.push_str(&row);
    }
    std::fs::write(out, rows).unwrap();
}
