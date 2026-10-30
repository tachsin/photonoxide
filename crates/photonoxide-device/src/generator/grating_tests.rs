//! Gratings, photonic crystals and the edge coupler.

use std::f64::consts::PI;

use super::checks::{self, drawn};
use super::*;
use crate::expr::{Expr, Parameters};
use crate::material;
use crate::stack::{Layer, LayerStack};

fn parameters() -> Parameters {
    Parameters::new(&[("dw".into(), "20 nm".into())]).unwrap()
}

#[test]
fn wang_et_als_gratings_are_drawn_as_they_say() {
    let e = checks::wang_error();
    assert!(e < 1e-9, "{e:e}");
}

#[test]
fn an_apodized_grating_follows_its_expression_and_a_sampled_one_its_bursts() {
    let p = parameters();
    let e = Expr::parse("dw*exp(-((x - 3.1 um)/(1 um))^2)", p.scope()).unwrap();
    let g = BraggGrating::new(um(0.5), um(0.02), um(0.31), 20).with_apodization(e, p);
    let d = drawn(&g);
    let Shape::Array(teeth) = &d.items()[1].shape else {
        panic!("teeth")
    };
    assert_eq!(teeth.len(), 40);
    for t in teeth.elements() {
        let b = t.shape.bounds();
        let x = 0.5 * (b.min.x + b.max.x).to_um();
        let want = 0.02 * (-((x - 3.1) / 1.0).powi(2)).exp();
        assert!(((b.max.y - b.min.y).to_um() - want).abs() < 1e-15);
        // on the core: its inner edge at the core's
        let inner = b.min.y.to_um().abs().min(b.max.y.to_um().abs());
        assert!((inner - 0.23).abs() < 1e-15, "{inner}");
    }
    // a tooth of zero height is left out, and one beyond the corrugation refused
    let zero = Expr::parse("if(x < 1 um, 0 nm, dw)", parameters().scope()).unwrap();
    let d = drawn(
        &BraggGrating::new(um(0.5), um(0.02), um(0.31), 20).with_apodization(zero, parameters()),
    );
    let Shape::Array(teeth) = &d.items()[1].shape else {
        panic!("teeth")
    };
    assert_eq!(teeth.len(), 2 * (20 - 3));
    let big = Expr::parse("2*dw", parameters().scope()).unwrap();
    let e = BraggGrating::new(um(0.5), um(0.02), um(0.31), 20)
        .with_apodization(big, parameters())
        .draw("Si")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("a tooth's height is from 0 to the corrugation"),
        "{e}"
    );
    // centred: the guide alternates about its width
    let c = drawn(&BraggGrating {
        centred: true,
        ..BraggGrating::new(um(0.5), um(0.02), um(0.31), 10)
    });
    assert!(c.contains("Si", Point::um(0.155, 0.259)));
    assert!(!c.contains("Si", Point::um(0.155, 0.261)));
    assert!(!c.contains("Si", Point::um(0.31, 0.241)));
    // sampled: bursts of 3 every 7 periods
    let s = drawn(&BraggGrating::new(um(0.5), um(0.02), um(0.31), 21).sampled(3, 7));
    let Shape::Array(teeth) = &s.items()[1].shape else {
        panic!("teeth")
    };
    let mut periods: Vec<i64> = teeth.elements().iter().map(|e| e.index[0]).collect();
    periods.dedup();
    assert_eq!(periods, [0, 1, 2, 7, 8, 9, 14, 15, 16]);
}

#[test]
fn a_subwavelength_grating_and_a_grating_coupler() {
    let s = drawn(&SwgWaveguide {
        width_end: um(0.3),
        bridge: um(0.1),
        ..SwgWaveguide::new(um(0.5), um(0.2), 0.5, 11)
    });
    let Shape::Array(segments) = &s.items()[1].shape else {
        panic!("segments")
    };
    for e in segments.elements() {
        let i = e.index[0] as f64;
        let b = e.shape.bounds();
        assert!(((b.max.y - b.min.y).to_um() - (0.5 - 0.02 * i)).abs() < 1e-15);
        assert!((b.min.x.to_um() - 0.2 * i).abs() < 1e-14);
    }
    // between two segments, the bridge only
    assert!(s.contains("Si", Point::um(0.15, 0.0)));
    assert!(!s.contains("Si", Point::um(0.15, 0.06)));
    // an apodized coupler: each tooth's fill F(x) of its period
    let p = Parameters::new(&[("f0".into(), "0.9".into())]).unwrap();
    let f = Expr::parse("f0 - 0.02*x/(1 um)", p.scope()).unwrap();
    let g = GratingCoupler::new(um(0.5), um(10.0), um(15.0), um(0.6), 0.5, 25, um(0.07))
        .with_apodization(f, p);
    let d = drawn(&g);
    assert_eq!(d.items()[1].etch, crate::stack::Etch::partial(um(0.07)));
    let Shape::Array(teeth) = &d.items()[1].shape else {
        panic!("teeth")
    };
    for t in teeth.elements() {
        let x = 0.6 * t.index[0] as f64;
        let b = t.shape.bounds();
        assert!(((b.max.x - b.min.x).to_um() - (0.9 - 0.02 * x) * 0.6).abs() < 1e-14);
        assert!((b.min.x.to_um() - (15.0 + x)).abs() < 1e-13);
    }
    // a fill out of (0, 1) is refused, naming x
    let p = Parameters::new(&[("f0".into(), "1.2".into())]).unwrap();
    let f = Expr::parse("f0 - 0.02*x/(1 um)", p.scope()).unwrap();
    let e = GratingCoupler::new(um(0.5), um(10.0), um(15.0), um(0.6), 0.5, 25, um(0.07))
        .with_apodization(f, p)
        .draw("Si")
        .unwrap_err()
        .to_string();
    assert!(e.contains("its fill at x = 0 um is 1.2"), "{e}");
    // on two layers: the teeth on the top one, the slab under them on the other
    let two = GratingCoupler::new(um(0.5), um(10.0), um(15.0), um(0.6), 0.5, 25, um(0.07))
        .with_slab_layer("slab")
        .draw("top")
        .unwrap();
    let silicon = |name: &str, t: f64| Layer {
        name: name.into(),
        thickness: um(t),
        material: material::silicon(),
        background: material::silica(),
    };
    let stack = LayerStack::new(
        material::silica(),
        vec![silicon("slab", 0.15), silicon("top", 0.07)],
        material::silica(),
    )
    .unwrap();
    let s = two.structure(stack).unwrap();
    let si = |x: f64, z: f64| s.material_at(Point::um(x, 0.0), um(z)).name().to_owned();
    // a trench, between the first two teeth: the slab only
    assert_eq!(si(15.0 + 0.45, 0.1), "Si");
    assert_ne!(si(15.0 + 0.45, 0.19), "Si");
    assert_eq!(si(15.0 + 0.1, 0.19), "Si");
    // beside the grating, nothing
    assert_ne!(si(-1.0, 0.1), "Si");
}

#[test]
fn photonic_crystals_have_their_defects_and_a_w1_its_guide() {
    let (a, r) = (0.42, 0.12);
    let holes = |c: &PhotonicCrystal| c.sites().unwrap().len();
    let base = PhotonicCrystal::hexagonal(um(a), um(r), [7, 4], Defect::None);
    assert_eq!(holes(&base), 15 * 9);
    let h1 = PhotonicCrystal {
        defect: Defect::H1,
        ..base.clone()
    };
    assert_eq!(holes(&h1), 15 * 9 - 1);
    let l3 = PhotonicCrystal {
        defect: Defect::L3 {
            shift: um(0.15 * a),
        },
        ..base.clone()
    };
    let sites = l3.sites().unwrap();
    assert_eq!(sites.len(), 15 * 9 - 3);
    let end = sites.site([2, 0]).unwrap();
    assert!((end.at.x.to_um() - 2.15 * a).abs() < 1e-15);
    let d = drawn(&l3);
    assert!(d.ports().is_empty());
    assert!(d.contains("Si", Point::um(0.0, 0.0)), "the cavity");
    assert!(!d.contains("Si", Point::um(2.15 * a, 0.0)), "its end hole");
    let w1 = drawn(&PhotonicCrystal {
        defect: Defect::W1,
        ..base
    });
    // along the guide's row, silicon end to end
    for k in 0..40 {
        let x = -3.0 + 0.155 * k as f64;
        assert!(w1.contains("Si", Point::um(x, 0.0)), "{x}");
    }
    let o1 = w1.port("o1").unwrap();
    assert!((o1.width.to_um() - (3f64.sqrt() * a - 2.0 * r)).abs() < 1e-15);
    // the slab less every hole
    let slab = (15.5 * a) * (8.0 * a * 3f64.sqrt() / 2.0 + a);
    assert!((w1.items()[0].shape.area() - (slab - 15.0 * 8.0 * PI * r * r)).abs() < 1e-10);
    // refusals
    let says = |c: PhotonicCrystal, what: &str| {
        let e = c.draw("Si").unwrap_err().to_string();
        assert!(e.contains(what), "{e}");
    };
    says(
        PhotonicCrystal::hexagonal(um(a), um(0.25), [7, 4], Defect::None),
        "less than half its pitch",
    );
    says(
        PhotonicCrystal::hexagonal(um(a), um(r), [1, 4], Defect::L3 { shift: um(0.06) }),
        "at least 2 sites",
    );
}

#[test]
fn an_edge_coupler_tapers_to_its_tip() {
    let e = EdgeCoupler {
        straight: um(2.0),
        profile: TaperProfile::Parabolic,
        ..EdgeCoupler::new(um(0.18), um(0.5), um(40.0))
    };
    let d = drawn(&e);
    assert_eq!(d.port("o1").unwrap().width, um(0.18));
    // w² linear in x: at the middle √((w_tip² + w²)/2)
    let mid = ((0.18f64 * 0.18 + 0.25) / 2.0).sqrt();
    assert!(d.contains("Si", Point::um(20.0, mid / 2.0 - 1e-6)));
    assert!(!d.contains("Si", Point::um(20.0, mid / 2.0 + 1e-6)));
    let wrong = EdgeCoupler::new(um(0.6), um(0.5), um(40.0))
        .draw("Si")
        .unwrap_err();
    assert!(
        wrong.to_string().contains("narrower than its guide"),
        "{wrong}"
    );
}
