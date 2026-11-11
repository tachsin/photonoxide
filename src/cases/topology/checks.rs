//! The validation cases of densities ([`super`]): the projection's derivative and identities,
//! the filter's transpose and what it reproduces exactly, and the whole chain's gradient from 2D
//! FDFD's adjoint against finite differences; and of robust designs: the filter's density across
//! a straight edge against its closed forms, the minimum length of Qian & Sigmund's Eq. 29 on
//! bars of whole pixels, and the epigraph's Jacobian through the three realizations' gradients.

use num_complex::Complex64 as c64;

use super::{
    ConicFilter, Density, Edge, Epigraph, Placement, Projection, across_edge_1d, across_edge_2d,
    minimum_length,
};
use crate::fdfd::checks::{Probe, two_ports};
use crate::fdfd::{Boundaries, Direction, Grid, Polarization, Solver2d};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "topology/projection-derivative",
            title: r"The threshold projection's derivative $d\hat\rho/d\tilde\rho$ against complex-step differentiation of its value, at 201 points of $[0, 1]$ for $\beta$ = 1, 8, 64 and 256 and $\eta$ = 0.25, 0.5 and 0.75 (largest difference relative to the largest derivative, shown)",
            tier: Tier::Analytic,
            source: r"F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (2010), doi:10.1007/s00158-010-0602-y, Eq. 9; the complex step, $f'(x) = \mathop{\mathrm{Im}} f(x + ih)/h$, is exact to round-off for an analytic $f$ (no difference is taken)",
            run: projection_derivative,
        },
        Case {
            id: "topology/projection-identities",
            title: r"The threshold projection's fixed values: $\hat\rho(0) = 0$ and $\hat\rho(1) = 1$ for every $\beta$ and $\eta$ above, and at $\eta = 1/2$ its symmetry $\hat\rho(1 - x) = 1 - \hat\rho(x)$ at 201 points (largest departure shown)",
            tier: Tier::Analytic,
            source: r"Wang et al. 2010, Eq. 9: its numerator is the denominator at $\tilde\rho = 1$ and vanishes at $\tilde\rho = 0$; tanh is odd, so at $\eta = 1/2$ the two halves are each other's reflections",
            run: projection_identities,
        },
        Case {
            id: "topology/filter-transpose",
            title: r"The conic filter's transpose: $\langle F x, y\rangle$ against $\langle x, F^{\mathsf T} y\rangle$ for densities drawn on regions of 160 × 160 pixels (radius 5.5 pixels) and 24 × 20 × 16 (radius 2.5), renormalized at the edges and padded (relative difference, largest shown)",
            tier: Tier::Analytic,
            source: r"Wang et al. 2010, Eqs. 4–7: the gradient with respect to the densities is the filter's transpose applied to the gradient with respect to the filtered densities; an exact identity of the two sums",
            run: filter_transpose,
        },
        Case {
            id: "topology/filter-preserves-linear",
            title: r"The conic filter reproduces a uniform density everywhere (renormalized, and padded with the same density), and a linear ramp $0.2 + 0.003\thinspace i + 0.002\thinspace j$ at every pixel a radius from the edges, in 2D and 3D (largest departure shown)",
            tier: Tier::Analytic,
            source: r"Wang et al. 2010, Eqs. 4 and 5: the weights are normalized, so a constant is kept; they are symmetric about the pixel, so a linear function's first moment cancels",
            run: filter_preserves_linear,
        },
        Case {
            id: "topology/fdfd-gradient",
            title: r"2D FDFD, $E$ along $z$: the gradient of a port mode's power with respect to a 10 × 10 design region's densities (conic filter of 3 pixels, projection at $\beta = 8$, $\eta = 1/2$, silicon in oxide beside a 220 nm slab, 20 nm grid), from the adjoint pulled back through the projection and the filter's transpose, against fourth-order central differences of the densities ($\delta = 2 \times 10^{-3}$) at a corner, an edge and the middle (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2–4, through Wang et al. 2010's chain rule, Eq. 7",
            run: fdfd_gradient,
        },
        Case {
            id: "topology/robust-edge-1d",
            title: r"The 1D hat filter's density across a solid half-line's edge on a pixel boundary, at every pixel centre within a radius of it, against $\frac12(1 - t)^2$ ($t$ the distance in radii, outside; its reflection inside) less the pixels' own offset $\frac18(\Delta/R)^2$, for $R/\Delta$ = 5, 20 and 80 (largest departure shown)",
            tier: Tier::Analytic,
            source: r"the hat's share of weight beyond a distance $x$, $\int_x^R (R - s)\,ds / R^2$; on pixels, the sum $\sum_{k > i} (R - k)$ over whole pixels is the integral less $\Delta^2/8$, exactly, when $R/\Delta$ is whole (the discrete midpoint rule on a linear function cut at a pixel boundary)",
            run: robust_edge_1d,
        },
        Case {
            id: "topology/robust-edge-2d",
            title: r"The 2D conic filter's density across a solid half-plane's straight edge on a pixel boundary, at the pixel centres within a radius of it on a row far from the region's ends, against the closed form $\frac12 - (2t\sqrt{1 - t^2} + \arcsin t - t^3 \mathop{\mathrm{arcosh}}(1/t))/\pi$, at $R/\Delta = 40$ (largest departure shown)",
            tier: Tier::Analytic,
            source: r"the cone $R - r$ over a disc integrated across a straight edge: its marginal is $m(s) = R\sqrt{R^2 - s^2} - s^2 \mathop{\mathrm{arcosh}}(R/s)$, and its share beyond a distance $x$ is the closed form, ½ at the edge and 0 at one radius (photonoxide's derivation in topology::robust); the conic filter of F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (2010), doi:10.1007/s00158-010-0602-y, Eqs. 4 and 5",
            run: robust_edge_2d,
        },
        Case {
            id: "topology/robust-length",
            title: r"The minimum width $b$ the nominal design keeps when the eroded design (threshold $1 - \eta$) just loses a bar, over the 1D hat filter's radius: bars of whole pixels at $R/\Delta = 100$, the width at ½ of the last bar erosion removes and of the first it keeps interpolated in their peaks to the threshold, for $\eta$ = 0.1 to 0.45 in steps of 0.05, against Eq. 29 (largest difference in $b/R$ shown)",
            tier: Tier::Published,
            source: r"X. Qian, O. Sigmund, Comput. Methods Appl. Mech. Engrg. 253, 237 (2013), doi:10.1016/j.cma.2012.08.020, Appendix, Eq. 29: $b/R = 2 - 2\sqrt{\eta}$ for $\eta \le 1/4$ and $2\sqrt{1/2 - \eta}$ above; Wang et al. 2010's Fig. 12 and Section 6 (1.1055 at $\eta = 0.2$, labelled $b/2R$ there and read as $b/R$ by Qian & Sigmund's Table 1)",
            run: robust_length,
        },
        Case {
            id: "topology/robust-epigraph",
            title: r"The epigraph form of the robust problem, min over the densities of the worst of the eroded, nominal and dilated designs' $-P$ (thresholds 0.75, 0.5 and 0.25 at $\beta = 8$, the 2D FDFD problem of topology/fdfd-gradient): its constraints' Jacobian in the densities, each realization's adjoint gradient pulled back through its own projection and the filter's transpose, against fourth-order differences of that realization's power ($\delta = 4 \times 10^{-3}$) at a corner, an edge and the middle; its column in $t$ and its constraints exact (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"Wang et al. 2010, Eq. 14 and Section 5.1 (the three realizations); the epigraph form of a min-max, $\min t$ subject to $f_k(x) - t \le 0$; the adjoint variable method, G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2–4",
            run: robust_epigraph,
        },
    ]
}

/// A case whose expected value is 0: its error is the value.
fn zero(error: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance,
        error,
    }
}

const BETAS: [f64; 4] = [1.0, 8.0, 64.0, 256.0];
const ETAS: [f64; 3] = [0.25, 0.5, 0.75];

fn points() -> impl Iterator<Item = f64> {
    (0..=200).map(|k| k as f64 / 200.0)
}

/// The projection's value at a complex argument, Eq. 9 with complex tanh.
fn projection_c(p: &Projection, z: c64) -> c64 {
    let (b, e) = (p.beta(), p.eta());
    let denominator = libm::tanh(b * e) + libm::tanh(b * (1.0 - e));
    (libm::tanh(b * e) + (b * (z - e)).tanh()) / denominator
}

/// The largest difference between the analytic derivative and the complex step's, relative to
/// the largest derivative, over every β, η and point.
pub(crate) fn projection_derivative_error() -> f64 {
    let h = 1e-30;
    let mut worst = 0.0f64;
    for beta in BETAS {
        for eta in ETAS {
            let p = Projection::new(beta, eta).expect("a valid projection");
            let top = points().map(|x| p.derivative(x)).fold(0.0, f64::max);
            for x in points() {
                let step = projection_c(&p, c64::new(x, h)).im / h;
                worst = worst.max((p.derivative(x) - step).abs() / top);
            }
        }
    }
    worst
}

fn projection_derivative() -> Outcome {
    zero(projection_derivative_error(), 1e-12)
}

/// The largest departure from ρ̂(0) = 0, ρ̂(1) = 1 and, at η = ½, ρ̂(1 − x) = 1 − ρ̂(x).
pub(crate) fn projection_identities_error() -> f64 {
    let mut worst = 0.0f64;
    for beta in BETAS {
        for eta in ETAS {
            let p = Projection::new(beta, eta).expect("a valid projection");
            worst = worst
                .max(p.value(0.0).abs())
                .max((p.value(1.0) - 1.0).abs());
        }
        let p = Projection::new(beta, 0.5).expect("a valid projection");
        for x in points() {
            worst = worst.max((p.value(1.0 - x) - (1.0 - p.value(x))).abs());
        }
    }
    worst
}

fn projection_identities() -> Outcome {
    zero(projection_identities_error(), 1e-14)
}

/// A deterministic pseudo-random density in [0, 1] (a linear congruential sequence).
pub(crate) fn draws(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (s >> 11) as f64 / (1u64 << 53) as f64
        })
        .collect()
}

fn regions() -> Vec<ConicFilter> {
    let mut filters = Vec::new();
    for edge in [Edge::Renormalized, Edge::Padded(1.0)] {
        filters.push(ConicFilter::new([160, 160, 1], 0.01, 0.055, edge).expect("a filter"));
        filters.push(ConicFilter::new([24, 20, 16], 0.02, 0.05, edge).expect("a filter"));
    }
    filters
}

/// The largest relative difference between ⟨F x, y⟩ and ⟨x, Fᵀ y⟩ (F's linear part: with
/// padding, F x − F 0).
pub(crate) fn filter_transpose_error() -> f64 {
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    regions()
        .iter()
        .enumerate()
        .map(|(k, f)| {
            let n: usize = f.shape().iter().product();
            let (x, y) = (draws(n, 2 * k as u64 + 1), draws(n, 2 * k as u64 + 2));
            let zero = f.apply(&vec![0.0; n]).expect("filtered");
            let fx: Vec<f64> = f
                .apply(&x)
                .expect("filtered")
                .iter()
                .zip(&zero)
                .map(|(a, b)| a - b)
                .collect();
            let left = dot(&fx, &y);
            let right = dot(&x, &f.transpose(&y).expect("transposed"));
            (left - right).abs() / left.abs()
        })
        .fold(0.0, f64::max)
}

fn filter_transpose() -> Outcome {
    zero(filter_transpose_error(), 1e-12)
}

/// The largest departure from a uniform density and from a linear ramp away from the edges.
pub(crate) fn filter_linear_error() -> f64 {
    let mut worst = 0.0f64;
    for (shape, pixel, radius) in [([60, 50, 1], 0.01, 0.055), ([24, 20, 16], 0.02, 0.05)] {
        let n: usize = shape.iter().product();
        let c = 0.37;
        for edge in [Edge::Renormalized, Edge::Padded(c)] {
            let f = ConicFilter::new(shape, pixel, radius, edge).expect("a filter");
            for v in f.apply(&vec![c; n]).expect("filtered") {
                worst = worst.max((v - c).abs());
            }
        }
        let f = ConicFilter::new(shape, pixel, radius, Edge::Renormalized).expect("a filter");
        let reach = (radius / pixel).ceil() as usize;
        let ramp = |p: usize| {
            let (i, j) = (p % shape[0], (p / shape[0]) % shape[1]);
            0.2 + 0.003 * i as f64 + 0.002 * j as f64
        };
        let x: Vec<f64> = (0..n).map(ramp).collect();
        for (p, v) in f.apply(&x).expect("filtered").into_iter().enumerate() {
            let at = [
                p % shape[0],
                (p / shape[0]) % shape[1],
                p / (shape[0] * shape[1]),
            ];
            let inside =
                (0..3).all(|a| shape[a] == 1 || (at[a] >= reach && at[a] + reach < shape[a]));
            if inside {
                worst = worst.max((v - ramp(p)).abs());
            }
        }
    }
    worst
}

fn filter_preserves_linear() -> Outcome {
    zero(filter_linear_error(), 1e-14)
}

/// The chain's problem: a 220 nm silicon slab (3.476 in 1.444) along x on a 20 nm grid between
/// two ports, a 10 × 10 design region (x from 0.9 to 1.1 µm, y from 0.12 to 0.32 µm) of silicon
/// and oxide beside it, filtered over 3 pixels and projected at β = 8: the solver at the start
/// densities, the background's cells, the region's placement and density, and the densities.
pub(crate) struct Chain {
    pub(crate) density: Density,
    pub(crate) placement: Placement,
    pub(crate) cells: Vec<c64>,
    pub(crate) rho: Vec<f64>,
    pub(crate) grid: Grid,
}

const LAMBDA_UM: f64 = 1.55;

pub(crate) fn chain() -> Chain {
    let (h, pml) = (0.02, 20);
    let grid = Grid {
        nx: (2.0 / h) as usize + 2 * pml,
        ny: (3.0 / h) as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let (core, clad) = (3.476f64.powi(2), 1.444f64.powi(2));
    let cells: Vec<c64> = (0..grid.ny)
        .flat_map(|j| (0..grid.nx).map(move |_| j))
        .map(|j| c64::new(if grid.y(j).abs() < 0.11 { core } else { clad }, 0.0))
        .collect();
    let index = |v: f64, v0: f64| ((v - v0) / h).round() as usize;
    let placement = Placement {
        origin: [index(0.9, grid.x0), index(0.12, grid.y0), 0],
        shape: [10, 10, 1],
        grid: [grid.nx, grid.ny, 1],
    };
    let density = Density::new([10, 10, 1], h, [clad, core])
        .and_then(|d| d.with_filter(3.0 * h, Edge::Renormalized))
        .map(|d| d.with_projection(Projection::new(8.0, 0.5).expect("a projection")))
        .expect("a density");
    // a varied start: values across the projection's steep middle
    let rho = (0..100)
        .map(|p| {
            let (i, j) = ((p % 10) as f64, (p / 10) as f64);
            0.5 + 0.3 * (0.7 * i).sin() * (0.45 * j + 0.3).cos()
        })
        .collect();
    Chain {
        density,
        placement,
        cells,
        rho,
        grid,
    }
}

impl Chain {
    fn cells_at(&self, density: &Density, rho: &[f64]) -> Vec<c64> {
        let mut cells = self.cells.clone();
        let eps = density.permittivity(rho).expect("a permittivity");
        self.placement.embed(&eps, &mut cells).expect("placed");
        cells
    }

    /// The probes at pixels `pixels`, the densities' differences `delta` wide.
    pub(crate) fn probes(&self, pixels: &[usize], delta: f64) -> Vec<Probe> {
        self.probes_of(&self.density, pixels, delta).1
    }

    /// With `density` in place of the chain's own (a robust realization): the power and its
    /// gradient in every density, and the probes at `pixels`, the densities' differences `delta`
    /// wide.
    pub(crate) fn probes_of(
        &self,
        density: &Density,
        pixels: &[usize],
        delta: f64,
    ) -> (Vec<f64>, Vec<Probe>) {
        let lam = Wavelength::um(LAMBDA_UM).expect("a wavelength");
        let base = Solver2d::from_cells(
            self.grid,
            Polarization::Ez,
            lam,
            &self.cells_at(density, &self.rho),
            Boundaries::pml(20),
        )
        .expect("a problem");
        let ports = two_ports(&base);
        let source = base.mode_source(&ports[0].mode, Direction::Forward);
        let power = |solver: &Solver2d| {
            let field = solver.solve_system(&source).expect("solved");
            field.mode_amplitudes(&ports[1].mode).0.norm_sqr()
        };
        let field = base.solve_system(&source).expect("solved");
        let (base_power, per_cell) = base
            .mode_power_gradient(&field, &ports[1].mode, Direction::Forward)
            .expect("a gradient");
        let d_eps = self.placement.restrict(&per_cell).expect("restricted");
        let gradient = density.gradient(&self.rho, &d_eps).expect("pulled back");
        let residual = base.residual(&field, &source).expect("a residual");
        let probes = pixels
            .iter()
            .map(|&p| {
                let at = |d: f64| {
                    let mut rho = self.rho.clone();
                    rho[p] += d;
                    power(
                        &base
                            .reuse_cells(lam, &self.cells_at(density, &rho))
                            .expect("a problem"),
                    )
                };
                Probe {
                    gradient: gradient[p],
                    power: base_power,
                    delta,
                    at: [-2.0, -1.0, 1.0, 2.0].map(|s| at(s * delta)),
                    residual,
                }
            })
            .collect();
        (gradient, probes)
    }
}

/// The probes' pixels: a corner, an edge and the middle.
pub(crate) const PIXELS: [usize; 3] = [0, 5, 55];

fn fdfd_gradient() -> Outcome {
    let probes = chain().probes(&PIXELS, 2e-3);
    crate::verdict::note_numerical(probes.iter().map(Probe::resolution).fold(0.0, f64::max));
    zero(
        probes
            .iter()
            .map(|p| (p.gradient - p.differences()).abs() / p.gradient.abs())
            .fold(0.0, f64::max),
        1e-8,
    )
}

/// The robust η the length case runs.
pub(crate) const ETAS_ROBUST: [f64; 8] = [0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45];

/// A solid half-space (the pixels left of a pixel boundary) filtered by the conic filter of
/// `r_px` pixels in `dims` dimensions: the middle row's filtered densities, and each pixel
/// centre's distance from the edge in radii, within a radius of the edge.
fn across_edge(dims: usize, r_px: f64) -> Vec<(f64, f64)> {
    let margin = (r_px.ceil() as usize) + 2;
    let nx = 4 * margin;
    let ny = if dims == 2 { 2 * margin + 1 } else { 1 };
    let edge = 2 * margin;
    let filter = ConicFilter::new([nx, ny, 1], 1.0, r_px, Edge::Renormalized).expect("a filter");
    let rho: Vec<f64> = (0..nx * ny)
        .map(|p| if p % nx < edge { 1.0 } else { 0.0 })
        .collect();
    let f = filter.apply(&rho).expect("filtered");
    let row = ny / 2;
    (0..nx)
        .map(|i| ((i as f64 + 0.5 - edge as f64) / r_px, f[row * nx + i]))
        .filter(|(t, _)| t.abs() < 1.0)
        .collect()
}

/// The largest difference between the discrete filter's density across an edge and the closed
/// form, in `dims` dimensions at `r_px` pixels per radius.
pub(crate) fn edge_values_error(dims: usize, r_px: f64) -> f64 {
    let closed = if dims == 1 {
        across_edge_1d
    } else {
        across_edge_2d
    };
    across_edge(dims, r_px)
        .into_iter()
        .map(|(t, v)| (v - closed(t)).abs())
        .fold(0.0, f64::max)
}

/// The 1D filter's density across an edge against the closed form less its exact pixel offset,
/// (Δ/R)²/8 below it outside the edge and above it inside.
fn edge_1d_exact_error() -> f64 {
    let mut worst = 0.0f64;
    for r_px in [5.0, 20.0, 80.0] {
        let offset = 1.0 / (8.0 * r_px * r_px);
        for (t, v) in across_edge(1, r_px) {
            worst = worst.max((v - (across_edge_1d(t) - offset * t.signum())).abs());
        }
    }
    worst
}

fn robust_edge_1d() -> Outcome {
    zero(edge_1d_exact_error(), 1e-14)
}

fn robust_edge_2d() -> Outcome {
    let r_px = 40.0;
    zero(edge_values_error(2, r_px), 1.0 / (8.0 * r_px * r_px))
}

/// b/R for a robust η on the 1D hat filter of `r_px` pixels: bars of whole pixels, the width at ½
/// (by linear interpolation between pixel centres) of the last bar the eroded threshold 1 − η
/// removes and of the first it keeps, interpolated in their peaks to the threshold itself.
pub(crate) fn bar_length(r_px: f64, eta: f64) -> f64 {
    let n = (6.0 * r_px) as usize;
    let filter = ConicFilter::new([n, 1, 1], 1.0, r_px, Edge::Renormalized).expect("a filter");
    // a bar of h pixels: its filtered peak and its width at ½, in pixels
    let bar = |h: usize| -> (f64, f64) {
        let start = n / 2 - h / 2;
        let rho: Vec<f64> = (0..n)
            .map(|i| {
                if (start..start + h).contains(&i) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        let f = filter.apply(&rho).expect("filtered");
        let peak = f.iter().copied().fold(0.0, f64::max);
        if peak < 0.5 {
            return (peak, 0.0);
        }
        let cross = |i: usize, j: usize| {
            i as f64 + 0.5 + (f[i] - 0.5) / (f[i] - f[j]) * (j as f64 - i as f64)
        };
        let left = (1..n)
            .find(|&i| f[i - 1] < 0.5 && f[i] >= 0.5)
            .expect("a left edge");
        let right = (0..n - 1)
            .rev()
            .find(|&i| f[i + 1] < 0.5 && f[i] >= 0.5)
            .expect("a right edge");
        (peak, cross(right, right + 1) - cross(left, left - 1))
    };
    let threshold = 1.0 - eta;
    let mut last = bar(1);
    for h in 2..n / 2 {
        let next = bar(h);
        if next.0 >= threshold {
            let ((p1, b1), (p2, b2)) = (last, next);
            return (b1 + (b2 - b1) * (threshold - p1) / (p2 - p1)) / r_px;
        }
        last = next;
    }
    f64::NAN
}

fn robust_length() -> Outcome {
    let r_px = 100.0;
    let worst = ETAS_ROBUST
        .iter()
        .map(|&eta| (bar_length(r_px, eta) - minimum_length(eta).expect("an η")).abs())
        .fold(0.0, f64::max);
    zero(worst, 4.0 / (r_px * r_px))
}

/// The epigraph's Jacobian against differences: the largest relative difference, and the
/// differences' resolution (noted).
pub(crate) fn robust_epigraph_error(delta: f64) -> (f64, f64) {
    let c = chain();
    let realizations = c.density.robust(0.25).expect("a projection");
    let n = c.rho.len();
    let (mut values, mut gradients, mut probes) = (Vec::new(), Vec::new(), Vec::new());
    for density in realizations.all() {
        let (gradient, p) = c.probes_of(density, &PIXELS, delta);
        // the robust problem maximizes the worst power: f = −P
        values.push(-p[0].power);
        gradients.extend(gradient.iter().map(|g| -g));
        probes.push(p);
    }
    let form = Epigraph::new(n, 3).expect("a form");
    let t = Epigraph::start(&values);
    let mut y = c.rho.clone();
    y.push(t);
    let (mut gradient, mut g, mut jacobian) =
        (vec![0.0; n + 1], vec![0.0; 3], vec![0.0; 3 * (n + 1)]);
    let objective = form
        .evaluate(
            &y,
            &values,
            &gradients,
            &mut gradient,
            &mut g,
            &mut jacobian,
        )
        .expect("evaluated");
    // the exact parts: the objective t, its gradient e_t, g = f − t and the t column −1
    let mut exact = (objective - t).abs() + (gradient[n] - 1.0).abs();
    exact += gradient[..n].iter().map(|v| v.abs()).sum::<f64>();
    for (k, &v) in values.iter().enumerate() {
        exact += (g[k] - (v - t)).abs() + (jacobian[k * (n + 1) + n] + 1.0).abs();
    }
    let mut worst = if exact == 0.0 { 0.0 } else { f64::INFINITY };
    let mut resolution = 0.0f64;
    for (k, ps) in probes.iter().enumerate() {
        for (probe, &p) in ps.iter().zip(&PIXELS) {
            let row = jacobian[k * (n + 1) + p];
            // the row is −dP/dρ: against −(the differences of P)
            worst = worst.max((row + probe.differences()).abs() / row.abs());
            resolution = resolution.max(probe.resolution());
        }
    }
    (worst, resolution)
}

fn robust_epigraph() -> Outcome {
    let (worst, resolution) = robust_epigraph_error(4e-3);
    crate::verdict::note_numerical(resolution);
    zero(worst, 3e-7)
}
