//! A task's parameters and the expressions in its shapes, resolved into the numbers the task's
//! tables read: `[task.parameters]` evaluated (one of them set to a sweep's value, if asked),
//! each shape's `center`, `size`, `radius` and `width` (expressions with units) and any string
//! in its `_um` keys turned into µm, and a sweep's `from` and `to` given as strings into
//! numbers. Numbers in `_um` keys are left as they are, to the bit.

use serde::de::DeserializeOwned;
use toml::{Table, Value};

use super::task_error;
use crate::Result;
use crate::expr::{COORDINATES, Dim, Expr, Parameters};

/// The shape tables and their fields that take lengths: the field's name and how many numbers
/// it holds (its `_um` key holds the same in µm).
const SHAPES: [(&str, &[(&str, usize)]); 3] = [
    ("rect", &[("center", 2), ("size", 2)]),
    ("circle", &[("center", 2), ("radius", 1)]),
    ("ring", &[("center", 2), ("radius", 1), ("width", 1)]),
];

/// The sweep parameters a modes job knows without a parameter table.
const OWN: [&str; 2] = ["wavelength", "width"];

/// The task, parameters and expressions resolved, as the kind's table `T`.
pub(super) fn task<T: DeserializeOwned>(job: &crate::run::Job) -> Result<T> {
    at(job.task(), None)
}

/// The task with the parameter `set.0` at the value `set.1`, as the kind's table `T`.
pub(super) fn at<T: DeserializeOwned>(task: &Table, set: Option<(&str, f64)>) -> Result<T> {
    resolve(task, set)?
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))
}

/// The names of the task's parameters.
pub(super) fn names(task: &Table) -> Vec<String> {
    task.get("parameters")
        .and_then(Value::as_table)
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

/// The task's parameters, evaluated.
fn parameters(task: &Table) -> Result<Parameters> {
    let Some(table) = task.get("parameters") else {
        return Ok(Parameters::none());
    };
    let table = table
        .as_table()
        .ok_or_else(|| task_error("[task.parameters] must be a table: name = \"expression\""))?;
    let mut definitions = Vec::with_capacity(table.len());
    for (name, value) in table {
        if OWN.contains(&name.as_str()) {
            return Err(task_error(format!(
                "[task.parameters] {name}: \"{name}\" is a sweep's own (the job's wavelength, or \
                 a rectangle's width): give the parameter another name"
            )));
        }
        definitions.push((
            name.clone(),
            text(value).ok_or_else(|| {
                task_error(format!(
                    "[task.parameters] {name}: must be an expression in a string, e.g. \"500 nm\", \
                 or a number, got {value}"
                ))
            })?,
        ));
    }
    Parameters::new(&definitions).map_err(|e| {
        task_error(if e.name.is_empty() {
            format!("[task.parameters]: {}", e.message)
        } else {
            format!("[task.parameters] {}: {}", e.name, e.message)
        })
    })
}

/// A value as an expression's text: a string as it is, a number as it reads.
fn text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        _ => None,
    }
}

/// What a value must be.
#[derive(Clone, Copy)]
enum Want {
    /// A length; a plain number only if it is zero.
    Length,
    /// A length, a plain number counting as µm (a `_um` key).
    Micrometres,
    /// Of this dimension; a plain number counting as one in its unit.
    Like(Option<Dim>),
}

/// `value` (an expression's string, or a number) evaluated at the parameters, as `want` says.
fn evaluate(value: &Value, p: &Parameters, want: Want) -> std::result::Result<f64, String> {
    let text =
        text(value).ok_or_else(|| format!("must be an expression or a number, got {value}"))?;
    let e = Expr::parse(&text, p.scope()).map_err(|e| e.to_string())?;
    if let Some(&slot) = e.slots().iter().find(|&&s| s < COORDINATES.len()) {
        return Err(format!(
            "{} is a coordinate, which this field has no value of",
            COORDINATES[slot].0
        ));
    }
    let ok = match (want, e.dim()) {
        (_, None) => true,
        (Want::Length, Some(d)) => d == Dim::LENGTH,
        (Want::Micrometres, Some(d)) => d == Dim::LENGTH || d == Dim::NONE,
        (Want::Like(None), Some(_)) => true,
        (Want::Like(Some(w)), Some(d)) => d.fits(w) || d == Dim::NONE,
    };
    if !ok {
        let got = e.dim().map_or("zero".to_owned(), Dim::name);
        return Err(match want {
            Want::Length if e.dim() == Some(Dim::NONE) => format!(
                "needs a length, got a number: give its unit (\"{text} um\" or \"{text} nm\"), or \
                 use the _um key"
            ),
            Want::Length | Want::Micrometres => format!("needs a length, got {got}"),
            Want::Like(w) => format!(
                "needs {}, got {got}",
                w.map_or("a value".to_owned(), Dim::name)
            ),
        });
    }
    e.try_eval(p.values()).map_err(|e| e.to_string())
}

/// The task's table with its parameters and expressions resolved; see the module.
pub(super) fn resolve(task: &Table, set: Option<(&str, f64)>) -> Result<Table> {
    let mut p = parameters(task)?;
    if let Some((name, value)) = set {
        p = p.with(name, value).map_err(|e| {
            task_error(format!(
                "[task.parameters] {}, at the sweep's {value}: {}",
                e.name, e.message
            ))
        })?;
    }
    let mut out = task.clone();
    out.remove("parameters");
    for (kind, fields) in SHAPES {
        let Some(Value::Array(entries)) = out.get_mut(kind) else {
            continue;
        };
        for (k, entry) in entries.iter_mut().enumerate() {
            let Value::Table(entry) = entry else {
                continue;
            };
            for &(field, count) in fields {
                let um = format!("{field}_um");
                let at = |message: String| {
                    task_error(format!("[[task.{kind}]] #{}, {field}: {message}", k + 1))
                };
                let resolved = match (entry.remove(field), entry.get(&um)) {
                    (Some(_), Some(_)) => {
                        return Err(at(format!("give {field} or {um}, not both")));
                    }
                    (Some(v), None) => Some(numbers(&v, count, &p, Want::Length).map_err(at)?),
                    (None, Some(v)) if has_string(v) => {
                        Some(numbers(v, count, &p, Want::Micrometres).map_err(at)?)
                    }
                    _ => None,
                };
                if let Some(v) = resolved {
                    entry.insert(um, v);
                }
            }
        }
    }
    if let Some(Value::Table(sweep)) = out.get_mut("sweep")
        && let Some(name) = sweep
            .get("parameter")
            .and_then(Value::as_str)
            .map(str::to_owned)
    {
        let want = match p.dim(&name) {
            Some(d) => Want::Like(d),
            None if OWN.contains(&name.as_str()) => Want::Micrometres,
            None => Want::Like(None),
        };
        for end in ["from", "to"] {
            if let Some(v @ Value::String(_)) = sweep.get(end) {
                let x = evaluate(v, &p, want)
                    .map_err(|m| task_error(format!("[task.sweep] {end}: {m}")))?;
                sweep.insert(end.to_owned(), Value::Float(x));
            }
        }
    }
    Ok(out)
}

fn has_string(v: &Value) -> bool {
    match v {
        Value::String(_) => true,
        Value::Array(a) => a.iter().any(|x| matches!(x, Value::String(_))),
        _ => false,
    }
}

/// A field's value (one expression, or an array of `count`) as µm: numbers in a `_um` key stay
/// as they are.
fn numbers(
    v: &Value,
    count: usize,
    p: &Parameters,
    want: Want,
) -> std::result::Result<Value, String> {
    let one = |x: &Value| -> std::result::Result<Value, String> {
        match (want, x) {
            (Want::Micrometres, Value::Float(_) | Value::Integer(_)) => Ok(x.clone()),
            _ => evaluate(x, p, want).map(Value::Float),
        }
    };
    if count == 1 {
        if v.is_array() {
            return Err("needs one value, not a list".to_owned());
        }
        return one(v);
    }
    match v {
        Value::Array(a) if a.len() == count => {
            let mut out = Vec::with_capacity(count);
            for (k, x) in a.iter().enumerate() {
                out.push(one(x).map_err(|m| format!("value {}: {m}", k + 1))?);
            }
            Ok(Value::Array(out))
        }
        _ => Err(format!("needs a list of {count} values")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        text.parse::<Table>().unwrap()
    }

    #[test]
    fn numbers_stay_as_they_are() {
        let t = table(
            "kind = \"structure\"\n[[rect]]\nlayer = \"Si\"\ncenter_um = [0.1, 0.2]\nsize_um = [3, 0.5]\n",
        );
        assert_eq!(resolve(&t, None).unwrap(), t);
    }

    #[test]
    fn expressions_become_micrometres() {
        let t = table(
            r#"
[parameters]
w = "500 nm"
gap = "w/2.5"
radius = "5 um"

[[ring]]
layer = "Si"
center = [0, "radius + w + gap"]
radius = "radius"
width = "w"

[[rect]]
layer = "Si"
center_um = [0.0, "-w"]
size = ["10 um", "w"]
"#,
        );
        let r = resolve(&t, None).unwrap();
        assert!(r.get("parameters").is_none());
        let ring = &r["ring"][0];
        assert_eq!(
            ring["center_um"],
            Value::Array(vec![Value::Float(0.0), Value::Float(5.7)])
        );
        assert_eq!(ring["radius_um"], Value::Float(5.0));
        assert_eq!(ring["width_um"], Value::Float(0.5));
        assert!(ring.get("radius").is_none());
        let rect = &r["rect"][0];
        assert_eq!(
            rect["center_um"],
            Value::Array(vec![Value::Float(0.0), Value::Float(-0.5)])
        );
        assert_eq!(
            rect["size_um"],
            Value::Array(vec![Value::Float(10.0), Value::Float(0.5)])
        );
        // a sweep's point: w at 0.6, the ring's centre follows it
        let r = resolve(&t, Some(("w", 0.6))).unwrap();
        let y = r["ring"][0]["center_um"][1].as_float().unwrap();
        assert!((y - (5.0 + 0.6 + 0.24)).abs() < 1e-15, "{y}");
    }

    #[test]
    fn errors_name_the_table_and_the_field() {
        let err = |text: &str| resolve(&table(text), None).unwrap_err().to_string();
        let e = err("[[rect]]\nlayer = \"Si\"\ncenter = [0, 0]\nsize = [\"1 um\", \"wdth\"]\n");
        assert!(
            e.contains("[[task.rect]] #1, size: value 2: column 1: unknown name \"wdth\""),
            "{e}"
        );
        let e = err("[[circle]]\nlayer = \"Si\"\ncenter = [0, 0]\nradius = \"30 deg\"\n");
        assert!(
            e.contains("[[task.circle]] #1, radius: needs a length, got an angle"),
            "{e}"
        );
        let e = err("[[circle]]\nlayer = \"Si\"\ncenter = [0, 0]\nradius = 2\n");
        assert!(e.contains("needs a length, got a number"), "{e}");
        let e = err("[[circle]]\nlayer = \"Si\"\ncenter = [0, 0]\nradius = \"x\"\n");
        assert!(e.contains("x is a coordinate"), "{e}");
        let e =
            err("[[circle]]\nlayer = \"Si\"\ncenter = [0, 0]\ncenter_um = [0, 0]\nradius_um = 1\n");
        assert!(e.contains("give center or center_um, not both"), "{e}");
        let e = err("[parameters]\na = \"b\"\nb = \"a\"\n");
        assert!(
            e.contains("[task.parameters]: the parameters a and b use each other"),
            "{e}"
        );
        let e = err("[parameters]\nw = \"1 um\"\nv = \"w + 1 deg\"\n");
        assert!(
            e.contains("[task.parameters] v: column 3: can't add an angle to a length"),
            "{e}"
        );
        let e = err("[parameters]\nwidth = \"1 um\"\n");
        assert!(e.contains("a sweep's own"), "{e}");
        let e = err(
            "[parameters]\nw = \"1 um\"\n[sweep]\nparameter = \"w\"\nfrom = \"1 deg\"\nto = 2\npoints = 2\n",
        );
        assert!(
            e.contains("[task.sweep] from: needs a length, got an angle"),
            "{e}"
        );
    }

    use crate::job::{Event, check, execute};
    use crate::run::{Job, Run, Stop, replay};

    const MODES: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 1

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [0.5, 10.0]
"#;

    /// The same strip, its width a parameter.
    const PARAMETRIC: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 1

[task.parameters]
w = "500 nm"
length = "20 * w"

[[task.rect]]
layer = "Si"
center = [0, "0 nm"]
size = ["w", "length"]
"#;

    /// A run's events but its last (which says how long it took).
    fn events(tag: &str, text: &str) -> Vec<Event> {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-params-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let job = Job::parse(text).unwrap();
        check(&job).unwrap();
        let mut run = Run::create(&dir, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let mut events = replay(run.dir()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(events.pop(), Some(Event::Finished { .. })));
        events
    }

    #[test]
    fn a_job_with_parameters_is_the_job_with_their_numbers() {
        assert_eq!(events("plain", MODES), events("parametric", PARAMETRIC));
    }

    #[test]
    fn a_parameters_sweep_is_the_width_sweep_it_stands_for() {
        let width = format!(
            "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 2\n"
        );
        let w = format!(
            "{PARAMETRIC}\n[task.sweep]\nparameter = \"w\"\nfrom = \"450 nm\"\nto = 0.55\npoints = \
             2\n"
        );
        let (a, b) = (events("width", &width), events("w", &w));
        let points = |events: &[Event]| -> Vec<(String, f64, Vec<[f64; 2]>)> {
            events
                .iter()
                .filter_map(|e| match e {
                    Event::SweepPoint {
                        parameter,
                        value,
                        effective_indices,
                        ..
                    } => Some((parameter.clone(), *value, effective_indices.clone())),
                    _ => None,
                })
                .collect()
        };
        let (pa, pb) = (points(&a), points(&b));
        assert_eq!(pa.len(), 2);
        assert_eq!(pb[0].0, "w");
        assert_eq!((pb[0].1, pb[1].1), (0.45, 0.55));
        // the strip is 20 w long now, which a cut across it doesn't see: the same indices
        for (x, y) in pa.iter().zip(&pb) {
            assert_eq!(x.1, y.1);
            assert_eq!(x.2, y.2);
        }
        assert!(pb[1].2[0][0] > pb[0].2[0][0]);
        // and the point's shapes are the parameter's
        let lengths: Vec<f64> = b
            .iter()
            .filter_map(|e| match e {
                Event::SweepShapes { shapes, .. } => {
                    let ys: Vec<f64> = shapes[0].outline.iter().map(|p| p[1]).collect();
                    Some(ys.iter().fold(f64::MIN, |m, v| m.max(*v)) * 2.0)
                }
                _ => None,
            })
            .collect();
        assert_eq!(lengths, [9.0, 11.0]);
    }

    #[test]
    fn the_check_names_the_table_and_field_of_an_expressions_error() {
        let refused = |text: &str| check(&Job::parse(text).unwrap()).unwrap_err().to_string();
        let e = refused(&PARAMETRIC.replace("\"length\"]", "\"lenght\"]"));
        assert!(
            e.contains("[[task.rect]] #1, size: value 2: column 1: unknown name \"lenght\""),
            "{e}"
        );
        let e = refused(&PARAMETRIC.replace("\"20 * w\"", "\"20 * w + 1 deg\""));
        assert!(
            e.contains("[task.parameters] length: column 8: can't add an angle to a length"),
            "{e}"
        );
        let e = refused(&format!(
            "{PARAMETRIC}\n[task.sweep]\nparameter = \"q\"\nfrom = 1\nto = 2\npoints = 2\n"
        ));
        assert!(e.contains("unknown sweep parameter \"q\""), "{e}");
        // a sweep whose far end draws no shape
        let e = refused(&format!(
            "{PARAMETRIC}\n[task.sweep]\nparameter = \"w\"\nfrom = 0.5\nto = -0.1\npoints = 2\n"
        ));
        assert!(e.contains("at the sweep's w = -0.1"), "{e}");
    }

    #[test]
    fn a_sweeps_ends_can_have_units() {
        let t = table(
            "[parameters]\ngap = \"200 nm\"\n[sweep]\nparameter = \"gap\"\nfrom = \"150 nm\"\nto = 0.25\npoints = 3\n",
        );
        let r = resolve(&t, None).unwrap();
        assert_eq!(r["sweep"]["from"], Value::Float(0.15));
        assert_eq!(r["sweep"]["to"], Value::Float(0.25));
        let t = table(
            "[sweep]\nparameter = \"wavelength\"\nfrom = \"1500 nm\"\nto = 1.6\npoints = 3\n",
        );
        assert_eq!(
            resolve(&t, None).unwrap()["sweep"]["from"],
            Value::Float(1.5)
        );
    }
}
