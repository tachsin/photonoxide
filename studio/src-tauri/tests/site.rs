//! The project site's static copy (site/lib/projects/photonoxide/meta.js) against ROADMAP.md:
//! the milestones it falls back on are the roadmap's headings, and every highlight names a
//! milestone the roadmap has, so its status (read from the roadmap) can't go stale.

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
