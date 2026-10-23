//! FDFD with an oriented crystal's diagonal tensor: the checks for its tests and the validation
//! report.

use num_complex::Complex64 as c64;

use super::{Axis, Boundaries, Boundaries3d, Grid, Grid3d, Polarization, Solver2d, Solver3d};
use crate::averaging::Averaging;
use crate::geometry::{Point, Shape};
use crate::material::catalogue::{Conditions, entry};
use crate::material::crystal::checks::{SLAB, cut_slab_exact};
use crate::material::{CrystalAxis, Material, Orientation};
use crate::stack::{Layer, LayerStack, Structure};
use crate::units::{Length, Wavelength};

fn c(x: f64) -> c64 {
    c64::new(x, 0.0)
}

/// The diagonal of [`SLAB`]'s tensor in the chip's frame, cut `cut` with `along_x` along the
/// chip's x.
fn slab_diagonal(cut: CrystalAxis, along_x: Option<CrystalAxis>) -> [c64; 3] {
    let (no, ne, ..) = SLAB;
    let t = Orientation::cut(cut, along_x)
        .map(|o| o.rotate([c(no * no), c(no * no), c(ne * ne)]))
        .unwrap_or([[c(f64::NAN); 3]; 3]);
    [t[0][0], t[1][1], t[2][2]]
}

/// The fundamental port mode of [`SLAB`] cut `cut`, guided along the chip's x, in 2D FDFD on the
/// guide's vertical cross-section at spacing `h` (µm): the solver's x is the chip's x, its y the
/// chip's z (up) and its z the chip's y, so E along z is the slab's TE and H along z its TM, and
/// the tensor's diagonal (ε_xx, ε_yy, ε_zz) in the chip's frame is (ε_xx, ε_zz, ε_yy) in the
/// solver's ([`Solver2d::diagonal`]). A 4 µm window, PMLs of 20 cells all round, the slab's faces
/// on the grid's. Its effective index and [`cut_slab_exact`]'s.
pub(crate) fn slab_index(cut: CrystalAxis, kind: crate::mode::Polarization, h: f64) -> (f64, f64) {
    let (_, _, clad, d, lam) = SLAB;
    let exact = cut_slab_exact(cut, kind);
    let t = slab_diagonal(cut, None);
    let core = [t[0], t[2], t[1]];
    let cladding = [c(clad * clad); 3];
    let pml = 20;
    let nx = (0.1 / h).round() as usize + 2 * pml;
    let ny = (4.0 / h).round() as usize + 2 * pml;
    let grid = Grid {
        nx,
        ny,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -2.0 - pml as f64 * h,
    };
    let polarization = match kind {
        crate::mode::Polarization::Te => Polarization::Ez,
        crate::mode::Polarization::Tm => Polarization::Hz,
    };
    let found = Wavelength::um(lam)
        .and_then(|w| {
            Solver2d::diagonal(
                grid,
                polarization,
                w,
                |_, y| if y.abs() < d / 2.0 { core } else { cladding },
                Boundaries::pml(pml),
            )
        })
        .and_then(|s| s.port_modes(pml + (0.05 / h) as usize, 1))
        .ok()
        .and_then(|m| m.first().map(|m| m.effective_index().re))
        .unwrap_or(f64::NAN);
    (found, exact)
}

/// [`slab_index`]'s error at `steps` (µm), and its order of convergence from each step to the
/// next, log₂ of the errors' ratio when each step is half the one before.
pub(crate) fn slab_errors(
    cut: CrystalAxis,
    kind: crate::mode::Polarization,
    steps: &[f64],
) -> (Vec<f64>, Vec<f64>) {
    let errors: Vec<f64> = steps
        .iter()
        .map(|&h| {
            let (found, exact) = slab_index(cut, kind, h);
            (found - exact).abs()
        })
        .collect();
    let orders = errors
        .windows(2)
        .zip(steps.windows(2))
        .map(|(e, s)| (e[0] / e[1]).ln() / (s[0] / s[1]).ln())
        .collect();
    (errors, orders)
}

/// A magnetic line source in 2D FDFD (H along z, E in the plane), on a 3 µm square of 20 nm
/// cells with PMLs of 20, beside an off-centre block of [`SLAB`]'s crystal x-cut (ε_xx = n_o²,
/// ε_yy = n_e²) in silica; and the problem mirrored across x = y (the block and the source
/// transposed), which turns the tensor as a quarter turn about z does: the crystal x-cut with
/// its c axis along the chip's x (ε_xx = n_e², ε_yy = n_o²). The largest difference between
/// H(i, j) and the mirrored problem's H(j, i), relative to the largest |H|: zero but for
/// rounding, as the grid maps onto itself.
pub(crate) fn turned_2d() -> f64 {
    let (_, _, clad, _, lam) = SLAB;
    let (a, b) = (
        slab_diagonal(CrystalAxis::X, None),
        slab_diagonal(CrystalAxis::X, Some(CrystalAxis::Z)),
    );
    let n = 150;
    let h = 0.02;
    let grid = Grid {
        nx: n,
        ny: n,
        dx: h,
        dy: h,
        x0: -1.5,
        y0: -1.5,
    };
    let block = |x: f64, y: f64| (0.1..0.9).contains(&x) && (-0.5..0.2).contains(&y);
    let cladding = [c(clad * clad); 3];
    let source = |turned: bool| {
        let mut m = vec![c(0.0); n * n];
        let (i, j) = (60, 85);
        let (i, j) = if turned { (j, i) } else { (i, j) };
        m[j * n + i] = c(1.0);
        m
    };
    let solve = |turned: bool| {
        Wavelength::um(lam)
            .and_then(|w| {
                Solver2d::diagonal(
                    grid,
                    Polarization::Hz,
                    w,
                    |x, y| match (turned, if turned { block(y, x) } else { block(x, y) }) {
                        (_, false) => cladding,
                        (false, true) => a,
                        (true, true) => b,
                    },
                    Boundaries::pml(20),
                )
            })
            .and_then(|s| s.solve(&source(turned)))
    };
    let (Ok(f), Ok(g)) = (solve(false), solve(true)) else {
        return f64::NAN;
    };
    let mut worst = 0.0f64;
    let mut largest = 0.0f64;
    for j in 0..n {
        for i in 0..n {
            worst = worst.max((f.at(i, j) - g.at(j, i)).norm());
            largest = largest.max(f.at(i, j).norm());
        }
    }
    worst / largest
}

/// Lithium niobate (the catalogue's) oriented `orientation`.
pub(crate) fn linbo3(orientation: Orientation) -> Option<Material> {
    entry("linbo3")?
        .oriented(None, Conditions::default(), orientation)
        .ok()
}

/// A 3D FDFD problem from a stack ([`Solver3d::from_structure`], averaged from samples): a
/// 300 nm film of lithium niobate x-cut on silica, under air, etched to an off-centre block, an
/// electric dipole along x beside it, on a 2 µm cube of 50 nm cells with PMLs of 8; and the
/// problem mirrored across x = y, the crystal turned a quarter turn about z (x-cut, its c axis
/// along the chip's x). The largest difference between E_x(i, j, k) and the mirrored problem's
/// E_y(j, i, k) (and E_y with E_x, E_z with E_z), relative to the largest |E|.
pub(crate) fn turned_3d() -> f64 {
    let lam = 1.55;
    let n = 40;
    let h = 0.05;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -1.0,
        y0: -1.0,
        z0: -1.0,
    };
    let structure = |turned: bool| -> Option<Structure> {
        let along = turned.then_some(CrystalAxis::Z);
        let film = linbo3(Orientation::cut(CrystalAxis::X, along).ok()?)?;
        let stack = LayerStack::new(
            crate::material::silica(),
            vec![Layer {
                name: "LN".into(),
                thickness: Length::nm(300.0),
                material: film,
                background: crate::material::vacuum(),
            }],
            crate::material::vacuum(),
        )
        .ok()?;
        let mut s = Structure::new(stack);
        let (cx, cy) = (0.25, -0.1);
        let (cx, cy, w, l) = if turned {
            (cy, cx, 0.5, 0.9)
        } else {
            (cx, cy, 0.9, 0.5)
        };
        let block = Shape::rect(Point::um(cx, cy), Length::um(w), Length::um(l)).ok()?;
        s.draw("LN", block).ok()?;
        Some(s)
    };
    let (Some(plain), Some(turned)) = (structure(false), structure(true)) else {
        return f64::NAN;
    };
    let source = |swap: bool| {
        let mut j = vec![c(0.0); grid.unknowns()];
        let (i, jj, k) = (14, 22, 23);
        let r = if swap {
            grid.index(Axis::Y, (jj, i, k))
        } else {
            grid.index(Axis::X, (i, jj, k))
        };
        j[r] = c(1.0);
        j
    };
    let solve = |s: &Structure, swap: bool| {
        Wavelength::um(lam)
            .and_then(|w| {
                Solver3d::from_structure(grid, w, s, Boundaries3d::pml(8), Averaging::Samples)
            })
            .and_then(|solver| solver.solve(&source(swap)))
    };
    let (Ok(f), Ok(g)) = (solve(&plain, false), solve(&turned, true)) else {
        return f64::NAN;
    };
    let mut worst = 0.0f64;
    let mut largest = 0.0f64;
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                for (a, b) in [(Axis::X, Axis::Y), (Axis::Y, Axis::X), (Axis::Z, Axis::Z)] {
                    let e = f.e(a, (i, j, k));
                    worst = worst.max((e - g.e(b, (j, i, k))).norm());
                    largest = largest.max(e.norm());
                }
            }
        }
    }
    worst / largest
}
