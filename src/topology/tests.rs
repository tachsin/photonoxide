use super::checks::{self, PIXELS, chain};
use super::*;

#[test]
fn the_projection_is_a_step_at_large_beta_and_its_derivative_is_exact() {
    assert!(checks::projection_derivative_error() < 1e-12);
    assert!(checks::projection_identities_error() < 1e-14);
    let p = Projection::new(256.0, 0.5).unwrap();
    assert!(p.value(0.45) < 1e-5 && p.value(0.55) > 1.0 - 1e-5);
    assert_eq!(p.derivative(0.0), p.derivative(1.0));
    // the cosh overflows far from η: the derivative is then 0, not NaN
    let steep = Projection::new(1e4, 0.5).unwrap();
    assert_eq!(steep.derivative(0.0), 0.0);
}

#[test]
fn the_filter_transposes_and_keeps_linear_densities() {
    assert!(checks::filter_transpose_error() < 1e-12);
    assert!(checks::filter_linear_error() < 1e-14);
}

#[test]
fn a_filter_below_a_pixel_changes_nothing() {
    let f = ConicFilter::new([7, 5, 1], 0.02, 0.015, Edge::Renormalized).unwrap();
    let rho = checks::draws(35, 3);
    assert_eq!(f.apply(&rho).unwrap(), rho);
    assert_eq!(f.transpose(&rho).unwrap(), rho);
}

#[test]
fn a_padded_edge_pulls_the_density_towards_its_value() {
    let n = 12 * 12;
    let f = ConicFilter::new([12, 12, 1], 0.01, 0.04, Edge::Padded(1.0)).unwrap();
    let out = f.apply(&vec![0.0; n]).unwrap();
    // the corner sees the most of the padding, the middle none
    assert!(out[0] > out[1] && out[1] > 0.0);
    assert_eq!(out[6 * 12 + 6], 0.0);
}

#[test]
fn the_chain_runs_through_the_placement() {
    let c = chain();
    let eps = c.density.permittivity(&c.rho).unwrap();
    let mut cells = c.cells.clone();
    c.placement.embed(&eps, &mut cells).unwrap();
    let back: Vec<f64> = cells.iter().map(|z| z.re).collect();
    assert_eq!(c.placement.restrict(&back).unwrap(), eps);
    let bad = Placement {
        origin: [c.grid.nx - 5, 0, 0],
        ..c.placement
    };
    assert!(bad.embed(&eps, &mut cells).is_err());
}

#[test]
fn invalid_values_are_refused() {
    assert!(Projection::new(0.0, 0.5).is_err());
    assert!(Projection::new(f64::INFINITY, 0.5).is_err());
    assert!(Projection::new(8.0, 1.5).is_err());
    assert!(ConicFilter::new([0, 4, 1], 0.01, 0.03, Edge::Renormalized).is_err());
    assert!(ConicFilter::new([4, 4, 1], 0.01, -1.0, Edge::Renormalized).is_err());
    assert!(ConicFilter::new([4, 4, 1], 0.01, 0.03, Edge::Padded(2.0)).is_err());
    let d = Density::new([4, 4, 1], 0.01, [1.0, 12.0]).unwrap();
    assert!(d.physical(&[0.5; 15]).is_err());
    assert!(d.pullback(&[0.5; 16], &[1.0; 3]).is_err());
}

#[test]
#[ignore = "slow in a debug build; the validation report runs it in release (topology/fdfd-gradient)"]
fn the_pulled_back_gradient_is_the_finite_differences() {
    let probes = chain().probes(&PIXELS, 1e-3);
    for p in &probes {
        let error = (p.gradient - p.differences()).abs() / p.gradient.abs();
        assert!(error < 1e-6, "{error} (resolution {})", p.resolution());
    }
}

#[test]
#[ignore = "the step study behind topology/fdfd-gradient's δ: prints, in release"]
fn the_chain_step_study() {
    let c = chain();
    let deltas = [1e-4, 2.5e-4, 5e-4, 1e-3, 2e-3, 4e-3, 8e-3, 1.6e-2, 3.2e-2];
    let start = std::time::Instant::now();
    for (k, &delta) in deltas.iter().enumerate() {
        for (p, probe) in PIXELS.iter().zip(c.probes(&PIXELS, delta)) {
            println!(
                "δ {delta:.1e} pixel {p:3}: gradient {:+.6e}, error {:.2e}, resolution {:.2e}",
                probe.gradient,
                (probe.gradient - probe.differences()).abs() / probe.gradient.abs(),
                probe.resolution()
            );
        }
        let done = k + 1;
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "step {done}/{} ({:.0} %), {elapsed:.0} s, about {:.0} s left",
            deltas.len(),
            100.0 * done as f64 / deltas.len() as f64,
            elapsed / done as f64 * (deltas.len() - done) as f64
        );
    }
}
