//! Finding a library's files: every candidate, where it was found, and which one is used.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// What to look for: a library's file names, the variables that may point at it, and where it
/// installs itself.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// The library's name, as photonoxide reports it.
    pub name: &'static str,
    /// Its file names on this platform, the preferred first. Empty where it doesn't exist.
    pub files: &'static [&'static str],
    /// Environment variables naming the file or a folder holding it, photonoxide's own first.
    pub variables: &'static [&'static str],
    /// Its default install folders on this platform, the newest first.
    pub install: fn() -> Vec<PathBuf>,
    /// The folder its vendor's Python wheels install under in `site-packages` (`nvidia`), if
    /// any: the wheels carry the library and nothing needs Python to load it.
    pub wheel: Option<&'static str>,
}

/// Where a candidate was found.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Source {
    /// The caller's setting (the studio's).
    Setting,
    /// An environment variable.
    Variable(String),
    /// The active conda environment (`CONDA_PREFIX`).
    Conda,
    /// The library's default install folder.
    InstallFolder,
    /// A Python wheel's folder.
    Wheel,
    /// The system's library paths (`PATH`, `LD_LIBRARY_PATH`, `DYLD_LIBRARY_PATH`, the usual
    /// folders).
    SystemPath,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Setting => write!(f, "the setting"),
            Source::Variable(name) => write!(f, "${name}"),
            Source::Conda => write!(f, "the conda environment"),
            Source::InstallFolder => write!(f, "the install folder"),
            Source::Wheel => write!(f, "a Python wheel"),
            Source::SystemPath => write!(f, "the system's library path"),
        }
    }
}

/// What became of a candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// The one loaded.
    Used,
    /// Not tried: an earlier one was used.
    NotTried,
    /// Tried, and it failed, with the reason.
    Failed(String),
}

/// A file that may be the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The file.
    pub path: PathBuf,
    /// Where it was found.
    pub source: Source,
    /// What became of it.
    pub status: Status,
}

/// Everything found for a library, in the order tried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discovery {
    /// The library's name.
    pub library: &'static str,
    /// The candidates, in the order they are tried.
    pub candidates: Vec<Candidate>,
}

impl Discovery {
    /// The candidate loaded, if one was.
    pub fn used(&self) -> Option<&Candidate> {
        self.candidates.iter().find(|c| c.status == Status::Used)
    }

    /// Why nothing is used: not found, or each candidate's failure.
    pub fn reason(&self) -> String {
        if self.candidates.is_empty() {
            return format!("{} wasn't found", self.library);
        }
        let failures: Vec<String> = self
            .candidates
            .iter()
            .filter_map(|c| match &c.status {
                Status::Failed(why) => Some(format!("{} ({}): {why}", c.path.display(), c.source)),
                _ => None,
            })
            .collect();
        format!("{} didn't load: {}", self.library, failures.join("; "))
    }
}

/// Every file that may be `spec`'s library, the setting first, then the variables, conda, the
/// install folders, the wheels and the system's paths; each file once.
pub fn discover(spec: &Spec, setting: Option<&Path>) -> Discovery {
    let mut found = Vec::new();
    let mut add = |dir_or_file: &Path, source: Source| {
        if dir_or_file.is_file() {
            found.push((dir_or_file.to_path_buf(), source));
            return;
        }
        for dir in with_library_folders(dir_or_file) {
            for file in spec.files {
                let path = dir.join(file);
                if path.is_file() {
                    found.push((path, source.clone()));
                }
            }
        }
    };
    if let Some(path) = setting {
        add(path, Source::Setting);
    }
    for name in spec.variables {
        if let Some(value) = env::var_os(name) {
            add(Path::new(&value), Source::Variable((*name).to_owned()));
        }
    }
    if let Some(prefix) = env::var_os("CONDA_PREFIX") {
        add(Path::new(&prefix), Source::Conda);
    }
    for dir in (spec.install)() {
        add(&dir, Source::InstallFolder);
    }
    if let Some(package) = spec.wheel {
        for site in site_packages() {
            for dir in folders_below(&site.join(package), 4) {
                add(&dir, Source::Wheel);
            }
        }
    }
    for dir in system_paths() {
        add(&dir, Source::SystemPath);
    }
    let mut seen = HashSet::new();
    let candidates = found
        .into_iter()
        .filter(|(path, _)| seen.insert(fs::canonicalize(path).unwrap_or_else(|_| path.clone())))
        .map(|(path, source)| Candidate {
            path,
            source,
            status: Status::NotTried,
        })
        .collect();
    Discovery {
        library: spec.name,
        candidates,
    }
}

/// A folder and the ones libraries sit in below an install root.
fn with_library_folders(dir: &Path) -> Vec<PathBuf> {
    let below = if cfg!(windows) {
        ["bin/x64", "bin", "Library/bin", "lib"].as_slice()
    } else {
        ["lib64", "lib", "lib/x64"].as_slice()
    };
    std::iter::once(dir.to_path_buf())
        .chain(below.iter().map(|b| dir.join(b)))
        .collect()
}

/// `parent`'s subfolders whose names start with `prefix`, the highest version first
/// (`v13.1` before `v12.8` before `v9.2`).
pub fn versioned(parent: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut dirs: Vec<(Vec<u64>, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix(prefix)
                .map(|rest| (numbers(rest), e.path()))
        })
        .collect();
    dirs.sort_by(|a, b| b.0.cmp(&a.0));
    dirs.into_iter().map(|(_, path)| path).collect()
}

fn numbers(text: &str) -> Vec<u64> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .collect()
}

/// `dir` and its subfolders, `depth` levels down.
fn folders_below(dir: &Path, depth: usize) -> Vec<PathBuf> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = vec![dir.to_path_buf()];
    if depth > 0
        && let Ok(entries) = fs::read_dir(dir)
    {
        let mut children: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        children.sort();
        for child in children {
            out.extend(folders_below(&child, depth - 1));
        }
    }
    out
}

/// Python's package folders: the active virtual environment's, then the user's and the
/// system's.
fn site_packages() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(venv) = env::var_os("VIRTUAL_ENV") {
        roots.push(PathBuf::from(venv));
    }
    if let Some(conda) = env::var_os("CONDA_PREFIX") {
        roots.push(PathBuf::from(conda));
    }
    let mut out = Vec::new();
    if cfg!(windows) {
        for root in &roots {
            out.push(root.join("Lib").join("site-packages"));
        }
        for var in ["LOCALAPPDATA", "APPDATA"] {
            if let Some(base) = env::var_os(var) {
                let base = PathBuf::from(base);
                for python in versioned(&base.join("Programs").join("Python"), "Python")
                    .into_iter()
                    .chain(versioned(&base.join("Python"), "Python"))
                {
                    out.push(python.join("Lib").join("site-packages"));
                    out.push(python.join("site-packages"));
                }
            }
        }
    } else {
        if let Some(home) = env::var_os("HOME") {
            roots.push(PathBuf::from(home).join(".local"));
        }
        roots.extend(["/usr", "/usr/local"].map(PathBuf::from));
        for root in roots {
            for python in versioned(&root.join("lib"), "python3") {
                out.push(python.join("site-packages"));
                out.push(python.join("dist-packages"));
            }
        }
    }
    out.retain(|p| p.is_dir());
    out
}

/// The folders the system loads libraries from.
fn system_paths() -> Vec<PathBuf> {
    let (variable, usual): (&str, &[&str]) = if cfg!(windows) {
        ("PATH", &[])
    } else if cfg!(target_os = "macos") {
        (
            "DYLD_LIBRARY_PATH",
            &["/opt/homebrew/lib", "/usr/local/lib"],
        )
    } else {
        (
            "LD_LIBRARY_PATH",
            &[
                "/usr/local/cuda/lib64",
                "/usr/lib/x86_64-linux-gnu",
                "/usr/lib64",
                "/usr/local/lib",
            ],
        )
    };
    let mut out: Vec<PathBuf> = env::var_os(variable)
        .map(|v| env::split_paths(&v).collect())
        .unwrap_or_default();
    out.extend(usual.iter().map(PathBuf::from));
    out
}
