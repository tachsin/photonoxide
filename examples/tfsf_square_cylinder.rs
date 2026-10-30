//! A plane wave on a square conducting cylinder, by total-field/scattered-field: the current on
//! its surface.
//!
//! K. Umashankar, A. Taflove, "A novel method to analyze electromagnetic scattering of complex
//! objects", IEEE Trans. Electromagn. Compat. EMC-24, 397 (1982),
//! [doi:10.1109/TEMC.1982.304054](https://doi.org/10.1109/TEMC.1982.304054), Section III.A and
//! Fig. 5(a):
//!
//! - **The problem:** a perfectly conducting square cylinder, k₀A_s = 1 (A_s half its side), lit
//!   by a TM plane wave (E_z) travelling along +y, at normal incidence to one face (φⁱ = 90°).
//! - **Their FD-TD:** 20 cells a side, the total-field/scattered-field surface 5 cells from the
//!   cylinder; the surface current is n̂ × H_tan, H half a cell from the surface.
//! - **Their reference:** the method of moments on their Eq. 8a, the electric field integral
//!   equation E_zⁱ = (k₀η₀/4) ∫ J_z H₀⁽²⁾(k₀|ρ − ρ'|) dl' (e^(+jωt)), with pulses and point
//!   matching at 80 points, the solid line of Fig. 5(a): |J_z|/|H_xⁱ| around the surface.
//!   Their FD-TD agrees with it "to better than ±1 percent ... at all comparison points more
//!   than 2 cells from the cylinder edges".
//!
//! Read off the figure (at 600 dpi, 365 pixels to a unit, the axis's ticks consistent to a
//! pixel), the curve is 1.750 at the middle of the lit face (a), 0.785 at the middle of the
//! side faces (between b and c) and 0.207 at the middle of the shadowed face (d).
//!
//! Here:
//!
//! - **FD-TD** as theirs: a continuous wave at λ = 1 µm (A_s = λ/2π; a conductor in vacuum
//!   scales with λ) switched on over 5 periods, its amplitude over the 30th to the 32nd, 100
//!   steps a period; beyond the box 10 cells of scattered field and a CPML of 20 cells (κ = 5,
//!   α = 0.5/µm, for the evanescent near field: moving it 70 cells further out changes nothing
//!   in the sixth digit). At 20 cells a side, then at 40 to see the grid's error.
//! - **The moment method** as theirs, Eq. 8a with e^(−iωt) and H₀⁽¹⁾, by pulses and point
//!   matching at 100 points a side (it moves by 1e-4 from 50 to 200), the self term
//!   integrated in closed form and the others by 5-point Gauss–Legendre.
//!
//! The FD-TD at 20 cells a side agrees with the figure within the reading (0.01) plus 2 % on
//! the lit and side faces, and the FD-TD at 40 cells with the moment method within the paper's
//! ±1 % at all three points. On the shadowed face both give 0.17, 21 % below the figure's 0.207:
//! the figure's moment-method curve there is above the converged solution of the very equation
//! it solves (at their 80 points the moment method here gives 0.1710), so that point is printed,
//! not checked against the figure.
//!
//! ```sh
//! cargo run --release --example tfsf_square_cylinder
//! ```

mod common;

use std::f64::consts::{EULER_GAMMA, PI, TAU};
use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::fdfd::{Axis, Grid3d};
use photonoxide::fdtd::{Boundaries, Cpml, Field, PlaneWave, Simulation, Waveform};
use photonoxide::units::Frequency;

/// The wavelength, µm.
const WAVELENGTH: f64 = 1.0;
/// At 20 cells a side, in cells: the total-field region's gap around the cylinder, the margin
/// beyond it, the CPML; and the steps a period. They scale with the cells a side.
const LAYOUT: (usize, usize, usize) = (5, 10, 20);
const STEPS_PER_PERIOD: usize = 100;
/// The continuous wave's ramp, periods.
const RAMP: f64 = 5.0;
/// The CPML's κ and α.
const KAPPA: f64 = 5.0;
const ALPHA: f64 = 0.5;
/// The periods run before the amplitudes are read, and the periods they are read over.
const PERIODS: (usize, usize) = (29, 3);
/// The FD-TD's cells a side, and the moment method's pulses a side: the reference, and theirs.
const SIDES: [usize; 2] = [20, 40];
const PULSES: [usize; 2] = [100, 20];

/// FD-TD's |J_z|/|H_xⁱ| at the middle of the lit face, a side face and the shadowed face, with
/// `side` cells along the cylinder's side.
fn fdtd(side: usize) -> [f64; 3] {
    let half = WAVELENGTH / TAU;
    let h = 2.0 * half / side as f64;
    let scale = side / 20;
    let (gap, margin, cpml) = (LAYOUT.0 * scale, LAYOUT.1 * scale, LAYOUT.2 * scale);
    let edge = cpml + margin + gap;
    let n = side + 2 * edge;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let mut boundaries = Boundaries::cpml_2d(cpml);
    boundaries.cpml = Cpml {
        kappa: KAPPA,
        alpha: ALPHA,
        ..Cpml::default()
    };
    let steps_per_period = STEPS_PER_PERIOD * scale;
    let dt = WAVELENGTH / steps_per_period as f64;
    let courant = dt * 2f64.sqrt() / h;
    let mut s = Simulation::new(grid, |_, _, _| 1.0, boundaries, courant).expect("the grid");
    // the cylinder: E_z held at zero on its nodes, the faces included
    let first = edge;
    for j in first..=first + side {
        for i in first..=first + side {
            s.conductor(Axis::Z, (i, j, 0));
        }
    }
    let frequency = Frequency::natural(1.0 / WAVELENGTH).expect("the frequency");
    let wave = s
        .add_plane_wave(PlaneWave {
            low: (first - gap, first - gap, 0),
            high: (first + side + gap, first + side + gap, 1),
            direction: (0, 1, 0),
            polarization: [0.0, 0.0, 1.0],
            eps: 1.0,
            waveform: Waveform::Continuous {
                frequency,
                amplitude: c64::new(1.0, 0.0),
                ramp: RAMP * WAVELENGTH,
            },
        })
        .expect("the plane wave");
    // H half a cell outside each face's middle: below the lit face, beside the right one,
    // above the shadowed one
    let middle = first + side / 2;
    let points = [
        (Axis::X, (middle, first - 1)),
        (Axis::Y, (first + side, middle)),
        (Axis::X, (middle, first + side)),
    ];
    s.run(PERIODS.0 * steps_per_period);
    let omega = frequency.angular();
    let mut amplitude = [c64::new(0.0, 0.0); 3];
    let mut incident = c64::new(0.0, 0.0);
    for _ in 0..PERIODS.1 * steps_per_period {
        s.step();
        // H̃ is half a step behind E
        let w = c64::new(0.0, omega * (s.time() - 0.5 * s.dt())).exp();
        for (a, &(component, (i, j))) in amplitude.iter_mut().zip(&points) {
            *a += s.h(component)[j * n + i] * w;
        }
        incident += s.incident(wave, Field::H, Axis::X, (middle, first - gap + 1, 0)) * w;
    }
    amplitude.map(|a| a.norm() / incident.norm())
}

/// H₀⁽¹⁾(x) = J₀(x) + iY₀(x) for 0 < x ≤ 3, by their power series.
fn hankel0(x: f64) -> c64 {
    let q = x * x / 4.0;
    let (mut j, mut tail) = (0.0, 0.0);
    // (x/2)^(2k)/(k!)², and the harmonic number H_k
    let (mut term, mut harmonic) = (1.0, 0.0);
    for k in 0..40 {
        if k > 0 {
            term *= q / (k * k) as f64;
            harmonic += 1.0 / k as f64;
        }
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        j += sign * term;
        tail -= sign * harmonic * term;
    }
    let y = 2.0 / PI * (((x / 2.0).ln() + EULER_GAMMA) * j + tail);
    c64::new(j, y)
}

/// The moment method's |J_z|/|H_xⁱ| at the middle of the lit, side and shadowed faces, with
/// `per` pulses along each side: (k₀/4) ∫ J̃_z H₀⁽¹⁾(k₀|ρ − ρ'|) dl' = E_zⁱ at each pulse's
/// middle, J̃ = η₀J in the units of H̃ = η₀H.
fn moments(per: usize) -> [f64; 3] {
    let k = TAU / WAVELENGTH;
    let a = 1.0 / k;
    let n = 4 * per;
    let d = 2.0 * a / per as f64;
    // the pulses counterclockwise from the lit face's left end: lit, right, shadowed, left
    let mut middle = Vec::with_capacity(n);
    let mut along = Vec::with_capacity(n);
    for side in 0..4 {
        for m in 0..per {
            let s = -a + (m as f64 + 0.5) * d;
            let (p, t) = match side {
                0 => ([s, -a], [1.0, 0.0]),
                1 => ([a, s], [0.0, 1.0]),
                2 => ([-s, a], [-1.0, 0.0]),
                _ => ([-a, -s], [0.0, -1.0]),
            };
            middle.push(p);
            along.push(t);
        }
    }
    let gauss = [
        (-0.906_179_845_938_664, 0.236_926_885_056_189),
        (-0.538_469_310_105_683, 0.478_628_670_499_366),
        (0.0, 0.568_888_888_888_889),
        (0.538_469_310_105_683, 0.478_628_670_499_366),
        (0.906_179_845_938_664, 0.236_926_885_056_189),
    ];
    let mut m = vec![c64::new(0.0, 0.0); n * n];
    for r in 0..n {
        for c in 0..n {
            let integral = if r == c {
                // ∫ H₀⁽¹⁾(k|x|) dx over the pulse, from H₀⁽¹⁾(z) ≈ 1 + (2i/π)(ln(z/2) + γ)
                c64::new(d, 2.0 / PI * d * ((k * d / 4.0).ln() - 1.0 + EULER_GAMMA))
            } else {
                gauss
                    .iter()
                    .map(|&(x, w)| {
                        let p = [
                            middle[c][0] + 0.5 * d * x * along[c][0],
                            middle[c][1] + 0.5 * d * x * along[c][1],
                        ];
                        let distance =
                            ((middle[r][0] - p[0]).powi(2) + (middle[r][1] - p[1]).powi(2)).sqrt();
                        hankel0(k * distance) * (0.5 * d * w)
                    })
                    .sum()
            };
            m[r * n + c] = integral * (k / 4.0);
        }
    }
    let mut b: Vec<c64> = middle
        .iter()
        .map(|p| c64::new(0.0, k * p[1]).exp())
        .collect();
    // Gaussian elimination with partial pivoting
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&p, &q| m[p * n + col].norm().total_cmp(&m[q * n + col].norm()))
            .expect("a row");
        if pivot != col {
            for c in 0..n {
                m.swap(col * n + c, pivot * n + c);
            }
            b.swap(col, pivot);
        }
        let diagonal = m[col * n + col];
        for r in col + 1..n {
            let f = m[r * n + col] / diagonal;
            for c in col..n {
                let v = m[col * n + c];
                m[r * n + c] -= f * v;
            }
            let v = b[col];
            b[r] -= f * v;
        }
    }
    let mut current = vec![c64::new(0.0, 0.0); n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for c in r + 1..n {
            s -= m[r * n + c] * current[c];
        }
        current[r] = s / m[r * n + r];
    }
    // the middle of each face: the mean of its two middle pulses
    let face = |side: usize| {
        (current[side * per + per / 2 - 1].norm() + current[side * per + per / 2].norm()) / 2.0
    };
    [face(0), face(1), face(2)]
}

pub fn main() -> ExitCode {
    // FD-TD at 20 cells against the figure: the reading, 0.01, plus 2 %; at 40 cells against
    // the moment method: their ±1 %
    let figure = [(1.750, 0.045), (0.785, 0.026)];
    let one_percent = [0.017, 0.0077, 0.0017];
    let mut setup = common::Setup::new();
    let side = 2.0 * WAVELENGTH / TAU;
    setup.item("medium", "vacuum, ε 1");
    setup.shape(
        "the plane",
        &format!(
            "a square perfectly conducting cylinder, {} a side (k₀A = 1), centred",
            common::length(side)
        ),
    );
    setup.item(
        "source",
        &format!(
            "a TM plane wave (E_z) along +y by total-field/scattered-field, continuous at \
             {WAVELENGTH} µm, ramped over {RAMP} periods"
        ),
    );
    setup.item(
        "window",
        &format!(
            "at 20 cells a side: the total field {} cells around the cylinder, {} cells beyond, \
             a CPML of {} cells (κ {KAPPA}, α {ALPHA}); all scale with the cells",
            LAYOUT.0, LAYOUT.1, LAYOUT.2
        ),
    );
    setup.item(
        "grid",
        &format!(
            "{} cells a side, {} steps a period at 20 (scaled), {} periods, then the amplitudes \
             over {}",
            SIDES.map(|n| n.to_string()).join(" and "),
            STEPS_PER_PERIOD,
            PERIODS.0,
            PERIODS.1
        ),
    );
    setup.item(
        "solver",
        &format!(
            "2D FDTD, TM; the moment method (Eq. 8a, pulses and point matching) at {} pulses a \
             side",
            PULSES.map(|n| n.to_string()).join(" and ")
        ),
    );
    setup.item(
        "tolerance",
        &format!(
            "at 20 cells, {} on the lit and {} on the side face (the reading plus 2 %); at 40, \
             {} on the lit, side and shadowed faces against the moment method (1 %)",
            figure[0].1,
            figure[1].1,
            one_percent.map(common::number).join(", ")
        ),
    );
    setup.print();
    println!("Umashankar and Taflove's square conducting cylinder, k0 A = 1, TM, normal incidence");
    println!("|J_z| / |H_x^i| at the middle of the lit, side and shadowed faces");
    let coarse = fdtd(SIDES[0]);
    let fine = fdtd(SIDES[1]);
    let reference = moments(PULSES[0]);
    let theirs = moments(PULSES[1]);
    let show = |what: &str, v: [f64; 3]| {
        println!(
            "{what:<46} lit {:.4}, side {:.4}, shadowed {:.4}",
            v[0], v[1], v[2]
        );
    };
    show("FD-TD, 20 cells a side (62.8 a wavelength):", coarse);
    show("FD-TD, 40 cells a side:", fine);
    show("moment method, 100 points a side:", reference);
    show("moment method, 20 points a side (their 80):", theirs);
    println!(
        "the figure's curve on the shadowed face: 0.207, {:.0} % above the moment method's",
        100.0 * (0.207 / reference[2] - 1.0)
    );
    let mut checks = common::Checks::default();
    // the reading, 0.01, plus 2 %
    checks.compare(
        "FD-TD 20: middle of the lit face",
        coarse[0],
        figure[0].0,
        figure[0].1,
    );
    checks.compare(
        "FD-TD 20: middle of a side face",
        coarse[1],
        figure[1].0,
        figure[1].1,
    );
    // their ±1 %
    for (what, got, value, tolerance) in [
        ("FD-TD 40: lit face", fine[0], reference[0], one_percent[0]),
        ("FD-TD 40: side face", fine[1], reference[1], one_percent[1]),
        (
            "FD-TD 40: shadowed face",
            fine[2],
            reference[2],
            one_percent[2],
        ),
    ] {
        checks.against(what, got, "Eq. 8a", value, tolerance);
    }
    checks.finish()
}
