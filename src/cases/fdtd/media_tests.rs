//! The dispersive media's and the Bloch boundaries' tests.

use super::checks::{grid, noise, periodic};
use super::*;
use crate::units::Wavelength;

#[test]
fn a_dispersive_boxs_steady_state_is_fdfds_field_with_the_leapfrogs_permittivity() {
    // exactly, to the solve's rounding; with the medium's own ε(ω̃), the ADE's O(Δt²) shows
    let (exact, own) = media_checks::against_fdfd();
    eprintln!("dispersive box: {exact:.2e} with the leapfrog's permittivity, {own:.2e} with ε(ω̃)");
    assert!(exact < 1e-10, "{exact}");
    assert!(own > 1e3 * exact, "{own}");
}

#[test]
fn the_leapfrogs_permittivity_is_the_mediums_to_second_order_in_the_step() {
    let medium = media_checks::drude_lorentz();
    for f in [0.3, 0.9, 1.4] {
        let frequency = Frequency::natural(f).unwrap();
        let exact = medium.permittivity(frequency);
        let errors: Vec<f64> = [0.02, 0.01, 0.005]
            .into_iter()
            .map(|dt| (medium.leapfrog_permittivity(frequency, dt) - exact).norm())
            .collect();
        assert!(
            (errors[0] / errors[1] - 4.0).abs() < 0.1 && (errors[1] / errors[2] - 4.0).abs() < 0.03,
            "{f}: {errors:?}"
        );
    }
}

#[test]
fn a_drude_half_space_reflects_as_fresnel_says_to_second_order() {
    let errors: Vec<f64> = [0.02, 0.01]
        .into_iter()
        .map(|h| media_checks::drude_reflection(h).0)
        .collect();
    eprintln!("Drude half-space: {errors:?}");
    assert!(errors[1] < errors[0] / 3.5, "{errors:?}");
    assert!(errors[1] < 2e-3, "{errors:?}");
}

#[test]
fn a_lorentz_slabs_group_delay_is_its_group_index() {
    let (coarse, _) = media_checks::lorentz_delay(0.01);
    let (fine, rows) = media_checks::lorentz_delay(0.005);
    eprintln!("Lorentz slab's group index: {coarse:.2e}, {fine:.2e}: {rows:?}");
    assert!(fine < coarse / 3.0, "{coarse} {fine}");
    assert!(fine < 1e-3, "{fine}");
    // the first pass's delay is the bulk group index's, to the interfaces' part
    for (_, measured, _, bulk) in rows {
        assert!((measured - bulk).abs() < 1.5e-3, "{measured} {bulk}");
    }
}

#[test]
fn a_medium_steps_stably_only_with_omega0_dt_below_2_and_a_courant_number_below_root_eps_inf() {
    // at C = 1 in 1D: Δt = Δ = 1, (KΔt)² up to 4
    let lorentz = |eps_inf: f64, w0dt: f64, damping: f64| Dispersive {
        eps_inf,
        poles: vec![Pole::Lorentz {
            strength: 1.0,
            resonance: Frequency::natural(w0dt / std::f64::consts::TAU).unwrap(),
            damping,
        }],
    };
    assert!(lorentz(1.0, 1.99, 0.0).stable(0.0, 1.0, 4.0));
    assert!(lorentz(1.0, 1.99, 0.05).stable(2.0, 1.0, 4.0));
    assert!(!lorentz(1.0, 2.01, 0.0).stable(0.0, 1.0, 4.0));
    assert!(!lorentz(0.99, 1.0, 0.0).stable(0.0, 1.0, 4.0));
    // a Drude term at any ω_pΔt
    for wpdt in [1.0, 5.0, 50.0] {
        let drude = Dispersive {
            eps_inf: 1.0,
            poles: vec![Pole::Drude {
                plasma: Frequency::natural(wpdt / std::f64::consts::TAU).unwrap(),
                damping: 0.1,
            }],
        };
        assert!(drude.stable(0.0, 1.0, 4.0), "{wpdt}");
    }
}

#[test]
fn media_that_would_grow_or_cant_be_stepped_are_refused() {
    let g = grid([10, 9, 8], 0.05);
    let make = || Simulation::new(g, |_, _, _| 1.0, Boundaries::walls(), 0.9).unwrap();
    let everywhere = |_: f64, _: f64, _: f64| true;
    let gain = Dispersive {
        eps_inf: 1.0,
        poles: vec![Pole::Lorentz {
            strength: -0.5,
            resonance: Frequency::natural(1.0).unwrap(),
            damping: 0.1,
        }],
    };
    assert!(make().with_medium(&gain, everywhere).is_err());
    let negative_damping = Dispersive {
        eps_inf: 1.0,
        poles: vec![Pole::Drude {
            plasma: Frequency::natural(1.0).unwrap(),
            damping: -0.1,
        }],
    };
    assert!(make().with_medium(&negative_damping, everywhere).is_err());
    // ω₀Δt = 2π × 40 × 0.026 = 6.5
    let fast = Dispersive {
        eps_inf: 1.0,
        poles: vec![Pole::Lorentz {
            strength: 1.0,
            resonance: Frequency::natural(40.0).unwrap(),
            damping: 0.0,
        }],
    };
    assert!(make().with_medium(&fast, everywhere).is_err());
    let thin = Dispersive {
        eps_inf: 0.5,
        poles: Vec::new(),
    };
    assert!(make().with_medium(&thin, everywhere).is_err());
    // the conductivity first, and the media before the run
    let drude = media_checks::drude_metal();
    let s = make().with_medium(&drude, everywhere).unwrap();
    assert!(s.with_conductivity(|_, _, _| 1.0).is_err());
    let mut s = make();
    s.step();
    assert!(s.with_medium(&drude, everywhere).is_err());
    // a total-field/scattered-field box needs no dispersive medium on its surface
    let g2 = Grid3d {
        nz: 1,
        ..grid([40, 40, 1], 0.05)
    };
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.5).unwrap();
    let wave = PlaneWave {
        low: (10, 10, 0),
        high: (25, 25, 1),
        direction: (1, 0, 0),
        polarization: [0.0, 0.0, 1.0],
        eps: 1.0,
        waveform: pulse,
    };
    let base = || Simulation::new(g2, |_, _, _| 1.0, Boundaries::cpml_2d(6), 0.9).unwrap();
    let inside = base().with_medium(&drude, |x, y, _| x.abs() < 0.1 && y.abs() < 0.1);
    assert!(inside.unwrap().add_plane_wave(wave).is_ok());
    let across = base().with_medium(&drude, |x, _, _| x > 0.0);
    assert!(across.unwrap().add_plane_wave(wave).is_err());
}

#[test]
fn a_sellmeier_formula_is_a_sum_of_lorentz_terms_exactly() {
    let silica = crate::material::silica();
    let medium = Dispersive::from_model(silica.model()).unwrap();
    for lam in [0.4, 0.8, 1.55, 2.0] {
        let wavelength = Wavelength::um(lam).unwrap();
        let exact = silica.permittivity(wavelength).unwrap();
        let got = medium.permittivity(Frequency::natural(1.0 / lam).unwrap());
        assert!((got - exact).norm() < 1e-13, "{lam}: {got} {exact}");
    }
    assert!(Dispersive::from_model(&crate::material::Model::Cauchy(vec![1.5, 0.01])).is_err());
}

#[test]
fn catalogue_fits_are_passive_and_within_their_stated_error_and_refuse_beyond_their_data() {
    let fits = media_checks::fits();
    eprintln!("fits: {fits:?}");
    assert!(fits[0].0 < 1e-4 && fits[1].0 < 1e-4, "{fits:?}");
    assert!(fits[2].0 < 1e-2, "{fits:?}");
    let silicon = media_checks::catalogue_material("si", None);
    let fit = Dispersive::fit(
        &silicon,
        Wavelength::um(1.3).unwrap(),
        Wavelength::um(1.6).unwrap(),
    )
    .unwrap();
    assert!(fit.medium.eps_inf >= 1.0);
    assert!(fit.medium.check().is_ok());
    // the stated error is the largest over the band
    for i in 0..=40 {
        let lam = 1.3 + 0.3 * i as f64 / 40.0;
        let d = (fit
            .medium
            .permittivity(Frequency::natural(1.0 / lam).unwrap())
            - silicon.permittivity(Wavelength::um(lam).unwrap()).unwrap())
        .norm();
        assert!(d <= fit.error * (1.0 + 1e-9), "{lam}: {d} {}", fit.error);
    }
    // Li's table starts at 1.2 µm
    let below = Dispersive::fit(
        &silicon,
        Wavelength::um(1.1).unwrap(),
        Wavelength::um(1.6).unwrap(),
    );
    assert!(matches!(below, Err(crate::Error::OutsideValidity { .. })));
}

#[test]
fn a_fitted_silicon_slab_reflects_as_airys_formula_with_the_catalogues_index() {
    let errors: Vec<f64> = [0.02, 0.01]
        .into_iter()
        .map(|h| media_checks::fitted_slab(h).0)
        .collect();
    eprintln!("fitted silicon slab: {errors:?}");
    assert!(errors[1] < errors[0] / 3.0, "{errors:?}");
    assert!(errors[1] < 1e-2, "{errors:?}");
}

#[test]
fn a_bloch_boxs_complex_steady_state_is_fdfds_bloch_field() {
    for dispersive in [false, true] {
        let error = bloch_checks::against_fdfd(dispersive);
        eprintln!("Bloch box (dispersive {dispersive}): {error:.2e}");
        assert!(error < 1e-10, "{dispersive}: {error}");
    }
}

#[test]
fn a_bloch_phase_of_one_turn_is_a_periodic_run() {
    // k L = 2π along x: the complex run's real part is the real periodic run's, its imaginary
    // part zero, to round-off
    let g = grid([10, 6, 4], 0.05);
    let turn = std::f64::consts::TAU / (g.nx as f64 * g.dx);
    let bloch = Boundaries {
        x: Edges::Bloch { k: turn },
        ..periodic()
    };
    let drude = media_checks::drude_metal();
    let start = |boundaries: Boundaries| {
        let mut s = Simulation::new(
            g,
            |x, _, _| if x > 0.0 { 2.0 } else { 1.0 },
            boundaries,
            0.9,
        )
        .unwrap()
        .with_medium(&drude, |_, y, _| y > 0.0)
        .unwrap();
        for c in Axis::ALL {
            for r in 0..g.cells() {
                s.e_mut(c)[r] = noise(r + 31 * c.index());
            }
        }
        s.run(200);
        s
    };
    let (complex, real) = (start(bloch), start(periodic()));
    assert!(complex.is_complex() && !real.is_complex());
    let largest = Axis::ALL
        .iter()
        .flat_map(|&c| real.e(c).iter())
        .fold(0.0f64, |m, v| m.max(v.abs()));
    for c in Axis::ALL {
        for r in 0..g.cells() {
            let re = (complex.e(c)[r] - real.e(c)[r]).abs();
            let im = complex.e_imaginary(c).unwrap()[r].abs();
            assert!(re < 1e-12 * largest && im < 1e-12 * largest, "{re} {im}");
        }
    }
}

#[test]
fn an_oblique_stack_transmits_as_the_transfer_matrices_say_to_second_order() {
    for tm in [false, true] {
        let errors: Vec<f64> = [0.02, 0.01]
            .into_iter()
            .map(|h| bloch_checks::multilayer(h, tm).0)
            .collect();
        eprintln!("oblique stack (TM {tm}): {errors:?}");
        assert!(errors[1] < errors[0] / 3.0, "{tm}: {errors:?}");
        assert!(errors[1] < 1e-2, "{tm}: {errors:?}");
    }
}

#[test]
fn a_photonic_crystals_bands_converge_to_the_analytic_ones() {
    let k = 0.3 * std::f64::consts::PI / 0.5;
    let exact = bloch_checks::analytic_bands(k, 2.0);
    let errors: Vec<f64> = [0.025, 0.0125]
        .into_iter()
        .map(|h| {
            let bands = bloch_checks::bands(h, k);
            assert_eq!(bands.len(), exact.len(), "{h}: {bands:?} {exact:?}");
            bands
                .iter()
                .zip(&exact)
                .fold(0.0f64, |m, (a, b)| m.max((a - b).abs() / b))
        })
        .collect();
    eprintln!("bands {exact:?}: {errors:?}");
    assert!(errors[1] < errors[0] / 3.5, "{errors:?}");
    assert!(errors[1] < 2e-3, "{errors:?}");
}

#[test]
fn sources_that_need_real_fields_are_refused_in_a_bloch_run() {
    let g = Grid3d {
        nz: 1,
        ..grid([30, 30, 1], 0.05)
    };
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 1.0 },
        ..Boundaries::cpml_2d(6)
    };
    let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.9).unwrap();
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.5).unwrap();
    assert!(
        s.add_plane_wave(PlaneWave {
            low: (0, 10, 0),
            high: (30, 20, 1),
            direction: (0, 1, 0),
            polarization: [0.0, 0.0, 1.0],
            eps: 1.0,
            waveform: pulse,
        })
        .is_err()
    );
    // a current's imaginary part is the imaginary part of a w(t)
    s.add_current(Current {
        field: Field::E,
        values: vec![(Axis::Z, (15, 15, 0), c64::new(0.3, 0.7))],
        waveform: pulse,
    })
    .unwrap();
    let p = s.add_probe(Field::E, Axis::Z, (15, 15, 0)).unwrap();
    s.run(5);
    assert_eq!(s.probe_complex(p).len(), 5);
    let twin = &s.bloch.as_ref().unwrap().twin;
    assert_eq!(twin.currents[0].values[0].2, c64::new(0.7, -0.3));
}
