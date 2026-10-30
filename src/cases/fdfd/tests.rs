use super::checks::*;
use super::*;

/// A plane wave at `angle` from air onto 220 nm of silicon on oxide, at 1.55 µm, on a grid of
/// `h` µm: the reflectance and transmittance from the fluxes, and what the transfer matrices
/// say. Layers lie along x; the film fills 0 < y < 0.22.
fn slab_on_oxide(polarization: Polarization, h: f64, angle: f64) -> ((f64, f64), (f64, f64)) {
    slab_ratios(&slab_run(polarization, h, angle, (PML, 1e-8)))
}

#[test]
fn a_slab_reflects_and_transmits_as_the_transfer_matrices_say_at_second_order() {
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let errors: Vec<f64> = [0.02, 0.01, 0.005]
            .iter()
            .map(|&h| {
                let ((r, _), (r_exact, _)) = slab_on_oxide(polarization, h, 30f64.to_radians());
                (r - r_exact).abs()
            })
            .collect();
        let orders: Vec<f64> = errors.windows(2).map(|e| (e[0] / e[1]).log2()).collect();
        assert!(errors[2] < 2e-3, "{polarization:?}: errors {errors:?}");
        assert!(
            orders.iter().all(|&p| p > 1.7),
            "{polarization:?}: errors {errors:?}, orders {orders:?}"
        );
    }
}

#[test]
fn the_flux_is_the_same_through_every_row_of_a_lossless_region() {
    // in the run with the slab, from the bottom PML up through the film to the source: the
    // scheme's own flux is conserved to round-off, interfaces included
    for polarization in [Polarization::Ez, Polarization::Hz] {
        for deg in [0.0, 30.0, 60.0_f64] {
            let run = slab_run(polarization, 0.01, deg.to_radians(), (PML, 1e-8));
            let spread = flux_spread(&run);
            assert!(spread < 1e-10, "{polarization:?} {deg}: {spread}");
        }
    }
}

#[test]
fn reflection_and_transmission_add_up_to_one_but_for_the_pmls() {
    // the PMLs send a little back, which interferes with the incident wave, more at grazing
    // incidence (a PML graded for R at normal incidence reflects about R^cos(theta)): what is
    // left of R + T - 1 is that, and a thicker, stronger PML shrinks it
    for polarization in [Polarization::Ez, Polarization::Hz] {
        for deg in [0.0, 30.0, 60.0_f64] {
            let angle = deg.to_radians();
            let ((r, t), _) = slab_ratios(&slab_run(polarization, 0.01, angle, (PML, 1e-8)));
            let ((rs, ts), _) = slab_ratios(&slab_run(polarization, 0.01, angle, (40, 1e-16)));
            let (loose, strong) = ((r + t - 1.0).abs(), (rs + ts - 1.0).abs());
            assert!(loose < 2e-4, "{polarization:?} {deg}: R {r} + T {t}");
            assert!(
                strong < 1e-6,
                "{polarization:?} {deg}: R {rs} + T {ts} with the strong PML"
            );
        }
    }
}

#[test]
fn the_pml_barely_reflects() {
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let r = pml_reflection(polarization);
        assert!(r < 1e-5, "{polarization:?}: the PML reflects {r}");
    }
}

#[test]
fn each_component_sees_its_own_average_and_bad_problems_are_errors() {
    let grid = Grid {
        nx: 2,
        ny: 2,
        dx: 1.0,
        dy: 1.0,
        x0: 0.0,
        y0: 0.0,
    };
    let lam = Wavelength::um(1.0).unwrap();
    // a vertical interface through the middle of the left column: half 1, half 3
    let eps = |x: f64, _: f64| c64::new(if x < 0.5 { 1.0 } else { 3.0 }, 0.0);
    let walls = Boundaries {
        x: Edges::Pml { low: 0, high: 0 },
        y: Edges::Pml { low: 0, high: 0 },
        reflection: 1e-8,
        order: 3.0,
    };
    assert!((averaged(&eps, (grid.x(0), grid.y(0)), (1.0, 1.0), 2, 8) - 2.0).norm() < 1e-12);
    assert!((averaged(&eps, (grid.x(1), grid.y(0)), (1.0, 1.0), 2, 8) - 3.0).norm() < 1e-12);
    // across the interface (E_x): harmonic, 1 / ((1/1 + 1/3) / 2) = 1.5; along it (E_y): 2
    assert!((averaged(&eps, (grid.x(0), grid.y(0)), (1.0, 1.0), 0, 8) - 1.5).norm() < 1e-12);
    assert!((averaged(&eps, (grid.x(0), grid.y(0)), (1.0, 1.0), 1, 8) - 2.0).norm() < 1e-12);
    let solver = Solver2d::new(grid, Polarization::Ez, lam, eps, walls).unwrap();
    assert!(solver.solve(&[c64::new(1.0, 0.0)]).is_err());

    let bad = |g: Grid, b: Boundaries| {
        Solver2d::new(g, Polarization::Ez, lam, |_, _| c64::new(1.0, 0.0), b).is_err()
    };
    assert!(bad(Grid { nx: 0, ..grid }, walls));
    assert!(bad(Grid { dx: -1.0, ..grid }, walls));
    assert!(bad(grid, Boundaries::pml(1)));
    assert!(bad(
        grid,
        Boundaries {
            reflection: 1.0,
            ..walls
        }
    ));
    assert!(bad(
        grid,
        Boundaries {
            x: Edges::Bloch { k: f64::NAN },
            ..walls
        }
    ));
}

/// The error of the fundamental port mode of a 220 nm silicon slab (3.476 in 1.444) against
/// the exact slab mode, on an h grid.
fn port_mode_error(polarization: Polarization, h: f64) -> f64 {
    let (got, exact) = port_mode_index(polarization, h);
    (got - exact).abs()
}

#[test]
fn a_port_mode_is_the_exact_slab_mode_at_second_order() {
    // from 10 nm down the core's faces are on the grid's faces
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let errors: Vec<f64> = [0.01, 0.005, 0.0025]
            .iter()
            .map(|&h| port_mode_error(polarization, h))
            .collect();
        let orders: Vec<f64> = errors.windows(2).map(|e| (e[0] / e[1]).log2()).collect();
        assert!(errors[2] < 3e-4, "{polarization:?}: {errors:?}");
        assert!(
            orders.iter().all(|&p| p > 1.9),
            "{polarization:?}: {errors:?} {orders:?}"
        );
    }
}

#[test]
fn a_straight_guide_transmits_everything_with_the_modes_phase() {
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let error = straight_guide_error(polarization);
        assert!(error < 1e-10, "{polarization:?}: {error}");
    }
}

#[test]
fn a_step_is_reciprocal_and_a_te_mode_reflects_as_fresnel_says() {
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let (s, n1, n2) = step(polarization);
        let (s21, s12) = (s[1][0], s[0][1]);
        assert!(
            (s21 - s12).norm() < 1e-10 * s21.norm(),
            "{polarization:?}: S21 {s21} S12 {s12}"
        );
        // a lossless step: what isn't reflected or transmitted in the mode radiates. For TE the
        // modal impedance is the effective index, and the mode reflects as Fresnel's formula on
        // the two says (1.1642e-3 against 1.1650e-3); a TM mode's isn't, and its weaker
        // confinement makes the step radiate 6 %
        let (r, t) = (s[0][0].norm_sqr(), s21.norm_sqr());
        let radiated = 1.0 - r - t;
        let fresnel = ((n1 - n2) / (n1 + n2)).powi(2);
        match polarization {
            Polarization::Ez => {
                assert!(radiated > 0.0 && radiated < 1e-3, "Ez: R {r} T {t}");
                assert!(
                    (r / fresnel - 1.0).abs() < 0.01,
                    "Ez: R {r}, Fresnel {fresnel}"
                );
            }
            Polarization::Hz => {
                assert!(radiated > 0.0 && radiated < 0.1, "Hz: R {r} T {t}");
            }
        }
    }
}

#[test]
fn a_reused_analysis_gives_the_same_answer_at_another_wavelength() {
    let eps = |_: f64, y: f64| {
        let n: f64 = if y.abs() < 0.11 { 3.476 } else { 1.444 };
        c64::new(n * n, 0.0)
    };
    let first = guide(Polarization::Ez, 0.02, 2.0, (3.476, 1.444), |_| 0.22);
    let lam = Wavelength::um(1.5).unwrap();
    let fresh = Solver2d::new(
        first.grid(),
        Polarization::Ez,
        lam,
        eps,
        Boundaries::pml(20),
    )
    .unwrap();
    let reused = first.reuse(lam, eps).unwrap();
    let s = |solver: &Solver2d| solver.s_matrix(&two_ports(solver)).unwrap();
    let (a, b) = (s(&fresh), s(&reused));
    for q in 0..2 {
        for p in 0..2 {
            assert!(
                (a[q][p] - b[q][p]).norm() < 1e-12,
                "{q}{p}: {} vs {}",
                a[q][p],
                b[q][p]
            );
        }
    }
}

#[test]
#[ignore = "a timing, for the docs: cargo test --release -- --ignored --nocapture"]
fn time_a_reused_analysis() {
    let solver = guide(Polarization::Ez, 0.01, 4.0, (3.476, 1.444), |x| {
        if x < 2.0 { 0.22 } else { 0.3 }
    });
    let eps = |_: f64, y: f64| c64::new(if y.abs() < 0.11 { 12.08 } else { 2.085 }, 0.0);
    let g = solver.grid();
    for lam in [1.5, 1.55, 1.6] {
        let lam = Wavelength::um(lam).unwrap();
        let t0 = std::time::Instant::now();
        let _ = Solver2d::new(g, Polarization::Ez, lam, eps, Boundaries::pml(20)).unwrap();
        let fresh = t0.elapsed();
        let t1 = std::time::Instant::now();
        let _ = solver.reuse(lam, eps).unwrap();
        let reused = t1.elapsed();
        println!(
            "TIMING {} x {} = {} unknowns: fresh {fresh:?}, reused {reused:?}",
            g.nx,
            g.ny,
            g.nx * g.ny
        );
        let ports = two_ports(&solver);
        let t2 = std::time::Instant::now();
        let _ = solver
            .solve_system(&solver.mode_source(&ports[0].mode, Direction::Forward))
            .unwrap();
        println!("TIMING one source: {:?}", t2.elapsed());
    }
}

#[test]
fn windowed_ports_see_one_guide_each() {
    // two silicon slabs at y = ±0.8 µm, 1.6 µm apart, each with its own port at each end
    let h = 0.02;
    let pml = 20;
    let grid = Grid {
        nx: (2.0 / h) as usize + 2 * pml,
        ny: (3.0 / h) as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let eps = |_: f64, y: f64| {
        let n: f64 = if (y.abs() - 0.8).abs() < 0.11 {
            3.476
        } else {
            1.444
        };
        c64::new(n * n, 0.0)
    };
    let solver = Solver2d::new(
        grid,
        Polarization::Ez,
        Wavelength::um(1.55).unwrap(),
        eps,
        Boundaries::pml(pml),
    )
    .unwrap();
    let row = |y: f64| ((y - grid.y0) / h).round() as usize;
    let (bottom, top) = (row(-1.5)..row(0.0), row(0.0)..row(1.5));
    let (a, b) = (column(&solver, 0.3), column(&solver, 1.7));
    let port = |c: usize, rows: std::ops::Range<usize>, side: Side| Port {
        mode: solver.port_modes_within(c, rows, 1).unwrap().remove(0),
        side,
    };
    let ports = [
        port(a, bottom.clone(), Side::Left),
        port(a, top.clone(), Side::Left),
        port(b, bottom, Side::Right),
        port(b, top, Side::Right),
    ];
    // each window's mode is the lone guide's, but for the window's wall 0.69 µm from the core,
    // where the field is down to about 1e-3 of its peak
    let lone = guide(Polarization::Ez, h, 2.0, (3.476, 1.444), |_| 0.22);
    let alone = lone.port_modes(column(&lone, 0.3), 1).unwrap().remove(0);
    for p in &ports {
        let d = (p.mode.effective_index() - alone.effective_index()).norm();
        assert!(d < 1e-5, "{d}");
    }
    let s = solver.s_matrix(&ports).unwrap();
    // each guide carries its mode through with the open guide's phase (the window's wall pulls
    // the window mode's β 1.2e-5 lower, 1.7e-5 rad over the 1.4 µm); what is left is the
    // guides' evanescent coupling across the gap, 2.5e-6, and reflections of 3e-7
    let beta = alone.beta();
    let through = (c64::new(0.0, 1.0) * beta * ((b - a) as f64 * h)).exp();
    for (from, to) in [(0, 2), (1, 3)] {
        assert!(
            (s[to][from] - through).norm() < 1e-5,
            "{from}->{to}: {} vs {through}",
            s[to][from]
        );
        assert!(s[from][from].norm() < 1e-6, "{from}: {}", s[from][from]);
    }
    assert!(
        s[3][0].norm() < 1e-5 && s[2][1].norm() < 1e-5,
        "{} {}",
        s[3][0],
        s[2][1]
    );
    let worst = (0..4)
        .flat_map(|q| (0..4).map(move |p| (q, p)))
        .map(|(q, p)| (s[q][p] - s[p][q]).norm())
        .fold(0.0, f64::max);
    assert!(worst < 1e-10, "{worst}");
}

#[test]
#[ignore = "slow in a debug build; the validation report runs it in release (fdfd/adjoint-gradient-ez, -hz)"]
fn the_adjoint_gradient_is_the_finite_differences() {
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let error = gradient_check(polarization);
        // 1.4e-7 and 8e-8: the finite differences' own round-off, about 1e-10 / δ
        assert!(error < 1e-6, "{polarization:?}: {error}");
    }
}

#[test]
fn an_h_gradient_needs_a_problem_given_cell_by_cell() {
    let solver = guide(Polarization::Hz, 0.02, 2.0, (3.476, 1.444), |_| 0.22);
    let ports = two_ports(&solver);
    let field = solver
        .solve_system(&solver.mode_source(&ports[0].mode, Direction::Forward))
        .unwrap();
    assert!(
        solver
            .mode_power_gradient(&field, &ports[1].mode, Direction::Forward)
            .is_err()
    );
}

#[test]
fn a_backward_source_launches_the_twin_with_the_same_tangential_e_at_unit_amplitude() {
    // the backward twin of a mode has the same tangential E: with H along z, the opposite H_z;
    // launched backward, its amplitude is 1 at the mode's column, and the field's H_z there is
    // minus the profile, its E_z (E along z) the profile itself
    for (polarization, twin) in [(Polarization::Ez, 1.0), (Polarization::Hz, -1.0)] {
        let solver = guide(polarization, 0.02, 2.0, (3.476, 1.444), |_| 0.22);
        let ports = two_ports(&solver);
        let mode = &ports[1].mode;
        let field = solver
            .solve_system(&solver.mode_source(mode, Direction::Backward))
            .unwrap();
        let (forward, backward) = field.mode_amplitudes(mode);
        assert!(
            // forward: what the far PML sends back, 5e-6
            (backward - 1.0).norm() < 1e-9 && forward.norm() < 1e-4,
            "{polarization:?}: {forward} {backward}"
        );
        let peak = (0..solver.grid().ny)
            .max_by(|&a, &b| {
                mode.profile()[a]
                    .norm()
                    .total_cmp(&mode.profile()[b].norm())
            })
            .unwrap();
        let ratio = field.at(mode.column(), peak) / mode.profile()[peak];
        assert!((ratio - twin).norm() < 1e-4, "{polarization:?}: {ratio}");
    }
}

#[test]
fn a_solved_fields_residual_is_rounding_and_tells_another_source() {
    let grid = Grid {
        nx: 40,
        ny: 40,
        dx: 0.05,
        dy: 0.05,
        x0: -1.0,
        y0: -1.0,
    };
    for polarization in [Polarization::Ez, Polarization::Hz] {
        // a silicon square in oxide, a point source at its centre
        let solver = Solver2d::new(
            grid,
            polarization,
            Wavelength::um(1.55).unwrap(),
            |x, y| {
                if x.abs() < 0.25 && y.abs() < 0.25 {
                    c64::new(12.0, 0.0)
                } else {
                    c64::new(2.1, 0.0)
                }
            },
            Boundaries::pml(10),
        )
        .unwrap();
        let mut rhs = vec![c64::new(0.0, 0.0); 1600];
        rhs[20 * 40 + 20] = c64::new(1.0, 0.0);
        let field = solver.solve_system(&rhs).unwrap();
        let solved = solver.residual(&field, &rhs).unwrap();
        assert!(solved < 1e-10, "{polarization:?}: {solved:e}");
        // against twice the source: b − A u = 2b − b, over ‖2b‖
        let twice: Vec<c64> = rhs.iter().map(|b| b * 2.0).collect();
        let off = solver.residual(&field, &twice).unwrap();
        assert!((off - 0.5).abs() < 1e-9, "{polarization:?}: {off}");
        assert!(solver.residual(&field, &rhs[1..]).is_err());
    }
}

#[test]
fn a_permittivity_or_a_source_that_isnt_finite_is_an_error() {
    // each of these used to factorize, or solve, into a field of NaNs
    let grid = Grid {
        nx: 40,
        ny: 30,
        dx: 0.05,
        dy: 0.05,
        x0: -1.0,
        y0: -0.75,
    };
    let w = Wavelength::um(1.55).unwrap();
    let nan = |_: f64, _: f64| c64::new(f64::NAN, 0.0);
    for polarization in [Polarization::Ez, Polarization::Hz] {
        let e = Solver2d::new(grid, polarization, w, nan, Boundaries::pml(8))
            .err()
            .unwrap()
            .to_string();
        assert!(e.contains("every value must be finite"), "{e}");
    }
    // with H along z the operator divides by the faces' permittivity
    let zero = |_: f64, _: f64| c64::new(0.0, 0.0);
    let e = Solver2d::new(grid, Polarization::Hz, w, zero, Boundaries::pml(8))
        .err()
        .unwrap()
        .to_string();
    assert!(e.contains("fdfd permittivity"), "{e}");
    let solver = Solver2d::new(
        grid,
        Polarization::Ez,
        w,
        |_, _| c64::new(2.1, 0.0),
        Boundaries::pml(8),
    )
    .unwrap();
    let mut source = vec![c64::new(0.0, 0.0); 40 * 30];
    source[600] = c64::new(f64::NAN, 0.0);
    let e = solver.solve(&source).err().unwrap().to_string();
    assert!(e.contains("every value must be finite"), "{e}");
}

#[test]
fn the_2d_solver_factorizes_and_solves_through_its_backend() {
    use crate::backend::Choice;
    use crate::backend::tests::Recording;
    // photonoxide's own backend under another name, counting what it is asked: each
    // factorization, each solve and each transposed solve goes through the registry, and
    // gives the bits of the solver that named no backend
    let recording = Recording::register("recording-2d");
    let choice = Choice::parse("recording-2d").unwrap();
    let grid = Grid {
        nx: 60,
        ny: 50,
        dx: 0.04,
        dy: 0.04,
        x0: -1.2,
        y0: -1.0,
    };
    let eps = |_: f64, y: f64| c64::new(if y.abs() < 0.2 { 12.0 } else { 2.1 }, 0.0);
    let lam = Wavelength::um(1.55).unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.nx * grid.ny];
    source[25 * grid.nx + 20] = c64::new(1.0, 0.0);
    let since = |before: [usize; 4]| {
        let now = recording.counts();
        [0, 1, 2, 3].map(|k| now[k] - before[k])
    };
    let bloch = Boundaries {
        x: Edges::Bloch { k: 0.7 },
        ..Boundaries::pml(10)
    };
    // PMLs all round: L D Lᵀ of the symmetric similarity; a Bloch side: LU
    for (boundaries, symmetric) in [(Boundaries::pml(10), true), (bloch, false)] {
        let own = Solver2d::new(grid, Polarization::Ez, lam, eps, boundaries).unwrap();
        assert_eq!(
            own.direct_solver(),
            format!("photonoxide {}", env!("CARGO_PKG_VERSION"))
        );
        let before = recording.counts();
        let routed =
            Solver2d::new_on(grid, Polarization::Ez, lam, eps, boundaries, &choice).unwrap();
        assert_eq!(
            routed.direct_solver(),
            format!("recording-2d {}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(routed.lu.symmetric(), symmetric);
        // one analysis, one factorization
        assert_eq!(since(before), [1, 1, 0, 0]);
        // a solve and its step of refinement, to the bits of photonoxide's own
        let field = routed.solve(&source).unwrap();
        assert_eq!(field.values, own.solve(&source).unwrap().values);
        assert_eq!(since(before), [1, 1, 2, 0]);
        // the transposed solve: by the transposed factors of an LU, by the same factors of a
        // symmetric similarity
        let adjoint = routed.solve_transposed(&source).unwrap();
        assert_eq!(adjoint, own.solve_transposed(&source).unwrap());
        assert_eq!(
            since(before),
            if symmetric {
                [1, 1, 4, 0]
            } else {
                [1, 1, 2, 2]
            }
        );
        // reused at another wavelength: the same backend, no new analysis, one factorization
        let other = Wavelength::um(1.5).unwrap();
        let reused = routed.reuse(other, eps).unwrap();
        assert_eq!(reused.direct_solver(), routed.direct_solver());
        let [analyses, factorizations, ..] = since(before);
        assert_eq!((analyses, factorizations), (1, 2));
        assert_eq!(
            reused.solve(&source).unwrap().values,
            own.reuse(other, eps)
                .unwrap()
                .solve(&source)
                .unwrap()
                .values
        );
    }
    // a backend nothing is registered under is an error, not photonoxide's own in its place
    let missing = Choice::parse("not-registered-2d").unwrap();
    let e = Solver2d::new_on(
        grid,
        Polarization::Ez,
        lam,
        eps,
        Boundaries::pml(10),
        &missing,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(e.contains("not-registered-2d"), "{e}");
    let cells = vec![c64::new(2.1, 0.0); grid.nx * grid.ny];
    assert!(
        Solver2d::from_cells_on(
            grid,
            Polarization::Ez,
            lam,
            &cells,
            Boundaries::pml(10),
            &missing
        )
        .is_err()
    );
}

#[test]
fn faers_lu_as_the_backend_gives_the_same_field_to_rounding() {
    use crate::backend::Choice;
    let grid = Grid {
        nx: 60,
        ny: 50,
        dx: 0.04,
        dy: 0.04,
        x0: -1.2,
        y0: -1.0,
    };
    let eps = |_: f64, y: f64| c64::new(if y.abs() < 0.2 { 12.0 } else { 2.1 }, 0.0);
    let lam = Wavelength::um(1.55).unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.nx * grid.ny];
    source[25 * grid.nx + 20] = c64::new(1.0, 0.0);
    let own = Solver2d::new(grid, Polarization::Ez, lam, eps, Boundaries::pml(10)).unwrap();
    let faer = Solver2d::new_on(
        grid,
        Polarization::Ez,
        lam,
        eps,
        Boundaries::pml(10),
        &Choice::parse("faer").unwrap(),
    )
    .unwrap();
    assert!(faer.direct_solver().starts_with("faer "));
    // it takes no symmetric form: the system's LU
    assert!(!faer.lu.symmetric());
    let (a, b) = (own.solve(&source).unwrap(), faer.solve(&source).unwrap());
    let largest = a.values.iter().map(|v| v.norm()).fold(0.0, f64::max);
    let off = a
        .values
        .iter()
        .zip(&b.values)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max);
    assert!(off < 1e-11 * largest, "{off} of {largest}");
    let (a, b) = (
        own.solve_transposed(&source).unwrap(),
        faer.solve_transposed(&source).unwrap(),
    );
    let off = a
        .iter()
        .zip(&b)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max);
    assert!(off < 1e-11 * largest, "{off} of {largest}");
}

#[test]
fn auto_takes_the_backend_this_machines_records_say_and_says_why() {
    use crate::backend::auto::{Measured, Outcome, with};
    use crate::backend::{Choice, Form};
    let grid = Grid {
        nx: 60,
        ny: 50,
        dx: 0.04,
        dy: 0.04,
        x0: -1.2,
        y0: -1.0,
    };
    let eps = |_: f64, y: f64| c64::new(if y.abs() < 0.2 { 12.0 } else { 2.1 }, 0.0);
    let lam = Wavelength::um(1.55).unwrap();
    let pml = Boundaries::pml(10);
    let mut source = vec![c64::new(0.0, 0.0); grid.nx * grid.ny];
    source[25 * grid.nx + 20] = c64::new(1.0, 0.0);
    let threads = rayon::current_num_threads();
    let record = |backend: &str, form: Form, seconds: f64| Measured {
        family: "fdfd2d".into(),
        form,
        unknowns: 3_000,
        backend: backend.into(),
        threads,
        seconds,
        peak_bytes: None,
        outcome: Outcome::Accurate,
        // 2026-10-12
        unix_seconds: 1_791_763_200,
    };
    // no records: photonoxide's own, as ever
    let plain = with(Vec::new(), None, || {
        Solver2d::new(grid, Polarization::Ez, lam, eps, pml).unwrap()
    });
    assert_eq!(
        plain.direct_solver(),
        format!("photonoxide {}", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        plain.direct_choice(),
        Some("no benchmark records on this machine")
    );
    // faer's LU measured twice as fast on symmetric 2D systems of this size: auto takes it
    let symmetric = vec![
        record("photonoxide", Form::Symmetric, 2.0),
        record("faer", Form::Symmetric, 1.0),
    ];
    let (chosen, named, bloch) = with(symmetric, None, || {
        let chosen = Solver2d::new(grid, Polarization::Ez, lam, eps, pml).unwrap();
        // a solver that names its backend isn't auto: the records aren't asked
        let named =
            Solver2d::new_on(grid, Polarization::Ez, lam, eps, pml, &Choice::Photonoxide).unwrap();
        // a Bloch side is a general system, of which there are no records
        let bloch = Boundaries {
            x: Edges::Bloch { k: 0.7 },
            ..pml
        };
        let bloch = Solver2d::new(grid, Polarization::Ez, lam, eps, bloch).unwrap();
        (chosen, named, bloch)
    });
    assert!(chosen.direct_solver().starts_with("faer "));
    let why = format!(
        "2.0 times faster than photonoxide's own at 3000 unknowns on {threads} thread{} on this \
         machine, measured 2026-10-12",
        if threads == 1 { "" } else { "s" }
    );
    assert_eq!(chosen.direct_choice(), Some(why.as_str()));
    assert!(named.direct_solver().starts_with("photonoxide "));
    assert_eq!(named.direct_choice(), None);
    assert!(bloch.direct_solver().starts_with("photonoxide "));
    // the field is the problem's, whichever backend: to rounding of photonoxide's own
    let (a, b) = (
        plain.solve(&source).unwrap(),
        chosen.solve(&source).unwrap(),
    );
    let largest = a.values.iter().map(|v| v.norm()).fold(0.0, f64::max);
    let off = a
        .values
        .iter()
        .zip(&b.values)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max);
    assert!(off < 1e-11 * largest, "{off} of {largest}");
    // a solver reused for another wavelength keeps the backend and the reason: one decision
    // for a sweep, whatever the records become
    let reused = chosen.reuse(Wavelength::um(1.5).unwrap(), eps).unwrap();
    assert_eq!(reused.direct_solver(), chosen.direct_solver());
    assert_eq!(reused.direct_choice(), chosen.direct_choice());
    // and the bits of photonoxide's own when the records leave the choice with it
    assert_eq!(named.solve(&source).unwrap().values, a.values);
}
