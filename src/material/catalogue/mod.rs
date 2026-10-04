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

use serde::Serialize;

use super::Material;
use crate::Result;

pub(crate) mod checks;
mod coming;
mod entries;
mod models;
mod references;
mod symmetry;
mod tags;
#[cfg(test)]
mod tests;

pub use coming::Coming;
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

    /// A reference by its key.
    pub fn reference(&self, key: &str) -> Option<&Reference> {
        self.references.iter().find(|r| r.key == key)
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
