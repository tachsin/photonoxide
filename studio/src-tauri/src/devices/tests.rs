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
        compact: None,
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
    let cases: [(&str, &[(&str, &str)]); 18] = [
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
        (
            "thermo-optic phase shifter",
            &[("heater_width", "3 um"), ("power", "10")],
        ),
        (
            "doped thermo-optic phase shifter",
            &[("buffer", "600 nm"), ("power", "5")],
        ),
        (
            "in-guide thermo-optic phase shifter",
            &[("reach", "4 um"), ("power", "8")],
        ),
        (
            "GST phase-change phase shifter",
            &[("film_thickness", "20 nm"), ("crystallized", "0.5")],
        ),
        (
            "Sb2S3 phase-change phase shifter",
            &[("width", "450 nm"), ("length", "30")],
        ),
        (
            "Sb2Se3 phase-change phase shifter",
            &[("coverage", "0.6"), ("crystallized", "1")],
        ),
        (
            "TFLN Mach-Zehnder modulator",
            &[("gap", "6 um"), ("voltage", "2"), ("frequency", "40")],
        ),
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
        // a shape for each the library draws, on its layer (an etched layer's top band: its
        // bands below, `LN 2` and on, draw the same shapes again)
        let Some(Event::Scene { shapes, .. }) = &view.scene else {
            panic!("no scene")
        };
        let layers = g.structure.stack().layers();
        let drawn: usize = layers
            .iter()
            .map(|l| g.structure.shapes(&l.name).len())
            .sum();
        let on_layers = shapes
            .iter()
            .filter(|s| layers.iter().any(|l| l.name == s.layer))
            .count();
        assert_eq!(on_layers, drawn, "{kind}");
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
        text.contains("device:") && text.contains("gap: w/2.5"),
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

/// A fixed piece of work, owing nothing to the code under test: a polygon of 4000 vertices
/// traced, its area summed, and its vertices sorted, twice over, the two as tasks on rayon's
/// pool. Timed beside a slider's updates, it says how busy the machine is, and how busy the
/// pool: an update's 501 wavelengths are solved on that pool too, and when other tests in the
/// same process keep it busy, they wait in its queue as long as the yardstick's tasks do.
fn yardstick() -> Duration {
    use rayon::prelude::*;
    let t = Instant::now();
    let area: f64 = (0..2u32)
        .into_par_iter()
        .map(|turn| {
            let n = 4000;
            let mut points: Vec<(f64, f64)> = (0..n)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / f64::from(n);
                    let r = 1.0 + 0.1 * (7.0 * a + f64::from(turn)).sin();
                    (r * a.cos(), r * a.sin())
                })
                .collect();
            let area = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
                .sum::<f64>()
                / 2.0;
            points.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
            area + std::hint::black_box(points[0].1)
        })
        .sum();
    std::hint::black_box(area);
    t.elapsed()
}

/// The middle of `times`.
fn median(mut times: Vec<Duration>) -> Duration {
    times.sort_unstable();
    times[times.len() / 2]
}

#[test]
fn a_slider_drag_keeps_up_at_the_analytic_fidelity() {
    // the page asks again 60 ms after a slider stops: each of a drag's updates (the form, the
    // geometry and scene, the figures, and 501 wavelengths of S-parameters) must take less
    const BUDGET: Duration = Duration::from_millis(60);
    // The budget is for a machine with nothing else to do and about as fast as CI's runners,
    // on which the yardstick takes this long at most: it takes 2.2 ms on a 20-thread desktop
    // with its two tasks one after the other, about half that side by side (its code
    // unoptimized, as the tests build it), and CI's runners are about half as fast.
    // A machine that other work slows (CI's runners share their hosts; a suite runs its tests
    // at once) slows the yardstick too: each update is timed beside it, and counted at an idle
    // machine's speed, its time divided by how many times this the yardstick took, if more.
    // Only the machine's slowness is divided out: an update that does more work, or slower
    // work, is as much slower beside the same yardstick, and still fails. And the median of a
    // drag's updates, not their mean, so that the few a burst of other work catches don't
    // decide.
    //
    // A drag of 20 updates, tried up to three times: other work can only make a drag slower, so
    // a device whose updates fit the budget at an idle machine's speed passes in one of them,
    // while one that does more work, or slower work, misses it in all three.
    const IDLE: Duration = Duration::from_millis(5);
    const STEPS: u32 = 20;
    const TRIES: usize = 3;
    for d in device::library() {
        // the rib and the MMIs have no analytic model: their cheapest solves slabs
        if d.fidelities()[0].fidelity != Fidelity::Analytic {
            continue;
        }
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
        let mut drags = Vec::new();
        for _ in 0..TRIES {
            let mut times = Vec::new();
            let mut idle = Vec::new();
            let mut yards = Vec::new();
            for k in 0..STEPS {
                let width = 0.45 + 0.1 * f64::from(k) / f64::from(STEPS - 1);
                let t = Instant::now();
                let spec = design(d.kind(), &[("width", &format!("{width:?}"))]);
                let view = geometry(&spec);
                assert_eq!(view.problem, None);
                derived(&spec, "analytic").unwrap();
                spectrum(&spec, "analytic", Sweep::default()).unwrap();
                let time = t.elapsed();
                let yard = yardstick();
                times.push(time);
                idle.push(time.div_f64((yard.as_secs_f64() / IDLE.as_secs_f64()).max(1.0)));
                yards.push(yard);
            }
            let mean = times.iter().sum::<Duration>() / STEPS;
            let slowest = *times.iter().max().unwrap();
            let yard = median(yards);
            let median = median(idle);
            println!(
                "{}: {mean:?} an update, the slowest {slowest:?}; the yardstick's median \
                 {yard:?}, and an update's at an idle machine's speed {median:?}",
                d.kind()
            );
            drags.push((median, mean, yard));
            if median < budget {
                break;
            }
        }
        let (median, mean, yard) = *drags.iter().min_by_key(|t| t.0).unwrap();
        assert!(
            median < budget,
            "{}: {median:?} an update at an idle machine's speed in the best of {} drags \
             ({mean:?} here, the yardstick's median {yard:?}), over the {budget:?} the page \
             waits",
            d.kind(),
            drags.len()
        );
    }
}

#[test]
fn every_devices_job_and_study_is_made_checked_and_read_back_into_the_form() {
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
            // a modulator millimetres long is more than an FDTD grid holds: refused, and said so
            if compute.fdtd.is_some() && d.kind().contains("modulator") {
                let e = job_text_of(&spec, &compute, "page-job").unwrap_err();
                assert!(e.contains("cells, more than"), "{e}");
                continue;
            }
            let text = job_text_of(&spec, &compute, "page-job")
                .unwrap_or_else(|e| panic!("{}: {e}", d.kind()));
            assert!(
                text.starts_with(&format!("# The {} by ", d.kind())),
                "{text}"
            );
            assert!(text.contains("photonoxide run jobs/page-job.yaml"));
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
        // and its studies: swept in a length of its own at its cheapest model, and its
        // corners, read back into the Study tab's form whole
        let p = d
            .parameters()
            .iter()
            .find(|p| p.unit == "µm" && !["wavelength", "span"].contains(&p.name.as_str()))
            .unwrap_or_else(|| panic!("{}: no length to sweep", d.kind()));
        let (lo, hi) = (p.default * 0.95, p.default * 1.05);
        let compute = ComputeIn {
            band: Some((1.55, 1.55, 1)),
            ..ComputeIn::default()
        };
        let sweep = StudyIn {
            kind: "sweep".into(),
            axes: vec![AxisIn {
                parameter: p.name.clone(),
                from: lo,
                to: hi,
                points: 3,
            }],
            corners: Vec::new(),
        };
        let corners = StudyIn {
            kind: "corners".into(),
            axes: Vec::new(),
            corners: vec![
                CornerIn {
                    name: "wide".into(),
                    offsets: vec![(p.name.clone(), p.default * 0.02)],
                    temperature_k: None,
                },
                CornerIn {
                    name: "narrow".into(),
                    offsets: vec![(p.name.clone(), -p.default * 0.02)],
                    temperature_k: None,
                },
            ],
        };
        for study in [sweep, corners] {
            let text = study_text_of(&spec, &compute, &study, "page-study")
                .unwrap_or_else(|e| panic!("{}: {e}", d.kind()));
            assert!(text.starts_with(&format!("# The {} ", d.kind())), "{text}");
            assert!(text.contains("--workers"));
            let form = read_job_form(&text);
            assert_eq!(form.error, None, "{}: {text}", d.kind());
            assert!(form.beyond.is_empty(), "{}: {:?}", d.kind(), form.beyond);
            assert_eq!(form.spec.as_ref(), Some(&spec), "{}", d.kind());
            assert_eq!(form.study.as_ref(), Some(&study), "{}", d.kind());
            assert_eq!(
                form.compute.as_ref().and_then(|c| c.band),
                Some((1.55, 1.55, 1))
            );
        }
    }
    // what the library refuses comes back as its message: a temperature at the analytic fidelity
    let hot = StudyIn {
        kind: "corners".into(),
        axes: Vec::new(),
        corners: vec![CornerIn {
            name: "hot".into(),
            offsets: Vec::new(),
            temperature_k: Some(340.0),
        }],
    };
    let e = study_text_of(
        &design("waveguide", &[]),
        &ComputeIn::default(),
        &hot,
        "hot",
    )
    .unwrap_err();
    assert!(e.contains("analytic"), "{e}");
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
    let form = read_job_form(&text.replace("points: 11", "points: 21"));
    assert!(form.beyond.is_empty() && form.error.is_none(), "{form:?}");
    assert_eq!(form.compute.unwrap().band, Some((1.5, 1.6, 21)));
    // a parameter's expression edited: the design follows
    let form = read_job_form(&text.replace("width: 450 nm", "width: 480 nm"));
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
    // the job in TOML, its tables added at its end; the form reads either
    let fdtd = photonoxide::run::Job::parse_as(&fdtd, photonoxide::run::Format::Yaml)
        .unwrap()
        .to_text(photonoxide::run::Format::Toml)
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
        read_job_form(&text.replace("points: 11", "points: 0"))
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
    let path = dir.join("guide-run.yaml");
    std::fs::write(&path, &text).unwrap();
    let saved = read_device(path.display().to_string()).unwrap();
    assert_eq!(saved.job.as_deref(), Some(text.as_str()));
    assert_eq!(saved.spec, spec);
    // a design alone has none
    let design_text = saved_text(&spec, "guide", "").unwrap();
    std::fs::write(&path, &design_text).unwrap();
    assert_eq!(read_device(path.display().to_string()).unwrap().job, None);
    // one saved in TOML before the studio wrote YAML reads as it did
    let old = dir.join("guide.toml");
    let toml = photonoxide::run::Job::parse_as(&design_text, photonoxide::run::Format::Yaml)
        .unwrap()
        .to_text(photonoxide::run::Format::Toml)
        .unwrap();
    std::fs::write(&old, toml).unwrap();
    let back = read_device(old.display().to_string()).unwrap();
    assert_eq!((back.spec, back.job), (spec, None));
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

#[test]
fn the_temperature_view_is_the_2d_model_s_heat_at_the_design_s_power() {
    use std::f64::consts::PI;
    for kind in [
        "thermo-optic phase shifter",
        "doped thermo-optic phase shifter",
        "in-guide thermo-optic phase shifter",
    ] {
        // at the defaults the power is zero: the picture is at P_π, and the rise the mode feels
        // gives a phase of π
        let spec = design(kind, &[]);
        let t = fields::temperature(&spec).unwrap();
        let s = spec.spec().unwrap();
        let (_, d) = s.build().unwrap();
        let values = d.values();
        assert_eq!(values.get("power"), Some(0.0));
        let shifter = fields::thermo_shifter(&s).unwrap().shifter(values).unwrap();
        let (length, lambda) = (
            values.get("length").unwrap() * 1e-6,
            values.get("wavelength").unwrap() * 1e-6,
        );
        let e = shifter.efficiency().unwrap();
        assert!(
            t.at_pi && (t.power_mw / (e.p_pi(0, 0) * 1e3) - 1.0).abs() < 1e-9,
            "{kind}"
        );
        assert_eq!(t.power_mw, t.p_pi_mw);
        let guide = &shifter.guides()[0];
        let phase =
            |t: &fields::Temperature| 2.0 * PI * length * t.guide * guide.thermo_optic() / lambda;
        assert!((phase(&t) / PI - 1.0).abs() < 1e-9, "{kind}: {}", phase(&t));
        // at the design's power (once: a solve each): P / L along the guide, and the 2D model's
        // phase there, its efficiency's 2π s P / λ from the same weights and rise
        if kind == "thermo-optic phase shifter" {
            let at = fields::temperature(&design(kind, &[("power", "10")])).unwrap();
            assert!(!at.at_pi && at.power_mw == 10.0 && at.p_pi_mw == t.p_pi_mw);
            let w = 10e-3 / length;
            assert!(
                (at.watts_per_metre / w - 1.0).abs() < 1e-12,
                "{}",
                at.watts_per_metre
            );
            let model = e.phase(0, 0, 10e-3);
            assert!(
                (phase(&at) / model - 1.0).abs() < 1e-9,
                "{} {model}",
                phase(&at)
            );
        }
        // the picture: the guide's centre in it, every pixel a rise up to the peak (none below
        // the ambient: the in-guide heater's contacts are held at it), the mode feeling less
        // than the peak
        let r = &t.raster;
        assert!(
            r.x0 < 0.0 && r.x1 > 0.0 && r.y0 < 0.0 && r.y1 > 0.2,
            "{kind}: {r:?}"
        );
        let (lo, hi) = r
            .values
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(
            lo >= 0.0 && f64::from(hi) <= t.peak * (1.0 + 1e-6),
            "{kind}: {lo} {hi} {}",
            t.peak
        );
        assert!(t.guide > 0.0 && t.guide < t.peak, "{kind}");
        // isotherms at round levels below the peak, each found; the media's edges; the mode's
        // two outlines, closed, around the guide's centre
        assert!(t.isotherms.len() >= 4, "{kind}: {}", t.isotherms.len());
        for i in &t.isotherms {
            assert!(
                i.level < t.peak && !i.lines.is_empty(),
                "{kind}: {}",
                i.level
            );
        }
        assert!(t.outline.len() >= 4, "{kind}");
        assert_eq!(t.mode.len(), 2);
        for m in &t.mode {
            let longest = m.lines.iter().max_by_key(|l| l.len()).unwrap();
            assert_eq!(longest.first(), longest.last(), "{kind}: closed");
            let (xs, ys): (Vec<f64>, Vec<f64>) = longest.iter().map(|p| (p[0], p[1])).unzip();
            let span = |v: &[f64]| {
                v.iter()
                    .copied()
                    .fold([f64::MAX, f64::MIN], |s, x| [s[0].min(x), s[1].max(x)])
            };
            let (sx, sy) = (span(&xs), span(&ys));
            assert!(
                sx[0] < 0.0 && sx[1] > 0.0 && sy[0] < 0.11 && sy[1] > 0.11,
                "{kind}: {sx:?} {sy:?}"
            );
        }
        assert!(t.grid.contains("heat on"), "{}", t.grid);
    }
    // a device without a heater has no temperature
    let e = fields::temperature(&design("waveguide", &[])).unwrap_err();
    assert!(e.contains("no heat model"), "{e}");
}

#[test]
fn the_temperature_in_time_is_the_2d_model_s_turn_on_its_times_the_device_s_figures() {
    let same = |a: f64, b: f64| a == b || (a / b - 1.0).abs() < 1e-12;
    for kind in [
        "thermo-optic phase shifter",
        "doped thermo-optic phase shifter",
        "in-guide thermo-optic phase shifter",
    ] {
        let spec = design(kind, &[]);
        let t = fields::temperature_in_time(&spec).unwrap();
        // the 1/e time plotted is the device's own 2D figure tau, from the same transient; the
        // 10-90 % rise time the difference of the two times marked
        let s = spec.spec().unwrap();
        let (d, design) = s.build().unwrap();
        let figures = d
            .component(design.values(), photonoxide::circuit::Fidelity::TwoD)
            .unwrap()
            .figures;
        let tau = figures.iter().find(|f| f.symbol == "tau").unwrap();
        assert_eq!(tau.unit, "µs");
        assert_eq!(
            t.tau_us.to_bits(),
            tau.value.to_bits(),
            "{kind}: {} {}",
            t.tau_us,
            tau.value
        );
        assert!(same(t.rise_us, t.ninety_us - t.ten_us), "{kind}");
        assert!(t.ten_us < t.tau_us && t.tau_us < t.ninety_us, "{kind}");
        // the curve plotted crosses 10 %, 1 - 1/e and 90 % of its steady value at those times
        let at = |time: f64| {
            let k = t.times_us.iter().position(|&x| x >= time).unwrap();
            let (a, b) = (t.times_us[k - 1], t.times_us[k]);
            t.guide[k - 1] + (time - a) / (b - a) * (t.guide[k] - t.guide[k - 1])
        };
        for (time, share) in [
            (t.ten_us, 0.1),
            (t.tau_us, 1.0 - (-1.0f64).exp()),
            (t.ninety_us, 0.9),
        ] {
            assert!(
                (at(time) / t.steady - share).abs() < 1e-9,
                "{kind}: {} at {time} us",
                at(time) / t.steady
            );
        }
        assert_eq!(t.guide[0], 0.0);
        assert_eq!(t.times_us.len(), t.guide.len());
        // where it goes is the steady view's rise the mode feels, at the same power
        let steady = fields::temperature(&spec).unwrap();
        assert!(
            (t.steady / steady.guide - 1.0).abs() < 1e-9,
            "{kind}: {} {}",
            t.steady,
            steady.guide
        );
        assert_eq!((t.power_mw, t.peak), (steady.power_mw, steady.peak));
        // the pictures: the start's (the ambient), then evenly to the record's end, warming
        assert_eq!(t.frames.len(), fields::FRAMES + 1, "{kind}");
        assert_eq!(t.frames[0].time_us, 0.0);
        assert!(same(
            t.frames.last().unwrap().time_us,
            *t.times_us.last().unwrap()
        ));
        assert!(t.frames[0].raster.values.iter().all(|v| *v == 0.0));
        let peak = |f: &fields::Frame| f.raster.values.iter().copied().fold(f32::MIN, f32::max);
        assert!(
            t.frames.windows(2).all(|w| peak(&w[1]) > peak(&w[0])),
            "{kind}"
        );
        assert!(f64::from(peak(t.frames.last().unwrap())) <= t.peak * (1.0 + 1e-6));
        // each picture's isotherms at the steady view's levels, the hottest ones found only late
        let levels: Vec<f64> = steady.isotherms.iter().map(|i| i.level).collect();
        assert_eq!(t.levels, levels);
        assert!(t.frames.iter().all(|f| f.isotherms.len() == levels.len()));
        assert!(t.frames[0].isotherms.iter().all(|i| i.lines.is_empty()));
        assert!(
            t.frames
                .last()
                .unwrap()
                .isotherms
                .iter()
                .any(|i| !i.lines.is_empty())
        );
        assert!(t.grid.contains("BDF2 steps of"), "{}", t.grid);
    }
    let e = fields::temperature_in_time(&design("waveguide", &[])).unwrap_err();
    assert!(e.contains("no heat model"), "{e}");
}

#[test]
fn a_modulator_sweep_saved_as_a_study_job_runs_to_its_points_bit_for_bit() {
    use photonoxide::job::{Event, check, execute};
    use photonoxide::run::{Format, Job, Run, Stop};
    let kind = "TFLN Mach-Zehnder modulator";
    // the sweep as the Study tab's Vπ·L tab takes it: from, to and the points, each value
    // from + (to − from) k / (n − 1), the page's own arithmetic
    for (parameter, from, to, n) in [
        ("gap", 3.0, 10.0, 5),
        ("length", 2000.0, 20000.0, 4),
        ("cladding", 0.1, 3.0, 4),
        ("cladding_permittivity", 1.0, 8.0, 3),
    ] {
        let xs: Vec<f64> = (0..n)
            .map(|k| from + ((to - from) * k as f64) / (n - 1) as f64)
            .collect();
        let spec = design(kind, &[]);
        let sweep = modulator::vpi_l_sweep(&spec, "analytic", parameter, &xs).unwrap();
        // saved: the design at the sweep's fidelity, its axis exactly the sweep's
        let study = StudyIn {
            kind: "sweep".into(),
            axes: vec![AxisIn {
                parameter: parameter.into(),
                from,
                to,
                points: n,
            }],
            corners: Vec::new(),
        };
        let name = format!("modulator-{parameter}");
        let text = study_text_of(&spec, &ComputeIn::default(), &study, &name).unwrap();
        let form = read_job_form(&text);
        assert_eq!(form.error, None, "{text}");
        assert_eq!(form.study.as_ref(), Some(&study));
        // run: each point's figures are the sweep's, to the bit
        let job = Job::parse_as(&text, Format::Yaml).unwrap();
        check(&job).unwrap();
        let mut run = Run::in_memory();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let points: Vec<(Vec<f64>, Vec<photonoxide::job::DeviceFigure>)> = run
            .lines()
            .iter()
            .filter_map(|l| match serde_json::from_str::<Event>(l) {
                Ok(Event::DevicePoint {
                    at, figures, error, ..
                }) => {
                    assert_eq!(error, None);
                    Some((at, figures))
                }
                _ => None,
            })
            .collect();
        assert_eq!(points.len(), sweep.points.len(), "{parameter}");
        for ((at, figures), p) in points.iter().zip(&sweep.points) {
            assert_eq!(
                at[0].to_bits(),
                p.x.to_bits(),
                "{parameter}: {at:?} {}",
                p.x
            );
            let value = |symbol: &str| figures.iter().find(|f| f.symbol == symbol).unwrap().value;
            for (symbol, v) in [
                ("VpiL", p.vpi_l),
                ("Vpi", p.vpi),
                ("f_3dB", p.f_3db),
                ("f_6dB", p.f_6db),
            ] {
                assert_eq!(
                    value(symbol).to_bits(),
                    v.to_bits(),
                    "{parameter} {}: {symbol} {} against {v}",
                    p.x,
                    value(symbol)
                );
            }
        }
    }
}

#[test]
fn a_compact_model_is_fitted_from_the_page_and_kept_next_to_its_design() {
    use photonoxide::job::{Event, execute};
    use photonoxide::run::{Job, Run, Stop};
    let dir = std::env::temp_dir().join(format!("photonoxide-page-compact-{}", std::process::id()));
    let jobs = dir.join("jobs");
    std::fs::create_dir_all(&jobs).unwrap();
    // the Model tab's form: the ring's gap on a grid of 5, 4 held out, degree 2
    let fit = compact::FitIn {
        axes: vec![compact::FitAxisIn {
            parameter: "gap".into(),
            from: 0.15,
            to: 0.25,
            points: 5,
        }],
        sampling: "grid".into(),
        count: 0,
        seed: 1,
        held_out: 4,
        poles: 12,
        degree: Some(2),
    };
    let compute = ComputeIn {
        band: Some((1.54, 1.56, 41)),
        ..ComputeIn::default()
    };
    let ring = design("all-pass ring", &[]);
    let text = compact::fit_text_of(&ring, &compute, &fit, "ring-compact").unwrap();
    assert!(
        text.starts_with("# A compact model of the all-pass ring over gap"),
        "{text}"
    );
    let job = Job::parse_as(&text, photonoxide::run::Format::Yaml).unwrap();
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let fitted = run
        .lines()
        .iter()
        .find_map(|l| match serde_json::from_str::<Event>(l) {
            Ok(Event::CompactFit { fitted }) => Some(*fitted),
            _ => None,
        })
        .unwrap();
    // as a run would leave it, outside the workspace's jobs/
    let made = dir.join("compact.json");
    fitted.write(&made).unwrap();
    let info = compact::info_of(&made, &fitted);
    assert_eq!(info.parameters, ["gap"]);
    assert_eq!(info.source, "analytic");
    assert_eq!(info.held_out, 4);
    // the design using it has the compact fidelity on its ladder, after the analytic one
    let using = SpecIn {
        fidelity: "compact".into(),
        compact: Some(made.display().to_string()),
        ..ring.clone()
    };
    let ladder: Vec<&str> = compact::fidelities_of(&using)
        .unwrap()
        .iter()
        .map(|m| m.fidelity)
        .collect();
    assert_eq!(ladder, ["analytic", "compact", "3D"]);
    // saved: the model copied next to the design, which names it by a relative path
    let kept = compact::kept_beside(&using, &jobs, "ring").unwrap();
    assert_eq!(kept.compact.as_deref(), Some("ring.compact.json"));
    assert!(jobs.join("ring.compact.json").exists());
    let saved = saved_text_in(&kept, "ring", "", Some(&jobs)).unwrap();
    assert!(saved.contains("compact: ring.compact.json"), "{saved}");
    // read back, found from the design's folder, and computed at its compact fidelity
    let (spec, _, _) = read_text(&saved).unwrap();
    let spec = compact::found_from(spec, &jobs);
    assert_eq!(
        spec.compact,
        Some(jobs.join("ring.compact.json").display().to_string())
    );
    let d = derived(&spec, "compact").unwrap();
    assert_eq!(d.figures[0].fidelity, "compact");
    // saving again over its own file copies nothing
    assert_eq!(
        compact::kept_beside(&spec, &jobs, "ring")
            .unwrap()
            .compact
            .as_deref(),
        Some("ring.compact.json")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The page's numbers are the Python package's: for the same design, the façade the package
/// wraps (`photonoxide::facade`) gives the S-parameters and figures this page shows, bit for
/// bit.
#[test]
fn the_page_and_the_facade_give_the_same_numbers() {
    use photonoxide::facade::{self, DeviceDesign};
    type Design<'a> = (&'a str, &'a str, &'a [(&'a str, f64)]);
    let designs: [Design; 5] = [
        (
            "directional coupler",
            "analytic",
            &[("gap", 0.25), ("length", 12.5)],
        ),
        ("all-pass ring", "analytic", &[("radius", 5.0)]),
        ("waveguide", "analytic", &[("width", 0.45)]),
        ("MMI 1x2", "2D", &[("length", 7.5)]),
        ("Y-branch MZI", "analytic", &[("upper", 120.0)]),
    ];
    let sweep = short();
    let w: Vec<f64> = sweep
        .wavelengths()
        .unwrap()
        .iter()
        .map(|w| w.to_um())
        .collect();
    for (kind, fidelity, values) in designs {
        // the page's design: each value as the form holds it, an expression
        let texts: Vec<(String, String)> = values
            .iter()
            .map(|&(n, v)| (n.to_owned(), format!("{v:?}")))
            .collect();
        let pairs: Vec<(&str, &str)> = texts
            .iter()
            .map(|(n, e)| (n.as_str(), e.as_str()))
            .collect();
        let spec = SpecIn {
            fidelity: fidelity.into(),
            ..design(kind, &pairs)
        };
        let page = spectrum(&spec, fidelity, short()).unwrap();
        let mut d = DeviceDesign::new(kind);
        d.values = values.iter().map(|&(n, v)| (n.to_owned(), v)).collect();
        d.fidelity = Some(fidelity.into());
        let ours = facade::device_spectrum(&d, &w).unwrap();
        let data = serde_json::to_value(&page.data).unwrap();
        let n = ours.ports.len();
        assert_eq!(data["ports"], serde_json::json!(ours.ports), "{kind}");
        for q in 0..n {
            for p in 0..n {
                for k in 0..w.len() {
                    let z = ours.s[(k * n + q) * n + p];
                    let re = data["re"][q][p][k].as_f64().unwrap();
                    let im = data["im"][q][p][k].as_f64().unwrap();
                    assert_eq!(
                        (re.to_bits(), im.to_bits()),
                        (z.re.to_bits(), z.im.to_bits()),
                        "{kind}: S{q}{p} at {}",
                        w[k]
                    );
                }
            }
        }
        assert_eq!(
            (page.fidelity, &page.grid),
            (ours.fidelity.as_str(), &ours.grid)
        );
        let figures = derived(&spec, fidelity).unwrap().figures;
        let theirs = facade::device_derived(&d).unwrap();
        assert_eq!(figures.len(), theirs.len(), "{kind}");
        for (f, g) in figures.iter().zip(&theirs) {
            assert_eq!(
                (&f.symbol, f.value.to_bits()),
                (&g.symbol, g.value.to_bits())
            );
            assert_eq!((f.fidelity, &f.grid), (g.fidelity.as_str(), &g.grid));
        }
    }
}
