//! The project site's static copy against the repository: the milestones meta.js falls back on
//! are ROADMAP.md's headings and every highlight names a milestone the roadmap has, so its status
//! (read from the roadmap) can't go stale; and every method write-up (docs/methods) is in one of
//! methods.js's categories, and its algorithm index points only at write-ups that exist.

use std::path::Path;

fn read(path: &str) -> String {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// ROADMAP.md's milestone headings, `<version>: <title>` with its ✅, in order.
fn milestones() -> Vec<String> {
    read("ROADMAP.md")
        .lines()
        .filter_map(|line| line.strip_prefix("### "))
        .filter(|heading| {
            heading.split_once(": ").is_some_and(|(version, _)| {
                version.starts_with(|c: char| c.is_ascii_digit())
                    && version
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == 'x')
            })
        })
        .map(|heading| heading.trim_end().to_string())
        .collect()
}

#[test]
fn the_fallback_milestones_are_the_roadmaps() {
    let meta = read("site/lib/projects/photonoxide/meta.js");
    let fallback: Vec<String> = meta
        .lines()
        .skip_while(|line| !line.starts_with("export const FALLBACK_MILESTONES = ["))
        .skip(1)
        .take_while(|line| !line.starts_with("];"))
        .map(|line| {
            line.trim()
                .trim_end_matches(',')
                .trim_matches('"')
                .to_string()
        })
        .collect();
    assert_eq!(
        fallback,
        milestones(),
        "FALLBACK_MILESTONES in site/lib/projects/photonoxide/meta.js isn't ROADMAP.md's milestones"
    );
}

#[test]
fn every_highlight_names_a_milestone_the_roadmap_has() {
    let meta = read("site/lib/projects/photonoxide/meta.js");
    let versions: Vec<String> = milestones()
        .iter()
        .map(|m| m.split_once(": ").unwrap().0.to_string())
        .collect();
    let named: Vec<&str> = meta
        .lines()
        .filter_map(|line| line.trim().strip_prefix("milestone: \""))
        .map(|rest| rest.trim_end_matches(['"', ',']))
        .collect();
    assert!(!named.is_empty(), "no highlight names a milestone");
    for m in named {
        assert!(
            versions.iter().any(|v| v == m),
            "a highlight's milestone {m:?} isn't in ROADMAP.md"
        );
    }
}

/// The lines of `site/lib/projects/photonoxide/methods.js` from `export const <name> = [` to the
/// `];` that closes it.
fn methods_js_array(name: &str) -> String {
    let methods = read("site/lib/projects/photonoxide/methods.js");
    let start = format!("export const {name} = [");
    let lines: Vec<&str> = methods
        .lines()
        .skip_while(|line| !line.starts_with(&start))
        .take_while(|line| !line.starts_with("];"))
        .collect();
    assert!(!lines.is_empty(), "methods.js has no {name}");
    lines.join("\n")
}

/// Every `key: [...]` list of quoted strings in `text`, each list's strings in order.
fn quoted_lists(text: &str, key: &str) -> Vec<Vec<String>> {
    let open = format!("{key}: [");
    text.match_indices(&open)
        .map(|(at, _)| {
            let rest = &text[at + open.len()..];
            let list = &rest[..rest.find(']').expect("an unclosed list")];
            list.split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .collect()
}

/// The write-ups in docs/methods, by slug (the file's name without `.md`).
fn method_slugs() -> Vec<String> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/methods"));
    let mut slugs: Vec<String> = std::fs::read_dir(root)
        .expect("docs/methods")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".md").map(str::to_string)
        })
        .collect();
    slugs.sort();
    slugs
}

#[test]
fn every_method_is_in_one_category_of_the_site() {
    let named: Vec<String> = quoted_lists(&methods_js_array("CATEGORIES"), "slugs")
        .into_iter()
        .flatten()
        .collect();
    let slugs = method_slugs();
    for slug in &slugs {
        let times = named.iter().filter(|n| *n == slug).count();
        assert_eq!(
            times, 1,
            "docs/methods/{slug}.md is named {times} times in CATEGORIES in \
             site/lib/projects/photonoxide/methods.js: give it one category and group"
        );
    }
    for name in &named {
        assert!(
            slugs.contains(name),
            "CATEGORIES in site/lib/projects/photonoxide/methods.js names {name:?}, which docs/methods doesn't have"
        );
    }
}

#[test]
fn every_algorithm_on_the_site_points_at_a_write_up() {
    let algorithms = methods_js_array("ALGORITHMS");
    let lists = quoted_lists(&algorithms, "methods");
    assert_eq!(
        lists.len(),
        algorithms.matches("name: \"").count(),
        "an algorithm in ALGORITHMS has no methods list"
    );
    let slugs = method_slugs();
    for list in lists {
        assert!(
            !list.is_empty(),
            "an algorithm in ALGORITHMS names no write-up"
        );
        for slug in list {
            assert!(
                slugs.contains(&slug),
                "ALGORITHMS in site/lib/projects/photonoxide/methods.js points at {slug:?}, which docs/methods doesn't have"
            );
        }
    }
}
