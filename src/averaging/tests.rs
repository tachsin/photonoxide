use std::f64::consts::PI;

use num_complex::Complex64 as c64;

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
fn a_disks_averaged_permittivity_summed_over_cells_is_its_area_weighted_permittivity() {
    let (eps_disk, eps_bg, r) = (12.0, 1.0, 0.437);
    let disk = Shape::circle(Point::um(0.0123, -0.0311), Length::um(r)).unwrap();
    let mut plane = Plane::new(re(eps_bg));
    plane.draw(disk, re(eps_disk));
    let h = 0.04;
    let mut excess = 0.0;
    for cell in cells(30, 30, h, (-0.6, -0.6)) {
        excess += (plane.shares(cell).mean().re - eps_bg) * h * h;
    }
    let exact = (eps_disk - eps_bg) * PI * r * r;
    assert!(
        (excess - exact).abs() < 1e-12 * exact,
        "{excess} {exact} ({:e})",
        excess / exact - 1.0
    );
}

#[test]
fn overlapping_shapes_of_one_material_fill_their_union() {
    // two disks of radius 0.3 um 0.4 um apart: their union is 2πr² less the lens
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
}

#[test]
fn a_shape_drawn_over_another_hides_what_it_covers() {
    // a disk of 9 over a block of 4 on a background of 1
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
