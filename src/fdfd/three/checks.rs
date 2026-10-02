//! The 3D setups that both the tests and the validation report run: a plane wave on a film, the
//! PML's own reflection, and a structure invariant along z against the 2D solver.

use num_complex::Complex64 as c64;

use super::{Axis, Boundaries3d, Field3d, Grid3d, Solver3d};
use crate::fdfd::{Boundaries, Edges, Grid, Polarization, Solver2d};
use crate::mode::Polarization as Kind;
use crate::mode::multilayer::Multilayer;
use crate::units::{Length, Wavelength};

/// The PMLs' thickness in the film and PML runs, cells.
pub(crate) const PML: usize = 20;

/// The azimuth of the plane of incidence in the film and PML runs: 30° from x, so that kx and
/// ky are both nonzero and every component of E is in play.
const AZIMUTH: f64 = 30.0;

/// A current sheet on node plane `k` (E_x and E_y), with the phase of a plane wave of transverse
/// wavenumbers (kx, ky), radiating the grid's own s (TE) or p (TM) wave: its discrete
/// divergence in the plane is zero for s, and its curl for p.
fn sheet(grid: &Grid3d, (kx, ky): (f64, f64), kind: Kind, k: usize) -> Vec<c64> {
    // the grid's own transverse wavenumbers, (2/h) sin(kh/2)
    let (tx, ty) = (
        2.0 / grid.dx * (kx * grid.dx / 2.0).sin(),
        2.0 / grid.dy * (ky * grid.dy / 2.0).sin(),
    );
    let norm = tx.hypot(ty);
    let (ux, uy) = if norm > 0.0 {
        (tx / norm, ty / norm)
    } else {
        (AZIMUTH.to_radians().cos(), AZIMUTH.to_radians().sin())
    };
    let (ax, ay) = match kind {
        Kind::Te => (-uy, ux),
        Kind::Tm => (ux, uy),
    };
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    for j in 0..grid.ny {
        for i in 0..grid.nx {
            for (component, a) in [(Axis::X, ax), (Axis::Y, ay)] {
                let [x, y, _] = grid.e_position(component, (i, j, k));
                source[grid.index(component, (i, j, k))] =
                    a * c64::from_polar(1.0, kx * x + ky * y);
            }
        }
    }
    source
}

/// A plane wave from air onto 220 nm of silicon (3.476) on oxide (1.444), at 1.55 µm: the run
/// with the film, and the run with air everywhere. The film is normal to z and fills
/// 0 < z < 0.22, its faces on nodes; x and y are Bloch-periodic, one cell each.
pub(crate) struct FilmRun {
    pub(crate) full: Field3d,
    pub(crate) empty: Field3d,
    pub(crate) grid: Grid3d,
    /// R and T by the transfer matrices.
    pub(crate) exact: (f64, f64),
}

impl FilmRun {
    /// The plane just below `z`: halfway between the node below it and the next.
    pub(crate) fn plane(&self, z: f64) -> usize {
        ((z - self.grid.z0) / self.grid.dz).floor() as usize
    }
}

/// The runs on an `h` grid, at `angle` from the normal in air, with PMLs of `pml.0` cells
/// graded to reflect `pml.1`.
pub(crate) fn film_run(kind: Kind, h: f64, angle: f64, pml: (usize, f64)) -> FilmRun {
    let (pml_cells, pml_reflection) = pml;
    let (cover, film, substrate) = (1.0, 3.476, 1.444);
    let lam = Wavelength::um(1.55).unwrap();
    let k0 = 2.0 * std::f64::consts::PI / 1.55;
    // along z: PML, 0.6 um of oxide, the film, 1.2 um of air, PML
    let below = (0.6 / h).round() as usize + pml_cells;
    let nz = below + (0.22 / h).round() as usize + (1.2 / h).round() as usize + pml_cells;
    let grid = Grid3d {
        nx: 1,
        ny: 1,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: -(below as f64) * h,
    };
    let kt = k0 * cover * angle.sin();
    let (kx, ky) = (
        kt * AZIMUTH.to_radians().cos(),
        kt * AZIMUTH.to_radians().sin(),
    );
    let boundaries = Boundaries3d {
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: ky },
        z: Edges::Pml {
            low: pml_cells,
            high: pml_cells,
        },
        reflection: pml_reflection,
        order: 3.0,
    };
    let layered = |z: f64| {
        let n = if z < 0.0 {
            substrate
        } else if z < 0.22 {
            film
        } else {
            cover
        };
        c64::new(n * n, 0.0)
    };
    let source = sheet(
        &grid,
        (kx, ky),
        kind,
        ((0.62 - grid.z0) / h).round() as usize,
    );
    let full = Solver3d::new(grid, lam, |_, _, z| layered(z), boundaries)
        .unwrap()
        .solve(&source)
        .unwrap();
    let empty = Solver3d::new(
        grid,
        lam,
        |_, _, _| c64::new(cover * cover, 0.0),
        boundaries,
    )
    .unwrap()
    .solve(&source)
    .unwrap();
    let tmm = Multilayer::new(
        c64::new(cover, 0.0),
        &[(c64::new(film, 0.0), Length::um(0.22))],
        c64::new(substrate, 0.0),
    )
    .unwrap()
    .reflection(kind, lam, angle)
    .unwrap();
    FilmRun {
        full,
        empty,
        grid,
        exact: (tmm.reflectance, tmm.transmittance),
    }
}

/// R and T from a run's fluxes through planes in the air above the source (the scattered
/// field), in the air between the source and the film (the incident one) and in the oxide; and
/// the exact ones.
pub(crate) fn film_ratios(run: &FilmRun) -> ((f64, f64), (f64, f64)) {
    let incident = -run.empty.flux(Axis::Z, run.plane(0.4));
    let reflected = run
        .full
        .minus(&run.empty)
        .unwrap()
        .flux(Axis::Z, run.plane(1.0));
    let transmitted = -run.full.flux(Axis::Z, run.plane(-0.3));
    ((reflected / incident, transmitted / incident), run.exact)
}

/// The largest difference between the fluxes through the planes of the film run between
/// z = −0.55 and 0.55 µm (oxide, the film, air, no source), relative to their mean.
pub(crate) fn flux_spread(run: &FilmRun) -> f64 {
    let fluxes: Vec<f64> = (run.plane(-0.55)..run.plane(0.55))
        .map(|p| run.full.flux(Axis::Z, p))
        .collect();
    let mean = fluxes.iter().sum::<f64>() / fluxes.len() as f64;
    fluxes.iter().map(|f| (f - mean).abs()).fold(0.0, f64::max) / mean.abs()
}

/// A plane wave in uniform oxide at 1.55 µm, 17° off the z axis in a plane 30° from x, launched
/// down into a PML of 20 cells graded to R = 1e-8 on a 20 nm grid: the amplitude of what comes
/// back up, relative to what went down, in E_x.
pub(crate) fn pml_reflection(kind: Kind) -> f64 {
    let h = 0.02;
    let grid = Grid3d {
        nx: 1,
        ny: 1,
        nz: 200,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let k0 = 2.0 * std::f64::consts::PI / 1.55;
    let kt = k0 * 1.444 * 0.3;
    let (kx, ky) = (
        kt * AZIMUTH.to_radians().cos(),
        kt * AZIMUTH.to_radians().sin(),
    );
    let boundaries = Boundaries3d {
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: ky },
        z: Edges::Pml {
            low: PML,
            high: PML,
        },
        reflection: 1e-8,
        order: 3.0,
    };
    let solver = Solver3d::new(
        grid,
        Wavelength::um(1.55).unwrap(),
        |_, _, _| c64::new(1.444 * 1.444, 0.0),
        boundaries,
    )
    .unwrap();
    let field = solver.solve(&sheet(&grid, (kx, ky), kind, 100)).unwrap();
    // between the source and the bottom PML the field is the downgoing wave and whatever the
    // PML sends back: separate them by the grid's own wavenumber along z
    let t = |k: f64| (2.0 / h * (k * h / 2.0).sin()).powi(2);
    let kz = ((k0 * 1.444).powi(2) - t(kx) - t(ky)).sqrt();
    let kd = 2.0 * (kz * h / 2.0).asin() / h;
    let (k1, k2) = (40, 60);
    let (u1, u2) = (field.e(Axis::X, (0, 0, k1)), field.e(Axis::X, (0, 0, k2)));
    let z = |k: usize| grid.node(Axis::Z, k);
    let wave = |sign: f64, k: usize| c64::from_polar(1.0, sign * kd * z(k));
    // u = a e^{-i kd z} + b e^{+i kd z}
    let (e1m, e1p, e2m, e2p) = (wave(-1.0, k1), wave(1.0, k1), wave(-1.0, k2), wave(1.0, k2));
    let det = e1m * e2p - e1p * e2m;
    let a = (u1 * e2p - e1p * u2) / det;
    let b = (e1m * u2 - u1 * e2m) / det;
    (b / a).norm()
}

/// A structure invariant along z (a silicon rod, 0.4 × 0.3 µm, in lossy oxide) in a cell
/// Bloch-periodic along x and y, on a 25 nm grid, solved by the 2D solver and by the 3D one with
/// one cell along z: the largest difference between the two fields relative to the largest
/// field. E along z: E_z for an electric line current; H along z: H̃_z for a magnetic one.
///
/// The 3D grid is placed so that its E_z (for E along z) or its H_z (for H along z) sits at the
/// 2D cells' centres; the averaging and the PML-free, periodic boundaries are then the same, and
/// the two discrete systems are the same equations, one eliminating H and the other E.
pub(crate) fn two_d_difference(polarization: Polarization) -> f64 {
    let h = 0.025;
    let (nx, ny) = (36, 28);
    let lam = Wavelength::um(1.55).unwrap();
    let (kx, ky) = (1.3, -0.7);
    let grid = Grid {
        nx,
        ny,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
    };
    let rod = |x: f64, y: f64| {
        let inside = (0.25..0.65).contains(&x) && (0.2..0.5).contains(&y);
        if inside {
            c64::new(3.476f64.powi(2), 0.0)
        } else {
            // the loss keeps a periodic problem without PMLs away from resonance
            c64::new(1.444f64.powi(2), 0.3)
        }
    };
    let boundaries = Boundaries {
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: ky },
        reflection: 1e-8,
        order: 3.0,
    };
    // a line source at one cell, and a weaker one at another
    let mut source = vec![c64::new(0.0, 0.0); nx * ny];
    source[5 * nx + 7] = c64::new(1.0, 0.0);
    source[20 * nx + 30] = c64::new(-0.4, 0.2);
    let flat = Solver2d::new(grid, polarization, lam, rod, boundaries)
        .unwrap()
        .solve(&source)
        .unwrap();
    let shift = match polarization {
        Polarization::Ez => 0.5 * h,
        Polarization::Hz => 0.0,
    };
    let grid3 = Grid3d {
        nx,
        ny,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: shift,
        y0: shift,
        z0: 0.0,
    };
    let boundaries3 = Boundaries3d {
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: ky },
        z: Edges::Bloch { k: 0.0 },
        reflection: 1e-8,
        order: 3.0,
    };
    let solver = Solver3d::new(grid3, lam, |x, y, _| rod(x, y), boundaries3).unwrap();
    let mut current = vec![c64::new(0.0, 0.0); grid3.unknowns()];
    for j in 0..ny {
        for i in 0..nx {
            current[grid3.index(Axis::Z, (i, j, 0))] = source[j * nx + i];
        }
    }
    let k0 = 2.0 * std::f64::consts::PI / 1.55;
    let deep: Box<dyn Fn(usize, usize) -> c64> = match polarization {
        Polarization::Ez => {
            let field = solver.solve(&current).unwrap();
            Box::new(move |i, j| field.e(Axis::Z, (i, j, 0)))
        }
        Polarization::Hz => {
            let field = solver.solve_magnetic(&current).unwrap();
            let m = source.clone();
            // where the magnetic current flows H̃ = (∇ × E + M) / (i k0)
            Box::new(move |i, j| field.h(Axis::Z, (i, j, 0)) + m[j * nx + i] / c64::new(0.0, k0))
        }
    };
    let mut worst: f64 = 0.0;
    let mut largest: f64 = 0.0;
    for j in 0..ny {
        for i in 0..nx {
            worst = worst.max((deep(i, j) - flat.at(i, j)).norm());
            largest = largest.max(flat.at(i, j).norm());
        }
    }
    worst / largest
}

/// Shin and Fan's 2D test system (Opt. Express 21, 22578 (2013), their Fig. 1): a square of
/// vacuum, 50 × 50 cells of 2 nm, periodic in x and y and uniform along z (one cell), an
/// x-polarized dipole at its centre, 1.55 µm. QMR on their Eq. 7 with this `s`, from zero to a
/// relative residual of `tolerance`: how it went.
pub(crate) fn shin_fan_square(s: f64, tolerance: f64) -> crate::fdfd::Convergence {
    use crate::fdfd::krylov::{Sparse, Stopping, qmr};
    let h = 0.002;
    let grid = Grid3d {
        nx: 50,
        ny: 50,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let periodic = Edges::Bloch { k: 0.0 };
    let boundaries = Boundaries3d {
        x: periodic,
        y: periodic,
        z: periodic,
        reflection: 1e-8,
        order: 3.0,
    };
    let (lattice, eps) = Solver3d::setup(
        grid,
        Wavelength::um(1.55).unwrap(),
        |_, _, _| c64::new(1.0, 0.0),
        boundaries,
    )
    .unwrap();
    let mut b = vec![c64::new(0.0, 0.0); grid.unknowns()];
    b[grid.index(Axis::X, (25, 25, 0))] = c64::new(0.0, -lattice.k0);
    let matrix = Sparse::new(
        grid.unknowns(),
        lattice
            .assemble_with(&eps, s)
            .into_iter()
            .map(|t| (t.row, t.col, t.val)),
    );
    let stopping = Stopping {
        tolerance,
        max_iterations: 5000,
    };
    qmr(&matrix, &lattice.transformed_rhs(&eps, &b, s), stopping)
        .unwrap()
        .1
}

/// A silicon strip (0.5 × 0.22 µm) along x in oxide, 16³ cells of 40 nm with PMLs of 6 cells
/// all round, a current on one edge at the centre, solved by QMR on `formulation` to a relative
/// residual of 1e-10 and by the sparse direct solver: the largest difference between the two
/// fields relative to the largest field, and the iterations QMR took.
pub(crate) fn qmr_against_direct(formulation: super::Formulation) -> (f64, usize) {
    use crate::fdfd::Stopping;
    let (n, h) = (16, 0.04);
    let half = n as f64 * h / 2.0;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -half,
        y0: -half,
        z0: -half,
    };
    let strip = |_: f64, y: f64, z: f64| {
        let m: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
            3.476
        } else {
            1.444
        };
        c64::new(m * m, 0.0)
    };
    let lam = Wavelength::um(1.55).unwrap();
    let boundaries = Boundaries3d::pml(6);
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    source[grid.index(Axis::Y, (n / 2, n / 2, n / 2))] = c64::new(1.0, 0.0);
    let direct = Solver3d::new(grid, lam, strip, boundaries)
        .unwrap()
        .solve(&source)
        .unwrap();
    let stopping = Stopping {
        tolerance: 1e-10,
        max_iterations: 10_000,
    };
    let (iterative, how) = super::IterativeSolver3d::new(grid, lam, strip, boundaries, formulation)
        .unwrap()
        .solve(&source, stopping)
        .unwrap();
    let largest = direct.values().iter().map(|v| v.norm()).fold(0.0, f64::max);
    let worst = direct
        .values()
        .iter()
        .zip(iterative.values())
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max);
    (worst / largest, how.iterations)
}
