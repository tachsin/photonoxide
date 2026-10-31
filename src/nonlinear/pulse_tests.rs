use super::*;

/// A guide of β₂ (ps²/m) and γ = 1 /(W·m) at 1.55 µm, without loss.
fn guide(beta2: f64) -> Guide {
    Guide::new(Dispersion::new(1.55, &[beta2]).unwrap(), 1.0, 0.0)
}

#[test]
fn the_dispersion_operator_is_the_taylor_series() {
    let d = Dispersion::new(1.55, &[-0.05, 0.002, 1e-5]).unwrap();
    let w: f64 = 3.0;
    let phase = -0.05 * w * w / 2.0 + 0.002 * w.powi(3) / 6.0 + 1e-5 * w.powi(4) / 24.0;
    assert!((d.phase(w) - phase).abs() < 1e-15);
    // only the even orders in the four-wave mismatch (Agrawal Eq. (10.4.12))
    let dk = -0.05 * w * w + 2.0 * 1e-5 * w.powi(4) / 24.0;
    assert!((d.four_wave_mismatch(w) - dk).abs() < 1e-14);
    assert_eq!(d.beta(5), 0.0);
    assert!(Dispersion::new(-1.0, &[]).is_err());
}

#[test]
fn the_fit_recovers_a_polynomial_beta() {
    // β(ω) = β₀ + β₁x + β₂x²/2 + β₃x³/6 about 1.55 µm, so n(λ) = cβ(ω)/ω
    let w0 = angular_frequency(1.55);
    let (b0, b1, b2, b3) = (6.0e6, 7.0e3, -0.08, 0.003);
    let n = |l: f64| -> crate::Result<f64> {
        let w = angular_frequency(l);
        let x = w - w0;
        Ok(C_M_PER_PS * (b0 + b1 * x + b2 * x * x / 2.0 + b3 * x.powi(3) / 6.0) / w)
    };
    let d = Dispersion::of_index(n, 1.55, (1.3, 1.8), 4).unwrap();
    assert!((d.beta(1) / b1 - 1.0).abs() < 1e-12, "{}", d.beta(1));
    assert!((d.beta(2) / b2 - 1.0).abs() < 1e-7, "{}", d.beta(2));
    assert!((d.beta(3) / b3 - 1.0).abs() < 1e-5, "{}", d.beta(3));
    assert!(d.beta(4).abs() < 1e-7, "{}", d.beta(4));
    assert!(d.error(2) < 1e-7 * b2.abs());
    assert!((d.group_index() - b1 * C_M_PER_PS).abs() < 1e-9);
    assert!(Dispersion::of_index(n, 1.9, (1.3, 1.8), 4).is_err());
}

#[test]
fn the_soliton_relations_agree() {
    let (b2, t0, g) = (-0.05, 0.1, 1.5);
    let p0 = soliton_power(g, t0, b2);
    assert!((soliton_order(g, p0, t0, b2) - 1.0).abs() < 1e-15);
    assert!((dispersion_length(t0, b2) / nonlinear_length(g, p0) - 1.0).abs() < 1e-15);
    assert!((soliton_period(t0, b2) - 0.5 * PI * 0.2).abs() < 1e-15);
    // u(0, τ) = 2 sech τ
    assert!((second_order_soliton(0.0, 0.3).re - 2.0 / 0.3f64.cosh()).abs() < 1e-14);
    assert!((effective_length(0.0, 2.0) - 2.0).abs() < 1e-15);
    assert!((effective_length(1e-12, 2.0) - 2.0).abs() < 1e-11);
    assert!((spm_rms_broadening(0.0) - 1.0).abs() < 1e-15);
}

#[test]
fn a_gaussian_has_its_closed_form_spectrum_and_the_carrier_its_sign() {
    let (p0, t0) = (2.0, 0.5);
    let pulse = Pulse::gaussian(p0, t0, 256, 20.0).unwrap();
    let spectrum = pulse.spectrum();
    for (w, s) in pulse.frequencies().iter().zip(&spectrum) {
        // ∫ √P₀ e^{−T²/2T₀²} e^{iΩT} dT = √P₀ √(2π) T₀ e^{−Ω²T₀²/2}
        let exact = p0.sqrt() * (2.0 * PI).sqrt() * t0 * (-0.5 * w * w * t0 * t0).exp();
        assert!((s - exact).norm() < 1e-12, "{w}: {s} {exact}");
    }
    assert!((pulse.rms_bandwidth() - 1.0 / (2f64.sqrt() * t0)).abs() < 1e-12);
    assert!((pulse.energy() - p0 * PI.sqrt() * t0).abs() < 1e-12);
    // a component e^{−iΩT} is at ω₀ + Ω: Ω > 0, bluer
    let shifted = Pulse::sampled(256, 20.0, |t| {
        c64::new((-0.5 * (t / t0).powi(2)).exp(), 0.0) * c64::new(0.0, -8.0 * t).exp()
    })
    .unwrap();
    let (k, _) = shifted
        .spectrum()
        .iter()
        .enumerate()
        .fold(
            (0, 0.0),
            |a, (i, v)| if v.norm() > a.1 { (i, v.norm()) } else { a },
        );
    assert!((shifted.frequencies()[k] - 8.0).abs() < 0.2);
    assert!(Pulse::sampled(100, 1.0, |_| c64::new(1.0, 0.0)).is_err());
}

#[test]
fn the_fundamental_soliton_keeps_its_shape() {
    let (b2, t0) = (-0.05, 0.1);
    let p0 = soliton_power(1.0, t0, b2);
    let start = Pulse::sech(p0, t0, 512, 40.0 * t0).unwrap();
    let z = 2.0 * soliton_period(t0, b2);
    let end = guide(b2).propagate(&start, z, 1000).unwrap();
    let ld = dispersion_length(t0, b2);
    let worst = end
        .times()
        .iter()
        .zip(&end.field)
        .map(|(&t, v)| {
            (v - c64::new(p0.sqrt() / (t / t0).cosh(), 0.0) * c64::new(0.0, 0.5 * z / ld).exp())
                .norm()
        })
        .fold(0.0, f64::max);
    assert!(worst < 2e-5 * p0.sqrt(), "{worst}");
    // and dispersion alone (γ = 0) spreads it
    let linear = Guide::new(Dispersion::new(1.55, &[b2]).unwrap(), 0.0, 0.0);
    assert!(linear.propagate(&start, z, 10).unwrap().peak_power() < 0.5 * p0);
}

#[test]
fn the_snapshots_end_where_the_propagation_does() {
    let start = Pulse::sech(5.0, 0.1, 256, 4.0).unwrap();
    let g = guide(-0.05);
    let shots = g.evolution(&start, 0.1, 40, 4).unwrap();
    assert_eq!(shots.len(), 4);
    let end = g.propagate(&start, 0.1, 40).unwrap();
    for (a, b) in shots[3].field.iter().zip(&end.field) {
        assert!((a - b).norm() < 1e-13);
    }
    // the first snapshot is a quarter of the way: a propagation of 10 steps over 0.025 m
    let quarter = g.propagate(&start, 0.025, 10).unwrap();
    for (a, b) in shots[0].field.iter().zip(&quarter.field) {
        assert!((a - b).norm() < 1e-13);
    }
    assert!(g.evolution(&start, 0.1, 10, 4).is_err());
}

#[test]
fn loss_alone_takes_the_energy_and_kerr_alone_keeps_it() {
    let start = Pulse::gaussian(3.0, 0.2, 256, 8.0).unwrap();
    let lossy = Guide::new(
        Dispersion::new(1.55, &[0.0]).unwrap(),
        0.0,
        loss_per_m(10.0),
    );
    let end = lossy.propagate(&start, 0.01, 7).unwrap();
    // 10 dB/cm over 1 cm
    assert!((end.energy() / start.energy() - 0.1).abs() < 1e-13);
    let kerr = guide(-0.02);
    let end = kerr.propagate(&start, 0.5, 50).unwrap();
    assert!((end.energy() / start.energy() - 1.0).abs() < 1e-13);
}

#[test]
fn ramans_response_is_normalized_and_its_moment_agrawals() {
    let r = Raman::SILICA;
    assert!((r.response(0.0) - 1.0).norm() < 1e-15);
    // T_R = f_R ∫ T h_R dT = f_R 2τ₁²τ₂/(τ₁² + τ₂²), 1.46 fs: "about 2 fs for silica fibers"
    // (Agrawal, p. 44, whose Eq. (2.3.45) prints f_R τ₁τ₂²/(τ₁² + τ₂²), 1.92 fs)
    assert!(
        (r.first_moment() * 1e3 - 1.46195).abs() < 1e-5,
        "{}",
        r.first_moment()
    );
    // the integral by the trapezoidal rule on 1 as steps to 2 ps
    let h = 1e-6;
    let integral: f64 = (1..2_000_000)
        .map(|k| {
            let t = k as f64 * h;
            t * (1.0 / (r.tau1 * r.tau1) + 1.0 / (r.tau2 * r.tau2))
                * r.tau1
                * (-t / r.tau2).exp()
                * (t / r.tau1).sin()
        })
        .sum::<f64>()
        * h;
    assert!((r.fraction * integral / r.first_moment() - 1.0).abs() < 1e-8);
    // the response's slope at Ω = 0 is i T_R/f_R: Im h̃ ≈ Ω ∫ T h_R dT
    let w = 1e-4;
    assert!((r.response(w).im / w - r.first_moment() / r.fraction).abs() < 1e-9);
}

#[test]
fn raman_shifts_a_soliton_red() {
    let (b2, t0) = (-0.05, 0.03);
    let p0 = soliton_power(1.0, t0, b2);
    let start = Pulse::sech(p0, t0, 1024, 60.0 * t0).unwrap();
    let g = guide(b2).with_raman(Raman::SILICA);
    let end = g.propagate(&start, 0.2, 400).unwrap();
    let mean = |p: &Pulse| {
        let s = p.spectrum();
        let w = p.frequencies();
        let total: f64 = s.iter().map(|v| v.norm_sqr()).sum();
        s.iter().zip(&w).map(|(v, x)| v.norm_sqr() * x).sum::<f64>() / total
    };
    // the soliton self-frequency shift is to lower frequencies (Agrawal Sec. 5.5.4)
    assert!(mean(&end) < -0.1, "{}", mean(&end));
    // without self-steepening the Raman term only moves the phase: |A| is kept at each T, so
    // the energy too, to the RK4 step's error
    assert!((end.energy() / start.energy() - 1.0).abs() < 1e-6);
}

#[test]
fn two_photon_absorption_alone_is_exact() {
    // β_T = 5e-12 m/W over 0.3 µm², 1 cm, no linear loss: one step is Lin's Eqs. (60)–(61)
    let tpa = TwoPhoton {
        coefficient: 5e-12,
        area: 0.3e-12,
        carriers: None,
    };
    let g = Guide::new(Dispersion::new(1.55, &[0.0]).unwrap(), 30.0, 0.0).with_two_photon(tpa);
    let start = Pulse::gaussian(20.0, 1.0, 256, 32.0).unwrap();
    let end = g.propagate(&start, 0.01, 1).unwrap();
    let j = 128;
    let p = two_photon_power(20.0, 5e-12, 0.3e-12, 0.0, 0.01);
    let phi = two_photon_phase(30.0, 20.0, 5e-12, 0.3e-12, 0.0, 0.01);
    assert!((end.field[j].norm_sqr() / p - 1.0).abs() < 1e-13);
    assert!(((end.field[j] / start.field[j]).arg() - phi).abs() < 1e-13);
    assert!(two_photon_phase(100.0, 20.0, 0.0, 0.3e-12, 0.0, 0.01) == 20.0);
}

#[test]
fn the_carriers_rise_through_the_pulse_and_decay_after_it() {
    let tpa = TwoPhoton {
        coefficient: 5e-12,
        area: 0.3e-12,
        carriers: Some(FreeCarriers::silicon(1.55, f64::INFINITY, 1.0)),
    };
    let g = Guide::new(Dispersion::new(1.55, &[0.0]).unwrap(), 0.0, 0.0).with_two_photon(tpa);
    let start = Pulse::gaussian(10.0, 1.0, 1024, 64.0).unwrap();
    let n = g.carriers(&start).unwrap();
    let total = gaussian_carriers(5e-12, 0.3e-12, 10.0, 1.0, 1.55);
    assert!(
        (n[1023] / total - 1.0).abs() < 1e-12,
        "{} {}",
        n[1023],
        total
    );
    assert!(n.windows(2).all(|w| w[1] >= w[0]));
    // with a 2 ps lifetime they fall back after the pulse
    let short = TwoPhoton {
        carriers: Some(FreeCarriers::silicon(1.55, 2.0, 1.0)),
        ..tpa
    };
    let n = Guide::new(Dispersion::new(1.55, &[0.0]).unwrap(), 0.0, 0.0)
        .with_two_photon(short)
        .carriers(&start)
        .unwrap();
    assert!(n[1023] < 1e-5 * total);
    let bad = TwoPhoton { area: 0.0, ..tpa };
    assert!(
        Guide::new(Dispersion::new(1.55, &[0.0]).unwrap(), 0.0, 0.0)
            .with_two_photon(bad)
            .carriers(&start)
            .is_err()
    );
}
