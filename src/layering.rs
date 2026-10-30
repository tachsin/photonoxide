//! The layering test (#508): the library's modules sit in layers, and production code imports
//! only downward. `docs/architecture.md` explains the layers and why; this test keeps them.
//!
//! It reads the source tree. For every file under `src/` that is not a check or a test (a file
//! named `checks.rs`, `tests.rs`, `*_checks.rs` or `*_tests.rs`, one under a `checks/` or
//! `tests/` directory, or one under `src/cases/`) it takes the `crate::<module>` paths outside
//! comments and `#[cfg(test)]` items, and asserts that each module is in the file's own layer or
//! in one it may depend on. A deliberate exception goes in [`ALLOWED`], with its reason; an
//! entry that no longer matches an import fails the test, so the list cannot go stale.

use std::fs;
use std::path::Path;

/// A layer: its name, its modules and the layers it may import from (besides its own).
struct Layer {
    name: &'static str,
    modules: &'static [&'static str],
    may_use: &'static [&'static str],
}

/// The layers, lowest first. Each becomes a crate in phase 2 of #508.
const LAYERS: &[Layer] = &[
    Layer {
        name: "core",
        modules: &[
            "units",
            "expr",
            "portable",
            "parallel",
            "traffic",
            "convergence",
            "lstsq",
            "error",
            "stop",
            "verdict",
        ],
        may_use: &[],
    },
    Layer {
        name: "geometry",
        modules: &[
            "geometry",
            "material",
            "stack",
            "averaging",
            "raster",
            "etch",
        ],
        may_use: &["core"],
    },
    Layer {
        name: "linalg",
        modules: &["dense", "sparse", "backend", "eigen"],
        may_use: &["core"],
    },
    Layer {
        name: "mode",
        modules: &["mode", "eme", "sweep"],
        may_use: &["core", "geometry", "linalg"],
    },
    Layer {
        name: "statics",
        modules: &["statics", "thermal", "electro"],
        may_use: &["core", "geometry", "linalg"],
    },
    Layer {
        name: "fdfd",
        modules: &["fdfd"],
        may_use: &["core", "geometry", "linalg"],
    },
    Layer {
        name: "fdtd",
        modules: &["fdtd"],
        may_use: &["core", "geometry", "linalg", "fdfd"],
    },
    Layer {
        name: "circuit",
        modules: &["circuit", "compact"],
        may_use: &["core", "geometry", "linalg", "mode", "statics"],
    },
    Layer {
        name: "device",
        modules: &["generator", "device", "shifter"],
        may_use: &[
            "core", "geometry", "linalg", "mode", "statics", "fdfd", "fdtd", "circuit",
        ],
    },
    Layer {
        name: "top",
        modules: &[
            "job",
            "run",
            "farm",
            "validation",
            "bench",
            "export",
            "facade",
        ],
        may_use: &[
            "core", "geometry", "linalg", "mode", "statics", "fdfd", "fdtd", "circuit", "device",
        ],
    },
];

/// Modules of the crate that belong to no layer: the test modules.
const UNLAYERED: &[&str] = &["layering"];

/// An import that points upward and is allowed for now: the file, the module it imports and
/// why. Each resolves when the crates are made (phase 2 of #508).
const ALLOWED: &[(&str, &str, &str)] = &[(
    "src/thermal.rs",
    "shifter",
    "`thermal` re-exports the thermo-optic shifter's types (`pub use crate::shifter::{..}`) so \
     that their `thermal::` paths keep working. The shifter is in the device layer, `thermal` \
     in the statics layer. In phase 2 `photonoxide::thermal` becomes a module of the top crate \
     that re-exports the statics crate's `thermal` and the shifter, and this entry goes.",
)];

fn layer_of(module: &str) -> Option<&'static Layer> {
    LAYERS.iter().find(|l| l.modules.contains(&module))
}

/// One import that breaks the layering.
#[derive(Debug, PartialEq, Eq)]
struct Violation {
    file: String,
    line: usize,
    from: String,
    to: String,
}

impl Violation {
    fn message(&self) -> String {
        let (from, to) = (
            layer_of(&self.from).map_or("?", |l| l.name),
            layer_of(&self.to).map_or("?", |l| l.name),
        );
        let may = layer_of(&self.from).map_or(String::new(), |l| l.may_use.join(", "));
        format!(
            "{}:{}: module `{}` (layer {from}) imports `crate::{}` (layer {to}), which is not \
             below it. The {from} layer may import from: {from}{}{may}. Move the code or the \
             check down or up (a check that compares solvers goes in src/cases/, see \
             docs/architecture.md), or, for a temporary exception, add it to ALLOWED with a \
             reason.",
            self.file,
            self.line,
            self.from,
            self.to,
            if may.is_empty() { "" } else { ", " }
        )
    }
}

/// Whether a path (relative to `src/`, with `/`) is a check or a test, which may look upward.
fn is_check_or_test(rel: &str) -> bool {
    if rel.starts_with("cases/") {
        return true;
    }
    let mut parts = rel.split('/').collect::<Vec<_>>();
    let file = parts.pop().unwrap_or("");
    if parts.iter().any(|d| *d == "checks" || *d == "tests") {
        return true;
    }
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    matches!(stem, "checks" | "tests") || stem.ends_with("_checks") || stem.ends_with("_tests")
}

/// The text with comments and `#[cfg(test)]` items blanked (their lines kept, so that line
/// numbers stay). The source is formatted by rustfmt, so an item ends at the first line of its
/// own indentation that closes it.
fn production_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim() == "#[cfg(test)]" {
            let indent = line.len() - line.trim_start().len();
            // the item: skip its other attributes, then to the end of the statement or block
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim_start().starts_with("#[") {
                j += 1;
            }
            let mut end = j;
            if j < lines.len() && !lines[j].trim_end().ends_with(';') {
                end = j + 1;
                while end < lines.len() {
                    let l = lines[end];
                    let ind = l.len() - l.trim_start().len();
                    let t = l.trim_start();
                    if ind == indent && matches!(t.chars().next(), Some('}' | ')' | ']')) {
                        break;
                    }
                    end += 1;
                }
            }
            for _ in i..=end.min(lines.len() - 1) {
                out.push(String::new());
            }
            i = end + 1;
            continue;
        }
        let code = match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        };
        out.push(code.to_string());
        i += 1;
    }
    out.join("\n")
}

fn ident_at(s: &str) -> &str {
    let end = s
        .char_indices()
        .find(|&(_, c)| !(c.is_alphanumeric() || c == '_'))
        .map_or(s.len(), |(i, _)| i);
    &s[..end]
}

/// The `crate::<module>` paths in `text`, each with its 1-based line: `crate::a::b`, and the
/// first segments of a group, `crate::{a::B, c}`.
fn crate_modules(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("crate::") {
        let start = from + at;
        from = start + "crate::".len();
        // a path that starts here, not the tail of an identifier (`my_crate::`)
        let before = text[..start].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let line = text[..start].matches('\n').count() + 1;
        let rest = &text[from..];
        if let Some(group) = rest.strip_prefix('{') {
            let mut depth = 1usize;
            let mut item_start = true;
            let mut k = 0;
            while k < group.len() && depth > 0 {
                let c = group[k..].chars().next().unwrap_or(' ');
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    ',' if depth == 1 => item_start = true,
                    c if c.is_whitespace() => {}
                    _ => {
                        if item_start && depth == 1 {
                            let id = ident_at(&group[k..]);
                            if !id.is_empty() {
                                let extra = group[..k].matches('\n').count();
                                found.push((line + extra, id.to_string()));
                                k += id.len();
                                item_start = false;
                                continue;
                            }
                        }
                        item_start = false;
                    }
                }
                k += c.len_utf8();
            }
        } else {
            let id = ident_at(rest);
            if !id.is_empty() {
                found.push((line, id.to_string()));
            }
        }
    }
    found
}

/// The imports of `files` (path relative to the repository, text) that break the layering, and
/// the entries of `allowed` that none of them used.
fn check(
    files: &[(String, String)],
    allowed: &[(&str, &str, &str)],
) -> (Vec<Violation>, Vec<String>) {
    let mut violations = Vec::new();
    let mut used = vec![false; allowed.len()];
    for (path, text) in files {
        let Some(rel) = path.strip_prefix("src/") else {
            continue;
        };
        if is_check_or_test(rel) {
            continue;
        }
        let first = rel.split('/').next().unwrap_or("");
        let module = first.strip_suffix(".rs").unwrap_or(first);
        let Some(layer) = layer_of(module) else {
            continue;
        };
        for (line, to) in crate_modules(&production_text(text)) {
            let Some(target) = layer_of(&to) else {
                continue; // `crate::Error`, a macro, a name that is no module
            };
            if target.name == layer.name || layer.may_use.contains(&target.name) {
                continue;
            }
            if let Some(n) = allowed.iter().position(|(f, m, _)| *f == path && *m == to) {
                used[n] = true;
                continue;
            }
            violations.push(Violation {
                file: path.clone(),
                line,
                from: module.to_string(),
                to,
            });
        }
    }
    let stale = allowed
        .iter()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|((f, m, _), _)| format!("{f} no longer imports crate::{m}"))
        .collect();
    (violations, stale)
}

fn source_files(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            out.push((rel, text));
        }
    }
}

fn repository_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    source_files(&root.join("src"), root, &mut files);
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> (String, String) {
        (path.to_string(), text.to_string())
    }

    #[test]
    fn production_code_imports_only_downward() {
        let (violations, stale) = check(&repository_sources(), ALLOWED);
        let report: Vec<String> = violations.iter().map(Violation::message).collect();
        assert!(
            report.is_empty(),
            "production code imports upward (#508, docs/architecture.md):\n{}",
            report.join("\n")
        );
        assert!(
            stale.is_empty(),
            "ALLOWED has entries that no import uses any more; remove them:\n{}",
            stale.join("\n")
        );
    }

    #[test]
    fn every_module_is_in_a_layer() {
        let lib = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .expect("src/lib.rs");
        let mut unplaced = Vec::new();
        for line in lib.lines() {
            let t = line.trim();
            let t = t.strip_prefix("pub ").unwrap_or(t);
            if let Some(name) = t.strip_prefix("mod ").and_then(|r| r.strip_suffix(';'))
                && layer_of(name).is_none()
                && !UNLAYERED.contains(&name)
            {
                unplaced.push(name.to_string());
            }
        }
        assert!(
            unplaced.is_empty(),
            "modules in no layer: {unplaced:?}. Add each to LAYERS in src/layering.rs and to \
             the table in docs/architecture.md"
        );
        let mut twice = Vec::new();
        for l in LAYERS {
            for m in l.modules {
                if LAYERS.iter().filter(|o| o.modules.contains(m)).count() != 1 {
                    twice.push(*m);
                }
            }
        }
        assert!(twice.is_empty(), "modules in two layers: {twice:?}");
    }

    #[test]
    fn layers_depend_in_one_direction() {
        // a layer may only name layers listed before it, so the table has no cycle
        for (i, l) in LAYERS.iter().enumerate() {
            for u in l.may_use {
                let at = LAYERS.iter().position(|o| o.name == *u);
                assert!(
                    at.is_some_and(|at| at < i),
                    "layer {} may use {u}, which is not below it",
                    l.name
                );
            }
        }
    }

    #[test]
    fn an_upward_import_fails_with_file_line_and_edge() {
        let files = [file(
            "src/geometry/body.rs",
            "use crate::units::Length;\n\nfn f() {\n    let _ = crate::fdtd::Simulation::new();\n}\n",
        )];
        let (violations, stale) = check(&files, &[]);
        assert!(stale.is_empty());
        assert_eq!(
            violations,
            vec![Violation {
                file: "src/geometry/body.rs".into(),
                line: 4,
                from: "geometry".into(),
                to: "fdtd".into(),
            }]
        );
        let message = violations[0].message();
        assert!(message.starts_with("src/geometry/body.rs:4:"), "{message}");
        assert!(message.contains("`geometry` (layer geometry)"), "{message}");
        assert!(message.contains("`crate::fdtd` (layer fdtd)"), "{message}");
    }

    #[test]
    fn an_upward_import_in_a_group_is_found_with_its_own_line() {
        let files = [file(
            "src/mode/slab.rs",
            "use crate::{\n    geometry::Point,\n    thermal::Rise,\n    units::Length,\n};\n",
        )];
        let (violations, _) = check(&files, &[]);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(
            (violations[0].line, violations[0].to.as_str()),
            (3, "thermal")
        );
    }

    #[test]
    fn downward_sideways_and_non_module_paths_pass() {
        let files = [
            file(
                "src/fdtd/yee.rs",
                "use crate::fdfd::Grid;\nuse crate::geometry::Point;\nuse crate::fdtd::pml;\nuse crate::{Error, Result};\n",
            ),
            file("src/job/run.rs", "use crate::fdtd::Simulation;\n"),
        ];
        let (violations, stale) = check(&files, &[]);
        assert!(violations.is_empty() && stale.is_empty(), "{violations:?}");
    }

    #[test]
    fn checks_tests_comments_and_cfg_test_items_may_look_upward() {
        let up = "use crate::fdtd::Simulation;\n";
        let files = [
            file("src/geometry/body_checks.rs", up),
            file("src/geometry/checks.rs", up),
            file("src/geometry/body_tests.rs", up),
            file("src/expr/tests.rs", up),
            file("src/fdfd/fill_checks/tests.rs", up),
            file("src/cases/geometry/body_checks.rs", up),
            file(
                "src/geometry/body.rs",
                "// crate::fdtd::Simulation is only named here\n/// [`crate::fdtd`]\n\
                 #[cfg(test)]\nmod tests {\n    use crate::fdtd::Simulation;\n\n    #[test]\n    fn t() {}\n}\n\
                 #[cfg(test)]\n#[path = \"../cases/geometry/body_tests.rs\"]\nmod body_tests;\n\
                 #[cfg(test)]\nuse crate::fdtd::Structure;\n",
            ),
        ];
        let (violations, _) = check(&files, &[]);
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn production_code_after_a_cfg_test_block_is_still_read() {
        let files = [file(
            "src/geometry/body.rs",
            "#[cfg(test)]\nmod tests {\n    fn t() {}\n}\n\nfn f() {\n    crate::fdtd::run();\n}\n",
        )];
        let (violations, _) = check(&files, &[]);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(violations[0].line, 7);
    }

    #[test]
    fn the_allow_list_lets_one_edge_through_and_reports_stale_entries() {
        let files = [file("src/thermal.rs", "pub use crate::shifter::Shifter;\n")];
        let allowed = [("src/thermal.rs", "shifter", "why")];
        let (violations, stale) = check(&files, &allowed);
        assert!(violations.is_empty() && stale.is_empty());
        // the same import in another file is not allowed
        let other = [file("src/electro.rs", "pub use crate::shifter::Shifter;\n")];
        let (violations, stale) = check(&other, &allowed);
        assert_eq!(violations.len(), 1);
        assert_eq!(stale.len(), 1);
    }
}
