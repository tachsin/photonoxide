//! Each external library's newest release, beside the version its guide was checked with and
//! the releases photonoxide-native accepts: `photonoxide libraries --releases`.
//!
//! The Releases workflow (`.github/workflows/releases.yml`) runs it every Monday and keeps its
//! table in one GitHub issue, "New releases of external libraries", commenting there when a
//! release appears. A release is read where its publisher puts it: anaconda.org's API for
//! conda-forge, PyPI's JSON API, NVIDIA's redistributables' manifests, the projects' GitHub
//! releases, MUMPS's download page, and the readmes of GitHub's macOS runner images for
//! Accelerate, which is part of macOS (Apple's own feed of macOS releases is signed by a
//! certificate authority of Apple's that ordinary clients don't trust). A source that can't
//! be read is said so in the table, and the run fails after writing it.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use photonoxide_native::{mumps, nvidia, superlu};
use serde_json::Value;

use super::GUIDES;
use crate::channels::github::Http;

/// The issue the workflow keeps up to date.
pub const ISSUE_TITLE: &str = "New releases of external libraries";

/// The macOS of the newest image the Libraries workflow runs Accelerate's tests on
/// (`macos-26` in `.github/workflows/libraries.yml`): Accelerate has no install to check.
pub const ACCELERATE_CHECKED: &str = "26";

/// What to do with a new release, in the issue and in docs/libraries.md.
pub const STEPS: &str = "\
1. **Read its release notes** for what photonoxide calls: MUMPS's `ZMUMPS_STRUC_C` and \
SuperLU's options change between releases, and NVIDIA's, oneMKL's and OpenBLAS's file names \
carry a major release.
2. **Outside what photonoxide-native accepts,** it is refused until its support is added: the \
release's layout in `native/src/mumps.rs` (from its `zmumps_c.h`), its major release in \
`native/src/superlu.rs`, its file names in `native/src/nvidia.rs`.
3. **Install it on clean machines:** the Libraries workflow on a branch (`gh workflow run \
libraries.yml --ref <branch>`) installs each method, asks photonoxide what it finds, and runs \
each backend's tests with its library required.
4. **Check it against photonoxide's own solvers** on a machine that has it (and a GPU, for \
NVIDIA's): the backend's tests (`cargo test -p photonoxide-native --release --test <backend>`, \
with `PHOTONOXIDE_REQUIRE_<LIBRARY>` set where the test has one) and the benchmark's problems \
(`photonoxide bench --tier standard --backends <backend>`), whose answers are checked.
5. **Then record it:** its `Checked` entries in `studio/src-tauri/src/libraries.rs`, and \
`photonoxide libraries --write docs/libraries.md`. Only then do the guides call it checked.
";

/// Where a library's newest release is read.
#[derive(Clone, Copy, Debug)]
pub enum Origin {
    /// A conda-forge package, from anaconda.org's API: the newest version on its main label.
    Conda(&'static str),
    /// A package on PyPI, from its JSON API: its newest release.
    PyPi(&'static str),
    /// A GitHub repository's latest release (`owner/name`).
    GitHub(&'static str),
    /// NVIDIA's redistributables: the newest manifest (`redistrib_<release>.json`) of a
    /// product, and one component's version in it.
    Nvidia {
        product: &'static str,
        component: &'static str,
        name: &'static str,
    },
    /// MUMPS's download page, its table of releases.
    Mumps,
    /// The macOS of GitHub's runner images, from their readmes.
    MacosRunners,
}

impl Origin {
    /// A short name, which with the library's names the row.
    pub fn name(&self) -> String {
        match self {
            Origin::Conda(p) => format!("conda-forge {p}"),
            Origin::PyPi(p) => format!("PyPI {p}"),
            Origin::GitHub(repo) => format!("GitHub {repo}"),
            Origin::Nvidia { name, .. } => format!("NVIDIA's {name} redistributables"),
            Origin::Mumps => "mumps-solver.org".into(),
            Origin::MacosRunners => "GitHub's macOS runners".into(),
        }
    }

    /// The page a reader can look at.
    pub fn page(&self) -> String {
        match self {
            Origin::Conda(p) => format!("https://anaconda.org/conda-forge/{p}"),
            Origin::PyPi(p) => format!("https://pypi.org/project/{p}/"),
            Origin::GitHub(repo) => format!("https://github.com/{repo}/releases"),
            Origin::Nvidia { product, .. } => {
                format!("https://developer.download.nvidia.com/compute/{product}/redist/")
            }
            Origin::Mumps => MUMPS_PAGE.into(),
            Origin::MacosRunners => {
                "https://github.com/actions/runner-images/tree/main/images/macos".into()
            }
        }
    }
}

/// The version a source is compared with.
#[derive(Clone, Copy, Debug)]
pub enum Baseline {
    /// The newest version of these packages that the guide's checks installed: with one
    /// manager's install, or with any of the guide's.
    Guide {
        manager: Option<&'static str>,
        packages: &'static [&'static str],
    },
    /// A version given here, for a library with nothing to install.
    Stated(&'static str),
}

/// A release's notes.
#[derive(Clone, Copy, Debug)]
pub enum Notes {
    /// One page for every release, and what to call it.
    Page(&'static str, &'static str),
    /// A GitHub repository's release of the version, by its tag `v<version>`.
    GitHubTag(&'static str),
}

/// One place a library's newest release is read.
#[derive(Clone, Copy, Debug)]
pub struct Source {
    /// The library, as its guide names it.
    pub library: &'static str,
    pub origin: Origin,
    pub baseline: Baseline,
    pub notes: Notes,
}

const INTEL_NOTES: Notes = Notes::Page(
    "Intel's release notes",
    "https://www.intel.com/content/www/us/en/developer/articles/release-notes/onemkl-release-notes.html",
);
const CUDA_NOTES: Notes = Notes::Page(
    "the CUDA toolkit's release notes",
    "https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html",
);
const CUDSS_NOTES: Notes = Notes::Page(
    "cuDSS's release notes",
    "https://docs.nvidia.com/cuda/cudss/release_notes.html",
);
const MUMPS_PAGE: &str = "https://mumps-solver.org/index.php?page=dwnld";
const MUMPS_NOTES: Notes = Notes::Page(
    "MUMPS's changelog",
    "https://mumps-solver.org/index.php?page=dwnld#cl",
);
const MACOS_NOTES: Notes = Notes::Page(
    "Apple's macOS release notes",
    "https://developer.apple.com/documentation/macos-release-notes",
);

const fn guide(manager: &'static str, packages: &'static [&'static str]) -> Baseline {
    Baseline::Guide {
        manager: Some(manager),
        packages,
    }
}

const fn any(packages: &'static [&'static str]) -> Baseline {
    Baseline::Guide {
        manager: None,
        packages,
    }
}

/// Every place a release is read, the library's publisher first where it publishes one
/// readably, then each package manager the guides install with (apt's packages are fixed for
/// each Ubuntu release, and winget's index is checked by the Libraries workflow).
pub const SOURCES: &[Source] = &[
    // Intel publishes the PyPI wheel itself
    Source {
        library: "oneMKL",
        origin: Origin::PyPi("mkl"),
        baseline: guide("pip", &["mkl"]),
        notes: INTEL_NOTES,
    },
    Source {
        library: "oneMKL",
        origin: Origin::Conda("mkl"),
        baseline: guide("conda-forge", &["mkl"]),
        notes: INTEL_NOTES,
    },
    Source {
        library: "CUDA runtime",
        origin: Origin::Nvidia {
            product: "cuda",
            component: "cuda_cudart",
            name: "CUDA",
        },
        baseline: any(&["cuda-cudart", "nvidia-cuda-runtime"]),
        notes: CUDA_NOTES,
    },
    Source {
        library: "CUDA runtime",
        origin: Origin::Conda("cuda-cudart"),
        baseline: guide("conda-forge", &["cuda-cudart"]),
        notes: CUDA_NOTES,
    },
    Source {
        library: "CUDA runtime",
        origin: Origin::PyPi("nvidia-cuda-runtime"),
        baseline: guide("pip", &["nvidia-cuda-runtime"]),
        notes: CUDA_NOTES,
    },
    Source {
        library: "cuSPARSE",
        origin: Origin::Nvidia {
            product: "cuda",
            component: "libcusparse",
            name: "CUDA",
        },
        baseline: any(&["libcusparse", "nvidia-cusparse"]),
        notes: CUDA_NOTES,
    },
    Source {
        library: "cuSPARSE",
        origin: Origin::Conda("libcusparse"),
        baseline: guide("conda-forge", &["libcusparse"]),
        notes: CUDA_NOTES,
    },
    Source {
        library: "cuSPARSE",
        origin: Origin::PyPi("nvidia-cusparse"),
        baseline: guide("pip", &["nvidia-cusparse"]),
        notes: CUDA_NOTES,
    },
    Source {
        library: "cuDSS",
        origin: Origin::Nvidia {
            product: "cudss",
            component: "libcudss",
            name: "cuDSS",
        },
        baseline: any(&["libcudss", "nvidia-cudss-cu13"]),
        notes: CUDSS_NOTES,
    },
    Source {
        library: "cuDSS",
        origin: Origin::Conda("libcudss"),
        baseline: guide("conda-forge", &["libcudss"]),
        notes: CUDSS_NOTES,
    },
    Source {
        library: "cuDSS",
        origin: Origin::PyPi("nvidia-cudss-cu13"),
        baseline: guide("pip", &["nvidia-cudss-cu13"]),
        notes: CUDSS_NOTES,
    },
    Source {
        library: "MUMPS",
        origin: Origin::Mumps,
        baseline: any(&["mumps-seq", "libmumps-seq-dev"]),
        notes: MUMPS_NOTES,
    },
    Source {
        library: "MUMPS",
        origin: Origin::Conda("mumps-seq"),
        baseline: guide("conda-forge", &["mumps-seq"]),
        notes: MUMPS_NOTES,
    },
    Source {
        library: "SuperLU",
        origin: Origin::GitHub("xiaoyeli/superlu"),
        baseline: any(&["superlu", "libsuperlu-dev"]),
        notes: Notes::GitHubTag("xiaoyeli/superlu"),
    },
    Source {
        library: "SuperLU",
        origin: Origin::Conda("superlu"),
        baseline: guide("conda-forge", &["superlu"]),
        notes: Notes::GitHubTag("xiaoyeli/superlu"),
    },
    Source {
        library: "OpenBLAS",
        origin: Origin::GitHub("OpenMathLib/OpenBLAS"),
        baseline: any(&["openblas", "libopenblas-dev"]),
        notes: Notes::GitHubTag("OpenMathLib/OpenBLAS"),
    },
    Source {
        library: "OpenBLAS",
        origin: Origin::Conda("openblas"),
        baseline: guide("conda-forge", &["openblas"]),
        notes: Notes::GitHubTag("OpenMathLib/OpenBLAS"),
    },
    Source {
        library: "Accelerate",
        origin: Origin::MacosRunners,
        baseline: Baseline::Stated(ACCELERATE_CHECKED),
        notes: MACOS_NOTES,
    },
];

/// The releases photonoxide-native accepts of a library.
#[derive(Clone, Debug, PartialEq)]
pub enum Accepts {
    /// These and no other.
    Releases(Vec<&'static str>),
    /// Those whose first number is one of these.
    Majors(Vec<u32>),
    /// This one and every later one.
    From(&'static str),
}

/// What photonoxide-native accepts of a library, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    pub accepts: Accepts,
    pub why: &'static str,
}

impl Range {
    pub fn contains(&self, version: &str) -> bool {
        match &self.accepts {
            Accepts::Releases(known) => {
                known.iter().any(|k| compare(k, version) == Ordering::Equal)
            }
            Accepts::Majors(majors) => numbers(version)
                .first()
                .is_some_and(|m| majors.iter().any(|k| u64::from(*k) == *m)),
            Accepts::From(least) => compare(version, least) != Ordering::Less,
        }
    }

    /// The range in words: `5.4.1 to 5.8.2`, `5 to 7`, `12 and 13`, `0.x`, `15.5 and later`.
    pub fn describe(&self) -> String {
        let span = match &self.accepts {
            Accepts::Releases(known) => match known.as_slice() {
                [] => "none".into(),
                [one] => (*one).into(),
                [first, .., last] => format!("{first} to {last}"),
            },
            Accepts::Majors(majors) => {
                let mut m = majors.clone();
                m.sort_unstable();
                match m.as_slice() {
                    [] => "none".into(),
                    [one] => format!("{one}.x"),
                    [a, b] => format!("{a}.x and {b}.x"),
                    [first, .., last] => format!("{first} to {last}"),
                }
            }
            Accepts::From(least) => format!("{least} and later"),
        };
        format!("{span}, {}", self.why)
    }
}

/// What photonoxide-native accepts of `library`, from its own constants where it has them.
pub fn range(library: &str) -> Option<Range> {
    let majors = |m: &[u32]| Accepts::Majors(m.to_vec());
    Some(match library {
        "oneMKL" => Range {
            accepts: Accepts::From("2020"),
            why: "by mkl_rt.3, mkl_rt.2 and Debian's unversioned mkl_rt",
        },
        "CUDA runtime" => Range {
            accepts: majors(nvidia::CUDA_RUNTIME_RELEASES),
            why: "by its files' names",
        },
        "cuSPARSE" => Range {
            accepts: majors(nvidia::CUSPARSE_RELEASES),
            why: "by its files' names",
        },
        "cuDSS" => Range {
            accepts: majors(nvidia::CUDSS_RELEASES),
            why: "by its files' names",
        },
        "MUMPS" => Range {
            accepts: Accepts::Releases(mumps::releases().collect()),
            why: "the releases whose structure it knows",
        },
        "SuperLU" => Range {
            accepts: Accepts::Majors(superlu::RELEASES.collect()),
            why: "the releases whose drivers it calls",
        },
        "OpenBLAS" => Range {
            accepts: majors(&[0]),
            why: "by libopenblas.so.0 and its likes",
        },
        "Accelerate" => Range {
            accepts: Accepts::From("15.5"),
            why: "macOS with complex LU (and L D Lᵀ from 26)",
        },
        _ => return None,
    })
}

/// A version's numbers, up to the first part that isn't one: `v0.3.34` is 0, 3, 34;
/// `2020.4.304-2ubuntu3` 2020, 4, 304.
pub fn numbers(version: &str) -> Vec<u64> {
    let v = version.trim().trim_start_matches(['v', 'V']);
    let v = v.split(['-', '+', ' ', '_']).next().unwrap_or("");
    v.split('.').map_while(|p| p.parse().ok()).collect()
}

/// Two versions by their numbers, a missing number as 0: `26` is `26.0`.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (a, b) = (numbers(a), numbers(b));
    for i in 0..a.len().max(b.len()) {
        let o = a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0));
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

/// The version a source is compared with: the newest its guide's checks installed, or the one
/// stated.
pub fn baseline(source: &Source) -> Option<String> {
    let (manager, packages) = match source.baseline {
        Baseline::Stated(v) => return Some(v.to_owned()),
        Baseline::Guide { manager, packages } => (manager, packages),
    };
    let guide = GUIDES.iter().find(|g| g.library == source.library)?;
    guide
        .installs
        .iter()
        .filter(|i| manager.is_none_or(|m| i.manager == m))
        .flat_map(|i| i.checked.iter())
        .flat_map(|c| installed(c.installed))
        .filter(|(package, _)| packages.contains(package))
        .map(|(_, version)| version)
        .max_by(|a, b| compare(a, b))
        .map(str::to_owned)
}

/// A check's `installed`, as packages and their versions: `mumps-seq 5.8.2, mkl 2026.1.0`.
fn installed(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.split(", ").filter_map(|p| p.trim().split_once(' '))
}

/// A release as its source gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    /// As `2026-09-16`, or `2026-07` where only the month is given.
    pub date: String,
    /// Where it came from, when the source says: a CUDA release, a runner image.
    pub label: Option<String>,
    /// Its notes' page, when the source gives one.
    pub notes: Option<String>,
}

/// GETs over HTTP, each URL once.
struct Fetch<'a> {
    http: &'a dyn Http,
    /// The workflow's token, sent to GitHub's API alone, for its higher rate limit.
    token: Option<String>,
    seen: HashMap<String, Result<Vec<u8>, String>>,
}

impl Fetch<'_> {
    fn bytes(&mut self, url: &str) -> Result<Vec<u8>, String> {
        if let Some(r) = self.seen.get(url) {
            return r.clone();
        }
        let mut headers = Vec::new();
        let auth;
        if url.starts_with("https://api.github.com/") {
            headers.push(("Accept", "application/vnd.github+json"));
            if let Some(t) = &self.token {
                auth = format!("Bearer {t}");
                headers.push(("Authorization", auth.as_str()));
            }
        }
        let r = self.http.get(url, &headers).and_then(|r| match r.status {
            200 => Ok(r.body),
            s => Err(format!("{url}: HTTP {s}")),
        });
        self.seen.insert(url.to_owned(), r.clone());
        r
    }

    fn text(&mut self, url: &str) -> Result<String, String> {
        self.bytes(url)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    fn json(&mut self, url: &str) -> Result<Value, String> {
        serde_json::from_slice(&self.bytes(url)?).map_err(|e| format!("{url}: {e}"))
    }
}

/// The first ten characters of a timestamp: its day.
fn day(timestamp: &str) -> Option<String> {
    let d = timestamp.get(..10)?;
    (d.len() == 10 && d.as_bytes()[4] == b'-').then(|| d.to_owned())
}

/// anaconda.org's package: the newest version on the main label, and its first upload.
fn conda(package: &Value) -> Result<Release, String> {
    let files = package["files"]
        .as_array()
        .ok_or("anaconda.org's answer has no files")?;
    let mut first: BTreeMap<String, String> = BTreeMap::new();
    for f in files {
        let main = f["labels"]
            .as_array()
            .is_some_and(|l| l.iter().any(|l| l == "main"));
        let (Some(v), Some(t)) = (f["version"].as_str(), f["upload_time"].as_str()) else {
            continue;
        };
        if !main {
            continue;
        }
        let t = day(t).unwrap_or_default();
        let e = first.entry(v.to_owned()).or_insert_with(|| t.clone());
        if t < *e {
            *e = t;
        }
    }
    let (version, date) = first
        .into_iter()
        .max_by(|a, b| compare(&a.0, &b.0))
        .ok_or("no version on conda-forge's main label")?;
    Ok(Release {
        version,
        date,
        label: None,
        notes: None,
    })
}

/// PyPI's project: its newest release, and the first upload of it.
fn pypi(project: &Value) -> Result<Release, String> {
    let version = project["info"]["version"]
        .as_str()
        .ok_or("PyPI's answer has no version")?;
    let date = project["releases"][version]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["upload_time_iso_8601"].as_str().and_then(day))
        .min()
        .unwrap_or_default();
    Ok(Release {
        version: version.to_owned(),
        date,
        label: None,
        notes: None,
    })
}

/// GitHub's latest release.
fn github(release: &Value) -> Result<Release, String> {
    let tag = release["tag_name"]
        .as_str()
        .ok_or("GitHub's answer has no tag")?;
    Ok(Release {
        version: tag.trim_start_matches(['v', 'V']).to_owned(),
        date: release["published_at"]
            .as_str()
            .and_then(day)
            .unwrap_or_default(),
        label: None,
        notes: release["html_url"].as_str().map(str::to_owned),
    })
}

/// The newest manifest named in a directory's listing: `redistrib_13.4.2.json`.
fn newest_manifest(listing: &str) -> Option<String> {
    listing
        .split("redistrib_")
        .skip(1)
        .filter_map(|rest| rest.split_once(".json").map(|(v, _)| v))
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
        .max_by(|a, b| compare(a, b))
        .map(str::to_owned)
}

/// A component's version in an NVIDIA manifest, and the manifest's release.
fn nvidia_release(manifest: &Value, component: &str, name: &str) -> Result<Release, String> {
    let version = manifest[component]["version"]
        .as_str()
        .ok_or_else(|| format!("the manifest has no {component}"))?;
    Ok(Release {
        version: version.to_owned(),
        date: manifest["release_date"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        label: manifest["release_label"]
            .as_str()
            .map(|l| format!("{name} {l}")),
        notes: None,
    })
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `July 2026` as `2026-07`.
fn month(text: &str) -> Option<String> {
    let mut words = text.split_whitespace();
    let m = words.next()?;
    let year: u32 = words.next()?.parse().ok()?;
    let n = MONTHS.iter().position(|x| x.eq_ignore_ascii_case(m))? + 1;
    Some(format!("{year}-{n:02}"))
}

/// MUMPS's download page: its table's newest release (`<td>Release 5.9.1</td><td>: July
/// 2026`).
fn mumps_release(page: &str) -> Result<Release, String> {
    page.split("Release ")
        .skip(1)
        .filter_map(|rest| {
            let version = rest.split(['<', ' ', '\n']).next()?;
            if numbers(version).len() != 3 {
                return None;
            }
            // the date, after the colon in the next cell
            let after = rest.split_once(':')?.1;
            let date = month(after.split('<').next()?.trim())?;
            Some((version.to_owned(), date))
        })
        .max_by(|a, b| compare(&a.0, &b.0))
        .map(|(version, date)| Release {
            version,
            date,
            label: None,
            notes: None,
        })
        .ok_or_else(|| "MUMPS's page has no table of releases".to_owned())
}

/// A runner image's readme: its macOS (`- OS Version: macOS 27.0.1 (26A434)`) and the day it
/// was built (`- Image Version: 20261006.0244.1`).
fn runner_macos(readme: &str) -> Option<(String, String)> {
    let line = |key: &str| {
        readme
            .lines()
            .find_map(|l| l.trim().strip_prefix(key).map(str::trim))
    };
    let os = line("- OS Version:")?
        .trim_start_matches("macOS")
        .split_whitespace()
        .next()?
        .to_owned();
    let image = line("- Image Version:")?;
    let date = match image.get(..8) {
        Some(d) if d.bytes().all(|b| b.is_ascii_digit()) => {
            format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])
        }
        _ => String::new(),
    };
    Some((os, date))
}

const RUNNER_IMAGES: &str =
    "https://api.github.com/repos/actions/runner-images/contents/images/macos";

/// The newest release `source` reads.
fn newest(fetch: &mut Fetch, source: &Source) -> Result<Release, String> {
    match source.origin {
        Origin::Conda(p) => {
            conda(&fetch.json(&format!("https://api.anaconda.org/package/conda-forge/{p}"))?)
        }
        Origin::PyPi(p) => pypi(&fetch.json(&format!("https://pypi.org/pypi/{p}/json"))?),
        Origin::GitHub(repo) => github(&fetch.json(&format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))?),
        Origin::Nvidia {
            product,
            component,
            name,
        } => {
            let base = format!("https://developer.download.nvidia.com/compute/{product}/redist/");
            let release = newest_manifest(&fetch.text(&base)?)
                .ok_or_else(|| format!("{base}: no manifest listed"))?;
            let manifest = fetch.json(&format!("{base}redistrib_{release}.json"))?;
            nvidia_release(&manifest, component, name)
        }
        Origin::Mumps => mumps_release(&fetch.text(MUMPS_PAGE)?),
        Origin::MacosRunners => {
            let listing = fetch.json(RUNNER_IMAGES)?;
            let mut best: Option<Release> = None;
            for entry in listing
                .as_array()
                .ok_or("the images' listing isn't a list")?
            {
                let (Some(name), Some(url)) =
                    (entry["name"].as_str(), entry["download_url"].as_str())
                else {
                    continue;
                };
                let Some(image) = name.strip_suffix("-Readme.md") else {
                    continue;
                };
                let Some((os, date)) = runner_macos(&fetch.text(url)?) else {
                    continue;
                };
                let newer = best.as_ref().is_none_or(|b| {
                    compare(&os, &b.version)
                        .then_with(|| date.cmp(&b.date))
                        .is_gt()
                });
                if newer {
                    best = Some(Release {
                        version: os,
                        date,
                        label: Some(format!("image {image}")),
                        notes: None,
                    });
                }
            }
            best.ok_or_else(|| "no macOS runner image's readme reads".to_owned())
        }
    }
}

/// One source's row: what it was checked with, what it has now, and what photonoxide-native
/// accepts.
#[derive(Clone, Debug)]
pub struct Row {
    pub source: &'static Source,
    pub checked: Option<String>,
    pub newest: Result<Release, String>,
    pub range: Option<Range>,
}

impl Row {
    /// The row's name, as the issue keeps it from one week to the next.
    pub fn key(&self) -> String {
        format!("{} | {}", self.source.library, self.source.origin.name())
    }

    /// Whether the newest release is newer than the one checked.
    pub fn newer(&self) -> bool {
        match (&self.newest, &self.checked) {
            (Ok(r), Some(c)) => compare(&r.version, c).is_gt(),
            (Ok(_), None) => true,
            _ => false,
        }
    }

    /// Whether photonoxide-native accepts the newest release.
    pub fn accepted(&self) -> Option<bool> {
        let r = self.newest.as_ref().ok()?;
        Some(self.range.as_ref()?.contains(&r.version))
    }

    fn notes(&self) -> Option<(String, String)> {
        let r = self.newest.as_ref().ok()?;
        Some(match self.source.notes {
            Notes::Page(name, url) => (name.to_owned(), url.to_owned()),
            Notes::GitHubTag(repo) => (
                format!("v{}", r.version),
                r.notes.clone().unwrap_or_else(|| {
                    format!("https://github.com/{repo}/releases/tag/v{}", r.version)
                }),
            ),
        })
    }

    /// The newest release in words: `MUMPS 5.9.1 (mumps-solver.org, 2026-07)`.
    fn named(&self) -> String {
        match &self.newest {
            Ok(r) => format!(
                "{} {}{} ({}, {})",
                if self.source.library == "Accelerate" {
                    "macOS"
                } else {
                    self.source.library
                },
                r.version,
                r.label
                    .as_ref()
                    .map(|l| format!(", {l}"))
                    .unwrap_or_default(),
                self.source.origin.name(),
                r.date
            ),
            Err(_) => self.key(),
        }
    }
}

/// Reads every source.
pub fn rows(http: &dyn Http, token: Option<String>) -> Vec<Row> {
    let mut fetch = Fetch {
        http,
        token,
        seen: HashMap::new(),
    };
    SOURCES
        .iter()
        .map(|s| Row {
            source: s,
            checked: baseline(s),
            newest: newest(&mut fetch, s),
            range: range(s.library),
        })
        .collect()
}

const MARKER: &str = "<!-- newest releases";

/// The newest versions an earlier issue body recorded, by row: none if it has no record.
pub fn recorded(body: &str) -> Option<BTreeMap<String, String>> {
    let start = body.find(MARKER)? + MARKER.len();
    let end = start + body[start..].find("-->")?;
    Some(
        body[start..end]
            .lines()
            .filter_map(|l| l.rsplit_once(" = "))
            .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
            .collect(),
    )
}

/// The rows whose newest release differs from what `previous` recorded: none when nothing
/// was recorded, as on the issue's first run.
pub fn news<'a>(rows: &'a [Row], previous: Option<&BTreeMap<String, String>>) -> Vec<&'a Row> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    rows.iter()
        .filter(|r| match (&r.newest, previous.get(&r.key())) {
            (Ok(n), Some(old)) => n.version != *old,
            _ => false,
        })
        .collect()
}

/// A Markdown cell, its pipes escaped.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// The issue's body: the record of the newest versions, the table, and what to do.
pub fn body(rows: &[Row], previous: Option<&BTreeMap<String, String>>, today: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{MARKER}");
    for r in rows {
        let version = match &r.newest {
            Ok(n) => Some(n.version.clone()),
            // a source that couldn't be read keeps what it had, so its return isn't news
            Err(_) => previous.and_then(|p| p.get(&r.key()).cloned()),
        };
        if let Some(v) = version {
            let _ = writeln!(out, "{} = {v}", r.key());
        }
    }
    out.push_str("-->\n");
    let _ = writeln!(
        out,
        "The newest release of each external library photonoxide can use, read on {today}, \
         beside the version its install guide was last checked with on a clean machine \
         ([docs/libraries.md](https://github.com/tachsin/photonoxide/blob/main/docs/libraries.md)) \
         and the releases photonoxide-native accepts. The Releases workflow \
         (`.github/workflows/releases.yml`) rewrites this every Monday with \
         `photonoxide libraries --releases`, and comments here when a release appears; it \
         closes nothing.\n"
    );
    out.push_str(
        "| Library | Where | Checked | Newest | Released | Notes | photonoxide-native accepts |\n\
         |---|---|---|---|---|---|---|\n",
    );
    for r in rows {
        let checked = r.checked.clone().unwrap_or_else(|| "—".into());
        let (newest, date) = match &r.newest {
            Ok(n) => {
                let label = n
                    .label
                    .as_ref()
                    .map(|l| format!(" ({l})"))
                    .unwrap_or_default();
                let v = if r.newer() {
                    format!("**{}**, newer{label}", n.version)
                } else {
                    format!("{}{label}", n.version)
                };
                (v, n.date.clone())
            }
            Err(_) => ("couldn't be read".into(), "—".into()),
        };
        let notes = r
            .notes()
            .map_or_else(|| "—".into(), |(name, url)| format!("[{name}]({url})"));
        let accepts = match (r.accepted(), &r.range) {
            (Some(true), Some(range)) => format!("yes: {}", range.describe()),
            (Some(false), Some(range)) => format!("**no**: {}", range.describe()),
            (_, Some(range)) => range.describe(),
            (_, None) => "—".into(),
        };
        let _ = writeln!(
            out,
            "| {} | [{}]({}) | {} | {} | {} | {} | {} |",
            cell(r.source.library),
            cell(&r.source.origin.name()),
            r.source.origin.page(),
            cell(&checked),
            cell(&newest),
            cell(&date),
            notes,
            cell(&accepts)
        );
    }
    out.push('\n');
    let newer: Vec<String> = rows.iter().filter(|r| r.newer()).map(Row::named).collect();
    if newer.is_empty() {
        out.push_str("Nothing is newer than the guides were checked with.\n\n");
    } else {
        let _ = writeln!(
            out,
            "**Newer than the guides were checked with:** {}.\n",
            newer.join("; ")
        );
    }
    let failed: Vec<String> = rows
        .iter()
        .filter_map(|r| r.newest.as_ref().err().map(|e| format!("{}: {e}", r.key())))
        .collect();
    if !failed.is_empty() {
        out.push_str("**Couldn't be read:**\n\n");
        for f in failed {
            let _ = writeln!(out, "- {}", cell(&f));
        }
        out.push('\n');
    }
    out.push_str("## When a release is new\n\n");
    out.push_str(STEPS);
    out
}

/// The comment for what appeared since the last run.
pub fn comment(new: &[&Row]) -> String {
    let mut out = String::from("New since the last check:\n\n");
    for r in new {
        let notes = r
            .notes()
            .map(|(name, url)| format!(": [{name}]({url})"))
            .unwrap_or_default();
        let accepted = match (r.accepted(), &r.range) {
            (Some(false), Some(range)) => format!(
                "; photonoxide-native doesn't accept it yet ({})",
                range.describe()
            ),
            _ => String::new(),
        };
        let _ = writeln!(out, "- {}{notes}{accepted}", r.named());
    }
    out.push_str(
        "\nThe issue's table is up to date, with what to do with a new release under it.\n",
    );
    out
}

/// Days since 1970 as a day of the Gregorian calendar (Hinnant's `civil_from_days`).
pub fn date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    date(i64::try_from(secs / 86_400).unwrap_or(0))
}

/// `photonoxide libraries --releases [--previous <file>] [--write <file>] [--news <file>]`:
/// reads every source, prints the issue's body or writes it, and writes the comment on what is
/// new since `--previous` (an earlier body) when something is, an empty file otherwise. Fails,
/// after writing, when a source couldn't be read.
pub fn run(args: &[String]) -> std::process::ExitCode {
    use std::process::ExitCode;
    let (mut previous, mut write, mut news_file) = (None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let slot = match a.as_str() {
            "--previous" => &mut previous,
            "--write" => &mut write,
            "--news" => &mut news_file,
            _ => return crate::usage(),
        };
        match it.next() {
            Some(file) => *slot = Some(file.clone()),
            None => return crate::usage(),
        }
    }
    let previous = match &previous {
        Some(file) => match std::fs::read_to_string(file) {
            Ok(text) => recorded(&text),
            Err(e) => return crate::fail(format!("{file}: {e}")),
        },
        None => None,
    };
    let web = crate::channels::github::Web {
        user_agent: format!("photonoxide/{} (library releases)", photonoxide::VERSION),
    };
    let token = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty());
    let rows = rows(&web, token);
    let text = body(&rows, previous.as_ref(), &today());
    let new = news(&rows, previous.as_ref());
    let written = match &write {
        Some(file) => std::fs::write(file, &text).map_err(|e| format!("{file}: {e}")),
        None => {
            print!("{text}");
            Ok(())
        }
    };
    if let Err(e) = written {
        return crate::fail(e);
    }
    if let Some(file) = &news_file {
        let c = if new.is_empty() {
            String::new()
        } else {
            comment(&new)
        };
        if let Err(e) = std::fs::write(file, c) {
            return crate::fail(format!("{file}: {e}"));
        }
    }
    let failed = rows.iter().filter(|r| r.newest.is_err()).count();
    for r in &rows {
        if let Err(e) = &r.newest {
            eprintln!("{}: {e}", r.key());
        }
    }
    if failed > 0 {
        eprintln!("{failed} of {} sources couldn't be read", rows.len());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::github::Response;
    use std::path::Path;

    /// Answers from canned bodies by URL, and counts the requests.
    struct Canned {
        answers: Vec<(String, u16, String)>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl Http for Canned {
        fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
            self.asked.lock().unwrap().push(url.to_owned());
            if !url.starts_with("https://api.github.com/") {
                assert!(headers.iter().all(|(k, _)| *k != "Authorization"), "{url}");
            }
            self.answers
                .iter()
                .find(|(u, _, _)| u == url)
                .map(|(_, status, body)| Response {
                    status: *status,
                    headers: Vec::new(),
                    body: body.clone().into_bytes(),
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
    fn versions_compare_by_their_numbers() {
        assert_eq!(numbers("v0.3.34"), [0, 3, 34]);
        assert_eq!(numbers("2020.4.304-2ubuntu3"), [2020, 4, 304]);
        assert_eq!(numbers("0.8.0.10"), [0, 8, 0, 10]);
        assert_eq!(numbers("7.0.0rc1"), [7, 0]);
        assert_eq!(compare("5.10.0", "5.9.1"), Ordering::Greater);
        assert_eq!(compare("26", "26.0"), Ordering::Equal);
        assert_eq!(compare("27.0.1", "26"), Ordering::Greater);
        assert_eq!(compare("13.4.92", "13.4.92"), Ordering::Equal);
        assert_eq!(compare("12.8.6.49", "12.8.6.72"), Ordering::Less);
    }

    #[test]
    fn the_ranges_are_photonoxide_natives() {
        let mumps = range("MUMPS").unwrap();
        assert!(mumps.contains("5.8.2") && mumps.contains("5.4.1"));
        assert!(!mumps.contains("5.9.1") && !mumps.contains("5.3.5") && !mumps.contains("5.6.3"));
        assert!(mumps.describe().starts_with("5.4.1 to 5.8.2"));
        let superlu = range("SuperLU").unwrap();
        assert!(superlu.contains("7.0.1") && superlu.contains("5.3.0"));
        assert!(!superlu.contains("8.0.0") && !superlu.contains("4.3"));
        assert!(superlu.describe().starts_with("5 to 7"));
        let cuda = range("CUDA runtime").unwrap();
        assert!(cuda.contains("13.4.92") && cuda.contains("12.9.0") && !cuda.contains("14.0.1"));
        assert!(cuda.describe().starts_with("12.x and 13.x"));
        assert!(range("cuSPARSE").unwrap().contains("12.8.6.72"));
        assert!(!range("cuSPARSE").unwrap().contains("13.0.0"));
        let cudss = range("cuDSS").unwrap();
        assert!(cudss.contains("0.8.0.10") && !cudss.contains("1.0.0"));
        assert!(cudss.describe().starts_with("0.x"));
        assert!(range("OpenBLAS").unwrap().contains("0.3.34"));
        assert!(range("oneMKL").unwrap().contains("2026.1.0"));
        assert!(range("oneMKL").unwrap().contains("2020.4.304"));
        let accelerate = range("Accelerate").unwrap();
        assert!(accelerate.contains("27.0.1") && accelerate.contains("15.5"));
        assert!(!accelerate.contains("15.4") && !accelerate.contains("14.7.6"));
        // every source's library has a range, and a guide
        for s in SOURCES {
            assert!(range(s.library).is_some(), "{}", s.library);
            assert!(
                GUIDES.iter().any(|g| g.library == s.library),
                "{}",
                s.library
            );
        }
    }

    #[test]
    fn the_checked_versions_are_the_guides() {
        let of = |library: &str, origin: &str| {
            let s = SOURCES
                .iter()
                .find(|s| s.library == library && s.origin.name() == origin)
                .unwrap();
            baseline(s)
        };
        assert_eq!(of("oneMKL", "conda-forge mkl").as_deref(), Some("2026.1.0"));
        assert_eq!(of("oneMKL", "PyPI mkl").as_deref(), Some("2026.1.0"));
        // the newest of conda-forge's and Ubuntu's
        assert_eq!(of("MUMPS", "mumps-solver.org").as_deref(), Some("5.8.2"));
        assert_eq!(
            of("SuperLU", "GitHub xiaoyeli/superlu").as_deref(),
            Some("7.0.1")
        );
        assert_eq!(
            of("CUDA runtime", "NVIDIA's CUDA redistributables").as_deref(),
            Some("13.4.92")
        );
        assert_eq!(
            of("cuSPARSE", "PyPI nvidia-cusparse").as_deref(),
            Some("12.8.6.72")
        );
        assert_eq!(
            of("cuDSS", "NVIDIA's cuDSS redistributables").as_deref(),
            Some("0.8.0.10")
        );
        assert_eq!(
            of("Accelerate", "GitHub's macOS runners").as_deref(),
            Some("26")
        );
        // every guided source has a checked version, and every key is distinct
        let mut keys = std::collections::HashSet::new();
        for s in SOURCES {
            assert!(baseline(s).is_some(), "{} {}", s.library, s.origin.name());
            assert!(keys.insert(format!("{} | {}", s.library, s.origin.name())));
        }
        assert_eq!(
            installed("mumps-seq 5.8.2, mkl 2026.1.0").collect::<Vec<_>>(),
            [("mumps-seq", "5.8.2"), ("mkl", "2026.1.0")]
        );
    }

    #[test]
    fn accelerate_is_checked_on_the_runner_the_libraries_workflow_uses() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.github/workflows/libraries.yml"
        );
        let workflow = std::fs::read_to_string(path).unwrap();
        assert!(
            workflow.contains(&format!("os: macos-{ACCELERATE_CHECKED}")),
            "libraries.yml runs Accelerate on another macOS than {ACCELERATE_CHECKED}"
        );
    }

    #[test]
    fn days_are_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(11_016), "2000-02-29");
        assert_eq!(date(20_736), "2026-10-10");
    }

    const CONDA: &str = r#"{"name":"mumps-seq","latest_version":"5.8.2","files":[
        {"version":"5.8.1","upload_time":"2026-01-02 10:00:00.000000+00:00","labels":["main"]},
        {"version":"5.8.2","upload_time":"2026-09-21 19:27:39.958000+00:00","labels":["main"]},
        {"version":"5.8.2","upload_time":"2026-09-20 08:00:00.000000+00:00","labels":["main"]},
        {"version":"5.9.0","upload_time":"2026-10-01 08:00:00.000000+00:00","labels":["broken"]}]}"#;

    const PYPI: &str = r#"{"info":{"version":"2026.1.0"},"releases":{
        "2026.0.0":[{"upload_time_iso_8601":"2026-03-01T00:00:00.000Z"}],
        "2026.1.0":[{"upload_time_iso_8601":"2026-07-02T15:11:10.123Z"},
                    {"upload_time_iso_8601":"2026-07-01T15:11:10.123Z"}]}}"#;

    const MUMPS_HTML: &str = "<p>Latest release (July 2026) :</p>\n<table summary=\"\">\n\
        <tr><td>Release 5.9.1</td><td>: July 2026\n</td></tr>\n\
        <tr><td>Release 5.9.0</td><td>: May 2026</td></tr>\n\
        <tr><td>Release 5.8.2</td><td>: January 2026</td></tr>\n</table>";

    const LISTING: &str = "<a href='redistrib_13.4.1.json'>redistrib_13.4.1.json</a>\n\
        <a href='redistrib_13.10.0.json'>x</a><a href='redistrib_13.4.2.json'>x</a>";

    const README: &str = "# macOS 27\n- OS Version: macOS 27.0.1 (26A434)\n\
        - Kernel Version: Darwin 27.0.0\n- Image Version: 20261006.0244.1\n";

    #[test]
    fn each_source_reads_its_answer() {
        let conda = conda(&serde_json::from_str(CONDA).unwrap()).unwrap();
        // the main label alone, and the first upload of the newest
        assert_eq!(
            (conda.version.as_str(), conda.date.as_str()),
            ("5.8.2", "2026-09-20")
        );
        let pypi = pypi(&serde_json::from_str(PYPI).unwrap()).unwrap();
        assert_eq!(
            (pypi.version.as_str(), pypi.date.as_str()),
            ("2026.1.0", "2026-07-01")
        );
        let gh = github(
            &serde_json::json!({"tag_name": "v0.3.34", "published_at": "2026-07-16T19:58:36Z",
                "html_url": "https://github.com/OpenMathLib/OpenBLAS/releases/tag/v0.3.34"}),
        )
        .unwrap();
        assert_eq!(
            (gh.version.as_str(), gh.date.as_str()),
            ("0.3.34", "2026-07-16")
        );
        assert_eq!(newest_manifest(LISTING).as_deref(), Some("13.10.0"));
        assert_eq!(newest_manifest("nothing here"), None);
        let manifest = serde_json::json!({"release_date": "2026-09-16", "release_label": "13.4.2",
            "cuda_cudart": {"version": "13.4.92"}, "libcusparse": {"version": "12.8.6.72"}});
        let cudart = nvidia_release(&manifest, "cuda_cudart", "CUDA").unwrap();
        assert_eq!(cudart.version, "13.4.92");
        assert_eq!(cudart.label.as_deref(), Some("CUDA 13.4.2"));
        assert!(nvidia_release(&manifest, "libcudss", "CUDA").is_err());
        let mumps = mumps_release(MUMPS_HTML).unwrap();
        assert_eq!(
            (mumps.version.as_str(), mumps.date.as_str()),
            ("5.9.1", "2026-07")
        );
        assert!(mumps_release("<p>no table</p>").is_err());
        assert_eq!(
            runner_macos(README),
            Some(("27.0.1".into(), "2026-10-06".into()))
        );
        assert_eq!(runner_macos("# nothing"), None);
        assert_eq!(month("September 2025").as_deref(), Some("2025-09"));
        assert_eq!(month("Sept 2025"), None);
    }

    fn canned() -> Canned {
        let mut answers = Vec::new();
        for s in SOURCES {
            match s.origin {
                Origin::Conda(p) => answers.push((
                    format!("https://api.anaconda.org/package/conda-forge/{p}"),
                    200,
                    CONDA.replace("5.8.2", "1.0.0"),
                )),
                Origin::PyPi(p) => answers.push((
                    format!("https://pypi.org/pypi/{p}/json"),
                    200,
                    PYPI.to_owned(),
                )),
                Origin::GitHub(repo) => answers.push((
                    format!("https://api.github.com/repos/{repo}/releases/latest"),
                    // SuperLU's is out of reach
                    if repo.contains("superlu") { 503 } else { 200 },
                    r#"{"tag_name":"v0.3.35","published_at":"2026-10-01T00:00:00Z"}"#.into(),
                )),
                Origin::Nvidia { product, .. } => {
                    let base =
                        format!("https://developer.download.nvidia.com/compute/{product}/redist/");
                    answers.push((base.clone(), 200, LISTING.into()));
                    answers.push((
                        format!("{base}redistrib_13.10.0.json"),
                        200,
                        r#"{"release_date":"2026-10-01","release_label":"13.10.0",
                            "cuda_cudart":{"version":"13.10.1"},"libcusparse":{"version":"12.9.0"},
                            "libcudss":{"version":"0.9.0.1"}}"#
                            .into(),
                    ));
                }
                Origin::Mumps => answers.push((MUMPS_PAGE.into(), 200, MUMPS_HTML.into())),
                Origin::MacosRunners => {
                    answers.push((
                        RUNNER_IMAGES.into(),
                        200,
                        r#"[{"name":"macos-26-arm64-Readme.md","download_url":"https://raw.example/26"},
                            {"name":"xcode-27-arm64-Readme.md","download_url":"https://raw.example/27"},
                            {"name":"scripts","download_url":null}]"#
                            .into(),
                    ));
                    answers.push((
                        "https://raw.example/26".into(),
                        200,
                        "- OS Version: macOS 26.6.2 (25G83)\n- Image Version: 20260907.0351.1\n"
                            .into(),
                    ));
                    answers.push(("https://raw.example/27".into(), 200, README.into()));
                }
            }
        }
        Canned {
            answers,
            asked: std::sync::Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn the_issue_says_what_is_newer_and_comments_on_what_is_new() {
        let http = canned();
        let rows = rows(&http, Some("token".into()));
        // NVIDIA's manifest, read for the runtime and for cuSPARSE, was asked for once
        let asked = http.asked.lock().unwrap().clone();
        let manifest =
            "https://developer.download.nvidia.com/compute/cuda/redist/redistrib_13.10.0.json";
        assert_eq!(asked.iter().filter(|u| *u == manifest).count(), 1);
        let row = |key: &str| rows.iter().find(|r| r.key() == key).unwrap();
        let mumps = row("MUMPS | mumps-solver.org");
        assert!(mumps.newer() && mumps.accepted() == Some(false));
        let cuda = row("CUDA runtime | NVIDIA's CUDA redistributables");
        assert!(cuda.newer() && cuda.accepted() == Some(true));
        let macos = row("Accelerate | GitHub's macOS runners");
        let release = macos.newest.as_ref().unwrap();
        assert_eq!(release.version, "27.0.1");
        assert_eq!(release.label.as_deref(), Some("image xcode-27-arm64"));
        assert!(macos.newer() && macos.accepted() == Some(true));
        let mkl = row("oneMKL | PyPI mkl");
        assert!(!mkl.newer() && mkl.accepted() == Some(true));
        let superlu = row("SuperLU | GitHub xiaoyeli/superlu");
        assert!(superlu.newest.as_ref().unwrap_err().contains("HTTP 503"));

        // the first run records and doesn't comment
        let first = body(&rows, None, "2026-10-12");
        assert!(news(&rows, None).is_empty());
        let record = recorded(&first).unwrap();
        assert_eq!(record["MUMPS | mumps-solver.org"], "5.9.1");
        assert!(!record.contains_key("SuperLU | GitHub xiaoyeli/superlu"));
        assert!(first.contains("| MUMPS | [mumps-solver.org](https://mumps-solver.org/index.php?page=dwnld) | 5.8.2 | **5.9.1**, newer | 2026-07 | [MUMPS's changelog](https://mumps-solver.org/index.php?page=dwnld#cl) | **no**: 5.4.1 to 5.8.2, the releases whose structure it knows |"), "{first}");
        assert!(first.contains("| oneMKL | [PyPI mkl](https://pypi.org/project/mkl/) | 2026.1.0 | 2026.1.0 | 2026-07-01 |"));
        assert!(first.contains("**Couldn't be read:**\n\n- SuperLU \\| GitHub xiaoyeli/superlu: "));
        assert!(first.contains("MUMPS 5.9.1 (mumps-solver.org, 2026-07)"));
        assert!(first.contains("## When a release is new"));

        // a week on, with a record of an older MUMPS and SuperLU's last version: MUMPS's is
        // news, and SuperLU's version is kept while its source is out of reach
        let mut earlier = record.clone();
        earlier.insert("MUMPS | mumps-solver.org".into(), "5.9.0".into());
        earlier.insert("SuperLU | GitHub xiaoyeli/superlu".into(), "7.0.1".into());
        let new = news(&rows, Some(&earlier));
        assert_eq!(
            new.iter().map(|r| r.key()).collect::<Vec<_>>(),
            ["MUMPS | mumps-solver.org"]
        );
        let c = comment(&new);
        assert!(c.contains("- MUMPS 5.9.1 (mumps-solver.org, 2026-07): [MUMPS's changelog](https://mumps-solver.org/index.php?page=dwnld#cl); photonoxide-native doesn't accept it yet (5.4.1 to 5.8.2"), "{c}");
        let second = body(&rows, Some(&earlier), "2026-10-19");
        assert_eq!(
            recorded(&second).unwrap()["SuperLU | GitHub xiaoyeli/superlu"],
            "7.0.1"
        );
        // and the same week again is no news
        assert!(news(&rows, recorded(&second).as_ref()).is_empty());
        assert_eq!(recorded("an issue without a record"), None);
    }
}
