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
    let exact = vector::guided_modes(&direct, Wavelength::um(1.55).unwrap(), 2, None).unwrap();
    // of the two asked for, one is the strip's: the other is the closed window's own, below the
    // oxide's index, and left out
    let raw = vector::modes(&direct, Wavelength::um(1.55).unwrap(), 2, None).unwrap();
    assert_eq!((modes.len(), exact.len(), raw.len()), (1, 1, 2));
    assert!(raw[1].effective_index().re < 2.085f64.sqrt());
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

/// A 1 × 0.4 µm core of ε 4 in ε 2.1025 (indices 2 and 1.45), whole, on a 50 nm grid with its
/// edges on nodes, 1 µm of cladding round it, the field zero beyond the window.
fn rectangle() -> CrossSection {
    let x: Vec<f64> = (0..=60).map(|i| -1.5 + f64::from(i) * 0.05).collect();
    let y: Vec<f64> = (0..=48).map(|j| -1.2 + f64::from(j) * 0.05).collect();
    let eps = (0..48)
        .flat_map(|j| (0..60).map(move |i| (i, j)))
        .map(|(i, j)| {
            let core = (20..40).contains(&i) && (20..28).contains(&j);
            c64::new(if core { 4.0 } else { 2.1025 }, 0.0)
        })
        .collect();
    CrossSection::new(x, y, eps)
}

#[test]
fn vector_modes_are_only_the_guided_ones_of_a_closed_window() {
    let cs = rectangle();
    let w = Wavelength::um(1.55).unwrap();
    let stop = Stop::new(None);
    let modes = vector_modes(&cs, 1.55, 12, None, &stop).unwrap().unwrap();
    let direct = cs.build().unwrap();
    let guided = vector::guided_modes(&direct, w, 12, None).unwrap();
    let raw = vector::modes(&direct, w, 12, None).unwrap();
    // fewer than asked for, each above the cladding, and the library's guided ones to the bit
    assert!(!modes.is_empty() && modes.len() < 12, "{}", modes.len());
    assert_eq!(modes.len(), guided.len());
    for (m, g) in modes.iter().zip(&guided) {
        assert_eq!(bits(m.effective_index), bits(g.effective_index()));
        assert!(m.effective_index.re > 1.45);
    }
    // what was left out: the window's own, below the cladding
    assert_eq!(raw.len(), 12);
    assert!(
        raw[guided.len()..]
            .iter()
            .all(|m| m.effective_index().re < 1.45)
    );
}

#[test]
fn vector_modes_in_an_open_window_are_every_one_solved() {
    // with PMLs the window is open: every mode of the count comes back, as the library's
    let mut cs = rectangle();
    cs.pml_um = [0.5; 4];
    cs.pml_strength = 4.0;
    let w = Wavelength::um(1.55).unwrap();
    let modes = vector_modes(&cs, 1.55, 8, None, &Stop::new(None))
        .unwrap()
        .unwrap();
    let raw = vector::modes(&cs.build().unwrap(), w, 8, None).unwrap();
    assert_eq!(modes.len(), 8);
    for (m, r) in modes.iter().zip(&raw) {
        assert_eq!(bits(m.effective_index), bits(r.effective_index()));
    }
    assert!(modes.iter().any(|m| m.effective_index.re < 1.45));
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

/// A run's tables, as data and as files: the effective indices its events recorded, to the bit,
/// and the CSV `export_run` writes the one the run wrote itself.
#[test]
fn a_runs_tables_are_its_events_numbers_to_the_bit() {
    let scratch = Scratch::new();
    let job = JobSource::Text {
        text: conformance::modes_job().to_string(),
        format: "json".into(),
    };
    let run = run_job(&job, &scratch.0.join("runs"), &Stop::new(None)).unwrap();
    let indices: Vec<[f64; 2]> = run
        .events
        .iter()
        .map(|e| serde_json::from_str::<serde_json::Value>(e).unwrap())
        .filter(|e| e["type"] == "mode")
        .map(|e| {
            let n = &e["effective_index"];
            [n[0].as_f64().unwrap(), n[1].as_f64().unwrap()]
        })
        .collect();
    assert!(!indices.is_empty());
    let tables = run_tables(&run.dir).unwrap();
    let modes = tables.iter().find(|t| t.name == "modes").unwrap();
    assert_eq!(modes.file, "modes.csv");
    let column = |name: &str| {
        modes
            .columns
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .numbers
            .clone()
            .unwrap()
    };
    let (re, im) = (column("n_eff_re"), column("n_eff_im"));
    for (k, n) in indices.iter().enumerate() {
        assert_eq!(re[k].to_bits(), n[0].to_bits());
        assert_eq!(im[k].to_bits(), n[1].to_bits());
    }
    let label = modes.columns.iter().find(|c| c.name == "label").unwrap();
    assert!(label.numbers.is_none() && label.texts.is_some());

    let out = scratch.0.join("out");
    let written = export_run(&run.dir, None, Some(&out)).unwrap();
    assert!(written.contains(&out.join("modes.csv")));
    assert_eq!(
        std::fs::read_to_string(out.join("modes.csv")).unwrap(),
        std::fs::read_to_string(run.dir.join("modes.csv")).unwrap()
    );
    assert!(export_run(&run.dir, Some(&["fields".to_owned()]), None).is_err());
}

/// A run's field files: each one's size estimated before writing it again at a precision and a
/// level, the size it then has, to the byte (the mode's few members deflate whole); its fields
/// the f32s of the run's own, and never written over the run's folder.
#[test]
fn a_runs_fields_are_estimated_then_written_again_as_asked() {
    let scratch = Scratch::new();
    let job = JobSource::Text {
        text: conformance::modes_fields_job().to_string(),
        format: "json".into(),
    };
    let run = run_job(&job, &scratch.0.join("runs"), &Stop::new(None)).unwrap();
    let out = scratch.0.join("fields");
    for (precision, level) in [("f64", 0), ("f32", 0), ("f32", 9), ("f64", 1)] {
        let estimated = estimate_fields(&run.dir, precision, level).unwrap();
        assert_eq!(estimated.len(), 1);
        let f = &estimated[0];
        assert_eq!(f.file, "mode-1-of-1.npz");
        assert!(f.rewritable && f.exact);
        assert_eq!(f.precision, precision);
        assert_eq!(
            f.bytes,
            std::fs::metadata(run.dir.join(&f.file)).unwrap().len()
        );
        let written = export_fields(&run.dir, None, &out, precision, level).unwrap();
        assert_eq!(written, [out.join(&f.file)]);
        assert_eq!(std::fs::metadata(&written[0]).unwrap().len(), f.estimate);
    }
    // at the run's own precision and level, the same file
    export_fields(&run.dir, None, &out, "f64", 6).unwrap();
    assert_eq!(
        std::fs::read(out.join("mode-1-of-1.npz")).unwrap(),
        std::fs::read(run.dir.join("mode-1-of-1.npz")).unwrap()
    );
    assert!(estimate_fields(&run.dir, "f16", 6).is_err());
    assert!(export_fields(&run.dir, None, &run.dir, "f32", 6).is_err());
    assert!(export_fields(&run.dir, Some(&["field.npz".to_owned()]), &out, "f32", 6).is_err());
}

#[test]
fn the_conformance_cases_are_computed_and_name_their_calls() {
    let scratch = Scratch::new();
    let cases = conformance(&scratch.0).unwrap();
    let functions: std::collections::BTreeSet<&str> =
        cases.iter().map(|c| c.function.as_str()).collect();
    for f in facade_functions() {
        match THROUGH.iter().find(|&&(g, _, _)| g == f) {
            // reached through another binding function's argument: a case passes it
            Some(&(_, binding, argument)) => assert!(
                cases.iter().any(|c| c.function == binding
                    && serde_json::from_str::<serde_json::Value>(&c.args).unwrap()[argument]
                        .is_number()),
                "no case calls {binding} with {argument}, which calls {f}"
            ),
            None => assert!(functions.contains(f), "no case calls {f}"),
        }
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
    // six modes asked of the strip's quarter: its guided ones only, each above the oxide
    let many = cases
        .iter()
        .find(|c| c.name == "vector_modes/strip-quarter-guided")
        .unwrap();
    let result: serde_json::Value = serde_json::from_str(&many.result).unwrap();
    let modes = result.as_array().unwrap();
    assert!(!modes.is_empty() && modes.len() < 6, "{}", modes.len());
    for m in modes {
        let n = m["effective_index"][0].as_f64().unwrap();
        assert!(n > 2.085f64.sqrt(), "{n}");
    }
}

/// The façade's functions the bindings reach through another's argument, not by a function of
/// their own: (the façade's function, the bindings' function, its argument).
/// [`device_spectrum_until`] is `device_spectrum` with a `timeout_s`, the [`Stop`]'s deadline.
pub(super) const THROUGH: [(&str, &str, &str); 1] =
    [("device_spectrum_until", "device_spectrum", "timeout_s")];

/// The façade's functions, read from its source: every `pub fn` at the start of a line (not a
/// method), but `conformance`, which lists calls of the others.
fn facade_functions() -> Vec<&'static str> {
    let sources = [
        include_str!("../facade.rs"),
        include_str!("circuit.rs"),
        include_str!("conformance.rs"),
        include_str!("device.rs"),
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
    // docs/features.md says what Python reaches: a function added to the façade (and
    // so to the Python package) goes in its table, by name
    let table = include_str!("../../docs/features.md");
    let functions = facade_functions();
    assert_eq!(functions.len(), 22, "{functions:?}");
    for f in functions {
        assert!(
            table.contains(&format!("`{f}`")),
            "docs/features.md doesn't name the façade's {f}"
        );
    }
}

fn design(kind: &str, values: &[(&str, f64)]) -> DeviceDesign {
    let mut d = DeviceDesign::new(kind);
    d.values = values.iter().map(|&(n, v)| (n.to_owned(), v)).collect();
    d
}

#[test]
fn the_devices_are_the_librarys() {
    let all = devices().unwrap();
    let library = crate::device::library();
    assert_eq!(all.len(), library.len());
    for (info, d) in all.iter().zip(&library) {
        assert_eq!(info.kind, d.kind());
        assert_eq!(info.parameters.len(), d.parameters().len());
        for (p, q) in info.parameters.iter().zip(d.parameters()) {
            assert_eq!((&p.name, &p.unit), (&q.name, &q.unit));
            assert_eq!(
                [p.default, p.min, p.max].map(f64::to_bits),
                [q.default, q.min, q.max].map(f64::to_bits)
            );
        }
        let fidelities: Vec<String> = d
            .fidelities()
            .iter()
            .map(|m| m.fidelity.to_string())
            .collect();
        let ours: Vec<&String> = info.fidelities.iter().map(|m| &m.fidelity).collect();
        assert_eq!(ours, fidelities.iter().collect::<Vec<_>>());
        // a design naming no fidelity takes the first, the cheapest
        let spec = crate::device::Spec::new(d.kind());
        assert_eq!(info.fidelities[0].fidelity, spec.fidelity.to_string());
    }
    let coupler = all
        .iter()
        .find(|d| d.kind == "directional coupler")
        .unwrap();
    assert_eq!(coupler.ports, ["o1", "o2", "o3", "o4"]);
    assert!(
        coupler
            .parameters
            .iter()
            .any(|p| p.name == "gap" && p.unit == "µm")
    );
}

#[test]
fn a_device_spectrum_is_its_components_to_the_bit() {
    use crate::device::{Device, StripCoupler, Values};
    let w = [1.5, 1.55, 1.6];
    let s = device_spectrum(&design("directional coupler", &[("gap", 0.25)]), &w).unwrap();
    let device = StripCoupler::new();
    let values = Values::new(&device, &[("gap", 0.25)]).unwrap();
    let built = device
        .component(&values, crate::circuit::Fidelity::Analytic)
        .unwrap();
    let n = built.component.ports().len();
    assert_eq!(s.ports.len(), n);
    assert_eq!(s.wavelength_um, w);
    assert_eq!(s.fidelity, "analytic");
    assert_eq!(s.grid, built.figures[0].grid);
    for (k, &wk) in w.iter().enumerate() {
        let m = built.s_matrix(Wavelength::um(wk).unwrap()).unwrap();
        for q in 0..n {
            for p in 0..n {
                assert_eq!(bits(s.s[(k * n + q) * n + p]), bits(m[(q, p)]));
            }
        }
    }
}

#[test]
fn a_waveguides_analytic_index_is_the_librarys() {
    use crate::device::{Device, StripWaveguide, Values};
    let figures = device_derived(&design("waveguide", &[("width", 0.45)])).unwrap();
    let device = StripWaveguide::new();
    let values = Values::new(&device, &[("width", 0.45)]).unwrap();
    let direct = device
        .derived(&values, crate::circuit::Fidelity::Analytic)
        .unwrap();
    assert_eq!(figures.len(), direct.len());
    for (f, g) in figures.iter().zip(&direct) {
        assert_eq!((&f.symbol, &f.unit, &f.grid), (&g.symbol, &g.unit, &g.grid));
        assert_eq!(f.value.to_bits(), g.value.to_bits());
        assert_eq!(f.fidelity, "analytic");
    }
    // a 450 nm strip's n_eff at 1.55 µm, near the book's (Chrostowski and Hochberg, Fig. 3.9)
    let n = figures.iter().find(|f| f.symbol == "n_eff").unwrap().value;
    assert!((2.2..2.5).contains(&n), "{n}");
}

#[test]
fn a_device_geometry_is_its_structures_polygons() {
    use crate::device::{Device, StripRing, Values};
    use crate::geometry::Region;
    let g = device_geometry(&design("all-pass ring", &[("radius", 3.0)])).unwrap();
    let device = StripRing::all_pass();
    let values = Values::new(&device, &[("radius", 3.0)]).unwrap();
    let direct = device.geometry(&values).unwrap();
    let s = &direct.structure;
    assert_eq!(g.layers.len(), s.stack().layers().len());
    for layer in &g.layers {
        let shapes = s.shapes(&layer.name);
        let polygons: usize = shapes
            .iter()
            .map(|sh| {
                sh.polygons(crate::geometry::Tolerance::default())
                    .contours
                    .len()
            })
            .sum();
        assert_eq!(layer.polygons.len(), polygons, "{}", layer.name);
        assert!(layer.z_um.1 > layer.z_um.0);
    }
    let si = g.layers.iter().find(|l| l.name == "Si").unwrap();
    assert_eq!(si.material, "Si");
    // the ring is a polygon with a hole, its radius within 1 nm
    let ring = si.polygons.iter().find(|p| !p.holes.is_empty()).unwrap();
    assert!(ring.etch_depth_um.is_none() && ring.sidewall_deg == 90.0);
    assert_eq!(g.ports.len(), direct.ports.len());
    for (p, q) in g.ports.iter().zip(&direct.ports) {
        assert_eq!(p.name, q.name);
        assert_eq!(p.position_um, (q.position.x.to_um(), q.position.y.to_um()));
        assert_eq!(p.width_um.to_bits(), q.width.to_um().to_bits());
    }
    // a rib: its layer etched part of the way
    let rib = device_geometry(&DeviceDesign::new("rib waveguide")).unwrap();
    let core = rib.layers.iter().find(|l| !l.polygons.is_empty()).unwrap();
    let depth = core.polygons[0].etch_depth_um.unwrap();
    assert!(depth > 0.0 && depth < core.z_um.1 - core.z_um.0, "{depth}");
    // another stack's core
    let mut nitride = DeviceDesign::new("waveguide");
    nitride.stack = Some("nitride".into());
    let g = device_geometry(&nitride).unwrap();
    assert!(
        g.layers
            .iter()
            .any(|l| l.name == "SiN" && !l.polygons.is_empty())
    );
}

#[test]
fn a_device_job_runs_to_its_spectrum() {
    let scratch = Scratch::new();
    let d = design("all-pass ring", &[("radius", 5.0)]);
    let toml = device_job(&d, "ring-5um", "toml").unwrap();
    assert!(
        toml.contains("[device]") && toml.contains("radius = \"5.0\""),
        "{toml}"
    );
    let json = device_job(&d, "ring-5um", "json").unwrap();
    let source = JobSource::Text {
        text: json,
        format: "json".into(),
    };
    check_job(&source).unwrap();
    let run = run_job(&source, &scratch.0, &Stop::new(None)).unwrap();
    let events: Vec<serde_json::Value> = run
        .events
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect();
    let s: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| e["type"] == "s_parameters")
        .collect();
    assert_eq!(s.len(), 101);
    let w: Vec<f64> = s
        .iter()
        .map(|e| e["wavelength_um"].as_f64().unwrap())
        .collect();
    let direct = device_spectrum(&d, &w).unwrap();
    let n = direct.ports.len();
    for (k, e) in s.iter().enumerate() {
        for q in 0..n {
            for p in 0..n {
                let v = &e["s"][q][p];
                let z = direct.s[(k * n + q) * n + p];
                assert_eq!(v[0].as_f64().unwrap().to_bits(), z.re.to_bits());
                assert_eq!(v[1].as_f64().unwrap().to_bits(), z.im.to_bits());
            }
        }
    }
    assert!(device_job(&d, "ring", "ini").is_err());
}

#[test]
fn a_design_names_what_is_wrong() {
    let e = |d: &DeviceDesign| device_derived(d).unwrap_err().to_string();
    assert!(e(&DeviceDesign::new("laser")).contains("waveguide"));
    let m = e(&design("waveguide", &[("gap", 0.2)]));
    assert!(m.contains("no parameter gap") && m.contains("width"), "{m}");
    assert!(e(&design("waveguide", &[("width", 50.0)])).contains("width"));
    assert!(e(&design("waveguide", &[("width", f64::NAN)])).contains("finite"));
    let mut d = DeviceDesign::new("waveguide");
    d.fidelity = Some("4D".into());
    assert!(e(&d).contains("analytic"));
    d.fidelity = Some("2D".into());
    assert!(e(&d).contains("2D"), "{}", e(&d));
    d.fidelity = None;
    d.stack = Some("sapphire".into());
    assert!(e(&d).contains("nitride"));
    assert!(device_spectrum(&DeviceDesign::new("waveguide"), &[0.0]).is_err());
}

/// An MZI of devices as a netlist names them: two directional couplers and two strip arms.
const MZI_OF_DEVICES: &str = r#"{
    "instances": {
        "split": {"device": {"kind": "directional coupler", "fidelity": "analytic",
                             "parameters": {"gap": 0.25, "length": "w/0.02", "w": "0.45"}}},
        "upper": {"device": {"kind": "waveguide", "parameters": {"length": 150.0}}},
        "lower": {"device": {"kind": "waveguide", "parameters": {"length": 100}}, "loss": 3.5},
        "combine": {"kind": "coupler", "coupling": 0.5}
    },
    "connections": [["split.o3", "upper.o1"], ["split.o4", "lower.o1"],
                    ["upper.o2", "combine.o2"], ["lower.o2", "combine.o1"]],
    "ports": {"in": "split.o1", "in2": "split.o2", "out1": "combine.o3", "out2": "combine.o4"}
}"#;

#[test]
fn a_netlist_of_devices_is_the_chips_to_the_bit() {
    use crate::device::Spec;
    use crate::job::{ChipDesign, ChipInstance};
    let w = [1.54, 1.55, 1.56];
    let s = circuit_spectrum(MZI_OF_DEVICES, &w).unwrap();
    // the same chip as a chip file holds it (P10's model): each device a [device] table, its
    // parameters' expressions as the netlist's numbers read
    let spec = |kind: &str, fidelity: Option<&str>, parameters: &[(&str, &str)]| {
        let mut s = Spec::new(kind);
        if let Some(f) = fidelity {
            s.fidelity = crate::device::fidelity(f).unwrap();
        }
        s.parameters = parameters
            .iter()
            .map(|&(n, e)| (n.to_owned(), e.to_owned()))
            .collect();
        s
    };
    let chip = ChipDesign::new()
        .instance(ChipInstance::device(
            "split",
            spec(
                "directional coupler",
                Some("analytic"),
                &[("gap", "0.25"), ("length", "w/0.02"), ("w", "0.45")],
            ),
        ))
        .instance(ChipInstance::device(
            "upper",
            spec("waveguide", None, &[("length", "150.0")]),
        ))
        .instance(
            ChipInstance::device("lower", spec("waveguide", None, &[("length", "100")]))
                .with_value("loss", 3.5),
        )
        .instance(ChipInstance::component("combine", "coupler").with_value("coupling", 0.5))
        .connect("split.o3", "upper.o1")
        .connect("split.o4", "lower.o1")
        .connect("upper.o2", "combine.o2")
        .connect("lower.o2", "combine.o1")
        .port("in", "split.o1")
        .port("in2", "split.o2")
        .port("out1", "combine.o3")
        .port("out2", "combine.o4");
    let wavelengths: Vec<Wavelength> = w.iter().map(|&w| Wavelength::um(w).unwrap()).collect();
    let direct = chip.circuit().unwrap().spectrum(&wavelengths).unwrap();
    assert_eq!(s.ports, direct.ports());
    for (k, m) in direct.matrices().iter().enumerate() {
        for q in 0..4 {
            for p in 0..4 {
                assert_eq!(bits(s.s[(k * 4 + q) * 4 + p]), bits(m[(q, p)]));
            }
        }
    }
}

#[test]
fn a_netlist_of_one_device_is_its_spectrum() {
    let netlist = r#"{
        "instances": {"dc": {"device": {"kind": "directional coupler",
                                        "parameters": {"gap": 0.3}}}},
        "ports": {"o1": "dc.o1", "o2": "dc.o2", "o3": "dc.o3", "o4": "dc.o4"}
    }"#;
    let w = [1.5, 1.55, 1.6];
    let s = circuit_spectrum(netlist, &w).unwrap();
    let d = device_spectrum(&design("directional coupler", &[("gap", 0.3)]), &w).unwrap();
    assert_eq!(s.ports, d.ports);
    assert_eq!(
        s.s.iter().map(|&z| bits(z)).collect::<Vec<_>>(),
        d.s.iter().map(|&z| bits(z)).collect::<Vec<_>>()
    );
}

#[test]
fn a_device_in_a_netlist_names_what_is_wrong() {
    let w = [1.55];
    let message = |instance: &str| {
        let text = format!(r#"{{"instances": {{"a": {instance}}}, "ports": {{"x": "a.o1"}}}}"#);
        circuit_spectrum(&text, &w).unwrap_err().to_string()
    };
    let e = message(r#"{"device": {"kind": "laser"}}"#);
    assert!(e.contains("\"a\"") && e.contains("waveguide"), "{e}");
    let e = message(r#"{"device": {"kind": "waveguide"}, "kind": "bend"}"#);
    assert!(e.contains("not both"), "{e}");
    let e = message(r#"{"device": "waveguide"}"#);
    assert!(e.contains("[device] table"), "{e}");
    let e = message(r#"{"device": {"kind": "waveguide", "parameters": {"width": 50}}}"#);
    assert!(e.contains("width"), "{e}");
    let e = message(r#"{"device": {"kind": "waveguide"}, "loss": "lots"}"#);
    assert!(e.contains("loss"), "{e}");
}

#[test]
fn a_compact_model_is_the_designs_compact_fidelity() {
    let scratch = Scratch::new();
    let fit = r#"{"name": "facade-fit", "timeout_minutes": 5,
        "device": {"kind": "all-pass ring", "fidelity": "analytic"},
        "task": {"kind": "fit", "from_um": 1.54, "to_um": 1.56, "points": 21, "held_out": 2,
                 "poles": 8, "degree": 2,
                 "axis": [{"parameter": "gap", "from": 0.15, "to": 0.25, "points": 4}]}}"#;
    let source = JobSource::Text {
        text: fit.into(),
        format: "json".into(),
    };
    let file = run_job(&source, &scratch.0, &Stop::new(None))
        .unwrap()
        .dir
        .join("compact.json");
    let mut d = design("all-pass ring", &[("gap", 0.21)]);
    d.fidelity = Some("compact".into());
    d.compact = Some(file.clone());
    let w = [1.545, 1.55, 1.555];
    let s = device_spectrum(&d, &w).unwrap();
    assert_eq!(s.fidelity, "compact");
    // the library's own: the design naming the file
    let mut spec = crate::device::Spec::new("all-pass ring");
    spec.fidelity = crate::circuit::Fidelity::Compact;
    spec.parameters = vec![("gap".into(), "0.21".into())];
    spec.compact = Some(file.display().to_string());
    let (device, design) = spec.build().unwrap();
    let built = device.component(design.values(), spec.fidelity).unwrap();
    let n = built.component.ports().len();
    for (k, &wk) in w.iter().enumerate() {
        let m = built.s_matrix(Wavelength::um(wk).unwrap()).unwrap();
        for q in 0..n {
            for p in 0..n {
                assert_eq!(bits(s.s[(k * n + q) * n + p]), bits(m[(q, p)]));
            }
        }
    }
    // close to the closed form it was fitted to, at a gap it wasn't
    d.fidelity = Some("analytic".into());
    let exact = device_spectrum(&d, &w).unwrap();
    let worst =
        s.s.iter()
            .zip(&exact.s)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max);
    assert!(worst < 1e-3, "{worst}");
    // a compact fidelity without its file says how to get one
    d.fidelity = Some("compact".into());
    d.compact = None;
    assert!(device_derived(&d).unwrap_err().to_string().contains("fit"));
}

#[test]
fn a_device_spectrum_until_its_stop_is_device_spectrums_to_the_bit() {
    let w = [1.54, 1.55, 1.56];
    for (kind, values, fidelity) in [
        ("directional coupler", &[("gap", 0.25)][..], "analytic"),
        ("MMI 1x2", &[("length", 7.5)][..], "2D"),
    ] {
        let mut d = design(kind, values);
        d.fidelity = Some(fidelity.into());
        let direct = device_spectrum(&d, &w).unwrap();
        let stop = Stop::new(Some(std::time::Duration::from_secs(600)));
        let until = device_spectrum_until(&d, &w, &stop).unwrap().unwrap();
        assert_eq!(until.ports, direct.ports);
        assert_eq!(
            (
                &until.fidelity,
                &until.grid,
                &until.source,
                until.validity_um
            ),
            (
                &direct.fidelity,
                &direct.grid,
                &direct.source,
                direct.validity_um
            )
        );
        assert_eq!(
            until.s.iter().map(|&z| bits(z)).collect::<Vec<_>>(),
            direct.s.iter().map(|&z| bits(z)).collect::<Vec<_>>(),
            "{kind}"
        );
    }
}

#[test]
fn a_device_spectrum_stops_on_request_and_at_its_deadline() {
    let mut d = design("MMI 1x2", &[]);
    d.fidelity = Some("2D".into());
    let w = [1.55];
    let stop = Stop::new(None);
    stop.request();
    assert_eq!(device_spectrum_until(&d, &w, &stop).unwrap(), None);
    let past = Stop::new(Some(std::time::Duration::ZERO));
    assert_eq!(device_spectrum_until(&d, &w, &past).unwrap(), None);
    // a design's errors come first, stopped or not
    let mut bad = design("MMI 1x2", &[("length", -1.0)]);
    bad.fidelity = Some("2D".into());
    assert!(device_spectrum_until(&bad, &w, &stop).is_err());
    assert!(device_spectrum_until(&d, &[0.0], &stop).is_err());
}

#[test]
fn the_stacks_are_the_catalogues_and_a_design_takes_each_by_name() {
    use crate::device::{LayerSpec, MaterialSpec, PRESETS, StackSpec};
    let all = stacks();
    assert_eq!(
        all.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        PRESETS
    );
    for s in &all {
        assert!(!s.about.is_empty(), "{}", s.name);
        // the layers listed build the preset's own stack, to the bit
        let m = |id: &str| MaterialSpec::id(id);
        let layers = StackSpec::Layers {
            substrate: m(&s.substrate),
            layers: s
                .layers
                .iter()
                .map(|l| LayerSpec {
                    name: l.name.clone(),
                    thickness_um: l.thickness_um,
                    material: m(&l.material),
                    background: m(&l.background),
                })
                .collect(),
            cladding: m(&s.cladding),
            core: s.core.clone(),
        };
        assert_eq!(
            layers.build().unwrap(),
            StackSpec::preset(&s.name).build().unwrap()
        );
        // each of its materials is the catalogue's, which refractive_index takes
        for id in s
            .layers
            .iter()
            .flat_map(|l| [&l.material, &l.background])
            .chain([&s.substrate, &s.cladding])
        {
            assert!(
                refractive_index(id, &[1.55], &IndexOptions::default()).is_ok(),
                "{id}"
            );
        }
        // and a design names it: its geometry's core layer is the stack's
        let mut d = design("waveguide", &[]);
        d.stack = Some(s.name.clone());
        let g = device_geometry(&d).unwrap();
        assert!(
            g.layers
                .iter()
                .any(|l| l.name == s.core && !l.polygons.is_empty()),
            "{}",
            s.name
        );
    }
    let mut d = design("waveguide", &[]);
    d.stack = Some("gaas".into());
    assert!(device_geometry(&d).is_err());
}
