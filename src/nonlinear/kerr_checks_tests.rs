use super::*;

/// The numbers behind the rules and the write-up's tables:
/// `cargo test --release --lib kerr_checks::tests::tables -- --ignored --nocapture`.
#[test]
#[ignore = "prints the convergence tables; a few seconds in release"]
fn tables() {
    for k in 1..=4 {
        let (fit, error, exact) = silica_beta(k);
        println!(
            "silica beta{k}: fit {fit:.9e} exact {exact:.9e} rel {:.2e} estimate {:.2e}",
            fit / exact - 1.0,
            error / exact.abs()
        );
    }
    for steps in [500, 1000, 2000, 4000, 8000] {
        println!(
            "steps {steps}: fundamental {:.3e} second {:.3e} third {:.3e}",
            fundamental_soliton(steps),
            second_order(steps),
            third_order_period(steps)
        );
    }
    for steps in [100, 200, 400, 800] {
        let (a, b) = spm_phase(steps);
        println!("spm {steps}: {a} vs {b}, rel {:.2e}", a / b - 1.0);
    }
    for points in [512, 1024, 2048] {
        let (a, b) = spm_broadening(points);
        println!("broadening {points}: {a} vs {b}, rel {:.2e}", a / b - 1.0);
    }
    for steps in [500, 1000, 2000, 4000] {
        let (n, e) = photon_number(steps);
        println!("photons {steps}: drift {n:.2e} energy {e:.4}");
    }
    for steps in [100, 200, 400, 800, 1600] {
        let (a, b) = fwm_gain(-2.0, steps);
        let (c, d) = fwm_gain(1.0, steps);
        println!(
            "fwm {steps}: matched {a} vs {b} ({:.2e}), mismatched {c} vs {d} ({:.2e}), invariants {:.2e}",
            a / b - 1.0,
            c / d - 1.0,
            fwm_invariants(steps)
        );
    }
    for steps in [500, 1000, 2000, 4000] {
        let (a, b) = fwm_nlse(steps);
        println!("fwm nlse {steps}: {a} vs {b} ({:.2e})", a / b - 1.0);
    }
    for n in [101, 201, 301] {
        let (a, b) = sfwm_purity(n);
        println!("sfwm {n}: {a} vs {b} ({:.2e})", a / b - 1.0);
    }
    for steps in [1600, 3200, 6400, 12_800] {
        let (a, b) = lle_homogeneous(false, steps);
        let (c, d) = lle_homogeneous(true, steps);
        println!(
            "lle {steps}: lower {a} vs {b} ({:.2e}), upper {c} vs {d} ({:.2e})",
            a / b - 1.0,
            c / d - 1.0
        );
    }
    for steps in [800, 1600, 3200, 6400] {
        let (a, b) = lle_instability(steps);
        println!("instability {steps}: {a} vs {b} ({:.2e})", a / b - 1.0);
    }
    for step in [8e-4, 4e-4, 2e-4] {
        let (w, p) = coen_2012_soliton(step);
        println!("coen {step}: fwhm {w} fs, peak {p} W");
    }
    let ring = coen_2012_ring().unwrap();
    let r = ring.resonator(512).unwrap();
    let s = r.soliton(ring.d2()).unwrap();
    let p = c64::new(s.cos_phase, (1.0 - s.cos_phase.powi(2)).sqrt());
    println!(
        "coen approximation: fwhm {} fs, peak {} W, d2 {}, f {}",
        ring.time_of(s.width) * 2.0 * (2f64.sqrt()).acosh() * 1e3,
        (s.background + p * s.amplitude).norm_sqr() * ring.power_scale(),
        ring.d2(),
        ring.normalized_drive()
    );
}

/// A silicon nitride strip in silica from its modes: β₂, β₃ and β₄ at 1.55 µm with their
/// estimated errors, the effective area and γ with the catalogue's n₂, and D₂ = −(c/n_g)D₁²β₂
/// at Pfeiffer et al.'s FSR. The strip is Pfeiffer et al.'s Fig. 5 cross-section, 1.75 µm wide
/// and 0.85 µm high (Optica 3, 20 (2016), p. 23), straight, the catalogue's default indices:
/// `cargo test --release --lib kerr_checks::tests::silicon_nitride_strip -- --ignored --nocapture`.
#[test]
#[ignore = "the modes of a strip at many wavelengths on two grids: minutes in release"]
fn silicon_nitride_strip() {
    use crate::material::catalogue::kerr::kerr;
    use crate::mode::pockels::Along;
    use crate::nonlinear::guide::{Family, Grid, ModeIndex, Ridge, fundamental};
    use crate::nonlinear::guide_checks::isotropic;
    let ridge = Ridge {
        film: 0.85,
        etch: 0.85,
        width: 1.75,
        angle: 90.0,
        core: isotropic("si3n4"),
        substrate: isotropic("sio2"),
        cladding: isotropic("sio2"),
        along: Along::Y,
    };
    let n2 = kerr("si3n4")
        .into_iter()
        .find(|k| k.n2().is_some())
        .expect("a nitride n2");
    let lam = Wavelength::um(1.55).unwrap();
    for h in [0.02, 0.01] {
        let grid = Grid::new(h).halved();
        for family in [Family::Te, Family::Tm] {
            let mode = ModeIndex::of_ridge(&ridge, &grid, family, 4, (1.2, 2.0), 1e-9).unwrap();
            let d = Dispersion::of_mode(&mode, 1.55, 4).unwrap();
            let cs = ridge.cross_section(lam, &grid, family).unwrap();
            let m = fundamental(&cs, lam, family, 4).unwrap();
            let f = m.fields_one_sided(&cs).unwrap();
            let area = f.effective_area();
            let gamma = f.kerr_coefficient(lam, &n2).unwrap();
            let d1 = 2.0 * PI * 95.65e9;
            let ng = d.group_index();
            // D₂ = −(c/n_g) D₁² β₂, β₂ in s²/m
            let d2 = -(crate::units::SPEED_OF_LIGHT / ng) * d1 * d1 * d.beta(2) * 1e-24;
            println!(
                "h {h} {family:?}: {} points, n_eff {:.6}, n_g {ng:.5}; beta2 {:.5e} ± {:.1e} ps2/m, beta3 {:.4e} ± {:.1e} ps3/m, beta4 {:.3e} ± {:.1e} ps4/m; A_eff {area:.4} um2, gamma {gamma:.4} /W/m; D2/2pi {:.4} MHz; N=1 power for 100 fs {:.3} W",
                mode.points(),
                mode.n_eff(1.55),
                d.beta(2),
                d.error(2),
                d.beta(3),
                d.error(3),
                d.beta(4),
                d.error(4),
                d2 / (2.0 * PI) * 1e-6,
                crate::nonlinear::pulse::soliton_power(gamma, 0.1, d.beta(2)),
            );
        }
    }
}

/// The strip's β₂ three more ways on 20 nm cells: the mode index's own λ-derivative
/// (`ModeIndex::gvd`), and a central difference of β(ω) = ωn_eff/c from three separate solves.
#[test]
#[ignore = "mode solves of a strip: a minute in release"]
fn silicon_nitride_strip_by_differences() {
    use crate::mode::pockels::Along;
    use crate::nonlinear::guide::{Family, Grid, ModeIndex, Ridge, fundamental};
    use crate::nonlinear::guide_checks::isotropic;
    let ridge = Ridge {
        film: 0.85,
        etch: 0.85,
        width: 1.75,
        angle: 90.0,
        core: isotropic("si3n4"),
        substrate: isotropic("sio2"),
        cladding: isotropic("sio2"),
        along: Along::Y,
    };
    let grid = Grid::new(0.02).halved();
    let n = |w: f64| {
        let lam = Wavelength::um(crate::nonlinear::pulse::wavelength_of(w)).unwrap();
        let cs = ridge.cross_section(lam, &grid, Family::Te).unwrap();
        fundamental(&cs, lam, Family::Te, 4)
            .unwrap()
            .effective_index()
            .re
    };
    let w0 = angular_frequency(1.55);
    let h = 20.0;
    let beta = |w: f64| w * n(w) / C_M_PER_PS;
    let b2 = (beta(w0 + h) - 2.0 * beta(w0) + beta(w0 - h)) / (h * h);
    let mode = ModeIndex::of_ridge(&ridge, &grid, Family::Te, 4, (1.2, 2.0), 1e-9).unwrap();
    println!(
        "TE beta2: central difference {b2:.5e} ps2/m, ModeIndex::gvd {:.5e} ps2/m",
        mode.gvd(1.55) * 1e-3
    );
}

/// The approximate soliton against the settled one at f = 3, d₂ = 0.004 on 512 lines, followed
/// up in ζ₀ from 8 until it decays: the write-up's table.
#[test]
#[ignore = "prints the soliton's existence table"]
fn soliton_existence() {
    use crate::nonlinear::resonator::max_detuning;
    let (f, d2, n) = (3.0, 0.004, 512);
    let mut r = Resonator::quadratic(8.0, f, d2, n).unwrap();
    let mut psi = r.soliton_field(&r.soliton(d2).unwrap());
    println!("pi^2 f^2/8 = {}", max_detuning(f));
    let mut z = 8.0;
    while z < 13.0 {
        r = r.with_detuning(z);
        let Ok((end, _)) = r.settle(&psi, 2e-3, 1.0, 1e-9, 400.0) else {
            println!("{z}: doesn't settle");
            break;
        };
        psi = end;
        let peak = psi.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
        let lower = r.homogeneous()[0];
        if peak < 3.0 * lower.max(1.0) {
            println!("{z}: decayed to {peak}");
            break;
        }
        let fwhm = peak_width(&psi).unwrap();
        let approx = r.soliton(d2);
        let approx_peak = approx.map(|s| {
            let p = c64::new(s.cos_phase, (1.0 - s.cos_phase.powi(2)).sqrt());
            (s.background + p * s.amplitude).norm_sqr()
        });
        println!(
            "{z}: peak {peak:.4} (approx {approx_peak:?}), fwhm {fwhm:.6} (approx {:?})",
            approx.map(|s| s.width * 2.0 * (2f64.sqrt()).acosh())
        );
        z += 0.25;
    }
}

#[test]
fn the_split_steps_are_second_order() {
    let a = second_order(1000) / second_order(2000);
    assert!((a.log2() - 2.0).abs() < 0.1, "{a}");
    let (x, y) = lle_instability(800);
    let (u, v) = lle_instability(1600);
    let b = (x / y - 1.0).abs() / (u / v - 1.0).abs();
    assert!((b.log2() - 2.0).abs() < 0.1, "{b}");
}

#[test]
fn the_sellmeier_series_matches_differences() {
    // β₂ of silica at 1.55 µm by a central difference of β(ω) = ωn/c
    let m = silica();
    let n = |w: f64| {
        let l = crate::nonlinear::pulse::wavelength_of(w);
        m.refractive_index(Wavelength::um(l).unwrap()).unwrap().re
    };
    let w0 = angular_frequency(1.55);
    let h = 1.0;
    let beta = |w: f64| w * n(w) / C_M_PER_PS;
    let b2 = (beta(w0 + h) - 2.0 * beta(w0) + beta(w0 - h)) / (h * h);
    let (_, _, exact) = silica_beta(2);
    assert!((b2 / exact - 1.0).abs() < 1e-4, "{b2} {exact}");
}
