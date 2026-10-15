//! The strip waveguide: a straight silicon strip, its phase and loss over a length.

use crate::Result;
use crate::circuit::components::{Dispersion, Waveguide};
use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::generator::{Generator, Straight};
use crate::mode::vector::CrossSection;
use crate::stack::{LayerStack, Structure};
use crate::units::Wavelength;

use super::guide::{Profile, Strip, um};
use super::section::{self, Mesh};
use super::{
    Built, Device, Figure, Geometry, Method, Range, Stated, Values, figure, method, parameter,
};

/// The papers of a strip's analytic model.
pub(super) fn analytic_papers() -> Vec<String> {
    vec![
        "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press \
         (2015), doi:10.1017/CBO9781316084168, Eqs. 3.5-3.6"
            .into(),
        "G. R. Hadley, J. Lightwave Technol. 20, 1210 (2002), doi:10.1109/JLT.2002.800361".into(),
        "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371".into(),
    ]
}

/// The papers of the full-vector solver.
pub(super) fn solver_papers() -> Vec<String> {
    vec![
        "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008), \
         doi:10.1109/JLT.2008.923643"
            .into(),
        "G. R. Hadley, J. Lightwave Technol. 20, 1210 (2002), doi:10.1109/JLT.2002.800361".into(),
        "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371".into(),
    ]
}

/// The ranges the analytic fit holds over: width, thickness and the band.
pub(super) fn fit_validity() -> Vec<Range> {
    let w = super::fit::WIDTHS;
    let t = super::fit::THICKNESSES;
    vec![
        Range::new("width", w[0], w[w.len() - 1]),
        Range::new("thickness", t[0], t[t.len() - 1]),
        Range::new("wavelength", super::fit::BAND.0, super::fit::BAND.1),
    ]
}

/// The figures of a straight guide's dispersion.
pub(super) fn guide_figures(d: &Dispersion, fidelity: Fidelity, grid: &str) -> Vec<Figure> {
    let mut f = vec![
        figure(
            "effective index",
            "n_eff",
            d.effective_index,
            "",
            fidelity,
            grid,
        ),
        figure("group index", "n_g", d.group_index, "", fidelity, grid),
    ];
    if fidelity != Fidelity::Analytic {
        f.push(figure(
            "dispersion",
            "D",
            d.dispersion,
            "ps/(nm km)",
            fidelity,
            grid,
        ));
    }
    f
}

/// A straight silicon strip on a stack: ports `o1` and `o2`, its component a
/// [`Waveguide`], e^(iγL).
///
/// Parameters `width`, `thickness` (the core layer's), `slab` and `sidewall` (how the layer is
/// cut beside the strip: a slab of it left there makes a rib, and walls below 90° slope out
/// going down; a strip has neither), `length`, `loss` (dB/cm, the propagation loss the mode
/// solvers don't give: sidewall roughness, absorption), and the band its mode is modelled over,
/// `wavelength` (the centre) and `span`.
///
/// - **Analytic:** the closed form on the fit's dispersion (n_eff and n_g at 1.55 µm, first
///   order in λ), or on a stated one ([`StripWaveguide::with_guide`]). The fit is a strip's
///   with vertical walls, and refuses a slab or a sloped wall.
/// - **3D:** the full-vector solver's mode at five wavelengths across the span, by Hadley's
///   equations or the standard scheme ([`Mesh::scheme`]); a rib or a sloped wall on the half
///   behind the vertical mirror plane, and sloped walls by the standard scheme.
#[derive(Clone, Debug, PartialEq)]
pub struct StripWaveguide {
    strip: Strip,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

impl Default for StripWaveguide {
    fn default() -> StripWaveguide {
        StripWaveguide::new()
    }
}

/// The parameters every strip device has: width and thickness.
pub(super) fn strip_parameters() -> Vec<Parameter> {
    vec![
        parameter("width", "µm", 0.5, 0.1, 5.0),
        parameter("thickness", "µm", 0.22, 0.05, 2.0),
    ]
}

/// How a strip device's core layer is cut beside its guides: `slab` (µm of the layer left
/// beside them, 0 for a strip etched through) and `sidewall` (the walls' angle from the
/// layer's plane, degrees, 90 vertical). A slab makes them ribs.
pub(super) fn etch_parameters() -> Vec<Parameter> {
    vec![
        parameter("slab", "µm", 0.0, 0.0, 5.0),
        parameter("sidewall", "°", 90.0, 30.0, 90.0),
    ]
}

/// The profile `values` give a strip device's guides: its `slab` and `sidewall`.
pub(super) fn profile(values: &Values) -> Profile {
    Profile {
        slab: values.at("slab"),
        sidewall: values.at("sidewall"),
    }
}

/// Refuses a rib or a sloped wall at a fit made on strips with vertical walls.
pub(super) fn strips_only(profile: Profile) -> Result<()> {
    if profile.is_strip() {
        Ok(())
    } else {
        Err(crate::Error::invalid(
            "device",
            format!(
                "the analytic fit is for strips with vertical walls, not a {} um slab with walls \
                 at {} degrees: solve the modes (3D)",
                profile.slab, profile.sidewall
            ),
        ))
    }
}

/// Where the analytic fit holds in a strip's profile: no slab, vertical walls.
pub(super) fn strip_validity() -> Vec<Range> {
    vec![
        Range::new("slab", 0.0, 0.0),
        Range::new("sidewall", 90.0, 90.0),
    ]
}

/// The band parameters: the centre wavelength and the span its solves sample.
pub(super) fn band_parameters() -> Vec<Parameter> {
    vec![
        parameter("wavelength", "µm", 1.55, 0.4, 10.0),
        parameter("span", "µm", 0.1, 0.004, 0.4),
    ]
}

impl StripWaveguide {
    /// A 500 × 220 nm silicon strip on 220 nm SOI ([`LayerStack::soi_220`]), solved on cells
    /// fitted about 10 nm apart by Hadley's equations.
    pub fn new() -> StripWaveguide {
        StripWaveguide::of(Strip::soi())
    }

    /// The strip drawn on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str) -> Result<StripWaveguide> {
        Ok(StripWaveguide::of(Strip::on(stack, core)?))
    }

    fn of(strip: Strip) -> StripWaveguide {
        let mut parameters = strip_parameters();
        parameters.extend(etch_parameters());
        parameters.push(parameter("length", "µm", 10.0, 0.0, 1e5));
        parameters.push(parameter("loss", "dB/cm", 2.7, 0.0, 1e3));
        parameters.extend(band_parameters());
        StripWaveguide {
            methods: methods(&strip),
            strip,
            parameters,
        }
    }

    /// The same device, solved on `mesh`.
    #[must_use]
    pub fn with_mesh(mut self, mesh: Mesh) -> StripWaveguide {
        self.strip.mesh = mesh;
        self
    }

    /// The same device, its analytic fidelity the closed form on `guide`, a dispersion stated
    /// for its geometry (measured, or solved elsewhere) instead of the fit's; its loss is the
    /// `loss` parameter's.
    #[must_use]
    pub fn with_guide(mut self, guide: Dispersion) -> StripWaveguide {
        self.strip.stated = Some(guide);
        self.methods = methods(&self.strip);
        self
    }

    /// The mesh its solves use.
    pub fn mesh(&self) -> Mesh {
        self.strip.mesh
    }

    /// The layer its strip is drawn on.
    pub(super) fn core(&self) -> &str {
        &self.strip.core
    }

    /// What the full-vector solver sees at `values` and `wavelength`: the strip's
    /// cross-section on the mesh.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for values that aren't this device's, a stack that isn't
    /// mirrored on a mirrored mesh, or a material outside its range.
    pub fn cross_section(&self, values: &Values, wavelength: Wavelength) -> Result<CrossSection> {
        values.check(self)?;
        self.strip.cross_section_of(
            values.at("width"),
            values.at("thickness"),
            profile(values),
            wavelength,
        )
    }

    /// Its mode's dispersion at `fidelity` and `values`, and the grid it names: the fit's, or
    /// the cross-section's (`"nx × ny cells, h nm"`).
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    pub fn dispersion(&self, values: &Values, fidelity: Fidelity) -> Result<(Dispersion, String)> {
        let (guide, grid) = self.build(values, fidelity)?;
        Ok((*guide.dispersion(), grid))
    }

    /// The waveguide at `fidelity` and `values`, as its component.
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    pub fn waveguide(&self, values: &Values, fidelity: Fidelity) -> Result<Waveguide> {
        Ok(self.build(values, fidelity)?.0)
    }

    /// The waveguide, and the grid its figures name.
    fn build(&self, values: &Values, fidelity: Fidelity) -> Result<(Waveguide, String)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let (w, t) = (values.at("width"), values.at("thickness"));
        let (l0, span) = (values.at("wavelength"), values.at("span"));
        let p = profile(values);
        if m.fidelity == Fidelity::Analytic {
            if self.strip.stated.is_none() {
                strips_only(p)?;
            }
            let (d, grid) = self.strip.analytic(w, t, l0, values.at("loss"))?;
            return Ok((Waveguide::new(d), grid));
        }
        let at = Wavelength::um(l0)?;
        let grid = Strip::grid(&self.strip.cross_section_of(w, t, p, at)?);
        let guide = match self.strip.scheme_for(p) {
            section::Scheme::Standard => Waveguide::from_modes(
                |wl| self.strip.cross_section_of(w, t, p, wl),
                at,
                span / 4.0,
                None,
            )?,
            section::Scheme::Hadley => {
                let (d, residual, grid) = self.strip.solved_of(w, t, p, l0, span)?;
                let provenance = hadley_provenance(&d, residual, &grid, l0, span);
                Waveguide::new(d).with_provenance(provenance)
            }
        };
        Ok((guide, grid))
    }
}

/// The waveguide's models.
fn methods(strip: &Strip) -> Vec<Method> {
    let analytic = if strip.stated.is_some() {
        Method {
            fidelity: Fidelity::Analytic,
            name: "e^(iγL) on a stated dispersion: n_eff and n_g at λ₀, first order in λ".into(),
            write_up: "devices".into(),
            papers: analytic_papers()[..1].to_vec(),
            error: None,
            validity: Vec::new(),
        }
    } else {
        Method {
            fidelity: Fidelity::Analytic,
            name: "e^(iγL) on a fit of n_eff and n_g at 1.55 µm over the width and thickness, \
                   first order in λ"
                .into(),
            write_up: "devices".into(),
            papers: analytic_papers(),
            error: Some(Stated {
                quantity: "n_g".into(),
                bound: 0.02,
                unit: String::new(),
                against: Fidelity::ThreeD,
            }),
            validity: [fit_validity(), strip_validity()].concat(),
        }
    };
    vec![
        analytic,
        Method {
            fidelity: Fidelity::ThreeD,
            name: "e^(iγL) on the full-vector solver's mode at five wavelengths across the span"
                .into(),
            write_up: "devices".into(),
            papers: solver_papers(),
            error: None,
            validity: Vec::new(),
        },
    ]
}

impl Device for StripWaveguide {
    fn kind(&self) -> &str {
        "waveguide"
    }

    fn about(&self) -> &str {
        "A straight strip: its mode's phase and loss over a length, from the strip's width and \
         thickness."
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        self.strip.stack(values.at("thickness"))
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let (w, l) = (values.at("width"), values.at("length"));
        let drawing = Straight::new(um(w), um(l)).draw(&self.strip.core)?;
        let p = profile(values);
        let drawing = if p.is_strip() {
            drawing
        } else {
            super::etched(&drawing, p.etch(values.at("thickness"))?)?
        };
        super::drawn(Structure::new(self.stack(values)?), &drawing)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (guide, grid) = self.build(values, fidelity)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let figures = guide_figures(guide.dispersion(), f, &grid);
        let n = self.parameters.len();
        // the component's length and loss are the device's own
        let derivatives = (0..n)
            .map(|k| match self.parameters[k].name.as_str() {
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

/// The provenance of a waveguide whose mode is Hadley's, as [`Waveguide::from_modes`] states
/// the standard scheme's.
pub(super) fn hadley_provenance(
    d: &Dispersion,
    residual: f64,
    grid: &str,
    l0: f64,
    span: f64,
) -> Provenance {
    Provenance {
        fidelity: Fidelity::ThreeD,
        source: format!(
            "the full-vector mode solver by Hadley's high-accuracy equations \
             (doi:10.1109/JLT.2002.800361, doi:10.1109/JLT.2002.800371) on {grid}: n_eff {:.6}, \
             n_g {:.4}, D {:.0} ps/(nm km) at {l0} um; n_eff second order in λ, within \
             {residual:.1e} of the solver at ±{} um",
            d.effective_index,
            d.group_index,
            d.dispersion,
            span / 2.0
        ),
        error: None,
        validity: Some((l0 - span / 2.0, l0 + span / 2.0)),
    }
}
