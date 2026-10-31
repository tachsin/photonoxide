//! A flow's export: its JSON read back is the same flow, its cross-sections are well-formed
//! XML with the layers from the top down, and its traveller holds every step and every source.

use super::*;
use crate::fabrication::fabricate;

/// The three first designs, at their defaults, and a rib waveguide, a timed partial etch.
const DESIGNS: [&str; 4] = [
    "waveguide",
    "TFLN Mach-Zehnder modulator",
    "thermo-optic phase shifter",
    "rib waveguide",
];

fn export(kind: &str) -> FlowExport {
    FlowExport::of_design(&Spec::new(kind), &Choices::default())
        .unwrap_or_else(|e| panic!("{kind}: {e}"))
}

/// Checks that `text` is well-formed XML: one root, every tag closed in order, every attribute
/// quoted, no `<` or bare `&` in the text, and returns the elements' names in document order.
fn well_formed(text: &str) -> Vec<String> {
    let mut stack: Vec<String> = Vec::new();
    let mut names = Vec::new();
    let mut roots = 0;
    let mut rest = text.trim_start();
    if let Some(after) = rest.strip_prefix("<?xml") {
        rest = &after[after.find("?>").expect("the declaration ends") + 2..];
    }
    let text_ok = |t: &str| {
        assert!(!t.contains('<'), "a < in text: {t:?}");
        for (i, _) in t.match_indices('&') {
            let entity = &t[i..t[i..].find(';').map_or(t.len(), |e| i + e + 1)];
            assert!(
                ["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"].contains(&entity),
                "a bare & in text: {t:?}"
            );
        }
    };
    while let Some(open) = rest.find('<') {
        text_ok(&rest[..open]);
        if stack.is_empty() {
            assert!(
                rest[..open].trim().is_empty(),
                "text outside the root: {:?}",
                &rest[..open]
            );
        }
        let close = rest[open..].find('>').expect("a tag ends") + open;
        let tag = &rest[open + 1..close];
        rest = &rest[close + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            let top = stack.pop().expect("a closing tag closes an open one");
            assert_eq!(top, name.trim(), "tags closed out of order");
            continue;
        }
        let empty = tag.ends_with('/');
        let body = tag.trim_end_matches('/');
        let name: String = body.chars().take_while(|c| !c.is_whitespace()).collect();
        assert!(
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "a tag's name: {tag:?}"
        );
        // its attributes: name="value", each value quoted and without < or a bare &
        let mut attributes = body[name.len()..].trim();
        while !attributes.is_empty() {
            let eq = attributes.find('=').expect("an attribute has a value");
            let value = &attributes[eq + 1..];
            assert!(value.starts_with('"'), "an unquoted attribute in {tag:?}");
            let end = value[1..].find('"').expect("a quoted value ends") + 1;
            text_ok(&value[1..end]);
            attributes = value[end + 1..].trim_start();
        }
        if stack.is_empty() {
            roots += 1;
        }
        names.push(name.clone());
        if !empty {
            stack.push(name);
        }
    }
    assert!(rest.trim().is_empty(), "text after the root: {rest:?}");
    assert!(stack.is_empty(), "tags left open: {stack:?}");
    assert_eq!(roots, 1, "one root");
    names
}

#[test]
fn the_flows_json_read_back_is_the_same_flow_and_its_record() {
    for kind in DESIGNS {
        let e = export(kind);
        let text = e.file.json();
        let back = FlowFile::read(&text).unwrap();
        assert_eq!(back, e.file, "{kind}");
        assert_eq!(back.json(), text, "{kind}: written again, the same text");
        // the flow and the assessment are what a run records for the design
        let (device, design) = Spec::new(kind).build().unwrap();
        let physical = device.physical(design.values()).unwrap();
        let choices = Choices::default();
        assert_eq!(
            e.file.flow,
            fabricate(&physical, &choices).unwrap(),
            "{kind}"
        );
        let flow = generate(&physical, &choices).unwrap();
        assert_eq!(
            e.file.assessment,
            assess(&physical, &flow).unwrap(),
            "{kind}"
        );
        assert!(e.file.flow.round_trip.unwrap().exact(), "{kind}");
        assert_eq!(e.file.stacks.len(), e.file.flow.steps.len(), "{kind}");
        // its keys, in their order
        let keys: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with("  \""))
            .map(|l| l.trim().split('"').nth(1).unwrap())
            .collect();
        assert_eq!(
            keys,
            [
                "device",
                "parameters",
                "cut",
                "flow",
                "assessment",
                "stacks"
            ],
            "{kind}"
        );
    }
    assert!(FlowFile::read("{\"device\": 1}").is_err());
}

#[test]
fn each_steps_cross_section_is_well_formed_svg_with_its_layers_from_the_top_down() {
    for kind in DESIGNS {
        let e = export(kind);
        let svgs = e.svgs();
        assert_eq!(svgs.len(), e.file.flow.steps.len(), "{kind}");
        for (k, svg) in svgs.iter().enumerate() {
            let names = well_formed(svg);
            assert_eq!(names[0], "svg", "{kind} {k}");
            assert!(svg.contains(&format!("Step {} of {}", k + 1, svgs.len())));
            // the legend lists every layer of the stack, the cladding first, the substrate last
            let rows = &e.file.stacks[k];
            let legend = &svg[svg.find("<g id=\"layers\">").unwrap()..];
            let mut at = 0;
            for row in rows {
                let found = legend[at..]
                    .find(&escape(&describe(row)))
                    .unwrap_or_else(|| panic!("{kind} {k}: {row:?} in order"));
                at += found + 1;
            }
            assert_eq!(rows.first().unwrap().name, "cladding");
            assert_eq!(rows.last().unwrap().name, "substrate");
            assert!(svg.contains(LAYERS_TOP_DOWN) || svg.contains("from the top down"));
        }
        // the last step's wafer: every patterned layer is drawn as a profile
        let last = svgs.last().unwrap();
        let patterned = e
            .file
            .stacks
            .last()
            .unwrap()
            .iter()
            .filter(|r| r.elsewhere.is_some())
            .count();
        assert!(
            last.matches("<polygon").count() >= patterned.min(1),
            "{kind}"
        );
    }
}

#[test]
fn a_rib_is_drawn_with_its_slab_and_a_strip_with_none() {
    // across the strip, the drawn silicon stands 0.22 µm only over the strip; the rib's stands
    // at its slab's height beside it
    let strip = export("waveguide");
    let k = strip.file.flow.steps.len() - 1;
    let s = Section::new(&strip.file, &strip.made[k], k);
    let profile = s.profile("Si", 2.0);
    let heights: Vec<f64> = profile.iter().map(|p| p.1).collect();
    let floor = s.pz(2.0);
    let top = s.pz(2.22);
    assert!(heights.iter().any(|&h| (h - top).abs() < 1e-6));
    assert!(
        heights
            .iter()
            .all(|&h| h <= floor + 1e-9 && h >= top - 1e-9)
    );
    // the strip's edges: few points, the collinear ones dropped
    assert!(profile.len() < 20, "{}", profile.len());
    let rib = export("rib waveguide");
    let k = rib.file.flow.steps.len() - 1;
    let row = rib.file.stacks[k]
        .iter()
        .find(|r| r.elsewhere.is_some())
        .expect("the rib's layer is patterned")
        .clone();
    let s = Section::new(&rib.file, &rib.made[k], k);
    let mut bottom = 0.0;
    for l in rib.made[k].structure.stack().layers() {
        if l.name == row.name {
            break;
        }
        bottom += l.thickness.to_um();
    }
    let profile = s.profile(&row.name, bottom);
    // beside the rib, its slab: neither the layer's floor nor its top
    let edge = profile[0].1;
    assert!(edge < s.pz(bottom) - 1e-6, "{profile:?}");
    assert!(
        edge > s.pz(bottom + row.thickness_um.unwrap()) + 1e-6,
        "{profile:?}"
    );
}

#[test]
fn the_traveller_has_every_step_every_source_and_every_dimension() {
    for kind in DESIGNS {
        let e = export(kind);
        let md = e.file.markdown();
        let n = e.file.flow.steps.len();
        let mut at = 0;
        for (k, s) in e.file.flow.steps.iter().enumerate() {
            let heading = format!("### Step {} of {n}: {}\n", k + 1, s.name);
            let found = md[at..]
                .find(&heading)
                .unwrap_or_else(|| panic!("{kind}: {heading}"));
            at += found;
            assert!(md.contains(&s.reason), "{kind}: step {k}'s reason");
            assert!(md.contains(&s.does), "{kind}: step {k}'s operation");
            assert!(md.contains(&FlowExport::svg_name(k, n)));
            for a in &s.alternatives {
                assert!(md.contains(&a.why_not), "{kind}: {a:?}");
            }
            for c in &s.capabilities {
                if let Some(src) = c.value.source() {
                    assert!(md.contains(&e.file.cite(src)), "{kind}: {src:?}");
                }
            }
        }
        for r in &e.file.flow.references {
            assert!(md.contains(&r.citation), "{kind}: {}", r.key);
            assert!(r.doi.is_empty() || md.contains(&r.doi), "{kind}: {}", r.key);
        }
        for d in &e.file.assessment.accuracy {
            assert!(md.contains(&d.text), "{kind}: {}", d.what);
            // under the step that sets it, too
            let k = e
                .file
                .owner(d)
                .unwrap_or_else(|| panic!("{kind}: no step sets {}", d.what));
            let start = md.find(&format!("### Step {} of {n}:", k + 1)).unwrap();
            let end = md[start + 1..]
                .find("\n### ")
                .map_or(md.len(), |e| start + 1 + e);
            assert!(
                md[start..end].contains(&format!("  - {}\n", d.text)),
                "{kind}: {}",
                d.what
            );
        }
        for w in &e.file.flow.warnings {
            assert!(md.contains(w), "{kind}: {w}");
        }
        assert!(md.contains(LAYERS_TOP_DOWN));
        // the design's layers from the top down: the cladding first, the substrate last
        let table = &md[md.find("| Layer | Film |").unwrap()..];
        let table = &table[..table.find("\n\n").unwrap()];
        let layers: Vec<&str> = table
            .lines()
            .skip(2)
            .map(|l| l.split('|').nth(1).unwrap().trim())
            .collect();
        let want: Vec<&str> = e
            .file
            .stacks
            .last()
            .unwrap()
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(layers, want, "{kind}");
        assert_eq!(layers.first(), Some(&"cladding"));
        assert_eq!(layers.last(), Some(&"substrate"));
    }
}

#[test]
fn the_export_writes_its_files_into_a_folder() {
    let dir = std::env::temp_dir().join(format!("photonoxide-flow-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let e = export("waveguide");
    let files = e.write(&dir).unwrap();
    let n = e.file.flow.steps.len();
    assert_eq!(files.len(), 2 + n);
    assert_eq!(files[0], dir.join("flow.json"));
    assert_eq!(files[1], dir.join("flow.md"));
    assert_eq!(files[2], dir.join("flow-01.svg"));
    let back = FlowFile::read(&std::fs::read_to_string(&files[0]).unwrap()).unwrap();
    assert_eq!(back, e.file);
    assert_eq!(
        std::fs::read_to_string(&files[1]).unwrap(),
        e.file.markdown()
    );
    assert_eq!(std::fs::read_to_string(&files[2]).unwrap(), e.svg(0));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(FlowExport::svg_name(0, 9), "flow-01.svg");
    assert_eq!(FlowExport::svg_name(99, 120), "flow-100.svg");
}
