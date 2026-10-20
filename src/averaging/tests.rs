use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::checks::strip_pieces;
use super::*;
use crate::geometry::{Polygon, Shape};
use crate::units::Length;

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

/// The cells of a grid of `n` × `m` cells of `h` µm from (x0, y0), with their centres.
fn cells(n: usize, m: usize, h: f64, (x0, y0): (f64, f64)) -> Vec<Bounds> {
    let mut out = Vec::new();
    for j in 0..m {
        for i in 0..n {
            let (x, y) = (x0 + i as f64 * h, y0 + j as f64 * h);
            out.push(Bounds::new(Point::um(x, y), Point::um(x + h, y + h)));
        }
    }
    out
}

#[test]
fn names_round_trip_and_others_are_refused() {
    for a in Averaging::ALL {
        assert_eq!(Averaging::parse(a.name()).unwrap(), a);
    }
    assert_eq!(Averaging::default(), Averaging::Samples);
    let e = Averaging::parse("staircase").unwrap_err().to_string();
    assert!(e.contains("staircase"), "{e}");
    assert!(!Averaging::Samples.reads_fill() && !Averaging::Point.reads_fill());
    assert!(Averaging::Mean.reads_fill() && Averaging::Tensor.reads_fill());
}

#[test]
fn a_slab_off_the_grid_lines_gives_the_exact_layer_integrals() {
    // a film 0.2237 um thick from y = 0.0413, on a 0.05 um grid: neither edge on a grid line
    let (eps_film, eps_bg) = (12.0, 2.085);
    let (a, t) = (0.0413, 0.2237);
    let film = Shape::rect(Point::um(0.0, a + t / 2.0), Length::um(10.0), Length::um(t)).unwrap();
    let mut plane = Plane::new(re(eps_bg));
    plane.draw(film, re(eps_film));
    let h = 0.05;
    let column = cells(1, 12, h, (-0.025, -0.2));
    // ∫ε dy and ∫ dy/ε over the column, exactly
    let span = 12.0 * h;
    let integral = eps_film * t + eps_bg * (span - t);
    let inverse = t / eps_film + (span - t) / eps_bg;
    let (mut sum, mut sum_inverse, mut sum_x) = (0.0, 0.0, 0.0);
    for cell in column {
        let s = plane.shares(cell);
        sum += s.mean().re * h;
        // E_y crosses the film: the harmonic mean; E_x lies along it: the mean
        sum_inverse += h / s.component(1, Averaging::Diagonal).re;
        sum_x += s.component(0, Averaging::Diagonal).re * h;
        if !s.is_uniform() {
            let n = s.normal.unwrap();
            assert!(
                n[0].abs() < 1e-15 && (n[1].abs() - 1.0).abs() < 1e-15,
                "{n:?}"
            );
        }
    }
    assert!(
        (sum - integral).abs() < 1e-14 * integral,
        "{sum} {integral}"
    );
    assert!(
        (sum_x - integral).abs() < 1e-14 * integral,
        "{sum_x} {integral}"
    );
    assert!(
        (sum_inverse - inverse).abs() < 1e-14 * inverse,
        "{sum_inverse} {inverse}"
    );
}

#[test]
fn an_interface_on_the_grid_lines_gives_what_point_sampling_does_to_the_bit() {
    // a 0.4 x 0.3 um block whose edges are the grid's lines
    let block = Shape::rect(Point::um(0.2, 0.15), Length::um(0.4), Length::um(0.3)).unwrap();
    let mut plane = Plane::new(re(2.085));
    plane.draw(block, re(12.1));
    let h = 0.05;
    for cell in cells(16, 16, h, (-0.2, -0.2)) {
        let s = plane.shares(cell);
        assert!(s.is_uniform(), "{cell:?}");
        let c = centre_of(cell);
        let point = plane.at(c.x.to_um(), c.y.to_um());
        for along in 0..3 {
            for a in [Averaging::Mean, Averaging::Diagonal, Averaging::Tensor] {
                assert_eq!(s.component(along, a), point);
            }
        }
    }
}

#[test]
fn drawn_shapes_fill_their_union_and_the_last_drawn_hides_what_it_covers() {
    // overlapping shapes of one material fill their union: two disks of radius 0.3 um 0.4 um
    // apart, 2πr² less the lens
    let r: f64 = 0.3;
    let d: f64 = 0.4;
    let a = Shape::circle(Point::um(-d / 2.0, 0.0), Length::um(r)).unwrap();
    let b = Shape::circle(Point::um(d / 2.0, 0.0), Length::um(r)).unwrap();
    let mut plane = Plane::new(re(1.0));
    plane.draw(a, re(4.0)).draw(b, re(4.0));
    let lens = 2.0 * r * r * (d / (2.0 * r)).acos() - d / 2.0 * (4.0 * r * r - d * d).sqrt();
    let union = 2.0 * PI * r * r - lens;
    let h = 0.03;
    let mut area = 0.0;
    for cell in cells(40, 30, h, (-0.6, -0.45)) {
        area += (plane.shares(cell).mean().re - 1.0) / 3.0 * h * h;
    }
    assert!((area - union).abs() < 1e-8 * union, "{area} {union}");
    // a shape drawn over another hides what it covers: a disk of 9 over a block of 4 on a
    // background of 1
    let block = Shape::rect(Point::um(0.0, 0.0), Length::um(0.8), Length::um(0.5)).unwrap();
    let disk = Shape::circle(Point::um(0.4, 0.0), Length::um(0.2)).unwrap();
    let mut plane = Plane::new(re(1.0));
    plane.draw(block, re(4.0)).draw(disk, re(9.0));
    let h = 0.03;
    let (n, m) = (40, 30);
    let mut total = 0.0;
    for cell in cells(n, m, h, (-0.6, -0.45)) {
        total += plane.shares(cell).mean().re * h * h;
    }
    // the half of the disk over the block hides 4 there
    let (a_block, a_disk) = (0.8 * 0.5, PI * 0.04);
    let exact = (n * m) as f64 * h * h + 3.0 * (a_block - a_disk / 2.0) + 8.0 * a_disk;
    assert!((total - exact).abs() < 1e-8 * exact, "{total} {exact}");
}

#[test]
fn the_tensor_is_the_harmonic_mean_across_an_oblique_interface_and_the_mean_along_it() {
    // a half-plane through the cell at 30°, as a large triangle-free polygon
    let t = 30f64.to_radians();
    let (nx, ny) = (t.cos(), t.sin());
    // the line nx x + ny y = 0.01; the region below it
    let (tx, ty) = (-ny, nx);
    let o = [0.01 * nx, 0.01 * ny];
    let far = 10.0;
    let corner =
        |s: f64, back: f64| Point::um(o[0] + s * tx - back * nx, o[1] + s * ty - back * ny);
    let half = Polygon::new(vec![
        corner(-far, 0.0),
        corner(-far, far),
        corner(far, far),
        corner(far, 0.0),
    ])
    .unwrap();
    let mut plane = Plane::new(re(1.0));
    plane.draw(Shape::Polygon(half), re(12.0));
    let s = plane.shares(Bounds::new(Point::um(-0.05, -0.05), Point::um(0.05, 0.05)));
    assert_eq!(s.parts.len(), 2);
    let n = s.normal.unwrap();
    assert!(
        (n[0] - nx).abs() < 1e-12 && (n[1] - ny).abs() < 1e-12,
        "{n:?}"
    );
    let [xx, yy, xy] = s.inverse_tensor();
    let (a, b) = (1.0 / s.harmonic(), 1.0 / s.mean());
    // ε̃⁻¹ n = ⟨ε⁻¹⟩ n, ε̃⁻¹ t = ⟨ε⟩⁻¹ t
    let along_n = [xx * nx + xy * ny, xy * nx + yy * ny];
    let along_t = [xx * tx + xy * ty, xy * tx + yy * ty];
    for (v, want) in [(along_n, [a * nx, a * ny]), (along_t, [b * tx, b * ty])] {
        let off = (v[0] - want[0]).norm().max((v[1] - want[1]).norm());
        assert!(off < 1e-12, "{off:e}");
    }
    // the shares are the triangle the line cuts off: exact
    let f12 = s.parts.iter().find(|p| p.0 == re(12.0)).unwrap().1;
    let mean = 12.0 * f12 + (1.0 - f12);
    assert!((s.mean().re - mean).abs() < 1e-14);
}

#[test]
fn a_cell_without_area_is_its_centres_material() {
    let disk = Shape::circle(Point::um(0.0, 0.0), Length::um(1.0)).unwrap();
    let mut plane = Plane::new(re(1.0));
    plane.draw(disk, re(4.0));
    let s = plane.shares(Bounds::new(Point::um(0.2, 0.0), Point::um(0.2, 0.0)));
    assert_eq!(s, Shares::uniform(re(4.0)));
}

#[test]
fn abutting_shapes_of_one_material_leave_no_seam_in_the_shares_or_the_normals() {
    let (whole, cuts) = strip_pieces();
    let mut one = Plane::new(re(2.085));
    one.draw(whole, re(12.1));
    for pieces in cuts {
        let mut plane = Plane::new(re(2.085));
        for p in pieces {
            plane.draw(p, re(12.1));
        }
        // the cells, and those half a cell over (the faces' cells)
        for offset in [(0.0, 0.0), (0.05, 0.0), (0.0, 0.05)] {
            for cell in cells(22, 12, 0.1, (-1.1 + offset.0, -0.6 + offset.1)) {
                let (a, b) = (one.shares(cell), plane.shares(cell));
                assert_eq!(a.is_uniform(), b.is_uniform(), "{cell:?}: {a:?} {b:?}");
                assert!(
                    (a.mean() - b.mean()).norm() < 1e-13,
                    "{cell:?}: {a:?} {b:?}"
                );
                match (a.normal, b.normal) {
                    (None, None) => {}
                    (Some(n), Some(m)) => assert!(
                        (n[0] - m[0]).abs() < 1e-12 && (n[1] - m[1]).abs() < 1e-12,
                        "{cell:?}: {n:?} {m:?}"
                    ),
                    (n, m) => panic!("{cell:?}: {n:?} {m:?}"),
                }
                for along in 0..2 {
                    for avg in [Averaging::Diagonal, Averaging::Tensor] {
                        let d = (a.component(along, avg) - b.component(along, avg)).norm();
                        assert!(d < 1e-12, "{cell:?} {along} {avg:?}: {d:e}");
                    }
                }
            }
        }
    }
}

#[test]
fn a_seam_between_two_materials_is_an_interface() {
    let rect = |x0: f64, x1: f64| {
        Shape::rect(
            Point::um(0.5 * (x0 + x1), 0.0137),
            Length::um(x1 - x0),
            Length::um(0.5),
        )
        .unwrap()
    };
    let mut plane = Plane::new(re(2.085));
    plane
        .draw(rect(-2.0, 0.037), re(12.1))
        .draw(rect(0.037, 2.0), re(8.0));
    // inside the strip, across the seam: both materials, the seam's normal
    let s = plane.shares(Bounds::new(Point::um(0.0, -0.05), Point::um(0.1, 0.05)));
    assert_eq!(s.parts.len(), 2);
    let f = s.parts.iter().find(|p| p.0 == re(12.1)).unwrap().1;
    assert!((f - 0.37).abs() < 1e-14, "{f}");
    let n = s.normal.unwrap();
    assert!(
        (n[0].abs() - 1.0).abs() < 1e-15 && n[1].abs() < 1e-15,
        "{n:?}"
    );
    // on the seam and the strip's edge: three materials
    let s = plane.shares(Bounds::new(Point::um(0.0, 0.22), Point::um(0.1, 0.32)));
    assert_eq!(s.parts.len(), 3);
}

#[test]
fn a_shape_over_a_seam_sees_the_material_under_it_as_one_region() {
    // a disk of 4 drawn over the strip, whole and in pieces: the same shares, to the booleans'
    // tolerance, and the same averages but where a seam is nearer the cell's centre than the
    // disk's edge (there the normal is the shares' gradient, the mean of the disk's normals
    // over the cell, not the one at the nearest point)
    let (whole, cuts) = strip_pieces();
    let disk = Shape::circle(Point::um(0.05, 0.2), Length::um(0.23)).unwrap();
    let mut one = Plane::new(re(2.085));
    one.draw(whole, re(12.1)).draw(disk.clone(), re(4.0));
    for pieces in cuts {
        let mut plane = Plane::new(re(2.085));
        for p in &pieces {
            plane.draw(p.clone(), re(12.1));
        }
        plane.draw(disk.clone(), re(4.0));
        for cell in cells(22, 12, 0.1, (-1.1, -0.6)) {
            let (a, b) = (one.shares(cell), plane.shares(cell));
            // to the booleans' tolerance: their polygons within 1e-7 of the 8 µm strip, a few
            // 1e-9 of a cell's shares
            for (x, y) in [(a.mean(), b.mean()), (a.harmonic(), b.harmonic())] {
                assert!((x - y).norm() < 1e-6, "{cell:?}: {a:?} {b:?}");
            }
            assert_eq!(a.normal.is_some(), b.normal.is_some(), "{cell:?}");
            let seam = pieces.iter().filter(|p| p.fill(cell) > 0.0).count() > 1;
            if !seam {
                for along in 0..2 {
                    let d = (a.component(along, Averaging::Tensor)
                        - b.component(along, Averaging::Tensor))
                    .norm();
                    assert!(d < 1e-6, "{cell:?} {along}: {d:e} {a:?} {b:?}");
                }
            }
        }
    }
}

#[test]
fn a_piece_drawn_over_a_shape_of_another_material_hides_it() {
    // the strip's left piece, a disk of 4 across the seam, then the right piece over the disk:
    // the right piece's material where it lies, the disk's left of the seam
    let rect = |x0: f64, x1: f64| {
        Shape::rect(
            Point::um(0.5 * (x0 + x1), 0.0137),
            Length::um(x1 - x0),
            Length::um(0.5),
        )
        .unwrap()
    };
    let disk = Shape::circle(Point::um(0.05, 0.0137), Length::um(0.2)).unwrap();
    let mut plane = Plane::new(re(2.085));
    plane
        .draw(rect(-2.0, 0.037), re(12.1))
        .draw(disk, re(4.0))
        .draw(rect(0.037, 2.0), re(12.1));
    let mut bare = Plane::new(re(2.085));
    bare.draw(rect(-2.0, 0.037), re(12.1))
        .draw(rect(0.037, 2.0), re(12.1));
    // the disk, inside the strip, shows only left of the seam, 0.013 µm left of its centre
    let h = 0.01;
    let mut excess = 0.0;
    for cell in cells(60, 60, h, (-0.3, -0.2863)) {
        excess += (plane.shares(cell).mean().re - bare.shares(cell).mean().re) * h * h;
    }
    let (r, a): (f64, f64) = (0.2, 0.05 - 0.037);
    let left = r * r * (a / r).acos() - a * (r * r - a * a).sqrt();
    let exact = (4.0 - 12.1) * left;
    assert!(
        (excess - exact).abs() < 1e-8 * exact.abs(),
        "{excess} {exact}"
    );
    // a cell on the seam inside the disk: the disk's 4 and the right piece's 12.1, its normal
    // the seam's
    let s = plane.shares(Bounds::new(Point::um(0.0, -0.04), Point::um(0.08, 0.04)));
    assert_eq!(s.parts.len(), 2);
    let n = s.normal.unwrap();
    assert!((n[0].abs() - 1.0).abs() < 1e-12, "{n:?}");
}
