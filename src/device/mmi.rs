//! The multimode interference coupler, 1 × 2 and 2 × 2: a wide section whose modes beat into
//! images of what its access guides launch.

use crate::circuit::components::{Mmi, MmiKind, Planar};
use crate::circuit::{Component, Fidelity, Parameter, SMatrix};
use crate::generator::{self, Generator};
use crate::material::Material;
use crate::mode::Polarization;
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::guide::{Profile, Strip, um};
use super::solve::{self, FdfdGrid};
use super::waveguide::{band_parameters, etch_parameters, profile};
use super::{
    Built, Device, Figure, Geometry, Method, Range, Stated, Values, figure, method, parameter,
};

/// Which model a device's 2D fidelity is, where it has two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Model2d {
    /// The cheap one: an MMI's guided-mode propagation analysis on its plane.
    GuidedModes,
    /// 2D FDFD of the drawn geometry by the effective index method.
    Fdfd,
}

/// A multimode interference coupler on a strip's stack: a section `width` × `length`, its
/// access guides `access_width` wide where they meet it (centred at 0 and ±W/4 for the 1 × 2,
/// at ±W/6 for the 2 × 2), tapering over `taper_length` to `port_width` at the ports (no tapers
/// at 0: the ports are on the section's faces, `access_width` wide). Ports `o1` (and `o2`) on
/// the left, bottom to top, then the outputs on the right, top to bottom, as the circuit's
/// [`Mmi`] names them; drawn by [`generator::Mmi`].
///
/// Parameters `width`, `length`, `access_width`, `taper_length`, `port_width`, `thickness`
/// (the core's), `slab` and `sidewall` (how the core is cut beside the guides: a rib MMI on a
/// slab, sloped walls), `wavelength` (where the default length is the first two-fold image)
/// and `span`.
///
/// - **2D, guided modes** (the default), its stated error the 2D FDFD's of the same plane: Soldano & Pennings's guided-mode propagation analysis
///   ([`Mmi`]) on the plane seen from above by the effective index method, the section's and
///   the access guides' exact slab modes: the S-matrix between the access guides' fundamental
///   modes at the section's faces (the tapers taken as adiabatic and lossless, their phase
///   left out). On the soi_220 stack, a film of Li's silicon in Malitson's silica; on another
///   stack, or a rib, the slabs' indices at the design wavelength. Radiation and reflections
///   at the faces are left out. Labelled 2D, never the device's performance.
/// - **2D, FDFD** ([`StripMmi::with_model_2d`]): the drawn geometry by 2D FDFD on the effective
///   index method's plane ([`FdfdGrid`]), each cell averaged from the fill (the diagonal of
///   Farjadpour et al.'s tensor), its ports' fundamental modes.
/// - **3D:** the drawn geometry on its stack by 3D FDFD, TE-like.
#[derive(Clone, Debug, PartialEq)]
pub struct StripMmi {
    kind: MmiKind,
    strip: Strip,
    parameters: Vec<Parameter>,
    methods: Vec<Method>,
    model: Model2d,
    grid_2d: FdfdGrid,
    grid_3d: FdfdGrid,
}

/// The defaults: the studio's MMIs', 6 µm wide with 1.5 µm access guides on 220 nm SOI.
const WIDTH: f64 = 6.0;
const ACCESS: f64 = 1.5;
const WAVELENGTH: f64 = 1.55;

/// The plane the circuit's MMI sees for a strip MMI of `thickness` on `strip`'s stack, cut by
/// `profile`, at `wavelength`: the film of the core between the medium around it (as the
/// studio's MMIs have always been built) when the stack is the same below, above and beside the
/// core and the core is etched through; otherwise the slabs' indices at `wavelength`.
pub(super) fn planar(
    strip: &Strip,
    thickness: f64,
    profile: Profile,
    wavelength: Wavelength,
) -> Result<Planar> {
    if profile.sidewall != 90.0 {
        return Err(Error::invalid(
            "device",
            "the guided modes take vertical walls: a sloped wall is the FDFD's (with_model_2d) or \
             3D",
        ));
    }
    let stack = strip.stack(thickness)?;
    let layers = stack.layers();
    let k = layers
        .iter()
        .position(|l| l.name == strip.core)
        .ok_or_else(|| {
            Error::invalid("device", format!("the stack has no layer {}", strip.core))
        })?;
    let below: &Material = if k == 0 {
        stack.substrate()
    } else {
        &layers[k - 1].material
    };
    let above = layers
        .get(k + 1)
        .map_or(stack.cladding(), |l| &l.background);
    let (core, beside) = (&layers[k].material, &layers[k].background);
    if profile.slab == 0.0 && below == beside && above == beside {
        return Ok(Planar::Film {
            core: core.clone(),
            cladding: beside.clone(),
            thickness: Length::um(thickness),
            vertical: Polarization::Te,
        });
    }
    let (nb, nc, na, ns) = strip.indices(thickness, wavelength)?;
    if ns != na {
        return Err(Error::invalid(
            "device",
            format!(
                "the guided modes here take the medium beside the guides ({ns}) to be the one \
                 above the core ({na}): solve by FDFD"
            ),
        ));
    }
    // the vertical slabs' indices: the ridge's and its sides'
    let (ridge, side) =
        super::rib::slab_indices((nb, nc, na), thickness, profile.slab, wavelength)?;
    Ok(Planar::Indices {
        ridge,
        cladding: side,
        polarization: Polarization::Tm,
    })
}

impl StripMmi {
    /// A 1 × 2 MMI on 220 nm SOI, 6 µm wide with 1.5 µm access guides, its length the first
    /// two-fold image at 1.55 µm (3L_π/8): the studio's.
    pub fn one_by_two() -> StripMmi {
        StripMmi::of(MmiKind::OneByTwo, Strip::soi())
    }

    /// A 2 × 2 MMI on 220 nm SOI, 6 µm wide with 1.5 µm access guides at ±W/6, its length the
    /// first two-fold image at 1.55 µm (L_π/2): the studio's.
    pub fn two_by_two() -> StripMmi {
        StripMmi::of(MmiKind::TwoByTwo, Strip::soi())
    }

    /// The MMI of `kind` drawn on the layer `core` of `stack`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn on(kind: MmiKind, stack: LayerStack, core: &str) -> Result<StripMmi> {
        Ok(StripMmi::of(kind, Strip::on(stack, core)?))
    }

    fn of(kind: MmiKind, strip: Strip) -> StripMmi {
        let at = Wavelength::from_um_unchecked(WAVELENGTH);
        let length = planar(&strip, 0.22, Profile::STRIP, at)
            .and_then(|p| Mmi::new(kind, p, WIDTH, ACCESS, at))
            .map_or(10.0, |m| m.defaults()[0]);
        let mut parameters = vec![
            parameter("width", "µm", WIDTH, 0.5, 100.0),
            parameter("length", "µm", length, 0.1, 1e4),
            parameter("access_width", "µm", ACCESS, 0.1, 20.0),
            parameter("taper_length", "µm", 0.0, 0.0, 200.0),
            parameter("port_width", "µm", ACCESS, 0.1, 20.0),
            parameter("thickness", "µm", 0.22, 0.05, 5.0),
        ];
        parameters.extend(etch_parameters());
        parameters.extend(band_parameters());
        StripMmi {
            kind,
            methods: methods(Model2d::GuidedModes),
            strip,
            parameters,
            model: Model2d::GuidedModes,
            grid_2d: FdfdGrid::new(0.02).with_averaging(crate::averaging::Averaging::Diagonal),
            grid_3d: FdfdGrid::new(0.04).with_step_z(0.02).with_margin(0.6),
        }
    }

    /// The same device, its 2D fidelity `model`.
    #[must_use]
    pub fn with_model_2d(mut self, model: Model2d) -> StripMmi {
        self.model = model;
        self.methods = methods(model);
        self
    }

    /// The same device, its 2D FDFD on `grid`.
    #[must_use]
    pub fn with_grid_2d(mut self, grid: FdfdGrid) -> StripMmi {
        self.grid_2d = grid;
        self
    }

    /// The same device, its 3D FDFD on `grid`.
    #[must_use]
    pub fn with_grid_3d(mut self, grid: FdfdGrid) -> StripMmi {
        self.grid_3d = grid;
        self
    }

    /// Its FDFD grids: the 2D one's and the 3D one's.
    pub(super) fn grids(&self) -> (FdfdGrid, FdfdGrid) {
        (self.grid_2d, self.grid_3d)
    }

    /// The size of the FDFD solve `fidelity` takes at `values`, when it is one: the cells along
    /// x and y, and the unknowns of a solve.
    ///
    /// # Errors
    ///
    /// As [`Device::geometry`], and [`Error::InvalidValue`] for a fidelity it doesn't have.
    pub(super) fn fdfd_size(
        &self,
        values: &Values,
        fidelity: Fidelity,
    ) -> Result<Option<(usize, usize, f64)>> {
        let m = method(self.kind(), &self.methods, fidelity)?;
        if m.fidelity == Fidelity::TwoD && self.model == Model2d::GuidedModes {
            return Ok(None);
        }
        let three = m.fidelity == Fidelity::ThreeD;
        let grid = if three { self.grid_3d } else { self.grid_2d };
        solve::size(&self.geometry(values)?, &self.strip.core, &grid, three).map(Some)
    }

    /// Its kind of MMI.
    pub fn mmi_kind(&self) -> MmiKind {
        self.kind
    }

    /// The circuit's guided-mode MMI at `values` (2D), its own values `[length, width]`.
    ///
    /// # Errors
    ///
    /// As [`Device::component`] at 2D with guided modes.
    pub fn guided(&self, values: &Values) -> Result<(Mmi, Vec<f64>)> {
        values.check(self)?;
        let at = Wavelength::um(values.at("wavelength"))?;
        let p = planar(&self.strip, values.at("thickness"), profile(values), at)?;
        let mmi = Mmi::new(
            self.kind,
            p,
            values.at("width"),
            values.at("access_width"),
            at,
        )?;
        Ok((mmi, vec![values.at("length"), values.at("width")]))
    }

    /// The generator that draws it at `values`.
    fn generator(&self, values: &Values) -> generator::Mmi {
        let (w, l, wa) = (
            values.at("width"),
            values.at("length"),
            values.at("access_width"),
        );
        let (lt, wp) = (values.at("taper_length"), values.at("port_width"));
        let (inputs, outputs) = match self.kind {
            MmiKind::OneByTwo => (1, 2),
            MmiKind::TwoByTwo => (2, 2),
        };
        let mut g = if lt > 0.0 {
            generator::Mmi::new(um(w), um(l), inputs, outputs, um(wp)).with_tapers(um(wa), um(lt))
        } else {
            generator::Mmi::new(um(w), um(l), inputs, outputs, um(wa))
        };
        if self.kind == MmiKind::TwoByTwo {
            let sixth = vec![um(-w / 6.0), um(w / 6.0)];
            g.input_positions = Some(sixth.clone());
            g.output_positions = Some(sixth);
        }
        g
    }

    /// The component at `fidelity` and `values`, its own values, and the grid its figures name.
    fn build(
        &self,
        values: &Values,
        fidelity: Fidelity,
    ) -> Result<(Box<dyn Component>, Vec<f64>, String)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        if m.fidelity == Fidelity::TwoD && self.model == Model2d::GuidedModes {
            let (mmi, v) = self.guided(values)?;
            return Ok((Box::new(mmi), v, GUIDED_GRID.into()));
        }
        let three = m.fidelity == Fidelity::ThreeD;
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
        Ok((Box::new(sampled), Vec::new(), description))
    }
}

/// The grid the guided modes' figures name.
const GUIDED_GRID: &str =
    "2D, guided modes by the effective index method: exact slab modes, no grid";

/// Soldano & Pennings, the guided-mode analysis.
pub(super) fn soldano() -> String {
    "L. B. Soldano, E. C. M. Pennings, J. Lightwave Technol. 13, 615 (1995), doi:10.1109/50.372474"
        .into()
}

/// The papers of the FDFD fidelities.
pub(super) fn fdfd_papers(three: bool) -> Vec<String> {
    let mut p = vec![
        "R. C. Rumpf, Prog. Electromagn. Res. B 36, 221 (2012), doi:10.2528/PIERB11092006".into(),
    ];
    if !three {
        p.push(
            "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), doi:10.1364/AO.16.000113".into(),
        );
    }
    p
}

/// The MMIs' models, the 2D one `model`.
fn methods(model: Model2d) -> Vec<Method> {
    let two = match model {
        Model2d::GuidedModes => Method {
            fidelity: Fidelity::TwoD,
            name: "guided-mode propagation analysis on the plane seen from above by the effective \
                   index method, the exact slab modes: labelled 2D"
                .into(),
            write_up: "devices".into(),
            papers: vec![
                soldano(),
                "G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977), \
                 doi:10.1364/AO.16.000113"
                    .into(),
            ],
            error: Some(Stated {
                quantity: "|S|²".into(),
                bound: GUIDED_ERROR,
                unit: String::new(),
                against: Fidelity::TwoD,
            }),
            validity: vec![
                Range::new("width", 3.0, 6.0),
                Range::new("access_width", 0.5, 1.5),
                Range::new("slab", 0.0, 0.0),
                Range::new("sidewall", 90.0, 90.0),
                Range::new("wavelength", 1.5, 1.6),
            ],
        },
        Model2d::Fdfd => Method {
            fidelity: Fidelity::TwoD,
            name: "2D FDFD of the drawn geometry on the effective index method's plane, the \
                   ports' fundamental modes: labelled 2D"
                .into(),
            write_up: "devices".into(),
            papers: fdfd_papers(false),
            error: None,
            validity: vec![Range::new("wavelength", 1.5, 1.6)],
        },
    };
    vec![
        two,
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

/// The guided modes' largest |Δ|S_qp|²| against 2D FDFD of the same plane, the next model up
/// (`device/mmi-guided-2d`: 1.4e-3 on a 5 nm grid, whose own error is about 1e-3). Against 3D,
/// 0.05 on the 2 µm MMI at its image (`device/mmi-guided-3d`, docs/methods/devices.md).
pub(super) const GUIDED_ERROR: f64 = 5e-3;

/// An MMI's or a splitter's figures from its S-matrix at the design wavelength: the powers out
/// of input `o1`, the excess loss and the imbalance between the two outputs, `outputs` (their
/// indices).
pub(super) fn split_figures(
    s: &SMatrix,
    outputs: [usize; 2],
    names: [&str; 2],
    f: Fidelity,
    grid: &str,
) -> Vec<Figure> {
    let (a, b) = (s.power(outputs[0], 0), s.power(outputs[1], 0));
    vec![
        figure(
            &format!("power into {} from o1", names[0]),
            &format!("P_{}", names[0]),
            a,
            "",
            f,
            grid,
        ),
        figure(
            &format!("power into {} from o1", names[1]),
            &format!("P_{}", names[1]),
            b,
            "",
            f,
            grid,
        ),
        figure(
            "excess loss",
            "excess_loss",
            -10.0 * (a + b).log10(),
            "dB",
            f,
            grid,
        ),
        figure(
            "imbalance between the outputs",
            "imbalance",
            10.0 * (a / b).log10(),
            "dB",
            f,
            grid,
        ),
    ]
}

impl Device for StripMmi {
    fn kind(&self) -> &str {
        match self.kind {
            MmiKind::OneByTwo => "MMI 1x2",
            MmiKind::TwoByTwo => "MMI 2x2",
        }
    }

    fn about(&self) -> &str {
        match self.kind {
            MmiKind::OneByTwo => {
                "A 1 × 2 multimode interference splitter: a wide section fed in its middle, whose \
                 modes beat into two images of the input at 3L_π/8."
            }
            MmiKind::TwoByTwo => {
                "A 2 × 2 multimode interference coupler: a wide section fed at ±W/6, whose modes \
                 beat into two images of each input at L_π/2, a 3 dB coupler."
            }
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
        values.check(self)?;
        let drawing = self.generator(values).draw(&self.strip.core)?;
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
        let (component, own, grid) = self.build(values, fidelity)?;
        let f = method(self.kind(), &self.methods, fidelity)?.fidelity;
        let at = Wavelength::um(values.at("wavelength"))?;
        let s = component.s_matrix(at, &own)?;
        let mut figures = match self.kind {
            MmiKind::OneByTwo => split_figures(&s, [1, 2], ["o2", "o3"], f, &grid),
            // from o1 (lower left): o3 crosses (upper right), o4 is the bar (lower right)
            MmiKind::TwoByTwo => split_figures(&s, [2, 3], ["o3", "o4"], f, &grid),
        };
        if f == Fidelity::TwoD && self.model == Model2d::GuidedModes {
            let (mmi, _) = self.guided(values)?;
            let beat = mmi.beat_length(at, values.at("width"))?;
            let image = match self.kind {
                MmiKind::OneByTwo => 3.0 * beat / 8.0,
                MmiKind::TwoByTwo => beat / 2.0,
            };
            figures.push(figure("beat length", "L_pi", beat, "µm", f, &grid));
            figures.push(figure(
                "first two-fold image's length",
                "L_image",
                image,
                "µm",
                f,
                &grid,
            ));
        }
        let derivatives = self
            .parameters
            .iter()
            .map(|p| {
                if own.is_empty() {
                    return None;
                }
                match p.name.as_str() {
                    "length" => Some(vec![1.0, 0.0]),
                    "width" => Some(vec![0.0, 1.0]),
                    _ => None,
                }
            })
            .collect();
        Ok(Built {
            values: own,
            derivatives,
            figures,
            component,
        })
    }
}
