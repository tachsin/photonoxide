//! The rib, the MMIs, the Y-branch and the MZIs: their geometry, their models and their
//! bits.

use std::time::Instant;

use super::*;
use crate::circuit::Fidelity;
use crate::portable;

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
            let expect = crate::portable::complex::exp(num_complex::Complex64::new(
                0.0,
                2.0 * std::f64::consts::PI * n / 1.55 * 2.0,
            ));
            println!(
                "{d}: |S21|² {} |S11|² {:e} reciprocity {:e} S21 - e^(ibL) {:e}",
                m.power(1, 0),
                m.power(0, 0),
                m.reciprocity_error(),
                portable::complex::abs(m[(1, 0)] - expect)
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

/// A device's 3D FDFD solves directly up to 1.2 million unknowns and iteratively above, to its
/// stated residual by the default multigrid; it refuses what the iterative solve can't hold here.
#[test]
fn a_3d_solve_is_direct_up_to_its_size_and_iterative_above() {
    use super::solve::{RESIDUAL_3D, Route, check_size};
    assert_eq!(Route::of(1.2e6), Route::Direct);
    assert_eq!(
        Route::of(1.2e6 + 3.0),
        Route::Iterative {
            multigrid: crate::fdfd::Multigrid::default(),
            residual: RESIDUAL_3D
        }
    );
    assert!(check_size(1.2e7).is_ok());
    let e = check_size(1e9).unwrap_err().to_string();
    assert!(e.contains("million unknowns"), "{e}");
}

/// The 3D grid study's runs, one a process (its peak memory the process's), by hand:
///
/// ```sh
/// DEV=mmi GRID=0.044:0.022 PML=8 MARGIN=0.4 ROUTE=auto cargo test --release --lib \
///     device::splitter_tests::survey_3d -- --ignored --nocapture
/// ```
///
/// `DEV` as [`super::splitter_checks::splitter_3d`], `LENGTH` its length instead (µm, the MMI's
/// length sweep); `GRID` in-plane:vertical steps (µm); `ROUTE` `auto` (the device's
/// own), `direct`, `stretched` (the direct solver in the iterative route's PMLs), `galerkin` or
/// `rediscretized` (the iterative route with those coarse operators), `RES` its residual; and
/// `2d`, the 2D models of the same design (guided modes, and 2D FDFD on `GRID`'s in-plane steps,
/// comma-separated). Prints one line a run, `LOG` appended with it too.
#[test]
#[ignore = "a survey, not a check"]
fn survey_3d() {
    use crate::fdfd::{Coarse, Multigrid};
    use std::io::Write;
    let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let which = var("DEV", "mmi");
    let (device, v) = super::splitter_checks::splitter_3d(&which);
    let v = match std::env::var("LENGTH") {
        Ok(l) => v
            .with(device.as_ref(), "length", l.parse().unwrap())
            .unwrap(),
        Err(_) => v,
    };
    let which = format!("{which} L {:.4}", v.at("length"));
    let route = var("ROUTE", "auto");
    let log = |line: &str| {
        println!("{line}");
        if let Ok(path) = std::env::var("LOG") {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(f, "{line}").unwrap();
        }
    };
    let lam = um(1.55);
    let geometry = device.geometry(&v).unwrap();
    let core = "Si";
    let report = |s: &crate::circuit::SMatrix| {
        if s.size() == 2 {
            return format!(
                "T {:.6} R {:.2e} reciprocity {:.1e}",
                s.power(1, 0),
                s.power(0, 0),
                s.reciprocity_error()
            );
        }
        let (a, b) = (s.power(1, 0), s.power(2, 0));
        format!(
            "P_o2 {a:.6} P_o3 {b:.6} sum {:.6} excess {:.5} dB imbalance {:.2e} dB R {:.2e} reciprocity {:.1e}",
            a + b,
            -10.0 * portable::log10(a + b),
            10.0 * portable::log10(a / b),
            s.power(0, 0),
            s.reciprocity_error()
        )
    };
    if route == "2d" {
        let b = device.component(&v, Fidelity::TwoD).unwrap();
        let s = b.s_matrix(lam).unwrap();
        log(&format!("{which} 2D default model: {}", report(&s)));
        for step in var("GRID", "0.02,0.01,0.005").split(',') {
            let h: f64 = step.parse().unwrap();
            let t = Instant::now();
            let g = FdfdGrid::new(h)
                .with_samples(1)
                .with_averaging(crate::averaging::Averaging::Diagonal);
            let (sampled, _) =
                super::solve::sampled("m", &geometry, core, &[lam], &g, false).unwrap();
            let s = sampled.s_matrix(lam, &[]).unwrap();
            log(&format!(
                "{which} 2D FDFD {h}: {} {:?}",
                report(&s),
                t.elapsed()
            ));
        }
        return;
    }
    let (h, hz) = var("GRID", "0.044:0.022")
        .split_once(':')
        .map(|(a, b)| (a.parse::<f64>().unwrap(), b.parse::<f64>().unwrap()))
        .unwrap();
    let pml: usize = var("PML", "8").parse().unwrap();
    let margin: f64 = var("MARGIN", "0.4").parse().unwrap();
    let residual: f64 = var("RES", "1e-9").parse().unwrap();
    let grid = FdfdGrid::new(h)
        .with_step_z(hz)
        .with_margin(margin)
        .with_pml(pml)
        .with_samples(1);
    let iterative = |coarse| super::solve::Route::Iterative {
        multigrid: Multigrid {
            coarse,
            ..Multigrid::default()
        },
        residual,
    };
    let chosen = match route.as_str() {
        "direct" => Some(super::solve::Route::Direct),
        "stretched" => Some(super::solve::Route::DirectStretched),
        "galerkin" => Some(iterative(Coarse::Galerkin)),
        "rediscretized" => Some(iterative(Coarse::Rediscretized)),
        _ => None,
    };
    let (nx, ny, unknowns) = super::solve::size(&geometry, core, &grid, true).unwrap();
    log(&format!(
        "{which} 3D {h}:{hz} PML {pml} margin {margin} {route} res {residual:e}: {nx} x {ny}, {unknowns} unknowns, started"
    ));
    let t = Instant::now();
    let (sampled, description, iterations) =
        super::solve::sampled_by("s", &geometry, core, &[lam], &grid, true, chosen).unwrap();
    let s = sampled.s_matrix(lam, &[]).unwrap();
    log(&format!(
        "{which} 3D {h}:{hz} PML {pml} margin {margin} {route} res {residual:e}: {} iterations {iterations:?} {:.1} s; {description}",
        report(&s),
        t.elapsed().as_secs_f64()
    ));
    // the S-matrix whole, to compare runs by
    let n = s.size();
    let rows: Vec<String> = (0..n)
        .map(|q| {
            (0..n)
                .map(|p| format!("{:.10e}{:+.10e}i", s[(q, p)].re, s[(q, p)].im))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    log(&format!("  S = [{}]", rows.join("; ")));
}

/// The 2 µm MMI's section, the even modes its symmetric input excites, in 2D (the effective
/// index method's plane: the film's TE slab index between the oxide, its lateral TM modes) and
/// in 3D (Hadley's equations on the strip's quarter, cells fitted at `STEPS` µm): each mode's
/// index and the length $\pi/(\beta_0 - \beta_2)$ where modes 0 and 2 are in phase again, the
/// 1 × 2 image's in the quadratic approximation, and how far mode 4 is from it in phase. By hand:
///
/// ```sh
/// cargo test --release --lib device::splitter_tests::survey_mmi_modes -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a survey, not a check"]
fn survey_mmi_modes() {
    use super::guide::{Profile, Strip};
    use super::section::{Mesh, Scheme};
    use crate::mode::Polarization;
    use crate::mode::slab::Slab;
    use crate::units::Length;
    let lam = um(1.55);
    let k = lam.wavenumber();
    let (width, thickness) = (2.0, 0.22);
    let report = |label: &str, n: &[f64]| {
        let image = std::f64::consts::PI / (k * (n[0] - n[1]));
        // mode 4's phase against modes 0 and 2's at that length, in units of π: 0 (mod 2) when
        // the quadratic law holds (β0 − β4 = 3(β0 − β2))
        let slip = n
            .get(2)
            .map(|n4| k * (n[0] - n4) * image / std::f64::consts::PI);
        println!(
            "{label}: even modes {n:.6?}; pi/(b0-b2) {image:.4} um; mode 4's phase there {slip:?} pi"
        );
    };
    let soi = Strip::soi();
    let (below, core, above, _) = super::section::indices(&soi.stack, &soi.core, lam).unwrap();
    let film = Slab::new(below, core, above, Length::um(thickness)).unwrap();
    let ns = film.modes(Polarization::Te, lam)[0].effective_index();
    let lateral = Slab::new(below, ns, below, Length::um(width)).unwrap();
    let all: Vec<f64> = lateral
        .modes(Polarization::Tm, lam)
        .iter()
        .map(|m| m.effective_index())
        .collect();
    println!("2D plane: film's TE index {ns:.6}; lateral TM modes {all:.6?}");
    let even: Vec<f64> = all.iter().step_by(2).copied().collect();
    report("2D, effective index method", &even);
    let steps = std::env::var("STEPS").unwrap_or_else(|_| "0.02,0.01".to_owned());
    let mut three = Vec::new();
    for step in steps.split(',') {
        let h: f64 = step.parse().unwrap();
        let strip = Strip {
            mesh: Mesh::fitted(h, 1.0, Scheme::Hadley),
            ..Strip::soi()
        };
        let cs = strip
            .cross_section_of(width, thickness, Profile::STRIP, lam)
            .unwrap();
        let t = Instant::now();
        let n: Vec<f64> = crate::mode::hadley::modes(&cs, lam, 4, None)
            .unwrap()
            .iter()
            .map(|m| m.effective_index().re)
            .collect();
        report(&format!("3D, Hadley, {h} um ({:.1?})", t.elapsed()), &n);
        three = n;
    }
    // the guided-mode analysis on the plane, its modes' couplings the plane's, but the even
    // modes travelling with the 3D indices; one below the oxide's isn't guided in 3D, left out
    let (_, v) = super::splitter_checks::splitter_3d("mmi");
    let (guided, values) = StripMmi::one_by_two().guided(&v).unwrap();
    let indices: Vec<Option<f64>> = (0..all.len())
        .map(|m| {
            if m % 2 == 1 {
                // odd: the centred input doesn't excite them
                Some(all[m])
            } else {
                three.get(m / 2).copied().filter(|&n| n > below)
            }
        })
        .collect();
    let power = |l: f64, with: &[Option<f64>]| {
        let s = guided.s_matrix_with(lam, &[l, values[1]], with).unwrap();
        s.power(1, 0)
    };
    for (label, with) in [
        ("2D", &[][..]),
        ("2D, the 3D even modes' indices", &indices),
    ] {
        // the best length by a scan at 1 nm, then golden section
        let ls: Vec<f64> = (0..=600).map(|i| 3.1 + 1e-3 * f64::from(i)).collect();
        let i = (0..ls.len())
            .max_by(|&a, &b| power(ls[a], with).total_cmp(&power(ls[b], with)))
            .unwrap();
        let (mut a, mut b) = (ls[i.saturating_sub(1)], ls[(i + 1).min(600)]);
        let r = 0.5 * (5f64.sqrt() - 1.0);
        while b - a > 1e-8 {
            let (x, y) = (b - r * (b - a), a + r * (b - a));
            if power(x, with) > power(y, with) {
                b = y;
            } else {
                a = x;
            }
        }
        let l = 0.5 * (a + b);
        let p = power(l, with);
        println!(
            "guided modes, {label}: P {:.6} at {:.4} um; best {p:.6} at {l:.4} um, {:.4} dB",
            power(v.at("length"), with),
            v.at("length"),
            -10.0 * portable::log10(2.0 * p)
        );
    }
}

/// The Poynting flux along `axis` across the plane between nodes `plane` and `plane + 1`, cell
/// by cell over `window` (index ranges along `axis.others()`), as [`crate::fdfd::Field3d::flux`]
/// sums it over the whole plane: (u, v, power).
fn flux_cells(
    field: &crate::fdfd::Field3d,
    axis: crate::fdfd::Axis,
    plane: usize,
    window: (std::ops::Range<usize>, std::ops::Range<usize>),
) -> Vec<(usize, usize, f64)> {
    let g = field.grid();
    let (b, c) = axis.others();
    let area = g.step(b) * g.step(c);
    let mut out = Vec::with_capacity(window.0.len() * window.1.len());
    for v in window.1.clone() {
        for u in window.0.clone() {
            let mut s = 0.0;
            for (component, h, sign) in [(b, c, 0.5), (c, b, -0.5)] {
                let mut at = [0; 3];
                at[axis.index()] = plane;
                at[b.index()] = u;
                at[c.index()] = v;
                let low = field.e(component, (at[0], at[1], at[2]));
                at[axis.index()] = plane + 1;
                let high = field.e(component, (at[0], at[1], at[2]));
                at[axis.index()] = plane;
                let hv = field.h(h, (at[0], at[1], at[2]));
                s += sign * (0.5 * (low + high) * hv.conj()).re;
            }
            out.push((u, v, s * area));
        }
    }
    out
}

/// Where the 2 µm MMI's power goes in 3D (issue #814): the input launched alone through the
/// device's own iterative 3D route, and in its field the modes of every guide (the input's
/// reflection, the section's guided modes after the input's face and before the outputs', each
/// output's modes) and the power through the faces of boxes around the device, out of the plane
/// (above and below) and in it (beside, and through the outputs' plane outside their modes).
/// `BOTH` launches the output `o2` too: by reciprocity, each of the section's modes' coupling
/// into it. By hand, one grid a process:
///
/// ```sh
/// GRID=0.044:0.044 LENGTH=3.337 BOTH=1 cargo test --release --lib \
///     device::splitter_tests::survey_mmi_budget -- --ignored --nocapture
/// ```
///
/// `GRID` in-plane:vertical steps (µm), `PML` and `MARGIN` as `survey_3d`'s; `MODES` the
/// section's modes solved on each plane, `PORT_MODES` each port's; `DIST` the planes' distances
/// from the section's faces (µm); `BOXES` the boxes' faces' distances out from the section's
/// sides and the core's faces (µm, y:z, comma-separated); `MAPS` a folder for the flux maps.
#[test]
#[ignore = "a survey, not a check"]
#[allow(clippy::too_many_lines)]
fn survey_mmi_budget() {
    use crate::fdfd::{Axis, Direction, Field3d, PortMode3d};
    use num_complex::Complex64 as c64;
    use std::io::Write;
    let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let started = Instant::now();
    let log = |line: &str| {
        let line = format!("[{:7.1} s] {line}", started.elapsed().as_secs_f64());
        println!("{line}");
        if let Ok(path) = std::env::var("LOG") {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(f, "{line}").unwrap();
        }
    };
    let (device, v) = super::splitter_checks::splitter_3d("mmi");
    let length: f64 = var("LENGTH", "3.337").parse().unwrap();
    let v = v.with(device.as_ref(), "length", length).unwrap();
    let lam = um(1.55);
    let geometry = device.geometry(&v).unwrap();
    let (h, hz) = var("GRID", "0.044:0.044")
        .split_once(':')
        .map(|(a, b)| (a.parse::<f64>().unwrap(), b.parse::<f64>().unwrap()))
        .unwrap();
    let pml: usize = var("PML", "8").parse().unwrap();
    let margin: f64 = var("MARGIN", "0.4").parse().unwrap();
    let grid = FdfdGrid::new(h)
        .with_step_z(hz)
        .with_margin(margin)
        .with_pml(pml)
        .with_samples(1);
    let modes: usize = var("MODES", "12").parse().unwrap();
    let port_modes: usize = var("PORT_MODES", "4").parse().unwrap();
    let (solver, ports, stopping) =
        super::solve::iterative_3d(&geometry, "Si", lam, &grid).unwrap();
    let g = solver.grid();
    log(&format!(
        "MMI L {length} um, 3D {h}:{hz} PML {pml} margin {margin}: {} x {} x {} cells, {} unknowns",
        g.nx,
        g.ny,
        g.nz,
        g.unknowns()
    ));
    let soi = super::guide::Strip::soi();
    let (cladding, ..) = super::section::indices(&soi.stack, &soi.core, lam).unwrap();
    let (_, bottom, top) = geometry.structure.stack().layer("Si").unwrap();
    let (zb, zt) = (bottom.to_um(), top.to_um());
    let ix = |x: f64| ((x - g.x0) / g.dx).round() as usize;
    let iy = |y: f64| ((y - g.y0) / g.dy).round() as usize;
    let iz = |z: f64| ((z - g.z0) / g.dz).round() as usize;
    let (x_in, x_out) = (
        geometry.ports[0].position.x.to_um(),
        geometry.ports[1].position.x.to_um(),
    );
    let half = 0.5 * v.at("width");
    let guided = |m: &PortMode3d| {
        let n = m.effective_index();
        n.re > cladding && n.im.abs() < 1e-6 * n.re
    };
    // each mode's index and its forward and backward power (|a|², the guided modes' power)
    let describe = |field: &Field3d, ms: &[PortMode3d]| -> (String, f64, f64) {
        let (mut fwd, mut bwd) = (0.0, 0.0);
        let parts: Vec<String> = ms
            .iter()
            .map(|m| {
                let (f, b) = field.mode_amplitudes(m);
                let n = m.effective_index();
                let (pf, pb) = (f.norm_sqr(), b.norm_sqr());
                if guided(m) {
                    fwd += pf;
                    bwd += pb;
                    format!("[{:.5} fwd {pf:.6} bwd {pb:.2e}]", n.re)
                } else {
                    format!("({:.4}{:+.1e}i)", n.re, n.im)
                }
            })
            .collect();
        (parts.join(" "), fwd, bwd)
    };
    let launch = |p: usize| -> Field3d {
        let (port, _) = &ports[p];
        let direction = if p == 0 {
            Direction::Forward
        } else {
            Direction::Backward
        };
        let t = Instant::now();
        let (field, how) = solver
            .solve_system(&solver.mode_source(&port.mode, direction), stopping)
            .unwrap();
        log(&format!(
            "launched port {p}: {} iterations, {:.1} s",
            how.iterations,
            t.elapsed().as_secs_f64()
        ));
        field
    };
    let field = launch(0);
    // the ports' fundamental modes, as the S-matrix measures them
    let amplitude = |field: &Field3d, p: usize| field.mode_amplitudes(&ports[p].0.mode);
    let (a0, b0) = amplitude(&field, 0);
    let (o2, _) = amplitude(&field, 1);
    let (o3, _) = amplitude(&field, 2);
    let incident = a0.norm_sqr();
    let (p2, p3) = (o2.norm_sqr() / incident, o3.norm_sqr() / incident);
    log(&format!(
        "incident |a|^2 {incident:.8}; P_o2 {p2:.6} P_o3 {p3:.6} sum {:.6} excess {:.5} dB; R {:.3e}",
        p2 + p3,
        -10.0 * portable::log10(p2 + p3),
        b0.norm_sqr() / incident
    ));
    // every port's modes
    for (p, (port, rows)) in ports.iter().enumerate() {
        let ms = solver
            .port_modes_within(
                Axis::X,
                port.mode.plane(),
                (rows.clone(), 0..g.nz),
                port_modes,
            )
            .unwrap();
        let (text, fwd, bwd) = describe(&field, &ms);
        log(&format!(
            "port {p} at x {:.4}: guided fwd {fwd:.6} bwd {bwd:.3e}; {text}",
            g.node(Axis::X, port.mode.plane())
        ));
    }
    // the section's modes on planes after the input's face and before the outputs'
    let dists: Vec<f64> = var("DIST", "0.15,0.4,0.8")
        .split(',')
        .map(|d| d.parse().unwrap())
        .collect();
    let core_window = (
        iy(-half - 0.4) + 1..iy(half + 0.4) + 1,
        iz(zb - 0.4) + 1..iz(zt + 0.4) + 1,
    );
    let mut section: Vec<(usize, Vec<PortMode3d>)> = Vec::new();
    for &d in &dists {
        for (label, plane) in [
            ("after the input", ix(x_in + d)),
            ("before the outputs", ix(x_out - d) - 1),
        ] {
            let t = Instant::now();
            let ms = solver.port_modes(Axis::X, plane, modes).unwrap();
            let (text, fwd, bwd) = describe(&field, &ms);
            let flux: f64 = flux_cells(&field, Axis::X, plane, core_window.clone())
                .iter()
                .map(|c| c.2)
                .sum();
            log(&format!(
                "section {d} um {label} (x {:.4}, {:.1} s): guided fwd {fwd:.6} bwd {bwd:.3e} net {:.6}; flux within 0.4 um {flux:.6}; {text}",
                g.node(Axis::X, plane),
                t.elapsed().as_secs_f64(),
                fwd - bwd
            ));
            section.push((plane, ms));
        }
    }
    // boxes: the faces' power out, the input's face just past the source (total field there)
    let a_face = ports[0].0.mode.plane() + 2;
    let d_face = ports[1].0.mode.plane();
    let mid = ix(0.5 * (x_in + x_out));
    let maps = std::env::var("MAPS").ok();
    for b in var("BOXES", "0.15:0.15,0.3:0.3").split(',') {
        let (by, bz) = b
            .split_once(':')
            .map(|(a, c)| (a.parse::<f64>().unwrap(), c.parse::<f64>().unwrap()))
            .unwrap();
        let (j0, j1) = (iy(-half - by), iy(half + by));
        let (k0, k1) = (iz(zb - bz), iz(zt + bz));
        let (ys, zs, xs) = (j0 + 1..j1 + 1, k0 + 1..k1 + 1, a_face + 1..d_face + 1);
        let sum = |c: &[(usize, usize, f64)]| c.iter().map(|c| c.2).sum::<f64>();
        // a face's power, split at the section's middle along x where it runs along x
        let split = |c: &[(usize, usize, f64)], x_is_u: bool| {
            let mut halves = (0.0, 0.0);
            for &(u, v, p) in c {
                let x = if x_is_u { u } else { v };
                if x < mid {
                    halves.0 += p;
                } else {
                    halves.1 += p;
                }
            }
            halves
        };
        let into = flux_cells(&field, Axis::X, a_face, (ys.clone(), zs.clone()));
        let out = flux_cells(&field, Axis::X, d_face, (ys.clone(), zs.clone()));
        // Y's others are (Z, X): x is v; Z's are (X, Y): x is u
        let y_lo = flux_cells(&field, Axis::Y, j0, (zs.clone(), xs.clone()));
        let y_hi = flux_cells(&field, Axis::Y, j1, (zs.clone(), xs.clone()));
        let z_lo = flux_cells(&field, Axis::Z, k0, (xs.clone(), ys.clone()));
        let z_hi = flux_cells(&field, Axis::Z, k1, (xs.clone(), ys.clone()));
        let (yl, yh, zl, zh) = (
            split(&y_lo, false),
            split(&y_hi, false),
            split(&z_lo, true),
            split(&z_hi, true),
        );
        let beside = (yh.0 - yl.0, yh.1 - yl.1);
        let vertical = (zh.0 - zl.0, zh.1 - zl.1);
        let (fin, fout) = (sum(&into), sum(&out));
        let leaves = fout + beside.0 + beside.1 + vertical.0 + vertical.1;
        log(&format!(
            "box {by}:{bz} um (y {:.3} to {:.3}, z {:.3} to {:.3}): in {fin:.6} (so back {:.6}); out the outputs' plane {fout:.6}; \
             above and below {:.6} (input half {:.6}, output half {:.6}); beside {:.6} ({:.6}, {:.6}); closure {:.2e}",
            g.node(Axis::Y, j0),
            g.node(Axis::Y, j1),
            g.node(Axis::Z, k0),
            g.node(Axis::Z, k1),
            incident - fin,
            vertical.0 + vertical.1,
            vertical.0,
            vertical.1,
            beside.0 + beside.1,
            beside.0,
            beside.1,
            fin - leaves
        ));
        if let Some(dir) = &maps {
            for (name, cells, axes) in [
                ("outputs-plane", &out, ("y", "z", Axis::Y, Axis::Z)),
                ("above", &z_hi, ("x", "y", Axis::X, Axis::Y)),
                ("beside", &y_hi, ("z", "x", Axis::Z, Axis::X)),
            ] {
                let path = format!("{dir}/mmi-{h}-{by}-{bz}-{name}.csv");
                let mut f = std::fs::File::create(&path).unwrap();
                writeln!(f, "{},{},power", axes.0, axes.1).unwrap();
                for &(u, w, p) in cells {
                    writeln!(
                        f,
                        "{:.4},{:.4},{p:.6e}",
                        g.node(axes.2, u),
                        g.node(axes.3, w)
                    )
                    .unwrap();
                }
            }
        }
    }
    // the guided-mode model on the plane: its modes and each port's couplings into them
    let (model, values) = StripMmi::one_by_two().guided(&v).unwrap();
    let analysis = model.analysis(lam, values[1]).unwrap();
    let (c_in, c_out) = (analysis.couplings(0), analysis.couplings(1));
    let plane_n: Vec<f64> = analysis
        .propagation_constants()
        .iter()
        .map(|b| b / lam.wavenumber())
        .collect();
    let squares = |c: &[f64]| c.iter().map(|c| c * c).collect::<Vec<_>>();
    log(&format!(
        "plane's guided modes {plane_n:.5?}; input couplings squared {:.6?}, sum {:.6}; o2's {:.6?}",
        squares(c_in),
        c_in.iter().map(|c| c * c).sum::<f64>(),
        squares(c_out)
    ));
    if var("BOTH", "0") != "1" {
        return;
    }
    let fields = [field, launch(1), launch(2)];
    // the S-matrix as the study's: S A = B over the ports' fundamental modes, so that what the
    // PMLs send back is part of A
    let mut incoming = faer::Mat::<c64>::zeros(3, 3);
    let mut outgoing = faer::Mat::<c64>::zeros(3, 3);
    for (p, f) in fields.iter().enumerate() {
        for q in 0..3 {
            let (fwd, bwd) = f.mode_amplitudes(&ports[q].0.mode);
            let (a, b) = if q == 0 { (fwd, bwd) } else { (bwd, fwd) };
            incoming[(q, p)] = a;
            outgoing[(q, p)] = b;
        }
    }
    let st = {
        use faer::linalg::solvers::Solve;
        incoming
            .transpose()
            .to_owned()
            .partial_piv_lu()
            .solve(outgoing.transpose().to_owned())
    };
    let s = |q: usize, p: usize| st[(p, q)];
    let (s21, s31) = (s(1, 0).norm_sqr(), s(2, 0).norm_sqr());
    log(&format!(
        "S: P_o2 {s21:.6} P_o3 {s31:.6} sum {:.6} excess {:.5} dB; R {:.3e}; S23 {:.3e}",
        s21 + s31,
        -10.0 * portable::log10(s21 + s31),
        s(0, 0).norm_sqr(),
        s(1, 2).norm_sqr()
    ));
    // the guided modes alone, the 3D's: on each plane before the outputs, each even mode's
    // amplitude from o1 (a) and, by reciprocity, its coupling into o2 (t, what o2's launch puts
    // into it going back), each launch over its own incident amplitude. Σ a t is o2's amplitude
    // carried by the guided modes; the length moved by ΔL turns each term by e^(iβΔL).
    let (a0, _) = fields[0].mode_amplitudes(&ports[0].0.mode);
    let (_, a1) = fields[1].mode_amplitudes(&ports[1].0.mode);
    let best = |p: &dyn Fn(f64) -> f64| -> (f64, f64) {
        let xs: Vec<f64> = (0..=1200).map(|i| -0.6 + 1e-3 * f64::from(i)).collect();
        let i = (0..xs.len())
            .max_by(|&a, &b| p(xs[a]).total_cmp(&p(xs[b])))
            .unwrap();
        let (mut a, mut b) = (xs[i.saturating_sub(1)], xs[(i + 1).min(1200)]);
        let r = 0.5 * (5f64.sqrt() - 1.0);
        while b - a > 1e-9 {
            let (x, y) = (b - r * (b - a), a + r * (b - a));
            if p(x) > p(y) {
                b = y;
            } else {
                a = x;
            }
        }
        let x = 0.5 * (a + b);
        (x, p(x))
    };
    let db = |p: f64| -10.0 * portable::log10(2.0 * p);
    // the section's even modes' β, converged: Hadley's equations on 10 nm cells, as
    // `survey_mmi_modes` solves them
    let converged: Vec<f64> = {
        use super::guide::{Profile, Strip};
        use super::section::{Mesh, Scheme};
        let strip = Strip {
            mesh: Mesh::fitted(0.01, 1.0, Scheme::Hadley),
            ..Strip::soi()
        };
        let cs = strip
            .cross_section_of(2.0 * half, zt - zb, Profile::STRIP, lam)
            .unwrap();
        crate::mode::hadley::modes(&cs, lam, 3, None)
            .unwrap()
            .iter()
            .map(|m| m.effective_index().re * lam.wavenumber())
            .collect()
    };
    log(&format!(
        "converged even modes (Hadley, 10 nm): {:.6?}",
        converged
            .iter()
            .map(|b| b / lam.wavenumber())
            .collect::<Vec<_>>()
    ));
    for (plane, ms) in &section {
        if g.node(Axis::X, *plane) < 0.5 * (x_in + x_out) {
            continue;
        }
        let even: Vec<(f64, c64, c64)> = ms
            .iter()
            .filter(|m| guided(m))
            .filter_map(|m| {
                let (a, _) = fields[0].mode_amplitudes(m);
                let (_, t) = fields[1].mode_amplitudes(m);
                (a.norm_sqr() > 1e-8).then(|| (m.beta().re, a / a0, t / a1))
            })
            .collect();
        let each: Vec<String> = even
            .iter()
            .map(|(b, a, t)| {
                format!(
                    "[n {:.5} |a|^2 {:.6} |t|^2 {:.6} arg(at)/pi {:+.4}]",
                    b / lam.wavenumber(),
                    a.norm_sqr(),
                    t.norm_sqr(),
                    (a * t).arg() / std::f64::consts::PI
                )
            })
            .collect();
        log(&format!(
            "guided even modes at x {:.4}: {}; sum |a|^2 {:.6}, 2 sum |t|^2 {:.6}",
            g.node(Axis::X, *plane),
            each.join(" "),
            even.iter().map(|e| e.1.norm_sqr()).sum::<f64>(),
            2.0 * even.iter().map(|e| e.2.norm_sqr()).sum::<f64>()
        ));
        // the models, each with the length moved by ΔL from LENGTH, each mode's β the grid's
        // own (the FDFD modes') or the converged one (Hadley's equations on 10 nm cells): the
        // plane's couplings, the first three even modes (the fourth isn't guided in 3D); the 3D
        // couplings' sizes, with the plane's signs and no phase but the length's; the 3D
        // couplings whole, their junctions' phases too
        let turn = |b: f64, d: f64| portable::complex::exp(c64::new(0.0, b * d));
        for (which, betas) in [
            (
                "grid's beta",
                even.iter().map(|e| e.0).collect::<Vec<f64>>(),
            ),
            ("converged beta", converged.clone()),
        ] {
            let plane = |d: f64| -> f64 {
                betas
                    .iter()
                    .enumerate()
                    .map(|(k, b)| c_in[2 * k] * c_out[2 * k] * turn(*b, length + d))
                    .sum::<c64>()
                    .norm_sqr()
            };
            let sizes = |d: f64| -> f64 {
                even.iter()
                    .zip(&betas)
                    .enumerate()
                    .map(|(k, ((_, a, t), b))| {
                        let sign = (c_in[2 * k] * c_out[2 * k]).signum();
                        sign * (a * t).norm() * turn(*b, length + d)
                    })
                    .sum::<c64>()
                    .norm_sqr()
            };
            let three = |d: f64| -> f64 {
                even.iter()
                    .zip(&betas)
                    .map(|((own, a, t), b)| a * t * turn(b - own, length) * turn(*b, d))
                    .sum::<c64>()
                    .norm_sqr()
            };
            for (label, p) in [
                ("the plane's couplings", &plane as &dyn Fn(f64) -> f64),
                ("3D couplings' sizes, the plane's signs", &sizes),
                ("3D couplings", &three),
            ] {
                let (d, pb) = best(p);
                log(&format!(
                    "  guided only, {which}, {label}: P {:.6} at {length} um ({:.4} dB); best {pb:.6} at {:.4} um ({:.4} dB)",
                    p(0.0),
                    db(p(0.0)),
                    length + d,
                    db(pb)
                ));
            }
        }
    }
    // o2's amplitude from every mode of a plane before the outputs, by reciprocity: across any
    // cut between the two launches' sources, Lorentz's form of the two fields is o2's
    // amplitude, and on the plane's modes it is Σ (a⁺ w⁻ − a⁻ w⁺), a from o1's launch and w from
    // o2's. The guided modes' terms, then the rest's as more modes are taken; each non-guided
    // mode put above and below the core or beside it, by where most of its field is.
    let counts: Vec<usize> = var("RAD_MODES", "12,50,150,300")
        .split(',')
        .map(|n| n.parse().unwrap())
        .collect();
    let most = *counts.iter().max().unwrap();
    let (b2, _) = fields[0].mode_amplitudes(&ports[1].0.mode);
    let direct = b2 / a0;
    let (k_lo, k_hi) = (iz(zb), iz(zt));
    let (j_lo, j_hi) = (iy(-half), iy(half));
    for (plane, _) in &section {
        if g.node(Axis::X, *plane) < 0.5 * (x_in + x_out) {
            continue;
        }
        let t = Instant::now();
        let ms = solver.port_modes(Axis::X, *plane, most).unwrap();
        let mut guided_sum = c64::new(0.0, 0.0);
        let mut back = c64::new(0.0, 0.0);
        let (mut vertical, mut beside) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        let (mut n_vertical, mut n_beside) = (0, 0);
        let mut partial = Vec::new();
        for (i, m) in ms.iter().enumerate() {
            let (ap, am) = fields[0].mode_amplitudes(m);
            let (wp, wm) = fields[1].mode_amplitudes(m);
            let (ap, am, wp, wm) = (ap / a0, am / a0, wp / a1, wm / a1);
            if guided(m) {
                guided_sum += ap * wm;
                back -= am * wp;
            } else {
                let term = ap * wm - am * wp;
                // where the mode's tangential E is: above and below the section, or beside it
                // in the core layer (the corners left out)
                let (mut up, mut side) = (0.0, 0.0);
                for k in 0..g.nz {
                    for j in 0..g.ny {
                        let e = m.e(Axis::Y, (j, k)).norm_sqr() + m.e(Axis::Z, (j, k)).norm_sqr();
                        let over = j >= j_lo && j < j_hi;
                        let level = k >= k_lo && k < k_hi;
                        if over && !level {
                            up += e;
                        } else if level && !over {
                            side += e;
                        }
                    }
                }
                if up >= side {
                    vertical += term;
                    n_vertical += 1;
                } else {
                    beside += term;
                    n_beside += 1;
                }
            }
            if counts.contains(&(i + 1)) {
                partial.push((
                    i + 1,
                    guided_sum + back + vertical + beside,
                    vertical,
                    beside,
                ));
            }
        }
        let g2 = guided_sum.norm_sqr();
        // S's power into o2, split exactly: the guided modes going forward; their reflections
        // (the guided modes going back); the rest of the field (the junctions' radiation); and
        // what the PMLs send back (the S-matrix against o1's launch alone)
        let with_back = (guided_sum + back).norm_sqr();
        log(&format!(
            "split of P_o2 {s21:.6}: guided forward {g2:.6}, reflections {:+.6}, the rest of the field {:+.6}, the PMLs {:+.6}",
            with_back - g2,
            direct.norm_sqr() - with_back,
            s21 - direct.norm_sqr()
        ));
        log(&format!(
            "every mode at x {:.4} ({} modes, {:.1} s): o2 directly {:.6}{:+.6}i, |.|^2 {:.6}; guided forward {:.6}{:+.6}i, |.|^2 {g2:.6}; guided back {:.2e}{:+.2e}i",
            g.node(Axis::X, *plane),
            ms.len(),
            t.elapsed().as_secs_f64(),
            direct.re,
            direct.im,
            direct.norm_sqr(),
            guided_sum.re,
            guided_sum.im,
            back.re,
            back.im
        ));
        for (n, total, up, side) in partial {
            // each part's share of the power, to first order: 2 Re(conj(guided) part)
            let share = |r: c64| 2.0 * (guided_sum.conj() * r).re;
            log(&format!(
                "  {n} modes: sum {:.6}{:+.6}i, |.|^2 {:.6} (direct {:.6}); above and below {:.2e}{:+.2e}i, share {:+.6}; beside {:.2e}{:+.2e}i, share {:+.6}; back's share {:+.6}",
                total.re,
                total.im,
                total.norm_sqr(),
                direct.norm_sqr(),
                up.re,
                up.im,
                share(up),
                side.re,
                side.im,
                share(side),
                share(back)
            ));
        }
        log(&format!(
            "  non-guided modes: {n_vertical} above and below, {n_beside} beside"
        ));
    }
}

/// A strip with its guides led out of its ends, through the devices' FDFD with each cell
/// averaged from the fill: the leads abut the strip, and the seams no longer reflect (issue
/// #376: |S₁₁|² 6e-5 and |S₂₁ − S₁₂| 3e-4 on these 100 nm cells), as with samples.
#[test]
fn a_strip_and_its_leads_leave_no_seam_with_the_fill() {
    use crate::averaging::Averaging;
    let [r0, a0, p0] = super::splitter_checks::straight(Averaging::Samples);
    for averaging in [Averaging::Mean, Averaging::Diagonal, Averaging::Tensor] {
        let [r, a, p] = super::splitter_checks::straight(averaging);
        assert!(
            r <= r0 + 1e-28 && a < 1e-13 && p < 1e-4,
            "{averaging:?}: |S11|² {r:e}, |S21 - S12| {a:e}, phase {p:e} (sampled: {r0:e}, {a0:e}, {p0:e})"
        );
    }
}

/// ROADMAP's pitfall: 125 nm pixels capped a design at 0.85 however it was binarized. A design
/// is its parameters and the exact shapes they draw, set apart from the grid that solves it. On
/// a grid four times finer than its own, an MMI's and a Y-branch's design are the same shapes to
/// the bit, solved on about four times the cells along each axis; and the job that re-checks a
/// design on the finer grid carries the same design, only its task's grid changed.
#[test]
fn a_design_is_set_apart_from_its_grid_and_rechecked_on_a_finer_one() {
    use super::spec::Grids;
    use crate::job::{Compute, from_device};
    for kind in ["MMI 1x2", "MMI 2x2", "Y-branch"] {
        let mut spec = Spec::new(kind);
        spec.fidelity = Fidelity::TwoD;
        let fdfd_2d = kind.starts_with("MMI");
        let own_step = if fdfd_2d {
            StripMmi::one_by_two().grids().0.step
        } else {
            StripYBranch::new().grids().0.step
        };
        let own = Grids {
            fdfd_2d,
            ..Grids::default()
        };
        let finer = Grids {
            step: Some(own_step / 4.0),
            ..own
        };
        let (_, design) = spec.build().unwrap();
        let values = design.values();
        let shapes = |grids: &Grids| spec.device_with(grids).unwrap().geometry(values).unwrap();
        assert_eq!(shapes(&own), shapes(&finer), "{kind}");
        let (nx, ny, _) = spec.fdfd_size(&own).unwrap().unwrap();
        let (fx, fy, _) = spec.fdfd_size(&finer).unwrap().unwrap();
        assert!(
            fx > 3 * nx && fy > 3 * ny,
            "{kind}: {nx} x {ny}, then {fx} x {fy}"
        );
        let job = |step_nm| {
            let compute = Compute {
                step_nm,
                fdfd_2d,
                ..Compute::model()
            };
            from_device("recheck", &spec, &compute).unwrap()
        };
        let (coarse, fine) = (job(None), job(Some(own_step * 250.0)));
        assert_eq!(coarse.device(), fine.device(), "{kind}");
        assert_ne!(coarse.task(), fine.task(), "{kind}");
    }
}
