//! Tests of the catalogue: its data is consistent, and its models evaluate where they say.

use super::models;
use super::references::all_keys;
use super::*;
use crate::units::Wavelength;

fn n(m: &Material, lam: f64) -> f64 {
    m.refractive_index(Wavelength::um(lam).unwrap()).unwrap().re
}

#[test]
fn ids_are_unique_and_every_source_is_cited() {
    let all = catalogue();
    let mut ids: Vec<&str> = all.iter().map(|e| e.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), all.len());
    let keys = all_keys();
    for e in &all {
        let mut sources: Vec<&Source> = e.index.iter().flat_map(|m| &m.sources).collect();
        sources.extend(e.tensors.iter().filter_map(|t| t.source.as_ref()));
        sources.extend(e.constants.iter().map(|c| &c.source));
        for s in sources {
            assert!(
                e.reference(&s.reference).is_some(),
                "{}: {} isn't among its references",
                e.id,
                s.reference
            );
        }
        for r in &e.references {
            assert!(keys.contains(&r.key.as_str()));
            assert!(r.doi.starts_with("10."), "{}", r.key);
        }
        let mut model_ids: Vec<&str> = e.index.iter().map(|m| m.id.as_str()).collect();
        model_ids.sort_unstable();
        model_ids.dedup();
        assert_eq!(model_ids.len(), e.index.len(), "{}", e.id);
        assert!(
            e.index.iter().filter(|m| m.default).count() <= 1,
            "{}",
            e.id
        );
    }
    assert!(entry("linbo3").is_some() && entry("nothing").is_none());
}

#[test]
fn every_model_evaluates_at_its_defaults_and_ends_of_its_ranges() {
    for e in catalogue() {
        for m in &e.index {
            let materials = m.materials(Conditions::default()).unwrap();
            assert_eq!(materials.len(), m.axes.len(), "{}", m.id);
            for mat in &materials {
                let (lo, hi) = mat.range();
                for lam in [lo, hi] {
                    let index = mat.refractive_index(lam).unwrap();
                    assert!(index.re > 1.0 && index.re < 5.0, "{}: {index}", m.id);
                    assert!(mat.group_index(lam).unwrap().is_finite(), "{}", m.id);
                }
                assert!(mat.provenance().doi.starts_with("10."), "{}", m.id);
            }
            if let Some(p) = &m.temperature {
                for t in [p.min, p.max] {
                    let c = Conditions {
                        temperature: Some(t),
                        ..Conditions::default()
                    };
                    assert!(m.materials(c).is_ok(), "{} at {t} K", m.id);
                }
                let c = Conditions {
                    temperature: Some(p.max + 10.0),
                    ..Conditions::default()
                };
                assert!(m.materials(c).is_err(), "{}", m.id);
            }
            if let Some(p) = &m.composition {
                for x in [p.min, 0.5 * (p.min + p.max), p.max] {
                    let c = Conditions {
                        composition: Some(x),
                        ..Conditions::default()
                    };
                    assert!(m.materials(c).is_ok(), "{} at x = {x}", m.id);
                }
                let c = Conditions {
                    composition: Some(p.max + 0.1),
                    ..Conditions::default()
                };
                assert!(m.materials(c).is_err(), "{}", m.id);
            }
            // the nominal range is the one the default material has
            let (lo, hi) = materials[0].range();
            assert!(
                (lo.to_um() - m.wavelength.0).abs() < 1e-3
                    && (hi.to_um() - m.wavelength.1).abs() < 1e-3,
                "{}: {} to {} against {:?}",
                m.id,
                lo,
                hi,
                m.wavelength
            );
        }
    }
}

#[test]
fn tensors_follow_their_point_groups() {
    for e in catalogue() {
        for t in &e.tensors {
            assert_eq!(t.point_group, e.crystal.point_group, "{}", e.id);
            let p = pattern(t.kind, &t.point_group).unwrap();
            let (rows, cols) = match t.kind {
                TensorKind::SecondOrder => (3, 6),
                TensorKind::ElectroOptic => (6, 3),
            };
            assert_eq!(t.cells.len(), rows);
            for (i, row) in t.cells.iter().enumerate() {
                assert_eq!(row.len(), cols);
                for (j, cell) in row.iter().enumerate() {
                    let ok = matches!(
                        (cell, p[i][j]),
                        (Cell::Zero, Pattern::Zero)
                            | (Cell::Value { .. } | Cell::Unknown, Pattern::Independent)
                    ) || matches!((cell, p[i][j]), (Cell::Same { row, col, sign }, Pattern::Same { row: r, col: c, sign: s }) if (row, col, sign) == (&r, &c, &s));
                    assert!(ok, "{} {:?}: element {}{}", e.id, t.label, i + 1, j + 1);
                }
            }
            if e.crystal.centrosymmetric {
                assert!(t.cells.iter().flatten().all(|c| *c == Cell::Zero));
            }
            if t.source.is_some() {
                assert_eq!(
                    t.clamping == Clamping::None,
                    t.kind == TensorKind::SecondOrder,
                    "{}",
                    t.label
                );
            }
        }
    }
    // a value that symmetry ties to another follows it, with its sign
    let ln = entry("linbo3").unwrap();
    let r = ln
        .tensors
        .iter()
        .find(|t| t.kind == TensorKind::ElectroOptic)
        .unwrap();
    assert_eq!(r.value(1, 2), Some(-3.40)); // r12 = -r22
    assert_eq!(r.value(4, 2), Some(18.1)); // r42 = r51
    assert_eq!(r.value(1, 1), Some(0.0));
    assert_eq!(r.value(7, 1), None);
}

#[test]
fn the_point_groups_patterns_are_the_textbook_ones() {
    use Pattern::{Independent as I, Zero as Z};
    let count = |kind, pg| {
        pattern(kind, pg)
            .unwrap()
            .iter()
            .flatten()
            .filter(|p| **p == I)
            .count()
    };
    // independent elements: -43m one (d14, r41), 3m four (d15 d22 d31 d33; r13 r22 r33 r51),
    // 6mm three (d15 d31 d33; r13 r33 r51)
    assert_eq!(count(TensorKind::SecondOrder, "-43m"), 1);
    assert_eq!(count(TensorKind::ElectroOptic, "-43m"), 1);
    assert_eq!(count(TensorKind::SecondOrder, "3m"), 4);
    assert_eq!(count(TensorKind::ElectroOptic, "3m"), 4);
    assert_eq!(count(TensorKind::SecondOrder, "6mm"), 3);
    assert_eq!(count(TensorKind::ElectroOptic, "6mm"), 3);
    let d = pattern(TensorKind::SecondOrder, "3m").unwrap();
    assert_eq!(
        d[1][0],
        Pattern::Same {
            row: 2,
            col: 2,
            sign: -1
        }
    ); // d21 = -d22
    assert_eq!(d[0][0], Z);
    assert!(pattern(TensorKind::SecondOrder, "4mm").is_none());
}

#[test]
fn lithium_niobate_is_negative_uniaxial_in_every_model() {
    for id in ["linbo3", "linbo3-mgo"] {
        let e = entry(id).unwrap();
        let m = e.default_model().unwrap();
        let [o, ex] = &m.materials(Conditions::default()).unwrap()[..] else {
            panic!("two indices")
        };
        for lam in [0.4, 0.633, 1.064, 1.55, 3.0, 5.0] {
            assert!(n(o, lam) > n(ex, lam) + 0.03, "{id} at {lam}");
        }
    }
    // Gayer's n_e and n_o of 5% MgO:LN at 1.064 um, 24.5 C: about 2.147 and 2.228
    let mgo = entry("linbo3-mgo").unwrap();
    let c = Conditions {
        temperature: Some(297.65),
        ..Conditions::default()
    };
    let e = &mgo.index[1].materials(c).unwrap()[0];
    let o = &mgo.index[2].materials(c).unwrap()[0];
    assert!(n(o, 1.064) - n(e, 1.064) > 0.07);
}

#[test]
fn the_printed_coefficients_are_the_ones_evaluated() {
    let parse = |s: &str| -> f64 {
        s.replace('−', "-")
            .replace(' ', "")
            .parse()
            .unwrap_or_else(|_| panic!("{s}"))
    };
    for (row, printed) in models::LEVITON.iter().zip(models::LEVITON_PRINTED) {
        for (v, p) in row.iter().zip(printed) {
            assert_eq!(*v, parse(p));
        }
    }
    for (columns, printed) in [
        (models::ZELMON_CONGRUENT, models::ZELMON_CONGRUENT_PRINTED),
        (models::ZELMON_MGO, models::ZELMON_MGO_PRINTED),
    ] {
        for (i, [e, o]) in printed.iter().enumerate() {
            assert_eq!(columns.0[i], parse(e));
            assert_eq!(columns.1[i], parse(o));
        }
    }
    let iv = models::TABLE_IV_PRINTED;
    for (i, row) in iv.iter().enumerate().skip(1) {
        assert_eq!(models::TABLE_IV_A[i - 1], parse(row[0]));
        assert_eq!(models::TABLE_IV_C1[i], parse(row[1]));
        assert_eq!(models::TABLE_IV_INV_C0[i], parse(row[3]));
        if i <= 2 {
            assert_eq!(models::TABLE_IV_E1[i - 1], parse(row[2]));
            assert_eq!(models::TABLE_IV_E0[i - 1], parse(row[4]));
        } else {
            assert_eq!(parse(row[2]), 0.0);
            assert_eq!(parse(row[4]), 0.0);
        }
    }
    assert_eq!(models::TABLE_IV_C1[0], parse(iv[0][1]));
    assert_eq!(models::TABLE_IV_INV_C0[0], parse(iv[0][3]));
}

#[test]
fn gehrsitz_reduces_to_its_binaries() {
    // x = 0 is GaAs Fit 2 of Table II at 298 K; x = 1 is close to Table II's AlAs fit
    let (a, c0, e0, c1, e1_sq) = models::gehrsitz_parameters(0.0, 298.0);
    assert!((a - 6.090524).abs() < 1e-4, "{a}");
    assert!((c0 - 0.019788).abs() < 1e-6, "{c0}");
    assert!((e0 * e0 - 1.321079).abs() < 1e-5);
    assert!((c1 - 21.5647).abs() < 1e-12);
    assert!((e1_sq - 4.500042).abs() < 1e-5);
    let (a, c0, e0, c1, e1_sq) = models::gehrsitz_parameters(1.0, 296.2);
    assert!((a - 2.18576).abs() < 0.03, "{a}");
    assert!((c0 - 0.0721).abs() < 0.002, "{c0}");
    assert!((e0 * e0 - 5.8777).abs() < 0.01);
    assert!((c1 - 73.908).abs() < 0.05);
    assert!((e1_sq - 12.4).abs() < 0.05);
    // the GaAs entry's second model is the AlGaAs model at x = 0
    let gaas = entry("gaas").unwrap();
    let algaas = entry("algaas").unwrap();
    let a = &gaas.index[1].materials(Conditions::default()).unwrap()[0];
    let b = &algaas.index[0]
        .materials(Conditions {
            composition: Some(0.0),
            ..Conditions::default()
        })
        .unwrap()[0];
    assert_eq!(n(a, 1.55), n(b, 1.55));
    // GaAs at 1.55 um: Skauli and Gehrsitz agree to about 3e-3
    let skauli = &gaas.index[0].materials(Conditions::default()).unwrap()[0];
    assert!((n(skauli, 1.55) - n(a, 1.55)).abs() < 5e-3);
    // the index falls as the Al fraction grows
    let at = |x: f64| {
        let c = Conditions {
            composition: Some(x),
            ..Conditions::default()
        };
        n(&algaas.index[0].materials(c).unwrap()[0], 1.55)
    };
    assert!(at(0.0) > at(0.3) && at(0.3) > at(0.6) && at(0.6) > at(1.0));
}

#[test]
fn afromowitz_reproduces_its_printed_constants() {
    // p. 60: GaAs E_f = 4.962, eta = 0.1032; AlAs eta = 0.03803 (the AlAs E_f is printed as
    // 5.966, which E_0 = 4.70 and E_gamma = 2.95 make 5.956: a misprint)
    let (ef, eta) = models::afromowitz_constants(3.65, 36.1, 1.424);
    assert!((ef - 4.962).abs() < 5e-4 && (eta - 0.1032).abs() < 5e-5);
    let (e0, ed, gap) = models::afromowitz_parameters(1.0);
    assert!((e0 - 4.70).abs() < 1e-9 && (ed - 33.65).abs() < 1e-9 && (gap - 2.95).abs() < 1e-9);
    let (ef, eta) = models::afromowitz_constants(e0, ed, gap);
    assert!(
        (eta - 0.03803).abs() < 5e-6 && (ef - 5.956).abs() < 5e-4,
        "{ef} {eta}"
    );
}

#[test]
fn every_catalogue_check_passes() {
    for (name, f) in [
        (
            "leviton",
            checks::leviton_table as fn() -> crate::validation::Outcome,
        ),
        ("zelmon_opo", checks::zelmon_opo),
        ("zelmon_633", checks::zelmon_633),
        ("jundt_opo", checks::jundt_opo),
        ("gayer_zelmon", checks::gayer_zelmon),
        ("gehrsitz_n_inf", checks::gehrsitz_n_inf),
        ("gehrsitz_gap", checks::gehrsitz_gap),
        ("gehrsitz_samples", checks::gehrsitz_samples),
        ("gehrsitz_papatryfonos", checks::gehrsitz_papatryfonos),
        ("skauli_shg", checks::skauli_shg),
        ("skauli_dndt", checks::skauli_dndt),
        ("afromowitz_eta", checks::afromowitz_eta),
        ("shoji_miller", checks::shoji_miller),
        ("tanaka_ueno", checks::tanaka_ueno),
        ("pettit_turner_suzuki", checks::pettit_turner_suzuki),
        ("suzuki_tada_voltages", checks::suzuki_tada_voltages),
        ("majkic_d33", checks::majkic_d33),
        ("rigler_2015_se", checks::rigler_2015_se),
        ("rigler_2015_maie", checks::rigler_2015_maie),
        (
            "ferrini_table_consistency",
            checks::ferrini_table_consistency,
        ),
        ("ferrini_table_knots", checks::ferrini_table_knots),
    ] {
        let o = f();
        assert!(o.passed(), "{name}: {o:?}");
    }
}

#[test]
fn tanakas_quaternary_ends_at_its_inalp_fit() {
    // Eqs. (3)-(4) at x = 0.51 against the In0.49Al0.51P fit, E0 = 4.01 and Ed = 29.79 eV
    let m = models::tanaka(0.51).unwrap();
    for e in [0.6, 1.0, 1.3] {
        let lam = crate::material::PHOTON_EV_UM / e;
        let direct = (1.0 + 4.01 * 29.79 / (4.01f64.powi(2) - e * e)).sqrt();
        assert!((n(&m, lam) - direct).abs() < 2e-4, "{e} eV");
    }
    // and Pettit & Turner's InP at 1 um, 298 K: 3.3265
    let inp = models::pettit_turner(298.0).unwrap();
    assert!((n(&inp, 1.0) - 3.3265).abs() < 1e-4);
    assert!(models::pettit_turner(200.0).is_err());
}

#[test]
fn the_nitrides_and_ferrinis_ingap_behave() {
    // AlN is positive uniaxial in both polarities, by about 2.5% (Rigler et al. 2015)
    for p in 0..2 {
        let m = models::rigler_2015(p).unwrap();
        for lam in [0.4, 0.658, 0.9] {
            let (o, e) = (n(&m[0], lam), n(&m[1], lam));
            assert!(e > o && (e / o - 1.0) < 0.04, "{p} {lam}");
        }
    }
    // AlGaN (Rigler et al. 2013): every film is positive uniaxial, and among the III-polar
    // films the index falls as the Al fraction grows
    let mut last = f64::INFINITY;
    for s in 1..=9u8 {
        let m = models::rigler_2013(s).unwrap();
        if m.len() == 2 {
            assert!(n(&m[1], 0.6328) > n(&m[0], 0.6328), "sample {s}");
        }
        if s <= 5 {
            let o = n(&m[0], 0.6328);
            assert!(o < last, "sample {s}");
            last = o;
        }
    }
    assert_eq!(models::rigler_2013(8).unwrap().len(), 1);
    assert!(models::rigler_2013(10).is_err());
    // Ferrini's Sellmeier against Tanaka's model where both hold (0.95-2.07 um): within 2%,
    // 0.6% at 1.55 um
    let f = models::ferrini_sellmeier().unwrap();
    let t = models::tanaka(0.0).unwrap();
    let (lo, hi) = t.range();
    for i in 0..=50 {
        let lam = lo.to_um() + (hi.to_um() - lo.to_um()) * f64::from(i) / 50.0;
        assert!((n(&f, lam) / n(&t, lam) - 1.0).abs() < 0.02, "{lam}");
    }
    assert!((n(&f, 1.55) / n(&t, 1.55) - 1.0).abs() < 0.007);
    // the interband table is lossy, the Sellmeier lossless
    let table = models::ferrini_table().unwrap();
    assert!(
        table
            .refractive_index(Wavelength::um(0.4).unwrap())
            .unwrap()
            .im
            > 0.1
    );
}
