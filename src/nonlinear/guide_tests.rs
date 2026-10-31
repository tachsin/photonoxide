use super::*;
use crate::nonlinear::guide_checks as checks;

#[test]
fn a_tensor_turns_into_the_guides_frame() {
    let mut v = [[0.0; 6]; 3];
    v[2][2] = 25.0; // d33
    v[2][0] = 4.6; // d31
    v[2][1] = 4.6; // d32
    v[0][4] = 4.6; // d15
    v[1][3] = 4.6; // d24
    v[1][1] = -2.0; // d22
    v[1][0] = 2.0; // d21 = -d22
    v[0][5] = 2.0; // d16 = -d22
    let d = SecondOrder::from_voigt(v).unwrap();
    assert_eq!(d.voigt(), v);
    assert_eq!(d.element(2, 2, 2), 25e-12);
    assert_eq!(d.element(0, 0, 2), 4.6e-12);
    assert_eq!(d.element(0, 2, 0), 4.6e-12);
    // x-cut, the guide along the crystal's y: c across, so d33 is d_xxx and d31 is d_xyy
    let x_cut = Orientation::cut(
        crate::material::CrystalAxis::X,
        Some(crate::material::CrystalAxis::Z),
    )
    .unwrap();
    let s = d.rotated(&x_cut).in_section(Along::Y);
    assert!((s.element(0, 0, 0) - 25e-12).abs() < 1e-24);
    assert!((s.element(0, 1, 1) - 4.6e-12).abs() < 1e-24);
    assert!((s.element(0, 2, 2) - 4.6e-12).abs() < 1e-24);
    // z-cut: c up, d33 is d_yyy
    let z_cut = Orientation::cut(crate::material::CrystalAxis::Z, None).unwrap();
    let s = d.rotated(&z_cut).in_section(Along::Y);
    assert!((s.element(1, 1, 1) - 25e-12).abs() < 1e-24);
    assert!((s.element(1, 0, 0) - 4.6e-12).abs() < 1e-24);
    // scaled, by a grating's coefficient say
    assert!((s.scaled(2.0).element(1, 1, 1) - 50e-12).abs() < 1e-24);
    assert!(SecondOrder::from_voigt([[f64::NAN; 6]; 3]).is_err());
}

#[test]
fn a_catalogue_tensor_is_completed_by_kleinman_and_another_source() {
    let e = crate::material::catalogue::entry("linbo3").unwrap();
    let find = |label: &str| e.tensors.iter().find(|t| t.label == label).unwrap().clone();
    let shoji = find("SHG, 1.064 µm");
    let d22 = find("d₂₂, SHG, 1.064 µm");
    // Shoji et al. leave d15 and d22 unknown
    assert!(SecondOrder::from_tensor(&shoji).is_err());
    assert!(SecondOrder::completed(&shoji, &[&d22], false).is_err());
    let d = SecondOrder::completed(&shoji, &[&d22], true).unwrap();
    let v = d.voigt();
    assert_eq!(v[2][2], 25.2);
    assert_eq!(v[2][0], 4.6);
    // d15 = d31 by Kleinman, d22 from Miller et al., d21 = -d22 by the point group
    assert_eq!(v[0][4], 4.6);
    assert_eq!(v[1][1], 1.9);
    assert_eq!(v[1][0], -1.9);
    // an electro-optic tensor isn't one
    let r = find("bulk, clamped, 633 nm");
    assert!(SecondOrder::from_tensor(&r).is_err());
}

#[test]
fn chebyshev_interpolants_and_their_derivatives_converge() {
    // e^x on 17 points: the interpolant, its first and second derivatives to near round-off
    let n = 16;
    let f: Vec<f64> = (0..=n)
        .map(|j| {
            (std::f64::consts::PI * f64::from(j) / f64::from(n))
                .cos()
                .exp()
        })
        .collect();
    let c = chebyshev_coefficients(&f);
    let d1 = chebyshev_derivative(&c);
    let d2 = chebyshev_derivative(&d1);
    for x in [-1.0, -0.3, 0.0, 0.71, 1.0] {
        let e: f64 = f64::exp(x);
        assert!((clenshaw(&c, x) - e).abs() < 1e-14, "{x}");
        assert!((clenshaw(&d1, x) - e).abs() < 1e-12, "{x}");
        assert!((clenshaw(&d2, x) - e).abs() < 1e-10, "{x}");
    }
    // a constant's derivative is zero
    assert_eq!(chebyshev_derivative(&[3.0]), vec![0.0]);
}

#[test]
fn a_ridge_is_checked_and_its_film_found() {
    let r = checks::zhao_ridge(None);
    let w = Wavelength::um(1.57).unwrap();
    let mut bad = r.clone();
    bad.etch = 0.4;
    assert!(bad.cross_section(w, &Grid::new(0.04), Family::Te).is_err());
    bad = r.clone();
    bad.angle = 0.0;
    assert!(bad.cross_section(w, &Grid::new(0.04), Family::Te).is_err());
    // the film: its slab beside the ridge, the ridge's slanted sides
    assert!(r.in_film(2.0, 0.1) && !r.in_film(2.0, 0.27));
    assert!(r.in_film(0.62, 0.29) && !r.in_film(0.64, 0.29) && r.in_film(0.63, 0.26));
    let half = r
        .cross_section(w, &Grid::new(0.04).halved(), Family::Tm)
        .unwrap();
    assert_eq!(half.boundaries().west, Boundary::MagneticWall);
    assert_eq!(half.x()[0], 0.0);
    // the film's share: its area
    let share = r.film_share(&half, |_, _| true);
    let (xs, ys) = (half.x(), half.y());
    let ny = ys.len() - 1;
    let area: f64 = share
        .iter()
        .enumerate()
        .map(|(n, s)| s * (xs[n / ny + 1] - xs[n / ny]) * (ys[n % ny + 1] - ys[n % ny]))
        .sum();
    let reach = xs[xs.len() - 1];
    let exact = reach * 0.25 + 0.5 * (0.62 + 0.62 + 0.05 / 75f64.to_radians().tan()) * 0.05;
    assert!((area / exact - 1.0).abs() < 2e-3, "{area} vs {exact}");
}

#[test]
fn half_a_guide_behind_its_wall_couples_as_the_whole() {
    // Zhao et al.'s ridge on 40 nm cells: the SHG coupling of its TE₀ modes on the whole window
    // and on its half behind an electric wall, the images added with their parities; the two
    // grids differ, so they agree to their discretisation
    let r = checks::zhao_ridge(None);
    let d = SecondOrder::from_voigt({
        let mut v = [[0.0; 6]; 3];
        v[2][2] = 25.0;
        v[2][0] = 4.4;
        v[2][1] = 4.4;
        v[0][4] = 4.4;
        v[1][3] = 4.4;
        v
    })
    .unwrap()
    .rotated(
        &Orientation::cut(
            crate::material::CrystalAxis::X,
            Some(crate::material::CrystalAxis::Z),
        )
        .unwrap(),
    )
    .in_section(Along::Y);
    let kappa = |grid: Grid| {
        let (cs, f) = checks::te0_mode(&r, &grid, 1.57);
        let (_, h) = checks::te0_mode(&r, &grid, 0.785);
        let ef = ModeField::new(&f, &cs, Wavelength::um(1.57).unwrap()).unwrap();
        let eh = ModeField::new(&h, &cs, Wavelength::um(0.785).unwrap()).unwrap();
        let share = r.film_share(&cs, |_, _| true);
        let shg = second_harmonic_coupling(&eh, &ef, &d, &share).unwrap();
        let sfg = sum_frequency_coupling(&eh, &ef, &ef, &d, &share).unwrap();
        assert!((sfg / shg - 2.0).abs() < 1e-12, "{sfg} vs {shg}");
        shg
    };
    let (whole, half) = (kappa(Grid::new(0.04)), kappa(Grid::new(0.04).halved()));
    assert!((half / whole - 1.0).abs() < 1e-2, "{half} vs {whole}");
    // at degeneracy the pairs' density is η P L² (the sum's κ is twice SHG's)
    let density = pair_rate_density(2.0 * whole, 1.0, 1.0, 1e-3, 5000.0, 0.0);
    let eta = whole * whole;
    assert!((density / (eta * 1e-3 * 5000.0 * 5000.0) - 1.0).abs() < 1e-12);
}

// ---------------------------------------------------------------------------------------------
// The write-up's numbers that aren't cases: Chang et al.'s strip, Zhao et al.'s efficiency,
// brightness and pair bandwidth (docs/methods/nonlinear.md, "In waveguides")
// ---------------------------------------------------------------------------------------------

/// Chang et al. 2018's GaAs strip (Laser Photonics Rev. 12, 1800149, Sec. 2 and Fig. 1(a)):
/// `thickness` µm of GaAs (the catalogue's model `model`) `width` µm wide in silica on every
/// side; the sidewalls, which the main text doesn't give, vertical; the 5 nm SiN bonding layer
/// (p. 2), whose index it doesn't give, left out.
fn chang_strip(thickness: f64, width: f64, model: &str) -> Ridge {
    use crate::material::catalogue::{Conditions, entry};
    let core = entry("gaas")
        .unwrap()
        .index
        .into_iter()
        .find(|m| m.id == model)
        .unwrap()
        .materials(Conditions::default())
        .unwrap()
        .remove(0);
    Ridge {
        film: thickness,
        etch: thickness,
        width,
        angle: 90.0,
        core,
        substrate: checks::isotropic("sio2"),
        cladding: checks::isotropic("sio2"),
        along: Along::Y,
    }
}

/// n_TE₀₀(λ) − n_TM₀₀(λ/2) of a strip on 10 nm cells (half the window).
fn chang_mismatch(thickness: f64, width: f64, lam: f64, model: &str) -> f64 {
    let r = chang_strip(thickness, width, model);
    let g = Grid::new(0.01).halved();
    let n = |l: f64, family: Family| {
        let w = Wavelength::um(l).unwrap();
        let cs = r.cross_section(w, &g, family).unwrap();
        fundamental(&cs, w, family, 6).unwrap().effective_index().re
    };
    n(lam, Family::Te) - n(lam / 2.0, Family::Tm)
}

#[test]
#[ignore = "prints Chang et al.'s strip: its mismatch, its phase-matched width and wavelength, and their slopes"]
fn chang_numbers() {
    for model in ["skauli-2003", "gehrsitz-2000-gaas"] {
        let f = |w: f64| chang_mismatch(0.15, w, 2.0, model);
        // the secant method for the width from 1.5 and 2.0 µm
        let (mut a, mut b) = (1.5, 2.0);
        let (mut fa, mut fb) = (f(a), f(b));
        eprintln!("{model}: Δn at 1.5 µm {fa:.5}, at 2.0 µm {fb:.5}");
        for _ in 0..8 {
            let c = b - fb * (b - a) / (fb - fa);
            (a, fa) = (b, fb);
            b = c;
            fb = f(b);
            if (b - a).abs() < 1e-4 {
                break;
            }
        }
        eprintln!("  matched width at 2 µm: {b:.4} µm");
    }
    let m = "skauli-2003";
    let (t, w) = (0.15, 1.5);
    let per_lam = (chang_mismatch(t, w, 2.01, m) - chang_mismatch(t, w, 1.99, m)) / 0.02;
    let per_t =
        (chang_mismatch(t + 0.001, w, 2.0, m) - chang_mismatch(t - 0.001, w, 2.0, m)) / 0.002;
    let per_w = (chang_mismatch(t, w + 0.01, 2.0, m) - chang_mismatch(t, w - 0.01, 2.0, m)) / 0.02;
    let at = chang_mismatch(t, w, 2.0, m);
    eprintln!(
        "150 nm, 1.5 µm: matched near {:.3} µm; dλ/dt {:.2} nm/nm, dλ/dW {:.3} nm/nm",
        2.0 - at / per_lam,
        -per_t / per_lam,
        -per_w / per_lam
    );
}

#[test]
#[ignore = "prints Zhao et al.'s efficiency, brightness, areas and pair bandwidth"]
fn zhao_numbers() {
    use crate::nonlinear::phase::{Device, Process, State, Variable, Wave, fourier_coefficient};
    let r = checks::zhao_ridge(None);
    let x_cut = Orientation::cut(
        crate::material::CrystalAxis::X,
        Some(crate::material::CrystalAxis::Z),
    )
    .unwrap();
    // their d33 = 27 pm/V (p. 1) alone
    let mut v = [[0.0; 6]; 3];
    v[2][2] = 27.0;
    let d = SecondOrder::from_voigt(v)
        .unwrap()
        .rotated(&x_cut)
        .in_section(Along::Y);
    for h in [0.02, 0.01, 0.005] {
        let grid = Grid::new(h).halved();
        let (cs, f) = checks::te0_mode(&r, &grid, 1.57);
        let (_, s) = checks::te0_mode(&r, &grid, 0.785);
        let ef = ModeField::new(&f, &cs, Wavelength::um(1.57).unwrap()).unwrap();
        let es = ModeField::new(&s, &cs, Wavelength::um(0.785).unwrap()).unwrap();
        let full = r.film_share(&cs, |_, _| true);
        // poled from the top down to 245 nm (p. 3), our reading of their poling depth
        let poled = r.film_share(&cs, |_, y| y >= 0.3 - 0.245);
        let k = second_harmonic_coupling(&es, &ef, &d, &full).unwrap();
        let kp = second_harmonic_coupling(&es, &ef, &d, &poled).unwrap();
        let (g50, g55) = (fourier_coefficient(1, 0.5), fourier_coefficient(1, 0.552));
        // η P L² per hertz, per GHz and mW, over 5 mm
        let b = |k: f64| k * k * 1e-3 * 5000.0f64.powi(2) * 1e9;
        let ff = f.fields_one_sided(&cs).unwrap();
        let fs = s.fields_one_sided(&cs).unwrap();
        eprintln!(
            "h {h}: η {:.1} %/(W cm²) at 50 %, {:.1} at 55.2 % poled 245 nm; B {:.3e}, {:.3e} pairs/(s GHz mW); A_eff Eq. 41 {:.4} {:.4}, Eq. 44 {:.4} {:.4}",
            percent_per_watt_cm2(k * g50),
            percent_per_watt_cm2(kp * g55),
            b(k * g50),
            b(kp * g55),
            ff.effective_area(),
            fs.effective_area(),
            ff.effective_area_transverse(),
            fs.effective_area_transverse()
        );
    }
    let (fi, si) = checks::zhao_indices(0.01);
    // the matched fundamental's shift across the period's half digit, 2.75 to 2.85 µm
    let span = 0.5
        * (checks::matched_wavelength(&fi, &si, 2.85) - checks::matched_wavelength(&fi, &si, 2.75))
            .abs();
    eprintln!(
        "the matched fundamental moves by ±{:.4} µm from 2.75 to 2.85 µm",
        span
    );
    let process = Process::difference_frequency(
        Wave::new(Wavelength::um(0.785).unwrap(), si.index().unwrap()),
        Wave::new(Wavelength::um(1.57).unwrap(), fi.index().unwrap()),
        fi.index().unwrap(),
    )
    .unwrap();
    let device = Device::quasi_phase_matched(process, 5000.0, State::default(), 1, 0.5).unwrap();
    let a = device.acceptance(Variable::Wavelength).unwrap();
    eprintln!(
        "degenerate pairs over 5 mm: the signal's FWHM {:.2} nm ({:.4} to {:.4} µm); n_g {:.4} {:.4}, β₂ {:.0} fs²/mm; {} and {} points",
        a.numerical * 1e3,
        a.lower,
        a.upper,
        fi.group_index(1.57),
        si.group_index(0.785),
        fi.gvd(1.57),
        fi.points(),
        si.points()
    );
}
#[test]
#[ignore = "prints the slab cases' grids and RK4's steps"]
fn slab_numbers() {
    for h in [0.01, 0.005, 0.0025] {
        eprintln!(
            "h {h}: η·H {:.3} (exact {:.3}); n_g {:.7}",
            checks::slab_efficiency(h) * 4.0 * h,
            checks::slab_efficiency_exact(h) * 4.0 * h,
            checks::slab_group_index(h)
        );
    }
    eprintln!("exact n_g {:.7}", checks::slab_group_index_exact());
    for steps in [250, 500, 1000, 2000] {
        eprintln!("RK4 on {steps}: {:.2e}", checks::slab_coupled(steps));
    }
}

#[test]
#[ignore = "writes the guide cases' rows of the validation report to $PHOTONOXIDE_GUIDE_ROWS"]
fn the_guide_cases_rows() {
    let mut rows = String::new();
    for case in checks::cases() {
        let t = std::time::Instant::now();
        let (row, passed) = crate::validation::row(&case);
        eprintln!(
            "{:8.1} s  {} {}",
            t.elapsed().as_secs_f64(),
            case.id,
            if passed { "pass" } else { "FAIL" }
        );
        rows.push_str(&row);
    }
    if let Some(out) = std::env::var_os("PHOTONOXIDE_GUIDE_ROWS") {
        std::fs::write(out, &rows).unwrap();
    }
    eprintln!("{rows}");
}
