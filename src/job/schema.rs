//! The job file's JSON Schema (draft 2020-12): what a job file may hold, for editors to complete
//! and check a job before it runs. `photonoxide schema` writes it, and `docs/job.schema.json` is
//! the copy a file's first line points an editor at.
//!
//! It is derived, not written by hand. Each task's tables are the schemas of the structs the
//! task is read into (their fields, types, defaults and doc comments, and no key they refuse);
//! the forms the resolver turns into those structs ([`super::params`]: expressions with units
//! beside each `_um` key, `[task.parameters]`, `[[task.cell]]`, ports named by their component)
//! come from the resolver's own tables; and the `[device]` table's kinds and parameters from the
//! device library. A test checks every built-in job against it, and that it refuses what the
//! library refuses: an unknown key, an unknown kind, a missing field.
//!
//! [`super::check`] stays the authority: the schema says what a file's keys and types are, and
//! what only a solve or the stack can tell (a layer the stack has, a window clear of its PMLs)
//! is left to the check.

use std::borrow::Cow;

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde_json::{Map, Value, json};

use super::params::{PATH, PLACE, SEGMENT, SHAPE, SHAPES, SHIFT, Want};

/// Where the published schema is, for a job file's `$schema` and an editor's settings.
pub const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/tachsin/photonoxide/main/docs/job.schema.json";

/// A table the library reads by hand: any keys.
pub(crate) struct Open;

impl JsonSchema for Open {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "Open".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "object" })
    }
}

/// A number, or an expression of numbers with units and the task's parameters.
pub(crate) struct NumberOrExpression;

impl JsonSchema for NumberOrExpression {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "NumberOrExpression".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "anyOf": [{ "type": "number" }, { "$ref": "#/$defs/Expression" }] })
    }
}

/// A `[[task.component]]`: a generator's component by its `type`, with its own fields.
pub(crate) struct Component;

impl JsonSchema for Component {
    fn schema_name() -> Cow<'static, str> {
        "Component".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "A component drawn by its generator (docs/methods/generators.md): its \
                type (straight, bend, taper, mmi, directional coupler, ring, grating coupler, \
                ...) and that type's fields; name, layer, etch, origin, rotate, mirror and \
                connect for any of them.",
            "required": ["type"],
            "properties": {
                "type": { "type": "string", "description": "The generator, e.g. \"straight\"." },
                "name": { "type": "string", "description": "Its name, by which its ports are found: \"name.port\"." },
                "layer": { "type": "string", "description": "The layer it is drawn on: the task's by default." }
            }
        })
    }
}

/// A `[[task.body]]`: a body in space (3D), by its `type`.
pub(crate) struct Body;

impl JsonSchema for Body {
    fn schema_name() -> Cow<'static, str> {
        "Body".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "A body in space over the structure, 3D only (docs/methods/bodies.md): \
                a sphere, a box, an extrusion of a shape, or a union, intersection or difference \
                of bodies, with its medium.",
            "required": ["type"],
            "properties": {
                "type": { "type": "string", "description": "\"sphere\", \"box\", \"extrusion\", \"union\", \"intersection\" or \"difference\"." }
            }
        })
    }
}

/// A `[task.stack]` table of layers.
pub(crate) struct Layers;

impl JsonSchema for Layers {
    fn schema_name() -> Cow<'static, str> {
        "Layers".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "A stack of your own (docs/methods/stacks.md): a substrate, a \
                cladding and layers of the materials catalogue's, bottom to top, each with its \
                thickness (thickness_um, or thickness as an expression).",
        })
    }
}

/// The job file's schema, as `docs/job.schema.json` holds it.
///
/// # Panics
///
/// Only if the library's own job types don't make the schema it expects of them (a struct
/// renamed, a resolver's table naming a field its struct lacks): a bug its tests catch.
pub fn schema() -> Value {
    let mut g = SchemaSettings::draft2020_12().into_generator();
    let job = g.subschema_for::<crate::run::JobFile>();
    let solver = kinds(
        &mut g,
        &[
            ("structure", task::<super::StructureTask>),
            ("modes", task::<super::ModesTask>),
            ("fdfd", task::<super::fdfd::FdfdTask>),
            ("fdtd", task::<super::fdtd::FdtdTask>),
            ("static", task::<super::statics::StaticTask>),
        ],
    );
    // a device's task may be empty, or left out: the design at its fidelity, its own model
    let mut device = kinds(
        &mut g,
        &[
            ("spectrum", task::<super::device::SpectrumTask>),
            ("sweep", task::<super::study::StudyTask>),
            ("corners", task::<super::study::StudyTask>),
            ("fit", task::<super::fit::FitTask>),
            ("optimize", task::<super::optimize::OptimizeTask>),
            ("fdtd", task::<super::fdtd::FdtdTask>),
        ],
    );
    let device_task = device.as_object_mut().expect("a task's schema");
    device_task.remove("required");
    device_task["allOf"]
        .as_array_mut()
        .expect("its kinds")
        .push(json!({ "anyOf": [{ "maxProperties": 0 }, { "required": ["kind"] }] }));
    let chip = kinds(
        &mut g,
        &[
            ("spectrum", task::<super::chip::ChipTask>),
            ("sweep", task::<super::chip::ChipTask>),
            ("corners", task::<super::chip::ChipTask>),
            ("optimize", task::<super::optimize::OptimizeTask>),
        ],
    );
    let mut defs = g.take_definitions(true);
    let name = job
        .as_object()
        .and_then(|o| o.get("$ref"))
        .and_then(Value::as_str)
        .and_then(|r| r.rsplit('/').next())
        .unwrap_or("JobFile")
        .to_owned();
    let mut root = defs.remove(&name).unwrap_or_else(|| json!({}));
    for def in defs.values_mut() {
        strip_null(def);
        tidy(def);
    }
    strip_null(&mut root);
    tidy(&mut root);
    expressions(&mut defs);
    resolved_tasks(&mut defs);
    defs.insert("Expression".into(), expression());
    defs.insert("Device".into(), device_table());
    defs.insert("Circuit".into(), circuit_table());
    let properties = root["properties"]
        .as_object_mut()
        .expect("JobFile's properties");
    properties.insert(
        "$schema".into(),
        json!({ "type": "string", "description": "The schema an editor checks this file with." }),
    );
    let describe = |p: &mut Map<String, Value>, key: &str, v: Value| {
        let d = p[key].get("description").cloned();
        let mut v = v;
        if let Some(d) = d {
            v["description"] = d;
        }
        p.insert(key.into(), v);
    };
    describe(properties, "device", json!({ "$ref": "#/$defs/Device" }));
    describe(properties, "circuit", json!({ "$ref": "#/$defs/Circuit" }));
    describe(properties, "task", json!({ "type": "object" }));
    let task_is = |tasks: Value| json!({ "properties": { "task": tasks } });
    let mut out = Map::new();
    out.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    out.insert("$id".into(), json!(SCHEMA_URL));
    out.insert("title".into(), json!("photonoxide job"));
    for (k, v) in root.as_object().expect("JobFile's schema") {
        if k != "$schema" && k != "title" {
            out.insert(k.clone(), v.clone());
        }
    }
    out.insert(
        "allOf".into(),
        json!([
            { "if": { "required": ["circuit"] }, "then": task_is(chip) },
            {
                "if": { "required": ["device"], "not": { "required": ["circuit"] } },
                "then": task_is(device)
            },
            {
                "if": { "not": { "anyOf": [{ "required": ["device"] }, { "required": ["circuit"] }] } },
                "then": task_is(solver)
            }
        ]),
    );
    out.insert("$defs".into(), Value::Object(defs));
    Value::Object(out)
}

/// [`schema`] as `docs/job.schema.json` holds it: indented JSON, ending in a newline.
///
/// # Panics
///
/// As [`schema`] does.
pub fn schema_text() -> String {
    let mut text = serde_json::to_string_pretty(&schema()).expect("a schema is JSON");
    text.push('\n');
    text
}

/// A function writing a task struct's schema: [`task`] at its type.
type TaskSchema = fn(&mut SchemaGenerator) -> Value;

/// A task's schema: a reference to its struct's.
fn task<T: JsonSchema>(g: &mut SchemaGenerator) -> Value {
    g.subschema_for::<T>().to_value()
}

/// A task table that is one of `kinds`: its `kind` one of their names, the rest its kind's.
fn kinds(g: &mut SchemaGenerator, kinds: &[(&str, TaskSchema)]) -> Value {
    let names: Vec<&str> = kinds.iter().map(|(k, _)| *k).collect();
    let cases: Vec<Value> = kinds
        .iter()
        .map(|(k, schema)| {
            json!({
                "if": { "properties": { "kind": { "const": k } }, "required": ["kind"] },
                "then": schema(g)
            })
        })
        .collect();
    json!({
        "type": "object",
        "required": ["kind"],
        "properties": { "kind": { "enum": names, "description": "What the task computes." } },
        "allOf": cases
    })
}

/// `"type": [T, "null"]` and `{"type": "null"}` alternatives taken out: schemars' `Option`, which a
/// job file can't write (it has no null), so an editor never offers it.
fn strip_null(v: &mut Value) {
    match v {
        Value::Object(o) => {
            if let Some(Value::Array(types)) = o.get_mut("type") {
                types.retain(|t| t != "null");
                if types.len() == 1 {
                    let t = types[0].clone();
                    o.insert("type".into(), t);
                }
            }
            for key in ["anyOf", "oneOf"] {
                if let Some(Value::Array(options)) = o.get_mut(key) {
                    options.retain(|s| s.get("type") != Some(&json!("null")));
                    if options.len() == 1 {
                        let only = options.remove(0);
                        o.remove(key);
                        if let Value::Object(only) = only {
                            for (k, x) in only {
                                o.entry(k).or_insert(x);
                            }
                        }
                    }
                }
            }
            if o.get("default") == Some(&Value::Null) {
                o.remove("default");
            }
            for x in o.values_mut() {
                strip_null(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(strip_null),
        _ => {}
    }
}

/// What an editor shows, without what only the Rust source needs: each description as plain
/// text (a doc comment's wrapped lines joined, its links to the library's items as their names),
/// and no `format` (schemars' `double`, `uint`, which say nothing a job's writer needs).
fn tidy(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.remove("format");
            if o.get("required") == Some(&json!([])) {
                o.remove("required");
            }
            if let Some(Value::String(d)) = o.get_mut("description") {
                *d = plain(d);
            }
            for (k, x) in o.iter_mut() {
                // a property named "description" or "format" is a field, not a keyword
                if k != "properties" {
                    tidy(x);
                } else if let Value::Object(p) = x {
                    p.values_mut().for_each(tidy);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(tidy),
        _ => {}
    }
}

/// A doc comment as plain text: its lines joined (a blank line kept as a paragraph's end), and
/// each link, ``[`name`]`` or `[text](target)`, its text alone.
fn plain(doc: &str) -> String {
    let joined = doc
        .split("\n\n")
        .map(|p| p.split('\n').map(str::trim).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut out = String::with_capacity(joined.len());
    let mut rest = joined.as_str();
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        // a link: [`name`] (no other bracket inside), or [text](target); anything else, such
        // as [[task.cell]] or [x, y], is text
        let link = after.find(']').filter(|&close| {
            let inner = &after[..close];
            !inner.contains('[')
                && ((inner.len() > 1 && inner.starts_with('`') && inner.ends_with('`'))
                    || after[close + 1..].starts_with('('))
        });
        let Some(close) = link else {
            out.push('[');
            rest = after;
            continue;
        };
        out.push_str(&after[..close]);
        rest = &after[close + 1..];
        // a link's target is the source's, not the reader's
        if rest.starts_with('(')
            && let Some(end) = rest.find(')')
        {
            rest = &rest[end + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// An expression's schema.
fn expression() -> Value {
    json!({
        "type": "string",
        "description": "An expression of numbers with units and the task's parameters \
            (docs/methods/expressions.md), e.g. \"500 nm\", \"w/2 + gap\", \"10 deg\"."
    })
}

/// `schema` taking an expression wherever it takes a number, in a list or alone.
fn or_expression(schema: &Value) -> Value {
    let number = |s: &Value| {
        matches!(
            s.get("type").and_then(Value::as_str),
            Some("number" | "integer")
        )
    };
    if number(schema) {
        let mut s = schema.clone();
        let description = s.as_object_mut().and_then(|o| o.remove("description"));
        let mut out = json!({ "anyOf": [s, { "$ref": "#/$defs/Expression" }] });
        if let Some(d) = description {
            out["description"] = d;
        }
        return out;
    }
    let mut s = schema.clone();
    for key in ["items", "prefixItems"] {
        match s.get_mut(key) {
            Some(Value::Array(items)) => {
                for item in items.iter_mut() {
                    *item = or_expression(item);
                }
            }
            Some(item @ Value::Object(_)) => *item = or_expression(item),
            _ => {}
        }
    }
    s
}

/// What a resolver's `Want` reads, in words.
fn wanted(want: Want) -> &'static str {
    match want {
        Want::Length | Want::Micrometres => "a length with its unit, e.g. \"500 nm\"",
        Want::Angle | Want::Radians => "an angle with its unit, e.g. \"90 deg\"",
        Want::Like(_) | Want::Number => "a number or an expression",
    }
}

/// The definition `def` taking `field` (`count` values, as `want` says) in place of its `key`,
/// and an expression in `key` too; when `field` is `key`, only the latter.
fn alias(defs: &mut Map<String, Value>, def: &str, field: &str, key: &str, want: Want) {
    let Some(Value::Object(d)) = defs.get_mut(def) else {
        panic!("the schema has no {def}");
    };
    let Some(Value::Object(properties)) = d.get_mut("properties") else {
        panic!("{def} has no properties");
    };
    let Some(original) = properties.get(key).cloned() else {
        panic!("{def} has no {key}");
    };
    properties.insert(key.into(), or_expression(&original));
    if field == key {
        return;
    }
    let mut plain = or_expression(&original);
    let what = format!(
        "{}{} in place of {key}: {}.",
        original
            .get("description")
            .and_then(Value::as_str)
            .map(|d| format!("{d} "))
            .unwrap_or_default(),
        "As expressions",
        wanted(want)
    );
    plain["description"] = json!(what);
    properties.insert(field.into(), plain);
    let required = d
        .get_mut("required")
        .and_then(Value::as_array_mut)
        .map(|r| {
            let had = r.iter().any(|x| x == key);
            r.retain(|x| x != key);
            had
        })
        .unwrap_or(false);
    let all = d
        .entry("allOf")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("allOf is a list");
    all.push(json!({ "not": { "required": [field, key] } }));
    if required {
        all.push(json!({ "anyOf": [{ "required": [key] }, { "required": [field] }] }));
    }
}

/// The resolver's forms of each table's lengths and angles: its tables' fields beside their keys.
fn expressions(defs: &mut Map<String, Value>) {
    for (kind, fields) in SHAPES {
        let def = match kind {
            "rect" => "RectSpec",
            "circle" => "CircleSpec",
            "ring" => "RingSpec",
            other => panic!("a shape table {other} the schema doesn't know"),
        };
        for &(field, _) in fields {
            alias(defs, def, field, &format!("{field}_um"), Want::Length);
        }
    }
    for (def, table) in [
        ("PathSpec", &PATH[..]),
        ("SegmentSpec", &SEGMENT[..]),
        ("ShapeSpec", &SHAPE[..]),
        ("ShiftSpec", &SHIFT[..]),
        ("PlaceSpec", &PLACE[..]),
    ] {
        for &(field, _, want, key) in table {
            alias(defs, def, field, key, want);
        }
    }
    alias(defs, "ShapeSpec", "points", "points_um", Want::Length);
    alias(defs, "ShapeSpec", "pitch", "pitch_um", Want::Length);
    // one pitch, or two: a bare value as well as a list
    for key in ["pitch", "pitch_um"] {
        let p = &mut defs["ShapeSpec"]["properties"][key];
        let one = p["items"].clone();
        let description = p.get("description").cloned();
        *p = json!({ "anyOf": [one, p.clone()] });
        if let Some(d) = description {
            p["description"] = d;
        }
    }
    alias(defs, "EtchSpec", "depth", "depth_um", Want::Length);
    alias(defs, "EtchSpec", "sidewall", "sidewall_deg", Want::Angle);
    // a sweep's ends may be expressions of the swept parameter's dimension
    for key in ["from", "to"] {
        alias(defs, "SweepSpec", key, key, Want::Like(None));
    }
    // a port or a mode source or monitor placed on a component's port, by its name
    for (def, placed) in [
        ("PortSpec", &["x_um", "side"][..]),
        ("ItemSpec", &["normal", "at_um", "direction"][..]),
    ] {
        let d = defs[def].as_object_mut().expect("an object");
        d["properties"]["port"] = json!({
            "type": "string",
            "description": format!(
                "A component's port, \"name.port\", that places it: in place of {}.",
                placed.join(", ")
            )
        });
        // what the port places is needed only without it
        let needed: Vec<Value> = match d.get_mut("required") {
            Some(Value::Array(required)) => {
                let (placed_ones, rest): (Vec<Value>, Vec<Value>) = required
                    .drain(..)
                    .partition(|k| placed.iter().any(|p| k == p));
                *required = rest;
                placed_ones
            }
            _ => Vec::new(),
        };
        let all = d
            .entry("allOf")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("allOf is a list");
        for key in placed {
            all.push(json!({ "not": { "required": ["port", key] } }));
        }
        if !needed.is_empty() {
            all.push(json!({ "anyOf": [{ "required": needed }, { "required": ["port"] }] }));
        }
    }
}

/// The tasks the resolver reads ([`super::params::task`]): `[task.parameters]` and
/// `[[task.cell]]` beside their own tables.
fn resolved_tasks(defs: &mut Map<String, Value>) {
    for def in [
        "StructureTask",
        "ModesTask",
        "FdfdTask",
        "FdtdTask",
        "StaticTask",
    ] {
        let p = defs[def]["properties"]
            .as_object_mut()
            .expect("a task's properties");
        p.insert(
            "parameters".into(),
            json!({
                "type": "object",
                "description": "Named parameters, each an expression of numbers with units and \
                    of the others, in any order: what the shapes' expressions and a sweep read.",
                "additionalProperties": { "anyOf": [{ "type": "number" }, { "$ref": "#/$defs/Expression" }] }
            }),
        );
        p.insert(
            "cell".into(),
            json!({
                "type": "array",
                "description": "Named groups of shapes, placed by a [[task.shape]] of type \"reference\".",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "shapes"],
                    "properties": {
                        "name": { "type": "string" },
                        "shapes": { "type": "array", "items": { "$ref": "#/$defs/ShapeSpec" } }
                    }
                }
            }),
        );
    }
}

/// The `[device]` table: a device of the library, by its kind, with its parameters.
fn device_table() -> Value {
    let library = crate::device::library();
    let kinds: Vec<&str> = library.iter().map(|d| d.kind()).collect();
    let cases: Vec<Value> = library
        .iter()
        .map(|d| {
            let parameters: Map<String, Value> = d
                .parameters()
                .iter()
                .map(|p| {
                    let unit = if p.unit.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", p.unit)
                    };
                    (
                        p.name.clone(),
                        json!({
                            "anyOf": [{ "type": "number" }, { "$ref": "#/$defs/Expression" }],
                            "description": format!(
                                "{}{unit}: {} by default, from {} to {}.",
                                p.name, p.default, p.min, p.max
                            )
                        }),
                    )
                })
                .collect();
            json!({
                "if": { "properties": { "kind": { "const": d.kind() } }, "required": ["kind"] },
                "then": {
                    "description": d.about(),
                    "properties": { "parameters": { "properties": parameters } }
                }
            })
        })
        .collect();
    let keys = crate::device::spec::DEVICE_KEYS;
    let mut properties = Map::new();
    for key in keys {
        let key: &str = key;
        properties.insert(
            key.to_owned(),
            match key {
                "kind" => json!({ "enum": kinds, "description": "The device, of the library's." }),
                "fidelity" => json!({
                    "enum": crate::device::spec::FIDELITIES,
                    "description": "The model it is computed by: the device's cheapest by default."
                }),
                "stack" => json!({
                    "description": "A preset (\"soi\", \"soi_220\", \"nitride\"), { preset, bottom_oxide_um }, or layers of your own.",
                    "anyOf": [{ "type": "string" }, { "type": "object" }]
                }),
                "parameters" => json!({
                    "type": "object",
                    "description": "The design's values, each a number in its unit or an expression; \
                        the others keep their defaults, and names of your own may be read by the rest.",
                    "additionalProperties": { "anyOf": [{ "type": "number" }, { "$ref": "#/$defs/Expression" }] }
                }),
                "compact" => json!({
                    "type": "string",
                    "description": "A compact model's file, for the compact fidelity."
                }),
                other => panic!("the [device] key {other} has no schema"),
            },
        );
    }
    json!({
        "type": "object",
        "description": "A device of the library (docs/methods/devices.md), computed at a fidelity, \
            studied, fitted or optimized by the task.",
        "additionalProperties": false,
        "required": ["kind"],
        "properties": properties,
        "allOf": cases
    })
}

/// The `[circuit]` table: a chip of instances, their connections and its ports.
fn circuit_table() -> Value {
    json!({
        "type": "object",
        "description": "A chip of instances (devices of the library, or the circuit's own \
            components), connected port to port, with its external ports.",
        "additionalProperties": false,
        "properties": {
            "instance": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string", "description": "Its name: its ports are \"name.o1\", ..." },
                        "device": { "$ref": "#/$defs/Device" },
                        "component": { "enum": super::chip::COMPONENTS, "description": "A component of the circuit's own, in place of a device." },
                        "values": {
                            "type": "object",
                            "description": "The component's parameters' values.",
                            "additionalProperties": { "type": "number" }
                        }
                    },
                    "oneOf": [{ "required": ["device"] }, { "required": ["component"] }]
                }
            },
            "connections": {
                "type": "array",
                "description": "Pairs of ports connected: [\"a.o2\", \"b.o1\"].",
                "items": { "type": "array", "items": { "type": "string" }, "minItems": 2, "maxItems": 2 }
            },
            "port": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "at"],
                    "properties": {
                        "name": { "type": "string", "description": "The chip's port's name." },
                        "at": { "type": "string", "description": "The instance's port it is: \"name.o1\"." }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests_support {
    pub(super) use serde_json::Value;

    pub(super) use super::*;
    pub(super) use crate::run::{Format, Job};

    /// The schema, compiled.
    pub(super) fn compiled() -> (boon::Schemas, boon::SchemaIndex) {
        let mut schemas = boon::Schemas::new();
        let mut compiler = boon::Compiler::new();
        compiler
            .add_resource(SCHEMA_URL, schema())
            .expect("the schema is JSON Schema");
        let index = compiler
            .compile(SCHEMA_URL, &mut schemas)
            .unwrap_or_else(|e| panic!("the schema compiles: {e:#}"));
        (schemas, index)
    }

    /// A job's text as the JSON the schema checks: the file as written, in JSON.
    pub(super) fn as_json(text: &str, format: Format) -> Value {
        let job = Job::parse_as(text, format).unwrap_or_else(|e| panic!("{e}\n{text}"));
        serde_json::from_str(&job.to_text(Format::Json).unwrap()).unwrap()
    }

    /// A TOML text as JSON, whatever it holds: for a job the library refuses.
    pub(super) fn raw_json(text: &str) -> Value {
        serde_json::to_value(toml::from_str::<toml::Value>(text).unwrap()).unwrap()
    }

    /// Whether the schema takes `job`, and why not.
    pub(super) fn takes(job: &Value) -> Result<(), String> {
        let (schemas, index) = compiled();
        schemas.validate(job, index).map_err(|e| format!("{e:#}"))
    }

    /// The library reads and checks the TOML job `text`.
    pub(super) fn library_takes(text: &str) {
        let job = Job::parse(text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        crate::job::check(&job).unwrap_or_else(|e| panic!("{e}\n{text}"));
    }

    /// Whether the library refuses the TOML job `text`, reading it or checking it.
    pub(super) fn library_refuses(text: &str) -> bool {
        Job::parse(text)
            .and_then(|job| crate::job::check(&job))
            .is_err()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::tests_support::*;

    #[test]
    fn every_built_in_job_is_one_the_schema_takes() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let Some(format) = Format::from_path(&path) else {
                continue;
            };
            let job = as_json(&std::fs::read_to_string(&path).unwrap(), format);
            takes(&job).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            count += 1;
        }
        assert!(count >= 20, "{count} jobs");
    }

    /// Every job the job module's own tests write out in full (a raw string starting with its
    /// name) that the library reads and checks: the forms jobs/ doesn't use, from cells and
    /// references to components, bodies, paths, etches and ports placed by name.
    #[test]
    fn every_job_the_librarys_tests_hold_that_it_takes_the_schema_takes() {
        fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    sources(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        sources(&root.join("src/job"), &mut files);
        files.push(root.join("src/run.rs"));
        files.sort();
        let (mut taken, mut tried) = (0, 0);
        for path in files {
            let source = std::fs::read_to_string(&path).unwrap();
            for block in source.split("r#\"").skip(1) {
                let Some(end) = block.find("\"#") else {
                    continue;
                };
                let text = &block[..end];
                if !text.trim_start().starts_with("name = ") {
                    continue;
                }
                tried += 1;
                let Ok(job) = Job::parse(text) else { continue };
                if crate::job::check(&job).is_err() {
                    continue;
                }
                let json = as_json(text, Format::Toml);
                takes(&json).unwrap_or_else(|e| panic!("{}: {e}\n{text}", path.display()));
                taken += 1;
            }
        }
        assert!(taken >= 25, "{taken} of {tried} jobs taken");
    }

    #[test]
    fn docs_job_schema_json_is_the_schema_the_code_writes() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/job.schema.json");
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            committed == schema_text(),
            "docs/job.schema.json is out of date: run \
             `cargo run -p photonoxide-studio --release -- schema --write docs/job.schema.json`"
        );
    }

    #[test]
    fn a_json_job_may_name_its_schema_and_it_is_left_out_when_written() {
        let text = format!(
            "{{\"$schema\": \"{SCHEMA_URL}\", \"name\": \"a\", \"task\": {{\"kind\": \"modes\"}}}}"
        );
        let job = Job::parse_as(&text, Format::Json).unwrap();
        assert_eq!(job.name(), "a");
        for format in Format::ALL {
            assert!(
                !job.to_text(format).unwrap().contains("$schema"),
                "{format:?}"
            );
        }
        // and the schema takes it, on a job it takes
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs/strip-modes.toml");
        let mut json = as_json(&std::fs::read_to_string(path).unwrap(), Format::Toml);
        json["$schema"] = Value::String(SCHEMA_URL.into());
        takes(&json).unwrap();
        let again = Job::parse_as(&json.to_string(), Format::Json).unwrap();
        crate::job::check(&again).unwrap();
    }

    #[test]
    fn the_device_table_is_the_librarys() {
        let s = schema();
        let device = &s["$defs"]["Device"];
        let kinds: Vec<&str> = device["properties"]["kind"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap())
            .collect();
        let library: Vec<String> = crate::device::library()
            .iter()
            .map(|d| d.kind().to_owned())
            .collect();
        assert_eq!(kinds, library);
        for f in crate::device::spec::FIDELITIES {
            crate::device::spec::fidelity(f).unwrap_or_else(|e| panic!("{f}: {e}"));
        }
        let keys: Vec<&String> = device["properties"].as_object().unwrap().keys().collect();
        assert_eq!(keys.len(), crate::device::spec::DEVICE_KEYS.len());
    }

    #[test]
    fn descriptions_are_plain_text() {
        assert_eq!(
            plain("Its [`crate::geometry::Path`], placed by a [[task.shape]]\nat [x, y]."),
            "Its `crate::geometry::Path`, placed by a [[task.shape]] at [x, y]."
        );
        assert_eq!(
            plain("See [the method](docs/methods/pml.md).\n\nThen"),
            "See the method.\n\nThen"
        );
        let text = schema_text();
        assert!(!text.contains("\"format\""), "a format is left");
        assert!(!text.contains("[`"), "a link is left");
    }
}

#[cfg(test)]
mod refusals {
    use super::tests_support::*;

    /// A modes job of 500 nm strip, `task` and `rect` appended to its task and its rectangle.
    fn modes(task: &str, rect: &str) -> String {
        format!(
            "name = \"m\"\n\n[task]\nkind = \"modes\"\nstack = \"soi_220\"\nwavelength_um = 1.55\n\
             layer = \"Si\"\npropagation = \"x\"\ny_um = [-1.2, 1.2]\nstep_nm = 20.0\n{task}\n\n\
             [[task.rect]]\nlayer = \"Si\"\n{rect}\n"
        )
    }

    const RECT: &str = "center_um = [0.0, 0.0]\nsize_um = [10.0, 0.5]";

    #[test]
    fn the_schema_takes_what_the_library_takes() {
        for (what, text) in [
            ("a plain modes job", modes("", RECT)),
            (
                "expressions in place of _um keys",
                modes(
                    "[task.parameters]\nw = \"500 nm\"",
                    "center = [0, \"0 um\"]\nsize = [\"10 um\", \"w\"]",
                ),
            ),
            (
                "an expression in a _um key",
                modes("", "center_um = [0.0, \"0.1\"]\nsize_um = [10.0, 0.5]"),
            ),
            (
                "an etch by expressions",
                modes(
                    "",
                    &format!("{RECT}\netch = {{ depth = \"70 nm\", sidewall = \"80 deg\" }}"),
                ),
            ),
        ] {
            let job = as_json(&text, Format::Toml);
            library_takes(&text);
            takes(&job).unwrap_or_else(|e| panic!("{what}: {e}"));
        }
    }

    #[test]
    fn the_schema_refuses_what_the_library_refuses() {
        for (what, text) in [
            ("an unknown key in a task", modes("colour = \"red\"", RECT)),
            (
                "an unknown key in a rectangle",
                modes("", &format!("{RECT}\ncolour = 1")),
            ),
            (
                "a rectangle without its size",
                modes("", "center_um = [0.0, 0.0]"),
            ),
            (
                "a center and a center_um",
                modes("", &format!("{RECT}\ncenter = [0, 0]")),
            ),
            (
                "a step that isn't a number",
                modes("", RECT).replace("step_nm = 20.0", "step_nm = \"fine\""),
            ),
            (
                "a modes job without its wavelength",
                modes("", RECT).replace("wavelength_um = 1.55\n", ""),
            ),
            (
                "an unknown kind",
                modes("", RECT).replace("kind = \"modes\"", "kind = \"mode\""),
            ),
            (
                "a modes field in a structure job",
                modes("modes = 2", RECT).replace("kind = \"modes\"", "kind = \"structure\""),
            ),
            (
                "an unknown key at the top",
                format!("colour = 1\n{}", modes("", RECT)),
            ),
            (
                "a device of no kind the library has",
                "name = \"d\"\n\n[device]\nkind = \"flux capacitor\"\n".to_owned(),
            ),
            (
                "an unknown key in a device",
                "name = \"d\"\n\n[device]\nkind = \"waveguide\"\ncolour = 1\n".to_owned(),
            ),
            (
                "a solver's kind on a device",
                "name = \"d\"\n\n[device]\nkind = \"waveguide\"\n\n[task]\nkind = \"modes\"\n"
                    .to_owned(),
            ),
        ] {
            assert!(library_refuses(&text), "{what}: the library takes it");
            let job = raw_json(&text);
            assert!(takes(&job).is_err(), "{what}: the schema takes it\n{text}");
        }
    }
}
