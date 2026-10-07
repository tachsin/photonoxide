//! Subpixel smoothing's convergence: a mode of a square lattice of elliptical air holes in
//! ε = 12, its frequency's error against resolution with and without smoothing.
//!
//! A. Farjadpour et al., "Improving accuracy by subpixel smoothing in the finite-difference time
//! domain", Opt. Lett. 31, 2972 (2006),
//! [doi:10.1364/OL.31.002972](https://doi.org/10.1364/OL.31.002972), Fig. 1:
//!
//! - **The problem:** a square lattice (period a) of elliptical air holes in ε = 12, the TE
//!   polarization (E in the plane), its lowest frequency at a Bloch wave vector "not aligned
//!   with the grid", against a plane-wave calculation at a very high resolution; FDTD from 10 to
//!   100 pixels a period.
//! - **Their smoothing** (their Eq. 1): ε̃⁻¹ = P⟨ε⁻¹⟩ + (1 − P)⟨ε⟩⁻¹ over each pixel about the
//!   E component's point, its off-diagonal entries applied to "the Dy and Dz components from
//!   their four adjacent Yee points" averaged: `Coupling::Points` here. "As a trick to make the
//!   quadratic convergence of our method more apparent", also over pixels twice as wide (2Δx).
//! - **The result:** no smoothing converges erratically; the mean ε, and the others they tried,
//!   linearly; their method with "the smallest errors by a large margin", erratically at s = 1
//!   and quadratically at s = 2.
//!
//! Read off the figure (at 600 dpi, 1771 pixels a decade of resolution and 233 a decade of
//! error, the frame's ticks consistent to a pixel; the filled markers found by colour and
//! eroded clear of the lines): the new method at 2Δx is 1.96e-3 at 20 pixels a period, 3.80e-4
//! at 40 and 4.25e-5 at 100, a least-squares slope of −2.43 over 20 to 100; the mean ε is
//! 2.83e-3, 1.12e-3 and 3.34e-4, a slope of −1.33. Their s = 1 and unsmoothed curves are too
//! erratic for a slope.
//!
//! What the paper leaves open, and how it is set here:
//!
//! - **The ellipse:** from the inset, 0.19a × 0.14a, the long axis 53° from x, centred.
//! - **The wave vector:** periodic sides two periods apart along x, and a magnetic current
//!   ∝ cos(πx/a) on every H̃_z with two electric point currents, so that the modes found are
//!   those of k = (½, 0) 2π/a, the X point: the lowest at ωa/2πc = 0.14526 (the paper's
//!   choice, the smallest ω) and the next at 0.16405, 13 % higher. (A Bloch boundary would
//!   take any k; the lattice here needs only real fields.)
//! - **The frequency:** from the matrix pencil (Y. Hua, T. K. Sarkar, IEEE Trans. Acoust.
//!   Speech Signal Process. 38, 814 (1990)) on three probes over 80 a/c after the pulse, in
//!   place of filter diagonalization; it finds a sum of two sinusoids to 1e-10.
//! - **The reference:** Richardson's extrapolation of the new method at 2Δx from 64 and 96
//!   pixels, which converges smoothly as h², in place of a plane-wave calculation; for the
//!   lowest mode it agrees with the s = 1 method's from 96 and 128 pixels to 8e-6 (relative),
//!   a fifth of the new method's error at 64 pixels.
//!
//! What reproduces, for the lowest mode: the new method's slope at 2Δx (−2.26 against the
//! paper's −2.43), its quadratic convergence at s = 1 (−2.12, "asymptotically quadratic"), and
//! its error below the mean's at every resolution. For the second mode, the mean's first order
//! (−1.40 against −1.33), the new method's erratic convergence at s = 1 and quadratic at 2Δx.
//! What doesn't: for the lowest mode the mean ε converges at −1.84, not −1.33, and the new
//! method's margin over it is a factor of 2 to 3, not 10 to 30; the mean's first-order error
//! is the mode's D across the holes' surfaces, which at this k is weak (the paper doesn't give
//! its k). Werner and Cary's placement of the off-diagonal entries (`Coupling::Nodes`, stable,
//! photonoxide's default) is printed beside them: no better than the mean, as at any oblique
//! interface (docs/methods/fdtd.md).
//!
//! ```sh
//! cargo run --release --example subpixel_holes
//! ```

mod common;

use std::process::ExitCode;

use faer::Mat;
use faer::linalg::solvers::Solve;
use photonoxide::Complex64 as c64;
use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{
    Average, Body, Boundaries, Coupling, Cpml, Current, Field, Permittivity, Simulation, Smoothing,
    Structure, Waveform,
};
use photonoxide::units::Frequency;
use rayon::prelude::*;

/// The resolutions, pixels a period.
const RESOLUTIONS: [usize; 8] = [12, 16, 20, 24, 32, 40, 48, 64];

/// The lattice, two periods (1 µm each) along x and one along y.
fn holes() -> Structure {
    let mut s = Structure::new(Permittivity::isotropic(12.0).expect("ε"));
    for i in 0..2 {
        let centre = [i as f64 + 0.5, 0.5];
        let hole = Body::ellipse(centre, [0.19, 0.14], 53f64.to_radians()).expect("the hole");
        s = s.with(hole, Permittivity::isotropic(1.0).expect("air"));
    }
    s
}

/// The frequencies (c/µm) in real signals sampled every `dt`: the matrix pencil, each signal's
/// Hankel matrix of L + 1 columns stacked, L a third of its length; the right singular vectors
/// V above 1e-10 of the largest; the poles the eigenvalues of V₁⁺V₂, V₁ and V₂ those vectors
/// without their last and first rows.
fn frequencies(signals: &[Vec<f64>], dt: f64) -> Vec<f64> {
    let n = signals[0].len();
    let l = n / 3;
    let rows = n - l;
    let y = Mat::<f64>::from_fn(rows * signals.len(), l + 1, |i, j| {
        signals[i / rows][i % rows + j]
    });
    let svd = y.thin_svd().expect("the SVD of a small dense matrix");
    let s = svd.S().column_vector();
    let m = (0..s.nrows()).filter(|&k| s[k] > 1e-10 * s[0]).count();
    let v = svd.V();
    let v1 = Mat::<f64>::from_fn(l, m, |i, k| v[(i, k)]);
    let v2 = Mat::<f64>::from_fn(l, m, |i, k| v[(i + 1, k)]);
    let z = (v1.transpose() * &v1)
        .partial_piv_lu()
        .solve(v1.transpose() * &v2);
    let poles: Vec<c64> = z.eigenvalues().expect("the eigenvalues of a small matrix");
    poles
        .iter()
        .filter(|z| z.im < 0.0)
        .map(|z| -z.arg() / (std::f64::consts::TAU * dt))
        .collect()
}

/// The two lowest TE modes at the X point on `n` pixels a period, by `smoothing`, c/µm.
fn modes(smoothing: Smoothing, n: usize) -> [f64; 2] {
    let h = 1.0 / n as f64;
    let grid = Grid3d {
        nx: 2 * n,
        ny: n,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let p = Edges::Bloch { k: 0.0 };
    let boundaries = Boundaries {
        x: p,
        y: p,
        z: p,
        cpml: Cpml::default(),
    };
    let mut s =
        Simulation::smoothed(grid, &holes(), smoothing, boundaries, 0.99).expect("the grid");
    let pulse = Waveform::pulse(Frequency::natural(0.155).expect("f"), 0.08).expect("the pulse");
    // a magnetic current ∝ cos(πx) on every H̃_z and two electric point currents, all with
    // k = (½, 0) 2π/µm or folded onto it by the two periods
    let mut values: Vec<(Axis, (usize, usize, usize), c64)> = (0..2 * n * n)
        .map(|r| {
            let (i, j) = (r % (2 * n), r / (2 * n));
            let x = grid.h_position(Axis::Z, (i, j, 0))[0];
            let x = std::f64::consts::PI * x;
            (Axis::Z, (i, j, 0), c64::new(x.cos(), 0.0))
        })
        .collect();
    let at = |x: f64, y: f64| ((x * n as f64) as usize, (y * n as f64) as usize, 0);
    s.add_current(Current {
        field: Field::H,
        values: std::mem::take(&mut values),
        waveform: pulse,
    })
    .expect("the current");
    s.add_current(Current {
        field: Field::E,
        values: vec![
            (Axis::X, at(0.13, 0.71), c64::new(1.0, 0.0)),
            (Axis::Y, at(1.62, 0.27), c64::new(1.0, 0.0)),
        ],
        waveform: pulse,
    })
    .expect("the current");
    let probes = [
        s.add_probe(Field::H, Axis::Z, at(0.31, 0.12))
            .expect("a probe"),
        s.add_probe(Field::H, Axis::Z, at(1.77, 0.43))
            .expect("a probe"),
        s.add_probe(Field::E, Axis::X, at(1.21, 0.89))
            .expect("a probe"),
    ];
    let Waveform::Gaussian { delay, .. } = pulse else {
        unreachable!("a pulse is a Gaussian")
    };
    let start = (2.0 * delay / s.dt()).ceil() as usize;
    s.run_until(2.0 * delay + 80.0);
    // 16 samples a period of the carrier
    let every = (1.0 / 0.155 / 16.0 / s.dt()) as usize;
    let signals: Vec<Vec<f64>> = probes
        .iter()
        .map(|&q| s.probe(q)[start..].iter().step_by(every).copied().collect())
        .collect();
    let found = frequencies(&signals, every as f64 * s.dt());
    let nearest = |target: f64| {
        found
            .iter()
            .copied()
            .min_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()))
            .expect("a mode")
    };
    [nearest(0.1453), nearest(0.1641)]
}

/// The least-squares slope of log error against log resolution, from `from` pixels on.
fn slope(errors: &[f64], from: usize) -> f64 {
    let points: Vec<(f64, f64)> = RESOLUTIONS
        .iter()
        .zip(errors)
        .filter(|(n, _)| **n >= from)
        .map(|(&n, &e)| ((n as f64).ln(), e.ln()))
        .collect();
    let m = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let (sxx, sxy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x * x, b + x * y));
    (m * sxy - sx * sy) / (m * sxx - sx * sx)
}

pub fn main() -> ExitCode {
    println!("Farjadpour et al.'s square lattice of elliptical air holes in eps = 12, TE");
    println!("the two lowest modes at k = (1/2, 0) 2 pi/a: relative error against pixels a period");
    let with = |average, diameter, coupling| Smoothing {
        average,
        diameter,
        coupling,
    };
    let methods = [
        ("no smoothing", with(Average::Sampled, 1.0, Coupling::Nodes)),
        ("mean epsilon", with(Average::Mean, 1.0, Coupling::Nodes)),
        ("new method", with(Average::Subpixel, 1.0, Coupling::Points)),
        (
            "new method (2 dx)",
            with(Average::Subpixel, 2.0, Coupling::Points),
        ),
        (
            "new, at the nodes",
            with(Average::Subpixel, 1.0, Coupling::Nodes),
        ),
    ];
    let mut jobs: Vec<(usize, usize)> = (0..methods.len())
        .flat_map(|m| RESOLUTIONS.iter().map(move |&n| (m, n)))
        .collect();
    jobs.push((3, 96));
    // the slowest first
    jobs.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    let found: Vec<((usize, usize), [f64; 2])> = jobs
        .par_iter()
        .map(|&(m, n)| ((m, n), modes(methods[m].1, n)))
        .collect();
    let get = |m: usize, n: usize| {
        found
            .iter()
            .find(|(key, _)| *key == (m, n))
            .map(|(_, f)| *f)
            .expect("a run")
    };
    let mut checks = common::Checks::default();
    for band in 0..2 {
        let (coarse, fine) = (get(3, 64)[band], get(3, 96)[band]);
        let reference = fine + (fine - coarse) * 64.0 * 64.0 / (96.0 * 96.0 - 64.0 * 64.0);
        println!();
        println!(
            "band {}: {reference:.8} c/um, the new method (2 dx) at 64 and 96 pixels by \
             Richardson ({coarse:.8}, {fine:.8})",
            band + 1
        );
        print!("{:<20}", "pixels a period");
        for n in RESOLUTIONS {
            print!("{n:>9}");
        }
        println!("{:>9}", "slope");
        let mut errors = Vec::new();
        for (m, (name, _)) in methods.iter().enumerate() {
            let e: Vec<f64> = RESOLUTIONS
                .iter()
                .map(|&n| ((get(m, n)[band] - reference) / reference).abs())
                .collect();
            print!("{name:<20}");
            for v in &e {
                print!("{v:>9.1e}");
            }
            println!("{:>9.2}", slope(&e, 16));
            errors.push(e);
        }
        println!("slopes from 16 to 64 pixels; the paper's over 20 to 100");
        let label = |what: &str| format!("band {}: {what}", band + 1);
        // the figure is of the lowest mode: its reading's slope is good to 0.05, but their k
        // and ellipse are not ours, and a slope over a decade moves with its pre-asymptotic
        // terms: 0.3. The second mode's against the quadratic the paper shows it for.
        let two = slope(&errors[3], 16);
        if band == 0 {
            checks.compare(&label("new (2 dx), slope"), two, -2.43, 0.3);
        } else {
            checks.against(&label("new (2 dx), slope"), two, "quadratic", -2.0, 0.3);
        }
        let below = (0..RESOLUTIONS.len())
            .filter(|&q| errors[2][q] < errors[1][q])
            .count();
        if band == 0 {
            // "we expect our method to be asymptotically quadratic"
            let s = slope(&errors[2], 16);
            checks.against(&label("new, slope"), s, "quadratic", -2.0, 0.3);
            checks.count(&label("new below the mean"), below, RESOLUTIONS.len());
            println!(
                "band 1: mean epsilon's slope {:.2}, the paper's -1.33 (printed, not checked)",
                slope(&errors[1], 16)
            );
        } else {
            checks.compare(
                &label("mean epsilon, slope"),
                slope(&errors[1], 16),
                -1.33,
                0.3,
            );
            println!(
                "band 2: the new method below the mean at {below} of {} resolutions, its slope \
                 {:.2} (erratic, as the paper's s = 1; printed, not checked)",
                RESOLUTIONS.len(),
                slope(&errors[2], 16)
            );
        }
    }
    checks.finish()
}
