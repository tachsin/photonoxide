//! Permittivities and loss tangents at microwave and millimetre-wave frequencies, beside the
//! indices: what a travelling-wave electrode's dielectric loss weighs ([`crate::electro::line`]).
//! Each value is a measurement, with its sample, frequency, temperature and paper.
//!
//! - Lithium niobate: M. Lee's reflectometry of congruent, poled crystals from 90 to 147 GHz
//!   (Appl. Phys. Lett. 79, 1342 (2001), doi:10.1063/1.1399305, Fig. 2, read off the figure:
//!   the paper prints no table).
//! - Silicon: Krupka et al.'s dielectric resonators of float-zone high-resistivity silicon at 4.8
//!   to 6.8 GHz (IEEE Trans. Microw. Theory Tech. 54, 3995 (2006), doi:10.1109/TMTT.2006.883655,
//!   Table I), and the millimetre-wave values J. W. Lamb's compilation lists.
//! - Fused silica, crystalline quartz and sapphire: J. W. Lamb's compilation (Int. J. Infrared
//!   Millim. Waves 17, 1997 (1996), doi:10.1007/BF02069487, Table I), each row with the paper Lamb
//!   takes it from.
//!
//! A loss tangent depends on the sample far more than ε′ does: on the glass's water and
//! impurities, the crystal's growth and poling, a semiconductor's resistivity (its conduction
//! loss, σ/(ωε₀ε′), falls as 1/f). So each value keeps its sample, and a material's values spread.

use serde::Serialize;

use super::references::references;
use super::{Axis, Reference, Source};

/// A material's permittivity and loss tangent at microwave and millimetre-wave frequencies:
/// its measurements and their papers.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RfDielectric {
    /// The id: the catalogue entry's ([`super::Entry::id`]) where the material has one, e.g.
    /// `"sio2"` (fused silica); otherwise `"quartz"` (crystalline) or `"sapphire"`.
    pub id: String,
    /// The name, e.g. `"Silica (fused)"`.
    pub name: String,
    /// The formula, e.g. `"SiO₂"`.
    pub formula: String,
    /// The measurements, by paper and frequency.
    pub values: Vec<RfValue>,
    /// Every paper the values cite.
    pub references: Vec<Reference>,
}

/// One measurement of ε′ and tan δ = ε″/ε′, on one sample.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RfValue {
    /// The field's direction: [`Axis::Isotropic`] for a glass, [`Axis::Ordinary`] across a
    /// crystal's optic axis, [`Axis::Extraordinary`] along it; `None` where the paper doesn't say.
    pub axis: Option<Axis>,
    /// The sample, e.g. `"Spectrosil WF (water-free)"`.
    pub sample: String,
    /// The frequency, GHz: one (both ends equal) or the band the value spans.
    pub frequency_ghz: (f64, f64),
    /// The temperature, K, where the paper states one.
    pub temperature: Option<f64>,
    /// ε′, the real relative permittivity: one value (both ends equal) or the range given, when
    /// given.
    pub permittivity: Option<(f64, f64)>,
    /// tan δ: one value (both ends equal) or the range given.
    pub loss_tangent: (f64, f64),
    /// Where it comes from.
    pub source: Source,
    /// Anything else: how it was measured, what the source it cites is.
    pub notes: String,
}

/// Every material with a permittivity and loss tangent at microwave frequencies.
pub fn rf_dielectrics() -> Vec<RfDielectric> {
    vec![silica(), quartz(), sapphire(), silicon(), lithium_niobate()]
}

/// The material with this id (`"sio2"`, `"quartz"`, `"sapphire"`, `"si"`, `"linbo3"`) or formula
/// as a stack names it (`"SiO2"`, the glass; `"Si"`, `"LiNbO3"`, `"Al2O3"`; subscripts as
/// digits), in any case.
pub fn rf_dielectric(name: &str) -> Option<RfDielectric> {
    let plain = |s: &str| -> String {
        s.chars()
            .map(|c| match c {
                '₀'..='₉' => char::from(b'0' + (c as u32 - '₀' as u32) as u8),
                c => c,
            })
            .collect::<String>()
            .to_lowercase()
    };
    let name = plain(name);
    rf_dielectrics()
        .into_iter()
        .find(|m| m.id == name || plain(&m.formula) == name)
}

/// A value: its field's direction, sample, frequency (GHz), temperature (K), ε′, tan δ, paper and
/// place in it, and notes.
#[allow(clippy::too_many_arguments)]
fn value(
    axis: Option<Axis>,
    sample: &str,
    frequency_ghz: (f64, f64),
    temperature: Option<f64>,
    permittivity: Option<(f64, f64)>,
    loss_tangent: (f64, f64),
    (reference, location): (&str, &str),
    notes: &str,
) -> RfValue {
    RfValue {
        axis,
        sample: sample.into(),
        frequency_ghz,
        temperature,
        permittivity,
        loss_tangent,
        source: Source {
            reference: reference.into(),
            location: location.into(),
        },
        notes: notes.into(),
    }
}

/// One value, as an interval of zero width.
fn one(v: f64) -> (f64, f64) {
    (v, v)
}

/// A row of Lamb's Table I at room temperature: the page, his reference, the frequency (GHz),
/// the temperature (K), ε′ and tan δ × 10⁴ as he prints them.
#[allow(clippy::too_many_arguments)]
fn lamb(
    axis: Axis,
    sample: &str,
    (page, cites): (&str, &str),
    frequency_ghz: (f64, f64),
    temperature: f64,
    permittivity: (f64, f64),
    tan_e4: (f64, f64),
    notes: &str,
) -> RfValue {
    let location = format!("Table I, p. {page}, his ref. {cites}");
    value(
        Some(axis),
        sample,
        frequency_ghz,
        Some(temperature),
        Some(permittivity),
        (tan_e4.0 * 1e-4, tan_e4.1 * 1e-4),
        ("lamb-1996", &location),
        notes,
    )
}

const VON_HIPPEL: &str =
    "24: A. R. von Hippel, Dielectric Materials and Applications (Wiley, 1954)";
const PARSHIN: &str =
    "16: V. V. Parshin, Int. J. Infrared Millim. Waves 15, 339 (1994), gyrotron windows";
const DUTTA: &str = "19: J. M. Dutta, C. R. Jones, H. Dave, IEEE Trans. Microw. Theory Tech. 34, 932 (1986), 245 GHz";
const HARVEY: &str = "32: A. F. Harvey, Microwave Engineering (Academic Press, 1963)";
const JONES: &str = "51: R. G. Jones, IEE Conf. Publ. 129, 141 (1975), open resonators";
const BIRCH: &str = "6: J. R. Birch et al., IEEE Trans. Microw. Theory Tech. 42, 956 (1994), an intercomparison of laboratories";
const AFSAR: &str = "4 and 17: M. N. Afsar, K. J. Button, Proc. IEEE 73, 131 (1985); M. N. Afsar, IEEE Trans. Microw. Theory Tech. 32, 1598 (1984)";
const BRAGINSKY: &str = "53: V. B. Braginsky, V. S. Ilchenko, Phys. Lett. A 120, 300 (1987)";

fn silica() -> RfDielectric {
    let iso = Axis::Isotropic;
    RfDielectric {
        id: "sio2".into(),
        name: "Silica (fused)".into(),
        formula: "SiO₂".into(),
        values: vec![
            lamb(
                iso,
                "Spectrosil",
                ("2007", VON_HIPPEL),
                one(10.0),
                300.0,
                one(3.78),
                one(1.7),
                "",
            ),
            lamb(
                iso,
                "fused silica, grade QI",
                ("2006", PARSHIN),
                one(150.0),
                300.0,
                one(3.8103),
                (5.0, 5.3),
                "Lamb lists three grades of fused silica (QU, QV, QI) from this source",
            ),
            lamb(
                iso,
                "fused silica, grade QV",
                ("2006", PARSHIN),
                one(150.0),
                300.0,
                one(3.8142),
                (5.5, 7.1),
                "",
            ),
            lamb(
                iso,
                "fused silica, grade QU",
                ("2006", PARSHIN),
                one(150.0),
                300.0,
                one(3.8338),
                (14.0, 15.0),
                "",
            ),
            lamb(
                iso,
                "Dynasil 4000",
                ("2007", DUTTA),
                one(245.0),
                300.0,
                one(3.822),
                one(18.0),
                "",
            ),
            lamb(
                iso,
                "Spectrosil WF (water-free)",
                ("2007", DUTTA),
                one(245.0),
                300.0,
                one(3.8087),
                one(8.0),
                "",
            ),
            lamb(
                iso,
                "Spectrosil",
                ("2007", HARVEY),
                (2.0, 300.0),
                300.0,
                one(3.85),
                one(1.0),
                "one value for the whole band",
            ),
        ],
        references: references(&["lamb-1996"]),
    }
}

fn quartz() -> RfDielectric {
    let (o, e) = (Axis::Ordinary, Axis::Extraordinary);
    let birch = "ε′ over 30–900 GHz; tan δ at 300 GHz";
    RfDielectric {
        id: "quartz".into(),
        name: "Quartz (crystalline)".into(),
        formula: "SiO₂".into(),
        values: vec![
            lamb(
                o,
                "crystalline quartz",
                ("2017", JONES),
                one(35.0),
                300.0,
                one(4.43),
                one(0.31),
                "",
            ),
            lamb(
                o,
                "crystalline quartz",
                ("2017", PARSHIN),
                one(140.0),
                300.0,
                one(4.4420),
                one(5.1),
                "",
            ),
            lamb(
                e,
                "crystalline quartz",
                ("2017", PARSHIN),
                one(140.0),
                300.0,
                one(4.6440),
                one(2.4),
                "",
            ),
            lamb(
                o,
                "crystalline quartz",
                ("2017", DUTTA),
                one(245.0),
                300.0,
                one(4.4348),
                one(1.0),
                "",
            ),
            lamb(
                e,
                "crystalline quartz",
                ("2017", DUTTA),
                one(245.0),
                300.0,
                one(4.6367),
                one(1.4),
                "",
            ),
            lamb(
                o,
                "crystalline quartz",
                ("2017", BIRCH),
                one(300.0),
                293.0,
                one(4.435),
                one(2.0),
                birch,
            ),
            lamb(
                e,
                "crystalline quartz",
                ("2017", BIRCH),
                one(300.0),
                293.0,
                one(4.640),
                one(2.0),
                birch,
            ),
        ],
        references: references(&["lamb-1996"]),
    }
}

fn sapphire() -> RfDielectric {
    let (o, e) = (Axis::Ordinary, Axis::Extraordinary);
    let z_cut = "z-cut: the field across the c axis";
    let braginsky = |f: f64, tan: f64| {
        value(
            None,
            "single-crystal sapphire",
            one(f),
            Some(300.0),
            None,
            one(tan * 1e-4),
            (
                "lamb-1996",
                &format!("Table I, p. 2018, his ref. {BRAGINSKY}"),
            ),
            "the field's direction not given; tan δ falls to 2e-10 at 4 K (36 GHz) in the same rows",
        )
    };
    RfDielectric {
        id: "sapphire".into(),
        name: "Sapphire".into(),
        formula: "Al₂O₃".into(),
        values: vec![
            braginsky(36.0, 0.2),
            braginsky(72.0, 0.4),
            lamb(
                o,
                "z-cut sapphire",
                ("2019", AFSAR),
                one(100.0),
                300.0,
                one(9.3879),
                one(4.5),
                z_cut,
            ),
            lamb(
                o,
                "α-sapphire",
                ("2019", PARSHIN),
                one(140.0),
                300.0,
                (9.400, 9.431),
                (2.1, 2.5),
                "",
            ),
            lamb(
                e,
                "α-sapphire",
                ("2019", PARSHIN),
                one(140.0),
                300.0,
                (11.560, 11.594),
                (1.1, 1.4),
                "",
            ),
            lamb(
                o,
                "z-cut sapphire",
                ("2019", AFSAR),
                one(200.0),
                300.0,
                one(9.3854),
                one(6.0),
                z_cut,
            ),
            lamb(
                o,
                "z-cut sapphire",
                ("2019", AFSAR),
                one(300.0),
                300.0,
                one(9.3850),
                one(8.0),
                z_cut,
            ),
        ],
        references: references(&["lamb-1996"]),
    }
}

/// Krupka et al.'s silicon: ε′ read off their Fig. 2 at 300 K (they print none).
pub(crate) const KRUPKA_PERMITTIVITY: f64 = 11.66;

fn silicon() -> RfDielectric {
    let iso = Axis::Isotropic;
    let krupka = |sample: &str, f: f64, tan: f64, notes: &str| {
        value(
            Some(iso),
            sample,
            one(f),
            None,
            Some(one(KRUPKA_PERMITTIVITY)),
            one(tan),
            (
                "krupka-2006",
                "Table I (room temperature); ε′ read off Fig. 2 at 300 K",
            ),
            notes,
        )
    };
    let float_zone = "float-zone high-resistivity p-type (boron) silicon (Topsil)";
    let conduction = "the loss is the holes' conduction (Eq. (2): σ/(ωε₀ε_r), so ∝ 1/f; about \
                      12 kΩ·cm here, 1.25e-4 at 100 GHz); Q-factor-limited, 1–2 % (p. 3997). ε′ \
                      is independent of frequency, to 0.1 % (p. 3997)";
    let afsar = "his ref. 4: M. N. Afsar, K. J. Button, Proc. IEEE 73, 131 (1985)";
    let resistive = "the resistivity not given; tan δ ∝ 1/f, a conduction loss of about 800 Ω·cm";
    RfDielectric {
        id: "si".into(),
        name: "Silicon".into(),
        formula: "Si".into(),
        values: vec![
            krupka(
                &format!("{float_zone}, sample #3 (7.97 mm across, in a BZT ring resonator)"),
                4.824,
                0.002101,
                "quasi-TE₀₁₁ mode of the composite resonator, filling factor 0.0885",
            ),
            krupka(
                &format!("{float_zone}, sample #1 (15.455 mm across, 6.0 mm high)"),
                6.685,
                0.001866,
                conduction,
            ),
            krupka(
                &format!("{float_zone}, sample #1"),
                6.797,
                0.001901,
                "quasi-HE₁₁₁ mode, filling factor 0.4346; 3.5 % above TE₀₁₁'s scaled by 1/f, \
                 the hybrid mode's fields not all tangential to the faces (p. 3998)",
            ),
            lamb(
                iso,
                "silicon",
                ("2019", afsar),
                one(100.0),
                300.0,
                one(11.878),
                one(19.0),
                resistive,
            ),
            lamb(
                iso,
                "silicon",
                ("2019", DUTTA),
                one(245.0),
                300.0,
                one(11.684),
                one(7.6),
                "",
            ),
            lamb(
                iso,
                "silicon",
                ("2019", afsar),
                one(250.0),
                300.0,
                one(11.883),
                one(7.5),
                resistive,
            ),
        ],
        references: references(&["krupka-2006", "lamb-1996"]),
    }
}

/// Lee's Fig. 2 read off the figure at its 29 points, 91 to 147 GHz: (n, tan δ × 10³) of each
/// panel, (a) the field along y (ordinary), (b) along z (extraordinary). The figure is a bitmap
/// (750 × 695 pixels); each point is the curve's centre at its frequency against the axes' ticks
/// (one pixel is 6e-4 in n and 4e-6 in tan δ), and where a dash leaves a gap, the dashes either
/// side carried to it.
pub(crate) const LEE_FIG_2: [[(f64, f64); 29]; 2] = [
    [
        (6.5741, 6.23),
        (6.5767, 5.966),
        (6.5590, 5.955),
        (6.5676, 6.247),
        (6.5499, 6.024),
        (6.5561, 6.243),
        (6.5434, 6.207),
        (6.5358, 6.361),
        (6.5443, 6.197),
        (6.5284, 6.247),
        (6.5434, 6.026),
        (6.5375, 6.391),
        (6.5246, 6.435),
        (6.5151, 6.418),
        (6.5249, 6.376),
        (6.5308, 6.404),
        (6.5222, 6.233),
        (6.5028, 6.596),
        (6.5075, 6.310),
        (6.4986, 6.416),
        (6.4924, 6.337),
        (6.5042, 6.44),
        (6.4777, 6.540),
        (6.4615, 6.495),
        (6.4689, 6.396),
        (6.4703, 6.817),
        (6.4656, 6.576),
        (6.4418, 6.775),
        (6.4474, 6.76),
    ],
    [
        (5.3503, 7.21),
        (5.3525, 7.28),
        (5.3349, 7.33),
        (5.3345, 7.020),
        (5.3223, 7.126),
        (5.3259, 7.205),
        (5.3276, 7.172),
        (5.3166, 7.225),
        (5.3212, 7.429),
        (5.3241, 7.339),
        (5.3287, 7.23),
        (5.3155, 7.313),
        (5.3095, 7.495),
        (5.3116, 7.302),
        (5.3138, 7.415),
        (5.3065, 7.516),
        (5.3109, 7.509),
        (5.3085, 7.485),
        (5.3151, 7.551),
        (5.3025, 7.711),
        (5.3005, 7.513),
        (5.2917, 7.749),
        (5.3027, 7.610),
        (5.2942, 7.495),
        (5.2889, 7.623),
        (5.2787, 7.641),
        (5.2895, 7.971),
        (5.2818, 7.637),
        (5.2869, 7.799),
    ],
];

/// The frequency of Lee's k-th point, GHz: every 2 GHz from 91.
pub(crate) fn lee_frequency(k: usize) -> f64 {
    91.0 + 2.0 * k as f64
}

/// The extremes of a column of Lee's reading: ε′ = n² and tan δ.
fn lee_ranges(panel: &[(f64, f64); 29]) -> ((f64, f64), (f64, f64)) {
    let fold = |f: &dyn Fn(&(f64, f64)) -> f64| {
        panel
            .iter()
            .map(f)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    let (n_lo, n_hi) = fold(&|p| p.0);
    let (t_lo, t_hi) = fold(&|p| p.1 * 1e-3);
    ((n_lo * n_lo, n_hi * n_hi), (t_lo, t_hi))
}

fn lithium_niobate() -> RfDielectric {
    let sample = "congruent, poled along z (Crystal Technology); 25 mm square, 40 and 80 mm long, \
                  the wave normal to an x face";
    let lee = |panel: usize, axis: Axis, notes: &str| {
        let (eps, tan) = lee_ranges(&LEE_FIG_2[panel]);
        value(
            Some(axis),
            sample,
            (lee_frequency(0), lee_frequency(28)),
            None,
            Some(eps),
            tan,
            (
                "lee-2001-linbo3",
                if panel == 0 {
                    "Fig. 2(a), read off the figure (p. 1343); ε′ = n²"
                } else {
                    "Fig. 2(b), read off the figure (p. 1343); ε′ = n²"
                },
            ),
            notes,
        )
    };
    let common = "the range over the band's 29 points, rising with frequency (the paper: by 12–13 % \
                  across it, about as f^0.25, and three times the 10 GHz value of the same \
                  crystals; a least-squares power law through the reading rises 9 to 10 %); \
                  the reflection phase and magnitude of 20 spectra averaged, temperature not \
                  stated";
    RfDielectric {
        id: "linbo3".into(),
        name: "Lithium niobate (congruent)".into(),
        formula: "LiNbO₃".into(),
        values: vec![
            lee(
                0,
                Axis::Ordinary,
                &format!(
                    "field along y: n 6.44 to 6.58, mean 6.516 (ε′ 42.5); tan δ mean 6.36e-3; {common}"
                ),
            ),
            lee(
                1,
                Axis::Extraordinary,
                &format!(
                    "field along z: n 5.28 to 5.35, mean 5.312 (ε′ 28.2); tan δ mean 7.45e-3; {common}"
                ),
            ),
        ],
        references: references(&["lee-2001-linbo3"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_material_is_found_by_id_or_by_formula_and_its_values_hold_together() {
        for name in [
            "SiO2", "sio2", "quartz", "Al2O3", "sapphire", "Si", "LiNbO3", "linbo3",
        ] {
            assert!(rf_dielectric(name).is_some(), "{name}");
        }
        assert_eq!(rf_dielectric("SiO₂").unwrap().id, "sio2");
        assert!(rf_dielectric("TiN").is_none());
        for m in rf_dielectrics() {
            assert!(!m.values.is_empty(), "{}", m.id);
            for v in &m.values {
                assert!(
                    m.references.iter().any(|r| r.key == v.source.reference),
                    "{}: {}",
                    m.id,
                    v.source.reference
                );
                let (lo, hi) = v.loss_tangent;
                assert!(lo > 0.0 && lo <= hi && hi < 0.01, "{}: {lo} {hi}", m.id);
                let (f0, f1) = v.frequency_ghz;
                assert!(f0 > 0.0 && f0 <= f1, "{}", m.id);
                if let Some((e0, e1)) = v.permittivity {
                    assert!(e0 > 1.0 && e0 <= e1, "{}", m.id);
                }
            }
        }
        for id in ["sio2", "si", "linbo3"] {
            let e = super::super::entry(id).unwrap();
            assert_eq!(e.rf_dielectric().unwrap().id, id);
        }
        assert!(
            super::super::entry("gaas")
                .unwrap()
                .rf_dielectric()
                .is_none()
        );
    }

    #[test]
    fn lee_reading() {
        let ln = rf_dielectric("linbo3").unwrap();
        let (o, e) = (&ln.values[0], &ln.values[1]);
        assert_eq!(o.frequency_ghz, (91.0, 147.0));
        assert!(
            (o.loss_tangent.0 - 5.955e-3).abs() < 1e-12
                && (o.loss_tangent.1 - 6.817e-3).abs() < 1e-12
        );
        assert!(
            (e.loss_tangent.0 - 7.020e-3).abs() < 1e-12
                && (e.loss_tangent.1 - 7.971e-3).abs() < 1e-12
        );
        let mean = |panel: usize, f: &dyn Fn(&(f64, f64)) -> f64| {
            LEE_FIG_2[panel].iter().map(f).sum::<f64>() / 29.0
        };
        // the notes' means
        assert!((mean(0, &|p| p.1) - 6.36).abs() < 5e-3);
        assert!((mean(1, &|p| p.1) - 7.445).abs() < 1e-3);
        assert!((mean(0, &|p| p.0) - 6.516).abs() < 5e-4);
        assert!((mean(1, &|p| p.0) - 5.312).abs() < 5e-4);
        assert!((mean(0, &|p| p.0 * p.0) - 42.46).abs() < 5e-3);
        assert!((mean(1, &|p| p.0 * p.0) - 28.22).abs() < 5e-3);
        // a least-squares power law through each panel's tan δ rises 9 to 10 % from 91 to 147 GHz
        for (panel, rise) in [(0, 0.099), (1, 0.090)] {
            let xs: Vec<f64> = (0..29).map(|k| lee_frequency(k).ln()).collect();
            let ys: Vec<f64> = LEE_FIG_2[panel].iter().map(|p| p.1.ln()).collect();
            let (mx, my) = (xs.iter().sum::<f64>() / 29.0, ys.iter().sum::<f64>() / 29.0);
            let slope = xs
                .iter()
                .zip(&ys)
                .map(|(x, y)| (x - mx) * (y - my))
                .sum::<f64>()
                / xs.iter().map(|x| (x - mx) * (x - mx)).sum::<f64>();
            let fitted = (147.0f64 / 91.0).powf(slope) - 1.0;
            assert!((fitted - rise).abs() < 1e-3, "{panel}: {fitted}");
        }
    }

    #[test]
    fn krupka_implied_resistivity_and_scaling() {
        // σ = ωε₀ε′ tan δ at 6.685 GHz: about 12 kΩ·cm; tan δ at 100 GHz by 1/f
        let eps0 = 8.854_187_812_8e-12;
        let sigma = std::f64::consts::TAU * 6.685e9 * eps0 * KRUPKA_PERMITTIVITY * 0.001866;
        let rho_kohm_cm = 1.0 / sigma / 10.0;
        assert!((rho_kohm_cm - 12.36).abs() < 0.01, "{rho_kohm_cm}");
        assert!((0.001866f64 * 6.685 / 100.0 - 1.25e-4).abs() < 1e-6);
        // Lamb's silicon: tan δ × f nearly constant, a conduction loss of about 800 Ω·cm
        let si = rf_dielectric("si").unwrap();
        let lamb: Vec<&RfValue> = si
            .values
            .iter()
            .filter(|v| v.source.reference == "lamb-1996")
            .collect();
        for v in &lamb {
            let sigma = std::f64::consts::TAU
                * v.frequency_ghz.0
                * 1e9
                * eps0
                * v.permittivity.unwrap().0
                * v.loss_tangent.0;
            let rho = 1.0 / sigma * 100.0;
            assert!(rho > 780.0 && rho < 840.0, "{rho}");
        }
    }
}
