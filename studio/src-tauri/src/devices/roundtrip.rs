//! Every device's page, written as its job's text and read back into the page, measured (#668,
//! phase 1). The page is what the Design, Simulate and Study tabs hold: the design (its kind,
//! fidelity, stack and parameters' expressions), its name and description, how a run computes it,
//! and a study of it. Each case is written in YAML as the page writes it, converted by the
//! studio's own format switch to TOML and JSON, and read back as the page reads a file.
//!
//! A case either comes back exactly ("same"), comes back as the same job with the form filling in
//! what the library wrote out (the device's own band, FDTD's cells: "filled"), or loses something
//! or fails. The losses and failures are pinned in [`KNOWN`]: phase 2 of #668 fixes them, and
//! this test fails on any change to the list, a new loss as much as one fixed (take it off then).
//!
//! `cargo test -p photonoxide-studio round_trip -- --nocapture` prints the table that
//! docs/studio-job-editor.md shows.

use photonoxide::device::{self, Device};

use super::*;

/// Whether a loss, (device, tab, format, what was lost), falls under a rule.
type Rule = fn(&str, &str, &str, &str) -> bool;

/// The losses known, each a rule: phase 2 of #668 fixes them. Every part of every loss must fall
/// under one, and every rule must still catch one.
const KNOWN: &[(&str, Rule)] = &[
    (
        "JSON has no comments: a design's description, its opening comment, is lost",
        |_, tab, format, what| tab == "design" && format == "json" && what == "the description",
    ),
    (
        "the file writes [device.parameters] in alphabetical order (the library's TOML tables \
         are sorted); the page shows them in the device's order again, so only the text's \
         order differs",
        |_, tab, _, what| tab == "design" && what == "parameters' order",
    ),
    (
        "the TFLN modulator's page offers a stack of layers, which the library refuses: it is on \
         its own stack",
        |device, tab, _, what| {
            device == "TFLN Mach-Zehnder modulator"
                && tab == "design"
                && what.starts_with("not written: invalid device: a TFLN Mach-Zehnder modulator is on its own stack")
        },
    ),
];

const FORMATS: [(Format, &str); 3] = [
    (Format::Yaml, "yaml"),
    (Format::Toml, "toml"),
    (Format::Json, "json"),
];

#[derive(Clone, Debug, PartialEq)]
enum Outcome {
    Same,
    /// The same job; the form fills in what the library wrote out.
    Filled(String),
    /// Something of the page didn't come back, or the trip failed.
    Lost(String),
    /// The library refuses to write it, and says why: a job the page can't make either.
    Refused(String),
}

fn preset(kind: &str, fidelity: &str) -> SpecIn {
    SpecIn {
        kind: kind.into(),
        fidelity: fidelity.into(),
        stack: StackIn::Preset {
            name: "soi_220".into(),
            bottom_oxide_um: 2.0,
        },
        parameters: Vec::new(),
        compact: None,
    }
}

fn material(id: &str, model: Option<&str>) -> MaterialIn {
    MaterialIn {
        id: id.into(),
        model: model.map(Into::into),
        temperature: None,
        composition: None,
    }
}

/// A stack of layers of its own: 3 µm of oxide (Malitson's model, named) under 220 nm of silicon.
fn layers() -> StackIn {
    StackIn::Layers {
        substrate: material("si", None),
        layers: vec![
            LayerIn {
                name: "BOX".into(),
                thickness_um: 3.0,
                material: material("sio2", Some("malitson-1965")),
                background: material("sio2", None),
            },
            LayerIn {
                name: "Si".into(),
                thickness_um: 0.22,
                material: material("si", None),
                background: material("sio2", None),
            },
        ],
        cladding: material("sio2", None),
        core: "Si".into(),
    }
}

/// The page's designs of device `d` to round-trip, each with a short name for the table.
fn designs(d: &dyn Device) -> Vec<(&'static str, SpecIn)> {
    let kind = d.kind();
    let first = fidelity_name(d.fidelities()[0].fidelity);
    let mut out = vec![("defaults", preset(kind, first))];
    // every fidelity the device has, at its defaults
    let mut seen = vec![first];
    for m in d.fidelities() {
        let f = fidelity_name(m.fidelity);
        if !seen.contains(&f) {
            seen.push(f);
            out.push(("fidelity", preset(kind, f)));
        }
    }
    // every field of the form given, as the form shows it
    let view = geometry(&preset(kind, first));
    let every: Vec<(String, String)> = view
        .fields
        .iter()
        .map(|f| (f.name.clone(), f.expression.clone()))
        .collect();
    out.push((
        "every field",
        SpecIn {
            parameters: every,
            ..preset(kind, first)
        },
    ));
    // a free parameter, named first, that a device parameter takes as its expression
    if let Some(f) = view.fields.iter().find(|f| f.unit == "µm") {
        out.push((
            "expression",
            SpecIn {
                parameters: vec![
                    ("p".into(), f.expression.clone()),
                    (f.name.clone(), "p".into()),
                ],
                ..preset(kind, first)
            },
        ));
    }
    out.push((
        "own stack",
        SpecIn {
            stack: layers(),
            ..preset(kind, first)
        },
    ));
    out
}

/// What of `want` didn't come back as `got`, in words.
fn spec_diff(want: &SpecIn, got: &SpecIn) -> Option<String> {
    if want == got {
        return None;
    }
    let mut out = Vec::new();
    if want.kind != got.kind {
        out.push(format!("kind {} → {}", want.kind, got.kind));
    }
    if want.fidelity != got.fidelity {
        out.push(format!("fidelity {} → {}", want.fidelity, got.fidelity));
    }
    if want.stack != got.stack {
        out.push(format!("stack {:?} → {:?}", want.stack, got.stack));
    }
    if want.parameters != got.parameters {
        let mut sw = want.parameters.clone();
        let mut sg = got.parameters.clone();
        sw.sort();
        sg.sort();
        if sw == sg {
            out.push("parameters' order".into());
        } else {
            out.push(format!(
                "parameters {:?} → {:?}",
                want.parameters, got.parameters
            ));
        }
    }
    if want.compact != got.compact {
        out.push(format!("compact {:?} → {:?}", want.compact, got.compact));
    }
    Some(out.join("; "))
}

fn converted(text: &str, to: Format) -> Result<String, String> {
    crate::studio::convert_job(text.to_owned(), Format::Yaml, to)
}

/// The Design tab: the design saved under a name with a description, and opened again.
fn design_trip(spec: &SpecIn, to: Format) -> Outcome {
    let (name, about) = ("my-design", "A design of my own.\nIts second line.");
    let text = match saved_text(spec, name, about).and_then(|t| converted(&t, to)) {
        Ok(t) => t,
        Err(e) => return Outcome::Lost(format!("not written: {e}")),
    };
    match read_text(&text) {
        Err(e) => Outcome::Lost(format!("not read: {e}")),
        Ok((got, n, a)) => {
            let mut lost: Vec<String> = spec_diff(spec, &got).into_iter().collect();
            if n != name {
                lost.push(format!("name {name} → {n}"));
            }
            if a != about {
                lost.push(if a.is_empty() {
                    "the description".into()
                } else {
                    format!("description → {a:?}")
                });
            }
            if lost.is_empty() {
                Outcome::Same
            } else {
                Outcome::Lost(lost.join("; "))
            }
        }
    }
}

/// The Simulate tab: the job that computes the design as `compute` says, read back into the
/// form; the form then writes the job again, which must be the same text.
fn simulate_trip(spec: &SpecIn, compute: &ComputeIn, to: Format) -> Option<Outcome> {
    // what the library refuses to write (a modulator's FDTD grid), with its reason
    let yaml = match job_text_of(spec, compute, "page-job") {
        Ok(t) => t,
        Err(e) => return Some(Outcome::Refused(e)),
    };
    let text = match converted(&yaml, to) {
        Ok(t) => t,
        Err(e) => return Some(Outcome::Lost(format!("not converted: {e}"))),
    };
    let form = read_job_form(&text);
    if let Some(e) = &form.error {
        return Some(Outcome::Lost(format!("not read: {e}")));
    }
    let mut lost = Vec::new();
    if !form.beyond.is_empty() {
        lost.push(format!("beyond the form: {}", form.beyond.join(", ")));
    }
    match &form.spec {
        Some(got) => lost.extend(spec_diff(spec, got)),
        None => lost.push("the design".into()),
    }
    let Some(got) = &form.compute else {
        lost.push("how it is computed".into());
        return Some(Outcome::Lost(lost.join("; ")));
    };
    if !lost.is_empty() {
        return Some(Outcome::Lost(lost.join("; ")));
    }
    // read back, the form writes the same job
    match job_text_of(form.spec.as_ref()?, got, "page-job") {
        Ok(again) if again == yaml => {}
        Ok(_) => return Some(Outcome::Lost("written again, a different job".into())),
        Err(e) => return Some(Outcome::Lost(format!("not written again: {e}"))),
    }
    Some(if got == compute {
        Outcome::Same
    } else {
        let mut filled = Vec::new();
        let pairs: [(&str, bool); 9] = [
            ("fdtd", got.fdtd != compute.fdtd),
            ("band", got.band != compute.band),
            ("step", got.step_nm != compute.step_nm),
            ("step z", got.step_z_nm != compute.step_z_nm),
            ("margin", got.margin_um != compute.margin_um),
            ("samples", got.samples != compute.samples),
            ("PML", got.pml_cells != compute.pml_cells),
            ("2D FDFD", got.fdfd_2d != compute.fdfd_2d),
            ("timeout", got.timeout_minutes != compute.timeout_minutes),
        ];
        for (n, differs) in pairs {
            if differs {
                filled.push(n);
            }
        }
        Outcome::Filled(filled.join(", "))
    })
}

/// The Study tab: a sweep and corners of a length of the design, read back into its form.
fn study_trip(spec: &SpecIn, d: &dyn Device, to: Format) -> Vec<(&'static str, Outcome)> {
    let Some(p) = d
        .parameters()
        .iter()
        .find(|p| p.unit == "µm" && !["wavelength", "span"].contains(&p.name.as_str()))
    else {
        return Vec::new();
    };
    let compute = ComputeIn {
        band: Some((1.55, 1.55, 1)),
        ..ComputeIn::default()
    };
    let sweep = StudyIn {
        kind: "sweep".into(),
        axes: vec![AxisIn {
            parameter: p.name.clone(),
            from: p.default * 0.95,
            to: p.default * 1.05,
            points: 3,
        }],
        corners: Vec::new(),
    };
    let corners = StudyIn {
        kind: "corners".into(),
        axes: Vec::new(),
        corners: vec![CornerIn {
            name: "wide".into(),
            offsets: vec![(p.name.clone(), p.default * 0.02)],
            temperature_k: None,
        }],
    };
    let mut out = Vec::new();
    for (name, study) in [("sweep", sweep), ("corners", corners)] {
        let outcome = match study_text_of(spec, &compute, &study, "page-study")
            .and_then(|t| converted(&t, to))
        {
            Err(e) => Outcome::Lost(format!("not written: {e}")),
            Ok(text) => {
                let form = read_job_form(&text);
                let mut lost = Vec::new();
                if let Some(e) = form.error {
                    lost.push(format!("not read: {e}"));
                }
                if !form.beyond.is_empty() {
                    lost.push(format!("beyond the form: {}", form.beyond.join(", ")));
                }
                match &form.spec {
                    Some(got) => lost.extend(spec_diff(spec, got)),
                    None => lost.push("the design".into()),
                }
                if form.study.as_ref() != Some(&study) {
                    lost.push(format!("study {:?}", form.study));
                }
                if lost.is_empty() {
                    Outcome::Same
                } else {
                    Outcome::Lost(lost.join("; "))
                }
            }
        };
        out.push((name, outcome));
    }
    out
}

/// The Simulate tab's computes for device `d` at `spec`'s fidelity.
fn computes(d: &dyn Device) -> Vec<(&'static str, ComputeIn)> {
    let mut out = vec![("own model", ComputeIn::default())];
    out.push((
        "band and timeout",
        ComputeIn {
            band: Some((1.5, 1.6, 21)),
            timeout_minutes: Some(12.5),
            ..ComputeIn::default()
        },
    ));
    // the 2D model by FDFD, where the page offers it: at 2D, on a device with one (the MMIs)
    if fdfd_2d(d.kind()).is_some() {
        out.push((
            "2D FDFD",
            ComputeIn {
                fdfd_2d: true,
                ..ComputeIn::default()
            },
        ));
    }
    out.push((
        "FDTD 2D",
        ComputeIn {
            fdtd: Some(2),
            band: Some((1.5, 1.6, 11)),
            ..ComputeIn::default()
        },
    ));
    out
}

#[test]
fn round_trip_of_every_devices_page_through_its_jobs_text_is_measured() {
    // (device, tab, case, format, outcome)
    let mut rows: Vec<(String, &str, String, &str, Outcome)> = Vec::new();
    for d in device::library() {
        let kind = d.kind().to_owned();
        for (fmt, ext) in FORMATS {
            for (case, spec) in designs(d.as_ref()) {
                rows.push((
                    kind.clone(),
                    "design",
                    case.into(),
                    ext,
                    design_trip(&spec, fmt),
                ));
            }
            // the Simulate tab: at each fidelity, each way a run computes it
            let first = fidelity_name(d.fidelities()[0].fidelity);
            let mut fids = vec![first];
            for m in d.fidelities() {
                let f = fidelity_name(m.fidelity);
                if !fids.contains(&f) {
                    fids.push(f);
                }
            }
            for f in &fids {
                for (case, compute) in computes(d.as_ref()) {
                    // FDTD draws the structure: once, at the cheapest fidelity
                    if (compute.fdtd.is_some() && f != &first) || (compute.fdfd_2d && *f != "2D") {
                        continue;
                    }
                    let spec = preset(&kind, f);
                    if let Some(o) = simulate_trip(&spec, &compute, fmt) {
                        rows.push((kind.clone(), "simulate", format!("{f}, {case}"), ext, o));
                    }
                }
            }
            for (case, o) in study_trip(&preset(&kind, first), d.as_ref(), fmt) {
                rows.push((kind.clone(), "study", case.into(), ext, o));
            }
        }
    }

    // the table, a row per device and tab: how many cases came back exactly, filled in, lost,
    // or refused by the library
    println!("| Device | Tab | Cases | Same | Filled in | Lost | Refused |");
    println!("|---|---|---:|---:|---:|---:|---:|");
    let mut keys: Vec<(String, &str)> = Vec::new();
    for (k, t, ..) in &rows {
        if !keys.iter().any(|(a, b)| a == k && b == t) {
            keys.push((k.clone(), t));
        }
    }
    for (k, t) in &keys {
        let of: Vec<&Outcome> = rows
            .iter()
            .filter(|r| &r.0 == k && &r.1 == t)
            .map(|r| &r.4)
            .collect();
        let count = |f: fn(&Outcome) -> bool| of.iter().filter(|o| f(o)).count();
        println!(
            "| {k} | {t} | {} | {} | {} | {} | {} |",
            of.len(),
            count(|o| matches!(o, Outcome::Same)),
            count(|o| matches!(o, Outcome::Filled(_))),
            count(|o| matches!(o, Outcome::Lost(_))),
            count(|o| matches!(o, Outcome::Refused(_))),
        );
    }
    println!();
    println!("What the form fills in, by case:");
    let mut notes: Vec<String> = Vec::new();
    for (_, t, c, _, o) in &rows {
        if let Outcome::Filled(w) = o {
            let n = format!("- {t}: {}: {w}", c.split(", ").last().unwrap_or(c));
            if !notes.contains(&n) {
                notes.push(n);
            }
        }
    }
    println!("{}", notes.join("\n"));
    println!();
    println!("Refused by the library:");
    notes.clear();
    for (k, t, c, _, o) in &rows {
        if let Outcome::Refused(w) = o {
            let n = format!("- {k} | {t} | {c} | {w}");
            if !notes.contains(&n) {
                notes.push(n);
            }
        }
    }
    println!("{}", notes.join("\n"));
    println!();
    println!("Lost, by rule:");
    let mut caught = vec![0_usize; KNOWN.len()];
    let mut new = Vec::new();
    for (k, t, c, f, o) in &rows {
        if let Outcome::Lost(w) = o {
            for part in w.split("; ") {
                match KNOWN.iter().position(|(_, rule)| rule(k, t, f, part)) {
                    Some(i) => caught[i] += 1,
                    None => new.push(format!("{k} | {t} | {c} | {f} | {part}")),
                }
            }
        }
    }
    for ((why, _), n) in KNOWN.iter().zip(&caught) {
        println!("- {n} cases: {why}");
    }
    let fixed: Vec<&str> = KNOWN
        .iter()
        .zip(&caught)
        .filter(|(_, n)| **n == 0)
        .map(|((why, _), _)| *why)
        .collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "the round trip changed: new losses {new:#?}; fixed, take them off KNOWN: {fixed:#?}"
    );
}

/// The Custom device's page is the form of a job's text (form.rs): every job of `jobs/` that
/// isn't a device's, opened into the form, written by the form, in each format, opened again.
#[test]
fn round_trip_of_the_custom_devices_form_through_its_jobs_text_is_measured() {
    use crate::form::{Form, form_text, job_form};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jobs");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| Format::from_path(p).is_some())
        .collect();
    files.sort();
    let fields = |f: &Form| -> serde_json::Map<String, serde_json::Value> {
        match serde_json::to_value(f).unwrap() {
            serde_json::Value::Object(m) => m,
            _ => serde_json::Map::new(),
        }
    };
    let (mut opened, mut same, mut lost, mut device) = (0, 0, Vec::new(), 0);
    println!("| Job | Opens in the form | YAML | TOML | JSON |");
    println!("|---|---|---|---|---|");
    for p in &files {
        let text = std::fs::read_to_string(p).unwrap().replace("\r\n", "\n");
        let fmt = Format::from_path(p).unwrap();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let page = match job_form(text.clone(), fmt) {
            Ok(f) => f,
            Err(e) => {
                if e.contains("device") {
                    device += 1;
                } else {
                    println!("| {name} | no: {e} | | | |");
                }
                continue;
            }
        };
        opened += 1;
        let mut cells = Vec::new();
        for (to, _) in FORMATS {
            let back = form_text(page.clone())
                .and_then(|t| crate::studio::convert_job(t, Format::Toml, to))
                .and_then(|t| job_form(t, to));
            cells.push(match back {
                Ok(b) if b == page => {
                    same += 1;
                    "same".to_owned()
                }
                Ok(b) => {
                    let (a, b) = (fields(&page), fields(&b));
                    let differ: Vec<&String> =
                        a.keys().filter(|k| a.get(*k) != b.get(*k)).collect();
                    let w = format!(
                        "lost: {}",
                        differ
                            .iter()
                            .map(|k| k.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    lost.push(format!("{name} {to:?}: {w}"));
                    w
                }
                Err(e) => {
                    lost.push(format!("{name} {to:?}: {e}"));
                    format!("failed: {e}")
                }
            });
        }
        println!("| {name} | yes | {} |", cells.join(" | "));
    }
    println!();
    println!(
        "{} job files: {device} are devices' (they open on their pages), {opened} open in the form; {same} of {} trips come back the same.",
        files.len(),
        opened * 3
    );
    for l in &lost {
        println!("- {l}");
    }
    assert!(opened > 0);
    // the one loss known, phase 2's to fix: JSON has no comments, so the description goes
    let new: Vec<&String> = lost
        .iter()
        .filter(|l| !l.ends_with("Json: lost: about"))
        .collect();
    assert!(
        new.is_empty(),
        "the Custom device's round trip changed: {new:#?}"
    );
    assert!(
        !lost.is_empty(),
        "JSON keeps the description now: take its rule off"
    );
}
