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
            source: r"the hat's share of weight beyond a distance $x$, $\int_x^R (R - s) ds / R^2$; on pixels, the sum $\sum_{k \gt i} (R - k)$ over whole pixels is the integral less $\Delta^2/8$, exactly, when $R/\Delta$ is whole (the discrete midpoint rule on a linear function cut at a pixel boundary)",
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
        Case {
            id: "topology/ruler-shapes",
            title: r"The a-posteriori minimum length scale (morphological opening and closing by a disc, bisection to half a pixel) of shapes whose length is known, on 200 × 200 pixels: a solid bar 20 pixels wide in void, a void gap 30 pixels wide between solids, and a solid ring 15 pixels wide (radii 40 and 55) (largest difference from the shape's own, in pixels, shown)",
            tier: Tier::Analytic,
            source: r"M. Chen et al., J. Opt. Soc. Am. B 41, A161 (2024), doi:10.1364/JOSAB.506412, Section 2 and Eqs. 1–3: the minimum length is the smallest diameter at which the opening and closing differ inside the solid or the void; a straight bar or gap of width $w$ has $w$, and a ring its width",
            run: ruler_shapes_case,
        },
        Case {
            id: "topology/ruler-testbed",
            title: r"The same minimum length scale of eight of the testbed's waveguide mode converter designs (Ceviche's circle-brush designs, 6 to 20 pixel brushes), each placed in its 350 × 300 domain of 10 nm pixels with the two waveguides, against the lengths the testbed's README prints, from 90.2978515625 to 207.0947265625 nm (largest difference shown, nm)",
            tier: Tier::Published,
            source: r"NanoComp/photonics-opt-testbed, waveguide_mode_converter/README.md (commit a06518872de6, written on 2023-05-11 with imageruler's minimum_length on the model's density and its domain's size), the designs under the MIT licence (src/cases/topology/testbed/); Chen et al. 2024, Section 3.C",
            run: ruler_testbed_case,
        },
        Case {
            id: "topology/constraint-thresholds",
            title: r"The eroded and dilated thresholds that keep a minimum linewidth and spacing with a conic filter of radius $R$ (Hammond et al.'s Eqs. 14 and 15) against the minimum length the robust thresholds impose (Qian & Sigmund's Eq. 29, topology/robust-length): at 97 values of $\eta$ from 0.01 to 0.49, the threshold of the length $b(\eta)$ is $1 - \eta$ and $\eta$ (largest difference shown)",
            tier: Tier::Analytic,
            source: r"A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, Opt. Express 29, 23916 (2021), doi:10.1364/OE.431188, Eqs. 14 and 15; X. Qian, O. Sigmund 2013, Eq. 30 (the inverse of their Eq. 29), exact",
            run: constraint_thresholds_case,
        },
        Case {
            id: "topology/constraint-gradient",
            title: r"The minimum linewidth and spacing constraints (Zhou et al.'s Eqs. 10 and 11, indicators $\bar\rho  e^{-c\lvert\nabla\tilde\rho\rvert^2}$ and $(1 - \bar\rho)  e^{-c\lvert\nabla\tilde\rho\rvert^2}$, $c = (R/\Delta)^4$) on a 24 × 20 region (20 nm pixels, a conic filter of 3 pixels, $\beta = 8$, 90 nm lines and gaps, so $\eta_e = 0.9375$ and $\eta_d = 0.0625$): their gradients through the min, the indicator's exponential, the projection and the filter's transpose, against fourth-order differences of the densities ($\delta = 10^{-3}$) at five pixels, corners and edges among them (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"M. Zhou, B. S. Lazarov, F. Wang, O. Sigmund, Comput. Methods Appl. Mech. Engrg. 293, 266 (2015), doi:10.1016/j.cma.2015.05.003, Eqs. 8–11 and Section 2.2; Hammond et al. 2021, Eqs. 10–13",
            run: constraint_gradient_case,
        },
        Case {
            id: "topology/constraint-arrieta",
            title: r"The solid constraint at a strip exactly as wide as the target length, Arrieta et al.'s choices for a conic filter ($R = \ell_t$, $\eta_e = 3/4$, $c = 64R^2$ with $\nabla\tilde\rho$ in µm⁻¹, a step projection): a 1D bar of whole pixels $R$ wide in a region $6R$ long, summed over pixels at $R/\Delta = 200$, against their Eq. 18, $\epsilon = 2.113 \times 10^{-7}$ at this share $R/\lvert\Omega\rvert = 1/6$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"R. Arrieta, G. Romano, S. G. Johnson, Struct. Multidiscip. Optim. 69, 210 (2026), doi:10.1007/s00158-026-04388-6, Section 4.1, Eqs. 14–19 and Table 1: across the strip $\tilde\rho = 3/4 - \tilde x^2$, and $\epsilon$ is $\int \tilde x^4 e^{-4\tilde c \tilde x^2} d\tilde x$ over $\lvert\tilde x\rvert \le 1/2$, in closed form with erf (Eq. 18)",
            run: constraint_arrieta_case,
        },
        Case {
            id: "topology/helmholtz-residual",
            title: r"The Helmholtz filter by the cosine transform solves its own discrete equation, $(1 - r^2\Delta_h)\tilde\rho = \rho$ with the region's edges mirrored, on 48 × 36 and 14 × 12 × 10 pixels of varied densities (largest residual shown)",
            tier: Tier::Analytic,
            source: r"B. S. Lazarov, O. Sigmund, Int. J. Numer. Methods Eng. 86, 765 (published online 2010), doi:10.1002/nme.3072, Eqs. 11 and 12; the cell-centred second difference with mirrored edges is diagonal in the DCT-II, its eigenvalues $-4\sin^2(\pi k/2n)/\Delta^2$",
            run: helmholtz_residual_case,
        },
        Case {
            id: "topology/helmholtz-edge",
            title: r"The Helmholtz filter (radius 50 nm, so $r = 14.43$ nm) across a straight edge of a solid half-space, on 1.25 nm pixels: where its density crosses $\eta = 0.3$, the dilated threshold of Christiansen & Sigmund's demultiplexer, against $r \ln(1/2\eta) = 7.373$ nm (difference shown, nm)",
            tier: Tier::Analytic,
            source: r"the 1D Green's function of $-r^2\tilde\rho'' + \tilde\rho = \rho$, $e^{-\lvert x\rvert/r}/2r$, integrated over a half-space: $\tilde\rho = \frac12 e^{-x/r}$ outside; Lazarov & Sigmund's Eq. 34 for $r = R/2\sqrt3$; R. E. Christiansen, O. Sigmund, J. Opt. Soc. Am. B 38, 496 (2021), doi:10.1364/JOSAB.406048, Section 9: the ±8 nm the thresholds 0.3 and 0.7 impose",
            run: helmholtz_edge_case,
        },
        Case {
            id: "topology/double-filter-gradient",
            title: r"Christiansen & Sigmund's double filtering (Helmholtz filters of 100 and 50 nm, projections at $\beta$ = 5 and 2.5, the second at $\eta$ = 0.5 and 0.3) on a 30 × 24 region of 10 nm pixels: the gradient of $F = \sum_i w_i \hat\rho_i$ (fixed varied weights) back through both projections and both filters, against fourth-order differences ($\delta = 10^{-3}$) at five pixels (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"Christiansen & Sigmund 2021, Eq. 22; the chain rule through each stage, each filter its own transpose (symmetric)",
            run: double_filter_case,
        },
        Case {
            id: "topology/area-shapes",
            title: r"Hammond et al.'s minimum-area and minimum-enclosed-area constraints on binary shapes in a 1 µm² region of 10 nm pixels, the minimums 0.08 and 0.2 µm²: a square island of 0.04 µm² (its $g_A$ is its area's share, 0.04), one of 0.16 µm² (0), a square hole of 0.09 µm² in solid (its $g_{EA}$, 0.09) and an island of 0.04 µm² touching the region's edge (0, it continues outside) (largest difference shown)",
            tier: Tier::Analytic,
            source: r"A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, Opt. Express 29, 23916 (2021), doi:10.1364/OE.431188, Section 4, Eqs. 21 and 22: on a binary pattern the dilated ring around a region adds nothing ($\bar\rho = 0$ outside an island, 1 outside a hole), so each value is the small regions' area",
            run: area_shapes_case,
        },
        Case {
            id: "topology/ssp-plane",
            title: r"Hammond et al. 2025's subpixel-smoothed projection across a straight interface: their fill (d/\hat R)20 20 102 502 12 61 79 80 81 98 503 701 33 100 204 250 395 398 399 400Eq. 10) against the parabolic kernel's share of a half-space at 201 distances across hat R and the projection at eta = \infty a linear 	ilde\rho20 20 102 502 12 61 79 80 81 98 503 701 33 100 204 250 395 398 399 400a ramp at 17° on 40 × 30 pixels of 10 nm) against  each pixel's exact distance to its level set (largest difference shown)",
            tier: Tier::Analytic,
            source: r"A. M. Hammond, A. Oskooi, I. M. Hammond, M. Chen, S. E. Ralph, S. G. Johnson, Opt. Express 33, 33620 (2025), doi:10.1364/OE.563512, Eqs. 7, 9–12 and 23: the kernel hat k_2 \propto 1 - (r/\hat R)^2 along the third axis is (\hat R^2 - \rho^2)^{3/2}/2\pi\hat R^5 the plane, integrated over the half-space by quadrature; a linear 	ilde\rho its gradient exactly by differences, so  exact",
            run: ssp_plane_case,
        },
        Case {
            id: "topology/ssp-disc",
            title: r"The subpixel-smoothed projection at eta = \infty a curved interface: a disc of radius 60 0.25 µm, 	ilde\rho cone about its centre, on 5 nm pixels (hat R60 2.75 nm); each pixel within hat R the circle against the kernel's share of the disc there, the exact smoothed disc (largest difference shown)",
            tier: Tier::Analytic,
            source: r"Hammond et al. 2025, Eq. 6: the fill takes the interface as a plane across the smoothing sphere, an error first order in hat R/a a curve; the exact smoothed disc by quadrature of the kernel over arcs of the circle",
            run: ssp_disc_case,
        },
        Case {
            id: "topology/ssp-gradient",
            title: r"The subpixel-smoothed projection's gradient through a conic filter of 50 nm on a 30 × 24 region of 10 nm pixels, at eta60 64 and infty 
- of  = \sum_i w_i \hat{\tilde\rho}_i20 20 102 502 12 61 79 80 81 98 503 701 33 100 204 250 395 398 399 400fixed varied weights) against fourth-order differences (delta = 10^{-4}20 20 102 502 12 61 79 80 81 98 503 701 33 100 204 250 395 398 399 400 at every seventh pixel (largest difference relative to the largest derivative, shown)",
            tier: Tier::Analytic,
            source: r"Hammond et al. 2025, Eqs. 7, 10–12: the chain rule through each pixel's value and the differences of its gradient;  twice differentiable, so the projection is too, even at eta = \infty from changes in topology",
            run: ssp_gradient_case,
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

/// The testbed's waveguide mode converter designs the length case measures, with the minimum
/// length scale its README prints for each, nm (NanoComp/photonics-opt-testbed, MIT licence, at
/// commit a06518872de6ec82bb4511190e544211539f55f1; testbed/README.md).
pub(crate) const TESTBED: [(&str, &str, f64); 8] = [
    (
        "converter_generator_circle_10_x47530832_w11_s483",
        include_str!("testbed/converter_generator_circle_10_x47530832_w11_s483.hex"),
        90.2978515625,
    ),
    (
        "converter_generator_circle_10_x47530832_w27_s681",
        include_str!("testbed/converter_generator_circle_10_x47530832_w27_s681.hex"),
        101.9775390625,
    ),
    (
        "converter_generator_circle_12_x47530832_w12_s248",
        include_str!("testbed/converter_generator_circle_12_x47530832_w12_s248.hex"),
        119.4970703125,
    ),
    (
        "converter_generator_circle_14_x47530832_w13_s793",
        include_str!("testbed/converter_generator_circle_14_x47530832_w13_s793.hex"),
        142.8564453125,
    ),
    (
        "converter_generator_circle_16_x47530832_w14_s150",
        include_str!("testbed/converter_generator_circle_16_x47530832_w14_s150.hex"),
        166.2158203125,
    ),
    (
        "converter_generator_circle_18_x47530832_w15_s879",
        include_str!("testbed/converter_generator_circle_18_x47530832_w15_s879.hex"),
        169.1357421875,
    ),
    (
        "converter_generator_circle_20_x47530832_w16_s416",
        include_str!("testbed/converter_generator_circle_20_x47530832_w16_s416.hex"),
        204.1748046875,
    ),
    (
        "converter_generator_circle_20_x47530832_w40_s988",
        include_str!("testbed/converter_generator_circle_20_x47530832_w40_s988.hex"),
        207.0947265625,
    ),
];

/// A testbed design's 160 × 160 pixels from its rows of hex digits (each row a design row along
/// x, its first value the first digit's highest bit), placed as the testbed's model places it
/// in its 350 × 300 domain at 10 nm (ceviche-challenges' mode_converter_spec_12 with the
/// testbed's sizes): 20 pixels of PML, 75-pixel waveguides 40 pixels wide on y = 130 to 169 on
/// either side (the right one from x = 254), the design over x = 95 to 254 and y = 70 to 229.
pub(crate) fn testbed_domain(hex: &str) -> crate::topology::ruler::Pattern {
    let (ni, nj) = (350usize, 300usize);
    let mut solid = vec![false; ni * nj];
    for i in 0..ni {
        for j in 130..170 {
            if i < 95 || i >= ni - 96 {
                solid[j * ni + i] = true;
            }
        }
    }
    for (row, line) in hex.lines().enumerate() {
        for (k, c) in line.trim().chars().enumerate() {
            let nibble = c.to_digit(16).expect("a hex digit");
            for q in 0..4 {
                let col = 4 * k + q;
                solid[(70 + col) * ni + 95 + row] = nibble & (8 >> q) != 0;
            }
        }
    }
    crate::topology::ruler::Pattern::new([ni, nj], solid).expect("a pattern")
}

/// Each testbed design's measured minimum length against its printed one, nm.
pub(crate) fn testbed_lengths() -> Vec<(&'static str, f64, f64)> {
    TESTBED
        .iter()
        .map(|&(name, hex, printed)| {
            let measured =
                crate::topology::ruler::minimum_length(&testbed_domain(hex), [10.0, 10.0])
                    .expect("a length");
            (name, measured, printed)
        })
        .collect()
}

/// A shape: its name, whether a point (x, y) is solid, and its own minimum length in pixels.
type Shape = (&'static str, Box<dyn Fn(f64, f64) -> bool>, f64);

/// Shapes whose minimum length is known: a solid bar 20 pixels wide across a void image, a void
/// gap 30 pixels wide between solids, and a solid ring 15 pixels wide (radii 40 and 55) on a
/// 200 × 200 image of 1-pixel pixels: each shape's measured length and its own.
pub(crate) fn ruler_shapes() -> Vec<(&'static str, f64, f64)> {
    let n = 200usize;
    let pattern = |f: &dyn Fn(f64, f64) -> bool| {
        let solid = (0..n * n)
            .map(|p| f((p % n) as f64 + 0.5, (p / n) as f64 + 0.5))
            .collect();
        crate::topology::ruler::Pattern::new([n, n], solid).expect("a pattern")
    };
    let c = n as f64 / 2.0;
    let shapes: [Shape; 3] = [
        ("bar", Box::new(move |x, _| (x - c).abs() < 10.0), 20.0),
        ("gap", Box::new(move |x, _| (x - c).abs() >= 15.0), 30.0),
        (
            "ring",
            Box::new(move |x, y| {
                let r = ((x - c).powi(2) + (y - c).powi(2)).sqrt();
                (40.0..55.0).contains(&r)
            }),
            15.0,
        ),
    ];
    shapes
        .iter()
        .map(|(name, f, want)| {
            let measured = crate::topology::ruler::minimum_length(&pattern(f.as_ref()), [1.0, 1.0])
                .expect("a length");
            (*name, measured, *want)
        })
        .collect()
}

fn ruler_shapes_case() -> Outcome {
    let worst = ruler_shapes()
        .iter()
        .map(|(_, measured, own)| (measured - own).abs())
        .fold(0.0, f64::max);
    zero(worst, 1.0)
}

fn ruler_testbed_case() -> Outcome {
    let worst = testbed_lengths()
        .iter()
        .map(|(_, measured, printed)| (measured - printed).abs())
        .fold(0.0, f64::max);
    zero(worst, 5e-11)
}

/// The constraints' problem: a 24 × 20 region of 20 nm pixels, a conic filter of 3 pixels and a
/// projection at β = 8, a minimum linewidth and spacing of 90 nm (1.5 R), and densities varied
/// across both phases with features narrower than that.
pub(crate) fn constraint_problem() -> (crate::topology::LengthConstraints, Vec<f64>) {
    let (ni, nj, h) = (24usize, 20usize, 0.02);
    let density = Density::new([ni, nj, 1], h, [2.085, 12.08])
        .and_then(|d| d.with_filter(3.0 * h, Edge::Renormalized))
        .map(|d| d.with_projection(Projection::new(8.0, 0.5).expect("a projection")))
        .expect("a density");
    let constraints =
        crate::topology::LengthConstraints::new(&density, 0.09, 0.09).expect("constraints");
    let rho = (0..ni * nj)
        .map(|p| {
            let (i, j) = ((p % ni) as f64, (p / ni) as f64);
            (0.5 + 0.45 * (0.9 * i).sin() * (0.7 * j + 0.4).cos()).clamp(0.0, 1.0)
        })
        .collect();
    (constraints, rho)
}

/// The pixels the constraints' gradient case probes: a corner, an edge, the middle and two
/// others.
pub(crate) const CONSTRAINT_PIXELS: [usize; 5] = [0, 11, 250, 263, 479];

/// The constraints' gradients against fourth-order differences `delta` wide: the largest relative
/// difference over both constraints at the probed pixels, and the differences' resolution there
/// (the values' round-off over δ, relative to the gradient).
pub(crate) fn constraint_gradient_error(delta: f64) -> (f64, f64) {
    let (c, rho) = constraint_problem();
    let base = c.evaluate(&rho).expect("evaluated");
    let (mut worst, mut resolution) = (0.0f64, 0.0f64);
    for &p in &CONSTRAINT_PIXELS {
        let at = |d: f64| {
            let mut r = rho.clone();
            r[p] += d;
            c.evaluate(&r).expect("evaluated").values
        };
        let [m2, m1, p1, p2] = [-2.0, -1.0, 1.0, 2.0].map(|s| at(s * delta));
        for s in 0..2 {
            let fd = (m2[s] - 8.0 * m1[s] + 8.0 * p1[s] - p2[s]) / (12.0 * delta);
            let g = base.gradients[s][p];
            let scale = g.abs().max(1e-300);
            worst = worst.max((g - fd).abs() / scale);
            // the four values' round-off through the stencil's weights, 18/12 of ε|g|/δ
            resolution = resolution.max(1.5 * f64::EPSILON * base.values[s].abs() / delta / scale);
        }
    }
    (worst, resolution)
}

/// Hammond et al.'s thresholds of Eq. 29's lengths against the robust thresholds themselves.
pub(crate) fn constraint_thresholds_error() -> f64 {
    let radius = 0.06;
    (1..=97)
        .map(|k| 0.01 + 0.005 * (k - 1) as f64)
        .map(|eta| {
            let b = minimum_length(eta).expect("an η") * radius;
            let e = crate::topology::eroded_threshold(b, radius).expect("a threshold");
            let d = crate::topology::dilated_threshold(b, radius).expect("a threshold");
            (e - (1.0 - eta)).abs().max((d - eta).abs())
        })
        .fold(0.0, f64::max)
}

fn constraint_thresholds_case() -> Outcome {
    zero(constraint_thresholds_error(), 1e-14)
}

fn constraint_arrieta_case() -> Outcome {
    let (g, want) = arrieta_strip(200.0);
    Outcome {
        measured: g,
        expected: want,
        tolerance: 100.0 / (200.0 * 200.0) * want,
        error: (g - want).abs(),
    }
}

fn constraint_gradient_case() -> Outcome {
    let (worst, resolution) = constraint_gradient_error(1e-3);
    crate::verdict::note_numerical(resolution);
    zero(worst, 3e-9)
}

/// The testbed's wavelengths for the mode converter, nm (its run.py: six over 1265–1295 nm).
#[cfg(test)]
pub(crate) const TESTBED_WAVELENGTHS_NM: [f64; 6] =
    [1265.0, 1270.0, 1275.0, 1285.0, 1290.0, 1295.0];

/// A testbed design's worst reflection and transmission, dB, on photonoxide's 2D FDFD at 10 nm
/// (E along z, 20 cells of PML, the domain of [`testbed_domain`], ε 2.25 and 12.25): the left
/// port's fundamental mode in, its reflection in the same mode, and the right port's second mode
/// out, each port a 190-cell window (1.9 µm, the model's) at columns 25 and 325.
#[cfg(test)]
pub(crate) fn testbed_spectrum(hex: &str) -> (f64, f64) {
    testbed_spectrum_at(hex, 1)
}

/// [`testbed_spectrum`] on a grid `scale` times finer: each 10 nm pixel `scale` × `scale` cells,
/// the PML, the ports' columns and their windows scaled with it.
#[cfg(test)]
pub(crate) fn testbed_spectrum_at(hex: &str, scale: usize) -> (f64, f64) {
    use crate::fdfd::{Port, Side};
    let pattern = testbed_domain(hex);
    let [pi, pj] = pattern.shape();
    let (ni, nj) = (pi * scale, pj * scale);
    let h = 0.01 / scale as f64;
    let grid = Grid {
        nx: ni,
        ny: nj,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
    };
    let cells: Vec<c64> = (0..ni * nj)
        .map(|p| {
            let solid = pattern.is_solid((p % ni) / scale, (p / ni) / scale);
            c64::new(if solid { 12.25 } else { 2.25 }, 0.0)
        })
        .collect();
    let (mut worst_r, mut worst_t) = (f64::NEG_INFINITY, f64::INFINITY);
    for nm in TESTBED_WAVELENGTHS_NM {
        let lam = Wavelength::um(nm * 1e-3).expect("a wavelength");
        let solver = Solver2d::from_cells(
            grid,
            Polarization::Ez,
            lam,
            &cells,
            Boundaries::pml(20 * scale),
        )
        .expect("a problem");
        let window = 55 * scale..245 * scale;
        let left = solver
            .port_modes_within(25 * scale, window.clone(), 1)
            .expect("modes")
            .remove(0);
        let right = solver
            .port_modes_within(325 * scale, window, 2)
            .expect("modes")
            .remove(1);
        let ports = [
            Port {
                mode: left,
                side: Side::Left,
            },
            Port {
                mode: right,
                side: Side::Right,
            },
        ];
        let s = solver.s_matrix(&ports).expect("an S-matrix");
        worst_r = worst_r.max(20.0 * s[0][0].norm().log10());
        worst_t = worst_t.min(20.0 * s[1][0].norm().log10());
    }
    (worst_r, worst_t)
}

/// Arrieta et al.'s critical strip in 1D: a bar of whole pixels R wide (`r_px` pixels) in the
/// middle of a region 6R long, the conic filter of radius R, a projection sharp enough to be a
/// step (β = 10⁶), the target length R (η_e = ¾) and their decay c = 64 R²: the solid constraint
/// as the pixels sum it, and their Eq. 18 for it (the share R/|Ω| = 1/6).
pub(crate) fn arrieta_strip(r_px: f64) -> (f64, f64) {
    let r = r_px as usize;
    let n = 6 * r;
    let h = 1e-3;
    let density = Density::new([n, 1, 1], h, [1.0, 2.0])
        .and_then(|d| d.with_filter(r_px * h, Edge::Renormalized))
        .map(|d| d.with_projection(Projection::new(1e6, 0.5).expect("a projection")))
        .expect("a density");
    let c = crate::topology::LengthConstraints::new(&density, r_px * h, r_px * h)
        .expect("constraints")
        .with_arrieta_decay();
    let start = (n - r) / 2;
    let rho: Vec<f64> = (0..n)
        .map(|i| {
            if (start..start + r).contains(&i) {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    let g = c.evaluate(&rho).expect("evaluated").values[0];
    let want = crate::topology::geometric_threshold(64.0, 1.0 / 6.0).expect("a threshold");
    (g, want)
}

/// The Helmholtz filter's residual in its own discrete equation, (1 − r²Δ_h) ρ̃ − ρ with the
/// edges mirrored, relative to the largest density, on 2D and 3D regions with varied densities.
pub(crate) fn helmholtz_residual() -> f64 {
    let mut worst = 0.0f64;
    for (shape, h, radius) in [([48usize, 36, 1], 0.01, 0.07), ([14, 12, 10], 0.02, 0.09)] {
        let f = crate::topology::HelmholtzFilter::new(shape, h, radius).expect("a filter");
        let [ni, nj, nk] = shape;
        let n = ni * nj * nk;
        let rho: Vec<f64> = (0..n)
            .map(|p| 0.5 + 0.4 * ((0.37 * p as f64).sin() * (0.011 * p as f64).cos()))
            .collect();
        let t = f.apply(&rho).expect("filtered");
        let r = f.length_um() / h;
        let at = |i: isize, j: isize, k: isize| -> f64 {
            // the mirrored ghost: the neighbour inside
            let c = |v: isize, n: usize| v.clamp(0, n as isize - 1) as usize;
            t[(c(k, nk) * nj + c(j, nj)) * ni + c(i, ni)]
        };
        for k in 0..nk as isize {
            for j in 0..nj as isize {
                for i in 0..ni as isize {
                    let mut lap = 0.0;
                    for (d, n) in [(0, ni), (1, nj), (2, nk)] {
                        if n == 1 {
                            continue;
                        }
                        let (a, b) = match d {
                            0 => (at(i - 1, j, k), at(i + 1, j, k)),
                            1 => (at(i, j - 1, k), at(i, j + 1, k)),
                            _ => (at(i, j, k - 1), at(i, j, k + 1)),
                        };
                        lap += a + b - 2.0 * at(i, j, k);
                    }
                    let q = ((k as usize) * nj + j as usize) * ni + i as usize;
                    worst = worst.max((at(i, j, k) - r * r * lap - rho[q]).abs());
                }
            }
        }
    }
    worst
}

/// The Helmholtz filter's edge: a solid half of a 1D region of 1.6 µm, the edge on a pixel
/// boundary in the middle, radius 50 nm, `h_nm` pixels: where the filtered density crosses `eta`
/// (by linear interpolation), nm from the edge, against r ln(1/2η).
pub(crate) fn helmholtz_edge(h_nm: f64, eta: f64) -> (f64, f64) {
    let n = (1600.0 / h_nm).round() as usize;
    let f = crate::topology::HelmholtzFilter::new([n, 1, 1], h_nm * 1e-3, 0.05).expect("a filter");
    let rho: Vec<f64> = (0..n).map(|i| if i < n / 2 { 1.0 } else { 0.0 }).collect();
    let t = f.apply(&rho).expect("filtered");
    let edge = (n / 2) as f64;
    let mut x = f64::NAN;
    for i in n / 2 - 1..n - 1 {
        if t[i] >= eta && t[i + 1] < eta {
            x = i as f64 + 0.5 + (t[i] - eta) / (t[i] - t[i + 1]);
            break;
        }
    }
    let r = f.length_um() * 1e3;
    ((x - edge) * h_nm, r * (1.0 / (2.0 * eta)).ln())
}

/// The double filter's gradient against fourth-order differences of F = Σ wᵢ ρ̂ᵢ (fixed varied
/// weights) on a 30 × 24 region at 10 nm (radii 100 and 50 nm, β 5 and 2.5, the second
/// projection at `eta`), at five pixels: the largest relative difference.
pub(crate) fn double_filter_error(delta: f64, eta: f64) -> f64 {
    let (ni, nj) = (30usize, 24usize);
    let d = crate::topology::DoubleFiltered::new(
        [ni, nj, 1],
        0.01,
        [1.0, 12.11],
        [0.1, 0.05],
        [
            Projection::new(5.0, 0.5).expect("a projection"),
            Projection::new(2.5, eta).expect("a projection"),
        ],
    )
    .expect("a pipeline");
    let n = ni * nj;
    let rho: Vec<f64> = (0..n)
        .map(|p| 0.5 + 0.35 * (0.3 * (p % ni) as f64).sin() * (0.23 * (p / ni) as f64 + 0.2).cos())
        .collect();
    let w: Vec<f64> = (0..n).map(|p| (0.71 * p as f64).sin()).collect();
    let f = |r: &[f64]| -> f64 {
        d.physical(r)
            .expect("physical")
            .iter()
            .zip(&w)
            .map(|(a, b)| a * b)
            .sum()
    };
    let g = d.pullback(&rho, &w).expect("pulled back");
    let mut worst = 0.0f64;
    for p in [0usize, 13, 361, 400, n - 1] {
        let at = |s: f64| {
            let mut r = rho.clone();
            r[p] += s * delta;
            f(&r)
        };
        let fd = (at(-2.0) - 8.0 * at(-1.0) + 8.0 * at(1.0) - at(2.0)) / (12.0 * delta);
        worst = worst.max((g[p] - fd).abs() / g[p].abs());
    }
    worst
}

fn helmholtz_residual_case() -> Outcome {
    zero(helmholtz_residual(), 1e-12)
}

fn helmholtz_edge_case() -> Outcome {
    let (x, want) = helmholtz_edge(1.25, 0.3);
    let r = 50.0 / (2.0 * 3f64.sqrt());
    zero((x - want).abs(), 0.1 * (1.25 / r) * (1.25 / r) * r)
}

fn double_filter_case() -> Outcome {
    zero(
        double_filter_error(1e-3, 0.5).max(double_filter_error(1e-3, 0.3)),
        1e-8,
    )
}

/// The area constraints' shapes: the largest difference from their exact values.
pub(crate) fn area_shapes_error() -> f64 {
    let n = 100usize;
    let c = crate::topology::AreaConstraints::new([n, n], 0.01, 0.08, 0.2).expect("constraints");
    let square = |lo: usize, side: usize, value: f64, background: f64| -> Vec<f64> {
        (0..n * n)
            .map(|p| {
                let (i, j) = (p % n, p / n);
                if (lo..lo + side).contains(&i) && (lo..lo + side).contains(&j) {
                    value
                } else {
                    background
                }
            })
            .collect()
    };
    let cases = [
        (square(40, 20, 1.0, 0.0), [0.04, 0.0]),
        (square(30, 40, 1.0, 0.0), [0.0, 0.0]),
        (square(35, 30, 0.0, 1.0), [0.0, 0.09]),
        (square(0, 20, 1.0, 0.0), [0.0, 0.0]),
    ];
    cases
        .iter()
        .map(|(pattern, want)| {
            let v = c.evaluate(pattern).expect("evaluated").values;
            (v[0] - want[0]).abs().max((v[1] - want[1]).abs())
        })
        .fold(0.0, f64::max)
}

fn area_shapes_case() -> Outcome {
    zero(area_shapes_error(), 1e-14)
}

/// The 2D kernel the subpixel-smoothed projection's fill implies: Hammond et al. 2025's parabolic
/// k̂₂ of radius `r` (their Eq. 23) integrated along the extruded third axis,
/// 5 (r² − ρ²)^{3/2} / (2π r⁵) at distance ρ.
fn ssp_kernel(rho: f64, r: f64) -> f64 {
    let h2 = (r * r - rho * rho).max(0.0);
    5.0 * h2 * h2.sqrt() / (2.0 * std::f64::consts::PI * r.powi(5))
}

/// ∫ₐᵇ k(ρ) ρ θ(ρ) dρ by Simpson's rule on ρ = a + (b − a) u², which takes the square root that
/// θ (an arc's angle) has at a.
fn ssp_ring_integral(a: f64, b: f64, r: f64, theta: impl Fn(f64) -> f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    let n = 20_000;
    let f = |u: f64| {
        let rho = a + (b - a) * u * u;
        ssp_kernel(rho, r) * rho * theta(rho) * 2.0 * (b - a) * u
    };
    let h = 1.0 / n as f64;
    let mut s = f(0.0) + f(1.0);
    for k in 1..n {
        s += f(k as f64 * h) * if k % 2 == 1 { 4.0 } else { 2.0 };
    }
    s * h / 3.0
}

/// The kernel's share of the solid side of a plane at signed distance `d` (positive on the void
/// side), radius `r`: the convolution their Eq. 9 takes, by quadrature.
fn ssp_plane_fill(d: f64, r: f64) -> f64 {
    if d < 0.0 {
        return 1.0 - ssp_plane_fill(-d, r);
    }
    ssp_ring_integral(d, r, r, |rho| 2.0 * (d / rho).min(1.0).acos())
}

/// The kernel's share of a disc of radius `a` seen from a point `x` from its centre, radius `r`,
/// by quadrature: the exact smoothed disc.
fn ssp_disc_fill(x: f64, a: f64, r: f64) -> f64 {
    let pi = std::f64::consts::PI;
    // the circle of radius ρ about the point lies wholly inside the disc below |a − x| (when the
    // point is inside) and wholly outside it (when outside); above, an arc of it
    let split = (a - x).abs().min(r);
    let inner = if x < a {
        ssp_ring_integral(0.0, split, r, |_| 2.0 * pi)
    } else {
        0.0
    };
    let arc = |rho: f64| {
        let c = (a * a - x * x - rho * rho) / (2.0 * x * rho);
        if c >= 1.0 {
            2.0 * pi
        } else if c <= -1.0 {
            0.0
        } else {
            2.0 * pi - 2.0 * c.acos()
        }
    };
    inner + ssp_ring_integral(split, r, r, arc)
}

/// A straight interface: (i) their Eq. 10's F₂ against the kernel's share of a half-space by
/// quadrature at 201 distances across R̂; (ii) the smoothed projection at β = ∞ of a linear ρ̃
/// (a ramp at 17° to the axes on 40 × 30 pixels of 10 nm) against F₂ of each pixel's exact
/// distance to its level set. The larger difference.
pub(crate) fn ssp_plane_error() -> f64 {
    use crate::topology::smoothed::{RADIUS, fill};
    let mut worst = 0.0f64;
    for k in 0..=200 {
        let t = -1.0 + 0.01 * k as f64;
        worst = worst.max((fill(t) - ssp_plane_fill(t, 1.0)).abs());
    }
    let (nx, ny, h) = (40usize, 30usize, 0.01);
    let (c, s) = (0.3f64.cos(), 0.3f64.sin());
    let slope = 2.0; // per µm: the ramp crosses ½ over 0.25 µm
    let at = |p: usize| {
        let (x, y) = ((p % nx) as f64 * h, (p / nx) as f64 * h);
        0.5 + slope * ((x - 0.2) * c + (y - 0.15) * s)
    };
    let filtered: Vec<f64> = (0..nx * ny).map(at).collect();
    let ssp = crate::topology::SmoothedProjection::new(f64::INFINITY, 0.5).expect("a projection");
    let out = ssp.apply([nx, ny, 1], h, &filtered).expect("applied");
    for (p, v) in out.iter().enumerate() {
        let d = (0.5 - filtered[p]) / slope;
        worst = worst.max((v - fill(d / (RADIUS * h))).abs());
    }
    worst
}

/// A disc of radius 0.25 µm, ρ̃ a cone 0.5 + (a − r)/0.2 µm about its centre on pixels of `h_nm`
/// over 0.8 µm: at β = ∞, each pixel within R̂ of the circle against the kernel's share of the
/// disc there (the exact smoothed disc), the largest difference.
pub(crate) fn ssp_disc(h_nm: f64) -> f64 {
    use crate::topology::smoothed::RADIUS;
    let h = h_nm * 1e-3;
    let a = 0.25;
    let n = (0.8 / h).round() as usize;
    let centre = 0.5 * n as f64 * h;
    let radius = |p: usize| {
        let x = ((p % n) as f64 + 0.5) * h - centre;
        let y = ((p / n) as f64 + 0.5) * h - centre;
        (x * x + y * y).sqrt()
    };
    let filtered: Vec<f64> = (0..n * n).map(|p| 0.5 + (a - radius(p)) / 0.2).collect();
    let ssp = crate::topology::SmoothedProjection::new(f64::INFINITY, 0.5).expect("a projection");
    let out = ssp.apply([n, n, 1], h, &filtered).expect("applied");
    let r = RADIUS * h;
    let mut worst = 0.0f64;
    for (p, v) in out.iter().enumerate() {
        let x = radius(p);
        if (x - a).abs() < r {
            worst = worst.max((v - ssp_disc_fill(x, a, r)).abs());
        }
    }
    worst
}

/// The smoothed projection's gradient against fourth-order differences of F = Σ wᵢ ρ̃̂ᵢ (fixed
/// varied weights) through a conic filter of 50 nm on a 30 × 24 region of 10 nm pixels, at
/// β = `beta` (infinite for a level set), at the pixels within the filter's reach of a pixel
/// near the level set: the largest difference relative to the largest exact derivative.
pub(crate) fn ssp_gradient_error(beta: f64, delta: f64) -> f64 {
    let (ni, nj) = (30usize, 24usize);
    let n = ni * nj;
    let d = Density::new([ni, nj, 1], 0.01, [1.0, 12.11])
        .expect("a density")
        .with_filter(0.05, Edge::Renormalized)
        .expect("a filter")
        .with_smoothed_projection(
            crate::topology::SmoothedProjection::new(beta, 0.5).expect("a projection"),
        );
    let rho: Vec<f64> = (0..n)
        .map(|p| 0.5 + 0.35 * (0.3 * (p % ni) as f64).sin() * (0.23 * (p / ni) as f64 + 0.2).cos())
        .collect();
    let w: Vec<f64> = (0..n).map(|p| (0.71 * p as f64).sin()).collect();
    let f = |r: &[f64]| -> f64 {
        d.physical(r)
            .expect("physical")
            .iter()
            .zip(&w)
            .map(|(a, b)| a * b)
            .sum()
    };
    let g = d.pullback(&rho, &w).expect("pulled back");
    let scale = g.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut worst = 0.0f64;
    for p in (0..n).step_by(7) {
        let at = |s: f64| {
            let mut r = rho.clone();
            r[p] += s * delta;
            f(&r)
        };
        let fd = (at(-2.0) - 8.0 * at(-1.0) + 8.0 * at(1.0) - at(2.0)) / (12.0 * delta);
        worst = worst.max((g[p] - fd).abs() / scale);
    }
    worst
}

fn ssp_plane_case() -> Outcome {
    zero(ssp_plane_error(), 1e-9)
}

fn ssp_disc_case() -> Outcome {
    zero(ssp_disc(5.0), 0.1 * 0.55 * 5.0 / 250.0)
}

fn ssp_gradient_case() -> Outcome {
    zero(
        ssp_gradient_error(64.0, 1e-4).max(ssp_gradient_error(f64::INFINITY, 1e-4)),
        1e-8,
    )
}
