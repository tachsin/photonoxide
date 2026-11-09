//! Loss budgets: what share of photons a path keeps, stage by stage, with where each number comes
//! from; per photon (a source's efficiency, its path, a detector's) and per n-photon event; and the
//! published thresholds and tests they are held against, each with its assumptions.
//!
//! - A path's budget through a circuit is the product of |S_qp|² of each component it crosses,
//!   at the components' own fidelities ([`Budget::along`]); for a path that is the only route
//!   between two ports, it equals |S_out,in|² of the whole circuit.
//! - An n-photon event, every photon kept, is the product of the photons' budgets
//!   ([`event`]).
//! - Thresholds ([`Threshold`]): M. Varnava, D. E. Browne, T. Rudolph, Phys. Rev. Lett. 100,
//!   060502 (2008), doi:10.1103/PhysRevLett.100.060502: η_S η_D > 2/3 for a source of
//!   efficiency η_S and detectors of η_D; S. Bartolucci et al., Nat. Commun. 14, 912 (2023),
//!   doi:10.1038/s41467-023-36493-1: 10.4% of loss per fusion, 2.7% per photon.
//! - H. Qi, D. J. Brod, N. Quesada, R. García-Patrón, Phys. Rev. Lett. 124, 100502 (2020),
//!   doi:10.1103/PhysRevLett.124.100502: a Gaussian boson sampler of K squeezed modes is
//!   classically simulable to a total variation distance ε when its squeezing r, transmission η
//!   and detectors (η_D, dark counts p_D) satisfy their Eq. 1 ([`qi_simulable_error`]).

use std::collections::{HashMap, HashSet};

use crate::circuit::{Circuit, Netlist, PortRef, SMatrix};
use crate::units::Wavelength;
use crate::{Error, Result};

/// One stage of a budget: what it keeps of the power, and where that number comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    /// What the stage is: a component and its ports, a source, a detector.
    pub name: String,
    /// The share of the photons it keeps, in [0, 1].
    pub transmission: f64,
    /// Where the number comes from: a component's fidelity and grid, a paper, a measurement.
    pub source: String,
}

/// A loss budget: stages in order, each with its transmission and its source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Budget {
    stages: Vec<Stage>,
}

/// A step of a path through a circuit: an instance, the port the light enters it by and the port
/// it leaves by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hop {
    /// The instance's name in the circuit's netlist.
    pub instance: String,
    /// The port it enters by.
    pub from: String,
    /// The port it leaves by.
    pub to: String,
}

impl Hop {
    /// A hop through `instance` from port `from` to port `to`.
    pub fn new(instance: &str, from: &str, to: &str) -> Hop {
        Hop {
            instance: instance.into(),
            from: from.into(),
            to: to.into(),
        }
    }
}

impl Budget {
    /// An empty budget: it keeps everything.
    pub fn new() -> Budget {
        Budget::default()
    }

    /// The same budget with one more stage.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a transmission outside [0, 1].
    pub fn with(mut self, name: &str, transmission: f64, source: &str) -> Result<Budget> {
        if !(0.0..=1.0).contains(&transmission) {
            return Err(invalid(format!(
                "a stage keeps a share in [0, 1] of the photons, got {transmission} for {name}"
            )));
        }
        self.stages.push(Stage {
            name: name.into(),
            transmission,
            source: source.into(),
        });
        Ok(self)
    }

    /// The budget of a path through `circuit` at `wavelength`: one stage per hop, |S_to,from|²
    /// of the instance's component at its values, its source the component's provenance
    /// (fidelity and source). Each hop must leave by a port connected to the next hop's entry.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an instance or port the netlist doesn't have, hops that aren't
    /// connected, or a component that gains power; the components' own errors.
    pub fn along(circuit: &Circuit, path: &[Hop], wavelength: Wavelength) -> Result<Budget> {
        let netlist = circuit.netlist();
        let mut budget = Budget::new();
        for (k, hop) in path.iter().enumerate() {
            let instance = netlist
                .instance(&hop.instance)
                .ok_or_else(|| invalid(format!("no instance {}", hop.instance)))?;
            let port = |name: &str| {
                instance
                    .port(name)
                    .ok_or_else(|| invalid(format!("{} has no port {name}", hop.instance)))
            };
            let (p, q) = (port(&hop.from)?, port(&hop.to)?);
            if let Some(next) = path.get(k + 1) {
                let joined = netlist.connections().iter().any(|(a, b)| {
                    let (out, into) = (
                        (a.instance == hop.instance && a.port == hop.to),
                        (b.instance == next.instance && b.port == next.from),
                    );
                    let (out2, into2) = (
                        (b.instance == hop.instance && b.port == hop.to),
                        (a.instance == next.instance && a.port == next.from),
                    );
                    (out && into) || (out2 && into2)
                });
                if !joined {
                    return Err(invalid(format!(
                        "{}.{} isn't connected to {}.{}",
                        hop.instance, hop.to, next.instance, next.from
                    )));
                }
            }
            let s = instance.component.s_matrix(wavelength, &instance.values)?;
            let provenance = instance.component.provenance();
            budget = budget.with(
                &format!("{} {}→{}", hop.instance, hop.from, hop.to),
                s.power(q, p),
                &format!("{}: {}", provenance.fidelity, provenance.source),
            )?;
        }
        Ok(budget)
    }

    /// The stages, in order.
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    /// The share kept by every stage together: the product of their transmissions.
    pub fn transmission(&self) -> f64 {
        self.stages.iter().map(|s| s.transmission).product()
    }

    /// The loss, dB: −10 log₁₀ of [`Budget::transmission`].
    pub fn loss_db(&self) -> f64 {
        -10.0 * self.transmission().log10()
    }

    /// The probability that n photons, each with this budget, all arrive: its transmission to
    /// the n-th power.
    pub fn event(&self, photons: u32) -> f64 {
        self.transmission().powi(photons as i32)
    }
}

/// The routes a photon can take through `circuit` from its external port `from` to its external
/// port `to` at `wavelength`, each as the hops of [`Budget::along`]: the paths that cross no
/// instance twice and whose every hop carries power, |S_to,from|² > 0 of the instance's
/// component at its values. They are found depth first, each instance's ports in order, and at
/// most `most` of them are returned.
///
/// A route's budget is its own: where there are several, |S_to,from|² of the whole circuit holds
/// their interference, and it equals a route's budget only for the only route. A loop (a ring
/// from parts) is crossed once: the light that goes round it again isn't a route.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a name that isn't an external port, or `from` equal to `to`; the
/// components' own errors.
pub fn routes(
    circuit: &Circuit,
    from: &str,
    to: &str,
    wavelength: Wavelength,
    most: usize,
) -> Result<Vec<Vec<Hop>>> {
    let netlist = circuit.netlist();
    let external = |name: &str| {
        netlist
            .external()
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r.clone())
            .ok_or_else(|| {
                let names: Vec<&str> = netlist.external().iter().map(|(n, _)| n.as_str()).collect();
                invalid(format!(
                    "no external port {name}; the ports are {}",
                    names.join(", ")
                ))
            })
    };
    let (start, end) = (external(from)?, external(to)?);
    if from == to {
        return Err(invalid(format!(
            "a route runs from one port to another, not from {from} back to itself"
        )));
    }
    let mut walk = Walk {
        netlist,
        end,
        wavelength,
        most,
        matrices: HashMap::new(),
        crossed: HashSet::new(),
        path: Vec::new(),
        found: Vec::new(),
    };
    walk.enter(&start)?;
    Ok(walk.found)
}

/// A depth-first search for [`routes`].
struct Walk<'a> {
    netlist: &'a Netlist,
    end: PortRef,
    wavelength: Wavelength,
    most: usize,
    /// Each instance's S-matrix at the wavelength, computed when it is first reached.
    matrices: HashMap<String, SMatrix>,
    /// The instances on the path so far.
    crossed: HashSet<String>,
    path: Vec<Hop>,
    found: Vec<Vec<Hop>>,
}

impl Walk<'_> {
    /// Goes on from the light entering an instance by `entry`.
    fn enter(&mut self, entry: &PortRef) -> Result<()> {
        let instance = self
            .netlist
            .instance(&entry.instance)
            .ok_or_else(|| invalid(format!("no instance {}", entry.instance)))?;
        let p = instance
            .port(&entry.port)
            .ok_or_else(|| invalid(format!("{} has no port {}", entry.instance, entry.port)))?;
        if !self.matrices.contains_key(&instance.name) {
            let s = instance
                .component
                .s_matrix(self.wavelength, &instance.values)?;
            self.matrices.insert(instance.name.clone(), s);
        }
        let powers: Vec<f64> = {
            let s = &self.matrices[&instance.name];
            (0..s.size()).map(|q| s.power(q, p)).collect()
        };
        self.crossed.insert(instance.name.clone());
        for (q, port) in instance.component.ports().iter().enumerate() {
            if self.found.len() >= self.most {
                break;
            }
            if q == p || powers.get(q).is_none_or(|&t| t <= 0.0) {
                continue;
            }
            self.path
                .push(Hop::new(&instance.name, &entry.port, &port.name));
            let leaving = PortRef {
                instance: instance.name.clone(),
                port: port.name.clone(),
            };
            if leaving == self.end {
                self.found.push(self.path.clone());
            } else if let Some(next) = self.partner(&leaving)
                && !self.crossed.contains(&next.instance)
            {
                self.enter(&next)?;
            }
            self.path.pop();
        }
        self.crossed.remove(&instance.name);
        Ok(())
    }

    /// The port wired to `port`, if a connection joins it to one.
    fn partner(&self, port: &PortRef) -> Option<PortRef> {
        self.netlist.connections().iter().find_map(|(a, b)| {
            if a == port {
                Some(b.clone())
            } else if b == port {
                Some(a.clone())
            } else {
                None
            }
        })
    }
}

/// The probability that every photon of an event arrives, each with its own budget: the product
/// of their transmissions.
pub fn event(photons: &[Budget]) -> f64 {
    photons.iter().map(Budget::transmission).product()
}

/// A published loss threshold, with the assumptions it holds under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Threshold {
    /// Varnava, Browne and Rudolph (2008): efficient linear-optical computing on loss-tolerant
    /// tree clusters needs η_S η_D > 2/3.
    Varnava2008,
    /// Bartolucci et al. (2023): fusion-based computing on a (2,2)-Shor encoded 6-ring network
    /// tolerates 10.4% of loss per fusion, 2.7% per photon.
    Bartolucci2023,
}

impl Threshold {
    /// The threshold on a photon's transmission: the product of source and detector
    /// efficiencies (Varnava), or one less the loss per photon (Bartolucci).
    pub fn transmission(self) -> f64 {
        match self {
            Threshold::Varnava2008 => 2.0 / 3.0,
            Threshold::Bartolucci2023 => 1.0 - 0.027,
        }
    }

    /// How far a photon's transmission `eta` is above the threshold (negative below it).
    pub fn margin(self, eta: f64) -> f64 {
        eta - self.transmission()
    }

    /// What the threshold assumes, as its paper states it.
    pub fn assumptions(self) -> &'static str {
        match self {
            Threshold::Varnava2008 => {
                "sources that give at most one photon, in a mixture η_S|1⟩⟨1| + (1 − η_S)|0⟩⟨0|; \
                 detectors modelled as a beam splitter of transmission η_D before a perfect one, \
                 number-resolving where the GHZ states are made; ideal linear optics but for \
                 absorptive loss; dark counts rare; loss-tolerant tree clusters grown by type-II \
                 fusion (Phys. Rev. Lett. 100, 060502, p. 060502-2)"
            }
            Threshold::Bartolucci2023 => {
                "the linear-optical error model: the same loss on every photon, a fusion erased \
                 when any of its 1/p_fail photons is lost, boosted fusion; the (2,2)-Shor encoded \
                 6-ring fusion network; 10.4% per fusion and 2.7% per photon (Nat. Commun. 14, 912, \
                 abstract and Fig. 4)"
            }
        }
    }
}

/// The loss rate ε of Varnava et al.'s three-photon GHZ states from sources of efficiency η_S
/// and detectors of η_D: ε = 1 − η_S/(2 − η_D η_S) (their p. 060502-2); their protocol needs
/// (1 − ε) η_D > 1/2, which is η_S η_D > 2/3.
pub fn varnava_loss(eta_s: f64, eta_d: f64) -> f64 {
    1.0 - eta_s / (2.0 - eta_d * eta_s)
}

/// The probability that a fusion of Bartolucci et al.'s linear-optical model is erased by loss,
/// 1 − η^(1/p_fail): a fusion boosted to fail with probability p_fail measures 1/p_fail photons,
/// each kept with probability η.
pub fn fusion_loss(eta: f64, p_fail: f64) -> f64 {
    1.0 - eta.powf(1.0 / p_fail)
}

/// The largest fidelity between Qi et al.'s lossy squeezed state and a classical one their
/// detectors can't tell apart: F_max = sech{½ Θ[ln((1 − 2q_D)/(η e^(−2r) + 1 − η))]}, q_D =
/// p_D/η_D, Θ the ramp (their Eq. 12).
pub fn qi_fidelity(squeezing: f64, eta: f64, eta_d: f64, dark: f64) -> f64 {
    let q = dark / eta_d;
    let x = ((1.0 - 2.0 * q) / (eta * (-2.0 * squeezing).exp() + 1.0 - eta)).ln();
    1.0 / (0.5 * x.max(0.0)).cosh()
}

/// The smallest total variation distance ε to which Qi et al.'s algorithm simulates a Gaussian
/// boson sampler of `modes` K squeezed modes of squeezing r, transmission η and detectors of
/// efficiency η_D and dark-count probability p_D: their Eq. 1, F_max > e^(−ε²/4K), holds for
/// every ε above ε* = 2√(−K ln F_max). A sampler is classically simulable to ε when ε* < ε, so
/// ε* above 1 means their test can't simulate it at all.
pub fn qi_simulable_error(squeezing: f64, eta: f64, eta_d: f64, dark: f64, modes: usize) -> f64 {
    let f = qi_fidelity(squeezing, eta, eta_d, dark);
    2.0 * (-(modes as f64) * f.ln()).max(0.0).sqrt()
}

/// Qi et al.'s transmission below which the sampler is simulable exactly at any size,
/// η_∞ = q_D (1 + coth r) (their Eq. 2, the exact-sampling condition they take from their
/// Ref. 11).
pub fn qi_exact_transmission(squeezing: f64, eta_d: f64, dark: f64) -> f64 {
    dark / eta_d * (1.0 + 1.0 / squeezing.tanh())
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("loss budget", reason)
}
