use std::f64::consts::PI;

use super::checks::{self, drawn};
use super::*;
use crate::expr::{Expr, Parameters};
use crate::geometry::Path;

#[test]
fn every_port_is_its_closed_form_and_sits_on_its_guides_end() {
    let e = checks::port_error();
    assert!(e < 1e-12, "{e:e}");
}

#[test]
fn lengths_and_areas_are_their_closed_forms() {
    let e = checks::area_error();
    assert!(e < 1e-9, "{e:e}");
}

#[test]
fn gaps_are_the_gaps_asked_for() {
    let e = checks::gap_error();
    assert!(e < 1e-12, "{e:e}");
}

#[test]
fn couplers_and_crossings_are_symmetric() {
    assert_eq!(checks::symmetry_errors(), 0);
}

#[test]
fn the_devices_draw_what_they_drew_before() {
    assert_eq!(checks::device_errors(), 0);
}

#[test]
fn the_ends_of_the_paths_are_the_ports() {
    // the bends' and S-bends' centre lines end where their ports are, along their directions
    let w = um(0.5);
    for p in [0.0, 0.3, 1.0] {
        for angle in [90.0, -45.0, 180.0] {
            let b = Bend::euler(w, um(5.0), angle, p);
            let path = Path::builder(Point::um(0.0, 0.0), 0.0, w)
                .euler(um(5.0), f64::to_radians(angle), p)
                .build()
                .unwrap();
            let d = drawn(&b);
            let o2 = d.port("o2").unwrap();
            let end = path.end();
            assert!((end.position.x - o2.position.x).to_um().abs() < 1e-12);
            assert!((end.position.y - o2.position.y).to_um().abs() < 1e-12);
            assert!((degrees(end.direction.to_degrees()) - o2.direction).abs() < 1e-10);
        }
    }
    for kind in [
        SBendKind::Cosine,
        SBendKind::Sine,
        SBendKind::Euler { p: 0.4 },
        SBendKind::Bezier,
    ] {
        let s = SBend::new(w, um(10.0), um(-2.5), kind);
        let path = s.path().unwrap();
        let end = path.end();
        assert!((end.position.x.to_um() - 10.0).abs() < 1e-12, "{kind:?}");
        assert!((end.position.y.to_um() + 2.5).abs() < 1e-12, "{kind:?}");
        assert!(end.direction.abs() < 1e-12, "{kind:?}: {}", end.direction);
    }
}

#[test]
fn a_spiral_is_its_closed_form_and_never_meets_itself() {
    for (turns, straight) in [(1, 0.0), (2, 0.0), (3, 2.0), (6, 5.0)] {
        let s = Spiral {
            straight: um(straight),
            ..Spiral::new(um(0.5), um(1.5), um(4.0), turns)
        };
        // the path refuses an outline that crosses itself
        let d = drawn(&s);
        let Shape::Path(path) = &d.items()[0].shape else {
            panic!("a path")
        };
        assert!(
            (path.length().to_um() - s.length().to_um()).abs() < 1e-9,
            "{turns}: {} {}",
            path.length(),
            s.length()
        );
        let (o1, o2) = (d.port("o1").unwrap(), d.port("o2").unwrap());
        assert_eq!(o1.position.x, -o2.position.x);
        assert_eq!(o1.position.y, -o2.position.y);
        assert_eq!(degrees(o1.direction + 180.0), o2.direction);
        // B's outer guide at its foot: the next guide in is the pitch away
        let ys: Vec<f64> = (0..=turns).map(|k| y_after(&s, k)).collect();
        if turns >= 3 {
            // the left side's lowest point of B's semicircle 2 and A's semicircle 1's foot,
            // turned: d apart
            let gap = (ys[2] - (-(2.0 * 4.0))).abs();
            assert!((gap - 1.5).abs() < 1e-12, "{gap}");
        }
    }
    // too tight a pitch, or a first turn too small for it
    assert!(
        Spiral::new(um(0.5), um(0.4), um(4.0), 3)
            .draw("Si")
            .is_err()
    );
    assert!(
        Spiral::new(um(0.5), um(3.0), um(1.0), 3)
            .draw("Si")
            .is_err()
    );
}

/// B's y after its k-th semicircle.
fn y_after(s: &Spiral, k: usize) -> f64 {
    s.radii()[..k]
        .iter()
        .enumerate()
        .map(|(j, r)| if j % 2 == 0 { 2.0 * r } else { -2.0 * r })
        .sum()
}

#[test]
fn a_custom_taper_and_a_y_branch_outline_are_expressions() {
    let p =
        Parameters::new(&[("w0".into(), "500 nm".into()), ("l".into(), "4 um".into())]).unwrap();
    let e = Expr::parse("w0 + 0.5 um*(x/l)^2", p.scope()).unwrap();
    let t = Taper::custom(e.clone(), p.clone(), um(4.0));
    let d = drawn(&t);
    assert_eq!(d.port("o1").unwrap().width, um(0.5));
    assert!((d.port("o2").unwrap().width.to_um() - 1.0).abs() < 1e-15);
    // ∫ w = 4 (0.5 + 0.5/3)
    let area = d.items()[0].shape.area();
    assert!((area - 4.0 * (0.5 + 0.5 / 3.0)).abs() < 1e-9, "{area}");
    // a Y-branch on an outline ending at 2w + gap
    let y = YBranch::new(um(0.5), um(0.25), um(4.0), um(8.0), um(3.0)).with_outline(
        Expr::parse("0.5 um + 0.4 um*x/l", p.scope()).unwrap(),
        p.clone(),
    );
    let d = drawn(&YBranch {
        gap: um(0.4),
        ..y.clone()
    });
    assert_eq!(d.port("o1").unwrap().width, um(0.5));
    let e = YBranch { gap: um(0.2), ..y }
        .draw("Si")
        .unwrap_err()
        .to_string();
    assert!(e.contains("2 width + gap"), "{e}");
}

#[test]
fn errors_name_the_component_and_the_parameter() {
    let says = |g: &dyn Generator, what: &str| {
        let e = g.draw("Si").unwrap_err().to_string();
        assert!(e.contains(what), "{e}");
    };
    says(
        &Straight::new(um(-0.5), um(1.0)),
        "a straight's width must be positive",
    );
    says(
        &Ring::new(um(0.5), um(5.0), um(0.0)),
        "a ring's gap must be positive",
    );
    says(
        &Ring::new(um(0.5), um(5.0), um(0.2)).with_drop(um(-1.0)),
        "a ring's drop_gap must be positive",
    );
    says(
        &Bend::euler(um(0.5), um(5.0), 90.0, 1.5),
        "p must be from 0 to 1",
    );
    says(
        &SBend::new(um(0.5), um(5.0), um(0.0), SBendKind::Sine),
        "its offset must not be zero",
    );
    says(
        &Mmi::new(um(3.0), um(20.0), 4, 1, um(1.0)),
        "isn't inside its width",
    );
    says(
        &Mmi {
            input_positions: Some(vec![um(0.5), um(-0.5)]),
            ..Mmi::new(um(3.0), um(20.0), 2, 1, um(0.5))
        },
        "give them in order",
    );
    says(&Crossing::new(um(0.5), um(0.2)), "its arm");
    says(
        &YBranch::new(um(0.5), um(0.5), um(2.0), um(5.0), um(0.4)),
        "its separation",
    );
    says(
        &BentCoupler::new(um(0.5), um(0.2), um(5.0), 400.0),
        "below 360",
    );
}

#[test]
fn a_drawing_moves_turns_and_meets_another_port() {
    let ring = drawn(&Ring::new(um(0.5), um(5.0), um(0.2)));
    let through = ring.port("through").unwrap().clone();
    // a straight whose o1 meets the ring's through port: it continues the bus
    let s = drawn(&Straight::new(um(0.5), um(3.0)));
    let joined = s.connected("o1", &through, false).unwrap();
    assert!(joined.port("o1").unwrap().meets(&through, Length::ZERO));
    let o2 = joined.port("o2").unwrap();
    assert_eq!(
        o2.position,
        Point::um(through.position.x.to_um() + 3.0, 0.0)
    );
    assert_eq!(o2.direction, 0.0);
    // onto the drop port of an add-drop ring turned a quarter: exact
    let ring = drawn(&Ring::new(um(0.5), um(5.0), um(0.2)).with_drop(um(0.2)))
        .transformed(&rotation(90.0));
    let drop = ring.port("drop").unwrap().clone();
    assert_eq!(drop.direction, 270.0);
    let joined = s.connected("o2", &drop, true).unwrap();
    let o2 = joined.port("o2").unwrap();
    assert!(o2.meets(&drop, Length::ZERO), "{o2:?} {drop:?}");
    assert_eq!(o2.direction, 90.0);
    // turned by any angle, a port turns with its guide
    let t = Transform::rotate(0.3, Point::um(1.0, 2.0));
    let b = drawn(&Bend::arc(um(0.5), um(5.0), 60.0)).transformed(&t);
    let q = b.port("o2").unwrap();
    assert!((q.direction - (60.0 + 0.3f64.to_degrees())).abs() < 1e-12);
    assert!(!b.contains("Si", step(q, 1e-3)));
    assert!(b.contains("Si", step(q, -1e-3)));
    // a mirror reflects a port's direction
    let m = drawn(&Bend::arc(um(0.5), um(5.0), 60.0))
        .transformed(&Transform::mirror(Point::default(), 0.0));
    assert!((m.port("o2").unwrap().direction - 300.0).abs() < 1e-12);
    // a missing port is named
    let e = s.connected("o9", &drop, false).unwrap_err().to_string();
    assert!(
        e.contains("a straight has no port o9: its ports are o1, o2"),
        "{e}"
    );
}

#[test]
fn a_drawing_is_drawn_on_a_structure_and_its_ports_are_a_circuits() {
    let d = drawn(&Mmi::new(um(3.0), um(20.0), 1, 2, um(0.5)));
    let s = d.structure(crate::stack::LayerStack::soi_220()).unwrap();
    assert_eq!(s.shapes("Si").len(), d.items().len());
    let names: Vec<String> = d.circuit_ports().into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["o1", "o2", "o3"]);
    // the circuit's 1 × 2 MMI: o1 in, o2 and o3 out, top to bottom
    assert!(d.port("o2").unwrap().position.y > d.port("o3").unwrap().position.y);
    let e = d.structure(crate::stack::LayerStack::nitride(um(0.4), um(2.0)).unwrap());
    assert!(e.unwrap_err().to_string().contains("no layer Si"));
    let b = d.bounds().unwrap();
    assert_eq!(b.min.x, um(0.0));
    assert_eq!(b.max.x, um(20.0));
}

#[test]
fn self_imaging_lengths_are_soldano_and_pennings() {
    // L_π = 4 n_r W_e²/3λ₀ with W_e = W_M + (λ₀/π)(n_c/n_r)^2σ (n_r² − n_c²)^−½
    let (wm, nr, nc, l0): (f64, f64, f64, f64) = (6.0, 2.85, 1.44, 1.55);
    let we_te = wm + (l0 / PI) / (nr * nr - nc * nc).sqrt();
    let we_tm = wm + (l0 / PI) * (nc / nr).powi(2) / (nr * nr - nc * nc).sqrt();
    let te = Mmi::beat_length(um(wm), nr, nc, um(l0), false).to_um();
    let tm = Mmi::beat_length(um(wm), nr, nc, um(l0), true).to_um();
    assert!((te - 4.0 * nr * we_te * we_te / (3.0 * l0)).abs() < 1e-12);
    assert!((tm - 4.0 * nr * we_tm * we_tm / (3.0 * l0)).abs() < 1e-12);
    let lp = um(te);
    // a 1 × 2 splitter at 3L_π/8, a 2 × 2 3 dB coupler at L_π/2, a general 4 × 4 at 3L_π/4
    assert!((SelfImaging::Symmetric.length(2, lp).to_um() - 3.0 * te / 8.0).abs() < 1e-12);
    assert!((SelfImaging::Paired.length(2, lp).to_um() - te / 2.0).abs() < 1e-12);
    assert!((SelfImaging::General.length(4, lp).to_um() - 3.0 * te / 4.0).abs() < 1e-12);
}

/// The point `d` out of port `q` along its direction.
fn step(q: &Port, d: f64) -> Point {
    let [x, y] = q.outward();
    Point::um(q.position.x.to_um() + d * x, q.position.y.to_um() + d * y)
}

#[test]
fn ports_are_exact_under_quarter_turns() {
    let d = drawn(&DirectionalCoupler::new(um(0.5), um(0.2), um(4.0)));
    for k in 0..8 {
        let t = rotation(90.0 * k as f64);
        let turned = d.transformed(&t);
        for (a, b) in d.ports().iter().zip(turned.ports()) {
            assert_eq!(b.direction, degrees(a.direction + 90.0 * k as f64));
            assert_eq!(b.width, a.width);
        }
    }
    assert_eq!(angle_of([0.0, -2.0]), 270.0);
    assert_eq!(degrees(-0.0), 0.0);
    assert_eq!(degrees(-1e-300), 0.0);
}
