use super::*;

fn re(n: f64) -> c64 {
    c64::new(n, 0.0)
}

fn lam() -> Wavelength {
    Wavelength::um(1.55).unwrap()
}

#[test]
fn a_wide_windows_guided_mode_is_the_exact_slabs() {
    let slice = Slice::slab(re(3.473), Length::nm(220.0), re(1.444), Length::um(6.0)).unwrap();
    for pol in [Polarization::Te, Polarization::Tm] {
        let modes = Eme::new(pol, lam(), 10).unwrap().modes(&slice).unwrap();
        let exact = crate::mode::slab::Slab::new(1.444, 3.473, 1.444, Length::nm(220.0))
            .unwrap()
            .modes(pol, lam())[0]
            .effective_index();
        let got = modes[0].effective_index();
        assert!(
            (got.re - exact).abs() < 1e-12 && got.im == 0.0,
            "{pol:?}: {got} vs {exact}"
        );
        // the rest below the cladding's index: the window's radiation modes
        assert!(modes[1..].iter().all(|m| m.effective_index().re < 1.444));
    }
}

#[test]
fn a_slices_modes_are_orthonormal_with_and_without_a_pml() {
    let slice = Slice::slab(re(3.473), Length::nm(300.0), re(1.444), Length::um(3.0)).unwrap();
    for pol in [Polarization::Te, Polarization::Tm] {
        let closed = Eme::new(pol, lam(), 30).unwrap();
        let open = closed.with_pml(Length::um(1.0), 0.1).unwrap();
        for eme in [closed, open] {
            let modes = eme.modes(&slice).unwrap();
            let o = scattering::overlaps(&modes, &modes);
            for i in 0..modes.len() {
                for j in 0..modes.len() {
                    let e = if i == j { 1.0 } else { 0.0 };
                    assert!(
                        (o[(i, j)] - e).norm() < 1e-10,
                        "{pol:?} {i} {j}: {}",
                        o[(i, j)]
                    );
                }
            }
        }
    }
}

#[test]
fn a_pmls_modes_lose_power_and_the_guided_mode_stays() {
    let slice = checks::half(0.22, 1.5);
    let eme = Eme::new(Polarization::Te, lam(), 40)
        .unwrap()
        .with_walls(Wall::Magnetic, Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)
        .unwrap();
    let modes = eme.modes(&slice).unwrap();
    let exact = crate::mode::slab::Slab::new(1.444, 3.473, 1.444, Length::nm(220.0))
        .unwrap()
        .modes(Polarization::Te, lam())[0]
        .effective_index();
    assert!((modes[0].effective_index() - exact).norm() < 1e-12);
    assert!(modes[1..].iter().all(|m| m.effective_index().im > 0.0));
}

#[test]
fn the_field_is_normalized_and_continuous() {
    let slice = Slice::slab(re(3.473), Length::nm(220.0), re(1.444), Length::um(3.0)).unwrap();
    for pol in [Polarization::Te, Polarization::Tm] {
        let m = &Eme::new(pol, lam(), 3).unwrap().modes(&slice).unwrap()[0];
        // ⟨E, H⟩ = 1 by the midpoint rule, the interfaces on its cells' edges
        let steps = 300_000;
        let w = m.width();
        let sum: c64 = (0..steps)
            .map(|i| {
                let (e, h) = m.field((i as f64 + 0.5) * w / steps as f64);
                // TE: −E_y H_x; TM: E_x H_y
                match pol {
                    Polarization::Te => -e * h,
                    Polarization::Tm => e * h,
                }
            })
            .sum::<c64>()
            * (w / steps as f64);
        assert!((sum - 1.0).norm() < 1e-6, "{pol:?}: {sum}");
        // U is continuous at the core's edges
        for x in [1.39, 1.61] {
            let (a, b) = (m.field(x - 1e-9), m.field(x + 1e-9));
            let (ua, ub) = match pol {
                Polarization::Te => (a.0, b.0),
                Polarization::Tm => (a.1, b.1),
            };
            assert!((ua - ub).norm() < 1e-6 * ua.norm(), "{pol:?} at {x}");
        }
    }
}

#[test]
fn bad_input_is_an_error() {
    assert!(Slice::new(&[]).is_err());
    assert!(Slice::new(&[(re(1.5), Length::ZERO)]).is_err());
    assert!(Slice::new(&[(c64::new(f64::NAN, 0.0), Length::um(1.0))]).is_err());
    assert!(Slice::slab(re(3.5), Length::um(2.0), re(1.4), Length::um(1.0)).is_err());
    assert!(Eme::new(Polarization::Te, lam(), 0).is_err());
    let eme = Eme::new(Polarization::Te, lam(), 2).unwrap();
    assert!(eme.with_pml(Length::um(-1.0), 0.1).is_err());
    assert!(eme.s_matrix(&[]).is_err());
    let a = Slice::slab(re(3.5), Length::um(0.2), re(1.4), Length::um(2.0)).unwrap();
    let b = Slice::slab(re(3.5), Length::um(0.2), re(1.4), Length::um(3.0)).unwrap();
    assert!(
        eme.s_matrix(&[Section::new(a, Length::ZERO), Section::new(b, Length::ZERO)])
            .is_err()
    );
    assert!(
        Section::sampled(Length::um(1.0), 0, |_| Slice::new(&[(
            re(1.0),
            Length::um(1.0)
        )]))
        .is_err()
    );
}

#[test]
fn a_straight_guide_only_advances_its_modes() {
    let slice = Slice::slab(re(3.473), Length::nm(220.0), re(1.444), Length::um(3.0)).unwrap();
    let eme = Eme::new(Polarization::Te, lam(), 5).unwrap();
    let s = eme
        .s_matrix(&[
            Section::new(slice.clone(), Length::um(1.0)),
            Section::new(slice, Length::um(1.0)),
        ])
        .unwrap();
    let n = s.first[0];
    let expected = (c64::new(0.0, lam().wavenumber() * 2.0) * n).exp();
    assert!((s.s21[0][0] - expected).norm() < 1e-13);
    assert!(s.s11[0][0].norm() < 1e-15);
}

#[test]
fn the_closed_form_cases_hold() {
    for pol in [Polarization::Te, Polarization::Tm] {
        let ((r, t), (er, et)) = checks::one_mode_step(pol);
        assert!(
            ((er - r) / r).abs() < 1e-10 && ((et - t) / t).abs() < 1e-10,
            "{pol:?}: {r} {t} vs {er} {et}"
        );
        assert!(checks::fresnel(pol) < 1e-12, "{pol:?}");
        let (symmetry, power) = checks::lossless(pol);
        assert!(
            symmetry < 1e-12 && power < 1e-12,
            "{pol:?}: {symmetry:e} {power:e}"
        );
    }
}

#[test]
#[ignore = "Fresnel's formula on the bare step, by the expansion (fdfd/step-reflection-te's model): in release, prints"]
fn the_steps_fresnel_error_study() {
    for modes in [40, 80, 120, 160, 200] {
        let (r, fresnel) = checks::step_against_fresnel(modes);
        eprintln!(
            "{modes} modes: R {r:.6e}, Fresnel {fresnel:.6e}, its error {:.3e} ({:.3} %)",
            fresnel - r,
            100.0 * (fresnel - r) / r
        );
    }
}

/// The report's rows of the eigenmode-expansion cases, to paste into docs/validation.md.
#[test]
#[ignore = "runs the taper cases with FDFD: for the report"]
fn rows() {
    for case in checks::cases() {
        let (row, passed) = crate::validation::row(&case);
        print!("{row}");
        assert!(passed, "{row}");
    }
}

#[test]
fn the_transformer_and_bienstman_and_baetss_eq_3_agree_as_the_modes_grow() {
    // a closed window: R of the 220 to 300 nm step converges to the same from both, 1.16469e-3
    let a = Slice::slab(re(3.473), Length::nm(220.0), re(1.444), Length::um(3.0)).unwrap();
    let b = Slice::slab(re(3.473), Length::nm(300.0), re(1.444), Length::um(3.0)).unwrap();
    let eme = Eme::new(Polarization::Te, lam(), 160).unwrap();
    let (ma, mb) = (eme.modes(&a).unwrap(), eme.modes(&b).unwrap());
    let o12 = scattering::overlaps(&ma, &mb);
    let o21 = scattering::overlaps(&mb, &ma);
    let ours = scattering::interface(&o12).s11[(0, 0)].norm_sqr();
    let (r3, _) = scattering::interface_eq3(&o12, &o21);
    let theirs = r3[(0, 0)].norm_sqr();
    assert!(
        ((ours - theirs) / theirs).abs() < 1e-5,
        "{ours} vs {theirs}"
    );
    assert!((theirs - 1.16469e-3).abs() < 1e-8, "{theirs}");
}

/// A slab `t` µm thick in a closed window 3 µm wide.
fn slab3(t: f64) -> Slice {
    Slice::slab(re(3.473), Length::um(t), re(1.444), Length::um(3.0)).unwrap()
}

/// A slab widening from 220 to 600 nm over 1 µm as z⁸, in `slab3`'s window, from `steps` steps.
fn sharp_taper(steps: usize) -> [Piece<'static>; 3] {
    [
        Piece::Section(Section::new(slab3(0.22), Length::ZERO)),
        Piece::Taper(
            Taper::new(Length::um(1.0), steps, |z| {
                Ok(slab3(0.22 + 0.38 * z.to_um().powi(8)))
            })
            .unwrap(),
        ),
        Piece::Section(Section::new(slab3(0.60), Length::ZERO)),
    ]
}

#[test]
fn adaptive_steps_give_the_cascade_of_their_own_sections() {
    let eme = Eme::new(Polarization::Te, lam(), 16).unwrap();
    let a = eme.adaptive(&sharp_taper(2), 1e-3).unwrap();
    assert!(a.error <= 1e-3 && a.rounds.len() > 1, "{:?}", a.rounds);
    assert_eq!(a.rounds.last().unwrap().steps, a.edges[0].len() - 1);
    // unequal steps: the split ones are where the taper's response changes
    let lengths: Vec<f64> = a.edges[0].windows(2).map(|w| w[1] - w[0]).collect();
    assert!(lengths.iter().any(|&l| l != lengths[0]), "{lengths:?}");
    let mut sections = vec![Section::new(slab3(0.22), Length::ZERO)];
    for w in a.edges[0].windows(2) {
        let z = 0.5 * (w[0] + w[1]);
        sections.push(Section::new(
            slab3(0.22 + 0.38 * z.powi(8)),
            Length::um(w[1] - w[0]),
        ));
    }
    sections.push(Section::new(slab3(0.60), Length::ZERO));
    let s = eme.s_matrix(&sections).unwrap();
    for (x, y) in [
        (s.s11[0][0], a.scattering.s11[0][0]),
        (s.s21[0][0], a.scattering.s21[0][0]),
        (s.s22[1][0], a.scattering.s22[1][0]),
    ] {
        assert!((x - y).norm() < 1e-13, "{x} vs {y}");
    }
}

/// The numbers eme/taper-adaptive's source and the method page quote.
#[test]
#[ignore = "slow in a debug build: eme/taper-adaptive runs it in release"]
fn the_sharp_tapers_numbers_are_the_ones_quoted() {
    let (error, stated, steps, reference, uniform) = checks::sharp_taper();
    eprintln!("{error:e} {stated:e} {steps} {reference:e} {uniform:e}");
    assert_eq!(steps, 67);
    for (got, quoted) in [
        (error, 9.77e-5),
        (stated, 8.95e-5),
        (reference, 2.1e-6),
        (uniform, 4.58e-4),
    ] {
        assert!(((got - quoted) / quoted).abs() < 6e-3, "{got} vs {quoted}");
    }
}

#[test]
fn adaptive_steps_refuse_a_taper_as_a_port_and_an_unreachable_tolerance() {
    let eme = Eme::new(Polarization::Te, lam(), 8).unwrap();
    let [a, b, c] = sharp_taper(2);
    let err = eme.adaptive(&[b, a], 1e-3).unwrap_err().to_string();
    assert!(err.contains("the ports"), "{err}");
    let err = eme.adaptive(&[c], 0.0).unwrap_err().to_string();
    assert!(err.contains("above 0"), "{err}");
    // far below what the budget of steps reaches
    let err = eme
        .adaptive(&sharp_taper(2), 1e-15)
        .unwrap_err()
        .to_string();
    assert!(err.contains("did not reach the tolerance"), "{err}");
}
