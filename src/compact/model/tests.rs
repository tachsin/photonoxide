use super::*;
use crate::compact::checks::Ring;
use crate::compact::fit::Symmetry;

fn um(v: f64) -> Wavelength {
    Wavelength::um(v).unwrap()
}

/// A 1-port spectrum of `f(λ)` at `count` wavelengths over [lo, hi].
fn spectrum(f: impl Fn(f64) -> c64, lo: f64, hi: f64, count: usize) -> Spectrum {
    let ws: Vec<Wavelength> = (0..count)
        .map(|k| um(lo + (hi - lo) * k as f64 / (count - 1) as f64))
        .collect();
    let ms = ws
        .iter()
        .map(|w| SMatrix::from_fn(1, |_, _| f(w.to_um())))
        .collect();
    Spectrum::new(vec!["o1".into()], ws, ms).unwrap()
}

// Bogaerts et al. 2012, Eq. 1: its poles are exactly where r a e^(iφ) = 1
#[test]
fn an_all_pass_rings_poles_are_recovered() {
    let ring = Ring::example();
    let sp = spectrum(|w| ring.through(w), 1.53, 1.57, 401);
    let model = CompactModel::fit(&sp, &Options::new(12)).unwrap();
    assert!(model.error().max < 1e-9, "{:?}", model.error());
    let exact = ring.poles(TAU / 1.57, TAU / 1.53);
    assert_eq!(exact.len(), 4);
    let r = model.rational();
    for (a, c) in exact {
        let (j, b) = r
            .poles
            .iter()
            .enumerate()
            .min_by(|x, y| (x.1 - a).norm().total_cmp(&(y.1 - a).norm()))
            .unwrap();
        // to a small part of the resonance's half-width, Re a
        assert!((b - a).norm() < 1e-8 * a.re.abs(), "{b} {a}");
        assert!((r.residues[0][j] - c).norm() < 1e-8 * c.norm());
    }
    // between the samples too
    for k in 0..=1000 {
        let w = 1.53 + 0.04 * k as f64 / 1000.0;
        assert!((model.at(um(w)).unwrap()[(0, 0)] - ring.through(w)).norm() < 1e-8);
    }
    // a lossy ring is passive, and the model says so
    let p = model.passivity(2001).unwrap();
    assert!(p.passive(0.0) && p.violations.is_empty(), "{:?}", p.largest);
    // a real model needs a pair per resonance, and fits as well
    let real = CompactModel::fit(
        &sp,
        &Options {
            symmetry: Symmetry::Real,
            ..Options::new(24)
        },
    )
    .unwrap();
    assert!(real.error().max < 1e-9, "{:?}", real.error());
}

#[test]
fn a_compact_model_is_a_component() {
    let ring = Ring::example();
    let sp = spectrum(|w| ring.through(w), 1.53, 1.57, 201);
    let model = CompactModel::fit(&sp, &Options::new(12))
        .unwrap()
        .with_kind("ring")
        .with_source("Bogaerts et al. 2012, Eq. 1");
    assert_eq!(model.kind(), "ring");
    assert_eq!(model.ports()[0].name, "o1");
    assert!(model.parameters().is_empty());
    let p = model.provenance();
    assert_eq!(p.fidelity, Fidelity::Compact);
    assert!(
        p.source.contains("Bogaerts") && p.source.contains("doi:10.1109/61.772353"),
        "{}",
        p.source
    );
    assert_eq!(p.error, Some(model.error().max));
    assert_eq!(p.validity, Some((1.53, 1.57)));
    let s = model.s_matrix(um(1.55), &[]).unwrap();
    assert!((s[(0, 0)] - ring.through(1.55)).norm() < 1e-8);
    // outside its band it says nothing
    let e = model.s_matrix(um(1.6), &[]).unwrap_err().to_string();
    assert!(e.contains("1.53 to 1.57"), "{e}");
    // its spectrum as a file, and fitted again from the file
    let file = model.to_touchstone(301, Convention::Physics).unwrap();
    let again =
        CompactModel::from_touchstone(&file, Convention::Physics, &Options::new(12)).unwrap();
    for k in 0..=100 {
        let w = um(1.53 + 0.04 * k as f64 / 100.0);
        assert!(
            again
                .at(w)
                .unwrap()
                .max_difference(&model.at(w).unwrap())
                .unwrap()
                < 1e-8
        );
    }
}

#[test]
fn passivity_violations_are_found() {
    // a resonant gain of 1.2 at 1.55 um, 1 + 0.2 g/(g − i(k − k0)): |S| above 1
    let (g, k0) = (0.002, TAU / 1.55);
    let sp = spectrum(
        |w| 1.0 + 0.2 * g / c64::new(g, -(TAU / w - k0)),
        1.5,
        1.6,
        201,
    );
    let model = CompactModel::fit(&sp, &Options::new(2)).unwrap();
    let p = model.passivity(1001).unwrap();
    assert!(!p.passive(1e-3));
    assert!(
        (p.largest.0.to_um() - 1.55).abs() < 1e-3 && (p.largest.1 - 1.2).abs() < 1e-2,
        "{:?}",
        p.largest
    );
    // (1 + 0.2 g/(g − iΔ) has a positive real part at every detuning: above 1 everywhere)
    assert!(p.violations.len() == 1001 && p.violations.iter().all(|&(_, v)| v > 1.0));
    assert!(model.passivity(1).is_err());
}

#[test]
fn bad_spectra_are_errors() {
    let one = spectrum(|_| c64::new(1.0, 0.0), 1.5, 1.6, 2);
    let single = Spectrum::new(vec!["o1".into()], vec![um(1.55)], vec![SMatrix::zeros(1)]).unwrap();
    assert!(CompactModel::fit(&single, &Options::new(1)).is_err());
    assert!(CompactModel::fit(&one, &Options::new(4)).is_err());
}

#[test]
fn a_measured_spectrum_interpolates_in_frequency() {
    // S = f linear in frequency is reproduced exactly between samples
    let sp = spectrum(|w| c64::new(1.0 / w, 0.5 / w), 1.5, 1.6, 11);
    let m = Measured::new(&sp, "a test").unwrap().with_kind("detector");
    assert_eq!(m.kind(), "detector");
    for k in 0..=50 {
        let w = 1.5 + 0.1 * k as f64 / 50.0;
        let s = m.s_matrix(um(w), &[]).unwrap();
        assert!(
            (s[(0, 0)] - c64::new(1.0 / w, 0.5 / w)).norm() < 1e-14,
            "{w}"
        );
    }
    assert!(m.s_matrix(um(1.61), &[]).is_err());
    let p = m.provenance();
    assert_eq!(p.fidelity, Fidelity::Measured);
    assert!(p.source.contains("a test") && p.source.contains("11 wavelengths"));
    assert_eq!(p.error, None);
    let (lo, hi) = p.validity.unwrap();
    assert!((lo - 1.5).abs() < 1e-14 && (hi - 1.6).abs() < 1e-14);
    // read from a Touchstone file, in the engineering convention
    let dir = std::env::temp_dir().join(format!("photonoxide-measured-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("detector.s1p");
    Touchstone::from_spectrum(&sp, Convention::Engineering)
        .unwrap()
        .write_file(&path, super::super::touchstone::Precision::RoundTrip)
        .unwrap();
    let read = Measured::read(&path, Convention::Engineering).unwrap();
    let s = read.s_matrix(um(1.55), &[]).unwrap();
    assert!((s[(0, 0)] - c64::new(1.0 / 1.55, 0.5 / 1.55)).norm() < 1e-14);
    assert!(read.provenance().source.contains("detector.s1p"));
    std::fs::remove_dir_all(&dir).unwrap();
    // two samples at one wavelength
    let twice = Spectrum::new(
        vec!["o1".into()],
        vec![um(1.55), um(1.55)],
        vec![SMatrix::zeros(1), SMatrix::zeros(1)],
    )
    .unwrap();
    assert!(Measured::new(&twice, "x").is_err());
}

// a solver's spectrum: the ring-fdfd job at a coarse grid (slow in a debug build)
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "runs the ring-fdfd job; the validation report runs it in release"
)]
fn the_fdfd_rings_spectrum_is_fitted_and_predicts_between_samples() {
    let (fitted, held) = crate::compact::checks::fdfd_ring_held_out();
    assert!(fitted < 1e-5 && held < 2e-4, "{fitted} {held}");
}
