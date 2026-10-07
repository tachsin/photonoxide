use super::*;
use super::checks::*;

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
                let mut s = Simulation::new(g, |_, y, _| if y.abs() < 0.1 { 4.0 } else { 1.0 }, Boundaries::cpml(3), 0.9).unwrap();
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
                s.run(120);
                Axis::ALL
                    .iter()
                    .flat_map(|&c| s.e(c).iter().chain(s.h(c)).map(|v| v.to_bits()).collect::<Vec<_>>())
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
