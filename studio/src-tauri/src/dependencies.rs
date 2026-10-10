//! The major versions of our own dependencies that our requirements don't take:
//! `photonoxide dependencies`, run from the repository's root.
//!
//! Dependabot's weekly pull request brings the minor and patch updates (`.github/dependabot.yml`)
//! and leaves the major ones out, since they are what breaks a build. The Releases workflow runs
//! this every Monday and keeps its table in one issue, "Major updates available", so that a
//! major version is adopted on purpose, in a pull request of its own. "Major" is Dependabot's:
//! the first number of the version, so a 0.x crate's breaking 0.y comes in its pull request.
//!
//! - **Rust crates:** every workspace member's dependencies with a version, at the version
//!   Cargo.lock has, against the sparse index of crates.io (crates.io's API is asked only for a
//!   major version's date and repository).
//! - **npm:** the studio's package.json, against the npm registry.
//! - **Python:** python/pyproject.toml's requirements, against PyPI. A requirement without an
//!   upper bound takes every major version, and none is listed.
//! - **GitHub Actions:** every `uses: owner/name@v…` in the workflows, against the action's
//!   latest release on GitHub.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde_json::Value;

use crate::watch::{Fetch, Report, cell, compare, day, numbers, record};

/// Where a dependency comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Crate,
    Npm,
    Python,
    Action,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Crate => "crate",
            Kind::Npm => "npm",
            Kind::Python => "Python",
            Kind::Action => "Action",
        }
    }
}

/// One of our dependencies.
#[derive(Clone, Debug, PartialEq)]
pub struct Dependency {
    pub kind: Kind,
    /// Its name: a crate's, an npm or PyPI package's, an action's `owner/name`.
    pub name: String,
    /// Our version: Cargo.lock's, package.json's, the requirement, the workflow's tag.
    pub ours: String,
    /// The newest major version our requirement takes; none if it takes every one.
    pub takes: Option<u64>,
    /// The files that require it.
    pub files: Vec<String>,
}

impl Dependency {
    pub fn key(&self) -> String {
        format!("{} {}", self.kind.name(), self.name)
    }
}

/// Adds `d`, or merges it into the same dependency at the same major version.
fn add(list: &mut Vec<Dependency>, d: Dependency) {
    if let Some(e) = list
        .iter_mut()
        .find(|e| e.kind == d.kind && e.name == d.name && e.takes == d.takes)
    {
        for f in d.files {
            if !e.files.contains(&f) {
                e.files.push(f);
            }
        }
        if compare(&d.ours, &e.ours).is_gt() {
            e.ours = d.ours;
        }
    } else {
        list.push(d);
    }
}

/// The first number of a version or requirement.
fn major(version: &str) -> Option<u64> {
    let v = version.trim_start_matches(['^', '~', '=', '>', '<', ' ', 'v']);
    numbers(v).first().copied()
}

/// The crates a manifest requires with a version: their names on crates.io and requirements.
fn manifest_crates(manifest: &toml::Table) -> Vec<(String, String)> {
    let mut tables: Vec<&toml::Table> = Vec::new();
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        tables.extend(manifest.get(key).and_then(|t| t.as_table()));
    }
    if let Some(targets) = manifest.get("target").and_then(|t| t.as_table()) {
        for target in targets.values().filter_map(|t| t.as_table()) {
            for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                tables.extend(target.get(key).and_then(|t| t.as_table()));
            }
        }
    }
    if let Some(ws) = manifest.get("workspace").and_then(|w| w.as_table()) {
        tables.extend(ws.get("dependencies").and_then(|t| t.as_table()));
    }
    let mut out = Vec::new();
    for table in tables {
        for (name, spec) in table {
            let (req, package) = match spec {
                toml::Value::String(req) => (Some(req.as_str()), None),
                toml::Value::Table(t) => (
                    t.get("version").and_then(|v| v.as_str()),
                    t.get("package").and_then(|v| v.as_str()),
                ),
                _ => (None, None),
            };
            // a path or git dependency without a version isn't published
            if let Some(req) = req {
                out.push((package.unwrap_or(name).to_owned(), req.to_owned()));
            }
        }
    }
    out
}

/// The workspace's crates, at Cargo.lock's versions.
pub fn crates(root: &Path) -> Result<Vec<Dependency>, String> {
    let read = |p: &str| -> Result<toml::Table, String> {
        let text = std::fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"))?;
        toml::from_str(&text).map_err(|e| format!("{p}: {e}"))
    };
    let top = read("Cargo.toml")?;
    let mut manifests = vec![("Cargo.toml".to_owned(), top.clone())];
    let members = top
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    for m in members.iter().filter_map(|m| m.as_str()) {
        let p = format!("{m}/Cargo.toml");
        manifests.push((p.clone(), read(&p)?));
    }
    let lock = read("Cargo.lock")?;
    let locked: Vec<(String, semver::Version)> = lock
        .get("package")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let name = p.get("name")?.as_str()?;
            let version = semver::Version::parse(p.get("version")?.as_str()?).ok()?;
            Some((name.to_owned(), version))
        })
        .collect();
    let mut out = Vec::new();
    for (file, manifest) in &manifests {
        for (name, req) in manifest_crates(manifest) {
            let parsed = semver::VersionReq::parse(&req).ok();
            let ours = locked
                .iter()
                .filter(|(n, v)| *n == name && parsed.as_ref().is_none_or(|r| r.matches(v)))
                .map(|(_, v)| v)
                .max()
                .map_or_else(|| req.clone(), ToString::to_string);
            add(
                &mut out,
                Dependency {
                    kind: Kind::Crate,
                    takes: major(&req),
                    name,
                    ours,
                    files: vec![file.clone()],
                },
            );
        }
    }
    Ok(out)
}

/// package.json's dependencies and devDependencies.
pub fn npm(package_json: &str, file: &str) -> Result<Vec<Dependency>, String> {
    let json: Value = serde_json::from_str(package_json).map_err(|e| format!("{file}: {e}"))?;
    let mut out = Vec::new();
    for key in ["dependencies", "devDependencies"] {
        for (name, spec) in json[key].as_object().into_iter().flatten() {
            let Some(spec) = spec.as_str() else { continue };
            // a workspace's, a file's or a URL's package isn't the registry's
            let ours = spec.trim_start_matches(['^', '~', '=', ' ', 'v']);
            if spec.contains(':') || spec.contains('/') || major(ours).is_none() {
                continue;
            }
            add(
                &mut out,
                Dependency {
                    kind: Kind::Npm,
                    name: name.clone(),
                    ours: ours.to_owned(),
                    takes: major(ours),
                    files: vec![file.to_owned()],
                },
            );
        }
    }
    Ok(out)
}

/// A PEP 508 requirement's name and the newest major version its upper bound takes:
/// `maturin>=1.15,<2` is maturin, 1; `numpy>=1.21` numpy, every one.
fn requirement(text: &str) -> Option<(String, String, Option<u64>)> {
    let text = text.split(';').next()?.trim();
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || "-_.".contains(c)))
        .unwrap_or(text.len());
    let name = text[..end].to_owned();
    if name.is_empty() {
        return None;
    }
    let rest = text[end..].trim();
    // extras, as in name[extra]>=1
    let rest = match rest.strip_prefix('[') {
        Some(r) => r.split_once(']').map_or("", |(_, r)| r).trim(),
        None => rest,
    };
    let mut takes = None;
    for clause in rest.split(',').map(str::trim) {
        let bound = if let Some(v) = clause.strip_prefix("<=") {
            numbers(v).first().copied()
        } else if let Some(v) = clause.strip_prefix('<') {
            // <2 and <2.0 stop before 2; <2.1 takes 2
            let n = numbers(v);
            match n.iter().skip(1).any(|x| *x > 0) {
                true => n.first().copied(),
                false => n.first().and_then(|m| m.checked_sub(1)),
            }
        } else if let Some(v) = clause
            .strip_prefix("==")
            .or_else(|| clause.strip_prefix("~="))
        {
            numbers(v).first().copied()
        } else {
            None
        };
        if let Some(b) = bound {
            takes = Some(takes.map_or(b, |t: u64| t.min(b)));
        }
    }
    Some((name, rest.to_owned(), takes))
}

/// pyproject.toml's requirements: the build system's, the project's and its extras'.
pub fn python(pyproject: &str, file: &str) -> Result<Vec<Dependency>, String> {
    let t: toml::Table = toml::from_str(pyproject).map_err(|e| format!("{file}: {e}"))?;
    let mut texts: Vec<&str> = Vec::new();
    fn strings(v: Option<&toml::Value>) -> Vec<&str> {
        v.and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str())
            .collect()
    }
    texts.extend(strings(
        t.get("build-system").and_then(|b| b.get("requires")),
    ));
    let project = t.get("project");
    texts.extend(strings(project.and_then(|p| p.get("dependencies"))));
    if let Some(extras) = project
        .and_then(|p| p.get("optional-dependencies"))
        .and_then(|o| o.as_table())
    {
        for v in extras.values() {
            texts.extend(strings(Some(v)));
        }
    }
    let mut out = Vec::new();
    for text in texts {
        if let Some((name, spec, takes)) = requirement(text) {
            add(
                &mut out,
                Dependency {
                    kind: Kind::Python,
                    name,
                    ours: spec,
                    takes,
                    files: vec![file.to_owned()],
                },
            );
        }
    }
    Ok(out)
}

/// A workflow's actions with a version tag: `uses: actions/checkout@v7`, as `actions/checkout`
/// and `v7`. A local action, a branch (`@stable`) or a commit isn't one.
pub fn actions(workflow: &str, file: &str) -> Vec<Dependency> {
    let mut out = Vec::new();
    for line in workflow.lines() {
        let line = line.trim().trim_start_matches("- ").trim();
        let Some(uses) = line.strip_prefix("uses:") else {
            continue;
        };
        let uses = uses
            .split('#')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(['"', '\'']);
        let Some((path, tag)) = uses.split_once('@') else {
            continue;
        };
        let mut parts = path.split('/');
        let (Some(owner), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        let versioned = tag
            .strip_prefix('v')
            .is_some_and(|t| t.starts_with(|c: char| c.is_ascii_digit()));
        if owner.is_empty() || owner == "." || !versioned {
            continue;
        }
        add(
            &mut out,
            Dependency {
                kind: Kind::Action,
                name: format!("{owner}/{name}"),
                ours: tag.to_owned(),
                takes: major(tag),
                files: vec![file.to_owned()],
            },
        );
    }
    out
}

/// Every dependency, from the repository at `root`.
pub fn dependencies(root: &Path) -> Result<Vec<Dependency>, String> {
    let mut all = crates(root)?;
    let package = "studio/package.json";
    let text =
        std::fs::read_to_string(root.join(package)).map_err(|e| format!("{package}: {e}"))?;
    all.extend(npm(&text, package)?);
    let pyproject = "python/pyproject.toml";
    let text =
        std::fs::read_to_string(root.join(pyproject)).map_err(|e| format!("{pyproject}: {e}"))?;
    all.extend(python(&text, pyproject)?);
    let dir = root.join(".github/workflows");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    files.sort();
    let mut acts = Vec::new();
    for p in files {
        let file = format!(
            ".github/workflows/{}",
            p.file_name().unwrap_or_default().to_string_lossy()
        );
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{file}: {e}"))?;
        for a in actions(&text, &file) {
            add(&mut acts, a);
        }
    }
    all.extend(acts);
    Ok(all)
}

/// A dependency's newest release.
#[derive(Clone, Debug, PartialEq)]
pub struct Newest {
    pub version: String,
    /// As `2026-07-16`, when it was read.
    pub date: Option<String>,
    /// Its release notes, changelog or repository.
    pub notes: String,
}

/// A repository's URL in the browser's form: `git+https://github.com/x/y.git` is
/// `https://github.com/x/y`.
fn repository(url: &str) -> String {
    let u = url.trim().trim_start_matches("git+");
    let u = u
        .strip_prefix("git://")
        .map_or_else(|| u.to_owned(), |r| format!("https://{r}"));
    let u = u
        .strip_prefix("ssh://git@")
        .map_or(u.clone(), |r| format!("https://{r}"));
    u.trim_end_matches('/').trim_end_matches(".git").to_owned()
}

/// Where to read a release's notes: a GitHub repository's releases, or the repository.
fn notes_of(repo: Option<&str>, fallback: String) -> String {
    match repo.map(repository) {
        Some(r) if r.starts_with("https://github.com/") => format!("{r}/releases"),
        Some(r) if r.starts_with("https://") => r,
        _ => fallback,
    }
}

/// A crate's path in the sparse index.
fn index_path(name: &str) -> String {
    let n = name.to_lowercase();
    match n.len() {
        1 => format!("1/{n}"),
        2 => format!("2/{n}"),
        3 => format!("3/{}/{n}", &n[..1]),
        _ => format!("{}/{}/{n}", &n[..2], &n[2..4]),
    }
}

/// The newest stable version in a crate's sparse index file, yanked ones left out.
fn index_newest(lines: &str) -> Option<String> {
    lines
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| !v["yanked"].as_bool().unwrap_or(false))
        .filter_map(|v| semver::Version::parse(v["vers"].as_str()?).ok())
        .filter(|v| v.pre.is_empty())
        .max()
        .map(|v| v.to_string())
}

/// The newest release of `d`; `beyond` tells whether it is a major version `d` doesn't take,
/// for which the date and notes are read too.
fn newest(fetch: &mut Fetch, d: &Dependency) -> Result<Newest, String> {
    let beyond = |v: &str| d.takes.is_some_and(|t| major(v).is_some_and(|m| m > t));
    match d.kind {
        Kind::Crate => {
            let url = format!("https://index.crates.io/{}", index_path(&d.name));
            let version =
                index_newest(&fetch.text(&url)?).ok_or_else(|| format!("{url}: no release"))?;
            let page = format!("https://crates.io/crates/{}/versions", d.name);
            if !beyond(&version) {
                return Ok(Newest {
                    version,
                    date: None,
                    notes: page,
                });
            }
            let info = fetch.json(&format!("https://crates.io/api/v1/crates/{}", d.name))?;
            let date = info["versions"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|v| v["num"] == version.as_str())
                .and_then(|v| v["created_at"].as_str().and_then(day));
            Ok(Newest {
                notes: notes_of(info["crate"]["repository"].as_str(), page),
                version,
                date,
            })
        }
        Kind::Npm => {
            let latest = fetch.json(&format!("https://registry.npmjs.org/{}/latest", d.name))?;
            let version = latest["version"]
                .as_str()
                .ok_or("the registry's answer has no version")?
                .to_owned();
            let repo = match &latest["repository"] {
                Value::String(s) => Some(s.as_str()),
                r => r["url"].as_str(),
            };
            let notes = notes_of(
                repo,
                format!(
                    "https://www.npmjs.com/package/{}?activeTab=versions",
                    d.name
                ),
            );
            // the whole document, for the date, only for a major version: some are megabytes
            let date = if beyond(&version) {
                let all = fetch.json(&format!("https://registry.npmjs.org/{}", d.name))?;
                all["time"][version.as_str()].as_str().and_then(day)
            } else {
                None
            };
            Ok(Newest {
                version,
                date,
                notes,
            })
        }
        Kind::Python => {
            let project = fetch.json(&format!("https://pypi.org/pypi/{}/json", d.name))?;
            let version = project["info"]["version"]
                .as_str()
                .ok_or("PyPI's answer has no version")?
                .to_owned();
            let date = project["releases"][version.as_str()]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f["upload_time_iso_8601"].as_str().and_then(day))
                .min();
            let urls = project["info"]["project_urls"].as_object();
            let find = |keys: &[&str]| {
                urls.into_iter().flatten().find_map(|(k, v)| {
                    let k = k.to_lowercase();
                    keys.iter()
                        .any(|w| k.contains(w))
                        .then(|| v.as_str().map(str::to_owned))
                        .flatten()
                })
            };
            let notes = find(&["changelog", "release notes", "changes", "history"])
                .or_else(|| {
                    find(&["source", "repository", "github"]).map(|r| notes_of(Some(&r), r.clone()))
                })
                .unwrap_or_else(|| format!("https://pypi.org/project/{}/#history", d.name));
            Ok(Newest {
                version,
                date,
                notes,
            })
        }
        Kind::Action => {
            let api = format!("https://api.github.com/repos/{}", d.name);
            match fetch.json(&format!("{api}/releases/latest")) {
                Ok(r) => Ok(Newest {
                    version: r["tag_name"]
                        .as_str()
                        .ok_or("GitHub's answer has no tag")?
                        .to_owned(),
                    date: r["published_at"].as_str().and_then(day),
                    notes: r["html_url"].as_str().map_or_else(
                        || format!("https://github.com/{}/releases", d.name),
                        str::to_owned,
                    ),
                }),
                // an action without releases has its tags
                Err(_) => {
                    let tags = fetch.json(&format!("{api}/tags?per_page=100"))?;
                    let version = tags
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|t| t["name"].as_str())
                        .filter(|t| t.starts_with('v') && major(t).is_some())
                        .max_by(|a, b| compare(a, b))
                        .ok_or_else(|| format!("{}: no release and no version tag", d.name))?
                        .to_owned();
                    Ok(Newest {
                        version,
                        date: None,
                        notes: format!("https://github.com/{}/tags", d.name),
                    })
                }
            }
        }
    }
}

/// A dependency and its newest release.
#[derive(Clone, Debug)]
pub struct Row {
    pub dependency: Dependency,
    pub newest: Result<Newest, String>,
}

impl Row {
    /// Whether its newest release is a major version our requirement doesn't take.
    pub fn major(&self) -> bool {
        match (&self.newest, self.dependency.takes) {
            (Ok(n), Some(t)) => major(&n.version).is_some_and(|m| m > t),
            _ => false,
        }
    }
}

/// Reads every dependency's newest release.
pub fn rows(fetch: &mut Fetch, dependencies: Vec<Dependency>) -> Vec<Row> {
    dependencies
        .into_iter()
        .map(|d| Row {
            newest: newest(fetch, &d),
            dependency: d,
        })
        .collect()
}

/// The rows whose major version is new since the earlier record: listed now, at another major
/// version than recorded. Nothing is new on the issue's first run.
pub fn news<'a>(rows: &'a [Row], previous: Option<&BTreeMap<String, String>>) -> Vec<&'a Row> {
    rows.iter()
        .filter(|r| r.major())
        .filter(|r| {
            let (Some(p), Ok(n)) = (previous, &r.newest) else {
                return false;
            };
            p.get(&r.dependency.key())
                .is_some_and(|old| major(old) != major(&n.version))
        })
        .collect()
}

/// The issue's body.
pub fn body(rows: &[Row], previous: Option<&BTreeMap<String, String>>, today: &str) -> String {
    // a dependency that couldn't be read keeps what it had, so its return isn't news
    // one line a dependency, however many major versions of it we require
    let kept: BTreeMap<String, String> = rows
        .iter()
        .filter_map(|r| {
            let key = r.dependency.key();
            let v = match &r.newest {
                Ok(n) => Some(n.version.clone()),
                Err(_) => previous.and_then(|p| p.get(&key).cloned()),
            };
            v.map(|v| (key, v))
        })
        .collect();
    let mut out = record(kept.iter().map(|(k, v)| (k.clone(), v.as_str())));
    let count = |k: Kind| {
        rows.iter()
            .filter(|r| r.dependency.kind == k)
            .map(|r| &r.dependency.name)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    };
    let _ = writeln!(
        out,
        "The major versions of our own dependencies that our requirements don't take, read on \
         {today} from {} crates (Cargo.lock's versions, against crates.io), {} npm packages \
         (studio/package.json, against the npm registry), {} Python requirements \
         (python/pyproject.toml, against PyPI) and {} GitHub Actions (the workflows' tags, \
         against their releases). Dependabot's weekly pull request brings the minor and patch \
         updates and leaves these out. The Releases workflow (`.github/workflows/releases.yml`) \
         rewrites this every Monday with `photonoxide dependencies`, and comments here when a \
         major version appears; it closes nothing.\n",
        count(Kind::Crate),
        count(Kind::Npm),
        count(Kind::Python),
        count(Kind::Action)
    );
    let mut majors: Vec<&Row> = rows.iter().filter(|r| r.major()).collect();
    majors.sort_by_key(|r| (r.dependency.kind, r.dependency.name.clone()));
    if majors.is_empty() {
        out.push_str("No major version is available that our requirements don't take.\n\n");
    } else {
        out.push_str(
            "| Package | Kind | Ours | Newest | Released | Notes | Required in |\n\
             |---|---|---|---|---|---|---|\n",
        );
        for r in &majors {
            let d = &r.dependency;
            let Ok(n) = &r.newest else { continue };
            let _ = writeln!(
                out,
                "| {} | {} | {} | **{}** | {} | [{}]({}) | {} |",
                cell(&d.name),
                d.kind.name(),
                cell(&d.ours),
                cell(&n.version),
                n.date.as_deref().unwrap_or("—"),
                if n.notes.contains("/releases") {
                    "release notes"
                } else {
                    "changes"
                },
                n.notes,
                cell(&d.files.join(", "))
            );
        }
        out.push('\n');
    }
    let failed: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            r.newest
                .as_ref()
                .err()
                .map(|e| format!("{}: {e}", r.dependency.key()))
        })
        .collect();
    if !failed.is_empty() {
        out.push_str("**Couldn't be read:**\n\n");
        for f in failed {
            let _ = writeln!(out, "- {}", cell(&f));
        }
        out.push('\n');
    }
    out.push_str(
        "## Adopting a major version\n\n\
         In a pull request of its own: read its release notes, raise the requirement \
         (Cargo.toml, studio/package.json, python/pyproject.toml, or the workflows' `uses:`), \
         and fix what it breaks. When a crate's new release needs a newer Rust than \
         `rust-version`, raise `rust-version` and the CI's MSRV job in the same pull request \
         (AGENTS.md).\n",
    );
    out
}

/// The comment on the major versions that appeared.
pub fn comment(new: &[&Row]) -> String {
    let mut out = String::from("New major versions since the last check:\n\n");
    for r in new {
        let d = &r.dependency;
        if let Ok(n) = &r.newest {
            let _ = writeln!(
                out,
                "- {} {} {}{} (ours: {}): {}",
                d.kind.name(),
                d.name,
                n.version,
                n.date
                    .as_ref()
                    .map(|x| format!(", {x}"))
                    .unwrap_or_default(),
                d.ours,
                n.notes
            );
        }
    }
    out
}

/// `photonoxide dependencies [--previous <file>] [--write <file>] [--news <file>]`, from the
/// repository's root.
pub fn run(args: &[String]) -> std::process::ExitCode {
    let deps = match dependencies(Path::new(".")) {
        Ok(d) => d,
        Err(e) => {
            return crate::fail(format!(
                "{e} (photonoxide dependencies runs from the repository's root)"
            ));
        }
    };
    crate::watch::run(args, |fetch, previous, today| {
        let rows = rows(fetch, deps);
        let new = news(&rows, previous);
        Report {
            body: body(&rows, previous, today),
            news: (!new.is_empty()).then(|| comment(&new)),
            failures: rows
                .iter()
                .filter_map(|r| {
                    r.newest
                        .as_ref()
                        .err()
                        .map(|e| format!("{}: {e}", r.dependency.key()))
                })
                .collect(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::github::{Http, Response};
    use crate::watch::recorded;

    const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

    #[test]
    fn the_repositorys_dependencies_are_found() {
        let all = dependencies(Path::new(ROOT)).unwrap();
        let find = |k: Kind, n: &str| all.iter().find(|d| d.kind == k && d.name == n);
        // a crate at Cargo.lock's version, required in several manifests, and a path
        // dependency left out
        let serde = find(Kind::Crate, "serde").unwrap();
        assert_eq!(serde.takes, Some(1));
        assert!(
            serde.ours.starts_with("1.") && serde.files.len() >= 2,
            "{serde:?}"
        );
        assert!(find(Kind::Crate, "photonoxide").is_none());
        // a target's dependency, a dev-dependency, a build-dependency
        assert!(find(Kind::Crate, "private-gemm-x86").is_some());
        assert!(find(Kind::Crate, "genoxide").is_some());
        assert!(find(Kind::Crate, "tauri-build").is_some());
        let svelte = find(Kind::Npm, "svelte").unwrap();
        assert_eq!(svelte.takes, Some(5));
        assert!(find(Kind::Npm, "@tauri-apps/cli").is_some());
        let maturin = find(Kind::Python, "maturin").unwrap();
        assert_eq!(maturin.takes, Some(1));
        assert_eq!(find(Kind::Python, "numpy").unwrap().takes, None);
        let checkout = find(Kind::Action, "actions/checkout").unwrap();
        assert!(checkout.ours.starts_with('v') && checkout.files.len() > 3);
        // a branch and a local workflow aren't versions
        assert!(all.iter().all(|d| d.name != "dtolnay/rust-toolchain"));
        assert!(all.iter().all(|d| !d.name.starts_with('.')));
    }

    #[test]
    fn requirements_say_which_majors_they_take() {
        let r = requirement;
        assert_eq!(
            r("maturin>=1.15,<2"),
            Some(("maturin".into(), ">=1.15,<2".into(), Some(1)))
        );
        assert_eq!(r("numpy>=1.21").unwrap().2, None);
        assert_eq!(r("a<2.1").unwrap().2, Some(2));
        assert_eq!(r("a<=3.4").unwrap().2, Some(3));
        assert_eq!(r("a~=4.2").unwrap().2, Some(4));
        assert_eq!(r("a==5.*").unwrap().2, Some(5));
        assert_eq!(
            r("a[extra]>=1,<3 ; python_version<'3.11'").unwrap().2,
            Some(2)
        );
        assert_eq!(major("^6.12.4"), Some(6));
        assert_eq!(major("0.24.4"), Some(0));
        assert_eq!(major("v8.0.1"), Some(8));
        let acts = actions(
            "steps:\n  - uses: actions/checkout@v7 # pinned\n  - uses: dtolnay/rust-toolchain@1.95\n\
             \x20 - uses: ./.github/workflows/x.yml\n  - name: x\n    uses: 'PyO3/maturin-action@v1.51.0'\n\
             \x20 - uses: github/codeql-action/init@v4\n  permissions:\n    statuses: write\n",
            "w.yml",
        );
        let names: Vec<(&str, &str)> = acts
            .iter()
            .map(|a| (a.name.as_str(), a.ours.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("actions/checkout", "v7"),
                ("PyO3/maturin-action", "v1.51.0"),
                ("github/codeql-action", "v4")
            ]
        );
        assert_eq!(index_path("serde"), "se/rd/serde");
        assert_eq!(index_path("toml"), "to/ml/toml");
        assert_eq!(index_path("tar"), "3/t/tar");
        assert_eq!(index_path("cc"), "2/cc");
        assert_eq!(
            repository("git+https://github.com/vitejs/vite.git"),
            "https://github.com/vitejs/vite"
        );
        assert_eq!(
            index_newest(
                "{\"vers\":\"1.0.0\"}\n{\"vers\":\"2.0.0\",\"yanked\":true}\n{\"vers\":\"1.5.0-rc.1\"}\n{\"vers\":\"1.4.2\"}"
            )
            .as_deref(),
            Some("1.4.2")
        );
    }

    struct Canned(Vec<(String, String)>);

    impl Http for Canned {
        fn get(&self, url: &str, _: &[(&str, &str)]) -> Result<Response, String> {
            self.0
                .iter()
                .find(|(u, _)| u == url)
                .map(|(_, b)| Response {
                    status: 200,
                    headers: Vec::new(),
                    body: b.clone().into_bytes(),
                })
                .ok_or_else(|| format!("{url}: unreachable"))
        }

        fn download(
            &self,
            _: &str,
            _: &Path,
            _: &mut dyn FnMut(u64, Option<u64>),
            _: &dyn Fn() -> bool,
        ) -> Result<(), String> {
            unreachable!()
        }
    }

    #[test]
    fn the_issue_lists_the_majors_and_comments_on_new_ones() {
        let dep = |kind, name: &str, ours: &str, takes| Dependency {
            kind,
            name: name.into(),
            ours: ours.into(),
            takes,
            files: vec!["f".into()],
        };
        let deps = vec![
            dep(Kind::Crate, "serde", "1.0.200", Some(1)),
            dep(Kind::Crate, "wgpu", "30.0.1", Some(30)),
            dep(Kind::Npm, "typescript", "5.9.3", Some(5)),
            dep(Kind::Python, "numpy", ">=1.21", None),
            dep(Kind::Action, "actions/checkout", "v7", Some(7)),
            dep(Kind::Action, "gone/away", "v1", Some(1)),
        ];
        let http = Canned(vec![
            ("https://index.crates.io/se/rd/serde".into(), "{\"vers\":\"1.0.229\"}".into()),
            ("https://index.crates.io/wg/pu/wgpu".into(), "{\"vers\":\"30.0.1\"}\n{\"vers\":\"31.0.0\"}".into()),
            (
                "https://crates.io/api/v1/crates/wgpu".into(),
                r#"{"crate":{"repository":"https://github.com/gfx-rs/wgpu"},"versions":[{"num":"31.0.0","created_at":"2026-10-01T00:00:00Z"}]}"#.into(),
            ),
            (
                "https://registry.npmjs.org/typescript/latest".into(),
                r#"{"version":"7.0.2","repository":{"url":"git+https://github.com/microsoft/TypeScript.git"}}"#.into(),
            ),
            ("https://registry.npmjs.org/typescript".into(), r#"{"time":{"7.0.2":"2026-10-08T00:00:00Z"}}"#.into()),
            (
                "https://pypi.org/pypi/numpy/json".into(),
                r#"{"info":{"version":"3.0.0","project_urls":{"Source":"https://github.com/numpy/numpy"}},"releases":{}}"#.into(),
            ),
            (
                "https://api.github.com/repos/actions/checkout/releases/latest".into(),
                r#"{"tag_name":"v8.0.0","published_at":"2026-10-02T00:00:00Z","html_url":"https://github.com/actions/checkout/releases/tag/v8.0.0"}"#.into(),
            ),
        ]);
        let rows = rows(&mut Fetch::new(&http, None), deps);
        let majors: Vec<String> = rows
            .iter()
            .filter(|r| r.major())
            .map(|r| r.dependency.key())
            .collect();
        assert_eq!(
            majors,
            ["crate wgpu", "npm typescript", "Action actions/checkout"]
        );
        let first = body(&rows, None, "2026-10-12");
        assert!(news(&rows, None).is_empty());
        assert!(first.contains("| wgpu | crate | 30.0.1 | **31.0.0** | 2026-10-01 | [release notes](https://github.com/gfx-rs/wgpu/releases) | f |"), "{first}");
        assert!(first.contains("| typescript | npm | 5.9.3 | **7.0.2** | 2026-10-08 |"));
        // numpy's requirement takes every major version
        assert!(!first.contains("| numpy |"));
        assert!(first.contains("- Action gone/away: "));
        // a week on: typescript was at 6 then, checkout at 7; wgpu's 31 was known
        let mut earlier = recorded(&first).unwrap();
        earlier.insert("npm typescript".into(), "6.0.1".into());
        earlier.insert("Action actions/checkout".into(), "v7.2.0".into());
        let new = news(&rows, Some(&earlier));
        let keys: Vec<String> = new.iter().map(|r| r.dependency.key()).collect();
        assert_eq!(keys, ["npm typescript", "Action actions/checkout"]);
        let c = comment(&new);
        assert!(c.contains("- npm typescript 7.0.2, 2026-10-08 (ours: 5.9.3): https://github.com/microsoft/TypeScript/releases"), "{c}");
        // the unread action keeps its recorded version
        earlier.insert("Action gone/away".into(), "v1.0.0".into());
        let second = body(&rows, Some(&earlier), "2026-10-19");
        assert_eq!(recorded(&second).unwrap()["Action gone/away"], "v1.0.0");
    }
}
