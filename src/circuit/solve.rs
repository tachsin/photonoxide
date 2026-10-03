//! A circuit's S-matrix: one sparse solve of the whole netlist, and Filipsson's sub-network
//! growth as its reference.
//!
//! All ports of all instances are numbered together, an instance's ports one after the other in
//! the order of the netlist. S_b, block diagonal, holds every component's S-matrix: b = S_b a, a
//! the waves going into the ports and b those coming out. A connection between ports i and j
//! says a_i = b_j and a_j = b_i; an external port k's incoming wave is the circuit's input x_k.
//! So a = Γ b + E x, with Γ the connections' symmetric permutation and E the external ports'
//! selection, and
//!
//! (I − S_b Γ) b = S_b E x,  S = Eᵀ (I − S_b Γ)⁻¹ S_b E:
//!
//! one sparse system with as many unknowns as ports and a right-hand side per external port,
//! solved by faer's sparse LU. Its sparsity depends only on the netlist (every entry of every
//! component's block is kept, zero or not), so it is analysed once, when the netlist is
//! compiled, and every wavelength reuses it.
//!
//! G. Filipsson's sub-network growth (11th European Microwave Conference, 700 (1981),
//! doi:10.1109/EUMA.1981.332972) joins the ports one connection at a time, eliminating each
//! pair from the block-diagonal S by his Eq. 6; it is the reference the sparse solve is checked
//! against.

use faer::sparse::linalg::solvers::{Lu, SymbolicLu};
use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use super::netlist::{Netlist, NetlistError};
use super::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, Spectrum};
use crate::units::Wavelength;
use crate::{Error, Result};

/// Where an entry of I − S_b Γ comes from.
#[derive(Clone, Copy, Debug)]
enum Entry {
    /// The identity's 1, alone.
    One,
    /// −S_qp of instance `instance`.
    Block {
        instance: usize,
        q: usize,
        p: usize,
        /// Also on the diagonal: 1 − S_qp.
        diagonal: bool,
    },
}

/// A compiled netlist: the circuit's ports, its parameters, and its system's sparsity, analysed.
///
/// [`Circuit::s_matrix`] solves it at a wavelength, [`Circuit::spectrum`] at several. A circuit
/// is a [`Component`] too, with the netlist's external ports as its ports and its instances'
/// parameters, named `instance.parameter`, as its parameters, so it can be an instance of a
/// larger netlist.
#[derive(Clone, Debug)]
pub struct Circuit {
    netlist: Netlist,
    /// Each instance's first port's number.
    offsets: Vec<usize>,
    /// Each instance's first parameter's position in the circuit's values.
    parameter_offsets: Vec<usize>,
    /// Every port's instance and its position there.
    owner: Vec<(usize, usize)>,
    /// The external ports' numbers, in the circuit's order.
    external: Vec<usize>,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    values: Vec<f64>,
    /// The entries of I − S_b Γ, column by column.
    entries: Vec<(usize, usize, Entry)>,
    symbolic: SymbolicLu<usize>,
}

impl Netlist {
    /// The circuit this netlist describes, ready to solve: checked, numbered, and its system's
    /// sparsity analysed.
    ///
    /// # Errors
    ///
    /// [`Error::Netlist`] with the first of [`Netlist::problems`], and [`Error::InvalidValue`]
    /// if the sparsity can't be analysed.
    pub fn compile(&self) -> Result<Circuit> {
        self.validate()?;
        let instances = self.instances();
        let mut offsets = Vec::with_capacity(instances.len());
        let mut parameter_offsets = Vec::with_capacity(instances.len());
        let mut owner = Vec::new();
        let mut parameters = Vec::new();
        let mut values = Vec::new();
        for (i, inst) in instances.iter().enumerate() {
            offsets.push(owner.len());
            parameter_offsets.push(parameters.len());
            owner.extend((0..inst.component.ports().len()).map(|p| (i, p)));
            for (p, &value) in inst.component.parameters().iter().zip(&inst.values) {
                parameters.push(Parameter {
                    name: format!("{}.{}", inst.name, p.name),
                    default: value,
                    ..p.clone()
                });
            }
            values.extend_from_slice(&inst.values);
        }
        let n = owner.len();
        let number = |r| -> Result<usize> {
            let (i, p) = self.resolve(r)?;
            Ok(offsets[i] + p)
        };
        let mut partner = vec![None; n];
        for (a, b) in self.connections() {
            let (a, b) = (number(a)?, number(b)?);
            partner[a] = Some(b);
            partner[b] = Some(a);
        }
        let mut external = Vec::new();
        let mut ports = Vec::new();
        for (name, r) in self.external() {
            let g = number(r)?;
            external.push(g);
            let (i, p) = owner[g];
            ports.push(Port {
                name: name.clone(),
                mode: instances[i].component.ports()[p].mode.clone(),
            });
        }
        // I − S_b Γ: column j is e_j minus column partner(j) of S_b, which is nonzero only on
        // the partner's instance's rows
        let mut entries = Vec::new();
        for (j, &partner) in partner.iter().enumerate() {
            match partner {
                None => entries.push((j, j, Entry::One)),
                Some(m) => {
                    let (instance, p) = owner[m];
                    let rows = offsets[instance]
                        ..offsets[instance] + instances[instance].component.ports().len();
                    if !rows.contains(&j) {
                        entries.push((j, j, Entry::One));
                    }
                    for row in rows {
                        entries.push((
                            row,
                            j,
                            Entry::Block {
                                instance,
                                q: row - offsets[instance],
                                p,
                                diagonal: row == j,
                            },
                        ));
                    }
                }
            }
        }
        let pattern: Vec<Triplet<usize, usize, c64>> = entries
            .iter()
            .map(|&(row, col, _)| Triplet::new(row, col, c64::new(1.0, 0.0)))
            .collect();
        let matrix = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &pattern)
            .map_err(|e| Error::invalid("circuit", format!("can't assemble its system: {e:?}")))?;
        let symbolic = SymbolicLu::try_new(matrix.symbolic())
            .map_err(|e| Error::invalid("circuit", format!("can't analyse its system: {e:?}")))?;
        Ok(Circuit {
            netlist: self.clone(),
            offsets,
            parameter_offsets,
            owner,
            external,
            ports,
            parameters,
            values,
            entries,
            symbolic,
        })
    }
}

impl Circuit {
    /// The netlist it was compiled from, with the values it was compiled with.
    pub fn netlist(&self) -> &Netlist {
        &self.netlist
    }

    /// Its parameters' current values, in the order of [`Component::parameters`]: the
    /// instances in order, each one's parameters in order. Each is also its parameter's
    /// default, so a circuit added to a larger netlist starts from them.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Sets parameter `parameter` of instance `instance` to `value`.
    ///
    /// # Errors
    ///
    /// As [`Netlist::set`].
    pub fn set(&mut self, instance: &str, parameter: &str, value: f64) -> Result<()> {
        self.netlist.set(instance, parameter, value)?;
        let i = self.netlist.index(instance)?;
        let inst = &self.netlist.instances()[i];
        let start = self.parameter_offsets[i];
        self.values[start..start + inst.values.len()].copy_from_slice(&inst.values);
        for (k, &v) in inst.values.iter().enumerate() {
            self.parameters[start + k].default = v;
        }
        Ok(())
    }

    /// The circuit's S-matrix at `wavelength`, between its external ports, from one sparse solve
    /// (see the module's docs), its parameters at their current values.
    ///
    /// # Errors
    ///
    /// A component's own, [`Error::Netlist`] with [`NetlistError::SizeMismatch`] for a component
    /// whose S-matrix isn't one row and column per port, and [`Error::InvalidValue`] if the system
    /// is singular, as a lossless loop is exactly on a resonance it can't leave.
    pub fn s_matrix(&self, wavelength: Wavelength) -> Result<SMatrix> {
        self.solve(wavelength, &self.values)
    }

    /// The circuit's S-matrix at `wavelength` with its parameters at `values` (one per
    /// [`Component::parameters`]) instead of its own: what an optimizer calls.
    ///
    /// # Errors
    ///
    /// As [`Circuit::s_matrix`], and [`Error::InvalidValue`] for the wrong number of values.
    pub fn s_matrix_with(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        self.solve(wavelength, values)
    }

    /// The circuit's spectrum at `wavelengths`, one sparse solve each, the sparsity analysed
    /// once.
    ///
    /// # Errors
    ///
    /// As [`Circuit::s_matrix`].
    pub fn spectrum(&self, wavelengths: &[Wavelength]) -> Result<Spectrum> {
        Spectrum::of(self, wavelengths, &self.values)
    }

    /// Every instance's S-matrix at `wavelength`, its parameters at `values`, each checked for
    /// its size.
    fn blocks(&self, wavelength: Wavelength, values: &[f64]) -> Result<Vec<SMatrix>> {
        if values.len() != self.parameters.len() {
            return Err(Error::invalid(
                "circuit",
                format!(
                    "has {} parameters, got {} values",
                    self.parameters.len(),
                    values.len()
                ),
            ));
        }
        self.netlist
            .instances()
            .iter()
            .enumerate()
            .map(|(i, inst)| {
                let start = self.parameter_offsets[i];
                let own = &values[start..start + inst.values.len()];
                let s = inst.component.s_matrix(wavelength, own)?;
                let ports = inst.component.ports().len();
                if s.size() != ports {
                    return Err(NetlistError::SizeMismatch {
                        instance: inst.name.clone(),
                        ports,
                        size: s.size(),
                    }
                    .into());
                }
                Ok(s)
            })
            .collect()
    }

    /// The sparse solve, its parameters at `values`.
    fn solve(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        use faer::linalg::solvers::Solve;
        let blocks = self.blocks(wavelength, values)?;
        let n = self.owner.len();
        let k = self.external.len();
        if k == 0 {
            return Ok(SMatrix::zeros(0));
        }
        let one = c64::new(1.0, 0.0);
        let triplets: Vec<Triplet<usize, usize, c64>> = self
            .entries
            .iter()
            .map(|&(row, col, entry)| {
                let value = match entry {
                    Entry::One => one,
                    Entry::Block {
                        instance,
                        q,
                        p,
                        diagonal,
                    } => {
                        let s = -blocks[instance][(q, p)];
                        if diagonal { one + s } else { s }
                    }
                };
                Triplet::new(row, col, value)
            })
            .collect();
        let singular = || {
            Error::invalid(
                "circuit",
                format!("its system is singular at {wavelength}: a lossless resonance"),
            )
        };
        let matrix = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets)
            .map_err(|e| Error::invalid("circuit", format!("can't assemble its system: {e:?}")))?;
        let lu = Lu::try_new_with_symbolic(self.symbolic.clone(), matrix.as_ref())
            .map_err(|_| singular())?;
        // S_b E: column c is column `external[c]` of S_b, on its instance's rows
        let mut rhs = faer::Mat::<c64>::zeros(n, k);
        for (c, &g) in self.external.iter().enumerate() {
            let (instance, p) = self.owner[g];
            let start = self.offsets[instance];
            for q in 0..blocks[instance].size() {
                rhs[(start + q, c)] = blocks[instance][(q, p)];
            }
        }
        let mut b = lu.solve(&rhs);
        // one step of iterative refinement, against the factorization's rounding near a sharp
        // resonance
        let mut residual = rhs.clone();
        for t in &triplets {
            for c in 0..k {
                residual[(t.row, c)] -= t.val * b[(t.col, c)];
            }
        }
        let correction = lu.solve(&residual);
        b += &correction;
        let s = SMatrix::from_fn(k, |q, p| b[(self.external[q], p)]);
        if s.values.iter().any(|v| !v.is_finite()) {
            return Err(singular());
        }
        Ok(s)
    }

    /// The circuit's S-matrix at `wavelength` by Filipsson's sub-network growth (see the module's
    /// docs): the reference for [`Circuit::s_matrix`], dense, and slower by far on a large
    /// netlist.
    ///
    /// Starting from the block-diagonal S of every port (his Eq. 7), each connection between
    /// ports k and l, in the netlist's order, replaces every other S_ij by his Eq. 6,
    ///
    /// S_ij + [S_il S_kj (1 − S_lk) + S_il S_kk S_lj + S_ik S_lj (1 − S_kl) + S_ik S_ll S_kj]
    /// / [(1 − S_kl)(1 − S_lk) − S_kk S_ll],
    ///
    /// and drops ports k and l.
    ///
    /// # Errors
    ///
    /// As [`Circuit::s_matrix`], and [`Error::InvalidValue`] if a connection's denominator
    /// vanishes.
    pub fn s_matrix_by_growth(&self, wavelength: Wavelength) -> Result<SMatrix> {
        let blocks = self.blocks(wavelength, &self.values)?;
        let n = self.owner.len();
        // the ports still open, by number, and S between them
        let mut open: Vec<usize> = (0..n).collect();
        let mut s = vec![vec![c64::new(0.0, 0.0); n]; n];
        for (g, &(instance, p)) in self.owner.iter().enumerate() {
            let start = self.offsets[instance];
            for q in 0..blocks[instance].size() {
                s[start + q][g] = blocks[instance][(q, p)];
            }
        }
        let one = c64::new(1.0, 0.0);
        for (a, b) in self.netlist.connections() {
            let position = |r| -> Result<usize> {
                let (i, p) = self.netlist.resolve(r)?;
                let g = self.offsets[i] + p;
                open.iter()
                    .position(|&o| o == g)
                    .ok_or_else(|| Error::invalid("circuit", format!("{r} is joined twice")))
            };
            let (k, l) = (position(a)?, position(b)?);
            let (skk, skl, slk, sll) = (s[k][k], s[k][l], s[l][k], s[l][l]);
            let denominator = (one - skl) * (one - slk) - skk * sll;
            if denominator.norm() == 0.0 {
                return Err(Error::invalid(
                    "circuit",
                    format!("sub-network growth divides by zero at {wavelength}"),
                ));
            }
            let keep: Vec<usize> = (0..open.len()).filter(|&i| i != k && i != l).collect();
            let grown: Vec<Vec<c64>> = keep
                .iter()
                .map(|&i| {
                    keep.iter()
                        .map(|&j| {
                            let (sik, sil, skj, slj) = (s[i][k], s[i][l], s[k][j], s[l][j]);
                            s[i][j]
                                + (sil * skj * (one - slk)
                                    + sil * skk * slj
                                    + sik * slj * (one - skl)
                                    + sik * sll * skj)
                                    / denominator
                        })
                        .collect()
                })
                .collect();
            s = grown;
            open = keep.iter().map(|&i| open[i]).collect();
        }
        // what is left is the external ports, in the order of their numbers
        let at = self
            .external
            .iter()
            .map(|g| {
                open.iter()
                    .position(|o| o == g)
                    .ok_or_else(|| Error::invalid("circuit", "an external port was joined"))
            })
            .collect::<Result<Vec<usize>>>()?;
        Ok(SMatrix::from_fn(self.external.len(), |q, p| {
            s[at[q]][at[p]]
        }))
    }
}

impl Component for Circuit {
    fn kind(&self) -> &str {
        "circuit"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        self.solve(wavelength, values)
    }

    /// The coarsest fidelity among its components (analytic, then compact, 2D, 3D, measured),
    /// and no stated error: errors don't simply add through a circuit, and a resonance
    /// magnifies them.
    fn provenance(&self) -> Provenance {
        let fidelities = self
            .netlist
            .instances()
            .iter()
            .map(|i| i.component.provenance().fidelity);
        let rank = |f: Fidelity| match f {
            Fidelity::Analytic => 0,
            Fidelity::Compact => 1,
            Fidelity::TwoD => 2,
            Fidelity::ThreeD => 3,
            Fidelity::Measured => 4,
        };
        let fidelity = fidelities
            .min_by_key(|&f| rank(f))
            .unwrap_or(Fidelity::Analytic);
        Provenance {
            fidelity,
            source: format!(
                "a circuit of {} instances, solved as one sparse system",
                self.netlist.instances().len()
            ),
            error: None,
            validity: None,
        }
    }

    fn reciprocal(&self) -> bool {
        self.netlist
            .instances()
            .iter()
            .all(|i| i.component.reciprocal())
    }
}
