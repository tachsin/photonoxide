use std::f64::consts::PI;

use super::*;
use crate::geometry::{Point, Shape};
use crate::stack::{Etch, LayerStack};
use crate::units::Length;

/// The volume of material `m` over boxes of `h` µm tiling [x0, x0 + n h] × [y0, y0 + n h] ×
/// [z0, z0 + m h], from the slices' shares.
fn volume(fills: &Fills, m: usize, n: usize, layers: usize, h: f64, lo: [f64; 3]) -> f64 {
    let mut v = 0.0;
    for k in 0..layers {
        for j in 0..n {
            for i in 0..n {
                let a = [
                    lo[0] + i as f64 * h,
                    lo[1] + j as f64 * h,
                    lo[2] + k as f64 * h,
                ];
                let b = [a[0] + h, a[1] + h, a[2] + h];
                for s in fills.slices(a, b) {
                    if let Slice::Plane { weight, shares } = s {
                        for (key, f) in shares.parts {
                            if key == m {
                                v += weight * f * h * h * h;
                            }
                        }
                    } else {
                        panic!("no sampled slices here");
                    }
                }
            }
        }
    }
    v
}

#[test]
fn a_disk_on_a_layer_fills_its_cylinder_exactly() {
    let mut s = Structure::new(LayerStack::soi_220());
    let r = 0.437;
    s.draw(
        "Si",
        Shape::circle(Point::um(0.013, -0.021), Length::um(r)).unwrap(),
    )
    .unwrap();
    let fills = Fills::new(&s);
    let si = fills
        .materials()
        .iter()
        .position(|m| m.name() == "Si")
        .unwrap();
    // the device layer is material 4 (soi_220: BOX, Si); boxes of 37 nm from 1.95 to 2.29 um,
    // the layer's faces between their planes
    let h = 0.037;
    let v = volume(&fills, 4, 30, 10, h, [-0.555, -0.555, 1.95]);
    let exact = PI * r * r * 0.22;
    assert!((v - exact).abs() < 1e-12 * exact, "{v} {exact}");
    // the substrate's silicon is another material, and holds nothing here
    assert_eq!(si, 0);
}

#[test]
fn a_partial_etch_leaves_its_slab_under_the_shape() {
    let mut s = Structure::new(LayerStack::soi_220());
    let (w, d) = (0.5, 0.07);
    let rib = Shape::rect(Point::um(0.0, 0.0), Length::um(10.0), Length::um(w)).unwrap();
    s.draw_etched("Si", rib, Etch::partial(Length::um(d)))
        .unwrap();
    let fills = Fills::new(&s);
    let (h, n) = (0.023, 40);
    let lo = [-0.46, -0.4567, 1.97];
    let v = volume(&fills, 4, n, 12, h, lo);
    // the slab under the whole window, and the rib's top over its width
    let side = n as f64 * h;
    let exact = side * side * (0.22 - d) + side * w * d;
    assert!((v - exact).abs() < 1e-12 * exact, "{v} {exact}");
}

#[test]
fn a_box_without_height_is_one_slice_and_sloped_walls_are_sampled() {
    let mut s = Structure::new(LayerStack::soi_220());
    let strip = Shape::rect(Point::um(0.0, 0.0), Length::um(10.0), Length::um(0.5)).unwrap();
    s.draw_etched("Si", strip, Etch::FULL.with_sidewall_deg(80.0))
        .unwrap();
    let fills = Fills::new(&s);
    let thin = fills.slices([0.2, -0.3, 2.1], [0.3, -0.2, 2.1]);
    assert_eq!(thin.len(), 1);
    assert!(matches!(thin[0], Slice::Sampled { weight, .. } if weight == 1.0));
    // a box over the oxide and into the layer: oxide, then the sampled layer
    let both = fills.slices([0.2, -0.3, 1.9], [0.3, -0.2, 2.1]);
    assert_eq!(both.len(), 2);
    let total: f64 = both
        .iter()
        .map(|s| match s {
            Slice::Plane { weight, .. } | Slice::Sampled { weight, .. } => *weight,
        })
        .sum();
    assert!((total - 1.0).abs() < 1e-15);
}
