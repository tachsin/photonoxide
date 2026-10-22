//! The job file's shorter forms, read beside the keys they stand for and written in YAML:
//!
//! - **units in the value**: a key `x_um`, `step_nm`, `angle_rad` or `sidewall_deg` may be given
//!   as `x`, `step`, `angle` or `sidewall` with the unit in its value (`wavelength: 1.55 um`,
//!   `step: 20 nm`, `x: [-4 um, 4 um]`), in any table whose struct has the key and not the bare
//!   name. A plain number there is refused unless it is zero: a number is never read in a unit
//!   it doesn't name.
//! - **one line per shape**: `shapes: [- rect: { ... }, - ring: { ... }]`, each a
//!   `[[task.rect]]`, `[[task.circle]]`, `[[task.ring]]` or `[[task.path]]`, or a
//!   `[[task.shape]]` of the type it names (`- polygon: { ... }`), and the same lines in a cell's
//!   `shapes`.
//! - YAML's anchors, aliases and merge keys (`&guide`, `*guide`, `<<: *guide`), which the YAML
//!   reader resolves.
//!
//! Where a key's bare name stands for it is read off the job file's schema ([`super::schema`]),
//! which is derived from the structs each table is read into, so the forms can't drift from the
//! keys. [`read`] turns a file's tables into the keys the library reads, when the job is parsed;
//! [`write`] the other way, for the YAML the program writes. The two give back the same job, to
//! the bit.

use std::sync::OnceLock;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value as Schema;
use toml::{Table, Value};

use super::params::{Want, evaluate};
use crate::expr::Parameters;

/// The units a key's name may end in, and what its value must be.
pub(super) const UNITS: [(&str, Want); 4] = [
    ("um", Want::Length),
    ("nm", Want::Length),
    ("rad", Want::Angle),
    ("deg", Want::Angle),
];

/// The shape tables a line in `shapes` may be, besides a `[[task.shape]]` of its type.
const LINES: [&str; 4] = ["rect", "circle", "ring", "path"];

/// The schema the forms are read off: the structs' own keys, without the forms.
fn schema() -> &'static Schema {
    static BASE: OnceLock<Schema> = OnceLock::new();
    BASE.get_or_init(super::schema::base)
}

/// A file's tables (`task`, and `device` and `circuit` when it has them) with the shorter forms
/// turned into the keys the library reads.
///
/// # Errors
///
/// A message naming the key: a shape's line that isn't one shape, a bare key and its unit's
/// both given, a value without its unit or of the wrong dimension.
pub(crate) fn read(file: &mut Table) -> std::result::Result<(), String> {
    if let Some(Value::Table(task)) = file.get_mut("task") {
        lines(task)?;
    }
    let root = schema();
    walk_table(file, &[root], "", &|t, s, at| unit_keys_in(t, s, at))
}

/// The file's tables with the keys that have shorter forms written in them, for YAML: the
/// inverse of [`read`].
pub(crate) fn write(file: &Table) -> Table {
    let mut out = file.clone();
    let root = schema();
    // a key that can't be shortened is left as it is: writing never fails
    let _ = walk_table(&mut out, &[root], "", &|t, s, _| {
        shorten_in(t, s);
        Ok(())
    });
    out
}

/// What a table may hold, from the schemas it may be read by: each key's schemas, and those of
/// keys not named (an open table's).
#[derive(Default)]
struct Keys<'a> {
    named: Vec<(&'a str, Vec<&'a Schema>)>,
    other: Vec<&'a Schema>,
    items: Vec<&'a Schema>,
}

impl<'a> Keys<'a> {
    fn has(&self, key: &str) -> bool {
        self.named.iter().any(|(k, _)| *k == key)
    }

    fn of(&self, key: &str) -> Vec<&'a Schema> {
        let mut out: Vec<&'a Schema> = self
            .named
            .iter()
            .filter(|(k, _)| *k == key)
            .flat_map(|(_, s)| s.iter().copied())
            .collect();
        if out.is_empty() {
            out.clone_from(&self.other);
        }
        out
    }
}

/// Every schema `s` stands for: itself, what it refers to, and the alternatives and cases in it
/// (each case's `then` whatever its `if`, so a table's keys are those of any kind it may be).
fn expand<'a>(s: &'a Schema, out: &mut Vec<&'a Schema>, depth: usize) {
    let Some(o) = s.as_object() else {
        return;
    };
    if depth > 32 || out.iter().any(|x| std::ptr::eq(*x, s)) {
        return;
    }
    out.push(s);
    if let Some(r) = o.get("$ref").and_then(Schema::as_str)
        && let Some(name) = r.strip_prefix("#/$defs/")
        && let Some(def) = schema().get("$defs").and_then(|d| d.get(name))
    {
        expand(def, out, depth + 1);
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(Schema::Array(list)) = o.get(key) {
            for x in list {
                expand(x, out, depth + 1);
            }
        }
    }
    for key in ["then", "else"] {
        if let Some(x) = o.get(key) {
            expand(x, out, depth + 1);
        }
    }
}

/// The keys the schemas `of` give a table, and the items they give a list.
fn keys<'a>(of: &[&'a Schema]) -> Keys<'a> {
    let mut all = Vec::new();
    for s in of {
        expand(s, &mut all, 0);
    }
    let mut k = Keys::default();
    for s in all {
        if let Some(Schema::Object(p)) = s.get("properties") {
            for (name, x) in p {
                match k.named.iter_mut().find(|(n, _)| n == name) {
                    Some((_, list)) => list.push(x),
                    None => k.named.push((name.as_str(), vec![x])),
                }
            }
        }
        if let Some(x @ Schema::Object(_)) = s.get("additionalProperties") {
            k.other.push(x);
        }
        match s.get("items") {
            Some(x @ Schema::Object(_)) => k.items.push(x),
            Some(Schema::Array(list)) => k.items.extend(list),
            _ => {}
        }
        if let Some(Schema::Array(list)) = s.get("prefixItems") {
            k.items.extend(list);
        }
    }
    k
}

/// What [`walk_table`] does at each table: the table, the keys it may hold, and where it is.
type Visit<'f> = dyn Fn(&mut Table, &Keys<'_>, &str) -> std::result::Result<(), String> + 'f;

/// `visit` at the table `t` and at every table in it, each with the keys its schemas give it.
fn walk_table(
    t: &mut Table,
    of: &[&Schema],
    at: &str,
    visit: &Visit<'_>,
) -> std::result::Result<(), String> {
    let k = keys(of);
    visit(t, &k, at)?;
    for (name, v) in t.iter_mut() {
        let here = if at.is_empty() {
            name.clone()
        } else {
            format!("{at}.{name}")
        };
        walk_value(v, &k.of(name), &here, visit)?;
    }
    Ok(())
}

fn walk_value(
    v: &mut Value,
    of: &[&Schema],
    at: &str,
    visit: &Visit<'_>,
) -> std::result::Result<(), String> {
    if of.is_empty() {
        return Ok(());
    }
    match v {
        Value::Table(t) => walk_table(t, of, at, visit),
        Value::Array(a) => {
            let items = keys(of).items;
            for (j, x) in a.iter_mut().enumerate() {
                walk_value(x, &items, &format!("{at}[{}]", j + 1), visit)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The key `name` stands for in a table that may hold `k`: `name` with a unit's ending that the
/// table has, when it hasn't `name` itself; with the unit and what it measures.
fn unit_key(name: &str, k: &Keys<'_>) -> Option<(String, &'static str, Want)> {
    if k.has(name) {
        return None;
    }
    UNITS.iter().find_map(|&(unit, want)| {
        let key = format!("{name}_{unit}");
        k.has(&key).then_some((key, unit, want))
    })
}

/// The bare keys of `t` that stand for a unit's key, turned into it.
fn unit_keys_in(t: &mut Table, k: &Keys<'_>, at: &str) -> std::result::Result<(), String> {
    let bare: Vec<(String, String, &'static str, Want)> = t
        .keys()
        .filter_map(|name| unit_key(name, k).map(|(key, u, w)| (name.clone(), key, u, w)))
        .collect();
    for (name, key, unit, want) in bare {
        let path = if at.is_empty() {
            name.clone()
        } else {
            format!("{at}.{name}")
        };
        if t.contains_key(&key) {
            return Err(format!("{path}: give {name} or {key}, not both"));
        }
        let v = t.remove(&name).expect("a key of the table");
        let n = in_unit(&v, unit, want)
            .map_err(|m| format!("{path}: {}", m.replace("the _um key", &key)))?;
        t.insert(key, n);
    }
    Ok(())
}

/// `v` (a value with its unit, a zero, or a list of them) as numbers in `unit`.
fn in_unit(v: &Value, unit: &str, want: Want) -> std::result::Result<Value, String> {
    if let Value::Array(a) = v {
        return a
            .iter()
            .map(|x| in_unit(x, unit, want))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(Value::Array);
    }
    // the number as written, when it is written in the key's own unit: to the bit
    if let Value::String(s) = v
        && let Some(n) = s.trim().strip_suffix(unit)
        && let Ok(x) = n.trim_end().parse::<f64>()
        && x.is_finite()
    {
        return Ok(Value::Float(x));
    }
    let x = match v {
        Value::String(_) | Value::Integer(_) | Value::Float(_) => {
            evaluate(v, &Parameters::none(), want)?
        }
        other => {
            return Err(format!(
                "needs a value with its unit, e.g. \"1.55 um\", got {other}"
            ));
        }
    };
    // the expression's value, in µm or radians, in the key's unit
    Ok(Value::Float(match unit {
        "nm" => x * 1000.0,
        "deg" => x.to_degrees(),
        _ => x,
    }))
}

/// The keys of `t` that have a bare form, written in it with their unit: those holding floats
/// alone (an integer is left in its key, so it reads back an integer).
fn shorten_in(t: &mut Table, k: &Keys<'_>) {
    let names: Vec<String> = t.keys().cloned().collect();
    let mut out = Table::new();
    for name in names {
        let v = t.remove(&name).expect("a key of the table");
        let short = UNITS.iter().find_map(|&(unit, _)| {
            let bare = name.strip_suffix(&format!("_{unit}"))?;
            (!k.has(bare) && k.has(&name) && !out.contains_key(bare))
                .then_some(())
                .and_then(|()| with_unit(&v, unit))
                .map(|s| (bare.to_owned(), s))
        });
        match short {
            Some((bare, s)) => out.insert(bare, s),
            None => out.insert(name, v),
        };
    }
    *t = out;
}

/// `v`, floats alone or lists of them, written with `unit`; none if it holds anything else.
fn with_unit(v: &Value, unit: &str) -> Option<Value> {
    match v {
        Value::Float(x) if x.is_finite() => Some(Value::String(format!("{} {unit}", number(*x)))),
        Value::Array(a) if !a.is_empty() => a
            .iter()
            .map(|x| with_unit(x, unit))
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        _ => None,
    }
}

/// A float as it reads back to the bit: a whole number without its point.
fn number(x: f64) -> String {
    if x == x.trunc() && x.abs() < 1e15 && !(x == 0.0 && x.is_sign_negative()) {
        // whole and small: exact as an integer
        #[allow(clippy::cast_possible_truncation)]
        let n = x as i64;
        n.to_string()
    } else {
        format!("{x:?}")
    }
}

/// The task's `shapes`, one line each, put in their tables, and the lines in its cells' `shapes`
/// given their type.
fn lines(task: &mut Table) -> std::result::Result<(), String> {
    if let Some(v) = task.remove("shapes") {
        let Value::Array(items) = v else {
            return Err(
                "task.shapes: needs a list of shapes, one to a line: - rect: { ... }".into(),
            );
        };
        for (j, item) in items.into_iter().enumerate() {
            let (kind, fields) = line(item).map_err(|m| format!("task.shapes[{}]: {m}", j + 1))?;
            let (list, entry) = if LINES.contains(&kind.as_str()) {
                (kind, fields)
            } else {
                ("shape".to_owned(), typed(kind, fields))
            };
            match task
                .entry(list.clone())
                .or_insert_with(|| Value::Array(Vec::new()))
            {
                Value::Array(a) => a.push(Value::Table(entry)),
                _ => return Err(format!("task.{list}: needs a list of tables")),
            }
        }
    }
    if let Some(Value::Array(cells)) = task.get_mut("cell") {
        for (k, cell) in cells.iter_mut().enumerate() {
            let Some(Value::Array(shapes)) = cell.get_mut("shapes") else {
                continue;
            };
            for (j, shape) in shapes.iter_mut().enumerate() {
                if let Value::Table(t) = shape
                    && t.len() == 1
                    && !t.contains_key("type")
                {
                    let item = std::mem::replace(shape, Value::Table(Table::new()));
                    let (kind, fields) = line(item)
                        .map_err(|m| format!("task.cell[{}].shapes[{}]: {m}", k + 1, j + 1))?;
                    *shape = Value::Table(typed(kind, fields));
                }
            }
        }
    }
    Ok(())
}

/// A shape's line, `{ kind: { fields } }`, as its kind and fields.
fn line(item: Value) -> std::result::Result<(String, Table), String> {
    let one = "needs one shape, its type and its fields: - rect: { center: [...], ... }";
    let Value::Table(t) = item else {
        return Err(one.into());
    };
    if t.len() != 1 {
        return Err(one.into());
    }
    let (kind, fields) = t.into_iter().next().expect("one key");
    match fields {
        Value::Table(fields) if !fields.contains_key("type") => Ok((kind, fields)),
        Value::Table(_) => Err(format!("{kind}: its type is the key, not a field")),
        _ => Err(format!("{kind}: needs its fields, {{ ... }}")),
    }
}

/// A `[[task.shape]]` of type `kind`, with `fields`.
fn typed(kind: String, fields: Table) -> Table {
    let mut t = Table::new();
    t.insert("type".into(), Value::String(kind));
    t.extend(fields);
    t
}

/// A job file's YAML as the program writes it: the shorter forms ([`write`]), lists of plain
/// values on one line, and the rectangles, circles and rings one to a line in `shapes`.
pub(crate) fn yaml(top: &[(&str, Option<Value>)]) -> std::result::Result<String, String> {
    let mut file = Table::new();
    for (k, v) in top {
        if let Some(Value::Table(t)) = v
            && matches!(*k, "task" | "device" | "circuit")
        {
            file.insert((*k).to_owned(), Value::Table(t.clone()));
        }
    }
    let short = write(&file);
    let mut out = Vec::new();
    for (k, v) in top {
        let Some(v) = v else { continue };
        let node = match short.get(*k) {
            Some(Value::Table(t)) if *k == "task" => Node::Map(task_nodes(t)),
            Some(s) => node(s),
            None => node(v),
        };
        out.push(((*k).to_owned(), node));
    }
    serde_saphyr::to_string(&Node::Map(out)).map_err(|e| e.to_string())
}

/// What the YAML writer writes: a table, a list, each in a block or on one line, or a value.
enum Node {
    Map(Vec<(String, Node)>),
    Seq(Vec<Node>),
    FlowMap(Vec<(String, Node)>),
    FlowSeq(Vec<Node>),
    Leaf(Value),
}

/// A value as a node: lists of plain values on one line, tables and lists of tables as blocks.
fn node(v: &Value) -> Node {
    match v {
        Value::Table(t) => Node::Map(t.iter().map(|(k, x)| (k.clone(), node(x))).collect()),
        Value::Array(a) if a.iter().any(|x| matches!(x, Value::Table(_))) => {
            Node::Seq(a.iter().map(node).collect())
        }
        Value::Array(a) => Node::FlowSeq(a.iter().map(flow).collect()),
        other => Node::Leaf(other.clone()),
    }
}

/// A value on one line.
fn flow(v: &Value) -> Node {
    match v {
        Value::Table(t) => Node::FlowMap(t.iter().map(|(k, x)| (k.clone(), flow(x))).collect()),
        Value::Array(a) => Node::FlowSeq(a.iter().map(flow).collect()),
        other => Node::Leaf(other.clone()),
    }
}

/// A task's nodes: its rectangles, circles and rings in `shapes`, one to a line, where the first
/// of their lists was; a line keeps an etch on it.
fn task_nodes(t: &Table) -> Vec<(String, Node)> {
    let simple = |k: &str| matches!(k, "rect" | "circle" | "ring");
    let mut shapes = Vec::new();
    for (k, v) in t {
        if simple(k)
            && let Value::Array(a) = v
        {
            for x in a {
                shapes.push(Node::Map(vec![(k.clone(), flow(x))]));
            }
        }
    }
    let mut out = Vec::new();
    let mut placed = false;
    for (k, v) in t {
        if simple(k) && matches!(v, Value::Array(_)) {
            if !placed {
                out.push(("shapes".to_owned(), Node::Seq(std::mem::take(&mut shapes))));
                placed = true;
            }
            continue;
        }
        out.push((k.clone(), node(v)));
    }
    out
}

impl Serialize for Node {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        struct Entries<'a>(&'a [(String, Node)]);
        impl Serialize for Entries<'_> {
            fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                let mut m = s.serialize_map(Some(self.0.len()))?;
                for (k, v) in self.0 {
                    m.serialize_entry(k, v)?;
                }
                m.end()
            }
        }
        struct Items<'a>(&'a [Node]);
        impl Serialize for Items<'_> {
            fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                let mut q = s.serialize_seq(Some(self.0.len()))?;
                for v in self.0 {
                    q.serialize_element(v)?;
                }
                q.end()
            }
        }
        match self {
            Node::Map(e) => Entries(e).serialize(s),
            Node::Seq(i) => Items(i).serialize(s),
            Node::FlowMap(e) => serde_saphyr::FlowMap(Entries(e)).serialize(s),
            Node::FlowSeq(i) => serde_saphyr::FlowSeq(Items(i)).serialize(s),
            Node::Leaf(v) => v.serialize(s),
        }
    }
}
