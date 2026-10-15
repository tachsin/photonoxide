use super::*;
use crate::mode::Polarization as Kind;

#[test]
fn a_slab_off_the_grid_converges_at_second_order_from_the_fill_and_first_by_points() {
    let errors = |pol: Polarization, a: Averaging| -> Vec<(f64, f64)> {
        [16, 32, 64]
            .into_iter()
            .map(|cells| {
                let (found, exact) = slab_index(pol, a, cells);
                (SLAB.0 / (cells as f64 + 0.4), found - exact)
            })
            .collect()
    };
    let mean = last_order(&errors(Polarization::Ez, Averaging::Mean));
    let point = last_order(&errors(Polarization::Ez, Averaging::Point));
    assert!(mean > 1.9, "{mean}");
    assert!(point < 1.2, "{point}");
}

#[test]
fn the_adjoint_derivative_is_the_powers_finite_difference() {
    let (adjoint, fd) = radius_derivative(Polarization::Ez, Averaging::Mean);
    assert!((adjoint - fd).abs() < 1e-5 * fd.abs(), "{adjoint} {fd}");
}

/// The numbers the cases are built from:
/// `cargo test --release --lib fdfd::fill_checks::tests::numbers -- --ignored --nocapture`.
#[test]
#[ignore]
fn numbers() {
    let show = |e: &[(f64, f64)]| {
        e.iter()
            .map(|p| format!("{:.2e}", p.1))
            .collect::<Vec<_>>()
            .join(" ")
    };
    for pol in [Polarization::Ez, Polarization::Hz] {
        for a in Averaging::ALL {
            let e = slab_errors(pol, a);
            println!(
                "slab {pol:?} {:>8}: last {:.2} [{}]",
                a.name(),
                last_order(&e),
                show(&e)
            );
        }
    }
    for kind in [Kind::Te, Kind::Tm] {
        for a in [
            Averaging::Point,
            Averaging::Samples,
            Averaging::Mean,
            Averaging::Diagonal,
        ] {
            let e = film_errors(kind, a);
            println!(
                "film {kind:?} {:>8}: last {:.2} [{}]",
                a.name(),
                last_order(&e),
                show(&e)
            );
        }
    }
    for pol in [Polarization::Hz, Polarization::Ez] {
        for a in Averaging::ALL {
            let e = crystal_errors(pol, a);
            println!(
                "crystal {pol:?} {:>8}: order {:.2} [{}]",
                a.name(),
                order(&e[..FITTED]),
                show(&e)
            );
        }
    }
    for pol in [Polarization::Ez, Polarization::Hz] {
        for a in [Averaging::Mean, Averaging::Diagonal, Averaging::Tensor] {
            let (adj, fd) = radius_derivative(pol, a);
            println!(
                "radius {pol:?} {:>8}: adjoint {adj:.9e} fd {fd:.9e} ({:.1e})",
                a.name(),
                (adj - fd).abs() / fd.abs()
            );
        }
    }
}

/// The crystal's references: `cargo test --release --lib fdfd::fill_checks::tests::reference
/// -- --ignored --nocapture` (about 15 s).
#[test]
#[ignore]
fn reference() {
    for (pol, a) in [
        (Polarization::Hz, Averaging::Tensor),
        (Polarization::Ez, Averaging::Mean),
    ] {
        let w: Vec<f64> = [80, 160, 320]
            .iter()
            .map(|&r| crystal_band(pol, a, r))
            .collect();
        let rich = (4.0 * w[2] - w[1]) / 3.0;
        println!("{pol:?} {w:?} extrapolated {rich:.12}");
    }
}

/// The report's rows of the averaging's cases, to append to docs/validation.md:
/// `cargo test --release --lib fdfd::fill_checks::tests::rows -- --ignored --nocapture`.
#[test]
#[ignore]
fn rows() {
    for case in crate::validation::cases() {
        if case.id.starts_with("averaging/") || case.id.contains("/fill-") {
            let (row, passed) = crate::validation::row(&case);
            assert!(passed, "{row}");
            print!("ROW{row}");
        }
    }
}
