//! A `"fabrication"` job: checked, run, its record replayed to the bit on any threads.

use super::*;
use crate::run::{replay, replay_lines};

const TFLN: &str = "name = \"tfln-flow\"\n\n[device]\nkind = \"TFLN Mach-Zehnder modulator\"\n\n[task]\nkind = \"fabrication\"\n";

const HEATER: &str = "name = \"heater-flow\"\n\n[device]\nkind = \"thermo-optic phase shifter\"\n\n[task]\nkind = \"fabrication\"\n";

struct Dir(std::path::PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dir(tag: &str) -> Dir {
    let d = std::env::temp_dir().join(format!(
        "photonoxide-fabrication-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    Dir(d)
}

/// The record's lines on a pool of `threads`, without the finish's (its time).
fn lines_on(text: &str, threads: usize, tag: &str) -> (Vec<String>, Vec<Event>) {
    let root = dir(&format!("{tag}-{threads}"));
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| execute(&job, &mut run, &Stop::new(None)))
        .unwrap();
    let lines = replay_lines(run.dir())
        .unwrap()
        .into_iter()
        .filter(|l| !l.starts_with("{\"type\":\"finished\""))
        .collect();
    let events = replay(run.dir()).unwrap();
    (lines, events)
}

#[test]
fn a_fabrication_job_records_its_flow_and_the_round_trip_is_exact() {
    for (text, steps) in [(TFLN, 8), (HEATER, 12)] {
        let job = Job::parse(text).unwrap();
        check(&job).unwrap();
        assert!(matches!(preview(&job).unwrap(), Event::Scene { .. }));
        let (_, events) = lines_on(text, 1, "record");
        let flow = events
            .iter()
            .find_map(|e| match e {
                Event::Flow { flow } => Some(flow),
                _ => None,
            })
            .expect("a flow");
        assert_eq!(flow.steps.len(), steps, "{flow:#?}");
        let trip = flow.round_trip.expect("its round trip");
        assert!(trip.exact(), "{trip:?}");
        assert_eq!(flow.library, crate::fabrication::LIBRARY_VERSION);
        assert!(events.iter().any(|e| matches!(e, Event::Scene { .. })));
    }
}

#[test]
fn a_fabrication_run_writes_its_flows_json_traveller_and_cross_sections() {
    let root = dir("export");
    let job = Job::parse(HEATER).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events = replay(run.dir()).unwrap();
    let (flow, assessment) = (
        events.iter().find_map(|e| match e {
            Event::Flow { flow } => Some(flow.as_ref().clone()),
            _ => None,
        }),
        events.iter().find_map(|e| match e {
            Event::FlowAssessment { assessment } => Some(assessment.as_ref().clone()),
            _ => None,
        }),
    );
    let text = std::fs::read_to_string(run.dir().join("flow.json")).unwrap();
    let file = crate::export::flow::FlowFile::read(&text).unwrap();
    // the files hold what the record holds, to the bit
    assert_eq!(Some(file.flow.clone()), flow);
    assert_eq!(Some(file.assessment.clone()), assessment);
    assert_eq!(file.device, "thermo-optic phase shifter");
    let md = std::fs::read_to_string(run.dir().join("flow.md")).unwrap();
    assert_eq!(md, file.markdown());
    for k in 0..file.flow.steps.len() {
        let name = crate::export::flow::FlowExport::svg_name(k, file.flow.steps.len());
        let svg = std::fs::read_to_string(run.dir().join(&name)).unwrap();
        assert!(svg.starts_with("<?xml"), "{name}");
        assert!(md.contains(&name));
    }
}

/// The fenced block of `page` that opens with `fence`, its lines.
fn block<'a>(page: &'a str, fence: &str) -> Vec<&'a str> {
    let start = page
        .find(&format!("\n{fence}\n"))
        .unwrap_or_else(|| panic!("a {fence} block"))
        + fence.len()
        + 2;
    let end = start + page[start..].find("\n```\n").expect("the block ends");
    page[start..end].lines().collect()
}

/// docs/methods/fabrication.md's worked example: the strip's job (jobs/strip-flow.yaml, which
/// the page shows as written: job::tests checks that), what its Python block prints, which is the
/// flow's steps and dimensions, and the traveller's excerpt, each of its lines a line of the
/// `flow.md` the job's run writes, in order.
#[test]
fn the_write_ups_strip_flow_is_the_jobs_own() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(here.join("docs/methods/fabrication.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let job = Job::load(&here.join("jobs/strip-flow.yaml")).unwrap();
    let root = dir("write-up");
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let md = std::fs::read_to_string(run.dir().join("flow.md")).unwrap();
    let file = crate::export::flow::FlowFile::read(
        &std::fs::read_to_string(run.dir().join("flow.json")).unwrap(),
    )
    .unwrap();
    let printed: Vec<String> = file
        .flow
        .steps
        .iter()
        .enumerate()
        .map(|(k, s)| format!("{}. {} ({}): {}", k + 1, s.name, s.process, s.does))
        .chain(file.assessment.accuracy.iter().map(|d| d.text.clone()))
        .collect();
    assert_eq!(block(&page, "```text"), printed);
    assert_eq!(page.matches("\n```markdown\n").count(), 1);
    let excerpt = block(&page, "```markdown");
    assert!(excerpt.len() > 20, "{}", excerpt.len());
    let mut lines = md.lines();
    for line in excerpt.iter().filter(|l| !l.trim().is_empty()) {
        assert!(
            lines.any(|l| l == *line),
            "the traveller hasn't, in this order: {line}"
        );
    }
    // every dimension is in the excerpt's table
    for d in &file.assessment.accuracy {
        assert!(
            excerpt
                .iter()
                .any(|l| l.starts_with(&format!("| {} |", d.what))),
            "{}",
            d.what
        );
    }
}

#[test]
fn a_fabrication_job_made_from_a_design_is_one_that_runs() {
    let spec = crate::device::Spec::new("TFLN Mach-Zehnder modulator");
    let job = from_fabrication("tfln-flow", &spec, &["ebl-pmma".into()], &[]).unwrap();
    assert_eq!(job.name(), "tfln-flow");
    check(&job).unwrap();
    let text = job.to_text(crate::run::Format::Yaml).unwrap();
    assert!(text.contains("kind: fabrication"), "{text}");
    assert!(text.contains("ebl-pmma"), "{text}");
    assert!(!text.contains("avoid"), "{text}");
    let e = from_fabrication("tfln-flow", &spec, &["icp-si-hbr".into()], &[]).unwrap_err();
    assert!(e.to_string().contains("makes no step"), "{e}");
}

#[test]
fn a_fabrication_record_replays_and_repeats_on_any_threads() {
    let (one, events) = lines_on(TFLN, 1, "threads");
    let (four, _) = lines_on(TFLN, 4, "threads");
    assert_eq!(one, four);
    // each line read back and written again is the same line
    for (line, event) in one.iter().zip(&events) {
        assert_eq!(&serde_json::to_string(event).unwrap(), line);
    }
}

#[test]
fn a_fabrication_job_refuses_processes_the_library_hasnt_and_preferences_that_do_nothing() {
    let refused = |extra: &str, words: &str| {
        let job = Job::parse(&format!("{TFLN}{extra}\n")).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(words), "{extra}: {e}");
    };
    refused("prefer = [\"laser-writing\"]", "laser-writing");
    refused(
        "avoid = [\"ebl-hsq\"]\nprefer = [\"ebl-hsq\"]",
        "both preferred and avoided",
    );
    // a silicon etch makes no step of a lithium niobate design
    refused("prefer = [\"icp-si-hbr\"]", "makes no step");
    // a preference that qualifies is taken
    let job = Job::parse(&format!("{TFLN}prefer = [\"ebl-pmma\"]\n")).unwrap();
    check(&job).unwrap();
}

#[test]
fn the_fabrication_view_has_the_wafer_after_every_step() {
    let job = Job::parse(TFLN).unwrap();
    let spec = super::device::spec_of(&job).unwrap();
    let view = fabrication_of(&spec, &[], &[]).unwrap();
    assert_eq!(view.stages.len(), view.flow.steps.len());
    assert!(view.flow.round_trip.unwrap().exact());
    // the rib's etch is in the last stage: 0.3 µm deep, walls at 70°, on the 0.6 µm film
    let last = view.stages.last().unwrap();
    let rib = last.etched.first().expect("an etched shape");
    assert_eq!(rib.layer, "LN");
    assert!((rib.depth_um - 0.3).abs() < 1e-12, "{rib:?}");
    assert!((rib.sidewall_deg - 70.0).abs() < 1e-9, "{rib:?}");
    assert!((rib.z_um[1] - rib.z_um[0] - 0.6).abs() < 1e-12, "{rib:?}");
    assert!(last.etched.iter().all(|e| e.layer == "LN"));
    assert!(matches!(last.scene, Event::Scene { .. })); // the first stage is the bare wafer, open to the air above it
    let Event::Scene {
        cladding, shapes, ..
    } = &view.stages[0].scene
    else {
        panic!("a scene")
    };
    assert!(shapes.is_empty());
    assert!(cladding.eps < 1.0 + 1e-12, "{cladding:?}");
}

/// A scene's layers' names, bottom to top.
fn layer_names(e: &Event) -> Vec<String> {
    match e {
        Event::Scene { layers, .. } => layers.iter().map(|l| l.name.clone()).collect(),
        _ => panic!("a scene"),
    }
}

#[test]
fn the_frames_play_the_flow_from_the_bare_wafer_to_the_design() {
    for text in [TFLN, HEATER] {
        let job = Job::parse(text).unwrap();
        let view = fabrication_of_job(&job, &[], &[]).unwrap();
        let steps = &view.flow.steps;
        let patterns = steps.iter().filter(|s| s.operation == "pattern").count();
        assert!(patterns > 0, "{text}");
        // a frame a step, three a lithography: coated, written, developed
        assert_eq!(view.frames.len(), steps.len() + 2 * patterns, "{text}");
        assert!(view.frames.windows(2).all(|w| w[0].step <= w[1].step));
        assert_eq!(view.frames[0].phase, "wafer");
        assert_eq!(view.frames[0].step, 0);
        // the last frame is the design as the last stage has it, the resist gone
        let (last, stage) = (view.frames.last().unwrap(), view.stages.last().unwrap());
        assert_eq!(layer_names(&last.scene), layer_names(&stage.scene));
        assert_eq!(last.etched, stage.etched);
        assert!(
            !layer_names(&last.scene)
                .iter()
                .any(|l| l.starts_with("resist"))
        );
        assert!(last.films.is_empty(), "{:?}", last.films);
        // while the resist is on the wafer, it is drawn as films over the layers, named for
        // itself, its bottom on the wafer and its top over the wafer's top
        for f in view.frames.iter().filter(|f| f.phase == "develop") {
            assert!(!f.films.is_empty(), "{}", f.label);
            let Event::Scene { layers, .. } = &f.scene else {
                panic!("a scene")
            };
            let top = layers.last().unwrap().z_um[1];
            for film in &f.films {
                assert_eq!(film.layer, "resist", "{}", f.label);
                assert!(film.film.ends_with("resist"), "{}", film.film);
                assert!(film.z_um[1] > film.z_um[0] && film.z_um[1] <= top + 1.0);
            }
            assert!(f.films.iter().any(|film| film.z_um[1] > top + 0.1));
        }
        for f in view.frames.iter().filter(|f| f.phase == "coat") {
            assert_eq!(f.layer.as_deref(), Some("resist"));
            assert!(f.label.starts_with("spin the "), "{}", f.label);
        }
    }
    let tfln = fabrication_of_job(&Job::parse(TFLN).unwrap(), &[], &[]).unwrap();
    assert_eq!(tfln.platform, Some(crate::fabrication::Platform::Lnoi));
}

#[test]
fn a_structure_job_has_its_flow_and_a_job_without_a_design_says_why_not() {
    let text = "name = \"strip\"\n\n[task]\nkind = \"structure\"\nstack = \"soi_220\"\n\
                wavelength_um = 1.55\nlayer = \"Si\"\nx_um = [-2.0, 2.0]\ny_um = [-1.0, 1.0]\n\
                step_nm = 20.0\n\n[[task.rect]]\nlayer = \"Si\"\ncenter_um = [0.0, 0.0]\n\
                size_um = [4.0, 0.5]\n";
    let view = fabrication_of_job(&Job::parse(text).unwrap(), &[], &[]).unwrap();
    assert!(view.flow.round_trip.unwrap().exact());
    assert_eq!(view.platform, Some(crate::fabrication::Platform::Soi));
    let modes = "name = \"m\"\n\n[task]\nkind = \"modes\"\n";
    let e = fabrication_of_job(&Job::parse(modes).unwrap(), &[], &[])
        .unwrap_err()
        .to_string();
    assert!(e.contains("no [device] table"), "{e}");
}

// ---- the films over the topography: the resist spun on, a conformal film, the exposure ----

/// A scene outline's rings: the boundary back to its first vertex, then each hole back to its
/// own (see `SceneShape::outline`).
fn rings_of(outline: &[[f64; 2]]) -> Vec<Vec<[f64; 2]>> {
    let mut rings = Vec::new();
    let mut rest = outline;
    while !rest.is_empty() {
        let first = rest[0];
        let end = rest
            .iter()
            .skip(1)
            .position(|p| *p == first)
            .map_or(rest.len(), |k| k + 1);
        rings.push(rest[..end].to_vec());
        rest = if end < rest.len() {
            &rest[end + 1..]
        } else {
            &[]
        };
    }
    rings
}

/// A walk's signed area, µm²: the outer ring's less its holes'.
fn walk_area(outline: &[[f64; 2]]) -> f64 {
    rings_of(outline)
        .iter()
        .map(|r| {
            0.5 * (0..r.len())
                .map(|i| {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum::<f64>()
        })
        .sum()
}

/// A walk's length, µm, its rings closed.
fn walk_perimeter(outline: &[[f64; 2]]) -> f64 {
    rings_of(outline)
        .iter()
        .map(|r| {
            (0..r.len())
                .map(|i| {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    (b[0] - a[0]).hypot(b[1] - a[1])
                })
                .sum::<f64>()
        })
        .sum()
}

/// A film piece's volume, µm³: a prismatoid's h/6 (A_bottom + 4 A_middle + A_top), exact for a
/// loft whose walls are planes (its middle section is its vertices' midpoints).
fn piece_volume(f: &FrameFilm) -> f64 {
    let middle: Vec<[f64; 2]> = f
        .bottom
        .iter()
        .zip(&f.top)
        .map(|(a, b)| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0])
        .collect();
    let h = f.z_um[1] - f.z_um[0];
    h / 6.0 * (walk_area(&f.bottom) + 4.0 * walk_area(&middle) + walk_area(&f.top))
}

/// The geometry kernel's grid, µm: a boolean's or an offset's vertices are put on it, each
/// within half a step of where it would be, so a polygon's area is within its perimeter times
/// half a step.
const GRID_UM: f64 = 1e-3;

/// The bound on pieces' volume from the kernel's grid: each piece's perimeter times half a step
/// times its height, plus round-off (1e-9 relative).
fn grid_bound(films: &[&FrameFilm], volume: f64) -> f64 {
    films
        .iter()
        .map(|f| walk_perimeter(&f.top) * 0.5 * GRID_UM * (f.z_um[1] - f.z_um[0]))
        .sum::<f64>()
        + 1e-9 * volume.abs()
}

fn window_area(scene: &Event) -> f64 {
    let Event::Scene { x_um, y_um, .. } = scene else {
        panic!("a scene")
    };
    (x_um[1] - x_um[0]) * (y_um[1] - y_um[0])
}

/// A library device's view at its defaults, preferring `prefer`.
fn view_of_device(kind: &str, prefer: &[&str]) -> FabricationView {
    let job = Job::parse(&format!("name = \"f\"\n\n[device]\nkind = \"{kind}\"\n")).unwrap();
    let prefer: Vec<String> = prefer.iter().map(|s| (*s).to_owned()).collect();
    fabrication_of_job(&job, &prefer, &[]).unwrap()
}

/// The area of a library device's shapes on `layer`, at its defaults, µm².
fn shapes_area(kind: &str, layer: &str) -> f64 {
    let job = Job::parse(&format!("name = \"f\"\n\n[device]\nkind = \"{kind}\"\n")).unwrap();
    let (device, design) = super::device::spec_of(&job).unwrap().build().unwrap();
    let physical = device.physical(design.values()).unwrap();
    physical
        .structure()
        .shapes(layer)
        .iter()
        .map(|s| crate::geometry::Region::polygons(s, crate::geometry::Tolerance::default()).area())
        .sum()
}

#[test]
fn a_resist_spun_over_an_etched_strip_fills_its_trenches_and_is_flat_on_top() {
    // a phase-change shifter: its cell's lift-off resist is spun over the etched silicon strip
    let kind = "Sb2Se3 phase-change phase shifter";
    let view = view_of_device(kind, &[]);
    let coats: Vec<&FlowFrame> = view.frames.iter().filter(|f| f.phase == "coat").collect();
    assert_eq!(coats.len(), 2);
    let (flat, over) = (coats[0], coats[1]);
    // the first is spun on the flat film: one piece, from its top up
    assert_eq!(flat.films.len(), 1, "{:?}", flat.films);
    let Event::Scene { layers, .. } = &over.scene else {
        panic!("a scene")
    };
    let si = layers.iter().find(|l| l.name == "Si").unwrap();
    let (floor, top) = (si.z_um[0], si.z_um[1]);
    // the second fills the strip's trenches from the oxide up: the window less the strip there
    let low: Vec<&FrameFilm> = over.films.iter().filter(|f| f.z_um[0] == floor).collect();
    assert_eq!(low.len(), 1, "{:?}", over.films);
    assert_eq!(low[0].z_um[1], top);
    assert_eq!(rings_of(&low[0].top).len(), 2, "the strip is a hole in it");
    let t = UNSTATED_RESIST_UM;
    let got: f64 = over.films.iter().map(piece_volume).sum();
    let want =
        window_area(&over.scene) * (top + t - floor) - shapes_area(kind, "Si") * (top - floor);
    let all: Vec<&FrameFilm> = over.films.iter().collect();
    assert!(
        (got - want).abs() <= grid_bound(&all, want),
        "{got} µm³ drawn, {want} µm³ from the window and the strip"
    );
    // its top is flat: every piece over the strip's top ends at the same height
    let tops: Vec<f64> = over
        .films
        .iter()
        .filter(|f| f.z_um[1] > top)
        .map(|f| f.z_um[1])
        .collect();
    assert!(
        !tops.is_empty() && tops.iter().all(|z| (z - (top + t)).abs() < 1e-12),
        "{tops:?}"
    );
    assert!(
        over.note
            .as_deref()
            .unwrap()
            .contains("fills the etched regions")
    );
}

#[test]
fn a_resist_over_a_sloped_rib_starts_on_its_slab() {
    let view = view_of_device("TFLN Mach-Zehnder modulator", &[]);
    let coat = view
        .frames
        .iter()
        .filter(|f| f.phase == "coat")
        .nth(1)
        .unwrap();
    let Event::Scene { layers, .. } = &coat.scene else {
        panic!("a scene")
    };
    // the rib's slab top: the lowest band of LN with air around its shapes
    let slab_top = layers
        .iter()
        .filter(|l| l.name.starts_with("LN") && l.background.material == "air")
        .map(|l| l.z_um[0])
        .fold(f64::INFINITY, f64::min);
    let floor = coat
        .films
        .iter()
        .map(|f| f.z_um[0])
        .fold(f64::INFINITY, f64::min);
    assert!((floor - slab_top).abs() < 1e-12, "{floor} {slab_top}");
    assert!((slab_top - 2.3).abs() < 1e-9, "{slab_top}");
}

#[test]
fn a_film_deposited_over_the_strip_follows_it_and_holds_the_analytic_volume() {
    // the heater's oxide: 2 µm, and 0.22 µm more for the polish, by LPCVD (100 % step coverage),
    // over the 0.22 µm silicon strip; its walls vertical, so the pieces are exact prisms
    let kind = "thermo-optic phase shifter";
    let view = view_of_device(kind, &[]);
    let k = view
        .frames
        .iter()
        .position(|f| f.phase == "deposit" && f.layer.as_deref() == Some("gap"))
        .unwrap();
    let f = &view.frames[k];
    let note = f.note.as_deref().unwrap();
    assert!(note.contains("100 %"), "{note}");
    assert!(note.contains("polish after it"), "{note}");
    assert_eq!(view.frames[k + 1].phase, "planarize");
    assert!(view.frames[k + 1].films.is_empty());
    let Event::Scene {
        layers,
        x_um,
        y_um,
        shapes,
        ..
    } = &f.scene
    else {
        panic!("a scene")
    };
    let si = layers.iter().find(|l| l.name == "Si").unwrap();
    assert_eq!(
        si.background.material, "air",
        "the strip stands in air under the film"
    );
    let (zf, zt) = (si.z_um[0], si.z_um[1]);
    let (t, h) = (2.22, zt - zf);
    let n = t;
    // the strip: a w × l rectangle
    let strip: Vec<&[[f64; 2]]> = shapes
        .iter()
        .filter(|s| s.layer == "Si")
        .map(|s| s.outline.as_slice())
        .collect();
    assert_eq!(strip.len(), 1);
    let span = |i: usize| {
        let v: Vec<f64> = strip[0].iter().map(|p| p[i]).collect();
        v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - v.iter().copied().fold(f64::INFINITY, f64::min)
    };
    let (sx, sy) = (span(0), span(1));
    assert!((sx * sy - shapes_area(kind, "Si")).abs() < 1e-9);
    let a_w = window_area(&f.scene);
    // the film from first principles, t ≥ h: around the strip up to its top; t over the floor
    // everywhere above that; then over the strip, out to n from its walls, up to t over its top
    let around = (a_w - sx * sy) * h;
    let between = a_w * (t - h);
    let over_top = (sx + 2.0 * n) * (sy + 2.0 * n) * h;
    let want = around + between + over_top;
    let pieces: Vec<&FrameFilm> = f.films.iter().filter(|p| p.layer == "gap").collect();
    let got: f64 = pieces.iter().map(|p| piece_volume(p)).sum();
    assert!(
        (got - want).abs() <= grid_bound(&pieces, want),
        "{got} µm³ drawn, {want} µm³ analytic"
    );
    let top = pieces.iter().map(|p| p.z_um[1]).fold(0.0, f64::max);
    assert!((top - (zt + t)).abs() < 1e-12, "{top}");
    // the window holds the film, n out from the strip's walls, with 0.5 µm to spare
    let room = (x_um[1] - x_um[0] - sx).min(y_um[1] - y_um[0] - sy) / 2.0;
    assert!(room >= n + 0.5 - 1e-9, "{room}");
}

/// A rib's open layer: a `w` × `l` rectangle about the origin, its top at `top`, etched `h`
/// deep with walls at `deg` from the plane, with air around it.
fn rib(w: f64, l: f64, top: f64, h: f64, deg: f64) -> super::fabrication_films::Open {
    super::fabrication_films::Open {
        name: "rib".into(),
        z: [top - 0.6, top],
        floor: top - h,
        shapes: vec![rect_shape([-w / 2.0, w / 2.0], [-l / 2.0, l / 2.0])],
        ridge: true,
        sidewall: Some(deg.to_radians()),
    }
}

/// The rectangle `x` × `y` as a shape, µm.
fn rect_shape(x: [f64; 2], y: [f64; 2]) -> crate::geometry::Shape {
    crate::geometry::Shape::Rect {
        center: crate::geometry::Point::um((x[0] + x[1]) / 2.0, (y[0] + y[1]) / 2.0),
        width: crate::units::Length::um(x[1] - x[0]),
        height: crate::units::Length::um(y[1] - y[0]),
    }
}

/// ∫ (w + 2e)(l + 2e) dz over a height `h` with e linear from `e0` to `e1`: a rectangle mitred
/// out, exactly.
fn swept(w: f64, l: f64, h: f64, e0: f64, e1: f64) -> f64 {
    h * (w * l + (w + l) * (e0 + e1) + 4.0 / 3.0 * (e0 * e0 + e0 * e1 + e1 * e1))
}

#[test]
fn a_conformal_film_over_a_sloped_rib_has_its_analytic_volume_and_its_normal_thickness() {
    use super::fabrication_films::{Paint, conformal, rect};
    let paint = Paint {
        layer: "film".into(),
        film: "SiO2".into(),
        eps: 2.1,
    };
    let (w, l, zt, h, deg) = (1.0, 4.0, 0.6, 0.3, 70.0_f64);
    let theta = deg.to_radians();
    let window = rect([-5.0, 5.0], [-5.0, 5.0]);
    let a_w = 100.0;
    // (t, n): thinner than the step and thicker, its walls covered fully and thinly (a thin wall
    // film turns the outline in to the rib's own below the film's top: the kink)
    for (t, n) in [(0.2, 0.1), (0.2, 0.05), (0.45, 0.45), (0.45, 0.1)] {
        let o = rib(w, l, zt, h, deg);
        let pieces = conformal(&paint, &o, &window, t, n).unwrap();
        let zf = zt - h;
        // from first principles: t over the floor, the window; above it the wall's plane n
        // (normal) out from the rib's, a(z) = (z_t − z)/tan θ + n/sin θ out from the top's edge,
        // and no less than the rib's top
        let a = |z: f64| (zt - z) / theta.tan() + n / theta.sin();
        let (z0, z1) = (zf + t, zt + t);
        let zk = zt + n / theta.cos();
        let envelope = a_w * t
            + if zk <= z0 {
                swept(w, l, z1 - z0, 0.0, 0.0)
            } else if zk < z1 {
                swept(w, l, zk - z0, a(z0), 0.0) + swept(w, l, z1 - zk, 0.0, 0.0)
            } else {
                swept(w, l, z1 - z0, a(z0), a(z1))
            };
        // the rib, which the pieces over sloped walls are drawn over
        let rib_volume = swept(w, l, h, h / theta.tan(), 0.0);
        let film = envelope - rib_volume;
        let got: f64 = pieces.iter().map(piece_volume).sum::<f64>() - rib_volume;
        assert!(
            (got - film).abs() <= 1e-9 * film,
            "t {t}, n {n}: {got} µm³ drawn, {film} µm³ analytic"
        );
        // t on the floor, t over the top
        let box_top = pieces
            .iter()
            .filter(|p| p.z_um[0] == zf)
            .map(|p| p.z_um[1])
            .fold(f64::NAN, f64::max);
        assert!((box_top - (zf + t)).abs() < 1e-12, "{box_top}");
        let top = pieces.iter().map(|p| p.z_um[1]).fold(0.0, f64::max);
        assert!((top - (zt + t)).abs() < 1e-12, "{top}");
        // the film's wall n from the rib's, measured normal to it, where the floor's film meets it
        if z0 < zt {
            let wall = pieces.iter().find(|p| p.z_um[0] == z0).unwrap();
            let out = wall
                .bottom
                .iter()
                .map(|p| p[0])
                .fold(f64::NEG_INFINITY, f64::max);
            let rib_out = w / 2.0 + (zt - z0) / theta.tan();
            let normal = (out - rib_out) * theta.sin();
            assert!((normal - n).abs() < 1e-12, "t {t}, n {n}: {normal}");
        }
    }
}

#[test]
fn conformal_films_from_two_close_strips_merge() {
    use super::fabrication_films::{Open, Paint, conformal, rect};
    let paint = Paint {
        layer: "film".into(),
        film: "SiO2".into(),
        eps: 2.1,
    };
    // two 0.5 × 4 µm strips 0.4 µm apart, 0.22 µm tall, under a film 0.3 µm thick on the flat
    // and the walls: the walls' films meet in the gap
    let (w, gap, l, h, t) = (0.5, 0.4, 4.0, 0.22, 0.3);
    let o = Open {
        name: "Si".into(),
        z: [0.0, h],
        floor: 0.0,
        shapes: vec![
            rect_shape([-gap / 2.0 - w, -gap / 2.0], [-l / 2.0, l / 2.0]),
            rect_shape([gap / 2.0, gap / 2.0 + w], [-l / 2.0, l / 2.0]),
        ],
        ridge: true,
        sidewall: Some(std::f64::consts::FRAC_PI_2),
    };
    let window = rect([-5.0, 5.0], [-5.0, 5.0]);
    let pieces = conformal(&paint, &o, &window, t, t).unwrap();
    let a_w = 100.0;
    // t ≥ h: the window less the strips up to their top, the window up to t, then the strips
    // grown by t, one rectangle once merged
    let strips = 2.0 * w * l;
    let merged = (2.0 * w + gap + 2.0 * t) * (l + 2.0 * t);
    let want = (a_w - strips) * h + a_w * (t - h) + merged * h;
    let got: f64 = pieces.iter().map(piece_volume).sum();
    let all: Vec<&FrameFilm> = pieces.iter().collect();
    assert!((got - want).abs() <= grid_bound(&all, want), "{got} {want}");
    let top: Vec<&FrameFilm> = pieces.iter().filter(|p| p.z_um[1] == h + t).collect();
    assert_eq!(top.len(), 1, "the films merged: {top:?}");
}

#[test]
fn the_exposure_shows_what_the_resist_will_lose_by_its_polarity() {
    // HSQ, negative, kept on the modulator's rib: exposed on the shapes, which stay
    let tfln = view_of_device("TFLN Mach-Zehnder modulator", &[]);
    let expose = tfln.frames.iter().find(|f| f.phase == "expose").unwrap();
    assert!(expose.label.contains("negative"), "{}", expose.label);
    assert!(
        expose.label.contains("the beam writes the 8 shapes"),
        "{}",
        expose.label
    );
    assert!(
        expose.note.as_deref().unwrap().contains("description"),
        "{:?}",
        expose.note
    );
    let develop = tfln.frames.iter().find(|f| f.phase == "develop").unwrap();
    let exposed: f64 = expose
        .films
        .iter()
        .filter(|f| f.film.ends_with("exposed"))
        .map(piece_volume)
        .sum();
    let kept: f64 = develop.films.iter().map(piece_volume).sum();
    assert!(
        exposed > 0.0 && (exposed - kept).abs() <= 1e-9 * kept,
        "{exposed} {kept}"
    );
    // a photoresist, whose polarity the library doesn't name, kept on the strip: drawn
    // positive, exposed around the strip
    let strip = view_of_device("waveguide", &[]);
    let expose = strip.frames.iter().find(|f| f.phase == "expose").unwrap();
    assert!(expose.label.contains("positive"), "{}", expose.label);
    assert!(expose.label.contains("around"), "{}", expose.label);
    assert!(
        expose
            .note
            .as_deref()
            .unwrap()
            .contains("the frames' default")
    );
    let develop = strip.frames.iter().find(|f| f.phase == "develop").unwrap();
    let unexposed: f64 = expose
        .films
        .iter()
        .filter(|f| !f.film.ends_with("exposed"))
        .map(piece_volume)
        .sum();
    let kept: f64 = develop.films.iter().map(piece_volume).sum();
    assert!(
        (unexposed - kept).abs() <= 1e-9 * kept,
        "{unexposed} {kept}"
    );
    assert!(develop.films.iter().all(|f| !f.film.ends_with("exposed")));
}

#[test]
fn nanoimprint_presses_the_pattern_in_and_exposes_nothing() {
    let view = view_of_device("waveguide", &["nanoimprint"]);
    assert!(view.flow.steps.iter().any(|s| s.process == "nanoimprint"));
    let phases: Vec<&str> = view.frames.iter().map(|f| f.phase.as_str()).collect();
    assert!(
        phases.contains(&"imprint") && phases.contains(&"residue"),
        "{phases:?}"
    );
    assert!(!phases.contains(&"expose"), "{phases:?}");
    let imprint = view.frames.iter().find(|f| f.phase == "imprint").unwrap();
    assert!(imprint.films.iter().all(|f| !f.film.ends_with("exposed")));
    assert!(
        imprint.label.contains("no light, no beam"),
        "{}",
        imprint.label
    );
    assert!(view.flow.round_trip.unwrap().exact());
}
