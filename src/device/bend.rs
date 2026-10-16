//! The strip bend: a circular arc of the strip, its phase and loss along the arc.

use std::f64::consts::PI;

use crate::circuit::components::{Bend, Dispersion};
use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::generator::{self, Generator};
use crate::mode::Polarization;
use crate::mode::bend::SlabBend;
use crate::mode::eim::Ridge;
use crate::mode::vector::CrossSection;
use crate::stack::{LayerStack, Structure};
use crate::units::Wavelength;
use crate::{Error, Result};

use super::guide::{Strip, um};
use super::section::Mesh;
use super::waveguide::{
    analytic_papers, band_parameters, fit_validity, solver_papers, strip_parameters,
};
use super::{
    Built, Device, Figure, Geometry, Method, Range, Stated, Values, figure, method, parameter,
};

/// A circular bend of a silicon strip: ports `o1` and `o2`, its component a [`Bend`],
/// e^(iγRθ), the bent mode's index along the arc at the radius R (the strip's centre line).
///
/// Parameters `width`, `thickness`, `radius`, `angle` (degrees), `loss` (dB/cm besides the
/// radiation: the guide's own), `wavelength` and `span`.
///
/// - **Analytic:** the straight strip's mode (the [`super::StripWaveguide`]'s analytic model)
///   along the arc: no radiation, no change of the mode by the bend.
/// - **2D:** the exact bent slab ([`SlabBend`], Marcuse 1971's radial equation solved to the
///   outgoing Hankel function) of the strip seen from above by the effective index method: its
///   radiation loss and the bend's shift of the mode, on a 2D model whose own error (the
///   method's, a few per cent in n_eff) is larger. Labelled 2D, and never the device's
///   performance.
/// - **3D:** the straight strip's mode from the full-vector solver (the waveguide's 3D model,
///   on its own mesh), plus the bend's shift: the strip's cross-section mapped conformally
///   (Heiblum & Harris 1975), with a perfectly matched layer beyond the radiation's turning
///   point, less the same window straight, both by the standard scheme on the same cells, so
///   that the error at the strip's corners, which the standard scheme leaves on any practical
///   grid, cancels.
///
/// The transitions to straight guides (their mode mismatch) are not included.
#[derive(Clone, Debug, PartialEq)]
pub struct StripBend {
    strip: Strip,
    straight: Mesh,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
}

impl Default for StripBend {
    fn default() -> StripBend {
        StripBend::new()
    }
}

impl StripBend {
    /// A 500 × 220 nm silicon strip bent at 10 µm on 220 nm SOI, its 3D solve on cells fitted
    /// about 20 nm apart.
    pub fn new() -> StripBend {
        let mut strip = Strip::soi();
        strip.mesh = default_mesh();
        StripBend::of(strip)
    }

    /// The bend drawn on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str) -> Result<StripBend> {
        let mut strip = Strip::on(stack, core)?;
        strip.mesh = default_mesh();
        Ok(StripBend::of(strip))
    }

    fn of(strip: Strip) -> StripBend {
        let mut parameters = strip_parameters();
        parameters.push(parameter("radius", "µm", 10.0, 1.0, 1e4));
        parameters.push(parameter("angle", "°", 90.0, 0.0, 360.0));
        parameters.push(parameter("loss", "dB/cm", 2.7, 0.0, 1e3));
        parameters.extend(band_parameters());
        StripBend {
            methods: methods(&strip),
            strip,
            straight: super::guide::default_mesh(),
            parameters,
        }
    }

    /// The same device, its bend's shift solved on `mesh` (fitted: [`super::Cells::Fitted`]).
    #[must_use]
    pub fn with_mesh(mut self, mesh: Mesh) -> StripBend {
        self.strip.mesh = mesh;
        self
    }

    /// Its meshes: the bend's shift's, and the straight strip's.
    pub(super) fn meshes(&self) -> (Mesh, Mesh) {
        (self.strip.mesh, self.straight)
    }

    /// The same device, its straight strip solved on `mesh` (by default the waveguide's: cells
    /// fitted about 10 nm apart, Hadley's equations).
    #[must_use]
    pub fn with_straight_mesh(mut self, mesh: Mesh) -> StripBend {
        self.straight = mesh;
        self
    }

    /// The same device, its analytic fidelity on `guide`, a straight strip's stated dispersion.
    #[must_use]
    pub fn with_guide(mut self, guide: Dispersion) -> StripBend {
        self.strip.stated = Some(guide);
        self.methods = methods(&self.strip);
        self
    }

    /// The exact bent slab of the 2D model at `values` and `wavelength`: the strip seen from
    /// above by the effective index method ([`Ridge::strip`], its TE-like mode's vertical slab
    /// as the core, the cladding beside it), bent at the radius.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's, and those of the
    /// effective index method and [`SlabBend::new`].
    pub fn bent_slab(&self, values: &Values, wavelength: Wavelength) -> Result<SlabBend> {
        values.check(self)?;
        let (w, t, r) = (
            values.at("width"),
            values.at("thickness"),
            values.at("radius"),
        );
        let (centre, side) = self.lateral(w, t, wavelength)?;
        SlabBend::new(um(r), side, &[(centre, um(w))], side, um(-w / 2.0))
    }

    /// The effective index method's lateral slab: its core (the strip's vertical slab) and
    /// cladding indices.
    fn lateral(&self, w: f64, t: f64, wavelength: Wavelength) -> Result<(f64, f64)> {
        let (below, core, above, beside) = self.strip.indices(t, wavelength)?;
        if beside != above {
            return Err(Error::invalid(
                "device",
                format!(
                    "the 2D model takes the strip's sides to be its cladding; this stack's \
                     core layer has {beside:.3} beside the strip under {above:.3}"
                ),
            ));
        }
        let eim =
            Ridge::strip(below, core, above, um(t), um(w))?.mode(Polarization::Te, wavelength)?;
        Ok((eim.centre, eim.side))
    }

    /// What the full-vector solver sees at `values` and `wavelength`: the bent strip's
    /// cross-section by the conformal map, with its PML.
    ///
    /// # Errors
    ///
    /// As [`StripBend::bent_slab`], and [`Error::InvalidValue`] for a window too tight for the
    /// radius or a mesh that isn't fitted.
    pub fn cross_section(&self, values: &Values, wavelength: Wavelength) -> Result<CrossSection> {
        values.check(self)?;
        let (w, t, r) = (
            values.at("width"),
            values.at("thickness"),
            values.at("radius"),
        );
        let (_, caustic) = self.guess(w, t, r, wavelength)?;
        self.strip.bent_section(r, w, t, caustic, wavelength, true)
    }

    /// The cross-section of [`StripBend::cross_section`] straight: the same window and cells
    /// without the conformal map, what the bend shifts the mode from on the same grid.
    pub(crate) fn straight_section(
        &self,
        values: &Values,
        wavelength: Wavelength,
    ) -> Result<CrossSection> {
        values.check(self)?;
        let (w, t, r) = (
            values.at("width"),
            values.at("thickness"),
            values.at("radius"),
        );
        let (_, caustic) = self.guess(w, t, r, wavelength)?;
        self.strip.bent_section(r, w, t, caustic, wavelength, false)
    }

    /// The bent mode's index to search near (the effective index method's), and where it
    /// starts to radiate: x = R(n/n_side − 1) from the centre line, the turning point of the
    /// mapped index profile n_side(1 + x/R).
    fn guess(&self, w: f64, t: f64, r: f64, wavelength: Wavelength) -> Result<(f64, f64)> {
        let (below, core, above, _) = self.strip.indices(t, wavelength)?;
        let eim =
            Ridge::strip(below, core, above, um(t), um(w))?.mode(Polarization::Te, wavelength)?;
        Ok((
            eim.effective_index,
            r * (eim.effective_index / eim.side - 1.0),
        ))
    }
}

/// The bend's default 3D mesh: cells fitted about 20 nm apart, 1 µm of cladding inside and
/// beyond the turning point and a 1 µm PML, on the half above the mid-plane, standard scheme
/// (Hadley's equations take no PML).
fn default_mesh() -> Mesh {
    Mesh::fitted(0.02, 1.0, super::Scheme::Standard)
}

/// The bend's models.
fn methods(strip: &Strip) -> Vec<Method> {
    let mut validity = if strip.stated.is_some() {
        Vec::new()
    } else {
        fit_validity()
    };
    validity.push(Range::new("radius", 5.0, 1e4));
    vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: "e^(iγRθ) on the straight strip's dispersion: no radiation, the mode unchanged \
                   by the bend"
                .into(),
            write_up: "devices".into(),
            papers: analytic_papers(),
            error: Some(Stated {
                quantity: "n_eff".into(),
                bound: 0.003,
                unit: String::new(),
                against: Fidelity::ThreeD,
            }),
            validity,
        },
        Method {
            fidelity: Fidelity::TwoD,
            name: "the exact bent slab of the strip seen from above by the effective index \
                   method (2D)"
                .into(),
            write_up: "devices".into(),
            papers: vec![
                "D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), \
                 doi:10.1002/j.1538-7305.1971.tb02620.x"
                    .into(),
                "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University \
                 Press (2015), doi:10.1017/CBO9781316084168, Section 3.2.5"
                    .into(),
            ],
            error: Some(Stated {
                quantity: "n_eff".into(),
                bound: 0.06,
                unit: String::new(),
                against: Fidelity::ThreeD,
            }),
            validity: vec![Range::new("radius", 1.0, 1e4)],
        },
        Method {
            fidelity: Fidelity::ThreeD,
            name: "the full-vector solver: the straight strip, plus the bend's shift by the \
                   conformal map with a PML beyond the turning point, less the same cells straight"
                .into(),
            write_up: "devices".into(),
            papers: {
                let mut p = solver_papers();
                p.push(
                    "M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975), \
                     doi:10.1109/JQE.1975.1068563"
                        .into(),
                );
                p
            },
            error: None,
            validity: Vec::new(),
        },
    ]
}

/// The figures of a bend: its mode along the arc, and its loss per quarter turn.
fn bend_figures(
    d: &Dispersion,
    radius: f64,
    radiation: Option<f64>,
    fidelity: Fidelity,
    grid: &str,
) -> Vec<Figure> {
    let quarter = |db_per_cm: f64| db_per_cm * 1e-4 * PI * radius / 2.0;
    let mut f = vec![
        figure(
            "effective index along the arc",
            "n_eff",
            d.effective_index,
            "",
            fidelity,
            grid,
        ),
        figure(
            "group index along the arc",
            "n_g",
            d.group_index,
            "",
            fidelity,
            grid,
        ),
        figure(
            "loss per 90°",
            "loss_90",
            quarter(d.loss_db_per_cm),
            "dB",
            fidelity,
            grid,
        ),
    ];
    if let Some(r) = radiation {
        f.push(figure(
            "radiation loss per 90°",
            "radiation_90",
            quarter(r),
            "dB",
            fidelity,
            grid,
        ));
    }
    f
}

impl Device for StripBend {
    fn kind(&self) -> &str {
        "bend"
    }

    fn about(&self) -> &str {
        "A circular bend of the strip: its mode's phase and loss along the arc, from the strip \
         and the radius."
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        self.strip.stack(values.at("thickness"))
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        let (w, r, angle) = (values.at("width"), values.at("radius"), values.at("angle"));
        // a path from (0, 0) heading +x, turning left about (0, R): its shape the annular
        // sector, and its end the second port
        let drawing = generator::Bend::arc(um(w), um(r), angle).draw(&self.strip.core)?;
        super::drawn(Structure::new(self.stack(values)?), &drawing)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (bend, radiation, grid) = self.build(values, fidelity)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let figures = bend_figures(bend.dispersion(), values.at("radius"), radiation, f, &grid);
        let derivatives = self
            .parameters
            .iter()
            .map(|p| (p.name == "angle").then(|| vec![1.0]))
            .collect();
        Ok(Built {
            values: vec![values.at("angle")],
            derivatives,
            figures,
            component: Box::new(bend),
        })
    }
}

impl StripBend {
    /// The bend at `fidelity` and `values`, as its component.
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    pub fn bend(&self, values: &Values, fidelity: Fidelity) -> Result<Bend> {
        Ok(self.build(values, fidelity)?.0)
    }

    /// The bend, its radiation loss (dB/cm) where the model has one, and the grid its figures
    /// name.
    fn build(&self, values: &Values, fidelity: Fidelity) -> Result<(Bend, Option<f64>, String)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let (w, t, r) = (
            values.at("width"),
            values.at("thickness"),
            values.at("radius"),
        );
        let (l0, span, loss) = (
            values.at("wavelength"),
            values.at("span"),
            values.at("loss"),
        );
        Ok(match m.fidelity {
            Fidelity::Analytic => {
                let (d, grid) = self.strip.analytic(w, t, l0, loss)?;
                (Bend::new(r, d)?, None, grid)
            }
            Fidelity::TwoD => {
                let at = Wavelength::um(l0)?;
                let (mut d, residual) = Dispersion::from_bent_slab(
                    |wl| self.bent_slab(values, wl),
                    Polarization::Tm,
                    at,
                    span / 4.0,
                )?;
                let radiation = d.loss_db_per_cm;
                d.loss_db_per_cm += loss;
                let grid =
                    "2D: an exact bent slab by the effective index method, no grid".to_owned();
                let provenance = Provenance {
                    fidelity: Fidelity::TwoD,
                    source: format!(
                        "2D: an exact bent slab of radius {r} um (Marcuse 1971, \
                         doi:10.1002/j.1538-7305.1971.tb02620.x, Eq. 10), the {w} x {t} um strip \
                         seen from above by the effective index method: n_eff {:.6}, n_g {:.4}, \
                         {:.3} dB/cm of radiation and {loss} dB/cm more at {l0} um; within \
                         {residual:.1e} of the exact modes at ±{} um",
                        d.effective_index,
                        d.group_index,
                        radiation,
                        span / 2.0
                    ),
                    error: None,
                    validity: Some((l0 - span / 2.0, l0 + span / 2.0)),
                };
                (
                    Bend::new(r, d)?.with_provenance(provenance),
                    Some(radiation),
                    grid,
                )
            }
            _ => {
                let at = Wavelength::um(l0)?;
                let (guess, caustic) = self.guess(w, t, r, at)?;
                let window = Strip::grid(&self.strip.bent_section(r, w, t, caustic, at, true)?);
                // the straight strip on its own mesh, and the bend's shift: the mapped strip
                // less the same window straight, on the same cells, so that the corners' error
                // cancels
                let mut straight = self.strip.clone();
                straight.mesh = self.straight;
                let solve = |mapped: bool| {
                    Dispersion::from_modes(
                        |wl| self.strip.bent_section(r, w, t, caustic, wl, mapped),
                        at,
                        span / 4.0,
                        Some(guess),
                    )
                };
                let (alone, (bent, flat)) = rayon::join(
                    || straight.solved(w, t, l0, span),
                    || rayon::join(|| solve(true), || solve(false)),
                );
                let ((mut d, _, grid), (bent, residual, _), (flat, _, _)) = (alone?, bent?, flat?);
                d.effective_index += bent.effective_index - flat.effective_index;
                d.group_index += bent.group_index - flat.group_index;
                d.dispersion += bent.dispersion - flat.dispersion;
                let radiation = bent.loss_db_per_cm - flat.loss_db_per_cm;
                d.loss_db_per_cm = radiation + loss;
                let grid = format!("{grid} straight, its bend's shift on {window}");
                let provenance = Provenance {
                    fidelity: Fidelity::ThreeD,
                    source: format!(
                        "the full-vector mode solver: the {w} x {t} um strip straight on {grid}, \
                         bent at {r} um by Heiblum & Harris's conformal map \
                         (doi:10.1109/JQE.1975.1068563) less the same window straight: n_eff \
                         {:.6}, n_g {:.4}, {:.3} dB/cm of radiation and {loss} dB/cm more at \
                         {l0} um; within {residual:.1e} of the solver at ±{} um",
                        d.effective_index,
                        d.group_index,
                        radiation,
                        span / 2.0
                    ),
                    error: None,
                    validity: Some((l0 - span / 2.0, l0 + span / 2.0)),
                };
                (
                    Bend::new(r, d)?.with_provenance(provenance),
                    Some(radiation),
                    grid,
                )
            }
        })
    }
}
