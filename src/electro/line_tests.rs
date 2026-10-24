use super::line::{DB_PER_NEPER, Drive, Edge, Metal, Periodic, QuasiTem, response};
use super::line_checks::{self as checks, COPPER, GOLD, ZHANG_TANGENTS};
use super::{Boundary, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor};
use num_complex::Complex64 as c64;

#[test]
fn a_thick_strip_has_the_surface_resistance_of_its_skin_depth() {
    let gold = Metal::new(GOLD, 0.8).unwrap();
    // 0.8 µm is 40 skin depths at about 30 THz: the thick limit
    let f = 4e13;
    let z = gold.surface_impedance(f);
    let rs = 1.0 / (GOLD * gold.skin_depth(f));
    assert!((z.re - rs).abs() < 1e-12 * rs && (z.im - rs).abs() < 1e-12 * rs);
    // just below the switch to the limit, the formula agrees with it
    let f = 0.98 * 40.0f64.powi(2) / (std::f64::consts::PI * super::line::mu_0() * GOLD * 0.64e-12);
    let z = gold.surface_impedance(f);
    let rs = 1.0 / (GOLD * gold.skin_depth(f));
    assert!((z.re / rs - 1.0).abs() < 1e-9 && (z.im / rs - 1.0).abs() < 1e-9);
}

#[test]
fn a_thin_strip_at_low_frequency_has_its_sheet_resistance_on_both_sides() {
    let gold = Metal::new(GOLD, 0.8).unwrap();
    let z = gold.surface_impedance(1e3);
    let sheet = 2.0 / (GOLD * 0.8e-6);
    assert!(
        (z.re / sheet - 1.0).abs() < 1e-6,
        "{} against {sheet}",
        z.re
    );
}

#[test]
fn the_stopping_distance_reads_holloway_and_kuesters_table() {
    let gold = Metal::new(GOLD, 0.8).unwrap();
    // the frequency at which t/(2δ) = u: δ = t/(2u), f = 1/(π μ₀ σ δ²)
    let at = |u: f64| {
        let d = 0.8e-6 / (2.0 * u);
        1.0 / (std::f64::consts::PI * super::line::mu_0() * GOLD * d * d)
    };
    let d = gold.stopping_distance(at(1.0)).unwrap();
    assert!((0.8e-6 / d - 200.50).abs() < 1e-9);
    let d = gold
        .with_edge(Edge::Bevelled)
        .stopping_distance(at(4.0))
        .unwrap();
    assert!((0.8e-6 / d - 327.08).abs() < 1e-9);
    // halfway between two rows
    let d = gold.stopping_distance(at(0.5 * (0.5 + 0.64))).unwrap();
    assert!((0.8e-6 / d - 0.5 * (33.97 + 61.90)).abs() < 1e-9);
    // below the table, its first row
    let d = gold.stopping_distance(1e3).unwrap();
    assert!((0.8e-6 / d - 9.18).abs() < 1e-12);
    assert!(gold.stopping_distance(at(17.0)).is_err());
}

#[test]
fn the_response_is_one_at_dc_and_flat_when_everything_is_matched() {
    let drive = Drive::new(50.0, 2.2, 0.01).unwrap();
    let ends = drive.with_ends(20.0, 80.0).unwrap();
    let beta = |f: f64, n: f64| 2.0 * std::f64::consts::PI * f * n / crate::units::SPEED_OF_LIGHT;
    // a lossless line of another impedance at a very low frequency
    let m = response(
        c64::new(0.0, beta(1e3, 2.4)),
        c64::new(35.0, 0.0),
        1e3,
        &ends,
    );
    assert!((m - 1.0).abs() < 1e-9, "{m}");
    // matched, velocity-matched and lossless: 1 at any frequency
    for f in [1e9, 1e11, 1e12] {
        let m = response(c64::new(0.0, beta(f, 2.2)), c64::new(50.0, 0.0), f, &drive);
        assert!((m - 1.0).abs() < 1e-9, "{f}: {m}");
    }
}

/// A coplanar waveguide of strips (a = 3, b = 5 µm) on silicon, whole or as its half beside a
/// mirror, graded from 4 nm.
fn whole_and_half() -> (QuasiTem, QuasiTem) {
    let (a, b, reach) = (3.0, 5.0, 300.0);
    let build = |left: f64| {
        let mut section = Section::new("air");
        section.layer(-2.0 * reach, 0.0, "Si").unwrap();
        let mut features = section.features().clone();
        features
            .strip([left.max(-a), a], 0.0)
            .strip([b, reach], 0.0);
        if left < 0.0 {
            features.strip([-reach, -b], 0.0);
        }
        let grid = Grid::fitted(
            [left, reach],
            [-reach, reach],
            &Grading::new(5.0, 0.004, 1.1).unwrap(),
            &features,
        )
        .unwrap();
        let mut p = Electrostatic::new(section, grid);
        p.permittivity("air", Tensor::isotropic(1.0))
            .permittivity("Si", Tensor::isotropic(11.7))
            .boundary(Side::Right, Boundary::Fixed(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .conductor(Conductor::strip([left.max(-a), a], 0.0, 1.0))
            .conductor(Conductor::strip([b, reach], 0.0, 0.0));
        if left < 0.0 {
            p.boundary(Side::Left, Boundary::Fixed(0.0))
                .conductor(Conductor::strip([-reach, -b], 0.0, 0.0));
        }
        p
    };
    (
        QuasiTem::new(&build(-reach), 0).unwrap(),
        QuasiTem::mirrored(&build(0.0), 0).unwrap(),
    )
}

#[test]
fn a_mirrored_half_is_the_whole_line() {
    let (whole, half) = whole_and_half();
    let close = |a: f64, b: f64, tol: f64| (a / b - 1.0).abs() < tol;
    assert!(close(half.capacitance(), whole.capacitance(), 1e-4));
    assert!(close(
        half.vacuum_capacitance(),
        whole.vacuum_capacitance(),
        1e-4
    ));
    assert!(close(half.effective_permittivity(), 6.35, 1e-3));
    let (h, w) = (
        half.current_integral(5e-9).unwrap(),
        whole.current_integral(5e-9).unwrap(),
    );
    assert!(close(h, w, 2e-3), "{h} against {w}");
    let sum: f64 = half.shares().iter().map(|s| s.1).sum();
    assert!((sum - 1.0).abs() < 1e-12);
}

#[test]
fn the_dielectric_loss_is_the_mediums_tangent_weighed_by_its_share() {
    let (_, half) = whole_and_half();
    let q = half.shares().iter().find(|s| s.0 == "Si").unwrap().1;
    let tan = half.loss_tangent(&[("Si", 0.01)]).unwrap();
    assert!((tan - 0.01 * q).abs() < 1e-15);
    let metal = Metal::new(GOLD, 1.0).unwrap();
    let f = 50e9;
    let w = half.wave(f, &metal, &[("Si", 0.01)]).unwrap();
    let expected = std::f64::consts::PI * f * half.index() * tan / crate::units::SPEED_OF_LIGHT;
    assert!((w.dielectric / expected - 1.0).abs() < 1e-12);
    assert!(half.loss_tangent(&[("glass", 0.01)]).is_err());
    assert!(half.loss_tangent(&[("Si", -0.01)]).is_err());
}

#[test]
#[ignore = "prints the cases' numbers"]
fn print_the_numbers() {
    let t0 = std::time::Instant::now();
    for h in [0.004, 0.002, 0.001] {
        println!(
            "HK current {h}: {:e}  ({:.1} s)",
            checks::current_against_closed_form(h),
            t0.elapsed().as_secs_f64()
        );
    }
    for h in [5.0, 10.0, 20.0, 40.0] {
        println!("ghione-naldi h = {h}: {:?}", checks::ghione_naldi(h, 0.004));
    }
    println!("wheeler plates {:e}", checks::wheeler_plates());
    for g in [50.0, 100.0, 300.0, 1000.0, 3000.0] {
        println!(
            "wheeler against HK at {g} GHz: {:e}",
            checks::wheeler_against_holloway_kuester(g)
        );
    }
    println!("kharel {:?}", checks::kharel_numbers());
    println!("zhang {:?}", checks::zhang_numbers());
    for mm in [5.8, 10.8] {
        println!(
            "zhang {mm} mm: 3 dB at {} GHz",
            checks::zhang_bandwidth(mm, 0.004)
        );
    }
    for case in checks::cases() {
        let o = (case.run)();
        println!(
            "{}: measured {} expected {} tolerance {} -> {}  ({:.1} s)",
            case.id,
            o.measured,
            o.expected,
            o.tolerance,
            o.error <= o.tolerance,
            t0.elapsed().as_secs_f64()
        );
    }
}

#[test]
#[ignore = "prints Zhang et al.'s line against frequency"]
fn print_zhang_against_frequency() {
    let copper = Metal::new(COPPER, 0.83).unwrap();
    let line = checks::zhang_2022().line(0.004);
    println!("shares {:?}", line.shares());
    for g in [1.0, 10.0, 50.0, 100.0, 150.0, 200.0, 250.0, 300.0] {
        let w = checks::wave(&line, g, &copper, &ZHANG_TANGENTS);
        println!(
            "{g} GHz: n {:.4} Z {:.3}{:+.3}j loss {:.3} dB/cm (conductor {:.3}, dielectric {:.3})",
            w.index(),
            w.impedance.re,
            w.impedance.im,
            w.db_per_cm(),
            w.conductor * DB_PER_NEPER / 100.0,
            w.dielectric * DB_PER_NEPER / 100.0
        );
    }
}

#[test]
fn an_ideal_line_has_its_numbers_and_no_loss() {
    let (l, c) = (4e-7, 1.6e-10);
    let line = QuasiTem::ideal(l, c).unwrap();
    let speed = crate::units::SPEED_OF_LIGHT;
    assert!((line.impedance() / (l / c).sqrt() - 1.0).abs() < 1e-12);
    assert!((line.index() / (speed * (l * c).sqrt()) - 1.0).abs() < 1e-12);
    assert!((line.inductance() / l - 1.0).abs() < 1e-12);
    // gold at 30 THz is beyond Holloway and Kuester's table, but an ideal line has no current
    let gold = Metal::new(GOLD, 0.8).unwrap();
    assert!(gold.stopping_distance(3e13).is_err());
    let w = line.wave(3e13, &gold, &[]).unwrap();
    assert_eq!(w.attenuation(), 0.0);
    assert!((w.impedance.re / line.impedance() - 1.0).abs() < 1e-12);
    assert!(QuasiTem::ideal(-1.0, c).is_err());
    assert!(QuasiTem::ideal(l, f64::NAN).is_err());
    // an index below 1
    assert!(QuasiTem::ideal(l, 1e-12).is_err());
}

/// Half a coplanar waveguide of strips on silicon (a = 3, b = 6 µm) beside its mirror, graded
/// from 10 nm, with a rail strip from 4 to 4.5 µm at y = 0 drawn as conductor 2 when `rail`.
fn half_with_rail(rail: bool) -> Electrostatic {
    let (a, b, reach) = (3.0, 6.0, 300.0);
    let mut section = Section::new("air");
    section.layer(-2.0 * reach, 0.0, "Si").unwrap();
    let mut features = section.features().clone();
    features
        .strip([0.0, a], 0.0)
        .strip([4.0, 4.5], 0.0)
        .strip([b, reach], 0.0);
    let grid = Grid::fitted(
        [0.0, reach],
        [-reach, reach],
        &Grading::new(5.0, 0.01, 1.1).unwrap(),
        &features,
    )
    .unwrap();
    let mut p = Electrostatic::new(section, grid);
    p.permittivity("air", Tensor::isotropic(1.0))
        .permittivity("Si", Tensor::isotropic(11.7))
        .boundary(Side::Right, Boundary::Fixed(0.0))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(0.0))
        .conductor(Conductor::strip([0.0, a], 0.0, 1.0))
        .conductor(Conductor::strip([b, reach], 0.0, 0.0));
    if rail {
        p.conductor(Conductor::strip([4.0, 4.5], 0.0, 1.0));
    }
    p
}

#[test]
fn a_loaded_line_keeps_its_inductance_and_current_and_takes_the_rails_capacitance() {
    let channel = QuasiTem::mirrored(&half_with_rail(false), 0).unwrap();
    // loaded by its own section: itself
    let same = channel.loaded(&half_with_rail(false), &[0]).unwrap();
    assert!((same.capacitance() / channel.capacitance() - 1.0).abs() < 1e-12);
    let loaded = channel.loaded(&half_with_rail(true), &[0, 2]).unwrap();
    assert!(loaded.capacitance() > 1.2 * channel.capacitance());
    assert_eq!(loaded.inductance(), channel.inductance());
    assert_eq!(
        loaded.current_integral(5e-9).unwrap(),
        channel.current_integral(5e-9).unwrap()
    );
    let sum: f64 = loaded.shares().iter().map(|s| s.1).sum();
    assert!((sum - 1.0).abs() < 1e-12);
    // Z = √(L/C), n = c√(LC) of the loaded capacitance
    let speed = crate::units::SPEED_OF_LIGHT;
    let (l, c) = (loaded.inductance(), loaded.capacitance());
    assert!((loaded.impedance() / (l / c).sqrt() - 1.0).abs() < 1e-12);
    assert!((loaded.index() / (speed * (l * c).sqrt()) - 1.0).abs() < 1e-12);
    // the rail at the ground's potential instead loads the line too, less
    let to_ground = channel.loaded(&half_with_rail(true), &[0]).unwrap();
    assert!(to_ground.capacitance() > channel.capacitance());
    assert!(to_ground.capacitance() < loaded.capacitance());
    for bad in [&[][..], &[3], &[0, 1, 2]] {
        assert!(
            channel.loaded(&half_with_rail(true), bad).is_err(),
            "{bad:?}"
        );
    }
}

#[test]
fn a_period_is_unimodular_and_refuses_a_stop_band() {
    let c = crate::units::SPEED_OF_LIGHT;
    let line = |z: f64| QuasiTem::ideal(z * 2.2 / c, 2.2 / (z * c)).unwrap();
    let gold = Metal::new(GOLD, 0.8).unwrap();
    let p = Periodic::new(vec![(line(30.0), 25.0), (line(60.0), 25.0)]).unwrap();
    assert_eq!(p.period_um(), 50.0);
    assert_eq!(p.sections().len(), 2);
    let t = p.abcd(3e11, &gold, &[]).unwrap();
    let det = t[0][0] * t[1][1] - t[0][1] * t[1][0];
    assert!((det - c64::new(1.0, 0.0)).norm() < 1e-12, "{det}");
    // the quarter wave, θ = π/2, lies in the first stop band (from θ = arccos(1/3))
    let quarter = c / (4.0 * 2.2 * 25e-6);
    let e = p.bloch(quarter, &gold, &[]).unwrap_err().to_string();
    assert!(e.contains("stop band"), "{e}");
    // below the band edge it travels
    assert!(p.bloch(0.5 * quarter, &gold, &[]).is_ok());
    assert!(Periodic::new(Vec::new()).is_err());
    assert!(Periodic::new(vec![(line(50.0), 0.0)]).is_err());
}

#[test]
fn a_lossy_period_splits_its_loss_as_its_sections_do() {
    // a uniform lossy line cut in two: the Bloch wave's split is the line's
    let (line, _) = checks::coplanar(3.0, 5.0, 11.7, 600.0, 300.0, 0.01, 1.1);
    let gold = Metal::new(GOLD, 1.0).unwrap();
    let tangents = [("substrate", 0.01)];
    let p = Periodic::new(vec![(line.clone(), 10.0), (line.clone(), 40.0)]).unwrap();
    let (b, u) = (
        p.bloch(50e9, &gold, &tangents).unwrap(),
        line.wave(50e9, &gold, &tangents).unwrap(),
    );
    let close = |x: f64, y: f64, tol: f64| (x / y - 1.0).abs() < tol;
    // the line's own split takes its quasi-static Z₀, the Bloch wave's its Re Z_B
    let ratio = line.impedance() / u.impedance.re;
    assert!(
        close(b.conductor, u.conductor * ratio, 1e-9),
        "{} {}",
        b.conductor,
        u.conductor
    );
    assert!(close(b.dielectric, u.dielectric / ratio, 1e-9));
    assert!((b.internal - u.internal).norm() < 1e-9 * u.internal.norm());
}

#[test]
#[ignore = "prints the segmented electrodes' cases"]
fn print_segmented_cases() {
    let t0 = std::time::Instant::now();
    println!("shin {:?}", checks::shin_numbers());
    println!("kharel segmented {:?}", checks::kharel_segmented_numbers());
    for case in checks::cases() {
        if !["periodic", "shin", "segmented", "quartz"]
            .iter()
            .any(|k| case.id.contains(k))
        {
            continue;
        }
        let o = (case.run)();
        println!(
            "{}: measured {} expected {} tolerance {} -> {}  ({:.1} s)",
            case.id,
            o.measured,
            o.expected,
            o.tolerance,
            o.error <= o.tolerance,
            t0.elapsed().as_secs_f64()
        );
    }
}
