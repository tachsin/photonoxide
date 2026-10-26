//! The window's pages, and that nothing reachable became unreachable when the rail went from
//! fourteen pages to five (#339) and the job builder went into the Custom device (#342). The rail
//! holds Home, Components, Chip, Academy and Materials, with Validation, Libraries and Benchmarks
//! folded into its System group beside Settings. The run view is reached from what it shows.
//! Every example, job, circuit and run has a home:
//!
//! - every published result of `examples/` is on Validation's Published results tab, and in
//!   Ctrl+K;
//! - every job of `jobs/` is a template on Home and in the rail's Components list, and opens in
//!   the Custom device on Components (a device's job on its device's page), as do the
//!   workspace's own jobs;
//! - every circuit of `circuits/` is in the rail's Chip list, under New chip;
//! - a run started with `photonoxide run` opens the run view, and is pinned in the runs drawer
//!   while it goes; every run is in the drawer, a device's opening on its page (a study on its
//!   Study tab), and overlays of runs (once the Compare page, then the drawer's) are a device's
//!   Study tab's Compare, or the Custom device's for runs without a device (#344).
//!
//! The window is Svelte, so these read its sources beside the program's own lists.

use std::path::Path;

/// A file of the window's, `studio/src/<path>`.
fn source(path: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src")
        .join(path);
    std::fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("{}: {e}", file.display()))
        .replace("\r\n", "\n")
}

/// The strings quoted in `text`, in order.
fn quoted(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(String::from)
        .collect()
}

/// The pages of `lib/pages.ts`'s array `name` (`export const RAIL: Page[] = [...]`).
fn array(name: &str) -> Vec<String> {
    let pages = source("lib/pages.ts");
    let head = format!("export const {name}: Page[] = [");
    let start = pages
        .find(&head)
        .unwrap_or_else(|| panic!("no {name} in lib/pages.ts"))
        + head.len();
    quoted(&pages[start..start + pages[start..].find(']').unwrap()])
}

/// Every page, as `lib/app.svelte.ts`'s type `Page` names them.
fn pages() -> Vec<String> {
    let app = source("lib/app.svelte.ts");
    let start = app.find("export type Page = ").expect("no type Page");
    quoted(&app[start..start + app[start..].find(';').unwrap()])
}

/// The files in the repository's folder `dir` ending in `.ext`, sorted.
fn files(dir: &str, ext: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(dir);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Panics, naming `file`, unless its source holds `what`.
fn holds(file: &str, what: &str, why: &str) {
    assert!(source(file).contains(what), "{file}: {why}: no `{what}`");
}

#[test]
fn the_rail_holds_five_pages_and_the_system_group() {
    assert_eq!(
        array("RAIL"),
        ["home", "components", "chip", "academy", "materials"]
    );
    assert_eq!(array("SYSTEM"), ["validation", "libraries", "benchmarks"]);
    let rail = "components/NavRail.svelte";
    holds(rail, "{#each RAIL as page (page)}", "the rail's pages");
    holds(rail, "{#each SYSTEM as page (page)}", "the System group");
    holds(rail, r#"{@render item("settings")}"#, "Settings");
}

#[test]
fn every_page_is_drawn_named_and_reachable() {
    let all = pages();
    let (rail, system) = (array("RAIL"), array("SYSTEM"));
    let names = source("lib/pages.ts");
    let app = source("App.svelte");
    for p in &all {
        assert!(
            names.contains(&format!("  {p}: {{ label: ")),
            "{p}: no entry in lib/pages.ts's PAGES"
        );
        assert!(
            app.contains(&format!("app.page === \"{p}\""))
                || app.contains(&format!("app.page !== \"{p}\"")),
            "{p}: App.svelte doesn't draw it"
        );
    }
    // the rail, the System group and Settings, and the run view, reached from what it shows; the
    // job builder is no page of its own: it is the Custom device, on Components
    let mut rest: Vec<&str> = all
        .iter()
        .filter(|p| !rail.contains(p) && !system.contains(p))
        .map(String::as_str)
        .collect();
    rest.sort_unstable();
    assert_eq!(rest, ["settings", "viewer"]);
    assert!(
        !source("App.svelte").contains("Builder"),
        "the builder page is gone"
    );
    holds(
        "components/CommandPalette.svelte",
        r#"const pages: Page[] = [...RAIL, ...SYSTEM, "settings", "viewer"];"#,
        "Ctrl+K finds every page",
    );
    // Home reaches the Custom device (New job, a template's Open) and the run view (Continue)
    holds(
        "pages/Home.svelte",
        "newCustom(kind)",
        "a new job, in the Custom device",
    );
    holds(
        "pages/Home.svelte",
        "openCustom(j.text)",
        "a template, in the Custom device",
    );
    holds("pages/Home.svelte", r#"go("viewer")"#, "the run view");
    // and the runs drawer, from the status bar
    holds(
        "components/StatusBar.svelte",
        "onclick={() => (app.drawer = !app.drawer)}",
        "the runs drawer",
    );
    holds("App.svelte", "<RunsDrawer />", "the runs drawer");
}

#[test]
fn every_example_has_a_home() {
    let catalog = serde_json::to_value(crate::studio::catalog()).unwrap();
    let mut listed: Vec<String> = catalog["examples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| format!("{}.rs", e["name"].as_str().unwrap()))
        .collect();
    listed.sort();
    assert_eq!(listed, files("examples", "rs"), "the catalog's examples");
    // Validation's Published results tab lists the catalog's, all of them until a search narrows it
    holds(
        "pages/Validation.svelte",
        "<PublishedResults />",
        "the Published results tab",
    );
    let tab = "components/PublishedResults.svelte";
    holds(
        tab,
        "const examples = $derived(catalog.data?.examples ?? []);",
        "every example",
    );
    holds(tab, "return !q || ", "only a search narrows the list");
    holds(tab, "{#each shown as e (e.name)}", "the list");
    holds(tab, "api.startExample(e.name)", "Run on this machine");
    // Ctrl+K finds each, and opens it there; so do the lessons' ::example blocks
    let palette = "components/CommandPalette.svelte";
    holds(
        palette,
        "for (const e of catalog?.examples ?? []) {",
        "every example",
    );
    holds(
        palette,
        "run: () => showPublished(e.name),",
        "opens it on the tab",
    );
    holds(
        "components/LessonExample.svelte",
        "showPublished(name)",
        "a lesson's example",
    );
    let app = source("lib/app.svelte.ts");
    let show = &app[app.find("export function showPublished").unwrap()..];
    let show = &show[..show.find("\n}\n").unwrap()];
    assert!(
        show.contains(r#"app.validationTab = "published";"#)
            && show.contains(r#"go("validation");"#),
        "showPublished must open Validation's Published results tab"
    );
}

#[test]
fn every_job_has_a_home() {
    let catalog = serde_json::to_value(crate::studio::catalog()).unwrap();
    let mut listed: Vec<String> = catalog["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["file"].as_str().unwrap().to_owned())
        .collect();
    listed.sort();
    assert_eq!(listed, files("jobs", "yaml"), "the catalog's jobs");
    // each a template on Home, with Run and Open
    let home = "pages/Home.svelte";
    holds(home, "Start from a template", "the templates");
    holds(
        home,
        "{#each catalog.data?.jobs ?? [] as j (j.file)}",
        "every job",
    );
    holds(
        home,
        "onrun={() => startRun(() => api.runText(j.text)",
        "Run",
    );
    holds(home, "onopen={async () => {", "Open");
    holds(
        "components/JobCard.svelte",
        "<Play size={14} /> Run</button>",
        "Run",
    );
    holds(
        "components/JobCard.svelte",
        "<SquarePen size={14} /> Open</button>",
        "Open",
    );
}

#[test]
fn every_job_opens_in_the_custom_device_or_on_its_devices_page() {
    // the Custom device is on Components, in the rail's list of it and in Ctrl+K
    let app = source("lib/app.svelte.ts");
    assert!(app.contains(r#"export const CUSTOM = "custom";"#));
    let open = &app[app.find("export function openCustom").unwrap()..];
    let open = &open[..open.find("\n}\n").unwrap()];
    assert!(
        open.contains("app.component = CUSTOM;") && open.contains(r#"go("components");"#),
        "openCustom must show the Custom device on Components"
    );
    let components = "components/rail/ComponentsTree.svelte";
    holds(
        components,
        "onclick={() => choose(CUSTOM)}",
        "the list's Custom device",
    );
    holds("pages/Components.svelte", "<CustomDevice />", "the page");
    // the workspace's jobs: a device's on its page, any other in the Custom device; the
    // templates in the Custom device too
    holds(
        components,
        r#"designs = h.jobs.filter((j) => j.kind === "device");"#,
        "the designs",
    );
    holds(
        components,
        r#"jobs = h.jobs.filter((j) => j.kind !== "device");"#,
        "the other jobs",
    );
    holds(components, "openJob(j.path)", "a job of the workspace");
    holds(components, "openCustom(j.text)", "a template");
    let palette = "components/CommandPalette.svelte";
    holds(palette, "run: () => openDesign(j.path)", "Ctrl+K: a design");
    holds(palette, "run: () => openJob(j.path)", "Ctrl+K: a job");
    holds(palette, "run: () => openText(j.text)", "Ctrl+K: a template");
    // a template that is a device's job (a study, say) opens on its device's page
    for file in ["pages/Home.svelte", components, palette] {
        holds(file, "openDeviceText(", "a device's template, on its page");
        holds(file, "openChipText(", "a chip's template, on the Chip page");
    }
    holds(
        palette,
        r#"label: "Custom device""#,
        "Ctrl+K: the Custom device",
    );
    holds(
        "components/LessonView.svelte",
        "openCustom(job.text)",
        "a lesson's jobs",
    );
    // and every job of jobs/ reads into its form, or is a device's (form.rs's tests check each
    // comes back unchanged)
    for file in files("jobs", "toml") {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../jobs")
            .join(&file);
        let text = std::fs::read_to_string(&path).unwrap();
        let opens = crate::form::Form::of(&text, photonoxide::run::Format::Toml).is_ok()
            || crate::devices::read_text(&text).is_ok()
            || crate::circuits::chip::chip_of_job(&text).is_ok();
        assert!(opens, "{file} opens nowhere");
    }
}

#[test]
fn every_circuit_has_a_home() {
    let circuits = serde_json::to_value(crate::circuits::circuit_examples().unwrap()).unwrap();
    let mut listed: Vec<String> = circuits
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["file"].as_str().unwrap().to_owned())
        .collect();
    listed.sort();
    assert_eq!(listed, files("circuits", "toml"), "the built-in circuits");
    // each in the rail's Chip list, under New chip, after the empty chip
    let chip = source("components/rail/ChipTree.svelte");
    let menu = &chip[chip
        .find(r#"<TreeGroup key="chip/new" title="New chip""#)
        .expect("no New chip in the rail's Chip list")..];
    let menu = &menu[..menu.find("</TreeGroup>").unwrap()];
    assert!(
        menu.contains("An empty chip") && menu.contains("{#each starts as ex (ex.file)}"),
        "the Chip list's New chip lists every built-in circuit"
    );
    holds(
        "components/rail/ChipTree.svelte",
        "(examples = e)",
        "the built-in circuits",
    );
    holds(
        "components/CommandPalette.svelte",
        "for (const c of circuitExamples) {",
        "Ctrl+K finds every built-in circuit",
    );
}

#[test]
fn a_run_opens_its_view_and_is_pinned_while_it_goes() {
    // started with a run (`photonoxide run`, `photonoxide view`), the window opens on its view
    let app = source("lib/app.svelte.ts");
    let follow = &app[app.find("export async function follow()").unwrap()..];
    let follow = &follow[..follow.find("const tick").unwrap()];
    assert!(
        follow.contains(r#"if (run.info?.dir) go("viewer");"#),
        "follow() must open the run view"
    );
    // the drawer pins the run going on top, and lists every other run newest first
    let drawer = "components/RunsDrawer.svelte";
    holds(
        drawer,
        "const live = $derived(!!run.info?.dir && !run.finished);",
        "the run going",
    );
    holds(drawer, "{#if live && run.info}", "the run pinned");
    holds(drawer, ".then((h) => (runs = h.runs))", "every run");
    for what in ["openRun(r)", "revealItemInDir(r.dir)", "remove(r)"] {
        holds(drawer, what, "open, show on disk, delete");
    }
    // the Compare page and the drawer's overlay are gone (#338's decision 4, #344): a device's
    // runs and designs overlay on its Study tab, any other run in the Custom device
    let gone = ["RunOverlay", "app.compare", "(overlay = true)"];
    for what in gone {
        assert!(!source(drawer).contains(what), "{drawer}: still has {what}");
        assert!(
            !source("lib/app.svelte.ts").contains(what),
            "the app: {what}"
        );
    }
    assert!(
        !Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../src/components/RunOverlay.svelte")
            .exists(),
        "the drawer's overlay is gone"
    );
    holds(
        "components/device/DevicePage.svelte",
        r#"["study", "Study"]"#,
        "every device has Study",
    );
    holds(
        "components/device/DevicePage.svelte",
        "<StudyTab ",
        "the Study tab",
    );
    let study = "components/device/StudyTab.svelte";
    holds(
        study,
        "runs = h.runs.filter((r) => r.device === info.kind);",
        "a device's runs",
    );
    holds(
        study,
        "designs = h.jobs.filter((j) => j.device === info.kind);",
        "a device's designs",
    );
    holds(study, "<Overlay {items}", "overlaid on its Study tab");
    // every curve overlaid names its fidelity and grid in its legend, whatever its fidelity
    holds(
        "components/Overlay.svelte",
        "label: `${s.item!.label}${c.tag ? ` · ${c.tag}` : \"\"} · ${d.fidelity}${d.grid ? ` · ${d.grid}` : \"\"}`,",
        "a curve's fidelity and grid",
    );
    let custom = "components/custom/CustomDevice.svelte";
    holds(
        custom,
        "others = h.runs.filter((r) => !r.device)",
        "every other run",
    );
    holds(
        custom,
        "<Overlay items={runItems}",
        "overlaid in the Custom device",
    );
    // a device's study opens on its Study tab, any other device run on its page
    let device = source("lib/device.svelte.ts");
    let open = &device[device.find("export async function openRunItem").unwrap()..];
    let open = &open[..open.find("\n}\n").unwrap()];
    assert!(
        open.contains(r#"openDevice(d.id, "study");"#)
            && open.contains(r#"openDevice(d.id, "simulate");"#),
        "openRunItem must open a study on its Study tab and a device's run on Simulate"
    );
    // Ctrl+K finds every run, not just the newest
    holds(
        "components/CommandPalette.svelte",
        "for (const r of home?.runs ?? []) {",
        "every run",
    );
}
