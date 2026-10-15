//! The Y-branch: a stem that widens along an outline of its own into two arms.

use std::sync::Mutex;

use crate::circuit::components::YBranch;
use crate::circuit::{Component, Fidelity, Parameter};
use crate::expr::{Expr, Parameters};
use crate::generator::{self, Drawing, Generator, SBendKind};
use crate::stack::{LayerStack, Structure};
use crate::units::Wavelength;
use crate::{Error, Result};

use super::guide::{Strip, um};
use super::mmi::{fdfd_papers, split_figures};
use super::solve::{self, FdfdGrid};
use super::waveguide::{band_parameters, etch_parameters, profile};
use super::{Built, Device, Geometry, Method, Values, method, parameter};

/// The outline's widths at its 13 points: Y. Zhang et al., Opt. Express 21, 1310 (2013),
/// doi:10.1364/OE.21.001310, Table 1, from the 0.5 µm input to the two 0.5 µm arms 0.2 µm apart.
pub(super) const ZHANG: [f64; 13] = [
    0.5, 0.5, 0.6, 0.7, 0.9, 1.26, 1.4, 1.4, 1.4, 1.4, 1.31, 1.2, 1.2,
];

/// How many widths the outline is given by.
pub(super) const POINTS: usize = ZHANG.len();

/// A Y-branch on a strip's stack ([`generator::YBranch`]): a stem `length` long whose width is
/// the not-a-knot cubic spline through `w1` … `w13` at 13 evenly spaced points (the expression
/// language's `spline`, drawn between −w(x)/2 and w(x)/2), then two sine S-bend arms `width`
/// wide, starting `w13 − 2 width` apart and reaching `separation` (centre to centre) after
/// `arm_length`. Ports `o1` (the stem, `w1` wide), `o2` (the upper arm) and `o3` (the lower),
/// as the circuit's [`YBranch`] names them.
///
/// Parameters `width`, `length`, `w1` … `w13`, `arm_length`, `separation`, `thickness`, `slab`
/// and `sidewall` (the core's cut, as the waveguide's), `excess_loss` (dB, the ideal model's),
/// `wavelength` and `span`. The defaults are Zhang et al.'s junction (2013, Table 1: 2 µm long,
/// 1.2 µm at its end, its widths a spline), with arms 5 µm long to ports 2 µm apart.
///
/// - **Analytic:** ideal, the stem's power split equally with the excess loss ([`YBranch`]).
/// - **2D:** the drawn geometry by 2D FDFD on the effective index method's plane
///   ([`FdfdGrid`]), its ports' fundamental modes. Labelled 2D.
/// - **3D:** the drawn geometry on its stack by 3D FDFD, TE-like.
#[derive(Clone, Debug, PartialEq)]
pub struct StripYBranch {
    strip: Strip,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
    grid_2d: FdfdGrid,
    grid_3d: FdfdGrid,
}

impl Default for StripYBranch {
    fn default() -> StripYBranch {
        StripYBranch::new()
    }
}

impl StripYBranch {
    /// Zhang et al.'s Y-junction on 220 nm SOI, with arms 5 µm long to ports 2 µm apart.
    pub fn new() -> StripYBranch {
        StripYBranch::of(Strip::soi())
    }

    /// The Y-branch drawn on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn on(stack: LayerStack, core: &str) -> Result<StripYBranch> {
        Ok(StripYBranch::of(Strip::on(stack, core)?))
    }

    fn of(strip: Strip) -> StripYBranch {
        let mut parameters = vec![
            parameter("width", "µm", 0.5, 0.1, 5.0),
            parameter("length", "µm", 2.0, 0.1, 100.0),
        ];
        for (k, &w) in ZHANG.iter().enumerate() {
            parameter_into(&mut parameters, &format!("w{}", k + 1), w);
        }
        parameters.extend([
            parameter("arm_length", "µm", 5.0, 0.1, 1000.0),
            parameter("separation", "µm", 2.0, 0.1, 1000.0),
            parameter("thickness", "µm", 0.22, 0.05, 5.0),
        ]);
        parameters.extend(etch_parameters());
        parameters.push(parameter("excess_loss", "dB", 0.0, 0.0, 10.0));
        parameters.extend(band_parameters());
        StripYBranch {
            methods: methods(),
            strip,
            parameters,
            grid_2d: FdfdGrid::new(0.02),
            grid_3d: FdfdGrid::new(0.04).with_step_z(0.02).with_margin(0.6),
        }
    }

    /// The same device, its 2D FDFD on `grid`.
    #[must_use]
    pub fn with_grid_2d(mut self, grid: FdfdGrid) -> StripYBranch {
        self.grid_2d = grid;
        self
    }

    /// The same device, its 3D FDFD on `grid`.
    #[must_use]
    pub fn with_grid_3d(mut self, grid: FdfdGrid) -> StripYBranch {
        self.grid_3d = grid;
        self
    }

    /// The outline's widths at `values`, w1 … w13.
    fn widths(values: &Values) -> Vec<f64> {
        (1..=POINTS).map(|k| values.at(&format!("w{k}"))).collect()
    }

    /// The outline at `values`: w(x) as an expression, `spline(x, [knots], [w1, …, w13])`, and
    /// the parameters it names.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the expression can't be built (never for valid values).
    pub fn outline(&self, values: &Values) -> Result<(Expr, Parameters)> {
        let length = values.at("length");
        let knots: Vec<String> = (0..POINTS)
            .map(|k| format!("{:?} um", length * k as f64 / (POINTS - 1) as f64))
            .collect();
        let names: Vec<String> = (1..=POINTS).map(|k| format!("w{k}")).collect();
        let definitions: Vec<(String, String)> = names
            .iter()
            .zip(StripYBranch::widths(values))
            .map(|(n, w)| (n.clone(), format!("{w:?} um")))
            .collect();
        let parameters =
            Parameters::new(&definitions).map_err(|e| Error::invalid("device", e.to_string()))?;
        let text = format!("spline(x, [{}], [{}])", knots.join(", "), names.join(", "));
        let e = Expr::parse(&text, parameters.scope())
            .map_err(|e| Error::invalid("device", e.to_string()))?;
        Ok((e, parameters))
    }

    /// The generator that draws it at `values`.
    pub(super) fn generator(&self, values: &Values) -> Result<generator::YBranch> {
        let (w, l) = (values.at("width"), values.at("length"));
        let last = values.at(&format!("w{POINTS}"));
        let gap = last - 2.0 * w;
        if gap < 0.0 {
            return Err(Error::invalid(
                "device",
                format!(
                    "a Y-branch's last width, w{POINTS} = {last} um, must be at least its two \
                     arms' width, {} um",
                    2.0 * w
                ),
            ));
        }
        let (outline, parameters) = self.outline(values)?;
        let mut g = generator::YBranch::new(
            um(values.at("w1")),
            um(w),
            um(l),
            um(values.at("arm_length")),
            um(values.at("separation")),
        )
        .with_outline(outline, parameters);
        g.gap = um(gap);
        g.arm_kind = SBendKind::Sine;
        Ok(g)
    }
}

/// The Y-branches drawn lately, by their generator's parameters and layer: tracing the outline
/// (a region between two curves) and the arms takes milliseconds, and a page draws the same
/// design twice an update (its geometry, then its job's scene). The drawing is the generator's
/// own, so the same bits either way.
static DRAWN: Mutex<Vec<(String, Drawing)>> = Mutex::new(Vec::new());

/// How many drawings [`DRAWN`] keeps.
const KEPT: usize = 8;

impl StripYBranch {
    /// Its generator's drawing at `values` on `layer`, drawn or kept.
    ///
    /// # Errors
    ///
    /// As [`Generator::draw`], and [`StripYBranch::generator`]'s.
    pub(super) fn drawing(&self, values: &Values, layer: &str) -> Result<Drawing> {
        let g = self.generator(values)?;
        let key = format!("{layer}|{g:?}");
        let lock = || {
            DRAWN
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        };
        if let Some((_, d)) = lock().iter().find(|(k, _)| *k == key) {
            return Ok(d.clone());
        }
        let d = g.draw(layer)?;
        let mut kept = lock();
        kept.retain(|(k, _)| *k != key);
        kept.push((key, d.clone()));
        if kept.len() > KEPT {
            kept.remove(0);
        }
        Ok(d)
    }
}

/// Pushes a width parameter `name` with default `w`.
fn parameter_into(parameters: &mut Vec<Parameter>, name: &str, w: f64) {
    parameters.push(parameter(name, "µm", w, 0.05, 10.0));
}

/// The ideal model's largest |Δ|S_qp|²| against the 2D model, on Zhang et al.'s junction
/// (`device/y-branch-ideal-2d`: 0.032 at 10 nm, its arms 3 µm long).
pub(super) const IDEAL_ERROR: f64 = 0.05;

/// The Y-branch's models.
fn methods() -> Vec<Method> {
    vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: "an ideal splitter: the stem's power split equally between the arms, with an \
                   excess loss"
                .into(),
            write_up: "devices".into(),
            papers: Vec::new(),
            error: Some(super::Stated {
                quantity: "|S|²".into(),
                bound: IDEAL_ERROR,
                unit: String::new(),
                against: Fidelity::TwoD,
            }),
            validity: Vec::new(),
        },
        Method {
            fidelity: Fidelity::TwoD,
            name: "2D FDFD of the drawn geometry on the effective index method's plane, the \
                   ports' fundamental modes: labelled 2D"
                .into(),
            write_up: "devices".into(),
            papers: fdfd_papers(false),
            error: None,
            validity: Vec::new(),
        },
        Method {
            fidelity: Fidelity::ThreeD,
            name:
                "3D FDFD of the drawn geometry on its stack, the ports' fundamental TE-like modes"
                    .into(),
            write_up: "devices".into(),
            papers: fdfd_papers(true),
            error: None,
            validity: Vec::new(),
        },
    ]
}

impl Device for StripYBranch {
    fn kind(&self) -> &str {
        "Y-branch"
    }

    fn about(&self) -> &str {
        "A Y-branch: a stem that widens along an outline of its own (a spline through its \
         widths) and divides into two arms, splitting its power between them."
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        self.strip.stack(values.at("thickness"))
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        values.check(self)?;
        let drawing = self.drawing(values, &self.strip.core)?;
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
        values.check(self)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let (component, own, grid): (Box<dyn Component>, Vec<f64>, String) =
            if f == Fidelity::Analytic {
                (
                    Box::new(YBranch::new()),
                    vec![values.at("excess_loss")],
                    super::EXACT.into(),
                )
            } else {
                let three = f == Fidelity::ThreeD;
                let grid = if three { self.grid_3d } else { self.grid_2d };
                let wavelengths = grid.wavelengths(values.at("wavelength"), values.at("span"))?;
                let geometry = self.geometry(values)?;
                let (sampled, description) = solve::sampled(
                    self.kind(),
                    &geometry,
                    &self.strip.core,
                    &wavelengths,
                    &grid,
                    three,
                )?;
                (Box::new(sampled), Vec::new(), description)
            };
        let at = Wavelength::um(values.at("wavelength"))?;
        let s = component.s_matrix(at, &own)?;
        let figures = split_figures(&s, [1, 2], ["o2", "o3"], f, &grid);
        let derivatives = self
            .parameters
            .iter()
            .map(|p| (f == Fidelity::Analytic && p.name == "excess_loss").then(|| vec![1.0]))
            .collect();
        Ok(Built {
            values: own,
            derivatives,
            figures,
            component,
        })
    }
}
