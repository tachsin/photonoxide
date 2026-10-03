use std::collections::HashSet;

use photonoxide::Complex64 as c64;

use super::*;

fn here() -> &'static Path {
    Path::new(".")
}

fn part(kind: &str) -> Part {
    Part {
        kind: kind.into(),
        file: None,
        convention: None,
    }
}

fn check(chip: Chip) -> Vec<Problem> {
    chip.netlist(here()).1
}

fn simulate_(chip: Chip) -> Result<Simulation, String> {
    simulate(&chip, here())
}

fn spectrum_(kind: String, values: Vec<f64>, sweep: Sweep) -> Result<SpectrumData, String> {
    component(&part(&kind), &values, sweep, here()).map(|s| SpectrumData::of(&s))
}

fn example(file: &str) -> Chip {
    let (_, text) = example_texts()
        .into_iter()
        .find(|(f, _)| *f == file)
        .expect("a built-in circuit");
    Chip::parse(text).unwrap()
}

/// The transmission of a library component between two ports at `wavelength`, its parameters
/// at `values` by name and defaults otherwise.
fn element(kind: &str, values: &[(&str, f64)], q: usize, p: usize, wavelength: Wavelength) -> c64 {
    let c = (library::entry(kind).unwrap().build)().unwrap();
    let v: Vec<f64> = c
        .parameters()
        .iter()
        .map(|par| {
            values
                .iter()
                .find(|(n, _)| *n == par.name)
                .map_or(par.default, |(_, v)| *v)
        })
        .collect();
    c.s_matrix(wavelength, &v).unwrap()[(q, p)]
}

fn at(sim: &Simulation, out: &str, input: &str) -> Vec<c64> {
    let s = &sim.spectrum;
    let q = s.ports.iter().position(|p| p == out).unwrap();
    let p = s.ports.iter().position(|p| p == input).unwrap();
    s.re[q][p]
        .iter()
        .zip(&s.im[q][p])
        .map(|(&re, &im)| c64::new(re, im))
        .collect()
}

#[test]
fn every_kind_has_its_ports_on_its_symbol() {
    let kinds = component_library().unwrap();
    assert_eq!(kinds.len(), library::entries().len());
    let mut ids = HashSet::new();
    for k in &kinds {
        assert!(ids.insert(k.id.clone()), "two kinds with id {}", k.id);
        let pins: Vec<&str> = k.symbol.pins.iter().map(|p| p.port.as_str()).collect();
        let ports: Vec<&str> = k.ports.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(pins, ports, "{}", k.id);
        let mut places = HashSet::new();
        for p in &k.symbol.pins {
            assert!(
                places.insert(((p.x * 8.0) as i64, (p.y * 8.0) as i64)),
                "{}: two pins at one place",
                k.id
            );
            // on the symbol's edge, on the grid
            assert!(
                p.x.abs() <= k.symbol.width / 2.0 && p.y.abs() <= k.symbol.height / 2.0,
                "{}",
                k.id
            );
            assert_eq!(p.x % 10.0, 0.0, "{}", k.id);
            assert_eq!(p.y % 10.0, 0.0, "{}", k.id);
        }
        for par in &k.parameters {
            assert!(
                par.min <= par.default && par.default <= par.max,
                "{}.{}",
                k.id,
                par.name
            );
        }
        let built = (library::entry(&k.id).unwrap().build)().unwrap();
        assert_eq!(library::entry_of(&built).map(|e| e.id), Some(k.id.as_str()));
    }
}

#[test]
fn a_spectrum_has_every_element_at_every_wavelength() {
    for k in component_library().unwrap() {
        // over the model's own range, if it has one
        let (from_um, to_um) = k.provenance.validity.unwrap_or((1.5, 1.6));
        let sweep = Sweep {
            from_um,
            to_um,
            points: 11,
        };
        let values: Vec<f64> = k.parameters.iter().map(|p| p.default).collect();
        let s = spectrum_(k.id.clone(), values, sweep).unwrap();
        let n = k.ports.len();
        assert_eq!(s.ports.len(), n);
        assert_eq!(s.wavelength_um.len(), 11);
        assert!((s.wavelength_um[10] - to_um).abs() < 1e-12);
        assert_eq!(s.re.len(), n);
        assert!(
            s.re.iter()
                .chain(&s.im)
                .all(|row| row.len() == n && row.iter().all(|e| e.len() == 11)),
            "{}",
            k.id
        );
        // a passive library: no component makes power
        for k_ in 0..11 {
            let m = photonoxide::circuit::SMatrix::from_fn(n, |q, p| {
                c64::new(s.re[q][p][k_], s.im[q][p][k_])
            });
            assert!(
                m.largest_singular_value().unwrap() <= 1.0 + 1e-12,
                "{}",
                k.id
            );
        }
    }
    let sweep = Sweep::default();
    let wrong = spectrum_("waveguide".into(), vec![1.0], sweep).unwrap_err();
    assert!(wrong.contains("values"), "{wrong}");
    assert!(spectrum_("nothing".into(), vec![], sweep).is_err());
    let backwards = Sweep {
        from_um: 1.6,
        to_um: 1.5,
        points: 11,
    };
    assert!(
        spectrum_("terminator".into(), vec![], backwards)
            .unwrap_err()
            .contains("upwards")
    );
}

#[test]
fn every_built_in_circuit_is_complete_and_simulates() {
    let examples = circuit_examples().unwrap();
    assert_eq!(examples.len(), 4);
    for e in examples {
        assert_eq!(format!("{}.toml", e.chip.name), e.file);
        assert!(!e.chip.about.is_empty(), "{}", e.file);
        let problems = check(e.chip.clone());
        assert!(problems.is_empty(), "{}: {problems:?}", e.file);
        let sim = simulate_(e.chip.clone()).unwrap();
        assert_eq!(sim.spectrum.wavelength_um.len(), e.chip.sweep.points);
        assert_eq!(
            sim.spectrum.ports,
            e.chip
                .ports
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
        );
        assert!(sim.checks.reciprocal);
        assert!(
            sim.checks.reciprocity < 1e-12,
            "{}: {:?}",
            e.file,
            sim.checks
        );
        assert!(
            sim.checks.largest_singular_value <= 1.0 + 1e-12,
            "{}: {:?}",
            e.file,
            sim.checks
        );
    }
}

/// The MZI of parts against its closed form: the two arms' transmissions t_u and t_l, through
/// two ideal 3 dB couplers, give S_out1,in1 = (t_u − t_l)/2.
/// The MZI of parts against its closed form: the two arms' transmissions t_u and t_l, through
/// two ideal 3 dB couplers (through √½, across i√½), give S_out1,in1 = (t_u − t_l)/2.
#[test]
fn the_mzi_from_parts_is_the_closed_form() {
    let chip = example("mzi.toml");
    let sim = simulate_(chip.clone()).unwrap();
    let s = at(&sim, "out1", "in1");
    let length = |name: &str| {
        chip.instances
            .iter()
            .find(|i| i.name == name)
            .unwrap()
            .values["length"]
    };
    let mut worst: f64 = 0.0;
    for (k, &w) in sim.spectrum.wavelength_um.iter().enumerate() {
        let w = Wavelength::um(w).unwrap();
        let upper = element("waveguide", &[("length", length("upper"))], 1, 0, w);
        let lower = element("waveguide", &[("length", length("lower"))], 1, 0, w);
        worst = worst.max((s[k] - (upper - lower) / 2.0).norm());
    }
    assert!(worst < 1e-12, "{worst}");
    // its fringes are λ²/(n_g ΔL) apart: n_g 4.17 and ΔL 50 µm give 11.5 nm at 1.55 µm
    let power: Vec<f64> = s.iter().map(|z| z.norm_sqr()).collect();
    let peaks: Vec<f64> = (1..power.len() - 1)
        .filter(|&k| power[k] > power[k - 1] && power[k] > power[k + 1])
        .map(|k| sim.spectrum.wavelength_um[k])
        .collect();
    let fsr = (peaks[peaks.len() - 1] - peaks[0]) / (peaks.len() - 1) as f64;
    let expected = 1.55f64.powi(2) / (4.172901 * 50.0);
    assert!(
        (fsr - expected).abs() < 0.02 * expected,
        "{fsr} against {expected}"
    );
}

/// The MZI of parts is the library's own MZI (`components::mzi`), its ports o2 and o3 being
/// the chip's in1 and out1.
#[test]
fn the_mzi_from_parts_is_the_librarys() {
    let chip = example("mzi.toml");
    let sim = simulate_(chip.clone()).unwrap();
    let mzi = (library::entry("mzi").unwrap().build)().unwrap();
    let values: Vec<f64> = mzi
        .parameters()
        .iter()
        .map(|p| match p.name.as_str() {
            "upper.length" => 100.0,
            "lower.length" => 150.0,
            _ => p.default,
        })
        .collect();
    for (out, input, q, p) in [
        ("out1", "in1", 2, 1),
        ("out2", "in1", 3, 1),
        ("out1", "in2", 2, 0),
    ] {
        let s = at(&sim, out, input);
        let mut worst: f64 = 0.0;
        for (k, &w) in sim.spectrum.wavelength_um.iter().enumerate() {
            let m = mzi.s_matrix(Wavelength::um(w).unwrap(), &values).unwrap();
            worst = worst.max((s[k] - m[(q, p)]).norm());
        }
        assert!(worst < 1e-12, "{out} <- {input}: {worst}");
    }
}
/// The all-pass ring of parts against its closed form, (t − a e^{iφ})/(1 − t a e^{iφ}), with
/// a e^{iφ} the ring's round trip.
/// The all-pass ring of parts is the library's all-pass ring, Bogaerts et al.'s closed form, to
/// round-off: the same coupler convention and the same guide.
#[test]
fn the_all_pass_ring_from_parts_is_the_librarys() {
    let chip = example("ring-all-pass.toml");
    let sim = simulate_(chip.clone()).unwrap();
    let s = at(&sim, "through", "in");
    let value = |inst: &str, p: &str| {
        chip.instances
            .iter()
            .find(|i| i.name == inst)
            .unwrap()
            .values[p]
    };
    let values = [
        ("length", value("ring", "length")),
        ("coupling", value("bus", "coupling")),
    ];
    let mut worst: f64 = 0.0;
    for (k, &w) in sim.spectrum.wavelength_um.iter().enumerate() {
        let closed = element("ring-all-pass", &values, 1, 0, Wavelength::um(w).unwrap());
        worst = worst.max((s[k] - closed).norm());
    }
    assert!(worst < 1e-12, "{worst}");
}

/// The add-drop ring of parts is the library's add-drop ring, in power: its through and drop
/// ports' |S|² by Bogaerts et al.'s closed forms.
#[test]
fn the_add_drop_ring_from_parts_is_the_librarys() {
    let chip = example("ring-add-drop.toml");
    let sim = simulate_(chip.clone()).unwrap();
    let value = |inst: &str, p: &str| {
        chip.instances
            .iter()
            .find(|i| i.name == inst)
            .unwrap()
            .values[p]
    };
    let length = value("east", "length") + value("west", "length");
    let values = [
        ("length", length),
        ("coupling", value("upper", "coupling")),
        ("coupling_drop", value("lower", "coupling")),
    ];
    // the library's ports: in, through, add, drop
    for (out, q) in [("through", 1), ("drop", 3)] {
        let s = at(&sim, out, "in");
        let mut worst: f64 = 0.0;
        for (k, &w) in sim.spectrum.wavelength_um.iter().enumerate() {
            let closed = element("ring-add-drop", &values, q, 0, Wavelength::um(w).unwrap());
            worst = worst.max((s[k].norm_sqr() - closed.norm_sqr()).abs());
        }
        assert!(worst < 1e-12, "{out}: {worst}");
    }
}
#[test]
fn the_splitter_shares_power_four_ways() {
    let sim = simulate_(example("splitter-1x4.toml")).unwrap();
    for out in ["out1", "out2", "out3", "out4"] {
        for z in at(&sim, out, "in") {
            assert!(
                (z.norm_sqr() - 0.25).abs() < 1e-12,
                "{out}: {}",
                z.norm_sqr()
            );
        }
    }
}
#[test]
fn a_chip_file_round_trips() {
    for (file, text) in example_texts() {
        let chip = Chip::parse(text).unwrap();
        let again = Chip::parse(&chip.to_text().unwrap()).unwrap();
        assert_eq!(again, chip, "{file}");
    }
    // a chip with a mirror, a rotation and an unwired port keeps them
    let mut chip = example("ring-add-drop.toml");
    chip.ports.push(External {
        name: "spare".into(),
        at: String::new(),
        x: 5.0,
        y: -15.0,
        rotation: 90,
    });
    assert_eq!(Chip::parse(&chip.to_text().unwrap()).unwrap(), chip);
}

#[test]
fn a_chip_round_trips_through_the_library_netlist() {
    for (file, text) in example_texts() {
        let chip = Chip::parse(text).unwrap();
        let (netlist, problems) = chip.netlist(here());
        assert!(problems.is_empty(), "{file}");
        let back = Chip::from_netlist(&chip.name, &netlist).unwrap();
        let (again, problems) = back.netlist(here());
        assert!(problems.is_empty(), "{file}: {problems:?}");
        assert_eq!(again.connections(), netlist.connections(), "{file}");
        assert_eq!(again.external(), netlist.external(), "{file}");
        for (a, b) in again.instances().iter().zip(netlist.instances()) {
            assert_eq!((&a.name, &a.values), (&b.name, &b.values), "{file}");
            assert_eq!(a.component.kind(), b.component.kind(), "{file}");
        }
        // the same circuit: the same S-matrix
        let w = Wavelength::um(1.55).unwrap();
        let s = netlist.compile().unwrap().s_matrix(w).unwrap();
        let t = again.compile().unwrap().s_matrix(w).unwrap();
        assert!(s.max_difference(&t).unwrap() < 1e-14, "{file}");
        // and every instance's kind is the library's id, with every value written out
        for (p, q) in back.instances.iter().zip(&chip.instances) {
            assert_eq!(p.kind, q.kind);
            for (name, v) in &q.values {
                assert_eq!(p.values[name], *v);
            }
        }
    }
}

#[test]
fn each_problem_says_what_it_points_at() {
    let mut chip = example("mzi.toml");
    assert!(check(chip.clone()).is_empty());

    // a port used twice: the second connection is refused, with the reason
    chip.connections
        .push(["split.o3".into(), "lower.o2".into()]);
    let problems = check(chip.clone());
    let p = &problems[0];
    assert_eq!(p.connection, Some(4));
    assert_eq!(p.port.as_deref(), Some("split.o3"));
    assert!(p.message.contains("used twice"), "{}", p.message);
    assert!(!p.dangling);
    chip.connections.pop();

    // a port connected to itself
    chip.connections
        .push(["upper.o1".into(), "upper.o1".into()]);
    let p = &check(chip.clone())[0];
    assert!(p.message.contains("itself"), "{}", p.message);
    chip.connections.pop();

    // a value out of range
    chip.instances[0].values.insert("coupling".into(), 1.5);
    let p = &check(chip.clone())[0];
    assert_eq!(
        (p.instance.as_deref(), p.parameter.as_deref()),
        (Some("split"), Some("coupling"))
    );
    chip.instances[0].values.insert("coupling".into(), 0.5);

    // a kind the library hasn't: reported once, not again for each of its connections
    chip.instances[1].kind = "flux-capacitor".into();
    let problems = check(chip.clone());
    assert_eq!(
        problems.iter().filter(|p| !p.dangling).count(),
        1,
        "{problems:?}"
    );
    assert_eq!(problems[0].instance.as_deref(), Some("upper"));
    assert!(problems[0].message.contains("flux-capacitor"));
    // and the ports it was wired to dangle now
    assert!(
        problems
            .iter()
            .any(|p| p.dangling && p.port.as_deref() == Some("split.o3"))
    );
    chip.instances[1].kind = "waveguide".into();

    // an external port wired to nothing, and the port it left dangling
    chip.ports[3].at.clear();
    let problems = check(chip.clone());
    assert_eq!(problems[0].external, Some(3));
    assert!(
        problems
            .iter()
            .any(|p| p.dangling && p.port.as_deref() == Some("combine.o4"))
    );
    assert!(simulate_(chip.clone()).unwrap_err().contains("isn't wired"));

    // two external ports of one name
    chip.ports[3].at = "combine.o4".into();
    chip.ports[3].name = "out1".into();
    let p = &check(chip.clone())[0];
    assert_eq!(p.external, Some(3));
    assert!(p.message.contains("two external ports"), "{}", p.message);
}

#[test]
fn a_chip_without_external_ports_cant_be_simulated() {
    let mut chip = Chip::new("loop");
    chip.instances.push(Placed {
        name: "w".into(),
        kind: "waveguide".into(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        values: BTreeMap::new(),
        file: None,
        convention: None,
    });
    chip.connections.push(["w.o1".into(), "w.o2".into()]);
    assert!(check(chip.clone()).is_empty());
    assert!(simulate_(chip).unwrap_err().contains("no external ports"));
}

#[test]
fn chip_files_are_saved_read_and_listed() {
    let dir = std::env::temp_dir().join(format!("photonoxide-circuits-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let chip = example("mzi.toml");
    let path = save_in(&dir, &chip, false).unwrap();
    assert_eq!(path.file_name().unwrap(), "mzi.toml");
    assert!(
        save_in(&dir, &chip, false)
            .unwrap_err()
            .starts_with("exists")
    );
    save_in(&dir, &chip, true).unwrap();
    assert_eq!(read_circuit(path.display().to_string()).unwrap(), chip);
    let listed = circuits_in(&dir);
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].name.as_str(), listed[0].instances), ("mzi", 4));
    let mut bad = chip.clone();
    bad.name = "a b/c".into();
    assert!(
        save_in(&dir, &bad, false)
            .unwrap_err()
            .contains("can't name a file")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_later_format_or_an_unknown_field_is_refused() {
    assert!(
        Chip::parse("format = 2\nname = \"x\"\n")
            .unwrap_err()
            .contains("newer")
    );
    assert!(Chip::parse("name = \"x\"\ncolour = \"red\"\n").is_err());
    let chip = Chip::parse("name = \"empty\"\n").unwrap();
    assert_eq!((chip.format, chip.sweep), (FORMAT, Sweep::default()));
}

#[test]
fn a_sweep_is_checked() {
    let s = |from_um, to_um, points| {
        Sweep {
            from_um,
            to_um,
            points,
        }
        .wavelengths()
    };
    assert_eq!(s(1.5, 1.6, 3).unwrap().len(), 3);
    assert!(s(1.5, 1.6, 1).is_err());
    assert!(s(1.5, 1.6, Sweep::MOST + 1).is_err());
    assert!(s(1.5, 1.5, 3).is_err());
    assert!(s(-1.0, 1.6, 3).is_err());
    assert!(s(f64::NAN, 1.6, 3).is_err());
}

#[test]
fn a_spectrum_is_written_as_touchstone() {
    let dir = std::env::temp_dir().join(format!("photonoxide-touchstone-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let chip = example("ring-add-drop.toml");
    let path = dir.join("ring.s4p");
    write_touchstone(&path, &solve(&chip, here()).unwrap().1).unwrap();
    let file = Touchstone::read(&path).unwrap();
    let (_, spectrum, _) = solve(&chip, here()).unwrap();
    assert_eq!(file.ports(), 4);
    assert_eq!(file.frequencies_hz.len(), chip.sweep.points);
    // the file runs in frequency, so backwards in wavelength
    let back = file.spectrum(Convention::Physics).unwrap();
    let n = back.matrices().len();
    for (k, m) in spectrum.matrices().iter().enumerate() {
        let read = &back.matrices()[n - 1 - k];
        assert!(read.max_difference(m).unwrap() < 1e-15);
        let w = file.wavelengths().unwrap()[n - 1 - k];
        assert!((w.to_um() - spectrum.wavelengths()[k].to_um()).abs() < 1e-12);
    }
    let one = dir.join("guide.s2p");
    let sweep = Sweep {
        from_um: 1.5,
        to_um: 1.6,
        points: 5,
    };
    let guide = component(&part("waveguide"), &[10.0, 2.7], sweep, here()).unwrap();
    write_touchstone(&one, &guide).unwrap();
    assert_eq!(Touchstone::read(&one).unwrap().ports(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_touchstone_file_is_a_measured_component() {
    let dir = std::env::temp_dir().join(format!("photonoxide-measured-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("measured")).unwrap();
    // the add-drop ring, written out and read back as one measured component
    let ring = example("ring-add-drop.toml");
    let (_, spectrum, _) = solve(&ring, here()).unwrap();
    let path = dir.join("measured").join("ring.s4p");
    write_touchstone(&path, &spectrum).unwrap();
    let info = measured_in(&dir, &path, TimeConvention::Physics).unwrap();
    assert_eq!(info.file.as_deref(), Some("measured/ring.s4p"));
    assert_eq!(info.ports.len(), 4);
    assert_eq!(info.provenance.fidelity, "measured");
    assert_eq!(info.provenance.validity, Some((1.53, 1.57)));

    // a chip of it: the same S-parameters at the file's wavelengths
    let mut chip = Chip::new("measured-ring");
    chip.sweep = ring.sweep;
    chip.instances.push(Placed {
        name: "ring".into(),
        kind: MEASURED.into(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        values: BTreeMap::new(),
        file: info.file.clone(),
        convention: Some(TimeConvention::Physics),
    });
    for (k, name) in spectrum.ports().iter().enumerate() {
        chip.ports.push(External {
            name: name.clone(),
            at: format!("ring.o{}", k + 1),
            x: 0.0,
            y: 0.0,
            rotation: 0,
        });
    }
    let sim = simulate(&chip, &dir).unwrap();
    let ports: Vec<&str> = spectrum.ports().iter().map(String::as_str).collect();
    for (q, out) in ports.iter().enumerate() {
        for (p, input) in ports.iter().enumerate() {
            let a = at(&sim, out, input);
            let b = spectrum.element(q, p);
            let worst = a
                .iter()
                .zip(&b)
                .map(|(x, y)| (x - y).norm())
                .fold(0.0, f64::max);
            assert!(worst < 1e-9, "{out} <- {input}: {worst}");
        }
    }
    // and it round-trips through its file
    assert_eq!(Chip::parse(&chip.to_text().unwrap()).unwrap(), chip);
    // the wrong convention is another device: S conjugated
    chip.instances[0].convention = Some(TimeConvention::Engineering);
    let conj = simulate(&chip, &dir).unwrap();
    let drop = spectrum.port("drop").unwrap();
    let a = at(&conj, "drop", "in")[100];
    assert!((a - spectrum.element(drop, 0)[100].conj()).norm() < 1e-9);
    // a missing file is a problem of its instance
    chip.instances[0].file = Some("measured/nothing.s4p".into());
    let problems = check(chip.clone());
    assert_eq!(problems[0].instance.as_deref(), Some("ring"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The sweep shared out among the cores is the library's own `Circuit::spectrum`, bit for bit.
#[test]
fn a_sweep_on_every_core_is_the_librarys() {
    for (file, text) in example_texts() {
        let chip = Chip::parse(text).unwrap();
        let (netlist, spectrum, _) = solve(&chip, here()).unwrap();
        let wavelengths = chip.sweep.wavelengths().unwrap();
        let own = netlist.compile().unwrap().spectrum(&wavelengths).unwrap();
        assert_eq!(own.ports(), spectrum.ports(), "{file}");
        assert_eq!(own.matrices(), spectrum.matrices(), "{file}");
    }
}
