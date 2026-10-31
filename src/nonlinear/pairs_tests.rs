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
fn two_pump_photons_have_the_self_convolved_spectrum() {
    let p = Pump {
        center: 0.6,
        bandwidth: 0.002,
    };
    let two = p.two_photon();
    // α ⊗ α at ν, by the trapezoidal rule, against the two-photon Gaussian (its peak scaled)
    let conv = |nu: f64| {
        let h = 1e-5;
        (-2000..=2000)
            .map(|k| {
                let x = 0.6 + k as f64 * h;
                p.amplitude(x) * p.amplitude(nu - x) * h
            })
            .sum::<f64>()
    };
    let peak = conv(1.2);
    for d in [0.0005, 0.001, 0.002] {
        assert!(
            (conv(1.2 + d) / peak - two.amplitude(1.2 + d)).abs() < 1e-9,
            "{d}"
        );
    }
}

#[test]
fn spontaneous_four_wave_mixing_takes_one_pump() {
    let (ours, closed) = crate::nonlinear::kerr_checks::sfwm_purity(201);
    assert!(closed > 0.5 && closed < 0.99, "{closed}");
    assert!((ours / closed - 1.0).abs() < 1e-10, "{ours} {closed}");
    // two pumps of different wavelengths aren't a single pump's pairs
    let index = || crate::nonlinear::checks::linear_index(2.0, 0.0).unwrap();
    let wave = |l: f64| super::super::phase::Wave::new(Wavelength::um(l).unwrap(), index());
    let process =
        super::super::phase::Process::four_wave(wave(1.54), wave(1.56), wave(1.50), index())
            .unwrap();
    let device = Device::new(process, 1000.0, super::super::phase::State::default()).unwrap();
    let pump = Pump {
        center: 0.645,
        bandwidth: 1e-3,
    };
    assert!(joint_spectrum(&device, pump, Matching::Gaussian, 0.01, 11).is_err());
}

use crate::units::Wavelength;

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
