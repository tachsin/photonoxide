//! The setups that both the tests and the validation report run: a plane wave on a slab, and
//! the PML's own reflection.

use num_complex::Complex64 as c64;

use super::{Boundaries, Edges, Field2d, Grid, Polarization, Port, Side, Solver2d};
use crate::mode::multilayer::Multilayer;
use crate::units::{Length, Wavelength};

/// The PMLs' thickness in the slab and PML runs, cells.
pub(crate) const PML: usize = 20;

/// R and T from a run's fluxes, and the exact ones.
pub(crate) fn slab_ratios(run: &SlabRun) -> ((f64, f64), (f64, f64)) {
    let incident = -run.empty.flux_y(run.row(0.4));
    let reflected = run.full.minus(&run.empty).unwrap().flux_y(run.row(1.0));
    let transmitted = -run.full.flux_y(run.row(-0.3));
    ((reflected / incident, transmitted / incident), run.exact)
}

/// A plane wave from air onto 220 nm of silicon on oxide, at 1.55 µm: the run with the slab,
/// and the run with air everywhere. Layers lie along x; the film fills 0 < y < 0.22.
pub(crate) struct SlabRun {
    pub(crate) full: Field2d,
    pub(crate) empty: Field2d,
    pub(crate) grid: Grid,
    /// R and T by the transfer matrices.
    pub(crate) exact: (f64, f64),
}

impl SlabRun {
    /// The row whose centre is just below `y`.
    pub(crate) fn row(&self, y: f64) -> usize {
        ((y - self.grid.y0) / self.grid.dy).floor() as usize
    }
}

/// The runs, with PMLs of `pml.0` cells graded to reflect `pml.1`.
pub(crate) fn slab_run(
    polarization: Polarization,
    h: f64,
    angle: f64,
    pml: (usize, f64),
) -> SlabRun {
    let (pml_cells, pml_reflection) = pml;
    let (cover, film, substrate) = (1.0, 3.476, 1.444);
    let lam = Wavelength::um(1.55).unwrap();
    let k0 = 2.0 * std::f64::consts::PI / 1.55;
    // rows: PML, 0.6 um of oxide, the film, 1.2 um of air, PML; faces on the interfaces
    let below = (0.6 / h).round() as usize + pml_cells;
    let ny = below + (0.22 / h).round() as usize + (1.2 / h).round() as usize + pml_cells;
    let grid = Grid {
        nx: 4,
        ny,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: -(below as f64) * h,
    };
    let kx = k0 * cover * angle.sin();
    let boundaries = Boundaries {
        x: Edges::Bloch { k: kx },
        y: Edges::Pml {
            low: pml_cells,
            high: pml_cells,
        },
        reflection: pml_reflection,
        order: 3.0,
    };
    let layered = |y: f64| {
        let n = if y < 0.0 {
            substrate
        } else if y < 0.22 {
            film
        } else {
            cover
        };
        c64::new(n * n, 0.0)
    };
    let row = |y: f64| ((y - grid.y0) / h).floor() as usize;
    let source_row = row(0.62);
    let mut source = vec![c64::new(0.0, 0.0); grid.nx * grid.ny];
    for i in 0..grid.nx {
        source[source_row * grid.nx + i] = c64::from_polar(1.0, kx * grid.x(i));
    }
    let full = Solver2d::new(grid, polarization, lam, |_, y| layered(y), boundaries)
        .unwrap()
        .solve(&source)
        .unwrap();
    let empty = Solver2d::new(
        grid,
        polarization,
        lam,
        |_, _| c64::new(cover * cover, 0.0),
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
    .reflection(
        match polarization {
            Polarization::Ez => crate::mode::Polarization::Te,
            Polarization::Hz => crate::mode::Polarization::Tm,
        },
        lam,
        angle,
    )
    .unwrap();
    SlabRun {
        full,
        empty,
        grid,
        exact: (tmm.reflectance, tmm.transmittance),
    }
}

/// The largest difference between the fluxes through the rows of the slab run between
/// y = −0.55 and 0.55 µm (oxide, the film, air, no source), relative to their mean.
pub(crate) fn flux_spread(run: &SlabRun) -> f64 {
    let fluxes: Vec<f64> = (run.row(-0.55)..run.row(0.55))
        .map(|j| run.full.flux_y(j))
        .collect();
    let mean = fluxes.iter().sum::<f64>() / fluxes.len() as f64;
    fluxes.iter().map(|f| (f - mean).abs()).fold(0.0, f64::max) / mean.abs()
}

/// A plane wave in uniform oxide at 1.55 µm, 17° off the y axis, launched down into a PML of
/// 20 cells graded to R = 1e-8 on a 20 nm grid: the amplitude of what comes back up, relative
/// to what went down.
pub(crate) fn pml_reflection(polarization: Polarization) -> f64 {
    let h = 0.02;
    let grid = Grid {
        nx: 2,
        ny: 200,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
    };
    let k0 = 2.0 * std::f64::consts::PI / 1.55;
    let kx = k0 * 1.444 * 0.3;
    let boundaries = Boundaries {
        x: Edges::Bloch { k: kx },
        y: Edges::Pml {
            low: PML,
            high: PML,
        },
        reflection: 1e-8,
        order: 3.0,
    };
    let oxide = |_: f64, _: f64| c64::new(1.444 * 1.444, 0.0);
    let solver = Solver2d::new(
        grid,
        polarization,
        Wavelength::um(1.55).unwrap(),
        oxide,
        boundaries,
    )
    .unwrap();
    let mut source = vec![c64::new(0.0, 0.0); grid.nx * grid.ny];
    for i in 0..grid.nx {
        source[100 * grid.nx + i] = c64::from_polar(1.0, kx * grid.x(i));
    }
    let field = solver.solve(&source).unwrap();
    // between the source and the bottom PML the field is the downgoing wave and whatever
    // the PML sends back: separate them by their discrete wavenumber
    let ky = ((k0 * 1.444).powi(2) - (2.0 / h * (kx * h / 2.0).sin()).powi(2)).sqrt();
    let kd = 2.0 * (ky * h / 2.0).asin() / h; // the grid's own wavenumber
    let (j1, j2) = (40, 60);
    let (u1, u2) = (field.at(0, j1), field.at(0, j2));
    // u = a e^{-i kd y} + b e^{+i kd y}
    let (e1m, e1p) = (
        c64::from_polar(1.0, -kd * grid.y(j1)),
        c64::from_polar(1.0, kd * grid.y(j1)),
    );
    let (e2m, e2p) = (
        c64::from_polar(1.0, -kd * grid.y(j2)),
        c64::from_polar(1.0, kd * grid.y(j2)),
    );
    let det = e1m * e2p - e1p * e2m;
    let a = (u1 * e2p - e1p * u2) / det;
    let b = (e1m * u2 - u1 * e2m) / det;
    (b / a).norm()
}

/// A slab waveguide along x, centred on y = 0, `thickness(x)` µm of `core` index in `clad`,
/// on an h grid from x = 0 to `length` and y = ±1.5 µm, PMLs of 20 cells all round.
pub(crate) fn guide(
    polarization: Polarization,
    h: f64,
    length: f64,
    (core, clad): (f64, f64),
    thickness: impl Fn(f64) -> f64,
) -> Solver2d {
    let pml = 20;
    let nx = (length / h).round() as usize + 2 * pml;
    let ny = (3.0 / h).round() as usize + 2 * pml;
    let grid = Grid {
        nx,
        ny,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let eps = move |x: f64, y: f64| {
        let n = if y.abs() < thickness(x) / 2.0 {
            core
        } else {
            clad
        };
        c64::new(n * n, 0.0)
    };
    Solver2d::new(
        grid,
        polarization,
        Wavelength::um(1.55).unwrap(),
        eps,
        Boundaries::pml(pml),
    )
    .unwrap()
}

pub(crate) fn column(solver: &Solver2d, x: f64) -> usize {
    let g = solver.grid();
    ((x - g.x0) / g.dx).floor() as usize
}

/// The two ports of a guide: its fundamental modes at x = 0.3 (left) and 1.7 µm (right).
pub(crate) fn two_ports(solver: &Solver2d) -> [Port; 2] {
    let left = solver.port_modes(column(solver, 0.3), 1).unwrap().remove(0);
    let right = solver.port_modes(column(solver, 1.7), 1).unwrap().remove(0);
    [
        Port {
            mode: left,
            side: Side::Left,
        },
        Port {
            mode: right,
            side: Side::Right,
        },
    ]
}

/// A straight 220 nm silicon slab (3.476 in 1.444), 2 µm long, on a 20 nm grid: the largest of
/// |S11|, |S22|, |S21 − e^(iβL)| and |S12 − e^(iβL)|, L the ports' distance.
pub(crate) fn straight_guide_error(polarization: Polarization) -> f64 {
    let solver = guide(polarization, 0.02, 2.0, (3.476, 1.444), |_| 0.22);
    let ports = two_ports(&solver);
    let length = (ports[1].mode.column() - ports[0].mode.column()) as f64 * 0.02;
    let expected = (c64::new(0.0, 1.0) * ports[0].mode.beta() * length).exp();
    let s = solver.s_matrix(&ports).unwrap();
    [
        s[0][0].norm(),
        s[1][1].norm(),
        (s[1][0] - expected).norm(),
        (s[0][1] - expected).norm(),
    ]
    .into_iter()
    .fold(0.0, f64::max)
}

/// A slab (3.473 in 1.444) stepping from 220 to 300 nm at x = 1 µm, 10 nm grid: its S-matrix
/// between the two guides' fundamental modes, and their effective indices.
pub(crate) fn step(polarization: Polarization) -> (Vec<Vec<c64>>, f64, f64) {
    let solver = guide(polarization, 0.01, 2.0, (3.473, 1.444), |x| {
        if x < 1.0 { 0.22 } else { 0.30 }
    });
    let ports = two_ports(&solver);
    let s = solver.s_matrix(&ports).unwrap();
    let n = |p: &Port| p.mode.effective_index().re;
    (s, n(&ports[0]), n(&ports[1]))
}

/// The effective index of the fundamental port mode of a 220 nm silicon slab (3.476 in
/// 1.444) on an h grid, and the exact slab mode's.
pub(crate) fn port_mode_index(polarization: Polarization, h: f64) -> (f64, f64) {
    use crate::mode::slab::Slab;
    let (core, clad) = (3.476, 1.444);
    let kind = match polarization {
        Polarization::Ez => crate::mode::Polarization::Te,
        Polarization::Hz => crate::mode::Polarization::Tm,
    };
    let exact = Slab::new(clad, core, clad, Length::nm(220.0))
        .unwrap()
        .modes(kind, Wavelength::um(1.55).unwrap())[0]
        .effective_index();
    let solver = guide(polarization, h, 0.1, (core, clad), |_| 0.22);
    let mode = &solver.port_modes(column(&solver, 0.05), 1).unwrap()[0];
    (mode.effective_index().re, exact)
}
