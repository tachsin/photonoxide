//! The README's counts against the repository: every number in its "What it holds" section and
//! its Validation paragraph is counted here, so none can go stale.
//!
//! Each number is read from a fixed phrase of its section, the number's place marked `{}`, as
//! `"{} processes on {} platforms"`; the phrase must be in its section once (line breaks and
//! indents read as one space). How each number is counted:
//!
//! - **methods pages:** the files `docs/methods/*.md`;
//! - **topics:** the categories (`id: "…"`) of `CATEGORIES` in
//!   `site/lib/projects/photonoxide/methods.js`, which `docs/README.md` lists the pages under;
//! - **sources:** the distinct DOIs, compared without case, of the non-empty `doi:` lines in the
//!   methods pages' front matter (a book with no DOI has `doi: ""`; a DOI cited only in a page's
//!   text isn't counted);
//! - **lessons:** the files `academy/*.md` but its README, each written unless its front matter
//!   has `status: coming`;
//! - **examples:** `examples/*.rs`; **job files:** the `.yaml`, `.yml`, `.toml` and `.json` files
//!   in `jobs/`;
//! - **validation cases:** the rows of `docs/validation.md` that start with `` | ` ``, by their
//!   Tier column: `analytic` (against closed forms), `published` and `cross-code` (against
//!   another code);
//! - **processes:** `fabrication::library()`; **platforms:** the distinct platforms its processes
//!   are used on (`used_for`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use photonoxide::fabrication;

fn root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The files in `dir` whose names end in one of `extensions`, but `skip`.
fn files(dir: &str, extensions: &[&str], skip: &str) -> Vec<String> {
    std::fs::read_dir(root().join(dir))
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name != skip && extensions.iter().any(|e| name.ends_with(e)))
        .collect()
}

/// A Markdown file's front matter: the lines between its opening `---` and the next.
fn front_matter(text: &str) -> &str {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return "";
    };
    rest.find("\n---").map_or("", |end| &rest[..end])
}

/// The README's section under `## {heading}`, to the next `## `, its whitespace runs as one space.
fn readme_section(heading: &str) -> String {
    let readme = read("README.md");
    let start = format!("## {heading}");
    let lines: Vec<&str> = readme
        .lines()
        .skip_while(|line| line.trim_end() != start)
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .collect();
    assert!(!lines.is_empty(), "README.md has no section {start:?}");
    lines
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The numbers in `phrase`'s `{}` places where it starts at byte `at` of `text`, if it does.
fn phrase_at(text: &str, at: usize, parts: &[&str]) -> Option<Vec<usize>> {
    let mut pos = at;
    let mut numbers = Vec::new();
    for (k, part) in parts.iter().enumerate() {
        pos += text[pos..].starts_with(part).then_some(part.len())?;
        if k + 1 < parts.len() {
            let digits = text[pos..].bytes().take_while(u8::is_ascii_digit).count();
            numbers.push(text[pos..pos + digits].parse().ok()?);
            pos += digits;
        }
    }
    Some(numbers)
}

/// The numbers the README's section `heading` states in `phrase`, which must be there once.
fn stated(heading: &str, phrase: &str) -> Vec<usize> {
    let section = readme_section(heading);
    let parts: Vec<&str> = phrase.split("{}").collect();
    let found: Vec<Vec<usize>> = section
        .char_indices()
        .filter(|&(at, _)| !section[..at].ends_with(|c: char| c.is_ascii_digit()))
        .filter_map(|(at, _)| phrase_at(&section, at, &parts))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "README.md's \"{heading}\" has \"{phrase}\" {} times; the counts test \
         (studio/src-tauri/tests/readme.rs) reads its numbers, so it must be there once",
        found.len()
    );
    found.into_iter().next().unwrap()
}

/// `phrase` with its `{}` places filled by `numbers`.
fn fill(phrase: &str, numbers: &[usize]) -> String {
    let mut out = String::new();
    for (k, part) in phrase.split("{}").enumerate() {
        if k > 0 {
            out.push_str(&numbers[k - 1].to_string());
        }
        out.push_str(part);
    }
    out
}

/// Fails, saying what to write, when the README's section `heading` doesn't state `counted` in
/// `phrase`; `how` says how they were counted.
fn check(heading: &str, phrase: &str, counted: &[usize], how: &str) {
    let said = stated(heading, phrase);
    assert_eq!(
        said,
        counted,
        "README.md's \"{heading}\" says \"{}\", but the repository has \"{}\" ({how})",
        fill(phrase, &said),
        fill(phrase, counted)
    );
}

#[test]
fn the_methods_pages_and_their_sources_are_counted() {
    let pages = files("docs/methods", &[".md"], "");
    let methods = read("site/lib/projects/photonoxide/methods.js");
    let topics = methods
        .lines()
        .skip_while(|line| !line.starts_with("export const CATEGORIES = ["))
        .take_while(|line| !line.starts_with("];"))
        .filter(|line| line.trim_start().starts_with("id: \""))
        .count();
    check(
        "What it holds",
        "{} write-ups in {} topics",
        &[pages.len(), topics],
        "the files in docs/methods, and the categories of CATEGORIES in \
         site/lib/projects/photonoxide/methods.js",
    );

    let mut dois = BTreeSet::new();
    for page in &pages {
        let text = read(&format!("docs/methods/{page}"));
        for line in front_matter(&text).lines() {
            let line = line.trim_start();
            let line = line.strip_prefix("- ").unwrap_or(line);
            if let Some(doi) = line.strip_prefix("doi:") {
                let doi = doi.trim().trim_matches(['"', '\'']).trim();
                if !doi.is_empty() {
                    dois.insert(doi.to_lowercase());
                }
            }
        }
    }
    check(
        "What it holds",
        "{} distinct sources",
        &[dois.len()],
        "the distinct DOIs of the doi: lines in docs/methods' front matter",
    );
}

#[test]
fn the_lessons_are_counted_by_status() {
    let (mut written, mut coming) = (0, 0);
    for lesson in files("academy", &[".md"], "README.md") {
        let text = read(&format!("academy/{lesson}"));
        let status = front_matter(&text)
            .lines()
            .find_map(|line| line.strip_prefix("status:"))
            .map(|s| s.split('#').next().unwrap_or("").trim().to_string());
        match status.as_deref() {
            None => written += 1,
            Some("coming") => coming += 1,
            Some(other) => panic!(
                "academy/{lesson} has status {other:?}, which the counts test \
                 (studio/src-tauri/tests/readme.rs) doesn't know"
            ),
        }
    }
    check(
        "What it holds",
        "{} written, and {} more laid out",
        &[written, coming],
        "academy's lessons without status: coming, and with it",
    );
}

#[test]
fn the_fabrication_library_is_counted() {
    let library = fabrication::library();
    let mut platforms = Vec::new();
    for platform in library
        .iter()
        .flat_map(|process| process.used_for.iter().map(|u| u.platform))
    {
        if !platforms.contains(&platform) {
            platforms.push(platform);
        }
    }
    check(
        "What it holds",
        "{} processes on {} platforms",
        &[library.len(), platforms.len()],
        "fabrication::library(), and the platforms its processes are used on",
    );
}

#[test]
fn the_examples_and_job_files_are_counted() {
    check(
        "What it holds",
        "{} examples",
        &[files("examples", &[".rs"], "").len()],
        "examples/*.rs",
    );
    check(
        "What it holds",
        "{} job files",
        &[files("jobs", &[".yaml", ".yml", ".toml", ".json"], "").len()],
        "the job files in jobs/",
    );
}

#[test]
fn the_validation_cases_are_counted_by_tier() {
    let report = read("docs/validation.md");
    let mut tier = None;
    let (mut analytic, mut published, mut cross_code) = (0, 0, 0);
    for line in report.lines() {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if line.starts_with("| Case |") {
            tier = cells.iter().position(|c| *c == "Tier");
        } else if line.starts_with("| `") {
            let column = tier.expect("docs/validation.md has a case before its header's Tier");
            match cells[column] {
                "analytic" => analytic += 1,
                "published" => published += 1,
                "cross-code" => cross_code += 1,
                other => panic!(
                    "docs/validation.md has a case of tier {other:?}, which README.md's \
                     Validation paragraph doesn't count"
                ),
            }
        }
    }
    let all = analytic + published + cross_code;
    check(
        "What it holds",
        "{} validation cases",
        &[all],
        "the cases in docs/validation.md",
    );
    check(
        "Validation",
        "{} cases: {} against closed forms, {} against published results and {} against another code",
        &[all, analytic, published, cross_code],
        "docs/validation.md's cases, and its analytic, published and cross-code ones",
    );
}
