//! The materials catalogue: each material's refractive-index models, crystal structure,
//! second-order nonlinear and electro-optic tensors, and the papers every number comes from.
//!
//! [`catalogue`] lists the [`Entry`]s; [`entry`] finds one by its id. An entry's
//! [`IndexModel`]s build ordinary [`Material`]s for given [`Conditions`] (a temperature, a
//! composition), each with its range of validity, so everything the library does with a
//! material works on them. A model keeps its equation (as LaTeX, in the paper's own symbols),
//! its coefficients as the paper prints them, its ranges and stated accuracy, and where in the
//! paper each number is.
//!
//! The tensors ([`Tensor`]) are written in Voigt notation: d_il as a 3 × 6 matrix (l = 1 … 6
//! for jk = 11, 22, 33, 23 or 32, 13 or 31, 12 or 21), r_ij as a 6 × 3 matrix (i = 1 … 6 for
//! the same pairs). The convention is d = χ⁽²⁾/2, in pm/V; each tensor states its wavelength and
//! whether it is clamped (at constant strain, r^S) or unclamped (at constant stress, r^T). Which
//! elements vanish or are equal comes from the point group ([`Pattern`]), never from the source.
//!
//! Every value comes from its primary paper, read from the paper itself. A property without a
//! number yet is listed with the paper it will come from ([`Entry::coming`]) instead of being
//! filled from a secondary source. [`Entry::tags`] states what the crystal's symmetry allows.
//!
//! Beside the indices, thermal conductivities ([`Conductivity`], [`Entry::conductivity`]): bulk
//! and thin-film silicon, fused silica, the heater metals tungsten, titanium and platinum, and a
//! TiN film, each model with its paper, temperature range and, for a film, thickness. The static
//! solver ([`crate::thermal`]) takes them for a job's media it isn't given a value for.
//! Electrical resistivities of films ([`Resistivity`], [`resistivities`]): TiN's, film by film,
//! each with how it was made, its thickness and paper.
//!
//! Permittivities and loss tangents at microwave and millimetre-wave frequencies
//! ([`RfDielectric`], [`Entry::rf_dielectric`]): fused silica, crystalline quartz, sapphire,
//! silicon and lithium niobate, each value with its sample, frequency and paper ([`rf_dielectrics`]).
//!
//! Phase-change materials (Sb₂Se₃, Sb₂S₃, GST): each state an entry of its own
//! ([`Category::PhaseChange`], e.g. `sb2se3-amorphous` and `sb2se3-crystalline`), the pair in a
//! [`PhaseChange`] with the temperatures that switch it and its thermal properties
//! ([`phase_change`]).

use serde::Serialize;

use super::Material;
use crate::Result;
use crate::units::Wavelength;

pub(crate) mod batio3_film;
pub(crate) mod checks;
mod coming;
pub(crate) mod conductivity;
pub(crate) mod conductivity_checks;
mod entries;
pub(crate) mod ferroelectric;
pub(crate) mod litao3_dndt;
mod models;
#[path = "../../cases/material/catalogue/opo_checks.rs"]
pub(crate) mod opo_checks;
pub(crate) mod phase_change;
#[path = "../../cases/material/catalogue/phase_change_checks.rs"]
pub(crate) mod phase_change_checks;
pub(crate) mod phase_change_thermal;
mod references;
pub(crate) mod resistivity;
pub(crate) mod rf;
pub(crate) mod rf_checks;
mod symmetry;
mod tags;
#[cfg(test)]
mod tests;
pub(crate) mod thermo;
pub(crate) mod thermo_checks;

pub use coming::Coming;
pub use conductivity::{Conductivity, ConductivityModel, conductivities, conductivity};
pub(crate) use entries::lithium_rich_jundt_1990;
pub use phase_change::{
    PhaseChange, State, figure_of_merit, partly_crystalline, phase_change, phase_changes,
};
pub use resistivity::{Resistivity, ResistivityValue, resistivities, resistivity};
pub use rf::{RfDielectric, RfValue, rf_dielectric, rf_dielectrics};
pub use symmetry::{Pattern, pattern};
pub use tags::Tag;

/// What kind of material an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Category {
    /// A dielectric: a glass or a deposited film.
    Dielectric,
    /// A semiconductor.
    Semiconductor,
    /// A non-centrosymmetric crystal used for its χ⁽²⁾ or Pockels effect.
    NonlinearCrystal,
    /// One state of a phase-change material: amorphous or crystalline, each its own entry
    /// ([`PhaseChange`] has the pair and what switches them).
    PhaseChange,
}

/// The crystal system, or none for an amorphous material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CrystalSystem {
    /// No long-range order: a glass or an amorphous film.
    Amorphous,
    /// Cubic.
    Cubic,
    /// Hexagonal.
    Hexagonal,
    /// Trigonal.
    Trigonal,
    /// Tetragonal.
    Tetragonal,
    /// Orthorhombic: three unequal, orthogonal axes (a biaxial crystal, such as KTP or LBO).
    Orthorhombic,
    /// Monoclinic.
    Monoclinic,
    /// Triclinic.
    Triclinic,
}

/// How the material's index depends on the polarization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum OpticalClass {
    /// One index for every polarization.
    Isotropic,
    /// One optic axis: n_o for light polarized across it, n_e along it.
    Uniaxial {
        /// Positive (n_e > n_o) or negative (n_e < n_o).
        positive: bool,
        /// Which crystal axis the optic axis is, e.g. `"z (the c axis)"`.
        optic_axis: String,
    },
    /// Two optic axes: three principal indices n_x < n_y < n_z along the principal dielectric
    /// axes x, y and z, named as D. A. Roberts proposes (IEEE J. Quantum Electron. 28, 2057
    /// (1992), doi:10.1109/3.159516). Its index models give [`Axis::X`], [`Axis::Y`] and
    /// [`Axis::Z`].
    Biaxial {
        /// How the principal axes x, y, z lie on the crystallographic a, b, c, e.g.
        /// `"x, y, z along a, b, c"`.
        axes: String,
    },
}

/// The crystal structure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Crystal {
    /// The crystal system.
    pub system: CrystalSystem,
    /// The point group, in Hermann–Mauguin notation (`"3m"`, `"-43m"`, `"6mm"`, `"m-3m"`), or
    /// `"∞∞m"` for an isotropic amorphous material.
    pub point_group: String,
    /// The space group with its number, e.g. `"R3c (No. 161)"`, when the material has one.
    pub space_group: Option<String>,
    /// The structure's common name, e.g. `"zincblende"`.
    pub structure: String,
    /// The optical class.
    pub optical: OpticalClass,
    /// Whether the point group has a centre of inversion (then the bulk χ⁽²⁾ and r vanish in the
    /// electric-dipole approximation; [`Entry::tags`] says what remains).
    pub centrosymmetric: bool,
    /// Anything else, e.g. an ordering that lowers the symmetry.
    pub notes: String,
}

/// A paper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Reference {
    /// A short key, the file name its PDF is kept under without `.pdf`, e.g. `"zelmon-1997"`.
    pub key: String,
    /// The citation, e.g. `"D. E. Zelmon, D. L. Small, D. Jundt, J. Opt. Soc. Am. B 14, 3319 (1997)"`.
    pub citation: String,
    /// The title.
    pub title: String,
    /// The DOI, checked.
    pub doi: String,
    /// An open-access copy, when there is one.
    pub open_access: Option<String>,
}

/// Where a value comes from: a paper (by its key) and the place in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Source {
    /// The [`Reference::key`].
    pub reference: String,
    /// Where in the paper, e.g. `"Table 1"` or `"Eq. (12)"`.
    pub location: String,
}

/// A model's adjustable condition: the temperature or the composition.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Parameter {
    /// Its symbol, e.g. `"T"` or `"x"`.
    pub symbol: String,
    /// What it is, e.g. `"temperature"` or `"Al fraction"`.
    pub name: String,
    /// Its unit, e.g. `"K"`, or empty.
    pub unit: String,
    /// The smallest value the model is valid for.
    pub min: f64,
    /// The largest value the model is valid for.
    pub max: f64,
    /// The value used when none is given.
    pub default: f64,
}

/// The conditions to evaluate a model at; `None` takes the model's default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Conditions {
    /// The temperature, in kelvin.
    pub temperature: Option<f64>,
    /// The composition (e.g. x in AlₓGa₁₋ₓAs).
    pub composition: Option<f64>,
}

/// Which index a [`Material`] of a model is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Axis {
    /// The one index of an isotropic material.
    Isotropic,
    /// n_o, for light polarized perpendicular to the optic axis.
    Ordinary,
    /// n_e, for light polarized along the optic axis.
    Extraordinary,
    /// A biaxial crystal's n_x, for light polarized along its principal axis x.
    X,
    /// A biaxial crystal's n_y.
    Y,
    /// A biaxial crystal's n_z.
    Z,
}

/// A table of coefficients as the paper prints it: strings, digit for digit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CoefficientTable {
    /// What the table is, with its place in the paper, e.g. `"Table 1, congruent LiNbO₃"`.
    pub caption: String,
    /// The column headings (LaTeX allowed between `$`s); the first labels the rows.
    pub columns: Vec<String>,
    /// The rows, each as long as `columns`.
    pub rows: Vec<Vec<String>>,
}

/// dn/dT in closed form: λ (µm), T (K) and the composition to one value per axis, per K.
type Slope = fn(f64, f64, f64) -> Result<Vec<f64>>;

/// A refractive-index model of a material, from one paper.
#[derive(Clone, Debug, Serialize)]
pub struct IndexModel {
    /// A short stable id, e.g. `"zelmon-1997-congruent"`.
    pub id: String,
    /// A short name, e.g. `"Zelmon et al. 1997"`.
    pub name: String,
    /// The indices the model gives, in the order [`IndexModel::materials`] returns them.
    pub axes: Vec<Axis>,
    /// The equation, in LaTeX, in the paper's symbols.
    pub equation: String,
    /// What the symbols are, in text with LaTeX between `$`s.
    pub symbols: String,
    /// The coefficients as printed.
    pub coefficients: Vec<CoefficientTable>,
    /// The wavelengths (µm) the model is valid for at its default conditions; a model whose
    /// range moves with the conditions (a band gap) says so in `notes`.
    pub wavelength: (f64, f64),
    /// The temperature, when the model has one (kelvin).
    pub temperature: Option<Parameter>,
    /// The composition, when the model has one.
    pub composition: Option<Parameter>,
    /// The accuracy the paper states.
    pub accuracy: String,
    /// The papers and the places in them.
    pub sources: Vec<Source>,
    /// Whether this is the entry's default model.
    pub default: bool,
    /// Anything else a user should know.
    pub notes: String,
    #[serde(skip)]
    eval: fn(f64, f64) -> Result<Vec<Material>>,
    /// dn/dT in closed form (λ in µm, T, the composition), when the model has one.
    #[serde(skip)]
    dndt: Option<Slope>,
}

impl IndexModel {
    /// The model's materials at the given conditions, one per [`IndexModel::axes`], each with
    /// its range of wavelengths.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] when a condition is outside the model's range.
    pub fn materials(&self, conditions: Conditions) -> Result<Vec<Material>> {
        let t = conditions
            .temperature
            .or(self.temperature.as_ref().map(|p| p.default))
            .unwrap_or(f64::NAN);
        let x = conditions
            .composition
            .or(self.composition.as_ref().map(|p| p.default))
            .unwrap_or(f64::NAN);
        (self.eval)(t, x)
    }

    /// The thermo-optic coefficients dn/dT, per kelvin, of the model's materials at a
    /// wavelength and conditions, one per [`IndexModel::axes`]: the derivative of the model's
    /// own n(λ, T), in closed form where the paper gives the model as a formula in T (Li's
    /// Eq. (22)), otherwise by central differences 0.5 K wide (one-sided within 0.5 K of the
    /// range's ends).
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for a model that takes no temperature, or a condition or
    /// wavelength outside the model's range.
    pub fn thermo_optic(&self, wavelength: Wavelength, conditions: Conditions) -> Result<Vec<f64>> {
        let Some(p) = &self.temperature else {
            return Err(crate::Error::invalid(
                "temperature",
                format!("the model {} takes no temperature", self.id),
            ));
        };
        let t = conditions.temperature.unwrap_or(p.default);
        let x = conditions
            .composition
            .or(self.composition.as_ref().map(|p| p.default))
            .unwrap_or(f64::NAN);
        let lam = wavelength.to_um();
        match self.dndt {
            Some(f) => f(lam, t, x),
            None => thermo::by_differences(|t| (self.eval)(t, x), (p.min, p.max), lam, t),
        }
    }
}

/// One element of a tensor.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Cell {
    /// Zero by the point group's symmetry.
    Zero,
    /// An independent element and its value (pm/V), with the uncertainty the paper gives.
    Value {
        /// The value, in pm/V.
        value: f64,
        /// Its uncertainty, in pm/V, when given.
        uncertainty: Option<f64>,
    },
    /// An independent element this source doesn't give.
    Unknown,
    /// Equal by symmetry to `sign` times another element (1-based row and column).
    Same {
        /// The other element's row.
        row: u8,
        /// The other element's column.
        col: u8,
        /// +1 or −1.
        sign: i8,
    },
}

/// Which tensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TensorKind {
    /// The second-order nonlinear coefficients d_il, 3 × 6.
    SecondOrder,
    /// The linear electro-optic (Pockels) coefficients r_ij, 6 × 3.
    ElectroOptic,
}

/// Whether an electro-optic coefficient is measured with the crystal free or held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Clamping {
    /// At constant strain (r^S): modulation above the acoustic resonances.
    Clamped,
    /// At constant stress (r^T): low-frequency or static fields.
    Unclamped,
    /// Not applicable (d coefficients).
    None,
}

/// A second-order tensor of a material, from one source.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Tensor {
    /// d or r.
    pub kind: TensorKind,
    /// A short label, e.g. `"clamped, 633 nm"`.
    pub label: String,
    /// The point group the pattern follows.
    pub point_group: String,
    /// The elements: 3 rows × 6 for d, 6 rows × 3 for r.
    pub cells: Vec<Vec<Cell>>,
    /// The wavelength (µm) the values hold at; for d, the fundamental's.
    pub wavelength: Option<f64>,
    /// Clamped or unclamped (r only).
    pub clamping: Clamping,
    /// The convention, e.g. `"d = χ⁽²⁾/2, Voigt notation"`.
    pub convention: String,
    /// Where the values come from; `None` for a tensor zero by symmetry.
    pub source: Option<Source>,
    /// Anything else, e.g. which elements this source doesn't give.
    pub notes: String,
}

impl Tensor {
    /// The value (pm/V) at a 1-based row and column, following `Same` links; `None` when the
    /// element is unknown.
    pub fn value(&self, row: usize, col: usize) -> Option<f64> {
        match self
            .cells
            .get(row.checked_sub(1)?)?
            .get(col.checked_sub(1)?)?
        {
            Cell::Zero => Some(0.0),
            Cell::Value { value, .. } => Some(*value),
            Cell::Unknown => None,
            Cell::Same { row, col, sign } => self
                .value(*row as usize, *col as usize)
                .map(|v| f64::from(*sign) * v),
        }
    }
}

/// A scalar property with its source, e.g. a static permittivity or a combination of tensor
/// elements that a measurement resolves.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Constant {
    /// The symbol, in LaTeX, e.g. `"\\varepsilon^S_{11}"`.
    pub symbol: String,
    /// What it is.
    pub name: String,
    /// The value.
    pub value: f64,
    /// Its uncertainty, when given.
    pub uncertainty: Option<f64>,
    /// The unit, or empty.
    pub unit: String,
    /// The conditions it holds for, e.g. `"25 °C, clamped"`.
    pub conditions: String,
    /// Where it comes from.
    pub source: Source,
}

/// A property the catalogue doesn't have a number for yet, in short; [`Entry::coming`] has the
/// paper it will come from and what the papers in hand say.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Missing {
    /// The property, e.g. `"r₄₁(x)"`.
    pub property: String,
    /// Where it will come from, e.g. `"from Glick, Reinhart & Martin 1988, coming"`.
    pub reason: String,
}

/// One material of the catalogue.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    /// A short stable id, e.g. `"linbo3"`.
    pub id: String,
    /// The name, e.g. `"Lithium niobate (congruent)"`.
    pub name: String,
    /// The formula, e.g. `"LiNbO₃"`.
    pub formula: String,
    /// The category.
    pub category: Category,
    /// A sentence on what it is and what it's used for.
    pub summary: String,
    /// The crystal structure.
    pub crystal: Crystal,
    /// The index models; the default first.
    pub index: Vec<IndexModel>,
    /// The second-order and electro-optic tensors.
    pub tensors: Vec<Tensor>,
    /// Other properties: permittivities, combinations of coefficients.
    pub constants: Vec<Constant>,
    /// What has no number yet, in short (see [`Entry::coming`]).
    pub missing: Vec<Missing>,
    /// Every paper the entry cites.
    pub references: Vec<Reference>,
}

impl Entry {
    /// The default index model, if the entry has one.
    pub fn default_model(&self) -> Option<&IndexModel> {
        self.index.iter().find(|m| m.default).or(self.index.first())
    }

    /// The material's thermal conductivity, beside its index, when the catalogue has one
    /// ([`conductivity`]).
    pub fn conductivity(&self) -> Option<Conductivity> {
        conductivity(&self.id)
    }

    /// The material's permittivities and loss tangents at microwave frequencies, beside its
    /// index, when the catalogue has them ([`rf_dielectric`]).
    pub fn rf_dielectric(&self) -> Option<RfDielectric> {
        rf_dielectric(&self.id)
    }

    /// A reference by its key.
    pub fn reference(&self, key: &str) -> Option<&Reference> {
        self.references.iter().find(|r| r.key == key)
    }

    /// The model `model` (the default when `None`) at `conditions`, as the crystal's principal
    /// materials along its x, y and z: a uniaxial crystal's (its optic axis z) n_o, n_o, n_e, a
    /// biaxial one's n_x, n_y, n_z.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for an unknown model, an isotropic entry (it has no axes
    /// to turn), a model that doesn't give every principal index, an optic axis other than z,
    /// and conditions outside the model's range.
    pub fn principal(&self, model: Option<&str>, conditions: Conditions) -> Result<[Material; 3]> {
        let invalid = |reason: String| crate::Error::invalid("material", reason);
        let m = match model {
            None => self.default_model(),
            Some(id) => self.index.iter().find(|i| i.id == id),
        }
        .ok_or_else(|| {
            invalid(format!(
                "{} has no index model {}",
                self.id,
                model.unwrap_or("")
            ))
        })?;
        let materials = m.materials(conditions)?;
        let find = |axis: Axis| {
            m.axes
                .iter()
                .position(|a| *a == axis)
                .map(|k| materials[k].clone())
                .ok_or_else(|| {
                    invalid(format!(
                        "{}'s {} doesn't give its {} index",
                        self.id,
                        m.id,
                        format!("{axis:?}").to_lowercase()
                    ))
                })
        };
        match &self.crystal.optical {
            OpticalClass::Isotropic => Err(invalid(format!(
                "{} is isotropic: it has no crystal axes to orient",
                self.id
            ))),
            OpticalClass::Uniaxial { optic_axis, .. } => {
                if !optic_axis.starts_with('z') {
                    return Err(invalid(format!(
                        "{}'s optic axis is {optic_axis}, not z",
                        self.id
                    )));
                }
                let o = find(Axis::Ordinary)?;
                Ok([o.clone(), o, find(Axis::Extraordinary)?])
            }
            OpticalClass::Biaxial { .. } => Ok([find(Axis::X)?, find(Axis::Y)?, find(Axis::Z)?]),
        }
    }

    /// The model `model` (the default when `None`) at `conditions`, turned onto the chip as
    /// `orientation` says: a [`Material::oriented`] of its [`Entry::principal`] materials.
    ///
    /// # Errors
    ///
    /// Those of [`Entry::principal`] and [`Material::oriented`].
    pub fn oriented(
        &self,
        model: Option<&str>,
        conditions: Conditions,
        orientation: super::Orientation,
    ) -> Result<Material> {
        let principal = self.principal(model, conditions)?;
        Material::oriented(
            format!("{} ({orientation})", self.formula),
            principal,
            orientation,
        )
    }
}

/// Every material of the catalogue, grouped by category.
pub fn catalogue() -> Vec<Entry> {
    entries::all()
}

/// The entry with this id.
pub fn entry(id: &str) -> Option<Entry> {
    catalogue().into_iter().find(|e| e.id == id)
}
