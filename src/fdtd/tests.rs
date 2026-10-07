use super::checks::*;
use super::*;
use crate::units::Wavelength;

#[test]
fn the_scheme_keeps_taflove_and_brodwins_dispersion_relation_exactly() {
    for (m, courant) in [((3, 0), 0.9), ((2, 2), 0.9), ((1, 3), 0.5), ((4, 1), 1.0)] {
        let (measured, theory) = dispersion(m, courant);
        assert!(
            (measured - theory).abs() < 1e-12 * theory,
            "{m:?} at C = {courant}: {measured} against {theory}"
        );
    }
}

#[test]
fn a_courant_number_above_one_or_a_bad_boundary_is_refused() {
    let g = grid([8, 8, 8], 0.05);
    assert!(Simulation::new(g, |_, _, _| 1.0, Boundaries::walls(), 1.01).is_err());
    assert!(Simulation::new(g, |_, _, _| 1.0, Boundaries::walls(), 0.0).is_err());
    let bloch = Boundaries {
        x: Edges::Bloch { k: 1.0 },
        ..Boundaries::walls()
    };
    assert!(Simulation::new(g, |_, _, _| 1.0, bloch, 0.9).is_err());
    assert!(Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml(4), 0.9).is_err());
    assert!(Simulation::new(g, |_, _, _| -1.0, Boundaries::walls(), 0.9).is_err());
}

#[test]
fn the_leapfrogs_energy_is_conserved_in_a_conducting_box() {
    let drift = energy_drift();
    assert!(drift < 1e-12, "{drift}");
}

#[test]
fn the_fields_are_the_same_bits_on_any_number_of_threads() {
    let g = grid([14, 12, 10], 0.05);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut s = Simulation::new(
                    g,
                    |x, y, _| if y.abs() < 0.1 && x > 0.2 { 4.0 } else { 1.0 },
                    Boundaries::cpml(3),
                    0.9,
                )
                .unwrap();
                s.add_source(Source {
                    field: Field::E,
                    component: Axis::Y,
                    at: (7, 6, 5),
                    waveform: Waveform::Gaussian {
                        frequency: Frequency::natural(1.0).unwrap(),
                        width: 0.6,
                        delay: 1.8,
                    },
                })
                .unwrap();
                s.add_plane_wave(PlaneWave {
                    low: (4, 4, 4),
                    high: (10, 8, 6),
                    direction: (1, 2, -1),
                    polarization: [0.0, 1.0, 2.0],
                    eps: 1.0,
                    waveform: Waveform::pulse(Frequency::natural(1.2).unwrap(), 0.5).unwrap(),
                })
                .unwrap();
                s.add_dipole(Dipole {
                    field: Field::H,
                    component: Axis::X,
                    position: [0.012, -0.021, 0.033],
                    amplitude: c64::new(0.01, 0.002),
                    waveform: Waveform::pulse(Frequency::natural(0.9).unwrap(), 0.4).unwrap(),
                })
                .unwrap();
                s.run(120);
                Axis::ALL
                    .iter()
                    .flat_map(|&c| {
                        s.e(c)
                            .iter()
                            .chain(s.h(c))
                            .map(|v| v.to_bits())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<u64>>()
            })
    };
    let one = run(1);
    for threads in [2, 4, 5, 20] {
        assert!(run(threads) == one, "{threads} threads differ from one");
    }
}
#[test]
fn the_cpmls_reflection_falls_with_its_thickness() {
    let errors: Vec<f64> = [4, 8, 16].into_iter().map(cpml_error).collect();
    assert!(errors[0] > errors[1] && errors[1] > errors[2], "{errors:?}");
    // −60 dB at 16 cells
    assert!(errors[2] < 1e-3, "{errors:?}");
}

#[test]
fn a_lossy_boxs_steady_state_is_fdfds_field_at_the_leapfrogs_frequency() {
    // exactly, to the solves' rounding; at ω the leapfrog's dispersion shows
    let (tilde, omega) = against_fdfd(64, Some(2.0));
    eprintln!("lossy: {tilde:.2e} at w~, {omega:.2e} at w");
    assert!(tilde < 1e-10, "{tilde}");
    assert!(omega > 1e3 * tilde, "{omega}");
}

#[test]
fn with_cpmls_the_field_converges_to_fdfds_as_the_step_falls() {
    // the CPML's recursive convolution is FDFD's stretch to first order in Δt
    let (coarse, _) = against_fdfd(64, None);
    let (fine, _) = against_fdfd(128, None);
    eprintln!("cpml: {coarse:.2e} at 64 steps a period, {fine:.2e} at 128");
    assert!(fine < 0.6 * coarse, "{coarse} {fine}");
    assert!(fine < 5e-4, "{fine}");
}

#[test]
fn the_analytic_waveform_is_the_waveform_and_a_pulse_has_its_bandwidth() {
    let f = Frequency::natural(0.65).unwrap();
    let pulse = Waveform::pulse(f, 0.2).unwrap();
    for w in [
        pulse,
        Waveform::DifferentiatedGaussian {
            width: 0.4,
            delay: 2.0,
        },
        Waveform::Continuous {
            frequency: f,
            amplitude: c64::new(0.3, -1.2),
            ramp: 3.0,
        },
    ] {
        for n in 0..200 {
            let t = 0.07 * n as f64;
            assert!((w.complex_at(t).re - w.at(t)).abs() < 1e-15, "{w:?} at {t}");
        }
    }
    // half the power at half the bandwidth either side of the carrier
    let dt = 0.01;
    let power = |x: f64| {
        pulse
            .analytic_spectrum(Field::E, dt, 6000, Frequency::natural(x).unwrap())
            .norm_sqr()
    };
    let peak = power(0.65);
    for x in [0.55, 0.75] {
        assert!((power(x) / peak - 0.5).abs() < 1e-9, "{}", power(x) / peak);
    }
    assert!(Waveform::pulse(f, 0.0).is_err());
}

#[test]
fn a_dipole_between_values_keeps_its_integral() {
    let g = grid([10, 9, 8], 0.05);
    let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::walls(), 0.9).unwrap();
    let amplitude = c64::new(0.7, -0.2);
    s.add_dipole(Dipole {
        field: Field::E,
        component: Axis::X,
        position: [0.013, -0.031, 0.044],
        amplitude,
        waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.3).unwrap(),
    })
    .unwrap();
    let values = &s.currents[0].values;
    assert_eq!(values.len(), 8);
    let sum: c64 = values.iter().map(|v| v.2).sum::<c64>() * (g.dx * g.dy * g.dz);
    assert!((sum - amplitude).norm() < 1e-15, "{sum}");
    // outside the span of E_x's values along x
    assert!(
        s.add_dipole(Dipole {
            field: Field::E,
            component: Axis::X,
            position: [0.25, 0.0, 0.0],
            amplitude,
            waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.3).unwrap(),
        })
        .is_err()
    );
}

#[test]
fn a_runs_dft_per_unit_source_spectrum_is_fdfds_field_at_the_leapfrogs_frequency() {
    let error = spectrum_against_fdfd();
    eprintln!("spectrum: {error:.2e}");
    assert!(error < 1e-10, "{error}");
}

#[test]
fn a_plane_waves_box_leaks_nothing_but_round_off_at_any_angle() {
    for (direction, polarization, three_d) in [
        ((1, 0, 0), [0.0, 0.0, 1.0], false),
        ((2, 1, 0), [0.0, 0.0, 1.0], false),
        ((1, -3, 0), [1.0, 0.0, 0.0], false),
        ((0, 0, 1), [1.0, 0.0, 0.0], true),
        ((2, 1, 1), [1.0, -1.0, 0.0], true),
    ] {
        let leak = tfsf_leakage(direction, polarization, three_d);
        eprintln!("{direction:?} {polarization:?}: {leak:.2e}");
        assert!(leak < 1e-12, "{direction:?}: {leak}");
    }
}

#[test]
fn a_slab_in_a_box_reflects_as_airys_formula_says_to_second_order() {
    let errors: Vec<f64> = [0.02, 0.01, 0.005].into_iter().map(tfsf_slab).collect();
    eprintln!("slab: {errors:?}");
    assert!(
        errors[1] < errors[0] / 3.0 && errors[2] < errors[1] / 3.0,
        "{errors:?}"
    );
    assert!(errors[2] < 1e-3, "{errors:?}");
}

#[test]
fn a_mode_source_in_a_lossy_box_is_fdfds_mode_source_exactly() {
    let error = mode_source_against_fdfd();
    eprintln!("mode source against FDFD: {error:.2e}");
    assert!(error < 1e-10, "{error}");
}

#[test]
fn a_mode_source_launches_its_mode_one_way_with_its_power() {
    let (_, backward, forward) = mode_source_open(false)[0];
    eprintln!(
        "mode source: backward {backward:.2e} of forward, forward {forward:.6} of the mode's"
    );
    assert!(backward < 1e-5, "{backward}");
    assert!((forward - 1.0).abs() < 5e-4, "{forward}");
}

#[test]
fn a_pulsed_mode_source_sends_back_more_away_from_its_carrier() {
    let band = mode_source_open(true);
    eprintln!("pulsed mode source: {band:?}");
    // at the carrier as the continuous wave; either side the mode's dispersion
    assert!(band[1].1 < 1e-5, "{band:?}");
    assert!(band[0].1 > band[1].1 && band[2].1 > band[1].1, "{band:?}");
    assert!(band[0].1 < 1e-3 && band[2].1 < 1e-3, "{band:?}");
}

#[test]
fn a_gaussian_beam_keeps_its_waist_and_divergence() {
    let straight = beam(0.0);
    let (waist, focus, divergence) = beam_fit(&straight);
    let dt = 1.0 / 32.0;
    let on_grid = beam_divergence(1.5, 1.0, Some((0.05, dt)));
    let exact = beam_divergence(1.5, 1.0, None);
    let paraxial = 1.0 / (std::f64::consts::PI * 1.5);
    eprintln!(
        "beam: waist {waist:.6}, focus {focus:.4}, divergence {divergence:.6} (grid {on_grid:.6}, relative {:.2e}, \
         exact {exact:.6}, paraxial {paraxial:.6}), backward {:.2e}",
        (divergence - on_grid).abs() / on_grid,
        straight.backward
    );
    assert!((waist - 1.5).abs() < 1e-3, "{waist}");
    assert!((focus - 2.0).abs() < 0.1, "{focus}");
    assert!(
        (divergence - on_grid).abs() < 1e-4 * on_grid,
        "{divergence} {on_grid}"
    );
    assert!(
        (divergence - paraxial).abs() < 0.05 * paraxial,
        "{divergence} {paraxial}"
    );
    assert!(straight.backward < 1e-5, "{}", straight.backward);
    // tilted by 10°: one way, along its tilt to the grid's dispersion
    let tilted = beam(10f64.to_radians());
    let (d0, _, y0) = tilted.columns[0];
    let (d1, _, y1) = tilted.columns[4];
    let slope = (y1 - y0) / (d1 - d0);
    let predicted = beam_centre_slope(1.5, 10f64.to_radians(), 1.0, 0.05, dt);
    eprintln!(
        "tilted beam: centre's slope {slope:.6} (predicted {predicted:.6}, tan 10 degrees {:.6}), \
         backward {:.2e}",
        10f64.to_radians().tan(),
        tilted.backward
    );
    assert!(
        (slope - predicted).abs() < 1e-3 * predicted,
        "{slope} {predicted}"
    );
    assert!(tilted.backward < 1e-5, "{}", tilted.backward);
}

#[test]
fn sources_that_cant_be_launched_as_asked_are_refused() {
    let g = Grid3d {
        nz: 1,
        ..grid([40, 40, 1], 0.05)
    };
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.5).unwrap();
    let block = |x: f64, _: f64, _: f64| if x > 0.4 { 4.0 } else { 1.0 };
    let mut s = Simulation::new(g, block, Boundaries::cpml_2d(6), 0.9).unwrap();
    let wave = |low: (usize, usize, usize), high: (usize, usize, usize), direction| PlaneWave {
        low,
        high,
        direction,
        polarization: [0.0, 0.0, 1.0],
        eps: 1.0,
        waveform: pulse,
    };
    // in the vacuum, clear of the CPMLs: fine
    assert!(
        s.add_plane_wave(wave((10, 10, 0), (25, 25, 1), (2, 1, 0)))
            .is_ok()
    );
    // a face in the CPML, a face against it, the box not whole along the periodic z, a
    // direction along z, a box across the block
    assert!(
        s.add_plane_wave(wave((5, 10, 0), (25, 25, 1), (1, 0, 0)))
            .is_err()
    );
    assert!(
        s.add_plane_wave(wave((6, 10, 0), (25, 25, 1), (1, 0, 0)))
            .is_err()
    );
    assert!(
        s.add_plane_wave(wave((10, 10, 0), (25, 25, 0), (1, 0, 0)))
            .is_err()
    );
    assert!(
        s.add_plane_wave(wave((10, 10, 0), (25, 25, 1), (1, 0, 1)))
            .is_err()
    );
    assert!(
        s.add_plane_wave(wave((10, 10, 0), (30, 25, 1), (1, 0, 0)))
            .is_err()
    );
    // the polarization along the direction, a zero direction
    let mut along = wave((10, 10, 0), (25, 25, 1), (1, 0, 0));
    along.polarization = [1.0, 0.0, 0.0];
    assert!(s.add_plane_wave(along).is_err());
    assert!(
        s.add_plane_wave(wave((10, 10, 0), (25, 25, 1), (0, 0, 0)))
            .is_err()
    );
    // a beam needs a carrier and a uniform medium where it enters
    let beam = GaussianBeam {
        axis: Axis::Y,
        plane: 10,
        direction: crate::fdfd::Direction::Forward,
        centre: (0.0, -0.5),
        waist: 0.4,
        focus: 0.0,
        tilt: Axis::Z,
        angle: 0.0,
        polarization: BeamPolarization::Perpendicular,
        eps: 1.0,
    };
    let no_carrier = Waveform::DifferentiatedGaussian {
        width: 0.3,
        delay: 1.5,
    };
    assert!(s.add_beam(&beam, no_carrier).is_err());
    // its plane crosses the block
    assert!(s.add_beam(&beam, pulse).is_err());
    let along_x = GaussianBeam {
        axis: Axis::X,
        tilt: Axis::Y,
        ..beam
    };
    assert!(s.add_beam(&along_x, pulse).is_ok());
    // tilted towards the axis that doesn't vary
    let towards_z = GaussianBeam {
        tilt: Axis::Z,
        angle: 0.2,
        ..along_x
    };
    assert!(s.add_beam(&towards_z, pulse).is_err());
}

#[test]
fn a_mode_source_needs_its_own_modes_frequency() {
    use crate::fdfd::{Direction, Formulation, IterativeSolver3d};
    let g = grid([16, 14, 20], 0.05);
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let mut s = Simulation::new(g, guide, Boundaries::cpml(4), 0.9).unwrap();
    let mode_at = |lam: Wavelength| {
        IterativeSolver3d::new(
            g,
            lam,
            |x, y, z| c64::new(guide(x, y, z), 0.0),
            crate::fdfd::Boundaries3d::pml(4),
            Formulation::CurlCurl,
        )
        .unwrap()
        .port_modes(Axis::Z, 7, 1)
        .unwrap()
        .remove(0)
    };
    let cw = Waveform::Continuous {
        frequency,
        amplitude: c64::new(1.0, 0.0),
        ramp: 3.0,
    };
    // at 1.55 µm itself rather than at the leapfrog's frequency
    let wrong = mode_at(Wavelength::um(1.55).unwrap());
    assert!(s.add_mode_source(&wrong, Direction::Forward, cw).is_err());
    let right = mode_at(s.fdfd_wavelength(frequency).unwrap());
    assert!(s.add_mode_source(&right, Direction::Forward, cw).is_ok());
    assert!(s.add_mode_source(&right, Direction::Backward, cw).is_ok());
}

#[test]
fn a_plane_waves_e_is_its_waveform_to_second_order() {
    let errors: Vec<f64> = [0.05, 0.025, 0.0125]
        .into_iter()
        .map(|h| (plane_wave_amplitude(h) - 1.0).abs())
        .collect();
    eprintln!("plane wave's amplitude: {errors:?}");
    assert!(
        errors[1] < errors[0] / 3.0 && errors[2] < errors[1] / 3.0,
        "{errors:?}"
    );
    assert!(errors[2] < 2e-3, "{errors:?}");
}
