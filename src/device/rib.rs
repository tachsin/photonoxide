//! The rib waveguide: a strip etched part of the way through its core layer, standing on a slab
//! of the same material; its phase and loss over a length.

use crate::circuit::components::{Dispersion, Waveguide};
use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::generator::{Generator, Straight};
use crate::mode::Polarization;
use crate::mode::slab::{Slab, SlabMode};
use crate::mode::vector::CrossSection;
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

use super::guide::{Profile, Strip, um};
use super::section::{Mesh, Scheme};
use super::waveguide::{band_parameters, guide_figures, hadley_provenance, solver_papers};
use super::{Built, Device, Geometry, Method, Range, Stated, Values, method, parameter};

/// A rib: a strip `width` wide on top (its walls at `sidewall` degrees from the layer's plane,
/// so wider at its foot when they slope), etched `etch_depth` into a core layer `thickness`
/// thick, the layer's material left as a slab `thickness − etch_depth` thick beside it. Ports
/// `o1` and `o2`, its component a [`Waveguide`], e^(iγL).
///
/// Parameters `width`, `thickness`, `etch_depth`, `sidewall` (degrees), `length`, `loss` (dB/cm,
/// what the mode solvers don't give), `wavelength` and `span`.
///
/// - **2D:** the effective index method (Hocker & Burns 1977; [`crate::mode::eim`]): the slabs
///   through the rib and beside it give the lateral slab's indices, TE-like. Labelled 2D: it is
///   an approximation of the 3D guide, whose error against the full-vector solver the validation
///   report measures and its method states. Sloped walls are taken at their mean width.
/// - **3D:** the full-vector solver's mode at five wavelengths across the span, on the half
///   behind the vertical mirror plane (a rib isn't the same above and below its mid-plane), by
///   Hadley's equations for vertical walls (his standard rib reproduces to 1.4e-6) and by the
///   standard scheme for sloped ones.
#[derive(Clone, Debug, PartialEq)]
pub struct RibWaveguide {
    strip: Strip,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

impl Default for RibWaveguide {
    fn default() -> RibWaveguide {
        RibWaveguide::new()
    }
}

/// The rib's default mesh: cells fitted about 10 nm apart with 1.5 µm of cladding (the slab
/// carries the mode's tail farther out than a strip's), by Hadley's equations.
fn default_mesh() -> Mesh {
    Mesh::fitted(0.01, 1.5, Scheme::Hadley)
}

impl RibWaveguide {
    /// A rib 500 nm wide on 220 nm SOI ([`LayerStack::soi_220`]), etched 130 nm (a 90 nm
    /// slab) with vertical walls.
    pub fn new() -> RibWaveguide {
        let mut strip = Strip::soi();
        strip.mesh = default_mesh();
        RibWaveguide::of(strip)
    }

    /// The rib drawn on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str) -> Result<RibWaveguide> {
        let mut strip = Strip::on(stack, core)?;
        strip.mesh = default_mesh();
        Ok(RibWaveguide::of(strip))
    }

    fn of(strip: Strip) -> RibWaveguide {
        let mut parameters = vec![
            parameter("width", "µm", 0.5, 0.1, 50.0),
            parameter("thickness", "µm", 0.22, 0.05, 5.0),
            parameter("etch_depth", "µm", 0.13, 0.005, 5.0),
            parameter("sidewall", "°", 90.0, 30.0, 90.0),
            parameter("length", "µm", 10.0, 0.0, 1e5),
            parameter("loss", "dB/cm", 2.7, 0.0, 1e3),
        ];
        parameters.extend(band_parameters());
        RibWaveguide {
            methods: methods(),
            strip,
            parameters,
        }
    }

    /// The same device, solved on `mesh` (a mirrored one is used on its vertical plane only).
    #[must_use]
    pub fn with_mesh(mut self, mesh: Mesh) -> RibWaveguide {
        self.strip.mesh = mesh;
        self
    }

    /// The mesh its solves use.
    pub fn mesh(&self) -> Mesh {
        self.strip.mesh
    }

    /// The rib's profile at `values`.
    fn profile(values: &Values) -> Result<Profile> {
        let (t, d) = (values.at("thickness"), values.at("etch_depth"));
        if d >= t {
            return Err(Error::invalid(
                "device",
                format!(
                    "a rib's etch_depth, {d} um, must be less than its thickness, {t} um: \
                     etched through, it is a strip (the waveguide)"
                ),
            ));
        }
        Ok(Profile {
            slab: t - d,
            sidewall: values.at("sidewall"),
        })
    }

    /// What the full-vector solver sees at `values` and `wavelength`: the rib's cross-section
    /// on the mesh.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's, an etch through the core,
    /// or a material outside its range.
    pub fn cross_section(&self, values: &Values, wavelength: Wavelength) -> Result<CrossSection> {
        values.check(self)?;
        let profile = RibWaveguide::profile(values)?;
        self.strip.cross_section_of(
            values.at("width"),
            values.at("thickness"),
            profile,
            wavelength,
        )
    }

    /// Its mode's dispersion at `fidelity` and `values`, and the grid it names.
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    pub fn dispersion(&self, values: &Values, fidelity: Fidelity) -> Result<(Dispersion, String)> {
        let (guide, grid) = self.build(values, fidelity)?;
        Ok((*guide.dispersion(), grid))
    }

    /// The effective index method's mode at `values` and `wavelength`: n_eff, and the slab's
    /// beside the rib.
    ///
    /// # Errors
    ///
    /// As [`Device::component`] at 2D.
    pub fn effective_index(&self, values: &Values, wavelength: Wavelength) -> Result<(f64, f64)> {
        values.check(self)?;
        self.eim(values, wavelength)
    }

    /// The effective index method's n_eff and side index at `wavelength`: the rib at its mean
    /// width between the stack's indices.
    fn eim(&self, values: &Values, wavelength: Wavelength) -> Result<(f64, f64)> {
        let profile = RibWaveguide::profile(values)?;
        let t = values.at("thickness");
        let (below, core, above, beside) = self.strip.indices(t, wavelength)?;
        if beside != above {
            return Err(Error::invalid(
                "device",
                format!(
                    "the effective index method here takes the medium beside the rib ({beside}) \
                     to be the one above the core ({above}): solve the modes (3D)"
                ),
            ));
        }
        // a sloped wall at its mean width: the trapezoid's area over its height
        let depth = t - profile.slab;
        let run = if profile.sidewall == 90.0 {
            0.0
        } else {
            1.0 / profile.sidewall.to_radians().tan()
        };
        let width = values.at("width") + depth * run;
        let (centre, side) = slab_indices((below, core, above), t, profile.slab, wavelength)?;
        let lateral = Slab::new(side, centre, side, Length::um(width))?
            .modes(Polarization::Tm, wavelength)
            .first()
            .map(SlabMode::effective_index)
            .ok_or_else(|| {
                Error::invalid(
                    "effective index method",
                    "the lateral slab guides no mode: the rib is too narrow",
                )
            })?;
        Ok((lateral, side))
    }

    /// The waveguide, and the grid its figures name.
    fn build(&self, values: &Values, fidelity: Fidelity) -> Result<(Waveguide, String)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let profile = RibWaveguide::profile(values)?;
        let (w, t) = (values.at("width"), values.at("thickness"));
        let (l0, span, loss) = (
            values.at("wavelength"),
            values.at("span"),
            values.at("loss"),
        );
        let at = Wavelength::um(l0)?;
        if m.fidelity == Fidelity::TwoD {
            let step = span / 4.0;
            let mut n = [0.0; 5];
            for (k, slot) in n.iter_mut().enumerate() {
                let wl = Wavelength::um(l0 + (k as f64 - 2.0) * step)?;
                *slot = self.eim(values, wl)?.0;
            }
            let d1 = (n[3] - n[1]) / (2.0 * step);
            let d2 = (n[3] - 2.0 * n[2] + n[1]) / (step * step);
            let d = Dispersion::new(at, n[2], n[2] - l0 * d1)
                .with_dispersion(-l0 * d2 / SPEED_OF_LIGHT * 1e12)
                .with_loss(loss)
                .with_polarization(Polarization::Te);
            let grid = EIM_GRID.to_owned();
            let provenance = Provenance {
                fidelity: Fidelity::TwoD,
                source: format!(
                    "the effective index method (Hocker & Burns 1977, doi:10.1364/AO.16.000113) \
                     on the rib's slabs, TE-like: n_eff {:.6}, n_g {:.4} at {l0} um, from five \
                     wavelengths {step} um apart; an approximation of the 3D guide, {loss} dB/cm",
                    d.effective_index, d.group_index
                ),
                error: None,
                validity: Some((l0 - span / 2.0, l0 + span / 2.0)),
            };
            return Ok((Waveguide::new(d).with_provenance(provenance), grid));
        }
        let grid = Strip::grid(&self.strip.cross_section_of(w, t, profile, at)?);
        let guide = match self.strip.scheme_for(profile) {
            Scheme::Standard => Waveguide::from_modes(
                |wl| self.strip.cross_section_of(w, t, profile, wl),
                at,
                span / 4.0,
                None,
            )?,
            Scheme::Hadley => {
                let (d, residual, grid) = self.strip.solved_of(w, t, profile, l0, span)?;
                let provenance = hadley_provenance(&d, residual, &grid, l0, span);
                Waveguide::new(d).with_provenance(provenance)
            }
        };
        Ok((guide, grid))
    }
}

/// The effective index method's vertical slabs (Hocker & Burns 1977; [`crate::mode::eim`]): the
/// TE index through a core `thickness` µm thick of the indices `(below, core, above)`, and
/// beside it, through the slab `slab` thick left there (the cladding above's index for a strip;
/// where the slab guides nothing, the higher of below's and above's, which its light leaks into),
/// at `wavelength`.
///
/// # Errors
///
/// [`Error::InvalidValue`] if the core's slab guides nothing.
pub(super) fn slab_indices(
    (below, core, above): (f64, f64, f64),
    thickness: f64,
    slab: f64,
    wavelength: Wavelength,
) -> Result<(f64, f64)> {
    let te = |t: f64| -> Result<Option<f64>> {
        Ok(Slab::new(below, core, above, Length::um(t))?
            .modes(Polarization::Te, wavelength)
            .first()
            .map(SlabMode::effective_index))
    };
    let centre = te(thickness)?.ok_or_else(|| {
        Error::invalid(
            "effective index method",
            "the core's vertical slab guides no mode",
        )
    })?;
    let side = if slab > 0.0 {
        te(slab)?.unwrap_or(below.max(above))
    } else {
        above
    };
    Ok((centre, side))
}

/// The grid the effective index method's figures name: its slabs are exact.
const EIM_GRID: &str = "2D, the effective index method: exact slabs, no grid";

/// The papers of the effective index method.
fn eim_papers() -> Vec<String> {
    vec![
        "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), doi:10.1364/AO.16.000113".into(),
        "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press \
         (2015), doi:10.1017/CBO9781316084168, Section 3.2.5"
            .into(),
    ]
}

/// The rib's models.
fn methods() -> Vec<Method> {
    vec![
        Method {
            fidelity: Fidelity::TwoD,
            name: "e^(iγL) on the effective index method's mode (the slabs through and beside \
                   the rib, then the lateral slab), TE-like: an approximation, labelled 2D"
                .into(),
            write_up: "devices".into(),
            papers: eim_papers(),
            error: Some(Stated {
                quantity: "n_eff".into(),
                bound: EIM_ERROR,
                unit: String::new(),
                against: Fidelity::ThreeD,
            }),
            validity: vec![
                Range::new("width", 0.4, 3.0),
                Range::new("etch_depth", 0.05, 0.2),
                Range::new("sidewall", 90.0, 90.0),
            ],
        },
        Method {
            fidelity: Fidelity::ThreeD,
            name: "e^(iγL) on the full-vector solver's mode at five wavelengths across the span, \
                   on the half behind the vertical mirror plane"
                .into(),
            write_up: "devices".into(),
            papers: solver_papers(),
            error: None,
            validity: Vec::new(),
        },
    ]
}

/// The effective index method's largest |Δn_eff| against the 3D model over its stated range on
/// 220 nm SOI (`device/rib-eim`: 0.064, narrow and deep, 0.4 µm wide etched 200 nm; 3e-4 at 3 µm
/// wide).
pub(super) const EIM_ERROR: f64 = 0.07;

impl Device for RibWaveguide {
    fn kind(&self) -> &str {
        "rib waveguide"
    }

    fn about(&self) -> &str {
        "A rib: a strip etched part of the way into its core layer, on a slab of the same \
         material; its mode's phase and loss over a length."
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        self.strip.stack(values.at("thickness"))
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let (w, l, t) = (
            values.at("width"),
            values.at("length"),
            values.at("thickness"),
        );
        let etch = RibWaveguide::profile(values)?.etch(t)?;
        let drawing = Straight::new(um(w), um(l)).draw(&self.strip.core)?;
        super::drawn(
            Structure::new(self.stack(values)?),
            &super::etched(&drawing, etch)?,
        )
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (guide, grid) = self.build(values, fidelity)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let mut figures = guide_figures(guide.dispersion(), f, &grid);
        if f == Fidelity::TwoD {
            let at = Wavelength::um(values.at("wavelength"))?;
            let (_, side) = self.effective_index(values, at)?;
            figures.push(super::figure(
                "the slab's effective index beside it",
                "n_slab",
                side,
                "",
                f,
                &grid,
            ));
        }
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "length" => Some(vec![1.0, 0.0]),
                "loss" => Some(vec![0.0, 1.0]),
                _ => None,
            })
            .collect();
        Ok(Built {
            values: vec![values.at("length"), values.at("loss")],
            derivatives,
            figures,
            component: Box::new(guide),
        })
    }
}
