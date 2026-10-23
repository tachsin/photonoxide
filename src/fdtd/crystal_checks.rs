//! FDTD with an oriented crystal's diagonal tensor ([`Simulation::diagonal`]): the checks for its
//! tests and the validation report.

use super::{Boundaries, Field, Simulation, Source, Waveform};
use crate::fdfd::{Axis, Grid3d};
use crate::material::crystal::checks::SLAB;
use crate::material::{CrystalAxis, Orientation};
use crate::units::Frequency;

/// A pulse from an electric point source along x in 3D FDTD, on a cube of 32 cells of 40 nm
/// with CPMLs of 8, beside an off-centre block of [`SLAB`]'s crystal x-cut (ε_xx = n_o²,
/// ε_yy = n_e²) in silica, `steps` steps at a Courant number of 0.9; and the problem mirrored
/// across x = y (the block transposed, the source along y), which turns the tensor as a quarter
/// turn about z does: the crystal x-cut with its c axis along the chip's x. The largest
/// difference between E_x(i, j, k) and the mirrored problem's E_y(j, i, k) (E_y with E_x, E_z
/// with E_z), relative to the largest |E|: zero but for rounding, as the grid maps onto itself.
pub(crate) fn turned(steps: usize) -> f64 {
    let (no, ne, clad, _, lam) = SLAB;
    let tensor = |along: Option<CrystalAxis>| {
        Orientation::cut(CrystalAxis::X, along).map(|o| {
            let t =
                o.rotate([no * no, no * no, ne * ne].map(|e| num_complex::Complex64::new(e, 0.0)));
            [t[0][0].re, t[1][1].re, t[2][2].re]
        })
    };
    let (Ok(a), Ok(b)) = (tensor(None), tensor(Some(CrystalAxis::Z))) else {
        return f64::NAN;
    };
    let (n, h) = (32usize, 0.04);
    let g = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -0.64,
        y0: -0.64,
        z0: -0.64,
    };
    let block = |x: f64, y: f64, z: f64| {
        (0.0..0.4).contains(&x) && (-0.3..0.1).contains(&y) && (-0.15..0.15).contains(&z)
    };
    let cladding = [clad * clad; 3];
    let Ok(pulse) = Frequency::natural(1.0 / lam).and_then(|f| Waveform::pulse(f, 0.3)) else {
        return f64::NAN;
    };
    let run = |turned: bool| -> Option<Simulation> {
        let mut s = Simulation::diagonal(
            g,
            |x, y, z| {
                let inside = if turned {
                    block(y, x, z)
                } else {
                    block(x, y, z)
                };
                match (inside, turned) {
                    (false, _) => cladding,
                    (true, false) => a,
                    (true, true) => b,
                }
            },
            Boundaries::cpml(8),
            0.9,
        )
        .ok()?;
        let (i, j, k) = (11, 18, 16);
        let (component, at) = if turned {
            (Axis::Y, (j, i, k))
        } else {
            (Axis::X, (i, j, k))
        };
        s.add_source(Source {
            field: Field::E,
            component,
            at,
            waveform: pulse,
        })
        .ok()?;
        s.run(steps);
        Some(s)
    };
    let (Some(f), Some(t)) = (run(false), run(true)) else {
        return f64::NAN;
    };
    let index = |i: usize, j: usize, k: usize| (k * n + j) * n + i;
    let mut worst = 0.0f64;
    let mut largest = 0.0f64;
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                for (p, q) in [(Axis::X, Axis::Y), (Axis::Y, Axis::X), (Axis::Z, Axis::Z)] {
                    let e = f.e(p)[index(i, j, k)];
                    worst = worst.max((e - t.e(q)[index(j, i, k)]).abs());
                    largest = largest.max(e.abs());
                }
            }
        }
    }
    worst / largest
}
