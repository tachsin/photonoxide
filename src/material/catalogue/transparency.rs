//! Transparency windows: where a material stops absorbing at short wavelengths (the band edge),
//! where it starts again at long ones (the multiphonon edge), and where two-photon absorption sets
//! in between, each edge with its source or stated as not specified.
//!
//! - **III–V band gaps:** I. Vurgaftman, J. R. Meyer, L. R. Ram-Mohan, J. Appl. Phys. 89, 5815
//!   (2001), doi:10.1063/1.1368156: the Varshni form E_g(T) = E_g(0) − αT²/(T + β) (Eq. (2.13),
//!   p. 5818) with each binary's parameters (Tables I, II, IV, VI and X) and the alloys' bowing
//!   (Tables XII, XV and XXX), E_g(x) = (1 − x)E_g(A) + xE_g(B) − x(1 − x)C. The edge is the
//!   lowest of the Γ, X and L gaps; above the Γ–X crossover the gap is indirect and the edge weak.
//! - **Silicon:** the indirect gap E_gi = 1.12 eV and two-photon absorption vanishing beyond
//!   2200 nm, at E_gi/2 (A. D. Bristow et al., Appl. Phys. Lett. 90, 191104 (2007),
//!   doi:10.1063/1.2737359, pp. 191104-1 and -2).
//! - **Lithium niobate:** "the band edge at around 320 nm and the multiphonon absorption in the
//!   IR, starting at around 3000 nm" (M. Leidinger et al., Opt. Express 23, 21690 (2015),
//!   doi:10.1364/OE.23.021690, p. 21692), undoped congruent crystal.
//! - **Lithium tantalate:** the transparency range Barboza & Cudney state, 0.28 to 5.5 µm
//!   (Appl. Phys. B 95, 453 (2009), p. 453), stated, not measured there.
//! - **Two-photon absorption** sets in where the photon energy passes half the gap, below twice
//!   the band edge's wavelength: detuned below the half band gap "the detrimental effects of
//!   two-photon absorption can be ignored" (J. S. Aitchison et al., IEEE J. Quantum Electron. 33,
//!   341 (1997), doi:10.1109/3.556002, p. 342, for AlGaAs).
//!
//! The multiphonon edges of silicon, silicon nitride, GaAs, AlGaAs, InP, InGaP, AlN and AlGaN,
//! and every edge of silica, silicon nitride, MgO-doped lithium niobate and barium titanate,
//! have no paper in hand: they are not specified.

use serde::Serialize;

use super::entries::src;
use super::{Conditions, Source};

/// One edge of a window.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Edge {
    /// The vacuum wavelength, µm; `None` when no source in hand specifies it.
    pub wavelength: Option<f64>,
    /// How it was found, or why it is not specified.
    pub how: String,
    /// The paper and the place in it.
    pub source: Option<Source>,
}

/// A material's transparency window at given conditions.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Transparency {
    /// The short-wavelength edge: the band gap's, or where absorption is stated to start.
    pub band_edge: Edge,
    /// The long-wavelength edge: the multiphonon absorption's.
    pub multiphonon: Edge,
    /// Below this wavelength two-photon absorption sets in: twice the band edge's.
    pub two_photon: Edge,
    /// The wavelengths (µm) the entry's default index model covers at its default conditions.
    pub index_range: (f64, f64),
}

/// hc in eV µm.
const HC: f64 = 1.239_841_98;

/// A Varshni gap: E_g(0) (eV), α (meV/K), β (K).
type Varshni = (f64, f64, f64);

fn varshni((e0, alpha, beta): Varshni, kelvin: f64) -> f64 {
    e0 - alpha * 1e-3 * kelvin * kelvin / (kelvin + beta)
}

/// Vurgaftman et al.'s binaries: the Γ, X and L gaps' Varshni parameters.
pub(crate) const GAAS: [Varshni; 3] = [
    (1.519, 0.5405, 204.0),
    (1.981, 0.460, 204.0),
    (1.815, 0.605, 204.0),
];
pub(crate) const ALAS: [Varshni; 3] = [
    (3.099, 0.885, 530.0),
    (2.24, 0.70, 530.0),
    (2.46, 0.605, 204.0),
];
pub(crate) const INP: Varshni = (1.4236, 0.363, 162.0);
/// Wurtzite GaN and AlN (Table X).
pub(crate) const GAN: Varshni = (3.507, 0.909, 830.0);
pub(crate) const ALN: Varshni = (6.23, 1.799, 1462.0);
/// GaP's Γ gap, 2.886 + 0.1081[1 − coth(164/T)] eV (Table IV).
fn gap_gamma(kelvin: f64) -> f64 {
    2.886 + 0.1081 * (1.0 - 1.0 / (164.0 / kelvin).tanh())
}
/// The bowing parameters: AlGaAs's Γ (−0.127 + 1.310x), X (0.055) and L (0) (Table XII);
/// GaInP's Γ (0.65, Table XV); wurtzite AlGaN's (1.0, Table XXX).
const GAINP_BOWING: f64 = 0.65;
const ALGAN_BOWING: f64 = 1.0;

/// The Γ, X and L gaps of AlₓGa₁₋ₓAs at `kelvin` (eV).
pub(crate) fn algaas_gaps(x: f64, kelvin: f64) -> [f64; 3] {
    let bowing = [-0.127 + 1.310 * x, 0.055, 0.0];
    std::array::from_fn(|k| {
        (1.0 - x) * varshni(GAAS[k], kelvin) + x * varshni(ALAS[k], kelvin)
            - x * (1.0 - x) * bowing[k]
    })
}

/// Ga₀.₅₁In₀.₄₉P's Γ gap at `kelvin` (eV).
pub(crate) fn gainp_gap(kelvin: f64) -> f64 {
    let x = 0.51;
    x * gap_gamma(kelvin) + (1.0 - x) * varshni(INP, kelvin) - x * (1.0 - x) * GAINP_BOWING
}

/// AlₓGa₁₋ₓN's gap (wurtzite) at `kelvin` (eV).
pub(crate) fn algan_gap(x: f64, kelvin: f64) -> f64 {
    x * varshni(ALN, kelvin) + (1.0 - x) * varshni(GAN, kelvin) - x * (1.0 - x) * ALGAN_BOWING
}

fn not_specified(what: &str) -> Edge {
    Edge {
        wavelength: None,
        how: format!("not specified: {what}"),
        source: None,
    }
}

fn edge(wavelength: f64, how: String, source: Source) -> Edge {
    Edge {
        wavelength: Some(wavelength),
        how,
        source: Some(source),
    }
}

const NO_MULTIPHONON: &str = "no paper in hand gives this material's multiphonon edge";

/// A band edge from a gap (eV), and the two-photon edge twice its wavelength.
fn from_gap(gap: f64, how: String, source: Source) -> (Edge, Edge) {
    let lam = HC / gap;
    (
        edge(lam, format!("{how}: {gap:.4} eV"), source),
        edge(
            2.0 * lam,
            format!(
                "twice the band edge's wavelength, where the photon energy is half the gap, {:.4} eV",
                gap / 2.0
            ),
            src("aitchison-1997", "p. 342"),
        ),
    )
}

/// The window of the catalogue's entry `id` at `conditions` (temperature in K, 300 K when not
/// given; for AlGaAs and AlGaN the Al fraction, which must be given for AlGaN), or `None` for an
/// id the catalogue doesn't have.
pub fn transparency(id: &str, conditions: Conditions) -> Option<Transparency> {
    let entry = super::entry(id)?;
    let index_range = entry
        .default_model()
        .map_or((f64::NAN, f64::NAN), |m| m.wavelength);
    let kelvin = conditions.temperature.unwrap_or(300.0);
    let vurgaftman = |table: &str| src("vurgaftman-2001", table);
    let t = format!("at {kelvin} K");
    let (band_edge, two_photon, multiphonon) = match id {
        "si" => (
            edge(HC / 1.12, "the indirect gap, E_gi = 1.12 eV, as the paper writes it (room temperature)".into(), src("bristow-2007", "p. 191104-1")),
            edge(2.2, "two-photon absorption 'tends to zero for λ > 2200 nm, corresponding to a photon energy of E_gi/2'".into(), src("bristow-2007", "p. 191104-2, Fig. 2")),
            not_specified(NO_MULTIPHONON),
        ),
        "gaas" => {
            let gap = varshni(GAAS[0], kelvin);
            let (b, p) = from_gap(gap, format!("the direct Γ gap {t}, Varshni form"), vurgaftman("Table I (p. 5825), Eq. (2.13)"));
            (b, p, not_specified(NO_MULTIPHONON))
        }
        "algaas" => {
            let x = conditions.composition.unwrap_or(0.0);
            let gaps = algaas_gaps(x, kelvin);
            let (k, gap) = gaps.iter().enumerate().fold((0, f64::MAX), |b, (k, &g)| if g < b.1 { (k, g) } else { b });
            let valley = ["Γ (direct)", "X (indirect)", "L (indirect)"][k];
            let (b, p) = from_gap(gap, format!("x = {x}, the lowest gap {t}, {valley}, Varshni form with the bowing"), vurgaftman("Tables I, II and XII (pp. 5825, 5826, 5838)"));
            (b, p, not_specified(NO_MULTIPHONON))
        }
        "inp" => {
            let (b, p) = from_gap(varshni(INP, kelvin), format!("the direct Γ gap {t}, Varshni form"), vurgaftman("Table VI (p. 5829)"));
            (b, p, not_specified(NO_MULTIPHONON))
        }
        "ingap" if conditions.composition.unwrap_or(0.0) == 0.0 => {
            let (b, p) = from_gap(gainp_gap(kelvin), format!("Ga₀.₅₁In₀.₄₉P, the direct Γ gap {t} of the disordered alloy (CuPt ordering lowers it), Varshni form with the bowing"), vurgaftman("Tables IV, VI and XV (pp. 5828, 5829, 5841)"));
            (b, p, not_specified(NO_MULTIPHONON))
        }
        "aln" => {
            let (b, p) = from_gap(varshni(ALN, kelvin), format!("wurtzite AlN's direct gap {t}, Varshni form"), vurgaftman("Table X (p. 5835)"));
            (b, p, not_specified(NO_MULTIPHONON))
        }
        "algan" => match conditions.composition {
            Some(x) => {
                let (b, p) = from_gap(algan_gap(x, kelvin), format!("x = {x}, wurtzite, the direct gap {t}, Varshni form with the bowing"), vurgaftman("Tables X and XXX (pp. 5835, 5846)"));
                (b, p, not_specified(NO_MULTIPHONON))
            }
            None => (
                not_specified("give the Al fraction as the composition"),
                not_specified("give the Al fraction as the composition"),
                not_specified(NO_MULTIPHONON),
            ),
        },
        "linbo3" => {
            let leidinger = |at: &str| src("leidinger-2015", at);
            (
                edge(0.32, "'the band edge at around 320 nm' of the undoped congruent crystal; their spectrum covers 320 to 6000 nm (p. 21705)".into(), leidinger("p. 21692")),
                edge(0.64, "twice the band edge's wavelength (DeSalvo et al.'s gap, 3.9 eV, puts it at 0.636 µm, and their β at 532 nm is 0.38 cm/GW)".into(), src("aitchison-1997", "p. 342")),
                edge(3.0, "'the multiphonon absorption in the IR, starting at around 3000 nm'; it 'starts to dominate' at around 3300 nm (p. 21699), after the OH⁻ band at 2870 nm".into(), leidinger("p. 21692")),
            )
        }
        "litao3" => (
            edge(0.28, "the transparency range the paper states, 0.28 to 5.5 µm, in its introduction: stated, not measured there".into(), src("barboza-cudney-2009", "p. 453")),
            edge(0.56, "twice the band edge's wavelength".into(), src("aitchison-1997", "p. 342")),
            edge(5.5, "the same statement's long end".into(), src("barboza-cudney-2009", "p. 453")),
        ),
        _ => (
            not_specified("no paper in hand gives this material's band edge"),
            not_specified("no band edge to take it from"),
            not_specified(NO_MULTIPHONON),
        ),
    };
    Some(Transparency {
        band_edge,
        multiphonon,
        two_photon,
        index_range,
    })
}

impl super::Entry {
    /// The material's transparency window at `conditions` ([`transparency`]).
    pub fn transparency(&self, conditions: Conditions) -> Option<Transparency> {
        transparency(&self.id, conditions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edges() {
        let at = |id: &str, x: Option<f64>| {
            transparency(
                id,
                Conditions {
                    temperature: None,
                    composition: x,
                },
            )
            .unwrap()
        };
        // GaAs at 300 K: 1.4225 eV, 0.8716 µm
        let g = at("gaas", None);
        assert!((g.band_edge.wavelength.unwrap() - 0.8716).abs() < 1e-4);
        assert!((g.two_photon.wavelength.unwrap() - 1.7432).abs() < 2e-4);
        assert!(g.multiphonon.wavelength.is_none());
        // AlGaAs turns indirect between x = 0.4 and 0.45
        assert!(at("algaas", Some(0.4)).band_edge.how.contains("Γ"));
        assert!(at("algaas", Some(0.45)).band_edge.how.contains("indirect"));
        assert!((gainp_gap(300.0) - 1.9168).abs() < 1e-4);
        assert!(at("algan", None).band_edge.wavelength.is_none());
        assert!((at("aln", None).band_edge.wavelength.unwrap() - 0.2020).abs() < 1e-4);
        assert!(at("si3n4", None).band_edge.wavelength.is_none());
        assert!(transparency("nothing", Conditions::default()).is_none());
    }

    #[test]
    fn every_edge_names_a_source_its_entry_cites_or_says_not_specified() {
        for e in super::super::catalogue() {
            let x = (e.id == "algan").then_some(0.1);
            let t = e
                .transparency(Conditions {
                    temperature: None,
                    composition: x,
                })
                .unwrap();
            for edge in [&t.band_edge, &t.two_photon, &t.multiphonon] {
                match (&edge.wavelength, &edge.source) {
                    (Some(w), Some(s)) => {
                        assert!(*w > 0.1 && *w < 20.0, "{}: {w}", e.id);
                        assert!(
                            e.reference(&s.reference).is_some(),
                            "{}: {}",
                            e.id,
                            s.reference
                        );
                    }
                    (None, None) => assert!(edge.how.starts_with("not specified"), "{}", e.id),
                    _ => panic!("{}: an edge with a wavelength and no source", e.id),
                }
            }
        }
    }

    #[test]
    fn the_write_ups_table() {
        let at = |id: &str, x: Option<f64>| {
            let t = transparency(
                id,
                Conditions {
                    temperature: Some(300.0),
                    composition: x,
                },
            )
            .unwrap();
            (
                t.band_edge.wavelength.unwrap_or(f64::NAN),
                t.two_photon.wavelength.unwrap_or(f64::NAN),
            )
        };
        for (id, x, edge, two) in [
            ("gaas", None, 0.8716, 1.7432),
            ("algaas", Some(0.2), 0.7221, 1.4442),
            ("algaas", Some(0.4), 0.6328, 1.2655),
            ("inp", None, 0.9164, 1.8329),
            ("ingap", None, 0.6468, 1.2937),
            ("aln", None, 0.2020, 0.4040),
            ("algan", Some(0.1), 0.3430, 0.6860),
            ("algan", Some(0.3), 0.3072, 0.6144),
        ] {
            let (e, t) = at(id, x);
            assert!(
                (e - edge).abs() < 1e-4 && (t - two).abs() < 2e-4,
                "{id} {x:?}: {e} {t}"
            );
        }
        // Vurgaftman et al.'s text puts AlGaAs's Γ–X crossover at x = 0.39 at 300 K (0.38 at low
        // temperature, p. 5838); their Tables I, II and XII put it at 0.435 (0.417): reported,
        // not a case
        let cross = |kelvin: f64| {
            let (mut lo, mut hi) = (0.2, 0.6);
            for _ in 0..60 {
                let m = 0.5 * (lo + hi);
                let g = algaas_gaps(m, kelvin);
                if g[0] < g[1] { lo = m } else { hi = m }
            }
            lo
        };
        assert!((cross(300.0) - 0.4345).abs() < 5e-4 && (cross(0.0) - 0.4167).abs() < 5e-4);
    }
}
