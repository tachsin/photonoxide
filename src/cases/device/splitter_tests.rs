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
/// `DEV` as [`super::splitter_checks::splitter_3d`]; `GRID` in-plane:vertical steps (µm); `ROUTE` `auto` (the device's
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
            -10.0 * (a + b).log10(),
            10.0 * (a / b).log10(),
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
