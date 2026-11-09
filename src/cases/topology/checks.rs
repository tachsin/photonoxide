//! The validation cases of densities ([`super`]): the projection's derivative and identities,
//! the filter's transpose and what it reproduces exactly, and the whole chain's gradient from 2D
//! FDFD's adjoint against finite differences.

use num_complex::Complex64 as c64;

use super::{ConicFilter, Density, Edge, Placement, Projection};
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
            source: r"F. Wang, B. S. Lazarov, O. Sigmund, Struct. Multidiscip. Optim. 43, 767 (2010), doi:10.1007/s00158-010-0602-y, Eq. 9; the complex step, $f'(x) = \operatorname{Im} f(x + ih)/h$, is exact to round-off for an analytic $f$ (no difference is taken)",
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
            title: r"2D FDFD, $E$ along $z$: the gradient of a port mode's power with respect to a 10 × 10 design region's densities (conic filter of 3 pixels, projection at $\beta = 8$, $\eta = 1/2$, silicon in oxide beside a 220 nm slab, 20 nm grid), from the adjoint pulled back through the projection and the filter's transpose, against fourth-order central differences of the densities ($\delta = 10^{-3}$) at a corner, an edge and the middle (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2–4, through Wang et al. 2010's chain rule, Eq. 7",
            run: fdfd_gradient,
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
    fn cells_at(&self, rho: &[f64]) -> Vec<c64> {
        let mut cells = self.cells.clone();
        let eps = self.density.permittivity(rho).expect("a permittivity");
        self.placement.embed(&eps, &mut cells).expect("placed");
        cells
    }

    /// The probes at pixels `pixels`, the densities' differences `delta` wide.
    pub(crate) fn probes(&self, pixels: &[usize], delta: f64) -> Vec<Probe> {
        let lam = Wavelength::um(LAMBDA_UM).expect("a wavelength");
        let base = Solver2d::from_cells(
            self.grid,
            Polarization::Ez,
            lam,
            &self.cells_at(&self.rho),
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
        let gradient = self
            .density
            .gradient(&self.rho, &d_eps)
            .expect("pulled back");
        let residual = base.residual(&field, &source).expect("a residual");
        pixels
            .iter()
            .map(|&p| {
                let at = |d: f64| {
                    let mut rho = self.rho.clone();
                    rho[p] += d;
                    power(
                        &base
                            .reuse_cells(lam, &self.cells_at(&rho))
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
            .collect()
    }
}

/// The probes' pixels: a corner, an edge and the middle.
pub(crate) const PIXELS: [usize; 3] = [0, 5, 55];

fn fdfd_gradient() -> Outcome {
    let probes = chain().probes(&PIXELS, 1e-3);
    crate::verdict::note_numerical(probes.iter().map(Probe::resolution).fold(0.0, f64::max));
    zero(
        probes
            .iter()
            .map(|p| (p.gradient - p.differences()).abs() / p.gradient.abs())
            .fold(0.0, f64::max),
        1e-6,
    )
}
