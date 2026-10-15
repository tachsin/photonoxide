use std::time::{Duration, Instant};

use photonoxide::device::{self, Device, Values};

use super::*;

/// The device `kind` at its defaults, on 220 nm SOI, at `fidelity`, with `parameters`.
fn design(kind: &str, parameters: &[(&str, &str)]) -> SpecIn {
    SpecIn {
        kind: kind.into(),
        fidelity: "analytic".into(),
        stack: StackIn::Preset {
            name: "soi_220".into(),
            bottom_oxide_um: 2.0,
        },
        parameters: parameters
            .iter()
            .map(|&(n, e)| (n.to_owned(), e.to_owned()))
            .collect(),
    }
}

/// The device `kind` at its cheapest fidelity, with `parameters`.
fn cheapest(kind: &str, parameters: &[(&str, &str)]) -> SpecIn {
    let d = device::library()
        .into_iter()
        .find(|d| d.kind() == kind)
        .unwrap_or_else(|| panic!("no device {kind}"));
    SpecIn {
        fidelity: fidelity_name(d.fidelities()[0].fidelity).into(),
        ..design(kind, parameters)
    }
}

/// A short sweep for a model that solves at each wavelength (the MMIs' guided modes).
fn short() -> Sweep {
    Sweep {
        from_um: 1.5,
        to_um: 1.6,
        points: 11,
    }
}

fn field<'a>(view: &'a GeometryView, name: &str) -> &'a Field {
    view.fields
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no field {name}"))
}

#[test]
fn every_device_in_the_library_opens() {
    let library = device_library().unwrap();
    assert_eq!(library.len(), device::library().len());
    let entries: Vec<&str> = crate::circuits::library::entries()
        .iter()
        .map(|e| e.id)
        .collect();
    let lessons: Vec<String> = crate::academy::academy()
        .lessons
        .into_iter()
        .map(|l| l.id)
        .collect();
    let examples: Vec<String> = crate::examples::list()
        .into_iter()
        .map(|e| e.name)
        .collect();
    for info in &library {
        // the component it stands for, its lesson and its published results exist
        assert!(entries.contains(&info.id), "{}", info.id);
        assert!(lessons.iter().any(|l| l == info.lesson), "{}", info.lesson);
        assert!(!info.examples.is_empty());
        for e in &info.examples {
            assert!(examples.iter().any(|x| x == e), "{e}");
        }
        // at its cheapest model: analytic, or 2D for the rib and the MMIs
        let spec = cheapest(&info.kind, &[]);
        let f = spec.fidelity.clone();
        assert_eq!(f, info.fidelities[0].fidelity);
        let view = geometry(&spec);
        assert_eq!(view.problem, None, "{}", info.kind);
        assert!(view.outside.is_empty(), "{}: {:?}", info.kind, view.outside);
        let names: Vec<&str> = view.ports.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, info.ports, "{}", info.kind);
        assert!(matches!(view.scene, Some(Event::Scene { .. })));
        // every device parameter has its field, at its default
        for p in &info.parameters {
            let f = field(&view, &p.name);
            assert!(f.device && f.error.is_none());
            assert_eq!(f.value, Some(p.default), "{}.{}", info.kind, p.name);
        }
        let figures = derived(&spec, &f).unwrap();
        assert!(!figures.figures.is_empty());
        for g in &figures.figures {
            assert_eq!(g.fidelity, f);
            let f = g;
            assert!(
                !f.grid.is_empty(),
                "{}: {} names no grid",
                info.kind,
                f.symbol
            );
        }
        let sweep = if f == "analytic" {
            Sweep::default()
        } else {
            short()
        };
        let s = spectrum(&spec, &f, sweep).unwrap();
        assert_eq!(s.ports.len(), info.ports.len());
        assert_eq!(s.fidelity, f, "{}", info.kind);
        assert!(!s.grid.is_empty());
    }
}

#[test]
fn the_geometry_and_figures_are_the_librarys_for_the_same_values() {
    let cases: [(&str, &[(&str, &str)]); 11] = [
        ("waveguide", &[("width", "0.45"), ("length", "25")]),
        ("bend", &[("radius", "7.5 um"), ("angle", "60 deg")]),
        (
            "directional coupler",
            &[("w", "480 nm"), ("width", "w"), ("gap", "w/2.5")],
        ),
        ("all-pass ring", &[("radius", "12"), ("gap", "0.25")]),
        (
            "add-drop ring",
            &[("coupling_length", "2"), ("gap_drop", "0.3")],
        ),
        (
            "rib waveguide",
            &[("width", "0.6"), ("etch_depth", "70 nm")],
        ),
        ("MMI 1x2", &[("width", "5 um"), ("length", "24")]),
        (
            "MMI 2x2",
            &[
                ("width", "4.5"),
                ("access_width", "1.2"),
                ("taper_length", "5"),
            ],
        ),
        ("Y-branch", &[("w7", "1.35 um"), ("separation", "2.5")]),
        ("MZI", &[("upper", "120"), ("phase", "45 deg")]),
        ("Y-branch MZI", &[("lower", "80"), ("excess_loss", "0.2")]),
    ];
    for (kind, parameters) in cases {
        let spec = cheapest(kind, parameters);
        let f = device::fidelity(&spec.fidelity).unwrap();
        let view = geometry(&spec);
        assert_eq!(view.problem, None, "{kind}");
        let d = device::library()
            .into_iter()
            .find(|d| d.kind() == kind)
            .unwrap();
        let (_, design) = spec.spec().unwrap().build().unwrap();
        let values = design.values();
        assert_eq!(view.values.as_deref(), Some(values.as_slice()));
        // the ports, where the library puts them
        let g = d.geometry(values).unwrap();
        assert_eq!(view.ports.len(), g.ports.len());
        for (p, q) in view.ports.iter().zip(&g.ports) {
            assert_eq!(p.name, q.name);
            assert_eq!((p.x, p.y), (q.position.x.to_um(), q.position.y.to_um()));
            assert_eq!(p.direction, q.direction);
            assert_eq!(p.width, q.width.to_um());
        }
        // a shape for each the library draws
        let Some(Event::Scene { shapes, .. }) = &view.scene else {
            panic!("no scene")
        };
        let drawn: usize = g
            .structure
            .stack()
            .layers()
            .iter()
            .map(|l| g.structure.shapes(&l.name).len())
            .sum();
        assert_eq!(shapes.len(), drawn, "{kind}");
        // the figures, to the bit
        let ours = derived(&spec, &spec.fidelity).unwrap().figures;
        let theirs = d.derived(values, f).unwrap();
        assert_eq!(ours.len(), theirs.len());
        for (a, b) in ours.iter().zip(&theirs) {
            assert_eq!(a.symbol, b.symbol);
            assert_eq!(a.value.to_bits(), b.value.to_bits(), "{kind} {}", a.symbol);
            assert_eq!(a.grid, b.grid);
        }
    }
    // at the defaults, the figures are those of the library's own device
    let d = device::StripWaveguide::new();
    let ours = derived(&design("waveguide", &[]), "analytic")
        .unwrap()
        .figures;
    let theirs = d
        .derived(&Values::defaults(&d), Fidelity::Analytic)
        .unwrap();
    assert_eq!(ours[0].value, theirs[0].value);
}

#[test]
fn a_design_round_trips_through_the_form_and_the_file() {
    let spec = SpecIn {
        stack: StackIn::Layers {
            substrate: MaterialIn {
                id: "si".into(),
                model: None,
                temperature: None,
                composition: None,
            },
            layers: vec![
                LayerIn {
                    name: "BOX".into(),
                    thickness_um: 3.0,
                    material: MaterialIn {
                        id: "sio2".into(),
                        model: Some("malitson-1965".into()),
                        temperature: None,
                        composition: None,
                    },
                    background: MaterialIn {
                        id: "sio2".into(),
                        model: None,
                        temperature: None,
                        composition: None,
                    },
                },
                LayerIn {
                    name: "Si".into(),
                    thickness_um: 0.22,
                    material: MaterialIn {
                        id: "si".into(),
                        model: None,
                        temperature: None,
                        composition: None,
                    },
                    background: MaterialIn {
                        id: "sio2".into(),
                        model: None,
                        temperature: None,
                        composition: None,
                    },
                },
            ],
            cladding: MaterialIn {
                id: "sio2".into(),
                model: None,
                temperature: None,
                composition: None,
            },
            core: "Si".into(),
        },
        ..design(
            "directional coupler",
            &[
                ("w", "450 nm"),
                ("width", "w"),
                ("gap", "w/2.5"),
                ("length", "6.5 um"),
            ],
        )
    };
    // the form: the device's parameters in its order, then the others, each as it was given
    let view = geometry(&spec);
    assert_eq!(view.problem, None);
    for (name, expression) in &spec.parameters {
        assert_eq!(&field(&view, name).expression, expression);
    }
    let form: Vec<(String, String)> = view
        .fields
        .iter()
        .map(|f| (f.name.clone(), f.expression.clone()))
        .collect();
    let names: Vec<&str> = form.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "width",
            "thickness",
            "slab",
            "sidewall",
            "gap",
            "length",
            "loss",
            "wavelength",
            "span",
            "w"
        ]
    );
    // and in that order however a file lists them
    let mut sorted_spec = spec.clone();
    sorted_spec.parameters.sort();
    let again: Vec<String> = geometry(&sorted_spec)
        .fields
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(again, names);
    // what the form sends back, through the file and back: the same design, expressions and all
    let edited = SpecIn {
        parameters: form.clone(),
        ..spec.clone()
    };
    let text = saved_text(
        &edited,
        "narrow-coupler",
        "A narrower coupler.\nIts gap follows w.",
    )
    .unwrap();
    assert!(text.starts_with("# A narrower coupler.\n# Its gap follows w.\n"));
    assert!(
        text.contains("[device]") && text.contains("gap = \"w/2.5\""),
        "{text}"
    );
    let (back, name, about) = read_text(&text).unwrap();
    assert_eq!(name, "narrow-coupler");
    assert_eq!(about, "A narrower coupler.\nIts gap follows w.");
    assert_eq!(back.stack, edited.stack);
    assert_eq!(back.fidelity, edited.fidelity);
    let mut sorted = form;
    sorted.sort();
    assert_eq!(back.parameters, sorted);
    assert_eq!(geometry(&back).values, view.values);
    // a job that isn't a device's isn't read as one, and a bad design isn't saved
    assert!(read_text("name = \"x\"\n[task]\nkind = \"modes\"\n").is_err());
    assert!(saved_text(&design("waveguide", &[("width", "w")]), "x", "").is_err());
    assert!(saved_text(&design("waveguide", &[]), "not a name", "").is_err());
}

#[test]
fn errors_show_in_their_field_and_what_uses_a_parameter_is_listed() {
    let coupler = |extra: &[(&str, &str)]| {
        let mut p = vec![("w", "500 nm"), ("width", "w"), ("gap", "w/2.5")];
        p.extend_from_slice(extra);
        geometry(&design("directional coupler", &p))
    };
    let view = coupler(&[]);
    assert_eq!(view.problem, None);
    assert_eq!(field(&view, "w").used_by, ["width", "gap"]);
    assert_eq!(field(&view, "gap").uses, ["w"]);
    assert!(!field(&view, "w").device && field(&view, "w").unit == "µm");
    assert!((field(&view, "gap").value.unwrap() - 0.2).abs() < 1e-15);
    // an expression that doesn't parse: its column
    let view = coupler(&[("length", "3 +* 2")]);
    let e = field(&view, "length").error.as_ref().unwrap();
    assert_eq!(e.column, Some(4), "{}", e.message);
    assert!(view.values.is_none());
    // an unknown name, still with what the others use
    let view = coupler(&[("length", "2*wdth")]);
    let e = field(&view, "length").error.as_ref().unwrap();
    assert!(e.message.contains("wdth") && e.column == Some(3), "{e:?}");
    assert_eq!(field(&view, "gap").uses, ["w"]);
    // a unit that doesn't fit, and a value out of range, in their own fields
    let view = coupler(&[("loss", "2 um")]);
    assert!(field(&view, "loss").error.is_some(), "{:?}", view.problem);
    let view = geometry(&design(
        "directional coupler",
        &[("w", "9 um"), ("width", "w")],
    ));
    assert!(field(&view, "width").error.is_some(), "{:?}", view.problem);
    // a stack that can't be built is the stack's, not a field's
    let mut spec = design("waveguide", &[]);
    spec.stack = StackIn::Preset {
        name: "glass".into(),
        bottom_oxide_um: 2.0,
    };
    let view = geometry(&spec);
    assert!(view.stack_problem.is_some() && view.fields.iter().all(|f| f.error.is_none()));
}

#[test]
fn outside_the_fits_the_page_is_told_and_offered_the_3d_model() {
    // within the width's own range, outside the fit's 400 to 600 nm
    let spec = design("waveguide", &[("width", "350 nm")]);
    let view = geometry(&spec);
    assert_eq!(view.problem, None);
    assert_eq!(view.outside, ["width"]);
    let e = derived(&spec, "analytic").err().unwrap();
    assert!(e.contains("0.4 to 0.6") && e.contains("3D"), "{e}");
    // the 3D model has no such range
    let view = geometry(&SpecIn {
        fidelity: "3D".into(),
        ..spec
    });
    assert!(view.outside.is_empty());
    // a gap outside 100 to 500 nm, a thickness outside 200 to 240 nm
    for (kind, name, value) in [
        ("directional coupler", "gap", "0.6"),
        ("all-pass ring", "gap", "60 nm"),
        ("bend", "thickness", "0.25"),
    ] {
        let spec = design(kind, &[(name, value)]);
        assert_eq!(geometry(&spec).outside, [name], "{kind}");
        assert!(derived(&spec, "analytic").is_err(), "{kind}");
    }
    // and a stack that isn't silicon in silica
    let mut spec = design("waveguide", &[]);
    spec.stack = StackIn::Preset {
        name: "nitride".into(),
        bottom_oxide_um: 2.0,
    };
    assert_eq!(geometry(&spec).problem, None);
    assert!(derived(&spec, "analytic").err().unwrap().contains("3D"));
}

#[test]
fn a_slider_drag_keeps_up_at_the_analytic_fidelity() {
    // the page asks again 60 ms after a slider stops: each of a drag's updates (the form, the
    // geometry and scene, the figures, and 501 wavelengths of S-parameters) must take less
    const BUDGET: Duration = Duration::from_millis(60);
    for d in device::library() {
        // the rib and the MMIs have no analytic model: their cheapest solves slabs
        if d.fidelities()[0].fidelity != Fidelity::Analytic {
            continue;
        }
        let steps = 40;
        let started = Instant::now();
        let mut slowest = Duration::ZERO;
        for k in 0..steps {
            let width = 0.45 + 0.1 * f64::from(k) / f64::from(steps - 1);
            let t = Instant::now();
            let spec = design(d.kind(), &[("width", &format!("{width:?}"))]);
            let view = geometry(&spec);
            assert_eq!(view.problem, None);
            derived(&spec, "analytic").unwrap();
            spectrum(&spec, "analytic", Sweep::default()).unwrap();
            slowest = slowest.max(t.elapsed());
        }
        let mean = started.elapsed() / steps;
        println!("{}: {mean:?} an update, the slowest {slowest:?}", d.kind());
        // a Y-branch's update traces its spline outline, a region between two curves, and its
        // arms, each curve a polygon within 1 nm for the 3D scene (about 30 ms here, 60 on
        // CI's macOS runner); an MZI's draws a whole circuit, two splitters and the arms' bends
        // (the scene alone about 40 ms): they keep up within a few of the page's waits
        let budget = if d.kind().contains("MZI") {
            4 * BUDGET
        } else if d.kind() == "Y-branch" {
            2 * BUDGET
        } else {
            BUDGET
        };
        assert!(
            mean < budget,
            "{}: {mean:?} an update, over the {budget:?} the page waits",
            d.kind()
        );
    }
}

#[test]
fn every_devices_job_is_made_checked_and_read_back_into_the_form() {
    // each device by its own cheapest model, and by FDTD in 2D: the library's job, read back
    // into the form whole
    for d in device::library() {
        let spec = cheapest(d.kind(), &[]);
        for compute in [
            ComputeIn::default(),
            ComputeIn {
                fdtd: Some(2),
                band: Some((1.5, 1.6, 11)),
                ..ComputeIn::default()
            },
        ] {
            let text = job_text_of(&spec, &compute, "page-job")
                .unwrap_or_else(|e| panic!("{}: {e}", d.kind()));
            assert!(
                text.starts_with(&format!("# The {} by ", d.kind())),
                "{text}"
            );
            assert!(text.contains("photonoxide run jobs/page-job.toml"));
            let form = read_job_form(&text);
            assert_eq!(form.error, None, "{}: {text}", d.kind());
            assert!(form.beyond.is_empty(), "{}: {:?}", d.kind(), form.beyond);
            assert_eq!(form.spec.as_ref(), Some(&spec), "{}", d.kind());
            let mut want = compute.clone();
            // the device's own band is written out, as are FDTD's cells and PMLs
            if want.fdtd.is_none() {
                want.band = form.compute.as_ref().and_then(|c| c.band);
            } else {
                want.step_nm = Some(40.0);
                want.pml_cells = Some(20);
            }
            want.timeout_minutes = Some(30.0);
            assert_eq!(form.compute.as_ref(), Some(&want), "{}", d.kind());
        }
    }
}

#[test]
fn as_text_an_edit_the_form_holds_moves_it_and_one_it_cant_is_kept_and_named() {
    let spec = design("waveguide", &[("width", "450 nm")]);
    let compute = ComputeIn {
        band: Some((1.5, 1.6, 11)),
        ..ComputeIn::default()
    };
    let text = job_text_of(&spec, &compute, "w").unwrap();
    // the band edited: the form follows
    let form = read_job_form(&text.replace("points = 11", "points = 21"));
    assert!(form.beyond.is_empty() && form.error.is_none(), "{form:?}");
    assert_eq!(form.compute.unwrap().band, Some((1.5, 1.6, 21)));
    // a parameter's expression edited: the design follows
    let form = read_job_form(&text.replace("\"450 nm\"", "\"480 nm\""));
    assert!(
        form.spec
            .unwrap()
            .parameters
            .contains(&("width".into(), "480 nm".into()))
    );
    // an FDTD's stopping rule and a solver table: kept in the text, and named
    let fdtd = job_text_of(
        &spec,
        &ComputeIn {
            fdtd: Some(2),
            step_nm: Some(50.0),
            band: Some((1.5, 1.6, 3)),
            ..ComputeIn::default()
        },
        "w-fdtd",
    )
    .unwrap();
    let stopped = format!("{fdtd}\n[task.stop]\nuntil = \"time\"\ntime_um = 100.0\n");
    let form = read_job_form(&stopped);
    assert_eq!(form.error, None);
    assert_eq!(form.beyond, ["task.stop"]);
    let form = read_job_form(&format!("{fdtd}\n[solver]\ndirect = \"auto\"\n"));
    assert!(form.beyond.contains(&"solver".to_owned()), "{form:?}");
    // what isn't a job says why
    assert!(
        read_job_form("name = \"x\"\n[task]\nkind = \"modes\"\n")
            .error
            .is_some()
    );
    assert!(
        read_job_form(&text.replace("points = 11", "points = 0"))
            .error
            .is_some()
    );
    // a 3D FDFD says its size, and one too large says how large before it runs
    let mmi = SpecIn {
        fidelity: "3D".into(),
        ..design("MMI 1x2", &[("width", "3 um"), ("length", "3 um")])
    };
    let coarse = ComputeIn {
        step_nm: Some(200.0),
        step_z_nm: Some(110.0),
        margin_um: Some(0.2),
        samples: Some(1),
        pml_cells: Some(4),
        ..ComputeIn::default()
    };
    let form = read_job_form(&job_text_of(&mmi, &coarse, "mmi").unwrap());
    let (nx, ny, unknowns) = form.size.unwrap();
    assert!(
        nx > 0 && ny > 0 && unknowns > 3.0 * (nx * ny) as f64,
        "{:?}",
        form.size
    );
    let fine = ComputeIn {
        step_nm: Some(20.0),
        step_z_nm: Some(20.0),
        ..ComputeIn::default()
    };
    let e = job_text_of(&mmi, &fine, "mmi").unwrap_err();
    assert!(e.contains("million unknowns"), "{e}");
}

#[test]
fn a_saved_job_that_computes_the_design_reopens_with_its_text() {
    let spec = design("waveguide", &[]);
    let text = job_text_of(&spec, &ComputeIn::default(), "guide-run").unwrap();
    let dir = std::env::temp_dir().join(format!("photonoxide-device-job-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guide-run.toml");
    std::fs::write(&path, &text).unwrap();
    let saved = read_device(path.display().to_string()).unwrap();
    assert_eq!(saved.job.as_deref(), Some(text.as_str()));
    assert_eq!(saved.spec, spec);
    // a design alone has none
    std::fs::write(&path, saved_text(&spec, "guide", "").unwrap()).unwrap();
    assert_eq!(read_device(path.display().to_string()).unwrap().job, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn as_solved_shows_the_cells_the_chosen_model_solves_on() {
    // a closed form solves no grid
    let guide = design("waveguide", &[]);
    let s = solved(&guide, &ComputeIn::default(), "x", 5.0).unwrap();
    assert!(s.top.is_none() && s.cut.is_none(), "{}", s.note);
    // a mode solve: its cross-section only, on its mesh's step
    let three = SpecIn {
        fidelity: "3D".into(),
        ..guide.clone()
    };
    let s = solved(&three, &ComputeIn::default(), "x", 5.0).unwrap();
    let cut = s.cut.unwrap();
    assert!(s.top.is_none());
    assert_eq!(s.step_nm, 10.0);
    // the strip's silicon at its middle, oxide at the window's edge
    let mid = cut.at(cut.nx / 2, cut.ny / 2);
    assert!(mid > 11.0 && cut.at(0, 0) < 2.2, "{mid}");
    // 2D FDFD: the plane on its cells, its picture coarser when they are too many, and saying so
    let mmi = SpecIn {
        fidelity: "2D".into(),
        ..design("MMI 1x2", &[])
    };
    let fdfd = ComputeIn {
        fdfd_2d: true,
        ..ComputeIn::default()
    };
    let s = solved(&mmi, &fdfd, "x", 1.0).unwrap();
    let top = s.top.unwrap();
    assert_eq!(s.step_nm, 20.0);
    assert!(
        (top.nx * top.ny) as f64 <= 1.1 * MOST_PIXELS,
        "{} x {}",
        top.nx,
        top.ny
    );
    assert!(s.coarser > 1.0);
    assert!(s.note.contains("effective index"));
    // FDTD on its own cells
    let s = solved(
        &guide,
        &ComputeIn {
            fdtd: Some(3),
            ..ComputeIn::default()
        },
        "y",
        0.0,
    )
    .unwrap();
    assert_eq!(s.step_nm, 50.0);
    assert!(s.top.is_some() && s.cut.is_some());
}
