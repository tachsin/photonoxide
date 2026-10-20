//! Layer stacks: the vertical structure of a chip, and what is drawn on its layers.
//!
//! A [`LayerStack`] is a substrate, layers from bottom to top, and a cladding above. Each layer
//! has two materials: its `material` where the layout draws shapes, and its `background`
//! everywhere else (where the layer was etched away and refilled, usually by the cladding).
//! A [`Structure`] is a stack plus the shapes drawn on its layers, and answers which material
//! fills any point (x, y, z), the question a solver's grid asks.
//!
//! z = 0 is the bottom of the first layer; the substrate is below it, the cladding above the
//! last layer.
//!
//! **Etches and sidewalls.** A shape is drawn with an [`Etch`] ([`Structure::draw_etched`]):
//! how deep the etch that cut its edges went from the layer's top, and the angle its walls make
//! with the layer's plane. [`Structure::draw`] draws with [`Etch::FULL`], through the whole
//! layer with vertical walls, as every structure always has. A partial etch leaves the layer's
//! material below its floor, everywhere: a rib on its slab. Sloped walls lean out going down,
//! so a strip's cross-section is a trapezoid, its drawn outline its top face: at depth δ below
//! the layer's top a shape reaches out to the points within δ cot θ of its outline (θ the
//! sidewall's angle), exactly, through the shape's signed distance. A point of a layer is its
//! material when any of its shapes holds it so, or lies below a partial etch's floor.
//! Several etches on one layer are several shapes' etches; a slab of limited extent is a layer
//! of its own.
//!
//! **What the solvers read.** Every solver reads the same structure: [`Structure::material_at`]
//! at its points (FDTD and FDFD in 3D, the pictures), [`Structure::cut_cell`] for the cells of a
//! mode solver's cross-section, which a sloped wall crosses with the exact share of each
//! material and its normal (the cell then takes their [`mean`]), and [`Structure::kept`], the
//! height of a layer's material over a point, for the effective index method's columns. FDTD's
//! subpixel smoothing has the same walls as bodies
//! ([`crate::fdtd::Body::etched`]). docs/methods/stacks.md has the method and its
//! validation.
//!
//! **Bodies over the stack.** [`Structure::place`] puts a body in space
//! ([`crate::geometry::Body`]: a sphere, a pillar, a membrane's holes, any shape extruded
//! between two heights) over the stack, its shapes and the bodies placed before it, filled with
//! a [`Medium`]: a catalogue material, or a graded one ([`crate::material::Graded`]).
//! [`Structure::permittivity_at`] answers for every point, a graded medium's value included;
//! [`Structure::material_at`] gives a catalogue body's material. docs/methods/bodies.md.

use std::sync::OnceLock;

use crate::geometry::{Body, Index, Point, Region, Shape};
use crate::material::{self, Graded, Material};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// A layer: its thickness, the material of what is drawn, and the material elsewhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its thickness.
    pub thickness: Length,
    /// The material where the layout draws shapes on it.
    pub material: Material,
    /// The material everywhere else in the layer.
    pub background: Material,
}

/// A substrate, layers from bottom to top, and a cladding. Build it with [`LayerStack::new`]
/// or a preset such as [`LayerStack::soi`].
#[derive(Clone, Debug, PartialEq)]
pub struct LayerStack {
    substrate: Material,
    layers: Vec<Layer>,
    cladding: Material,
}

impl LayerStack {
    /// A stack of `layers`, bottom to top, between a substrate and a cladding.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with no layers, a thickness that isn't positive and finite, or
    /// two layers with the same name.
    pub fn new(substrate: Material, layers: Vec<Layer>, cladding: Material) -> Result<LayerStack> {
        if layers.is_empty() {
            return Err(Error::invalid("layer stack", "needs at least one layer"));
        }
        for (i, layer) in layers.iter().enumerate() {
            let t = layer.thickness.to_um();
            if !t.is_finite() || t <= 0.0 {
                return Err(Error::invalid(
                    "layer stack",
                    format!("layer {} has thickness {}", layer.name, layer.thickness),
                ));
            }
            if layers[..i].iter().any(|l| l.name == layer.name) {
                return Err(Error::invalid(
                    "layer stack",
                    format!("two layers are named {}", layer.name),
                ));
            }
        }
        Ok(LayerStack {
            substrate,
            layers,
            cladding,
        })
    }

    /// Silicon-on-insulator: a silicon substrate, `buried_oxide` of silica, a silicon device
    /// layer `device` thick (named `"Si"`), etched regions refilled with silica, and a silica
    /// cladding.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both thicknesses are positive and finite.
    pub fn soi(device: Length, buried_oxide: Length) -> Result<LayerStack> {
        LayerStack::new(
            material::silicon(),
            vec![
                Layer {
                    name: "BOX".into(),
                    thickness: buried_oxide,
                    material: material::silica(),
                    background: material::silica(),
                },
                Layer {
                    name: "Si".into(),
                    thickness: device,
                    material: material::silicon(),
                    background: material::silica(),
                },
            ],
            material::silica(),
        )
    }

    /// The standard silicon photonics platform: 220 nm of silicon on 2 µm of buried oxide,
    /// with an oxide cladding. L. Chrostowski, M. Hochberg, *Silicon Photonics Design*,
    /// Cambridge University Press (2015),
    /// [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168): "The typical
    /// 200 mm (8") wafer consists of a 725 µm silicon substrate, 2 µm of oxide (buried oxide,
    /// or BOX), and 220 nm of crystalline silicon" (chapter 3, p. 49, Fig. 3.1); a 2 µm BOX
    /// couples well at both 1310 and 1550 nm and "is a common standard among silicon photonics
    /// foundries" (chapter 5). The substrate is modelled as infinitely thick. A PDK sets its
    /// own thicknesses.
    pub fn soi_220() -> LayerStack {
        LayerStack {
            substrate: material::silicon(),
            layers: vec![
                Layer {
                    name: "BOX".into(),
                    thickness: Length::um(2.0),
                    material: material::silica(),
                    background: material::silica(),
                },
                Layer {
                    name: "Si".into(),
                    thickness: Length::nm(220.0),
                    material: material::silicon(),
                    background: material::silica(),
                },
            ],
            cladding: material::silica(),
        }
    }

    /// Silicon nitride on oxide: a silicon substrate, `bottom_oxide` of silica, a nitride
    /// layer `core` thick (named `"SiN"`), etched regions refilled with silica, and a silica
    /// cladding.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both thicknesses are positive and finite.
    pub fn nitride(core: Length, bottom_oxide: Length) -> Result<LayerStack> {
        LayerStack::new(
            material::silicon(),
            vec![
                Layer {
                    name: "BOX".into(),
                    thickness: bottom_oxide,
                    material: material::silica(),
                    background: material::silica(),
                },
                Layer {
                    name: "SiN".into(),
                    thickness: core,
                    material: material::silicon_nitride(),
                    background: material::silica(),
                },
            ],
            material::silica(),
        )
    }

    /// The layers, bottom to top.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The substrate, below the first layer.
    pub fn substrate(&self) -> &Material {
        &self.substrate
    }

    /// The cladding, above the last layer.
    pub fn cladding(&self) -> &Material {
        &self.cladding
    }

    /// The layer named `name`, with its bottom and top z.
    pub fn layer(&self, name: &str) -> Option<(&Layer, Length, Length)> {
        let mut z = Length::ZERO;
        for layer in &self.layers {
            let top = z + layer.thickness;
            if layer.name == name {
                return Some((layer, z, top));
            }
            z = top;
        }
        None
    }

    /// The total thickness of the layers.
    pub fn thickness(&self) -> Length {
        self.layers
            .iter()
            .fold(Length::ZERO, |sum, l| sum + l.thickness)
    }

    /// Where z falls: below the stack, in a layer (a layer's bottom belongs to it, its top to
    /// the one above), or above the stack.
    fn locate(&self, z: Length) -> Place {
        if z < Length::ZERO {
            return Place::Below;
        }
        let mut top = Length::ZERO;
        for (i, layer) in self.layers.iter().enumerate() {
            top += layer.thickness;
            if z < top {
                return Place::In(i);
            }
        }
        Place::Above
    }
}

/// Where a height falls in a stack.
enum Place {
    Below,
    In(usize),
    Above,
}

/// How the edges of a shape on a layer were etched: how deep from the layer's top, and the
/// angle the walls make with the layer's plane.
///
/// Build one with [`Etch::FULL`], [`Etch::partial`] and [`Etch::with_sidewall`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Etch {
    /// How deep the etch went from the layer's top: `None` through the whole layer (a strip),
    /// or a depth less than the layer's thickness (a rib: the layer's material is left below
    /// it, everywhere).
    pub depth: Option<Length>,
    /// The walls' angle from the layer's plane, radians, above 0 and at most π/2: π/2 for
    /// vertical walls, less for walls that lean out going down, so that a strip's
    /// cross-section is a trapezoid wider at its foot. The shape's outline is its top face.
    pub sidewall: f64,
}

impl Default for Etch {
    fn default() -> Etch {
        Etch::FULL
    }
}

impl Etch {
    /// Through the whole layer, with vertical walls: what [`Structure::draw`] draws.
    pub const FULL: Etch = Etch {
        depth: None,
        sidewall: std::f64::consts::FRAC_PI_2,
    };

    /// An etch `depth` deep from the layer's top, with vertical walls.
    pub fn partial(depth: Length) -> Etch {
        Etch {
            depth: Some(depth),
            ..Etch::FULL
        }
    }

    /// The same etch with walls at `radians` from the layer's plane.
    #[must_use]
    pub fn with_sidewall(self, radians: f64) -> Etch {
        Etch {
            sidewall: radians,
            ..self
        }
    }

    /// The same etch with walls at `degrees` from the layer's plane: 90 exactly is
    /// [`Etch::FULL`]'s π/2.
    #[must_use]
    pub fn with_sidewall_deg(self, degrees: f64) -> Etch {
        self.with_sidewall(if degrees == 90.0 {
            std::f64::consts::FRAC_PI_2
        } else {
            degrees.to_radians()
        })
    }

    /// How far a wall reaches out per unit of depth, cot θ: 0 for a vertical wall (θ within
    /// 1e-12 of π/2).
    pub fn run(&self) -> f64 {
        let (s, c) = self.sidewall.sin_cos();
        if c.abs() <= 1e-12 { 0.0 } else { c / s }
    }

    /// Whether the etch goes through the layer with vertical walls, as [`Etch::FULL`].
    pub fn is_full(&self) -> bool {
        self.depth.is_none() && self.run() == 0.0
    }

    /// Refuses a depth that isn't positive and less than `thickness` (one equal to it is a full
    /// etch, and is made `None`), or a sidewall outside (0, π/2].
    fn checked(self, layer: &str, thickness: Length) -> Result<Etch> {
        let mut etch = self;
        if let Some(d) = self.depth {
            let (d, t) = (d.to_um(), thickness.to_um());
            if !(d.is_finite() && d > 0.0 && d <= t) {
                return Err(Error::invalid(
                    "etch",
                    format!(
                        "an etch on {layer} must be deeper than 0 and at most its {thickness}, \
                         got {}",
                        Length::um(d)
                    ),
                ));
            }
            if d == t {
                etch.depth = None;
            }
        }
        let a = self.sidewall;
        if !(a.is_finite() && a > 0.0 && a <= std::f64::consts::FRAC_PI_2 + 1e-12) {
            return Err(Error::invalid(
                "etch",
                format!(
                    "a sidewall's angle must be above 0 and at most 90 degrees, got {} degrees",
                    a.to_degrees()
                ),
            ));
        }
        Ok(etch)
    }

    /// The etch's floor, µm below the layer's top: the layer's `thickness` for a full etch.
    fn floor(&self, thickness: f64) -> f64 {
        self.depth.map_or(thickness, Length::to_um)
    }

    /// Whether `shape`, so etched, holds `p` at `depth` µm below the top of a layer `thickness`
    /// µm thick.
    fn covers(&self, shape: &Shape, p: Point, depth: f64, thickness: f64) -> bool {
        if self.depth.is_some() && depth > self.floor(thickness) {
            return true;
        }
        let run = self.run();
        if run == 0.0 {
            return shape.contains(p);
        }
        let reach = depth * run;
        if beyond(shape, p, reach) {
            return false;
        }
        shape.distance(p).signed.to_um() <= reach
    }

    /// The least depth below a layer's top, µm, at which `shape`, so etched, holds `p`; the
    /// layer's material then fills the column down to the layer's bottom. `None` where it
    /// never does.
    fn top_at(&self, shape: &Shape, p: Point, thickness: f64) -> Option<f64> {
        let floor = self.floor(thickness);
        let run = self.run();
        let wall = if run == 0.0 {
            shape.contains(p).then_some(0.0)
        } else if beyond(shape, p, floor * run) {
            None
        } else {
            let d = shape.distance(p).signed.to_um();
            let depth = (d / run).max(0.0);
            (depth <= floor).then_some(depth)
        };
        match (wall, self.depth) {
            (Some(w), _) => Some(w),
            (None, Some(_)) => Some(floor),
            (None, None) => None,
        }
    }
}

/// Whether `p` is farther than `reach` (µm) outside `shape`'s bounding box: then outside the
/// shape grown by `reach`.
fn beyond(shape: &Shape, p: Point, reach: f64) -> bool {
    let b = shape.bounds();
    let (x, y) = (p.x.to_um(), p.y.to_um());
    x < b.min.x.to_um() - reach
        || x > b.max.x.to_um() + reach
        || y < b.min.y.to_um() - reach
        || y > b.max.y.to_um() + reach
}

/// What fills a cell of a vertical cut through a structure ([`Structure::cut_cell`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Fill<'a> {
    /// One material: the one at the cell's centre.
    Uniform(&'a Material),
    /// A sloped wall crosses the cell: `inside` (the layer's material) fills `fraction` of it,
    /// `outside` (its background) the rest, and `normal` is the wall's unit normal in the cut's
    /// plane, (across, up), pointing out of `inside`.
    Wall {
        /// The layer's material.
        inside: &'a Material,
        /// The layer's background.
        outside: &'a Material,
        /// The share of the cell `inside` fills, exactly for a plane wall.
        fraction: f64,
        /// The wall's outward normal, (across, up).
        normal: [f64; 2],
    },
}

impl Fill<'_> {
    /// The cell's permittivity at `wavelength` as the full-vector mode solver takes it: a
    /// uniform cell's material's, a wall's [`mean`] of its two materials' by their shares.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] if a material has no data at `wavelength`.
    pub fn permittivity(&self, wavelength: Wavelength) -> Result<num_complex::Complex64> {
        Ok(match *self {
            Fill::Uniform(m) => m.permittivity(wavelength)?,
            Fill::Wall {
                inside,
                outside,
                fraction,
                ..
            } => mean(
                inside.permittivity(wavelength)?,
                outside.permittivity(wavelength)?,
                fraction,
            ),
        })
    }
}

/// The permittivity of a cell `fraction` of which is `inside` and the rest `outside`: their
/// mean by area, `inside` or `outside` to the bit at a fraction of 1 or 0.
///
/// The full-vector scheme ([`crate::mode::vector`], Fallahkhair et al. 2008) assumes every
/// interface on its grid lines, so a sloped wall is a run of cells between them whatever their
/// value. Of the averages tried against exact answers on a 5 nm grid, the plain mean did best
/// there: on a silicon rod in silica (HE₁₁ exact from the step-index fibre's equation) it is
/// 1.6e-3 off in n_eff, on a strip turned 30° 8e-4 off the same strip on the grid; the tensor
/// of Farjadpour et al. (Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972), the right
/// average on Yee's grid ([`crate::fdtd`]), 3.4e-3 and 1.5e-2, and staircasing 6.1e-3 and
/// 9.3e-3 (docs/methods/stacks.md).
pub fn mean(
    inside: num_complex::Complex64,
    outside: num_complex::Complex64,
    fraction: f64,
) -> num_complex::Complex64 {
    if fraction >= 1.0 {
        inside
    } else if fraction <= 0.0 {
        outside
    } else {
        inside * fraction + outside * (1.0 - fraction)
    }
}

/// The share of the cell [−a/2, a/2] × [−b/2, b/2] where f0 + gx x + gy y ≤ 0: the exact area
/// the line cuts off (Sutherland–Hodgman clipping of the cell by the half-plane).
fn clipped(f0: f64, g: [f64; 2], size: [f64; 2]) -> f64 {
    let (a, b) = (size[0] / 2.0, size[1] / 2.0);
    let corners = [[-a, -b], [a, -b], [a, b], [-a, b]];
    let f = |p: [f64; 2]| f0 + g[0] * p[0] + g[1] * p[1];
    let mut kept: Vec<[f64; 2]> = Vec::with_capacity(5);
    for k in 0..4 {
        let (p, q) = (corners[k], corners[(k + 1) % 4]);
        let (fp, fq) = (f(p), f(q));
        if fp <= 0.0 {
            kept.push(p);
        }
        if (fp < 0.0 && fq > 0.0) || (fp > 0.0 && fq < 0.0) {
            let t = fp / (fp - fq);
            kept.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    let n = kept.len();
    let twice: f64 = (0..n)
        .map(|k| {
            let (p, q) = (kept[k], kept[(k + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    (twice.abs() / 2.0 / (size[0] * size[1])).clamp(0.0, 1.0)
}

/// A layer stack and the shapes drawn on its layers, and bodies placed over them.
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    stack: LayerStack,
    /// for each layer, the shapes drawn on it
    shapes: Vec<Vec<Shape>>,
    /// for each layer, how each of its shapes was etched
    etches: Vec<Vec<Etch>>,
    /// for each layer, whether every shape on it is etched through with vertical walls
    plain: Vec<bool>,
    /// for each layer, an index over its shapes, made when first asked for
    index: Indexes,
    /// bodies over the stack and its shapes, each over the ones before it
    bodies: Vec<(Body, Medium)>,
}

/// What fills a body placed over a structure ([`Structure::place`]): a catalogue material, or a
/// graded medium.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Medium {
    /// A material of the catalogue, as a layer's.
    Material(Material),
    /// A graded medium, n(x, y, z) or ε(x, y, z), the same at every wavelength.
    Graded(Graded),
}

impl Medium {
    /// The relative permittivity at `p` (µm) and `wavelength`.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] if a material has no data at `wavelength`.
    pub fn permittivity(
        &self,
        p: [f64; 3],
        wavelength: Wavelength,
    ) -> Result<num_complex::Complex64> {
        match self {
            Medium::Material(m) => m.permittivity(wavelength),
            Medium::Graded(g) => Ok(num_complex::Complex64::new(g.permittivity_at(p), 0.0)),
        }
    }

    /// Its name: the material's, or the graded medium's expression.
    pub fn name(&self) -> &str {
        match self {
            Medium::Material(m) => m.name(),
            Medium::Graded(g) => g.expr().text(),
        }
    }
}

/// The layers' indexes, made once on first use; equal whether made or not, since they follow
/// from the shapes.
#[derive(Clone, Debug, Default)]
struct Indexes(OnceLock<Vec<Index>>);

impl PartialEq for Indexes {
    fn eq(&self, _: &Indexes) -> bool {
        true
    }
}

/// A layer with more shapes than this is searched through its [`Index`]; with fewer, one by one.
const INDEXED: usize = 8;

impl Structure {
    /// A stack with nothing drawn yet.
    pub fn new(stack: LayerStack) -> Structure {
        let n = stack.layers.len();
        Structure {
            stack,
            shapes: vec![Vec::new(); n],
            etches: vec![Vec::new(); n],
            plain: vec![true; n],
            index: Indexes::default(),
            bodies: Vec::new(),
        }
    }

    /// Places `body`, filled with `medium`, over the stack, its shapes and every body placed
    /// before it: a body in space ([`crate::geometry::Body`]), at any height, through any
    /// layers.
    pub fn place(&mut self, body: Body, medium: Medium) -> &mut Structure {
        self.bodies.push((body, medium));
        self
    }

    /// The bodies placed over the stack, in the order placed (each over the ones before it).
    pub fn bodies(&self) -> &[(Body, Medium)] {
        &self.bodies
    }

    /// The medium of the last body placed that holds (x, y, z), if one does.
    pub fn body_at(&self, p: Point, z: Length) -> Option<&Medium> {
        let q = [p.x.to_um(), p.y.to_um(), z.to_um()];
        self.bodies
            .iter()
            .rev()
            .find(|(body, _)| body.contains(q))
            .map(|(_, m)| m)
    }

    /// The relative permittivity at (x, y, z) and `wavelength`: a body's medium where one holds
    /// the point, a graded one's value there; elsewhere the material [`Structure::material_at`]
    /// gives. What every solver that samples a structure with bodies reads.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] if a material has no data at `wavelength`.
    pub fn permittivity_at(
        &self,
        p: Point,
        z: Length,
        wavelength: Wavelength,
    ) -> Result<num_complex::Complex64> {
        match self.body_at(p, z) {
            Some(m) => m.permittivity([p.x.to_um(), p.y.to_um(), z.to_um()], wavelength),
            None => self.material_at(p, z).permittivity(wavelength),
        }
    }

    /// Draws `shape` on the layer named `layer`, etched through the layer with vertical walls
    /// ([`Etch::FULL`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn draw(&mut self, layer: &str, shape: Shape) -> Result<&mut Structure> {
        self.draw_etched(layer, shape, Etch::FULL)
    }

    /// Draws `shape` on the layer named `layer`, its edges cut by `etch`: the shape is the top
    /// face of what the etch left, its walls leaning out by `etch`'s sidewall, and a partial
    /// etch leaves the layer's material below its floor across the whole layer.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer, the etch's depth isn't positive
    /// and at most the layer's thickness, or its sidewall isn't above 0 and at most 90 degrees.
    pub fn draw_etched(&mut self, layer: &str, shape: Shape, etch: Etch) -> Result<&mut Structure> {
        let i = self
            .stack
            .layers
            .iter()
            .position(|l| l.name == layer)
            .ok_or_else(|| {
                Error::invalid("structure", format!("the stack has no layer {layer}"))
            })?;
        let etch = etch.checked(layer, self.stack.layers[i].thickness)?;
        self.plain[i] &= etch.is_full();
        self.shapes[i].push(shape);
        self.etches[i].push(etch);
        self.index = Indexes::default();
        Ok(self)
    }

    /// How each shape drawn on the layer named `layer` was etched, in the order of
    /// [`Structure::shapes`] (empty for an unknown name).
    pub fn etches(&self, layer: &str) -> &[Etch] {
        self.stack
            .layers
            .iter()
            .position(|l| l.name == layer)
            .map_or(&[], |i| &self.etches[i])
    }

    /// Whether every shape on the layer named `layer` is etched through it with vertical walls,
    /// as [`Structure::draw`] draws (true for an unknown name).
    pub fn is_plain(&self, layer: &str) -> bool {
        self.stack
            .layers
            .iter()
            .position(|l| l.name == layer)
            .is_none_or(|i| self.plain[i])
    }

    /// The bottom and top of layer `i`, µm.
    fn span(&self, i: usize) -> (f64, f64) {
        let bottom: f64 = self.stack.layers[..i]
            .iter()
            .map(|l| l.thickness.to_um())
            .sum();
        (bottom, bottom + self.stack.layers[i].thickness.to_um())
    }

    /// Whether a shape of the etched layer `i` holds `p` at height `z` (µm).
    fn etched(&self, i: usize, p: Point, z: f64) -> bool {
        let (_, top) = self.span(i);
        let thickness = self.stack.layers[i].thickness.to_um();
        let depth = top - z;
        self.shapes[i]
            .iter()
            .zip(&self.etches[i])
            .any(|(s, e)| e.covers(s, p, depth, thickness))
    }

    /// How high the layer named `layer`'s material stands over `p`, from the layer's bottom:
    /// its whole thickness inside a shape's top face, a partial etch's floor where its slab is
    /// left, the height a sloped wall reaches there, and zero where the layer is its background
    /// all through. Below it the column is the layer's material, above it its background.
    /// `None` for an unknown layer.
    pub fn kept(&self, layer: &str, p: Point) -> Option<Length> {
        let i = self.stack.layers.iter().position(|l| l.name == layer)?;
        let thickness = self.stack.layers[i].thickness;
        if self.plain[i] {
            return Some(if self.covered(i, p) {
                thickness
            } else {
                Length::ZERO
            });
        }
        let t = thickness.to_um();
        let top = self.shapes[i]
            .iter()
            .zip(&self.etches[i])
            .filter_map(|(s, e)| e.top_at(s, p, t))
            .fold(f64::INFINITY, f64::min);
        Some(if top == 0.0 {
            thickness
        } else if top.is_finite() {
            Length::um(t - top)
        } else {
            Length::ZERO
        })
    }

    /// What fills a cell of a vertical cut through the structure: the cell centred on the plan
    /// point `centre` at height `z` (µm), `size` µm across the cut (along the unit vector
    /// `across`, in the plane) and up.
    ///
    /// A cell that a sloped wall crosses, within one layer, is [`Fill::Wall`]: the share of it
    /// the layer's material fills and the wall's normal, from the wall's plane through the
    /// point of it nearest the centre, so exact for a straight edge cut square (a trapezoid's
    /// side). Every other cell is [`Fill::Uniform`], the material
    /// [`Structure::material_at`] gives at its centre: a vertical wall or a layer's interface
    /// is where the cut's nodes put it, as the solvers' grids always have.
    pub fn cut_cell(&self, centre: Point, z: f64, across: [f64; 2], size: [f64; 2]) -> Fill<'_> {
        let here = self.material_at(centre, Length::um(z));
        let Place::In(i) = self.stack.locate(Length::um(z)) else {
            return Fill::Uniform(here);
        };
        if self.plain[i] {
            return Fill::Uniform(here);
        }
        let (bottom, top) = self.span(i);
        let (z0, z1) = (z - size[1] / 2.0, z + size[1] / 2.0);
        let slack = 1e-9 * (top - bottom);
        if z0 < bottom - slack || z1 > top + slack {
            return Fill::Uniform(here);
        }
        let thickness = top - bottom;
        let deep = top - z0;
        let mut wall: Option<(f64, [f64; 2])> = None;
        for (shape, etch) in self.shapes[i].iter().zip(&self.etches[i]) {
            let run = etch.run();
            let floor = etch.floor(thickness);
            if run == 0.0 || (etch.depth.is_some() && top - z > floor) {
                // a vertical wall, or below the floor: as the centre has it
                if etch.covers(shape, centre, top - z, thickness) {
                    return Fill::Uniform(here);
                }
                continue;
            }
            if deep > floor + slack {
                // the cell holds the floor: sampled at its centre
                continue;
            }
            let reach = (top - z) * run;
            let half = 0.5 * size[0].hypot(size[1]) * (1.0 + run);
            if beyond(shape, centre, reach + half) {
                continue;
            }
            let d = shape.distance(centre);
            let f0 = d.signed.to_um() - reach;
            let g = [d.normal[0] * across[0] + d.normal[1] * across[1], run];
            let r = 0.5 * (g[0].abs() * size[0] + g[1].abs() * size[1]);
            if f0 <= -r {
                return Fill::Uniform(here);
            }
            if f0 < r && wall.is_none_or(|(w, _)| f0 < w) {
                wall = Some((f0, g));
            }
        }
        let Some((f0, g)) = wall else {
            return Fill::Uniform(here);
        };
        let layer = &self.stack.layers[i];
        let norm = g[0].hypot(g[1]);
        Fill::Wall {
            inside: &layer.material,
            outside: &layer.background,
            fraction: clipped(f0, g, size),
            normal: [g[0] / norm, g[1] / norm],
        }
    }

    /// The index over the shapes drawn on the layer named `layer`, for finding those near a
    /// point or a cell ([`Index::meeting`]); `None` for an unknown layer.
    pub fn index(&self, layer: &str) -> Option<&Index> {
        let i = self.stack.layers.iter().position(|l| l.name == layer)?;
        Some(&self.indexes()[i])
    }

    fn indexes(&self) -> &[Index] {
        self.index
            .0
            .get_or_init(|| self.shapes.iter().map(|s| Index::of(s)).collect())
    }

    /// Whether a shape drawn on layer `i` contains `p`: the shapes whose boxes hold it, through
    /// the layer's index when it has many. The answer is the one testing every shape gives.
    fn covered(&self, i: usize, p: Point) -> bool {
        let shapes = &self.shapes[i];
        if shapes.len() <= INDEXED {
            return shapes.iter().any(|s| s.contains(p));
        }
        self.indexes()[i].any_at(p, |k| shapes[k].contains(p))
    }

    /// The stack.
    pub fn stack(&self) -> &LayerStack {
        &self.stack
    }

    /// The shapes drawn on the layer named `layer` (empty for an unknown name).
    pub fn shapes(&self, layer: &str) -> &[Shape] {
        self.stack
            .layers
            .iter()
            .position(|l| l.name == layer)
            .map_or(&[], |i| &self.shapes[i])
    }

    /// The material at (x, y, z): the substrate below the stack, the cladding above it, and
    /// within a layer its material inside a drawn shape and its background elsewhere; over all
    /// of them, the material of the last body placed that holds the point, when that body's is
    /// a catalogue material (a graded medium has none: [`Structure::permittivity_at`] answers
    /// for it, and here the point is what lies under it).
    pub fn material_at(&self, p: Point, z: Length) -> &Material {
        if !self.bodies.is_empty()
            && let Some(Medium::Material(m)) = self.body_at(p, z)
        {
            return m;
        }
        match self.stack.locate(z) {
            Place::Below => &self.stack.substrate,
            Place::Above => &self.stack.cladding,
            Place::In(i) => {
                let layer = &self.stack.layers[i];
                let inside = if self.plain[i] {
                    self.covered(i, p)
                } else {
                    self.etched(i, p, z.to_um())
                };
                if inside {
                    &layer.material
                } else {
                    &layer.background
                }
            }
        }
    }
}

pub(crate) mod checks;
pub(crate) mod fill;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_presets_are_their_cores_on_their_buried_oxides() {
        // SOI: 220 nm of silicon on 2 µm of oxide
        let stack = LayerStack::soi_220();
        let (si, bottom, top) = stack.layer("Si").unwrap();
        assert_eq!(si.material.name(), "Si");
        assert_eq!(si.background.name(), "SiO2");
        assert!((bottom.to_um() - 2.0).abs() < 1e-12);
        assert!(((top - bottom).to_nm() - 220.0).abs() < 1e-9);
        assert_eq!(stack.cladding().name(), "SiO2");
        assert_eq!(stack.substrate().name(), "Si");
        assert!((stack.thickness().to_um() - 2.22).abs() < 1e-12);
        // the preset is what the general constructor builds
        assert_eq!(
            stack,
            LayerStack::soi(Length::nm(220.0), Length::um(2.0)).unwrap()
        );
        // nitride: its core on the buried oxide asked for
        let stack = LayerStack::nitride(Length::nm(300.0), Length::um(3.0)).unwrap();
        let (sin, bottom, _) = stack.layer("SiN").unwrap();
        assert_eq!(sin.material.name(), "Si3N4");
        assert!((bottom.to_um() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_waveguide_is_silicon_inside_and_oxide_around() {
        let mut s = Structure::new(LayerStack::soi_220());
        let wg = Shape::rect(Point::um(0.0, 0.0), Length::um(10.0), Length::nm(500.0)).unwrap();
        s.draw("Si", wg).unwrap();
        let mid = Length::um(2.11);
        assert_eq!(s.material_at(Point::um(0.0, 0.0), mid).name(), "Si");
        assert_eq!(s.material_at(Point::um(0.0, 0.3), mid).name(), "SiO2");
        // above the device layer: cladding; in the buried oxide: oxide; below: the substrate
        assert_eq!(
            s.material_at(Point::um(0.0, 0.0), Length::um(2.3)).name(),
            "SiO2"
        );
        assert_eq!(
            s.material_at(Point::um(0.0, 0.0), Length::um(1.0)).name(),
            "SiO2"
        );
        assert_eq!(
            s.material_at(Point::um(0.0, 0.0), Length::um(-0.1)).name(),
            "Si"
        );
        assert_eq!(s.shapes("Si").len(), 1);
        assert!(s.shapes("BOX").is_empty());
    }

    #[test]
    fn drawing_on_an_unknown_layer_is_an_error() {
        let mut s = Structure::new(LayerStack::soi_220());
        let shape = Shape::circle(Point::um(0.0, 0.0), Length::um(1.0)).unwrap();
        let e = s.draw("Metal", shape).unwrap_err();
        assert!(e.to_string().contains("Metal"), "{e}");
    }

    #[test]
    fn bad_stacks_are_errors() {
        assert!(LayerStack::soi(Length::ZERO, Length::um(2.0)).is_err());
        assert!(LayerStack::nitride(Length::nm(300.0), Length::um(-1.0)).is_err());
        assert!(LayerStack::new(material::silicon(), Vec::new(), material::silica()).is_err());
        let layer = Layer {
            name: "a".into(),
            thickness: Length::um(1.0),
            material: material::silica(),
            background: material::silica(),
        };
        assert!(
            LayerStack::new(
                material::silicon(),
                vec![layer.clone(), layer],
                material::silica()
            )
            .is_err()
        );
    }
}
