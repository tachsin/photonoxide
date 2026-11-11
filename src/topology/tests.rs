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

/// I4's prototype: the testbed's mode converter as an objective, its gradient and its cost.
mod converter {
    use num_complex::Complex64 as c64;

    use crate::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
    use crate::topology::{Density, Edge, Placement, Projection};
    use crate::units::Wavelength;

    pub(crate) const NM: [f64; 6] = [1265.0, 1270.0, 1275.0, 1285.0, 1290.0, 1295.0];

    pub(crate) struct Converter {
        pub(crate) density: Density,
        placement: Placement,
        background: Vec<c64>,
        grid: Grid,
        /// per wavelength: the solver at the background, the two ports' modes, P₂/P₁
        bands: Vec<(Wavelength, Solver2d, PortMode, PortMode, f64)>,
    }

    impl Converter {
        pub(crate) fn new(radius_px: f64, beta: f64) -> Converter {
            let (ni, nj, h) = (350usize, 300usize, 0.01);
            let grid = Grid {
                nx: ni,
                ny: nj,
                dx: h,
                dy: h,
                x0: 0.0,
                y0: 0.0,
            };
            let background: Vec<c64> = (0..ni * nj)
                .map(|p| {
                    let (i, j) = (p % ni, p / ni);
                    let guide = (130..170).contains(&j) && (i < 95 || i >= ni - 96);
                    c64::new(if guide { 12.25 } else { 2.25 }, 0.0)
                })
                .collect();
            let placement = Placement {
                origin: [95, 70, 0],
                shape: [160, 160, 1],
                grid: [ni, nj, 1],
            };
            let density = Density::new([160, 160, 1], h, [2.25, 12.25])
                .unwrap()
                .with_filter(radius_px * h, Edge::Renormalized)
                .unwrap()
                .with_projection(Projection::new(beta, 0.5).unwrap());
            let bands = NM
                .iter()
                .map(|&nm| {
                    let lam = Wavelength::um(nm * 1e-3).unwrap();
                    let s = Solver2d::from_cells(
                        grid,
                        Polarization::Ez,
                        lam,
                        &background,
                        Boundaries::pml(20),
                    )
                    .unwrap();
                    let left = s.port_modes_within(25, 55..245, 1).unwrap().remove(0);
                    let right = s.port_modes_within(325, 55..245, 2).unwrap().remove(1);
                    let ratio = right.power(h) / left.power(h);
                    (lam, s, left, right, ratio)
                })
                .collect();
            Converter {
                density,
                placement,
                background,
                grid,
                bands,
            }
        }

        fn cells(&self, rho: &[f64]) -> Vec<c64> {
            let mut cells = self.background.clone();
            let eps = self.density.permittivity(rho).unwrap();
            self.placement.embed(&eps, &mut cells).unwrap();
            cells
        }

        /// Per wavelength: R, T and their gradients in the densities.
        pub(crate) fn evaluate(&self, rho: &[f64]) -> Vec<(f64, f64, Vec<f64>, Vec<f64>)> {
            let cells = self.cells(rho);
            let _ = self.grid;
            self.bands
                .iter()
                .map(|(lam, base, left, right, ratio)| {
                    let s = base.reuse_cells(*lam, &cells).unwrap();
                    let src = s.mode_source(left, Direction::Forward);
                    let field = s.solve_system(&src).unwrap();
                    let (r, dr) = s
                        .mode_power_gradient(&field, left, Direction::Backward)
                        .unwrap();
                    let (t, dt) = s
                        .mode_power_gradient(&field, right, Direction::Forward)
                        .unwrap();
                    let pull = |d: Vec<f64>, scale: f64| {
                        let d_eps: Vec<f64> = self.placement.restrict(&d).unwrap();
                        let g = self.density.gradient(rho, &d_eps).unwrap();
                        g.into_iter().map(|v| v * scale).collect::<Vec<f64>>()
                    };
                    (r, t * ratio, pull(dr, 1.0), pull(dt, *ratio))
                })
                .collect()
        }
    }

    /// The epigraph's arrays at `y` = (ρ, t): objective t, constraints R_k − t and 1 − T_k − t.
    fn epigraph(
        c: &Converter,
        y: &[f64],
        gradient: &mut [f64],
        g: &mut [f64],
        jacobian: &mut [f64],
    ) -> (f64, f64, f64) {
        let n = y.len() - 1;
        let v = c.evaluate(&y[..n]);
        let mut values = Vec::with_capacity(12);
        let mut gradients = Vec::with_capacity(12 * n);
        for (r, t, dr, dt) in &v {
            values.push(*r);
            gradients.extend_from_slice(dr);
            values.push(1.0 - t);
            gradients.extend(dt.iter().map(|d| -d));
        }
        let form = crate::topology::Epigraph::new(n, 12).unwrap();
        let obj = form
            .evaluate(y, &values, &gradients, gradient, g, jacobian)
            .unwrap();
        let worst_r = v.iter().map(|x| x.0).fold(0.0, f64::max);
        let worst_t = v.iter().map(|x| x.1).fold(1.0, f64::min);
        let _ = obj;
        (crate::topology::Epigraph::start(&values), worst_r, worst_t)
    }

    #[test]
    #[ignore = "I4's MMA prototype: β from 8 to 256, unconstrained, prints"]
    fn the_converter_mma_prototype() {
        use genoxide::prelude::*;
        let start = std::time::Instant::now();
        let n = 160 * 160;
        // a start that breaks the mirror symmetry across the guides' axis: a uniform 0.5 can't
        // turn the even mode into the odd one, and its gradient there is zero
        let mut rho: Vec<f64> = (0..n)
            .map(|p| {
                let (i, j) = ((p % 160) as f64 / 160.0, (p / 160) as f64 / 160.0);
                0.5 + 0.25
                    * (std::f64::consts::PI * (j - 0.5)).sin()
                    * (std::f64::consts::PI * i).sin()
            })
            .collect();
        for beta in [8.0, 16.0, 32.0, 64.0, 128.0, 256.0] {
            let c = Converter::new(5.0, beta);
            // t starts at the worst objective
            let (mut gr, mut g, mut j) = (vec![0.0; n + 1], vec![0.0; 12], vec![0.0; 12 * (n + 1)]);
            let mut y = rho.clone();
            y.push(1.0);
            let (worst, _, _) = epigraph(&c, &y, &mut gr, &mut g, &mut j);
            y[n] = worst;
            let fitness = Constrained::differentiable(
                12,
                |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                    epigraph(&c, x, gradient, g, jacobian);
                    x[n]
                },
            );
            let mma = Mma::builder(Real::new(vec![0.0..=1.0; n + 1]).unwrap())
                .initial_genome(Reals::from(y))
                .minimize()
                .build()
                .unwrap();
            let outcome = Engine::new(mma, fitness)
                .stop_when(Stop::evaluations(30))
                .run()
                .unwrap();
            let best = outcome.best_genome();
            rho = best[..n].to_vec();
            let mut y = rho.clone();
            y.push(best[n]);
            let (worst, r, t) = epigraph(&c, &y, &mut gr, &mut g, &mut j);
            println!(
                "β {beta:>5}: worst objective {worst:.4e}, worst R {:.2} dB, worst T {:.3} dB, {:.0} s",
                10.0 * r.log10(),
                10.0 * t.log10(),
                start.elapsed().as_secs_f64()
            );
        }
        let report = |c: &Converter, rho: &[f64], what: &str| {
            let phys = c.density.physical(rho).unwrap();
            let pattern = crate::topology::ruler::Pattern::binarized([160, 160], &phys).unwrap();
            let len = crate::topology::ruler::minimum_length(&pattern, [10.0, 10.0]).unwrap();
            let grey = phys.iter().filter(|v| **v > 0.05 && **v < 0.95).count() as f64 / n as f64;
            println!("{what}: minimum length {len:.1} nm, grey share {grey:.4}");
        };
        report(&Converter::new(5.0, 256.0), &rho, "unconstrained");
        // then the 50 nm linewidth and spacing (Arrieta et al.: R = ℓ_t = 50 nm, c = 64 R², ε of
        // Eq. 18 for a strip across the 1.6 µm region), each as g/ε − 1 ≤ 0 beside the epigraph's
        let c = Converter::new(5.0, 256.0);
        let lc = crate::topology::LengthConstraints::new(&c.density, 0.05, 0.05)
            .unwrap()
            .with_arrieta_decay();
        let eps = crate::topology::geometric_threshold(64.0, 50.0 / 1600.0).unwrap();
        println!("ε {eps:.3e}, thresholds {:?}", lc.thresholds());
        let m = 14;
        let constrained = |x: &[f64], gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
            let w = n + 1;
            epigraph(&c, x, gradient, &mut g[..12], &mut jacobian[..12 * w]);
            let l = lc.evaluate(&x[..n]).unwrap();
            for s in 0..2 {
                g[12 + s] = l.values[s] / eps - 1.0;
                let row = &mut jacobian[(12 + s) * w..(13 + s) * w];
                for (r, d) in row[..n].iter_mut().zip(&l.gradients[s]) {
                    *r = d / eps;
                }
                row[n] = 0.0;
            }
            (l.values[0] / eps, l.values[1] / eps)
        };
        let (mut gr, mut g, mut j) = (vec![0.0; n + 1], vec![0.0; m], vec![0.0; m * (n + 1)]);
        let mut y = rho.clone();
        y.push(1.0);
        let (worst, _, _) = epigraph(&c, &y, &mut gr, &mut g[..12], &mut j[..12 * (n + 1)]);
        y[n] = worst;
        let (s0, v0) = constrained(&y, &mut gr, &mut g, &mut j);
        println!("before: g_LW/ε {s0:.3e}, g_LS/ε {v0:.3e}");
        {
            use genoxide::prelude::*;
            let fitness = Constrained::differentiable(
                m,
                |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                    constrained(x, gradient, g, jacobian);
                    x[n]
                },
            );
            let mma = Mma::builder(Real::new(vec![0.0..=1.0; n + 1]).unwrap())
                .initial_genome(Reals::from(y))
                .minimize()
                .build()
                .unwrap();
            let outcome = Engine::new(mma, fitness)
                .stop_when(Stop::evaluations(60))
                .run()
                .unwrap();
            let best = outcome.best_genome();
            rho = best[..n].to_vec();
            let mut y = rho.clone();
            y.push(best[n]);
            let (s1, v1) = constrained(&y, &mut gr, &mut g, &mut j);
            let (worst, r, t) = epigraph(&c, &y, &mut gr, &mut g[..12], &mut j[..12 * (n + 1)]);
            println!(
                "constrained: worst objective {worst:.4e}, worst R {:.2} dB, worst T {:.3} dB, g_LW/ε {s1:.3e}, g_LS/ε {v1:.3e}, {:.0} s",
                10.0 * r.log10(),
                10.0 * t.log10(),
                start.elapsed().as_secs_f64()
            );
        }
        report(&c, &rho, "constrained");
        // the design binarized, and its minimum length
        let c = Converter::new(5.0, 256.0);
        let phys = c.density.physical(&rho).unwrap();
        let pattern = crate::topology::ruler::Pattern::binarized([160, 160], &phys).unwrap();
        let len = crate::topology::ruler::minimum_length(&pattern, [10.0, 10.0]).unwrap();
        let grey = phys.iter().filter(|v| **v > 0.05 && **v < 0.95).count() as f64 / n as f64;
        println!("minimum length {len:.1} nm, grey share {grey:.3}");
    }

    #[test]
    #[ignore = "I4's prototype: the converter's gradient against differences and its cost, prints"]
    fn the_converter_prototype() {
        let start = std::time::Instant::now();
        let c = Converter::new(5.0, 8.0);
        println!("set up in {:.1} s", start.elapsed().as_secs_f64());
        // a start: the design's middle half grey, its edges void, the guides' rows solid
        let rho: Vec<f64> = (0..160 * 160)
            .map(|p| {
                let (i, j) = ((p % 160) as f64, (p / 160) as f64);
                0.5 + 0.3 * (0.11 * i).sin() * (0.07 * j + 0.5).cos()
            })
            .collect();
        let t0 = std::time::Instant::now();
        let v = c.evaluate(&rho);
        println!(
            "one evaluation (6 wavelengths, 3 solves each): {:.2} s",
            t0.elapsed().as_secs_f64()
        );
        for (k, (r, t, _, _)) in v.iter().enumerate() {
            println!("  λ {} nm: R {r:.4e}, T {t:.4e}", super::converter::NM[k]);
        }
        // the gradient at three pixels against fourth-order differences, at the first wavelength
        let delta = 2e-3;
        for p in [0usize, 80 * 160 + 80, 159 * 160 + 3] {
            let at = |d: f64| {
                let mut r = rho.clone();
                r[p] += d;
                let v = c.evaluate(&r);
                (v[0].0, v[0].1)
            };
            let [m2, m1, p1, p2] = [-2.0, -1.0, 1.0, 2.0].map(|s| at(s * delta));
            let fd = |a: f64, b: f64, c: f64, d: f64| (a - 8.0 * b + 8.0 * c - d) / (12.0 * delta);
            let (dr, dt) = (v[0].2[p], v[0].3[p]);
            println!(
                "  pixel {p}: dR {dr:+.6e} (differences {:+.6e}), dT {dt:+.6e} (differences {:+.6e})",
                fd(m2.0, m1.0, p1.0, p2.0),
                fd(m2.1, m1.1, p1.1, p2.1)
            );
        }
    }
}

/// Arrieta et al.'s threshold against the discrete constraint at the critical strip: prints.
#[test]
#[ignore = "the Arrieta study: prints, in release"]
fn the_arrieta_study() {
    for r_px in [25.0, 50.0, 100.0, 200.0] {
        let (g, want) = super::checks::arrieta_strip(r_px);
        println!(
            "R = {r_px} px: discrete {g:.6e}, Eq. 18 {want:.6e}, relative {:.3e}",
            (g - want).abs() / want
        );
    }
}
