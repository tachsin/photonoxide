//! `[[task.shape]]`: the shapes defined by functions, and masks, in `"structure"`, `"fdfd"` and
//! `"fdtd"` jobs ([`crate::geometry::Between`], [`crate::geometry::Polar`],
//! [`crate::geometry::ClosedCurve`], [`crate::geometry::Implicit`], [`crate::geometry::Mask`]).
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
//! Each takes an `etch` as the other shapes do. The resolver ([`super::params`]) puts the task's
//! parameters' definitions (and a sweep's point) in each entry as `parameters` (and `set`), so
//! that its expressions are parsed in their scope here.

use serde::Deserialize;

use super::{EtchSpec, task_error};
use crate::Result;
use crate::expr::{Expr, Parameters};
use crate::geometry::{Between, Bounds, ClosedCurve, Implicit, Mask, Point, Polar, Shape};
use crate::stack::Structure;
use crate::units::Length;

/// A `[[task.shape]]`. See the module.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ShapeSpec {
    #[serde(rename = "type")]
    kind: String,
    pub(super) layer: String,
    center_um: Option<[f64; 2]>,
    x_um: Option<[f64; 2]>,
    y_um: Option<[f64; 2]>,
    lower: Option<String>,
    upper: Option<String>,
    radius: Option<String>,
    curve: Option<[String; 2]>,
    inside: Option<String>,
    cells: Option<usize>,
    corner_um: Option<[f64; 2]>,
    pixel_um: Option<[f64; 2]>,
    rows: Option<Vec<Vec<f64>>>,
    pub(super) etch: Option<EtchSpec>,
    /// The task's parameters, (name, definition): put here by the resolver.
    #[serde(default)]
    parameters: Vec<[String; 2]>,
    /// A sweep's parameter and its value at this point: put here by the resolver.
    set: Option<(String, f64)>,
}

/// The fields each type takes besides `type`, `layer` and `etch`.
const FIELDS: [(&str, &[&str]); 5] = [
    ("between", &["x", "lower", "upper"]),
    ("polar", &["center", "radius"]),
    ("parametric", &["curve"]),
    ("implicit", &["inside", "x", "y", "cells"]),
    ("mask", &["corner", "pixel", "rows"]),
];

impl ShapeSpec {
    /// The shape, its errors naming the table (`k` from 0), its type and the field.
    pub(super) fn shape(&self, k: usize) -> Result<Shape> {
        let what = format!("[[task.shape]] #{} ({})", k + 1, self.kind);
        let at = |m: String| task_error(format!("{what}: {m}"));
        let Some((_, takes)) = FIELDS.iter().find(|(t, _)| *t == self.kind) else {
            return Err(task_error(format!(
                "[[task.shape]] #{}: unknown type \"{}\": between, polar, parametric, implicit or \
                 mask",
                k + 1,
                self.kind
            )));
        };
        let given = [
            ("center", self.center_um.is_some()),
            ("x", self.x_um.is_some()),
            ("y", self.y_um.is_some()),
            ("lower", self.lower.is_some()),
            ("upper", self.upper.is_some()),
            ("radius", self.radius.is_some()),
            ("curve", self.curve.is_some()),
            ("inside", self.inside.is_some()),
            ("cells", self.cells.is_some()),
            ("corner", self.corner_um.is_some()),
            ("pixel", self.pixel_um.is_some()),
            ("rows", self.rows.is_some()),
        ];
        if let Some((name, _)) = given.iter().find(|(n, g)| *g && !takes.contains(n)) {
            return Err(at(format!(
                "{name} isn't for this type: it takes {}",
                takes.join(", ")
            )));
        }
        let need = |name: &str| at(format!("needs {name}"));
        let definitions: Vec<(String, String)> = self
            .parameters
            .iter()
            .map(|[n, d]| (n.clone(), d.clone()))
            .collect();
        let mut p = Parameters::new(&definitions).map_err(|e| at(e.to_string()))?;
        if let Some((name, value)) = &self.set {
            p = p.with(name, *value).map_err(|e| at(e.to_string()))?;
        }
        let parse = |field: &str, text: &Option<String>| -> Result<Expr> {
            let text = text.as_deref().ok_or_else(|| need(field))?;
            Expr::parse(text, p.scope()).map_err(|e| at(format!("{field}: {e}")))
        };
        let um2 = |v: [f64; 2]| [Length::um(v[0]), Length::um(v[1])];
        let reason = |e: crate::Error| match e {
            crate::Error::InvalidValue { reason, .. } => at(reason),
            other => other,
        };
        Ok(match self.kind.as_str() {
            "between" => {
                let x = self.x_um.ok_or_else(|| need("x, the range of x"))?;
                Shape::Between(
                    Between::new(
                        &parse("lower", &self.lower)?,
                        &parse("upper", &self.upper)?,
                        um2(x),
                        &p,
                    )
                    .map_err(reason)?,
                )
            }
            "polar" => {
                let c = self.center_um.unwrap_or_default();
                Shape::Polar(
                    Polar::new(Point::um(c[0], c[1]), &parse("radius", &self.radius)?, &p)
                        .map_err(reason)?,
                )
            }
            "parametric" => {
                let [x, y] = self
                    .curve
                    .as_ref()
                    .ok_or_else(|| need("curve, [x(t), y(t)]"))?;
                Shape::ClosedCurve(
                    ClosedCurve::new(
                        &parse("curve x", &Some(x.clone()))?,
                        &parse("curve y", &Some(y.clone()))?,
                        &p,
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
                Shape::Implicit(Implicit::with_cells(&f, window, cells, &p).map_err(reason)?)
            }
            _ => {
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
        })
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
}
