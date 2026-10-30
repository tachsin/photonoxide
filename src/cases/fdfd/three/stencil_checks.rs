//! The multigrid without its finest matrix (#333): its smoother against its Fourier symbol and
//! against Erlangga, Oosterlee and Vuik's smoothing factors, shared by the tests and the
//! validation report.

use num_complex::Complex64 as c64;

use super::multigrid::{Multigrid, jacobi_on_error};
use super::{Boundaries3d, Grid3d, Solver3d};
use crate::fdfd::Edges;
use crate::units::Wavelength;

/// Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci. Comput. 27, 1471 (2006),
/// doi:10.1137/040615195, Table 5: the smoothing factors μ_h of ω-Jacobi for the complex shifted
/// Helmholtz operator −Δ − (β₁ − β₂ i)k² at k = 40 on the unit square with Dirichlet sides, on
/// h = 1/64, 1/32, 1/16 and 1/8. The rows with β₁ = 1, which photonoxide's shift k₀²(1 + iβ)ε
/// covers (β = β₂, the side of the loss our e^(−iωt) puts it on): β₂ and ω, and the four values.
pub(crate) const TABLE_5: [(f64, f64, [f64; 4]); 3] = [
    (0.0, 0.7, [0.47, 0.75, 2.31, 0.18]),
    (1.0, 0.7, [0.47, 0.56, 0.35, 0.13]),
    (0.5, 0.5, [0.60, 0.77, 0.81, 0.32]),
];

/// [`TABLE_5`] by the multigrid's own smoother: for each row and h, a grid of 1/h × 1/h cells of
/// h µm and one cell along z, walls on x and y (PMLs of no cells) and periodic along z, vacuum,
/// k₀ = 40 µm⁻¹, Shin and Fan's operator shifted by β from its stencil and its damped Jacobi
/// ([`jacobi_on_error`]). On E_z alone, constant along z, the operator is the 5-point Laplacian
/// with Dirichlet sides plus k₀²(1 + iβ), so the discrete sine modes sin(ℓπx) sin(mπy) are its
/// eigenvectors and the smoother's: one sweep multiplies each by its damping S(ℓ, m), read
/// here from the sweep itself (the Rayleigh quotient). μ_h, as Erlangga et al. compute it (their
/// Section 4.2, two smoothing sweeps, the low frequencies projected out), is the largest |S|²
/// over the modes with ℓ or m at least N/2. Also the largest departure of a swept mode from S
/// times itself, relative to the mode: how nearly each is an eigenvector. NaN if a grid fails.
pub(crate) fn table_5() -> (Vec<[f64; 4]>, f64) {
    let mut departure = 0.0f64;
    let mut rows = Vec::new();
    for &(beta, omega, _) in &TABLE_5 {
        let mut row = [f64::NAN; 4];
        for (slot, n) in [64usize, 32, 16, 8].into_iter().enumerate() {
            let Some((mu, off)) = smoothing_factor(n, beta, omega) else {
                continue;
            };
            row[slot] = mu;
            departure = departure.max(off);
        }
        rows.push(row);
    }
    (rows, departure)
}

/// μ_h on `n` × `n` cells for the shift `beta` and damping `omega`, and the largest departure of
/// a swept mode from an eigenvector's.
fn smoothing_factor(n: usize, beta: f64, omega: f64) -> Option<(f64, f64)> {
    let h = 1.0 / n as f64;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let walls = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: walls,
        y: walls,
        z: Edges::Bloch { k: 0.0 },
        ..Boundaries3d::pml(0)
    };
    let k0 = 40.0;
    let lam = Wavelength::um(std::f64::consts::TAU / k0).ok()?;
    let (lattice, eps) =
        Solver3d::setup(grid, lam, |_, _, _| c64::new(1.0, 0.0), boundaries).ok()?;
    let options = Multigrid {
        shift: beta,
        jacobi: super::multigrid::Jacobi {
            omega,
            pre: 1,
            post: 1,
        },
        ..Multigrid::default()
    };
    let sweep = jacobi_on_error(&lattice, &eps, &options);
    let (mut mu, mut departure) = (0.0f64, 0.0f64);
    for l in 1..n {
        for m in 1..n {
            if 2 * l < n && 2 * m < n {
                continue;
            }
            let mut e = vec![c64::new(0.0, 0.0); grid.unknowns()];
            for j in 0..n {
                for i in 0..n {
                    let v = (std::f64::consts::PI * (l * i) as f64 * h).sin()
                        * (std::f64::consts::PI * (m * j) as f64 * h).sin();
                    e[grid.index(super::Axis::Z, (i, j, 0))] = c64::new(v, 0.0);
                }
            }
            let swept = sweep(&e);
            let norm: f64 = e.iter().map(|z| z.norm_sqr()).sum();
            let s: c64 = e.iter().zip(&swept).map(|(p, q)| p.conj() * q).sum::<c64>() / norm;
            let off: f64 = e
                .iter()
                .zip(&swept)
                .map(|(p, q)| (q - s * p).norm_sqr())
                .sum::<f64>();
            departure = departure.max((off / norm).sqrt());
            mu = mu.max(s.norm_sqr());
        }
    }
    Some((mu, departure))
}

/// The smoother's damping of plane waves in a uniform periodic box against its Fourier symbol: a
/// box of 8 × 6 × 10 cells of 30, 40 and 50 nm, ε = 2.25, Bloch-periodic with k = 0 on every
/// side, at 1.55 µm, the shift 0.5 and ω = 0.8. Shin and Fan's operator is there the vector
/// Laplacian plus k₀²(1 + iβ)ε (∇ × ∇ × − ∇∇· = −Δ holds exactly on Yee's grid), so each
/// component's plane wave e^(i θ·n) is an eigenvector, with eigenvalue
/// λ = k₀²(1 + iβ)ε − Σ_a 4 sin²(θ_a/2)/h_a², and one sweep of damped Jacobi multiplies it by
/// S = 1 − ω λ / d, d = k₀²(1 + iβ)ε − Σ_a 2/h_a² the stencil's diagonal. The largest
/// |S_swept − S| over every wave of every component (S_swept the sweep's Rayleigh quotient), and
/// the largest departure of a swept wave from S_swept times itself, relative to the wave; NaN if
/// the box fails.
pub(crate) fn plane_wave_damping() -> (f64, f64) {
    let (n, h) = ([8usize, 6, 10], [0.03, 0.04, 0.05]);
    let grid = Grid3d {
        nx: n[0],
        ny: n[1],
        nz: n[2],
        dx: h[0],
        dy: h[1],
        dz: h[2],
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let periodic = Edges::Bloch { k: 0.0 };
    let boundaries = Boundaries3d {
        x: periodic,
        y: periodic,
        z: periodic,
        ..Boundaries3d::pml(0)
    };
    let (beta, omega, epsilon) = (0.5, 0.8, 2.25);
    let Ok(lam) = Wavelength::um(1.55) else {
        return (f64::NAN, f64::NAN);
    };
    let Ok((lattice, eps)) =
        Solver3d::setup(grid, lam, |_, _, _| c64::new(epsilon, 0.0), boundaries)
    else {
        return (f64::NAN, f64::NAN);
    };
    let options = Multigrid {
        shift: beta,
        jacobi: super::multigrid::Jacobi {
            omega,
            pre: 1,
            post: 1,
        },
        ..Multigrid::default()
    };
    let sweep = jacobi_on_error(&lattice, &eps, &options);
    let k2 = lattice.k0 * lattice.k0;
    let mass = c64::new(k2 * epsilon, beta * k2 * epsilon);
    let d = mass - (0..3).map(|a| 2.0 / (h[a] * h[a])).sum::<f64>();
    let (mut worst, mut departure) = (0.0f64, 0.0f64);
    for component in super::Axis::ALL {
        for q0 in 0..n[0] {
            for q1 in 0..n[1] {
                for q2 in 0..n[2] {
                    let q = [q0, q1, q2];
                    let theta: [f64; 3] =
                        [0, 1, 2].map(|a| std::f64::consts::TAU * q[a] as f64 / n[a] as f64);
                    let mut e = vec![c64::new(0.0, 0.0); grid.unknowns()];
                    for k in 0..n[2] {
                        for j in 0..n[1] {
                            for i in 0..n[0] {
                                let phase =
                                    theta[0] * i as f64 + theta[1] * j as f64 + theta[2] * k as f64;
                                e[grid.index(component, (i, j, k))] = c64::from_polar(1.0, phase);
                            }
                        }
                    }
                    let swept = sweep(&e);
                    let norm: f64 = e.iter().map(|z| z.norm_sqr()).sum();
                    let s: c64 =
                        e.iter().zip(&swept).map(|(p, q)| p.conj() * q).sum::<c64>() / norm;
                    let off: f64 = e
                        .iter()
                        .zip(&swept)
                        .map(|(p, q)| (q - s * p).norm_sqr())
                        .sum();
                    departure = departure.max((off / norm).sqrt());
                    let lambda = mass
                        - (0..3)
                            .map(|a| 4.0 * (theta[a] / 2.0).sin().powi(2) / (h[a] * h[a]))
                            .sum::<f64>();
                    let symbol = 1.0 - omega * lambda / d;
                    worst = worst.max((s - symbol).norm());
                }
            }
        }
    }
    (worst, departure)
}

/// The grids of [`refinement`]: the steps (nm), and each one's cells and PMLs.
pub(crate) const REFINEMENT: [(usize, [usize; 3], usize); 3] = [
    (40, [24, 20, 16], 6),
    (30, [32, 26, 22], 8),
    (20, [48, 40, 32], 12),
];

/// GMRES's iterations as the grid refines: fdfd3d/gmres-multigrid-direct's strip (a 500 × 220 nm
/// silicon strip in oxide in a 0.96 × 0.8 × 0.64 µm box, 1.55 µm) on [`REFINEMENT`]'s grids, its
/// stretched PMLs as thick in µm on each (6, 8 and 12 cells), a y-polarized point current at
/// the centre, to a relative residual of 1e-8; by the stored matrix's multigrid, and without
/// the finest matrix with Galerkin's coarse operators and discretized again. Each one's
/// iterations, grid by grid; 0 where a solve fails.
pub(crate) fn refinement() -> [[usize; 3]; 3] {
    use super::{Axis, Coarse, Formulation, IterativeSolver3d, Product};
    use crate::fdfd::Stopping;
    let strip = |_: f64, y: f64, z: f64| {
        let m: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
            3.476
        } else {
            1.444
        };
        c64::new(m * m, 0.0)
    };
    let stopping = Stopping {
        tolerance: 1e-8,
        max_iterations: 2000,
    };
    let ways = [
        (Product::Matrix, Coarse::Galerkin),
        (Product::MatrixFree, Coarse::Galerkin),
        (Product::MatrixFree, Coarse::Rediscretized),
    ];
    let mut out = [[0; 3]; 3];
    for (w, (product, coarse)) in ways.into_iter().enumerate() {
        for (g, &(nm, [nx, ny, nz], pml)) in REFINEMENT.iter().enumerate() {
            let h = nm as f64 * 1e-3;
            let grid = Grid3d {
                nx,
                ny,
                nz,
                dx: h,
                dy: h,
                dz: h,
                x0: -(nx as f64) * h / 2.0,
                y0: -(ny as f64) * h / 2.0,
                z0: -(nz as f64) * h / 2.0,
            };
            let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
            source[grid.index(Axis::Y, (nx / 2, ny / 2, nz / 2))] = c64::new(1.0, 0.0);
            let Ok(lam) = Wavelength::um(1.55) else {
                continue;
            };
            let solved = IterativeSolver3d::new_with(
                grid,
                lam,
                strip,
                Boundaries3d::stretched_pml(pml),
                Formulation::ShinFan,
                product,
            )
            .and_then(|s| {
                s.with_multigrid(Multigrid {
                    coarse,
                    ..Multigrid::default()
                })
            })
            .and_then(|s| s.solve(&source, stopping));
            if let Ok((_, how)) = solved {
                out[w][g] = how.iterations;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "a measurement, for the docs and fdfd3d/multigrid-stencil-refinement: cargo test --release fdfd::three::stencil_checks::tests::iterations_as_the_grid_refines -- --ignored --nocapture"]
    fn iterations_as_the_grid_refines() {
        println!("{:?}", refinement());
    }

    #[test]
    #[ignore = "a measurement, the validation cases' values for their sources: cargo test --release fdfd::three::stencil_checks::tests::the_cases_values -- --ignored --nocapture"]
    fn the_cases_values() {
        use super::super::{Coarse, Product};
        use crate::fdfd::recycle_checks3d::stencil_against_stored;
        for coarse in [Coarse::Galerkin, Coarse::Rediscretized] {
            let options = Multigrid {
                coarse,
                ..Multigrid::default()
            };
            let direct =
                crate::fdfd::checks3d::multigrid_of_against_direct(Product::MatrixFree, options);
            let sweep = stencil_against_stored(options);
            println!("{coarse:?}: against the direct solver {direct:?}; the sweep {sweep:?}");
        }
        println!(
            "stored against the direct solver {:?}",
            crate::fdfd::checks3d::multigrid_against_direct()
        );
        println!("plane waves {:?}", plane_wave_damping());
        println!("Table 5 {:?}", table_5());
    }

    #[test]
    fn the_smoother_damps_plane_waves_as_its_symbol_says() {
        let (worst, departure) = plane_wave_damping();
        assert!(worst < 1e-12, "{worst:e}");
        assert!(departure < 1e-12, "{departure:e}");
    }

    #[test]
    fn the_smoother_has_erlangga_oosterlee_and_vuiks_smoothing_factors() {
        let (rows, departure) = table_5();
        println!("{rows:?}, departure {departure:e}");
        assert!(departure < 1e-12, "{departure:e}");
        for (row, (_, _, theirs)) in rows.iter().zip(&TABLE_5) {
            for (ours, theirs) in row.iter().zip(theirs) {
                // one unit of their two decimals: their 0.75 at (1, 0), h = 1/32, is 0.7597 by
                // their own symbol
                assert!((ours - theirs).abs() <= 0.01, "{ours} against {theirs}");
            }
        }
    }
}
