use super::checks::*;
use super::*;
use crate::fdfd::{Edges, Polarization};
use crate::mode::Polarization as Kind;

fn film_error(kind: Kind, h: f64, angle: f64) -> f64 {
    let ((r, _), (exact, _)) = film_ratios(&film_run(kind, h, angle, (PML, 1e-8)));
    (r - exact).abs()
}

#[test]
fn a_film_reflects_as_the_transfer_matrices_say_at_second_order() {
    // 30 degrees from the normal in a plane 30 degrees from x: kx, ky and every component of E
    for kind in [Kind::Te, Kind::Tm] {
        let errors: Vec<f64> = [0.02, 0.01, 0.005]
            .iter()
            .map(|&h| film_error(kind, h, 30f64.to_radians()))
            .collect();
        let orders: Vec<f64> = errors.windows(2).map(|e| (e[0] / e[1]).log2()).collect();
        // measured: TE 2.8e-3, 7.3e-4, 1.9e-4; TM 1.9e-3, 4.9e-4, 1.2e-4
        assert!(errors[2] < 3e-4, "{kind:?}: errors {errors:?}");
        assert!(
            orders.iter().all(|&p| p > 1.8),
            "{kind:?}: errors {errors:?}, orders {orders:?}"
        );
    }
}

#[test]
fn the_flux_is_the_same_through_every_plane_of_a_lossless_region() {
    for kind in [Kind::Te, Kind::Tm] {
        for deg in [0.0, 30.0, 60.0_f64] {
            let run = film_run(kind, 0.01, deg.to_radians(), (PML, 1e-8));
            let spread = flux_spread(&run);
            assert!(spread < 1e-10, "{kind:?} {deg}: {spread}");
        }
    }
}

#[test]
fn reflection_and_transmission_add_up_to_one_but_for_the_pmls() {
    // as in 2D: what is left of R + T - 1 is what the PMLs send back, which a thicker, stronger
    // PML shrinks
    for kind in [Kind::Te, Kind::Tm] {
        for deg in [0.0, 60.0_f64] {
            let angle = deg.to_radians();
            let ((r, t), _) = film_ratios(&film_run(kind, 0.01, angle, (PML, 1e-8)));
            let ((rs, ts), _) = film_ratios(&film_run(kind, 0.01, angle, (40, 1e-16)));
            let (loose, strong) = ((r + t - 1.0).abs(), (rs + ts - 1.0).abs());
            assert!(loose < 2e-4, "{kind:?} {deg}: R {r} + T {t}");
            assert!(
                strong < 1e-6,
                "{kind:?} {deg}: R {rs} + T {ts} with the strong PML"
            );
        }
    }
}

#[test]
fn the_pml_barely_reflects() {
    for kind in [Kind::Te, Kind::Tm] {
        let r = pml_reflection(kind);
        // 2.5e-6, as in 2D
        assert!(r < 1e-5, "{kind:?}: the PML reflects {r}");
    }
}

#[test]
fn a_structure_invariant_along_z_is_the_2d_solvers() {
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let d = two_d_difference(polarization);
        assert!(d < 1e-11, "{polarization:?}: {d}");
    }
}

#[test]
fn the_wave_leaves_the_source_with_the_phase_of_e_to_the_minus_i_omega_t() {
    // a sheet of current in oxide: above it the field goes as e^(+i kz z), below as e^(-i kz z),
    // and power flows away from it on both sides
    let h = 0.02;
    let grid = Grid3d {
        nx: 1,
        ny: 1,
        nz: 160,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let boundaries = Boundaries3d {
        x: Edges::Bloch { k: 0.0 },
        y: Edges::Bloch { k: 0.0 },
        ..Boundaries3d::pml(20)
    };
    let solver = Solver3d::new(
        grid,
        Wavelength::um(1.55).unwrap(),
        |_, _, _| c64::new(1.444 * 1.444, 0.0),
        boundaries,
    )
    .unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    source[grid.index(Axis::X, (0, 0, 80))] = c64::new(1.0, 0.0);
    let field = solver.solve(&source).unwrap();
    let kd = 2.0 / h * (2.0 * std::f64::consts::PI / 1.55 * 1.444 * h / 2.0).asin();
    let step = |k: usize| field.e(Axis::X, (0, 0, k + 1)) / field.e(Axis::X, (0, 0, k));
    let up = step(100);
    let down = step(55);
    assert!((up - c64::from_polar(1.0, kd * h)).norm() < 1e-6, "{up}");
    assert!(
        (down - c64::from_polar(1.0, -kd * h)).norm() < 1e-6,
        "{down}"
    );
    assert!(field.flux(Axis::Z, 100) > 0.0 && field.flux(Axis::Z, 55) < 0.0);
    // nothing along the current's normal: E_y and E_z are zero
    let other = (0..grid.nz)
        .map(|k| field.e(Axis::Y, (0, 0, k)).norm() + field.e(Axis::Z, (0, 0, k)).norm())
        .fold(0.0, f64::max);
    assert!(other < 1e-12, "{other}");
}

#[test]
fn each_component_sees_its_own_average_and_walls_hold_no_field() {
    let grid = Grid3d {
        nx: 6,
        ny: 5,
        nz: 7,
        dx: 0.1,
        dy: 0.1,
        dz: 0.1,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    // an interface normal to x through the middle of the cells of index 2, at x = 0.25: E_x at
    // x = 0.25 sees the harmonic mean across it, E_y and E_z at the node x = 0.2 the arithmetic
    // one
    let eps = |x: f64, _: f64, _: f64| c64::new(if x < 0.25 { 1.0 } else { 3.0 }, 0.0);
    let lam = Wavelength::um(1.0).unwrap();
    let walls = Boundaries3d {
        x: Edges::Pml { low: 0, high: 0 },
        y: Edges::Pml { low: 0, high: 0 },
        z: Edges::Pml { low: 0, high: 0 },
        reflection: 1e-8,
        order: 3.0,
    };
    let solver = Solver3d::new(grid, lam, eps, walls).unwrap();
    let at = (2, 2, 2);
    assert!((solver.permittivity(Axis::X, at) - 1.5).norm() < 1e-12);
    assert!((solver.permittivity(Axis::Y, at) - 1.0).norm() < 1e-12);
    let mid = (2, 2, 2);
    let eps_mid = |x: f64, _: f64, _: f64| c64::new(if x < 0.2 { 1.0 } else { 3.0 }, 0.0);
    let solver_mid = Solver3d::new(grid, lam, eps_mid, walls).unwrap();
    assert!((solver_mid.permittivity(Axis::Y, mid) - 2.0).norm() < 1e-12);
    assert!((solver_mid.permittivity(Axis::Z, mid) - 2.0).norm() < 1e-12);
    assert!((solver_mid.permittivity(Axis::X, mid) - 3.0).norm() < 1e-12);
    // a current everywhere, walls included: the field tangential to a wall stays zero there
    let field = solver
        .solve(&vec![c64::new(1.0, 0.0); grid.unknowns()])
        .unwrap();
    for r in 0..grid.unknowns() {
        let (c, [i, j, k]) = grid.at(r);
        let on_wall =
            (c != Axis::X && i == 0) || (c != Axis::Y && j == 0) || (c != Axis::Z && k == 0);
        if on_wall {
            assert_eq!(field.values()[r], c64::new(0.0, 0.0), "{c:?} {i} {j} {k}");
        }
    }
    assert!(field.values().iter().any(|v| v.norm() > 0.0));
    assert!(solver.solve(&[c64::new(1.0, 0.0)]).is_err());
    assert!(solver.solve_magnetic(&[c64::new(1.0, 0.0)]).is_err());

    let bad = |g: Grid3d, b: Boundaries3d| {
        Solver3d::new(g, lam, |_, _, _| c64::new(1.0, 0.0), b).is_err()
    };
    assert!(bad(Grid3d { nz: 0, ..grid }, walls));
    assert!(bad(Grid3d { dy: -1.0, ..grid }, walls));
    assert!(bad(
        Grid3d {
            z0: f64::NAN,
            ..grid
        },
        walls
    ));
    assert!(bad(grid, Boundaries3d::pml(3)));
    assert!(bad(
        grid,
        Boundaries3d {
            reflection: 0.0,
            ..walls
        }
    ));
    assert!(bad(
        grid,
        Boundaries3d {
            z: Edges::Bloch { k: f64::INFINITY },
            ..walls
        }
    ));
}

#[test]
fn a_reused_analysis_gives_the_same_field() {
    let grid = Grid3d {
        nx: 8,
        ny: 8,
        nz: 8,
        dx: 0.05,
        dy: 0.05,
        dz: 0.05,
        x0: -0.2,
        y0: -0.2,
        z0: -0.2,
    };
    let cube = |x: f64, y: f64, z: f64| {
        let inside = x.abs() < 0.1 && y.abs() < 0.1 && z.abs() < 0.1;
        c64::new(if inside { 12.0 } else { 2.0 }, 0.0)
    };
    let lam = Wavelength::um(1.3).unwrap();
    let first = Solver3d::new(
        grid,
        Wavelength::um(1.55).unwrap(),
        cube,
        Boundaries3d::pml(2),
    )
    .unwrap();
    let fresh = Solver3d::new(grid, lam, cube, Boundaries3d::pml(2)).unwrap();
    let reused = first.reuse(lam, cube).unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    source[grid.index(Axis::Z, (4, 4, 4))] = c64::new(1.0, 0.0);
    let (a, b) = (
        fresh.solve(&source).unwrap(),
        reused.solve(&source).unwrap(),
    );
    let d = a
        .values()
        .iter()
        .zip(b.values())
        .map(|(x, y)| (x - y).norm())
        .fold(0.0, f64::max);
    assert!(d < 1e-12, "{d}");
    assert!(a.minus(&b).is_ok());
    let other = first.solve(&source).unwrap();
    assert!(a.minus(&other).is_err());
}

#[test]
#[ignore = "a timing, for the docs: FDFD3D_CELLS=32 cargo test --release fdfd::three -- --ignored --nocapture"]
fn time_a_3d_problem() {
    // a silicon strip (0.5 x 0.22 um) along x in oxide, PMLs of 6 cells all round, a current
    // on one edge: N^3 cells of 40 nm
    let n: usize = std::env::var("FDFD3D_CELLS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);
    let h = 0.04;
    let half = n as f64 * h / 2.0;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -half,
        y0: -half,
        z0: -half,
    };
    let strip = |_: f64, y: f64, z: f64| {
        let n: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
            3.476
        } else {
            1.444
        };
        c64::new(n * n, 0.0)
    };
    let lam = Wavelength::um(1.55).unwrap();
    let t0 = std::time::Instant::now();
    let (lattice, eps) = Solver3d::setup(grid, lam, strip, Boundaries3d::pml(6)).unwrap();
    let t_setup = t0.elapsed();
    let t1 = std::time::Instant::now();
    let entries = lattice.assemble(&eps);
    let t_assemble = t1.elapsed();
    let t2 = std::time::Instant::now();
    let matrix = faer::sparse::SparseColMat::<usize, c64>::try_new_from_triplets(
        grid.unknowns(),
        grid.unknowns(),
        &entries,
    )
    .unwrap();
    let symbolic = faer::sparse::linalg::solvers::SymbolicLu::try_new(matrix.symbolic()).unwrap();
    let t_symbolic = t2.elapsed();
    let t3 = std::time::Instant::now();
    let lu =
        faer::sparse::linalg::solvers::Lu::try_new_with_symbolic(symbolic.clone(), matrix.as_ref())
            .unwrap();
    let t_numeric = t3.elapsed();
    // one factorization alive at a time, so the peak memory is one solver's
    drop((lu, matrix, symbolic, lattice, eps));
    let nonzeros = entries.len();
    drop(entries);
    let solver = Solver3d::new(grid, lam, strip, Boundaries3d::pml(6)).unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    source[grid.index(Axis::Y, (n / 2, n / 2, n / 2))] = c64::new(1.0, 0.0);
    let t4 = std::time::Instant::now();
    let _ = solver.solve(&source).unwrap();
    let t_solve = t4.elapsed();
    println!(
        "TIMING {n}^3 cells, {} unknowns, {} nonzeros: permittivity {t_setup:?}, assembly \
         {t_assemble:?}, analysis {t_symbolic:?}, factorization {t_numeric:?}, one source \
         {t_solve:?}",
        grid.unknowns(),
        nonzeros
    );
}

#[test]
#[ignore = "a timing, for the docs: FDFD3D_LENGTH=40 cargo test --release fdfd::three::tests::time_qmr -- --ignored --nocapture"]
fn time_qmr_on_a_silicon_guide() {
    // Shin and Fan's Diel (their Fig. 9b), smaller: a silicon guide (400 x 300 nm, eps 12.09)
    // along x in vacuum, 1.55 um, 10 nm grid, PMLs of 10 cells all round, a y-polarized current
    // across the guide; 15 cells of vacuum beside it
    use crate::fdfd::{Formulation, IterativeSolver3d, Stopping};
    let nx: usize = std::env::var("FDFD3D_LENGTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let h = 0.01;
    let (ny, nz) = (40 + 2 * 25, 30 + 2 * 25);
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: -(ny as f64) * h / 2.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let guide = |_: f64, y: f64, z: f64| {
        c64::new(
            if y.abs() < 0.2 && z.abs() < 0.15 {
                12.09
            } else {
                1.0
            },
            0.0,
        )
    };
    let lam = Wavelength::um(1.55).unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    for k in 0..nz {
        for j in 0..ny {
            let [_, y, z] = grid.e_position(Axis::Y, (15, j, k));
            if y.abs() < 0.2 && z.abs() < 0.15 {
                source[grid.index(Axis::Y, (15, j, k))] = c64::new(1.0, 0.0);
            }
        }
    }
    let stopping = Stopping {
        tolerance: 1e-6,
        max_iterations: 200_000,
    };
    let mut fields = Vec::new();
    for formulation in [Formulation::CurlCurl, Formulation::ShinFan] {
        let t0 = std::time::Instant::now();
        let solver =
            IterativeSolver3d::new(grid, lam, guide, Boundaries3d::pml(10), formulation).unwrap();
        let t_build = t0.elapsed();
        let t1 = std::time::Instant::now();
        let (field, how) = solver.solve(&source, stopping).unwrap();
        let t_solve = t1.elapsed();
        println!(
            "QMR {formulation:?}: {nx} x {ny} x {nz} cells, {} unknowns, {} nonzeros: {} \
             iterations to {:e} in {t_solve:?} ({:?} per iteration), assembly {t_build:?}",
            grid.unknowns(),
            solver.nonzeros(),
            how.iterations,
            how.residual,
            t_solve / how.iterations as u32
        );
        fields.push(field);
    }
    // both against the curl-curl system itself
    let (lattice, eps) = Solver3d::setup(grid, lam, guide, Boundaries3d::pml(10)).unwrap();
    let a = crate::fdfd::krylov::Sparse::new(
        grid.unknowns(),
        lattice
            .assemble(&eps)
            .into_iter()
            .map(|t| (t.row, t.col, t.val)),
    );
    let b: Vec<c64> = source
        .iter()
        .map(|j| c64::new(0.0, -lattice.k0) * j)
        .collect();
    let norm = |v: &[c64]| v.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt();
    for (field, name) in fields.iter().zip(["CurlCurl", "ShinFan"]) {
        let ax = a.apply(field.values());
        let r: Vec<c64> = ax.iter().zip(&b).map(|(p, q)| p - q).collect();
        println!(
            "{name}: residual in the curl-curl system {:e}",
            norm(&r) / norm(&b)
        );
    }
    let d: Vec<c64> = fields[0]
        .values()
        .iter()
        .zip(fields[1].values())
        .map(|(p, q)| p - q)
        .collect();
    println!(
        "the two solutions differ by {:e} of the field",
        norm(&d) / norm(fields[0].values())
    );
}

#[test]
fn the_transformed_system_has_the_same_solution() {
    // Shin and Fan's Eq. 7 adds -s eps^-1 grad(div(eps E) - div b / k0^2), zero for the
    // solution: with the PML's stretch in grad and div, div curl = 0 holds on the grid, so the
    // solution is the same to round-off, here with silicon in oxide, PMLs on two axes and a
    // Bloch-periodic third
    let grid = Grid3d {
        nx: 10,
        ny: 12,
        nz: 9,
        dx: 0.04,
        dy: 0.04,
        dz: 0.04,
        x0: -0.2,
        y0: -0.24,
        z0: -0.18,
    };
    let boundaries = Boundaries3d {
        x: Edges::Bloch { k: 0.8 },
        ..Boundaries3d::pml(3)
    };
    let strip = |_: f64, y: f64, z: f64| {
        let n: f64 = if y.abs() < 0.1 && z.abs() < 0.06 {
            3.476
        } else {
            1.444
        };
        c64::new(n * n, 0.0)
    };
    let (lattice, eps) =
        Solver3d::setup(grid, Wavelength::um(1.55).unwrap(), strip, boundaries).unwrap();
    let mut b = vec![c64::new(0.0, 0.0); grid.unknowns()];
    b[grid.index(Axis::Y, (5, 6, 4))] = c64::new(0.0, -1.0);
    b[grid.index(Axis::Z, (2, 7, 5))] = c64::new(0.3, 0.2);
    let solve = |s: f64| -> Vec<c64> {
        use faer::linalg::solvers::Solve;
        let entries = lattice.assemble_with(&eps, s);
        let (lu, _) = crate::fdfd::factorize(&entries, grid.unknowns(), None).unwrap();
        let rhs = lattice.transformed_rhs(&eps, &b, s);
        let x = lu.solve(&faer::Mat::<c64>::from_fn(rhs.len(), 1, |r, _| rhs[r]));
        (0..rhs.len()).map(|r| x[(r, 0)]).collect()
    };
    let plain = solve(0.0);
    let largest = plain.iter().map(|v| v.norm()).fold(0.0, f64::max);
    for s in [-1.0, 1.0] {
        let d = solve(s)
            .iter()
            .zip(&plain)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max);
        // measured 7e-15 (s = -1) and 1.1e-14 (s = +1)
        assert!(d < 1e-12 * largest, "s {s}: {d} of {largest}");
    }
}

#[test]
fn qmr_converges_to_the_direct_solvers_field() {
    use crate::fdfd::{Formulation, IterativeSolver3d, Stopping};
    let (n, h) = (10, 0.05);
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -0.25,
        y0: -0.25,
        z0: -0.25,
    };
    let cube = |x: f64, y: f64, z: f64| {
        let inside = x.abs() < 0.1 && y.abs() < 0.1 && z.abs() < 0.1;
        c64::new(if inside { 12.0 } else { 2.0 }, 0.0)
    };
    let lam = Wavelength::um(1.55).unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    source[grid.index(Axis::Z, (5, 5, 5))] = c64::new(1.0, 0.0);
    let direct = Solver3d::new(grid, lam, cube, Boundaries3d::pml(3))
        .unwrap()
        .solve(&source)
        .unwrap();
    let largest = direct.values().iter().map(|v| v.norm()).fold(0.0, f64::max);
    for formulation in [Formulation::CurlCurl, Formulation::ShinFan] {
        let solver =
            IterativeSolver3d::new(grid, lam, cube, Boundaries3d::pml(3), formulation).unwrap();
        let stopping = Stopping {
            tolerance: 1e-10,
            max_iterations: 5000,
        };
        let (field, how) = solver.solve(&source, stopping).unwrap();
        assert!(how.residual <= 1e-10, "{formulation:?}: {how:?}");
        let d = field
            .values()
            .iter()
            .zip(direct.values())
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max);
        assert!(d < 1e-8 * largest, "{formulation:?}: {d} of {largest}");
        assert_eq!(field.grid(), grid);
        assert!(solver.solve(&source[1..], stopping).is_err());
    }
}

#[test]
fn shin_and_fans_square_converges_as_their_fig_3_shows() {
    // s = 0 stagnates at the residual's share in the near-null eigenspace, 0.707, then drops;
    // s = -1 doesn't stagnate; s = +1, strongly indefinite, is the slowest by far
    let curl_curl = shin_fan_square(0.0, 1e-6);
    let plateau = &curl_curl.history[4..40];
    assert!(
        plateau.iter().all(|r| (r - 0.707).abs() < 0.01),
        "{plateau:?}"
    );
    let shin_fan = shin_fan_square(-1.0, 1e-6);
    let indefinite = shin_fan_square(1.0, 1e-6);
    // read off their Fig. 3: about 114 and 77; measured 114, 79 and 801
    assert!(
        (curl_curl.iterations as f64 - 114.0).abs() <= 5.0,
        "{}",
        curl_curl.iterations
    );
    assert!(
        (shin_fan.iterations as f64 - 77.0).abs() <= 5.0,
        "{}",
        shin_fan.iterations
    );
    assert!(indefinite.iterations > 500, "{}", indefinite.iterations);
}

#[test]
#[ignore = "iteration counts, for the docs: cargo test --release fdfd::three::tests::iteration_counts -- --ignored --nocapture"]
fn iteration_counts_with_and_without_shin_and_fans_operator() {
    // 40^3 cells of 10 nm (0.4 um across), 1.55 um, an x-polarized current near the centre;
    // periodic, or PMLs of 10 cells all round; QMR to a relative residual of 1e-6 of each system
    use crate::fdfd::krylov::{Sparse, Stopping, qmr};
    let (n, h) = (40, 0.01);
    let half = n as f64 * h / 2.0;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -half,
        y0: -half,
        z0: -half,
    };
    let per = Edges::Bloch { k: 0.0 };
    let periodic = Boundaries3d {
        x: per,
        y: per,
        z: per,
        reflection: 1e-8,
        order: 3.0,
    };
    let pml = Boundaries3d::pml(10);
    type Eps = Box<dyn Fn(f64, f64, f64) -> c64>;
    let block = |eps: f64, size: f64, along_x: bool| -> Eps {
        Box::new(move |x: f64, y: f64, z: f64| {
            let inside = (along_x || x.abs() < size) && y.abs() < size && z.abs() < size;
            c64::new(if inside { eps } else { 1.0 }, 0.0)
        })
    };
    let cases: Vec<(&str, Boundaries3d, Eps)> = vec![
        ("vacuum, periodic", periodic, block(1.0, 0.0, false)),
        ("vacuum, PMLs", pml, block(1.0, 0.0, false)),
        (
            "oxide cube 200 nm, periodic",
            periodic,
            block(2.085, 0.1, false),
        ),
        (
            "silicon cube 200 nm, periodic",
            periodic,
            block(12.09, 0.1, false),
        ),
        (
            "oxide guide 100 nm through the PMLs",
            pml,
            block(2.085, 0.05, true),
        ),
        ("silicon cube 100 nm, PMLs", pml, block(12.09, 0.05, false)),
        (
            "silicon guide 100 nm through the PMLs",
            pml,
            block(12.09, 0.05, true),
        ),
    ];
    for (label, boundaries, eps) in cases {
        let (lattice, e) =
            Solver3d::setup(grid, Wavelength::um(1.55).unwrap(), eps, boundaries).unwrap();
        let mut rhs = vec![c64::new(0.0, 0.0); grid.unknowns()];
        rhs[grid.index(Axis::X, (n / 2 + 3, n / 2 + 3, n / 2 + 3))] = c64::new(0.0, -lattice.k0);
        let count = |s: f64| {
            let m = Sparse::new(
                grid.unknowns(),
                lattice
                    .assemble_with(&e, s)
                    .into_iter()
                    .map(|t| (t.row, t.col, t.val)),
            );
            let stopping = Stopping {
                tolerance: 1e-6,
                max_iterations: 30_000,
            };
            match qmr(&m, &lattice.transformed_rhs(&e, &rhs, s), stopping) {
                Ok((_, how)) => how.iterations.to_string(),
                Err(err) => err.to_string(),
            }
        };
        println!(
            "COUNT {label}: s = 0: {}, s = -1: {}",
            count(0.0),
            count(-1.0)
        );
    }
}

#[test]
#[ignore = "field errors, for the docs: cargo test --release fdfd::three::tests::field_error -- --ignored --nocapture"]
fn field_error_against_iterations() {
    // as iteration_counts_with_and_without_shin_and_fans_operator, with PMLs: QMR on each
    // operator to tolerances 1e-5 to 1e-8 of its own residual, and the field's error against a
    // reference converged to 1e-11 (checked against the curl-curl operator to 1e-8)
    use crate::fdfd::krylov::{Sparse, Stopping, qmr};
    let (n, h) = (40, 0.01);
    let half = n as f64 * h / 2.0;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -half,
        y0: -half,
        z0: -half,
    };
    let pml = Boundaries3d::pml(10);
    type Eps = Box<dyn Fn(f64, f64, f64) -> c64>;
    let block = |eps: f64, size: f64, along_x: bool| -> Eps {
        Box::new(move |x: f64, y: f64, z: f64| {
            let inside = (along_x || x.abs() < size) && y.abs() < size && z.abs() < size;
            c64::new(if inside { eps } else { 1.0 }, 0.0)
        })
    };
    let cases: Vec<(&str, Eps)> = vec![
        ("vacuum, PMLs", block(1.0, 0.0, false)),
        ("silicon cube 100 nm, PMLs", block(12.09, 0.05, false)),
        (
            "silicon guide 100 nm through the PMLs",
            block(12.09, 0.05, true),
        ),
    ];
    for (label, eps) in cases {
        let (lattice, e) = Solver3d::setup(grid, Wavelength::um(1.55).unwrap(), eps, pml).unwrap();
        let mut rhs = vec![c64::new(0.0, 0.0); grid.unknowns()];
        rhs[grid.index(Axis::X, (n / 2 + 3, n / 2 + 3, n / 2 + 3))] = c64::new(0.0, -lattice.k0);
        let solve = |s: f64, tolerance: f64| {
            let m = Sparse::new(
                grid.unknowns(),
                lattice
                    .assemble_with(&e, s)
                    .into_iter()
                    .map(|t| (t.row, t.col, t.val)),
            );
            let stopping = Stopping {
                tolerance,
                max_iterations: 40_000,
            };
            qmr(&m, &lattice.transformed_rhs(&e, &rhs, s), stopping).unwrap()
        };
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        let (reference, _) = solve(-1.0, 1e-11);
        let error = |x: &[c64]| {
            let d: Vec<c64> = x.iter().zip(&reference).map(|(a, b)| a - b).collect();
            norm(&d) / norm(&reference)
        };
        println!(
            "FIELD {label}: reference against s = 0 to 1e-8: {:.1e}",
            error(&solve(0.0, 1e-8).0)
        );
        for tolerance in [1e-5, 1e-6, 1e-7, 1e-8] {
            let (x0, h0) = solve(0.0, tolerance);
            let (x1, h1) = solve(-1.0, tolerance);
            println!(
                "FIELD {label}: {tolerance:e}: s = 0: {} iterations, error {:.1e}; s = -1: {} iterations, error {:.1e}",
                h0.iterations,
                error(&x0),
                h1.iterations,
                error(&x1)
            );
        }
    }
}

#[test]
#[ignore = "port modes against the mode solvers, for the docs: PORT_GRIDS=0.02,0.01,0.005 cargo test --release fdfd::three::tests::port_modes_against -- --ignored --nocapture"]
fn port_modes_against_the_mode_solvers() {
    use super::port_checks::{strip_port_indices, strip_solver_indices};
    let grids: Vec<f64> = std::env::var("PORT_GRIDS")
        .unwrap_or_else(|_| "0.02,0.01".into())
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    for h in grids {
        let t = std::time::Instant::now();
        let p = strip_port_indices(h);
        let s = strip_solver_indices(h, true);
        println!(
            "STRIP {h}: port TE {:.8} TM {:.8}; vector TE {:.8} TM {:.8}; Hadley TE {:.8} TM {:.8} ({:?})",
            p[0],
            p[1],
            s[0],
            s[1],
            s[2],
            s[3],
            t.elapsed()
        );
    }
}
