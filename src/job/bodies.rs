//! `[[task.body]]`: bodies in space placed over a structure ([`crate::geometry::Body`],
//! [`crate::stack::Structure::place`]), in `"structure"` jobs and 3D `"fdtd"` jobs, each filled
//! with a catalogue material or a graded medium.
//!
//! A body's `type` says which; its places and sizes are expressions with units of the task's
//! parameters (or numbers in µm in their `_um` keys), its angles expressions with their unit:
//!
//! ```yaml
//! task:
//!   body:
//!     - type: sphere                # also ellipsoid (semi_axes, angle about z), box (center,
//!       center: [0, 0, 1.11 um]     # size), cylinder (base, top, radius), cone (base, top,
//!       radius: 300 nm              # radii at each)
//!       material: si                # a catalogue id or table, as a layer's
//!
//!     - type: extrusion             # a shape between two heights, its walls at a sidewall angle
//!       z: [0, 220 nm]
//!       sidewall: 80 deg            # 90 deg (vertical) when not given
//!       of: [{ type: circle, center: [0, 0], radius: 1 um }]
//!       index: "1.45 + 0.05*exp(-(x/2um)^2 - (z/0.5um)^2)"   # a graded medium: n(x, y, z)
//!
//!     - type: difference            # also union, intersection, xor: of two bodies or more,
//!       material: sio2              # the first less the others; theirs have no medium of their own
//!       of:
//!         - { type: box, center: [0, 0, 0], size: [4 um, 4 um, 1 um] }
//!         - { type: cylinder, base: [0, 0, -1 um], top: [0, 0, 1 um], radius: 0.5 um }
//! ```
//!
//! A body's medium is `material`, or `index` (n) or `permittivity` (ε): an expression of x, y
//! and z with their units and of the parameters, a plain number, constant or graded
//! ([`crate::material::Graded`]). Each body lies over the stack, its shapes and the bodies
//! before it. [`super::check`] refuses a body that isn't one, naming the table and field; a run
//! refuses a graded medium where it isn't positive and finite at the grid's points.

use toml::{Table, Value};

use super::params::{self, Want};
use super::shapes::ShapeSpec;
use super::task_error;
use crate::Result;
use crate::device::spec::MaterialSpec;
use crate::expr::{Expr, Parameters};
use crate::geometry::{Body, Operation};
use crate::material::Graded;
use crate::stack::{Medium, Structure};

/// The fields each type takes besides `type`, `name` and the medium's.
const FIELDS: [(&str, &[&str]); 11] = [
    ("sphere", &["center", "radius"]),
    ("ellipsoid", &["center", "semi_axes", "angle"]),
    ("box", &["center", "size"]),
    ("cylinder", &["base", "top", "radius"]),
    ("cone", &["base", "top", "radii"]),
    ("extrusion", &["z", "sidewall", "of"]),
    ("union", &["of"]),
    ("intersection", &["of"]),
    ("difference", &["of"]),
    ("xor", &["of"]),
    ("", &[]),
];

/// The keys every body takes, and those the resolver puts in.
const COMMON: [&str; 7] = [
    "type",
    "name",
    "material",
    "index",
    "permittivity",
    "parameters",
    "set",
];

/// A body's table read: its fields at the task's parameters, `what` naming it in errors.
struct Fields<'a> {
    table: &'a Table,
    p: &'a Parameters,
    what: String,
}

impl Fields<'_> {
    fn err(&self, field: &str, m: impl std::fmt::Display) -> crate::Error {
        task_error(format!("{}, {field}: {m}", self.what))
    }

    /// `count` lengths, µm: `field`, expressions with units, or `field_um`, numbers in µm (or
    /// expressions read in µm).
    fn lengths(&self, field: &str, count: usize) -> Result<Option<Vec<f64>>> {
        let key = format!("{field}_um");
        let (name, value, want) = match (self.table.get(field), self.table.get(&key)) {
            (Some(_), Some(_)) => {
                return Err(self.err(field, format!("give {field} or {key}, not both")));
            }
            (Some(v), None) => (field, v, Want::Length),
            (None, Some(v)) => (key.as_str(), v, Want::Micrometres),
            (None, None) => return Ok(None),
        };
        let items: Vec<&Value> = match value {
            Value::Array(items) if count > 1 || items.len() == 1 => items.iter().collect(),
            single if count == 1 => vec![single],
            _ => {
                return Err(self.err(name, format!("needs {count} values")));
            }
        };
        if items.len() != count {
            return Err(self.err(name, format!("needs {count} values, got {}", items.len())));
        }
        items
            .iter()
            .enumerate()
            .map(|(k, v)| {
                params::evaluate(v, self.p, want).map_err(|m| {
                    if count == 1 {
                        self.err(name, m)
                    } else {
                        self.err(name, format!("value {}: {m}", k + 1))
                    }
                })
            })
            .collect::<Result<Vec<f64>>>()
            .map(Some)
    }

    fn need(&self, field: &str, count: usize) -> Result<Vec<f64>> {
        self.lengths(field, count)?.ok_or_else(|| {
            self.err(
                field,
                if count == 1 {
                    "is needed: a length".to_owned()
                } else {
                    format!("is needed: {count} lengths")
                },
            )
        })
    }

    fn point(&self, field: &str) -> Result<[f64; 3]> {
        let v = self.need(field, 3)?;
        Ok([v[0], v[1], v[2]])
    }

    /// An angle, radians: an expression with its unit, or degrees in `field_deg`.
    fn angle(&self, field: &str) -> Result<Option<f64>> {
        let key = format!("{field}_deg");
        match (self.table.get(field), self.table.get(&key)) {
            (Some(_), Some(_)) => Err(self.err(field, format!("give {field} or {key}, not both"))),
            (Some(v), None) => params::evaluate(v, self.p, Want::Angle)
                .map(Some)
                .map_err(|m| self.err(field, m)),
            (None, Some(v)) => params::evaluate(v, self.p, Want::Like(None))
                .map(|d| {
                    Some(if d == 90.0 {
                        std::f64::consts::FRAC_PI_2
                    } else {
                        d.to_radians()
                    })
                })
                .map_err(|m| self.err(&key, m)),
            (None, None) => Ok(None),
        }
    }
}

/// The parameters the resolver put in `table`: the task's, at a sweep's point.
fn parameters_of(table: &Table, what: &str) -> Result<Parameters> {
    let mut definitions = Vec::new();
    if let Some(Value::Array(list)) = table.get("parameters") {
        for d in list {
            if let Value::Array(pair) = d
                && let [Value::String(n), Value::String(e)] = pair.as_slice()
            {
                definitions.push((n.clone(), e.clone()));
            }
        }
    }
    let mut p = Parameters::new(&definitions).map_err(|e| task_error(format!("{what}: {e}")))?;
    if let Some(Value::Array(set)) = table.get("set")
        && let [Value::String(n), v] = set.as_slice()
        && let Some(v) = v.as_float()
    {
        p = p
            .with(n, v)
            .map_err(|e| task_error(format!("{what}: {e}")))?;
    }
    Ok(p)
}

/// The body `table` describes, `what` naming it in errors, at the parameters `p`.
fn body_of(table: &Table, p: &Parameters, what: &str) -> Result<Body> {
    let kind = match table.get("type") {
        Some(Value::String(k)) => k.as_str(),
        _ => {
            return Err(task_error(format!(
                "{what}: needs type: sphere, ellipsoid, box, cylinder, cone, extrusion, union, \
                 intersection, difference or xor"
            )));
        }
    };
    let Some((_, takes)) = FIELDS.iter().find(|(t, _)| *t == kind && !t.is_empty()) else {
        return Err(task_error(format!(
            "{what}: unknown type \"{kind}\": sphere, ellipsoid, box, cylinder, cone, extrusion, \
             union, intersection, difference or xor"
        )));
    };
    let what = format!("{what} ({kind})");
    for key in table.keys() {
        let base = key
            .strip_suffix("_um")
            .or_else(|| key.strip_suffix("_deg"))
            .unwrap_or(key);
        if !COMMON.contains(&key.as_str()) && !takes.contains(&base) {
            return Err(task_error(format!(
                "{what}: {key} isn't for this type: it takes {}",
                takes.join(", ")
            )));
        }
    }
    let f = Fields {
        table,
        p,
        what: what.clone(),
    };
    let reason = |e: crate::Error| match e {
        crate::Error::InvalidValue { reason, .. } => task_error(format!("{what}: {reason}")),
        other => other,
    };
    let body = match kind {
        "sphere" => Body::sphere(f.point("center")?, f.need("radius", 1)?[0]).map_err(reason)?,
        "ellipsoid" => {
            let c = f.point("center")?;
            let s = f.need("semi_axes", 3)?;
            let (sin, cos) = f.angle("angle")?.unwrap_or(0.0).sin_cos();
            Body::ellipsoid(
                c,
                [s[0], s[1], s[2]],
                [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
            )
            .map_err(reason)?
        }
        "box" => {
            let (c, s) = (f.point("center")?, f.point("size")?);
            Body::cuboid(
                std::array::from_fn(|k| c[k] - 0.5 * s[k]),
                std::array::from_fn(|k| c[k] + 0.5 * s[k]),
            )
            .map_err(reason)?
        }
        "cylinder" => Body::cylinder(f.point("base")?, f.point("top")?, f.need("radius", 1)?[0])
            .map_err(reason)?,
        "cone" => {
            let r = f.need("radii", 2)?;
            Body::cone(f.point("base")?, f.point("top")?, [r[0], r[1]]).map_err(reason)?
        }
        "extrusion" => {
            let z = f.need("z", 2)?;
            let sidewall = f.angle("sidewall")?.unwrap_or(std::f64::consts::FRAC_PI_2);
            let shapes = of(table, &what)?;
            let [shape] = shapes.as_slice() else {
                return Err(f.err("of", "needs one shape: the outline extruded"));
            };
            let spec: ShapeSpec = Value::Table(shape.clone())
                .try_into()
                .map_err(|e: toml::de::Error| f.err("of", e.to_string()))?;
            let outline = spec.inner(p, &format!("{what}, of 1"))?;
            Body::etched(outline, [z[0], z[1]], sidewall).map_err(reason)?
        }
        _ => {
            let op = match kind {
                "union" => Operation::Union,
                "intersection" => Operation::Intersection,
                "difference" => Operation::Difference,
                _ => Operation::Xor,
            };
            let parts = of(table, &what)?;
            if parts.len() < 2 {
                return Err(f.err("of", "needs two bodies or more"));
            }
            let mut bodies = Vec::with_capacity(parts.len());
            for (j, part) in parts.iter().enumerate() {
                let inner = format!("{what}, of {}", j + 1);
                for key in ["material", "index", "permittivity"] {
                    if part.contains_key(key) {
                        return Err(task_error(format!(
                            "{inner}: {key}: a body in another's of has no medium of its own"
                        )));
                    }
                }
                bodies.push(body_of(part, p, &inner)?);
            }
            let mut it = bodies.into_iter();
            let first = it
                .next()
                .ok_or_else(|| f.err("of", "needs two bodies or more"))?;
            it.fold(first, |acc, b| Body::boolean(op, acc, b))
        }
    };
    Ok(body)
}

/// The tables in `table`'s `of`.
fn of(table: &Table, what: &str) -> Result<Vec<Table>> {
    match table.get("of") {
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(j, v)| match v {
                Value::Table(t) => Ok(t.clone()),
                _ => Err(task_error(format!("{what}, of {}: must be a table", j + 1))),
            })
            .collect(),
        Some(_) => Err(task_error(format!("{what}, of: must be a list of tables"))),
        None => Err(task_error(format!("{what}, of: is needed"))),
    }
}

/// A body's medium: `material`, a catalogue id or table, or `index` or `permittivity`, an
/// expression of x, y and z (a graded medium, or a constant one).
fn medium_of(table: &Table, p: &Parameters, what: &str) -> Result<Medium> {
    let given: Vec<&str> = ["material", "index", "permittivity"]
        .into_iter()
        .filter(|k| table.contains_key(*k))
        .collect();
    let at = |key: &str, m: String| task_error(format!("{what}, {key}: {m}"));
    match given[..] {
        ["material"] => {
            let spec = MaterialSpec::from_value(&table["material"], &format!("{what}, material"))?;
            Ok(Medium::Material(
                spec.material().map_err(|e| at("material", e.to_string()))?,
            ))
        }
        [key @ ("index" | "permittivity")] => {
            let text = match &table[key] {
                Value::String(s) => s.clone(),
                Value::Float(x) => x.to_string(),
                Value::Integer(n) => n.to_string(),
                other => {
                    return Err(at(
                        key,
                        format!("must be an expression or a number, got {other}"),
                    ));
                }
            };
            let e = Expr::parse(&text, p.scope()).map_err(|e| at(key, e.to_string()))?;
            let g = if key == "index" {
                Graded::index(&e, p)
            } else {
                Graded::permittivity(&e, p)
            }
            .map_err(|e| match e {
                crate::Error::InvalidValue { reason, .. } => at(key, reason),
                other => other,
            })?;
            Ok(Medium::Graded(g))
        }
        [] => Err(task_error(format!(
            "{what}: needs its medium: material (a catalogue id), or index or permittivity (an \
             expression of x, y and z)"
        ))),
        _ => Err(task_error(format!(
            "{what}: give one medium: material, index or permittivity, not {}",
            given.join(" and ")
        ))),
    }
}

/// `structure` with every body of `entries` placed over it, in order.
pub(super) fn place(mut structure: Structure, entries: &[Table]) -> Result<Structure> {
    for (k, table) in entries.iter().enumerate() {
        let what = format!("[[task.body]] #{}", k + 1);
        let p = parameters_of(table, &what)?;
        let body = body_of(table, &p, &what)?;
        let kind = table.get("type").and_then(Value::as_str).unwrap_or("");
        let medium = medium_of(table, &p, &format!("{what} ({kind})"))?;
        structure.place(body, medium);
    }
    Ok(structure)
}

/// A body's centre for the scene's medium: its bounds' middle along each axis they bound.
pub(super) fn centre_of(body: &Body) -> [f64; 3] {
    let b = body.bounds();
    std::array::from_fn(|k| {
        let (lo, hi) = (b.min[k], b.max[k]);
        if lo.is_finite() && hi.is_finite() {
            0.5 * (lo + hi)
        } else if lo.is_finite() {
            lo
        } else if hi.is_finite() {
            hi
        } else {
            0.0
        }
    })
}
