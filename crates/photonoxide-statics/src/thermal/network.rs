//! A lumped thermal RC network: nodes with heat capacities, conductances between them and to
//! the ambient, heaters at some nodes; its exact compact model by its modes.

use super::rc::{RcModel, eigen, solve};
use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("thermal network", reason)
}

/// A lumped thermal network, as Gurses et al. 2022 draw a heater, its waveguide and the
/// substrate (their Figs. 17 to 19): nodes, each with a heat capacity (J/K, 0 for a node of no
/// mass), conductances (W/K) between nodes and from nodes to the ambient. Its temperatures above
/// the ambient follow C dT/dt = −G T + P.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Network {
    capacitances: Vec<f64>,
    /// (a, b, g): g between nodes a and b, or from a to the ambient when b is `None`.
    conductances: Vec<(usize, Option<usize>, f64)>,
}

impl Network {
    /// A network of `nodes` nodes of no mass, connected to nothing.
    pub fn new(nodes: usize) -> Network {
        Network {
            capacitances: vec![0.0; nodes],
            conductances: Vec::new(),
        }
    }

    /// How many nodes.
    pub fn nodes(&self) -> usize {
        self.capacitances.len()
    }

    fn node(&self, n: usize) -> Result<()> {
        if n >= self.nodes() {
            return Err(invalid(format!(
                "the network has {} nodes, not a node {n}",
                self.nodes()
            )));
        }
        Ok(())
    }

    /// Gives node `node` the heat capacity `joules_per_kelvin` (added to what it has).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a node the network hasn't, or a capacity that isn't finite
    /// and positive.
    pub fn capacitance(&mut self, node: usize, joules_per_kelvin: f64) -> Result<&mut Network> {
        self.node(node)?;
        if !(joules_per_kelvin.is_finite() && joules_per_kelvin > 0.0) {
            return Err(invalid(format!(
                "a heat capacity must be finite and positive, not {joules_per_kelvin} J/K"
            )));
        }
        self.capacitances[node] += joules_per_kelvin;
        Ok(self)
    }

    /// Connects nodes `a` and `b` by `watts_per_kelvin`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a node the network hasn't, a node joined to itself, or a
    /// conductance that isn't finite and positive.
    pub fn conductance(
        &mut self,
        a: usize,
        b: usize,
        watts_per_kelvin: f64,
    ) -> Result<&mut Network> {
        self.node(a)?;
        self.node(b)?;
        if a == b {
            return Err(invalid(format!("node {a} can't be joined to itself")));
        }
        self.join(a, Some(b), watts_per_kelvin)
    }

    /// Connects node `a` to the ambient by `watts_per_kelvin`.
    ///
    /// # Errors
    ///
    /// As [`Network::conductance`].
    pub fn to_ambient(&mut self, a: usize, watts_per_kelvin: f64) -> Result<&mut Network> {
        self.node(a)?;
        self.join(a, None, watts_per_kelvin)
    }

    fn join(&mut self, a: usize, b: Option<usize>, g: f64) -> Result<&mut Network> {
        if !(g.is_finite() && g > 0.0) {
            return Err(invalid(format!(
                "a conductance must be finite and positive, not {g} W/K"
            )));
        }
        self.conductances.push((a, b, g));
        Ok(self)
    }

    /// The conductance matrix G: Σ g on the diagonal, −g between the joined nodes.
    fn matrix(&self) -> Vec<Vec<f64>> {
        let n = self.nodes();
        let mut g = vec![vec![0.0; n]; n];
        for &(a, b, v) in &self.conductances {
            g[a][a] += v;
            if let Some(b) = b {
                g[b][b] += v;
                g[a][b] -= v;
                g[b][a] -= v;
            }
        }
        g
    }

    /// The steady temperatures (K above the ambient) at every node for `powers` (W) into each.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another number of powers than nodes, or a network with a
    /// part not joined to the ambient (its temperature undetermined).
    pub fn steady(&self, powers: &[f64]) -> Result<Vec<f64>> {
        if powers.len() != self.nodes() {
            return Err(invalid(format!(
                "a network of {} nodes takes {} powers",
                self.nodes(),
                self.nodes()
            )));
        }
        let g = self.matrix();
        let t = solve(g.clone(), powers.to_vec());
        // the residual of the solve shows a singular network
        let r: f64 = g
            .iter()
            .zip(powers)
            .map(|(row, p)| (row.iter().zip(&t).map(|(a, b)| a * b).sum::<f64>() - p).abs())
            .fold(0.0, f64::max);
        let scale = powers
            .iter()
            .fold(0.0f64, |a, p| a.max(p.abs()))
            .max(f64::MIN_POSITIVE);
        if r.is_nan() || r > 1e-9 * scale || t.iter().any(|v| !v.is_finite()) {
            return Err(invalid("a part of the network isn't joined to the ambient"));
        }
        Ok(t)
    }

    /// The network's exact compact model from the heaters at nodes `inputs` (W) to the
    /// temperatures at nodes `outputs` (K): its nodes of no mass eliminated (their temperatures
    /// follow the others' at once), then its modes, C^(−1/2) G C^(−1/2) = V Λ Vᵀ, each a pole
    /// τ_k = 1/λ_k with residues R_k = u_k u_kᵀ/λ_k between the nodes, u_k = C^(−1/2) v_k. Where
    /// the inputs are the outputs, every R_k is positive semidefinite: the model is passive, as
    /// the network is.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a node the network hasn't, no input or output, an output at
    /// a node of no mass (it would answer at once, which no pole gives), a network with no node
    /// of mass, or one with a part not joined to the ambient.
    pub fn model(&self, inputs: &[usize], outputs: &[usize]) -> Result<RcModel> {
        if inputs.is_empty() || outputs.is_empty() {
            return Err(invalid("a model needs an input and an output"));
        }
        for &k in inputs.iter().chain(outputs) {
            self.node(k)?;
        }
        if let Some(&o) = outputs.iter().find(|&&o| self.capacitances[o] == 0.0) {
            return Err(invalid(format!(
                "the output at node {o} has no heat capacity: it would answer at once, which no \
                 pole of a compact model gives"
            )));
        }
        let g = self.matrix();
        let mass: Vec<usize> = (0..self.nodes())
            .filter(|&k| self.capacitances[k] > 0.0)
            .collect();
        let none: Vec<usize> = (0..self.nodes())
            .filter(|&k| self.capacitances[k] == 0.0)
            .collect();
        if mass.is_empty() {
            return Err(invalid("a network needs a node with a heat capacity"));
        }
        let (nm, nz) = (mass.len(), none.len());
        // the massless nodes' T_z = G_zz⁻¹ (P_z − G_zm T_m): X = G_zz⁻¹ G_zm, and y_j = G_zz⁻¹ e_j
        let gzz: Vec<Vec<f64>> = none
            .iter()
            .map(|&a| none.iter().map(|&b| g[a][b]).collect())
            .collect();
        let column = |b: Vec<f64>| -> Vec<f64> {
            if nz == 0 {
                Vec::new()
            } else {
                solve(gzz.clone(), b)
            }
        };
        let x: Vec<Vec<f64>> = mass
            .iter()
            .map(|&m| column(none.iter().map(|&z| g[z][m]).collect()))
            .collect(); // x[m][z]
        // the reduced conductance G_mm − G_mz X
        let mut red = vec![vec![0.0; nm]; nm];
        for (i, &a) in mass.iter().enumerate() {
            for (j, &b) in mass.iter().enumerate() {
                let s: f64 = none
                    .iter()
                    .enumerate()
                    .map(|(k, &z)| g[a][z] * x[j][k])
                    .sum();
                red[i][j] = g[a][b] - s;
            }
        }
        // each input's heat into the massive nodes: e_m, or −G_mz G_zz⁻¹ e_z for a massless node
        let feed: Vec<Vec<f64>> = inputs
            .iter()
            .map(|&k| {
                if let Some(i) = mass.iter().position(|&m| m == k) {
                    let mut b = vec![0.0; nm];
                    b[i] = 1.0;
                    b
                } else {
                    let zi = none.iter().position(|&z| z == k).unwrap_or(0);
                    let mut e = vec![0.0; nz];
                    e[zi] = 1.0;
                    let y = column(e);
                    mass.iter()
                        .map(|&m| {
                            -none
                                .iter()
                                .enumerate()
                                .map(|(k, &z)| g[m][z] * y[k])
                                .sum::<f64>()
                        })
                        .collect()
                }
            })
            .collect();
        let inv_sqrt: Vec<f64> = mass
            .iter()
            .map(|&m| 1.0 / self.capacitances[m].sqrt())
            .collect();
        let a: Vec<Vec<f64>> = (0..nm)
            .map(|i| {
                (0..nm)
                    .map(|j| inv_sqrt[i] * red[i][j] * inv_sqrt[j])
                    .collect()
            })
            .collect();
        let (lambda, v) = eigen(&a);
        let scale = lambda.iter().fold(0.0f64, |m, l| m.max(l.abs()));
        if lambda.iter().any(|&l| l.is_nan() || l <= 1e-12 * scale) {
            return Err(invalid("a part of the network isn't joined to the ambient"));
        }
        let at: Vec<usize> = outputs
            .iter()
            .map(|o| mass.iter().position(|m| m == o).unwrap_or(0))
            .collect();
        let mut taus = Vec::with_capacity(nm);
        let mut residues = Vec::with_capacity(nm);
        for k in 0..nm {
            let u: Vec<f64> = (0..nm).map(|i| inv_sqrt[i] * v[i][k]).collect();
            let into: Vec<f64> = feed
                .iter()
                .map(|b| u.iter().zip(b).map(|(a, b)| a * b).sum::<f64>())
                .collect();
            taus.push(1.0 / lambda[k]);
            residues.push(
                at.iter()
                    .map(|&i| into.iter().map(|w| u[i] * w / lambda[k]).collect())
                    .collect(),
            );
        }
        RcModel::new(taus, residues)
    }
}
