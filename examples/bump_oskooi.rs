//! Subpixel smoothing's convergence: the power a small bump on a guide scatters.
//!
//! A. F. Oskooi, D. Roundy, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, "Meep: A
//! flexible free-software package for electromagnetic simulations by the FDTD method", Comput.
//! Phys. Commun. 181, 687 (2010),
//! [doi:10.1016/j.cpc.2009.11.008](https://doi.org/10.1016/j.cpc.2009.11.008), Section 3 and
//! Fig. 7:
//!
//! - **The problem:** "a small semicircular bump in a dielectric waveguide (ε = 12), excited by
//!   a point-dipole source in the waveguide", in 2D, at resolutions of 10 to 100 pixels a.
//! - **The result:** "Appropriate subpixel smoothing of the dielectric interfaces leads to
//!   roughly second-order [O(Δx²)] convergence ..., whereas the unsmoothed structure has only
//!   first-order convergence", "erratic linear convergence" (Section 3).
//!
//! Settings the paper leaves to its Fig. 7's inset, read off it here (rendered at 8 times):
//!
//! - **The guide:** a = 1 µm wide (121 pixels in the inset), along x.
//! - **The bump:** a half disc of radius 0.25a (60 pixels across) on the guide's upper face, its
//!   centre 4.5a (548 pixels) from the dipole.
//! - **The frequency:** f = 0.2 c/a. The inset's guided wavelength, 1.64a (198 pixels from crest
//!   to crest), is the slab's at f = 0.2: n_eff = 3.04, 1/(0.2 × 3.04) = 1.64.
//! - **The polarization:** E_z, out of the plane, from a dipole J_z on the guide's axis: the
//!   inset's field is a scalar, and its source a ring about a point.
//!
//! Settings the paper leaves open, set here:
//!
//! - **The cell:** 6.75a × 3.5a inside CPMLs 1.5a thick (R = 1e-8, order 3).
//! - **The scattered power:** the flux of the scattered field, E with the bump less E without
//!   it on the same grid, out of a box 2a × 2.5a about the bump: what the bump sends into the
//!   guide either way and into the cladding. At f = 0.2, from a pulse 0.1 c/a wide, each run's
//!   transforms divided by the pulse's spectrum.
//! - **Smoothing:** photonoxide's subpixel smoothing (`Smoothing::default()`) against ε sampled
//!   at each value of E (`Average::Sampled`, "no smoothing"). For E_z, along every interface,
//!   the smoothed ε is the mean over each value's cell.
//! - **The reference:** the smoothed power at 50 pixels a. (At 60 it differs by 4e-4 of
//!   itself, a tenth of the smoothed error at 30.)
//! - **Resolutions:** 10 to 30 pixels a, two by two (the paper's run to 100; here the runs grow
//!   as the cube of the resolution, 50 taking as long as all the others).
//!
//! The paper's orders are slopes on a log–log plot ("roughly" and "erratic"), so each is a
//! least-squares slope here, compared within 0.6 of 2 and 0.5 of 1; and smoothing is the more
//! accurate at every resolution, as in Fig. 7. Fig. 7's errors themselves depend on the
//! geometry it only draws; they are printed beside the paper's, read at 10 and 30 pixels a
//! (smoothed 0.08 and about 0.005, unsmoothed 0.2 and about 0.07), not checked.
//!
//! ```sh
//! cargo run --release --example bump_oskooi
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{
    Average, Body, Boundaries, Cpml, Field, Permittivity, Simulation, Smoothing, Source, Structure,
    Waveform,
};
use photonoxide::geometry::{Point, Shape};
use photonoxide::units::{Frequency, Length};

/// The guide's permittivity, the bump's radius and its centre along x (a = 1 µm).
const CORE: f64 = 12.0;
const RADIUS: f64 = 0.25;
const BUMP: f64 = 2.25;
/// The dipole's position along x.
const DIPOLE: f64 = -2.25;
/// The cell inside the CPMLs: x and y extents (a), and the CPMLs' thickness.
const X: (f64, f64) = (-3.0, 3.75);
const Y: (f64, f64) = (-1.5, 2.0);
const CPML: f64 = 1.5;
/// The box about the bump: x and y extents.
const BOX_X: (f64, f64) = (1.25, 3.25);
const BOX_Y: (f64, f64) = (-1.0, 1.5);

/// The structure, with or without the bump.
fn structure(bump: bool) -> Structure {
    let um = |v: f64| Length::um(v);
    let core = Permittivity::isotropic(CORE).expect("the core");
    let guide = Shape::rect(Point::um(0.0, 0.0), um(100.0), um(1.0)).expect("the guide");
    let s = Structure::new(Permittivity::isotropic(1.0).expect("air"))
        .with(Body::extruded(guide), core);
    if bump {
        let disc = Shape::circle(Point::um(BUMP, 0.5), um(RADIUS)).expect("the bump");
        s.with(Body::extruded(disc), core)
    } else {
        s
    }
}

/// The flux of the scattered field out of the box about the bump at `resolution` pixels a,
/// with ε smoothed or sampled.
fn scattered(resolution: usize, smoothed: bool) -> f64 {
    let h = 1.0 / resolution as f64;
    let cells = |v: f64| (v * resolution as f64).round() as usize;
    let pml = cells(CPML);
    let n = [cells(X.1 - X.0) + 2 * pml, cells(Y.1 - Y.0) + 2 * pml, 1];
    let grid = Grid3d {
        nx: n[0],
        ny: n[1],
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: X.0 - CPML,
        y0: Y.0 - CPML,
        z0: 0.0,
    };
    // node indices: the guide's axis y = 0 and the dipole are on nodes at every resolution
    let node = |v: f64, origin: f64| cells(v - origin);
    let boundaries = Boundaries {
        x: Edges::Pml {
            low: pml,
            high: pml,
        },
        y: Edges::Pml {
            low: pml,
            high: pml,
        },
        z: Edges::Bloch { k: 0.0 },
        cpml: Cpml::default(),
    };
    let smoothing = if smoothed {
        Smoothing::default()
    } else {
        Smoothing::with(Average::Sampled)
    };
    let frequency = Frequency::natural(0.2).expect("0.2 c/a");
    let pulse = Waveform::pulse(frequency, 0.1).expect("the pulse");
    let (x0, x1) = (node(BOX_X.0, grid.x0), node(BOX_X.1, grid.x0));
    let (y0, y1) = (node(BOX_Y.0, grid.y0), node(BOX_Y.1, grid.y0));
    let fields = |bump: bool| {
        let mut s = Simulation::smoothed(grid, &structure(bump), smoothing, boundaries, 0.9)
            .expect("the cell");
        let axis = node(0.0, grid.y0);
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (node(DIPOLE, grid.x0), axis, 0),
            waveform: pulse,
        })
        .expect("the dipole");
        // the box's four sides, each two nodes deep
        let sides = [
            ((x0, y0, 0), (x0 + 1, y1 + 1, 0)),
            ((x1, y0, 0), (x1 + 1, y1 + 1, 0)),
            ((x0, y0, 0), (x1 + 1, y0 + 1, 0)),
            ((x0, y1, 0), (x1 + 1, y1 + 1, 0)),
        ]
        .map(|(low, high)| s.add_dft(low, high, &[frequency]).expect("a side"));
        let probe = s
            .add_probe(Field::E, Axis::Z, (x1 + 1, axis, 0))
            .expect("the probe");
        let decayed = s
            .run_until_decayed(&[probe], 1e-11, 10.0, 2000.0)
            .expect("the run");
        assert!(decayed, "the field didn't decay");
        let spectrum = pulse.spectrum(Field::E, s.dt(), s.steps(), frequency);
        move |field: Field, component: Axis, i: usize, j: usize| -> c64 {
            sides
                .iter()
                .find_map(|&m| s.dft(m).value(field, component, (i, j, 0), 0))
                .expect("a value on the box")
                / spectrum
        }
    };
    let with = fields(true);
    let without = fields(false);
    let e = |i, j| with(Field::E, Axis::Z, i, j) - without(Field::E, Axis::Z, i, j);
    let hx = |i, j| with(Field::H, Axis::X, i, j) - without(Field::H, Axis::X, i, j);
    let hy = |i, j| with(Field::H, Axis::Y, i, j) - without(Field::H, Axis::Y, i, j);
    // out of the box through the planes halfway after nodes x0, x1 (normal x) and y0, y1
    // (normal y): S_x = -1/2 Re(E_z H_y*), S_y = 1/2 Re(E_z H_x*), E_z the mean of the nodes
    // either side, H on the plane
    let mut flux = 0.0;
    for (plane, sign) in [(x0, -1.0), (x1, 1.0)] {
        for j in y0 + 1..=y1 {
            let ez = 0.5 * (e(plane, j) + e(plane + 1, j));
            flux += sign * -0.5 * (ez * hy(plane, j).conj()).re * h;
        }
    }
    for (plane, sign) in [(y0, -1.0), (y1, 1.0)] {
        for i in x0 + 1..=x1 {
            let ez = 0.5 * (e(i, plane) + e(i, plane + 1));
            flux += sign * 0.5 * (ez * hx(i, plane).conj()).re * h;
        }
    }
    // the dipole is a current on one value, J = p/h² for a moment p: per unit moment
    flux * (resolution as f64).powi(4)
}

/// The least-squares slope of log y against log x.
fn slope(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let (lx, ly): (Vec<f64>, Vec<f64>) = points.iter().map(|&(x, y)| (x.ln(), y.ln())).unzip();
    let (mx, my) = (lx.iter().sum::<f64>() / n, ly.iter().sum::<f64>() / n);
    let sxy: f64 = lx.iter().zip(&ly).map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = lx.iter().map(|x| (x - mx).powi(2)).sum();
    sxy / sxx
}

/// A value rounded to `digits` significant digits, so that what is printed doesn't change with
/// the last bits of a platform's math library.
fn rounded(v: f64, digits: i32) -> f64 {
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

pub fn main() -> ExitCode {
    let reference_resolution = 50;
    println!("Oskooi et al.'s Fig. 7: a half disc of radius 0.25a on a guide of eps = 12, a wide,");
    println!("E_z from a dipole 4.5a from it, f = 0.2 c/a; the scattered field's flux out of a");
    println!("2a x 2.5a box about the bump, per unit dipole moment (a = 1 um)");
    let reference = scattered(reference_resolution, true);
    println!(
        "reference: smoothed at {reference_resolution} pixels/a, {:.5e}",
        rounded(reference, 6)
    );
    println!("pixels/a   smoothed: power, error   no smoothing: power, error");
    let (mut smoothed, mut sampled) = (Vec::new(), Vec::new());
    for r in (10..=30).step_by(2) {
        let (s, u) = (scattered(r, true), scattered(r, false));
        let (es, eu) = (
            (s - reference).abs() / reference,
            (u - reference).abs() / reference,
        );
        println!(
            "{r:>8}   {:.5e}, {:.2e}   {:.5e}, {:.2e}",
            rounded(s, 6),
            rounded(es, 3),
            rounded(u, 6),
            rounded(eu, 3)
        );
        smoothed.push((r as f64, es));
        sampled.push((r as f64, eu));
    }
    let mut checks = common::Checks::default();
    // the paper's errors, read off Fig. 7 at 10 and 30 pixels/a (not checked: its geometry is
    // only drawn)
    println!("Fig. 7, read: smoothed 0.08 at 10 and about 0.005 at 30; no smoothing 0.2 and 0.07");
    checks.compare(
        "order, smoothed (least squares)",
        rounded(-slope(&smoothed), 3),
        2.0,
        0.6,
    );
    checks.compare(
        "order, no smoothing (least squares)",
        rounded(-slope(&sampled), 3),
        1.0,
        0.5,
    );
    // smoothing is the more accurate at every resolution, as in Fig. 7
    let below = smoothed
        .iter()
        .zip(&sampled)
        .filter(|(s, u)| s.1 < u.1)
        .count();
    checks.count(
        "resolutions where smoothing is better",
        below,
        smoothed.len(),
    );
    checks.finish()
}
