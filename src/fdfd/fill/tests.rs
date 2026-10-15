use num_complex::Complex64 as c64;

use super::*;
use crate::geometry::{Point, Shape};
use crate::units::Length;

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

fn disk_plane() -> Plane {
    let mut plane = Plane::new(re(2.085));
    let disk = Shape::circle(Point::um(0.013, -0.021), Length::um(0.31)).unwrap();
    plane.draw(disk, re(12.1));
    plane
}

fn grid() -> Grid {
    Grid {
        nx: 40,
        ny: 36,
        dx: 0.03,
        dy: 0.03,
        x0: -0.6,
        y0: -0.54,
    }
}

#[test]
fn samples_from_a_plane_are_the_permittivity_function_to_the_bit() {
    let plane = disk_plane();
    let lam = Wavelength::um(1.55).unwrap();
    let g = grid();
    let mut source = vec![c64::new(0.0, 0.0); g.nx * g.ny];
    source[5 * g.nx + 7] = c64::new(1.0, 0.0);
    for pol in [Polarization::Ez, Polarization::Hz] {
        let a = Solver2d::from_plane(g, pol, lam, &plane, Boundaries::pml(8), Averaging::Samples)
            .unwrap();
        let b = Solver2d::new(g, pol, lam, |x, y| plane.at(x, y), Boundaries::pml(8)).unwrap();
        assert_eq!(a.solve(&source).unwrap(), b.solve(&source).unwrap());
        assert_eq!(a.averaging(), Averaging::Samples);
    }
}

#[test]
fn the_fills_values_are_the_same_on_any_number_of_threads() {
    let plane = disk_plane();
    let g = grid();
    let on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| plane_values(g, Polarization::Hz, &plane, Averaging::Tensor))
    };
    assert_eq!(on(1), on(7));
}

/// The operator on a uniform anisotropic medium, on Bloch-periodic sides, applied to its own
/// Bloch wave e^{i(kx x + ky y)}: the scheme's symbol, which is k₀² − η_yy kx² − η_xx ky² +
/// 2 η_xy kx ky to second order in the step.
#[test]
fn the_mixed_terms_are_the_cross_derivative_on_a_plane_wave() {
    let (nx, ny, h) = (24usize, 20usize, 0.01);
    let tau = 2.0 * std::f64::consts::PI;
    let (kx, ky) = (tau / (nx as f64 * h), -tau / (ny as f64 * h));
    let g = Grid {
        nx,
        ny,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
    };
    let b = Boundaries {
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: ky },
        reflection: 1e-8,
        order: 3.0,
    };
    let (xx, yy, xy) = (0.3, 0.2, 0.07);
    let k0 = 1.0;
    let eps_y = vec![re(1.0 / yy); (nx + 1) * ny];
    let eps_x = vec![re(1.0 / xx); nx * (ny + 1)];
    let eta = (vec![re(xy); nx * (ny + 1)], vec![re(xy); (nx + 1) * ny]);
    let entries = super::super::assemble(g, Polarization::Hz, k0, &[], &eps_y, &eps_x, &b);
    let entries = with_mixed(entries, g, k0, &eta, &b);
    let u: Vec<c64> = (0..nx * ny)
        .map(|r| c64::from_polar(1.0, kx * g.x(r % nx) + ky * g.y(r / nx)))
        .collect();
    let mut au = vec![c64::new(0.0, 0.0); nx * ny];
    for t in &entries {
        au[t.row] += t.val * u[t.col];
    }
    // the scheme's own symbol: the centred differences' and the four-point means'
    let sx = 2.0 * (kx * h / 2.0).sin() / h;
    let sy = 2.0 * (ky * h / 2.0).sin() / h;
    let mixed = 2.0 * xy * (kx * h).sin() / h * (ky * h).sin() / h;
    let symbol = k0 * k0 - yy * sx * sx - xx * sy * sy + mixed;
    for r in 0..nx * ny {
        let ratio = au[r] / u[r];
        assert!(
            (ratio - symbol).norm() < 1e-9 * symbol.abs().max(1.0),
            "{r} {ratio} {symbol}"
        );
    }
    // and the continuous one to second order
    let exact = k0 * k0 - yy * kx * kx - xx * ky * ky + 2.0 * xy * kx * ky;
    assert!(
        (symbol - exact).abs() < 0.02 * exact.abs(),
        "{symbol} {exact}"
    );
}

/// Where η_xy is the same everywhere, each placement is the other's transpose, so the operator is
/// Hermitian with Bloch sides and symmetric with walls; across a disk's edge it is neither, and
/// the problem is factorized by LU.
#[test]
fn a_uniform_tensors_mixed_terms_are_symmetric_and_a_disks_are_not() {
    let g = grid();
    let (nx, ny) = (g.nx, g.ny);
    for bloch in [true, false] {
        let (x, y) = if bloch {
            (Edges::Bloch { k: 1.3 }, Edges::Bloch { k: -0.7 })
        } else {
            let wall = Edges::Pml { low: 0, high: 0 };
            (wall, wall)
        };
        let b = Boundaries {
            x,
            y,
            reflection: 1e-8,
            order: 3.0,
        };
        let eps_y = vec![re(4.0); (nx + 1) * ny];
        let eps_x = vec![re(5.0); nx * (ny + 1)];
        let eta = (vec![re(0.03); nx * (ny + 1)], vec![re(0.03); (nx + 1) * ny]);
        let entries = super::super::assemble(g, Polarization::Hz, 1.0, &[], &eps_y, &eps_x, &b);
        let entries = with_mixed(entries, g, 1.0, &eta, &b);
        let mut m = std::collections::BTreeMap::new();
        for t in &entries {
            m.insert((t.row, t.col), t.val);
        }
        for (&(r, c), &v) in &m {
            let w = m.get(&(c, r)).copied().unwrap_or_default();
            let mirror = if bloch { w.conj() } else { w };
            assert!((v - mirror).norm() < 1e-9 * v.norm(), "{r} {c} {v} {w}");
        }
    }
    let lam = Wavelength::um(1.55).unwrap();
    let s = Solver2d::from_plane_on(
        g,
        Polarization::Hz,
        lam,
        &disk_plane(),
        Boundaries::pml(8),
        Averaging::Tensor,
        &Choice::Photonoxide,
    )
    .unwrap();
    assert!(!s.lu.symmetric());
    let d = Solver2d::from_plane_on(
        g,
        Polarization::Hz,
        lam,
        &disk_plane(),
        Boundaries::pml(8),
        Averaging::Diagonal,
        &Choice::Photonoxide,
    )
    .unwrap();
    assert!(d.lu.symmetric());
}

/// A straight strip drawn in pieces, abutting, at angles, three and four of them, reflects no
/// more than the strip drawn whole on the same 100 nm grid, and its S-matrix is symmetric, to
/// rounding, by every averaging (issue #376: the diagonal's and the tensor's |S₁₁|² was 4.5e-4
/// with two pieces, H along z, the seam taken for an interface).
#[test]
fn a_strip_in_pieces_reflects_no_more_than_the_strip_whole() {
    use super::super::fill_checks::strip_s;
    let (whole, cuts) = crate::averaging::checks::strip_pieces();
    for polarization in [Polarization::Ez, Polarization::Hz] {
        for averaging in Averaging::ALL {
            let (r0, a0) = strip_s(std::slice::from_ref(&whole), polarization, averaging);
            assert!(r0 < 1e-28 && a0 < 1e-13, "{polarization:?} {averaging:?}");
            for pieces in &cuts {
                let (r, a) = strip_s(pieces, polarization, averaging);
                if averaging.reads_fill() {
                    assert!(
                        r <= r0 + 1e-28 && a < 1e-13,
                        "{polarization:?} {averaging:?}, {} pieces: |S11|² {r:e} (whole \
                         {r0:e}), |S21 - S12| {a:e}",
                        pieces.len()
                    );
                } else {
                    // points and samples see the same permittivity at every point
                    assert_eq!((r, a), (r0, a0), "{polarization:?} {averaging:?}");
                }
            }
        }
    }
}
