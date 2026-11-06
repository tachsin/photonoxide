//! The charts of the lesson on FDFD and adjoint gradients, all on photonoxide's 2D FDFD
//! (`fdfd::Solver2d`) and the problems of its validation cases:
//!
//! - **Convergence** (`fdfd-convergence`): the fundamental port mode of a silicon slab (3.476 in
//!   1.444) on grids whose lines miss its faces by 0.3 and 0.7 of a cell, against the exact slab
//!   (`mode::slab::Slab`), with three ways of giving a cell the permittivity of a slab that cuts
//!   it: the solver's 8 × 8 samples, the material at the cell's centre (staircasing), and the
//!   mean over the cell (the fill).
//! - **The adjoint gradient** (`fdfd-gradient`): the slab of `fdfd/adjoint-gradient-ez` and -hz,
//!   with its bump beside it on a 20 nm grid: the power the left port's mode delivers into the
//!   right port's, its gradient with respect to every cell's permittivity from
//!   `Solver2d::mode_power_gradient` (one more back-substitution), the forward field, and the
//!   right port's mode launched backwards, which is the adjoint field in front of the monitor
//!   (Lalau-Keraly et al. 2013, Eq. 8; `fdtd/adjoint-backward-mode` checks it exactly).
//! - **Finite differences against the adjoint** (`fdfd-difference-step`): the same problem, the
//!   gradient at one cell against second- and fourth-order central differences of the power at
//!   steps from 10⁻⁶ to 0.3: truncation falling as δ² and δ⁴, round-off rising as 1/δ.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
use photonoxide::mode::Polarization as Kind;
use photonoxide::mode::slab::Slab;
use photonoxide::raster::Raster;
use photonoxide::units::{Length, Wavelength};
use rayon::prelude::*;

use super::{ChartData, ChartSpec, Curve, FieldMap, figure, invalid, param, sig};

/// Silicon and oxide as the library's FDFD checks take them (src/cases/fdfd/checks.rs).
const CORE: f64 = 3.476;
const CLAD: f64 = 1.444;
/// The wavelength of every chart here, µm.
const LAMBDA: f64 = 1.55;
/// PML cells on each side.
const PML: usize = 20;

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

fn polarization_param() -> super::Param {
    param(
        "polarization",
        "Field along $z$",
        "",
        (1.0, 2.0, 1.0),
        1.0,
        false,
        "1: E along z, the slab's TE mode; 2: H along z, its TM mode.",
    )
}

/// The chart specs.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "fdfd-convergence",
            title: "A port mode's error against the grid",
            about: "The fundamental mode of a silicon slab solved on a port column of 2D FDFD grids whose lines miss its faces by 0.3 and 0.7 of a cell, against the exact slab: its effective index's error as the grid step falls, for three ways of giving a cell that the slab cuts its permittivity. A slope of 1 is first order, 2 second order.",
            computed_by: "fdfd::Solver2d::new (8 × 8 samples per cell) and Solver2d::from_cells (the centre's material, or the mean over the cell), Solver2d::port_modes; mode::slab::Slab::modes for the exact index",
            params: vec![
                polarization_param(),
                param(
                    "thickness",
                    "Slab thickness $t$",
                    "nm",
                    (150.0, 400.0, 220.0),
                    1.0,
                    false,
                    "The silicon slab's thickness, nm, in oxide, at 1.55 µm.",
                ),
            ],
        },
        ChartSpec {
            id: "fdfd-gradient",
            title: "A forward field, its adjoint and the gradient",
            about: "A silicon slab with a bump of permittivity ε_b beside it, between two ports, 2D FDFD on 20 nm cells at 1.55 µm: the field the left port's mode makes, the right port's mode launched backwards (the adjoint field in front of the monitor), and the gradient of the power delivered into the right port's mode with respect to every cell's permittivity, inside the dashed design window, from one more back-substitution.",
            computed_by: "fdfd::Solver2d::from_cells, mode_source, solve_system, mode_power_gradient (the adjoint variable method); a fourth-order central difference at one cell by reuse_cells as the check",
            params: vec![
                polarization_param(),
                param(
                    "bump",
                    "Bump's permittivity $\\varepsilon_b$",
                    "",
                    (1.0, 14.0, 6.0),
                    0.1,
                    false,
                    "The permittivity of the 0.2 × 0.2 µm block beside the slab: 6 in the validation cases, 2.085 (oxide) for none.",
                ),
            ],
        },
        ChartSpec {
            id: "fdfd-difference-step",
            title: "Finite differences against the adjoint gradient",
            about: "The same slab and bump: the derivative of the delivered power with respect to one cell's permittivity by central differences of step δ, against the adjoint gradient, their relative difference as δ runs from 10⁻⁶ to 0.3. Large steps err by truncation, small ones by the powers' round-off.",
            computed_by: "fdfd::Solver2d::mode_power_gradient at the cell; second- and fourth-order central differences of the power from Solver2d::reuse_cells (4 solves a step); Solver2d::residual for the powers' precision",
            params: vec![
                polarization_param(),
                param(
                    "probe",
                    "Cell",
                    "",
                    (1.0, 3.0, 1.0),
                    1.0,
                    false,
                    "Which cell's permittivity: 1 in the bump, 2 in the core, 3 in the oxide below the slab (the validation cases' three).",
                ),
            ],
        },
    ]
}

fn polarization(code: f64) -> Polarization {
    if code < 1.5 {
        Polarization::Ez
    } else {
        Polarization::Hz
    }
}

fn wavelength() -> photonoxide::Result<Wavelength> {
    Wavelength::um(LAMBDA)
}

// ---------------------------------------------------------------------------------------------
// Convergence

/// How a cell that the slab cuts is given its permittivity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fill {
    /// The solver's own: 8 × 8 samples, harmonic along each component and arithmetic across.
    Samples,
    /// The material at the cell's centre: staircasing.
    Centre,
    /// The mean of the permittivity over the cell, exact for the slab.
    Mean,
}

/// Cells across the slab, plus 0.4: its faces lie 0.3 and 0.7 of a cell off the grid's lines.
/// About √2 apart, so a doubling of the resolution is two steps along them.
pub const ACROSS: [usize; 11] = [4, 6, 8, 11, 16, 23, 32, 45, 64, 91, 128];

/// The grid the order is fitted from with the finest: half its cells, 64.
const HALF: usize = 8;

/// Cladding on each side of the slab in the solves, µm.
const SIDE: f64 = 1.5;

/// The effective index of the fundamental port mode of a slab `t` µm thick on a grid of
/// `t/(n + 0.4)`, its cells set by `fill`.
pub fn port_index(pol: Polarization, t: f64, n: usize, fill: Fill) -> photonoxide::Result<f64> {
    let h = t / (n as f64 + 0.4);
    // m cells and 0.3 of one below the slab, as many above; the faces cut cells m and m + n
    let m = (SIDE / h).round() as usize + PML;
    let ny = 2 * m + n + 1;
    // a short guide: the port's column and the next two cells clear of each PML
    let nx = 2 * PML + 6;
    let grid = Grid {
        nx,
        ny,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: -t / 2.0 - (m as f64 + 0.3) * h,
    };
    let (core, clad) = (CORE * CORE, CLAD * CLAD);
    let lambda = wavelength()?;
    let solver = match fill {
        Fill::Samples => Solver2d::new(
            grid,
            pol,
            lambda,
            move |_, y| c64::new(if y.abs() < t / 2.0 { core } else { clad }, 0.0),
            Boundaries::pml(PML),
        )?,
        Fill::Centre | Fill::Mean => {
            let row = |j: usize| {
                let (lo, hi) = (grid.y0 + j as f64 * h, grid.y0 + (j + 1) as f64 * h);
                let inside = (hi.min(t / 2.0) - lo.max(-t / 2.0)).max(0.0) / h;
                let share = match fill {
                    Fill::Centre => {
                        if ((lo + hi) / 2.0).abs() < t / 2.0 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    _ => inside,
                };
                c64::new(clad + share * (core - clad), 0.0)
            };
            let cells: Vec<c64> = (0..ny)
                .flat_map(|j| std::iter::repeat_n(row(j), nx))
                .collect();
            Solver2d::from_cells(grid, pol, lambda, &cells, Boundaries::pml(PML))?
        }
    };
    let mode = solver
        .port_modes(PML + 2, 1)?
        .into_iter()
        .next()
        .ok_or_else(|| invalid("fdfd-convergence", "no port mode"))?;
    Ok(mode.effective_index().re)
}

/// The exact slab's fundamental effective index.
pub fn exact_index(pol: Polarization, t: f64) -> photonoxide::Result<f64> {
    let kind = match pol {
        Polarization::Ez => Kind::Te,
        Polarization::Hz => Kind::Tm,
    };
    Slab::new(CLAD, CORE, CLAD, Length::um(t))?
        .modes(kind, wavelength()?)
        .first()
        .map(|m| m.effective_index())
        .ok_or_else(|| invalid("fdfd-convergence", "the slab guides no mode"))
}

/// Each fill's errors |n_eff − exact| on the grids of [`ACROSS`], and the exact index; cached.
pub fn errors(pol: Polarization, t: f64) -> photonoxide::Result<([Vec<f64>; 3], f64)> {
    type Key = (u8, u64);
    type Value = ([Vec<f64>; 3], f64);
    static CACHE: OnceLock<Mutex<HashMap<Key, Value>>> = OnceLock::new();
    let key = (pol as u8, t.to_bits());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return Ok(v);
    }
    let exact = exact_index(pol, t)?;
    let fills = [Fill::Samples, Fill::Centre, Fill::Mean];
    let jobs: Vec<(usize, usize)> = (0..3)
        .flat_map(|f| (0..ACROSS.len()).map(move |k| (f, k)))
        .collect();
    let found: Vec<photonoxide::Result<f64>> = jobs
        .par_iter()
        .map(|&(f, k)| port_index(pol, t, ACROSS[k], fills[f]).map(|n| (n - exact).abs()))
        .collect();
    let mut out: [Vec<f64>; 3] = Default::default();
    for (&(f, _), e) in jobs.iter().zip(found) {
        out[f].push(e?);
    }
    let value = (out, exact);
    if let Ok(mut c) = cache.lock() {
        c.insert(key, value.clone());
    }
    Ok(value)
}

/// The order of convergence between the finest grid and the one of half its cells: the log of
/// the errors' ratio over the log of the steps' ratio.
pub fn finest_order(errors: &[f64]) -> f64 {
    let k = ACROSS.len() - 1;
    let step = |n: usize| 1.0 / (n as f64 + 0.4);
    (errors[HALF] / errors[k]).ln() / (step(ACROSS[HALF]) / step(ACROSS[k])).ln()
}

pub fn convergence(code: f64, thickness_nm: f64) -> photonoxide::Result<ChartData> {
    let pol = polarization(code);
    let t = thickness_nm * 1e-3;
    let (errors, exact) = errors(pol, t)?;
    let labels = [
        "8 × 8 samples a cell (the solver's default)",
        "each cell its centre's material (staircasing)",
        "each cell the mean over it (the fill)",
    ];
    let mut series = Vec::new();
    let mut figures = Vec::new();
    for (f, e) in errors.iter().enumerate() {
        let points = ACROSS
            .iter()
            .zip(e)
            .map(|(&n, &err)| [(1e3 * t / (n as f64 + 0.4)).log10(), err])
            .collect();
        series.push(curve(labels[f], points, false, f));
        let short = ["8 × 8 samples", "staircasing", "the mean"][f];
        figures.push(figure(
            &format!("order, {short}"),
            finest_order(e),
            3,
            "",
            &format!(
                "between grids of {} and {} nm; error {} on the finest",
                sig(1e3 * t / (ACROSS[HALF] as f64 + 0.4), 3),
                sig(1e3 * t / (ACROSS[ACROSS.len() - 1] as f64 + 0.4), 3),
                sig(e[e.len() - 1], 2)
            ),
        ));
    }
    figures.push(figure(
        "exact $n_\\text{eff}$",
        exact,
        6,
        "",
        "the slab's transfer-matrix mode, exact: no grid",
    ));
    let field = match pol {
        Polarization::Ez => "E along z (TE)",
        Polarization::Hz => "H along z (TM)",
    };
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "grid step: log₁₀(h / 1 nm)".to_owned(),
        x_length: false,
        y_label: "error in n_eff".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: None,
        figures,
        note: format!(
            "2D FDFD port modes, {field}, {} nm of silicon (3.476) in oxide (1.444) at 1.55 µm, on \
             grids of {} to {} cells across it, the slab's faces 0.3 and 0.7 of a cell off the \
             grid's lines; against the exact slab. The x axis is the logarithm of the step: a \
             line of slope 1 is first order, of slope 2 second order.",
            sig(thickness_nm, 3),
            ACROSS[0] as f64 + 0.4,
            ACROSS[ACROSS.len() - 1] as f64 + 0.4
        ),
    })
}

// ---------------------------------------------------------------------------------------------
// The adjoint gradient

/// The cell size of the gradient's problem, µm.
const H: f64 = 0.02;
/// The design window, where the gradient is drawn: x and y, µm.
const WINDOW: [[f64; 2]; 2] = [[0.5, 1.5], [-0.6, 0.6]];
/// The pictures' extent: x and y, µm.
const VIEW: [[f64; 2]; 2] = [[0.0, 2.0], [-0.8, 0.8]];

/// The slab and bump of `fdfd/adjoint-gradient-ez` and -hz: 220 nm of silicon in oxide along x,
/// a 0.2 × 0.2 µm block of `bump` beside it at x = 0.9 to 1.1 µm, y = 0.11 to 0.31 µm, on a
/// 20 nm grid from x = 0 to 2 µm and y = ±1.5 µm, PMLs of 20 cells all round.
pub struct Problem {
    pub grid: Grid,
    pub cells: Vec<c64>,
    pub solver: Solver2d,
    pub right: PortMode,
    pub source: Vec<c64>,
}

impl Problem {
    pub fn new(pol: Polarization, bump: f64) -> photonoxide::Result<Problem> {
        let grid = Grid {
            nx: (2.0 / H).round() as usize + 2 * PML,
            ny: (3.0 / H).round() as usize + 2 * PML,
            dx: H,
            dy: H,
            x0: -(PML as f64) * H,
            y0: -1.5 - PML as f64 * H,
        };
        let cells: Vec<c64> = (0..grid.ny)
            .flat_map(|j| (0..grid.nx).map(move |i| (i, j)))
            .map(|(i, j)| {
                let (x, y) = (grid.x(i), grid.y(j));
                let e = if y.abs() < 0.11 {
                    CORE * CORE
                } else if (0.9..1.1).contains(&x) && (0.11..0.31).contains(&y) {
                    bump
                } else {
                    CLAD * CLAD
                };
                c64::new(e, 0.0)
            })
            .collect();
        let solver = Solver2d::from_cells(grid, pol, wavelength()?, &cells, Boundaries::pml(PML))?;
        let column = |x: f64| ((x - grid.x0) / H).floor() as usize;
        let mode = |x: f64| -> photonoxide::Result<PortMode> {
            solver
                .port_modes(column(x), 1)?
                .into_iter()
                .next()
                .ok_or_else(|| invalid("fdfd-gradient", "no port mode"))
        };
        let (left, right) = (mode(0.3)?, mode(1.7)?);
        let source = solver.mode_source(&left, Direction::Forward);
        Ok(Problem {
            grid,
            cells,
            solver,
            right,
            source,
        })
    }

    /// The cell holding (x, y), as `[j * nx + i]`.
    pub fn cell(&self, x: f64, y: f64) -> usize {
        let i = ((x - self.grid.x0) / H).floor() as usize;
        let j = ((y - self.grid.y0) / H).floor() as usize;
        j * self.grid.nx + i
    }

    /// The power the right port's mode carries forward with cell `k`'s permittivity moved by `d`.
    pub fn power_with(&self, k: usize, d: f64) -> photonoxide::Result<f64> {
        let mut varied = self.cells.clone();
        varied[k] += d;
        let solver = self.solver.reuse_cells(wavelength()?, &varied)?;
        let field = solver.solve_system(&self.source)?;
        Ok(field.mode_amplitudes(&self.right).0.norm_sqr())
    }
}

/// The validation cases' three probes: a cell in the bump, one in the core, one in the oxide.
pub const PROBES: [(f64, f64); 3] = [(1.0, 0.2), (1.0, 0.0), (1.2, -0.2)];

/// The adjoint gradient's chart data: the forward field, the backward mode and the gradient,
/// each over [`VIEW`], the power and the gradient at the bump's probe.
#[derive(Clone)]
pub struct Gradient {
    pub power: f64,
    pub forward: Raster,
    pub backward: Raster,
    pub gradient: Raster,
    pub probe: f64,
    /// The same derivative by a fourth-order central difference, δ = 4e-3.
    pub differences: f64,
    pub largest: (f64, f64, f64),
    pub window_cells: usize,
}

pub fn gradient_data(pol: Polarization, bump: f64) -> photonoxide::Result<Gradient> {
    type Key = (u8, u64);
    static CACHE: OnceLock<Mutex<HashMap<Key, Gradient>>> = OnceLock::new();
    let key = (pol as u8, bump.to_bits());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return Ok(v);
    }
    let p = Problem::new(pol, bump)?;
    let field = p.solver.solve_system(&p.source)?;
    let (power, gradient) = p
        .solver
        .mode_power_gradient(&field, &p.right, Direction::Forward)?;
    let back = p
        .solver
        .solve_system(&p.solver.mode_source(&p.right, Direction::Backward))?;
    let g = p.grid;
    let span = |[lo, hi]: [f64; 2], origin: f64| {
        let a = ((lo - origin) / H).round() as usize;
        let b = ((hi - origin) / H).round() as usize;
        a..b
    };
    let (is, js) = (span(VIEW[0], g.x0), span(VIEW[1], g.y0));
    let inside = |i: usize, j: usize| {
        let (x, y) = (g.x(i), g.y(j));
        (WINDOW[0][0]..WINDOW[0][1]).contains(&x) && (WINDOW[1][0]..WINDOW[1][1]).contains(&y)
    };
    let raster = |value: &dyn Fn(usize, usize) -> f64, signed: bool| -> Raster {
        let mut values = Vec::with_capacity(is.len() * js.len());
        for j in js.clone() {
            for i in is.clone() {
                values.push(value(i, j));
            }
        }
        let top = values
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()))
            .max(1e-300);
        Raster {
            nx: is.len(),
            ny: js.len(),
            x0: VIEW[0][0],
            x1: VIEW[0][1],
            y0: VIEW[1][0],
            y1: VIEW[1][1],
            values: values
                .iter()
                .map(|v| {
                    if signed {
                        (0.5 + 0.5 * v / top) as f32
                    } else {
                        (v / top) as f32
                    }
                })
                .collect(),
        }
    };
    let forward = raster(&|i, j| field.at(i, j).norm(), false);
    let backward = raster(&|i, j| back.at(i, j).norm(), false);
    let in_window = |i: usize, j: usize| {
        if inside(i, j) {
            gradient[j * g.nx + i]
        } else {
            0.0
        }
    };
    let gradient_map = raster(&in_window, true);
    let mut largest: (f64, f64, f64) = (0.0, 0.0, 0.0);
    let mut window_cells = 0;
    for j in 0..g.ny {
        for i in 0..g.nx {
            if inside(i, j) {
                window_cells += 1;
                let v = gradient[j * g.nx + i];
                if v.abs() > largest.0.abs() {
                    largest = (v, g.x(i), g.y(j));
                }
            }
        }
    }
    // the check: the bump's cell by fourth-order central differences, δ = 4e-3 as the
    // validation cases take it
    let k = p.cell(PROBES[0].0, PROBES[0].1);
    let (differences, _) = fourth_order(&p, k, 4e-3)?;
    let value = Gradient {
        power,
        forward,
        backward,
        gradient: gradient_map,
        probe: gradient[k],
        differences,
        largest,
        window_cells,
    };
    if let Ok(mut c) = cache.lock() {
        c.insert(key, value.clone());
    }
    Ok(value)
}

/// The fourth-order central difference of the power at cell `k`, step `delta`, and the four
/// powers it takes.
pub fn fourth_order(p: &Problem, k: usize, delta: f64) -> photonoxide::Result<(f64, [f64; 4])> {
    let at: Vec<f64> = [-2.0, -1.0, 1.0, 2.0]
        .par_iter()
        .map(|s| p.power_with(k, s * delta))
        .collect::<photonoxide::Result<_>>()?;
    let [m2, m1, p1, p2] = [at[0], at[1], at[2], at[3]];
    Ok((
        (-p2 + 8.0 * p1 - 8.0 * m1 + m2) / (12.0 * delta),
        [m2, m1, p1, p2],
    ))
}

fn rectangle([[x0, x1], [y0, y1]]: [[f64; 2]; 2]) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
}

pub fn gradient(code: f64, bump: f64) -> photonoxide::Result<ChartData> {
    let pol = polarization(code);
    let d = gradient_data(pol, bump)?;
    let differences = d.differences;
    let outline = vec![
        rectangle([[VIEW[0][0], VIEW[0][1]], [-0.11, 0.11]]),
        rectangle([[0.9, 1.1], [0.11, 0.31]]),
        rectangle(WINDOW),
    ];
    let name = match pol {
        Polarization::Ez => "E_z",
        Polarization::Hz => "H_z",
    };
    let maps = vec![
        FieldMap {
            label: format!("forward: $\\lvert {name}\\rvert$, the left port's mode sent in"),
            raster: d.forward,
            outline: outline.clone(),
        },
        FieldMap {
            label: format!(
                "adjoint: $\\lvert {name}\\rvert$, the right port's mode sent back from its monitor"
            ),
            raster: d.backward,
            outline: outline.clone(),
        },
        FieldMap {
            label: "$\\partial F/\\partial\\varepsilon$ in the window: the middle of the scale is 0, brighter positive, darker negative".to_owned(),
            raster: d.gradient,
            outline,
        },
    ];
    let figures = vec![
        figure(
            "$F$, power into the right port's mode",
            d.power,
            6,
            "",
            "of the left port's mode's unit power",
        ),
        figure(
            "the window's largest $\\partial F/\\partial\\varepsilon$ in size",
            d.largest.0,
            4,
            "",
            &format!(
                "per unit of a cell's relative permittivity, at x = {} µm, y = {} µm",
                sig(d.largest.1, 3),
                sig(d.largest.2, 3)
            ),
        ),
        figure(
            "$\\partial F/\\partial\\varepsilon$ in the bump, adjoint",
            d.probe,
            8,
            "",
            "at the cell (1.0, 0.2) µm: the forward solve and one back-substitution",
        ),
        figure(
            "the same by finite differences",
            differences,
            8,
            "",
            &format!(
                "fourth-order central, δ = 4 × 10⁻³: 4 more solves; relative difference {}",
                sig((differences - d.probe).abs() / d.probe.abs().max(1e-300), 2)
            ),
        ),
        figure(
            "cells in the window",
            d.window_cells as f64,
            5,
            "",
            &format!(
                "central differences for all of them: {} solves; the adjoint: 1",
                2 * d.window_cells
            ),
        ),
    ];
    Ok(ChartData {
        maps,
        x_label: "x".to_owned(),
        x_length: false,
        y_label: "y".to_owned(),
        y_log: false,
        y_range: None,
        series: Vec::new(),
        marker: None,
        figures,
        note: format!(
            "2D FDFD, {}, 20 nm cells, PMLs of 20, at 1.55 µm: 220 nm of silicon (3.476) in oxide \
             (1.444), a 0.2 × 0.2 µm block of ε = {} beside it, ports at x = 0.3 and 1.7 µm. \
             Each picture is scaled to its own largest value; the gradient only inside the dashed \
             window, away from the ports, whose sources and modes are held fixed.",
            match pol {
                Polarization::Ez => "E along z",
                Polarization::Hz => "H along z",
            },
            sig(bump, 3)
        ),
    })
}

// ---------------------------------------------------------------------------------------------
// Finite differences against the adjoint

/// The steps, as log₁₀ δ: every half decade from 10⁻⁶ to 10⁻⁰·⁵.
pub const STEPS: [f64; 12] = [
    -6.0, -5.5, -5.0, -4.5, -4.0, -3.5, -3.0, -2.5, -2.0, -1.5, -1.0, -0.5,
];

/// The differences at one probe: the adjoint gradient, the power, the relative residual of the
/// forward solve, and for each step of [`STEPS`] the second- and fourth-order differences.
#[derive(Clone)]
pub struct Steps {
    pub gradient: f64,
    pub power: f64,
    pub residual: f64,
    pub second: Vec<f64>,
    pub fourth: Vec<f64>,
}

pub fn steps(pol: Polarization, probe: usize) -> photonoxide::Result<Steps> {
    type Key = (u8, usize);
    static CACHE: OnceLock<Mutex<HashMap<Key, Steps>>> = OnceLock::new();
    let key = (pol as u8, probe);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return Ok(v);
    }
    let p = Problem::new(pol, 6.0)?;
    let field = p.solver.solve_system(&p.source)?;
    let (power, gradient) = p
        .solver
        .mode_power_gradient(&field, &p.right, Direction::Forward)?;
    let residual = p.solver.residual(&field, &p.source)?;
    let (x, y) = PROBES[probe];
    let k = p.cell(x, y);
    let found: Vec<(f64, f64)> = STEPS
        .par_iter()
        .map(|&e| {
            let delta = 10f64.powf(e);
            let (fourth, [_, m1, p1, _]) = fourth_order(&p, k, delta)?;
            Ok(((p1 - m1) / (2.0 * delta), fourth))
        })
        .collect::<photonoxide::Result<_>>()?;
    let value = Steps {
        gradient: gradient[k],
        power,
        residual,
        second: found.iter().map(|f| f.0).collect(),
        fourth: found.iter().map(|f| f.1).collect(),
    };
    if let Ok(mut c) = cache.lock() {
        c.insert(key, value.clone());
    }
    Ok(value)
}

/// The relative error the fourth-order formula's round-off allows at step δ, as the validation
/// cases bound it (src/cases/fdfd/checks.rs, `Probe::resolution`): the powers' precision u,
/// twice the solve's relative residual rounded up to a power of ten, through the formula's
/// weights, (1 + 8 + 8 + 1)/12 · u · P / (δ |g|).
pub fn resolution(s: &Steps, delta: f64) -> f64 {
    let u = 10f64.powf((2.0 * s.residual).log10().ceil());
    18.0 / 12.0 * u * s.power.abs() / (delta * s.gradient.abs())
}

pub fn difference_step(code: f64, probe: f64) -> photonoxide::Result<ChartData> {
    let pol = polarization(code);
    let probe = (probe.round() as usize).clamp(1, 3) - 1;
    let s = steps(pol, probe)?;
    let rel = |d: f64| (d - s.gradient).abs() / s.gradient.abs().max(1e-300);
    let second: Vec<[f64; 2]> = STEPS
        .iter()
        .zip(&s.second)
        .map(|(&e, &d)| [e, rel(d)])
        .collect();
    let fourth: Vec<[f64; 2]> = STEPS
        .iter()
        .zip(&s.fourth)
        .map(|(&e, &d)| [e, rel(d)])
        .collect();
    let bound: Vec<[f64; 2]> = STEPS
        .iter()
        .map(|&e| [e, resolution(&s, 10f64.powf(e))])
        .collect();
    let best = fourth
        .iter()
        .copied()
        .filter(|p| p[1] > 0.0)
        .min_by(|a, b| a[1].total_cmp(&b[1]))
        .unwrap_or([0.0, 0.0]);
    let where_ = ["in the bump", "in the core", "in the oxide"][probe];
    let figures = vec![
        figure(
            "$\\partial F/\\partial\\varepsilon$, adjoint",
            s.gradient,
            8,
            "",
            &format!(
                "the cell {where_}, at ({}, {}) µm",
                PROBES[probe].0, PROBES[probe].1
            ),
        ),
        figure(
            "smallest difference, fourth order",
            best[1],
            2,
            "",
            &format!("relative, at δ = {}", sig(10f64.powf(best[0]), 2)),
        ),
        figure(
            "the forward solve's relative residual",
            s.residual,
            2,
            "",
            "‖b − Ax‖/‖b‖, which sets the powers' precision",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "step: log₁₀ δ".to_owned(),
        x_length: false,
        y_label: "relative difference from the adjoint".to_owned(),
        y_log: true,
        y_range: None,
        series: vec![
            curve("second-order central difference", second, false, 0),
            curve("fourth-order central difference", fourth, false, 1),
            curve(
                "what the fourth order can resolve (round-off)",
                bound,
                true,
                2,
            ),
        ],
        marker: Some((4e-3f64).log10()),
        figures,
        note: format!(
            "2D FDFD, {}, the slab and bump of the gradient chart (ε = 6, 20 nm cells): the power's \
             derivative with respect to one cell's permittivity, the adjoint's against central \
             differences, 4 solves a step. The marker is the validation cases' δ = 4 × 10⁻³. A \
             difference below the dashed line is all the differences can tell.",
            match pol {
                Polarization::Ez => "E along z",
                Polarization::Hz => "H along z",
            }
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The port modes converge to the exact slab: with the mean over each cell at second order
    /// for E along z (`fdfd/fill-slab-te-mean`'s 2.00), staircased at first order
    /// (`fdfd/fill-slab-te-point`'s 1.01); the solver's 8 × 8 samples place a face within a
    /// sixteenth of a cell, an error of first order with a small constant.
    #[test]
    fn the_port_modes_converge_at_the_orders_of_their_averages() {
        let (e, exact) = errors(Polarization::Ez, 0.22).unwrap();
        // the exact TE index of 220 nm of 3.476 in 1.444 at 1.55 µm (fdfd/port-mode-te: 2.84778)
        assert!((exact - 2.84778).abs() < 1e-5, "{exact}");
        let [samples, centre, mean] = [&e[0], &e[1], &e[2]].map(|v| finest_order(v));
        assert!((mean - 2.0).abs() < 0.2, "the mean: {mean}");
        assert!((centre - 1.0).abs() < 0.2, "staircasing: {centre}");
        assert!(samples > 0.8, "8 × 8 samples: {samples}");
        // on the finest grid the mean is the most accurate and staircasing the least
        let last = ACROSS.len() - 1;
        assert!(e[2][last] < e[0][last] && e[0][last] < e[1][last], "{e:?}");
        // with H along z the mean is the wrong average across the slab: first order
        // (fdfd/fill-slab-tm-mean's 1.02)
        let (h, _) = errors(Polarization::Hz, 0.22).unwrap();
        let tm_mean = finest_order(&h[2]);
        assert!((tm_mean - 1.0).abs() < 0.2, "TM, the mean: {tm_mean}");
    }

    /// The gradient chart's adjoint derivative at the bump's cell is the fourth-order central
    /// difference to the validation cases' tolerance (fdfd/adjoint-gradient-ez and -hz: 1e-6).
    #[test]
    fn the_gradient_is_the_finite_differences() {
        for code in [1.0, 2.0] {
            let data = gradient(code, 6.0).unwrap();
            let adjoint = data.figures[2].value.unwrap();
            let differences = data.figures[3].value.unwrap();
            let rel = (adjoint - differences).abs() / adjoint.abs();
            assert!(rel < 1e-6, "{code}: {adjoint} against {differences}: {rel}");
            assert_eq!(data.maps.len(), 3);
            // the window's cells: 50 × 60 of 20 nm
            assert_eq!(data.figures[4].value, Some(3000.0));
        }
    }

    /// Finite differences against the adjoint: the second order's truncation falls as δ² and the
    /// fourth's as δ⁴ at large steps, and at the validation cases' step the fourth order meets
    /// the adjoint within what its round-off resolves.
    #[test]
    fn the_differences_converge_to_the_adjoint_until_round_off() {
        let s = steps(Polarization::Ez, 0).unwrap();
        let rel = |d: f64| (d - s.gradient).abs() / s.gradient.abs();
        // the second order's slope from δ = 10⁻¹·⁵ to 10⁻⁰·⁵, a decade: its truncation, δ²
        let n = STEPS.len();
        let two = (rel(s.second[n - 1]) / rel(s.second[n - 3])).log10();
        assert!((two - 2.0).abs() < 0.3, "second order: {two}");
        // the fourth order truncates far less at the largest step
        assert!(
            rel(s.fourth[n - 1]) < 0.1 * rel(s.second[n - 1]),
            "{} against {}",
            rel(s.fourth[n - 1]),
            rel(s.second[n - 1])
        );
        // at δ = 10⁻² · 10^0.5 ≈ 3.2e-3, near the cases' 4e-3: within the round-off bound
        let k = STEPS.iter().position(|&e| e == -2.5).unwrap();
        let delta = 10f64.powf(-2.5);
        assert!(
            rel(s.fourth[k]) <= resolution(&s, delta),
            "{} against {}",
            rel(s.fourth[k]),
            resolution(&s, delta)
        );
        assert!(rel(s.fourth[k]) < 1e-6, "{}", rel(s.fourth[k]));
    }
}
