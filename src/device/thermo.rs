//! The thermo-optic phase shifter as a device: a heater beside a guide, its phase following the
//! heater's power, at analytic, 2D and measured fidelity.

use std::f64::consts::PI;

use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::generator::{Generator, Straight};
use crate::geometry::{Point, Shape};
use crate::stack::{Etch, Layer, LayerStack, Structure};
use crate::thermal::shifter_checks::{jacques_grading, table3};
use crate::thermal::{
    Boundary, Drive, Features, Grid, Guide, Heat, Optics, PhaseShifter, Scheme, Section, Side,
    Stepping, Tensor,
};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::active::{Driven, Measurement};
use super::guide::um;
use super::section::{core_span, with_thickness};
use super::thermo_closed::{Sheets, Slab, thermo_optic};
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
}

/// The 2D model's region, as Jacques et al.'s: 18 µm high, its bottom (in the substrate) held at
/// the ambient, 10 W/(m²·K) to the air above the oxide, at least 40 µm wide with insulated sides
/// (wider to hold a neighbour 10 µm inside it).
const REGION_HEIGHT: f64 = 18.0;
/// The region's half width, µm, before a neighbour widens it.
const REGION_HALF: f64 = 20.0;
/// The air's film coefficient over the oxide, W/(m²·K).
const AIR: f64 = 10.0;

/// The measured devices the library holds, each a file of [`Measurement`]'s.
const MEASURED: [&str; 2] = [
    include_str!("measured/jacques-2019-tin.toml"),
    include_str!("measured/jacques-2019-npp.toml"),
];

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
///
/// Both take `length`, `pitch` (a neighbour's centre from the guide's, for the crosstalk),
/// `power`, `loss` (dB/cm, the propagation loss the heat model doesn't give), `wavelength` (1.5
/// to 1.6 µm: the materials' constants are given at 1550 nm) and `span`. The buried oxide is the
/// stack's (the layers under the core, as oxide); the heat and thermo-optic constants are
/// Jacques et al.'s Table 3 at 300 K (silicon's dn/dT 1.8e-4/K, the oxide's neglected), so the
/// core must be silicon.
///
/// - **Analytic:** closed forms (docs/methods/devices.md): the cross-section thermally all
///   oxide between the isothermal substrate and an insulated top, the heaters sheets at their
///   height, its eigenfunctions with each strip's Green's function across in closed form: the
///   rise at the guide's centre steady, at a frequency and in time; times Marcatili's
///   dn_eff/dT.
/// - **2D:** [`crate::thermal::PhaseShifter`] on the cross-section in Jacques et al.'s region:
///   the static solver's rise, each optical cell's weight by first-order perturbation of the
///   full-vector mode, the 1/e time of the phase turned on (BDF2), its −3 dB frequency, and the
///   neighbour's crosstalk (the guide's mode moved by `pitch`, a probe whose own silicon isn't
///   drawn). A 2D number: a straight section's, any length.
/// - **Measured:** a measured device's file ([`Measurement`]): Jacques et al.'s TiN and N++
///   shifters, or one given ([`ThermoShifter::with_measurement`]), at its design's values only.
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

    /// The metal (`doped` false) or doped heater's shifter on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer, or its core isn't silicon (an
    /// index at 1.55 µm below 3: the heat model's constants are silicon's).
    pub fn on(stack: LayerStack, core: &str, doped: bool) -> Result<ThermoShifter> {
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
        let heater = if doped { Heater::Doped } else { Heater::Metal };
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
        }
        parameters.extend([
            parameter("length", "µm", 320.0, 1.0, 1e5),
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
    /// 0.5 µm growing by 1.2, and 10 nm optical cells.
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
                }
            }
        })
    }

    /// The figures and the phase's slope in the power (rad/mW) at `fidelity`, and the provenance
    /// of its phase.
    fn model(&self, values: &Values, fidelity: Fidelity) -> Result<(Vec<Figure>, Provenance)> {
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
        figures.push(figure(
            "insertion loss",
            "IL",
            values.at("loss") * values.at("length") * 1e-4,
            "dB",
            f,
            EXACT,
        ));
        let provenance = Provenance {
            fidelity: f,
            source: format!(
                "the phase π P/P_π, P_π = {:.4} mW at {lambda} um by {} ({})",
                p_pi.0, m.name, p_pi.1
            ),
            error: None,
            validity: Some((1.5, 1.6)),
        };
        Ok((figures, provenance))
    }
}

/// The kind of a heater's shifter.
fn kind_of(heater: Heater) -> &'static str {
    match heater {
        Heater::Metal => "thermo-optic phase shifter",
        Heater::Doped => "doped thermo-optic phase shifter",
    }
}

/// Jacques et al. 2019, with what the device takes from it.
fn jacques() -> String {
    "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 \
     (2019), doi:10.1364/OE.27.010456: the region, the materials (Table 3) and Eqs. 4-5"
        .into()
}

/// The shifter's models.
fn methods(d: &ThermoShifter) -> Vec<Method> {
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
            "closed forms: the slab as a fin of silicon leaking through the buried oxide (exact \
             in each of the rib, the buffer and the doped strips), times Marcatili's dn_eff/dT; \
             the oxide slab's 1/e time and -3 dB frequency, heated in the slab",
            STATED_DOPED_2D,
            STATED_DOPED_MEASURED,
        ),
    };
    let mut methods = vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: closed.into(),
            write_up: "devices".into(),
            papers: vec![
                jacques(),
                "E. A. J. Marcatili, Bell Syst. Tech. J. 48, 2071 (1969), \
                 doi:10.1002/j.1538-7305.1969.tb01166.x"
                    .into(),
            ],
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
            name: "the static solver's rise in Jacques et al.'s region, each optical cell's \
                   weight by first-order perturbation of the full-vector mode \
                   (thermal::PhaseShifter); its step response by BDF2 and its -3 dB frequency; \
                   a straight section's, any length"
                .into(),
            write_up: "phase-shifter".into(),
            papers: vec![
                jacques(),
                "S. G. Johnson et al., Phys. Rev. E 65, 066611 (2002), \
                 doi:10.1103/PhysRevE.65.066611"
                    .into(),
                "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 \
                 (2008), doi:10.1109/JLT.2008.923643"
                    .into(),
            ],
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
fn short(source: &str) -> &str {
    source.split(':').next().unwrap_or(source)
}

/// The analytic models' largest error in P_π against the 2D model over their validity, %, as
/// the validation report measures it (`device/thermo-analytic-*`).
const STATED_METAL_2D: f64 = 15.0;
const STATED_DOPED_2D: f64 = 15.0;
/// The 2D model's against the measured devices, %, (`device/thermo-*-measured`).
const STATED_METAL_MEASURED: f64 = 15.0;
const STATED_DOPED_MEASURED: f64 = 15.0;

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
    /// The region's bottom, in the substrate.
    fn bottom(&self) -> f64 {
        self.top - REGION_HEIGHT
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
            &jacques_grading(r),
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

    /// The grid the 2D figures name.
    fn grid_2d(&self, shifter: &PhaseShifter, r: f64) -> String {
        let g = shifter.heat().grid();
        format!(
            "heat on {} × {} cells graded from {:.3} nm to {:.3} um (growth {:.3}) over {} × {} \
             um; mode on {:.3} nm cells",
            g.x().cells(),
            g.y().cells(),
            20.0 / r,
            0.5 / r,
            1.0 + 0.2 / r,
            2.0 * (self.pitch + 10.0).max(REGION_HALF),
            REGION_HEIGHT,
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
            Heater::Doped => 0.1,
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
    /// resistance) to the oxide's top, and the isothermal plane's height, µm.
    fn slab(&self) -> (Slab, f64) {
        use table3::*;
        let substrate = -self.buried - self.bottom();
        let plane = -self.buried - substrate * K_OX / K_SUBSTRATE;
        (
            Slab {
                height: (self.top - plane) * 1e-6,
                conductivity: K_OX,
                diffusivity: K_OX / C_OX,
            },
            plane,
        )
    }

    /// The heaters as sheets in the slab whose isothermal plane is at `plane` (µm): the metal
    /// strip at its mid-height, or each doped strip at the slab's, half the power in each.
    fn sheets(&self, plane: f64) -> Sheets {
        let m = |y: f64| (y - plane) * 1e-6;
        let a = self.heater_width * 1e-6;
        match self.heater {
            Heater::Metal => Sheets {
                height: m(self.heater_bottom + 0.5 * self.heater_thickness),
                strips: vec![(0.0, a, 1.0)],
            },
            Heater::Doped => {
                let c = (self.inner() + 0.5 * self.heater_width) * 1e-6;
                Sheets {
                    height: m(0.5 * self.slab),
                    strips: vec![(-c, a, 0.5), (c, a, 0.5)],
                }
            }
        }
    }

    /// The analytic model's figures.
    fn analytic(&self, lambda: f64) -> Result<Vec<Figure>> {
        use table3::*;
        let (slab, plane) = self.slab();
        let sheets = self.sheets(plane);
        let guide = (0.5 * self.thickness - plane) * 1e-6;
        let rise = slab.steady(&sheets, 0.0, guide);
        let crosstalk = slab.steady(&sheets, self.pitch * 1e-6, guide) / rise;
        let (dn_dt, n_eff) = thermo_optic(
            self.width,
            self.thickness,
            (N_SI, DN_DT),
            (N_OX, 0.0),
            Wavelength::um(lambda)?,
        )?;
        let s = dn_dt * rise;
        let p_pi = lambda * 1e-6 / (2.0 * s) * 1e3;
        let f = Fidelity::Analytic;
        Ok(vec![
            figure("power for a π shift", "P_pi", p_pi, "mW", f, EXACT),
            figure(
                "1/e time, turned on",
                "tau",
                slab.one_over_e(&sheets, 0.0, guide) * 1e6,
                "µs",
                f,
                EXACT,
            ),
            figure(
                "−3 dB frequency",
                "f_3dB",
                slab.cutoff(&sheets, 0.0, guide) * 1e-3,
                "kHz",
                f,
                EXACT,
            ),
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
        }
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        let stack = with_thickness(&self.stack, &self.core, values.at("thickness"))?;
        if self.heater == Heater::Doped {
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
            Heater::Doped => {
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
        let (figures, provenance) = self.model(values, fidelity)?;
        let p_pi = figures
            .iter()
            .find(|g| g.symbol == "P_pi")
            .map_or(f64::NAN, |g| g.value);
        let (phase, loss) = (
            PI * values.at("power") / p_pi,
            values.at("loss") * values.at("length") * 1e-4,
        );
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "power" => Some(vec![PI / p_pi, 0.0]),
                "loss" => Some(vec![0.0, values.at("length") * 1e-4]),
                "length" => Some(vec![0.0, values.at("loss") * 1e-4]),
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
