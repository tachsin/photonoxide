//! Components as job files (#668, phases 2 to 5): the built-in components' files are what their
//! pages build at their defaults, to the bit; each is the same component in YAML, TOML and JSON;
//! an edit of the text moves the page; a component designed is saved under its name, reloads the
//! same, passes the check and runs; a built-in component's name, or one taken, is refused.

use photonoxide::run::{Run, Stop};

use super::*;
use crate::examples::components;

const FORMATS: [Format; 3] = [Format::Yaml, Format::Toml, Format::Json];

/// The design a device's page holds when it is first shown (`defaultSpec`, lib/device.svelte.ts):
/// its cheapest model on 220 nm SOI, every parameter at its default.
fn page_default(d: &dyn Device) -> SpecIn {
    SpecIn {
        kind: d.kind().into(),
        fidelity: fidelity_name(d.fidelities()[0].fidelity).into(),
        stack: StackIn::Preset {
            name: "soi_220".into(),
            bottom_oxide_um: 2.0,
        },
        parameters: Vec::new(),
        compact: None,
    }
}

/// `spec` with every field of its form given as the form shows it: each parameter in the file,
/// so that the file says every number.
fn every_field(spec: &SpecIn) -> SpecIn {
    SpecIn {
        parameters: geometry(spec)
            .fields
            .into_iter()
            .map(|f| (f.name, f.expression))
            .collect(),
        ..spec.clone()
    }
}

fn id_of(kind: &str) -> &'static str {
    DEVICES.iter().find(|e| e.0 == kind).unwrap().1
}

fn bits(v: Option<Vec<f64>>) -> Vec<u64> {
    v.unwrap().iter().map(|x| x.to_bits()).collect()
}

/// Phase 5: each device of the library has its file in jobs/components/, and the file is what its
/// page builds at its defaults: the text its Save writes with every field shown, which reads back
/// to the same design and the same values to the bit. `PHOTONOXIDE_WRITE_COMPONENTS=1` writes
/// the files when the library changes them.
#[test]
fn every_built_in_component_is_what_its_page_builds_at_its_defaults() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jobs/components");
    let write = std::env::var_os("PHOTONOXIDE_WRITE_COMPONENTS").is_some();
    let built = components();
    let library = device::library();
    assert_eq!(built.len(), library.len(), "a file for each device");
    for (d, file) in library.iter().zip(&built) {
        let id = id_of(d.kind());
        assert_eq!(
            file.file,
            format!("components/{id}.yaml"),
            "the library's order"
        );
        let page = page_default(d.as_ref());
        let shown = every_field(&page);
        let want = saved_text(&shown, id, d.about()).unwrap();
        if write {
            std::fs::write(dir.join(format!("{id}.yaml")), &want).unwrap();
            continue;
        }
        assert_eq!(
            file.text, want,
            "{id}: not what the page builds; PHOTONOXIDE_WRITE_COMPONENTS=1 writes it"
        );
        // read back: the design the page holds, its name and description
        let (spec, name, about) = read_text(&file.text).unwrap();
        assert_eq!((name.as_str(), about.as_str()), (id, d.about()));
        assert_eq!(spec, shown, "{id}");
        // the same values as the page's default, to the bit, and the same structure
        let (a, b) = (geometry(&spec), geometry(&page));
        assert!(a.problem.is_none(), "{id}: {:?}", a.problem);
        assert_eq!(bits(a.values), bits(b.values), "{id}");
        assert_eq!(
            serde_json::to_string(&a.scene).unwrap(),
            serde_json::to_string(&b.scene).unwrap(),
            "{id}"
        );
        // and the Design page lists it, a design alone
        let read = read_design(&file.text, Format::Yaml);
        assert!(
            read.error.is_none() && read.beyond.is_empty(),
            "{id}: {read:?}"
        );
    }
    assert!(
        !write,
        "the files are written: run again without PHOTONOXIDE_WRITE_COMPONENTS"
    );
}

/// Phase 4: every built-in component is the same in YAML, TOML and JSON (the same job, the same
/// design, name and description), and the editor's format switch goes from any of them to any
/// other and back to the same text.
#[test]
fn every_built_in_component_is_the_same_in_yaml_toml_and_json() {
    let content = |j: &Job| {
        (
            j.name().to_owned(),
            j.description().map(str::to_owned),
            j.device().cloned(),
            j.task().clone(),
        )
    };
    for c in components() {
        let yaml = c.text.clone();
        let read = read_design(&yaml, Format::Yaml);
        assert!(read.spec.is_some() && read.error.is_none(), "{}", c.file);
        let job = Job::parse_as(&yaml, Format::Yaml).unwrap();
        for from in FORMATS {
            let text = crate::studio::convert_job(yaml.clone(), Format::Yaml, from).unwrap();
            // the page writes the same text in this format as the switch makes
            let (spec, name, about) = (read.spec.as_ref().unwrap(), &read.name, &read.about);
            assert_eq!(
                text_in(spec, name, about, from).unwrap(),
                text,
                "{} {from:?}",
                c.file
            );
            assert_eq!(read_design(&text, from), read, "{} {from:?}", c.file);
            assert_eq!(
                content(&Job::parse_as(&text, from).unwrap()),
                content(&job),
                "{} {from:?}",
                c.file
            );
            for to in FORMATS {
                let there = crate::studio::convert_job(text.clone(), from, to).unwrap();
                assert_eq!(
                    read_design(&there, to),
                    read,
                    "{} {from:?} → {to:?}",
                    c.file
                );
                let back = crate::studio::convert_job(there, to, Format::Yaml).unwrap();
                assert_eq!(back, yaml, "{} {from:?} → {to:?} → YAML", c.file);
            }
        }
    }
}

/// Phase 2: an edit made in the text, in any format, reads back into the page: the directional
/// coupler's gap typed in the text is the form's gap.
#[test]
fn an_edit_in_the_text_moves_the_page_in_every_format() {
    let d = device::library()
        .into_iter()
        .find(|d| d.kind() == "directional coupler")
        .unwrap();
    let shown = every_field(&page_default(d.as_ref()));
    let gap = shown
        .parameters
        .iter()
        .find(|(n, _)| n == "gap")
        .unwrap()
        .1
        .clone();
    for format in FORMATS {
        let text = text_in(&shown, "my-coupler", "Mine.", format).unwrap();
        let at = text.find("gap").unwrap();
        let edited = format!("{}{}", &text[..at], text[at..].replacen(&gap, "0.25", 1));
        let read = read_design(&edited, format);
        assert!(read.error.is_none(), "{format:?}: {read:?}\n{edited}");
        let spec = read.spec.unwrap();
        let got = spec.parameters.iter().find(|(n, _)| n == "gap").unwrap();
        assert_eq!(got.1, "0.25", "{format:?}");
        // and the page writes the text again as it was typed
        assert_eq!(
            text_in(&spec, "my-coupler", "Mine.", format).unwrap(),
            edited,
            "{format:?}"
        );
    }
}

/// Phase 2: the page offers a stack to edit wherever the library takes one: on every device but
/// the TFLN modulator, which is on its own.
#[test]
fn only_the_modulator_is_on_a_stack_of_its_own() {
    let own: Vec<String> = device::library()
        .iter()
        .filter(|d| own_stack(d.kind(), d.fidelities()[0].fidelity))
        .map(|d| d.kind().to_owned())
        .collect();
    assert_eq!(own, ["TFLN Mach-Zehnder modulator"]);
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("photonoxide-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Phase 3: a component designed, named and saved in each format reloads as the same component,
/// is listed as one of the workspace's, passes the library's check and runs.
#[test]
fn a_component_designed_is_saved_reloads_the_same_and_runs() {
    let dir = scratch("components");
    let wide = |spec: &SpecIn| SpecIn {
        parameters: spec
            .parameters
            .iter()
            .map(|(n, e)| {
                (
                    n.clone(),
                    if n == "width" {
                        "0.6".into()
                    } else {
                        e.clone()
                    },
                )
            })
            .collect(),
        ..spec.clone()
    };
    let base = read_design(&components()[0].text, Format::Yaml);
    let mine = wide(base.spec.as_ref().unwrap());
    for format in FORMATS {
        let name = format!("wide-guide-{}", format.extension());
        let about = "A 600 nm guide of my own.\nIts second line.";
        let text = text_in(&mine, &name, about, format).unwrap();
        let path = save_component_in(&dir, &text, format, false).unwrap();
        assert_eq!(path.extension().unwrap(), format.extension());
        // reloaded as the page opens it: the same design, name and description
        let saved = read_device(path.display().to_string()).unwrap();
        assert_eq!(
            (&saved.spec, saved.name.as_str(), saved.about.as_str()),
            (&mine, name.as_str(), about),
            "{format:?}"
        );
        assert!(saved.job.is_none(), "a design alone");
        // the file passes the check and runs, as `photonoxide run` runs it
        let job = Job::load(&path).unwrap();
        job::check(&job).unwrap();
        let mut run = Run::in_memory();
        job::execute(&job, &mut run, &Stop::new(None)).unwrap();
        assert!(
            run.lines().iter().any(|l| l.contains("\"scene\"")),
            "{format:?}"
        );
    }
    let listed: Vec<(String, String)> = mine_in(&dir)
        .into_iter()
        .map(|f| (f.name, f.kind))
        .collect();
    assert_eq!(
        listed,
        [
            ("wide-guide-json".to_owned(), "waveguide".to_owned()),
            ("wide-guide-toml".to_owned(), "waveguide".to_owned()),
            ("wide-guide-yaml".to_owned(), "waveguide".to_owned()),
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 3: names that clash are refused, saying with what: a built-in component's (in any
/// case), a component's of the workspace in any format; a text that isn't a design alone too.
#[test]
fn a_name_taken_or_a_built_in_ones_is_refused() {
    let dir = scratch("component-names");
    let base = read_design(&components()[2].text, Format::Yaml);
    let spec = base.spec.unwrap();
    for name in ["directional-coupler", "Directional-Coupler", "waveguide"] {
        let text = text_in(&spec, name, "", Format::Yaml).unwrap();
        let e = save_component_in(&dir, &text, Format::Yaml, false).unwrap_err();
        let kind = built_in_kind(name).unwrap();
        assert!(e.contains(&format!("the built-in {kind}'s name")), "{e}");
    }
    // one of mine, then the same name in another format: refused, unless replaced
    let yaml = text_in(&spec, "my-coupler", "", Format::Yaml).unwrap();
    let first = save_component_in(&dir, &yaml, Format::Yaml, false).unwrap();
    let json = text_in(&spec, "my-coupler", "Now in JSON.", Format::Json).unwrap();
    let e = save_component_in(&dir, &json, Format::Json, false).unwrap_err();
    assert_eq!(e, format!("exists: {}", first.display()));
    let second = save_component_in(&dir, &json, Format::Json, true).unwrap();
    assert!(second.exists() && !first.exists(), "one file a name");
    // a job that computes the design isn't a component
    let job = job_text_of(&spec, &ComputeIn::default(), "my-run").unwrap();
    let e = save_component_in(&dir, &job, Format::Yaml, false).unwrap_err();
    assert!(e.starts_with("a component is a design alone; task"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}
