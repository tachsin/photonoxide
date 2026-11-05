//! Eigenmode expansion in 3D: the tests. The validation report runs the cases at their sizes;
//! these run the same checks small.

use num_complex::Complex64 as c64;

use super::plane::{Stretch, operators};
use super::three_checks::{centred, fresnel, strip};
use super::{Eme3d, Plane, PlaneGrid, Pml3d, Section3d};
use crate::units::{Length, Wavelength};

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

fn small() -> (Plane, Pml3d) {
    let grid = centred(14, 10, 0.1, 0.1);
    (
        strip(grid, 0.5, 0.2),
        Pml3d::new(0.3, 0.3, 0.2, 0.2, 1.5)
            .unwrap()
            .graded(2.0)
            .unwrap(),
    )
}

#[test]
fn the_operators_are_symmetric_in_the_stretches_weights() {
    let (plane, pml) = small();
    symmetric(&plane, Some(&pml));
    // magnetic walls on two sides, a corner between them, a PML on the electric ones
    let walls = super::Walls {
        west: super::Wall::Magnetic,
        east: super::Wall::Electric,
        south: super::Wall::Magnetic,
        north: super::Wall::Electric,
    };
    let grid = centred(14, 10, 0.1, 0.1).with_walls(walls);
    let pml = Pml3d::new(0.0, 0.3, 0.0, 0.2, 1.5).unwrap();
    symmetric(&strip(grid, 0.5, 0.2), Some(&pml));
    // a PML on a magnetic wall is refused
    let bad = Pml3d::new(0.3, 0.0, 0.0, 0.0, 1.0).unwrap();
    assert!(
        Eme3d::new(lam(), 2)
            .unwrap()
            .with_pml(bad)
            .modes(&strip(grid, 0.5, 0.2))
            .is_err()
    );
}

/// The PML study the method page states: run by hand (`--ignored`), one line per setting.
#[test]
#[ignore = "a study for the method page"]
fn pml_study() {
    use super::three_checks::{open_taper, taper_fdfd};
    for pml in [0.5, 1.0, 1.5, 2.0] {
        let t = std::time::Instant::now();
        let r = taper_fdfd(0.025, pml, 1.5);
        println!(
            "fdfd pml {pml} lead 1.5 dz 0.025: {r:?} in {:?}",
            t.elapsed()
        );
    }
    for thickness in [0.25, 0.5] {
        for profile in [(0.5, 0.0), (1.0, 0.0), (3.0, 2.0), (6.0, 2.0)] {
            for modes in [Some(60), Some(150), None] {
                let t = std::time::Instant::now();
                let (r, n) = open_taper(thickness, profile, modes);
                println!(
                    "eme pml {thickness} {profile:?} modes {n}: {r:?} in {:?}",
                    t.elapsed()
                );
            }
        }
    }
}

/// V C and V C A are symmetric for `plane` inside `pml`.
fn symmetric(plane: &Plane, pml: Option<&Pml3d>) {
    let g = plane.grid();
    let stretch = Stretch::new(&g, pml).unwrap();
    let w = stretch.weights(&g);
    let (c, d) = operators(plane, &stretch, lam().wavenumber());
    let entry = |rows: &super::plane::Rows, r: usize, s: usize| {
        rows[r]
            .iter()
            .find(|&&(col, _)| col == s)
            .map_or(c64::new(0.0, 0.0), |&(_, v)| v)
    };
    // V C symmetric; and V C A = V C (C + D) symmetric, A applied through vectors
    let n = g.unknowns();
    let mut worst: f64 = 0.0;
    for r in 0..n {
        for &(s, v) in &c[r] {
            worst = worst.max((w[r] * v - w[s] * entry(&c, s, r)).norm() / (w[r] * v).norm());
        }
    }
    assert!(worst < 1e-13, "V C is not symmetric: {worst:e}");
    let unit = |k: usize| {
        (0..n)
            .map(|i| c64::new(f64::from(u8::from(i == k)), 0.0))
            .collect::<Vec<_>>()
    };
    let apply = super::plane::apply;
    // pairs on the walls (the first and last unknowns), beside them, and inside
    for (a, b) in [
        (0, 1),
        (0, 15),
        (3, 40),
        (17, 151),
        (90, 200),
        (5, n - 1),
        (n - 1, n - 2),
    ] {
        let (ua, ub) = (unit(a), unit(b));
        let a_ub: Vec<c64> = apply(&c, &ub)
            .iter()
            .zip(apply(&d, &ub))
            .map(|(x, y)| x + y)
            .collect();
        let a_ua: Vec<c64> = apply(&c, &ua)
            .iter()
            .zip(apply(&d, &ua))
            .map(|(x, y)| x + y)
            .collect();
        let cua = apply(&c, &ua);
        let cub = apply(&c, &ub);
        let left: c64 = (0..n).map(|i| w[i] * cua[i] * a_ub[i]).sum();
        let right: c64 = (0..n).map(|i| w[i] * cub[i] * a_ua[i]).sum();
        assert!((left - right).norm() <= 1e-12 * left.norm().max(right.norm()).max(1.0));
    }
}

#[test]
fn a_sections_modes_are_orthonormal_with_a_pml() {
    let (plane, pml) = small();
    let modes = Eme3d::new(lam(), 30)
        .unwrap()
        .with_pml(pml)
        .modes(&plane)
        .unwrap();
    for (i, a) in modes.iter().enumerate() {
        for (j, b) in modes.iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            let p = a.product(b).unwrap();
            assert!((p - expected).norm() < 1e-10, "<E{i}, H{j}> = {p}");
        }
        assert!(a.effective_index().im >= 0.0 || a.effective_index().re > 0.0);
    }
    // the guided TE-like mode first
    assert!(modes[0].effective_index().re > 1.444 && modes[0].te_fraction() > 0.5);
}

#[test]
fn a_step_between_uniform_media_is_fresnels_on_every_mode() {
    assert!(fresnel() < 1e-10);
}

/// A short strip taper, on a small plane, with `modes` modes.
fn taper(modes: usize) -> super::Scattering {
    let grid = centred(12, 8, 0.1, 0.1);
    let sections = [
        Section3d::new(strip(grid, 0.4, 0.2), Length::ZERO),
        Section3d::new(strip(grid, 0.6, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.4, 0.2), Length::um(0.2)),
        Section3d::new(strip(grid, 0.6, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.8, 0.2), Length::ZERO),
    ];
    Eme3d::new(lam(), modes)
        .unwrap()
        .s_matrix(&sections)
        .unwrap()
}

#[test]
fn the_same_bits_on_one_thread_and_on_four() {
    let on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| taper(20))
    };
    assert_eq!(on(1), on(4));
}

#[test]
fn reciprocal_for_any_number_of_modes_and_lossless_with_every_mode() {
    let n = centred(12, 8, 0.1, 0.1).unknowns();
    for modes in [10, n] {
        let s = taper(modes);
        for i in 0..modes {
            for j in 0..modes {
                assert!((s.s21[i][j] - s.s12[j][i]).norm() < 1e-12);
                assert!((s.s11[i][j] - s.s11[j][i]).norm() < 1e-12);
            }
        }
    }
    // every mode: the propagating block is unitary
    let s = taper(n);
    let open: Vec<usize> = (0..n)
        .filter(|&i| s.first[i].im.abs() < 1e-12 * s.first[i].re.abs())
        .collect();
    let back: Vec<usize> = (0..n)
        .filter(|&i| s.last[i].im.abs() < 1e-12 * s.last[i].re.abs())
        .collect();
    for &p in &open {
        let power: f64 = open.iter().map(|&q| s.s11[q][p].norm_sqr()).sum::<f64>()
            + back.iter().map(|&q| s.s21[q][p].norm_sqr()).sum::<f64>();
        assert!((power - 1.0).abs() < 1e-11, "mode {p}: {power}");
    }
}

#[test]
fn with_every_mode_the_tangential_fields_are_continuous_at_a_step() {
    // the fundamental from the left onto a step: E_t = e₀ + Σ r_i e_i on the left and Σ t_j e_j
    // on the right, H_t = h₀ − Σ r_i h_i and Σ t_j h_j, every value of the plane
    let grid = centred(10, 8, 0.1, 0.1);
    let (a, b) = (strip(grid, 0.4, 0.2), strip(grid, 0.6, 0.4));
    let n = grid.unknowns();
    let eme = Eme3d::new(lam(), n).unwrap();
    let (ma, mb) = (eme.modes(&a).unwrap(), eme.modes(&b).unwrap());
    let s = eme
        .s_matrix(&[
            Section3d::new(a, Length::ZERO),
            Section3d::new(b, Length::ZERO),
        ])
        .unwrap();
    let field = |i: usize, j: usize, e: bool| {
        let value = |m: &super::Mode3d, x: bool| match (e, x) {
            (true, true) => m.ex(i, j),
            (true, false) => m.ey(i, j),
            (false, true) => m.hy(i, j),
            (false, false) => m.hx(i, j),
        };
        let sign = if e { 1.0 } else { -1.0 };
        [true, false].map(|x| {
            let left: c64 = value(&ma[0], x)
                + (0..n)
                    .map(|k| sign * s.s11[k][0] * value(&ma[k], x))
                    .sum::<c64>();
            let right: c64 = (0..n).map(|k| s.s21[k][0] * value(&mb[k], x)).sum();
            (left - right).norm()
        })
    };
    let mut worst: f64 = 0.0;
    for i in 0..=grid.nx {
        for j in 0..=grid.ny {
            for e in [true, false] {
                worst = worst.max(field(i, j, e).into_iter().fold(0.0, f64::max));
            }
        }
    }
    assert!(worst < 1e-9, "the fields jump by {worst:e}");
}

#[test]
fn a_field_uniform_in_y_separates() {
    let s = |ny: usize| {
        let grid = PlaneGrid::new(60, ny, (0.05, 0.05), (-1.5, 0.0));
        let slab = |t: f64| {
            Plane::new(grid, move |x, _| {
                c64::new(if x.abs() < 0.5 * t { 12.06 } else { 2.085 }, 0.0)
            })
            .unwrap()
        };
        Eme3d::new(lam(), 8)
            .unwrap()
            .s_matrix(&[
                Section3d::new(slab(0.2), Length::ZERO),
                Section3d::new(slab(0.4), Length::um(0.1)),
                Section3d::new(slab(0.3), Length::ZERO),
            ])
            .unwrap()
    };
    let (a, b) = (s(1), s(3));
    for i in 0..8 {
        for j in 0..8 {
            assert!((a.s11[i][j] - b.s11[i][j]).norm() < 1e-10);
            assert!((a.s21[i][j] - b.s21[i][j]).norm() < 1e-10);
        }
    }
}

#[test]
fn the_z_limit_fit_is_exact_on_its_own_form() {
    let dz = [0.05, 0.04, 0.03, 0.02];
    let v: Vec<f64> = dz
        .iter()
        .map(|h: &f64| 0.7 - 3.0 * h * h + 40.0 * h.powi(4))
        .collect();
    let (limit, residual) = super::three_checks::z_limit(&dz, &v);
    assert!((limit - 0.7).abs() < 1e-12 && residual < 1e-12);
}

#[test]
fn refusals() {
    let grid = centred(4, 4, 0.1, 0.1);
    let plane = strip(grid, 0.2, 0.2);
    // more modes than unknowns
    assert!(Eme3d::new(lam(), 25).unwrap().modes(&plane).is_err());
    assert!(Eme3d::new(lam(), 0).is_err());
    // a window of one cell, a grid without steps
    let one = PlaneGrid {
        nx: 1,
        ny: 1,
        ..grid
    };
    assert!(Plane::new(one, |_, _| c64::new(1.0, 0.0)).is_err());
    let flat = PlaneGrid { dx: 0.0, ..grid };
    assert!(Plane::new(flat, |_, _| c64::new(1.0, 0.0)).is_err());
    // sections on different grids, a PML thicker than the window, bad PMLs
    let other = strip(centred(4, 6, 0.1, 0.1), 0.2, 0.2);
    let eme = Eme3d::new(lam(), 4).unwrap();
    assert!(
        eme.s_matrix(&[
            Section3d::new(plane.clone(), Length::ZERO),
            Section3d::new(other, Length::ZERO)
        ])
        .is_err()
    );
    let thick = Pml3d::new(0.3, 0.3, 0.0, 0.0, 1.0).unwrap();
    assert!(eme.with_pml(thick).modes(&plane).is_err());
    assert!(Pml3d::new(0.0, 0.0, 0.0, 0.0, 1.0).is_err());
    assert!(Pml3d::new(0.1, 0.0, 0.0, 0.0, 0.0).is_err());
    assert!(
        Pml3d::new(0.1, 0.0, 0.0, 0.0, 1.0)
            .unwrap()
            .graded(-1.0)
            .is_err()
    );
    assert!(eme.s_matrix(&[]).is_err());
}
