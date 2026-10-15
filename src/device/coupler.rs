//! The strip directional coupler: two strips side by side, coupled through their gap.

use crate::Result;
use crate::circuit::components::{DirectionalCoupler, Dispersion, Supermodes};
use crate::circuit::{Component, Fidelity, Parameter, Provenance};
use crate::generator::{self, Generator};
use crate::mode::vector::{Boundary, CrossSection};
use crate::stack::{LayerStack, Structure};
use crate::units::Wavelength;

use super::fit;
use super::guide::{Strip, um};
use super::section::Mesh;
use super::waveguide::{
    analytic_papers, band_parameters, etch_parameters, profile, solver_papers, strip_parameters,
    strip_validity, strips_only,
};
use super::{
    Built, Device, Figure, Geometry, Method, Range, Stated, Values, figure, method, parameter,
};

/// The coupled section of a directional coupler: two silicon strips a gap apart, ports `o1`
/// (lower left), `o2` (upper left), `o3` (upper right) and `o4` (lower right), its component a
/// [`DirectionalCoupler`] by coupled-mode theory from the pair's two supermodes (Chrostowski &
/// Hochberg 2015, Section 4.1).
///
/// Parameters `width`, `thickness`, `slab` and `sidewall` (the guides' profile, as the
/// [`super::StripWaveguide`]'s: ribs on a slab, sloped walls; the analytic fit is strips'),
/// `gap` (edge to edge, at the guides' tops), `length` (the coupled section's),
/// `loss` (dB/cm, each guide's), `wavelength` and `span`.
///
/// - **Analytic:** supermodes split by Δn = Cλ/π about the strip's analytic mode (Chrostowski
///   & Hochberg's Eq. 4.3), C from the fit of the solver's coupling over the gap, width and
///   thickness at 1.55 µm, constant in λ.
/// - **3D:** the full-vector solver's even and odd supermodes, each on half the pair behind a
///   mirror wall, at five wavelengths across the span.
///
/// The bends that bring the guides together and apart are not included.
#[derive(Clone, Debug, PartialEq)]
pub struct StripCoupler {
    strip: Strip,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

impl Default for StripCoupler {
    fn default() -> StripCoupler {
        StripCoupler::new()
    }
}

/// The analytic 3 dB length at the defaults, the coupler's own default (L_x/2).
fn default_length() -> f64 {
    let guide = Dispersion::new(
        Wavelength::from_um_unchecked(fit::WAVELENGTH),
        fit::ANCHOR.0,
        fit::ANCHOR.1,
    )
    .with_loss(2.7);
    DirectionalCoupler::new(Supermodes::from_coupling(guide, fit::COUPLING_ANCHOR)).map_or_else(
        |_| unreachable!("the library's supermodes are split"),
        |c| c.defaults()[0],
    )
}

impl StripCoupler {
    /// Two 500 × 220 nm silicon strips 200 nm apart on 220 nm SOI, their supermodes solved on
    /// cells fitted about 10 nm apart.
    pub fn new() -> StripCoupler {
        StripCoupler::of(Strip::soi())
    }

    /// The coupler drawn on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str) -> Result<StripCoupler> {
        Ok(StripCoupler::of(Strip::on(stack, core)?))
    }

    fn of(strip: Strip) -> StripCoupler {
        let mut parameters = strip_parameters();
        parameters.extend(etch_parameters());
        parameters.push(parameter("gap", "µm", 0.2, 0.02, 2.0));
        parameters.push(parameter("length", "µm", default_length(), 0.0, 1e4));
        parameters.push(parameter("loss", "dB/cm", 2.7, 0.0, 1e3));
        parameters.extend(band_parameters());
        StripCoupler {
            methods: methods(),
            strip,
            parameters,
        }
    }

    /// The same device, solved on `mesh`.
    #[must_use]
    pub fn with_mesh(mut self, mesh: Mesh) -> StripCoupler {
        self.strip.mesh = mesh;
        self
    }

    /// The mesh its solves use.
    pub(super) fn mesh(&self) -> Mesh {
        self.strip.mesh
    }

    /// What the full-vector solver sees at `values` and `wavelength`: the pair's even and odd
    /// cross-sections (an electric and a magnetic wall between the guides on the mirrored
    /// domain; on the full domain both are the same).
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for values that aren't this device's, a stack that isn't
    /// mirrored on a mirrored mesh, or a material outside its range.
    pub fn cross_sections(
        &self,
        values: &Values,
        wavelength: Wavelength,
    ) -> Result<(CrossSection, CrossSection)> {
        values.check(self)?;
        let (w, t, g) = (values.at("width"), values.at("thickness"), values.at("gap"));
        let p = profile(values);
        Ok((
            self.strip
                .pair_section_of(w, t, p, g, Boundary::ElectricWall, wavelength)?,
            self.strip
                .pair_section_of(w, t, p, g, Boundary::MagneticWall, wavelength)?,
        ))
    }

    /// The coupler at `fidelity` and `values`, as its component.
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    pub fn coupler(&self, values: &Values, fidelity: Fidelity) -> Result<DirectionalCoupler> {
        Ok(self.build(values, fidelity)?.0)
    }

    /// The coupler and the grid its figures name.
    fn build(&self, values: &Values, fidelity: Fidelity) -> Result<(DirectionalCoupler, String)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let (w, t, g) = (values.at("width"), values.at("thickness"), values.at("gap"));
        let (l0, span, loss) = (
            values.at("wavelength"),
            values.at("span"),
            values.at("loss"),
        );
        let p = profile(values);
        match m.fidelity {
            Fidelity::Analytic => {
                strips_only(p)?;
                let (guide, _) = self.strip.analytic(w, t, l0, loss)?;
                let (c, _) = fit::coupling(g, w, t)?;
                Ok((
                    DirectionalCoupler::new(Supermodes::from_coupling(guide, c))?,
                    fit::COUPLING_GRID.into(),
                ))
            }
            _ => {
                let at = Wavelength::um(l0)?;
                let even = |wl| {
                    self.strip
                        .pair_section_of(w, t, p, g, Boundary::ElectricWall, wl)
                };
                let odd = |wl| {
                    self.strip
                        .pair_section_of(w, t, p, g, Boundary::MagneticWall, wl)
                };
                let grid = Strip::grid(&even(at)?);
                if loss == 0.0 {
                    return Ok((
                        DirectionalCoupler::from_modes(even, odd, at, span / 4.0)?,
                        grid,
                    ));
                }
                // the solver's supermodes, each losing the guides' loss too
                let (mut modes, residual, _) = Supermodes::from_modes(even, odd, at, span / 4.0)?;
                modes.even.loss_db_per_cm += loss;
                modes.odd.loss_db_per_cm += loss;
                let provenance = Provenance {
                    fidelity: Fidelity::ThreeD,
                    source: format!(
                        "coupled-mode theory (Chrostowski & Hochberg 2015, Eqs. 4.1-4.11) from \
                         the full-vector solver's supermodes on {grid}: n_eff {:.6} and {:.6}, \
                         cross-over length {:.3} um at {l0} um, the coupled section only, \
                         {loss} dB/cm; within {residual:.1e} of the solver at ±{} um",
                        modes.even.effective_index,
                        modes.odd.effective_index,
                        modes.cross_over_length(at),
                        span / 2.0
                    ),
                    error: None,
                    validity: Some((l0 - span / 2.0, l0 + span / 2.0)),
                };
                Ok((
                    DirectionalCoupler::new(modes)?.with_provenance(provenance),
                    grid,
                ))
            }
        }
    }
}

/// The coupler's models.
fn methods() -> Vec<Method> {
    let mut validity = super::waveguide::fit_validity();
    validity.retain(|r| r.name != "thickness" && r.name != "wavelength");
    let t = fit::PAIR_THICKNESSES;
    validity.push(Range::new("thickness", t[0], t[t.len() - 1]));
    validity.push(Range::new(
        "gap",
        fit::GAPS[0],
        fit::GAPS[fit::GAPS.len() - 1],
    ));
    validity.push(Range::new("wavelength", 1.548, 1.552));
    validity.extend(strip_validity());
    vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: "coupled-mode theory on supermodes split by Δn = Cλ/π, C from a fit of the \
                   solver's coupling over the gap, width and thickness at 1.55 µm"
                .into(),
            write_up: "devices".into(),
            papers: analytic_papers(),
            error: Some(Stated {
                quantity: "L_x".into(),
                bound: 3.0,
                unit: "%".into(),
                against: Fidelity::ThreeD,
            }),
            validity,
        },
        Method {
            fidelity: Fidelity::ThreeD,
            name: "coupled-mode theory on the full-vector solver's even and odd supermodes, with \
                   mirror walls"
                .into(),
            write_up: "devices".into(),
            papers: solver_papers()[..1].to_vec(),
            error: None,
            validity: Vec::new(),
        },
    ]
}

impl Device for StripCoupler {
    fn kind(&self) -> &str {
        "directional coupler"
    }

    fn about(&self) -> &str {
        "Two strips side by side, a gap apart: the light crosses from one to the other and back \
         along the coupled length."
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        self.strip.stack(values.at("thickness"))
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let (w, g, l) = (values.at("width"), values.at("gap"), values.at("length"));
        let drawing =
            generator::DirectionalCoupler::new(um(w), um(g), um(l)).draw(&self.strip.core)?;
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
        let (coupler, grid) = self.build(values, fidelity)?;
        let modes = *coupler.supermodes();
        let l0 = modes.even.wavelength;
        let length = values.at("length");
        let s = coupler.s_matrix(l0, &[length])?;
        let f = fidelity;
        let lx = modes.cross_over_length(l0);
        let figures: Vec<Figure> = vec![
            figure(
                "supermodes' index difference",
                "dn",
                modes.splitting(l0),
                "",
                f,
                &grid,
            ),
            figure(
                "coupling coefficient",
                "C",
                modes.coupling(l0),
                "1/µm",
                f,
                &grid,
            ),
            figure("cross-over length", "L_x", lx, "µm", f, &grid),
            figure("3 dB length", "L_3dB", lx / 2.0, "µm", f, &grid),
            figure(
                "power across at its length",
                "cross",
                s.power(2, 0),
                "",
                f,
                &grid,
            ),
            figure(
                "power through at its length",
                "through",
                s.power(3, 0),
                "",
                f,
                &grid,
            ),
        ];
        let derivatives = self
            .parameters
            .iter()
            .map(|p| (p.name == "length").then(|| vec![1.0]))
            .collect();
        Ok(Built {
            values: vec![length],
            derivatives,
            figures,
            component: Box::new(coupler),
        })
    }
}
