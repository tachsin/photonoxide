//! `[[task.component]]`: components of [`crate::generator`] in `"structure"`, `"fdfd"` and
//! `"fdtd"` jobs, placed and named, and their ports found by name.
//!
//! A component's `type` names its generator, and its fields are the generator's parameters,
//! each an expression of the task's parameters with its unit (`"500 nm"`, `"w + gap"`, `"90
//! deg"`), or a number in a `_um` (length) or `_deg` (angle) key:
//!
//! ```toml
//! [[task.component]]
//! type = "ring"
//! name = "ring"               # its ports are ring.in, ring.through (ring.add, ring.drop)
//! layer = "Si"                # the task's layer by default
//! width = "w"
//! radius = "5 um"
//! gap = "gap"
//! origin = ["0 um", "0 um"]   # where its frame's origin goes (optional)
//! rotate = "90 deg"           # turned about it (optional); mirror = true reflects it first
//!
//! [[task.component]]
//! type = "straight"
//! name = "lead"
//! width = "w"
//! length = "3 um"
//! connect = ["o1", "ring.through"]   # its o1 put on ring.through, facing it
//! ```
//!
//! The types and their fields (lengths but where said; `?` optional):
//!
//! - `straight`: `width`, `length`;
//! - `taper`: `width_in`, `width_out`, `length`, `profile?` (`"linear"`, `"exponential"`,
//!   `"parabolic"`), or `width`, w(x), an expression of `x`, for a custom one;
//! - `bend`: `width`, `radius`, `angle` (an angle), `p?` (a number: 0 an arc, 1 an Euler bend);
//!   `euler_bend`: the same, `p` 1 by default;
//! - `s_bend`: `width`, `length`, `offset`, `shape?` (`"cosine"`, `"sine"`, `"euler"`,
//!   `"bezier"`), `p?` (for `"euler"`);
//! - `spiral`: `width`, `pitch`, `inner_radius`, `turns` (a whole number), `straight?`, `lead?`;
//! - `ring` and `racetrack`: `width`, `radius`, `gap`, `coupling_length` (a racetrack's),
//!   `drop_gap?`, `reach?`; `disk`: `width`, `radius`, `gap`, `drop_gap?`, `reach?`;
//! - `directional_coupler`: `width`, `gap`, `length`, `arm_length?`, `arm_offset?`,
//!   `arm_shape?`; `bent_coupler`: `width`, `gap`, `radius`, `angle`;
//! - `mmi`: `width`, `length`, `inputs`, `outputs` (whole numbers), `access_width`,
//!   `taper_width?`, `taper_length?`, `input_positions?`, `output_positions?` (lists);
//! - `y_branch`: `input_width`, `width`, `length`, `arm_length`, `separation`, `gap?`,
//!   `arm_shape?`, `outline?` (w(x));
//! - `crossing`: `width`, `arm`, `centre_width?`;
//! - `bragg_grating`: `width`, `corrugation`, `period`, `periods`, `duty?` (a number, 0.5),
//!   `centred?` (true or false), `apodization?` (Δw(x), a length), `sampling?` (`[burst,
//!   every]` periods);
//! - `swg_waveguide`: `width`, `period`, `duty`, `periods`, `width_end?`, `bridge?`;
//! - `grating_coupler`: `width`, `grating_width`, `taper_length`, `period`, `periods`,
//!   `etch_depth`, `fill?` (0.5) or `apodization?` (F(x)), `slab_layer?`;
//! - `photonic_crystal`: `pitch`, `radius`, `half` (`[i, j]`), `lattice?` (`"hexagonal"`,
//!   `"square"`), `defect?` (`"none"`, `"l3"`, `"h1"`, `"w1"`), `shift?` (an L3's);
//! - `edge_coupler`: `tip_width`, `width`, `taper_length`, `straight?`, `profile?`.
//!
//! **Ports by name.** An `"fdfd"` job's `[[task.port]]` takes `port = "ring.in"` in place of its
//! `x_um` and `side`: its column at the port, its side the one light comes in from (a port must
//! face 0° or 180°), and its window across, unless given, the guide's width and 1 µm each side,
//! but no closer than half way to another port in the same column, inside the job's window.
//! An `"fdtd"` job's `"mode"` source and monitor take `port` in place of `normal`, `at_um` and a
//! direction (light goes into the component from a source) the same way, on planes normal to x
//! or y, and a monitor is named after its port unless named.

use toml::{Table, Value};

use super::params::{self, Want};
use super::{EtchSpec, task_error};
use crate::Result;
use crate::expr::{Expr, Parameters};
use crate::generator::{
    Bend, BentCoupler, BraggGrating, Crossing, CrystalLattice, Defect, DirectionalCoupler, Disk,
    Drawing, EdgeCoupler, Generator, GratingCoupler, Mmi, PhotonicCrystal, Port, Ring, SBend,
    SBendKind, Spiral, Straight, SwgWaveguide, Taper, TaperProfile, YBranch, rotation,
};
use crate::geometry::{Point, Transform};
use crate::stack::Structure;
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

/// The types and the fields each takes besides `type`, `name`, `layer`, `etch`, `origin`,
/// `rotate`, `mirror` and `connect`.
const TYPES: [(&str, &[&str]); 19] = [
    ("straight", &["width", "length"]),
    (
        "taper",
        &["width_in", "width_out", "length", "profile", "width"],
    ),
    ("bend", &["width", "radius", "angle", "p"]),
    ("euler_bend", &["width", "radius", "angle", "p"]),
    ("s_bend", &["width", "length", "offset", "shape", "p"]),
    (
        "spiral",
        &[
            "width",
            "pitch",
            "inner_radius",
            "turns",
            "straight",
            "lead",
        ],
    ),
    (
        "ring",
        &[
            "width",
            "radius",
            "gap",
            "coupling_length",
            "drop_gap",
            "reach",
        ],
    ),
    (
        "racetrack",
        &[
            "width",
            "radius",
            "gap",
            "coupling_length",
            "drop_gap",
            "reach",
        ],
    ),
    ("disk", &["width", "radius", "gap", "drop_gap", "reach"]),
    (
        "directional_coupler",
        &[
            "width",
            "gap",
            "length",
            "arm_length",
            "arm_offset",
            "arm_shape",
        ],
    ),
    ("bent_coupler", &["width", "gap", "radius", "angle"]),
    (
        "mmi",
        &[
            "width",
            "length",
            "inputs",
            "outputs",
            "access_width",
            "taper_width",
            "taper_length",
            "input_positions",
            "output_positions",
        ],
    ),
    (
        "y_branch",
        &[
            "input_width",
            "width",
            "length",
            "gap",
            "arm_length",
            "separation",
            "arm_shape",
            "outline",
        ],
    ),
    ("crossing", &["width", "arm", "centre_width"]),
    (
        "bragg_grating",
        &[
            "width",
            "corrugation",
            "period",
            "duty",
            "periods",
            "centred",
            "apodization",
            "sampling",
        ],
    ),
    (
        "swg_waveguide",
        &["width", "width_end", "period", "duty", "periods", "bridge"],
    ),
    (
        "grating_coupler",
        &[
            "width",
            "grating_width",
            "taper_length",
            "period",
            "fill",
            "periods",
            "etch_depth",
            "apodization",
            "slab_layer",
        ],
    ),
    (
        "photonic_crystal",
        &["lattice", "pitch", "radius", "half", "defect", "shift"],
    ),
    (
        "edge_coupler",
        &["tip_width", "width", "taper_length", "straight", "profile"],
    ),
];

/// The keys every component takes.
const COMMON: [&str; 8] = [
    "type", "name", "layer", "etch", "origin", "rotate", "mirror", "connect",
];

/// The keys the resolver puts in: the task's parameters, and a sweep's point.
const RESOLVED: [&str; 2] = ["parameters", "set"];

/// A component drawn and placed.
pub(super) struct Placed {
    /// Its name, if it has one.
    pub name: Option<String>,
    /// Its shapes and ports, placed.
    pub drawing: Drawing,
    /// Its etch.
    pub etch: Option<EtchSpec>,
}

/// A component table's fields, read as its type takes them; `what` names it in errors.
struct Fields<'a> {
    table: &'a Table,
    p: &'a Parameters,
    what: String,
}

impl Fields<'_> {
    fn err(&self, field: &str, m: impl std::fmt::Display) -> crate::Error {
        task_error(format!("{}, {field}: {m}", self.what))
    }

    /// The value of `field` (an expression) or of its number key `field` + `suffix`, as `want`
    /// and `loose` (the number key's) say.
    fn value(&self, field: &str, suffix: &str, want: Want, loose: Want) -> Result<Option<f64>> {
        let key = format!("{field}{suffix}");
        match (self.table.get(field), self.table.get(&key)) {
            (Some(_), Some(_)) => Err(self.err(field, format!("give {field} or {key}, not both"))),
            (Some(v), None) => params::evaluate(v, self.p, want)
                .map(Some)
                .map_err(|m| self.err(field, m)),
            (None, Some(v)) => params::evaluate(v, self.p, loose)
                .map(Some)
                .map_err(|m| self.err(&key, m)),
            (None, None) => Ok(None),
        }
    }

    /// A length, µm.
    fn length(&self, field: &str) -> Result<Option<Length>> {
        Ok(self
            .value(field, "_um", Want::Length, Want::Micrometres)?
            .map(Length::um))
    }

    fn need_length(&self, field: &str) -> Result<Length> {
        self.length(field)?
            .ok_or_else(|| self.err(field, "is needed: a length"))
    }

    /// An angle, degrees: an expression with its unit, or degrees in `field_deg`.
    fn angle(&self, field: &str) -> Result<Option<f64>> {
        let key = format!("{field}_deg");
        match (self.table.get(field), self.table.get(&key)) {
            (Some(_), Some(_)) => Err(self.err(field, format!("give {field} or {key}, not both"))),
            (Some(v), None) => params::evaluate(v, self.p, Want::Angle)
                .map(|rad| Some(degrees_of(rad)))
                .map_err(|m| self.err(field, m)),
            (None, Some(v)) => params::evaluate(v, self.p, Want::Like(None))
                .map(Some)
                .map_err(|m| self.err(&key, m)),
            (None, None) => Ok(None),
        }
    }

    fn need_angle(&self, field: &str) -> Result<f64> {
        self.angle(field)?
            .ok_or_else(|| self.err(field, "is needed: an angle, e.g. \"90 deg\""))
    }

    /// A plain number.
    fn number(&self, field: &str) -> Result<Option<f64>> {
        self.table
            .get(field)
            .map(|v| params::evaluate(v, self.p, Want::Number).map_err(|m| self.err(field, m)))
            .transpose()
    }

    /// A whole number, at least `least`.
    fn count(&self, field: &str, least: usize) -> Result<usize> {
        let n = self
            .number(field)?
            .ok_or_else(|| self.err(field, "is needed: a whole number"))?;
        if !(n.fract() == 0.0 && n >= least as f64 && n <= 1e6) {
            return Err(self.err(
                field,
                format!("must be a whole number, at least {least}, got {n}"),
            ));
        }
        Ok(n as usize)
    }

    /// A word.
    fn word(&self, field: &str) -> Result<Option<&str>> {
        match self.table.get(field) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(other) => Err(self.err(field, format!("must be a string, got {other}"))),
        }
    }

    /// An expression of x (a curve's), parsed in the parameters' scope.
    fn of_x(&self, field: &str) -> Result<Option<(Expr, Parameters)>> {
        let Some(text) = self.word(field)? else {
            return Ok(None);
        };
        let e = Expr::parse(text, self.p.scope()).map_err(|e| self.err(field, e))?;
        Ok(Some((e, self.p.clone())))
    }

    /// A list of lengths.
    fn lengths(&self, field: &str) -> Result<Option<Vec<Length>>> {
        let Some(v) = self.table.get(field) else {
            return Ok(None);
        };
        let Value::Array(items) = v else {
            return Err(self.err(field, "must be a list of lengths"));
        };
        items
            .iter()
            .enumerate()
            .map(|(k, x)| {
                params::evaluate(x, self.p, Want::Length)
                    .map(Length::um)
                    .map_err(|m| self.err(field, format!("value {}: {m}", k + 1)))
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    }

    /// An S-bend's shape from `field` (and `p`).
    fn shape(&self, field: &str, default: SBendKind) -> Result<SBendKind> {
        Ok(match self.word(field)? {
            None => default,
            Some("cosine") => SBendKind::Cosine,
            Some("sine") => SBendKind::Sine,
            Some("bezier") => SBendKind::Bezier,
            Some("euler") => SBendKind::Euler {
                p: self.number("p")?.unwrap_or(1.0),
            },
            Some(other) => {
                return Err(self.err(
                    field,
                    format!("unknown shape \"{other}\": cosine, sine, euler or bezier"),
                ));
            }
        })
    }

    /// A taper's profile from `profile`, linear by default.
    fn profile(&self) -> Result<TaperProfile> {
        Ok(match self.word("profile")?.unwrap_or("linear") {
            "linear" => TaperProfile::Linear,
            "exponential" => TaperProfile::Exponential,
            "parabolic" => TaperProfile::Parabolic,
            other => {
                return Err(self.err(
                    "profile",
                    format!(
                        "unknown profile \"{other}\": linear, exponential or parabolic \
                         (or give width, w(x), for one of your own)"
                    ),
                ));
            }
        })
    }

    /// Whether `field`, true or false, is true (false when left out).
    fn flag(&self, field: &str) -> Result<bool> {
        match self.table.get(field) {
            None => Ok(false),
            Some(Value::Boolean(b)) => Ok(*b),
            Some(other) => Err(self.err(field, format!("must be true or false, got {other}"))),
        }
    }

    /// Two whole numbers, `[a, b]`.
    fn pair(&self, field: &str) -> Result<Option<[usize; 2]>> {
        let Some(v) = self.table.get(field) else {
            return Ok(None);
        };
        match v.as_array().map(Vec::as_slice) {
            Some([Value::Integer(a), Value::Integer(b)]) if *a >= 0 && *b >= 0 => {
                Ok(Some([*a as usize, *b as usize]))
            }
            _ => Err(self.err(field, format!("must be two whole numbers, [a, b], got {v}"))),
        }
    }
}

/// `rad` radians in degrees, a whole number of degrees when it is one to round-off (so that
/// `"90 deg"` gives a port facing 90°, as `"90 deg"` read in radians and back gives 90 + 1 ulp).
fn degrees_of(rad: f64) -> f64 {
    let d = rad.to_degrees();
    let whole = d.round();
    if (d - whole).abs() <= 4.0 * f64::EPSILON * whole.abs().max(1.0) {
        whole
    } else {
        d
    }
}

/// The component in `table` (k from 0), its generator and parameters read, drawn on its layer.
fn generator_of(f: &Fields<'_>, kind: &str) -> Result<Box<dyn Generator>> {
    let len = |n: &str| f.need_length(n);
    let opt = |n: &str| f.length(n);
    Ok(match kind {
        "straight" => Box::new(Straight::new(len("width")?, len("length")?)),
        "taper" => match f.of_x("width")? {
            Some((e, p)) => {
                for n in ["width_in", "width_out", "profile"] {
                    if f.table.contains_key(n) || f.table.contains_key(&format!("{n}_um")) {
                        return Err(f.err(n, "isn't for a custom taper: its width w(x) gives it"));
                    }
                }
                Box::new(Taper::custom(e, p, len("length")?))
            }
            None => Box::new(
                Taper::new(len("width_in")?, len("width_out")?, len("length")?)
                    .with_profile(f.profile()?),
            ),
        },
        "bragg_grating" => {
            let mut g = BraggGrating::new(
                len("width")?,
                len("corrugation")?,
                len("period")?,
                f.count("periods", 1)?,
            );
            if let Some(d) = f.number("duty")? {
                g.duty = d;
            }
            g.centred = f.flag("centred")?;
            if let Some((e, p)) = f.of_x("apodization")? {
                g = g.with_apodization(e, p);
            }
            if let Some([burst, every]) = f.pair("sampling")? {
                g = g.sampled(burst, every);
            }
            Box::new(g)
        }
        "swg_waveguide" => {
            let duty = f
                .number("duty")?
                .ok_or_else(|| f.err("duty", "is needed: a number from 0 to 1"))?;
            let mut s =
                SwgWaveguide::new(len("width")?, len("period")?, duty, f.count("periods", 1)?);
            if let Some(w) = opt("width_end")? {
                s.width_end = w;
            }
            if let Some(b) = opt("bridge")? {
                s.bridge = b;
            }
            Box::new(s)
        }
        "grating_coupler" => {
            let apodized = f.of_x("apodization")?;
            if apodized.is_some() && f.table.contains_key("fill") {
                return Err(f.err(
                    "fill",
                    "isn't for an apodized coupler: its apodization, F(x), gives it",
                ));
            }
            let mut g = GratingCoupler::new(
                len("width")?,
                len("grating_width")?,
                len("taper_length")?,
                len("period")?,
                f.number("fill")?.unwrap_or(0.5),
                f.count("periods", 1)?,
                len("etch_depth")?,
            );
            if let Some((e, p)) = apodized {
                g = g.with_apodization(e, p);
            }
            if let Some(slab) = f.word("slab_layer")? {
                g = g.with_slab_layer(slab);
            }
            Box::new(g)
        }
        "photonic_crystal" => {
            let lattice = match f.word("lattice")?.unwrap_or("hexagonal") {
                "hexagonal" => CrystalLattice::Hexagonal,
                "square" => CrystalLattice::Square,
                other => {
                    return Err(f.err(
                        "lattice",
                        format!("unknown lattice \"{other}\": hexagonal or square"),
                    ));
                }
            };
            let defect = match f.word("defect")?.unwrap_or("none") {
                "none" => Defect::None,
                "h1" => Defect::H1,
                "w1" => Defect::W1,
                "l3" => Defect::L3 {
                    shift: opt("shift")?.unwrap_or(Length::ZERO),
                },
                other => {
                    return Err(f.err(
                        "defect",
                        format!("unknown defect \"{other}\": none, l3, h1 or w1"),
                    ));
                }
            };
            if (f.table.contains_key("shift") || f.table.contains_key("shift_um"))
                && !matches!(defect, Defect::L3 { .. })
            {
                return Err(f.err("shift", "is for an l3 cavity's end holes"));
            }
            let [ni, nj] = f.pair("half")?.ok_or_else(|| {
                f.err(
                    "half",
                    "is needed: [i, j], the sites either side of the centre",
                )
            })?;
            Box::new(PhotonicCrystal {
                lattice,
                ..PhotonicCrystal::hexagonal(
                    len("pitch")?,
                    len("radius")?,
                    [ni as i64, nj as i64],
                    defect,
                )
            })
        }
        "edge_coupler" => {
            let mut e = EdgeCoupler::new(len("tip_width")?, len("width")?, len("taper_length")?);
            if let Some(l) = opt("straight")? {
                e.straight = l;
            }
            e.profile = f.profile()?;
            Box::new(e)
        }
        "bend" | "euler_bend" => {
            let p = f
                .number("p")?
                .unwrap_or(if kind == "bend" { 0.0 } else { 1.0 });
            Box::new(Bend::euler(
                len("width")?,
                len("radius")?,
                f.need_angle("angle")?,
                p,
            ))
        }
        "s_bend" => {
            if f.table.contains_key("p") && f.word("shape")? != Some("euler") {
                return Err(f.err("p", "is for an euler S-bend"));
            }
            Box::new(SBend::new(
                len("width")?,
                len("length")?,
                len("offset")?,
                f.shape("shape", SBendKind::Cosine)?,
            ))
        }
        "spiral" => {
            let mut s = Spiral::new(
                len("width")?,
                len("pitch")?,
                len("inner_radius")?,
                f.count("turns", 1)?,
            );
            if let Some(l) = opt("straight")? {
                s.straight = l;
            }
            if let Some(l) = opt("lead")? {
                s.lead = l;
            }
            Box::new(s)
        }
        "ring" | "racetrack" => {
            let lc = match (kind, opt("coupling_length")?) {
                ("racetrack", None) => return Err(f.err("coupling_length", "is needed: a length")),
                (_, l) => l.unwrap_or(Length::ZERO),
            };
            let mut r = Ring::racetrack(len("width")?, len("radius")?, len("gap")?, lc);
            if let Some(g) = opt("drop_gap")? {
                r = r.with_drop(g);
            }
            if let Some(l) = opt("reach")? {
                r = r.with_reach(l);
            }
            Box::new(r)
        }
        "disk" => {
            let mut d = Disk::new(len("width")?, len("radius")?, len("gap")?);
            if let Some(g) = opt("drop_gap")? {
                d = d.with_drop(g);
            }
            if let Some(l) = opt("reach")? {
                d = d.with_reach(l);
            }
            Box::new(d)
        }
        "directional_coupler" => {
            let mut c = DirectionalCoupler::new(len("width")?, len("gap")?, len("length")?);
            if let Some(la) = opt("arm_length")? {
                c = c.with_arms(
                    la,
                    len("arm_offset")?,
                    f.shape("arm_shape", SBendKind::Cosine)?,
                );
            } else if f.table.contains_key("arm_offset") || f.table.contains_key("arm_shape") {
                return Err(f.err("arm_length", "is needed with arm_offset or arm_shape"));
            }
            Box::new(c)
        }
        "bent_coupler" => Box::new(BentCoupler::new(
            len("width")?,
            len("gap")?,
            len("radius")?,
            f.need_angle("angle")?,
        )),
        "mmi" => {
            let mut m = Mmi::new(
                len("width")?,
                len("length")?,
                f.count("inputs", 1)?,
                f.count("outputs", 1)?,
                len("access_width")?,
            );
            if let Some(w) = opt("taper_width")? {
                m.taper_width = w;
            }
            if let Some(l) = opt("taper_length")? {
                m.taper_length = l;
            }
            m.input_positions = f.lengths("input_positions")?;
            m.output_positions = f.lengths("output_positions")?;
            Box::new(m)
        }
        "y_branch" => {
            let mut y = YBranch::new(
                match f.of_x("outline")? {
                    Some(_) => opt("input_width")?.unwrap_or(Length::ZERO),
                    None => len("input_width")?,
                },
                len("width")?,
                len("length")?,
                len("arm_length")?,
                len("separation")?,
            );
            if let Some(g) = opt("gap")? {
                y.gap = g;
            }
            y.arm_kind = f.shape("arm_shape", SBendKind::Bezier)?;
            if let Some((e, p)) = f.of_x("outline")? {
                if opt("input_width")?.is_some() {
                    return Err(f.err(
                        "input_width",
                        "isn't for a Y-branch with an outline: its w(0) gives it",
                    ));
                }
                y = y.with_outline(e, p);
            }
            Box::new(y)
        }
        "crossing" => {
            let mut c = Crossing::new(len("width")?, len("arm")?);
            if let Some(w) = opt("centre_width")? {
                c.centre_width = w;
            }
            Box::new(c)
        }
        _ => unreachable!("the types are checked before"),
    })
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

/// Every component of `entries`, drawn and placed in order (a `connect` meets an earlier one),
/// at the parameters `p` (`None`: those each entry holds, as the resolver left them).
pub(super) fn build_all(entries: &[Table], p: Option<&Parameters>) -> Result<Vec<Placed>> {
    let mut placed: Vec<Placed> = Vec::with_capacity(entries.len());
    for (k, table) in entries.iter().enumerate() {
        let kind = table.get("type").and_then(Value::as_str).unwrap_or("");
        let name = match table.get("name") {
            None => None,
            Some(Value::String(n)) => Some(n.clone()),
            Some(other) => {
                return Err(task_error(format!(
                    "[[task.component]] #{}: name must be a string, got {other}",
                    k + 1
                )));
            }
        };
        let what = match &name {
            Some(n) => format!("[[task.component]] #{} ({kind} \"{n}\")", k + 1),
            None => format!("[[task.component]] #{} ({kind})", k + 1),
        };
        let Some((_, takes)) = TYPES.iter().find(|(t, _)| *t == kind) else {
            let known: Vec<&str> = TYPES.iter().map(|(t, _)| *t).collect();
            return Err(task_error(format!(
                "[[task.component]] #{}: unknown type \"{kind}\": {}",
                k + 1,
                known.join(", ")
            )));
        };
        if let Some(n) = &name {
            if n.is_empty() || n.contains('.') || n.contains(char::is_whitespace) {
                return Err(task_error(format!(
                    "{what}: a name is a word without dots or spaces, as a port is named \
                     name.port"
                )));
            }
            if placed.iter().any(|q| q.name.as_deref() == Some(n.as_str())) {
                return Err(task_error(format!(
                    "{what}: there is already a component named \"{n}\""
                )));
            }
        }
        for key in table.keys() {
            let bare = key
                .strip_suffix("_um")
                .or_else(|| key.strip_suffix("_deg"))
                .unwrap_or(key);
            if !(COMMON.contains(&key.as_str())
                || RESOLVED.contains(&key.as_str())
                || takes.contains(&bare))
            {
                return Err(task_error(format!(
                    "{what}: {key} isn't for a {kind}: it takes {}",
                    takes.join(", ")
                )));
            }
        }
        let own;
        let p = match p {
            Some(p) => p,
            None => {
                own = parameters_of(table, &what)?;
                &own
            }
        };
        let f = Fields {
            table,
            p,
            what: what.clone(),
        };
        let layer = f
            .word("layer")?
            .ok_or_else(|| task_error(format!("{what}: needs layer, the layer it is drawn on")))?;
        let generator = generator_of(&f, kind)?;
        let drawing = generator.draw(layer).map_err(|e| match e {
            crate::Error::InvalidValue { reason, .. } => task_error(format!("{what}: {reason}")),
            other => other,
        })?;
        let mirrored = match table.get("mirror") {
            None => false,
            Some(Value::Boolean(b)) => *b,
            Some(other) => {
                return Err(f.err("mirror", format!("must be true or false, got {other}")));
            }
        };
        let drawing = match table.get("connect") {
            Some(c) => {
                if table.contains_key("origin")
                    || table.contains_key("origin_um")
                    || table.contains_key("rotate")
                    || table.contains_key("rotate_deg")
                {
                    return Err(f.err(
                        "connect",
                        "places the component itself: give it, or origin and rotate, not both",
                    ));
                }
                let pair = match c {
                    Value::Array(a) => match a.as_slice() {
                        [Value::String(own), Value::String(to)] => Some((own, to)),
                        _ => None,
                    },
                    _ => None,
                };
                let (own, to) = pair.ok_or_else(|| {
                    f.err(
                        "connect",
                        "needs [its port, another's], e.g. [\"o1\", \"ring.through\"]",
                    )
                })?;
                let target = find(&placed, to).map_err(|m| f.err("connect", m))?;
                drawing
                    .connected(own, &target, mirrored)
                    .map_err(|e| f.err("connect", reason(e)))?
            }
            None => {
                let origin = match (table.get("origin"), table.get("origin_um")) {
                    (None, None) => [0.0, 0.0],
                    (Some(_), Some(_)) => {
                        return Err(f.err("origin", "give origin or origin_um, not both"));
                    }
                    (Some(v), None) => {
                        pair_of(v, p, Want::Length).map_err(|m| f.err("origin", m))?
                    }
                    (None, Some(v)) => {
                        pair_of(v, p, Want::Micrometres).map_err(|m| f.err("origin_um", m))?
                    }
                };
                let rotate = f.angle("rotate")?.unwrap_or(0.0);
                let mut t = if mirrored {
                    Transform::mirror(Point::default(), 0.0)
                } else {
                    Transform::IDENTITY
                };
                if rotate != 0.0 {
                    t = t.then(&rotation(rotate));
                }
                t = t.then(&Transform::translate(
                    Length::um(origin[0]),
                    Length::um(origin[1]),
                ));
                if t == Transform::IDENTITY {
                    drawing
                } else {
                    drawing.transformed(&t)
                }
            }
        };
        let etch = match table.get("etch") {
            None => None,
            Some(v) => Some(
                v.clone()
                    .try_into::<EtchSpec>()
                    .map_err(|e| f.err("etch", e))?,
            ),
        };
        placed.push(Placed {
            name,
            drawing,
            etch,
        });
    }
    Ok(placed)
}

/// An error's reason, without the kind it names.
fn reason(e: crate::Error) -> String {
    match e {
        crate::Error::InvalidValue { reason, .. } => reason,
        other => other.to_string(),
    }
}

/// A place [x, y] from `v`.
fn pair_of(v: &Value, p: &Parameters, want: Want) -> std::result::Result<[f64; 2], String> {
    match v {
        Value::Array(a) if a.len() == 2 => Ok([
            params::evaluate(&a[0], p, want).map_err(|m| format!("value 1: {m}"))?,
            params::evaluate(&a[1], p, want).map_err(|m| format!("value 2: {m}"))?,
        ]),
        _ => Err("needs [x, y]".into()),
    }
}

/// The port `reference` (`component.port`) among the components `placed`.
fn find(placed: &[Placed], reference: &str) -> std::result::Result<Port, String> {
    let (component, port) = reference.split_once('.').ok_or_else(|| {
        format!("\"{reference}\" isn't a port's name: component.port, e.g. \"ring.through\"")
    })?;
    let c = placed
        .iter()
        .find(|q| q.name.as_deref() == Some(component))
        .ok_or_else(|| {
            let names: Vec<&str> = placed.iter().filter_map(|q| q.name.as_deref()).collect();
            format!(
                "no component named \"{component}\"{}",
                if names.is_empty() {
                    String::new()
                } else {
                    format!(" before it: there are {}", names.join(", "))
                }
            )
        })?;
    c.drawing.port(port).cloned().ok_or_else(|| {
        let names: Vec<&str> = c.drawing.ports().iter().map(|q| q.name.as_str()).collect();
        format!(
            "the {} \"{component}\" has no port {port}: its ports are {}",
            c.drawing.kind(),
            names.join(", ")
        )
    })
}

/// `s` with the components in `entries` drawn on it, after its other shapes.
pub(super) fn draw(mut s: Structure, entries: &[Table]) -> Result<Structure> {
    for (k, c) in build_all(entries, None)?.iter().enumerate() {
        let etch = EtchSpec::etch(c.etch);
        for item in c.drawing.items() {
            s.draw_etched(&item.layer, item.shape.clone(), etch)
                .map_err(|e| match e {
                    crate::Error::InvalidValue { reason, .. } => {
                        task_error(format!("[[task.component]] #{}: {reason}", k + 1))
                    }
                    other => other,
                })?;
        }
    }
    Ok(s)
}

/// The named ports of `entries`' components, each as `component.port`.
pub(super) fn ports(entries: &[Table]) -> Result<Vec<(String, Port)>> {
    Ok(build_all(entries, None)?
        .into_iter()
        .filter_map(|c| {
            let name = c.name?;
            Some(
                c.drawing
                    .ports()
                    .iter()
                    .map(|q| (format!("{name}.{}", q.name), q.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect())
}

/// Which way a port faces, on an axis: the axis (0 for x, 1 for y) and the sign light leaves by.
fn facing(q: &Port) -> Option<(usize, f64)> {
    let d = q.direction;
    let near = |a: f64| (d - a).abs() <= 1e-9 || (d - a - 360.0).abs() <= 1e-9;
    if near(0.0) {
        Some((0, 1.0))
    } else if near(180.0) {
        Some((0, -1.0))
    } else if near(90.0) {
        Some((1, 1.0))
    } else if near(270.0) {
        Some((1, -1.0))
    } else {
        None
    }
}

/// A port's window across its plane: its guide's width and 1 µm each side, no closer than half
/// way to another port in the same plane, inside `window`.
fn window_of(q: &Port, axis: usize, all: &[(String, Port)], window: Option<[f64; 2]>) -> [f64; 2] {
    let along = |p: &Port| [p.position.x.to_um(), p.position.y.to_um()];
    let (at, across) = (along(q)[axis], along(q)[1 - axis]);
    let mut half = q.width.to_um() / 2.0 + 1.0;
    for (_, other) in all {
        if facing(other).map(|f| f.0) == Some(axis)
            && (along(other)[axis] - at).abs() <= 1e-9
            && other.position != q.position
        {
            half = half.min((along(other)[1 - axis] - across).abs() / 2.0);
        }
    }
    let (mut lo, mut hi) = (across - half, across + half);
    if let Some([a, b]) = window {
        lo = lo.max(a);
        hi = hi.min(b);
    }
    [lo, hi]
}

/// The task's window along x or y, if it has one.
fn task_window(task: &Table, key: &str) -> Option<[f64; 2]> {
    let a = task.get(key)?.as_array()?;
    match a.as_slice() {
        [x, y] => Some([
            x.as_float().or_else(|| x.as_integer().map(|i| i as f64))?,
            y.as_float().or_else(|| y.as_integer().map(|i| i as f64))?,
        ]),
        _ => None,
    }
}

/// `out`'s `[[task.port]]`, and its `"mode"` sources and monitors, that name a port (`port =
/// "ring.in"`) given their places from it; see the module.
pub(super) fn place_ports(out: &mut Table, all: &[(String, Port)]) -> Result<()> {
    let windows = [task_window(out, "x_um"), task_window(out, "y_um")];
    let lookup = |reference: &str, what: &str| -> Result<Port> {
        all.iter()
            .find(|(n, _)| n == reference)
            .map(|(_, q)| q.clone())
            .ok_or_else(|| {
                let names: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
                task_error(format!(
                    "{what}: no port \"{reference}\"{}",
                    if names.is_empty() {
                        ": the task's components have no named ports (name them with name = \
                         \"...\")"
                            .to_owned()
                    } else {
                        format!(": the ports are {}", names.join(", "))
                    }
                ))
            })
    };
    if let Some(Value::Array(entries)) = out.get_mut("port") {
        for (k, entry) in entries.iter_mut().enumerate() {
            let Value::Table(entry) = entry else { continue };
            let Some(reference) = entry.remove("port") else {
                continue;
            };
            let what = format!("[[task.port]] #{}", k + 1);
            let reference = reference.as_str().ok_or_else(|| {
                task_error(format!("{what}: port must be a string, \"name.port\""))
            })?;
            for key in ["x_um", "side"] {
                if entry.contains_key(key) {
                    return Err(task_error(format!(
                        "{what}: give port or {key}, not both: the port \"{reference}\" places it"
                    )));
                }
            }
            let q = lookup(reference, &what)?;
            let Some((0, sign)) = facing(&q) else {
                return Err(task_error(format!(
                    "{what}: the port \"{reference}\" faces {}°: an fdfd job's ports are columns \
                     normal to x, so a port must face 0° or 180°",
                    q.direction
                )));
            };
            entry.insert("x_um".into(), Value::Float(q.position.x.to_um()));
            entry.insert(
                "side".into(),
                Value::String(if sign < 0.0 { "left" } else { "right" }.into()),
            );
            if !entry.contains_key("y_um") {
                let [lo, hi] = window_of(&q, 0, all, windows[1]);
                entry.insert(
                    "y_um".into(),
                    Value::Array(vec![Value::Float(lo), Value::Float(hi)]),
                );
            }
        }
    }
    for list in ["source", "monitor"] {
        let Some(Value::Array(entries)) = out.get_mut(list) else {
            continue;
        };
        for (k, entry) in entries.iter_mut().enumerate() {
            let Value::Table(entry) = entry else { continue };
            let Some(reference) = entry.remove("port") else {
                continue;
            };
            let what = format!("[[task.{list}]] #{}", k + 1);
            let reference = reference.as_str().ok_or_else(|| {
                task_error(format!("{what}: port must be a string, \"name.port\""))
            })?;
            if entry.get("type").and_then(Value::as_str) != Some("mode") {
                return Err(task_error(format!(
                    "{what}: port is for a \"mode\" {list}: it puts its plane on the port"
                )));
            }
            for key in ["normal", "at_um", "direction"] {
                if entry.contains_key(key) {
                    return Err(task_error(format!(
                        "{what}: give port or {key}, not both: the port \"{reference}\" places it"
                    )));
                }
            }
            let q = lookup(reference, &what)?;
            let Some((axis, sign)) = facing(&q) else {
                return Err(task_error(format!(
                    "{what}: the port \"{reference}\" faces {}°: a plane is normal to x or y, so \
                     a port must face 0°, 90°, 180° or 270°",
                    q.direction
                )));
            };
            let at = if axis == 0 {
                q.position.x.to_um()
            } else {
                q.position.y.to_um()
            };
            entry.insert(
                "normal".into(),
                Value::String(if axis == 0 { "x" } else { "y" }.into()),
            );
            entry.insert("at_um".into(), Value::Float(at));
            let across = if axis == 0 { "y_um" } else { "x_um" };
            if !entry.contains_key(across) {
                let [lo, hi] = window_of(&q, axis, all, windows[1 - axis]);
                entry.insert(
                    across.into(),
                    Value::Array(vec![Value::Float(lo), Value::Float(hi)]),
                );
            }
            if list == "source" {
                // into the component: against the way light leaves by
                entry.insert(
                    "direction".into(),
                    Value::String(if sign < 0.0 { "+" } else { "-" }.into()),
                );
            } else if !entry.contains_key("name") {
                entry.insert("name".into(), Value::String(reference.to_owned()));
            }
        }
    }
    Ok(())
}

/// The validation cases of components in jobs: every parameter an expression, and ports found
/// by name.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "generator/job-expressions",
            title: r"Components in a job with every parameter an expression of the task's parameters (a ring, a racetrack, a disk, a directional coupler with arms, a bent coupler, a 2 × 2 MMI, a Y-branch, a crossing, three tapers, a bend, an Euler bend, four S-bends, a spiral, an apodized Bragg grating, a tapered subwavelength grating on a bridge, an apodized grating coupler, a W1 photonic-crystal guide and an inverse taper, some placed by origin, rotate and mirror, one connected to another's port): their shapes and ports against the generators called with the numbers (differing in any bit shown)",
            tier: Tier::Analytic,
            source: r"the expressions' values; measured 0",
            run: || {
                let n = expression_errors() as f64;
                Outcome {
                    measured: n,
                    expected: 0.0,
                    tolerance: 0.0,
                    error: n,
                }
            },
        },
        Case {
            id: "generator/job-ports",
            title: r"Ports found by name: a 2D FDFD job of a ring with its bus and leads, its ports named ring.in and ring.through, against the same job with the ports' columns, sides and windows written out, its S-parameters at 1.55 µm on a 50 nm grid (differing in any bit shown)",
            tier: Tier::Analytic,
            source: r"a port by name is its column, side and window; measured 0",
            run: || {
                let n = port_errors() as f64;
                Outcome {
                    measured: n,
                    expected: 0.0,
                    tolerance: 0.0,
                    error: n,
                }
            },
        },
    ]
}

/// A job's components, every parameter an expression.
pub(super) const EXPRESSIONS: &str = r#"
name = "components"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-20.0, 120.0]
y_um = [-30.0, 30.0]
step_nm = 200.0

[task.parameters]
w = "500 nm"
r = "2*w*5"
g = "w/2.5"
a = "30 deg"

[[task.component]]
type = "ring"
name = "ring"
width = "w"
radius = "r"
gap = "g"
drop_gap = "g + 100 nm"

[[task.component]]
type = "straight"
name = "lead"
width = "w"
length = "6*w"
connect = ["o1", "ring.through"]

[[task.component]]
type = "racetrack"
name = "rt"
width = "w"
radius = "r"
gap = "g"
coupling_length = "4 um"
origin = ["0 um", "20 um"]

[[task.component]]
type = "disk"
name = "disk"
width = "w"
radius = "r"
gap = "g"
origin = ["0 um", "-20 um"]
rotate = "90 deg"

[[task.component]]
type = "directional_coupler"
name = "dc"
width = "w"
gap = "g"
length = "8 um"
arm_length = "6 um"
arm_offset = "1.5 um"
arm_shape = "sine"
origin = ["30 um", "0 um"]

[[task.component]]
type = "bent_coupler"
name = "bent"
width = "w"
gap = "g"
radius = "r"
angle = "a"
origin = ["30 um", "15 um"]
mirror = true

[[task.component]]
type = "mmi"
name = "mmi"
width = "6*w"
length = "40*w"
inputs = 2
outputs = 2
access_width = "w"
taper_width = "2*w"
taper_length = "5 um"
origin = ["60 um", "0 um"]

[[task.component]]
type = "y_branch"
name = "y"
input_width = "w"
width = "w"
length = "2 um"
gap = "0.1*w"
arm_length = "10 um"
separation = "3 um"
origin = ["60 um", "15 um"]

[[task.component]]
type = "crossing"
name = "x"
width = "w"
arm = "3 um"
centre_width = "1.2 um"
origin = ["60 um", "-15 um"]

[[task.component]]
type = "taper"
name = "t1"
width_in = "w"
width_out = "4*w"
length = "15 um"
origin = ["90 um", "0 um"]

[[task.component]]
type = "taper"
name = "t2"
width_in = "w"
width_out = "4*w"
length = "15 um"
profile = "exponential"
origin = ["90 um", "5 um"]

[[task.component]]
type = "taper"
name = "t3"
width = "w + 0.1*x"
length = "15 um"
origin = ["90 um", "10 um"]

[[task.component]]
type = "bend"
name = "b"
width = "w"
radius = "r"
angle = "3*a"
origin = ["90 um", "-10 um"]

[[task.component]]
type = "euler_bend"
name = "e"
width = "w"
radius = "r"
angle = "-3*a"
p = "0.5"
origin = ["90 um", "-15 um"]

[[task.component]]
type = "s_bend"
name = "s1"
width = "w"
length = "12 um"
offset = "3 um"
origin = ["100 um", "15 um"]

[[task.component]]
type = "s_bend"
name = "s2"
width = "w"
length = "12 um"
offset = "-3 um"
shape = "euler"
p = "0.3"
origin = ["100 um", "25 um"]

[[task.component]]
type = "s_bend"
name = "s3"
width = "w"
length = "12 um"
offset = "3 um"
shape = "bezier"
origin = ["100 um", "-25 um"]

[[task.component]]
type = "s_bend"
name = "s4"
width = "w"
length = "12 um"
offset = "3 um"
shape = "sine"
origin = ["100 um", "-28 um"]

[[task.component]]
type = "spiral"
name = "sp"
width = "w"
pitch = "4*w"
inner_radius = "r"
turns = 3
straight = "2 um"
origin = ["0 um", "-60 um"]

[[task.component]]
type = "bragg_grating"
name = "bragg"
width = "w"
corrugation = "g/10"
period = "310 nm"
periods = 40
apodization = "g/10*exp(-((x - 6.2 um)/(3 um))^2)"
origin = ["30 um", "-40 um"]

[[task.component]]
type = "swg_waveguide"
name = "swg"
width = "w"
width_end = "0.8*w"
period = "250 nm"
duty = "0.6"
periods = 20
bridge = "w/5"
origin = ["30 um", "-45 um"]

[[task.component]]
type = "grating_coupler"
name = "gc"
width = "w"
grating_width = "10 um"
taper_length = "15 um"
period = "630 nm"
apodization = "0.8 - 0.01*x/(1 um)"
periods = 20
etch_depth = "70 nm"
origin = ["60 um", "-45 um"]

[[task.component]]
type = "photonic_crystal"
name = "pc"
pitch = "420 nm"
radius = "120 nm"
half = [7, 4]
defect = "w1"
origin = ["90 um", "-45 um"]

[[task.component]]
type = "edge_coupler"
name = "edge"
tip_width = "150 nm"
width = "w"
taper_length = "30 um"
straight = "r"
origin = ["-15 um", "-25 um"]
rotate = "180 deg"
"#;

/// The drawings of [`EXPRESSIONS`]'s components, from the generators called with numbers.
fn expected() -> Vec<Drawing> {
    let um = Length::um;
    let w = 0.5;
    let r = 5.0;
    let g = 0.2;
    let a = 30.0;
    let at = |x: f64, y: f64| Transform::translate(um(x), um(y));
    let d = |g: &dyn Generator| g.draw("Si").expect("a component");
    let ring = d(&Ring::new(um(w), um(r), um(g)).with_drop(um(g + 0.1)));
    let lead = d(&Straight::new(um(w), um(6.0 * w)))
        .connected("o1", ring.port("through").expect("through"), false)
        .expect("connected");
    // the task's parameters, which the custom taper's w(x) reads, in the order a job's table
    // gives them (by name)
    let p = Parameters::new(&[
        ("a".into(), "30 deg".into()),
        ("g".into(), "w/2.5".into()),
        ("r".into(), "2*w*5".into()),
        ("w".into(), "500 nm".into()),
    ])
    .expect("parameters");
    let custom = Expr::parse("w + 0.1*x", p.scope()).expect("w(x)");
    vec![
        ring,
        lead,
        d(&Ring::racetrack(um(w), um(r), um(g), um(4.0))).transformed(&at(0.0, 20.0)),
        d(&Disk::new(um(w), um(r), um(g))).transformed(&rotation(90.0).then(&at(0.0, -20.0))),
        d(&DirectionalCoupler::new(um(w), um(g), um(8.0)).with_arms(
            um(6.0),
            um(1.5),
            SBendKind::Sine,
        ))
        .transformed(&at(30.0, 0.0)),
        d(&BentCoupler::new(um(w), um(g), um(r), a))
            .transformed(&Transform::mirror(Point::default(), 0.0).then(&at(30.0, 15.0))),
        d(&Mmi::new(um(6.0 * w), um(40.0 * w), 2, 2, um(w)).with_tapers(um(2.0 * w), um(5.0)))
            .transformed(&at(60.0, 0.0)),
        d(&YBranch {
            gap: um(0.1 * w),
            ..YBranch::new(um(w), um(w), um(2.0), um(10.0), um(3.0))
        })
        .transformed(&at(60.0, 15.0)),
        d(&Crossing::new(um(w), um(3.0)).with_centre_width(um(1.2))).transformed(&at(60.0, -15.0)),
        d(&Taper::new(um(w), um(4.0 * w), um(15.0))).transformed(&at(90.0, 0.0)),
        d(&Taper::new(um(w), um(4.0 * w), um(15.0)).with_profile(TaperProfile::Exponential))
            .transformed(&at(90.0, 5.0)),
        d(&Taper::custom(custom, p.clone(), um(15.0))).transformed(&at(90.0, 10.0)),
        d(&Bend::arc(um(w), um(r), 3.0 * a)).transformed(&at(90.0, -10.0)),
        d(&Bend::euler(um(w), um(r), -3.0 * a, 0.5)).transformed(&at(90.0, -15.0)),
        d(&SBend::new(um(w), um(12.0), um(3.0), SBendKind::Cosine)).transformed(&at(100.0, 15.0)),
        d(&SBend::new(
            um(w),
            um(12.0),
            um(-3.0),
            SBendKind::Euler { p: 0.3 },
        ))
        .transformed(&at(100.0, 25.0)),
        d(&SBend::new(um(w), um(12.0), um(3.0), SBendKind::Bezier)).transformed(&at(100.0, -25.0)),
        d(&SBend::new(um(w), um(12.0), um(3.0), SBendKind::Sine)).transformed(&at(100.0, -28.0)),
        d(&Spiral {
            straight: um(2.0),
            ..Spiral::new(um(w), um(4.0 * w), um(r), 3)
        })
        .transformed(&at(0.0, -60.0)),
        d(
            &BraggGrating::new(um(w), um(g / 10.0), um(0.31), 40).with_apodization(
                Expr::parse("g/10*exp(-((x - 6.2 um)/(3 um))^2)", p.scope()).expect("Δw(x)"),
                p.clone(),
            ),
        )
        .transformed(&at(30.0, -40.0)),
        d(&SwgWaveguide {
            width_end: um(0.8 * w),
            bridge: um(w / 5.0),
            ..SwgWaveguide::new(um(w), um(0.25), 0.6, 20)
        })
        .transformed(&at(30.0, -45.0)),
        d(
            &GratingCoupler::new(um(w), um(10.0), um(15.0), um(0.63), 0.5, 20, um(0.07))
                .with_apodization(
                    Expr::parse("0.8 - 0.01*x/(1 um)", p.scope()).expect("F(x)"),
                    p.clone(),
                ),
        )
        .transformed(&at(60.0, -45.0)),
        d(&PhotonicCrystal::hexagonal(
            um(0.42),
            um(0.12),
            [7, 4],
            Defect::W1,
        ))
        .transformed(&at(90.0, -45.0)),
        d(&EdgeCoupler {
            straight: um(r),
            ..EdgeCoupler::new(um(0.15), um(w), um(30.0))
        })
        .transformed(&rotation(180.0).then(&at(-15.0, -25.0))),
    ]
}

/// [`EXPRESSIONS`]'s components against [`expected`]: the shapes and ports that differ (the
/// shapes by their definitions, the ports in any bit), and any count that does.
pub(crate) fn expression_errors() -> usize {
    let Ok(job) = crate::run::Job::parse(EXPRESSIONS) else {
        return 1000;
    };
    let Ok(task) = params::resolve(job.task(), None) else {
        return 1000;
    };
    let entries: Vec<Table> = match task.get("component") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_table().cloned()).collect(),
        _ => return usize::MAX,
    };
    let Ok(placed) = build_all(&entries, None) else {
        return 1000;
    };
    let want = expected();
    let mut wrong = placed.len().abs_diff(want.len());
    for (got, want) in placed.iter().zip(&want) {
        let got = &got.drawing;
        wrong += got.items().len().abs_diff(want.items().len());
        wrong += got
            .items()
            .iter()
            .zip(want.items())
            .filter(|(a, b)| a != b)
            .count();
        wrong += got.ports().len().abs_diff(want.ports().len());
        wrong += got
            .ports()
            .iter()
            .zip(want.ports())
            .filter(|(a, b)| {
                a.name != b.name
                    || a.position.x.to_um().to_bits() != b.position.x.to_um().to_bits()
                    || a.position.y.to_um().to_bits() != b.position.y.to_um().to_bits()
                    || a.direction.to_bits() != b.direction.to_bits()
                    || a.width.to_um().to_bits() != b.width.to_um().to_bits()
            })
            .count();
    }
    wrong
}

/// An FDFD job of a ring with its bus, leads carrying the bus into the PMLs, its ports named,
/// and the same job with them written out.
pub(super) fn port_jobs() -> (String, String) {
    let head = r#"
name = "ring-ports"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-6.0, 6.0]
y_um = [-2.0, 7.0]
step_nm = 50.0

[[task.component]]
type = "ring"
name = "ring"
width = "500 nm"
radius = "2.5 um"
gap = "200 nm"
reach = "4.5 um"

[[task.component]]
type = "straight"
name = "left"
width = "500 nm"
length = "2 um"
connect = ["o2", "ring.in"]

[[task.component]]
type = "straight"
name = "right"
width = "500 nm"
length = "2 um"
connect = ["o1", "ring.through"]
"#;
    let named = format!(
        "{head}\n[[task.port]]\nport = \"ring.in\"\n\n[[task.port]]\nport = \"ring.through\"\n"
    );
    // the bus at y = 0, 0.5 µm wide, no other port in either column (the leads' ends on the
    // ring's are the same place): each window is the guide and 1 µm each side
    let written = format!(
        "{head}\n[[task.port]]\nx_um = -4.5\nside = \"left\"\ny_um = [-1.25, 1.25]\n\n\
         [[task.port]]\nx_um = 4.5\nside = \"right\"\ny_um = [-1.25, 1.25]\n"
    );
    (named, written)
}

/// [`port_jobs`]' S-parameters against each other: the entries differing in any bit (1000 if
/// a job fails).
pub(crate) fn port_errors() -> usize {
    let (named, written) = port_jobs();
    let s = |text: &str| -> Option<super::FdfdSParameters> {
        let job = crate::run::Job::parse(text).ok()?;
        super::fdfd_s_parameters(&job, None).ok()
    };
    let (Some(a), Some(b)) = (s(&named), s(&written)) else {
        return 1000;
    };
    let flat = |x: &super::FdfdSParameters| -> Vec<u64> {
        x.s.iter()
            .flatten()
            .flatten()
            .flat_map(|c| [c.re.to_bits(), c.im.to_bits()])
            .collect()
    };
    let (fa, fb) = (flat(&a), flat(&b));
    fa.len().abs_diff(fb.len()) + fa.iter().zip(&fb).filter(|(x, y)| x != y).count()
}
