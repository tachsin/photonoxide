//! The Mach–Zehnder interferometer: two splitters joined by two arms, as the netlist of its
//! devices.

use std::sync::{Arc, Mutex};

use crate::Result;
use crate::circuit::components::PhaseShifter;
use crate::circuit::{Circuit, Component, Fidelity, Netlist, Parameter};
use crate::generator::{self, Bend, Drawing, Generator, Port, SBendKind, Straight};
use crate::stack::{LayerStack, Structure};
use crate::units::Wavelength;

use super::guide::um;
use super::{
    Built, Device, Figure, Geometry, Method, StripCoupler, StripWaveguide, StripYBranch, Values,
    figure, method, parameter,
};

/// The S-bends a coupler MZI's drawing parts its guides with: their length and how far each
/// moves its guide out, µm.
const ARM_BENDS: (f64, f64) = (10.0, 2.0);

/// The largest radius of the bends a longer arm's detour is drawn with, µm.
const DETOUR_RADIUS: f64 = 5.0;

/// Draws `piece`'s shapes on `d`.
fn add(d: &mut Drawing, piece: &Drawing) {
    for i in piece.items() {
        d.add_etched(&i.layer, i.shape.clone(), i.etch);
    }
}

/// Draws an arm on `d` from the splitter's output `start` (facing 0°): `length` µm of guide
/// `width` wide whose end is `reach` µm on along x, at `start`'s height, facing 0°. Longer than
/// its reach, it detours away from the other arm (`side` +1 up, −1 down) by four quarter turns
/// of radius r and the straights between: length = 2πr + 2a + b, reach = 4r + b. Returns its end.
fn arm(
    d: &mut Drawing,
    start: &Port,
    length: f64,
    reach: f64,
    side: f64,
    width: f64,
    layer: &str,
) -> Result<Port> {
    let mut end = start.clone();
    let mut chain = |piece: Drawing, end: &mut Port| -> Result<()> {
        let placed = piece.connected("o1", end, false)?;
        *end = placed.port_named("o2")?.clone();
        add(d, &placed);
        Ok(())
    };
    let extra = length - reach;
    let turn = 2.0 * std::f64::consts::PI - 4.0;
    let r = DETOUR_RADIUS.min(extra / turn).min(reach / 4.0);
    if extra <= 0.0 || r < width {
        if reach > 0.0 {
            chain(Straight::new(um(width), um(reach)).draw(layer)?, &mut end)?;
        }
        return Ok(end);
    }
    let a = (extra - turn * r) / 2.0;
    let b = reach - 4.0 * r;
    let bend = |angle: f64| Bend::arc(um(width), um(r), angle).draw(layer);
    let straight = |l: f64| Straight::new(um(width), um(l)).draw(layer);
    for (k, &angle) in [90.0, -90.0, -90.0, 90.0].iter().enumerate() {
        chain(bend(side * angle)?, &mut end)?;
        let run = if k == 1 {
            b
        } else if k == 3 {
            0.0
        } else {
            a
        };
        if run > 0.0 {
            chain(straight(run)?, &mut end)?;
        }
    }
    Ok(end)
}

/// The MZIs drawn lately, by kind and values: a page draws the same design twice an update
/// (its geometry, then its job's scene), and an MZI's curves take milliseconds to trace.
static DRAWN: Mutex<Vec<(String, Drawing)>> = Mutex::new(Vec::new());

/// How many drawings [`DRAWN`] keeps.
const KEPT: usize = 8;

/// The splitters an MZI is built of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Splitters {
    /// Directional couplers ([`StripCoupler`]): four ports.
    Couplers,
    /// Y-branches ([`StripYBranch`]): two ports.
    YBranches,
}

/// A Mach–Zehnder interferometer on a strip's stack: a splitter, two strip arms (`upper` and
/// `lower` long, `width` wide) with a phase `phase` (degrees, an ideal shifter in the upper
/// arm: a heater, as a circuit sees it) and a combiner, solved as the netlist of its devices at
/// one fidelity, each part at that fidelity: [`StripCoupler`]s (ports `o1`, `o2` on the left,
/// `o3`, `o4` on the right, as [`crate::circuit::components::mzi`] wires them) or
/// [`StripYBranch`]es (ports `o1`, the splitter's stem, and `o2`, the combiner's, as
/// [`crate::circuit::components::mzi_y`]).
///
/// Parameters `width`, `thickness`, `upper`, `lower` (the arms' lengths), `phase`, `loss` (the
/// arms', dB/cm), the splitters' own (a coupler's `gap` and `coupler_length`; a Y-branch's
/// `excess_loss`, its outline at its defaults), `wavelength` and `span`. Its fidelities are
/// those every part has: analytic and 3D.
///
/// Its geometry is its splitters' and arms' drawn side by side in their own frames, the
/// splitter at the origin, the combiner `upper` µm on (the arms are drawn straight between
/// them, the longer one as long as the netlist takes it: a layout's bends are a chip's).
#[derive(Clone, Debug, PartialEq)]
pub struct StripMzi {
    splitters: Splitters,
    coupler: StripCoupler,
    branch: StripYBranch,
    arm: StripWaveguide,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

impl StripMzi {
    /// Its drawing at `values`: the splitter, the arms and the combiner, with the ports.
    fn drawing(&self, values: &Values) -> Result<Drawing> {
        let (_, v) = self.splitter(values)?;
        let layer = self.arm.core().to_owned();
        let w = values.at("width");
        // the splitter at the origin; a coupler's guides parted by S-bends, drawn, not modelled
        let (split, upper_out, lower_out, inputs) = match self.splitters {
            Splitters::Couplers => (
                generator::DirectionalCoupler::new(
                    um(w),
                    um(values.at("gap")),
                    um(values.at("coupler_length")),
                )
                .with_arms(um(ARM_BENDS.0), um(ARM_BENDS.1), SBendKind::Sine)
                .draw(&layer)?,
                "o3",
                "o4",
                vec!["o1", "o2"],
            ),
            Splitters::YBranches => (self.branch.drawing(&v, &layer)?, "o2", "o3", vec!["o1"]),
        };
        let mut d = Drawing::new("mzi");
        add(&mut d, &split);
        let (upper, lower) = (values.at("upper"), values.at("lower"));
        let reach = upper.min(lower);
        let up = arm(
            &mut d,
            split.port_named(upper_out)?,
            upper,
            reach,
            1.0,
            w,
            &layer,
        )?;
        arm(
            &mut d,
            split.port_named(lower_out)?,
            lower,
            reach,
            -1.0,
            w,
            &layer,
        )?;
        // the combiner: the splitter mirrored, its upper output on the upper arm's end
        let combiner = split.connected(upper_out, &up, true)?;
        add(&mut d, &combiner);
        let mut ports: Vec<Port> = inputs
            .iter()
            .map(|n| split.port_named(n).cloned())
            .collect::<Result<_>>()?;
        // the combiner's inputs, now on the right: top to bottom
        let mut right: Vec<Port> = inputs
            .iter()
            .map(|n| combiner.port_named(n).cloned())
            .collect::<Result<_>>()?;
        right.sort_by(|a, b| b.position.y.to_um().total_cmp(&a.position.y.to_um()));
        let first = ports.len() + 1;
        for (k, p) in right.iter_mut().enumerate() {
            p.name = format!("o{}", first + k);
        }
        ports.extend(right);
        for p in ports {
            d.add_port(p)?;
        }
        Ok(d)
    }

    /// An MZI of two directional couplers (500 × 220 nm strips 200 nm apart, each at its 3 dB
    /// length) and arms 150 and 100 µm long, on 220 nm SOI.
    pub fn couplers() -> StripMzi {
        StripMzi::of(
            Splitters::Couplers,
            StripCoupler::new(),
            StripYBranch::new(),
            StripWaveguide::new(),
        )
    }

    /// An MZI of two Y-branches (Zhang et al.'s) and arms 150 and 100 µm long, on 220 nm SOI.
    pub fn y_branches() -> StripMzi {
        StripMzi::of(
            Splitters::YBranches,
            StripCoupler::new(),
            StripYBranch::new(),
            StripWaveguide::new(),
        )
    }

    /// The MZI of couplers (`y_branches` false) or Y-branches drawn on the layer `core` of
    /// `stack`.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str, y_branches: bool) -> Result<StripMzi> {
        Ok(StripMzi::of(
            if y_branches {
                Splitters::YBranches
            } else {
                Splitters::Couplers
            },
            StripCoupler::on(stack.clone(), core)?,
            StripYBranch::on(stack.clone(), core)?,
            StripWaveguide::on(stack, core)?,
        ))
    }

    fn of(
        splitters: Splitters,
        coupler: StripCoupler,
        branch: StripYBranch,
        arm: StripWaveguide,
    ) -> StripMzi {
        let mut parameters = vec![
            parameter("width", "µm", 0.5, 0.1, 5.0),
            parameter("thickness", "µm", 0.22, 0.05, 2.0),
            parameter("upper", "µm", 150.0, 0.0, 1e5),
            parameter("lower", "µm", 100.0, 0.0, 1e5),
            parameter("phase", "°", 0.0, -360.0, 360.0),
            parameter("loss", "dB/cm", 2.7, 0.0, 1e3),
        ];
        match splitters {
            Splitters::Couplers => {
                let length = Values::defaults(&coupler).at("length");
                parameters.push(parameter("gap", "µm", 0.2, 0.02, 2.0));
                parameters.push(parameter("coupler_length", "µm", length, 0.0, 1e4));
            }
            Splitters::YBranches => {
                parameters.push(parameter("excess_loss", "dB", 0.0, 0.0, 10.0));
            }
        }
        parameters.extend(super::waveguide::band_parameters());
        let methods = methods(&coupler, &branch, &arm, splitters);
        StripMzi {
            splitters,
            coupler,
            branch,
            arm,
            parameters,
            methods,
        }
    }

    /// The splitter's device and its values at `values`.
    fn splitter(&self, values: &Values) -> Result<(&dyn Device, Values)> {
        let shared = |d: &dyn Device| -> Result<Values> {
            let mut v = Values::defaults(d);
            for name in ["width", "thickness", "wavelength", "span"] {
                v = v.with(d, name, values.at(name))?;
            }
            Ok(v)
        };
        Ok(match self.splitters {
            Splitters::Couplers => {
                let v = shared(&self.coupler)?
                    .with(&self.coupler, "gap", values.at("gap"))?
                    .with(&self.coupler, "length", values.at("coupler_length"))?
                    .with(&self.coupler, "loss", values.at("loss"))?;
                (&self.coupler, v)
            }
            Splitters::YBranches => {
                let w = values.at("width");
                let mut v = shared(&self.branch)?.with(
                    &self.branch,
                    "excess_loss",
                    values.at("excess_loss"),
                )?;
                // the outline's end holds the two arms
                let last = format!("w{}", super::ybranch::POINTS);
                if v.at(&last) < 2.0 * w {
                    v = v.with(&self.branch, &last, 2.0 * w)?;
                }
                (&self.branch, v)
            }
        })
    }

    /// An arm's values at `values`: `length` µm of the strip.
    fn arm_values(&self, values: &Values, length: f64) -> Result<Values> {
        let mut v = Values::defaults(&self.arm);
        for name in ["width", "thickness", "loss", "wavelength", "span"] {
            v = v.with(&self.arm, name, values.at(name))?;
        }
        v.with(&self.arm, "length", length)
    }

    /// The netlist at `fidelity` and `values`, compiled, its instances' values set.
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    pub fn circuit(&self, values: &Values, fidelity: Fidelity) -> Result<Circuit> {
        values.check(self)?;
        method(self.kind(), &self.methods, fidelity)?;
        let (device, v) = self.splitter(values)?;
        let split = device.component(&v, fidelity)?;
        let upper = self
            .arm
            .component(&self.arm_values(values, values.at("upper"))?, fidelity)?;
        let lower = self
            .arm
            .component(&self.arm_values(values, values.at("lower"))?, fidelity)?;
        let names = |c: &dyn Component| -> Vec<String> {
            c.parameters().iter().map(|p| p.name.clone()).collect()
        };
        let (split_names, arm_names) = (
            names(split.component.as_ref()),
            names(upper.component.as_ref()),
        );
        let splitter: Arc<dyn Component> = Arc::from(split.component);
        let mut n = Netlist::new();
        n.add("splitter", splitter.clone())?;
        n.add("combiner", splitter)?;
        n.add("upper", Arc::from(upper.component))?;
        n.add("lower", Arc::from(lower.component))?;
        n.add("shift", Arc::new(PhaseShifter::new()))?;
        for instance in ["splitter", "combiner"] {
            for (name, &x) in split_names.iter().zip(&split.values) {
                n.set(instance, name, x)?;
            }
        }
        for (name, (&u, &l)) in arm_names.iter().zip(upper.values.iter().zip(&lower.values)) {
            n.set("upper", name, u)?;
            n.set("lower", name, l)?;
        }
        n.set("shift", "phase", values.at("phase").to_radians())?;
        match self.splitters {
            Splitters::Couplers => {
                n.connect("splitter.o3", "upper.o1")?;
                n.connect("upper.o2", "shift.o1")?;
                n.connect("shift.o2", "combiner.o2")?;
                n.connect("splitter.o4", "lower.o1")?;
                n.connect("lower.o2", "combiner.o1")?;
                n.expose("o1", "splitter.o1")?;
                n.expose("o2", "splitter.o2")?;
                n.expose("o3", "combiner.o3")?;
                n.expose("o4", "combiner.o4")?;
            }
            Splitters::YBranches => {
                n.connect("splitter.o2", "upper.o1")?;
                n.connect("upper.o2", "shift.o1")?;
                n.connect("shift.o2", "combiner.o2")?;
                n.connect("splitter.o3", "lower.o1")?;
                n.connect("lower.o2", "combiner.o3")?;
                n.expose("o1", "splitter.o1")?;
                n.expose("o2", "combiner.o1")?;
            }
        }
        n.compile()
    }
}

/// The MZI's models: the fidelities its splitter and its arms both have.
fn methods(
    coupler: &StripCoupler,
    branch: &StripYBranch,
    arm: &StripWaveguide,
    splitters: Splitters,
) -> Vec<Method> {
    let split: &dyn Device = match splitters {
        Splitters::Couplers => coupler,
        Splitters::YBranches => branch,
    };
    split
        .fidelities()
        .iter()
        .filter(|m| arm.fidelities().iter().any(|a| a.fidelity == m.fidelity))
        .map(|m| Method {
            fidelity: m.fidelity,
            name: format!(
                "the netlist of its devices at the {} fidelity, solved as a circuit: the \
                 splitters' ({}) and the arms' ({})",
                m.fidelity,
                m.name,
                arm.fidelities()
                    .iter()
                    .find(|a| a.fidelity == m.fidelity)
                    .map_or("", |a| a.name.as_str())
            ),
            write_up: "devices".into(),
            papers: m.papers.clone(),
            error: None,
            validity: Vec::new(),
        })
        .collect()
}

impl Device for StripMzi {
    fn kind(&self) -> &str {
        match self.splitters {
            Splitters::Couplers => "MZI",
            Splitters::YBranches => "Y-branch MZI",
        }
    }

    fn about(&self) -> &str {
        match self.splitters {
            Splitters::Couplers => {
                "A Mach–Zehnder interferometer of two directional couplers and two strip arms, \
                 solved as the netlist of its devices."
            }
            Splitters::YBranches => {
                "A Mach–Zehnder interferometer of two Y-branches and two strip arms, solved as \
                 the netlist of its devices."
            }
        }
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        let (device, v) = self.splitter(values)?;
        device.stack(&v)
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        values.check(self)?;
        let (device, v) = self.splitter(values)?;
        let stack = device.stack(&v)?;
        let key = format!(
            "{}|{}|{:?}",
            self.kind(),
            self.arm.core(),
            values.as_slice()
        );
        let lock = || {
            DRAWN
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        };
        let kept = lock()
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, d)| d.clone());
        let d = match kept {
            Some(d) => d,
            None => {
                let d = self.drawing(values)?;
                let mut kept = lock();
                kept.retain(|(k, _)| *k != key);
                kept.push((key, d.clone()));
                if kept.len() > KEPT {
                    kept.remove(0);
                }
                d
            }
        };
        super::drawn(Structure::new(stack), &d)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let circuit = self.circuit(values, fidelity)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let at = Wavelength::um(values.at("wavelength"))?;
        let s = circuit.s_matrix(at)?;
        let grid = format!("as its parts at the {f} fidelity");
        let figures: Vec<Figure> = match self.splitters {
            Splitters::Couplers => vec![
                figure(
                    "power through, o1 to o4",
                    "bar",
                    s.power(3, 0),
                    "",
                    f,
                    &grid,
                ),
                figure(
                    "power across, o1 to o3",
                    "cross",
                    s.power(2, 0),
                    "",
                    f,
                    &grid,
                ),
            ],
            Splitters::YBranches => vec![figure(
                "power through, o1 to o2",
                "T",
                s.power(1, 0),
                "",
                f,
                &grid,
            )],
        };
        let own = circuit.values().to_vec();
        let slot = |name: &str| circuit.parameter(name);
        let unit = |k: Option<usize>, scale: f64| {
            k.map(|k| {
                let mut d = vec![0.0; own.len()];
                d[k] = scale;
                d
            })
        };
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "upper" => unit(slot("upper.length"), 1.0),
                "lower" => unit(slot("lower.length"), 1.0),
                "phase" => unit(slot("shift.phase"), std::f64::consts::PI / 180.0),
                "loss" if self.splitters == Splitters::YBranches => {
                    let (Some(u), Some(l)) = (slot("upper.loss"), slot("lower.loss")) else {
                        return None;
                    };
                    let mut d = vec![0.0; own.len()];
                    d[u] = 1.0;
                    d[l] = 1.0;
                    Some(d)
                }
                _ => None,
            })
            .collect();
        Ok(Built {
            values: own,
            derivatives,
            figures,
            component: Box::new(circuit),
        })
    }
}
