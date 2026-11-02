//! Every method write-up's front matter (docs/methods/*.md) against the repository, as
//! CONTRIBUTING.md describes it: the page has front matter, it has the keys a page has and no
//! other, each validation case it names is a row of docs/validation.md, each example it names is
//! a file in examples/, and each DOI is of the form Crossref writes. What the DOI names and what
//! the page quotes of it are for review; the math is rendered by `pnpm build` (vite.config.ts).

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A page's front matter: every key CONTRIBUTING.md lists, and no other (an unknown key is an
/// error, so a misspelt `validaton:` can't pass for a page with no validation).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // read by the checks below, or only by being a key a page may have
struct Front {
    title: String,
    module: String,
    summary: String,
    order: f64,
    #[serde(default)]
    papers: Vec<Paper>,
    #[serde(default)]
    validation: Vec<String>,
    #[serde(default)]
    examples: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Paper {
    cite: String,
    /// `""`, or left out, for a source that has none (a standard, an arXiv paper).
    #[serde(default)]
    doi: String,
}

/// The write-ups in docs/methods, by file name.
fn pages() -> Vec<String> {
    let mut pages: Vec<String> = std::fs::read_dir(root().join("docs/methods"))
        .expect("docs/methods")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".md"))
        .collect();
    pages.sort();
    pages
}

/// The text between a page's opening `---` line and the next `---` line.
fn front_matter(page: &str, text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let rest = text.strip_prefix("---\n").unwrap_or_else(|| {
        panic!("docs/methods/{page} has no front matter: begin it with a --- line")
    });
    let end = rest
        .find("\n---\n")
        .unwrap_or_else(|| panic!("docs/methods/{page}'s front matter has no closing --- line"));
    rest[..end].to_string()
}

fn fronts() -> Vec<(String, Front)> {
    pages()
        .into_iter()
        .map(|page| {
            let yaml = front_matter(&page, &read(&format!("docs/methods/{page}")));
            let front: Front = serde_saphyr::from_str(&yaml)
                .unwrap_or_else(|e| panic!("docs/methods/{page}'s front matter: {e}"));
            (page, front)
        })
        .collect()
}

/// A DOI as Crossref writes it: `10.`, a registrant code of 4 to 9 digits, `/`, and a suffix
/// without spaces; never the resolver's address in front of it.
fn well_formed(doi: &str) -> bool {
    let Some(rest) = doi.strip_prefix("10.") else {
        return false;
    };
    let Some((registrant, suffix)) = rest.split_once('/') else {
        return false;
    };
    (4..=9).contains(&registrant.len())
        && registrant.chars().all(|c| c.is_ascii_digit())
        && !suffix.is_empty()
        && !suffix.chars().any(|c| c.is_whitespace() || c == '"')
}

#[test]
fn every_page_has_the_front_matter_a_page_has() {
    let fronts = fronts();
    assert!(!fronts.is_empty(), "docs/methods has no pages");
    for (page, front) in &fronts {
        for (key, value) in [
            ("title", &front.title),
            ("module", &front.module),
            ("summary", &front.summary),
        ] {
            assert!(
                !value.trim().is_empty(),
                "docs/methods/{page}: {key} is empty"
            );
        }
        assert!(
            front.order.is_finite(),
            "docs/methods/{page}: order isn't a number"
        );
        for paper in &front.papers {
            assert!(
                !paper.cite.trim().is_empty(),
                "docs/methods/{page}: a paper has an empty cite"
            );
        }
    }
}

#[test]
fn every_doi_is_well_formed() {
    for (page, front) in fronts() {
        for paper in &front.papers {
            assert!(
                paper.doi.is_empty() || well_formed(&paper.doi),
                "docs/methods/{page}: {:?} (for {:?}) isn't a DOI: write it as 10.<registrant>/<suffix>, \
                 without a URL in front, or \"\" for a source that has none",
                paper.doi,
                paper.cite
            );
        }
    }
}

#[test]
fn every_validation_case_a_page_names_is_in_the_report() {
    let report = read("docs/validation.md");
    let cases: BTreeSet<&str> = report
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|rest| rest.split_once('`').map(|(id, _)| id))
        .collect();
    assert!(!cases.is_empty(), "docs/validation.md lists no cases");
    for (page, front) in fronts() {
        for case in &front.validation {
            assert!(
                cases.contains(case.as_str()),
                "docs/methods/{page} names the validation case {case:?}, which docs/validation.md \
                 doesn't list"
            );
        }
    }
}

#[test]
fn every_example_a_page_names_is_in_examples() {
    for (page, front) in fronts() {
        for example in &front.examples {
            let file = format!("examples/{example}.rs");
            assert!(
                root().join(&file).is_file(),
                "docs/methods/{page} names the example {example:?}, which {file} doesn't have"
            );
        }
    }
}

#[test]
fn a_page_names_nothing_twice() {
    for (page, front) in fronts() {
        for (what, names) in [
            ("validation case", &front.validation),
            ("example", &front.examples),
        ] {
            let mut seen = BTreeSet::new();
            for name in names {
                assert!(
                    seen.insert(name),
                    "docs/methods/{page} names the {what} {name:?} twice"
                );
            }
        }
    }
}
