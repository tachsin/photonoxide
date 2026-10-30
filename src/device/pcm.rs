//! The phase-change phase shifter as a device: a silicon guide with a cell of a phase-change
//! material (GST, Sb₂S₃ or Sb₂Se₃) on top, its effective index, phase and loss in either state
//! or part-way between, at analytic and 2D fidelity.

use std::f64::consts::{LOG10_E, TAU};

use num_complex::Complex64 as c64;

use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::generator::{Generator, Straight};
use crate::geometry::{Point, Shape};
use crate::material::catalogue::{State, partly_crystalline, phase_change};
use crate::material::{Material, Model, Provenance as Source};
use crate::mode::Polarization;
use crate::mode::multilayer::Multilayer;
use crate::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity, graded_nodes};
use crate::stack::{Layer, LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::active::Driven;
use super::guide::{Profile, um};
use super::section::{self, Cut, core_span, indices, with_thickness};
use super::spec::MaterialSpec;
use super::{
    Built, Device, EXACT, Figure, Geometry, Method, Range, Stated, Values, figure, method,
    parameter,
};

/// The cladding the 2D model's window keeps around the guide and the film, µm.
const MARGIN: f64 = 1.0;
/// The 2D model's cells in and near the guide, µm; they grow with the distance from it by
/// [`GROWTH`], up to [`LARGEST`].
const STEP: f64 = 0.005;
/// The cells' growth with the distance from the guide, µm per µm.
const GROWTH: f64 = 0.1;
/// The cells' largest size, µm.
const LARGEST: f64 = 0.1;
/// Where the cells are fine about the guide and the film, µm beyond their faces.
const NEAR: f64 = 0.1;
/// The steps the effective index method switches a film's absorption on in.
const ABSORPTION_STEPS: u32 = 50;
/// Power in dB per unit of e^(−2 k₀ Im n_eff z): 10 log₁₀ e × 2.
const DB: f64 = 2.0 * 10.0 * LOG10_E;

/// Which phase-change material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pcm {
    Gst,
    Sb2S3,
    Sb2Se3,
}

impl Pcm {
    /// The catalogue's id.
    fn id(self) -> &'static str {
        match self {
            Pcm::Gst => "gst",
            Pcm::Sb2S3 => "sb2s3",
            Pcm::Sb2Se3 => "sb2se3",
        }
    }

    fn named(id: &str) -> Result<Pcm> {
        [Pcm::Gst, Pcm::Sb2S3, Pcm::Sb2Se3]
            .into_iter()
            .find(|p| p.id() == id)
            .ok_or_else(|| {
                Error::invalid(
                    "device",
                    format!("no phase-change material \"{id}\": gst, sb2s3 or sb2se3"),
                )
            })
    }

    /// The device's kind.
    fn kind(self) -> &'static str {
        match self {
            Pcm::Gst => "GST phase-change phase shifter",
            Pcm::Sb2S3 => "Sb2S3 phase-change phase shifter",
            Pcm::Sb2Se3 => "Sb2Se3 phase-change phase shifter",
        }
    }

    /// The formula, for the text.
    fn formula(self) -> &'static str {
        match self {
            Pcm::Gst => "Ge₂Sb₂Te₅",
            Pcm::Sb2S3 => "Sb₂S₃",
            Pcm::Sb2Se3 => "Sb₂Se₃",
        }
    }
}

/// A silicon guide `width` × `thickness` µm (a rib, if `slab` µm of the silicon is left
/// beside it) with a cell of a phase-change material `film_thickness` µm thick on top, as wide
/// as `coverage` of the guide's width, `length` µm long, ports `o1` and `o2`. The cell is
/// crystallized to the fraction `crystallized` of its volume (0 amorphous, 1 crystalline); its
/// component is the phase it adds against the amorphous state, φ = 2π (n_eff − n_eff,a) L/λ,
/// and the insertion loss of the cell over its length, a phase shifter of parameters `phase` and
/// `loss`, S₂₁ = 10^(−IL/20) e^(iφλ₀/λ). The state is the one the material holds without power:
/// the phase shifter's static power is zero.
///
/// - [`PcmShifter::gst`], [`PcmShifter::sb2s3`] and [`PcmShifter::sb2se3`] (kinds `"GST
///   phase-change phase shifter"`, `"Sb2S3 …"` and `"Sb2Se3 …"`): the catalogue's two states
///   (our fits to Delaney et al.'s ellipsometry), a part-way state as Frantz et al.'s linear
///   effective medium of their permittivities, ε = (1 − f) ε_a + f ε_c. By default Delaney et
///   al.'s 25 nm film on a 500 × 220 nm silicon strip, the film on the guide's top only.
///
/// All take `loss` (dB/cm, the propagation loss of the guide that the models don't give, added
/// to the cell's) and `wavelength` (1.25 to 1.65 µm: the fits' range). The film can follow the
/// guide: `wall_film` µm on its walls and `slab_film` µm over the slab beside it (or the oxide
/// beside a strip), out to the window's edge, each thickness measured normal to the face it
/// lies on, the corners square; both 0 (the default) is the film on the top alone, and equal to
/// `film_thickness` a conformal film. The mode is the fundamental TE-like one, or with `tm` 1
/// the TM-like one.
///
/// - **Analytic:** the effective index method with the slabs' modes exact and complex
///   (docs/methods/devices.md): the vertical column through the guide (cladding, the film at
///   its fraction, silicon, buried oxide), beside it (a rib's slab, or the cladding alone), and
///   the lateral stack across (the film's width and the bare silicon either side of it), each
///   the root of a transfer matrix's dispersion relation. The loss is the imaginary part of the
///   effective index. For the TM-like mode the columns are solved in TM and the lateral stack
///   in TE; a film on the walls adds a column through it beside the guide's. Its stated error
///   holds for the film on the top and the TE-like mode.
/// - **2D:** the full-vector mode solver on the cross-section ([`crate::mode::vector`], the
///   standard scheme with complex permittivities), half of it behind an electric wall (a
///   magnetic one for the TM-like mode), cells from 5 nm in and near the guide and the film,
///   1 µm of cladding around. A 2D number: a straight section's, any length. A rib's TM-like
///   mode leaks into its slab, as does a mode under a film that raises the slab's index above
///   the guide's: the model refuses them.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmShifter {
    pcm: Pcm,
    stack: LayerStack,
    core: String,
    refinement: f64,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

/// The effective indices of the cell in its three states: amorphous, at its fraction, and
/// crystalline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct States {
    /// The amorphous state's n_eff.
    pub(super) amorphous: c64,
    /// The cell's n_eff.
    pub(super) now: c64,
    /// The crystalline state's n_eff.
    pub(super) crystalline: c64,
}

impl PcmShifter {
    /// A GST cell on a 500 × 220 nm silicon strip on 220 nm SOI.
    pub fn gst() -> PcmShifter {
        PcmShifter::of(Pcm::Gst, LayerStack::soi_220(), "Si")
    }

    /// An Sb₂S₃ cell on a 500 × 220 nm silicon strip on 220 nm SOI.
    pub fn sb2s3() -> PcmShifter {
        PcmShifter::of(Pcm::Sb2S3, LayerStack::soi_220(), "Si")
    }

    /// An Sb₂Se₃ cell on a 500 × 220 nm silicon strip on 220 nm SOI.
    pub fn sb2se3() -> PcmShifter {
        PcmShifter::of(Pcm::Sb2Se3, LayerStack::soi_220(), "Si")
    }

    /// The cell of the catalogue's phase-change material `material` (`"gst"`, `"sb2s3"` or
    /// `"sb2se3"`) on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another material, or a stack with no such layer.
    pub fn on(material: &str, stack: LayerStack, core: &str) -> Result<PcmShifter> {
        let pcm = Pcm::named(material)?;
        section::core_span(&stack, core)?;
        Ok(PcmShifter::of(pcm, stack, core))
    }

    fn of(pcm: Pcm, stack: LayerStack, core: &str) -> PcmShifter {
        let parameters = vec![
            parameter("width", "µm", 0.5, 0.2, 3.0),
            parameter("thickness", "µm", 0.22, 0.1, 1.0),
            parameter("slab", "µm", 0.0, 0.0, 0.5),
            parameter("film_thickness", "µm", 0.025, 0.005, 0.2),
            parameter("coverage", "", 1.0, 0.1, 1.0),
            parameter("wall_film", "µm", 0.0, 0.0, 0.2),
            parameter("slab_film", "µm", 0.0, 0.0, 0.2),
            parameter("tm", "", 0.0, 0.0, 1.0),
            parameter("crystallized", "", 0.0, 0.0, 1.0),
            parameter("length", "µm", 10.0, 0.1, 1e5),
            parameter("loss", "dB/cm", 0.0, 0.0, 1e3),
            parameter("wavelength", "µm", 1.55, 1.25, 1.65),
        ];
        let mut d = PcmShifter {
            pcm,
            stack,
            core: core.to_owned(),
            refinement: 1.0,
            parameters,
            methods: Vec::new(),
        };
        d.methods = methods();
        d
    }

    /// The same device, its 2D model's cells divided by `factor` (≥ 1), as the convergence
    /// studies take them. 1, the device's own, is 5 nm in and near the guide and the film,
    /// growing by 0.1 per µm from them to 0.1 µm.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a factor below 1 or not finite.
    pub fn with_refinement(mut self, factor: f64) -> Result<PcmShifter> {
        if !(factor.is_finite() && factor >= 1.0) {
            return Err(Error::invalid(
                "device",
                format!("a refinement is a factor of at least 1, got {factor}"),
            ));
        }
        self.refinement = factor;
        Ok(self)
    }

    /// The catalogue's material of one state.
    pub(super) fn state(&self, state: State) -> Result<Material> {
        let pc = phase_change(self.pcm.id()).ok_or_else(|| {
            Error::invalid("device", format!("the catalogue has no {}", self.pcm.id()))
        })?;
        MaterialSpec::id(pc.entry(state)).material()
    }

    /// The film's material at the crystallized `fraction` and wavelength `lambda` (µm): a
    /// state's own, or the linear mix of their permittivities as a constant index at `lambda`.
    fn film(&self, fraction: f64, lambda: f64) -> Result<Material> {
        let (a, c) = (
            self.state(State::Amorphous)?,
            self.state(State::Crystalline)?,
        );
        if fraction == 0.0 {
            return Ok(a);
        }
        if fraction == 1.0 {
            return Ok(c);
        }
        let wavelength = Wavelength::um(lambda)?;
        let n = partly_crystalline(&a, &c, fraction, wavelength)?;
        Material::new(
            format!(
                "{} ({:.1} % crystalline)",
                self.pcm.formula(),
                100.0 * fraction
            ),
            Model::Constant(n),
            wavelength,
            wavelength,
            Source {
                reference: "J. A. Frantz et al., Opt. Mater. Express 13, 3631 (2023)".into(),
                doi: "10.1364/OME.506019".into(),
                data: "the linear effective medium of the catalogue's two states' permittivities"
                    .into(),
                temperature: None,
                notes: format!("ε = (1 − f) ε_a + f ε_c at f = {fraction}, at {lambda} µm only"),
            },
        )
    }

    /// The stack with the core `thickness` thick and the film's layer, of `film`, on top of it.
    fn film_stack(&self, values: &Values, film: Material) -> Result<LayerStack> {
        let stack = with_thickness(&self.stack, &self.core, values.at("thickness"))?;
        let mut layers = stack.layers().to_vec();
        let k = layers
            .iter()
            .position(|l| l.name == self.core)
            .ok_or_else(|| Error::invalid("device", "the stack has no core"))?;
        let oxide = layers[k].background.clone();
        layers.insert(
            k + 1,
            Layer {
                name: "PCM".into(),
                thickness: Length::um(values.at("film_thickness")),
                material: film,
                background: oxide,
            },
        );
        LayerStack::new(stack.substrate().clone(), layers, stack.cladding().clone())
    }

    /// The film's thicknesses on the guide's walls and over the slab beside it, µm, when either
    /// isn't 0 (a film that follows the guide); `None` for the film on the top alone.
    fn conformal(values: &Values) -> Option<(f64, f64)> {
        let (wall, slab) = (values.at("wall_film"), values.at("slab_film"));
        (wall > 0.0 || slab > 0.0).then_some((wall, slab))
    }

    /// The mode the figures are of: TE-like (`tm` 0) or TM-like (`tm` 1).
    fn polarization(values: &Values) -> Polarization {
        if values.at("tm") == 1.0 {
            Polarization::Tm
        } else {
            Polarization::Te
        }
    }

    /// Refuses a design the models can't draw: `tm` other than 0 or 1, and a film on the walls
    /// or the slab with a patch narrower than the guide, or a film over the slab as thick as the
    /// rib's step or thicker.
    fn check_design(values: &Values) -> Result<()> {
        let tm = values.at("tm");
        if tm != 0.0 && tm != 1.0 {
            return Err(Error::invalid(
                "device",
                format!("tm is 0 (the TE-like mode) or 1 (the TM-like mode), got {tm}"),
            ));
        }
        if let Some((_, slab_film)) = PcmShifter::conformal(values) {
            if values.at("coverage") < 1.0 {
                return Err(Error::invalid(
                    "device",
                    "a film on the guide's walls or over its slab covers the whole guide: \
                     coverage 1",
                ));
            }
            let step = values.at("thickness") - values.at("slab");
            if slab_film >= step {
                return Err(Error::invalid(
                    "device",
                    format!(
                        "the film over the slab ({slab_film} µm) must be thinner than the rib's \
                         step ({step} µm)"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The profile of the guide's etch.
    fn profile(values: &Values) -> Profile {
        Profile {
            slab: values.at("slab"),
            sidewall: 90.0,
        }
    }

    /// The structure of a short piece of the device (x from −0.5 to 0.5 µm), its film of
    /// `film`, and the core's mid-plane.
    fn piece(&self, values: &Values, film: Material) -> Result<(Structure, f64)> {
        let stack = self.film_stack(values, film)?;
        let (bottom, top) = core_span(&stack, &self.core)?;
        let (w, t) = (values.at("width"), values.at("thickness"));
        let etch = PcmShifter::profile(values).etch(t)?;
        let mut s = Structure::new(stack);
        s.draw_etched(&self.core, section::straight(w, 0.0, -0.5, 0.5)?, etch)?;
        s.draw("PCM", section::straight(top_width(values), 0.0, -0.5, 0.5)?)?;
        Ok((s, 0.5 * (bottom + top)))
    }

    /// The full-vector solver's cross-section at the film's `fraction`: half of it, x ≥ 0, the
    /// guide's and the film's faces on nodes.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's, and for a material outside
    /// its range.
    pub fn cross_section(&self, values: &Values, fraction: f64) -> Result<CrossSection> {
        values.check(self)?;
        PcmShifter::check_design(values)?;
        let film = self.film(fraction, values.at("wavelength"))?;
        self.cross_section_of(values, film)
    }

    /// The cross-section with a film of `film` (any material): the guide's and the film's faces
    /// on nodes, behind an electric wall on x = 0 for the TE-like mode and a magnetic one for
    /// the TM-like.
    pub(super) fn cross_section_of(&self, values: &Values, film: Material) -> Result<CrossSection> {
        let wavelength = Wavelength::um(values.at("wavelength"))?;
        let eps = film.permittivity(wavelength)?;
        let (structure, mid) = self.piece(values, film)?;
        let (w, t, s) = (
            values.at("width"),
            values.at("thickness"),
            values.at("slab"),
        );
        let (cw, tf) = (values.at("coverage") * w, values.at("film_thickness"));
        let h = STEP / self.refinement;
        let (bottom, top) = (-0.5 * t, 0.5 * t);
        let cs = match PcmShifter::conformal(values) {
            None => {
                let x = graded_nodes(
                    0.0,
                    0.5 * w + MARGIN,
                    (0.0, 0.5 * w + NEAR),
                    h,
                    GROWTH,
                    LARGEST,
                    &[0.5 * cw, 0.5 * w],
                );
                let y = graded_nodes(
                    bottom - MARGIN,
                    top + tf + MARGIN,
                    (bottom - NEAR, top + tf + NEAR),
                    h,
                    GROWTH,
                    LARGEST,
                    &[bottom, top, bottom + s, top + tf],
                );
                Cut {
                    structure: &structure,
                    at: 0.0,
                    mid,
                }
                .raster(x, y, wavelength)?
            }
            Some((wall, over)) => {
                let (x, y) = self.conformal_nodes(values);
                let flat = Cut {
                    structure: &structure,
                    at: 0.0,
                    mid,
                }
                .raster(x, y, wavelength)?;
                conformal_film(&flat, 0.5 * w, bottom + s, top, (wall, over), eps)?
            }
        };
        match PcmShifter::polarization(values) {
            Polarization::Te => Ok(cs),
            Polarization::Tm => cs.with_boundaries(Boundaries {
                west: Boundary::MagneticWall,
                ..Boundaries::default()
            }),
        }
    }

    /// The nodes of the cross-section of a film that follows the guide: the guide's top and
    /// walls, the film's faces on the top, the walls and the slab, and the slab's top on nodes.
    fn conformal_nodes(&self, values: &Values) -> (Vec<f64>, Vec<f64>) {
        let (w, t, s) = (
            values.at("width"),
            values.at("thickness"),
            values.at("slab"),
        );
        let (wall, over) = (values.at("wall_film"), values.at("slab_film"));
        let tf = values.at("film_thickness");
        let h = STEP / self.refinement;
        let (bottom, top) = (-0.5 * t, 0.5 * t);
        let edge = 0.5 * w + wall;
        let x = graded_nodes(
            0.0,
            edge + MARGIN,
            (0.0, edge + NEAR),
            h,
            GROWTH,
            LARGEST,
            &[0.5 * w, edge],
        );
        let y = graded_nodes(
            bottom - MARGIN,
            top + tf + MARGIN,
            (bottom - NEAR, top + tf + NEAR),
            h,
            GROWTH,
            LARGEST,
            &[bottom, top, bottom + s, bottom + s + over, top + tf],
        );
        (x, y)
    }

    /// The film of a cell that follows the guide (`values` with a film on its walls and over its
    /// slab) in the limit of a guide of no step, its core as wide as the window, at the film's
    /// `fraction`: the conformal construction (the film on the top `wall_film` wider than the
    /// guide each side, then on the walls and over the slab, as thick as on the top) against the
    /// stack's film layer drawn across the window, on the same nodes. Their cross-sections, and
    /// each one's mode of the device's polarization nearest the vertical column's exact n_eff.
    ///
    /// # Errors
    ///
    /// As [`PcmShifter::cross_section`], and [`Error::InvalidValue`] for a film on the top alone.
    pub(super) fn zero_step(
        &self,
        values: &Values,
        fraction: f64,
    ) -> Result<[(CrossSection, c64); 2]> {
        values.check(self)?;
        PcmShifter::check_design(values)?;
        let Some((wall, _)) = PcmShifter::conformal(values) else {
            return Err(Error::invalid(
                "device",
                "the film is on the guide's top alone",
            ));
        };
        let lambda = values.at("wavelength");
        let wavelength = Wavelength::um(lambda)?;
        let film = self.film(fraction, lambda)?;
        let eps = film.permittivity(wavelength)?;
        let film_index = film.refractive_index(wavelength)?;
        let stack = self.film_stack(values, film)?;
        let (bottom, top) = core_span(&stack, &self.core)?;
        let mid = 0.5 * (bottom + top);
        let (w, t, tf) = (
            values.at("width"),
            values.at("thickness"),
            values.at("film_thickness"),
        );
        let (x, y) = self.conformal_nodes(values);
        // wider than the window: the core and the film run into its edge
        let across = 4.0 * x[x.len() - 1];
        let raster = |film_width: f64| -> Result<CrossSection> {
            let mut s = Structure::new(stack.clone());
            s.draw(&self.core, section::straight(across, 0.0, -0.5, 0.5)?)?;
            s.draw("PCM", section::straight(film_width, 0.0, -0.5, 0.5)?)?;
            Cut {
                structure: &s,
                at: 0.0,
                mid,
            }
            .raster(x.clone(), y.clone(), wavelength)
        };
        let conformal = conformal_film(
            &raster(w + 2.0 * wall)?,
            0.5 * w,
            0.5 * t,
            0.5 * t,
            (wall, tf),
            eps,
        )?;
        let layered = raster(across)?;
        // the column's exact n_eff: the film over the core, between the core's neighbours
        let (below, core, above, _) = indices(
            &with_thickness(&self.stack, &self.core, t)?,
            &self.core,
            wavelength,
        )?;
        let polarization = PcmShifter::polarization(values);
        let near = Multilayer::new(
            c64::new(above, 0.0),
            &[
                (c64::new(film_index.re, 0.0), Length::um(tf)),
                (c64::new(core, 0.0), Length::um(t)),
            ],
            c64::new(below, 0.0),
        )?
        .bound_modes(polarization, wavelength)?
        .first()
        .map_or(core, |m| m.effective_index().re);
        let walls = |cs: CrossSection| -> Result<CrossSection> {
            match polarization {
                Polarization::Te => Ok(cs),
                Polarization::Tm => cs.with_boundaries(Boundaries {
                    west: Boundary::MagneticWall,
                    ..Boundaries::default()
                }),
            }
        };
        let solve = |cs: CrossSection| -> Result<(CrossSection, c64)> {
            let cs = walls(cs)?;
            let n = vector::modes(&cs, wavelength, 1, Some(near))?
                .first()
                .map(vector::VectorMode::effective_index)
                .ok_or_else(|| Error::invalid("device", "no mode near the column's"))?;
            Ok((cs, n))
        };
        Ok([solve(conformal)?, solve(layered)?])
    }

    /// The cell's n_eff by the effective index method with exact complex slabs, the film at
    /// `fraction`.
    pub(super) fn eim(&self, values: &Values, fraction: f64) -> Result<c64> {
        let lambda = values.at("wavelength");
        let wavelength = Wavelength::um(lambda)?;
        let base = with_thickness(&self.stack, &self.core, values.at("thickness"))?;
        let (below, core, above, side) = indices(&base, &self.core, wavelength)?;
        let film = self.film(fraction, lambda)?.refractive_index(wavelength)?;
        let (w, t, tf) = (
            values.at("width"),
            values.at("thickness"),
            values.at("film_thickness"),
        );
        let (cw, slab) = (values.at("coverage") * w, values.at("slab"));
        // the fundamental mode of a stack: the lossless stack's, followed as the absorption is
        // switched on in steps (a strongly absorbing film would otherwise lead the iteration to
        // another root)
        let root = |polarization: Polarization,
                    cover: c64,
                    films: &[(c64, f64)],
                    substrate: c64,
                    what: &str|
         -> Result<c64> {
            let stack = |share: f64| -> Result<Multilayer> {
                let at = |n: c64| c64::new(n.re, share * n.im);
                let films: Vec<(c64, Length)> =
                    films.iter().map(|&(n, d)| (at(n), Length::um(d))).collect();
                Multilayer::new(at(cover), &films, at(substrate))
            };
            let mut guess = stack(0.0)?
                .bound_modes(polarization, wavelength)?
                .first()
                .map(|m| m.effective_index())
                .ok_or_else(|| Error::invalid("device", format!("{what} guides no mode")))?;
            for k in 1..=ABSORPTION_STEPS {
                guess = stack(f64::from(k) / f64::from(ABSORPTION_STEPS))?
                    .mode_near(polarization, wavelength, guess)?
                    .effective_index();
            }
            Ok(guess)
        };
        let (n_above, n_below, n_core) = (
            c64::new(above, 0.0),
            c64::new(below, 0.0),
            c64::new(core, 0.0),
        );
        // the vertical columns' polarization is the mode's, the lateral stack's the other
        let (vertical, lateral) = match PcmShifter::polarization(values) {
            Polarization::Te => (Polarization::Te, Polarization::Tm),
            Polarization::Tm => (Polarization::Tm, Polarization::Te),
        };
        let with_film = root(
            vertical,
            n_above,
            &[(film, tf), (n_core, t)],
            n_below,
            "the column through the guide",
        )?;
        if let Some((wall, over)) = PcmShifter::conformal(values) {
            // the film over the slab beside the guide, and down the walls from the top's film
            let mut beside_films = Vec::new();
            if over > 0.0 {
                beside_films.push((film, over));
            }
            if slab > 0.0 {
                beside_films.push((n_core, slab));
            }
            let beside = if beside_films.is_empty() {
                c64::new(side, 0.0)
            } else {
                root(
                    vertical,
                    n_above,
                    &beside_films,
                    n_below,
                    "the column beside the guide",
                )?
            };
            let mut across = vec![(with_film, w)];
            if wall > 0.0 {
                let mut films = vec![(film, t - slab + tf)];
                if slab > 0.0 {
                    films.push((n_core, slab));
                }
                let on_wall = root(
                    vertical,
                    n_above,
                    &films,
                    n_below,
                    "the column through the film on the wall",
                )?;
                across = vec![(on_wall, wall), (with_film, w), (on_wall, wall)];
            }
            return root(lateral, beside, &across, beside, "the lateral stack");
        }
        let bare = root(
            vertical,
            n_above,
            &[(n_core, t)],
            n_below,
            "the column through the guide without the film",
        )?;
        let beside = if slab > 0.0 {
            root(
                vertical,
                n_above,
                &[(n_core, slab)],
                n_below,
                "the column beside the guide",
            )?
        } else {
            c64::new(side, 0.0)
        };
        let across: Vec<(c64, f64)> = if cw >= w * (1.0 - 1e-12) {
            vec![(with_film, w)]
        } else {
            let edge = 0.5 * (w - cw);
            vec![(bare, edge), (with_film, cw), (bare, edge)]
        };
        root(lateral, beside, &across, beside, "the lateral stack")
    }

    /// The full-vector solver's n_eff at the film's `fraction`, and the grid it names: the
    /// guided TE-like mode of the largest Re n_eff, found near the effective index method's.
    fn solved(&self, values: &Values, fraction: f64) -> Result<(c64, String)> {
        let cs = self.cross_section(values, fraction)?;
        let near = self.eim(values, fraction)?.re;
        let polarization = PcmShifter::polarization(values);
        let mode = fundamental_of(
            &cs,
            Wavelength::um(values.at("wavelength"))?,
            near,
            polarization,
        )?;
        let h = STEP / self.refinement;
        let mut grid = format!(
            "{} × {} cells of the half cross-section, {:.3} nm in and near the guide and the \
             film, growing by {GROWTH} per µm to {:.0} nm, {MARGIN} µm of cladding",
            cs.x().len() - 1,
            cs.y().len() - 1,
            h * 1e3,
            LARGEST * 1e3,
        );
        if polarization == Polarization::Tm {
            grid.push_str(", the TM-like mode");
        }
        Ok((mode.effective_index(), grid))
    }

    /// The cell's three states at `fidelity`, and the grid they name.
    pub(super) fn states(&self, values: &Values, fidelity: Fidelity) -> Result<(States, String)> {
        values.check(self)?;
        PcmShifter::check_design(values)?;
        let f = values.at("crystallized");
        match fidelity {
            Fidelity::Analytic => Ok((
                States {
                    amorphous: self.eim(values, 0.0)?,
                    now: self.eim(values, f)?,
                    crystalline: self.eim(values, 1.0)?,
                },
                EXACT.to_owned(),
            )),
            _ => {
                let (amorphous, grid) = self.solved(values, 0.0)?;
                let now = if f == 0.0 {
                    amorphous
                } else {
                    self.solved(values, f)?.0
                };
                let crystalline = if f == 1.0 {
                    now
                } else {
                    self.solved(values, 1.0)?.0
                };
                Ok((
                    States {
                        amorphous,
                        now,
                        crystalline,
                    },
                    grid,
                ))
            }
        }
    }

    /// The figures of the three states at `fidelity`, for the cell at `values`.
    fn figures(
        &self,
        values: &Values,
        states: &States,
        fidelity: Fidelity,
        grid: &str,
    ) -> Vec<Figure> {
        let lambda = values.at("wavelength");
        let k0 = TAU / lambda;
        let alpha = |n: c64| loss(n, k0);
        let (a, c) = (alpha(states.amorphous), alpha(states.crystalline));
        let dn = states.now.re - states.amorphous.re;
        let dn_ac = states.crystalline.re - states.amorphous.re;
        let length = values.at("length");
        let f = fidelity;
        let mut figures = vec![
            figure("effective index", "n_eff", states.now.re, "", f, grid),
            figure(
                "loss of the cell",
                "alpha",
                alpha(states.now),
                "dB/µm",
                f,
                grid,
            ),
            figure(
                "effective index change from the amorphous state",
                "dn_eff",
                dn,
                "",
                f,
                grid,
            ),
            figure(
                "effective index change, amorphous to crystalline",
                "dn_eff_ac",
                dn_ac,
                "",
                f,
                grid,
            ),
            figure("loss, amorphous", "alpha_a", a, "dB/µm", f, grid),
            figure("loss, crystalline", "alpha_c", c, "dB/µm", f, grid),
            figure(
                "length for a π shift, amorphous to crystalline",
                "L_pi",
                lambda / (2.0 * dn_ac.abs()),
                "µm",
                f,
                grid,
            ),
        ];
        // Delaney et al.'s Eq. (1): the phase per dB of the lossier state's loss, undefined for
        // a film that absorbs nothing
        let worst = a.max(c);
        if worst > 1e-9 {
            figures.push(figure(
                "figure of merit",
                "FOM",
                crate::material::catalogue::figure_of_merit(
                    dn_ac.abs(),
                    worst,
                    Wavelength::um(lambda).unwrap_or_else(|_| unreachable!("a checked wavelength")),
                ),
                "rad/dB",
                f,
                grid,
            ));
        }
        figures.push(figure(
            "phase against the amorphous state",
            "phi",
            k0 * dn * length,
            "rad",
            f,
            grid,
        ));
        figures
    }

    /// The figures at `fidelity`, the component's phase (rad) and loss (dB), and its
    /// provenance.
    fn model(
        &self,
        values: &Values,
        fidelity: Fidelity,
    ) -> Result<(Vec<Figure>, [f64; 2], Provenance)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let length = values.at("length");
        let extra = values.at("loss") * length * 1e-4;
        let (states, grid) = self.states(values, m.fidelity)?;
        let mut figures = self.figures(values, &states, m.fidelity, &grid);
        let find = |figures: &[Figure], s: &str| {
            figures
                .iter()
                .find(|g| g.symbol == s)
                .map_or(f64::NAN, |g| g.value)
        };
        let phase = find(&figures, "phi");
        let cell_loss = find(&figures, "alpha") * length;
        let loss = cell_loss + extra;
        figures.push(figure(
            "insertion loss of the cell",
            "IL",
            loss,
            "dB",
            m.fidelity,
            &grid,
        ));
        let provenance = Provenance {
            fidelity: m.fidelity,
            source: format!(
                "the phase 2π Δn_eff L/λ = {phase:.4} rad against the amorphous state and the \
                 loss {loss:.4} dB of the cell, by {} ({grid})",
                m.name
            ),
            error: None,
            validity: Some((1.25, 1.65)),
        };
        Ok((figures, [phase, loss], provenance))
    }
}

/// The width of the film on the guide's top, µm: `coverage` of the guide's, or, for a film that
/// follows the guide, the guide's and the film on both walls (the corners square).
fn top_width(values: &Values) -> f64 {
    let w = values.at("width");
    match PcmShifter::conformal(values) {
        None => values.at("coverage") * w,
        Some((wall, _)) => w + 2.0 * wall,
    }
}

/// `flat` (the guide with the film on its top, `wall` wider than the guide's half
/// `half_width` µm on either side) with the film also on the guide's wall, `wall` µm thick from
/// the slab's top `floor` up to the guide's `top`, and over the slab, `over` µm thick from the
/// wall out to the window's edge: the cells whose centres lie there take the film's permittivity
/// `eps`. The faces are the caller's nodes.
///
/// # Errors
///
/// As [`CrossSection::new`] and [`CrossSection::with_boundaries`] (never, for a valid `flat`).
pub(super) fn conformal_film(
    flat: &CrossSection,
    half_width: f64,
    floor: f64,
    top: f64,
    (wall, over): (f64, f64),
    eps: c64,
) -> Result<CrossSection> {
    let (x, y) = (flat.x(), flat.y());
    let ny = y.len() - 1;
    let mut cells = flat.cells().to_vec();
    for i in 0..x.len() - 1 {
        let cx = 0.5 * (x[i] + x[i + 1]);
        if cx < half_width {
            continue;
        }
        for j in 0..ny {
            let cy = 0.5 * (y[j] + y[j + 1]);
            let on_wall = cx < half_width + wall && cy > floor && cy < top;
            let on_slab = cy > floor && cy < floor + over;
            if on_wall || on_slab {
                cells[i * ny + j] = Permittivity::isotropic(eps);
            }
        }
    }
    CrossSection::new(x.to_vec(), y.to_vec(), cells)?.with_boundaries(flat.boundaries())
}

/// The power loss, dB/µm, of a mode of effective index `n` at the free-space wavenumber `k0`
/// (rad/µm). A passive medium's imaginary part is not negative, and below 1e-12 it is round-off
/// (the lossless films' solves leave 1e-16 of either sign): both give 0.
pub(super) fn loss(n: c64, k0: f64) -> f64 {
    if n.im > 1e-12 { DB * k0 * n.im } else { 0.0 }
}

/// The fundamental guided mode of `polarization`, TE-like or TM-like, of `cs`: the one of the
/// largest Re n_eff of that polarization among the three guided modes nearest `near`.
fn fundamental_of(
    cs: &CrossSection,
    wavelength: Wavelength,
    near: f64,
    polarization: Polarization,
) -> Result<vector::VectorMode> {
    let te = polarization == Polarization::Te;
    vector::guided_modes(cs, wavelength, 3, Some(near))?
        .into_iter()
        .filter(|m| (m.te_fraction() > 0.5) == te)
        .max_by(|a, b| a.effective_index().re.total_cmp(&b.effective_index().re))
        .ok_or_else(|| {
            let which = if te { "TE" } else { "TM" };
            Error::invalid(
                "device",
                format!(
                    "the cross-section guides no {which}-like mode (beside a rib, a mode below \
                     the slab's leaks into it: a rib's TM-like mode, or one under a film over \
                     the slab that raises the slab's index above the guide's)"
                ),
            )
        })
}

/// Delaney et al. 2020, with what the device takes from it.
fn delaney() -> String {
    "M. Delaney, I. Zeimpekis, D. Lawson, D. W. Hewak, O. L. Muskens, Adv. Funct. Mater. 30, \
     2002447 (2020), doi:10.1002/adfm.202002447: the indices of both states (their ellipsometry), \
     the 25 nm film and the figure of merit (Eq. 1)"
        .into()
}

/// The shifter's models.
fn methods() -> Vec<Method> {
    vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: "the effective index method with exact complex slabs: the column through the \
                   guide with the film, the column beside it and the lateral stack, each the \
                   root of a transfer matrix's dispersion relation"
                .into(),
            write_up: "devices".into(),
            papers: vec![
                delaney(),
                "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), doi:10.1364/AO.16.000113"
                    .into(),
                "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), \
                 doi:10.1364/JOSAA.1.000742"
                    .into(),
            ],
            error: Some(Stated {
                quantity: "dn_eff_ac".into(),
                bound: STATED_ANALYTIC_2D,
                unit: "%".into(),
                against: Fidelity::TwoD,
            }),
            validity: analytic_validity(),
        },
        Method {
            fidelity: Fidelity::TwoD,
            name: "the full-vector mode solver on the cross-section, complex permittivities, \
                   half of it behind an electric wall, cells from 5 nm in and near the guide \
                   and the film"
                .into(),
            write_up: "devices".into(),
            papers: vec![
                delaney(),
                "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 \
                 (2008), doi:10.1109/JLT.2008.923643"
                    .into(),
                "J. A. Frantz et al., Opt. Mater. Express 13, 3631 (2023), \
                 doi:10.1364/OME.506019"
                    .into(),
            ],
            error: None,
            validity: Vec::new(),
        },
    ]
}

/// The analytic model's largest error in the index change from amorphous to crystalline
/// against the 2D model over its validity, %, as the validation report measures it
/// (`device/pcm-analytic`).
const STATED_ANALYTIC_2D: f64 = 15.0;

/// Where the analytic model is checked against the 2D model.
fn analytic_validity() -> Vec<Range> {
    vec![
        Range::new("width", 0.4, 0.6),
        Range::new("film_thickness", 0.01, 0.05),
        Range::new("slab", 0.0, 0.1),
        // the film on the top alone and the TE-like mode: against the 2D model the method is
        // off by up to 15 % (a film over a rib's slab) and 23 % (the TM-like mode's loss under
        // GST) beyond them (docs/methods/devices.md)
        Range::new("wall_film", 0.0, 0.0),
        Range::new("slab_film", 0.0, 0.0),
        Range::new("tm", 0.0, 0.0),
    ]
}

impl Device for PcmShifter {
    fn kind(&self) -> &str {
        self.pcm.kind()
    }

    fn about(&self) -> &str {
        match self.pcm {
            Pcm::Gst => {
                "A silicon guide with a cell of GST on top: the effective index, phase and loss \
                 of its amorphous and crystalline states and those between, held without power."
            }
            Pcm::Sb2S3 => {
                "A silicon guide with a cell of Sb₂S₃ on top: the effective index, phase and \
                 loss of its amorphous and crystalline states and those between, held without \
                 power."
            }
            Pcm::Sb2Se3 => {
                "A silicon guide with a cell of Sb₂Se₃ on top: the effective index, phase and \
                 loss of its amorphous and crystalline states and those between, held without \
                 power."
            }
        }
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        let film = self.film(values.at("crystallized"), values.at("wavelength"))?;
        self.film_stack(values, film)
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let (w, l, t) = (
            values.at("width"),
            values.at("length"),
            values.at("thickness"),
        );
        let drawing = Straight::new(um(w), um(l)).draw(&self.core)?;
        let p = PcmShifter::profile(values);
        let drawing = if p.is_strip() {
            drawing
        } else {
            super::etched(&drawing, p.etch(t)?)?
        };
        let mut g = super::drawn(Structure::new(self.stack(values)?), &drawing)?;
        g.structure.draw(
            "PCM",
            Shape::rect(
                Point::um(0.5 * l, 0.0),
                Length::um(l),
                Length::um(top_width(values)),
            )?,
        )?;
        Ok(g)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (figures, [phase, loss], provenance) = self.model(values, fidelity)?;
        let value = |symbol: &str| {
            figures
                .iter()
                .find(|g| g.symbol == symbol)
                .map_or(f64::NAN, |g| g.value)
        };
        let length = values.at("length");
        // the phase and the cell's loss grow with the length, the guide's own loss with it too
        let (dphase, dloss) = (phase / length, (loss / length));
        let _ = value;
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "length" => Some(vec![dphase, dloss]),
                "loss" => Some(vec![0.0, length * 1e-4]),
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
