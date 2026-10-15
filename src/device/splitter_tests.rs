//! The rib, the MMIs, the Y-branch and the MZIs: their geometry, their models and their
//! bits.

use std::time::Instant;

use super::*;
use crate::circuit::Fidelity;

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).unwrap()
}

/// A port's name, position (µm), direction and width.
fn port(g: &Geometry, name: &str) -> (f64, f64, f64, f64) {
    let p = g.ports.iter().find(|p| p.name == name).unwrap();
    (
        p.position.x.to_um(),
        p.position.y.to_um(),
        p.direction,
        p.width.to_um(),
    )
}

fn close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9 && a.2 == b.2 && (a.3 - b.3).abs() < 1e-9
}

#[test]
fn every_new_kind_is_a_job_checked_and_drawn() {
    for (kind, parameters) in [
        (
            "rib waveguide",
            "etch_depth = \"70 nm\"\nsidewall = \"80 deg\"",
        ),
        (
            "MMI 1x2",
            "width = \"4 um\"\ntaper_length = \"3\"\nport_width = \"0.5\"",
        ),
        ("MMI 2x2", "w = \"5 um\"\nwidth = \"w\""),
        ("Y-branch", "w7 = \"w6\"\nseparation = \"3\""),
        ("MZI", "upper = \"lower + 40\""),
        ("Y-branch MZI", "phase = \"45 deg\""),
    ] {
        let text = format!(
            "name = \"d\"\n\n[device]\nkind = \"{kind}\"\n\n[device.parameters]\n{parameters}\n"
        );
        let job = crate::run::Job::parse(&text).unwrap();
        crate::job::check(&job).unwrap_or_else(|e| panic!("{kind}: {e}"));
        let crate::job::Event::Scene { shapes, .. } = crate::job::preview(&job).unwrap() else {
            panic!("{kind}: not a scene")
        };
        assert!(!shapes.is_empty(), "{kind}");
        // the [device] table reads back to the same design
        let spec = Spec::from_table(job.device().unwrap()).unwrap();
        assert_eq!(Spec::from_table(&spec.to_table()).unwrap(), spec);
        let (device, _) = spec.build().unwrap();
        assert_eq!(device.kind(), kind);
    }
    // a fidelity the device hasn't is refused
    let text = "name = \"d\"\n[device]\nkind = \"MMI 1x2\"\nfidelity = \"analytic\"\n";
    let job = crate::run::Job::parse(text).unwrap();
    assert!(crate::job::check(&job).is_err());
}

#[test]
fn the_rib_is_etched_part_of_the_way_and_solved_on_its_half() {
    let rib = RibWaveguide::new();
    let v = Values::defaults(&rib);
    let g = rib.geometry(&v).unwrap();
    let etch = g.structure.etches("Si")[0];
    assert!((etch.depth.unwrap().to_um() - 0.13).abs() < 1e-15);
    // its cross-section: the half from the vertical plane, over the whole height, the slab's
    // top on a node
    let cs = rib.cross_section(&v, um(1.55)).unwrap();
    assert_eq!(cs.x()[0], 0.0);
    let y = cs.y();
    assert!((y[0] + y[y.len() - 1]).abs() < 1e-12);
    let slab_top = -0.11 + 0.09;
    assert!(y.iter().any(|&v| (v - slab_top).abs() < 1e-12), "{y:?}");
    // a rib etched through is the waveguide's
    let through = Values::new(&rib, &[("etch_depth", 0.22)]).unwrap();
    let e = rib.component(&through, Fidelity::TwoD).unwrap_err();
    assert!(e.to_string().contains("the waveguide"), "{e}");
    // its 2D model is labelled 2D, with the slab's index beside it
    let b = rib.component(&v, Fidelity::TwoD).unwrap();
    assert!(b.figures.iter().all(|f| f.fidelity == Fidelity::TwoD));
    let (n, slab) = (
        b.figure("n_eff").unwrap().value,
        b.figure("n_slab").unwrap().value,
    );
    assert!(n > slab && slab > 1.444, "{n} {slab}");
    assert_eq!(b.component.provenance().fidelity, Fidelity::TwoD);
    assert!(rib.component(&v, Fidelity::Analytic).is_err());
}

#[test]
fn the_waveguide_and_coupler_take_ribs_but_not_at_their_fits() {
    let guide = StripWaveguide::new();
    let rib = Values::new(&guide, &[("slab", 0.09)]).unwrap();
    let e = guide.component(&rib, Fidelity::Analytic).unwrap_err();
    assert!(e.to_string().contains("3D"), "{e}");
    let g = guide.geometry(&rib).unwrap();
    assert!(g.structure.etches("Si")[0].depth.is_some());
    // the cross-section: on the half, as the rib device's
    let cs = guide.cross_section(&rib, um(1.55)).unwrap();
    let y = cs.y();
    assert!((y[0] + y[y.len() - 1]).abs() < 1e-12);
    // a coupler of ribs: its pair on the half, an electric and a magnetic wall between them
    let coupler = StripCoupler::new().with_mesh(Mesh::fitted(0.04, 0.5, Scheme::Standard));
    let v = Values::new(&coupler, &[("slab", 0.09), ("sidewall", 85.0)]).unwrap();
    let (even, odd) = coupler.cross_sections(&v, um(1.55)).unwrap();
    assert_eq!(even.x()[0], 0.0);
    assert_eq!(even.y(), odd.y());
    assert!(coupler.component(&v, Fidelity::Analytic).is_err());
    // the strips themselves are as they were
    let strip = Values::defaults(&guide);
    assert_eq!(strip.get("slab"), Some(0.0));
    assert_eq!(strip.get("sidewall"), Some(90.0));
}

#[test]
fn the_mmis_ports_are_the_circuits() {
    let one = StripMmi::one_by_two();
    let v = Values::new(&one, &[("width", 5.0), ("length", 20.0)]).unwrap();
    let g = one.geometry(&v).unwrap();
    let names: Vec<&str> = g.ports.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["o1", "o2", "o3"]);
    assert!(close(port(&g, "o1"), (0.0, 0.0, 180.0, 1.5)));
    assert!(close(port(&g, "o2"), (20.0, 1.25, 0.0, 1.5)));
    assert!(close(port(&g, "o3"), (20.0, -1.25, 0.0, 1.5)));
    let (mmi, _) = one.guided(&v).unwrap();
    let names: Vec<&str> = mmi.ports().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["o1", "o2", "o3"]);
    // the 2 x 2's at ±W/6, tapered to its ports
    let two = StripMmi::two_by_two();
    let v = Values::new(
        &two,
        &[
            ("width", 6.0),
            ("length", 40.0),
            ("taper_length", 5.0),
            ("port_width", 0.5),
        ],
    )
    .unwrap();
    let g = two.geometry(&v).unwrap();
    assert!(close(port(&g, "o1"), (-5.0, -1.0, 180.0, 0.5)));
    assert!(close(port(&g, "o2"), (-5.0, 1.0, 180.0, 0.5)));
    assert!(close(port(&g, "o3"), (45.0, 1.0, 0.0, 0.5)));
    assert!(close(port(&g, "o4"), (45.0, -1.0, 0.0, 0.5)));
    // its default length is its first two-fold image at 1.55 µm
    let d = Values::defaults(&two);
    let b = two.component(&d, Fidelity::TwoD).unwrap();
    let image = b.figure("L_image").unwrap().value;
    assert!((image - d.get("length").unwrap()).abs() < 1e-12);
    // on its figures, the fidelity and grid, 2D
    assert!(
        b.figures
            .iter()
            .all(|f| f.fidelity == Fidelity::TwoD && f.grid.contains("2D"))
    );
}

#[test]
fn the_y_branch_draws_zhangs_spline() {
    let y = StripYBranch::new();
    let v = Values::defaults(&y);
    let (outline, parameters) = y.outline(&v).unwrap();
    for (k, w) in super::ybranch::ZHANG.iter().enumerate() {
        let x = 2.0 * k as f64 / 12.0;
        let at = parameters.at_coordinate("x", x).unwrap();
        assert!((outline.eval(at.values()) - w).abs() < 1e-14, "{k}");
    }
    assert!(outline.text().starts_with("spline(x, ["));
    let g = y.geometry(&v).unwrap();
    assert!(close(port(&g, "o1"), (0.0, 0.0, 180.0, 0.5)));
    assert!(close(port(&g, "o2"), (7.0, 1.0, 0.0, 0.5)));
    assert!(close(port(&g, "o3"), (7.0, -1.0, 0.0, 0.5)));
    // an end too narrow for the arms is refused
    let narrow = Values::new(&y, &[("w13", 0.9)]).unwrap();
    assert!(y.geometry(&narrow).is_err());
    // the ideal model at its defaults
    let b = y.component(&v, Fidelity::Analytic).unwrap();
    assert_eq!(b.values, [0.0]);
    assert!(b.figure("excess_loss").unwrap().value.abs() < 1e-14);
}

#[test]
fn the_mzis_are_their_parts_netlists() {
    for (mzi, ports) in [
        (StripMzi::couplers(), vec!["o1", "o2", "o3", "o4"]),
        (StripMzi::y_branches(), vec!["o1", "o2"]),
    ] {
        let fidelities: Vec<Fidelity> = mzi.fidelities().iter().map(|m| m.fidelity).collect();
        assert_eq!(fidelities, [Fidelity::Analytic, Fidelity::ThreeD]);
        let v = Values::defaults(&mzi);
        let g = mzi.geometry(&v).unwrap();
        let names: Vec<&str> = g.ports.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ports);
        let b = mzi.component(&v, Fidelity::Analytic).unwrap();
        assert_eq!(b.component.ports().len(), ports.len());
        // the outputs face out on the right, level with the inputs' mirror image
        let right: Vec<&Port> = g.ports.iter().filter(|p| p.direction == 0.0).collect();
        assert!(right.windows(2).all(|w| w[0].position.x == w[1].position.x));
        // a longer upper arm turns the outputs' phase
        let longer = v.with(&mzi, "upper", 160.0).unwrap();
        let s = mzi
            .component(&longer, Fidelity::Analytic)
            .unwrap()
            .s_matrix(um(1.55))
            .unwrap();
        assert_ne!(s, b.s_matrix(um(1.55)).unwrap());
    }
}

/// The surveys behind docs/methods/devices.md's numbers, too heavy for a check:
///
/// ```sh
/// PART=smdy cargo test --release --lib device::splitter_tests::survey -- --ignored --nocapture
/// PART=3 GRIDS=0.1:0.055,0.08:0.044,0.0667:0.0367 cargo test --release --lib \
///     device::splitter_tests::survey -- --ignored --nocapture
/// ```
///
/// `s`: a straight strip through the FDFD solves, 2D and 3D; `m`: a 3 µm MMI's guided modes
/// and 2D FDFD at 20, 10 and 5 nm; `d`: the default MMIs' guided modes and 2D FDFD at 20 and
/// 10 nm; `y`: Zhang's Y-branch's 2D FDFD at 20, 10 and 5 nm; `3`: the 3 µm MMI's 3D FDFD on
/// `GRIDS` (in-plane:vertical steps, µm), `W` its width (3 µm).
#[test]
#[ignore = "a survey, not a check"]
fn survey() {
    let part = std::env::var("PART").unwrap_or_default();
    if part.contains('s') {
        let guide = StripWaveguide::new();
        let v = Values::new(&guide, &[("length", 2.0)]).unwrap();
        let g = guide.geometry(&v).unwrap();
        for three in [false, true] {
            let grid = FdfdGrid::new(0.1)
                .with_step_z(0.055)
                .with_margin(0.4)
                .with_pml(8)
                .with_samples(1);
            let (s, d) = super::solve::sampled("w", &g, "Si", &[um(1.55)], &grid, three).unwrap();
            let m = s.s_matrix(um(1.55), &[]).unwrap();
            let n = s.ports()[0].mode.as_ref().unwrap().effective_index;
            let expect = crate::portable::exp_c(num_complex::Complex64::new(
                0.0,
                2.0 * std::f64::consts::PI * n / 1.55 * 2.0,
            ));
            println!(
                "{d}: |S21|² {} |S11|² {:e} reciprocity {:e} S21 - e^(ibL) {:e}",
                m.power(1, 0),
                m.power(0, 0),
                m.reciprocity_error(),
                (m[(1, 0)] - expect).norm()
            );
        }
    }
    if part.contains('m') {
        let mmi = StripMmi::one_by_two();
        let v = Values::new(
            &mmi,
            &[("width", 3.0), ("access_width", 0.5), ("port_width", 0.5)],
        )
        .unwrap();
        let image = mmi
            .component(&v, Fidelity::TwoD)
            .unwrap()
            .figure("L_image")
            .unwrap()
            .value;
        let v = v.with(&mmi, "length", image).unwrap();
        let b = mmi.component(&v, Fidelity::TwoD).unwrap();
        println!(
            "MMI W3 L {image}: guided {:?}",
            b.figures.iter().map(|f| f.value).collect::<Vec<_>>()
        );
        for step in [0.02, 0.01, 0.005] {
            let t = Instant::now();
            let d = mmi
                .clone()
                .with_model_2d(Model2d::Fdfd)
                .with_grid_2d(FdfdGrid::new(step).with_samples(1));
            let b = d.component(&v, Fidelity::TwoD).unwrap();
            let s = b.s_matrix(um(1.55)).unwrap();
            println!(
                "MMI W3 2D {step}: {:?} reciprocity {:e} {:?}",
                b.figures.iter().map(|f| f.value).collect::<Vec<_>>(),
                s.reciprocity_error(),
                t.elapsed()
            );
        }
    }
    if part.contains('d') {
        for mmi in [StripMmi::one_by_two(), StripMmi::two_by_two()] {
            let v = Values::defaults(&mmi);
            let b = mmi.component(&v, Fidelity::TwoD).unwrap();
            println!(
                "{} guided {:?}",
                mmi.kind(),
                b.figures.iter().map(|f| f.value).collect::<Vec<_>>()
            );
            for step in [0.02, 0.01] {
                let t = Instant::now();
                let d = mmi
                    .clone()
                    .with_model_2d(Model2d::Fdfd)
                    .with_grid_2d(FdfdGrid::new(step).with_samples(1));
                let b = d.component(&v, Fidelity::TwoD).unwrap();
                println!(
                    "{} 2D {step}: {:?} {:?}",
                    mmi.kind(),
                    b.figures.iter().map(|f| f.value).collect::<Vec<_>>(),
                    t.elapsed()
                );
            }
        }
    }
    if part.contains('y') {
        let y = StripYBranch::new();
        let v = Values::new(&y, &[("arm_length", 3.0), ("separation", 1.5)]).unwrap();
        for step in [0.02, 0.01, 0.005] {
            let t = Instant::now();
            let b = y
                .clone()
                .with_grid_2d(FdfdGrid::new(step).with_samples(1))
                .component(&v, Fidelity::TwoD)
                .unwrap();
            println!(
                "Y 2D {step}: {:?} {:?}",
                b.figures.iter().map(|f| f.value).collect::<Vec<_>>(),
                t.elapsed()
            );
        }
    }
    if part.contains('3') {
        let grids: Vec<(f64, f64)> = std::env::var("GRIDS")
            .unwrap_or_else(|_| "0.1:0.055".into())
            .split(',')
            .map(|p| {
                let (a, b) = p.split_once(':').unwrap();
                (a.parse().unwrap(), b.parse().unwrap())
            })
            .collect();
        let width: f64 = std::env::var("W").map_or(3.0, |w| w.parse().unwrap());
        let mmi = StripMmi::one_by_two();
        let v = Values::new(
            &mmi,
            &[("width", width), ("access_width", 0.5), ("port_width", 0.5)],
        )
        .unwrap();
        let image = mmi
            .component(&v, Fidelity::TwoD)
            .unwrap()
            .figure("L_image")
            .unwrap()
            .value;
        let v = v.with(&mmi, "length", image).unwrap();
        for (step, z) in grids {
            let t = Instant::now();
            let d = mmi.clone().with_grid_3d(
                FdfdGrid::new(step)
                    .with_step_z(z)
                    .with_margin(0.4)
                    .with_pml(8)
                    .with_samples(1),
            );
            let b = d.component(&v, Fidelity::ThreeD).unwrap();
            let s = b.s_matrix(um(1.55)).unwrap();
            println!(
                "MMI W{width} 3D {step} {z}: {:?} {:?} {} reciprocity {:e}",
                b.figures.iter().map(|f| f.value).collect::<Vec<_>>(),
                t.elapsed(),
                b.figures[0].grid,
                s.reciprocity_error()
            );
        }
    }
}
