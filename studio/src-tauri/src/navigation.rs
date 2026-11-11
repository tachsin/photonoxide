//! The window's pages, and that everything built into the program has a place: every page is
//! drawn and reachable, and every example, job and circuit the repository holds is in the
//! program's lists, every job opening somewhere.
//!
//! These check data, the program's lists against the repository's files and the pages the window
//! declares against those it draws, not how a page is written: a redesign that keeps every page
//! and every list passes.

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

/// Every page is named, drawn, and on the rail or in its System group, but for the run view,
/// which a run opens.
#[test]
fn every_page_is_drawn_named_and_reachable() {
    let (rail, system) = (array("RAIL"), array("SYSTEM"));
    let names = source("lib/pages.ts");
    let app = source("App.svelte");
    for p in &pages() {
        assert!(
            names.contains(&format!("  {p}: {{ label: ")),
            "{p}: no entry in lib/pages.ts's PAGES"
        );
        assert!(
            app.contains(&format!("app.page === \"{p}\""))
                || app.contains(&format!("app.page !== \"{p}\"")),
            "{p}: App.svelte doesn't draw it"
        );
        assert!(
            rail.contains(p) || system.contains(p) || p == "viewer",
            "{p}: on neither the rail nor in its System group"
        );
    }
}

/// The program's lists hold every example, job and circuit of the repository, and nothing else.
#[test]
fn every_example_job_and_circuit_is_listed() {
    let catalog = serde_json::to_value(crate::studio::catalog()).unwrap();
    let listed = |key: &str, name: fn(&serde_json::Value) -> String| {
        let mut l: Vec<String> = catalog[key].as_array().unwrap().iter().map(name).collect();
        l.sort();
        l
    };
    assert_eq!(
        listed("examples", |e| format!(
            "{}.rs",
            e["name"].as_str().unwrap()
        )),
        files("examples", "rs"),
        "the catalog's examples"
    );
    assert_eq!(
        listed("jobs", |j| j["file"].as_str().unwrap().to_owned()),
        files("jobs", "yaml"),
        "the catalog's jobs"
    );
    let circuits = serde_json::to_value(crate::circuits::circuit_examples().unwrap()).unwrap();
    let mut chips: Vec<String> = circuits
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["file"].as_str().unwrap().to_owned())
        .collect();
    chips.sort();
    assert_eq!(chips, files("circuits", "toml"), "the built-in circuits");
}

/// Every job of `jobs/` opens somewhere: in the Custom device's form, on its device's page, or on
/// the Chip page.
#[test]
fn every_job_opens_somewhere() {
    let jobs = files("jobs", "yaml");
    assert!(!jobs.is_empty(), "no jobs in jobs/");
    for file in jobs {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../jobs")
            .join(&file);
        let text = std::fs::read_to_string(&path).unwrap();
        let opens = crate::form::Form::of(&text, photonoxide::run::Format::Yaml).is_ok()
            || crate::devices::read_text(&text).is_ok()
            || crate::circuits::chip::chip_of_job(&text).is_ok();
        assert!(opens, "{file} opens nowhere");
    }
}
