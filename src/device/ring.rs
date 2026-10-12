//! The strip ring resonator, all-pass and add-drop: a ring of the strip coupled to one bus or
//! two, by Bogaerts et al.'s closed forms on the guide's dispersion and the couplers' coupling.

use std::f64::consts::{PI, TAU};

use crate::circuit::components::{AddDropRing, AllPassRing, Dispersion};
use crate::circuit::{Component, Fidelity, Parameter};
use crate::geometry::{Point, Shape};
use crate::mode::vector::{self, Boundary};
use crate::stack::{LayerStack, Structure};
use crate::units::Wavelength;
use crate::{Error, Result};

use super::fit;
use super::guide::{Strip, um};
use super::section::{self, Mesh};
use super::waveguide::{analytic_papers, band_parameters, solver_papers, strip_parameters};
use super::{
    Built, Device, Figure, Geometry, Method, Port, Range, Stated, Values, figure, method, parameter,
};

/// The gap's step for the 3D coupling's decay, µm: γ = ln(C(g − δ)/C(g + δ))/2δ, δ at most
/// half the gap.
const DECAY_STEP: f64 = 0.025;

/// A ring resonator of a silicon strip: a ring (a racetrack, with straight coupling sections)
/// beside one bus (all-pass: ports `in` and `through`) or between two (add-drop: also `add`
/// and `drop`); its component an [`AllPassRing`] or [`AddDropRing`] by Bogaerts et al. 2011's
/// closed forms.
///
/// Parameters `width`, `thickness`, `radius` (to the guide's centre line), `gap` (to the input
/// bus, edge to edge; and `gap_drop` to the drop bus), `coupling_length` (each straight
/// section, zero for a circle), `loss` (dB/cm, the guide's), `round_trip_loss` (dB more per
/// round trip: the bends' and couplers', Bogaerts's Eq. 19), `wavelength` and `span`.
///
/// The round trip is L = 2πR + 2L_c. Each coupler is a point coupler of power cross-coupling
/// κ² = sin²(C(g)(L_c + √(2πR/γ))): coupled-mode theory's sin²(CL) (Chrostowski & Hochberg,
/// Eq. 4.1) over the straight section and the curved one, whose coupling falls with the gap
/// as e^(−γΔg) and the gap grows as z²/2R along the bus, so its length is the Gaussian
/// integral √(2πR/γ) (Little et al. 1997's Appendix treats the curve as parabolic in the same
/// way). C and γ are the straight pair's at the gap.
///
/// - **Analytic:** the strip's analytic mode, and C and γ from the fit of the solver's coupling.
/// - **3D:** the strip's mode from the full-vector solver across the span, and C and γ from its
///   supermodes at the gap and 50 nm wider, at the band's centre.
///
/// The ring's mode is the straight strip's (the bend's shift of it and its radiation are left
/// to `round_trip_loss`), and the coupling is the band centre's at every wavelength.
#[derive(Clone, Debug, PartialEq)]
pub struct StripRing {
    strip: Strip,
    drop: bool,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

/// A ring's component, of either kind.
#[derive(Clone, Debug)]
enum Ring {
    AllPass(AllPassRing),
    AddDrop(AddDropRing),
}

/// What a fidelity gives a ring: its guide, and C and γ at each gap.
struct Model {
    guide: Dispersion,
    couplings: Vec<(f64, f64)>,
    guide_grid: String,
    coupling_grid: String,
}

impl StripRing {
    /// An all-pass ring of a 500 × 220 nm silicon strip, 10 µm in radius and 200 nm from its
    /// bus, on 220 nm SOI.
    pub fn all_pass() -> StripRing {
        StripRing::of(Strip::soi(), false)
    }

    /// An add-drop ring of the same strip, 200 nm from each bus.
    pub fn add_drop() -> StripRing {
        StripRing::of(Strip::soi(), true)
    }

    /// The ring drawn on the layer `core` of `stack`, with a drop bus if `drop`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str, drop: bool) -> Result<StripRing> {
        Ok(StripRing::of(Strip::on(stack, core)?, drop))
    }

    fn of(strip: Strip, drop: bool) -> StripRing {
        let mut parameters = strip_parameters();
        parameters.push(parameter("radius", "µm", 10.0, 1.0, 1e4));
        parameters.push(parameter("gap", "µm", 0.2, 0.02, 2.0));
        if drop {
            parameters.push(parameter("gap_drop", "µm", 0.2, 0.02, 2.0));
        }
        parameters.push(parameter("coupling_length", "µm", 0.0, 0.0, 1e3));
        parameters.push(parameter("loss", "dB/cm", 2.7, 0.0, 1e3));
        parameters.push(parameter("round_trip_loss", "dB", 0.0, 0.0, 50.0));
        parameters.extend(band_parameters());
        StripRing {
            methods: methods(&strip),
            strip,
            drop,
            parameters,
        }
    }

    /// The same device, solved on `mesh`.
    #[must_use]
    pub fn with_mesh(mut self, mesh: Mesh) -> StripRing {
        self.strip.mesh = mesh;
        self
    }

    /// The same device, its analytic fidelity on `guide`, the ring guide's stated dispersion;
    /// its loss is the `loss` parameter's.
    #[must_use]
    pub fn with_guide(mut self, guide: Dispersion) -> StripRing {
        self.strip.stated = Some(guide);
        self.methods = methods(&self.strip);
        self
    }

    /// The all-pass ring at `fidelity` and `values`, as its component.
    ///
    /// # Errors
    ///
    /// As [`Device::component`], and [`Error::InvalidValue`] for an add-drop ring.
    pub fn all_pass_ring(&self, values: &Values, fidelity: Fidelity) -> Result<AllPassRing> {
        match self.build(values, fidelity)?.0 {
            Ring::AllPass(r) => Ok(r),
            Ring::AddDrop(_) => Err(Error::invalid("device", "this ring has a drop bus")),
        }
    }

    /// The add-drop ring at `fidelity` and `values`, as its component.
    ///
    /// # Errors
    ///
    /// As [`Device::component`], and [`Error::InvalidValue`] for an all-pass ring.
    pub fn add_drop_ring(&self, values: &Values, fidelity: Fidelity) -> Result<AddDropRing> {
        match self.build(values, fidelity)?.0 {
            Ring::AddDrop(r) => Ok(r),
            Ring::AllPass(_) => Err(Error::invalid("device", "this ring has no drop bus")),
        }
    }

    /// The gaps: to the input bus, and to the drop bus.
    fn gaps(&self, values: &Values) -> Vec<f64> {
        let mut g = vec![values.at("gap")];
        if self.drop {
            g.push(values.at("gap_drop"));
        }
        g
    }

    /// The guide, and C and γ at each gap, at `fidelity`.
    fn model(&self, values: &Values, fidelity: Fidelity) -> Result<Model> {
        let (w, t) = (values.at("width"), values.at("thickness"));
        let (l0, span, loss) = (
            values.at("wavelength"),
            values.at("span"),
            values.at("loss"),
        );
        match fidelity {
            Fidelity::Analytic => {
                let (guide, guide_grid) = self.strip.analytic(w, t, l0, loss)?;
                let couplings = self
                    .gaps(values)
                    .into_iter()
                    .map(|g| fit::coupling(g, w, t))
                    .collect::<Result<Vec<_>>>()?;
                Ok(Model {
                    guide,
                    couplings,
                    guide_grid,
                    coupling_grid: fit::COUPLING_GRID.into(),
                })
            }
            _ => {
                let (mut guide, _, guide_grid) = self.strip.solved(w, t, l0, span)?;
                if loss != 0.0 {
                    guide.loss_db_per_cm += loss;
                }
                let at = Wavelength::um(l0)?;
                let c = |g: f64| -> Result<f64> {
                    let n = |wall| -> Result<f64> {
                        let cs = self.strip.pair_section(w, t, g, wall, at)?;
                        Ok(vector::modes(&cs, at, 1, None)?[0].effective_index().re)
                    };
                    Ok(PI * (n(Boundary::ElectricWall)? - n(Boundary::MagneticWall)?) / l0)
                };
                // C at the gap, and γ by a central difference about it
                let mut couplings = Vec::new();
                for g in self.gaps(values) {
                    let d = DECAY_STEP.min(g / 2.0);
                    let (narrower, here, wider) = (c(g - d)?, c(g)?, c(g + d)?);
                    couplings.push((here, libm::log(narrower / wider) / (2.0 * d)));
                }
                let coupling_grid = Strip::grid(&self.strip.pair_section(
                    w,
                    t,
                    values.at("gap"),
                    Boundary::ElectricWall,
                    at,
                )?);
                Ok(Model {
                    guide,
                    couplings,
                    guide_grid,
                    coupling_grid,
                })
            }
        }
    }

    /// The ring, its model, and the component's values and their derivatives.
    #[allow(clippy::type_complexity)]
    fn build(
        &self,
        values: &Values,
        fidelity: Fidelity,
    ) -> Result<(Ring, Model, Vec<f64>, Vec<Option<Vec<f64>>>)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let model = self.model(values, m.fidelity)?;
        let (r, lc) = (values.at("radius"), values.at("coupling_length"));
        let extra = values.at("round_trip_loss");
        let ring = if self.drop {
            Ring::AddDrop(AddDropRing::new(model.guide)?.with_round_trip_loss(extra)?)
        } else {
            Ring::AllPass(AllPassRing::new(model.guide)?.with_round_trip_loss(extra)?)
        };
        // L = 2πR + 2L_c, and each κ² = sin²φ, φ = C(L_c + √(2πR/γ))
        let mut settings = vec![TAU * r + 2.0 * lc];
        let mut by_radius = vec![TAU];
        let mut by_length = vec![2.0];
        for &(c, gamma) in &model.couplings {
            let curved = (TAU * r / gamma).sqrt();
            let phi = c * (lc + curved);
            // libm's sine, the same bits on every system (crate::portable)
            settings.push(libm::sin(phi).powi(2));
            let dk = libm::sin(2.0 * phi);
            by_radius.push(dk * c * PI / (gamma * curved));
            by_length.push(dk * c);
        }
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "radius" => Some(by_radius.clone()),
                "coupling_length" => Some(by_length.clone()),
                _ => None,
            })
            .collect();
        Ok((ring, model, settings, derivatives))
    }
}

/// The ring's models.
fn methods(strip: &Strip) -> Vec<Method> {
    let mut validity = if strip.stated.is_some() {
        Vec::new()
    } else {
        super::waveguide::fit_validity()
    };
    validity.retain(|r| r.name != "thickness");
    let t = fit::PAIR_THICKNESSES;
    validity.push(Range::new("thickness", t[0], t[t.len() - 1]));
    validity.push(Range::new(
        "gap",
        fit::GAPS[0],
        fit::GAPS[fit::GAPS.len() - 1],
    ));
    let mut papers = vec![
        "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the \
         January 2012 issue), doi:10.1002/lpor.201100017"
            .into(),
        "B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, \
         998 (1997), doi:10.1109/50.588673"
            .into(),
    ];
    let mut solved = papers.clone();
    papers.extend(analytic_papers());
    solved.extend(solver_papers());
    vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: "Bogaerts's closed forms on the strip's analytic mode and the fit's coupling"
                .into(),
            write_up: "devices".into(),
            papers,
            error: Some(Stated {
                quantity: "FSR".into(),
                bound: 0.5,
                unit: "%".into(),
                against: Fidelity::ThreeD,
            }),
            validity,
        },
        Method {
            fidelity: Fidelity::ThreeD,
            name: "Bogaerts's closed forms on the full-vector solver's mode and supermodes".into(),
            write_up: "devices".into(),
            papers: solved,
            error: None,
            validity: Vec::new(),
        },
    ]
}

impl Device for StripRing {
    fn kind(&self) -> &str {
        if self.drop {
            "add-drop ring"
        } else {
            "all-pass ring"
        }
    }

    fn about(&self) -> &str {
        if self.drop {
            "A ring of the strip between two buses: on resonance the light leaves by the drop \
             port."
        } else {
            "A ring of the strip beside one bus: the bus's transmission dips at each resonance."
        }
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        self.strip.stack(values.at("thickness"))
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let mut s = Structure::new(self.stack(values)?);
        let core = self.strip.core.clone();
        let (w, r, lc) = (
            values.at("width"),
            values.at("radius"),
            values.at("coupling_length"),
        );
        // the input bus along y = 0, the ring's lower straight (or point) w + gap above it
        let low = w + values.at("gap");
        let top = low + 2.0 * r;
        let centre = low + r;
        let reach = lc / 2.0 + r + w + 1.0;
        s.draw(&core, section::straight(w, 0.0, -reach, reach)?)?;
        if lc > 0.0 {
            s.draw(
                &core,
                Shape::sector(
                    Point::um(-lc / 2.0, centre),
                    um(r - w / 2.0),
                    um(r + w / 2.0),
                    PI / 2.0,
                    PI,
                )?,
            )?;
            s.draw(
                &core,
                Shape::sector(
                    Point::um(lc / 2.0, centre),
                    um(r - w / 2.0),
                    um(r + w / 2.0),
                    -PI / 2.0,
                    PI,
                )?,
            )?;
            for y in [low, top] {
                s.draw(&core, section::straight(w, y, -lc / 2.0, lc / 2.0)?)?;
            }
        } else {
            s.draw(&core, Shape::ring(Point::um(0.0, centre), um(r), um(w))?)?;
        }
        let port = |name: &str, x: f64, y: f64, direction: f64| Port {
            name: name.into(),
            position: Point::um(x, y),
            direction,
            width: um(w),
            layer: core.clone(),
        };
        let mut ports = vec![
            port("in", -reach, 0.0, 180.0),
            port("through", reach, 0.0, 0.0),
        ];
        if self.drop {
            let drop = top + w + values.at("gap_drop");
            s.draw(&core, section::straight(w, drop, -reach, reach)?)?;
            ports.push(port("add", reach, drop, 0.0));
            ports.push(port("drop", -reach, drop, 180.0));
        }
        Ok(Geometry {
            structure: s,
            ports,
        })
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (ring, model, settings, derivatives) = self.build(values, fidelity)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let l0 = model.guide.wavelength;
        let length = settings[0];
        let (gg, cg) = (model.guide_grid.as_str(), model.coupling_grid.as_str());
        let mut figures: Vec<Figure> = super::waveguide::guide_figures(&model.guide, f, gg);
        figures.push(figure("round trip", "L", length, "µm", f, super::EXACT));
        figures.push(figure(
            "coupling coefficient",
            "C",
            model.couplings[0].0,
            "1/µm",
            f,
            cg,
        ));
        figures.push(figure(
            "power coupling to the bus",
            "kappa2",
            settings[1],
            "",
            f,
            cg,
        ));
        if self.drop {
            figures.push(figure(
                "power coupling to the drop bus",
                "kappa2_drop",
                settings[2],
                "",
                f,
                cg,
            ));
        }
        let component: Box<dyn Component> = match ring {
            Ring::AllPass(ring) => {
                figures.push(figure(
                    "free spectral range",
                    "FSR",
                    ring.fsr(l0, length)? * 1e3,
                    "nm",
                    f,
                    gg,
                ));
                figures.push(figure(
                    "resonance",
                    "resonance",
                    ring.resonance(l0, length)?.to_um(),
                    "µm",
                    f,
                    gg,
                ));
                if let Ok(q) = ring.q_factor(l0, &settings) {
                    figures.push(figure("loaded Q", "Q", q, "", f, gg));
                }
                if let Ok((top, bottom)) = ring.extremes(l0, &settings) {
                    figures.push(figure(
                        "extinction",
                        "extinction",
                        10.0 * (top / bottom).log10(),
                        "dB",
                        f,
                        gg,
                    ));
                }
                Box::new(ring)
            }
            Ring::AddDrop(ring) => {
                figures.push(figure(
                    "free spectral range",
                    "FSR",
                    ring.fsr(l0, length)? * 1e3,
                    "nm",
                    f,
                    gg,
                ));
                figures.push(figure(
                    "resonance",
                    "resonance",
                    ring.resonance(l0, length)?.to_um(),
                    "µm",
                    f,
                    gg,
                ));
                if let Ok(q) = ring.q_factor(l0, &settings) {
                    figures.push(figure("loaded Q", "Q", q, "", f, gg));
                }
                if let Ok([_, _, peak, _]) = ring.extremes(l0, &settings) {
                    figures.push(figure("drop on resonance", "drop", peak, "", f, gg));
                }
                Box::new(ring)
            }
        };
        Ok(Built {
            component,
            values: settings,
            derivatives,
            figures,
        })
    }
}
