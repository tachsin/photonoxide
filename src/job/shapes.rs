//! `[[task.shape]]`: the shapes defined by functions, masks, and booleans, offsets and corner
//! rounding of any shapes, in `"structure"`, `"fdfd"` and `"fdtd"` jobs
//! ([`crate::geometry::Between`], [`crate::geometry::Polar`], [`crate::geometry::ClosedCurve`],
//! [`crate::geometry::Implicit`], [`crate::geometry::Mask`], [`crate::geometry::Boolean`],
//! [`crate::geometry::Offset`], [`crate::geometry::Rounded`]).
//!
//! A shape's `type` says which; its definition's expressions read the coordinates it gives them
//! and the task's parameters, and its places and sizes are expressions with units or `_um`
//! numbers, as every shape's are:
//!
//! ```toml
//! [[task.shape]]
//! type = "between"            # y_low(x) <= y <= y_high(x)
//! layer = "Si"
//! x = [0, "2 um"]
//! lower = "-(0.25 um + 0.1*x)"
//! upper = "0.25 um + 0.1*x"
//!
//! [[task.shape]]
//! type = "polar"              # r <= rho(t), t the polar angle
//! layer = "Si"
//! center = ["5 um", 0]
//! radius = "2 um*(1 + 0.43*cos(t))"
//!
//! [[task.shape]]
//! type = "parametric"         # inside (x(t), y(t)), t from 0 to 1
//! layer = "Si"
//! curve = ["1.3 um*cos(2*pi*t)", "0.6 um*sin(2*pi*t)"]
//!
//! [[task.shape]]
//! type = "implicit"           # f(x, y) <= 0 inside the window x by y
//! layer = "Si"
//! inside = "(x/2um)^4 + (y/1um)^4 - 1"
//! x = ["-3 um", "3 um"]
//! y = ["-2 um", "2 um"]
//! cells = 256                 # the sampling grid along the window's longer side (optional)
//!
//! [[task.shape]]
//! type = "mask"               # fill values from 0 to 1, the top row first
//! layer = "Si"
//! corner = ["-1 um", "-1 um"] # the lower left corner
//! pixel = ["50 nm", "50 nm"]
//! rows = [[0, 1, 1], [1, 0.5, 0]]
//! ```
//!
//! **Booleans, offsets and corner rounding.** `type = "union"`, `"intersection"`,
//! `"difference"` or `"xor"` combines the shapes in `of`, each an inline table with a `type` of
//! its own and that type's fields but no `layer` or `etch`: a union, intersection or xor of all
//! of them in turn, the first less the others for a difference. `"offset"` grows the one shape
//! in `of` by `distance` (shrinks it where negative), its corners joined `"round"` (the default),
//! `"miter"` (with a `miter_limit`, 2 by default) or `"square"`; `"round"` rounds its convex
//! corners to at least `convex` and its concave ones to at least `concave`. Besides the kinds
//! above, `of` takes a `"rect"` (`center`, `size`), a `"circle"` (`center`, `radius`), a
//! `"ring"` (`center`, `radius`, `width`), a `"polygon"` (`points`, a list of [x, y]) and a
//! `"path"` (`start`, `heading`, `width`, `segments`, as `[[task.path]]`); so does
//! `[[task.shape]]` itself. The result takes an `etch` as any shape does.
//!
//! ```toml
//! [[task.shape]]
//! type = "difference"         # a ring's bus cut where a heater's via lands
//! layer = "Si"
//! of = [
//!   { type = "rect", center = [0, 0], size = ["10 um", "500 nm"] },
//!   { type = "circle", center = ["2 um", 0], radius = "150 nm" },
//! ]
//!
//! [[task.shape]]
//! type = "offset"             # a cladding clearance 1 um about a disk
//! layer = "Si"
//! distance = "1 um"
//! join = "round"
//! of = [{ type = "circle", center = ["5 um", 0], radius = "2 um" }]
//! ```
//!
//! **Arrays and references** ([`crate::geometry::Array`], [`crate::geometry::Lattice`],
//! [`crate::geometry::Reference`]). `type = "array"` draws its `element` at every site of a
//! `lattice`: `"rectangular"` (`pitch`, one value or `[x, y]`; `i` and `j`, each `[first,
//! last]`), `"hexagonal"` (`pitch`, `i`, `j`: rows along x, the odd ones half a pitch along) or
//! `"polar"` (`pitch` between `rings`, ring j holding `count` sites, an expression of `j`;
//! `inner` and `start_angle` optional). Its defects are `remove = [[i, j], …]`,
//! `shift = [{ at = [i, j], by = [dx, dy] }, …]` and `resize = [{ at = [i, j], by = factor }, …]`;
//! `origin` and `angle` place the lattice. The element is drawn about its site, and its places
//! and sizes are expressions of the site too: `i`, `j`, its index, and `x`, `y`, its place in
//! the lattice's frame. `type = "reference"` places a `[[task.cell]]` (`name`, `shapes`, a list
//! as `of` takes) at each of `place = [{ translate, rotate, mirror, scale }, …]`: mirrored
//! (`"x"` or `"y"`), scaled, rotated about the cell's origin, then moved. A cell's shapes may be
//! arrays and references again; one that places itself, and a cell placed nowhere, are refused.
//!
//! ```toml
//! [[task.shape]]
//! type = "array"              # an L3 cavity's hole lattice
//! layer = "Si"
//! lattice = "hexagonal"
//! pitch = "420 nm"
//! i = [-7, 7]
//! j = [-4, 4]
//! element = { type = "circle", radius = "0.29*420 nm" }
//! remove = [[-1, 0], [0, 0], [1, 0]]
//! shift = [{ at = [-2, 0], by = ["-63 nm", 0] }, { at = [2, 0], by = ["63 nm", 0] }]
//! ```
//!
//! Each takes an `etch` as the other shapes do. The resolver ([`super::params`]) puts the task's
//! parameters' definitions (and a sweep's point) in each entry as `parameters` (and `set`), so
//! that its expressions are parsed in their scope here.

use serde::Deserialize;

use super::{EtchSpec, SegmentSpec, path_of, task_error};
use crate::Result;
use crate::expr::{Expr, Parameters};
use crate::geometry::{
    Array, Between, Bounds, Cell, ClosedCurve, Implicit, Join, Lattice, Mask, Operation, Point,
    Polar, Polygon, Reference, Shape, Transform,
};
use crate::stack::Structure;
use crate::units::Length;

/// A `[[task.shape]]`, or a shape in another's `of`. See the module.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ShapeSpec {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    pub(super) layer: String,
    center_um: Option<[f64; 2]>,
    x_um: Option<[f64; 2]>,
    y_um: Option<[f64; 2]>,
    lower: Option<String>,
    upper: Option<String>,
    radius: Option<String>,
    radius_um: Option<f64>,
    curve: Option<[String; 2]>,
    inside: Option<String>,
    cells: Option<usize>,
    corner_um: Option<[f64; 2]>,
    pixel_um: Option<[f64; 2]>,
    rows: Option<Vec<Vec<f64>>>,
    size_um: Option<[f64; 2]>,
    width_um: Option<f64>,
    points_um: Option<Vec<[f64; 2]>>,
    start_um: Option<[f64; 2]>,
    heading_rad: Option<f64>,
    segments: Option<Vec<SegmentSpec>>,
    of: Option<Vec<ShapeSpec>>,
    distance_um: Option<f64>,
    join: Option<String>,
    miter_limit: Option<f64>,
    convex_um: Option<f64>,
    concave_um: Option<f64>,
    lattice: Option<String>,
    pitch_um: Option<Vec<f64>>,
    i: Option<[i64; 2]>,
    j: Option<[i64; 2]>,
    rings: Option<usize>,
    count: Option<toml::Value>,
    inner_um: Option<f64>,
    start_angle_rad: Option<f64>,
    origin_um: Option<[f64; 2]>,
    angle_rad: Option<f64>,
    element: Option<toml::Table>,
    remove: Option<Vec<[i64; 2]>>,
    shift: Option<Vec<ShiftSpec>>,
    resize: Option<Vec<ResizeSpec>>,
    cell: Option<String>,
    place: Option<Vec<PlaceSpec>>,
    /// A reference's cell's shapes: put here by the resolver.
    cell_shapes: Option<Vec<ShapeSpec>>,
    pub(super) etch: Option<EtchSpec>,
    /// The task's parameters, (name, definition): put here by the resolver.
    #[serde(default)]
    parameters: Vec<[String; 2]>,
    /// A sweep's parameter and its value at this point: put here by the resolver.
    set: Option<(String, f64)>,
}

/// A site an array's defect moves: `at = [i, j]`, `by = [dx, dy]`.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShiftSpec {
    at: [i64; 2],
    by_um: [f64; 2],
}

/// A site whose element an array's defect scales: `at = [i, j]`, `by` a factor.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResizeSpec {
    at: [i64; 2],
    by: f64,
}

/// A reference's placement: mirrored (`mirror = "x"`, in the x axis, y to −y; `"y"`, x to −x),
/// scaled, rotated about the cell's origin, then moved, as GDSII places a cell.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaceSpec {
    translate_um: Option<[f64; 2]>,
    rotate_rad: Option<f64>,
    mirror: Option<String>,
    scale: Option<f64>,
}

/// The fields each type takes besides `type`, `layer` and `etch`.
const FIELDS: [(&str, &[&str]); 18] = [
    (
        "array",
        &[
            "lattice",
            "pitch",
            "i",
            "j",
            "rings",
            "count",
            "inner",
            "start_angle",
            "origin",
            "angle",
            "element",
            "remove",
            "shift",
            "resize",
        ],
    ),
    ("reference", &["cell", "place"]),
    ("between", &["x", "lower", "upper"]),
    ("polar", &["center", "radius"]),
    ("parametric", &["curve"]),
    ("implicit", &["inside", "x", "y", "cells"]),
    ("mask", &["corner", "pixel", "rows"]),
    ("rect", &["center", "size"]),
    ("circle", &["center", "radius"]),
    ("ring", &["center", "radius", "width"]),
    ("polygon", &["points"]),
    ("path", &["start", "heading", "width", "segments"]),
    ("union", &["of"]),
    ("intersection", &["of"]),
    ("difference", &["of"]),
    ("xor", &["of"]),
    ("offset", &["of", "distance", "join", "miter_limit"]),
    ("round", &["of", "convex", "concave"]),
];

impl ShapeSpec {
    /// The shape, its errors naming the table (`k` from 0), its type and the field.
    pub(super) fn shape(&self, k: usize) -> Result<Shape> {
        let what = format!("[[task.shape]] #{} ({})", k + 1, self.kind);
        let at = |m: String| task_error(format!("{what}: {m}"));
        let definitions: Vec<(String, String)> = self
            .parameters
            .iter()
            .map(|[n, d]| (n.clone(), d.clone()))
            .collect();
        let mut p = Parameters::new(&definitions).map_err(|e| at(e.to_string()))?;
        if let Some((name, value)) = &self.set {
            p = p.with(name, *value).map_err(|e| at(e.to_string()))?;
        }
        if self.layer.is_empty() {
            return Err(at("needs layer, the layer it is drawn on".into()));
        }
        self.build(&p, &what, &format!("[[task.shape]] #{}", k + 1))
    }

    /// The shape at the parameters `p`, its errors naming it `what` (with its type) or `bare`
    /// (without, for an unknown type).
    fn build(&self, p: &Parameters, what: &str, bare: &str) -> Result<Shape> {
        let at = |m: String| task_error(format!("{what}: {m}"));
        let Some((_, takes)) = FIELDS.iter().find(|(t, _)| *t == self.kind) else {
            return Err(task_error(format!(
                "{bare}: unknown type \"{}\": between, polar, parametric, implicit, mask, rect, \
                 circle, ring, polygon, path, union, intersection, difference, xor, offset, \
                 round, array or reference",
                self.kind
            )));
        };
        let given = [
            ("center", self.center_um.is_some()),
            ("x", self.x_um.is_some()),
            ("y", self.y_um.is_some()),
            ("lower", self.lower.is_some()),
            ("upper", self.upper.is_some()),
            ("radius", self.radius.is_some() || self.radius_um.is_some()),
            ("curve", self.curve.is_some()),
            ("inside", self.inside.is_some()),
            ("cells", self.cells.is_some()),
            ("corner", self.corner_um.is_some()),
            ("pixel", self.pixel_um.is_some()),
            ("rows", self.rows.is_some()),
            ("size", self.size_um.is_some()),
            ("width", self.width_um.is_some()),
            ("points", self.points_um.is_some()),
            ("start", self.start_um.is_some()),
            ("heading", self.heading_rad.is_some()),
            ("segments", self.segments.is_some()),
            ("of", self.of.is_some()),
            ("distance", self.distance_um.is_some()),
            ("join", self.join.is_some()),
            ("miter_limit", self.miter_limit.is_some()),
            ("convex", self.convex_um.is_some()),
            ("concave", self.concave_um.is_some()),
            ("lattice", self.lattice.is_some()),
            ("pitch", self.pitch_um.is_some()),
            ("i", self.i.is_some()),
            ("j", self.j.is_some()),
            ("rings", self.rings.is_some()),
            ("count", self.count.is_some()),
            ("inner", self.inner_um.is_some()),
            ("start_angle", self.start_angle_rad.is_some()),
            ("origin", self.origin_um.is_some()),
            ("angle", self.angle_rad.is_some()),
            ("element", self.element.is_some()),
            ("remove", self.remove.is_some()),
            ("shift", self.shift.is_some()),
            ("resize", self.resize.is_some()),
            ("cell", self.cell.is_some()),
            ("place", self.place.is_some()),
        ];
        if let Some((name, _)) = given.iter().find(|(n, g)| *g && !takes.contains(n)) {
            return Err(at(format!(
                "{name} isn't for this type: it takes {}",
                takes.join(", ")
            )));
        }
        let need = |name: &str| at(format!("needs {name}"));
        let parse = |field: &str, text: &Option<String>| -> Result<Expr> {
            let text = text.as_deref().ok_or_else(|| need(field))?;
            Expr::parse(text, p.scope()).map_err(|e| at(format!("{field}: {e}")))
        };
        let um2 = |v: [f64; 2]| [Length::um(v[0]), Length::um(v[1])];
        let reason = |e: crate::Error| match e {
            crate::Error::InvalidValue { reason, .. } => at(reason),
            other => other,
        };
        // a length's radius: an expression with its unit, or radius_um
        let radius = || -> Result<Length> {
            match (&self.radius, self.radius_um) {
                (Some(_), Some(_)) => Err(at("give radius or radius_um, not both".into())),
                (Some(text), None) => super::params::length(text, p)
                    .map(Length::um)
                    .map_err(|m| at(format!("radius: {m}"))),
                (None, Some(r)) => Ok(Length::um(r)),
                (None, None) => Err(need("radius")),
            }
        };
        let center = || {
            let c = self.center_um.unwrap_or_default();
            Point::um(c[0], c[1])
        };
        Ok(match self.kind.as_str() {
            "between" => {
                let x = self.x_um.ok_or_else(|| need("x, the range of x"))?;
                Shape::Between(
                    Between::new(
                        &parse("lower", &self.lower)?,
                        &parse("upper", &self.upper)?,
                        um2(x),
                        p,
                    )
                    .map_err(reason)?,
                )
            }
            "polar" => Shape::Polar(
                Polar::new(center(), &parse("radius", &self.radius)?, p).map_err(reason)?,
            ),
            "parametric" => {
                let [x, y] = self
                    .curve
                    .as_ref()
                    .ok_or_else(|| need("curve, [x(t), y(t)]"))?;
                Shape::ClosedCurve(
                    ClosedCurve::new(
                        &parse("curve x", &Some(x.clone()))?,
                        &parse("curve y", &Some(y.clone()))?,
                        p,
                    )
                    .map_err(reason)?,
                )
            }
            "implicit" => {
                let (x, y) = (
                    self.x_um
                        .ok_or_else(|| need("x, its window's range of x"))?,
                    self.y_um
                        .ok_or_else(|| need("y, its window's range of y"))?,
                );
                let window = Bounds::new(Point::um(x[0], y[0]), Point::um(x[1], y[1]));
                let f = parse("inside", &self.inside)?;
                let cells = self.cells.unwrap_or(crate::geometry::IMPLICIT_CELLS);
                Shape::Implicit(Implicit::with_cells(&f, window, cells, p).map_err(reason)?)
            }
            "mask" => {
                let corner = self.corner_um.unwrap_or_default();
                let pixel = self
                    .pixel_um
                    .ok_or_else(|| need("pixel, the pixels' sides"))?;
                let rows = self
                    .rows
                    .as_ref()
                    .ok_or_else(|| need("rows, the top row first"))?;
                Shape::Mask(
                    Mask::from_rows(rows, Point::um(corner[0], corner[1]), um2(pixel))
                        .map_err(reason)?,
                )
            }
            "rect" => {
                let [w, h] = self.size_um.ok_or_else(|| need("size, [width, height]"))?;
                Shape::rect(center(), Length::um(w), Length::um(h)).map_err(reason)?
            }
            "circle" => Shape::circle(center(), radius()?).map_err(reason)?,
            "ring" => {
                let w = self
                    .width_um
                    .ok_or_else(|| need("width, the ring's waveguide's"))?;
                Shape::ring(center(), radius()?, Length::um(w)).map_err(reason)?
            }
            "polygon" => {
                let points = self
                    .points_um
                    .as_ref()
                    .ok_or_else(|| need("points, a list of [x, y]"))?;
                Shape::Polygon(
                    Polygon::new(points.iter().map(|v| Point::um(v[0], v[1])).collect())
                        .map_err(reason)?,
                )
            }
            "path" => {
                let w = self.width_um.ok_or_else(|| need("width"))?;
                let segments = self.segments.as_deref().ok_or_else(|| need("segments"))?;
                Shape::Path(path_of(
                    self.start_um.unwrap_or_default(),
                    self.heading_rad.unwrap_or_default(),
                    w,
                    segments,
                    what,
                )?)
            }
            "array" => self.array(p, what)?,
            "reference" => self.reference(p, what)?,
            kind => self.combined(kind, p, what)?,
        })
    }

    /// An array: its lattice, its defects, and its element made at each site.
    fn array(&self, p: &Parameters, what: &str) -> Result<Shape> {
        let at = |m: String| task_error(format!("{what}: {m}"));
        let reason = |e: crate::Error| match e {
            crate::Error::InvalidValue { reason, .. } => at(reason),
            other => other,
        };
        let kind = self.lattice.as_deref().ok_or_else(|| {
            at("needs lattice: \"rectangular\", \"hexagonal\" or \"polar\"".into())
        })?;
        let pitch = self
            .pitch_um
            .as_deref()
            .ok_or_else(|| at("needs pitch, the lattice's spacing".into()))?;
        let one_pitch = || -> Result<Length> {
            match pitch {
                [a] => Ok(Length::um(*a)),
                _ => Err(at(format!(
                    "pitch: a {kind} lattice takes one, got {}",
                    pitch.len()
                ))),
            }
        };
        let planar = |field: &str, given: bool| -> Result<()> {
            if given {
                Err(at(format!("{field} is for a polar lattice")))
            } else {
                Ok(())
            }
        };
        let ranges = || -> Result<([i64; 2], [i64; 2])> {
            planar("rings", self.rings.is_some())?;
            planar("count", self.count.is_some())?;
            planar("inner", self.inner_um.is_some())?;
            planar("start_angle", self.start_angle_rad.is_some())?;
            Ok((
                self.i
                    .ok_or_else(|| at("needs i, the range of i: [first, last]".into()))?,
                self.j
                    .ok_or_else(|| at("needs j, the range of j: [first, last]".into()))?,
            ))
        };
        let mut lattice = match kind {
            "rectangular" => {
                let (i, j) = ranges()?;
                let pitch = match pitch {
                    [a] => [Length::um(*a); 2],
                    [a, b] => [Length::um(*a), Length::um(*b)],
                    _ => return Err(at("pitch: one value, or two (x and y)".into())),
                };
                Lattice::rectangular(pitch, i, j).map_err(reason)?
            }
            "hexagonal" => {
                let (i, j) = ranges()?;
                Lattice::hexagonal(one_pitch()?, i, j).map_err(reason)?
            }
            "polar" => {
                if self.i.is_some() || self.j.is_some() {
                    return Err(at(
                        "i and j are for a rectangular or hexagonal lattice: a polar one takes \
                         rings and count"
                            .into(),
                    ));
                }
                let rings = self
                    .rings
                    .filter(|&n| n > 0)
                    .ok_or_else(|| at("needs rings, how many (at least 1)".into()))?;
                let count = self.count.as_ref().ok_or_else(|| {
                    at("needs count, the sites on ring j: a number, or an expression of j".into())
                })?;
                let text = match count {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Integer(n) => n.to_string(),
                    other => {
                        return Err(at(format!("count: a number or an expression, got {other}")));
                    }
                };
                let e = Expr::parse(&text, p.scope()).map_err(|e| at(format!("count: {e}")))?;
                let mut counts = Vec::with_capacity(rings);
                for j in 0..rings {
                    let q = p
                        .at_coordinate("j", j as f64)
                        .map_err(|e| at(e.to_string()))?;
                    let n = e
                        .try_eval(q.values())
                        .map_err(|e| at(format!("count at ring {j}: {e}")))?;
                    if !(n >= 0.0 && n.fract() == 0.0 && n <= crate::geometry::MAX_SITES as f64) {
                        return Err(at(format!(
                            "count at ring {j} is {n}: it must be a whole number of sites"
                        )));
                    }
                    counts.push(n as usize);
                }
                Lattice::polar(
                    Length::um(self.inner_um.unwrap_or_default()),
                    one_pitch()?,
                    &counts,
                    self.start_angle_rad.unwrap_or_default(),
                )
                .map_err(reason)?
            }
            other => {
                return Err(at(format!(
                    "lattice: unknown lattice \"{other}\": rectangular, hexagonal or polar"
                )));
            }
        };
        for &k in self.remove.as_deref().unwrap_or_default() {
            lattice = lattice.remove(k).map_err(reason)?;
        }
        for s in self.shift.as_deref().unwrap_or_default() {
            lattice = lattice
                .shift(s.at, [Length::um(s.by_um[0]), Length::um(s.by_um[1])])
                .map_err(reason)?;
        }
        for s in self.resize.as_deref().unwrap_or_default() {
            lattice = lattice.resize(s.at, s.by).map_err(reason)?;
        }
        let template = self
            .element
            .as_ref()
            .ok_or_else(|| at("needs element, the shape drawn at each site".into()))?;
        let array = Array::generate(&lattice, |site| {
            let [i, j] = site.index;
            let name = format!("{what}, element ({i}, {j})");
            let q = p
                .at_coordinate("i", i as f64)
                .and_then(|q| q.at_coordinate("j", j as f64))
                .and_then(|q| q.at_coordinate("x", site.at.x.to_um()))
                .and_then(|q| q.at_coordinate("y", site.at.y.to_um()))
                .map_err(|e| task_error(format!("{name}: {e}")))?;
            let mut entry = template.clone();
            super::params::resolve_element(&mut entry, &q, &name)?;
            let spec: ShapeSpec = toml::Value::Table(entry)
                .try_into()
                .map_err(|e: toml::de::Error| task_error(format!("{name}: {e}")))?;
            if !spec.layer.is_empty() || spec.etch.is_some() {
                return Err(task_error(format!(
                    "{name}: an element takes no layer or etch: the array's own are its"
                )));
            }
            spec.build(&q, &format!("{name} ({})", spec.kind), &name)
                .map(Some)
        })
        .map_err(reason)?;
        let (origin, angle) = (
            self.origin_um.unwrap_or_default(),
            self.angle_rad.unwrap_or_default(),
        );
        if origin == [0.0, 0.0] && angle == 0.0 {
            return Ok(Shape::Array(array));
        }
        let t = Transform::rotate(angle, Point::default()).then(&Transform::translate(
            Length::um(origin[0]),
            Length::um(origin[1]),
        ));
        Ok(Shape::Array(array.transformed(&t)))
    }

    /// A reference: its cell's shapes (put in `cell_shapes` by the resolver) at each placement.
    fn reference(&self, p: &Parameters, what: &str) -> Result<Shape> {
        let at = |m: String| task_error(format!("{what}: {m}"));
        let reason = |e: crate::Error| match e {
            crate::Error::InvalidValue { reason, .. } => at(reason),
            other => other,
        };
        let name = self
            .cell
            .as_deref()
            .ok_or_else(|| at("needs cell, the name of a [[task.cell]]".into()))?;
        let specs = self
            .cell_shapes
            .as_deref()
            .ok_or_else(|| at(format!("unknown cell \"{name}\"")))?;
        let mut shapes = Vec::with_capacity(specs.len());
        for (j, s) in specs.iter().enumerate() {
            let bare = format!("{what}, cell \"{name}\", shape {}", j + 1);
            if !s.layer.is_empty() || s.etch.is_some() {
                return Err(task_error(format!(
                    "{bare}: a cell's shape takes no layer or etch: the reference's own are its"
                )));
            }
            shapes.push(s.build(p, &format!("{bare} ({})", s.kind), &bare)?);
        }
        let places = self
            .place
            .as_deref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                at(
                    "needs place, a list of placements: { translate, rotate, mirror, scale }"
                        .into(),
                )
            })?;
        let mut placements = Vec::with_capacity(places.len());
        for (k, pl) in places.iter().enumerate() {
            let at = |m: String| task_error(format!("{what}, place {}: {m}", k + 1));
            let mut t = match pl.mirror.as_deref() {
                None => Transform::IDENTITY,
                Some("x") => Transform::mirror(Point::default(), 0.0),
                Some("y") => Transform::mirror(Point::default(), std::f64::consts::FRAC_PI_2),
                Some(other) => {
                    return Err(at(format!(
                        "mirror: \"{other}\": \"x\" (in the x axis) or \"y\" (in the y axis)"
                    )));
                }
            };
            if let Some(s) = pl.scale {
                let s = Transform::scale(s, Point::default()).map_err(|e| match e {
                    crate::Error::InvalidValue { reason, .. } => at(format!("scale: {reason}")),
                    other => other,
                })?;
                t = t.then(&s);
            }
            let r = pl.rotate_rad.unwrap_or_default();
            if !r.is_finite() {
                return Err(at(format!("rotate: {r} isn't finite")));
            }
            if r != 0.0 {
                t = t.then(&Transform::rotate(r, Point::default()));
            }
            if let Some([dx, dy]) = pl.translate_um {
                if !(dx.is_finite() && dy.is_finite()) {
                    return Err(at("translate isn't finite".into()));
                }
                t = t.then(&Transform::translate(Length::um(dx), Length::um(dy)));
            }
            placements.push(t);
        }
        let cell = Cell::new(name, shapes).map_err(reason)?;
        Ok(Shape::Reference(
            Reference::new(cell, placements).map_err(reason)?,
        ))
    }

    /// A boolean, offset or rounding of the shapes in `of`.
    fn combined(&self, kind: &str, p: &Parameters, what: &str) -> Result<Shape> {
        let at = |m: String| task_error(format!("{what}: {m}"));
        let reason = |e: crate::Error| match e {
            crate::Error::InvalidValue { reason, .. } => at(reason),
            other => other,
        };
        let of = self
            .of
            .as_deref()
            .filter(|of| !of.is_empty())
            .ok_or_else(|| at("needs of, the shapes it is made of".into()))?;
        let mut shapes = Vec::with_capacity(of.len());
        for (j, s) in of.iter().enumerate() {
            let bare = format!("{what}, of {}", j + 1);
            if !s.layer.is_empty() || s.etch.is_some() {
                return Err(task_error(format!(
                    "{bare}: a shape in of takes no layer or etch: the {kind}'s own are its"
                )));
            }
            shapes.push(s.build(p, &format!("{bare} ({})", s.kind), &bare)?);
        }
        let one = || -> Result<Shape> {
            match shapes.as_slice() {
                [s] => Ok(s.clone()),
                _ => Err(at(format!("takes one shape in of, got {}", shapes.len()))),
            }
        };
        let op = match kind {
            "union" => Operation::Union,
            "intersection" => Operation::Intersection,
            "difference" => Operation::Difference,
            "xor" => Operation::Xor,
            "offset" => {
                let d = self
                    .distance_um
                    .ok_or_else(|| at("needs distance (negative shrinks)".into()))?;
                let join = match self.join.as_deref().unwrap_or("round") {
                    "round" => {
                        if self.miter_limit.is_some() {
                            return Err(at("miter_limit is for a miter join".into()));
                        }
                        Join::Round
                    }
                    "miter" => Join::miter(self.miter_limit.unwrap_or(2.0)),
                    "square" => {
                        if self.miter_limit.is_some() {
                            return Err(at("miter_limit is for a miter join".into()));
                        }
                        Join::Square
                    }
                    other => {
                        return Err(at(format!(
                            "join: unknown join \"{other}\": round, miter or square"
                        )));
                    }
                };
                return one()?.offset(Length::um(d), join).map_err(reason);
            }
            _ => {
                let (convex, concave) = (
                    self.convex_um.unwrap_or_default(),
                    self.concave_um.unwrap_or_default(),
                );
                if convex == 0.0 && concave == 0.0 {
                    return Err(at("needs convex or concave, a corner's least radius".into()));
                }
                return one()?
                    .rounded(Length::um(convex), Length::um(concave))
                    .map_err(reason);
            }
        };
        if shapes.len() < 2 {
            return Err(at(format!(
                "needs two or more shapes in of, got {}",
                shapes.len()
            )));
        }
        let mut out = shapes[0].clone();
        for s in &shapes[1..] {
            // a difference takes the others from the first, one after another
            out = out.boolean(op, s).map_err(reason)?;
        }
        Ok(out)
    }
}

/// `s` with `shapes` drawn on it, after its other shapes.
pub(super) fn draw(mut s: Structure, shapes: &[ShapeSpec]) -> Result<Structure> {
    for (k, spec) in shapes.iter().enumerate() {
        let shape = spec.shape(k)?;
        match spec.etch {
            None => s.draw(&spec.layer, shape).map(|_| ())?,
            Some(e) => s
                .draw_etched(&spec.layer, shape, EtchSpec::etch(Some(e)))
                .map(|_| ())
                .map_err(|e| match e {
                    crate::Error::InvalidValue { reason, .. } => {
                        task_error(format!("[[task.shape]] #{}: {reason}", k + 1))
                    }
                    other => other,
                })?,
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use crate::geometry::{Point, Shape};
    use crate::job::{Event, StructureTask, check, params, preview};
    use crate::run::Job;
    use crate::units::Length;

    const JOB: &str = r#"
name = "shapes"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-4.0, 12.0]
y_um = [-4.0, 4.0]
step_nm = 100.0

[task.parameters]
w = "500 nm"
r0 = "2 um"

[[task.shape]]
type = "between"
layer = "Si"
x = [0, "2 um"]
lower = "-(w/2 + 0.1*x)"
upper = "w/2 + 0.1*x"

[[task.shape]]
type = "polar"
layer = "Si"
center = ["6 um", 0]
radius = "r0*(1 + 0.43*cos(t))"

[[task.shape]]
type = "parametric"
layer = "Si"
curve = ["-2 um + 1.3 um*cos(2*pi*t)", "0.6 um*sin(2*pi*t)"]

[[task.shape]]
type = "implicit"
layer = "Si"
inside = "((x - 10 um)/1um)^4 + (y/1um)^4 - 1"
x = ["8.5 um", "11.5 um"]
y = ["-1.5 um", "1.5 um"]
cells = 128

[[task.shape]]
type = "mask"
layer = "Si"
corner = ["-1 um", "2 um"]
pixel = ["100 nm", "100 nm"]
rows = [[0, 1, 1], [1, 0.5, 0]]
etch = { depth = "70 nm" }
"#;

    #[test]
    fn shapes_in_a_job_are_checked_drawn_and_previewed() {
        let job = Job::parse(JOB).unwrap();
        check(&job).unwrap();
        let Event::Scene { shapes, .. } = preview(&job).unwrap() else {
            panic!("a structure's preview is a scene")
        };
        assert!(shapes.len() >= 5, "{}", shapes.len());
        let task: StructureTask = params::task(&job).unwrap();
        assert_eq!(task.shape.len(), 5);
        let kinds: Vec<Shape> = (0..5).map(|k| task.shape[k].shape(k).unwrap()).collect();
        // the taper's area, (0.5 + 0.9)/2 2 µm², from the parameter w
        assert!((kinds[0].area() - 1.4).abs() < 1e-13, "{}", kinds[0].area());
        // the limaçon's, π r0² (1 + ε²/2)
        let limacon = std::f64::consts::PI * 4.0 * (1.0 + 0.43 * 0.43 / 2.0);
        assert!((kinds[1].area() - limacon).abs() < 1e-12);
        assert!(
            (kinds[4].area() - 0.035).abs() < 1e-15,
            "{}",
            kinds[4].area()
        );
        let s = task.structure().unwrap();
        let z = Length::um(s.stack().layer("Si").unwrap().1.to_um() + 0.2);
        let at = |x: f64, y: f64| s.material_at(Point::um(x, y), z).name().to_owned();
        assert_eq!(at(1.0, 0.3), "Si");
        assert_ne!(at(1.0, 0.5), "Si");
        assert_eq!(at(7.5, 0.0), "Si");
        assert_eq!(at(-2.0, 0.5), "Si");
        assert_eq!(at(10.8, 0.8), "Si");
    }

    #[test]
    fn what_a_shape_in_a_job_refuses() {
        let bad = |from: &str, to: &str, says: &str| {
            let job = Job::parse(&JOB.replace(from, to)).unwrap();
            let e = check(&job).unwrap_err().to_string();
            assert!(e.contains(says), "{e}");
        };
        bad(
            "type = \"between\"",
            "type = \"spline\"",
            "unknown type \"spline\"",
        );
        bad(
            "upper = \"w/2 + 0.1*x\"",
            "upper = \"-w\"",
            "#1 (between): its lower curve meets",
        );
        bad(
            "upper = \"w/2 + 0.1*x\"",
            "upper = \"w/2 + wdth\"",
            "upper: column 7: unknown name",
        );
        bad(
            "r0*(1 + 0.43*cos(t))",
            "r0*(1 + 1.5*cos(t))",
            "#2 (polar): its radius",
        );
        bad(
            "type = \"parametric\"\nlayer = \"Si\"\n",
            "type = \"parametric\"\nlayer = \"Si\"\nradius = \"1 um\"\n",
            "radius isn't for this type",
        );
        bad(
            "x = [\"8.5 um\", \"11.5 um\"]",
            "x = [\"9.5 um\", \"11.5 um\"]",
            "window's edge",
        );
        bad(
            "rows = [[0, 1, 1], [1, 0.5, 0]]",
            "rows = [[0, 1, 1], [1, 0.5]]",
            "same length",
        );
        bad(
            "pixel = [\"100 nm\", \"100 nm\"]",
            "pixel = [\"100 nm\", 2]",
            "needs a length",
        );
    }

    const BOOLEANS: &str = r#"
name = "booleans"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-6.0, 14.0]
y_um = [-5.0, 5.0]
step_nm = 100.0

[task.parameters]
w = "500 nm"
r = "2 um"

[[task.shape]]
type = "difference"
layer = "Si"
of = [
  { type = "rect", center = [0, 0], size = ["10 um", "w"] },
  { type = "circle", center = ["2 um", 0], radius = "150 nm" },
  { type = "rect", center = ["-2 um", 0], size = ["1 um", "1 um"] },
]

[[task.shape]]
type = "union"
layer = "Si"
of = [
  { type = "ring", center = ["9 um", 0], radius = "r", width = "w" },
  { type = "path", start = ["9 um", "-4 um"], width = "w", segments = [{ straight = "3 um" }] },
  { type = "polygon", points = [["8 um", "2 um"], ["10 um", "2 um"], ["9 um", "3 um"]] },
]

[[task.shape]]
type = "offset"
layer = "Si"
distance = "-50 nm"
join = "miter"
miter_limit = 3
of = [{ type = "polar", center = ["4 um", "3 um"], radius = "1 um*(1 + 0.2*cos(3*t))" }]

[[task.shape]]
type = "round"
layer = "Si"
convex = "100 nm"
concave = "200 nm"
of = [{ type = "xor", of = [
  { type = "rect", center = ["4 um", "-3 um"], size = ["2 um", "1 um"] },
  { type = "rect", center = ["4.5 um", "-2.5 um"], size = ["2 um", "1 um"] },
] }]
etch = { depth = "70 nm", sidewall = "80 deg" }
"#;

    #[test]
    fn booleans_offsets_and_rounding_in_a_job() {
        let job = Job::parse(BOOLEANS).unwrap();
        check(&job).unwrap();
        let task: StructureTask = params::task(&job).unwrap();
        let shapes: Vec<Shape> = (0..4).map(|k| task.shape[k].shape(k).unwrap()).collect();
        // the guide less a hole and a square bite across it: two pieces
        let hole = std::f64::consts::PI * 0.15 * 0.15;
        let guide = 10.0 * 0.5 - hole - 1.0 * 0.5;
        assert!(
            (shapes[0].area() - guide).abs() < 1e-9,
            "{}",
            shapes[0].area()
        );
        let s = task.structure().unwrap();
        let top = s.stack().layer("Si").unwrap().2.to_um();
        let z = Length::um(top - 0.01);
        let at = |x: f64, y: f64| s.material_at(Point::um(x, y), z).name().to_owned();
        assert_eq!(at(0.0, 0.1), "Si");
        assert_ne!(at(2.0, 0.0), "Si", "the hole");
        assert_ne!(at(-2.0, 0.0), "Si", "the bite");
        assert_eq!(at(11.0, 0.0), "Si", "the ring");
        assert_eq!(at(9.0, 2.5), "Si", "the triangle");
        assert_eq!(at(10.0, -4.0), "Si", "the path");
        assert_eq!(at(4.0, 3.0), "Si", "the shrunk flower");
        // the xor's overlap is out; its two arms are in, below the etch's floor everywhere
        assert_ne!(at(4.25, -2.75), "Si");
        assert_eq!(at(3.3, -3.3), "Si");
        let floor = Length::um(top - 0.1);
        let below = |x: f64, y: f64| s.material_at(Point::um(x, y), floor).name().to_owned();
        assert_eq!(below(4.25, -2.75), "Si", "the partial etch's slab");
        // the scene draws the difference's two pieces and the rest
        let Event::Scene { shapes: scene, .. } = preview(&job).unwrap() else {
            panic!("a structure's preview is a scene")
        };
        assert!(scene.len() >= 5, "{}", scene.len());
        // each outline's first vertex closes no other polygon: a hole is walked back to its own
        // first vertex, and the two pieces of the guide are outlines of their own
        let guide_pieces = scene
            .iter()
            .filter(|o| o.layer == "Si")
            .filter(|o| {
                o.outline
                    .iter()
                    .all(|p| p[1].abs() <= 0.25 + 1e-9 && p[0] <= 5.0 + 1e-9)
            })
            .count();
        assert_eq!(guide_pieces, 2);
    }

    const ARRAYS: &str = r#"
name = "arrays"

[task]
kind = "structure"
wavelength_um = 1.55
layer = "Si"
x_um = [-4.0, 14.0]
y_um = [-5.0, 9.0]
step_nm = 100.0

[task.stack]
substrate = "sio2"
cladding = "sio2"

[[task.stack.layer]]
name = "Si"
thickness = "252 nm"
material = "si"
background = "sio2"

[task.parameters]
a = "420 nm"
r = "0.29*a"

[[task.cell]]
name = "pair"
shapes = [
  { type = "circle", center = [0, 0], radius = "100 nm" },
  { type = "rect", center = ["0.5 um", 0], size = ["300 nm", "200 nm"] },
]

[[task.shape]]
type = "difference"
layer = "Si"
of = [
  { type = "rect", center = [0, 0], size = ["7 um", "4 um"] },
  { type = "array", lattice = "hexagonal", pitch = "a", i = [-7, 7], j = [-4, 4],
    element = { type = "circle", radius = "r" },
    remove = [[-1, 0], [0, 0], [1, 0]],
    shift = [{ at = [-2, 0], by = ["-0.15*a", 0] }, { at = [2, 0], by = ["0.15*a", 0] }] },
]

[[task.shape]]
type = "array"
layer = "Si"
lattice = "polar"
pitch = "0.5 um"
rings = 4
count = "max(1, 6*j)"
origin = ["10 um", 0]
element = { type = "circle", radius = "60 nm + 10 nm*j + 0.01*sqrt(x^2 + y^2)" }

[[task.shape]]
type = "array"
layer = "Si"
lattice = "rectangular"
pitch = ["0.5 um", "1 um"]
i = [0, 9]
j = [0, 0]
origin = ["8 um", "-4 um"]
element = { type = "rect", center = ["2 nm*i^2", 0], size = ["200 nm + 10 nm*i", "600 nm"] }
etch = { depth = "70 nm" }

[[task.shape]]
type = "reference"
layer = "Si"
cell = "pair"
place = [
  { translate = ["6 um", "6 um"] },
  { translate = ["8 um", "6 um"], rotate = "90 deg" },
  { translate = ["10 um", "6 um"], mirror = "y", scale = 2 },
]
"#;

    #[test]
    fn arrays_and_references_in_a_job() {
        use crate::geometry::Region;
        let job = Job::parse(ARRAYS).unwrap();
        check(&job).unwrap();
        let task: StructureTask = params::task(&job).unwrap();
        let shapes: Vec<Shape> = (0..4).map(|k| task.shape[k].shape(k).unwrap()).collect();
        let (a, r) = (0.42, 0.29 * 0.42);
        // the slab less 15 × 9 − 3 holes
        let holes = (15.0 * 9.0 - 3.0) * std::f64::consts::PI * r * r;
        let slab = 7.0 * 4.0 - holes;
        // (some holes at the edges are cut: their parts outside the slab don't count)
        assert!(shapes[0].area() < slab + 1e-9 && shapes[0].area() > 20.0);
        // the polar array: 1 + 6 + 12 + 18 pillars about (10, 0), radii from j and the distance
        let Shape::Array(lens) = &shapes[1] else {
            panic!("an array")
        };
        assert_eq!(lens.len(), 37);
        let e = lens.element([3, 2]).unwrap();
        let Shape::Circle { center, radius } = &e.shape else {
            panic!("a circle")
        };
        let rho = ((center.x.to_um() - 10.0).powi(2) + center.y.to_um().powi(2)).sqrt();
        assert!((rho - 1.0).abs() < 1e-12);
        assert!((radius.to_um() - (0.06 + 0.02 + 0.01 * rho)).abs() < 1e-12);
        // the grating: tooth i, 0.2 + 0.01 i µm wide, at 8 + 0.5 i + 0.002 i² µm
        let Shape::Array(grating) = &shapes[2] else {
            panic!("an array")
        };
        for el in grating.elements() {
            let i = el.index[0] as f64;
            let b = el.shape.bounds();
            assert!(((b.max.x - b.min.x).to_um() - (0.2 + 0.01 * i)).abs() < 1e-12);
            let c = 0.5 * (b.max.x + b.min.x).to_um();
            assert!((c - (8.0 + 0.5 * i + 0.002 * i * i)).abs() < 1e-12, "{c}");
        }
        // the reference: the pair three times, turned and mirrored and doubled
        let Shape::Reference(pairs) = &shapes[3] else {
            panic!("a reference")
        };
        assert_eq!(pairs.array().len(), 6);
        assert!(pairs.contains(Point::um(6.5, 6.0)));
        assert!(pairs.contains(Point::um(8.0, 6.5)), "turned a quarter");
        assert!(
            pairs.contains(Point::um(9.0, 6.0)),
            "mirrored in y and doubled"
        );
        assert!(!pairs.contains(Point::um(11.0, 6.0)));
        // drawn: the cavity's centre is silicon, a hole's air
        let s = task.structure().unwrap();
        let z = Length::um(0.2);
        let at = |x: f64, y: f64| s.material_at(Point::um(x, y), z).name().to_owned();
        assert_eq!(at(0.0, 0.0), "Si");
        assert_eq!(at(a, 0.0), "Si");
        assert_ne!(at(3.0 * a, 0.0), "Si");
        assert_eq!(at(10.0, 0.0), "Si", "the lens's centre pillar");
        // the preview draws every hole, pillar and tooth
        let Event::Scene { shapes: scene, .. } = preview(&job).unwrap() else {
            panic!("a structure's preview is a scene")
        };
        assert!(scene.len() >= 1 + 37 + 10 + 6, "{}", scene.len());
    }

    #[test]
    fn what_an_array_or_a_reference_in_a_job_refuses() {
        let bad = |from: &str, to: &str, says: &str| {
            let text = ARRAYS.replace(from, to);
            assert_ne!(text, ARRAYS, "{from}");
            let job = Job::parse(&text).unwrap();
            let e = check(&job).unwrap_err().to_string();
            assert!(e.contains(says), "{e}");
        };
        bad(
            "lattice = \"hexagonal\"",
            "lattice = \"square\"",
            "unknown lattice \"square\"",
        );
        bad(
            "remove = [[-1, 0], [0, 0], [1, 0]]",
            "remove = [[-1, 0], [0, 0], [9, 0]]",
            "can't remove site (9, 0): i runs from -7 to 7",
        );
        bad(
            "{ at = [2, 0], by = [\"0.15*a\", 0] }",
            "{ at = [0, 0], by = [\"0.15*a\", 0] }",
            "can't shift site (0, 0): it was removed",
        );
        bad(
            "radius = \"60 nm + 10 nm*j + 0.01*sqrt(x^2 + y^2)\"",
            "radius = \"60 nm - 30 nm*j\"",
            "[[task.shape]] #2 (array), element (0, 2) (circle): needs a finite centre",
        );
        bad(
            "radius = \"60 nm + 10 nm*j + 0.01*sqrt(x^2 + y^2)\"",
            "radius = \"60 nm + z\"",
            "element (0, 0) (circle): radius: z is a coordinate",
        );
        bad(
            "count = \"max(1, 6*j)\"",
            "count = \"6.5*j\"",
            "count at ring 1 is 6.5",
        );
        bad("rings = 4", "rings = 4\ni = [0, 1]", "i and j are for");
        bad(
            "cell = \"pair\"",
            "cell = \"pear\"",
            "unknown cell \"pear\": the cells are pair",
        );
        bad(
            "{ type = \"circle\", center = [0, 0], radius = \"100 nm\" },",
            "{ type = \"reference\", cell = \"pair\", place = [{ translate = [0, 0] }] },",
            "the cell \"pair\" places itself",
        );
        bad(
            "[[task.cell]]\nname = \"pair\"",
            "[[task.cell]]\nname = \"spare\"\nshapes = [{ type = \"circle\", radius = \"1 um\" }]\n\n[[task.cell]]\nname = \"pair\"",
            "\"spare\" is placed nowhere",
        );
        bad("mirror = \"y\"", "mirror = \"z\"", "place 3: mirror: \"z\"");
        bad(
            "{ type = \"circle\", center = [0, 0], radius = \"100 nm\" },",
            "{ type = \"circle\", layer = \"Si\", center = [0, 0], radius = \"100 nm\" },",
            "a cell's shape takes no layer",
        );
    }

    #[test]
    fn what_a_boolean_in_a_job_refuses() {
        let bad = |from: &str, to: &str, says: &str| {
            let job = Job::parse(&BOOLEANS.replace(from, to)).unwrap();
            let e = check(&job).unwrap_err().to_string();
            assert!(e.contains(says), "{e}");
        };
        bad(
            "{ type = \"circle\", center = [\"2 um\", 0], radius = \"150 nm\" }",
            "{ type = \"circle\", center = [\"2 um\", 0], radius = \"150\" }",
            "of 2 (circle): radius: needs a length",
        );
        bad(
            "{ type = \"circle\", center = [\"2 um\", 0], radius = \"150 nm\" }",
            "{ type = \"circle\", layer = \"Si\", center = [\"2 um\", 0], radius = \"150 nm\" }",
            "takes no layer",
        );
        bad(
            "join = \"miter\"",
            "join = \"bevel\"",
            "unknown join \"bevel\"",
        );
        bad("miter_limit = 3", "miter_limit = 0.5", "at least 1");
        bad(
            "distance = \"-50 nm\"",
            "distance = \"-5 um\"",
            "leaves nothing",
        );
        bad(
            "type = \"union\"",
            "type = \"intersection\"",
            "#2 (intersection): the intersection is empty",
        );
        bad(
            "{ type = \"rect\", center = [0, 0], size = [\"10 um\", \"w\"] },",
            "{ type = \"rect\", center = [0, 0], size = [\"10 um\", \"wdth\"] },",
            "[[task.shape]] #1, of 1, size: value 2: column 1: unknown name",
        );
        bad(
            "{ type = \"rect\", center = [0, 0], size = [\"10 um\", \"w\"] },",
            "{ type = \"rectangle\", center = [0, 0], size = [\"10 um\", \"w\"] },",
            "[[task.shape]] #1 (difference), of 1: unknown type \"rectangle\"",
        );
        bad(
            "convex = \"100 nm\"\nconcave = \"200 nm\"\n",
            "",
            "needs convex or concave",
        );
    }
}
