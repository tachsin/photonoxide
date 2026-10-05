//! The direct solvers' systems written out for other solvers to factorize: the baseline of the
//! performance plan (docs/plans/performance.md, Phase A0), photonoxide's sparse LU against
//! PARDISO (O. Schenk, K. Gärtner, Future Gener. Comput. Syst. 20, 475 (2004),
//! doi:10.1016/j.future.2003.07.011) and MUMPS (P. R. Amestoy et al., SIAM J. Matrix Anal. Appl.
//! 23, 15 (2001), doi:10.1137/S0895479899358194), which run as programs of their own, never
//! linked. `photonoxide bench --export <dir>` writes them; docs/benchmarks.md says how the
//! others were run.
//!
//! Each system is three files in the Matrix Market format (R. F. Boisvert, R. Pozo, K. Remington,
//! "The Matrix Market exchange formats: initial design", NIST IR 5935 (1996)): `A.mtx`, the
//! matrix as complex coordinates, 1-based; `b.mtx`, the right-hand side; `x.mtx`, photonoxide's
//! solution, to check against. Every value is written as its shortest decimal that reads back to
//! the same bits.

use std::fmt::Write as _;
use std::path::Path;

use num_complex::Complex64 as c64;

use crate::fdfd::{Boundaries, Boundaries3d, Grid, Grid3d, Polarization, Solver3d};
use crate::units::Wavelength;
use crate::{Error, Result};

/// A system to export: its id, its grid, its matrix and right-hand side, and its solution.
#[derive(Clone, Debug)]
pub struct System {
    /// A short, stable identifier, e.g. `"strip-32"`.
    pub id: &'static str,
    /// The grid, e.g. `"32 × 32 × 32 cells of 40 nm, PMLs of 6"`.
    pub grid: String,
    /// The unknowns.
    pub n: usize,
    /// The matrix's entries, (row, column, value), 0-based, repeated entries summed.
    pub entries: Vec<(usize, usize, c64)>,
    /// The right-hand side.
    pub rhs: Vec<c64>,
}

/// The systems [`write`](fn@write) exports, by id: `slab-2d` (the 2D solver's 440 × 340 slab, E along z),
/// and `strip-24`, `strip-32`, `strip-40` (the 3D direct solver's strip in oxide, n³ cells of
/// 40 nm). Building one assembles it, nothing more.
pub const IDS: [&str; 4] = ["slab-2d", "strip-24", "strip-32", "strip-40"];

/// The system `id` (one of [`IDS`]).
///
/// # Errors
///
/// [`Error::InvalidValue`] for another id, and the assembly's errors.
pub fn system(id: &str) -> Result<System> {
    // the 3D lattice's values fixed by its walls, whose right-hand side is zero
    let mut fixed: Vec<bool> = Vec::new();
    let (id, grid, n, triplets): (&'static str, String, usize, Vec<_>) = match id {
        "slab-2d" => {
            // the benchmark's slab (fdfd2d/slab-lu): 440 x 340 cells of 10 nm, PMLs of 20
            let (h, pml) = (0.01, 20);
            let g = Grid {
                nx: 400 + 2 * pml,
                ny: 300 + 2 * pml,
                dx: h,
                dy: h,
                x0: -(pml as f64) * h,
                y0: -1.5 - pml as f64 * h,
            };
            let eps_z: Vec<c64> = (0..g.nx * g.ny)
                .map(|k| {
                    let y = g.y0 + ((k / g.nx) as f64 + 0.5) * h;
                    let m: f64 = if y.abs() < 0.11 { 3.476 } else { 1.444 };
                    c64::new(m * m, 0.0)
                })
                .collect();
            let faces = vec![c64::new(1.0, 0.0); (g.nx + 1) * (g.ny + 1)];
            let k0 = std::f64::consts::TAU / Wavelength::um(1.55)?.to_um();
            let t = crate::fdfd::assemble(
                g,
                Polarization::Ez,
                k0,
                &eps_z,
                &faces,
                &faces,
                &Boundaries::pml(pml),
            );
            (
                "slab-2d",
                format!(
                    "{} × {} cells of 10 nm, PMLs of {pml}, E along z",
                    g.nx, g.ny
                ),
                g.nx * g.ny,
                t,
            )
        }
        "strip-24" | "strip-32" | "strip-40" => {
            let (name, cells) = match id {
                "strip-24" => ("strip-24", 24),
                "strip-32" => ("strip-32", 32),
                _ => ("strip-40", 40),
            };
            let (grid, lam) = strip_grid(cells)?;
            let (lattice, eps) = Solver3d::setup(grid, lam, strip, Boundaries3d::pml(6))?;
            fixed = (0..grid.unknowns()).map(|r| lattice.fixed(r)).collect();
            (
                name,
                format!("{cells} × {cells} × {cells} cells of 40 nm, PMLs of 6"),
                grid.unknowns(),
                lattice.assemble(&eps),
            )
        }
        other => {
            return Err(Error::invalid(
                "bench export",
                format!("no system \"{other}\": {}", IDS.join(", ")),
            ));
        }
    };
    // repeated entries summed, sorted by column then row, as Matrix Market readers expect
    let mut entries: Vec<(usize, usize, c64)> =
        triplets.iter().map(|t| (t.row, t.col, t.val)).collect();
    entries.sort_by_key(|&(r, c, _)| (c, r));
    entries.dedup_by(|next, kept| {
        let same = (next.0, next.1) == (kept.0, kept.1);
        if same {
            kept.2 += next.2;
        }
        same
    });
    // a fixed right-hand side, the same on every machine (zero on a 3D wall's fixed values, as
    // the solver sets it)
    let rhs = (0..n)
        .map(|i| {
            if fixed.get(i).copied().unwrap_or(false) {
                c64::new(0.0, 0.0)
            } else {
                c64::new(((i * 7919) % 13) as f64, 1.0)
            }
        })
        .collect();
    Ok(System {
        id,
        grid,
        n,
        entries,
        rhs,
    })
}

/// The 3D strip's grid (n³ cells of 40 nm) and wavelength.
fn strip_grid(cells: usize) -> Result<(Grid3d, Wavelength)> {
    let h = 0.04;
    let half = cells as f64 * h / 2.0;
    let grid = Grid3d {
        nx: cells,
        ny: cells,
        nz: cells,
        dx: h,
        dy: h,
        dz: h,
        x0: -half,
        y0: -half,
        z0: -half,
    };
    Ok((grid, Wavelength::um(1.55)?))
}

/// The 3D direct solver's strip (docs/methods/fdfd-3d.md, "Cost"): 500 × 220 nm of silicon in
/// oxide.
fn strip(_: f64, y: f64, z: f64) -> c64 {
    let m: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
        3.476
    } else {
        1.444
    };
    c64::new(m * m, 0.0)
}

/// `system` solved by photonoxide to a relative residual of round-off: in 2D by faer's sparse LU
/// and steps of refinement; in 3D by the direct solver ([`Solver3d::solve_system`]: its factors
/// in nested-dissection order, refined, and QMR where they're inaccurate).
///
/// # Errors
///
/// The solve's.
pub fn solve(system: &System) -> Result<Vec<c64>> {
    if let Some(cells) = system.id.strip_prefix("strip-") {
        let cells: usize = cells.parse().unwrap_or(24);
        let (grid, lam) = strip_grid(cells)?;
        let solver = Solver3d::new(grid, lam, strip, Boundaries3d::pml(6))?;
        return Ok(solver.solve_system(&system.rhs)?.values().to_vec());
    }
    let n = system.n;
    let matrix = faer::sparse::SparseColMat::<usize, c64>::try_new_from_triplets(
        n,
        n,
        &system
            .entries
            .iter()
            .map(|&(r, c, v)| faer::sparse::Triplet::new(r, c, v))
            .collect::<Vec<_>>(),
    )
    .map_err(|e| Error::invalid("bench export", format!("{e:?}")))?;
    use faer::linalg::solvers::Solve;
    let lu = matrix
        .sp_lu()
        .map_err(|e| Error::invalid("bench export", format!("{e:?}")))?;
    let column = |v: &[c64]| faer::Mat::<c64>::from_fn(n, 1, |r, _| v[r]);
    let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    let solved = lu.solve(column(&system.rhs));
    let mut x: Vec<c64> = (0..n).map(|r| solved[(r, 0)]).collect();
    // steps of refinement to round-off, as the 2D solver takes one
    for _ in 0..5 {
        let mut r = system.rhs.clone();
        for &(i, j, v) in &system.entries {
            r[i] -= v * x[j];
        }
        if norm(&r) <= 1e-13 * norm(&system.rhs) {
            break;
        }
        let d = lu.solve(column(&r));
        for (xk, k) in x.iter_mut().zip(0..n) {
            *xk += d[(k, 0)];
        }
    }
    Ok(x)
}

/// How photonoxide's own sparse LU did on a system, factorized as its solvers factorize it.
#[derive(Clone, Debug, PartialEq)]
pub struct Factorized {
    /// The column ordering: `"nested dissection"`, on the grid, as the 2D and 3D solvers order.
    pub ordering: &'static str,
    /// The matching, the ordering and the symbolic analysis, seconds.
    pub analysis_seconds: f64,
    /// The numeric factorization, seconds.
    pub factorization_seconds: f64,
    /// One solve with the factors, seconds.
    pub solve_seconds: f64,
    /// ‖b − A x‖ / ‖b‖ for that solve, unrefined.
    pub residual: f64,
    /// The factors' entries, each front's diagonal block and its panels: L and U, or for a
    /// complex symmetric system L alone (with D). The fill to set against PARDISO's and MUMPS's
    /// counts of their factors' entries.
    pub factor_entries: usize,
}

/// Factorizes `system` as photonoxide's solvers do (the multifrontal factorization with static
/// pivoting, ordered by nested dissection on the grid: L D Lᵀ if the system is complex
/// symmetric, as [`symmetric`]'s form is, LU otherwise), solves once, and times each step: what
/// PARDISO and MUMPS are compared with on the same matrix.
///
/// # Errors
///
/// The analysis' and the factorization's.
pub fn factorize(system: &System) -> Result<Factorized> {
    use std::sync::Arc;
    use std::time::Instant;
    let n = system.n;
    let matrix = faer::sparse::SparseColMat::<usize, c64>::try_new_from_triplets(
        n,
        n,
        &system
            .entries
            .iter()
            .map(|&(r, c, v)| faer::sparse::Triplet::new(r, c, v))
            .collect::<Vec<_>>(),
    )
    .map_err(|e| Error::invalid("bench export", format!("{e:?}")))?;
    let residual = |x: &[c64]| {
        let mut r = system.rhs.clone();
        for &(i, j, v) in &system.entries {
            r[i] -= v * x[j];
        }
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        norm(&r) / norm(&system.rhs)
    };
    let t = Instant::now();
    let positions = positions(system);
    let analysis = Arc::new(
        match crate::sparse::Analysis::new_symmetric(matrix.as_ref(), positions.as_deref())? {
            Some(symmetric) => symmetric,
            None => crate::sparse::Analysis::new(matrix.as_ref(), positions.as_deref())?,
        },
    );
    let analysis_seconds = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let lu = crate::sparse::Multifrontal::new(analysis.clone(), matrix.as_ref())?;
    let factorization_seconds = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let x = lu.solve(&system.rhs);
    let solve_seconds = t.elapsed().as_secs_f64();
    Ok(Factorized {
        ordering: "nested dissection",
        analysis_seconds,
        factorization_seconds,
        solve_seconds,
        residual: residual(&x),
        factor_entries: analysis.factor_entries(),
    })
}

/// Each unknown's place on its grid, for nested dissection: Yee's edges for a strip, the cells'
/// centres (by index) for the slab.
pub(crate) fn positions(system: &System) -> Option<Vec<[f64; 3]>> {
    if let Some(cells) = system.id.strip_prefix("strip-") {
        let cells: usize = cells.parse().ok()?;
        let (grid, _) = strip_grid(cells).ok()?;
        let mut positions = vec![[0.0; 3]; system.n];
        for component in [
            crate::fdfd::Axis::X,
            crate::fdfd::Axis::Y,
            crate::fdfd::Axis::Z,
        ] {
            for k in 0..cells {
                for j in 0..cells {
                    for i in 0..cells {
                        positions[grid.index(component, (i, j, k))] =
                            grid.e_position(component, (i, j, k));
                    }
                }
            }
        }
        return Some(positions);
    }
    // the slab: row-major cells, 440 a row
    (system.id == "slab-2d").then(|| {
        (0..system.n)
            .map(|k| [(k % 440) as f64, (k / 440) as f64, 0.0])
            .collect()
    })
}

/// The matrix in Matrix Market's coordinate format, complex, general, 1-based.
pub fn matrix_market(n: usize, entries: &[(usize, usize, c64)]) -> String {
    let mut s = String::with_capacity(48 * entries.len() + 128);
    s.push_str("%%MatrixMarket matrix coordinate complex general\n");
    s.push_str("% written by photonoxide bench --export\n");
    let _ = writeln!(s, "{n} {n} {}", entries.len());
    for &(r, c, v) in entries {
        let _ = writeln!(s, "{} {} {:e} {:e}", r + 1, c + 1, v.re, v.im);
    }
    s
}

/// A vector in Matrix Market's array format, complex, general: one column.
pub fn matrix_market_vector(v: &[c64]) -> String {
    let mut s = String::with_capacity(48 * v.len() + 64);
    s.push_str("%%MatrixMarket matrix array complex general\n");
    let _ = writeln!(s, "{} 1", v.len());
    for z in v {
        let _ = writeln!(s, "{:e} {:e}", z.re, z.im);
    }
    s
}

/// The system made complex symmetric, when it can be: B = S A S⁻¹, with S² = D the diagonal for
/// which D A is symmetric (the cells' stretch factors, for the 3D curl-curl operator with PMLs;
/// see `fdfd-3d.md`, "QMR for complex symmetric matrices"), and B (S x) = S b. With `solution`'s
/// image, S x. `None` if no diagonal makes `system` symmetric.
///
/// A solver for complex symmetric matrices (an LDLᵀ) stores half of B's factors: the gain this
/// form measures, against the general LU of A.
pub fn symmetric(system: &System, solution: &[c64]) -> Option<(System, Vec<c64>)> {
    let a = crate::fdfd::krylov::Sparse::new(system.n, system.entries.iter().copied());
    let (s, b) = a.symmetrized()?;
    let mut entries: Vec<(usize, usize, c64)> = (0..system.n)
        .flat_map(|r| b.row(r).map(move |(c, v)| (r, c, v)))
        .collect();
    entries.sort_by_key(|&(r, c, _)| (c, r));
    let scaled = |v: &[c64]| -> Vec<c64> { v.iter().zip(&s).map(|(x, s)| x * s).collect() };
    Some((
        System {
            id: system.id,
            grid: system.grid.clone(),
            n: system.n,
            entries,
            rhs: scaled(&system.rhs),
        },
        scaled(solution),
    ))
}

/// Writes `system` and its solution into `dir/<id>/`: `A.mtx`, `b.mtx`, `x.mtx` and `about.json`
/// (the id, the grid, the unknowns and the nonzeros); and when the system can be made complex
/// symmetric ([`symmetric`]), the same four files for B = S A S⁻¹, S b and S x in
/// `dir/<id>/symmetric/`. Returns whether it could.
///
/// # Errors
///
/// [`Error::Io`] if a file can't be written.
pub fn write(dir: &Path, system: &System, solution: &[c64]) -> Result<bool> {
    let at = dir.join(system.id);
    write_into(&at, system, solution, None)?;
    match symmetric(system, solution) {
        Some((b, sx)) => {
            write_into(
                &at.join("symmetric"),
                &b,
                &sx,
                Some("B = S A S⁻¹, S² = D, D A symmetric"),
            )?;
            Ok(true)
        }
        None => Ok(false),
    }
}

fn write_into(at: &Path, system: &System, solution: &[c64], form: Option<&str>) -> Result<()> {
    let io = |path: &Path, e: std::io::Error| Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    };
    std::fs::create_dir_all(at).map_err(|e| io(at, e))?;
    let mut about = serde_json::json!({
        "id": system.id,
        "grid": system.grid,
        "unknowns": system.n,
        "nonzeros": system.entries.len(),
        "photonoxide": crate::VERSION,
    });
    if let Some(form) = form {
        about["form"] = form.into();
    }
    for (name, text) in [
        ("A.mtx", matrix_market(system.n, &system.entries)),
        ("b.mtx", matrix_market_vector(&system.rhs)),
        ("x.mtx", matrix_market_vector(solution)),
        ("about.json", format!("{about:#}\n")),
    ] {
        let path = at.join(name);
        std::fs::write(&path, text).map_err(|e| io(&path, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Matrix Market's coordinate format read back: n and the entries, 0-based.
    fn read(text: &str) -> (usize, Vec<(usize, usize, c64)>) {
        let mut lines = text.lines().filter(|l| !l.starts_with('%'));
        let size: Vec<usize> = lines
            .next()
            .unwrap()
            .split_whitespace()
            .map(|w| w.parse().unwrap())
            .collect();
        let entries = lines
            .map(|l| {
                let w: Vec<&str> = l.split_whitespace().collect();
                (
                    w[0].parse::<usize>().unwrap() - 1,
                    w[1].parse::<usize>().unwrap() - 1,
                    c64::new(w[2].parse().unwrap(), w[3].parse().unwrap()),
                )
            })
            .collect();
        (size[0], entries)
    }

    #[test]
    fn a_systems_matrix_reads_back_to_the_last_bit() {
        let system = system("strip-24").unwrap();
        assert_eq!(system.n, 3 * 24 * 24 * 24);
        let (n, entries) = read(&matrix_market(system.n, &system.entries));
        assert_eq!(n, system.n);
        assert_eq!(entries.len(), system.entries.len());
        for (a, b) in entries.iter().zip(&system.entries) {
            assert_eq!((a.0, a.1), (b.0, b.1));
            assert_eq!(a.2.re.to_bits(), b.2.re.to_bits());
            assert_eq!(a.2.im.to_bits(), b.2.im.to_bits());
        }
        // and awkward values: the smallest subnormal, the largest, a third
        let odd = [
            (0, 0, c64::new(5e-324, f64::MAX)),
            (1, 0, c64::new(1.0 / 3.0, -0.1)),
        ];
        let (_, back) = read(&matrix_market(2, &odd));
        for (a, b) in back.iter().zip(&odd) {
            assert_eq!(a.2.re.to_bits(), b.2.re.to_bits());
            assert_eq!(a.2.im.to_bits(), b.2.im.to_bits());
        }
    }

    #[test]
    fn the_strips_symmetric_form_is_symmetric_and_similar() {
        let a = system("strip-24").unwrap();
        // any x: B (S x) = S (A x), with S x and S (A x) from `symmetric`'s scaling of x and A x
        let x: Vec<c64> = (0..a.n)
            .map(|i| c64::new((i % 11) as f64 - 5.0, (i % 7) as f64))
            .collect();
        let mut ax = vec![c64::new(0.0, 0.0); a.n];
        for &(r, c, v) in &a.entries {
            ax[r] += v * x[c];
        }
        let with_ax = System {
            rhs: ax,
            ..a.clone()
        };
        let (b, sx) = symmetric(&with_ax, &x).expect("the curl-curl operator with PMLs");
        let mut bsx = vec![c64::new(0.0, 0.0); b.n];
        for &(r, c, v) in &b.entries {
            bsx[r] += v * sx[c];
        }
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        let r: Vec<c64> = bsx.iter().zip(&b.rhs).map(|(p, q)| p - q).collect();
        assert!(
            norm(&r) < 1e-13 * norm(&b.rhs),
            "{}",
            norm(&r) / norm(&b.rhs)
        );
        // and B is its own transpose, to the last bit
        let mut by: std::collections::HashMap<(usize, usize), c64> = Default::default();
        for &(r, c, v) in &b.entries {
            by.insert((r, c), v);
        }
        assert!(b.entries.iter().all(|&(r, c, v)| by[&(c, r)] == v));
        assert_eq!(b.entries.len(), a.entries.len());
    }

    #[test]
    fn the_exported_solution_solves_the_system() {
        let exported = system("strip-24").unwrap();
        let x = solve(&exported).unwrap();
        let mut ax = vec![c64::new(0.0, 0.0); exported.n];
        for &(r, c, v) in &exported.entries {
            ax[r] += v * x[c];
        }
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        let r: Vec<c64> = ax.iter().zip(&exported.rhs).map(|(p, q)| p - q).collect();
        // refined by the 3D solver to round-off, on any machine
        assert!(
            norm(&r) < 1e-11 * norm(&exported.rhs),
            "{}",
            norm(&r) / norm(&exported.rhs)
        );
        assert!(system("nothing").is_err());
    }

    #[test]
    fn photonoxides_factorization_is_timed_with_its_own_ordering() {
        // the residual is the raw factors', unrefined: static pivoting on the matched and scaled
        // matrix leaves round-off
        let strip = factorize(&system("strip-24").unwrap()).unwrap();
        assert_eq!(strip.ordering, "nested dissection");
        assert!(strip.factorization_seconds > 0.0);
        assert!(strip.residual < 1e-11, "{}", strip.residual);
        let slab = factorize(&system("slab-2d").unwrap()).unwrap();
        assert_eq!(slab.ordering, "nested dissection");
        assert!(slab.residual < 1e-11, "{}", slab.residual);
        // the fill: more than the matrix, less than dense
        for (f, s) in [(&strip, 3 * 24 * 24 * 24), (&slab, 149_600)] {
            assert!(
                f.factor_entries > 5 * s && f.factor_entries < s * s / 10,
                "{s}"
            );
        }
        // the strip's complex symmetric form, by L D Lᵀ: half the entries
        let a = system("strip-24").unwrap();
        let zero = vec![c64::new(0.0, 0.0); a.n];
        let (b, _) = symmetric(&a, &zero).unwrap();
        let half = factorize(&b).unwrap();
        assert!(half.residual < 1e-11, "{}", half.residual);
        let ratio = half.factor_entries as f64 / strip.factor_entries as f64;
        assert!((0.45..0.55).contains(&ratio), "{ratio}");
    }
}
