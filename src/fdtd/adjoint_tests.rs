use super::adjoint_checks::*;
use super::{Boundaries, Design, Simulation};
use crate::fdfd::{Axis, Edges};

/// A design on a small grid with a background that varies across it, and the permittivity
/// with the design's densities in its cells, as a function.
fn small() -> (crate::fdfd::Grid3d, Design, Vec<f64>) {
    let g = super::checks::grid([8, 7, 6], 0.1);
    let design = Design {
        cells: [2..5, 3..5, 1..4],
        eps: [1.5, 9.0],
    };
    let density = uneven(&design);
    (g, design, density)
}

fn background(x: f64, y: f64, _: f64) -> f64 {
    if x + 0.3 * y < 0.05 { 4.0 } else { 2.0 }
}

#[test]
fn a_designs_permittivity_is_simulation_news_average_of_it() {
    let (g, design, density) = small();
    let inside = |x: f64, y: f64, z: f64| {
        let cell = |axis: Axis, v: f64| ((v - g.node(axis, 0)) / g.step(axis)).floor() as usize;
        design.index((cell(Axis::X, x), cell(Axis::Y, y), cell(Axis::Z, z)))
    };
    let composed = |x: f64, y: f64, z: f64| match inside(x, y, z) {
        Some(v) => design.permittivity(density[v]),
        None => background(x, y, z),
    };
    let whole = Simulation::new(g, composed, Boundaries::walls(), 0.9).unwrap();
    let designed = Simulation::new(g, background, Boundaries::walls(), 0.9)
        .unwrap()
        .with_design(&design, &density, background)
        .unwrap();
    for c in 0..3 {
        assert!(
            whole.cb[c]
                .iter()
                .zip(&designed.cb[c])
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "component {c}"
        );
    }
}

#[test]
fn a_design_is_refused_where_its_gradient_wouldnt_be_exact() {
    let (g, design, density) = small();
    let s = || Simulation::new(g, background, Boundaries::walls(), 0.9).unwrap();
    // into a CPML
    let cpml = Simulation::new(g, background, Boundaries::cpml(2), 0.9).unwrap();
    assert!(cpml.with_design(&design, &density, background).is_err());
    // a density of the wrong length, or one giving a permittivity that isn't positive
    assert!(s().with_design(&design, &density[1..], background).is_err());
    let mut negative = density.clone();
    negative[0] = -1.0;
    assert!(s().with_design(&design, &negative, background).is_err());
    // a conductivity on the design's values
    let lossy = s().with_conductivity(|_, _, _| 0.5).unwrap();
    assert!(lossy.with_design(&design, &density, background).is_err());
    // a Bloch phase
    let bloch = Simulation::new(
        g,
        background,
        Boundaries {
            x: Edges::Bloch { k: 1.0 },
            ..Boundaries::walls()
        },
        0.9,
    )
    .unwrap();
    assert!(bloch.with_design(&design, &density, background).is_err());
    // off the grid
    let off = Design {
        cells: [6..9, 0..2, 0..2],
        ..design
    };
    assert!(s().with_design(&off, &uneven(&off), background).is_err());
}

#[test]
fn a_designs_cells_are_numbered_with_i_fastest() {
    let (_, design, _) = small();
    assert_eq!(design.len(), 18);
    assert_eq!(design.index((2, 3, 1)), Some(0));
    assert_eq!(design.index((3, 3, 1)), Some(1));
    assert_eq!(design.index((2, 4, 1)), Some(3));
    assert_eq!(design.index((2, 3, 2)), Some(6));
    assert_eq!(design.index((5, 3, 1)), None);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn a_mode_objectives_gradient_is_the_finite_differences() {
    let (worst, pairs) = strip_against_differences(Objective::Modes);
    eprintln!("modes: {worst:e} {pairs:?}");
    assert!(worst < 1e-6, "{worst:e}: {pairs:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn a_flux_objectives_gradient_is_the_finite_differences() {
    let (worst, pairs) = strip_against_differences(Objective::Flux);
    eprintln!("flux: {worst:e} {pairs:?}");
    assert!(worst < 1e-6, "{worst:e}: {pairs:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn dropping_the_modes_imaginary_part_makes_the_gradient_wrong() {
    let error = real_mode_error();
    eprintln!("the mode taken as real: {error:e}");
    assert!(error > 1e-2, "{error:e}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn the_adjoint_field_is_the_mode_sent_backwards() {
    let (worst, mean) = adjoint_is_the_mode_sent_backward();
    eprintln!("Eq. 8: {worst:e}, mean ratio {mean}");
    assert!(worst < 1e-8, "{worst:e}");
}

#[test]
#[ignore = "a measurement: cargo test --release --lib what_a_gradient_costs -- --ignored --nocapture"]
fn what_a_gradient_costs() {
    for (cells, design) in [([36, 20, 20], [3, 2, 2]), ([160, 64, 64], [40, 16, 16])] {
        let c = cost(cells, design);
        eprintln!(
            "{} cells, design {}: forward {:.2} s, {} steps, {:.1} MB; with the design's \
             transforms {:.2} s, {:.1} MB; adjoint {:.2} s, {} steps, {:.1} MB; product {:.4} s",
            c.cells,
            c.design,
            c.forward.0,
            c.forward.1,
            c.forward.2 as f64 / 1e6,
            c.recorded.0,
            c.recorded.1 as f64 / 1e6,
            c.adjoint.0,
            c.adjoint.1,
            c.adjoint.2 as f64 / 1e6,
            c.product
        );
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn the_gradient_is_the_same_bits_on_any_number_of_threads() {
    let one = strip_gradient_on(1);
    for threads in [4, 20] {
        let other = strip_gradient_on(threads);
        assert!(
            one.iter()
                .zip(&other)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "{threads} threads: {one:?} against {other:?}"
        );
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn in_a_closed_lossy_box_the_gradient_is_fdfds() {
    let (worst, count) = against_fdfd_closed();
    eprintln!("closed: {worst:e} over {count} values");
    assert!(worst < 1e-9, "{worst:e}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn with_cpmls_the_gradient_converges_to_fdfds_with_the_step() {
    let errors: Vec<f64> = [64, 128, 256]
        .iter()
        .map(|&steps| against_fdfd_open(steps))
        .collect();
    eprintln!("open: {errors:?}");
    assert!(errors[2] < errors[1] && errors[1] < errors[0], "{errors:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "slow unoptimized: run with --release")]
fn a_slabs_gradient_converges_to_airys_at_second_order() {
    let errors = slab_errors();
    let orders: Vec<f64> = errors.windows(2).map(|e| (e[0] / e[1]).log2()).collect();
    eprintln!("slab: {errors:?} orders {orders:?}");
    assert!(orders.iter().all(|&o| (o - 2.0).abs() < 0.2), "{orders:?}");
}
