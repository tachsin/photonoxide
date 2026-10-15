//! The fill fractions' checks, which the tests and the validation report both run: a slab's
//! index off the grid's lines against its exact one, a photonic crystal's lowest band by every
//! averaging (A. Farjadpour et al., Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972, Fig. 1
//! and its TM remark), and the geometric derivative by the adjoint against finite differences.

use num_complex::Complex64 as c64;

use super::{Boundaries, Direction, Edges, Grid, Polarization, Solver2d};
use crate::averaging::{Averaging, Plane};
use crate::geometry::{Point, Shape};
use crate::mode::slab::Slab;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

/// The slab: 220 nm of 3.476 in 1.444, at 1.55 µm.
pub(crate) const SLAB: (f64, f64, f64, f64) = (0.22, 3.476, 1.444, 1.55);

/// The slab's index by 2D FDFD with `averaging`, on a grid of h = t/(cells + 0.4), the film's
/// lower face 0.3 of a cell above a grid line and its upper face 0.7 below one: (found, exact).
/// The guide runs along x; E along z is the slab's TE mode, H along z its TM mode. The port's
/// modes take the grid's own dispersion along x, so the error is the cross-section's.
pub(crate) fn slab_index(
    polarization: Polarization,
    averaging: Averaging,
    cells: usize,
) -> (f64, f64) {
    let (t, core, clad, lam) = SLAB;
    let h = t / (cells as f64 + 0.4);
    // 2 µm of cladding on each side, walls beyond: the TM mode is e⁻¹² there
    let below = (2.0 / h).ceil() as usize;
    let ny = 2 * below + cells + 1;
    let a = 0.0;
    let grid = Grid {
        nx: 6,
        ny,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: a - (below as f64 + 0.3) * h,
    };
    let film = Shape::rect(
        Point::um(0.0, a + t / 2.0),
        Length::um(100.0),
        Length::um(t),
    )
    .unwrap();
    let mut plane = Plane::new(re(clad * clad));
    plane.draw(film, re(core * core));
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries {
        x: wall,
        y: wall,
        ..Boundaries::pml(0)
    };
    let wavelength = Wavelength::um(lam).unwrap();
    let solver = Solver2d::from_plane(
        grid,
        polarization,
        wavelength,
        &plane,
        boundaries,
        averaging,
    )
    .expect("the slab's problem is well posed");
    let found = solver.port_modes(2, 1).expect("the slab guides")[0]
        .effective_index()
        .re;
    let kind = match polarization {
        Polarization::Ez => crate::mode::Polarization::Te,
        Polarization::Hz => crate::mode::Polarization::Tm,
    };
    let exact = Slab::new(clad, core, clad, Length::um(t))
        .unwrap()
        .modes(kind, wavelength)[0]
        .effective_index();
    (found, exact)
}

/// The same slab in 3D, a layer of a stack ([`crate::stack::Structure`]) with silicon and
/// silica from the catalogue, normal to z and uniform along y (Bloch-periodic, one cell), its
/// fundamental `kind` mode on a port normal to x by the grid's full-vector port modes: (found,
/// exact). The film's faces are 0.3 of a cell above a node and 0.7 below one, as in 2D.
pub(crate) fn film_index(
    kind: crate::mode::Polarization,
    averaging: Averaging,
    cells: usize,
) -> (f64, f64) {
    use crate::fdfd::three::port_checks::share;
    use crate::fdfd::{Axis, Boundaries3d, Grid3d};
    use crate::material::{silica, silicon};
    use crate::stack::{Layer, LayerStack, Structure};
    let (t, lam) = (0.22, 1.55);
    let wavelength = Wavelength::um(lam).unwrap();
    let layer = |name: &str, thickness: f64, m: crate::material::Material| Layer {
        name: name.into(),
        thickness: Length::um(thickness),
        material: m.clone(),
        background: m,
    };
    let stack = LayerStack::new(
        silica(),
        vec![layer("BOX", 1.5, silica()), layer("film", t, silicon())],
        silica(),
    )
    .unwrap();
    let structure = Structure::new(stack);
    let h = t / (cells as f64 + 0.4);
    let below = (2.0 / h).ceil() as usize;
    let nz = 2 * below + cells + 1;
    let grid = Grid3d {
        nx: 5,
        ny: 1,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 1.5 - (below as f64 + 0.3) * h,
    };
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: wall,
        y: Edges::Bloch { k: 0.0 },
        z: wall,
        ..Boundaries3d::pml(0)
    };
    let eps = crate::fdfd::three::fill::structure_values(grid, wavelength, &structure, averaging)
        .expect("the film's permittivity");
    let k0 = 2.0 * std::f64::consts::PI / lam;
    let lattice = crate::fdfd::three::Lattice::new(grid, boundaries, k0);
    let modes = lattice
        .port_modes(&eps, (Axis::X, 2), [0..1, 0..nz], 2)
        .expect("the film guides");
    let along = match kind {
        crate::mode::Polarization::Te => Axis::Y,
        crate::mode::Polarization::Tm => Axis::Z,
    };
    let mode = modes
        .iter()
        .find(|m| share(m, &grid, along) > 0.5)
        .expect("the film's mode");
    let n = |m: crate::material::Material| {
        crate::units::refractive_index(m.permittivity(wavelength).unwrap()).re
    };
    let exact = Slab::new(n(silica()), n(silicon()), n(silica()), Length::um(t))
        .unwrap()
        .modes(kind, wavelength)[0]
        .effective_index();
    (mode.effective_index().re, exact)
}

/// The guide and disk of the derivative's check: a 220 nm silicon slab (3.476 in 1.444) along
/// x through a 2 × 3 µm window, and a disk of permittivity 6 and radius `r` beside it at
/// (1.0, 0.33) µm, between the grid's lines.
fn guide_and_disk(r: f64) -> Plane {
    let mut plane = Plane::new(re(1.444 * 1.444));
    let guide = Shape::rect(Point::um(1.0, 0.0), Length::um(20.0), Length::um(0.22)).unwrap();
    plane.draw(guide, re(3.476 * 3.476));
    let disk = Shape::circle(Point::um(1.003, 0.331), Length::um(r)).unwrap();
    plane.draw(disk, re(6.0));
    plane
}

/// The derivative with respect to the disk's radius (0.13 µm) of the power the guide delivers
/// into its right port's mode from its left port's, on a 20 nm grid, with `averaging` (one that
/// reads the fill): by the adjoint ([`Solver2d::mode_power_derivative`], the matrices at
/// r ± 0.1 nm, one more back-substitution) and by central differences of the power itself
/// (two more full solves, at r ± 0.1 nm). Returns (adjoint, finite differences).
///
/// The fill makes the power a smooth function of the radius between the radii at which the
/// circle passes a cell's corner, where its second derivative jumps; so the differences are
/// taken over 0.1 nm, which no corner falls in here. (Over ± 1 and ± 2 nm, fourth-order
/// differences of the mean and the diagonal differ from the adjoint by 4e-4 to 1.6e-2.)
pub(crate) fn radius_derivative(polarization: Polarization, averaging: Averaging) -> (f64, f64) {
    let (h, pml) = (0.02, 20);
    let grid = Grid {
        nx: (2.0 / h) as usize + 2 * pml,
        ny: (3.0 / h) as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let lam = Wavelength::um(1.55).unwrap();
    let r = 0.13;
    let solver = |r: f64| {
        Solver2d::from_plane(
            grid,
            polarization,
            lam,
            &guide_and_disk(r),
            Boundaries::pml(pml),
            averaging,
        )
        .unwrap()
    };
    let base = solver(r);
    let ports = super::checks::two_ports(&base);
    let source = base.mode_source(&ports[0].mode, Direction::Forward);
    let power = |s: &Solver2d| {
        let field = s.solve_system(&source).unwrap();
        field.mode_amplitudes(&ports[1].mode).0.norm_sqr()
    };
    let field = base.solve_system(&source).unwrap();
    let delta = 1e-4;
    let (low, high) = (guide_and_disk(r - delta), guide_and_disk(r + delta));
    let (_, adjoint) = base
        .mode_power_derivative(
            &field,
            &ports[1].mode,
            Direction::Forward,
            [&low, &high],
            2.0 * delta,
        )
        .unwrap();
    // the same geometry's powers, each a full solve: the derivative to O(δ²), round-off of
    // about 1e-12/δ
    let fd = (power(&solver(r + delta)) - power(&solver(r - delta))) / (2.0 * delta);
    (adjoint, fd)
}

/// The least-squares slope of log |error| against log h: the order of convergence.
pub(crate) fn order(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let xs: Vec<f64> = points.iter().map(|p| p.0.ln()).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.1.abs().ln()).collect();
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    sxy / sxx
}

/// The slab's errors at film thicknesses of 4 to 32 cells (h 50 to 6.8 nm): (h, error).
pub(crate) fn slab_errors(polarization: Polarization, averaging: Averaging) -> Vec<(f64, f64)> {
    FILM_CELLS
        .into_iter()
        .map(|cells| {
            let (found, exact) = slab_index(polarization, averaging, cells);
            (SLAB.0 / (cells as f64 + 0.4), found - exact)
        })
        .collect()
}

/// The film's cells across its thickness in the slab's runs, halving the step each time: h from
/// 26 to 0.86 nm.
pub(crate) const FILM_CELLS: [usize; 6] = [8, 16, 32, 64, 128, 256];

/// [`slab_errors`] for the 3D film ([`film_index`]).
pub(crate) fn film_errors(
    kind: crate::mode::Polarization,
    averaging: Averaging,
) -> Vec<(f64, f64)> {
    FILM_CELLS
        .into_iter()
        .map(|cells| {
            let (found, exact) = film_index(kind, averaging, cells);
            (SLAB.0 / (cells as f64 + 0.4), found - exact)
        })
        .collect()
}

/// The order between the two finest grids of a run of halvings: log₂ of their errors' ratio
/// (the steps' ratio is not quite 2, h = t/(cells + 0.4), so it is taken exactly).
pub(crate) fn last_order(errors: &[(f64, f64)]) -> f64 {
    let [.., (h1, e1), (h2, e2)] = errors else {
        return f64::NAN;
    };
    (e1.abs() / e2.abs()).ln() / (h1 / h2).ln()
}

/// The photonic crystal of Farjadpour et al.'s Fig. 1, as we draw it: a square lattice of
/// period a = 1 µm, ε = 12, with an elliptical air hole of semi-axes 0.32a and 0.2a turned 30°
/// in each cell (their inset shows an ellipse at a slant; the paper gives no sizes).
pub(crate) fn crystal() -> Plane {
    let mut plane = Plane::new(re(12.0));
    let hole = Shape::ellipse(
        Point::um(0.5, 0.5),
        [Length::um(0.32), Length::um(0.2)],
        30f64.to_radians(),
    )
    .unwrap();
    plane.draw(hole, re(1.0));
    plane
}

/// The Bloch wavevector, not along the grid's axes: (0.3, 0.2) 2π/a.
pub(crate) const BLOCH: [f64; 2] = [0.3, 0.2];

/// The lowest band's frequency ωa/2πc at [`BLOCH`] for `polarization`, `averaging` and
/// `resolution` cells per period: the smallest eigenvalue of the grid's operator (no source,
/// Bloch-periodic on both axes), by shift-and-invert at 0. With H along z the operator
/// ∂(ε̃⁻¹ ∂ H) is symmetric and its eigenvalue (ω/c)²; with E along z, ε⁻¹ ∇² E is taken.
pub(crate) fn crystal_band(
    polarization: Polarization,
    averaging: Averaging,
    resolution: usize,
) -> f64 {
    let plane = crystal();
    let h = 1.0 / resolution as f64;
    let grid = Grid {
        nx: resolution,
        ny: resolution,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
    };
    let tau = 2.0 * std::f64::consts::PI;
    let boundaries = Boundaries {
        x: Edges::Bloch { k: BLOCH[0] * tau },
        y: Edges::Bloch { k: BLOCH[1] * tau },
        ..Boundaries::pml(0)
    };
    // any k₀: it only adds k₀² ε on the diagonal, taken off again
    let k0 = 1.0;
    let solver = Solver2d::from_plane(
        grid,
        polarization,
        Wavelength::um(tau / k0).unwrap(),
        &plane,
        boundaries,
        averaging,
    )
    .expect("the crystal's problem is well posed");
    let n = grid.nx * grid.ny;
    let entries: Vec<(usize, usize, c64)> = solver
        .entries
        .iter()
        .map(|t| {
            let (r, c) = (t.row, t.col);
            match polarization {
                Polarization::Hz => {
                    let d = if r == c { k0 * k0 } else { 0.0 };
                    (r, c, -(t.val - d))
                }
                Polarization::Ez => {
                    let e = solver.eps_z[r];
                    let d = if r == c { k0 * k0 * e } else { re(0.0) };
                    (r, c, -(t.val - d) / e)
                }
            }
        })
        .collect();
    let pairs = crate::eigen::nearest(n, &entries, re(0.0), 1, 1e-10)
        .unwrap_or_else(|e| panic!("the lowest band converges: {e}"));
    pairs[0].value.re.sqrt() / tau
}

/// The resolutions of the crystal's runs, cells per period: Farjadpour et al.'s 10 to 80 of
/// their 10 to 100.
pub(crate) const RESOLUTIONS: [usize; 8] = [10, 14, 20, 28, 40, 48, 56, 80];

/// The crystal's lowest TE (H along z) band by the tensor at 160 and 320 cells per period
/// (0.12696791 and 0.12696572; at 80, 0.12698373), extrapolated at second order: the reference
/// its errors are taken from (`fill_checks::tests::reference`). It is 5.8e-6 (relative) from
/// the value at 320 cells, so the fits stop at 56 cells, where the errors are well above that.
pub(crate) const CRYSTAL_TE: f64 = 0.126_964_991_431;

/// The same for TM (E along z): the mean at 160 and 320 cells per period (0.11414446 and
/// 0.11414483), extrapolated at the second order its last three resolutions show (the changes
/// shrink by 3.85).
pub(crate) const CRYSTAL_TM: f64 = 0.114_144_958_603;

/// The resolutions the crystal's orders are fitted over: 10 to 56 cells per period.
pub(crate) const FITTED: usize = 7;

/// The relative errors of the lowest band at [`RESOLUTIONS`]: (1/resolution, error).
pub(crate) fn crystal_errors(polarization: Polarization, averaging: Averaging) -> Vec<(f64, f64)> {
    let reference = match polarization {
        Polarization::Hz => CRYSTAL_TE,
        Polarization::Ez => CRYSTAL_TM,
    };
    RESOLUTIONS
        .into_iter()
        .map(|r| {
            let w = crystal_band(polarization, averaging, r);
            (1.0 / r as f64, (w - reference) / reference)
        })
        .collect()
}

/// An order, to two decimals as the report shows it, against `expected` within `tolerance`.
fn order_outcome(order: f64, expected: f64, tolerance: f64) -> Outcome {
    let measured = (order * 100.0).round() / 100.0;
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The largest relative difference between the adjoint's derivative and the finite
/// differences', over the averagings that read the fill.
pub(crate) fn derivative_error(polarization: Polarization) -> f64 {
    [Averaging::Mean, Averaging::Diagonal, Averaging::Tensor]
        .into_iter()
        .map(|a| {
            let (adjoint, fd) = radius_derivative(polarization, a);
            (adjoint - fd).abs() / fd.abs()
        })
        .fold(0.0, f64::max)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    use crate::mode::Polarization as Kind;
    vec![
        Case {
            id: "fdfd/fill-slab-te-mean",
            title: r"2D FDFD, $E$ along $z$, averaged from the fill (the mean): a 220 nm silicon slab in silica at 1.55 µm, its faces 0.3 and 0.7 of a cell off the grid's lines; the order of its TE index's convergence from 128 to 256 cells across it (h = 1.7 and 0.86 nm; shown)",
            tier: Tier::Analytic,
            source: r"the exact slab (mode::slab); Yee's centred differences are second order, and the mean is the right average for $E$ along every interface (A. Farjadpour et al., Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972): errors 8.1e-3 at 26 nm to 9.0e-6 at 0.86 nm",
            run: || {
                order_outcome(
                    last_order(&slab_errors(Polarization::Ez, Averaging::Mean)),
                    2.0,
                    0.1,
                )
            },
        },
        Case {
            id: "fdfd/fill-slab-te-point",
            title: r"The same slab with each cell the material at its point (staircasing): the order from 128 to 256 cells (shown)",
            tier: Tier::Analytic,
            source: "the staircased film is a share of a cell too thick or thin: an error of order h, first order",
            run: || {
                order_outcome(
                    last_order(&slab_errors(Polarization::Ez, Averaging::Point)),
                    1.0,
                    0.1,
                )
            },
        },
        Case {
            id: "fdfd/fill-slab-tm-diagonal",
            title: r"2D FDFD, $H$ along $z$, averaged from the fill (the tensor's diagonal: the harmonic mean for $E_y$ across the slab, the mean for $E_x$ along it): the same slab's TM index, the order from 128 to 256 cells (shown)",
            tier: Tier::Analytic,
            source: r"the exact slab; Farjadpour et al.'s average, exact for an interface along the grid, keeps the second order",
            run: || {
                order_outcome(
                    last_order(&slab_errors(Polarization::Hz, Averaging::Diagonal)),
                    2.0,
                    0.2,
                )
            },
        },
        Case {
            id: "fdfd/fill-slab-tm-mean",
            title: r"The same TM index with the mean for every component, the wrong average for $E_y$ across the slab: the order from 128 to 256 cells (shown)",
            tier: Tier::Analytic,
            source: r"the mean perturbs $D_\perp$ to first order (Farjadpour et al.'s criterion), so the error is first order",
            run: || {
                order_outcome(
                    last_order(&slab_errors(Polarization::Hz, Averaging::Mean)),
                    1.0,
                    0.1,
                )
            },
        },
        Case {
            id: "fdfd3d/fill-film-te-diagonal",
            title: r"3D FDFD from a stack, averaged from the fill (diagonal): a 220 nm silicon film in silica (the catalogue's, 1.55 µm), its faces off the grid's planes; the order of its TE port mode's index from 128 to 256 cells across it (shown)",
            tier: Tier::Analytic,
            source: "the exact slab (mode::slab) of the catalogue's indices; the mean along the film, second order",
            run: || {
                order_outcome(
                    last_order(&film_errors(Kind::Te, Averaging::Diagonal)),
                    2.0,
                    0.1,
                )
            },
        },
        Case {
            id: "fdfd3d/fill-film-tm-diagonal",
            title: r"The same film's TM port mode, $E_z$ given the harmonic mean across the film's slices: the order from 128 to 256 cells (shown)",
            tier: Tier::Analytic,
            source: "the exact slab; the harmonic mean across a layer keeps the second order",
            run: || {
                order_outcome(
                    last_order(&film_errors(Kind::Tm, Averaging::Diagonal)),
                    2.0,
                    0.2,
                )
            },
        },
        Case {
            id: "fdfd3d/fill-film-tm-point",
            title: r"The same TM port mode with each value of E the material at its point: the order from 128 to 256 cells (shown)",
            tier: Tier::Analytic,
            source: "staircasing: first order",
            run: || {
                order_outcome(
                    last_order(&film_errors(Kind::Tm, Averaging::Point)),
                    1.0,
                    0.1,
                )
            },
        },
        Case {
            id: "fdfd/fill-crystal-te-tensor",
            title: r"Farjadpour et al.'s Fig. 1 as we draw it: a square lattice of elliptical air holes (semi-axes 0.32$a$ and 0.2$a$, turned 30°) in $\varepsilon$ = 12, the lowest TE ($H$ along $z$) band at $k = (0.3, 0.2)\thinspace 2\pi/a$ by 2D FDFD with the whole tensor; the order fitted to its relative errors at 10 to 56 cells per period (shown)",
            tier: Tier::Published,
            source: r"A. Farjadpour et al., Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972, Fig. 1: only their tensor converges quadratically, with scatter; the reference is the tensor at 160 and 320 cells per period extrapolated at second order (5.8e-6 from the value at 320); errors 3.0e-3 at 10 to 1.5e-4 at 80",
            run: || {
                order_outcome(
                    order(&crystal_errors(Polarization::Hz, Averaging::Tensor)[..FITTED]),
                    2.0,
                    0.4,
                )
            },
        },
        Case {
            id: "fdfd/fill-crystal-te-others",
            title: r"The same band by every other average (point, samples, the mean, the tensor's diagonal): the largest order fitted at 10 to 56 cells per period (shown)",
            tier: Tier::Published,
            source: r"Farjadpour et al. (2006), Fig. 1: the mean and the diagonal (their VP-EP) converge linearly, no smoothing erratically; here point 1.05, samples 0.84, mean 1.33, diagonal 0.93",
            run: || {
                let worst = [
                    Averaging::Point,
                    Averaging::Samples,
                    Averaging::Mean,
                    Averaging::Diagonal,
                ]
                .into_iter()
                .map(|a| order(&crystal_errors(Polarization::Hz, a)[..FITTED]))
                .fold(f64::NEG_INFINITY, f64::max);
                order_outcome(worst, 1.0, 0.4)
            },
        },
        Case {
            id: "fdfd/fill-crystal-tm-mean",
            title: r"The same crystal's lowest TM ($E$ along $z$) band by the mean: the order fitted at 10 to 56 cells per period (shown)",
            tier: Tier::Published,
            source: r"Farjadpour et al. (2006): for TM every smoothing is the mean, and converges quadratically; the reference is the mean at 160 and 320 cells per period extrapolated at second order",
            run: || {
                order_outcome(
                    order(&crystal_errors(Polarization::Ez, Averaging::Mean)[..FITTED]),
                    2.0,
                    0.2,
                )
            },
        },
        Case {
            id: "fdfd/fill-derivative-ez",
            title: r"2D FDFD, $E$ along $z$, averaged from the fill: the derivative of a guide's transmitted power with respect to the radius of a disk beside it (0.13 µm, 20 nm grid) by the adjoint, from the matrices at $r \pm$ 0.1 nm, against central differences of the power over the same $\pm$ 0.1 nm (largest relative difference over the mean, diagonal and tensor shown)",
            tier: Tier::Analytic,
            source: r"the adjoint variable method (G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288): $\mathrm{d}F/\mathrm{d}p = -2\operatorname{Re}(\lambda^T (\mathrm{d}A/\mathrm{d}p) u)$; the fill makes $A$ a smooth function of the radius",
            run: || {
                let e = derivative_error(Polarization::Ez);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-4,
                    error: e,
                }
            },
        },
        Case {
            id: "fdfd/fill-derivative-hz",
            title: r"The same with $H$ along $z$, where the radius moves the mean, the diagonal or the whole tensor, its off-diagonal coupling too (shown)",
            tier: Tier::Analytic,
            source: "the adjoint variable method, as above",
            run: || {
                let e = derivative_error(Polarization::Hz);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-4,
                    error: e,
                }
            },
        },
    ]
}

#[cfg(test)]
mod tests;
