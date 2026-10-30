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
