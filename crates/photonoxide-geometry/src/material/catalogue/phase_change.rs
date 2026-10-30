//! Phase-change materials: Sb₂Se₃, Sb₂S₃ and Ge₂Sb₂Te₅ (GST), each state its own entry of the
//! catalogue, and the temperatures and thermal properties that switch them ([`PhaseChange`]).
//!
//! The indices are photonoxide's own fits to the ellipsometry of M. Delaney, I. Zeimpekis, D.
//! Lawson, D. W. Hewak, O. L. Muskens, Adv. Funct. Mater. 30, 2002447 (2020),
//! doi:10.1002/adfm.202002447, as its data set gives it (University of Southampton,
//! doi:10.5258/SOTON/D1247, CC BY 4.0: its Fig. 3, n and k from 194 to 1693 nm): a two-term
//! Sellmeier formula for each transparent state, crystalline Sb₂Se₃'s k beside it from the data
//! at ten wavelengths, and two Lorentz oscillators for each state of GST, fitted by least
//! squares over 1.0 to 1.69 µm (1.2 µm for crystalline Sb₂Se₃, whose k rises below it). The
//! coefficients are ours; the validation cases (`material/pcm-*`) check them against the data's
//! own points. GST also has J. A. Frantz et al.'s table (Opt. Mater. Express 13, 3631 (2023),
//! doi:10.1364/OME.506019, from the refractiveindex.info database, CC0), whose crystalline film
//! is a different one: 6.46 + 0.92i at 1550 nm against Delaney's 7.91 + 1.03i.

use num_complex::Complex64 as c64;
use serde::Serialize;

use super::entries::{amorphous, src, strings, table};
use super::references::references;
use super::{
    Axis, Category, Constant, Crystal, CrystalSystem, Entry, IndexModel, Missing, OpticalClass,
    Reference,
};
use crate::Result;
use crate::material::{Curve, LorentzPole, Material, Model, Provenance, SellmeierTerm, Table};
use crate::units::{Frequency, Wavelength};

/// The wavelengths (µm) the fits to Delaney's data cover: every state but crystalline Sb₂Se₃.
pub(crate) const DELANEY_RANGE: (f64, f64) = (1.0, 1.69);

/// Crystalline Sb₂Se₃'s range (µm): its data's wavelengths nearest 1.2 and 1.69 µm, inside
/// which its k curve lies.
pub(crate) const DELANEY_SB2SE3_C_RANGE: (f64, f64) = (1.201, 1.689);

/// A two-term Sellmeier fit, n² = A + Σ B λ²/(λ² − C²) (λ, C in µm): (A, [(B, C); 2]).
type Sellmeier = (f64, [(f64, f64); 2]);

/// Amorphous Sb₂Se₃: within 8.2e-5 of its 198 points from 1.0 to 1.69 µm.
pub(crate) const SB2SE3_A: Sellmeier = (6.036818, [(-312.6378, 41.32653), (3.791218, 0.5407071)]);
/// Crystalline Sb₂Se₃'s n: within 1.5e-5 of its 143 points from 1.2 to 1.69 µm.
pub(crate) const SB2SE3_C: Sellmeier = (8.418325, [(-8.283237, 7.186923), (6.252792, 0.6484891)]);
/// Amorphous Sb₂S₃: within 7.9e-6 of its 198 points from 1.0 to 1.69 µm.
pub(crate) const SB2S3_A: Sellmeier = (4.238215, [(3.481032, 0.3907105), (169.9692, 26.95118)]);
/// Crystalline Sb₂S₃: within 4.7e-5 of its 198 points from 1.0 to 1.69 µm.
pub(crate) const SB2S3_C: Sellmeier = (6.471128, [(4.070164, 0.494867), (552.9045, 151.1943)]);

/// Crystalline Sb₂Se₃'s k from Delaney's data at ten wavelengths (nm, k), interpolated by a
/// natural spline in λ.
pub(crate) const SB2SE3_C_K: [(f64, f64); 10] = [
    (1200.289429, 0.002427),
    (1299.749268, 0.001061),
    (1351.284058, 0.000726),
    (1399.441528, 0.000523),
    (1451.103516, 0.000377),
    (1499.383911, 0.000284),
    (1551.182007, 0.000213),
    (1599.593628, 0.000166),
    (1648.071655, 0.000132),
    (1689.678711, 0.000109),
];

/// Two Lorentz oscillators, ε = ε∞ + Σ Δε f₀²/(f₀² − f² − i g f) (f, f₀, g in c/µm):
/// (ε∞, [(Δε, f₀, g); 2]).
type Lorentz = (f64, [(f64, f64, f64); 2]);

/// Amorphous GST: within 1.4e-3 in n and 2.3e-3 in k of its 198 points from 1.0 to 1.69 µm.
pub(crate) const GST_A: Lorentz = (
    -0.8774424,
    [
        (-116.0557, 0.9103794, 1.920031),
        (137.5621, 1.786528, 7.029553),
    ],
);
/// Crystalline GST: within 1.7e-3 in n and 2.5e-3 in k of its 198 points from 1.0 to 1.69 µm.
pub(crate) const GST_C: Lorentz = (
    7.546306,
    [
        (169.528, 1.252739, 2.034896),
        (-133.2813, 0.9372895, 1.511564),
    ],
);

/// Frantz et al.'s amorphous GST (µm, n, k), every 15th row of their table from 0.99 to 1.71
/// µm.
pub(crate) const FRANTZ_A: [(f64, f64, f64); 15] = [
    (0.99021, 4.460, 0.644),
    (1.0388, 4.448, 0.542),
    (1.0900, 4.426, 0.445),
    (1.1413, 4.397, 0.358),
    (1.1927, 4.363, 0.283),
    (1.2441, 4.325, 0.217),
    (1.2956, 4.284, 0.161),
    (1.3472, 4.241, 0.113),
    (1.3988, 4.197, 0.075),
    (1.4505, 4.153, 0.045),
    (1.5023, 4.111, 0.023),
    (1.5541, 4.071, 0.008),
    (1.6060, 4.034, 0.001),
    (1.6580, 4.004, 0.0),
    (1.7098, 3.980, 0.0),
];

/// Frantz et al.'s crystalline GST, the same rows.
pub(crate) const FRANTZ_C: [(f64, f64, f64); 15] = [
    (0.99021, 5.851, 2.721),
    (1.0388, 6.005, 2.546),
    (1.0900, 6.139, 2.359),
    (1.1413, 6.248, 2.173),
    (1.1927, 6.334, 1.990),
    (1.2441, 6.398, 1.813),
    (1.2956, 6.444, 1.641),
    (1.3472, 6.474, 1.478),
    (1.3988, 6.489, 1.322),
    (1.4505, 6.491, 1.176),
    (1.5023, 6.481, 1.038),
    (1.5541, 6.463, 0.910),
    (1.6060, 6.435, 0.791),
    (1.6580, 6.401, 0.682),
    (1.7098, 6.360, 0.583),
];

/// Which of a phase-change material's two states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum State {
    /// As deposited, or melted and quenched.
    Amorphous,
    /// Annealed above the crystallization temperature.
    Crystalline,
}

/// A phase-change material: its two states' catalogue entries, the temperatures that switch it
/// and the thermal properties a pulse heating it needs, each with its source. What has no
/// source yet is in [`PhaseChange::missing`].
///
/// A film crystallizes when held above [`PhaseChange::crystallization`] long enough (the
/// kinetics, milliseconds for Sb₂Se₃ by Lawson et al., aren't modelled), and is amorphized
/// when it melts, above [`PhaseChange::melting`], and is quenched faster than it recrystallizes.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PhaseChange {
    /// A short id, e.g. `"sb2se3"`.
    pub id: String,
    /// The name, e.g. `"Antimony selenide"`.
    pub name: String,
    /// The formula.
    pub formula: String,
    /// The amorphous state's entry id.
    pub amorphous: String,
    /// The crystalline state's entry id.
    pub crystalline: String,
    /// The crystallization temperature, K.
    pub crystallization: Option<Constant>,
    /// The melting temperature, K.
    pub melting: Option<Constant>,
    /// The amorphous state's thermal conductivity, W/(m·K).
    pub amorphous_conductivity: Option<Constant>,
    /// The crystalline state's thermal conductivity, W/(m·K).
    pub crystalline_conductivity: Option<Constant>,
    /// The specific heat capacity c_p, J/(kg·K): the crystalline state's, or the one value both
    /// states take when the source gives one (its conditions say which).
    pub specific_heat: Option<Constant>,
    /// The density ρ, kg/m³: the crystalline state's, or the one value both states take.
    pub density: Option<Constant>,
    /// The amorphous state's c_p, J/(kg·K), where a source measures it apart from
    /// [`PhaseChange::specific_heat`].
    pub amorphous_specific_heat: Option<Constant>,
    /// The amorphous state's density, kg/m³, where a source measures it apart from
    /// [`PhaseChange::density`] (an amorphous film is less dense: GST's by 6.4%).
    pub amorphous_density: Option<Constant>,
    /// What has no source yet.
    pub missing: Vec<Missing>,
    /// Every paper it cites.
    pub references: Vec<Reference>,
}

impl PhaseChange {
    /// The heat capacity per volume ρc_p, J/(m³·K), when both are known: of
    /// [`PhaseChange::density`] and [`PhaseChange::specific_heat`], the crystalline state's (or
    /// both states').
    pub fn heat_capacity(&self) -> Option<f64> {
        Some(self.density.as_ref()?.value * self.specific_heat.as_ref()?.value)
    }

    /// A state's density: the amorphous state's own where it is measured, otherwise
    /// [`PhaseChange::density`].
    pub fn density_of(&self, state: State) -> Option<&Constant> {
        match state {
            State::Amorphous => self.amorphous_density.as_ref().or(self.density.as_ref()),
            State::Crystalline => self.density.as_ref(),
        }
    }

    /// A state's specific heat c_p: the amorphous state's own where it is measured, otherwise
    /// [`PhaseChange::specific_heat`].
    pub fn specific_heat_of(&self, state: State) -> Option<&Constant> {
        match state {
            State::Amorphous => self
                .amorphous_specific_heat
                .as_ref()
                .or(self.specific_heat.as_ref()),
            State::Crystalline => self.specific_heat.as_ref(),
        }
    }

    /// A state's heat capacity per volume ρc_p, J/(m³·K), from [`PhaseChange::density_of`] and
    /// [`PhaseChange::specific_heat_of`], when both are known.
    pub fn heat_capacity_of(&self, state: State) -> Option<f64> {
        Some(self.density_of(state)?.value * self.specific_heat_of(state)?.value)
    }

    /// The entry id of a state.
    pub fn entry(&self, state: State) -> &str {
        match state {
            State::Amorphous => &self.amorphous,
            State::Crystalline => &self.crystalline,
        }
    }

    /// A state's thermal conductivity, W/(m·K), when known.
    pub fn conductivity(&self, state: State) -> Option<&Constant> {
        match state {
            State::Amorphous => self.amorphous_conductivity.as_ref(),
            State::Crystalline => self.crystalline_conductivity.as_ref(),
        }
    }

    /// The state whose entry id is `id`, if it is one of this material's.
    pub fn state_of(&self, id: &str) -> Option<State> {
        if id == self.amorphous {
            Some(State::Amorphous)
        } else if id == self.crystalline {
            Some(State::Crystalline)
        } else {
            None
        }
    }
}

/// A partly crystallized film's index at `wavelength`: the linear effective medium of its two
/// states' permittivities, ε = (1 − f) ε_a + f ε_c, with f the crystallized fraction of its
/// volume, as J. A. Frantz et al. fit GST's intermediate states (Opt. Mater. Express 13, 3631
/// (2023), doi:10.1364/OME.506019, Section 4.2: the result insensitive to the choice of medium,
/// Bruggeman's or the linear one).
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a fraction outside [0, 1], or a wavelength outside either
/// material's range.
pub fn partly_crystalline(
    amorphous: &Material,
    crystalline: &Material,
    fraction: f64,
    wavelength: Wavelength,
) -> Result<c64> {
    if !(0.0..=1.0).contains(&fraction) {
        return Err(crate::Error::invalid(
            "crystallized fraction",
            format!("must be between 0 and 1, got {fraction}"),
        ));
    }
    let (a, c) = (
        amorphous.permittivity(wavelength)?,
        crystalline.permittivity(wavelength)?,
    );
    Ok(((1.0 - fraction) * a + fraction * c).sqrt())
}

/// Delaney et al.'s figure of merit (their Eq. (1)): the phase a waveguide's change of
/// effective index between the states gives per dB of its loss, 2π Δn_eff / (λ α), rad/dB,
/// with α the loss in dB/µm in the lossier state.
pub fn figure_of_merit(delta_n_eff: f64, loss_db_per_um: f64, wavelength: Wavelength) -> f64 {
    std::f64::consts::TAU * delta_n_eff / (wavelength.to_um() * loss_db_per_um)
}

/// Every phase-change material of the catalogue.
pub fn phase_changes() -> Vec<PhaseChange> {
    vec![sb2se3(), sb2s3(), gst()]
}

/// The phase-change material with this id (`"sb2se3"`, `"sb2s3"`, `"gst"`).
pub fn phase_change(id: &str) -> Option<PhaseChange> {
    phase_changes().into_iter().find(|p| p.id == id)
}

fn constant(
    symbol: &str,
    name: &str,
    value: f64,
    uncertainty: Option<f64>,
    unit: &str,
    conditions: &str,
    source: (&str, &str),
) -> Constant {
    Constant {
        symbol: symbol.into(),
        name: name.into(),
        value,
        uncertainty,
        unit: unit.into(),
        conditions: conditions.into(),
        source: src(source.0, source.1),
    }
}

fn missing(property: &str, reason: &str) -> Missing {
    Missing {
        property: property.into(),
        reason: reason.into(),
    }
}

/// Sb₂Se₃'s anneals in Delaney's data set (Supporting Fig. S1, `anneal_tests_all.txt`): each
/// anneal temperature (°C) and the film's n at 1551.18 nm after it.
pub(crate) const DELANEY_SB2SE3_ANNEALS: [(f64, f64); 6] = [
    (125.0, 3.333138),
    (150.0, 3.311244),
    (175.0, 3.340207),
    (200.0, 4.249707),
    (225.0, 4.320316),
    (240.0, 4.284534),
];

/// Sb₂S₃'s anneals, the same way.
pub(crate) const DELANEY_SB2S3_ANNEALS: [(f64, f64); 8] = [
    (220.0, 2.881186),
    (230.0, 2.842302),
    (240.0, 2.868777),
    (250.0, 2.878295),
    (260.0, 2.98655),
    (270.0, 3.370546),
    (280.0, 3.374542),
    (290.0, 3.379835),
];

/// The first anneal (K) after which a film's index stands more than half way from its
/// as-deposited value to its last: where it crystallized.
pub(crate) fn crystallized_at(anneals: &[(f64, f64)]) -> f64 {
    let (first, last) = (anneals[0].1, anneals[anneals.len() - 1].1);
    anneals
        .iter()
        .find(|(_, n)| (n - first) / (last - first) > 0.5)
        .map_or(f64::NAN, |(t, _)| t + 273.15)
}

/// Sb₂Se₃'s density, kg/m³: its molar mass over Avogadro's number times Efthimiopoulos et
/// al.'s zero-pressure volume per formula unit, 136.4 Å³.
pub(crate) fn sb2se3_density() -> f64 {
    // Sb 121.760 and Se 78.971 g/mol (IUPAC's standard atomic weights)
    let molar_kg = (2.0 * SB + 3.0 * 78.971) * 1e-3;
    molar_kg / (AVOGADRO * 136.4e-30)
}

/// IUPAC's standard atomic weights (g/mol) of Ge, Sb and Te, and Avogadro's number.
const GE: f64 = 72.630;
const SB: f64 = 121.760;
const TE: f64 = 127.60;
const AVOGADRO: f64 = 6.022_140_76e23;

/// Gurevich et al.'s molar mass of Sb₂S₃, g/mol, and the orthorhombic cell they print (Pbnm, a,
/// b, c in Å, four formula units).
pub(crate) const GUREVICH_MOLAR: f64 = 339.68;
pub(crate) const GUREVICH_CELL: [f64; 3] = [11.20, 11.28, 3.83];

/// Sb₂S₃'s crystal density, kg/m³: four formula units of Gurevich et al.'s molar mass in their
/// cell.
pub(crate) fn sb2s3_density() -> f64 {
    let [a, b, c] = GUREVICH_CELL;
    4.0 * GUREVICH_MOLAR * 1e-3 / (AVOGADRO * a * b * c * 1e-30)
}

/// Kaito et al.'s density of evaporated amorphous Sb₂S₃ films, kg/m³, and its uncertainty: Table
/// 1's 3.42 ± 0.06 g/cm³.
pub(crate) const KAITO_DENSITY: (f64, f64) = (3420.0, 60.0);

/// Kaito et al.'s bulk crystal density (g/cm³) and density deficit (%) beside it, Table 1.
pub(crate) const KAITO_TABLE_1: (f64, f64) = (4.64, 26.0);

/// Gurevich et al.'s C_p(298.15 K) of crystalline Sb₂S₃, J/(mol·K), and its uncertainty.
pub(crate) const GUREVICH_CP_298: (f64, f64) = (119.54, 0.06);

/// Zhao et al.'s specific heat of amorphous GST at 100 °C, J/(mol-atom·K), with its uncertainty,
/// and the drop on crystallization, 17 ± 5 %.
pub(crate) const ZHAO_CP: (f64, f64) = (27.2, 3.3);
pub(crate) const ZHAO_DROP: (f64, f64) = (0.17, 0.05);

/// Ge₂Sb₂Te₅'s mean atomic mass, kg per mole of atoms.
pub(crate) fn gst_atomic_mass() -> f64 {
    (2.0 * GE + 2.0 * SB + 5.0 * TE) / 9.0 * 1e-3
}

/// Njoroge et al.'s densities by x-ray reflectivity, kg/m³: amorphous, NaCl-type (fcc),
/// hexagonal, each ± 20.
pub(crate) const NJOROGE_DENSITY: [f64; 3] = [5870.0, 6270.0, 6390.0];

/// The density of an fcc Ge₂Sb₂.₀₄Te₄.₇₄ crystal (kg/m³) of lattice parameter `a` (Å), as
/// Njoroge et al. find it from their film's stoichiometry: Te on the 4 anion sites of the cell,
/// Ge and Sb in proportion on the cation sites (the rest vacancies).
pub(crate) fn njoroge_xrd_density(a: f64) -> f64 {
    let per_te = 4.0 / 4.74;
    let mass = per_te * (2.0 * GE + 2.04 * SB + 4.74 * TE) * 1e-3;
    mass / (AVOGADRO * (a * 1e-10).powi(3))
}

const DELANEY_LOCATION: &str = "the data set's 3_ellipsometry (the paper's Fig. 3)";

/// A thermal conductivity Aryana et al. measured by time-domain thermoreflectance at room
/// temperature on an RF-sputtered film of nominally 300 nm under a Pt/Ti transducer: an
/// effective value, the interfaces included (the intrinsic one may be slightly higher, theirs).
fn aryana_kappa(value: f64, state: &str) -> Constant {
    constant(
        r"\kappa",
        "thermal conductivity",
        value,
        None,
        "W/(m·K)",
        &format!(
            "{state}; time-domain thermoreflectance at room temperature on an RF-sputtered film of nominally 300 nm under a Pt/Ti transducer, an effective value with its interfaces"
        ),
        ("aryana-2023", "Section 4, Fig. 4(c) and Table 1"),
    )
}

fn sb2se3() -> PhaseChange {
    PhaseChange {
        id: "sb2se3".into(),
        name: "Antimony selenide".into(),
        formula: "Sb₂Se₃".into(),
        amorphous: "sb2se3-amorphous".into(),
        crystalline: "sb2se3-crystalline".into(),
        crystallization: Some(constant(
            "T_c",
            "crystallization temperature",
            473.15,
            Some(25.0),
            "K",
            "200 °C, a sputtered film annealed for 5 min in steps of 25 °C: amorphous after 175 °C, crystalline after 200 °C (n at 1551 nm 3.340 then 4.250, the data set's S1_ellipsometry); Lawson et al.'s model takes 473 K; Černošková et al.'s evaporated film by DSC at 10 °C/min: crystal growth from 161 °C, its peak at 197 °C",
            ("delaney-2020", "Section 3 and Supporting Fig. S1"),
        )),
        melting: Some(constant(
            "T_m",
            "melting temperature",
            884.0,
            None,
            "K",
            "the value Lawson et al.'s photothermal model takes (880 K in its text), without a source",
            ("lawson-2022", "Fig. 2's caption"),
        )),
        amorphous_conductivity: Some(aryana_kappa(
            0.22,
            "amorphous, as deposited (346 nm); Lawson et al.'s model takes 1 W/(m·K) for both states from Li et al. 2019, a measurement on zone-melted Sb₂Te₃₋ₓSeₓ, not Sb₂Se₃",
        )),
        crystalline_conductivity: Some(aryana_kappa(
            0.72,
            "crystalline (312 nm), back at room temperature after heating to 320 °C; 0.36 W/(m·K) while hot, from 220 to 320 °C",
        )),
        specific_heat: Some(constant(
            "c_p",
            "specific heat capacity",
            507.0,
            None,
            "J/(kg·K)",
            "a 900 nm evaporated amorphous film by StepScan DSC: 0.507 J/(g·K) at 105 °C rising to 0.574 at 260 °C, with no drop through crystallization (the amorphous film's c_p close to the crystal's). Lawson et al.'s model takes 0.15 J/(g·K), this paper's estimate of the c_p change at a glass transition, not c_p",
            (
                "cernoskova-2014",
                "Section 'Results and discussion', Fig. 4",
            ),
        )),
        density: Some(constant(
            r"\rho",
            "density",
            sb2se3_density(),
            None,
            "kg/m³",
            "the orthorhombic (Pnma) crystal at ambient pressure: M/(N_A V₀/Z) with V₀/Z = 136.4 Å³; an amorphous film is less dense",
            (
                "efthimiopoulos-2013",
                "Results, the zero-pressure volume per formula unit",
            ),
        )),
        amorphous_specific_heat: None,
        amorphous_density: None,
        missing: vec![missing(
            "the amorphous state's ρ",
            "no measurement found: the crystal's is taken for both states until one is (an X-ray reflectivity of as-deposited and crystallized films is what's wanted), coming",
        )],
        references: references(&[
            "delaney-2020",
            "delaney-2020-data",
            "lawson-2022",
            "cernoskova-2014",
            "efthimiopoulos-2013",
            "aryana-2023",
        ]),
    }
}

fn sb2s3() -> PhaseChange {
    PhaseChange {
        id: "sb2s3".into(),
        name: "Antimony sulfide".into(),
        formula: "Sb₂S₃".into(),
        amorphous: "sb2s3-amorphous".into(),
        crystalline: "sb2s3-crystalline".into(),
        crystallization: Some(constant(
            "T_c",
            "crystallization temperature",
            543.15,
            Some(10.0),
            "K",
            "270 °C, a sputtered film annealed in steps of 10 °C: amorphous to 250 °C, partly crystalline after 260 °C (n at 1551 nm 2.987, a fifth of the way), crystalline after 270 °C (3.371; the data set's S1_ellipsometry)",
            ("delaney-2020", "Section 3 and Supporting Fig. S1"),
        )),
        melting: Some(constant(
            "T_m",
            "melting temperature",
            823.0,
            Some(5.0),
            "K",
            "synthetic polycrystalline stibnite by differential thermal analysis, its one thermal event; the literature's 819 to 832.5 K (their refs. 10–14). Kassem et al. 2023 mark the bulk glass's melting on their figures without printing a number",
            ("gurevich-2002", "Section 1"),
        )),
        amorphous_conductivity: Some(aryana_kappa(0.21, "amorphous, as deposited (275 nm)")),
        crystalline_conductivity: Some(aryana_kappa(
            0.74,
            "crystalline (241 nm), back at room temperature after heating to 320 °C; 0.45 W/(m·K) at 250 to 320 °C",
        )),
        specific_heat: Some(constant(
            "c_p",
            "specific heat capacity",
            GUREVICH_CP_298.0 / (GUREVICH_MOLAR * 1e-3),
            Some(GUREVICH_CP_298.1 / (GUREVICH_MOLAR * 1e-3)),
            "J/(kg·K)",
            "crystalline (stibnite, a synthetic polycrystalline powder) at 298.15 K by adiabatic calorimetry: 119.54 ± 0.06 J/(mol·K) over M = 339.68 g/mol. It rises to 121.9 J/(mol·K) at 338 K and 133.5 at the melting point, 823 K (their Eq. (7), Table 3: 359 to 393 J/(kg·K)); Aryana et al. 2023's 368 J/(kg·K) is Table 3's value at 430 K. The amorphous film's isn't measured",
            (
                "gurevich-2002",
                "Section 2, Tables 2 and 3, Eqs. (1) and (7)",
            ),
        )),
        density: Some(constant(
            r"\rho",
            "density",
            sb2s3_density(),
            None,
            "kg/m³",
            "the orthorhombic crystal (Pbnm, a = 11.20, b = 11.28, c = 3.83 Å, four formula units) as Gurevich et al. print it from their ref. 15, with their M = 339.68 g/mol; an amorphous film is less dense",
            ("gurevich-2002", "Section 1, the cell"),
        )),
        amorphous_specific_heat: None,
        amorphous_density: Some(constant(
            r"\rho_a",
            "density, amorphous",
            KAITO_DENSITY.0,
            Some(KAITO_DENSITY.1),
            "kg/m³",
            "vacuum-evaporated films (the source at about 800 °C, 2e-3 Pa) on KCl at room temperature, stripped free onto holey carbon, amorphous by their halo rings: 3.42 ± 0.06 g/cm³ against the bulk crystal's 4.64, 26 % less, which the authors put down to voids between micro-crystallites. Measured as in their ref. 8 (Shiojiri et al., Jpn. J. Appl. Phys. 16, 1481 (1977)), a method this paper doesn't describe, on films of unstated thickness (those imaged are 8 to 28 nm). It disagrees, unexplained, with Kepič et al. 2024: their pulsed-laser-deposited films thin by about 12 % on crystallizing (250 °C for 5 min; Section 2, p. 4, the PDF's p. 5), which with the area held and no mass lost is about 0.88 of the crystal's density, against Kaito et al.'s 0.74",
            (
                "kaito-1984",
                "Table 1 (p. 160), Sections 2 and 3 (pp. 156, 159)",
            ),
        )),
        missing: vec![missing(
            "the amorphous state's c_p",
            "not measured in the papers in hand (Gurevich et al. measure the crystal); none found yet, coming",
        )],
        references: references(&[
            "delaney-2020",
            "delaney-2020-data",
            "aryana-2023",
            "gurevich-2002",
            "kassem-2023",
            "kaito-1984",
            "kepic-2024",
        ]),
    }
}

fn gst() -> PhaseChange {
    let lyeo = |value: f64, state: &str| {
        constant(
            r"\kappa",
            "thermal conductivity",
            value,
            None,
            "W/(m·K)",
            state,
            (
                "lyeo-2006",
                "Fig. 1 and its text (time-domain thermoreflectance at 27 °C)",
            ),
        )
    };
    PhaseChange {
        id: "gst".into(),
        name: "Germanium antimony telluride (GST-225)".into(),
        formula: "Ge₂Sb₂Te₅".into(),
        amorphous: "gst-amorphous".into(),
        crystalline: "gst-crystalline".into(),
        crystallization: Some(constant(
            "T_c",
            "crystallization temperature",
            423.15,
            None,
            "K",
            "about 150 °C, amorphous to rock salt (Fm-3m), a sputtered film heated at 1.6 K/min, by its resistance; rock salt to hexagonal at about 350 °C; a ZnS–SiO₂ cap raises the first",
            ("friedrich-2000", "Fig. 1 and its text"),
        )),
        melting: Some(constant(
            "T_m",
            "melting temperature",
            621.0 + 273.15,
            None,
            "K",
            "621 °C, the melting peak of a sputtered film (about 7 µm, peeled off) by DSC at 5 K/min; solidus 610 °C, liquidus 638 °C (melting over a range: off-eutectic); heat of fusion 14.7 ± 0.7 kJ/mol, entropy of fusion 16.5 ± 0.9 J/(mol·K). Zhao et al. 2022's nanocalorimetry of 10 to 40 nm films: 627 ± 15 °C, 16.7 ± 3.4 kJ/mol",
            ("kalb-2003", "Tables I and II, Fig. 3(b)"),
        )),
        amorphous_conductivity: Some(lyeo(0.19, "amorphous, a sputtered film as deposited")),
        crystalline_conductivity: Some(lyeo(
            0.57,
            "the cubic (rock-salt) phase, annealed 20 min at 180 °C; 1.58 W/(m·K) for the hexagonal (360 °C); Aryana et al. 2023 measure 0.20 amorphous and 1.02 after 320 °C (TDTR, 300 nm films)",
        )),
        specific_heat: Some(constant(
            "c_p",
            "specific heat capacity",
            ZHAO_CP.0 * (1.0 - ZHAO_DROP.0) / gst_atomic_mass(),
            None,
            "J/(kg·K)",
            "crystalline, at 100 °C: the amorphous film's 27.2 ± 3.3 J/(mol·K) per mole of atoms less the 17 ± 5 % drop on crystallization (the rescan of the crystallized film), over GST's mean atomic mass, 114.09 g/mol; nanocalorimetry of 10 to 40 nm sputtered films under 3 nm of SiOx, from the slope of C_p against the thickness. Aryana et al. 2023's 221/223 J/(kg·K) cite this paper and Aryana et al. 2021 (their refs. 43–44); the latter assumes 1.3 and 1.4 MJ/(m³·K) for amorphous and hexagonal GST (from Zalden et al. 2014) and measures none",
            ("zhao-2022", "Section 2, Figs. 2(b) and 3(a)"),
        )),
        density: Some(constant(
            r"\rho",
            "density",
            NJOROGE_DENSITY[1],
            Some(20.0),
            "kg/m³",
            "the NaCl-type (rock-salt, fcc) phase, Ge₂Sb₂.₀₄Te₄.₇₄ films sputtered at room temperature and annealed 10 min at 150 °C, by x-ray reflectivity (6.27 ± 0.02 g/cm³ from about 130 °C); the hexagonal phase, from about 280 °C, 6390 ± 20; the XRD densities of the stoichiometry and lattice 6430 to 6480 (nearly void-free films)",
            ("njoroge-2002", "Results and Discussion, Fig. 5"),
        )),
        amorphous_specific_heat: Some(constant(
            "c_p",
            "specific heat capacity, amorphous",
            ZHAO_CP.0 / gst_atomic_mass(),
            Some(ZHAO_CP.1 / gst_atomic_mass()),
            "J/(kg·K)",
            "the as-deposited amorphous film at 100 °C, 27.2 ± 3.3 J/(mol·K) per mole of atoms by nanocalorimetry, over GST's mean atomic mass; 'consistent with conventional DSC on bulk samples'",
            ("zhao-2022", "Section 2, Fig. 3(a)"),
        )),
        amorphous_density: Some(constant(
            r"\rho",
            "density, amorphous",
            NJOROGE_DENSITY[0],
            Some(20.0),
            "kg/m³",
            "as deposited, by x-ray reflectivity, constant to 120 °C (5.87 ± 0.02 g/cm³): 6.8 ± 0.2 % below the NaCl-type phase, the film 6.5 % thicker",
            ("njoroge-2002", "Results and Discussion, Fig. 5"),
        )),
        missing: vec![],
        references: references(&[
            "delaney-2020",
            "delaney-2020-data",
            "frantz-2023",
            "friedrich-2000",
            "lyeo-2006",
            "aryana-2023",
            "kalb-2003",
            "zhao-2022",
            "njoroge-2002",
        ]),
    }
}

// ---------------------------------------------------------------------------------------------
// The entries: one per state
// ---------------------------------------------------------------------------------------------

fn um(value: f64) -> Result<Wavelength> {
    Wavelength::um(value)
}

fn delaney_provenance(notes: &str) -> Provenance {
    Provenance {
        reference: "M. Delaney, I. Zeimpekis, D. Lawson, D. W. Hewak, O. L. Muskens, Adv. Funct. Mater. 30, 2002447 (2020)".into(),
        doi: "10.1002/adfm.202002447".into(),
        data: "photonoxide's own least-squares fit to the paper's ellipsometry, its data set doi:10.5258/SOTON/D1247 (University of Southampton, CC BY 4.0)".into(),
        temperature: None,
        notes: notes.into(),
    }
}

/// A Sellmeier fit's material over `range` µm, with `k` against λ (nm) when given.
fn sellmeier_material(
    name: &str,
    fit: Sellmeier,
    k: Option<&[(f64, f64)]>,
    range: (f64, f64),
    notes: &str,
) -> Result<Material> {
    let n = Model::Sellmeier {
        a: fit.0,
        terms: fit.1.iter().map(|&(b, c)| SellmeierTerm { b, c }).collect(),
    };
    let model = match k {
        None => n,
        Some(rows) => Model::WithExtinction {
            n: Box::new(n),
            k: Curve::new(
                rows.iter().map(|r| r.0 * 1e-3).collect(),
                rows.iter().map(|r| r.1).collect(),
            )?,
        },
    };
    Material::new(
        name,
        model,
        um(range.0)?,
        um(range.1)?,
        delaney_provenance(notes),
    )
}

fn lorentz_material(name: &str, fit: Lorentz, notes: &str) -> Result<Material> {
    let poles = fit
        .1
        .iter()
        .map(|&(strength, f0, damping)| {
            Ok(LorentzPole {
                strength,
                resonance: Frequency::natural(f0)?,
                damping,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Material::new(
        name,
        Model::Lorentz {
            eps_inf: fit.0,
            poles,
        },
        um(DELANEY_RANGE.0)?,
        um(DELANEY_RANGE.1)?,
        delaney_provenance(notes),
    )
}

fn frantz_material(name: &str, rows: &[(f64, f64, f64)], notes: &str) -> Result<Material> {
    let w: Vec<f64> = rows.iter().map(|r| r.0).collect();
    let (lo, hi) = (w[0], w[w.len() - 1]);
    Material::new(
        name,
        Model::Tabulated(Table::new(
            w,
            rows.iter().map(|r| r.1).collect(),
            Some(rows.iter().map(|r| r.2).collect()),
        )?),
        um(lo)?,
        um(hi)?,
        Provenance {
            reference: "J. A. Frantz, J. D. Myers, A. Clabeau, R. Y. Bekele, N. Hong, M. A. Vincenti, M. Gandolfi, J. S. Sanghera, Opt. Mater. Express 13, 3631 (2023)".into(),
            doi: "10.1364/OME.506019".into(),
            data: "the refractiveindex.info database (CC0), from the paper's supplemental material: every 15th row from 0.99 to 1.71 µm".into(),
            temperature: None,
            notes: notes.into(),
        },
    )
}

pub(crate) fn sb2se3_amorphous() -> Result<Material> {
    sellmeier_material(
        "Sb2Se3 (amorphous)",
        SB2SE3_A,
        None,
        DELANEY_RANGE,
        "an as-deposited sputtered film, room temperature; k = 0 in the data over the range",
    )
}

pub(crate) fn sb2se3_crystalline() -> Result<Material> {
    sellmeier_material(
        "Sb2Se3 (crystalline)",
        SB2SE3_C,
        Some(&SB2SE3_C_K),
        DELANEY_SB2SE3_C_RANGE,
        "a sputtered film annealed above its crystallization temperature, room temperature; k from the data (2.1e-4 at 1551 nm)",
    )
}

pub(crate) fn sb2s3_amorphous() -> Result<Material> {
    sellmeier_material(
        "Sb2S3 (amorphous)",
        SB2S3_A,
        None,
        DELANEY_RANGE,
        "an as-deposited sputtered film, room temperature; k = 0 in the data over the range",
    )
}

pub(crate) fn sb2s3_crystalline() -> Result<Material> {
    sellmeier_material(
        "Sb2S3 (crystalline)",
        SB2S3_C,
        None,
        DELANEY_RANGE,
        "a sputtered film annealed above its crystallization temperature, room temperature; k = 0 in the data over the range",
    )
}

pub(crate) fn gst_amorphous() -> Result<Material> {
    lorentz_material(
        "Ge2Sb2Te5 (amorphous)",
        GST_A,
        "an as-deposited sputtered film, room temperature; the oscillators are a fit over the range only (two have negative strengths), not a physical model outside it",
    )
}

pub(crate) fn gst_crystalline() -> Result<Material> {
    lorentz_material(
        "Ge2Sb2Te5 (crystalline)",
        GST_C,
        "an annealed sputtered film, room temperature; the oscillators are a fit over the range only (one has a negative strength), not a physical model outside it",
    )
}

pub(crate) fn frantz_amorphous() -> Result<Material> {
    frantz_material(
        "Ge2Sb2Te5 (amorphous)",
        &FRANTZ_A,
        "an as-deposited film, room temperature, by ellipsometry",
    )
}

pub(crate) fn frantz_crystalline() -> Result<Material> {
    frantz_material(
        "Ge2Sb2Te5 (crystalline)",
        &FRANTZ_C,
        "an annealed film, room temperature, by ellipsometry",
    )
}

fn sellmeier_rows(fit: Sellmeier) -> Vec<Vec<String>> {
    let mut rows = vec![strings(&["A", &format!("{}", fit.0), ""])];
    for (j, (b, c)) in fit.1.iter().enumerate() {
        rows.push(vec![
            format!("term {}", j + 1),
            format!("{b}"),
            format!("{c}"),
        ]);
    }
    rows
}

fn lorentz_rows(fit: Lorentz) -> Vec<Vec<String>> {
    let mut rows = vec![strings(&[
        r"$\varepsilon_\infty$",
        &format!("{}", fit.0),
        "",
        "",
    ])];
    for (j, (s, f0, g)) in fit.1.iter().enumerate() {
        rows.push(vec![
            format!("pole {}", j + 1),
            format!("{s}"),
            format!("{f0}"),
            format!("{g}"),
        ]);
    }
    rows
}

const SELLMEIER_EQUATION: &str =
    r"n^2 = A + \sum_{j=1}^{2} \frac{B_j\,\lambda^2}{\lambda^2 - C_j^2}";
const LORENTZ_EQUATION: &str = r"\varepsilon = \varepsilon_\infty + \sum_{j=1}^{2} \frac{\Delta\varepsilon_j\, f_j^2}{f_j^2 - f^2 - i g_j f}, \quad \tilde n = \sqrt{\varepsilon}";

#[allow(clippy::too_many_arguments)]
fn delaney_model(
    equation: &str,
    symbols: &str,
    caption: &str,
    columns: &[&str],
    rows: Vec<Vec<String>>,
    wavelength: (f64, f64),
    accuracy: &str,
    notes: &str,
    eval: fn(f64, f64) -> Result<Vec<Material>>,
) -> IndexModel {
    IndexModel {
        id: "delaney-2020-fit".into(),
        name: "Delaney et al. 2020 (our fit)".into(),
        axes: vec![Axis::Isotropic],
        equation: equation.into(),
        symbols: symbols.into(),
        coefficients: vec![table(caption, columns, rows)],
        wavelength,
        temperature: None,
        composition: None,
        accuracy: accuracy.into(),
        sources: vec![src("delaney-2020-data", DELANEY_LOCATION)],
        default: true,
        notes: notes.into(),
        eval,
        dndt: None,
    }
}

fn sellmeier_model(
    fit: Sellmeier,
    wavelength: (f64, f64),
    accuracy: &str,
    notes: &str,
    eval: fn(f64, f64) -> Result<Vec<Material>>,
) -> IndexModel {
    delaney_model(
        SELLMEIER_EQUATION,
        r"$\lambda$ and $C_j$ in µm; a fit over the range only, its long-wavelength term standing for the data's own curvature there",
        "photonoxide's least-squares fit to the data set's n",
        &["", "$B_j$ (or $A$)", r"$C_j$ (µm)"],
        sellmeier_rows(fit),
        wavelength,
        accuracy,
        notes,
        eval,
    )
}

fn lorentz_model(
    fit: Lorentz,
    accuracy: &str,
    notes: &str,
    eval: fn(f64, f64) -> Result<Vec<Material>>,
) -> IndexModel {
    delaney_model(
        LORENTZ_EQUATION,
        r"$f = 1/\lambda$, $f_j$ and $g_j$ in c/µm; a fit over the range only",
        "photonoxide's least-squares fit to the data set's n and k",
        &[
            "",
            r"$\Delta\varepsilon_j$ (or $\varepsilon_\infty$)",
            r"$f_j$ (c/µm)",
            r"$g_j$ (c/µm)",
        ],
        lorentz_rows(fit),
        DELANEY_RANGE,
        accuracy,
        notes,
        eval,
    )
}

fn frantz_model(
    rows: &[(f64, f64, f64)],
    eval: fn(f64, f64) -> Result<Vec<Material>>,
) -> IndexModel {
    IndexModel {
        id: "frantz-2023".into(),
        name: "Frantz et al. 2023".into(),
        axes: vec![Axis::Isotropic],
        equation: r"\tilde n(\lambda) = n + ik \text{ from the table, through natural cubic splines in } \lambda".into(),
        symbols: r"$\lambda$ in µm".into(),
        coefficients: vec![table(
            "the supplemental material's table (refractiveindex.info, CC0), every 15th row from 0.99 to 1.71 µm",
            &["λ (µm)", "$n$", "$k$"],
            rows.iter()
                .map(|r| vec![format!("{}", r.0), format!("{:.3}", r.1), format!("{:.3}", r.2)])
                .collect(),
        )],
        wavelength: (rows[0].0, rows[rows.len() - 1].0),
        temperature: None,
        composition: None,
        accuracy: "ellipsometry; the paper's stated uncertainty not yet read".into(),
        sources: vec![src("frantz-2023", "Supplement 1, through refractiveindex.info")],
        default: false,
        notes: "a different film from Delaney's: its crystalline index at 1550 nm is 6.46 + 0.92i against Delaney's 7.91 + 1.03i, so a GST device's numbers depend on whose film it is".into(),
        eval,
        dndt: None,
    }
}

fn polycrystalline(
    system: CrystalSystem,
    point_group: &str,
    space_group: &str,
    structure: &str,
    notes: &str,
) -> Crystal {
    Crystal {
        system,
        point_group: point_group.into(),
        space_group: Some(space_group.into()),
        structure: structure.into(),
        optical: OpticalClass::Isotropic,
        centrosymmetric: true,
        notes: notes.into(),
    }
}

fn state(
    id: &str,
    name: &str,
    formula: &str,
    summary: &str,
    crystal: Crystal,
    index: Vec<IndexModel>,
    keys: &[&str],
) -> Entry {
    Entry {
        id: id.into(),
        name: name.into(),
        formula: formula.into(),
        category: Category::PhaseChange,
        summary: summary.into(),
        crystal,
        index,
        tensors: vec![],
        constants: vec![],
        missing: vec![],
        references: references(keys),
    }
}

const SB_GRAINS: &str = "a polycrystalline film of orthorhombic (stibnite-type) grains: their three indices average to the one the ellipsometry fits";

/// The six entries, amorphous and crystalline for each material.
pub(super) fn entries() -> Vec<Entry> {
    let delaney = &["delaney-2020", "delaney-2020-data"];
    let gst_keys = &["delaney-2020", "delaney-2020-data", "frantz-2023"];
    vec![
        state(
            "sb2se3-amorphous",
            "Antimony selenide (amorphous)",
            "Sb₂Se₃",
            "A low-loss phase-change material, as deposited or melt-quenched: transparent in the telecom bands, 0.76 below its crystalline index at 1550 nm.",
            amorphous("a sputtered film, as deposited"),
            vec![sellmeier_model(
                SB2SE3_A,
                DELANEY_RANGE,
                "within 8.2e-5 of the data's 198 points from 1.0 to 1.69 µm; the paper's ellipsometric uncertainty not yet read",
                "a 40 nm sputtered film under 200 nm of ZnS:SiO₂ (the Experimental Section), room temperature; k = 0 in the data from 1.0 µm",
                |_, _| Ok(vec![sb2se3_amorphous()?]),
            )],
            delaney,
        ),
        state(
            "sb2se3-crystalline",
            "Antimony selenide (crystalline)",
            "Sb₂Se₃",
            "A low-loss phase-change material, annealed: 4.05 at 1550 nm, with a small k that rises toward its band edge.",
            polycrystalline(
                CrystalSystem::Orthorhombic,
                "mmm",
                "Pnma (No. 62)",
                "stibnite-type (U₂S₃)",
                SB_GRAINS,
            ),
            vec![sellmeier_model(
                SB2SE3_C,
                DELANEY_SB2SE3_C_RANGE,
                "n within 1.5e-5 of the data's 143 points from 1.2 to 1.69 µm; k the data's at ten wavelengths, splined",
                "a 40 nm sputtered film under 200 nm of ZnS:SiO₂ (the Experimental Section), room temperature; the data's k is 2.1e-4 at 1551 nm, above the abstract's k < 1e-5",
                |_, _| Ok(vec![sb2se3_crystalline()?]),
            )],
            delaney,
        ),
        state(
            "sb2s3-amorphous",
            "Antimony sulfide (amorphous)",
            "Sb₂S₃",
            "A low-loss, wide-gap phase-change material, as deposited or melt-quenched: transparent from the visible's red end through the telecom bands.",
            amorphous("a sputtered film, as deposited"),
            vec![sellmeier_model(
                SB2S3_A,
                DELANEY_RANGE,
                "within 7.9e-6 of the data's 198 points from 1.0 to 1.69 µm; the paper's ellipsometric uncertainty not yet read",
                "a 40 nm sputtered film under 200 nm of ZnS:SiO₂ (the Experimental Section), room temperature; k = 0 in the data over the range",
                |_, _| Ok(vec![sb2s3_amorphous()?]),
            )],
            delaney,
        ),
        state(
            "sb2s3-crystalline",
            "Antimony sulfide (crystalline)",
            "Sb₂S₃",
            "A low-loss, wide-gap phase-change material, annealed: 0.59 above its amorphous index at 1550 nm, lossless in the data.",
            polycrystalline(
                CrystalSystem::Orthorhombic,
                "mmm",
                "Pnma (No. 62)",
                "stibnite",
                SB_GRAINS,
            ),
            vec![sellmeier_model(
                SB2S3_C,
                DELANEY_RANGE,
                "within 4.7e-5 of the data's 198 points from 1.0 to 1.69 µm; the paper's ellipsometric uncertainty not yet read",
                "a 40 nm sputtered film under 200 nm of ZnS:SiO₂ (the Experimental Section), room temperature; k = 0 in the data over the range",
                |_, _| Ok(vec![sb2s3_crystalline()?]),
            )],
            delaney,
        ),
        state(
            "gst-amorphous",
            "Germanium antimony telluride, GST-225 (amorphous)",
            "Ge₂Sb₂Te₅",
            "The standard phase-change material, as deposited or melt-quenched: a large contrast, and absorbing in the telecom bands.",
            amorphous("a sputtered film, as deposited"),
            vec![
                lorentz_model(
                    GST_A,
                    "within 1.4e-3 in n and 2.3e-3 in k of the data's 198 points from 1.0 to 1.69 µm",
                    "the paper's own sputtered film, room temperature",
                    |_, _| Ok(vec![gst_amorphous()?]),
                ),
                frantz_model(&FRANTZ_A, |_, _| Ok(vec![frantz_amorphous()?])),
            ],
            gst_keys,
        ),
        state(
            "gst-crystalline",
            "Germanium antimony telluride, GST-225 (crystalline)",
            "Ge₂Sb₂Te₅",
            "The standard phase-change material, annealed: a large index and a large k in the telecom bands.",
            polycrystalline(
                CrystalSystem::Cubic,
                "m-3m",
                "Fm-3m (No. 225)",
                "rock salt (metastable)",
                "a polycrystalline film in the metastable rock-salt phase: Frantz et al.'s is face-centred cubic (their abstract), Delaney's states none",
            ),
            vec![
                lorentz_model(
                    GST_C,
                    "within 1.7e-3 in n and 2.5e-3 in k of the data's 198 points from 1.0 to 1.69 µm",
                    "the paper's own sputtered film, room temperature",
                    |_, _| Ok(vec![gst_crystalline()?]),
                ),
                frantz_model(&FRANTZ_C, |_, _| Ok(vec![frantz_crystalline()?])),
            ],
            gst_keys,
        ),
    ]
}
