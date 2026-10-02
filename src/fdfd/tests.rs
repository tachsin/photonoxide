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
