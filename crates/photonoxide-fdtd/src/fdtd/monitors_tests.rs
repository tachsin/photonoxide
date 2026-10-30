use super::monitors_checks::*;
use super::*;

#[test]
fn a_transform_monitor_is_the_sum_by_hand() {
    let error = transforms_against_hand();
    assert!(error < 1e-15, "{error}");
}

#[test]
fn the_flux_through_a_plane_and_out_of_a_box_is_fdfds() {
    let error = flux_against_fdfd();
    assert!(error < 1e-9, "{error}");
}

#[test]
fn no_flux_leaves_a_closed_box_with_no_source_and_no_loss_in_it() {
    let (net, largest) = box_balance(false);
    assert!(net < 1e-12 * largest, "{net} {largest}");
    // with the source inside, what leaves is what it radiates
    let (net, largest) = box_balance(true);
    assert!(net > 0.1 * largest, "{net} {largest}");
}

#[test]
fn a_mode_monitors_amplitudes_are_fdfds() {
    let error = modes_against_fdfd();
    assert!(error < 1e-9, "{error}");
}

#[test]
fn a_lossy_cavitys_resonances_are_the_leapfrogs_own_to_round_off() {
    let (count, worst) = cavity_resonances();
    assert!(count >= 4, "{count}");
    assert!(worst < 1e-11, "{count} {worst}");
}

#[test]
fn a_slabs_resonance_and_q_converge_to_the_continuums_at_second_order() {
    let (coarse, fine) = (slab_resonance(0.02), slab_resonance(0.01));
    for (c, f) in [(coarse.0, fine.0), (coarse.1, fine.1)] {
        assert!((c / f - 4.0).abs() < 0.1, "{coarse:?} {fine:?}");
    }
    assert!(fine.0 < 1e-3 && fine.1 < 1e-2, "{fine:?}");
}

#[test]
fn the_transforms_and_fluxes_are_the_same_bits_on_any_number_of_threads() {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let g = checks::grid([14, 12, 10], 0.05);
                let mut s = Simulation::new(
                    g,
                    |x, _, _| if x > 0.1 { 3.0 } else { 1.0 },
                    Boundaries::cpml(3),
                    0.9,
                )
                .unwrap();
                s.add_source(Source {
                    field: Field::E,
                    component: Axis::Y,
                    at: (7, 6, 5),
                    waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.5).unwrap(),
                })
                .unwrap();
                let frequencies = [0.8, 1.0, 1.3].map(|f| Frequency::natural(f).unwrap());
                let dft = s.add_dft((3, 2, 1), (11, 9, 8), &frequencies).unwrap();
                let flux = s
                    .add_flux_box([(4, 9), (3, 8), (3, 6)], &frequencies)
                    .unwrap();
                s.run(150);
                let mut bits: Vec<u64> = s.flux(flux).iter().map(|v| v.to_bits()).collect();
                for f in 0..3 {
                    for field in [Field::E, Field::H] {
                        for c in Axis::ALL {
                            for k in 1..=8 {
                                for j in 2..=9 {
                                    for i in 3..=11 {
                                        let v = s.dft(dft).value(field, c, (i, j, k), f).unwrap();
                                        bits.push(v.re.to_bits());
                                        bits.push(v.im.to_bits());
                                    }
                                }
                            }
                        }
                    }
                }
                bits
            })
    };
    let one = run(1);
    for threads in [2, 4, 20] {
        assert!(run(threads) == one, "{threads} threads differ from one");
    }
}

#[test]
fn a_straight_and_a_bent_guides_s_parameters_are_fdfds() {
    for bend in [false, true] {
        let s = guide_2d(bend);
        for (a, b) in s.fdtd.iter().zip(&s.fdfd) {
            assert!((a - b).norm() < 1e-4, "{bend}: {s:?}");
        }
    }
}
