//! The façade against the library's own calls, bit for bit: each of its functions gives what a
//! Rust program calling the library directly gets.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::circuit::Netlist;
use crate::circuit::components::{Coupler, Dispersion, Waveguide};
use crate::material::{silica, silicon};

/// A fresh folder under the system's temporary folder, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("photonoxide-facade-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bits(z: c64) -> (u64, u64) {
    (z.re.to_bits(), z.im.to_bits())
}

#[test]
fn indices_are_the_materials_own() {
    let w = [1.31, 1.55, 2.0];
    let none = IndexOptions::default();
    let si = refractive_index("si", &w, &none).unwrap();
    let ox = group_index("sio2", &w, &none).unwrap();
    for (k, &w) in w.iter().enumerate() {
        let w = Wavelength::um(w).unwrap();
        assert_eq!(bits(si[k]), bits(silicon().refractive_index(w).unwrap()));
        assert_eq!(ox[k].to_bits(), silica().group_index(w).unwrap().to_bits());
    }
    // an anisotropic crystal's extraordinary index, the catalogue's second material
    let options = IndexOptions {
        axis: Some("extraordinary".into()),
        ..IndexOptions::default()
    };
    let ne = refractive_index("linbo3", &[1.55], &options).unwrap()[0];
    let entry = catalogue::entry("linbo3").unwrap();
    let pair = entry
        .default_model()
        .unwrap()
        .materials(Conditions::default())
        .unwrap();
    let w = Wavelength::um(1.55).unwrap();
    assert_eq!(bits(ne), bits(pair[1].refractive_index(w).unwrap()));
    // a condition: AlGaAs at x = 0.3
    let options = IndexOptions {
        composition: Some(0.3),
        ..IndexOptions::default()
    };
    let n = refractive_index("algaas", &[1.55], &options).unwrap()[0];
    let algaas = catalogue::entry("algaas")
        .unwrap()
        .default_model()
        .unwrap()
        .materials(Conditions {
            temperature: None,
            composition: Some(0.3),
        });
    assert_eq!(
        bits(n),
        bits(algaas.unwrap()[0].refractive_index(w).unwrap())
    );
}

#[test]
fn indices_refuse_what_the_catalogue_lacks() {
    let none = IndexOptions::default();
    let e = refractive_index("unobtainium", &[1.55], &none).unwrap_err();
    assert!(e.to_string().contains("si"), "{e}");
    // two axes and none chosen, or one that isn't there
    let e = refractive_index("linbo3", &[1.55], &none).unwrap_err();
    assert!(e.to_string().contains("ordinary and extraordinary"), "{e}");
    let options = IndexOptions {
        axis: Some("isotropic".into()),
        ..IndexOptions::default()
    };
    assert!(refractive_index("linbo3", &[1.55], &options).is_err());
    let options = IndexOptions {
        model: Some("nobody-2026".into()),
        ..IndexOptions::default()
    };
    assert!(refractive_index("si", &[1.55], &options).is_err());
    // outside the data's range, and not a wavelength
    assert!(matches!(
        refractive_index("si", &[0.5], &none),
        Err(Error::OutsideValidity { .. })
    ));
    assert!(matches!(
        refractive_index("si", &[-1.55], &none),
        Err(Error::InvalidValue { .. })
    ));
}

#[test]
fn materials_list_the_catalogue_with_sources() {
    let all = materials();
    assert_eq!(all.len(), catalogue::catalogue().len());
    let si = all.iter().find(|m| m.id == "si").unwrap();
    let li = &si.models[0];
    assert!(li.default);
    assert_eq!(li.axes, ["isotropic"]);
    assert_eq!(li.range_um, (1.2, 14.0));
    assert!(
        li.sources[0].contains("doi:10.1063/1.555624"),
        "{:?}",
        li.sources
    );
    let linbo3 = all.iter().find(|m| m.id == "linbo3").unwrap();
    assert_eq!(linbo3.models[0].axes, ["ordinary", "extraordinary"]);
}

#[test]
fn slab_modes_are_the_slabs() {
    let x = [-0.1, 0.0, 0.11, 0.3];
    let modes = slab_modes(1.444, 3.473, 1.444, 0.22, "TE", 1.55, &x).unwrap();
    let slab = Slab::new(1.444, 3.473, 1.444, Length::um(0.22)).unwrap();
    let exact = slab.modes(Polarization::Te, Wavelength::um(1.55).unwrap());
    assert_eq!(modes.len(), exact.len());
    for (m, e) in modes.iter().zip(&exact) {
        assert_eq!(m.polarization, "te");
        assert_eq!(m.order, e.order());
        assert_eq!(m.effective_index.to_bits(), e.effective_index().to_bits());
        for (k, &xk) in x.iter().enumerate() {
            assert_eq!(m.field[k].to_bits(), e.field(Length::um(xk)).0.to_bits());
        }
    }
    // the validation report's mode/slab-te-book: 2.84482 against the book's 2.845
    assert!((modes[0].effective_index - 2.845).abs() < 5e-4);
    assert!(slab_modes(1.444, 3.473, 1.444, 0.22, "te-ish", 1.55, &x).is_err());
    assert!(slab_modes(1.444, 3.473, 1.444, 0.22, "te", 0.0, &x).is_err());
}

#[test]
fn vector_modes_are_the_solvers_in_c_order() {
    // a strip's quarter, uneven in x and y so that a transposition would show
    let x: Vec<f64> = (0..=16).map(|i| f64::from(i) * 0.0625).collect();
    let y: Vec<f64> = (0..=12).map(|j| f64::from(j) * 0.0625).collect();
    let core = |i: usize, j: usize| i < 4 && j < 2;
    let (nx, ny) = (16, 12);
    let eps = |i, j| c64::new(if core(i, j) { 12.0 } else { 2.085 }, 0.0);
    let flat: Vec<c64> = (0..ny)
        .flat_map(|j| (0..nx).map(move |i| eps(i, j)))
        .collect();
    let mut cs = CrossSection::new(x.clone(), y.clone(), flat);
    cs.boundaries = ["electric", "zero", "magnetic", "zero"].map(String::from);
    let modes = vector_modes(&cs, 1.55, 2, None, &Stop::new(None))
        .unwrap()
        .unwrap();

    let cells = (0..nx)
        .flat_map(|i| (0..ny).map(move |j| (i, j)))
        .map(|(i, j)| vector::Permittivity::isotropic(eps(i, j)))
        .collect();
    let direct = vector::CrossSection::new(x, y, cells)
        .unwrap()
        .with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            south: Boundary::MagneticWall,
            ..Boundaries::default()
        })
        .unwrap();
    let exact = vector::modes(&direct, Wavelength::um(1.55).unwrap(), 2, None).unwrap();
    assert_eq!(modes.len(), 2);
    for (m, e) in modes.iter().zip(&exact) {
        assert_eq!(bits(m.effective_index), bits(e.effective_index()));
        assert_eq!(m.te_fraction.to_bits(), e.te_fraction().to_bits());
        let fields = e.fields(&direct).unwrap();
        assert_eq!(m.x_um, fields.x());
        assert_eq!(m.y_um, fields.y());
        for j in 0..ny {
            for i in 0..nx {
                for c in 0..3 {
                    assert_eq!(bits(m.e[c][j * nx + i]), bits(fields.e(i, j)[c]));
                    assert_eq!(bits(m.h[c][j * nx + i]), bits(fields.h(i, j)[c]));
                }
            }
        }
    }
    assert!(modes[0].te_fraction > 0.9);
}

#[test]
fn a_stopped_mode_solve_gives_nothing() {
    let x: Vec<f64> = (0..=8).map(|i| f64::from(i) * 0.1).collect();
    let cs = CrossSection::new(x.clone(), x, vec![c64::new(2.085, 0.0); 64]);
    let stop = Stop::new(None);
    stop.request();
    assert_eq!(vector_modes(&cs, 1.55, 1, None, &stop).unwrap(), None);
}

#[test]
fn cross_sections_refuse_the_wrong_shape_and_boundary() {
    let x = vec![0.0, 0.1, 0.2, 0.3];
    let mut cs = CrossSection::new(x.clone(), x, vec![c64::new(2.0, 0.0); 8]);
    let stop = Stop::new(None);
    let e = vector_modes(&cs, 1.55, 1, None, &stop).unwrap_err();
    assert!(e.to_string().contains("3 × 3"), "{e}");
    cs.permittivity = vec![c64::new(2.0, 0.0); 9];
    cs.boundaries[0] = "mirror".into();
    assert!(vector_modes(&cs, 1.55, 1, None, &stop).is_err());
}

const FDFD: &str = r#"
name = "facade-fdfd"
[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
polarization = "te"
x_um = [-3.0, 3.0]
y_um = [-2.0, 2.0]
step_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [6.0, 0.5]

[[task.port]]
x_um = -1.5
side = "left"

[[task.port]]
x_um = 1.5
side = "right"
"#;

#[test]
fn s_parameters_are_the_jobs_flattened() {
    let source = JobSource::Text {
        text: FDFD.into(),
        format: "toml".into(),
    };
    let w = [1.5, 1.55];
    let s = fdfd_s_parameters(&source, Some(&w)).unwrap();
    let direct = crate::job::fdfd_s_parameters(&Job::parse(FDFD).unwrap(), Some(&w)).unwrap();
    assert_eq!(s.ports, direct.ports);
    assert_eq!(s.wavelength_um, direct.wavelengths_um);
    assert_eq!(s.cell_um, direct.cell_um);
    assert_eq!(s.polarization, "te");
    let n = s.ports.len();
    for k in 0..w.len() {
        for q in 0..n {
            assert_eq!(
                s.effective_index[k * n + q].to_bits(),
                direct.effective_indices[k][q].to_bits()
            );
            for p in 0..n {
                assert_eq!(bits(s.s[(k * n + q) * n + p]), bits(direct.s[k][q][p]));
            }
        }
    }
    // a straight guide passes its mode
    assert!(s.s[2].norm() > 0.99, "{}", s.s[2]);
}

#[test]
fn a_job_runs_into_its_folder_and_returns_its_record() {
    let scratch = Scratch::new();
    let source = JobSource::Text {
        text: FDFD.into(),
        format: "toml".into(),
    };
    check_job(&source).unwrap();
    let run = run_job(&source, &scratch.0, &Stop::new(None)).unwrap();
    assert!(run.dir.starts_with(&scratch.0));
    assert_eq!(run.stopped, None);
    let events: Vec<serde_json::Value> = run
        .events
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect();
    assert_eq!(events[0]["type"], "started");
    assert_eq!(events.last().unwrap()["type"], "finished");
    // the run's S-matrix is the direct call's, to the bit
    let direct = crate::job::fdfd_s_parameters(&Job::parse(FDFD).unwrap(), None).unwrap();
    let s = events.iter().find(|e| e["type"] == "s_parameters").unwrap();
    let s21 = &s["s"][1][0];
    assert_eq!(
        s21[0].as_f64().unwrap().to_bits(),
        direct.s[0][1][0].re.to_bits()
    );
    assert_eq!(
        s21[1].as_f64().unwrap().to_bits(),
        direct.s[0][1][0].im.to_bits()
    );
    // a run asked to stop before it starts records why
    let stop = Stop::new(None);
    stop.request();
    let run = run_job(&source, &scratch.0, &stop).unwrap();
    assert_eq!(run.stopped.as_deref(), Some("requested"));
    // a job that isn't one
    let bad = JobSource::Text {
        text: "name = \"x\"\n[task]\nkind = \"nothing\"\n".into(),
        format: "toml".into(),
    };
    assert!(check_job(&bad).is_err());
    assert!(
        check_job(&JobSource::Text {
            text: FDFD.into(),
            format: "ini".into()
        })
        .is_err()
    );
}

#[test]
fn a_circuit_from_data_is_the_netlists() {
    let netlist = r#"{
        "instances": {
            "split": {"kind": "coupler", "coupling": 0.3},
            "upper": {"kind": "waveguide", "n_eff": 2.4, "n_g": 4.2, "wavelength_um": 1.55,
                      "length": 120},
            "lower": {"kind": "waveguide", "n_eff": 2.4, "n_g": 4.2, "wavelength_um": 1.55,
                      "length": 100, "loss": 3},
            "combine": {"kind": "coupler"}
        },
        "connections": [["split.o3", "upper.o1"], ["split.o4", "lower.o1"],
                        ["upper.o2", "combine.o2"], ["lower.o2", "combine.o1"]],
        "ports": {"in1": "split.o2", "in2": "split.o1", "out2": "combine.o4", "out1": "combine.o3"}
    }"#;
    let w = [1.54, 1.55, 1.56];
    let s = circuit_spectrum(netlist, &w).unwrap();

    let guide = Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2);
    let mut net = Netlist::new();
    net.add("split", Arc::new(Coupler::new())).unwrap();
    net.add("upper", Arc::new(Waveguide::new(guide))).unwrap();
    net.add("lower", Arc::new(Waveguide::new(guide))).unwrap();
    net.add("combine", Arc::new(Coupler::new())).unwrap();
    net.set("split", "coupling", 0.3).unwrap();
    net.set("upper", "length", 120.0).unwrap();
    net.set("lower", "length", 100.0).unwrap();
    net.set("lower", "loss", 3.0).unwrap();
    for (a, b) in [
        ("split.o3", "upper.o1"),
        ("split.o4", "lower.o1"),
        ("upper.o2", "combine.o2"),
        ("lower.o2", "combine.o1"),
    ] {
        net.connect(a, b).unwrap();
    }
    // the ports in the order written, not sorted
    for (name, port) in [
        ("in1", "split.o2"),
        ("in2", "split.o1"),
        ("out2", "combine.o4"),
        ("out1", "combine.o3"),
    ] {
        net.expose(name, port).unwrap();
    }
    let wavelengths: Vec<Wavelength> = w.iter().map(|&w| Wavelength::um(w).unwrap()).collect();
    let direct = net.compile().unwrap().spectrum(&wavelengths).unwrap();
    assert_eq!(s.ports, ["in1", "in2", "out2", "out1"]);
    assert_eq!(s.wavelength_um, w);
    for (k, m) in direct.matrices().iter().enumerate() {
        for q in 0..4 {
            for p in 0..4 {
                assert_eq!(bits(s.s[(k * 4 + q) * 4 + p]), bits(m[(q, p)]));
            }
        }
    }
}

#[test]
fn a_netlist_names_what_is_wrong() {
    let w = [1.55];
    let message = |text: &str| circuit_spectrum(text, &w).unwrap_err().to_string();
    let e = message(r#"{"instances": {"a": {"kind": "laser"}}, "ports": {}}"#);
    assert!(e.contains("\"a\"") && e.contains("waveguide"), "{e}");
    let e = message(
        r#"{"instances": {"a": {"kind": "waveguide", "n_eff": 2.4, "wavelength_um": 1.55}},
            "ports": {}}"#,
    );
    assert!(e.contains("needs n_g"), "{e}");
    let e = message(r#"{"instances": {"a": {"kind": "coupler", "gap": 0.2}}, "ports": {}}"#);
    assert!(e.contains("gap"), "{e}");
    let e = message(r#"{"instances": {"a": {"kind": "coupler"}}, "ports": {"x": "a.o9"}}"#);
    assert!(e.contains("o9"), "{e}");
    assert!(matches!(
        circuit_spectrum("{\"instances\": 3}", &w),
        Err(Error::Parse { .. })
    ));
}

#[test]
fn fixed_and_measured_components_come_from_data() {
    let scratch = Scratch::new();
    // a fixed 2-port, written to a file and read back as a measured component
    let netlist = r#"{
        "instances": {"t": {"kind": "fixed", "ports": ["a", "b"],
                            "s": [[[0, 0], [0.6, 0.8]], [[0.6, 0.8], [0, 0]]]}},
        "ports": {"in": "t.a", "out": "t.b"}
    }"#;
    let w = [1.5, 1.55, 1.6];
    let fixed = circuit_spectrum(netlist, &w).unwrap();
    assert_eq!(fixed.s[1], c64::new(0.6, 0.8));
    let file = scratch.0.join("fixed.s2p");
    write_touchstone(&file, &fixed, "engineering", None).unwrap();
    let back = read_touchstone(&file, "engineering").unwrap();
    // in the file's order, decreasing wavelength, the same values
    assert_eq!(back.ports, ["o1", "o2"]);
    assert_eq!(back.s[1], c64::new(0.6, 0.8));
    assert!((back.wavelength_um[0] - 1.6).abs() < 1e-12);
    let measured = format!(
        r#"{{"instances": {{"m": {{"kind": "touchstone", "file": {:?}, "convention": "engineering"}}}},
            "ports": {{"in": "m.o1", "out": "m.o2"}}}}"#,
        file.display().to_string()
    );
    let again = circuit_spectrum(&measured, &w).unwrap();
    assert_eq!(bits(again.s[1]), bits(c64::new(0.6, 0.8)));
    // the conventions are each other's conjugates, and one must be named
    let physics = read_touchstone(&file, "physics").unwrap();
    assert_eq!(physics.s[1], c64::new(0.6, -0.8));
    assert!(read_touchstone(&file, "either").is_err());
    let library = Touchstone::read(&file)
        .unwrap()
        .spectrum(Convention::Engineering)
        .unwrap();
    assert_eq!(back, Spectrum::of(&library));
    assert!(Spectrum::new(vec!["a".into()], vec![1.55], vec![]).is_err());
}

#[test]
fn the_conformance_cases_are_computed_and_name_their_calls() {
    let scratch = Scratch::new();
    let cases = conformance(&scratch.0).unwrap();
    let functions: std::collections::BTreeSet<&str> =
        cases.iter().map(|c| c.function.as_str()).collect();
    for f in facade_functions() {
        assert!(functions.contains(f), "no case calls {f}");
    }
    for c in &cases {
        let args: serde_json::Value = serde_json::from_str(&c.args).unwrap();
        assert!(args.is_object(), "{}", c.name);
        let _: serde_json::Value = serde_json::from_str(&c.result).unwrap();
    }
    // the slab's case is the book's
    let te = cases
        .iter()
        .find(|c| c.name == "slab_modes/book-te")
        .unwrap();
    let result: serde_json::Value = serde_json::from_str(&te.result).unwrap();
    let n = result[0]["effective_index"].as_f64().unwrap();
    assert!((n - 2.845).abs() < 5e-4, "{n}");
}

/// The façade's functions, read from its source: every `pub fn` at the start of a line (not a
/// method), but `conformance`, which lists calls of the others.
fn facade_functions() -> Vec<&'static str> {
    let sources = [
        include_str!("../facade.rs"),
        include_str!("circuit.rs"),
        include_str!("conformance.rs"),
    ];
    let mut names: Vec<&str> = sources
        .iter()
        .flat_map(|s| s.lines())
        .filter_map(|line| line.strip_prefix("pub fn "))
        .map(|rest| rest.split(['(', '<']).next().unwrap_or(rest))
        .filter(|&name| name != "conformance")
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn every_facade_function_is_in_the_feature_table() {
    // docs/features.md says what Python and MATLAB reach: a function added to the façade (and
    // so to the Python package) goes in its table, by name
    let table = include_str!("../../docs/features.md");
    let functions = facade_functions();
    assert_eq!(functions.len(), 11, "{functions:?}");
    for f in functions {
        assert!(
            table.contains(&format!("`{f}`")),
            "docs/features.md doesn't name the façade's {f}"
        );
    }
}
