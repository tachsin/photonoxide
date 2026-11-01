//! Our own dependencies in the releases issue, beside the native libraries: the Rust crates,
//! the studio's pnpm packages and the Python requirements, each at the version we use and its
//! newest release on its registry.
//!
//! Direct dependencies only, as their manifests require them; the transitive ones are Cargo's,
//! pnpm's and pip's to resolve, and run to hundreds. Dev and build dependencies are listed too:
//! the studio's build tools (vite, tailwindcss, typescript, svelte-check, oxlint) are among them.
//!
//! - **Rust crates:** every workspace member's dependencies with a version (the manifests'
//!   `[dependencies]`, `[dev-dependencies]` and `[build-dependencies]`, for every target), at
//!   Cargo.lock's version, against crates.io's sparse index (`index.crates.io`, which crates.io
//!   asks programs to read in place of its API: one small file a crate).
//! - **Studio pnpm packages:** studio/package.json's dependencies and devDependencies, at
//!   studio/pnpm-lock.yaml's version, against the npm registry's `latest` tag.
//! - **Python requirements:** python/pyproject.toml's (the build system's, the project's and its
//!   extras'), against PyPI. There is no lockfile, so the version we use is the requirement.
//!
//! The registries are read one package at a time, each URL once, with the program's
//! User-Agent ([`crate::watch::run`]). A package that couldn't be read keeps the version the
//! issue last recorded, and its group says so.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::dependencies::{self, Kind, index_newest, index_path};
use crate::watch::{Fetch, cell, compare, numbers};

/// A group of the issue's table, after the native libraries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Crates,
    Pnpm,
    Python,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Crates, Group::Pnpm, Group::Python];

    /// The group's heading in the issue.
    pub fn heading(self) -> &'static str {
        match self {
            Group::Crates => "Rust crates",
            Group::Pnpm => "Studio pnpm packages",
            Group::Python => "Python requirements",
        }
    }

    /// The registry, as the record's keys name it.
    pub fn registry(self) -> &'static str {
        match self {
            Group::Crates => "crates.io",
            Group::Pnpm => "npm",
            Group::Python => "PyPI",
        }
    }
}

/// One of our direct dependencies.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub group: Group,
    /// Its name on its registry.
    pub name: String,
    /// The version we use: Cargo.lock's or pnpm-lock.yaml's; a Python requirement's text.
    pub uses: String,
}

impl Package {
    /// The row's name in the issue's record: `serde | crates.io`.
    pub fn key(&self) -> String {
        format!("{} | {}", self.name, self.group.registry())
    }
}

/// The versions pnpm-lock.yaml's root importer (`.`) locked, by package: `svelte` and `5.57.2`
/// from `version: 5.57.2(…)`, its peers' suffix left out. pnpm writes its own install first, as
/// a document of its own, and that importer has no dependencies.
pub fn pnpm_locked(lock: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let (mut importer, mut section, mut name) = (false, false, None::<String>);
    for line in lock.lines() {
        let indent = line.len() - line.trim_start().len();
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        match indent {
            0 => {
                importer = false;
                section = false;
            }
            2 => {
                importer = text == ".:" || text == "'.':";
                section = false;
            }
            4 => {
                section = importer
                    && matches!(
                        text,
                        "dependencies:" | "devDependencies:" | "optionalDependencies:"
                    );
                name = None;
            }
            6 if section => {
                name = text
                    .strip_suffix(':')
                    .map(|n| n.trim_matches(['\'', '"']).to_owned());
            }
            8 if section => {
                if let (Some(n), Some(v)) = (&name, text.strip_prefix("version:")) {
                    let v = v.trim().trim_matches(['\'', '"']);
                    let v = v.split('(').next().unwrap_or(v);
                    out.insert(n.clone(), v.to_owned());
                }
            }
            _ => {}
        }
    }
    out
}

/// Adds `p`, or keeps the higher version of a package already listed (a crate required at two
/// major versions is one row).
fn add(list: &mut Vec<Package>, p: Package) {
    match list
        .iter_mut()
        .find(|e| e.group == p.group && e.name == p.name)
    {
        Some(e) => {
            if compare(&p.uses, &e.uses).is_gt() {
                e.uses = p.uses;
            }
        }
        None => list.push(p),
    }
}

/// Every direct dependency, from the repository at `root`, in each group by name.
pub fn packages(root: &Path) -> Result<Vec<Package>, String> {
    let read = |p: &str| std::fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"));
    let mut out = Vec::new();
    for d in dependencies::crates(root)? {
        // the workspace's own crates, required by their paths and versions, aren't another's
        if d.name.starts_with("photonoxide") {
            continue;
        }
        add(
            &mut out,
            Package {
                group: Group::Crates,
                name: d.name,
                uses: d.ours,
            },
        );
    }
    let package_json = "studio/package.json";
    let locked = pnpm_locked(&read("studio/pnpm-lock.yaml")?);
    for d in dependencies::npm(&read(package_json)?, package_json)? {
        let uses = locked.get(&d.name).cloned().unwrap_or(d.ours);
        add(
            &mut out,
            Package {
                group: Group::Pnpm,
                name: d.name,
                uses,
            },
        );
    }
    let pyproject = "python/pyproject.toml";
    for d in dependencies::python(&read(pyproject)?, pyproject)? {
        debug_assert_eq!(d.kind, Kind::Python);
        add(
            &mut out,
            Package {
                group: Group::Python,
                name: d.name,
                uses: d.ours,
            },
        );
    }
    out.sort_by(|a, b| (a.group, &a.name).cmp(&(b.group, &b.name)));
    Ok(out)
}

/// Whether a version is a pre-release: anything but numbers and dots, after a build's `+…`
/// and a Python post-release's `.postN` (`1.0.0-rc.1`, `2.0.0b1`, `3.1.dev2`).
pub fn pre_release(version: &str) -> bool {
    let v = version.trim().trim_start_matches(['v', 'V']);
    let v = v.split('+').next().unwrap_or("");
    let v = v.find(".post").map_or(v, |i| &v[..i]);
    v.is_empty() || v.chars().any(|c| !(c.is_ascii_digit() || c == '.'))
}

/// Whether a Python requirement's specifiers (`>=1.15,<2`, `==16.0.0`, `~=4.2`) take
/// `version`; an empty one takes every version.
pub fn allows(spec: &str, version: &str) -> bool {
    spec.split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .all(|clause| {
            let ops = ["===", "~=", "==", "!=", "<=", ">=", "<", ">"];
            let Some(op) = ops.iter().find(|o| clause.starts_with(**o)) else {
                return true;
            };
            let x = clause[op.len()..].trim();
            let o = compare(version, x);
            match *op {
                ">=" => o.is_ge(),
                ">" => o.is_gt(),
                "<=" => o.is_le(),
                "<" => o.is_lt(),
                "==" | "===" => match x.strip_suffix(".*") {
                    Some(prefix) => numbers(version).starts_with(&numbers(prefix)),
                    None => o.is_eq(),
                },
                "!=" => !o.is_eq(),
                // ~=4.2.1 is >=4.2.1 and ==4.2.*
                "~=" => {
                    let n = numbers(x);
                    o.is_ge() && numbers(version).starts_with(&n[..n.len().saturating_sub(1)])
                }
                _ => true,
            }
        })
}

/// A package's newest release on its registry, pre-releases left out where the registry says
/// which they are (crates.io's index; npm's `latest` tag and PyPI's version are releases).
fn newest(fetch: &mut Fetch, p: &Package) -> Result<String, String> {
    match p.group {
        Group::Crates => {
            let url = format!("https://index.crates.io/{}", index_path(&p.name));
            index_newest(&fetch.text(&url)?).ok_or_else(|| format!("{url}: no release"))
        }
        Group::Pnpm => {
            let url = format!("https://registry.npmjs.org/{}/latest", p.name);
            fetch.json(&url)?["version"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{url}: no version"))
        }
        Group::Python => {
            let url = format!("https://pypi.org/pypi/{}/json", p.name);
            fetch.json(&url)?["info"]["version"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{url}: no version"))
        }
    }
}

/// A package and its newest release.
#[derive(Clone, Debug)]
pub struct Row {
    pub package: Package,
    pub newest: Result<String, String>,
}

/// Reads every package's newest release.
pub fn rows(fetch: &mut Fetch, packages: Vec<Package>) -> Vec<Row> {
    packages
        .into_iter()
        .map(|p| Row {
            newest: newest(fetch, &p),
            package: p,
        })
        .collect()
}

/// Whether `newest` is newer than what `p` uses: for a Python requirement, a release it doesn't
/// take. A pre-release never is.
pub fn newer(p: &Package, newest: &str) -> bool {
    if pre_release(newest) {
        return false;
    }
    match p.group {
        Group::Python => !allows(&p.uses, newest),
        _ => compare(newest, &p.uses).is_gt(),
    }
}

/// Each row's newest version for the record: the one read, or the one recorded last when it
/// couldn't be read.
pub fn kept(rows: &[Row], previous: Option<&BTreeMap<String, String>>) -> Vec<(String, String)> {
    rows.iter()
        .filter_map(|r| {
            let key = r.package.key();
            let v = match &r.newest {
                Ok(n) => Some(n.clone()),
                Err(_) => previous.and_then(|p| p.get(&key).cloned()),
            };
            v.map(|v| (key, v))
        })
        .collect()
}

/// A group's table, with what couldn't be read under it.
pub fn table(
    out: &mut String,
    group: Group,
    rows: &[Row],
    previous: Option<&BTreeMap<String, String>>,
    today: &str,
) {
    let _ = writeln!(out, "### {}\n", group.heading());
    out.push_str("| Library | We use | Newest |\n|---|---|---|\n");
    let mut unread = Vec::new();
    for r in rows.iter().filter(|r| r.package.group == group) {
        let p = &r.package;
        let uses = match group {
            Group::Python if p.uses.is_empty() => "any".to_owned(),
            Group::Python => format!("`{}`", p.uses),
            _ => p.uses.clone(),
        };
        let shown = match &r.newest {
            Ok(n) => Some((n.clone(), "")),
            Err(_) => {
                unread.push(format!("`{}`", p.name));
                previous
                    .and_then(|prev| prev.get(&p.key()))
                    .map(|n| (n.clone(), " (last read)"))
            }
        };
        let newest = match shown {
            Some((n, mark)) if newer(p, &n) => format!("**{n}**{mark}"),
            Some((n, mark)) => format!("{n}{mark}"),
            None => "not read yet".into(),
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            cell(&p.name),
            cell(&uses),
            cell(&newest)
        );
    }
    if !unread.is_empty() {
        let _ = writeln!(
            out,
            "\n{} couldn't be read on {today}, so the newest shown is the last one read.",
            unread.join(", ")
        );
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

    #[test]
    fn the_repositorys_packages_are_found() {
        let all = packages(Path::new(ROOT)).unwrap();
        let find = |g: Group, n: &str| all.iter().find(|p| p.group == g && p.name == n);
        // a crate at Cargo.lock's version, once, and none of the workspace's own
        let serde = find(Group::Crates, "serde").unwrap();
        assert!(serde.uses.starts_with("1.") && !serde.uses.starts_with('^'));
        assert_eq!(all.iter().filter(|p| p.name == "serde").count(), 1);
        assert!(all.iter().all(|p| !p.name.starts_with("photonoxide")));
        assert!(find(Group::Crates, "tauri").is_some());
        // a studio package at pnpm-lock.yaml's version, a build tool among them
        let svelte = find(Group::Pnpm, "svelte").unwrap();
        assert!(numbers(&svelte.uses).len() == 3, "{svelte:?}");
        assert!(find(Group::Pnpm, "vite").is_some());
        assert!(find(Group::Pnpm, "@tauri-apps/cli").is_some());
        // a requirement as written
        assert_eq!(find(Group::Python, "maturin").unwrap().uses, ">=1.15,<2");
        assert!(find(Group::Python, "numpy").is_some());
        // grouped, and by name in each group
        let order: Vec<(Group, &str)> = all.iter().map(|p| (p.group, p.name.as_str())).collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
    }

    #[test]
    fn pnpm_lock_gives_the_root_importers_versions() {
        let lock = "---\nlockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    configDependencies: {}\n\
            \x20   packageManagerDependencies:\n      pnpm:\n        specifier: 12.4.1\n        version: 12.4.1\n\n\
            packages:\n\n  'x@1.0.0':\n    resolution: {}\n\n---\nlockfileVersion: '9.0'\n\nimporters:\n\n  .:\n\
            \x20   dependencies:\n      '@codemirror/view':\n        specifier: ^6.43.15\n        version: 6.43.15\n\
            \x20     svelte:\n        specifier: ^5.57.2\n        version: 5.57.3\n\
            \x20   devDependencies:\n      '@sveltejs/vite-plugin-svelte':\n        specifier: ^7.3.1\n\
            \x20       version: 7.3.1(svelte@5.57.3)(vite@8.3.5)\n\npackages:\n\n  svelte@5.57.3:\n    version: 9.9.9\n";
        let locked = pnpm_locked(lock);
        assert_eq!(locked.len(), 3, "{locked:?}");
        assert_eq!(locked["@codemirror/view"], "6.43.15");
        assert_eq!(locked["svelte"], "5.57.3");
        assert_eq!(locked["@sveltejs/vite-plugin-svelte"], "7.3.1");
        // the repository's own lockfile has the studio's packages
        let real = std::fs::read_to_string(Path::new(ROOT).join("studio/pnpm-lock.yaml")).unwrap();
        let real = pnpm_locked(&real);
        assert!(real.contains_key("svelte") && real.contains_key("vite"));
        assert!(!real.contains_key("pnpm"));
    }

    #[test]
    fn requirements_and_pre_releases() {
        assert!(allows(">=1.15,<2", "1.20.0"));
        assert!(!allows(">=1.15,<2", "2.0.0"));
        assert!(allows("==16.0.0", "16.0.0"));
        assert!(!allows("==16.0.0", "16.1.0"));
        assert!(allows("==5.*", "5.3.1") && !allows("==5.*", "6.0"));
        assert!(allows("~=4.2", "4.9") && !allows("~=4.2", "5.0"));
        assert!(allows("~=4.2.1", "4.2.7") && !allows("~=4.2.1", "4.3.0"));
        assert!(allows(">=2.2.6", "3.0.0") && allows("", "1.0"));
        assert!(!allows("!=1.0", "1.0") && allows(">1.0", "1.0.1"));
        for pre in ["1.0.0-rc.1", "2.0.0b1", "3.1.dev2", "7.0.0rc1", "1.0a1"] {
            assert!(pre_release(pre), "{pre}");
        }
        for release in ["1.0.0", "v2.3", "2.0.0+build.5", "1.4.post2", "2026.1.0"] {
            assert!(!pre_release(release), "{release}");
        }
    }
}
