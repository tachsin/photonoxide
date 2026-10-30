//! What the validation report and the tests check graded media on: a parabolic-index medium,
//! whose guided waves are a harmonic oscillator's, read by the planar mode solver and by FDTD
//! through its smoothing structure, against the closed form, and the order they converge at.
//!
//! In ε(x) = n₀²(1 − 2Δ x²/a²), a wave ψ(x) e^(iβy) with ψ″ + (k²ε − β²)ψ = 0 is the harmonic
//! oscillator −ψ″ + Ω²x²ψ = (k²n₀² − β²)ψ, Ω = k n₀ √(2Δ)/a, so k²n₀² − β²ₘ = (2m + 1) Ω: at a
//! wavelength, β²ₘ = k²n₀² − (2m + 1) k n₀ √(2Δ)/a (the planar mode solver's TE modes); at a
//! wavenumber β along y, n₀² ω² − (2m + 1) g ω − β² = 0, g = n₀ √(2Δ)/a (FDTD's E_z between
//! Bloch sides). The medium is cut off by walls where the modes' fields are below 1e-5.

use super::Graded;
use crate::Result;
use crate::expr::{Expr, Parameters};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

/// The medium: n₀ = 1.5, a = `a` µm, Δ = `delta`, as an index n = n₀ √(1 − 2Δ (x/a)²) with its
/// units.
fn parabola(a: f64, delta: f64) -> Result<Graded> {
    let defs = [
        ("n0".to_owned(), "1.5".to_owned()),
        ("a".to_owned(), format!("{a} um")),
        ("delta".to_owned(), format!("{delta}")),
    ];
    let parameters = Parameters::new(&defs)
        .map_err(|e| crate::Error::invalid("graded medium", e.to_string()))?;
    let e = Expr::parse("n0*sqrt(1 - 2*delta*(x/a)^2)", parameters.scope())
        .map_err(|e| crate::Error::invalid("graded medium", e.to_string()))?;
    Graded::index(&e, &parameters)
}

/// The mode solver's medium: a = 10 µm, Δ = 0.01, at 1.55 µm, within ±25 µm.
const MODE: (f64, f64, f64, f64) = (10.0, 0.01, 1.55, 25.0);

/// The closed form's effective indices of the first `count` TE modes at the mode solver's
/// wavelength.
pub(crate) fn mode_exact(count: usize) -> Vec<f64> {
    let (a, delta, lambda, _) = MODE;
    let k = std::f64::consts::TAU / lambda;
    let n0 = 1.5;
    (0..count)
        .map(|m| {
            let b2 = k * k * n0 * n0 - (2 * m + 1) as f64 * k * n0 * (2.0 * delta).sqrt() / a;
            b2.sqrt() / k
        })
        .collect()
}

/// The planar mode solver's first four TE modes in the parabolic medium on `cells` cells
/// across ±25 µm, the medium's index read at each cell's centre: their effective indices,
/// highest first.
pub(crate) fn mode_solved(cells: usize) -> Result<Vec<f64>> {
    let (a, delta, lambda, half) = MODE;
    let g = parabola(a, delta)?;
    g.check_over(
        crate::geometry::Bounds3d::new([-half, 0.0, 0.0], [half, 0.0, 0.0]),
        cells,
    )?;
    let profile = crate::mode::slab_fd::Profile::uniform(-half, half, cells, |x| {
        num_complex::Complex64::new(g.permittivity_at([x, 0.0, 0.0]), 0.0)
    })?;
    let mut n: Vec<f64> = profile
        .modes(
            crate::mode::Polarization::Te,
            Wavelength::um(lambda)?,
            4,
            Some(1.5),
        )?
        .iter()
        .map(|m| m.effective_index.re)
        .collect();
    n.sort_by(|x, y| y.total_cmp(x));
    Ok(n)
}

/// The largest difference of the mode solver's four effective indices on `cells` cells from the
/// closed form's.
pub(crate) fn mode_error(cells: usize) -> f64 {
    let Ok(n) = mode_solved(cells) else {
        return f64::INFINITY;
    };
    n.iter()
        .zip(mode_exact(4))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

/// FDTD's medium: a = 2 µm, Δ = 0.05, β = 2π rad/µm along y, within ±5 µm.
const WAVE: (f64, f64, f64, f64) = (2.0, 0.05, std::f64::consts::TAU, 5.0);

/// The closed form's frequencies (c/µm) of the first `count` modes at FDTD's β.
pub(crate) fn fdtd_exact(count: usize) -> Vec<f64> {
    let (a, delta, beta, _) = WAVE;
    let n0: f64 = 1.5;
    let g = n0 * (2.0 * delta).sqrt() / a;
    (0..count)
        .map(|m| {
            let s = (2 * m + 1) as f64 * g;
            let omega = (s + (s * s + 4.0 * n0 * n0 * beta * beta).sqrt()) / (2.0 * n0 * n0);
            omega / std::f64::consts::TAU
        })
        .collect()
}

/// FDTD's first three modes of the parabolic medium on cells of `h` µm across x (walls at ±5
/// µm), the medium a graded body of its smoothing structure, between Bloch sides along y at
/// β = 2π rad/µm (a cell of 1 nm, so that the phase across it is exact to 1e-6) and periodic
/// along z: E_z from a pulse through the modes (an off-centre current), at a probe off the
/// centre, its frequencies by harmonic inversion over the run's second half. The frequencies,
/// c/µm, lowest first.
pub(crate) fn fdtd_solved(h: f64) -> Result<Vec<f64>> {
    use crate::fdfd::{Axis, Edges, Grid3d};
    use crate::fdtd::{
        Body, Boundaries, Cpml, Current, Field, Permittivity, Simulation, Smoothing, Structure,
        Waveform, harmonic_inversion,
    };
    use crate::units::Frequency;
    use num_complex::Complex64 as c64;
    let (a, delta, beta, half) = WAVE;
    let nx = (2.0 * half / h).round() as usize;
    let thin = 1e-3;
    let grid = Grid3d {
        nx,
        ny: 1,
        nz: 1,
        dx: 2.0 * half / nx as f64,
        dy: thin,
        dz: thin,
        x0: -half,
        y0: 0.0,
        z0: 0.0,
    };
    let boundaries = Boundaries {
        x: Edges::Pml { low: 0, high: 0 },
        y: Edges::Bloch { k: beta },
        z: Edges::Bloch { k: 0.0 },
        cpml: Cpml::default(),
    };
    let medium = parabola(a, delta)?;
    let everywhere = Body::cuboid([-2.0 * half; 3], [2.0 * half; 3])?;
    let structure = Structure::new(Permittivity::isotropic(1.0)?).with_graded(everywhere, medium);
    let mut s = Simulation::smoothed(grid, &structure, Smoothing::default(), boundaries, 0.9)?;
    let exact = fdtd_exact(3);
    let centre = 0.5 * (exact[0] + exact[2]);
    let pulse = Waveform::pulse(Frequency::natural(centre)?, 4.0 * (exact[2] - exact[0]))?;
    let at = |x: f64| ((x + half) / grid.dx).round() as usize;
    s.add_current(Current {
        field: Field::E,
        values: vec![(Axis::Z, (at(0.37), 0, 0), c64::new(1.0, 0.0))],
        waveform: pulse,
    })?;
    let probe = s.add_probe(Field::E, Axis::Z, (at(-0.81), 0, 0))?;
    s.run_until(600.0);
    let series = s.probe_complex(probe);
    let tail = &series[series.len() / 2..];
    let (low, high) = (exact[0] - 0.02, exact[2] + 0.02);
    let mut found: Vec<f64> = harmonic_inversion(
        tail,
        s.dt(),
        Frequency::natural(low)?,
        Frequency::natural(high)?,
    )?
    .into_iter()
    .filter(|r| {
        r.error < 1e-6
            && r.amplitude.norm() > 1e-6 * tail.iter().fold(0.0f64, |m, v| m.max(v.norm()))
    })
    .map(|r| r.frequency)
    .collect();
    found.sort_by(f64::total_cmp);
    Ok(found)
}

/// The largest relative difference of FDTD's three frequencies on cells of `h` µm from the
/// closed form's (infinite if one is missing).
pub(crate) fn fdtd_error(h: f64) -> f64 {
    let Ok(found) = fdtd_solved(h) else {
        return f64::INFINITY;
    };
    fdtd_exact(3)
        .iter()
        .map(|want| {
            found
                .iter()
                .map(|f| ((f - want) / want).abs())
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

fn order(coarse: f64, fine: f64) -> Outcome {
    let o = ((coarse / fine).log2() * 100.0).round() / 100.0;
    Outcome {
        measured: o,
        expected: 2.0,
        tolerance: 0.25,
        error: (o - 2.0).abs(),
    }
}

/// The validation cases of graded media.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mode/graded-parabolic",
            title: r"A graded medium in the planar mode solver: the parabolic index $n = n_0\sqrt{1 - 2\Delta (x/a)^2}$ ($n_0$ = 1.5, $a$ = 10 µm, $\Delta$ = 0.01, an expression with its units) across ±25 µm on 4000 cells (12.5 nm), its four lowest TE modes at 1.55 µm, the index read at each cell's centre (largest difference in $n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: r"the harmonic oscillator: $\beta_m^2 = k^2 n_0^2 - (2m + 1)\thinspace k n_0 \sqrt{2\Delta}/a$; measured 3.1e-8",
            run: || error(mode_error(4000), 1e-7),
        },
        Case {
            id: "mode/graded-parabolic-order",
            title: r"The same modes on 1000 and 2000 cells (50 and 25 nm): $\log_2$ of the ratio of their largest errors (shown)",
            tier: Tier::Analytic,
            source: r"the planar scheme with the medium at the cells' centres: second order for a smooth medium",
            run: || order(mode_error(1000), mode_error(2000)),
        },
        Case {
            id: "fdtd/graded-parabolic",
            title: r"A graded medium in FDTD: the parabolic index $n_0\sqrt{1 - 2\Delta (x/a)^2}$ ($n_0$ = 1.5, $a$ = 2 µm, $\Delta$ = 0.05) as a graded body of the smoothing structure, across ±5 µm between walls on cells of 12.5 nm, $E_z$ between Bloch sides at $\beta = 2\pi$ rad/µm along y: its three lowest modes' frequencies by harmonic inversion of a probe after a pulse, over 600 µm/c (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$n_0^2\omega^2 - (2m + 1)\thinspace g\thinspace\omega - \beta^2 = 0$, $g = n_0\sqrt{2\Delta}/a$ (the harmonic oscillator); measured 1.0e-4",
            run: || error(fdtd_error(0.0125), 2e-4),
        },
        Case {
            id: "fdtd/graded-parabolic-order",
            title: r"The same modes on cells of 50 and 25 nm: $\log_2$ of the ratio of their largest relative errors (shown)",
            tier: Tier::Analytic,
            source: r"Yee's scheme with the graded medium at each value's nodes: second order",
            run: || order(fdtd_error(0.05), fdtd_error(0.025)),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_solver_reads_a_parabolic_medium() {
        let e = mode_error(2000);
        assert!(e < 1e-6, "{e}");
    }
}
