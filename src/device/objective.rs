//! A device's objectives and constraints ([`Device::objectives`], [`Device::constraints`]): the
//! quantities an optimization of it makes small, large or near a target, and keeps within
//! limits.
//!
//! Each is an expression ([`crate::expr`]) of the device's **outputs** at a design, at each
//! wavelength (and corner) it is taken at ([`outputs`]):
//!
//! - `T_q_p`: |S_qp|², the power from port p into port q, e.g. `T_o3_o1`;
//! - each figure of its model by its symbol ([`Device::derived`]), e.g. `n_g`, `FSR`, `Q`;
//! - each of its parameters by name, e.g. `gap`, `loss` (a figure of the same name hides it);
//! - `lambda`, the wavelength, µm.
//!
//! Every output is a plain number in its own unit (a length in µm, the FSR in nm): an
//! expression writes `gap >= 0.1`, not `100 nm`. A device declares the objectives and
//! constraints its page offers; a job may also give an expression of its own.
//!
//! **Derivatives.** An expression of the powers and the parameters has exact derivatives in a
//! parameter wherever the device's component does ([`super::derivative`]: a coupler's length, a
//! ring's radius, an MZI's arms through the circuit adjoint), by forward mode through the
//! expression ([`crate::expr::Expr::eval_dual`]). A figure, or a parameter the component is built
//! from (a width, a gap), is differentiated by central differences. An optimization checks every
//! exact derivative against fourth-order differences before its optimizer uses it
//! ([`crate::job`]'s `"optimize"` task).
//!
//! docs/methods/optimization.md describes them, with the validation of each.

use crate::circuit::Fidelity;
use crate::expr::{Dim, Expr, Scope};
use crate::{Error, Result};

use super::{Built, Device, Values};

/// What an optimization does with a quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Goal {
    /// Make it as small as it goes.
    Minimize,
    /// Make it as large as it goes.
    Maximize,
    /// Bring it to a target: (value − target)² is made small.
    Target,
}

impl Goal {
    /// Its name in a job: `"minimize"`, `"maximize"` or `"target"`.
    pub fn name(self) -> &'static str {
        match self {
            Goal::Minimize => "minimize",
            Goal::Maximize => "maximize",
            Goal::Target => "target",
        }
    }

    /// The goal named `name`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another name.
    pub fn named(name: &str) -> Result<Goal> {
        match name {
            "minimize" => Ok(Goal::Minimize),
            "maximize" => Ok(Goal::Maximize),
            "target" => Ok(Goal::Target),
            other => Err(Error::invalid(
                "objective",
                format!("a goal is \"minimize\", \"maximize\" or \"target\", not \"{other}\""),
            )),
        }
    }
}

/// How a quantity taken at several wavelengths (and corners) becomes one number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reduce {
    /// The worst: the largest cost of an objective, the value nearest breaking a constraint.
    Worst,
    /// The average.
    Average,
    /// The largest value less the smallest: how far it moves over the band (a flatness).
    Spread,
}

impl Reduce {
    /// Its name in a job: `"worst"`, `"average"` or `"spread"`.
    pub fn name(self) -> &'static str {
        match self {
            Reduce::Worst => "worst",
            Reduce::Average => "average",
            Reduce::Spread => "spread",
        }
    }

    /// The reduction named `name`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another name.
    pub fn named(name: &str) -> Result<Reduce> {
        match name {
            "worst" => Ok(Reduce::Worst),
            "average" => Ok(Reduce::Average),
            "spread" => Ok(Reduce::Spread),
            other => Err(Error::invalid(
                "objective",
                format!(
                    "a reduction over the band is \"worst\", \"average\" or \"spread\", not \"{other}\""
                ),
            )),
        }
    }
}

/// Which side of its bound a constraint keeps a quantity on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Limit {
    /// At most the bound.
    AtMost,
    /// At least the bound.
    AtLeast,
}

impl Limit {
    /// Its name in a job: `"at_most"` or `"at_least"`.
    pub fn name(self) -> &'static str {
        match self {
            Limit::AtMost => "at_most",
            Limit::AtLeast => "at_least",
        }
    }
}

/// One of a device's objectives: a quantity of its outputs to make small, large, or near a
/// target.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Objective {
    /// What it is, e.g. `"split ratio"`.
    pub name: String,
    /// Its name in a job, unique among the device's objectives, e.g. `"split"`.
    pub symbol: String,
    /// Its unit, e.g. `"dB"`, or `""`.
    pub unit: String,
    /// The expression of the device's outputs it is, e.g. `"T_o3_o1/(T_o3_o1 + T_o4_o1)"`.
    pub expression: String,
    /// What is done with it.
    pub goal: Goal,
    /// The target it starts with, for a [`Goal::Target`], e.g. 0.5 for a split ratio.
    pub target: Option<f64>,
    /// The reduction over the band it always takes (a flatness is a spread); `None` takes the
    /// task's.
    pub reduce: Option<Reduce>,
    /// What it means, in a sentence.
    pub about: String,
}

/// One of a device's constraints: a quantity of its outputs kept at most or at least a bound.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Constraint {
    /// What it is, e.g. `"smallest gap"`.
    pub name: String,
    /// Its name in a job, unique among the device's constraints, e.g. `"gap"`.
    pub symbol: String,
    /// Its unit, e.g. `"µm"`, or `""`.
    pub unit: String,
    /// The expression of the device's outputs it bounds, e.g. `"gap"`.
    pub expression: String,
    /// Which side of the bound it keeps.
    pub limit: Limit,
    /// The bound it starts with, e.g. 0.1 µm for a foundry's smallest gap.
    pub bound: f64,
    /// What it means, in a sentence.
    pub about: String,
}

/// One of the outputs an expression may name.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Output {
    /// Its name in an expression, e.g. `"T_o3_o1"`.
    pub name: String,
    /// Its unit, e.g. `"nm"`, or `""`.
    pub unit: String,
    /// What it is, e.g. `"power from o1 into o3"`.
    pub about: String,
    /// Whether it changes with the wavelength (a power) or is the design's (a figure, a
    /// parameter).
    pub per_wavelength: bool,
}

#[allow(clippy::too_many_arguments)]
fn objective(
    name: &str,
    symbol: &str,
    unit: &str,
    expression: &str,
    goal: Goal,
    target: Option<f64>,
    reduce: Option<Reduce>,
    about: &str,
) -> Objective {
    Objective {
        name: name.into(),
        symbol: symbol.into(),
        unit: unit.into(),
        expression: expression.into(),
        goal,
        target,
        reduce,
        about: about.into(),
    }
}

fn constraint(
    name: &str,
    symbol: &str,
    unit: &str,
    expression: &str,
    limit: Limit,
    bound: f64,
    about: &str,
) -> Constraint {
    Constraint {
        name: name.into(),
        symbol: symbol.into(),
        unit: unit.into(),
        expression: expression.into(),
        limit,
        bound,
        about: about.into(),
    }
}

/// The round-trip power transmission of a ring, a², from its guide's loss (dB/cm) over its round
/// trip L (µm) and its extra round-trip loss (dB).
const ROUND_TRIP: &str = "10^(-(loss*L*1e-4 + round_trip_loss)/10)";

/// The objectives the library's devices of `kind` declare.
pub(super) fn objectives_of(kind: &str) -> Vec<Objective> {
    use Goal::{Maximize, Minimize, Target};
    let loss = |a: &str, b: &str| format!("-10*log10(T_{b}_{a})");
    let split = |a: &str, b: &str, c: &str| {
        (
            format!("-10*log10(T_{b}_{a} + T_{c}_{a})"),
            format!("abs(10*log10(T_{b}_{a}/T_{c}_{a}))"),
        )
    };
    match kind {
        "waveguide" | "rib waveguide" => vec![
            objective(
                "insertion loss",
                "loss",
                "dB",
                &loss("o1", "o2"),
                Minimize,
                None,
                None,
                "the power lost along the guide, o1 to o2",
            ),
            objective(
                "group index",
                "n_g",
                "",
                "n_g",
                Target,
                Some(4.2),
                None,
                "the mode's group index brought to a target (a delay line's, a ring's FSR)",
            ),
            objective(
                "effective index",
                "n_eff",
                "",
                "n_eff",
                Target,
                Some(2.4),
                None,
                "the mode's effective index brought to a target (phase matching)",
            ),
        ],
        "bend" => vec![
            objective(
                "insertion loss",
                "loss",
                "dB",
                &loss("o1", "o2"),
                Minimize,
                None,
                None,
                "the power lost through the bend, o1 to o2, at its radius and angle",
            ),
            objective(
                "loss per 90°",
                "loss_90",
                "dB",
                "loss_90",
                Minimize,
                None,
                None,
                "the loss of a quarter turn at the radius, the guide's and the radiation's",
            ),
        ],
        "directional coupler" => vec![
            objective(
                "split ratio",
                "split",
                "",
                "T_o3_o1/(T_o3_o1 + T_o4_o1)",
                Target,
                Some(0.5),
                None,
                "the share of the light that crosses, o1 to o3, of what leaves by o3 and o4",
            ),
            objective(
                "flatness of the split",
                "flatness",
                "",
                "T_o3_o1/(T_o3_o1 + T_o4_o1)",
                Minimize,
                None,
                Some(Reduce::Spread),
                "how far the split ratio moves over the band: its largest less its smallest",
            ),
            objective(
                "excess loss",
                "excess_loss",
                "dB",
                "-10*log10(T_o3_o1 + T_o4_o1)",
                Minimize,
                None,
                None,
                "the power that leaves by neither output",
            ),
        ],
        "MMI 1x2" | "Y-branch" => {
            let (excess, imbalance) = split("o1", "o2", "o3");
            vec![
                objective(
                    "imbalance",
                    "imbalance",
                    "dB",
                    &imbalance,
                    Minimize,
                    None,
                    None,
                    "how far apart the two outputs' powers are, in dB",
                ),
                objective(
                    "excess loss",
                    "excess_loss",
                    "dB",
                    &excess,
                    Minimize,
                    None,
                    None,
                    "the power that leaves by neither output",
                ),
            ]
        }
        "MMI 2x2" => {
            let (excess, imbalance) = split("o1", "o3", "o4");
            vec![
                objective(
                    "imbalance",
                    "imbalance",
                    "dB",
                    &imbalance,
                    Minimize,
                    None,
                    None,
                    "how far apart the two outputs' powers are, in dB",
                ),
                objective(
                    "excess loss",
                    "excess_loss",
                    "dB",
                    &excess,
                    Minimize,
                    None,
                    None,
                    "the power that leaves by neither output",
                ),
            ]
        }
        "all-pass ring" => vec![
            objective(
                "extinction",
                "extinction",
                "dB",
                "extinction",
                Maximize,
                None,
                None,
                "how deep the through port's dip is at resonance",
            ),
            objective(
                "critical coupling",
                "critical",
                "",
                &format!("kappa2 - (1 - {ROUND_TRIP})"),
                Target,
                Some(0.0),
                None,
                "the power coupled to the bus less the power lost in a round trip: zero at \
                 critical coupling, where the dip goes to zero",
            ),
            objective(
                "loaded Q",
                "Q",
                "",
                "Q",
                Target,
                Some(1e4),
                None,
                "the resonance's loaded quality factor brought to a target",
            ),
            objective(
                "free spectral range",
                "FSR",
                "nm",
                "FSR",
                Target,
                Some(10.0),
                None,
                "the spacing of the resonances brought to a target",
            ),
        ],
        "add-drop ring" => vec![
            objective(
                "drop on resonance",
                "drop",
                "",
                "drop",
                Maximize,
                None,
                None,
                "the power into the drop port at resonance",
            ),
            objective(
                "loaded Q",
                "Q",
                "",
                "Q",
                Target,
                Some(1e4),
                None,
                "the resonance's loaded quality factor brought to a target",
            ),
            objective(
                "free spectral range",
                "FSR",
                "nm",
                "FSR",
                Target,
                Some(10.0),
                None,
                "the spacing of the resonances brought to a target",
            ),
        ],
        "MZI" => vec![
            objective(
                "power across",
                "cross",
                "",
                "T_o3_o1",
                Maximize,
                None,
                None,
                "the power from o1 into o3",
            ),
            objective(
                "power through",
                "bar",
                "",
                "T_o4_o1",
                Maximize,
                None,
                None,
                "the power from o1 into o4",
            ),
            objective(
                "extinction over the band",
                "extinction",
                "dB",
                "10*log10(T_o4_o1)",
                Maximize,
                None,
                Some(Reduce::Spread),
                "the through port's largest power over its smallest across the band, in dB",
            ),
        ],
        "Y-branch MZI" => vec![
            objective(
                "power through",
                "T",
                "",
                "T_o2_o1",
                Maximize,
                None,
                None,
                "the power from o1 into o2",
            ),
            objective(
                "extinction over the band",
                "extinction",
                "dB",
                "10*log10(T_o2_o1)",
                Maximize,
                None,
                Some(Reduce::Spread),
                "the largest power through over the smallest across the band, in dB",
            ),
        ],
        "thermo-optic phase shifter"
        | "doped thermo-optic phase shifter"
        | "in-guide thermo-optic phase shifter" => vec![
            objective(
                "power for a π shift",
                "P_pi",
                "mW",
                "P_pi",
                Minimize,
                None,
                None,
                "the heater's power a π shift takes: its efficiency, the same for any length",
            ),
            objective(
                "time constant",
                "tau",
                "µs",
                "tau",
                Minimize,
                None,
                None,
                "the phase's 1/e time, turned on",
            ),
            objective(
                "power and speed",
                "FOM",
                "mW µs",
                "P_pi*tau",
                Minimize,
                None,
                None,
                "Jacques et al.'s figure of merit P_π τ (their Eq. 3): efficiency and speed \
                 traded against each other",
            ),
            objective(
                "crosstalk",
                "crosstalk",
                "",
                "X",
                Minimize,
                None,
                None,
                "the share of the guide's phase the neighbour at the pitch takes",
            ),
        ],
        "GST phase-change phase shifter"
        | "Sb2S3 phase-change phase shifter"
        | "Sb2Se3 phase-change phase shifter" => vec![
            objective(
                "length for a π shift",
                "L_pi",
                "µm",
                "L_pi",
                Minimize,
                None,
                None,
                "the cell's length that turns the phase by π between the amorphous and the \
                 crystalline state",
            ),
            objective(
                "index change",
                "dn_eff_ac",
                "",
                "dn_eff_ac",
                Maximize,
                None,
                None,
                "the change of effective index from the amorphous to the crystalline state",
            ),
            objective(
                "crystalline loss",
                "alpha_c",
                "dB/µm",
                "alpha_c",
                Minimize,
                None,
                None,
                "the cell's loss per length in the crystalline state, the lossier one for the \
                 materials of the library",
            ),
        ],
        _ => Vec::new(),
    }
}

/// The constraints the library's devices of `kind` declare.
pub(super) fn constraints_of(kind: &str) -> Vec<Constraint> {
    use Limit::{AtLeast, AtMost};
    let smallest_width = || {
        constraint(
            "smallest width",
            "width",
            "µm",
            "width",
            AtLeast,
            0.15,
            "a foundry's smallest line: the guide no narrower",
        )
    };
    let smallest_gap = |name: &str| {
        constraint(
            &format!("smallest {}", name.replace('_', " ")),
            name,
            "µm",
            name,
            AtLeast,
            0.1,
            "a foundry's smallest space: the gap no narrower",
        )
    };
    match kind {
        "waveguide" | "rib waveguide" => vec![
            smallest_width(),
            constraint(
                "insertion loss",
                "loss",
                "dB",
                "-10*log10(T_o2_o1)",
                AtMost,
                0.1,
                "the power lost along the guide, o1 to o2, no more than the bound",
            ),
        ],
        "bend" => vec![
            smallest_width(),
            constraint(
                "footprint",
                "radius",
                "µm",
                "radius",
                AtMost,
                10.0,
                "the bend's radius no larger than the room for it",
            ),
            constraint(
                "insertion loss",
                "loss",
                "dB",
                "-10*log10(T_o2_o1)",
                AtMost,
                0.1,
                "the power lost through the bend no more than the bound",
            ),
        ],
        "directional coupler" => vec![
            smallest_width(),
            smallest_gap("gap"),
            constraint(
                "footprint",
                "length",
                "µm",
                "length",
                AtMost,
                50.0,
                "the coupled section no longer than the room for it",
            ),
            constraint(
                "excess loss",
                "excess_loss",
                "dB",
                "-10*log10(T_o3_o1 + T_o4_o1)",
                AtMost,
                0.1,
                "the power that leaves by neither output no more than the bound",
            ),
        ],
        "MMI 1x2" | "Y-branch" => vec![constraint(
            "excess loss",
            "excess_loss",
            "dB",
            "-10*log10(T_o2_o1 + T_o3_o1)",
            AtMost,
            0.5,
            "the power that leaves by neither output no more than the bound",
        )],
        "MMI 2x2" => vec![constraint(
            "excess loss",
            "excess_loss",
            "dB",
            "-10*log10(T_o3_o1 + T_o4_o1)",
            AtMost,
            0.5,
            "the power that leaves by neither output no more than the bound",
        )],
        "all-pass ring" => vec![
            smallest_width(),
            smallest_gap("gap"),
            constraint(
                "free spectral range",
                "FSR",
                "nm",
                "FSR",
                AtLeast,
                5.0,
                "the resonances at least this far apart",
            ),
            constraint(
                "loaded Q",
                "Q",
                "",
                "Q",
                AtLeast,
                5e3,
                "a resonance at least this sharp",
            ),
            constraint(
                "extinction",
                "extinction",
                "dB",
                "extinction",
                AtLeast,
                20.0,
                "a dip at least this deep",
            ),
        ],
        "add-drop ring" => vec![
            smallest_width(),
            smallest_gap("gap"),
            smallest_gap("gap_drop"),
            constraint(
                "free spectral range",
                "FSR",
                "nm",
                "FSR",
                AtLeast,
                5.0,
                "the resonances at least this far apart",
            ),
            constraint(
                "loaded Q",
                "Q",
                "",
                "Q",
                AtLeast,
                5e3,
                "a resonance at least this sharp",
            ),
        ],
        "MZI" => vec![smallest_width(), smallest_gap("gap")],
        "Y-branch MZI" => vec![smallest_width()],
        "in-guide thermo-optic phase shifter" => {
            let mut c = shifter_constraints();
            c.push(constraint(
                "p++ clear of the rib",
                "clearance",
                "µm",
                "(doped_width - width)/2",
                AtLeast,
                0.25,
                "the p++ silicon at least this far beyond the rib's sides, clear of the mode \
                 (Harris et al.'s 0.25 µm: the free carriers' loss the heat model doesn't give)",
            ));
            c
        }
        "thermo-optic phase shifter" | "doped thermo-optic phase shifter" => {
            let mut c = shifter_constraints();
            c.push(constraint(
                "smallest heater",
                "heater_width",
                "µm",
                "heater_width",
                AtLeast,
                if kind == "thermo-optic phase shifter" {
                    2.0
                } else {
                    1.0
                },
                "a foundry's narrowest heater (Jacques et al.'s: 2.0 µm of TiN, 1.0 µm of N++)",
            ));
            c
        }
        "GST phase-change phase shifter"
        | "Sb2S3 phase-change phase shifter"
        | "Sb2Se3 phase-change phase shifter" => vec![
            constraint(
                "insertion loss",
                "IL",
                "dB",
                "IL",
                AtMost,
                3.0,
                "the cell at its crystallized fraction loses no more than this",
            ),
            constraint(
                "cell length",
                "length",
                "µm",
                "length",
                AtMost,
                100.0,
                "a cell no longer than this",
            ),
            constraint(
                "thinnest film",
                "film_thickness",
                "µm",
                "film_thickness",
                AtLeast,
                0.01,
                "the thinnest film the models are checked on (10 nm)",
            ),
        ],
        _ => Vec::new(),
    }
}

/// The constraints every thermo-optic phase shifter declares.
fn shifter_constraints() -> Vec<Constraint> {
    use Limit::AtMost;
    vec![
        constraint(
            "power for a π shift",
            "P_pi",
            "mW",
            "P_pi",
            AtMost,
            30.0,
            "a π shift for no more than the driver gives",
        ),
        constraint(
            "time constant",
            "tau",
            "µs",
            "tau",
            AtMost,
            10.0,
            "the phase within 1 − 1/e of its step in no more than this",
        ),
        constraint(
            "crosstalk",
            "crosstalk",
            "",
            "X",
            AtMost,
            0.1,
            "the neighbour at the pitch takes no more than this share of the phase",
        ),
    ]
}

/// Where an output's value comes from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
    /// |S_qp|², by the ports' indices.
    Power(usize, usize),
    /// A figure of the model, by its symbol.
    Figure(String),
    /// A device parameter, by its index.
    Parameter(usize),
    /// The wavelength, µm.
    Lambda,
    /// An optimization's continuation parameter at its current stage.
    Stage,
}

/// The names an objective's expression may use at a design, each with where its value comes
/// from: the scope its expressions are parsed in.
#[derive(Clone, Debug)]
pub(crate) struct Names {
    pub(crate) scope: Scope,
    /// (slot, source), in the order defined.
    pub(crate) sources: Vec<(usize, Source)>,
    pub(crate) outputs: Vec<Output>,
}

impl Names {
    /// The outputs of `device` at `values`: its ports' powers (the geometry's ports, in its
    /// component's order), the figures `built` gives, its parameters and the wavelength.
    ///
    /// # Errors
    ///
    /// What the device's geometry refuses.
    pub(crate) fn of(device: &dyn Device, values: &Values, built: Option<&Built>) -> Result<Names> {
        let ports: Vec<String> = device
            .geometry(values)?
            .ports
            .into_iter()
            .map(|p| p.name)
            .collect();
        let mut names = Names {
            scope: Scope::new(),
            sources: Vec::new(),
            outputs: Vec::new(),
        };
        for (q, a) in ports.iter().enumerate() {
            for (p, b) in ports.iter().enumerate() {
                names.add(
                    &format!("T_{a}_{b}"),
                    Source::Power(q, p),
                    "",
                    &format!("power from {b} into {a}"),
                    true,
                );
            }
        }
        if let Some(built) = built {
            for f in &built.figures {
                names.add(
                    &f.symbol,
                    Source::Figure(f.symbol.clone()),
                    &f.unit,
                    &f.name,
                    false,
                );
            }
        }
        for (k, p) in device.parameters().iter().enumerate() {
            names.add(
                &p.name,
                Source::Parameter(k),
                &p.unit,
                &format!("the parameter {}", p.name),
                false,
            );
        }
        names.add("lambda", Source::Lambda, "µm", "the wavelength", true);
        Ok(names)
    }

    /// Adds a name, unless one hides it (a figure hides a parameter of its name).
    pub(crate) fn add(
        &mut self,
        name: &str,
        source: Source,
        unit: &str,
        about: &str,
        per_wavelength: bool,
    ) {
        if let Ok(slot) = self.scope.define(name, Some(Dim::NONE)) {
            self.sources.push((slot, source));
            self.outputs.push(Output {
                name: name.to_owned(),
                unit: unit.to_owned(),
                about: about.to_owned(),
                per_wavelength,
            });
        }
    }

    /// `text` parsed in this scope.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an expression that doesn't parse, names an output the device
    /// hasn't, or isn't a number.
    pub(crate) fn parse(&self, what: &str, text: &str) -> Result<Expr> {
        let e = Expr::parse(text, &self.scope).map_err(|e| {
            Error::invalid(
                "objective",
                format!(
                    "{what} \"{text}\": column {}: {}; it may use {}",
                    e.column,
                    e.message,
                    self.outputs
                        .iter()
                        .map(|o| o.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })?;
        if let Some(d) = e.dim()
            && d != Dim::NONE
        {
            return Err(Error::invalid(
                "objective",
                format!(
                    "{what} \"{text}\" is {}: the outputs are numbers in their units, so an \
                     expression of them is a number (write 0.1 for 100 nm of a length in µm)",
                    d.name()
                ),
            ));
        }
        Ok(e)
    }
}

/// The outputs an objective or constraint of `device` may name at `values`, with `fidelity`'s
/// figures: its ports' powers, the model's figures (none where the model fails there), its
/// parameters and the wavelength.
///
/// # Errors
///
/// What the device's geometry refuses.
pub fn outputs(device: &dyn Device, values: &Values, fidelity: Fidelity) -> Result<Vec<Output>> {
    let built = device.component(values, fidelity).ok();
    Ok(Names::of(device, values, built.as_ref())?.outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::library;

    #[test]
    fn every_declared_expression_parses_on_its_devices_outputs() {
        for device in library() {
            let values = Values::defaults(device.as_ref());
            let cheapest = device.fidelities()[0].fidelity;
            let built = device.component(&values, cheapest).ok();
            let names = Names::of(device.as_ref(), &values, built.as_ref()).unwrap();
            let objectives = device.objectives();
            let constraints = device.constraints();
            assert!(
                !objectives.is_empty(),
                "{} declares objectives",
                device.kind()
            );
            assert!(
                !constraints.is_empty(),
                "{} declares constraints",
                device.kind()
            );
            for o in &objectives {
                names
                    .parse(&o.symbol, &o.expression)
                    .unwrap_or_else(|e| panic!("{}: {e}", device.kind()));
                assert_eq!(o.goal == Goal::Target, o.target.is_some(), "{}", o.symbol);
            }
            for c in &constraints {
                names
                    .parse(&c.symbol, &c.expression)
                    .unwrap_or_else(|e| panic!("{}: {e}", device.kind()));
            }
            let mut symbols: Vec<&str> = objectives.iter().map(|o| o.symbol.as_str()).collect();
            symbols.sort_unstable();
            symbols.dedup();
            assert_eq!(symbols.len(), objectives.len(), "{}", device.kind());
        }
    }

    #[test]
    fn a_figure_hides_a_parameter_of_its_name_and_lengths_are_refused() {
        let device = crate::device::StripYBranch::new();
        let values = Values::defaults(&device);
        let built = device.component(&values, Fidelity::Analytic).unwrap();
        let names = Names::of(&device, &values, Some(&built)).unwrap();
        let slot = names.scope.slot("excess_loss").unwrap();
        let source = &names.sources.iter().find(|(s, _)| *s == slot).unwrap().1;
        assert_eq!(*source, Source::Figure("excess_loss".into()));
        let e = names.parse("a constraint", "width - 100 nm").unwrap_err();
        assert!(e.to_string().contains("a length"), "{e}");
        let e = names.parse("a constraint", "wdth").unwrap_err();
        assert!(e.to_string().contains("T_o2_o1"), "{e}");
    }
}
