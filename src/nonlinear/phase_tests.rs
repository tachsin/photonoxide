use num_complex::Complex64;

use super::*;

fn um(v: f64) -> Wavelength {
    Wavelength::um(v).unwrap()
}

/// A dispersionless index n, 0.2 to 10 µm.
fn constant(n: f64) -> Index {
    Index::material(
        Material::new(
            "constant",
            Model::Constant(Complex64::new(n, 0.0)),
            um(0.2),
            um(10.0),
            Provenance {
                reference: String::new(),
                doi: String::new(),
                data: String::new(),
                temperature: None,
                notes: String::new(),
            },
        )
        .unwrap(),
    )
}

/// n(T) = n₀ + s (T − 300 K), at any wavelength from 0.2 to 10 µm, from 250 to 350 K.
fn linear_in_t(n0: f64, s: f64) -> Index {
    let w = vec![0.2, 10.0];
    Index::effective_over_temperature(vec![
        (250.0, w.clone(), vec![n0 - 50.0 * s; 2]),
        (350.0, w, vec![n0 + 50.0 * s; 2]),
    ])
    .unwrap()
}

fn shg(n1: f64, n2: f64) -> Process {
    Process::second_harmonic(um(1.0), constant(n1), constant(n1), constant(n2))
}

#[test]
fn the_half_maximum_is_a_root_of_sinc2_minus_a_half() {
    assert!((sinc2(HALF_MAXIMUM) - 0.5).abs() < 1e-15);
    // Newton on sin x / x − 1/√2 from Fejer's 0.4429π lands on it
    let mut x = 0.4429 * PI;
    for _ in 0..20 {
        let f = x.sin() / x - std::f64::consts::FRAC_1_SQRT_2;
        let d = (x * x.cos() - x.sin()) / (x * x);
        x -= f / d;
    }
    assert!((x - HALF_MAXIMUM).abs() < 1e-15);
}

#[test]
fn sinc2_is_continuous_across_its_series() {
    for x in [9.9e-5, 1e-4, 1.01e-4] {
        assert!((sinc2(x) - (x.sin() / x).powi(2)).abs() < 1e-15);
    }
    assert_eq!(sinc2(0.0), 1.0);
    assert!(sinc2(PI).abs() < 1e-30);
}

#[test]
fn the_undepleted_power_grows_as_the_length_squared_at_phase_matching() {
    assert_eq!(undepleted(0.0, 1000.0), 1e6);
    assert!((undepleted(0.0, 2000.0) / undepleted(0.0, 1000.0) - 4.0).abs() < 1e-15);
    // off phase matching it oscillates: zero after one full slip
    assert!(undepleted(TAU / 1000.0, 1000.0) < 1e-20);
}

#[test]
fn the_period_order_and_coherence_length_follow_the_mismatch() {
    let dk = 0.5;
    assert!((coherence_length(dk) - 2.0 * PI).abs() < 1e-15);
    assert!((coherence_length(-dk) - 2.0 * PI).abs() < 1e-15);
    assert!((period(dk, 1).unwrap() - 4.0 * PI).abs() < 1e-14);
    // the period is 2 m l_c
    assert!((period(dk, 3).unwrap() - 6.0 * coherence_length(dk)).abs() < 1e-13);
    assert!(period(dk, 0).is_err() && period(0.0, 1).is_err());
    let (m, residual) = nearest_order(dk, 3.0 * TAU / dk * 1.01);
    assert_eq!(m, 3);
    assert!(residual > 0.0 && residual < 0.01);
}

#[test]
fn the_duty_cycle_sets_the_harmonics_amplitude() {
    // Eq. (23): 2/(mπ) at 50 % for odd m
    assert!((fourier_coefficient(1, 0.5) - 2.0 / PI).abs() < 1e-15);
    // the third harmonic's sign is opposite; its magnitude is what matters
    assert!((fourier_coefficient(3, 0.5) + 2.0 / (3.0 * PI)).abs() < 1e-15);
    // even orders vanish at 50 % and peak at 25 % (Fejer et al., after Eq. (23))
    assert!(fourier_coefficient(2, 0.5).abs() < 1e-16);
    assert!((fourier_coefficient(2, 0.25) - 1.0 / PI).abs() < 1e-15);
    assert!((effective_d(25.0, 1, 0.39) - 25.0 * 2.0 / PI * (0.39 * PI).sin()).abs() < 1e-13);
    let g = Grating::new(4.1, 1, 0.39).unwrap();
    assert!((g.fourier_coefficient() - fourier_coefficient(1, 0.39)).abs() < 1e-16);
    assert!(Grating::new(4.1, 1, 1.0).is_err() && Grating::new(-1.0, 1, 0.5).is_err());
}

#[test]
fn shg_mismatch_is_the_harmonics_wave_number_less_twice_the_fundamentals() {
    let p = shg(2.0, 2.1);
    let dk = p.mismatch(&State::default()).unwrap();
    // k₂ − 2k₁ = 4π(n₂ − n₁)/λ (Eq. (2))
    assert!((dk - 4.0 * PI * 0.1).abs() < 1e-13);
    assert!((p.coherence_length(&State::default()).unwrap() - 2.5).abs() < 1e-12);
    assert_eq!(p.signs(), &[-1.0, -1.0, 1.0]);
    assert!((p.waves()[2].wavelength.to_um() - 0.5).abs() < 1e-15);
}

#[test]
fn sum_and_difference_frequency_conserve_energy() {
    let sfg = Process::sum_frequency(
        Wave::new(um(1.55), constant(2.0)),
        Wave::new(um(1.0), constant(2.0)),
        constant(2.0),
    )
    .unwrap();
    let nu = |p: &Process, j: usize| 1.0 / p.waves()[j].wavelength.to_um();
    assert!((nu(&sfg, 2) - nu(&sfg, 0) - nu(&sfg, 1)).abs() < 1e-15);
    // dispersionless: Δk = 2π n (ν₃ − ν₁ − ν₂) = 0
    assert!(sfg.mismatch(&State::default()).unwrap().abs() < 1e-14);
    let dfg = Process::difference_frequency(
        Wave::new(um(1.064), constant(2.2)),
        Wave::new(um(1.55), constant(2.1)),
        constant(2.0),
    )
    .unwrap();
    assert!((nu(&dfg, 2) - nu(&dfg, 0) - nu(&dfg, 1)).abs() < 1e-15);
    let expected = TAU * (2.2 * nu(&dfg, 2) - 2.1 * nu(&dfg, 0) - 2.0 * nu(&dfg, 1));
    assert!((dfg.mismatch(&State::default()).unwrap() - expected).abs() < 1e-14);
    assert!(
        Process::difference_frequency(
            Wave::new(um(1.55), constant(2.0)),
            Wave::new(um(1.064), constant(2.0)),
            constant(2.0)
        )
        .is_err()
    );
}

#[test]
fn four_wave_mixing_puts_the_highest_frequency_on_the_high_side() {
    let p = |lam: f64| Wave::new(um(lam), constant(2.0));
    // degenerate pump at 1.55 µm, signal at 1.5: the signal is highest, Δk = k_s + k_i − 2k_p
    let fwm = Process::four_wave(p(1.55), p(1.55), p(1.5), constant(2.0)).unwrap();
    assert_eq!(fwm.signs(), &[-1.0, -1.0, 1.0, 1.0]);
    let nu: Vec<f64> = fwm
        .waves()
        .iter()
        .map(|w| 1.0 / w.wavelength.to_um())
        .collect();
    assert!((nu[0] + nu[1] - nu[2] - nu[3]).abs() < 1e-15);
    assert!(fwm.mismatch(&State::default()).unwrap().abs() < 1e-13);
    // Bragg scattering: pumps at 1.0 and 1.6, signal at 1.3: a pump is highest
    let bs = Process::four_wave(p(1.0), p(1.6), p(1.3), constant(2.0)).unwrap();
    assert_eq!(bs.signs(), &[1.0, 1.0, -1.0, -1.0]);
    assert!(Process::four_wave(p(3.0), p(3.0), p(1.0), constant(2.0)).is_err());
}

#[test]
fn a_tuning_must_conserve_energy_and_quote_a_tuned_wave() {
    assert!(shg(2.0, 2.1).with_tuning(vec![1.0, 1.0, 1.0], 0).is_err());
    assert!(shg(2.0, 2.1).with_tuning(vec![1.0, 1.0], 0).is_err());
    assert!(shg(2.0, 2.1).with_tuning(vec![0.0, 1.0, 1.0], 0).is_err());
    let p = shg(2.0, 2.1).with_tuning(vec![0.0, 1.0, 1.0], 1).unwrap();
    assert_eq!(p.quoted(), 1);
}

#[test]
fn a_designed_grating_cancels_the_mismatch_of_either_sign() {
    for (n1, n2) in [(2.0, 2.1), (2.1, 2.0)] {
        let d = Device::quasi_phase_matched(shg(n1, n2), 1000.0, State::default(), 1, 0.5).unwrap();
        let g = d.grating().unwrap();
        assert!((g.period() - 5.0).abs() < 1e-12, "Λ = 2 l_c = 5 µm");
        assert_eq!(g.wavevector().signum(), (n2 - n1).signum());
        assert!(d.mismatch(&State::default()).unwrap().abs() < 1e-14);
        assert!((d.response(&State::default()).unwrap() - 1.0).abs() < 1e-15);
    }
    assert!(Device::new(shg(2.0, 2.1), 0.0, State::default()).is_err());
}

#[test]
fn an_extraordinary_wave_goes_from_n_o_along_the_axis_to_n_e_across_it() {
    let e = Index::extraordinary(constant(2.3), constant(2.2));
    let at = |theta: f64| {
        e.at(
            1.0,
            &State {
                angle: theta,
                ..State::default()
            },
        )
        .unwrap()
    };
    assert!((at(0.0) - 2.3).abs() < 1e-15);
    assert!((at(FRAC_PI_2) - 2.2).abs() < 1e-15);
    // Boyd's Eq. (2.3.8) at 45°
    let expected = 1.0 / (0.5_f64 / (2.3 * 2.3) + 0.5 / (2.2 * 2.2)).sqrt();
    assert!((at(PI / 4.0) - expected).abs() < 1e-15);
}

#[test]
fn types_follow_boyds_table() {
    use Polarization::{Extraordinary as E, Ordinary as O};
    assert_eq!(Type::I.polarizations(true), [O, O, E]);
    assert_eq!(Type::I.polarizations(false), [E, E, O]);
    assert_eq!(Type::II.polarizations(true), [O, E, E]);
    assert_eq!(Type::II.polarizations(false), [O, E, O]);
    assert_eq!(Type::Zero(E).polarizations(true), [E, E, E]);
}

#[test]
fn type_i_angle_tuning_meets_the_closed_form() {
    // a dispersive negative crystal: n_o 2.0 and 2.1, n̄_e 1.9 and 1.95 at ω and 2ω
    let o = Index::effective(vec![0.4, 0.6, 1.2], vec![2.1, 2.05, 2.0]).unwrap();
    let e = Index::effective(vec![0.4, 0.6, 1.2], vec![1.95, 1.925, 1.9]).unwrap();
    let crystal = Uniaxial::new(o.clone(), e.clone());
    let state = State::default();
    assert!(crystal.negative(1.0, &state).unwrap());
    let p = crystal.second_harmonic(um(1.0), Type::I, &state).unwrap();
    let theta = phase_matching_angle(&p, &state).unwrap();
    // n_e(θ, 2ω) = n_o(ω): sin²θ = (n_o1⁻² − n_o2⁻²)/(n_e2⁻² − n_o2⁻²) (Boyd, Sec. 2.3)
    let (no1, no2, ne2) = (
        o.at(1.0, &state).unwrap(),
        o.at(0.5, &state).unwrap(),
        e.at(0.5, &state).unwrap(),
    );
    let s2: f64 = (no1.powi(-2) - no2.powi(-2)) / (ne2.powi(-2) - no2.powi(-2));
    assert!((theta - s2.sqrt().asin()).abs() < 1e-12, "{theta}");
    // critical: a finite acceptance in the tilt toward the axis, none about it
    let d = Device::new(
        p,
        1000.0,
        State {
            angle: theta,
            ..state
        },
    )
    .unwrap();
    let toward = d
        .acceptance(Variable::Tilt(Plane::TowardOpticAxis))
        .unwrap();
    assert!(toward.linear.is_finite() && (toward.numerical / toward.linear - 1.0).abs() < 0.01);
    let about = d.acceptance(Variable::Tilt(Plane::AboutOpticAxis)).unwrap();
    assert!(about.numerical.is_infinite(), "{about:?}");
}

#[test]
fn noncritical_temperature_tuning_finds_the_crossing() {
    // n_o rises by 1e-5/K, n_e by 5e-5/K: type I matches where their 0.001 gap closes
    let crystal = Uniaxial::new(linear_in_t(2.0, 1e-5), linear_in_t(1.999, 5e-5));
    let state = State {
        temperature: 300.0,
        ..State::default()
    };
    // dispersionless n_o at ω, n_e at 2ω: Δk ∝ n_e(T) − n_o(T) = −0.001 + 4e-5 (T − 300)
    let p = crystal.second_harmonic(um(1.0), Type::I, &state).unwrap();
    let t = phase_matching_temperature(&p, &state, 250.0, 350.0).unwrap();
    assert!((t - 325.0).abs() < 1e-9, "{t}");
    assert!(phase_matching_temperature(&p, &state, 250.0, 320.0).is_err());
}

#[test]
fn temperature_acceptance_of_a_linear_drift_is_its_linear_estimate() {
    // n₂ − n₁ = 0.1 + 4e-5 (T − 300): Δk is linear in T, so the numerical width is the linear one
    let p = Process::second_harmonic(
        um(1.0),
        linear_in_t(2.0, 1e-5),
        linear_in_t(2.0, 1e-5),
        linear_in_t(2.1, 5e-5),
    );
    let state = State {
        temperature: 300.0,
        ..State::default()
    };
    let d = Device::quasi_phase_matched(p, 1000.0, state, 1, 0.5).unwrap();
    let a = d.acceptance(Variable::Temperature).unwrap();
    // Eq. (44) without expansion: δT = 0.4429λ/(L |∂Δn/∂T|)
    let expected = 4.0 * HALF_MAXIMUM / (1000.0 * 4.0 * PI * 4e-5);
    assert!((a.linear - expected).abs() < 1e-9 * expected, "{a:?}");
    assert!((a.numerical - expected).abs() < 1e-9 * expected, "{a:?}");
    assert!((a.upper - 300.0 - expected / 2.0).abs() < 1e-8);
    // none of a dispersionless crystal's indices depends on T
    let flat =
        Device::quasi_phase_matched(shg(2.0, 2.1), 1000.0, State::default(), 1, 0.5).unwrap();
    assert!(flat.acceptance(Variable::Temperature).is_err());
}

#[test]
fn thermal_expansion_enters_as_eq_44() {
    // with an expansion α the slope is ∂Δk'/∂T + αΔk' (Eq. (43))
    let p = Process::second_harmonic(
        um(1.0),
        linear_in_t(2.0, 1e-5),
        linear_in_t(2.0, 1e-5),
        linear_in_t(2.1, 5e-5),
    );
    let state = State {
        temperature: 300.0,
        ..State::default()
    };
    let alpha = 1.5e-5;
    let d = Device::new(p.clone(), 1000.0, state)
        .unwrap()
        .with_expansion(Expansion {
            reference: 300.0,
            linear: alpha,
            quadratic: 0.0,
        });
    let d = d.with_grating(Grating::for_mismatch(p.mismatch(&state).unwrap(), 1, 0.5).unwrap());
    let a = d.acceptance(Variable::Temperature).unwrap();
    let expected = 4.0 * HALF_MAXIMUM / (1000.0 * 4.0 * PI * (4e-5 + alpha * 0.1));
    assert!(
        (a.linear - expected).abs() < 1e-8 * expected,
        "{a:?} {expected}"
    );
}

#[test]
fn wavelength_acceptance_of_shg_is_eq_30() {
    // Cauchy's n(λ) = 2 + 0.1/λ² (µm): δλ = 0.4429λ/(L |(n₂ − n₁)/λ + n₁′ − n₂′/2|) (Eq. (30))
    let index = Index::material(
        Material::new(
            "cauchy",
            Model::Cauchy(vec![2.0, 0.1]),
            um(0.2),
            um(10.0),
            Provenance {
                reference: String::new(),
                doi: String::new(),
                data: String::new(),
                temperature: None,
                notes: String::new(),
            },
        )
        .unwrap(),
    );
    let p = Process::second_harmonic(um(1.2), index.clone(), index.clone(), index);
    let d = Device::quasi_phase_matched(p, 1000.0, State::default(), 1, 0.5).unwrap();
    let a = d.acceptance(Variable::Wavelength).unwrap();
    let n = |l: f64| 2.0 + 0.1 / (l * l);
    let dn = |l: f64| -0.2 / (l * l * l);
    let bracket = (n(0.6) - n(1.2)) / 1.2 + dn(1.2) - dn(0.6) / 2.0;
    let expected = 4.0 * HALF_MAXIMUM * 1.2 / (4.0 * PI * 1000.0 * bracket.abs());
    assert!(
        (a.linear - expected).abs() < 1e-6 * expected,
        "{a:?} {expected}"
    );
    assert!((a.numerical / a.linear - 1.0).abs() < 1e-3);
}

#[test]
fn noncritical_tilt_of_an_isotropic_crystal_is_eq_41() {
    let p = shg(2.0, 2.1);
    let d = Device::quasi_phase_matched(p, 1000.0, State::default(), 1, 0.5).unwrap();
    let a = d.acceptance(Variable::Tilt(Plane::AboutOpticAxis)).unwrap();
    // δν = 2 (1.772 (n₂/n₁)(l_c/L))^½, l_c = 2.5 µm
    let expected = 2.0 * (4.0 * HALF_MAXIMUM / PI * 2.1 / 2.0 * 2.5 / 1000.0).sqrt();
    assert!(
        (a.noncritical - expected).abs() < 1e-6 * expected,
        "{a:?} {expected}"
    );
    assert!(
        a.linear > 1e3 * a.noncritical,
        "no first-order dependence at ν = 0"
    );
    assert!((a.numerical / a.noncritical - 1.0).abs() < 0.01);
    // and Eq. (A1.9) without birefringence is Eq. (41)
    assert!((first_order_tilt(1.0, 2.0, 2.1, 0.0, 0.0, 1000.0) - expected).abs() < 1e-12);
}

#[test]
fn the_first_order_tilt_follows_its_curvature() {
    // a tilt-dependent harmonic: B₂ > 0 widens the curvature and narrows the acceptance
    let narrow = first_order_tilt(1.0, 2.0, 2.1, 0.0, 0.05, 1000.0);
    let wide = first_order_tilt(1.0, 2.0, 2.1, 0.0, 0.0, 1000.0);
    assert!(narrow < wide);
}

#[test]
fn an_opo_idler_is_found_by_tuning() {
    // dispersionless, with a grating: phase matching where 2π n (ν_p − ν_s − ν_i) = K never
    // happens, so the search fails; with dispersion it finds the idler
    let flat = Process::difference_frequency(
        Wave::new(um(1.0), constant(2.0)),
        Wave::new(um(1.5), constant(2.0)),
        constant(2.0),
    )
    .unwrap();
    let d = Device::new(flat, 1.0, State::default())
        .unwrap()
        .with_grating(Grating::new(30.0, 1, 0.5).unwrap());
    assert!(d.phase_matching_wavelength(2.0 + 1e-9, 5.0).is_err());
    let w: Vec<f64> = (0..=50).map(|i| 0.5 + 0.1 * f64::from(i)).collect();
    let n: Vec<f64> = w.iter().map(|l| 2.1 + 0.02 / (l * l)).collect();
    let index = Index::effective(w, n).unwrap();
    let p = Process::difference_frequency(
        Wave::new(um(1.0), index.clone()),
        Wave::new(um(1.5), index.clone()),
        index,
    )
    .unwrap()
    .with_tuning(vec![-1.0, 1.0, 0.0], 1)
    .unwrap();
    let mismatch = p.mismatch(&State::default()).unwrap();
    // the grating that phase-matches the signal at 1.5 µm finds its idler, 3 µm, again
    let d = Device::new(p, 1.0, State::default())
        .unwrap()
        .with_grating(Grating::for_mismatch(mismatch, 1, 0.5).unwrap());
    let idler = d.phase_matching_wavelength(2.0 + 1e-9, 5.0).unwrap();
    assert!((idler - 3.0).abs() < 1e-9, "{idler}");
}

#[test]
fn indices_from_the_catalogue_and_from_data() {
    assert!(Index::catalogue("linbo3", Some("jundt-1997"), Axis::Ordinary).is_err());
    assert!(Index::catalogue("nothing", None, Axis::Ordinary).is_err());
    let jundt = Index::catalogue("linbo3", Some("jundt-1997"), Axis::Extraordinary).unwrap();
    assert!(jundt.depends_on_temperature());
    let zelmon = Index::catalogue("linbo3", None, Axis::Ordinary).unwrap();
    assert!(!zelmon.depends_on_temperature());
    let lnb = Uniaxial::catalogue("linbo3", None, Some("jundt-1997")).unwrap();
    assert!(lnb.negative(1.55, &State::default()).unwrap());
    // data at two temperatures, interpolated between and refused outside
    let t = linear_in_t(2.0, 1e-5);
    let at = |k: f64| {
        t.at(
            1.0,
            &State {
                temperature: k,
                ..State::default()
            },
        )
    };
    assert!((at(325.0).unwrap() - 2.00025).abs() < 1e-14);
    assert!(at(400.0).is_err());
    assert!(Index::effective_over_temperature(vec![]).is_err());
}
