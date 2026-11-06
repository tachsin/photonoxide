//! The FDTD lesson's charts: the grid's numerical dispersion (`fdtd::grid_wavenumber`), the
//! growth of the fastest wave past the Courant limit (`fdtd::grid_frequency`), and a CPML's
//! reflection against its thickness for σ graded as (x/L)^m, from 1D runs of photonoxide's
//! FDTD (`fdtd::Simulation` with `Edges::Pml`).

use std::f64::consts::{PI, TAU};

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{
    Boundaries, Cpml, Field, Simulation, Source, Waveform, grid_frequency, grid_wavenumber,
    time_step,
};
use photonoxide::units::Frequency;

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The palette's grey, for reference lines.
const GREY: usize = 7;

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// The lesson's charts.
pub fn specs() -> Vec<ChartSpec> {
    let dims = |default: f64| {
        param(
            "dims",
            "Dimensions $d$",
            "",
            (1.0, 3.0, default),
            1.0,
            false,
            "The axes the field varies along: 1 for a line, 2 for a plane, 3 for a volume, on square or cubic cells.",
        )
    };
    vec![
        ChartSpec {
            id: "fdtd-dispersion",
            title: "How slow a wave travels on Yee's grid",
            about: "The phase velocity's shortfall, 1 − v/c, of a plane wave on Yee's grid against the cells per wavelength, along an axis and along the diagonals, from Taflove and Brodwin's relation at the chosen Courant number.",
            computed_by: "fdtd::grid_wavenumber, fdtd::time_step",
            params: vec![
                param(
                    "courant",
                    "Courant number $C$",
                    "",
                    (0.05, 1.0, 0.5),
                    0.01,
                    false,
                    "The time step as a share of the stability limit: Δt = C/√(Σ 1/Δᵢ²) with c = 1.",
                ),
                dims(2.0),
            ],
        },
        ChartSpec {
            id: "fdtd-stability",
            title: "Past the Courant limit",
            about: "How much the grid's fastest wave, two cells a wavelength along every axis, grows each step against the time step in units of the cell's crossing time Δ/c, on a line, a plane and in a volume of cubic cells: nothing up to the limit Δ/(c√d), exponentially beyond it.",
            computed_by: "fdtd::grid_frequency, fdtd::time_step",
            params: vec![param(
                "step",
                "Time step $c\\Delta t/\\Delta$",
                "",
                (0.3, 1.2, 0.6),
                0.01,
                false,
                "The time step in units of the time light takes to cross a cell.",
            )],
        },
        ChartSpec {
            id: "fdtd-pml",
            title: "A PML's reflection against its thickness",
            about: "The power a convolutional PML L cells thick reflects of a wave meeting it head on, in 1D runs of photonoxide's FDTD, for its conductivity graded as (x/L)^m with m = 1, 2 and 3 at one target round-trip reflection, against L.",
            computed_by: "fdtd::Simulation on a line with Edges::Pml and Cpml { reflection, order }, a probe's transform against a run with a thick PML",
            params: vec![
                param(
                    "target",
                    "Round trip $-\\log_{10} R$",
                    "",
                    (4.0, 20.0, 15.0),
                    1.0,
                    false,
                    "The reflection the conductivity's profile is set for, as if the PML were continuous: σ's maximum is (m + 1)(−ln R)/2d for a PML d thick.",
                ),
                param(
                    "cells",
                    "Cells a wavelength",
                    "",
                    (10.0, 40.0, 20.0),
                    1.0,
                    false,
                    "The grid: how many cells a vacuum wavelength spans at the frequency read.",
                ),
            ],
        },
    ]
}

/// The axes along each direction: along the first axis, the face diagonal, the body diagonal.
fn directions(dims: usize) -> Vec<(&'static str, [f64; 3])> {
    let mut out = vec![("along an axis", [1.0, 0.0, 0.0])];
    if dims >= 2 {
        out.push(("along a face diagonal", [1.0, 1.0, 0.0]));
    }
    if dims >= 3 {
        out.push(("along the body diagonal", [1.0, 1.0, 1.0]));
    }
    out
}

/// 1 − v/c of a wave at 1 c/µm along `direction` on cubic cells of 1/`cells` µm, at Courant
/// number `courant` in `dims` dimensions.
pub fn shortfall(cells: f64, courant: f64, dims: usize, direction: [f64; 3]) -> Option<f64> {
    let d = 1.0 / cells;
    let dt = time_step(&vec![d; dims], courant);
    let f = Frequency::natural(1.0).ok()?;
    let k = grid_wavenumber(f, direction, [d; 3], dt, 1.0)?;
    Some(1.0 - f.angular() / k)
}

/// The smallest shortfall drawn: below it a curve is exact to round-off.
const FLOOR: f64 = 1e-16;

/// The phase velocity's shortfall against the cells per wavelength.
pub fn dispersion(courant: f64, dims: f64) -> photonoxide::Result<ChartData> {
    let dims = dims as usize;
    let ns: Vec<f64> = (0..=112).map(|j| 4.0 + 0.5 * j as f64).collect();
    let mut series = Vec::new();
    for (k, (label, dir)) in directions(dims).into_iter().enumerate() {
        let points: Vec<[f64; 2]> = ns
            .iter()
            .filter_map(|&n| shortfall(n, courant, dims, dir).map(|e| [n, e.max(FLOOR)]))
            .collect();
        series.push(curve(label, points, false, k));
    }
    let axis = |n: f64| shortfall(n, courant, dims, [1.0, 0.0, 0.0]).unwrap_or(f64::NAN);
    let order = (axis(20.0) / axis(40.0)).log2();
    let mut figures = vec![
        figure(
            "along an axis, 10 cells",
            axis(10.0),
            3,
            "",
            "$1 - v/c$ at 10 cells a wavelength",
        ),
        figure(
            "along an axis, 20 cells",
            axis(20.0),
            3,
            "",
            "at 20 cells a wavelength",
        ),
        figure(
            "order",
            order,
            3,
            "",
            "$\\log_2$ of the shortfall at 20 cells over 40: 2 for a second-order scheme",
        ),
    ];
    if dims >= 2 {
        let diag = shortfall(10.0, courant, dims, [1.0, 1.0, 0.0]).unwrap_or(f64::NAN);
        figures.push(figure(
            "diagonal, 10 cells",
            diag,
            3,
            "",
            "the diagonal is faster: the grid is anisotropic",
        ));
    }
    let exact = dims == 1 && courant == 1.0;
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "cells a wavelength".to_owned(),
        x_length: false,
        y_label: "phase velocity's shortfall 1 − v/c".to_owned(),
        y_log: true,
        y_range: Some([FLOOR, 1.0]),
        series,
        marker: Some(20.0),
        figures,
        note: format!(
            "Exact, no grid of its own: Taflove and Brodwin's relation solved for k at {} cell counts, cubic cells, C = {courant}{}. The line marks 20 cells a wavelength.",
            ns.len(),
            if exact {
                "; in 1D at C = 1 the grid is exact, drawn at 1e-16"
            } else {
                ""
            }
        ),
    })
}

/// The growth per step of the grid's fastest wave (kΔ = π along every axis) at a time step of
/// `ratio` times the limit, in `dims` dimensions on unit cells.
pub fn growth(ratio: f64, dims: usize) -> f64 {
    let dt = time_step(&vec![1.0; dims], 1.0) * ratio;
    let mut k = [0.0; 3];
    for v in k.iter_mut().take(dims) {
        *v = PI;
    }
    (grid_frequency(k, [1.0; 3], dt, 1.0).im * dt).exp()
}

/// The fastest wave's growth per step against cΔt/Δ, in 1, 2 and 3 dimensions.
pub fn stability(step: f64) -> photonoxide::Result<ChartData> {
    let labels = ["on a line", "on a plane", "in a volume"];
    let mut series = Vec::new();
    let mut figures = Vec::new();
    for (k, dims) in [1_usize, 2, 3].into_iter().enumerate() {
        let limit = time_step(&vec![1.0; dims], 1.0);
        let points: Vec<[f64; 2]> = (0..=180)
            .map(|j| {
                let s = 0.3 + 0.005 * j as f64;
                [s, growth(s / limit, dims)]
            })
            .collect();
        series.push(curve(labels[k], points, false, k));
        let g = growth(step / limit, dims);
        figures.push(figure(
            &format!("growth a step, {}D", dims),
            g,
            4,
            "",
            &if g > 1.0 {
                format!(
                    "a millionfold in {} steps; the limit is $c\\Delta t/\\Delta$ = {}",
                    sig(1e6_f64.ln() / g.ln(), 3),
                    sig(limit, 4)
                )
            } else {
                format!(
                    "stable: the limit is $c\\Delta t/\\Delta$ = {}",
                    sig(limit, 4)
                )
            },
        ));
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "time step cΔt/Δ".to_owned(),
        x_length: false,
        y_label: "growth a step of the fastest wave".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: Some(step),
        figures,
        note: "Exact: Taflove and Brodwin's relation for kΔ = π along each axis on cubic cells, the root that grows past the limit Δ/(c√d). The line marks the chosen step; photonoxide refuses a Courant number above 1.".to_owned(),
    })
}

/// The cells of vacuum between the source and the PML under test, and from the source to the
/// probe, which sits between them.
const GAP: usize = 60;
const PROBE: usize = 30;
/// The reference PML's thickness, cells: its own reflection far below the ones measured.
const THICK: usize = 160;

/// The power a CPML `cells_thick` cells thick reflects at normal incidence, at 1 c/µm on a grid
/// of `per_wavelength` cells a micrometre, for σ graded as (x/L)^`order` at the round-trip
/// target `target`: |r|², r from the probe's transform with this PML less the one with a
/// thick PML, over the incident transform. 1D: a line of cells along x, periodic across.
pub fn reflection(
    cells_thick: usize,
    order: f64,
    target: f64,
    per_wavelength: f64,
) -> photonoxide::Result<f64> {
    let d = 1.0 / per_wavelength;
    let f = Frequency::natural(1.0)?;
    let pulse = Waveform::pulse(f, 0.5)?;
    let low = THICK;
    let run = |high: usize| -> photonoxide::Result<(Vec<f64>, f64)> {
        let n = low + GAP + high;
        let grid = Grid3d {
            nx: n,
            ny: 1,
            nz: 1,
            dx: d,
            dy: d,
            dz: d,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        };
        let boundaries = Boundaries {
            x: Edges::Pml { low, high },
            y: Edges::Bloch { k: 0.0 },
            z: Edges::Bloch { k: 0.0 },
            cpml: Cpml {
                reflection: target,
                sigma: None,
                order,
                kappa: 1.0,
                alpha: 0.0,
            },
        };
        // Meep's step, Δt = Δx/2
        let mut s = Simulation::new(grid, |_, _, _| 1.0, boundaries, 0.5)?;
        let source = low + 10;
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (source, 0, 0),
            waveform: pulse,
        })?;
        let p = s.add_probe(Field::E, Axis::Z, (source + PROBE, 0, 0))?;
        // the pulse there and back across the gap and the PML, and its tail
        let span = (2 * (GAP + high + THICK)) as f64 * d;
        s.run_until(span + 40.0);
        Ok((s.probe(p).to_vec(), s.dt()))
    };
    let (with, dt) = run(cells_thick)?;
    let (reference, _) = run(THICK)?;
    let w = f.angular();
    let dft = |series: &[f64]| -> c64 {
        series
            .iter()
            .enumerate()
            .map(|(n, v)| v * c64::new(0.0, w * (n + 1) as f64 * dt).exp())
            .sum()
    };
    let len = with.len().min(reference.len());
    let difference: Vec<f64> = with[..len]
        .iter()
        .zip(&reference[..len])
        .map(|(a, b)| a - b)
        .collect();
    let incident = dft(&reference[..len]);
    if incident.norm() == 0.0 {
        return Err(invalid("fdtd-pml", "no incident wave at the probe"));
    }
    Ok((dft(&difference) / incident).norm_sqr())
}

/// The thicknesses drawn, cells.
const THICKNESSES: [usize; 12] = [2, 4, 6, 8, 10, 12, 16, 20, 24, 28, 32, 40];

/// The slope of log |r|² against log L from `a` to `b` cells: −(the rate).
fn rate(points: &[[f64; 2]], a: f64, b: f64) -> f64 {
    let at = |l: f64| points.iter().find(|p| p[0] == l).map_or(f64::NAN, |p| p[1]);
    -(at(b) / at(a)).ln() / (b / a).ln()
}

/// A CPML's reflection against its thickness for m = 1, 2, 3.
pub fn pml(target_exponent: f64, per_wavelength: f64) -> photonoxide::Result<ChartData> {
    let target = 10f64.powf(-target_exponent);
    let orders = [1.0, 2.0, 3.0];
    let curves: Vec<Vec<[f64; 2]>> = orders
        .iter()
        .map(|&m| {
            THICKNESSES
                .iter()
                .map(|&l| Ok([l as f64, reflection(l, m, target, per_wavelength)?]))
                .collect::<photonoxide::Result<Vec<_>>>()
        })
        .collect::<photonoxide::Result<Vec<_>>>()?;
    let mut series = Vec::new();
    let mut figures = Vec::new();
    for (k, (m, points)) in orders.iter().zip(&curves).enumerate() {
        figures.push(figure(
            &format!("rate, $m = {m}$"),
            rate(points, 16.0, 32.0),
            3,
            "",
            &format!(
                "$|r|^2 \\propto 1/L^{{\\text{{rate}}}}$ from 16 to 32 cells; Oskooi et al. give {}",
                2.0 * m + 2.0
            ),
        ));
        series.push(curve(&format!("m = {m}"), points.clone(), false, k));
    }
    figures.push(figure(
        "$|r|^2$ at 10 cells, $m = 3$",
        curves[2][4][1],
        3,
        "",
        "half a wavelength of PML at 20 cells a wavelength",
    ));
    let line = |v: f64| vec![[2.0, v], [40.0, v]];
    series.push(curve("target R", line(target), true, GREY));
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "PML thickness L, cells".to_owned(),
        x_length: false,
        y_label: "reflected power |r|²".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: None,
        figures,
        note: format!(
            "1D runs of photonoxide's FDTD on {per_wavelength} cells a wavelength at C = 0.5 (Meep's Δt = Δx/2), a pulse at 1 c/µm meeting the PML head on; κ = 1, α = 0, σ's maximum (m + 1)(−ln R)/2d with R = {}. The reflection is read at 1 c/µm against a run whose PML is {THICK} cells thick.",
            sig(target, 2)
        ),
    })
}

/// Every chart of the lesson, by id, at its parameters.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "fdtd-dispersion" => dispersion(at("courant"), at("dims")),
        "fdtd-stability" => stability(at("step")),
        "fdtd-pml" => pml(at("target"), at("cells")),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    /// The shortfall falls as 1/N², is largest along an axis, vanishes in 1D at C = 1, and is
    /// (πΔ/λ)²(1 − C²)/6 in 1D to leading order: (kΔ)²(1 − C²)/24.
    #[test]
    fn the_grid_is_slow_at_second_order() {
        let data = dispersion(0.5, 2.0).unwrap();
        assert!((fig(&data, "order") - 2.0).abs() < 0.02);
        assert!(fig(&data, "along an axis, 10 cells") > fig(&data, "diagonal, 10 cells"));
        for n in [10.0, 20.0, 40.0] {
            let e = shortfall(n, 0.5, 1, [1.0, 0.0, 0.0]).unwrap();
            let leading = (TAU / n).powi(2) * (1.0 - 0.25) / 24.0;
            assert!(
                (e / leading - 1.0).abs() < 0.05,
                "{n}: {e} against {leading}"
            );
        }
        assert!(shortfall(10.0, 1.0, 1, [1.0, 0.0, 0.0]).unwrap().abs() < 1e-15);
        // 3D at the limit: along the body diagonal the scheme is exact as well
        assert!(shortfall(10.0, 1.0, 3, [1.0, 1.0, 1.0]).unwrap().abs() < 1e-14);
    }

    /// Up to the limit nothing grows; past it the fastest wave grows by (s + √(s² − 1))².
    #[test]
    fn past_the_limit_the_fastest_wave_grows() {
        for dims in [1, 2, 3] {
            assert!((growth(0.99, dims) - 1.0).abs() < 1e-12);
            let s: f64 = 1.05;
            let g = (s + (s * s - 1.0).sqrt()).powi(2);
            assert!((growth(s, dims) / g - 1.0).abs() < 1e-12);
        }
    }

    /// A thicker PML reflects less, and from 16 to 32 cells its reflected power falls as
    /// 1/L^(2m + 2), Oskooi and Johnson's rate for σ turned on as (x/L)^m: measured 4.13, 6.01
    /// and 8.00, held to 0.2, the m = 1 curve still nearing its asymptote.
    #[test]
    fn a_thicker_pml_reflects_less() {
        let data = pml(15.0, 20.0).unwrap();
        for m in [1.0, 2.0, 3.0] {
            let r = fig(&data, &format!("rate, $m = {m}$"));
            assert!((r - (2.0 * m + 2.0)).abs() < 0.2, "m = {m}: rate {r}");
        }
        let thin = reflection(8, 3.0, 1e-15, 20.0).unwrap();
        let thick = reflection(32, 3.0, 1e-15, 20.0).unwrap();
        assert!(thick < 1e-3 * thin, "{thin} {thick}");
    }
}
