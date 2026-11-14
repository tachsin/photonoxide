//! The validation cases of 2D FDFD's forward mode and of its Kerr problem (0.7, #761's I9):
//! forward mode against the adjoint, a uniform Kerr medium against its scalar equation's root,
//! and the Kerr adjoint's gradient against differences through Newton's method.

use num_complex::Complex64 as c64;

use super::checks::two_ports;
use super::{Boundaries, Direction, Edges, Grid, Kerr2d, Polarization, Solver2d};
use crate::portable;
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid problem";

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fdfd/forward-mode",
            title: r"2D FDFD, $E$ and $H$ along $z$: the derivative of a port mode's power along a change of 100 cells' permittivities (a 10 × 10 region beside a 220 nm slab, 20 nm grid) by forward mode, $du = -A^{-1}(\partial A/\partial\varepsilon \cdot \delta\varepsilon) u$, against the adjoint's gradient dotted with the same change (larger relative difference shown)",
            tier: Tier::Analytic,
            source: r"T. W. Hughes, I. A. D. Williamson, M. Minkov, S. Fan, ACS Photonics 6, 3010 (2019), doi:10.1021/acsphotonics.9b01238, Eqs. 5–9 (forward mode) and 10–14 (the adjoint): the same Jacobian evaluated in the two orders",
            run: forward_mode,
        },
        Case {
            id: "fdfd/kerr-uniform",
            title: r"A uniform Kerr medium ($\varepsilon = 2 + 0.3i$, $\chi = 0.8$ per unit $\lvert u\rvert^2$) on a periodic 6 × 5 grid at 2 µm, driven by a uniform source $b = 25 + 10i$: Newton's method's $\lvert u\rvert^2$ against the root of $s \lvert k_0^2(\varepsilon + \chi s)\rvert^2 = \lvert b\rvert^2$, where $\chi s$ is 0.4 of $\varepsilon$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"T. W. Hughes, M. Minkov, I. A. D. Williamson, S. Fan, ACS Photonics 5, 4781 (2018), doi:10.1021/acsphotonics.8b01522, Eqs. 14–15: $\tilde\varepsilon = \varepsilon + \chi\lvert u\rvert^2$; a uniform field makes the discrete Laplacian vanish, and the equation is a scalar cubic in $s = \lvert u\rvert^2$, with one root for $\chi \gt 0$ and $\mathop{\mathrm{Re}}\varepsilon \gt 0$",
            run: kerr_uniform,
        },
        Case {
            id: "fdfd/kerr-gradient",
            title: r"2D FDFD, $E$ along $z$, at 2 µm: a 0.3 µm guide of 5.95 through a 1 × 1 µm region of varied permittivity whose Kerr coefficient follows its density ($\chi \propto (\varepsilon - 1)/(\varepsilon_m - 1)$), driven to a largest $\Delta\varepsilon$ of 0.05: the gradient of the transmitted mode's power with respect to the region's permittivities by the nonlinear adjoint, against fourth-order differences through Newton's method ($\delta = 10^{-3}$) at a corner, an edge and the middle (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"Hughes et al. 2018, Eqs. 6–8 and 16–18: $J^{\mathsf T}\lambda + \bar K\bar\lambda = \partial F/\partial u$ with $J = A(\varepsilon + 2\chi\lvert u\rvert^2)$ and $K = k_0^2\,\mathrm{diag}(\chi u^2)$, and $dF/d\varepsilon_k = -2\mathop{\mathrm{Re}}(\lambda_k k_0^2 (1 + \chi'_k\lvert u_k\rvert^2) u_k)$",
            run: kerr_gradient,
        },
    ]
}

fn zero(error: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance,
        error,
    }
}

/// A 220 nm slab (3.476 in 1.444) along x, 2 × 3 µm with 20 cells of PML at 20 nm, and its
/// permittivities.
fn slab_problem() -> (Grid, Vec<c64>, usize) {
    let (h, pml) = (0.02, 20);
    let grid = Grid {
        nx: (2.0 / h) as usize + 2 * pml,
        ny: (3.0 / h) as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let cells = (0..grid.ny)
        .flat_map(|j| (0..grid.nx).map(move |_| j))
        .map(|j| {
            let e = if grid.y(j).abs() < 0.11 {
                portable::powi(3.476f64, 2)
            } else {
                portable::powi(1.444f64, 2)
            };
            c64::new(e, 0.0)
        })
        .collect();
    (grid, cells, pml)
}

/// Forward mode against the adjoint, for one polarization: their relative difference.
pub(crate) fn forward_mode_error(polarization: Polarization) -> f64 {
    let (grid, mut cells, _) = slab_problem();
    let lam = Wavelength::um(1.55).expect(OK);
    // a region beside the slab, its change varied cell by cell
    let first = ((0.9 - grid.x0) / grid.dx) as usize;
    let row = ((0.12 - grid.y0) / grid.dy) as usize;
    let mut delta = vec![0.0; cells.len()];
    for j in 0..10 {
        for i in 0..10 {
            let k = (row + j) * grid.nx + first + i;
            cells[k] = c64::new(4.0 + 0.1 * (i as f64), 0.0);
            delta[k] = portable::sin(1.0 + 0.7 * i as f64 + 1.3 * j as f64);
        }
    }
    let solver =
        Solver2d::from_cells(grid, polarization, lam, &cells, Boundaries::pml(20)).expect(OK);
    let ports = two_ports(&solver);
    let source = solver.mode_source(&ports[0].mode, Direction::Forward);
    let field = solver.solve_system(&source).expect(OK);
    let (_, gradient) = solver
        .mode_power_gradient(&field, &ports[1].mode, Direction::Forward)
        .expect(OK);
    let reverse: f64 = gradient.iter().zip(&delta).map(|(g, d)| g * d).sum();
    let tangent = solver.field_tangent(&field, &delta).expect(OK);
    let a = field.mode_amplitudes(&ports[1].mode).0;
    let da = tangent.mode_amplitudes(&ports[1].mode).0;
    let forward = 2.0 * (a.conj() * da).re;
    (forward - reverse).abs() / reverse.abs()
}

fn forward_mode() -> Outcome {
    let error = forward_mode_error(Polarization::Ez).max(forward_mode_error(Polarization::Hz));
    zero(error, 1e-9)
}

/// The uniform medium's |u|² against the scalar equation's root, relative.
pub(crate) fn kerr_uniform_error() -> f64 {
    let grid = Grid {
        nx: 6,
        ny: 5,
        dx: 0.05,
        dy: 0.05,
        x0: 0.0,
        y0: 0.0,
    };
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 0.0 },
        y: Edges::Bloch { k: 0.0 },
        reflection: 1e-8,
        order: 3.0,
    };
    let (eps, chi, b) = (c64::new(2.0, 0.3), 0.8, c64::new(25.0, 10.0));
    let n = grid.nx * grid.ny;
    let lam = Wavelength::um(2.0).expect(OK);
    let kerr = Kerr2d::new(grid, lam, &vec![eps; n], &vec![chi; n], boundaries).expect(OK);
    let solved = kerr.solve(&vec![b; n], None).expect(OK);
    let k0 = 2.0 * std::f64::consts::PI / 2.0;
    let k2 = k0 * k0;
    let left = |s: f64| s * (k2 * (eps + chi * s)).norm_sqr() - b.norm_sqr();
    let (mut lo, mut hi) = (0.0, 10.0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if left(mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let s = 0.5 * (lo + hi);
    let mut worst: f64 = 0.0;
    for j in 0..grid.ny {
        for i in 0..grid.nx {
            worst = worst.max((solved.field().at(i, j).norm_sqr() - s).abs() / s);
        }
    }
    worst
}

fn kerr_uniform() -> Outcome {
    zero(kerr_uniform_error(), 1e-10)
}

/// The Kerr problem of fdfd/kerr-gradient, its region's permittivities changed at `cell` by
/// `change`: (the problem, its source, the output port's mode, the region's cells).
fn kerr_problem(
    change: Option<(usize, f64)>,
) -> (Kerr2d, Vec<c64>, super::PortMode, Vec<usize>) {
    let (h, pml) = (0.05, 15);
    let grid = Grid {
        nx: (3.0 / h) as usize + 2 * pml,
        ny: (2.5 / h) as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.25 - pml as f64 * h,
    };
    let (eps_m, chi_m) = (5.95, 1.0);
    let lam = Wavelength::um(2.0).expect(OK);
    let mut eps = vec![c64::new(1.0, 0.0); grid.nx * grid.ny];
    let mut region = Vec::new();
    for j in 0..grid.ny {
        for i in 0..grid.nx {
            let (x, y) = (grid.x(i), grid.y(j));
            let k = j * grid.nx + i;
            if (1.0..2.0).contains(&x) && (-0.5..0.5).contains(&y) {
                let t = portable::sin(3.0 * x + 2.0 * y);
                eps[k] = c64::new(1.0 + (eps_m - 1.0) * (0.5 + 0.4 * t), 0.0);
                region.push(k);
            } else if y.abs() < 0.15 {
                eps[k] = c64::new(eps_m, 0.0);
            }
        }
    }
    if let Some((cell, d)) = change {
        eps[cell] += d;
    }
    // χ follows the density (ε − 1)/(ε_m − 1) inside the region, 0 elsewhere
    let mut chi = vec![0.0; eps.len()];
    let mut slope = vec![0.0; eps.len()];
    for &k in &region {
        chi[k] = chi_m * (eps[k].re - 1.0) / (eps_m - 1.0);
        slope[k] = chi_m / (eps_m - 1.0);
    }
    let kerr = Kerr2d::new(grid, lam, &eps, &chi, Boundaries::pml(pml))
        .expect(OK)
        .with_chi_slope(&slope)
        .expect(OK);
    let column = |x: f64| ((x - grid.x0) / h) as usize;
    let input = kerr.linear().port_modes(column(0.4), 1).expect(OK).remove(0);
    let output = kerr.linear().port_modes(column(2.6), 1).expect(OK).remove(0);
    let unit = kerr.linear().mode_source(&input, Direction::Forward);
    // the source's amplitude that makes the largest Δε 0.05 in the linear field, fixed
    let linear = kerr.linear().solve_system(&unit).expect(OK);
    let top = region
        .iter()
        .map(|&k| chi_m * linear.at(k % grid.nx, k / grid.nx).norm_sqr())
        .fold(0.0, f64::max);
    let amplitude = (0.05 / top).sqrt();
    let source = unit.iter().map(|s| s * amplitude).collect();
    (kerr, source, output, region)
}

/// The Kerr adjoint's gradient against its differences at three cells: (largest relative
/// difference, what the differences resolve).
pub(crate) fn kerr_agreement() -> (f64, f64) {
    let (kerr, source, output, region) = kerr_problem(None);
    let field = kerr.solve(&source, None).expect(OK);
    let (_, gradient) = kerr
        .mode_power_gradient(&field, &output, Direction::Forward)
        .expect(OK);
    let power = |cell: usize, d: f64| {
        // the same source amplitude as the unchanged problem: it is fixed by the linear field
        // there, and the change is away from the ports
        let (k, _, out, _) = kerr_problem(Some((cell, d)));
        let f = k.solve(&source, Some(field.field())).expect(OK);
        f.field().mode_amplitudes(&out).0.norm_sqr()
    };
    let (mut error, mut resolution) = (0.0f64, 0.0f64);
    let picks = [region[0], region[region.len() / 2 + 10], region[region.len() - 1]];
    for cell in picks {
        let diff = |h: f64| {
            (8.0 * (power(cell, h) - power(cell, -h)) - (power(cell, 2.0 * h) - power(cell, -2.0 * h)))
                / (12.0 * h)
        };
        let (one, two) = (diff(1e-3), diff(2e-3));
        error = error.max((one - gradient[cell]).abs() / gradient[cell].abs());
        resolution = resolution.max((one - two).abs() / gradient[cell].abs());
    }
    (error, resolution)
}

fn kerr_gradient() -> Outcome {
    let (error, resolution) = kerr_agreement();
    crate::verdict::note_numerical(resolution);
    zero(error, 1e-6)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers the rules quote.
    #[test]
    #[ignore = "a study, printed: cargo test --release --lib the_kerr_study -- --ignored --nocapture"]
    fn the_kerr_study() {
        println!("forward mode Ez {:.3e}", forward_mode_error(Polarization::Ez));
        println!("forward mode Hz {:.3e}", forward_mode_error(Polarization::Hz));
        println!("kerr uniform {:.3e}", kerr_uniform_error());
        println!("kerr (error, resolution) {:?}", kerr_agreement());
    }
}
