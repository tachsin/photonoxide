//! Components and circuits: devices described by their S-matrices, connected into netlists.
//!
//! A **component** ([`Component`]) is anything with ports and an S-matrix at a wavelength: a
//! closed-form model, a compact model fitted to a solver, a 2D or 3D FDFD result, or a
//! measurement. A **netlist** ([`Netlist`]) is instances of components, the connections between
//! their ports, and the ports left open to the outside; compiled ([`Netlist::compile`]) it is a
//! [`Circuit`], whose own S-matrix comes from one sparse linear solve. A circuit is a component
//! itself, so circuits nest. The [circuit adjoint](adjoint) gives a response's gradient with
//! respect to every parameter of every instance from one more solve ([`Circuit::gradient`],
//! with the [`objective`]s), for genoxide's optimizers.
//!
//! **Conventions** (docs/design/components.md has them in full, docs/methods/circuits.md the solve), the same as the FDFD solver's
//! ([`crate::fdfd`]):
//!
//! - fields go as e^(−iωt), so a waveguide of effective index n and length L transmits
//!   e^(+i 2π n L/λ);
//! - amplitudes are **power-normalized**: |a|² is the power a wave carries, so |S_qp|² is the
//!   share of the power going in at port p that comes out at port q;
//! - `s[(q, p)]` is from port p into port q (row: out, column: in), the ports in the order the
//!   component lists them;
//! - each port's phase is referred to its **reference plane**, where the component ends; a
//!   connection joins two reference planes with nothing between them;
//! - a reciprocal component has S = Sᵀ, which the FDFD solver's S-matrices are by construction
//!   (the unconjugated Lorentz form normalizes them); a passive one has every singular value of S
//!   at most 1; a lossless one has SᴴS = I.
//!
//! A netlist, built and solved (the unit test `circuit::tests::the_module_example` runs this):
//!
//! ```ignore
//! use std::sync::Arc;
//! use photonoxide::circuit::{Component, Fixed, Netlist, NetlistError, SMatrix};
//! use photonoxide::units::Wavelength;
//! use photonoxide::Complex64 as c64;
//!
//! // a lossless 2-port that delays the phase by a quarter turn
//! let quarter = SMatrix::from_rows(vec![
//!     vec![c64::new(0.0, 0.0), c64::new(0.0, 1.0)],
//!     vec![c64::new(0.0, 1.0), c64::new(0.0, 0.0)],
//! ])?;
//! assert!(quarter.unitarity_error() < 1e-15);
//! let delay: Arc<dyn Component> = Arc::new(Fixed::new("delay", &["a", "b"], quarter)?);
//!
//! // two of them in series
//! let mut netlist = Netlist::new();
//! netlist.add("first", delay.clone())?;
//! netlist.add("second", delay)?;
//! netlist.connect("first.b", "second.a")?;
//! netlist.expose("in", "first.a")?;
//! assert!(matches!(&netlist.problems()[..], [NetlistError::Dangling(p)] if p.to_string() == "second.b"));
//! netlist.expose("out", "second.b")?;
//! let circuit = netlist.compile()?;
//!
//! let s = circuit.s_matrix(Wavelength::um(1.55)?)?;
//! assert!((s[(1, 0)] - c64::new(-1.0, 0.0)).norm() < 1e-15); // a half turn
//! # Ok::<(), photonoxide::Error>(())
//! ```

pub mod adjoint;
pub mod components;
pub(crate) mod ideal;
mod netlist;
pub mod objective;
mod solve;

#[cfg(test)]
mod tests;

pub use netlist::{Instance, Netlist, NetlistError, PortRef};
pub use solve::Circuit;

use std::fmt;
use std::ops::{Index, IndexMut};

use num_complex::Complex64 as c64;

use crate::units::Wavelength;
use crate::{Error, Result};

/// A square scattering matrix: `s[(q, p)]` is the amplitude out of port q for unit amplitude in
/// at port p, power-normalized.
#[derive(Clone, Debug, PartialEq)]
pub struct SMatrix {
    n: usize,
    /// Row by row: `values[q * n + p]` is S_qp.
    values: Vec<c64>,
}

impl SMatrix {
    /// The n × n zero matrix: n ports that absorb everything, a perfect termination each.
    pub fn zeros(n: usize) -> SMatrix {
        SMatrix {
            n,
            values: vec![c64::new(0.0, 0.0); n * n],
        }
    }

    /// The matrix with S_qp = `f(q, p)`.
    pub fn from_fn(n: usize, f: impl Fn(usize, usize) -> c64) -> SMatrix {
        SMatrix {
            n,
            values: (0..n * n).map(|k| f(k / n, k % n)).collect(),
        }
    }

    /// The matrix from its rows, `rows[q][p]` = S_qp: the form [`crate::fdfd::Solver2d::s_matrix`]
    /// returns.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the rows make a square matrix of finite values.
    pub fn from_rows(rows: Vec<Vec<c64>>) -> Result<SMatrix> {
        let n = rows.len();
        if let Some(row) = rows.iter().find(|r| r.len() != n) {
            return Err(Error::invalid(
                "S-matrix",
                format!(
                    "must be square: {n} rows, but a row of {} values",
                    row.len()
                ),
            ));
        }
        if let Some(v) = rows
            .iter()
            .flatten()
            .find(|v| !(v.re.is_finite() && v.im.is_finite()))
        {
            return Err(Error::invalid(
                "S-matrix",
                format!("every value must be finite, got {v}"),
            ));
        }
        Ok(SMatrix {
            n,
            values: rows.into_iter().flatten().collect(),
        })
    }

    /// The number of ports.
    pub fn size(&self) -> usize {
        self.n
    }

    /// The rows, `rows[q][p]` = S_qp.
    pub fn rows(&self) -> Vec<Vec<c64>> {
        self.values
            .chunks(self.n.max(1))
            .map(<[c64]>::to_vec)
            .collect()
    }

    /// Sᵀ.
    pub fn transpose(&self) -> SMatrix {
        SMatrix::from_fn(self.n, |q, p| self[(p, q)])
    }

    /// |S_qp|², the share of the power in at p that comes out at q.
    ///
    /// # Panics
    ///
    /// If q or p isn't a port.
    pub fn power(&self, q: usize, p: usize) -> f64 {
        self[(q, p)].norm_sqr()
    }

    /// How far S is from reciprocal: the largest |S_qp − S_pq|. Zero for a reciprocal
    /// component's exact S.
    pub fn reciprocity_error(&self) -> f64 {
        (0..self.n)
            .flat_map(|q| (0..q).map(move |p| (q, p)))
            .map(|(q, p)| (self[(q, p)] - self[(p, q)]).norm())
            .fold(0.0, f64::max)
    }

    /// The largest singular value of S: the largest power gain, over every combination of
    /// inputs, as an amplitude ratio. At most 1 for a passive component (or circuit).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the singular value decomposition doesn't converge, which only
    /// non-finite entries cause.
    pub fn largest_singular_value(&self) -> Result<f64> {
        if self.n == 0 {
            return Ok(0.0);
        }
        let m = faer::Mat::<c64>::from_fn(self.n, self.n, |q, p| self[(q, p)]);
        let values = m
            .singular_values()
            .map_err(|e| Error::invalid("S-matrix", format!("its singular values: {e:?}")))?;
        Ok(values.into_iter().fold(0.0, f64::max))
    }

    /// Whether S is passive: every singular value at most 1 + `tolerance`.
    ///
    /// # Errors
    ///
    /// As [`SMatrix::largest_singular_value`].
    pub fn is_passive(&self, tolerance: f64) -> Result<bool> {
        Ok(self.largest_singular_value()? <= 1.0 + tolerance)
    }

    /// How far S is from unitary: the largest entry of |SᴴS − I|. Zero for a lossless component:
    /// every input's power comes out somewhere, and different inputs' outputs are orthogonal.
    pub fn unitarity_error(&self) -> f64 {
        let n = self.n;
        let mut worst: f64 = 0.0;
        for i in 0..n {
            for j in 0..n {
                let dot: c64 = (0..n).map(|q| self[(q, i)].conj() * self[(q, j)]).sum();
                let identity = if i == j { 1.0 } else { 0.0 };
                worst = worst.max((dot - identity).norm());
            }
        }
        worst
    }

    /// The largest |S_qp − O_qp| against another matrix of the same size.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the sizes differ.
    pub fn max_difference(&self, other: &SMatrix) -> Result<f64> {
        if other.n != self.n {
            return Err(Error::invalid(
                "S-matrix",
                format!("can't compare {} ports with {}", self.n, other.n),
            ));
        }
        Ok(self
            .values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max))
    }
}

impl Index<(usize, usize)> for SMatrix {
    type Output = c64;

    /// S_qp for `(q, p)`.
    fn index(&self, (q, p): (usize, usize)) -> &c64 {
        assert!(q < self.n && p < self.n, "port out of range");
        &self.values[q * self.n + p]
    }
}

impl IndexMut<(usize, usize)> for SMatrix {
    fn index_mut(&mut self, (q, p): (usize, usize)) -> &mut c64 {
        assert!(q < self.n && p < self.n, "port out of range");
        &mut self.values[q * self.n + p]
    }
}

/// The light a port carries, when it is known: which mode of its waveguide, at a reference
/// wavelength. Connecting two ports of different polarizations or mode orders is an error.
#[derive(Clone, Debug, PartialEq)]
pub struct PortMode {
    /// TE or TM, as the mode solvers label it.
    pub polarization: crate::mode::Polarization,
    /// The mode's order: 0 for the fundamental mode of that polarization.
    pub order: usize,
    /// Its effective index at `wavelength`.
    pub effective_index: f64,
    /// Its group index at `wavelength`, if known.
    pub group_index: Option<f64>,
    /// The wavelength the indices are for.
    pub wavelength: Wavelength,
}

/// A port of a component: one mode of one waveguide where it crosses the component's edge, at
/// the reference plane its phase is measured from.
///
/// A waveguide that carries several modes has a port for each, e.g. `o1` and `o1@TE1`.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    /// The port's name, unique within its component, e.g. `"o1"`. No dots: a netlist names a
    /// port as `instance.port`.
    pub name: String,
    /// Its mode, if the component states it.
    pub mode: Option<PortMode>,
}

impl Port {
    /// A port named `name`, its mode unstated.
    pub fn new(name: impl Into<String>) -> Port {
        Port {
            name: name.into(),
            mode: None,
        }
    }

    /// The same port, carrying `mode`.
    #[must_use]
    pub fn with_mode(mut self, mode: PortMode) -> Port {
        self.mode = Some(mode);
        self
    }
}

/// Ports named `names`, their modes unstated.
pub fn ports(names: &[&str]) -> Vec<Port> {
    names.iter().map(|&n| Port::new(n)).collect()
}

/// A component's parameter: a continuous value its S-matrix depends on, e.g. a waveguide's
/// length or a coupler's gap.
///
/// Anything that changes the ports (a splitter's number of outputs) is not a parameter: it is
/// fixed when the component is built.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    /// Its name, unique within its component, e.g. `"length"`. No whitespace; a circuit names
    /// its instances' parameters `instance.parameter`.
    pub name: String,
    /// Its unit, e.g. `"µm"`, `"rad"`, or `""` for a pure number.
    pub unit: String,
    /// The value a new instance starts with.
    pub default: f64,
    /// The smallest value allowed.
    pub min: f64,
    /// The largest value allowed.
    pub max: f64,
}

impl Parameter {
    /// A parameter `name` in `unit`, starting at `default`, allowed from `min` to `max`.
    pub fn new(
        name: impl Into<String>,
        unit: impl Into<String>,
        default: f64,
        min: f64,
        max: f64,
    ) -> Parameter {
        Parameter {
            name: name.into(),
            unit: unit.into(),
            default,
            min,
            max,
        }
    }

    /// Whether `value` is allowed: finite and within [`min`, `max`].
    ///
    /// [`min`]: Parameter::min
    /// [`max`]: Parameter::max
    pub fn allows(&self, value: f64) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }
}

/// What a component's S-matrix comes from, cheapest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fidelity {
    /// A closed form, e.g. a lossless coupler or a straight waveguide's phase.
    Analytic,
    /// A compact model fitted to a solver's or a measurement's results.
    Compact,
    /// A 2D solve: the effective index method and 2D FDFD.
    TwoD,
    /// A 3D solve.
    ThreeD,
    /// Measured S-parameters, e.g. read from a Touchstone file.
    Measured,
}

impl fmt::Display for Fidelity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Fidelity::Analytic => "analytic",
            Fidelity::Compact => "compact",
            Fidelity::TwoD => "2D",
            Fidelity::ThreeD => "3D",
            Fidelity::Measured => "measured",
        })
    }
}

/// Where a component's S-matrix comes from, and how far it is from its source.
#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    /// The kind of model.
    pub fidelity: Fidelity,
    /// What it reproduces: a paper with its DOI and equation, a solver with its grid, or a
    /// measurement's file.
    pub source: String,
    /// The largest |ΔS_qp| against that source over [`Provenance::validity`], if it was
    /// measured: a fit's residual, a solve's convergence error, a measurement's uncertainty.
    /// `Some(0.0)` for a closed form computed as its paper states it.
    pub error: Option<f64>,
    /// The wavelengths the model holds for, in µm, if limited.
    pub validity: Option<(f64, f64)>,
}

/// A device with ports and an S-matrix: the unit a [`Netlist`] connects.
///
/// A component is a **model**: its parameters' values aren't part of it but are passed in, so one
/// component can be shared (`Arc<dyn Component>`) by every instance of it in a netlist, each with
/// its own values, and an optimizer can move the values without touching the components.
///
/// Implementations state their ports and parameters once (the slices don't change), return S in
/// the order of [`Component::ports`] with the conventions of the [module](self), and say where
/// S comes from ([`Component::provenance`]).
pub trait Component: fmt::Debug + Send + Sync {
    /// What kind of component it is, e.g. `"waveguide"` or `"directional coupler"`.
    fn kind(&self) -> &str;

    /// Its ports, in the order of its S-matrix's rows and columns.
    fn ports(&self) -> &[Port];

    /// Its parameters, in the order `values` lists them.
    fn parameters(&self) -> &[Parameter];

    /// Its S-matrix at `wavelength`, with its parameters at `values` (one per
    /// [`Component::parameters`], each allowed).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a wavelength outside the model's validity or values it can't
    /// take; a component's own reasons otherwise.
    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix>;

    /// Where its S-matrix comes from, and its error against that source.
    fn provenance(&self) -> Provenance;

    /// Whether it is reciprocal, S = Sᵀ: true unless it has a magneto-optic or time-varying
    /// part.
    fn reciprocal(&self) -> bool {
        true
    }

    /// ∂S/∂θ_k at `wavelength` and `values`, one matrix per parameter in the order of
    /// [`Component::parameters`], for the circuit adjoint; `None` if the component doesn't
    /// provide them, and the circuit then takes finite differences of
    /// [`Component::s_matrix`].
    ///
    /// # Errors
    ///
    /// As [`Component::s_matrix`].
    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let _ = (wavelength, values);
        Ok(None)
    }

    /// The default values of its parameters.
    fn defaults(&self) -> Vec<f64> {
        self.parameters().iter().map(|p| p.default).collect()
    }
}

/// A component whose S-matrix is the same at every wavelength, with no parameters: an ideal
/// device, or one measured at a single wavelength.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixed {
    kind: String,
    ports: Vec<Port>,
    s: SMatrix,
}

impl Fixed {
    /// A component of kind `kind`, ports `names` and S-matrix `s`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `s` isn't one row and column per port.
    pub fn new(kind: impl Into<String>, names: &[&str], s: SMatrix) -> Result<Fixed> {
        if s.size() != names.len() {
            return Err(Error::invalid(
                "S-matrix",
                format!("has {} ports, but {} names", s.size(), names.len()),
            ));
        }
        Ok(Fixed {
            kind: kind.into(),
            ports: ports(names),
            s,
        })
    }
}

impl Component for Fixed {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &[]
    }

    fn s_matrix(&self, _: Wavelength, _: &[f64]) -> Result<SMatrix> {
        Ok(self.s.clone())
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "a fixed S-matrix".into(),
            error: Some(0.0),
            validity: None,
        }
    }

    fn reciprocal(&self) -> bool {
        self.s.reciprocity_error() == 0.0
    }
}

/// S-matrices at several wavelengths, with the ports' names: a component's or a circuit's
/// spectrum.
#[derive(Clone, Debug, PartialEq)]
pub struct Spectrum {
    ports: Vec<String>,
    wavelengths: Vec<Wavelength>,
    matrices: Vec<SMatrix>,
}

impl Spectrum {
    /// The spectrum of S-matrices `matrices` at `wavelengths`, between ports `ports`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless there is one matrix per wavelength, each with one row and
    /// column per port.
    pub fn new(
        ports: Vec<String>,
        wavelengths: Vec<Wavelength>,
        matrices: Vec<SMatrix>,
    ) -> Result<Spectrum> {
        if matrices.len() != wavelengths.len() {
            return Err(Error::invalid(
                "spectrum",
                format!(
                    "needs one S-matrix per wavelength: {} wavelengths, {} matrices",
                    wavelengths.len(),
                    matrices.len()
                ),
            ));
        }
        if let Some(m) = matrices.iter().find(|m| m.size() != ports.len()) {
            return Err(Error::invalid(
                "spectrum",
                format!("has {} ports, but an S-matrix of {}", ports.len(), m.size()),
            ));
        }
        Ok(Spectrum {
            ports,
            wavelengths,
            matrices,
        })
    }

    /// A component's spectrum at `wavelengths`, its parameters at `values`.
    ///
    /// The wavelengths are solved in parallel on rayon's threads (`RAYON_NUM_THREADS` sets how
    /// many): each S-matrix is computed on its own, as [`Component::s_matrix`] gives it, and
    /// collected in order, so the spectrum is the same bit for bit on any number of threads.
    ///
    /// # Errors
    ///
    /// The component's at the first wavelength that fails, and [`Error::InvalidValue`] if it
    /// returns S-matrices of the wrong size.
    pub fn of(
        component: &dyn Component,
        wavelengths: &[Wavelength],
        values: &[f64],
    ) -> Result<Spectrum> {
        use rayon::prelude::*;
        // every wavelength solved, then the first error in wavelength order: the same error
        // whichever thread finishes first
        let solved: Vec<Result<SMatrix>> = wavelengths
            .par_iter()
            .map(|&w| component.s_matrix(w, values))
            .collect();
        let matrices = solved.into_iter().collect::<Result<Vec<_>>>()?;
        let ports = component.ports().iter().map(|p| p.name.clone()).collect();
        Spectrum::new(ports, wavelengths.to_vec(), matrices)
    }

    /// The ports' names, in the matrices' order.
    pub fn ports(&self) -> &[String] {
        &self.ports
    }

    /// The position of the port named `name`.
    pub fn port(&self, name: &str) -> Option<usize> {
        self.ports.iter().position(|p| p == name)
    }

    /// The wavelengths.
    pub fn wavelengths(&self) -> &[Wavelength] {
        &self.wavelengths
    }

    /// The S-matrices, one per wavelength.
    pub fn matrices(&self) -> &[SMatrix] {
        &self.matrices
    }

    /// S_qp at every wavelength.
    ///
    /// # Panics
    ///
    /// If q or p isn't a port.
    pub fn element(&self, q: usize, p: usize) -> Vec<c64> {
        self.matrices.iter().map(|m| m[(q, p)]).collect()
    }
}
