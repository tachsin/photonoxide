use super::port_checks::*;
use super::*;
use crate::fdfd::{Edges, Polarization, Side, Stopping};
use crate::mode::Polarization as Kind;

const WALL: Edges = Edges::Pml { low: 0, high: 0 };

fn lam() -> Wavelength {
    Wavelength::um(1.55).unwrap()
}

#[test]
fn a_slabs_port_modes_converge_to_the_exact_slab_at_second_order() {
    for kind in [Kind::Te, Kind::Tm] {
        let errors: Vec<f64> = [0.01, 0.005, 0.0025]
            .iter()
            .map(|&h| {
                let (got, exact) = slab_port_index(kind, h);
                (got - exact).abs()
            })
            .collect();
        // measured: TE 9.57e-4, 2.39e-4, 5.97e-5; TM 2.46e-3, 6.13e-4, 1.53e-4, as the 2D ports
        let orders: Vec<f64> = errors.windows(2).map(|e| (e[0] / e[1]).log2()).collect();
        assert!(errors[2] < 2e-4, "{kind:?}: {errors:?}");
        assert!(
            orders.iter().all(|&p| (p - 2.0).abs() < 0.05),
            "{kind:?}: {errors:?}, orders {orders:?}"
        );
    }
}

/// A straight strip along x in oxide with walls all round, a port at `plane`, and the solver's
/// lattice and permittivity.
fn walled_strip(n: (usize, usize, usize)) -> (Lattice, Vec<c64>, Grid3d) {
    let h = 0.05;
    let grid = Grid3d {
        nx: n.0,
        ny: n.1,
        nz: n.2,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: -(n.1 as f64) * h / 2.0,
        z0: -(n.2 as f64) * h / 2.0,
    };
    let boundaries = Boundaries3d {
        x: Edges::Pml { low: 4, high: 4 },
        y: WALL,
        z: WALL,
        reflection: 1e-8,
        order: 3.0,
    };
    let strip = |_: f64, y: f64, z: f64| {
        let n: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
            SILICON
        } else {
            OXIDE
        };
        c64::new(n * n, 0.0)
    };
    let (lattice, eps) = Solver3d::setup(grid, lam(), strip, boundaries).unwrap();
    (lattice, eps, grid)
}

#[test]
fn a_port_mode_solves_every_row_of_the_3d_system_along_a_straight_guide() {
    // the mode extended as e^(i beta x), E_x included, against the 3D matrix: zero in every row
    // clear of the PMLs, to round-off
    let (lattice, eps, grid) = walled_strip((14, 16, 12));
    let mode = lattice
        .port_modes(&eps, (Axis::X, 6), [0..16, 0..12], 1)
        .unwrap()
        .remove(0);
    let field: Vec<c64> = (0..grid.unknowns())
        .map(|r| mode.value(&lattice, r, crate::fdfd::Direction::Forward))
        .collect();
    let mut row = Vec::new();
    let (mut worst, mut largest): (f64, f64) = (0.0, 0.0);
    for r in 0..grid.unknowns() {
        let (_, at) = grid.at(r);
        if !(5..=8).contains(&at[0]) {
            continue;
        }
        lattice.row_into(&eps, 0.0, r, &mut row);
        let applied: c64 = row.iter().map(|&(s, a)| a * field[s]).sum();
        let scale: f64 = row.iter().map(|&(s, a)| (a * field[s]).norm()).sum();
        worst = worst.max(applied.norm());
        largest = largest.max(scale);
    }
    assert!(worst < 1e-13 * largest, "{worst} of {largest}");
}

#[test]
fn a_real_mode_normalized_by_its_lorentz_form_carries_unit_power() {
    let (lattice, eps, _) = walled_strip((14, 16, 12));
    let modes = lattice
        .port_modes(&eps, (Axis::X, 6), [0..16, 0..12], 3)
        .unwrap();
    for mode in modes
        .iter()
        .filter(|m| m.beta().im.abs() < 1e-9 * m.beta().norm())
    {
        assert!((mode.power() - 1.0).abs() < 1e-12, "{}", mode.power());
        // the tangential field real, and the normal one, half a step on (e^(i beta dx / 2)),
        // imaginary
        let half = (c64::new(0.0, -0.025) * mode.beta()).exp();
        let (mut largest, mut wrong): (f64, f64) = (0.0, 0.0);
        for v in 0..12 {
            for u in 0..16 {
                for component in Axis::ALL {
                    let e = mode.e(component, (u, v));
                    let part = if component == Axis::X {
                        (e * half).re
                    } else {
                        e.im
                    };
                    largest = largest.max(e.norm());
                    wrong = wrong.max(part.abs());
                }
            }
        }
        assert!(wrong < 1e-12 * largest, "{wrong} of {largest}");
    }
}

#[test]
fn the_projection_separates_a_mode_going_each_way() {
    let (lattice, eps, _) = walled_strip((14, 16, 12));
    let modes = lattice
        .port_modes(&eps, (Axis::X, 6), [0..16, 0..12], 2)
        .unwrap();
    for (m, mode) in modes.iter().enumerate() {
        for other in &modes {
            for (direction, want) in [
                (crate::fdfd::Direction::Forward, (1.0, 0.0)),
                (crate::fdfd::Direction::Backward, (0.0, 1.0)),
            ] {
                let (a, b) =
                    lattice.mode_amplitudes(other, &|r| mode.value(&lattice, r, direction));
                let same = std::ptr::eq(mode, other);
                let want = if same { want } else { (0.0, 0.0) };
                assert!(
                    (a - want.0).norm() < 1e-12 && (b - want.1).norm() < 1e-12,
                    "mode {m} {direction:?}: {a} {b}"
                );
            }
        }
    }
}

#[test]
fn a_straight_strip_carries_its_mode_whole() {
    // S11 = S22 = 0 and S21 = S12 = e^(i beta L), to round-off: measured 1.8e-15
    let (worst, n) = straight_strip();
    assert!(worst < 1e-12, "{worst}");
    // a lossy mode: the PMLs are close
    assert!(n.re > 2.4 && n.re < 2.5 && n.im > 0.0, "{n}");
}

#[test]
fn s_is_symmetric_through_a_width_step() {
    // Lorentz reciprocity, kept by the scheme with the unconjugated form: measured 2e-15
    let s = width_step();
    let asymmetry = (s[1][0] - s[0][1]).norm() / s[1][0].norm();
    assert!(asymmetry < 1e-12, "{asymmetry}");
    // the step reflects a little and transmits most
    assert!(s[0][0].norm() > 1e-3 && s[0][0].norm() < 0.1, "{s:?}");
    assert!(s[1][0].norm() > 0.9, "{s:?}");
}

#[test]
fn a_closed_lossless_guide_keeps_its_power_but_for_evanescent_modes() {
    // all the propagating modes on both sides: S is unitary but for the power the evanescent
    // modes carry across the ports' planes, which falls as e^(-2 kappa d) with the ports'
    // distance d from the step: measured 1e-4 at 8 cells and 2e-6 at 20
    let (near, count) = closed_step(8);
    let (far, _) = closed_step(20);
    assert_eq!(count, 3);
    assert_eq!(near.len(), 6);
    let (u_near, u_far) = (unitarity(&near), unitarity(&far));
    assert!(u_far < 5e-6, "{u_far}");
    assert!(u_near > 20.0 * u_far, "{u_near} against {u_far}");
}

#[test]
fn a_backward_wave_is_forward_by_its_power() {
    // a silicon strip in a small metal box of air guides a mode whose power flows against its
    // phase: it is given with Re beta < 0, and with it S is unitary
    let h = 0.05;
    let (nx, ny, nz) = (34, 12, 8);
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
    let boundaries = Boundaries3d {
        x: Edges::Pml { low: 6, high: 6 },
        y: WALL,
        z: WALL,
        reflection: 1e-8,
        order: 3.0,
    };
    let strip = |_: f64, y: f64, z: f64| {
        let n: f64 = if (y - 0.05).abs() < 0.2 && z.abs() < 0.1 {
            SILICON
        } else {
            1.0
        };
        c64::new(n * n, 0.0)
    };
    let solver = Solver3d::new(grid, lam(), strip, boundaries).unwrap();
    let propagating = |plane: usize| -> Vec<PortMode3d> {
        solver
            .port_modes(Axis::X, plane, 8)
            .unwrap()
            .into_iter()
            .filter(|m| m.beta().im.abs() < 1e-9 * m.beta().norm())
            .collect()
    };
    let (left, right) = (propagating(8), propagating(25));
    assert_eq!(left.len(), 4);
    let backward: Vec<&PortMode3d> = left.iter().filter(|m| m.beta().re < 0.0).collect();
    assert_eq!(backward.len(), 1);
    assert!((backward[0].power() - 1.0).abs() < 1e-12);
    let ports: Vec<Port3d> = left
        .into_iter()
        .map(|mode| Port3d {
            mode,
            side: Side::Left,
        })
        .chain(right.into_iter().map(|mode| Port3d {
            mode,
            side: Side::Right,
        }))
        .collect();
    let s = solver.s_matrix(&ports).unwrap();
    assert!(unitarity(&s) < 1e-11, "{}", unitarity(&s));
}

#[test]
fn uniform_along_z_the_s_matrix_is_the_2d_solvers() {
    // H along z: the same equations, to the eigensolver's tolerance (measured 2.1e-10); E along
    // z: the PMLs half a cell apart in the two grids (measured 8.9e-9)
    let ez = two_d_s_difference(Polarization::Ez);
    let hz = two_d_s_difference(Polarization::Hz);
    assert!(hz < 1e-9, "{hz}");
    assert!(ez < 5e-8, "{ez}");
}

#[test]
fn a_port_normal_to_y_or_z_is_a_port_normal_to_x_turned() {
    // the same walled strip along each axis in turn: the same effective indices, and S21 the
    // same
    let h = 0.05;
    let along = |axis: Axis| {
        let (b, c) = axis.others();
        let mut n = [0usize; 3];
        n[axis.index()] = 16;
        n[b.index()] = 14;
        n[c.index()] = 10;
        let mut step = [h; 3];
        step[c.index()] = 0.04;
        let grid = Grid3d {
            nx: n[0],
            ny: n[1],
            nz: n[2],
            dx: step[0],
            dy: step[1],
            dz: step[2],
            x0: -(n[0] as f64) * step[0] / 2.0,
            y0: -(n[1] as f64) * step[1] / 2.0,
            z0: -(n[2] as f64) * step[2] / 2.0,
        };
        let pml = Edges::Pml { low: 4, high: 4 };
        let mut edges = [WALL; 3];
        edges[axis.index()] = pml;
        let boundaries = Boundaries3d {
            x: edges[0],
            y: edges[1],
            z: edges[2],
            reflection: 1e-8,
            order: 3.0,
        };
        let eps = move |x: f64, y: f64, z: f64| {
            let p = [x, y, z];
            let (u, v) = (p[b.index()], p[c.index()]);
            let n: f64 = if u.abs() < 0.2 && v.abs() < 0.1 {
                SILICON
            } else {
                OXIDE
            };
            c64::new(n * n, 0.0)
        };
        let solver = Solver3d::new(grid, lam(), eps, boundaries).unwrap();
        let mode = |plane: usize| solver.port_modes(axis, plane, 1).unwrap().remove(0);
        let ports = [
            Port3d {
                mode: mode(6),
                side: Side::Left,
            },
            Port3d {
                mode: mode(9),
                side: Side::Right,
            },
        ];
        let s = solver.s_matrix(&ports).unwrap();
        (ports[0].mode.effective_index(), s[1][0])
    };
    let (nx, sx) = along(Axis::X);
    for axis in [Axis::Y, Axis::Z] {
        let (n, s) = along(axis);
        assert!((n - nx).norm() < 1e-12, "{axis:?}: {n} against {nx}");
        assert!((s - sx).norm() < 1e-12, "{axis:?}: {s} against {sx}");
    }
}

#[test]
fn qmr_gives_the_direct_solvers_s_matrix() {
    let h = 0.05;
    let (nx, ny, nz) = (20, 12, 10);
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
    // a closed box of oxide, 0.6 x 0.5 um, the strip stepping from 0.3 to 0.4 um wide
    let boundaries = Boundaries3d {
        x: Edges::Pml { low: 5, high: 5 },
        y: WALL,
        z: WALL,
        reflection: 1e-8,
        order: 3.0,
    };
    let eps = |x: f64, y: f64, z: f64| {
        let half = if x < 0.5 { 0.15 } else { 0.2 };
        let n: f64 = if y.abs() < half && z.abs() < 0.11 {
            SILICON
        } else {
            OXIDE
        };
        c64::new(n * n, 0.0)
    };
    let direct = Solver3d::new(grid, lam(), eps, boundaries).unwrap();
    let ports = |modes: &dyn Fn(usize) -> PortMode3d| {
        [
            Port3d {
                mode: modes(7),
                side: Side::Left,
            },
            Port3d {
                mode: modes(12),
                side: Side::Right,
            },
        ]
    };
    let exact = direct
        .s_matrix(&ports(&|p| {
            direct.port_modes(Axis::X, p, 1).unwrap().remove(0)
        }))
        .unwrap();
    for formulation in [Formulation::CurlCurl, Formulation::ShinFan] {
        let iterative = IterativeSolver3d::new(grid, lam(), eps, boundaries, formulation).unwrap();
        let stopping = Stopping {
            tolerance: 1e-10,
            max_iterations: 20_000,
        };
        let s = iterative
            .s_matrix(
                &ports(&|p| iterative.port_modes(Axis::X, p, 1).unwrap().remove(0)),
                stopping,
            )
            .unwrap();
        for q in 0..2 {
            for p in 0..2 {
                let d = (s[q][p] - exact[q][p]).norm();
                // measured 6e-12 to 6e-11
                assert!(d < 1e-9, "{formulation:?} S{}{}: {d}", q + 1, p + 1);
            }
        }
    }
}

#[test]
fn ports_that_dont_fit_are_errors() {
    let (lattice, eps, _) = walled_strip((14, 16, 12));
    let whole = || [0..16, 0..12];
    // in or beside the PML along x, and past the grid's end
    assert!(lattice.port_modes(&eps, (Axis::X, 5), whole(), 1).is_err());
    assert!(lattice.port_modes(&eps, (Axis::X, 8), whole(), 1).is_err());
    // a window too small or past the grid
    assert!(
        lattice
            .port_modes(&eps, (Axis::X, 6), [0..2, 0..12], 1)
            .is_err()
    );
    assert!(
        lattice
            .port_modes(&eps, (Axis::X, 6), [0..17, 0..12], 1)
            .is_err()
    );
    // along an axis without PMLs: y is walled, so a port normal to y is too close to its ends
    // only if it sits by them; Bloch-periodic along the port's axis is an error
    let grid = Grid3d {
        nx: 10,
        ny: 10,
        nz: 10,
        dx: 0.05,
        dy: 0.05,
        dz: 0.05,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let bloch = Boundaries3d {
        x: Edges::Bloch { k: 0.0 },
        y: Edges::Bloch { k: 0.5 },
        ..Boundaries3d::pml(2)
    };
    let (l, e) = Solver3d::setup(grid, lam(), |_, _, _| c64::new(2.0, 0.0), bloch).unwrap();
    assert!(l.port_modes(&e, (Axis::X, 5), [0..10, 0..10], 1).is_err());
    // a Bloch-periodic side with k != 0
    assert!(l.port_modes(&e, (Axis::Z, 5), [0..10, 0..10], 1).is_err());
    // a mode of another problem
    let (other, other_eps, _) = walled_strip((14, 18, 12));
    let foreign = other
        .port_modes(&other_eps, (Axis::X, 6), [0..18, 0..12], 1)
        .unwrap()
        .remove(0);
    let ports = [Port3d {
        mode: foreign,
        side: Side::Left,
    }];
    assert!(
        lattice
            .s_matrix(&ports, |_| unreachable!("no solve for a foreign mode"))
            .is_err()
    );
}
