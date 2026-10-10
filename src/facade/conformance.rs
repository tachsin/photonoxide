//! Calls of the façade described as data, with their results: [`conformance`].

use std::path::Path;

use serde_json::{Value, json};

use super::{
    CrossSection, IndexModelInfo, IndexOptions, JobSource, MaterialInfo, SParameters, SlabMode,
    Spectrum, Stop, VectorMode, check_job, circuit_spectrum, fdfd_s_parameters, group_index,
    materials, read_touchstone, refractive_index, run_job, slab_modes, vector_modes,
    write_touchstone,
};
use crate::Complex64 as c64;
use crate::{Error, Result};

/// A call of the façade and its result, as data.
///
/// `args` are the call's keyword arguments, by the names the bindings give them (the façade's,
/// with `wavelength_um` for one wavelength or several), as a JSON object; `result` is what it
/// returns, as JSON, in plain terms:
///
/// - a complex number is `[re, im]`;
/// - an array is nested lists, by its shape: a field `(ny, nx)`, S-matrices `(nλ, n, n)`;
/// - a result is an object of its fields by name, a mode's E and H as `ex`, `ey`, `ez`, `hx`,
///   `hy` and `hz`; a pair is a list; nothing is `null`;
/// - a run's events are their JSON objects.
///
/// `ignore` names the members whose values change from one call to the next (a run's folder,
/// its duration), to leave out of a comparison wherever they are.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Case {
    /// The case's name, e.g. `"slab_modes/book-te"`.
    pub name: String,
    /// The façade's function, e.g. `"slab_modes"`.
    pub function: String,
    /// The keyword arguments, a JSON object.
    pub args: String,
    /// The result, JSON.
    pub result: String,
    /// Members to leave out of a comparison.
    pub ignore: Vec<String>,
}

fn case(name: &str, function: &str, args: &Value, result: &Value, ignore: &[&str]) -> Case {
    Case {
        name: name.into(),
        function: function.into(),
        args: args.to_string(),
        result: result.to_string(),
        ignore: ignore.iter().map(|&s| s.to_owned()).collect(),
    }
}

fn complex(z: c64) -> Value {
    json!([z.re, z.im])
}

/// Values in C order as nested lists of the given shape (the last axis innermost).
fn nested(values: &[Value], shape: &[usize]) -> Value {
    match shape {
        [] | [_] => Value::Array(values.to_vec()),
        [first, rest @ ..] => {
            let size: usize = rest.iter().product();
            Value::Array(
                (0..*first)
                    .map(|k| nested(&values[k * size..(k + 1) * size], rest))
                    .collect(),
            )
        }
    }
}

fn complexes(values: &[c64], shape: &[usize]) -> Value {
    let flat: Vec<Value> = values.iter().map(|&z| complex(z)).collect();
    nested(&flat, shape)
}

fn model(m: &IndexModelInfo) -> Value {
    json!({
        "id": m.id,
        "name": m.name,
        "default": m.default,
        "axes": m.axes,
        "range_um": [m.range_um.0, m.range_um.1],
        "temperature_k": m.temperature_k,
        "composition": m.composition,
        "accuracy": m.accuracy,
        "sources": m.sources,
    })
}

fn material(m: &MaterialInfo) -> Value {
    json!({
        "id": m.id,
        "name": m.name,
        "formula": m.formula,
        "models": m.models.iter().map(model).collect::<Vec<_>>(),
    })
}

fn slab_mode(m: &SlabMode) -> Value {
    json!({
        "polarization": m.polarization,
        "order": m.order,
        "effective_index": m.effective_index,
        "x_um": m.x_um,
        "field": m.field,
    })
}

fn vector_mode(m: &VectorMode) -> Value {
    let shape = [m.y_um.len(), m.x_um.len()];
    json!({
        "effective_index": complex(m.effective_index),
        "te_fraction": m.te_fraction,
        "x_um": m.x_um,
        "y_um": m.y_um,
        "ex": complexes(&m.e[0], &shape),
        "ey": complexes(&m.e[1], &shape),
        "ez": complexes(&m.e[2], &shape),
        "hx": complexes(&m.h[0], &shape),
        "hy": complexes(&m.h[1], &shape),
        "hz": complexes(&m.h[2], &shape),
    })
}

/// A spectrum as plain data.
pub(super) fn spectrum(s: &Spectrum) -> Value {
    let n = s.ports.len();
    json!({
        "ports": s.ports,
        "wavelength_um": s.wavelength_um,
        "s": complexes(&s.s, &[s.wavelength_um.len(), n, n]),
    })
}

fn s_parameters(s: &SParameters) -> Value {
    let (k, n) = (s.wavelength_um.len(), s.ports.len());
    let indices: Vec<Value> = s.effective_index.iter().map(|&v| json!(v)).collect();
    json!({
        "ports": s.ports,
        "wavelength_um": s.wavelength_um,
        "s": complexes(&s.s, &[k, n, n]),
        "effective_index": nested(&indices, &[k, n]),
        "cell_um": [s.cell_um.0, s.cell_um.1],
        "polarization": s.polarization,
    })
}

/// A small modes job, as data: a 500 × 220 nm strip's fundamental mode on a 40 nm grid.
fn modes_job() -> Value {
    json!({
        "name": "conformance-modes",
        "timeout_minutes": 5,
        "task": {
            "kind": "modes",
            "stack": "soi_220",
            "wavelength_um": 1.55,
            "layer": "Si",
            "propagation": "x",
            "y_um": [-1.0, 1.0],
            "step_nm": 40.0,
            "modes": 1,
            "rect": [{"layer": "Si", "center_um": [0.0, 0.0], "size_um": [10.0, 0.5]}],
        },
    })
}

/// A small `"fdfd"` job, as data: a straight 500 nm guide between two ports on a 50 nm grid.
fn fdfd_job() -> Value {
    json!({
        "name": "conformance-fdfd",
        "timeout_minutes": 5,
        "task": {
            "kind": "fdfd",
            "stack": "soi_220",
            "wavelength_um": 1.55,
            "layer": "Si",
            "polarization": "te",
            "x_um": [-3.0, 3.0],
            "y_um": [-2.0, 2.0],
            "step_nm": 50.0,
            "rect": [{"layer": "Si", "center_um": [0.0, 0.0], "size_um": [6.0, 0.5]}],
            "port": [
                {"x_um": -1.5, "side": "left"},
                {"x_um": 1.5, "side": "right"},
            ],
        },
    })
}

fn job(data: &Value) -> JobSource {
    JobSource::Text {
        text: data.to_string(),
        format: "json".into(),
    }
}

/// A Mach-Zehnder interferometer of two 3 dB couplers and strip arms 50 µm apart, as data.
fn mzi() -> Value {
    let arm = |length: f64| {
        json!({"kind": "waveguide", "n_eff": 2.44506, "n_g": 4.172901, "wavelength_um": 1.55,
               "length": length, "loss": 2.7})
    };
    json!({
        "instances": {
            "split": {"kind": "coupler", "coupling": 0.5},
            "upper": arm(150.0),
            "lower": arm(100.0),
            "combine": {"kind": "coupler", "coupling": 0.5},
        },
        "connections": [["split.o3", "upper.o1"], ["split.o4", "lower.o1"],
                        ["upper.o2", "combine.o2"], ["lower.o2", "combine.o1"]],
        "ports": {"in1": "split.o2", "in2": "split.o1", "out1": "combine.o3", "out2": "combine.o4"},
    })
}

/// A quarter of a 500 × 220 nm silicon strip in oxide (ε 12.0 and 2.085), on a 50 × 55 nm grid,
/// an electric wall at x = 0 and a magnetic one at y = 0 for its TE-like modes.
fn strip() -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let x: Vec<f64> = (0..=20).map(|i| f64::from(i) * 0.05).collect();
    let y: Vec<f64> = (0..=15).map(|j| f64::from(j) * 0.055).collect();
    let eps = (0..15)
        .map(|j| {
            (0..20)
                .map(|i| if i < 5 && j < 2 { 12.0 } else { 2.085 })
                .collect()
        })
        .collect();
    (x, y, eps)
}

/// Calls of each function of the façade, with their results. `scratch` is a folder the calls
/// may write in: the runs of [`run_job`]'s case, and Touchstone files to read and write; the
/// cases name the files in it, so a binding checking them takes the same folder.
///
/// The results are the façade's own, and the façade's are the library's (its tests check that,
/// bit for bit): a binding that gives these results to the bit adds nothing to the numbers.
/// They are computed when asked for, on the machine that asks, so they can be compared with
/// `==` there.
///
/// # Errors
///
/// [`Error::Io`] if `scratch` can't be written, and any error of the calls (none is expected).
pub fn conformance(scratch: &Path) -> Result<Vec<Case>> {
    let mut cases = Vec::new();
    let none = IndexOptions::default();

    // materials
    cases.push(case(
        "materials/catalogue",
        "materials",
        &json!({}),
        &Value::Array(materials().iter().map(material).collect()),
        &[],
    ));
    let w = [1.31, 1.55, 2.0];
    cases.push(case(
        "refractive_index/si",
        "refractive_index",
        &json!({"material": "si", "wavelength_um": w}),
        &complexes(&refractive_index("si", &w, &none)?, &[w.len()]),
        &[],
    ));
    cases.push(case(
        "refractive_index/si-one",
        "refractive_index",
        &json!({"material": "si", "wavelength_um": 1.55}),
        &complex(refractive_index("si", &[1.55], &none)?[0]),
        &[],
    ));
    let extraordinary = IndexOptions {
        axis: Some("extraordinary".into()),
        ..IndexOptions::default()
    };
    let w = [1.064, 1.55];
    cases.push(case(
        "refractive_index/linbo3-extraordinary",
        "refractive_index",
        &json!({"material": "linbo3", "wavelength_um": w, "axis": "extraordinary"}),
        &complexes(&refractive_index("linbo3", &w, &extraordinary)?, &[w.len()]),
        &[],
    ));
    let algaas = IndexOptions {
        composition: Some(0.3),
        ..IndexOptions::default()
    };
    cases.push(case(
        "refractive_index/algaas-0.3",
        "refractive_index",
        &json!({"material": "algaas", "wavelength_um": [1.55], "composition": 0.3}),
        &complexes(&refractive_index("algaas", &[1.55], &algaas)?, &[1]),
        &[],
    ));
    cases.push(case(
        "group_index/sio2",
        "group_index",
        &json!({"material": "sio2", "wavelength_um": 1.55}),
        &json!(group_index("sio2", &[1.55], &none)?[0]),
        &[],
    ));

    // slab modes: the book's slab (Chrostowski and Hochberg, Section 3.2.2), and a thick one
    let x = [-0.1, 0.0, 0.05, 0.11, 0.22, 0.3];
    for (name, polarization, thickness) in [
        ("book-te", "te", 0.22),
        ("book-tm", "tm", 0.22),
        ("thick-te", "te", 1.0),
    ] {
        let modes = slab_modes(1.444, 3.473, 1.444, thickness, polarization, 1.55, &x)?;
        cases.push(case(
            &format!("slab_modes/{name}"),
            "slab_modes",
            &json!({"below": 1.444, "core": 3.473, "above": 1.444, "thickness_um": thickness,
                    "polarization": polarization, "wavelength_um": 1.55, "x_um": x}),
            &Value::Array(modes.iter().map(slab_mode).collect()),
            &[],
        ));
    }

    // full-vector modes
    let (x, y, eps) = strip();
    let permittivity = eps.iter().flatten().map(|&e| c64::new(e, 0.0)).collect();
    let mut cs = CrossSection::new(x.clone(), y.clone(), permittivity);
    cs.boundaries = ["electric", "zero", "magnetic", "zero"].map(String::from);
    let modes = vector_modes(&cs, 1.55, 2, None, &Stop::new(None))?.unwrap_or_default();
    cases.push(case(
        "vector_modes/strip-quarter",
        "vector_modes",
        &json!({"x_um": x, "y_um": y, "permittivity": eps, "wavelength_um": 1.55, "count": 2,
                "boundaries": cs.boundaries}),
        &Value::Array(modes.iter().map(vector_mode).collect()),
        &[],
    ));

    // jobs
    let modes = modes_job();
    check_job(&job(&modes))?;
    cases.push(case(
        "check_job/modes",
        "check_job",
        &json!({"job": modes}),
        &Value::Null,
        &[],
    ));
    let runs = scratch.join("runs");
    let run = run_job(&job(&modes), &runs, &Stop::new(None))?;
    let events = run
        .events
        .iter()
        .map(|e| serde_json::from_str(e))
        .collect::<std::result::Result<Vec<Value>, _>>()
        .map_err(|e| Error::Parse {
            what: "event".into(),
            reason: e.to_string(),
        })?;
    cases.push(case(
        "run_job/modes",
        "run_job",
        &json!({"job": modes, "runs_dir": runs.display().to_string()}),
        &json!({"dir": run.dir.display().to_string(), "events": events, "stopped": run.stopped}),
        &["dir", "seconds"],
    ));
    let fdfd = fdfd_job();
    let w = [1.5, 1.55];
    cases.push(case(
        "fdfd_s_parameters/straight",
        "fdfd_s_parameters",
        &json!({"job": fdfd, "wavelength_um": w}),
        &s_parameters(&fdfd_s_parameters(&job(&fdfd), Some(&w))?),
        &[],
    ));

    // circuits
    let netlist = mzi();
    let w: Vec<f64> = (0..11).map(|k| 1.54 + 0.002 * f64::from(k)).collect();
    let spectrum_ = circuit_spectrum(&netlist.to_string(), &w)?;
    cases.push(case(
        "circuit_spectrum/mzi",
        "circuit_spectrum",
        &json!({"instances": netlist["instances"], "connections": netlist["connections"],
                "ports": netlist["ports"], "wavelength_um": w}),
        &spectrum(&spectrum_),
        &[],
    ));

    // Touchstone
    let file = scratch.join("mzi.s4p");
    write_touchstone(&file, &spectrum_, "engineering", None)?;
    cases.push(case(
        "read_touchstone/mzi",
        "read_touchstone",
        &json!({"path": file.display().to_string(), "convention": "engineering"}),
        &spectrum(&read_touchstone(&file, "engineering")?),
        &[],
    ));
    let ours = scratch.join("written-by-rust.s4p");
    write_touchstone(&ours, &spectrum_, "physics", Some(12))?;
    let text = std::fs::read_to_string(&ours).map_err(|e| Error::Io {
        path: ours.display().to_string(),
        reason: e.to_string(),
    })?;
    cases.push(case(
        "write_touchstone/mzi",
        "write_touchstone",
        &json!({"path": scratch.join("written-by-binding.s4p").display().to_string(),
                "spectrum": spectrum(&spectrum_), "convention": "physics",
                "significant_digits": 12}),
        &Value::String(text),
        &[],
    ));
    Ok(cases)
}
