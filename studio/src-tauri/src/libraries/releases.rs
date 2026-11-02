//! The version of each native library we use, and the releases photonoxide-native accepts:
//! the table under "New releases" in docs/libraries.md.
//!
//! The Dependencies page on tachsin.gr (`site/lib/projects/photonoxide/dependencies/`) reads
//! that table and sets each library's newest release beside it, read where its publisher puts
//! it, so the version we use has one source: the guides' checks ([`GUIDES`]) and the two
//! releases checked without an install ([`MUMPS_CHECKED`], [`ACCELERATE_CHECKED`]).

#[cfg(test)]
use std::cmp::Ordering;

use photonoxide_native::{mumps, nvidia, superlu};

use super::GUIDES;
use crate::watch::compare;
#[cfg(test)]
use crate::watch::numbers;

/// The macOS of the newest image the Libraries workflow runs Accelerate's tests on, as its
/// `sw_vers` gives it (`xcode-27`, GitHub's macOS 27 preview, image xcode-27-arm64, in
/// `.github/workflows/libraries.yml`): Accelerate has no install to check.
pub const ACCELERATE_CHECKED: &str = "27.0.1";

/// The newest MUMPS the Libraries workflow builds from mumps-solver.org's source and runs its
/// backends' tests on (its `mumps-source` job): MUMPS's own releases have no package to check,
/// and conda-forge's and Ubuntu's packages come later.
pub const MUMPS_CHECKED: &str = "5.9.1";

/// What to do with a new release, in docs/libraries.md.
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
`photonoxide libraries --write docs/libraries.md`. Only then do the guides call it checked, \
and the Dependencies page takes it as the version we use.
";

/// Where a library's version we use comes from.
#[derive(Clone, Copy, Debug)]
pub enum Baseline {
    /// The newest version of these packages that the guide's checks installed: with one
    /// manager's install, or with any of the guide's.
    Guide {
        manager: Option<&'static str>,
        packages: &'static [&'static str],
    },
    /// A version given here, for a library with nothing to install or a release checked built
    /// from its source.
    Stated(&'static str),
}

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

/// Each native library, in the guides' order, and where the version we use comes from: the
/// newest of these.
pub const CHECKED: &[(&str, &[Baseline])] = &[
    (
        "oneMKL",
        &[guide("pip", &["mkl"]), guide("conda-forge", &["mkl"])],
    ),
    (
        "CUDA runtime",
        &[any(&["cuda-cudart", "nvidia-cuda-runtime"])],
    ),
    ("cuSPARSE", &[any(&["libcusparse", "nvidia-cusparse"])]),
    ("cuDSS", &[any(&["libcudss", "nvidia-cudss-cu13"])]),
    (
        "MUMPS",
        &[
            Baseline::Stated(MUMPS_CHECKED),
            guide("conda-forge", &["mumps-seq"]),
        ],
    ),
    ("SuperLU", &[any(&["superlu", "libsuperlu-dev"])]),
    ("OpenBLAS", &[any(&["openblas", "libopenblas-dev"])]),
    ("Accelerate", &[Baseline::Stated(ACCELERATE_CHECKED)]),
    ("HDF5", &[any(&["hdf5", "libhdf5-dev"])]),
    ("Zstandard", &[any(&["zstd", "libzstd1"])]),
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
    /// Whether it takes `version`: what the tests check the ranges by.
    #[cfg(test)]
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

    /// The range in words: `5.4.1 to 5.9.1`, `5 to 7`, `12 and 13`, `0.x`, `15.5 and later`.
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
        "HDF5" => Range {
            accepts: Accepts::From("1.10"),
            why: "from the release whose identifiers are 64-bit",
        },
        "Zstandard" => Range {
            accepts: Accepts::From("1.4"),
            why: "libzstd.so.1 and its likes, from the release whose levels run to 22",
        },
        _ => return None,
    })
}

/// The version one baseline gives `library`: the newest its guide's checks installed, or the
/// one stated.
fn baseline(library: &str, b: &Baseline) -> Option<String> {
    let (manager, packages) = match b {
        Baseline::Stated(v) => return Some((*v).to_owned()),
        Baseline::Guide { manager, packages } => (manager, packages),
    };
    let guide = GUIDES.iter().find(|g| g.library == library)?;
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

/// The version we use of `library`: the newest of its baselines.
pub fn uses(library: &str) -> Option<String> {
    let (_, baselines) = CHECKED.iter().find(|(l, _)| *l == library)?;
    baselines
        .iter()
        .filter_map(|b| baseline(library, b))
        .max_by(|a, b| compare(a, b))
}

/// A check's `installed`, as packages and their versions: `mumps-seq 5.8.2, mkl 2026.1.0`.
fn installed(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.split(", ").filter_map(|p| p.trim().split_once(' '))
}

/// docs/libraries.md's table: each library, the version we use and what photonoxide-native
/// accepts. The Dependencies page reads it, by its heading and its columns.
pub fn table() -> String {
    let mut out =
        String::from("| Library | We use | photonoxide-native accepts |\n|---|---|---|\n");
    for (library, _) in CHECKED {
        let uses = uses(library).unwrap_or_else(|| "—".into());
        let accepts = range(library).map_or_else(|| "—".into(), |r| r.describe());
        out.push_str(&format!(
            "| {library} | {uses} | {} |\n",
            accepts.replace('|', "\\|")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ranges_are_photonoxide_natives() {
        let mumps = range("MUMPS").unwrap();
        assert!(mumps.contains("5.8.2") && mumps.contains("5.4.1") && mumps.contains("5.9.1"));
        assert!(!mumps.contains("5.9.2") && !mumps.contains("5.3.5") && !mumps.contains("5.6.3"));
        assert!(mumps.describe().starts_with("5.4.1 to 5.9.1"));
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
        assert!(
            cudss.contains("0.8.0.10") && cudss.contains("1.0.0.35") && !cudss.contains("2.0.0")
        );
        assert!(cudss.describe().starts_with("0.x and 1.x"));
        assert!(range("OpenBLAS").unwrap().contains("0.3.34"));
        assert!(range("oneMKL").unwrap().contains("2026.1.0"));
        assert!(range("oneMKL").unwrap().contains("2020.4.304"));
        let accelerate = range("Accelerate").unwrap();
        assert!(accelerate.contains("27.0.1") && accelerate.contains("15.5"));
        assert!(!accelerate.contains("15.4") && !accelerate.contains("14.7.6"));
        // every library has a range, and a guide
        for (library, _) in CHECKED {
            assert!(range(library).is_some(), "{library}");
            assert!(GUIDES.iter().any(|g| g.library == *library), "{library}");
        }
    }

    #[test]
    fn the_versions_we_use_are_the_guides_checks() {
        // the newest any of the library's checks installed, from pip or conda-forge
        assert_eq!(uses("oneMKL").as_deref(), Some("2026.2.0"));
        // MUMPS's own release, built from its source, over conda-forge's package
        assert_eq!(uses("MUMPS").as_deref(), Some("5.9.1"));
        assert_eq!(uses("SuperLU").as_deref(), Some("7.0.1"));
        assert_eq!(uses("CUDA runtime").as_deref(), Some("13.4.92"));
        assert_eq!(uses("cuDSS").as_deref(), Some("1.0.0.35"));
        assert_eq!(uses("Accelerate").as_deref(), Some("27.0.1"));
        // every library has one
        for (library, _) in CHECKED {
            assert!(uses(library).is_some(), "{library}");
        }
        assert_eq!(
            installed("mumps-seq 5.8.2, mkl 2026.1.0").collect::<Vec<_>>(),
            [("mumps-seq", "5.8.2"), ("mkl", "2026.1.0")]
        );
        // the table the Dependencies page reads: a row per library, under its header
        let table = table();
        assert!(
            table.starts_with("| Library | We use | photonoxide-native accepts |\n|---|---|---|\n")
        );
        assert!(table.contains("| MUMPS | 5.9.1 | 5.4.1 to 5.9.1, "));
        assert_eq!(table.lines().count(), CHECKED.len() + 2);
    }

    #[test]
    fn accelerate_is_checked_on_the_runner_the_libraries_workflow_uses() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.github/workflows/libraries.yml"
        );
        let workflow = std::fs::read_to_string(path).unwrap();
        // a macOS image by its own name, or a preview by its Xcode's, both by the major release
        let major = ACCELERATE_CHECKED.split('.').next().unwrap();
        assert!(
            workflow.contains(&format!("os: macos-{major},"))
                || workflow.contains(&format!("os: xcode-{major},")),
            "libraries.yml runs Accelerate on another macOS than {ACCELERATE_CHECKED}"
        );
    }

    #[test]
    fn mumps_is_checked_built_from_the_source_the_libraries_workflow_builds() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.github/workflows/libraries.yml"
        );
        let workflow = std::fs::read_to_string(path).unwrap();
        let versions = workflow
            .lines()
            .find_map(|l| l.trim().strip_prefix("version: ["))
            .expect("libraries.yml has no MUMPS versions to build");
        assert!(
            versions.contains(&format!("'{MUMPS_CHECKED}'")),
            "libraries.yml doesn't build MUMPS {MUMPS_CHECKED}: {versions}"
        );
        // and photonoxide-native knows its structure
        assert!(mumps::releases().any(|r| r == MUMPS_CHECKED));
    }
}
