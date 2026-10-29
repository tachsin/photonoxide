//! The thermo-optic phase shifter as a device: a heater beside a guide, its phase following the
//! heater's power, at analytic, 2D and measured fidelity.

use std::f64::consts::PI;

use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::electro::{Conductor, Current};
use crate::generator::{Generator, Straight};
use crate::geometry::{Point, Shape};
use crate::stack::{Etch, Layer, LayerStack, Structure};
use crate::thermal::shifter_checks::{finer, table3};
use crate::thermal::{
    Boundary, Drive, Features, Grid, Guide, Heat, Optics, PhaseShifter, Scheme, Section, Side,
    Stepping, Tensor,
};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::active::{Driven, Measurement};
use super::guide::um;
use super::section::{core_span, with_thickness};
use super::thermo_closed::{Fin, Sheets, Slab, Stretch, thermo_optic};
use super::{
    Built, Device, EXACT, Figure, Geometry, Method, Range, Stated, Values, figure, method,
    parameter,
};

/// Which heater a [`ThermoShifter`] has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Heater {
    /// A metal strip (TiN) in the oxide above a silicon strip.
    Metal,
    /// Two doped silicon strips (N++) in a slab beside a silicon rib.
    Doped,
    /// The guide itself heated (Harris et al. 2014): a current across a doped rib, p silicon in
    /// it, p++ beyond, through heat channels to the contacts.
    InGuide,
}

/// The 2D model's region, as Jacques et al.'s: 18 µm high, its bottom (in the substrate) held at
/// the ambient, 10 W/(m²·K) to the air above the oxide, at least 40 µm wide with insulated sides
/// (wider to hold a neighbour 10 µm inside it). The in-guide heater's is Harris et al.'s model's:
/// from the buried oxide's bottom to the oxide's top, both held at the ambient.
const REGION_HEIGHT: f64 = 18.0;
/// The region's half width, µm, before a neighbour widens it.
const REGION_HALF: f64 = 20.0;
/// The air's film coefficient over the oxide, W/(m²·K).
const AIR: f64 = 10.0;

/// The in-guide heater's contacts: the slab's last 0.1 µm at each end, held at 1 and 0 V and at
/// the ambient (the made device's vias and aluminium; `shifter/harris-*`'s).
const CONTACT: f64 = 0.1;
/// Harris et al.'s doped silicon (Section 2): its measured sheet resistances, 136 Ω (p++) and
/// 13.6 kΩ (p) in the 90 nm slab and 3.87 kΩ (p) in the 220 nm ridge, as conductivities
/// σ = 1/(R_s t), S/m; silicon's dn/dT at 1550 nm (Section 1), 1/K; the oxide's index.
const SIGMA_PP: f64 = 1.0 / (136.0 * 0.09e-6);
const SIGMA_P: f64 = 1.0 / (13.6e3 * 0.09e-6);
const SIGMA_RIDGE: f64 = 1.0 / (3.87e3 * 0.22e-6);
const HARRIS_DN_DT: f64 = 1.86e-4;
const HARRIS_N_OX: f64 = 1.444;

/// The measured devices the library holds, each a file of [`Measurement`]'s.
const MEASURED: [&str; 3] = [
    include_str!("measured/jacques-2019-tin.toml"),
    include_str!("measured/jacques-2019-npp.toml"),
    include_str!("measured/harris-2014.toml"),
];

/// The catalogue's thermal conductivity of `id` by `model` at 300 K, a film `thickness` µm
/// thick (or bulk), W/(m·K).
fn conductivity(id: &str, model: &str, thickness: Option<f64>) -> Result<f64> {
    let missing = || Error::invalid("device", format!("the catalogue has no {id} by {model}"));
    let c = crate::material::catalogue::conductivity(id).ok_or_else(missing)?;
    c.model(Some(model))
        .ok_or_else(missing)?
        .conductivity(Some(300.0), thickness)
}

/// A thermo-optic phase shifter on SOI: a straight silicon guide `length` µm long and a heater
/// beside it, ports `o1` and `o2`, its component the phase the heater's `power` (mW) adds,
/// φ = π P / P_π, and the guide's loss over its length (a phase shifter of parameters `phase`
/// and `loss`, S₂₁ = 10^(−IL/20) e^(iφλ₀/λ)).
///
/// - [`ThermoShifter::metal`] (kind `"thermo-optic phase shifter"`): a metal heater (TiN),
///   `heater_width` wide and `heater_thickness` thick, `height` µm above a `width` ×
///   `thickness` strip, `cover` µm of oxide over it, air above.
/// - [`ThermoShifter::doped`] (`"doped thermo-optic phase shifter"`): a `width` × `thickness`
///   rib on a `slab` µm slab, two doped strips (N++) `heater_width` wide in the slab `buffer` µm
///   beyond the rib's sides, the slab ending past them; the oxide's top `cladding` µm above the
///   buried oxide's, air above.
/// - [`ThermoShifter::in_guide`] (`"in-guide thermo-optic phase shifter"`): the guide heated
///   itself, Harris et al. 2014's design: a `width` × `thickness` rib on a `slab` µm slab, a
///   current across it from contacts at ±`reach` µm, its p silicon `doped_width` wide across
///   the guide and p++ beyond, the slab continuous to ±`channel_start` and beyond it heat
///   channels of p++ slab, a share `fill` of the length (averaged into the slab: the 2D model is
///   a straight section's); `cover` µm of oxide over the rib, its top held at the ambient.
///
/// All take `length`, `pitch` (a neighbour's centre from the guide's, for the crosstalk),
/// `power`, `loss` (dB/cm, the propagation loss the heat model doesn't give), `wavelength` (1.5
/// to 1.6 µm: the materials' constants are given at 1550 nm) and `span`. The buried oxide is the
/// stack's (the layers under the core, as oxide); the heat and thermo-optic constants are
/// Jacques et al.'s Table 3 at 300 K (silicon's dn/dT 1.8e-4/K, the oxide's neglected), or, in
/// the guide, the catalogue's silicon films (Ju & Goodson) and silica (Powell), Jacques et al.'s
/// heat capacities and Harris et al.'s sheet resistances and dn/dT (1.86e-4/K); so the core must
/// be silicon.
///
/// - **Analytic:** closed forms (docs/methods/devices.md): the cross-section thermally all
///   oxide between the isothermal substrate and an insulated top, the heaters sheets at their
///   height, its eigenfunctions with each strip's Green's function across in closed form: the
///   rise at the guide's centre steady, at a frequency and in time; times Marcatili's
///   dn_eff/dT. In the guide, the silicon a fin carrying the current's heat (shared by the
///   stretches' series sheet resistances) to the held contacts and into the oxide, the oxide's
///   spreading closed by the slab's eigenfunctions with its top held; its time by fixed Talbot.
/// - **2D:** [`crate::thermal::PhaseShifter`] on the cross-section in Jacques et al.'s region
///   (Harris et al.'s for the in-guide heater, its current by current continuity and its Joule
///   heat the drive): the static solver's rise, each optical cell's weight by first-order
///   perturbation of the full-vector mode, the 1/e time of the phase turned on (BDF2), its −3 dB
///   frequency, and the neighbour's crosstalk (the guide's mode moved by `pitch`, a probe whose
///   own silicon isn't drawn). A 2D number: a straight section's, any length.
/// - **Measured:** a measured device's file ([`Measurement`]): Jacques et al.'s TiN and N++
///   shifters and Harris et al.'s in-guide one, or one given
///   ([`ThermoShifter::with_measurement`]), at its design's values only. A measured insertion
///   loss is the component's loss.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermoShifter {
    heater: Heater,
    stack: LayerStack,
    core: String,
    refinement: f64,
    parameters: Vec<Parameter>,
    measurements: Vec<Measurement>,
    methods: Vec<Method>,
}

impl ThermoShifter {
    /// A TiN heater over a 500 × 220 nm strip on 220 nm SOI, Jacques et al.'s fabricated design
    /// (7.5 µm wide, 2 µm above the strip, 320 µm long; 100 nm of TiN and 1 µm of oxide over it,
    /// which the paper doesn't give).
    pub fn metal() -> ThermoShifter {
        ThermoShifter::of(Heater::Metal, LayerStack::soi_220(), "Si")
    }

    /// Two N++ strips 1 µm wide in a 90 nm slab, 0.8 µm beyond a 500 × 220 nm rib, on 220 nm
    /// SOI: Jacques et al.'s design, 320 µm long, the oxide's top 3.32 µm over the buried
    /// oxide's (which the paper doesn't give).
    pub fn doped() -> ThermoShifter {
        ThermoShifter::of(Heater::Doped, LayerStack::soi_220(), "Si")
    }

    /// Harris et al.'s in-guide heater on 220 nm SOI, 61.6 µm long: a 90 nm slab, its p region
    /// 1.0 µm wide across the guide, heat channels a share 0.8/2.44 of the length; what the paper
    /// doesn't give as `shifter/harris-*` takes it: the rib 500 nm wide, the slab continuous to
    /// 1.0 µm from the guide's centre, the contacts 3.0 µm out, 10 µm of oxide over the rib.
    pub fn in_guide() -> ThermoShifter {
        ThermoShifter::of(Heater::InGuide, LayerStack::soi_220(), "Si")
    }

    /// The metal (`doped` false) or doped heater's shifter on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer, or its core isn't silicon (an
    /// index at 1.55 µm below 3: the heat model's constants are silicon's).
    pub fn on(stack: LayerStack, core: &str, doped: bool) -> Result<ThermoShifter> {
        let heater = if doped { Heater::Doped } else { Heater::Metal };
        ThermoShifter::checked(heater, stack, core)
    }

    /// The in-guide heater's shifter on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// As [`ThermoShifter::on`]'s.
    pub fn in_guide_on(stack: LayerStack, core: &str) -> Result<ThermoShifter> {
        ThermoShifter::checked(Heater::InGuide, stack, core)
    }

    fn checked(heater: Heater, stack: LayerStack, core: &str) -> Result<ThermoShifter> {
        let (layer, _, _) = stack
            .layer(core)
            .ok_or_else(|| Error::invalid("device", format!("the stack has no layer {core}")))?;
        let n = layer.material.refractive_index(Wavelength::um(1.55)?)?.re;
        if n < 3.0 {
            return Err(Error::invalid(
                "device",
                format!(
                    "a thermo-optic phase shifter's heat and thermo-optic constants are \
                     silicon's (Jacques et al.'s Table 3): its core, {core}, has an index of {n:.3} \
                     at 1.55 um"
                ),
            ));
        }
        Ok(ThermoShifter::of(heater, stack, core))
    }

    fn of(heater: Heater, stack: LayerStack, core: &str) -> ThermoShifter {
        let mut parameters = vec![
            parameter("width", "µm", 0.5, 0.2, 3.0),
            parameter("thickness", "µm", 0.22, 0.1, 1.0),
        ];
        match heater {
            Heater::Metal => parameters.extend([
                parameter("heater_width", "µm", 7.5, 0.2, 50.0),
                parameter("heater_thickness", "µm", 0.1, 0.01, 2.0),
                parameter("height", "µm", 2.0, 0.1, 10.0),
                parameter("cover", "µm", 1.0, 0.0, 10.0),
            ]),
            Heater::Doped => parameters.extend([
                parameter("slab", "µm", 0.09, 0.01, 0.5),
                parameter("heater_width", "µm", 1.0, 0.1, 20.0),
                parameter("buffer", "µm", 0.8, 0.0, 10.0),
                parameter("cladding", "µm", 3.32, 0.2, 20.0),
            ]),
            // the slab no thinner than Ju & Goodson's films (74 nm)
            Heater::InGuide => parameters.extend([
                parameter("slab", "µm", 0.09, 0.074, 0.5),
                parameter("doped_width", "µm", 1.0, 0.2, 5.0),
                parameter("channel_start", "µm", 1.0, 0.2, 10.0),
                parameter("reach", "µm", 3.0, 0.5, 20.0),
                parameter("fill", "", 0.8 / 2.44, 0.05, 1.0),
                parameter("cover", "µm", 10.0, 0.5, 20.0),
            ]),
        }
        let length = if heater == Heater::InGuide {
            61.6
        } else {
            320.0
        };
        parameters.extend([
            parameter("length", "µm", length, 1.0, 1e5),
            parameter("pitch", "µm", 5.0, 1.0, 500.0),
            parameter("power", "mW", 0.0, 0.0, 1e3),
            parameter("loss", "dB/cm", 0.0, 0.0, 1e3),
            parameter("wavelength", "µm", 1.55, 1.5, 1.6),
            parameter("span", "µm", 0.1, 0.004, 0.1),
        ]);
        let kind = kind_of(heater);
        let measurements: Vec<Measurement> = MEASURED
            .iter()
            .map(|text| Measurement::parse(text).expect("the library's measured devices parse"))
            .filter(|m| m.kind == kind)
            .collect();
        let mut d = ThermoShifter {
            heater,
            stack,
            core: core.to_owned(),
            refinement: 1.0,
            parameters,
            measurements,
            methods: Vec::new(),
        };
        d.methods = methods(&d);
        d
    }

    /// The same device, its 2D model's grids refined by `factor` (≥ 1): the heat's grading
    /// (both steps, and the growth's excess over 1) and the optical cells divided by it, as the
    /// convergence studies take them. 1, the device's own, is 20 nm at the shapes' corners to
    /// 0.5 µm (1 µm in the guide's heater) growing by 1.2, and 10 nm optical cells.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a factor below 1 or not finite.
    pub fn with_refinement(mut self, factor: f64) -> Result<ThermoShifter> {
        if !(factor.is_finite() && factor >= 1.0) {
            return Err(Error::invalid(
                "device",
                format!("a refinement is a factor of at least 1, got {factor}"),
            ));
        }
        self.refinement = factor;
        Ok(self)
    }

    /// The same device with `measurement` among its measured devices (before the library's).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a measurement of another kind, or of a design this device
    /// can't take.
    pub fn with_measurement(mut self, measurement: Measurement) -> Result<ThermoShifter> {
        measurement.check_for(self.kind(), &self.parameters)?;
        self.measurements.insert(0, measurement);
        self.methods = methods(&self);
        Ok(self)
    }

    /// Its measured devices.
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }

    /// The 2D model at `values`: the heat problem on the cross-section, its drive (the heater at
    /// 1 W/m), and its guides (the guide's mode, then the neighbour's at `pitch`).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's or make no cross-section,
    /// and the mode solver's.
    pub fn shifter(&self, values: &Values) -> Result<PhaseShifter> {
        values.check(self)?;
        let geometry = self.cross_section(values)?;
        geometry.shifter(self.refinement, values.at("wavelength"))
    }

    /// The in-guide heater's fin at `values`, its oxide's flow `spreading` times straight
    /// across's.
    #[cfg(test)]
    pub(super) fn fin(&self, values: &Values, spreading: f64) -> Result<Fin> {
        self.cross_section(values)?.fin(spreading)
    }

    /// The cross-section's numbers at `values`.
    fn cross_section(&self, values: &Values) -> Result<Section2d> {
        let (w, t) = (values.at("width"), values.at("thickness"));
        let (bottom, _) = core_span(&self.stack, &self.core)?;
        let pitch = values.at("pitch");
        Ok(match self.heater {
            Heater::Metal => {
                let heater_bottom = t + values.at("height");
                let ht = values.at("heater_thickness");
                Section2d {
                    heater: self.heater,
                    width: w,
                    thickness: t,
                    buried: bottom,
                    top: heater_bottom + ht + values.at("cover"),
                    heater_bottom,
                    heater_thickness: ht,
                    heater_width: values.at("heater_width"),
                    slab: 0.0,
                    buffer: 0.0,
                    pitch,
                    ..Section2d::NONE
                }
            }
            Heater::Doped => {
                let (slab, top) = (values.at("slab"), values.at("cladding"));
                if !(slab < t && top > t) {
                    return Err(Error::invalid(
                        "device",
                        format!(
                            "a doped shifter's slab ({slab} um) must be thinner than its rib \
                             ({t} um), and its cladding's top ({top} um) above the rib's"
                        ),
                    ));
                }
                Section2d {
                    heater: self.heater,
                    width: w,
                    thickness: t,
                    buried: bottom,
                    top,
                    heater_bottom: 0.0,
                    heater_thickness: slab,
                    heater_width: values.at("heater_width"),
                    slab,
                    buffer: values.at("buffer"),
                    pitch,
                    ..Section2d::NONE
                }
            }
            Heater::InGuide => {
                let (slab, doped) = (values.at("slab"), values.at("doped_width"));
                let (start, reach) = (values.at("channel_start"), values.at("reach"));
                if !(slab < t && w <= doped && 0.5 * doped <= start && start < reach - CONTACT) {
                    return Err(Error::invalid(
                        "device",
                        format!(
                            "an in-guide shifter's slab ({slab} um) must be thinner than its rib \
                             ({t} um), its p region ({doped} um) no narrower than the rib ({w} \
                             um) and inside the continuous slab (to {start} um from the centre), \
                             and the channels start short of the contacts (from {} um)",
                            reach - CONTACT
                        ),
                    ));
                }
                Section2d {
                    heater: self.heater,
                    width: w,
                    thickness: t,
                    buried: bottom,
                    top: t + values.at("cover"),
                    heater_thickness: slab,
                    slab,
                    pitch,
                    doped,
                    channel_start: start,
                    reach,
                    fill: values.at("fill"),
                    ..Section2d::NONE
                }
            }
        })
    }

    /// The figures at `fidelity`, the provenance of its phase, and whether its insertion loss
    /// (the figure `IL`) is a measured device's rather than `loss` over `length`.
    fn model(
        &self,
        values: &Values,
        fidelity: Fidelity,
    ) -> Result<(Vec<Figure>, Provenance, bool)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let lambda = values.at("wavelength");
        let section = self.cross_section(values)?;
        let mut figures = match m.fidelity {
            Fidelity::Analytic => section.analytic(lambda)?,
            Fidelity::TwoD => section.two_d(self.refinement, lambda)?,
            _ => {
                let measured = self
                    .measurements
                    .iter()
                    .find(|x| x.matches(values))
                    .ok_or_else(|| match self.measurements.first() {
                        Some(x) => x.require(values).unwrap_err(),
                        None => Error::invalid(
                            "device",
                            format!("a {} has no measured device", self.kind()),
                        ),
                    })?;
                if measured.figure("P_pi").is_none() {
                    return Err(Error::invalid(
                        "device",
                        format!("the device measured by {} gives no P_pi", measured.source),
                    ));
                }
                measured.figures.clone()
            }
        };
        let f = m.fidelity;
        let p_pi = figures
            .iter()
            .find(|g| g.symbol == "P_pi")
            .map(|g| (g.value, g.grid.clone()))
            .ok_or_else(|| Error::invalid("device", "the model gives no P_pi"))?;
        let power = values.at("power");
        figures.push(figure(
            "phase at the power",
            "phi",
            PI * power / p_pi.0,
            "rad",
            f,
            &p_pi.1,
        ));
        let measured_loss = figures.iter().any(|g| g.symbol == "IL");
        if !measured_loss {
            figures.push(figure(
                "insertion loss",
                "IL",
                values.at("loss") * values.at("length") * 1e-4,
                "dB",
                f,
                EXACT,
            ));
        }
        let provenance = Provenance {
            fidelity: f,
            source: format!(
                "the phase π P/P_π, P_π = {:.4} mW at {lambda} um by {} ({})",
                p_pi.0, m.name, p_pi.1
            ),
            error: None,
            validity: Some((1.5, 1.6)),
        };
        Ok((figures, provenance, measured_loss))
    }
}

/// The kind of a heater's shifter.
fn kind_of(heater: Heater) -> &'static str {
    match heater {
        Heater::Metal => "thermo-optic phase shifter",
        Heater::Doped => "doped thermo-optic phase shifter",
        Heater::InGuide => "in-guide thermo-optic phase shifter",
    }
}

/// Jacques et al. 2019, with what the device takes from it.
fn jacques() -> String {
    "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 \
     (2019), doi:10.1364/OE.27.010456: the region, the materials (Table 3) and Eqs. 4-5"
        .into()
}

/// Harris et al. 2014, with what the in-guide heater takes from it.
fn harris() -> String {
    "N. C. Harris, Y. Ma, J. Mower, T. Baehr-Jones, D. Englund, M. Hochberg, C. Galland, Opt. \
     Express 22, 10487 (2014), doi:10.1364/OE.22.010487: the design (Section 2, Fig. 1), its \
     sheet resistances and the model's held boundaries (Section 3)"
        .into()
}

/// Marcatili 1969, the analytic models' mode.
fn marcatili() -> String {
    "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969), \
     doi:10.1002/j.1538-7305.1969.tb01166.x"
        .into()
}

/// The shifter's models.
fn methods(d: &ThermoShifter) -> Vec<Method> {
    let in_guide = d.heater == Heater::InGuide;
    let (closed, against_2d, against_measured) = match d.heater {
        Heater::Metal => (
            "closed forms: the heater's rise at the guide's centre in the oxide as a slab \
             between the isothermal substrate and an insulated top (its eigenfunctions, the \
             heater's width in closed form), times Marcatili's dn_eff/dT; the oxide slab's 1/e \
             time and -3 dB frequency, heated at the heater's height",
            STATED_METAL_2D,
            STATED_METAL_MEASURED,
        ),
        Heater::Doped => (
            "closed forms: each doped strip's heat a sheet at the slab's mid-height, half the \
             power across its width, in the oxide as a slab between the isothermal substrate and \
             an insulated top (its eigenfunctions, the strips' widths in closed form), times \
             Marcatili's dn_eff/dT; the oxide slab's 1/e time and -3 dB frequency, heated in the \
             slab",
            STATED_DOPED_2D,
            STATED_DOPED_MEASURED,
        ),
        Heater::InGuide => (
            "closed forms: the silicon as a fin to the held contacts (exact in each of the ridge, \
             the p and p++ slab and the channels, each heated by its share of the current's \
             Joule heat by their series sheet resistances), its heat to the oxide straight \
             across, the oxide's spreading closed by the oxide as a slab between the buried \
             oxide's bottom and the oxide's top, both held; times Marcatili's dn_eff/dT; the \
             fin's 1/e time (fixed Talbot) and -3 dB frequency",
            STATED_IN_GUIDE_2D,
            STATED_IN_GUIDE_MEASURED,
        ),
    };
    let source = if in_guide { harris() } else { jacques() };
    let two_d = if in_guide {
        "the current across the guide by current continuity, its Joule heat the drive of the \
         static solver's rise in Harris et al.'s model's region (the channels averaged into the \
         slab, the contacts held), each optical cell's weight by first-order perturbation of the \
         full-vector mode (thermal::PhaseShifter); its step response by BDF2 and its -3 dB \
         frequency; a straight section's, any length"
    } else {
        "the static solver's rise in Jacques et al.'s region, each optical cell's weight by \
         first-order perturbation of the full-vector mode (thermal::PhaseShifter); its step \
         response by BDF2 and its -3 dB frequency; a straight section's, any length"
    };
    let mut two_d_papers = vec![source.clone()];
    if in_guide {
        two_d_papers.push(jacques());
    }
    two_d_papers.extend([
        "S. G. Johnson et al., Phys. Rev. E 65, 066611 (2002), doi:10.1103/PhysRevE.65.066611"
            .to_owned(),
        "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008), \
         doi:10.1109/JLT.2008.923643"
            .to_owned(),
    ]);
    let mut methods = vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: closed.into(),
            write_up: "devices".into(),
            papers: if in_guide {
                vec![
                    source,
                    marcatili(),
                    "J. Abate, P. P. Valkó, Int. J. Numer. Methods Eng. 60, 979 (2004), \
                     doi:10.1002/nme.995"
                        .into(),
                ]
            } else {
                vec![source, marcatili()]
            },
            error: Some(Stated {
                quantity: "P_pi".into(),
                bound: against_2d,
                unit: "%".into(),
                against: Fidelity::TwoD,
            }),
            validity: analytic_validity(d.heater),
        },
        Method {
            fidelity: Fidelity::TwoD,
            name: two_d.into(),
            write_up: "phase-shifter".into(),
            papers: two_d_papers,
            error: (!d.measurements.is_empty()).then(|| Stated {
                quantity: "P_pi".into(),
                bound: against_measured,
                unit: "%".into(),
                against: Fidelity::Measured,
            }),
            validity: Vec::new(),
        },
    ];
    if let Some(m) = d.measurements.first() {
        methods.push(Method {
            fidelity: Fidelity::Measured,
            name: format!("the device measured by {}", short(&m.source)),
            write_up: "devices".into(),
            papers: d.measurements.iter().map(|m| m.source.clone()).collect(),
            error: None,
            validity: m.validity(),
        });
    }
    methods
}

/// A source's citation, up to its first colon.
pub(super) fn short(source: &str) -> &str {
    source.split(':').next().unwrap_or(source)
}

/// The analytic models' largest error in P_π against the 2D model over their validity, %, as
/// the validation report measures it (`device/thermo-analytic-*`).
const STATED_METAL_2D: f64 = 15.0;
const STATED_DOPED_2D: f64 = 15.0;
const STATED_IN_GUIDE_2D: f64 = 15.0;
/// The 2D model's against the measured devices, %, (`device/thermo-*-measured`).
const STATED_METAL_MEASURED: f64 = 15.0;
const STATED_DOPED_MEASURED: f64 = 15.0;
const STATED_IN_GUIDE_MEASURED: f64 = 15.0;

/// Where the analytic models are checked against the 2D model.
fn analytic_validity(heater: Heater) -> Vec<Range> {
    match heater {
        Heater::Metal => vec![
            Range::new("heater_width", 1.0, 10.0),
            Range::new("height", 1.0, 3.0),
            Range::new("width", 0.4, 0.6),
        ],
        Heater::Doped => vec![
            Range::new("buffer", 0.4, 1.6),
            Range::new("heater_width", 0.5, 2.0),
            Range::new("width", 0.4, 0.6),
        ],
        Heater::InGuide => vec![
            Range::new("doped_width", 0.6, 2.0),
            // the contacts' reach, from 2 µm (the channels 0.9 µm long) to 6
            Range::new("reach", 2.0, 6.0),
            Range::new("width", 0.4, 0.6),
        ],
    }
}

/// A shifter's cross-section, µm: y = 0 at the buried oxide's top (the core's bottom), x = 0 at
/// the guide's centre.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Section2d {
    heater: Heater,
    width: f64,
    thickness: f64,
    buried: f64,
    /// The oxide's top.
    top: f64,
    /// A metal heater's bottom.
    heater_bottom: f64,
    /// A metal heater's thickness, or the doped heaters' (the slab's).
    heater_thickness: f64,
    /// A heater's width (each of the doped ones).
    heater_width: f64,
    slab: f64,
    buffer: f64,
    pitch: f64,
    /// The in-guide heater's p region's width, its channels' start and its contacts' reach
    /// (from the guide's centre), and the channels' share of the length.
    doped: f64,
    channel_start: f64,
    reach: f64,
    fill: f64,
}

impl Section2d {
    /// Nothing drawn, the fields a heater doesn't take.
    const NONE: Section2d = Section2d {
        heater: Heater::Metal,
        width: 0.0,
        thickness: 0.0,
        buried: 0.0,
        top: 0.0,
        heater_bottom: 0.0,
        heater_thickness: 0.0,
        heater_width: 0.0,
        slab: 0.0,
        buffer: 0.0,
        pitch: 0.0,
        doped: 0.0,
        channel_start: 0.0,
        reach: 0.0,
        fill: 0.0,
    };
}

/// A rectangle from x₀ to x₁ and y₀ to y₁, µm.
fn rect(x: [f64; 2], y: [f64; 2]) -> Result<Shape> {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
}

impl Section2d {
    /// The region's bottom: in the substrate, or the buried oxide's bottom for the in-guide
    /// heater.
    fn bottom(&self) -> f64 {
        match self.heater {
            Heater::InGuide => -self.buried,
            _ => self.top - REGION_HEIGHT,
        }
    }

    /// The heat's grading's largest step, µm.
    fn largest(&self) -> f64 {
        match self.heater {
            Heater::InGuide => 1.0,
            _ => 0.5,
        }
    }

    /// The doped strips' inner edge, |x|.
    fn inner(&self) -> f64 {
        0.5 * self.width + self.buffer
    }

    /// The heat problem, its drive and its guides, as [`crate::thermal::shifter_checks`]'s
    /// Jacques et al. builds them, on grids refined by `r`.
    fn shifter(&self, r: f64, lambda: f64) -> Result<PhaseShifter> {
        use table3::*;
        let (w, t) = (self.width, self.thickness);
        let half = (self.pitch + 10.0).max(REGION_HALF);
        let mut section = Section::new("SiO2");
        let mut drawn = Vec::new();
        let (drive, window) = match self.heater {
            Heater::InGuide => return self.in_guide(r, lambda),
            Heater::Metal => {
                let strip = rect([-0.5 * w, 0.5 * w], [0.0, t])?;
                let b = self.heater_bottom;
                let hw = 0.5 * self.heater_width;
                let tin = rect([-hw, hw], [b, b + self.heater_thickness])?;
                drawn.push((strip, "Si"));
                drawn.push((tin.clone(), "TiN"));
                let x = 0.5 * w + 1.0;
                (Drive::heater("TiN", tin, 1.0), ([-x, x], [-0.75, t + 0.75]))
            }
            Heater::Doped => {
                let (inner, s) = (self.inner(), self.slab);
                let outer = inner + self.heater_width;
                let slab = rect([-inner, inner], [0.0, s])?;
                let rib = rect([-0.5 * w, 0.5 * w], [0.0, t])?;
                let left = rect([-outer, -inner], [0.0, s])?;
                let right = rect([inner, outer], [0.0, s])?;
                drawn.push((slab, "slab"));
                drawn.push((rib, "Si"));
                drawn.push((left.clone(), "N++"));
                drawn.push((right.clone(), "N++"));
                let x = 0.5 * w + 1.25;
                (
                    Drive::heater("N++", left, 0.5).and(right, 0.5),
                    ([-x, x], [-0.75, t + 0.75]),
                )
            }
        };
        section.layer(self.bottom(), -self.buried, "substrate")?;
        let mut features = Features::new();
        for (shape, medium) in &drawn {
            section.draw(shape.clone(), medium);
        }
        features.extend(section.features());
        let grid = Grid::fitted(
            [-half, half],
            [self.bottom(), self.top],
            &finer(self.largest(), 0.02, 1.2, r),
            &features,
        )?;
        let mut heat = Heat::new(section.clone(), grid);
        for (m, k, c) in [
            ("SiO2", K_OX, C_OX),
            ("substrate", K_SUBSTRATE, C_SI),
            ("Si", K_WG, C_SI),
            ("slab", K_SLAB, C_SI),
            ("N++", K_NPP, C_SI),
            ("TiN", K_TIN, C_TIN),
        ] {
            heat.conductivity(m, Tensor::isotropic(k))
                .heat_capacity(m, c);
        }
        heat.boundary(Side::Bottom, Boundary::Fixed(0.0)).boundary(
            Side::Top,
            Boundary::Convective {
                coefficient: AIR,
                ambient: 0.0,
            },
        );
        let mut optics = Optics::new(Wavelength::um(lambda)?, 0.01 / r)?;
        optics
            .medium("Si", N_SI, DN_DT)
            .medium("slab", N_SI, DN_DT)
            .medium("N++", N_NPP, DN_DT)
            .medium("SiO2", N_OX, 0.0);
        let guide = Guide::new("guide", &section, window, &optics)?;
        let neighbour = guide.moved("neighbour", self.pitch);
        PhaseShifter::new(heat, vec![drive], vec![guide, neighbour])
    }

    /// The in-guide heater's heat problem, its drive and its guides, as
    /// [`crate::thermal::crosstalk_checks`]'s Harris et al. builds them (the averaged section,
    /// the oxide's top held, the contacts held): the current across the guide from 1 V at the
    /// left contact to 0 at the right, its Joule heat the drive (the efficiency is per watt, so
    /// the voltage is any).
    fn in_guide(&self, r: f64, lambda: f64) -> Result<PhaseShifter> {
        use table3::{C_OX, C_SI, N_SI};
        let (w, t, s) = (self.width, self.thickness, self.slab);
        let (k_ridge, k_slab, k_ox) = (
            conductivity("si", "ju-goodson-1999", Some(t))?,
            conductivity("si", "ju-goodson-1999", Some(s))?,
            conductivity("sio2", "powell-1966", None)?,
        );
        let (reach, start, f) = (self.reach, self.channel_start, self.fill);
        let half = (self.pitch + 10.0).max(REGION_HALF);
        let mut section = Section::new("SiO2");
        section
            .draw(rect([-reach, reach], [0.0, s])?, "p++")
            .draw(rect([-0.5 * self.doped, 0.5 * self.doped], [0.0, s])?, "p")
            .draw(rect([-0.5 * w, 0.5 * w], [0.0, t])?, "ridge")
            .draw(rect([-reach, -start], [0.0, s])?, "channels")
            .draw(rect([start, reach], [0.0, s])?, "channels");
        let mut features = Features::new();
        features.extend(section.features());
        let grid = Grid::fitted(
            [-half, half],
            [self.bottom(), self.top],
            &finer(self.largest(), 0.02, 1.2, r),
            &features,
        )?;
        let mut current = Current::new(section.clone(), grid.clone());
        for (m, sigma) in [
            ("SiO2", 1e-12),
            ("ridge", SIGMA_RIDGE),
            ("p", SIGMA_P),
            ("p++", SIGMA_PP),
            ("channels", f * SIGMA_PP),
        ] {
            current.conductivity(m, Tensor::isotropic(sigma));
        }
        let left = rect([-reach, -reach + CONTACT], [0.0, s])?;
        let right = rect([reach - CONTACT, reach], [0.0, s])?;
        current
            .contact(Conductor::shape(left.clone(), 1.0))
            .contact(Conductor::shape(right.clone(), 0.0));
        let flow = current.solve()?;
        let mut heat = Heat::new(section.clone(), grid);
        heat.sink(left, 0.0).sink(right, 0.0);
        for (m, k, c) in [
            ("SiO2", k_ox, C_OX),
            ("ridge", k_ridge, C_SI),
            ("p", k_slab, C_SI),
            ("p++", k_slab, C_SI),
            (
                "channels",
                f * k_slab + (1.0 - f) * k_ox,
                f * C_SI + (1.0 - f) * C_OX,
            ),
        ] {
            heat.conductivity(m, Tensor::isotropic(k))
                .heat_capacity(m, c);
        }
        heat.boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0));
        let mut optics = Optics::new(Wavelength::um(lambda)?, 0.01 / r)?;
        for m in ["ridge", "p", "p++", "channels"] {
            optics.medium(m, N_SI, HARRIS_DN_DT);
        }
        optics.medium("SiO2", HARRIS_N_OX, 0.0);
        let x = 0.5 * w + 1.25;
        let guide = Guide::new("guide", &section, ([-x, x], [-0.75, t + 0.75]), &optics)?;
        let neighbour = guide.moved("neighbour", self.pitch);
        PhaseShifter::new(
            heat,
            vec![Drive::joule("current", &flow)],
            vec![guide, neighbour],
        )
    }

    /// The grid the 2D figures name.
    fn grid_2d(&self, shifter: &PhaseShifter, r: f64) -> String {
        let g = shifter.heat().grid();
        format!(
            "heat on {} × {} cells graded from {:.3} nm to {:.3} um (growth {:.3}) over {} × {} \
             um; mode on {:.3} nm cells",
            g.x().cells(),
            g.y().cells(),
            20.0 / r,
            self.largest() / r,
            1.0 + 0.2 / r,
            2.0 * (self.pitch + 10.0).max(REGION_HALF),
            match self.heater {
                Heater::InGuide => self.top + self.buried,
                _ => REGION_HEIGHT,
            },
            10.0 / r
        )
    }

    /// The 2D model's figures.
    fn two_d(&self, r: f64, lambda: f64) -> Result<Vec<Figure>> {
        let shifter = self.shifter(r, lambda)?;
        let grid = self.grid_2d(&shifter, r);
        let e = shifter.efficiency()?;
        let p_pi = e.p_pi(0, 0) * 1e3;
        let step = match self.heater {
            Heater::Metal => 0.2,
            Heater::Doped | Heater::InGuide => 0.1,
        };
        // the phase turned on, its record lengthened until it gets 90 % of the way
        let mut timing = None;
        let mut steps = 300;
        for _ in 0..4 {
            let stepping = Stepping::new(step * 1e-6, steps, Scheme::Bdf2);
            match shifter.times(0, 0, &stepping, true) {
                Ok(t) => {
                    timing = Some(t);
                    break;
                }
                Err(_) => steps *= 4,
            }
        }
        let timing = timing.ok_or_else(|| {
            Error::invalid(
                "device",
                "the phase doesn't get 90% of the way to its steady value within 19 ms",
            )
        })?;
        let cutoff = shifter.cutoff(0, 0)?;
        let guide = &shifter.guides()[0];
        let f = Fidelity::TwoD;
        let time_grid = format!("{grid}; BDF2 steps of {step} us");
        Ok(vec![
            figure("power for a π shift", "P_pi", p_pi, "mW", f, &grid),
            figure(
                "1/e time, turned on",
                "tau",
                timing.one_over_e_s * 1e6,
                "µs",
                f,
                &time_grid,
            ),
            figure("−3 dB frequency", "f_3dB", cutoff * 1e-3, "kHz", f, &grid),
            figure(
                "crosstalk on the neighbour",
                "X",
                e.crosstalk(1, 0, 0),
                "",
                f,
                &grid,
            ),
            figure(
                "index change per power",
                "s",
                e.slopes[0][0],
                "m/W",
                f,
                &grid,
            ),
            figure(
                "thermo-optic coefficient",
                "dneff_dT",
                guide.thermo_optic(),
                "1/K",
                f,
                &grid,
            ),
            figure(
                "effective index",
                "n_eff",
                guide.effective_index(),
                "",
                f,
                &grid,
            ),
        ])
    }

    /// The oxide as a slab from the substrate (its silicon as the oxide of the same thermal
    /// resistance) to the oxide's top, insulated, and the isothermal plane's height, µm; for the
    /// in-guide heater from the buried oxide's bottom to the oxide's top, both held.
    fn slab(&self) -> Result<(Slab, f64)> {
        use table3::*;
        if self.heater == Heater::InGuide {
            let k = conductivity("sio2", "powell-1966", None)?;
            let plane = -self.buried;
            return Ok((
                Slab {
                    height: (self.top - plane) * 1e-6,
                    conductivity: k,
                    diffusivity: k / C_OX,
                    held: true,
                },
                plane,
            ));
        }
        let substrate = -self.buried - self.bottom();
        let plane = -self.buried - substrate * K_OX / K_SUBSTRATE;
        Ok((
            Slab {
                height: (self.top - plane) * 1e-6,
                conductivity: K_OX,
                diffusivity: K_OX / C_OX,
                held: false,
            },
            plane,
        ))
    }

    /// The heaters as sheets in the slab whose isothermal plane is at `plane` (µm): the metal
    /// strip at its mid-height, or each doped strip at the slab's, half the power in each.
    fn sheets(&self, plane: f64) -> Vec<Sheets> {
        let m = |y: f64| (y - plane) * 1e-6;
        let a = self.heater_width * 1e-6;
        match self.heater {
            Heater::Metal => vec![Sheets {
                height: m(self.heater_bottom + 0.5 * self.heater_thickness),
                strips: vec![(0.0, a, 1.0)],
            }],
            _ => {
                let c = (self.inner() + 0.5 * self.heater_width) * 1e-6;
                vec![Sheets {
                    height: m(0.5 * self.slab),
                    strips: vec![(-c, a, 0.5), (c, a, 0.5)],
                }]
            }
        }
    }

    /// The in-guide heater's stretches outward from the centre, µm: (from, to) across, and the
    /// share of the drive's Joule heat on that side. The current crosses, in series, the ridge
    /// (`width` wide, `thickness` thick; its share split between the sides) and on each side the
    /// p slab to the p region's edge, the p++ slab to the channels' start and the channels to the
    /// contact (their conductivity the fill's share of p++'s), so each takes its sheet
    /// resistance's share of their sum. Empty stretches are left out.
    fn stretches(&self) -> Vec<(f64, f64, f64)> {
        let (s, t) = (self.slab, self.thickness);
        let edges = [
            0.0,
            0.5 * self.width,
            0.5 * self.doped,
            self.channel_start,
            self.reach - CONTACT,
        ];
        let conductance = [
            SIGMA_RIDGE * t,
            SIGMA_P * s,
            SIGMA_PP * s,
            self.fill * SIGMA_PP * s,
        ];
        let resistance: Vec<f64> = (0..4)
            .map(|i| (edges[i + 1] - edges[i]) / conductance[i])
            .collect();
        let total = 2.0 * resistance.iter().sum::<f64>();
        (0..4)
            .filter(|&i| edges[i + 1] > edges[i])
            .map(|i| (edges[i], edges[i + 1], resistance[i] / total))
            .collect()
    }

    /// The in-guide heater's silicon as a fin ([`Fin`]): the ridge's stretch `thickness` thick,
    /// the rest the slab's (the channels' conductivity and heat capacity the fill's share of
    /// silicon's, the rest oxide's), each heated by its share of the Joule heat, the buried
    /// oxide under it and the oxide over it to the held top.
    fn fin(&self, spreading: f64) -> Result<Fin> {
        use table3::{C_OX, C_SI};
        let (s, t, f) = (self.slab, self.thickness, self.fill);
        let (k_ridge, k_slab, k_ox) = (
            conductivity("si", "ju-goodson-1999", Some(t))?,
            conductivity("si", "ju-goodson-1999", Some(s))?,
            conductivity("sio2", "powell-1966", None)?,
        );
        let start = self.channel_start;
        let stretches = self
            .stretches()
            .into_iter()
            .map(|(from, to, share)| {
                let (k, c, h) = if to <= 0.5 * self.width {
                    (k_ridge, C_SI, t)
                } else if from >= start {
                    (
                        f * k_slab + (1.0 - f) * k_ox,
                        f * C_SI + (1.0 - f) * C_OX,
                        s,
                    )
                } else {
                    (k_slab, C_SI, s)
                };
                let length = (to - from) * 1e-6;
                Stretch {
                    length,
                    conductance: k * h * 1e-6,
                    capacity: c * h * 1e-6,
                    heat: share / length,
                    below: self.buried * 1e-6,
                    above: (self.top - h) * 1e-6,
                }
            })
            .collect();
        Ok(Fin {
            stretches,
            oxide: (k_ox, k_ox / C_OX),
            spreading,
        })
    }

    /// The oxide's unit responses to each stretch's heat, sheets at the slab's mid-height in
    /// the slab whose isothermal plane is at `plane` (µm), and to the held contacts' sinks: the
    /// steady rise (K per W/m) at each of `points` (x, y, m) from a watt per metre spread evenly
    /// across each stretch on both sides (the ridge's one strip across the centre), and then
    /// across both contacts.
    fn unit_responses(&self, slab: &Slab, plane: f64, points: &[(f64, f64)]) -> Vec<Vec<f64>> {
        let height = (0.5 * self.slab - plane) * 1e-6;
        let xc = (self.reach - 0.5 * CONTACT) * 1e-6;
        let mut sources: Vec<Vec<(f64, f64, f64)>> = self
            .stretches()
            .iter()
            .map(|&(from, to, _)| {
                if from == 0.0 {
                    vec![(0.0, 2.0 * to * 1e-6, 1.0)]
                } else {
                    let (c, w) = (0.5 * (from + to) * 1e-6, (to - from) * 1e-6);
                    vec![(-c, w, 0.5), (c, w, 0.5)]
                }
            })
            .collect();
        sources.push(vec![(-xc, CONTACT * 1e-6, 0.5), (xc, CONTACT * 1e-6, 0.5)]);
        sources
            .into_iter()
            .map(|strips| {
                let sheet = [Sheets { height, strips }];
                points
                    .iter()
                    .map(|&(x, y)| slab.steady_accelerated(&sheet, x, y))
                    .collect()
            })
            .collect()
    }

    /// The in-guide heater's fin closed by the oxide: its spreading the one at which the fin's
    /// rise at the centre is the oxide's (the slab's eigenfunctions, exact) from the heat the fin
    /// gives it and the held contacts in it (a sink at each, the strength that holds its centre
    /// at the ambient), by bisection in its log from 1 to 100 to 1e-9; and the oxide's rise at
    /// the neighbour from the same.
    fn closed_fin(&self, slab: &Slab, plane: f64, guide: f64) -> Result<(Fin, f64)> {
        let height = (0.5 * self.slab - plane) * 1e-6;
        let xc = (self.reach - 0.5 * CONTACT) * 1e-6;
        // at the guide, at a contact and at the neighbour
        let units = self.unit_responses(
            slab,
            plane,
            &[(0.0, guide), (xc, height), (self.pitch * 1e-6, guide)],
        );
        let (contact, stretches) = units.split_last().expect("the contacts' responses");
        let oxide = |fin: &Fin| -> [f64; 3] {
            // each side's leak, both sides' heat
            let leaks: Vec<f64> = fin.leaks().iter().map(|l| 2.0 * l).collect();
            let at = |i: usize| -> f64 { leaks.iter().zip(stretches).map(|(l, u)| l * u[i]).sum() };
            let sink = -at(1) / contact[1];
            [0, 1, 2].map(|i| at(i) + sink * contact[i])
        };
        let gap = |spreading: f64| -> Result<f64> {
            let fin = self.fin(spreading)?;
            Ok(fin.steady() - oxide(&fin)[0])
        };
        let (mut lo, mut hi) = (1.0f64, 100.0f64);
        while hi / lo > 1.0 + 1e-9 {
            let mid = (lo * hi).sqrt();
            if gap(mid)? > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let fin = self.fin((lo * hi).sqrt())?;
        let neighbour = oxide(&fin)[2];
        Ok((fin, neighbour))
    }

    /// The analytic model's figures.
    fn analytic(&self, lambda: f64) -> Result<Vec<Figure>> {
        use table3::*;
        let (slab, plane) = self.slab()?;
        let guide = (0.5 * self.thickness - plane) * 1e-6;
        let (rise, crosstalk, tau, cutoff) = if self.heater == Heater::InGuide {
            // the fin's centre, and the heat it gives the oxide as sheets: their rise at the
            // neighbour over the fin's
            let (fin, neighbour) = self.closed_fin(&slab, plane, guide)?;
            let rise = fin.steady();
            (rise, neighbour / rise, fin.one_over_e(), fin.cutoff())
        } else {
            let sheets = self.sheets(plane);
            let rise = slab.steady(&sheets, 0.0, guide);
            (
                rise,
                slab.steady(&sheets, self.pitch * 1e-6, guide) / rise,
                slab.one_over_e(&sheets, 0.0, guide),
                slab.cutoff(&sheets, 0.0, guide),
            )
        };
        let (core, cladding) = match self.heater {
            Heater::InGuide => ((N_SI, HARRIS_DN_DT), (HARRIS_N_OX, 0.0)),
            _ => ((N_SI, DN_DT), (N_OX, 0.0)),
        };
        let (dn_dt, n_eff) = thermo_optic(
            self.width,
            self.thickness,
            core,
            cladding,
            Wavelength::um(lambda)?,
        )?;
        let s = dn_dt * rise;
        let p_pi = lambda * 1e-6 / (2.0 * s) * 1e3;
        let f = Fidelity::Analytic;
        Ok(vec![
            figure("power for a π shift", "P_pi", p_pi, "mW", f, EXACT),
            figure("1/e time, turned on", "tau", tau * 1e6, "µs", f, EXACT),
            figure("−3 dB frequency", "f_3dB", cutoff * 1e-3, "kHz", f, EXACT),
            figure("crosstalk on the neighbour", "X", crosstalk, "", f, EXACT),
            figure("index change per power", "s", s, "m/W", f, EXACT),
            figure(
                "thermo-optic coefficient",
                "dneff_dT",
                dn_dt,
                "1/K",
                f,
                EXACT,
            ),
            figure("effective index", "n_eff", n_eff, "", f, EXACT),
        ])
    }
}

impl Device for ThermoShifter {
    fn kind(&self) -> &str {
        kind_of(self.heater)
    }

    fn about(&self) -> &str {
        match self.heater {
            Heater::Metal => {
                "A metal heater over a silicon strip: the phase it adds follows its power, P_π \
                 for a π shift, with its time constant, bandwidth and crosstalk on a neighbour."
            }
            Heater::Doped => {
                "Doped silicon heaters in the slab beside a rib: the phase they add follows their \
                 power, P_π for a π shift, with its time constant, bandwidth and crosstalk on a \
                 neighbour."
            }
            Heater::InGuide => {
                "A doped rib heated by a current across it: the phase the guide's own heat adds \
                 follows its power, P_π for a π shift, with its time constant, bandwidth and \
                 crosstalk on a neighbour."
            }
        }
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        let stack = with_thickness(&self.stack, &self.core, values.at("thickness"))?;
        if self.heater != Heater::Metal {
            return Ok(stack);
        }
        // the oxide up to the heater, its layer and the oxide over it; the heater's optics as
        // the oxide's (the mode is solved without it)
        let (layer, _, _) = stack
            .layer(&self.core)
            .ok_or_else(|| Error::invalid("device", "the stack has no core"))?;
        let oxide = layer.background.clone();
        let mut layers = stack.layers().to_vec();
        let mut add = |name: &str, thickness: f64| {
            if thickness > 0.0 {
                layers.push(Layer {
                    name: name.into(),
                    thickness: Length::um(thickness),
                    material: oxide.clone(),
                    background: oxide.clone(),
                });
            }
        };
        add("gap", values.at("height"));
        add("TiN", values.at("heater_thickness"));
        add("cover", values.at("cover"));
        LayerStack::new(stack.substrate().clone(), layers, stack.cladding().clone())
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let (w, l, t) = (
            values.at("width"),
            values.at("length"),
            values.at("thickness"),
        );
        let drawing = Straight::new(um(w), um(l)).draw(&self.core)?;
        let drawing = match self.heater {
            Heater::Metal => drawing,
            Heater::Doped | Heater::InGuide => {
                super::etched(&drawing, Etch::partial(Length::um(t - values.at("slab"))))?
            }
        };
        let mut g = super::drawn(Structure::new(self.stack(values)?), &drawing)?;
        if self.heater == Heater::Metal {
            g.structure.draw(
                "TiN",
                Shape::rect(
                    Point::um(0.5 * l, 0.0),
                    Length::um(l),
                    Length::um(values.at("heater_width")),
                )?,
            )?;
        }
        Ok(g)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (figures, provenance, measured_loss) = self.model(values, fidelity)?;
        let value = |symbol: &str| {
            figures
                .iter()
                .find(|g| g.symbol == symbol)
                .map_or(f64::NAN, |g| g.value)
        };
        let p_pi = value("P_pi");
        // the loss over the length, or the measured device's insertion loss, which neither moves
        let (phase, loss) = (PI * values.at("power") / p_pi, value("IL"));
        let (by_loss, by_length) = if measured_loss {
            (0.0, 0.0)
        } else {
            (values.at("length") * 1e-4, values.at("loss") * 1e-4)
        };
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "power" => Some(vec![PI / p_pi, 0.0]),
                "loss" => Some(vec![0.0, by_loss]),
                "length" => Some(vec![0.0, by_length]),
                _ => None,
            })
            .collect();
        Ok(Built {
            component: Box::new(Driven::new(
                self.kind(),
                values.at("wavelength"),
                provenance,
            )),
            values: vec![phase, loss],
            derivatives,
            figures,
        })
    }
}
