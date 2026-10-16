//! Calls of the façade described as data, with their results: [`conformance`].

use std::path::Path;

use serde_json::{Value, json};

use super::{
    CrossSection, DeviceDesign, DeviceFigure, DeviceGeometry, DeviceInfo, DeviceSpectrum,
    IndexModelInfo, IndexOptions, JobSource, MaterialInfo, SParameters, SlabMode, Spectrum, Stop,
    VectorMode, check_job, circuit_spectrum, device_derived, device_geometry, device_job,
    device_spectrum, devices, fdfd_s_parameters, group_index, materials, read_touchstone,
    refractive_index, run_job, slab_modes, vector_modes, write_touchstone,
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

fn device_info(d: &DeviceInfo) -> Value {
    json!({
        "kind": d.kind,
        "about": d.about,
        "parameters": d.parameters.iter().map(|p| json!({
            "name": p.name, "unit": p.unit, "default": p.default, "min": p.min, "max": p.max,
        })).collect::<Vec<_>>(),
        "fidelities": d.fidelities.iter().map(|m| json!({
            "fidelity": m.fidelity,
            "name": m.name,
            "write_up": m.write_up,
            "papers": m.papers,
            "error": m.error.as_ref().map(|e| json!({
                "quantity": e.quantity, "bound": e.bound, "unit": e.unit, "against": e.against,
            })),
            "validity": m.validity.iter().map(|r| json!({
                "name": r.name, "min": r.min, "max": r.max,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "ports": d.ports,
    })
}

fn device_spectrum_(s: &DeviceSpectrum) -> Value {
    let n = s.ports.len();
    json!({
        "ports": s.ports,
        "wavelength_um": s.wavelength_um,
        "s": complexes(&s.s, &[s.wavelength_um.len(), n, n]),
        "fidelity": s.fidelity,
        "grid": s.grid,
        "source": s.source,
        "validity_um": s.validity_um.map(|(a, b)| [a, b]),
    })
}

fn figure(f: &DeviceFigure) -> Value {
    json!({
        "name": f.name, "symbol": f.symbol, "value": f.value, "unit": f.unit,
        "fidelity": f.fidelity, "grid": f.grid,
    })
}

fn points(p: &[(f64, f64)]) -> Value {
    Value::Array(p.iter().map(|&(x, y)| json!([x, y])).collect())
}

fn geometry(g: &DeviceGeometry) -> Value {
    json!({
        "layers": g.layers.iter().map(|l| json!({
            "name": l.name,
            "z_um": [l.z_um.0, l.z_um.1],
            "material": l.material,
            "background": l.background,
            "polygons": l.polygons.iter().map(|p| json!({
                "outer": points(&p.outer),
                "holes": p.holes.iter().map(|h| points(h)).collect::<Vec<_>>(),
                "etch_depth_um": p.etch_depth_um,
                "sidewall_deg": p.sidewall_deg,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "substrate": g.substrate,
        "cladding": g.cladding,
        "ports": g.ports.iter().map(|p| json!({
            "name": p.name,
            "position_um": [p.position_um.0, p.position_um.1],
            "direction_deg": p.direction_deg,
            "width_um": p.width_um,
            "layer": p.layer,
        })).collect::<Vec<_>>(),
    })
}

/// A design as the bindings take it: the call's keyword arguments `kind`, `values` (an object of
/// numbers, in its members' order), `fidelity` and `stack`, those not given left out.
fn design(kind: &str, values: &[(&str, f64)], fidelity: Option<&str>) -> (DeviceDesign, Value) {
    let mut d = DeviceDesign::new(kind);
    d.values = values.iter().map(|&(n, v)| (n.to_owned(), v)).collect();
    d.fidelity = fidelity.map(str::to_owned);
    let mut args = serde_json::Map::new();
    args.insert("kind".into(), json!(kind));
    if !values.is_empty() {
        let v: serde_json::Map<String, Value> = values
            .iter()
            .map(|&(n, v)| (n.to_owned(), json!(v)))
            .collect();
        args.insert("values".into(), Value::Object(v));
    }
    if let Some(f) = fidelity {
        args.insert("fidelity".into(), json!(f));
    }
    (d, Value::Object(args))
}

/// The same object with `more` members.
fn with(mut args: Value, more: &[(&str, Value)]) -> Value {
    if let Value::Object(m) = &mut args {
        for (k, v) in more {
            m.insert((*k).to_owned(), v.clone());
        }
    }
    args
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

/// An MZI of devices, as data: two directional couplers at their 3 dB length (their default),
/// the second's gap given as an expression, and strip arms 150 and 100 µm long, the lower's loss
/// set to 3.5 dB/cm.
fn mzi_of_devices() -> Value {
    let coupler = json!({"device": {"kind": "directional coupler", "fidelity": "analytic"}});
    let arm =
        |length: f64| json!({"device": {"kind": "waveguide", "parameters": {"length": length}}});
    let mut lower = arm(100.0);
    lower["loss"] = json!(3.5);
    json!({
        "instances": {
            "split": coupler,
            "upper": arm(150.0),
            "lower": lower,
            "combine": {"device": {"kind": "directional coupler", "parameters": {"gap": "200 nm"}}},
        },
        "connections": [["split.o3", "upper.o1"], ["split.o4", "lower.o1"],
                        ["upper.o2", "combine.o2"], ["lower.o2", "combine.o1"]],
        "ports": {"in1": "split.o1", "in2": "split.o2", "out1": "combine.o3", "out2": "combine.o4"},
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
    // more modes than the strip has: only its guided ones come back, not the window's own
    let modes = vector_modes(&cs, 1.55, 6, None, &Stop::new(None))?.unwrap_or_default();
    cases.push(case(
        "vector_modes/strip-quarter-guided",
        "vector_modes",
        &json!({"x_um": x, "y_um": y, "permittivity": eps, "wavelength_um": 1.55, "count": 6,
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
    // the same MZI of devices: directional couplers at their analytic model, strip arms
    let devices_ = mzi_of_devices();
    cases.push(case(
        "circuit_spectrum/mzi-of-devices",
        "circuit_spectrum",
        &json!({"instances": devices_["instances"], "connections": devices_["connections"],
                "ports": devices_["ports"], "wavelength_um": w}),
        &spectrum(&circuit_spectrum(&devices_.to_string(), &w)?),
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

    // devices: the library, and designs at their cheapest models, closed forms and fits: no
    // solve, so the bindings' tests stay fast
    cases.push(case(
        "devices/library",
        "devices",
        &json!({}),
        &Value::Array(devices()?.iter().map(device_info).collect()),
        &[],
    ));
    let w: Vec<f64> = (0..11).map(|k| 1.54 + 0.002 * f64::from(k)).collect();
    for (name, kind, values, fidelity) in [
        (
            "coupler-gap",
            "directional coupler",
            &[("gap", 0.25), ("length", 12.5)][..],
            Some("analytic"),
        ),
        ("ring", "all-pass ring", &[("radius", 5.0)][..], None),
        ("y-branch", "Y-branch", &[][..], Some("analytic")),
        // a 2D model: guided modes by the effective index method, exact slabs, no grid
        ("mmi-2d", "MMI 1x2", &[("length", 7.5)][..], Some("2D")),
    ] {
        let (d, args) = design(kind, values, fidelity);
        cases.push(case(
            &format!("device_spectrum/{name}"),
            "device_spectrum",
            &with(args, &[("wavelength_um", json!(w))]),
            &device_spectrum_(&device_spectrum(&d, &w)?),
            &[],
        ));
    }
    // the compact fidelity: a model fitted to the ring's analytic runs over its gap, by a "fit"
    // job run here, its file named by the design
    let fit = json!({
        "name": "conformance-fit",
        "timeout_minutes": 5,
        "device": {"kind": "all-pass ring", "fidelity": "analytic"},
        "task": {"kind": "fit", "from_um": 1.54, "to_um": 1.56, "points": 21, "held_out": 2,
                 "poles": 8, "degree": 2,
                 "axis": [{"parameter": "gap", "from": 0.15, "to": 0.25, "points": 4}]},
    });
    let fitted = run_job(&job(&fit), &scratch.join("fits"), &Stop::new(None))?
        .dir
        .join("compact.json");
    let (mut d, args) = design("all-pass ring", &[("gap", 0.2)], Some("compact"));
    d.compact = Some(fitted.clone());
    let args = with(args, &[("compact", json!(fitted.display().to_string()))]);
    cases.push(case(
        "device_spectrum/ring-compact",
        "device_spectrum",
        &with(args.clone(), &[("wavelength_um", json!(w))]),
        &device_spectrum_(&device_spectrum(&d, &w)?),
        &[],
    ));
    cases.push(case(
        "device_derived/ring-compact",
        "device_derived",
        &args,
        &Value::Array(device_derived(&d)?.iter().map(figure).collect()),
        &[],
    ));
    for (name, kind, values) in [
        ("waveguide-width", "waveguide", &[("width", 0.45)][..]),
        (
            "add-drop-ring",
            "add-drop ring",
            &[("gap", 0.18), ("radius", 8.0)][..],
        ),
        ("bend", "bend", &[][..]),
        ("rib-2d", "rib waveguide", &[("etch_depth", 0.1)][..]),
    ] {
        let (d, args) = design(kind, values, None);
        cases.push(case(
            &format!("device_derived/{name}"),
            "device_derived",
            &args,
            &Value::Array(device_derived(&d)?.iter().map(figure).collect()),
            &[],
        ));
    }
    for (name, kind, values) in [
        ("ring", "all-pass ring", &[("radius", 3.0)][..]),
        ("rib", "rib waveguide", &[][..]),
        ("mmi", "MMI 2x2", &[("taper_length", 2.0)][..]),
    ] {
        let (d, args) = design(kind, values, None);
        cases.push(case(
            &format!("device_geometry/{name}"),
            "device_geometry",
            &args,
            &geometry(&device_geometry(&d)?),
            &[],
        ));
    }
    let (d, args) = design("directional coupler", &[("gap", 0.25)], Some("analytic"));
    cases.push(case(
        "device_job/coupler-toml",
        "device_job",
        &with(
            args.clone(),
            &[("name", json!("narrow-coupler")), ("format", json!("toml"))],
        ),
        &Value::String(device_job(&d, "narrow-coupler", "toml")?),
        &[],
    ));
    // without a format, the job as data: the object its JSON is
    let text = device_job(&d, "narrow-coupler", "json")?;
    cases.push(case(
        "device_job/coupler",
        "device_job",
        &with(args, &[("name", json!("narrow-coupler"))]),
        &serde_json::from_str(&text).map_err(|e| Error::Parse {
            what: "job".into(),
            reason: e.to_string(),
        })?,
        &[],
    ));
    Ok(cases)
}
