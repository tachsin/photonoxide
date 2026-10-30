use super::*;

#[test]
fn the_gaussian_stand_in_has_the_sincs_half_maximum() {
    let g = gaussian_gamma();
    let x = HALF_MAXIMUM;
    assert!(((-2.0 * g * x * x).exp() - 0.5).abs() < 1e-15);
}

#[test]
fn a_gaussian_without_correlation_is_pure() {
    assert_eq!(gaussian_purity(2.0, 3.0, 0.0), 1.0);
    assert!((gaussian_purity(1.0, 1.0, 0.6) - 0.8).abs() < 1e-15);
}

#[test]
fn the_pumps_intensity_is_half_at_its_half_width() {
    let p = Pump {
        center: 1.0,
        bandwidth: 0.01,
    };
    assert!((p.amplitude(1.005).powi(2) - 0.5).abs() < 1e-12);
}

#[test]
fn a_separable_sampled_amplitude_has_one_mode() {
    let (s, i) = (vec![0.0, 1.0, 2.0], vec![0.0, 1.0]);
    let amplitude = s
        .iter()
        .flat_map(|x| i.iter().map(move |y| c64::new((1.0 + x) * (2.0 + y), 0.0)))
        .collect();
    let jsa = JointSpectrum {
        signal: s,
        idler: i,
        amplitude,
    };
    let schmidt = jsa.schmidt().unwrap();
    assert!((schmidt.purity() - 1.0).abs() < 1e-14);
    assert!((schmidt.number() - 1.0).abs() < 1e-14);
    let zero = JointSpectrum {
        signal: vec![0.0, 1.0],
        idler: vec![0.0, 1.0],
        amplitude: vec![c64::new(0.0, 0.0); 4],
    };
    assert!(zero.schmidt().is_err());
}

#[test]
fn the_sampled_gaussian_meets_its_closed_form() {
    let (ours, closed) = crate::nonlinear::checks::pairs_gaussian(201);
    assert!(closed > 0.1 && closed < 0.9, "{closed}");
    assert!((ours / closed - 1.0).abs() < 1e-10, "{ours} {closed}");
}

#[test]
#[ignore]
fn explore_pairs_grid() {
    for n in [101, 201, 301, 401] {
        println!(
            "{n}: {:e} {:?}",
            crate::nonlinear::checks::pairs_errors(n),
            crate::nonlinear::checks::pairs_gaussian(n)
        );
    }
}
