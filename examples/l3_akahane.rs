//! Akahane et al.'s L3 photonic-crystal nanocavity as a body in space, in 3D FDTD.
//!
//! Y. Akahane, T. Asano, B.-S. Song, S. Noda, "High-Q photonic nanocavity in a two-dimensional
//! photonic crystal", Nature 425, 944 (2003),
//! [doi:10.1038/nature02063](https://doi.org/10.1038/nature02063), Fig. 1, p. 945 and Fig. 4:
//!
//! - **The device:** a silicon slab 0.6a thick (0.25 µm) in air, a triangular lattice of air
//!   holes of radius 0.29a (0.12 µm), a = 0.42 µm, three holes missing in a row (the L3 cavity),
//!   and the holes at both its ends moved out along the row by 0.15a (their Fig. 1c).
//! - **The result:** the cavities they made, each with the end holes moved by 0 to 0.25a, all
//!   resonate between 1.565 and 1.580 µm (Fig. 4a's axis); moving the end holes by 0.15a raises
//!   Q/V by "a factor of 10" (Fig. 4c), V unchanged ("small and constant at (6–7) × 10⁻¹⁴
//!   cm³"), so Q by about 10, to 45,000. Their own 3D FDTD is shown, but its numbers aren't
//!   printed.
//!
//! Here the cavity is one body ([`photonoxide::geometry::Body`]): a box of silicon (the catalogue's,
//! Li's, at 1.57 µm) less the lattice's disks extruded through it, 21 × 15 sites; FDTD's
//! smoothing ([`photonoxide::fdtd::Structure`]) takes it as it takes any body, each value of E's
//! permittivity the subpixel average over its cell from the body's surface and normal
//! (`Coupling::Diagonal`). CPMLs of 12 cells, a·0.7 of air above and below the slab inside them.
//! A pulse of E_y at the cavity's centre, 0.04/a wide about a/λ = 0.264; E_y at a point near
//! the centre after the pulse, by harmonic inversion (`fdtd::harmonic_inversion`): the
//! fundamental mode's frequency and Q = ω/2γ.
//!
//! - **CI** (about a minute and a half on 20 threads): the cavity's geometry as a body, exactly:
//!   its volume from its cells' fills against T (|A| − N π r²), the cavity's sites silicon and
//!   every hole air through the slab; and the unshifted and shifted cavities at 8 cells a lattice
//!   constant (52.5 nm cells), the record 300 µm/c after the pulse.
//! - **`--full`** (about 40 minutes on 20 threads): 12 and 16 cells a lattice constant, the
//!   record 1000 µm/c after the pulse.
//! - **The checks:** each resonance against 1.565 to 1.580 µm widened by 2 % (1.5 % at 12 and
//!   16 cells) for the fabricated holes' radius and slab's thickness, which they give only to
//!   their nominal values, and the grid; Q's rise against the measured factor of about 10, and
//!   the shifted cavity's Q against the measured 45,000, each within a factor of 2 (the
//!   measured Q is limited by the fabrication's disorder, the computed one by the lattice's 21 ×
//!   15 sites and the grid).
//!
//! What it found (`--full`, 20 threads): a/λ 0.2635, 0.2641 and 0.2639 unshifted and 0.2618,
//! 0.2623 and 0.2621 shifted at 8, 12 and 16 cells, so λ = 1.591 and 1.603 µm, 0.7 % and 1.4 %
//! above the measured span; Q 5574, 5413 and 5107 unshifted and 56 156, 52 275 and 38 182
//! shifted, a rise of 10.1, 9.7 and 7.5. The frequencies settle to 1e-3; the shifted cavity's Q,
//! decaying 4 % over the record, doesn't yet settle with the grid, and is reported, not
//! claimed, beyond the factor of 2.
//!//! ```sh
//! cargo run --release --example l3_akahane             # the body, and 8 cells a lattice constant
//! cargo run --release --example l3_akahane -- --full   # 12 and 16 cells, 1000 µm/c records
//! ```

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Axis, Grid3d};
use photonoxide::fdtd::{
    Average, Body, Boundaries, Coupling, Current, Field, Permittivity, Simulation, Smoothing,
    Structure, Waveform, harmonic_inversion,
};
use photonoxide::geometry::{Array, Bounds3d, Lattice, Point, Shape};
use photonoxide::material;
use photonoxide::units::{Frequency, Length, Wavelength};

/// The lattice constant, µm.
const A: f64 = 0.42;
/// The holes' radius, the slab's thickness and the end holes' shift, over a.
const R: f64 = 0.29;
const T: f64 = 0.6;
const SHIFT: f64 = 0.15;
/// The lattice's sites: i from −10 to 10 along a row, rows j from −7 to 7.
const SITES: [i64; 2] = [10, 7];
/// The wavelength the materials are at, µm.
const MATERIALS: f64 = 1.57;
/// The resonances Fig. 4a shows, µm.
const MEASURED: [f64; 2] = [1.565, 1.580];

fn um(v: f64) -> Length {
    Length::um(v)
}

/// The holes: the L3 lattice's disks, the end holes moved out by `shift` (µm).
fn holes(shift: f64) -> photonoxide::Result<Array> {
    let lattice = Lattice::hexagonal(um(A), [-SITES[0], SITES[0]], [-SITES[1], SITES[1]])?
        .remove([-1, 0])?
        .remove([0, 0])?
        .remove([1, 0])?
        .shift([-2, 0], [um(-shift), um(0.0)])?
        .shift([2, 0], [um(shift), um(0.0)])?;
    Array::of(&lattice, &Shape::circle(Point::um(0.0, 0.0), um(R * A))?)
}

/// The cavity as one body: a slab `half` µm across each way from the centre, less the holes.
fn cavity(shift: f64, half: [f64; 2]) -> photonoxide::Result<Body> {
    let t = T * A;
    let slab = Body::cuboid([-half[0], -half[1], -0.5 * t], [half[0], half[1], 0.5 * t])?;
    Ok(Body::difference(
        slab,
        Body::extruded(Shape::Array(holes(shift)?)),
    ))
}

/// The body against the paper's geometry: its volume from its fills over a grid of a/4 cells
/// against T (|A| − N π r²), and the cavity's sites and the band the shift uncovers silicon,
/// every hole air (its centre and a point just inside its edge), through the slab. The
/// relative difference in volume, the points wrong and the points tested.
fn geometry() -> photonoxide::Result<(f64, usize, usize)> {
    let (r, t, s) = (R * A, T * A, SHIFT * A);
    let row = A * 3f64.sqrt() / 2.0;
    let half = [(SITES[0] as f64 + 1.0) * A, (SITES[1] as f64 + 1.0) * row];
    let body = cavity(s, half)?;
    let n = holes(s)?.len() as f64;
    let want = t * (4.0 * half[0] * half[1] - n * PI * r * r);
    let h = 0.25 * A;
    let (nx, ny) = (
        (2.0 * half[0] / h).ceil() as usize + 2,
        (2.0 * half[1] / h).ceil() as usize + 2,
    );
    let mut sum = 0.0;
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..4 {
                let lo = [
                    -half[0] - h + i as f64 * h,
                    -half[1] - h + j as f64 * h,
                    -2.0 * h + k as f64 * h,
                ];
                sum += body.fill(Bounds3d::new(lo, [lo[0] + h, lo[1] + h, lo[2] + h]));
            }
        }
    }
    let (mut wrong, mut points) = (0, 0);
    for z in [-0.49 * t, 0.0, 0.49 * t] {
        for x in [-A, 0.0, A, 2.0 * A - r + 0.5 * s, -(2.0 * A - r + 0.5 * s)] {
            wrong += usize::from(!body.contains([x, 0.0, z]));
            points += 1;
        }
        for e in holes(s)?.elements() {
            let b = e.shape.bounds();
            let (x, y) = (
                0.5 * (b.min.x + b.max.x).to_um(),
                0.5 * (b.min.y + b.max.y).to_um(),
            );
            wrong += usize::from(body.contains([x, y, z]));
            wrong += usize::from(body.contains([x + 0.99 * r, y, z]));
            points += 2;
        }
    }
    Ok(((sum - want).abs() / want, wrong, points))
}

/// The fundamental mode of the cavity with its end holes moved by `shift` (over a), on `cells`
/// cells a lattice constant, its record `record` µm/c after the pulse: the frequency (c/µm),
/// Q and the grid.
fn resonance(shift: f64, cells: usize, record: f64) -> photonoxide::Result<(f64, f64, Grid3d)> {
    let h = A / cells as f64;
    let pml = 12;
    let row = A * 3f64.sqrt() / 2.0;
    // the lattice and a lattice constant beyond, a·0.7 of air above and below, then the CPMLs
    let span = [
        2.0 * (SITES[0] as f64 + 1.0) * A,
        2.0 * (SITES[1] as f64 + 1.0) * row,
        (T + 1.4) * A,
    ];
    let n = span.map(|s| (s / h).round() as usize + 2 * pml);
    let grid = Grid3d {
        nx: n[0],
        ny: n[1],
        nz: n[2],
        dx: h,
        dy: h,
        dz: h,
        x0: -0.5 * n[0] as f64 * h,
        y0: -0.5 * n[1] as f64 * h,
        z0: -0.5 * n[2] as f64 * h,
    };
    let lambda = Wavelength::um(MATERIALS)?;
    let si = material::silicon().permittivity(lambda)?.re;
    // the slab and the holes run into the CPMLs
    let far = [n[0] as f64 * h, n[1] as f64 * h];
    let structure = Structure::new(Permittivity::isotropic(1.0)?)
        .with(cavity(shift * A, far)?, Permittivity::isotropic(si)?);
    let smoothing = Smoothing {
        average: Average::Subpixel,
        diameter: 1.0,
        coupling: Coupling::Diagonal,
    };
    let mut sim = Simulation::smoothed(grid, &structure, smoothing, Boundaries::cpml(pml), 0.9)?;
    let centre = 0.264 / A;
    let pulse = Waveform::pulse(Frequency::natural(centre)?, 0.04 / A)?;
    let Waveform::Gaussian { delay, width, .. } = pulse else {
        unreachable!("a pulse is a Gaussian")
    };
    let at = |x: f64, y: f64| {
        (
            ((x - grid.x0) / h).round() as usize,
            ((y - grid.y0) / h).round() as usize,
            ((0.0 - grid.z0) / h).round() as usize,
        )
    };
    sim.add_current(Current {
        field: Field::E,
        values: vec![(Axis::Y, at(0.0, 0.0), c64::new(1.0, 0.0))],
        waveform: pulse,
    })?;
    let probe = sim.add_probe(Field::E, Axis::Y, at(0.13 * A, 0.07 * A))?;
    let over = delay + 6.0 * width;
    common::run_until(
        &mut sim,
        over + record,
        &format!("l3_akahane, {cells} cells a lattice constant, end holes moved {shift}a"),
    );
    let dt = sim.dt();
    // the record after the pulse, about four samples a period
    let every = ((0.25 / centre / dt).floor() as usize).max(1);
    let first = (over / dt).ceil() as usize;
    let p = sim.probe(probe);
    let signal: Vec<c64> = (first..p.len())
        .step_by(every)
        .map(|k| c64::new(p[k], 0.0))
        .collect();
    let found = harmonic_inversion(
        &signal,
        every as f64 * dt,
        Frequency::natural(centre - 0.02 / A)?,
        Frequency::natural(centre + 0.02 / A)?,
    )?;
    let best = found
        .iter()
        .filter(|r| r.error < 1e-3)
        .max_by(|x, y| x.amplitude.norm().total_cmp(&y.amplitude.norm()))
        .ok_or_else(|| photonoxide::Error::InvalidValue {
            what: "resonance",
            reason: "harmonic inversion found none in the window".to_owned(),
        })?;
    Ok((best.frequency, best.q(), grid))
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let full = std::env::args().any(|a| a == "--full");
    println!(
        "Akahane et al.'s L3 cavity: a = {A} um, slab {T}a, holes of radius {R}a, {} x {} sites, \
         the end holes moved by 0 and {SHIFT}a",
        2 * SITES[0] + 1,
        2 * SITES[1] + 1
    );
    let mut checks = common::Checks::default();
    let (volume, wrong, points) = geometry()?;
    println!(
        "the cavity as one body (exact: no grid): its volume from its fills over a/4 cells \
         {volume:.1e} off T(|A| - N pi r^2); {wrong} of {points} points on the wrong side"
    );
    checks.against("volume, relative difference", volume, "fills", 0.0, 1e-7);
    checks.count("points on the wrong side", wrong, 0);
    let cells: &[usize] = if full { &[12, 16] } else { &[8] };
    let record = if full { 1000.0 } else { 300.0 };
    let middle = 0.5 * (MEASURED[0] + MEASURED[1]);
    for &n in cells {
        let (f0, q0, grid) = resonance(0.0, n, record)?;
        let (f1, q1, _) = resonance(SHIFT, n, record)?;
        println!(
            "3D FDTD, {n} cells a lattice constant ({:.2} nm cells, {} x {} x {}), the record \
             {record} um/c after the pulse:",
            1e3 * A / n as f64,
            grid.nx,
            grid.ny,
            grid.nz
        );
        println!(
            "  unshifted: a/lambda {:.4}, lambda {:.4} um, Q {q0:.0}",
            f0 * A,
            1.0 / f0
        );
        println!(
            "  shifted {SHIFT}a: a/lambda {:.4}, lambda {:.4} um, Q {q1:.0} ({:.1} times)",
            f1 * A,
            1.0 / f1,
            q1 / q0
        );
        // the measured span, widened by the fabricated holes' and slab's uncertainty and the grid
        let widened = if n >= 12 { 0.015 } else { 0.02 };
        let tolerance = 0.5 * (MEASURED[1] - MEASURED[0]) + widened * middle;
        checks.compare(
            &format!("lambda unshifted (um), {n} cells"),
            1.0 / f0,
            middle,
            tolerance,
        );
        checks.compare(
            &format!("lambda shifted (um), {n} cells"),
            1.0 / f1,
            middle,
            tolerance,
        );
        // Q/V up tenfold, V the same; and the measured Q, within a factor of 2 (log2)
        checks.compare(
            &format!("log2 of Q's rise, {n} cells"),
            (q1 / q0).log2(),
            10f64.log2(),
            1.0,
        );
        checks.compare(
            &format!("log2 of Q shifted, {n} cells"),
            q1.log2(),
            45000f64.log2(),
            1.0,
        );
    }
    Ok(checks.finish())
}
