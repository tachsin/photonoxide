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

/// The robust cases' study (their rules): the discrete filters' values across a straight edge
/// against the closed forms, and Eq. 29 on bars of whole pixels, as the radius grows in pixels.
#[test]
#[ignore = "the robust cases' study: prints, in release"]
fn the_robust_study() {
    use crate::topology::checks::{bar_length, edge_values_error};
    for r_px in [5.0, 10.0, 20.0, 40.0, 80.0] {
        let (e1, e2) = (edge_values_error(1, r_px), edge_values_error(2, r_px));
        println!(
            "R = {r_px:4} px: 1D {e1:.3e} (1/(8R²) {:.3e}), 2D {e2:.3e}, 2D × (R/Δ)² {:.3}",
            1.0 / (8.0 * r_px * r_px),
            e2 * r_px * r_px
        );
    }
    for r_px in [25.0, 50.0, 100.0, 200.0] {
        let worst = super::checks::ETAS_ROBUST
            .iter()
            .map(|&eta| (bar_length(r_px, eta) - super::minimum_length(eta).unwrap()).abs())
            .fold(0.0, f64::max);
        println!(
            "R = {r_px:5} px: Eq. 29 worst {worst:.3e}, × (R/Δ)² {:.2}",
            worst * r_px * r_px
        );
    }
    for delta in [1e-3, 2e-3, 4e-3, 8e-3] {
        let (worst, resolution) = super::checks::robust_epigraph_error(delta);
        println!("epigraph, δ {delta:.0e}: largest error {worst:.2e}, resolution {resolution:.2e}");
    }
}

#[test]
fn the_edge_shifts_solve_their_closed_forms() {
    for eta in [0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99] {
        let t = super::edge_shift_2d(eta).unwrap();
        assert!((super::across_edge_2d(t) - eta).abs() < 1e-15, "{eta}: {t}");
        let t1 = super::edge_shift_1d(eta).unwrap();
        assert!(
            (super::across_edge_1d(t1) - eta).abs() < 1e-15,
            "{eta}: {t1}"
        );
    }
    assert!(super::edge_shift_2d(0.5).unwrap().abs() < 1e-15);
    // the closed form's ends, and its slope, −(3/π)(√(1 − t²) − t² arcosh(1/t)), against
    // central differences
    assert_eq!(super::across_edge_2d(0.0), 0.5);
    assert_eq!(super::across_edge_2d(1.0), 0.0);
    for t in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let h = 1e-5;
        let fd = (super::across_edge_2d(t + h) - super::across_edge_2d(t - h)) / (2.0 * h);
        let slope =
            -3.0 / std::f64::consts::PI * ((1.0 - t * t).sqrt() - t * t * (1.0f64 / t).acosh());
        assert!((fd - slope).abs() < 1e-9, "{t}: {fd} {slope}");
    }
    assert!(super::edge_shift_2d(0.0).is_err() && super::edge_shift_1d(1.0).is_err());
    // the shift is a fraction of the radius: 2D moves less than 1D at the same threshold
    let (s1, s2) = (
        super::edge_shift_1d(0.25).unwrap(),
        super::edge_shift_2d(0.25).unwrap(),
    );
    assert!(s2 > 0.0 && s2 < s1, "{s1} {s2}");
    // the page's numbers: 0.293 R and 0.283 R at η = ¼
    assert!(
        (s1 - 0.2929).abs() < 1e-4 && (s2 - 0.2828).abs() < 1e-4,
        "{s1} {s2}"
    );
}

#[test]
fn the_robust_realizations_and_the_epigraph() {
    let d = super::Density::new([4, 4, 1], 0.02, [2.0, 12.0])
        .unwrap()
        .with_filter(0.05, super::Edge::Renormalized)
        .unwrap();
    assert!(d.robust(0.25).is_err(), "no projection");
    let d = d.with_projection(super::Projection::new(8.0, 0.5).unwrap());
    let r = d.robust(0.25).unwrap();
    let etas = r.all().map(|x| x.projection().unwrap().eta());
    assert_eq!(etas, [0.75, 0.5, 0.25]);
    assert!(d.robust(0.5).is_err() && d.robust(0.0).is_err());
    // a uniform 0.6: eroded below nominal below dilated
    let rho = vec![0.6; 16];
    let at = |x: &super::Density| x.physical(&rho).unwrap()[5];
    assert!(at(&r.eroded) < at(&r.nominal) && at(&r.nominal) < at(&r.dilated));
    // Eq. 29's branches meet at η = ¼, and its value at Wang et al.'s η = 0.2
    assert!((super::minimum_length(0.25).unwrap() - 1.0).abs() < 1e-15);
    assert!((super::minimum_length(0.2).unwrap() - 1.1055728).abs() < 1e-7);
    // the epigraph: objective t, constraints f − t, rows [∇f, −1]
    let e = super::Epigraph::new(2, 3).unwrap();
    let (values, gradients) = ([1.0, 3.0, 2.0], [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let t = super::Epigraph::start(&values);
    let (mut gr, mut g, mut j) = ([0.0; 3], [0.0; 3], [0.0; 9]);
    let f = e
        .evaluate(&[0.1, 0.2, t], &values, &gradients, &mut gr, &mut g, &mut j)
        .unwrap();
    assert_eq!((f, gr, g), (3.0, [0.0, 0.0, 1.0], [-2.0, 0.0, -1.0]));
    assert_eq!(j, [1.0, 2.0, -1.0, 3.0, 4.0, -1.0, 5.0, 6.0, -1.0]);
    assert!(
        e.evaluate(&[0.0; 2], &values, &gradients, &mut gr, &mut g, &mut j)
            .is_err()
    );
}

/// The ruler's study: shapes of known length and the testbed's designs, printed.
#[test]
#[ignore = "the ruler's study: prints, in release"]
fn the_ruler_study() {
    for (name, measured, want) in super::checks::ruler_shapes() {
        println!("{name}: measured {measured:.4} px, its own {want}");
    }
    let start = std::time::Instant::now();
    for (name, measured, printed) in super::checks::testbed_lengths() {
        println!(
            "{name}: measured {measured} nm, printed {printed} nm, difference {:.4}",
            measured - printed
        );
    }
    println!("{:.1} s", start.elapsed().as_secs_f64());
}

#[test]
fn the_ruler_measures_what_it_should_and_refuses_what_it_cant() {
    use super::ruler::{Pattern, minimum_length, violates};
    assert!(Pattern::new([0, 4], vec![]).is_err());
    assert!(Pattern::new([2, 2], vec![true; 3]).is_err());
    assert!(Pattern::binarized([2, 1], &[0.0, f64::NAN]).is_err());
    // binarized at the middle of its range: 0.2 and 0.6 of [0, 0.8]
    let p = Pattern::binarized([3, 1], &[0.0, 0.2, 0.8]).unwrap();
    assert!(!p.is_solid(0, 0) && !p.is_solid(1, 0) && p.is_solid(2, 0));
    // a uniform pattern: the image's shorter side
    let uniform = Pattern::new([10, 6], vec![true; 60]).unwrap();
    assert_eq!(minimum_length(&uniform, [1.0, 2.0]).unwrap(), 10.0);
    assert!(minimum_length(&uniform, [0.0, 1.0]).is_err());
    // a 9-pixel bar in a 40 × 40 void: no violation at 5 pixels, one at 15
    let bar = Pattern::new(
        [40, 40],
        (0..1600).map(|p| (16..25).contains(&(p % 40))).collect(),
    )
    .unwrap();
    assert!(!violates(&bar, [1.0, 1.0], 5.0) && violates(&bar, [1.0, 1.0], 15.0));
    let d = minimum_length(&bar, [1.0, 1.0]).unwrap();
    assert!((d - 9.0).abs() <= 1.0, "{d}");
    // the same bar with pixels 10 wide: ten times the length
    let d10 = minimum_length(&bar, [10.0, 10.0]).unwrap();
    assert!((d10 - 10.0 * d).abs() < 1e-9, "{d10} {d}");
}

/// The constraints' study: their values, and the gradient against differences per δ.
#[test]
#[ignore = "the constraints' study: prints, in release"]
fn the_constraint_study() {
    let (c, rho) = super::checks::constraint_problem();
    let v = c.evaluate(&rho).unwrap();
    println!(
        "thresholds {:?}, c {}, values {:?}",
        c.thresholds(),
        c.decay(),
        v.values
    );
    for &p in &super::checks::CONSTRAINT_PIXELS {
        println!(
            "  pixel {p}: ∂g_LW {:+.3e}, ∂g_LS {:+.3e}",
            v.gradients[0][p], v.gradients[1][p]
        );
    }
    for delta in [1e-5, 1e-4, 1e-3, 1e-2] {
        let (worst, resolution) = super::checks::constraint_gradient_error(delta);
        println!("δ {delta:.0e}: largest error {worst:.2e}, resolution {resolution:.2e}");
    }
}

#[test]
fn a_bar_narrower_than_the_linewidth_violates_and_a_wider_one_does_not() {
    let (ni, nj, h) = (60usize, 8usize, 0.02);
    let density = super::Density::new([ni, nj, 1], h, [2.085, 12.08])
        .unwrap()
        .with_filter(3.0 * h, super::Edge::Renormalized)
        .unwrap()
        .with_projection(super::Projection::new(8.0, 0.5).unwrap());
    let c = super::LengthConstraints::new(&density, 0.09, 0.09).unwrap();
    // a bar along j, `w` pixels wide, in the middle of the void
    let bar = |w: usize| -> Vec<f64> {
        (0..ni * nj)
            .map(|p| {
                let i = p % ni;
                if (30 - w / 2..30 - w / 2 + w).contains(&i) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect()
    };
    let narrow = c.evaluate(&bar(3)).unwrap().values[0];
    let wide = c.evaluate(&bar(12)).unwrap().values[0];
    // the wide bar isn't 0: at its edges, with c = (R/Δ)⁴ = 81 and a gradient near 1/6 a pixel,
    // the indicator is e^-2.25, about 0.1 (Zhou et al.'s c keeps it finite at any flat point)
    assert!(
        narrow > 1e-4 && narrow > 10.0 * wide,
        "narrow {narrow}, wide {wide}"
    );
    // and the refusals
    let bare = super::Density::new([4, 4, 1], h, [1.0, 2.0]).unwrap();
    assert!(super::LengthConstraints::new(&bare, 0.09, 0.09).is_err());
    assert!(super::eroded_threshold(0.0, 0.06).is_err());
    assert_eq!(super::eroded_threshold(0.2, 0.06).unwrap(), 1.0);
    assert_eq!(super::dilated_threshold(0.2, 0.06).unwrap(), 0.0);
}

/// The testbed's designs on photonoxide's 2D FDFD at 10 nm against the README's numbers.
#[test]
#[ignore = "I4's first study: the published designs re-evaluated, prints, in release"]
fn the_testbed_spectrum_study() {
    let printed = [
        (-22.50, -0.19),
        (-27.17, -0.19),
        (-28.49, -0.35),
        (-33.62, -0.19),
        (-21.23, -0.81),
        (-23.66, -0.93),
        (-18.43, -1.23),
        (-18.16, -1.34),
    ];
    let start = std::time::Instant::now();
    for ((name, hex, _), (pr, pt)) in super::checks::TESTBED.iter().zip(printed) {
        let (r, t) = super::checks::testbed_spectrum(hex);
        println!("{name}: R {r:+.2} dB (printed {pr:+.2}), T {t:+.3} dB (printed {pt:+.2})");
    }
    println!("{:.1} s", start.elapsed().as_secs_f64());
}

/// The same at 5 nm, for the grid's own error beside the comparison.
#[test]
#[ignore = "I4's grid study: prints, in release"]
fn the_testbed_grid_study() {
    let start = std::time::Instant::now();
    for (name, hex, _) in super::checks::TESTBED.iter().take(3) {
        let (r1, t1) = super::checks::testbed_spectrum_at(hex, 1);
        let (r2, t2) = super::checks::testbed_spectrum_at(hex, 2);
        println!(
            "{name}: 10 nm R {r1:+.3} T {t1:+.4}; 5 nm R {r2:+.3} T {t2:+.4}; changes R {:+.3} T {:+.4} dB",
            r2 - r1,
            t2 - t1
        );
    }
    println!("{:.1} s", start.elapsed().as_secs_f64());
}
