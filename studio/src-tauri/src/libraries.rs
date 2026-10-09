//! The external libraries photonoxide can use, as found on this machine: `photonoxide libraries`
//! and the studio's Libraries page.
//!
//! - **Found in a process of its own.** Probing loads each library and runs its backend's smoke
//!   test; a library that aborts can't be caught in-process, so the window asks a child
//!   (`photonoxide libraries --json`) and reads its report. A fresh process is also what makes
//!   "Detect again" see a library installed since the window opened.
//! - **The guides** ([`GUIDES`]): each library's licence, its official download, the variables
//!   and folders its loader searches, and the package-manager commands that install it where
//!   one is standard. The window shows a command to copy; only a winget command on Windows can
//!   be run from it, in a terminal of its own, after the user confirms having read the licence
//!   and the command ([`install`]). Elevation, if needed, is winget's to ask for. Nothing is
//!   redistributed with photonoxide.

use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use photonoxide::backend::{self, Listed, Threads};
use photonoxide_native::{Candidate, Discovery, Spec, Status, intel, nvidia};
use serde::Serialize;

/// A command that installs a library.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Install {
    /// Its id within the library's guide, for [`install`].
    pub id: &'static str,
    /// The package manager: `winget`, `conda-forge` or `pip`.
    pub manager: &'static str,
    /// The command, exactly as it is run or copied.
    pub command: &'static str,
    /// The systems it works on (`windows`, `linux`, `macos`).
    pub systems: &'static [&'static str],
    /// What to know before running it.
    pub note: &'static str,
    /// Where it was run on a clean machine and the library then found: none if it never was.
    pub checked: &'static [Checked],
}

/// One run of an install command on a clean machine (a GitHub runner, by the Libraries
/// workflow), after which photonoxide found the library.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Checked {
    /// The system: `windows`, `linux` or `macos`.
    pub system: &'static str,
    /// The runner's image.
    pub image: &'static str,
    /// The day, as `2026-10-09`.
    pub date: &'static str,
    /// What the command installed.
    pub installed: &'static str,
    /// What photonoxide then found, and what it couldn't check there.
    pub found: &'static str,
}

/// A library photonoxide has no backend for yet: how it installs, checked the same way, for
/// the issue that adds its backend.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Planned {
    /// The library's name.
    pub library: &'static str,
    /// What it is, and what photonoxide would use it for.
    pub about: &'static str,
    /// The issue that adds its backend.
    pub issue: u32,
    /// Its licence, by name.
    pub licence: &'static str,
    /// Its home page.
    pub home: &'static str,
    /// Package-manager commands that install it.
    pub installs: &'static [Install],
    /// The systems it has no package for here, with what to do instead.
    pub unsupported: &'static [(&'static str, &'static str)],
}

const DAY: &str = "2026-10-09";
const UBUNTU: &str = "ubuntu-24.04";
const UBUNTU_22: &str = "ubuntu-22.04";
const WINDOWS: &str = "windows-2025-vs2026";
const MACOS: &str = "macos-latest (Apple silicon)";

/// What to know about a library before installing it.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Guide {
    /// The library's name, as discovery reports it (a [`Spec`]'s name), or a backend's name for
    /// photonoxide's own and faer.
    pub library: &'static str,
    /// What it is, in a sentence.
    pub about: &'static str,
    /// What it provides photonoxide: `direct`, `iterative`, `eigen`, `dense`, or what other
    /// libraries need.
    pub provides: &'static [&'static str],
    /// The backends built on it, by name.
    pub backends: &'static [&'static str],
    /// The libraries it needs.
    pub needs: &'static [&'static str],
    /// Its licence, by name.
    pub licence: &'static str,
    /// The licence's text.
    pub licence_url: &'static str,
    /// The vendor's download page.
    pub download: &'static str,
    /// Package-manager commands that install it.
    pub installs: &'static [Install],
    /// The systems it has no build for, with the alternative.
    pub unsupported: &'static [(&'static str, &'static str)],
    /// Whether it is part of photonoxide, always present.
    pub built_in: bool,
}

const CUDA_UNSUPPORTED: &[(&str, &str)] = &[(
    "macos",
    "NVIDIA's libraries have no macOS build: photonoxide's own solvers run there.",
)];

/// The libraries photonoxide knows, its own first. Each install command says where it was run
/// on a clean machine ([`Checked`]): the Libraries workflow runs them again every week, and
/// fails when one stops working. winget's installers (gigabytes, elevation) aren't run there:
/// the workflow checks that the index still has the packages.
pub const GUIDES: &[Guide] = &[
    Guide {
        library: "photonoxide",
        about: "photonoxide's own multifrontal LU and L D Lᵀ, and its QMR: the default and the reference every other backend is checked against.",
        provides: &["direct", "iterative", "eigen", "dense"],
        backends: &["photonoxide"],
        needs: &[],
        licence: "MIT OR Apache-2.0",
        licence_url: "https://github.com/tachsin/photonoxide/blob/main/LICENSE-MIT",
        download: "https://github.com/tachsin/photonoxide",
        installs: &[],
        unsupported: &[],
        built_in: true,
    },
    Guide {
        library: "faer",
        about: "faer's sparse LU, pure Rust, built in as the baseline.",
        provides: &["direct"],
        backends: &["faer"],
        needs: &[],
        licence: "MIT",
        licence_url: "https://codeberg.org/sarah-quinones/faer/src/branch/main/LICENSE",
        download: "https://codeberg.org/sarah-quinones/faer",
        installs: &[],
        unsupported: &[],
        built_in: true,
    },
    Guide {
        library: "oneMKL",
        about: "Intel's oneAPI Math Kernel Library, through its single library mkl_rt: PARDISO, its sparse direct solver.",
        provides: &["direct"],
        backends: &["pardiso"],
        needs: &[],
        licence: "Intel Simplified Software License (October 2022)",
        licence_url: "https://cdrdv2-public.intel.com/749362/intel-simplified-license-software-october-2022.pdf",
        download: "https://www.intel.com/content/www/us/en/developer/tools/oneapi/onemkl-download.html",
        installs: &[
            Install {
                id: "winget",
                manager: "winget",
                command: "winget install --id Intel.oneMKL --exact",
                systems: &["windows"],
                note: "Intel's installer, into Program Files\\Intel\\oneAPI, where photonoxide looks.",
                checked: &[],
            },
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge mkl",
                systems: &["windows", "linux", "macos"],
                note: "Into the active conda environment: start photonoxide from it, so $CONDA_PREFIX points there. On macOS, Intel Macs only.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "mkl 2026.1.0",
                        found: "oneMKL 2026.1, and the pardiso backend after its smoke test",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "mkl 2026.1.0",
                        found: "oneMKL 2026.1, and the pardiso backend after its smoke test",
                    },
                ],
            },
            Install {
                id: "pip",
                manager: "pip",
                command: "pip install mkl",
                systems: &["windows", "linux"],
                note: "The wheel puts mkl_rt beside Python (Library\\bin on Windows, lib elsewhere), where photonoxide looks: the Python on PATH, or the active virtual environment's.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "mkl 2026.1.0",
                        found: "oneMKL 2026.1, and the pardiso backend after its smoke test",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "mkl 2026.1.0",
                        found: "oneMKL 2026.1, and the pardiso backend after its smoke test",
                    },
                ],
            },
            Install {
                id: "apt",
                manager: "apt",
                command: "sudo apt-get install libmkl-rt",
                systems: &["linux"],
                note: "Debian's and Ubuntu's package: MKL 2020.4, from 2020, where the other methods give this year's. photonoxide finds it in the system's library path.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "libmkl-rt 2020.4.304-4",
                        found: "oneMKL 2020.4, and the pardiso backend after its smoke test",
                    },
                    Checked {
                        system: "linux",
                        image: UBUNTU_22,
                        date: DAY,
                        installed: "libmkl-rt 2020.4.304-2ubuntu3",
                        found: "oneMKL 2020.4, and the pardiso backend after its smoke test",
                    },
                ],
            },
        ],
        unsupported: &[(
            "macos-aarch64",
            "oneMKL has no build for Apple silicon: photonoxide's own solvers run there.",
        )],
        built_in: false,
    },
    Guide {
        library: "CUDA runtime",
        about: "NVIDIA's CUDA runtime: the GPU backends' memory and devices. It needs NVIDIA's driver and a CUDA GPU.",
        provides: &["the GPU backends' runtime"],
        backends: &[],
        needs: &[],
        licence: "NVIDIA CUDA Toolkit End User License Agreement",
        licence_url: "https://docs.nvidia.com/cuda/eula/index.html",
        download: "https://developer.nvidia.com/cuda-downloads",
        installs: &[
            Install {
                id: "winget",
                manager: "winget",
                command: "winget install --id Nvidia.CUDA --exact",
                systems: &["windows"],
                note: "The whole CUDA toolkit (several GB), with cuSPARSE; $CUDA_PATH points at it.",
                checked: &[],
            },
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge cuda-cudart libcusparse",
                systems: &["windows", "linux"],
                note: "The runtime and cuSPARSE alone, into the active conda environment: start photonoxide from it.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "cuda-cudart 13.4.92, libcusparse 12.8.6.72",
                        found: "the CUDA runtime 13.4, which reports no GPU there",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "cuda-cudart 13.4.92, libcusparse 12.8.6.72",
                        found: "the CUDA runtime's file, which reports no GPU there",
                    },
                ],
            },
            Install {
                id: "pip",
                manager: "pip",
                command: "pip install nvidia-cuda-runtime nvidia-cusparse",
                systems: &["windows", "linux"],
                note: "NVIDIA's CUDA 13 wheels, found in site-packages/nvidia; nothing needs Python to load them.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72",
                        found: "the CUDA runtime 13.4, which reports no GPU there",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72",
                        found: "the CUDA runtime's file, which reports no GPU there",
                    },
                ],
            },
        ],
        unsupported: CUDA_UNSUPPORTED,
        built_in: false,
    },
    Guide {
        library: "cuSPARSE",
        about: "The CUDA toolkit's sparse kernels: photonoxide's QMR with ILU(0) run on the GPU.",
        provides: &["iterative"],
        backends: &["cusparse"],
        needs: &["CUDA runtime"],
        licence: "NVIDIA CUDA Toolkit End User License Agreement",
        licence_url: "https://docs.nvidia.com/cuda/eula/index.html",
        download: "https://developer.nvidia.com/cuda-downloads",
        installs: &[
            Install {
                id: "winget",
                manager: "winget",
                command: "winget install --id Nvidia.CUDA --exact",
                systems: &["windows"],
                note: "The whole CUDA toolkit (several GB), with the runtime; $CUDA_PATH points at it.",
                checked: &[],
            },
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge cuda-cudart libcusparse",
                systems: &["windows", "linux"],
                note: "Into the active conda environment: start photonoxide from it.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "cuda-cudart 13.4.92, libcusparse 12.8.6.72",
                        found: "cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "cuda-cudart 13.4.92, libcusparse 12.8.6.72",
                        found: "cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                ],
            },
            Install {
                id: "pip",
                manager: "pip",
                command: "pip install nvidia-cuda-runtime nvidia-cusparse",
                systems: &["windows", "linux"],
                note: "NVIDIA's CUDA 13 wheels, found in site-packages/nvidia.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72",
                        found: "cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72",
                        found: "cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                ],
            },
        ],
        unsupported: CUDA_UNSUPPORTED,
        built_in: false,
    },
    Guide {
        library: "cuDSS",
        about: "NVIDIA's sparse direct solver on the GPU.",
        provides: &["direct"],
        backends: &["cudss"],
        needs: &["CUDA runtime"],
        licence: "NVIDIA cuDSS Software License Agreement",
        licence_url: "https://docs.nvidia.com/cuda/cudss/license.html",
        download: "https://developer.nvidia.com/cudss-downloads",
        installs: &[
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge libcudss",
                systems: &["windows", "linux"],
                note: "Into the active conda environment: start photonoxide from it.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "libcudss 0.8.0.10",
                        found: "cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "libcudss 0.8.0.10",
                        found: "cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                ],
            },
            Install {
                id: "pip",
                manager: "pip",
                command: "pip install nvidia-cudss-cu13",
                systems: &["windows", "linux"],
                note: "The CUDA 13 build (nvidia-cudss-cu12 for CUDA 12), found in site-packages/nvidia.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "nvidia-cudss-cu13 0.8.0.10",
                        found: "cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "nvidia-cudss-cu13 0.8.0.10",
                        found: "cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run",
                    },
                ],
            },
        ],
        unsupported: CUDA_UNSUPPORTED,
        built_in: false,
    },
];

const HAS_LIBRARY: &str =
    "the package's library is there; photonoxide has no backend to load it with yet";

/// The libraries the plan names that photonoxide has no backend for yet (docs/plans/backends.md):
/// how each installs, checked on clean machines, for the issue that adds its backend.
pub const PLANNED: &[Planned] = &[
    Planned {
        library: "MUMPS",
        about: "A multifrontal sparse direct solver, with block low-rank compression: a direct backend. The sequential build.",
        issue: 176,
        licence: "CeCILL-C",
        home: "https://mumps-solver.org/",
        installs: &[
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge mumps-seq",
                systems: &["windows", "linux", "macos"],
                note: "The sequential build, into the active conda environment. On Windows it brings conda-forge's mkl with it, and so oneMKL's PARDISO too.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "mumps-seq 5.8.2",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "mumps-seq 5.8.2, mkl 2026.1.0",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "macos",
                        image: MACOS,
                        date: DAY,
                        installed: "mumps-seq 5.8.2",
                        found: HAS_LIBRARY,
                    },
                ],
            },
            Install {
                id: "apt",
                manager: "apt",
                command: "sudo apt-get install libmumps-seq-dev",
                systems: &["linux"],
                note: "Debian's and Ubuntu's package, older than conda-forge's: 5.6.2 on Ubuntu 24.04, 5.4.1 on 22.04.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "libmumps-seq-dev 5.6.2",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "linux",
                        image: UBUNTU_22,
                        date: DAY,
                        installed: "libmumps-seq-dev 5.4.1",
                        found: HAS_LIBRARY,
                    },
                ],
            },
        ],
        unsupported: &[],
    },
    Planned {
        library: "SuperLU",
        about: "A supernodal sparse direct solver with partial pivoting: a direct backend.",
        issue: 177,
        licence: "BSD-3-Clause",
        home: "https://portal.nersc.gov/project/sparse/superlu/",
        installs: &[
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge superlu",
                systems: &["linux", "macos"],
                note: "Into the active conda environment.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "superlu 7.0.1",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "macos",
                        image: MACOS,
                        date: DAY,
                        installed: "superlu 7.0.1",
                        found: HAS_LIBRARY,
                    },
                ],
            },
            Install {
                id: "apt",
                manager: "apt",
                command: "sudo apt-get install libsuperlu-dev",
                systems: &["linux"],
                note: "Debian's and Ubuntu's package, older than conda-forge's: 6.0.1 on Ubuntu 24.04, 5.3.0 on 22.04.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "libsuperlu-dev 6.0.1",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "linux",
                        image: UBUNTU_22,
                        date: DAY,
                        installed: "libsuperlu-dev 5.3.0",
                        found: HAS_LIBRARY,
                    },
                ],
            },
        ],
        unsupported: &[(
            "windows",
            "conda-forge has no superlu for Windows (the install fails with PackagesNotFoundInChannelsError, 2026-10-09). vcpkg builds it from source, which wasn't run here.",
        )],
    },
    Planned {
        library: "OpenBLAS",
        about: "An open BLAS and LAPACK: dense kernels for the multifrontal fronts, where no vendor's library is installed.",
        issue: 186,
        licence: "BSD-3-Clause",
        home: "https://www.openblas.net/",
        installs: &[
            Install {
                id: "conda",
                manager: "conda-forge",
                command: "conda install -c conda-forge openblas",
                systems: &["windows", "linux", "macos"],
                note: "Into the active conda environment.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "openblas 0.3.34",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "windows",
                        image: WINDOWS,
                        date: DAY,
                        installed: "openblas 0.3.34",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "macos",
                        image: MACOS,
                        date: DAY,
                        installed: "openblas 0.3.34",
                        found: HAS_LIBRARY,
                    },
                ],
            },
            Install {
                id: "apt",
                manager: "apt",
                command: "sudo apt-get install libopenblas-dev",
                systems: &["linux"],
                note: "Debian's and Ubuntu's package: 0.3.26 on Ubuntu 24.04, 0.3.20 on 22.04.",
                checked: &[
                    Checked {
                        system: "linux",
                        image: UBUNTU,
                        date: DAY,
                        installed: "libopenblas-dev 0.3.26",
                        found: HAS_LIBRARY,
                    },
                    Checked {
                        system: "linux",
                        image: UBUNTU_22,
                        date: DAY,
                        installed: "libopenblas-dev 0.3.20",
                        found: HAS_LIBRARY,
                    },
                ],
            },
        ],
        unsupported: &[],
    },
    Planned {
        library: "AMD AOCL",
        about: "AMD's BLIS, libFLAME and AOCL-Sparse: dense kernels and iterative solvers on AMD processors.",
        issue: 186,
        licence: "BSD-3-Clause and MIT, by component (to confirm in #186)",
        home: "https://www.amd.com/en/developer/aocl.html",
        installs: &[],
        unsupported: &[
            (
                "windows",
                "AMD's installer, from its page: no package manager has it, and it wasn't run here.",
            ),
            (
                "linux",
                "AMD's packages or Spack, from its page: not run here.",
            ),
            ("macos", "AMD has no macOS build."),
        ],
    },
    Planned {
        library: "Apple Accelerate",
        about: "macOS's own BLAS, LAPACK and sparse solvers: dense kernels and a direct backend on a Mac.",
        issue: 187,
        licence: "part of macOS",
        home: "https://developer.apple.com/documentation/accelerate",
        installs: &[],
        unsupported: &[
            (
                "macos",
                "Built in: nothing to install. Which macOS first has its complex sparse solvers is for #187 to confirm.",
            ),
            ("windows", "A macOS framework."),
            ("linux", "A macOS framework."),
        ],
    },
    Planned {
        library: "Arm Performance Libraries",
        about: "Arm's BLAS and LAPACK: dense kernels on Arm processors.",
        issue: 186,
        licence: "Arm's terms, free of charge (to confirm in #186)",
        home: "https://developer.arm.com/",
        installs: &[],
        unsupported: &[
            (
                "windows",
                "Arm's installer, for Windows on Arm: not run here.",
            ),
            ("linux", "Arm's packages, for AArch64: not run here."),
            ("macos", "Arm's installer, for Apple silicon: not run here."),
        ],
    },
];

/// The discovery spec of a library photonoxide looks for.
fn spec(library: &str) -> Option<&'static Spec> {
    [
        &nvidia::CUDA_RUNTIME,
        &nvidia::CUSPARSE,
        &nvidia::CUDSS,
        &intel::MKL,
    ]
    .into_iter()
    .find(|s| s.name == library)
}

/// A candidate file, as the page shows it.
#[derive(Debug, Serialize)]
pub struct CandidateView {
    pub path: String,
    pub source: String,
    /// `used`, `not tried` or `failed`.
    pub status: &'static str,
    pub reason: Option<String>,
}

/// One library as found.
#[derive(Debug, Serialize)]
pub struct LibraryView {
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
    /// The file loaded, and where it was found.
    pub path: Option<String>,
    pub source: Option<String>,
    /// Why it isn't used.
    pub reason: Option<String>,
    pub candidates: Vec<CandidateView>,
    /// What else it says: its threads, the GPUs.
    pub details: Vec<String>,
    /// The file names looked for.
    pub files: Vec<String>,
    /// The environment variables that may point at it, photonoxide's own first, and their
    /// values here.
    pub variables: Vec<(String, Option<String>)>,
    /// Its install folders, as looked for on this machine.
    pub folders: Vec<String>,
    /// Whether it is looked for in Python wheels' folders, and which.
    pub wheel: Option<String>,
}

/// One backend, as photonoxide's registry lists it.
#[derive(Debug, PartialEq, Serialize)]
pub struct BackendView {
    pub name: String,
    /// `direct` or `iterative`.
    pub kind: &'static str,
    pub available: bool,
    pub version: Option<String>,
    pub licence: Option<String>,
    pub deterministic: Option<bool>,
    pub threads: Option<String>,
    pub symmetric: Option<bool>,
    pub transpose: Option<bool>,
    /// Why it can't be had: its library isn't found, or its smoke test failed.
    pub unavailable: Option<String>,
}

/// What `photonoxide libraries --json` reports.
#[derive(Debug, Serialize)]
pub struct Report {
    /// `windows-x86_64` and the like.
    pub platform: String,
    pub libraries: Vec<LibraryView>,
    pub backends: Vec<BackendView>,
    pub guides: &'static [Guide],
    /// `$CONDA_PREFIX`, if the program was started from a conda environment.
    pub conda: Option<String>,
}

fn candidate(c: &Candidate) -> CandidateView {
    CandidateView {
        path: c.path.display().to_string(),
        source: c.source.to_string(),
        status: match c.status {
            Status::Used => "used",
            Status::NotTried => "not tried",
            _ => "failed",
        },
        reason: match &c.status {
            Status::Failed(why) => Some(why.clone()),
            _ => None,
        },
    }
}

fn library(d: &Discovery, version: Option<&str>, details: &[String]) -> LibraryView {
    let used = d.used();
    let spec = spec(d.library);
    LibraryView {
        name: d.library.to_owned(),
        found: used.is_some(),
        version: version.map(str::to_owned),
        path: used.map(|c| c.path.display().to_string()),
        source: used.map(|c| c.source.to_string()),
        reason: used.is_none().then(|| d.reason()),
        candidates: d.candidates.iter().map(candidate).collect(),
        details: details.to_vec(),
        files: spec.map_or_else(Vec::new, |s| {
            s.files.iter().map(|f| (*f).to_owned()).collect()
        }),
        variables: spec.map_or_else(Vec::new, |s| {
            s.variables
                .iter()
                .map(|v| {
                    (
                        (*v).to_owned(),
                        std::env::var_os(v).map(|x| x.to_string_lossy().into_owned()),
                    )
                })
                .collect()
        }),
        folders: spec.map_or_else(Vec::new, |s| {
            (s.install)()
                .iter()
                .map(|p| p.display().to_string())
                .collect()
        }),
        wheel: spec
            .and_then(|s| s.wheel)
            .map(|w| format!("site-packages/{w}")),
    }
}

fn backend(listed: &Listed, kind: &'static str) -> BackendView {
    let c = listed.capabilities.as_ref();
    BackendView {
        name: listed.name.clone(),
        kind,
        available: c.is_some(),
        version: c.map(|c| c.version.clone()),
        licence: c.map(|c| c.licence.clone()),
        deterministic: c.map(|c| c.deterministic),
        threads: c.map(|c| match &c.threads {
            Threads::One => "one".to_owned(),
            Threads::Rayon => "rayon's (RAYON_NUM_THREADS)".to_owned(),
            Threads::Library(how) => how.clone(),
            other => format!("{other:?}"),
        }),
        symmetric: c.map(|c| c.symmetric),
        transpose: c.map(|c| c.transpose),
        unavailable: listed.unavailable.clone(),
    }
}

/// The report of what was probed and what the registry lists.
pub fn report(libraries: Vec<LibraryView>, direct: &[Listed], iterative: &[Listed]) -> Report {
    Report {
        platform: platform(),
        libraries,
        backends: direct
            .iter()
            .map(|l| backend(l, "direct"))
            .chain(iterative.iter().map(|l| backend(l, "iterative")))
            .collect(),
        guides: GUIDES,
        conda: std::env::var_os("CONDA_PREFIX").map(|p| p.to_string_lossy().into_owned()),
    }
}

fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// The systems a page's table has a column for.
const SYSTEMS: [(&str, &str); 3] = [
    ("windows", "Windows"),
    ("linux", "Linux"),
    ("macos", "macOS"),
];

/// A table cell: the managers that install on `system`, each marked checked or not, or what
/// the library's guide says of a system it has no package for.
fn cell(system: &str, installs: &[Install], unsupported: &[(&str, &str)]) -> String {
    let methods: Vec<String> = installs
        .iter()
        .filter(|i| i.systems.contains(&system))
        .map(|i| {
            let checked = i.checked.iter().any(|c| c.system == system);
            format!("{}{}", i.manager, if checked { " ✓" } else { "" })
        })
        .collect();
    if !methods.is_empty() {
        return methods.join(", ");
    }
    if unsupported
        .iter()
        .any(|(s, _)| s.split('-').next() == Some(system))
    {
        "see below".into()
    } else {
        "the vendor's installer".into()
    }
}

fn write_installs(out: &mut String, installs: &[Install], unsupported: &[(&str, &str)]) {
    use std::fmt::Write as _;
    for i in installs {
        let systems: Vec<&str> = SYSTEMS
            .iter()
            .filter(|(s, _)| i.systems.contains(s))
            .map(|(_, name)| *name)
            .collect();
        let _ = writeln!(out, "- **{}** ({}):\n", i.manager, systems.join(", "));
        let _ = writeln!(out, "  ```sh\n  {}\n  ```\n", i.command);
        let _ = writeln!(out, "  {}\n", i.note);
        if i.checked.is_empty() && i.manager == "winget" {
            let _ = writeln!(
                out,
                "  Not run on a clean machine: the installer is several gigabytes and asks for \
                 elevation. The workflow checks that winget's index still has the package.\n"
            );
        } else if i.checked.is_empty() {
            let _ = writeln!(out, "  Not run on a clean machine.\n");
        }
        for c in i.checked {
            let _ = writeln!(
                out,
                "  - Checked on {} ({}), {}: installed {}; found {}.",
                SYSTEMS
                    .iter()
                    .find(|(s, _)| *s == c.system)
                    .map_or(c.system, |(_, n)| n),
                c.image,
                c.date,
                c.installed,
                c.found
            );
        }
        if !i.checked.is_empty() {
            out.push('\n');
        }
    }
    for (system, instead) in unsupported {
        let _ = writeln!(out, "- **{system}:** {instead}");
    }
    if !unsupported.is_empty() {
        out.push('\n');
    }
}

/// docs/libraries.md: every library's install methods per system, and where each was checked.
pub fn markdown() -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "# Installing the external libraries\n\n\
         photonoxide needs none of these: its own solvers are built in, and the default build, CI \
         and the released app work with no library installed. An external library is optional, \
         installed by you under its own licence, found and loaded at run time, and never \
         redistributed with photonoxide ([the plan](plans/backends.md)). The studio's Libraries \
         page shows the same guides beside what it finds on your machine, and \
         `photonoxide libraries` prints it.\n\n\
         This page is written by `photonoxide libraries --write docs/libraries.md` from the \
         guides in the program; a test fails when it is out of date.\n\n\
         **Checked** means the command was run on a clean machine, one of GitHub's runners, and \
         photonoxide then found the library: the Libraries workflow \
         (`.github/workflows/libraries.yml`) does that again every week and on every change to \
         the guides, and fails when a method stops working. What it can't check is said with \
         each method: the runners have no GPU, so NVIDIA's libraries are found and loaded there \
         but their backends' smoke tests don't run; winget's installers are several gigabytes \
         and ask for elevation, so only the index is checked for the packages; and there is no \
         Intel Mac among the runners.\n\n",
    );
    out.push_str("## Which library installs where\n\n");
    out.push_str("| Library | Backends | Windows | Linux | macOS |\n|---|---|---|---|---|\n");
    for g in GUIDES.iter().filter(|g| !g.built_in) {
        let _ = writeln!(
            out,
            "| [{}](#{}) | {} | {} | {} | {} |",
            g.library,
            anchor(g.library),
            if g.backends.is_empty() {
                "—".into()
            } else {
                g.backends
                    .iter()
                    .map(|b| format!("`{b}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            cell("windows", g.installs, g.unsupported),
            cell("linux", g.installs, g.unsupported),
            cell("macos", g.installs, g.unsupported),
        );
    }
    for p in PLANNED {
        let _ = writeln!(
            out,
            "| [{}](#{}) | none yet (#{}) | {} | {} | {} |",
            p.library,
            anchor(p.library),
            p.issue,
            cell("windows", p.installs, p.unsupported),
            cell("linux", p.installs, p.unsupported),
            cell("macos", p.installs, p.unsupported),
        );
    }
    out.push_str("\n✓: run on a clean machine and found. A manager without it is listed by its package index alone.\n\n");
    out.push_str("## Where photonoxide looks\n\n");
    out.push_str(
        "For each library, in this order, and the first file that loads and passes its check is used:\n\n\
         1. the library's own variable, naming the file or its folder (`PHOTONOXIDE_MKL`, say), then the vendor's (`MKLROOT`, `CUDA_PATH`);\n\
         2. the active conda environment (`CONDA_PREFIX`): start photonoxide from it;\n\
         3. the vendor's default install folders;\n\
         4. Python's package folders, for the vendors' wheels: the active virtual environment's, the Python on `PATH`, the user's and the system's. Nothing needs Python to load them;\n\
         5. the system's library paths.\n\n",
    );
    out.push_str("## The libraries photonoxide has a backend for\n\n");
    for g in GUIDES.iter().filter(|g| !g.built_in) {
        let _ = writeln!(out, "### {}\n", g.library);
        let _ = writeln!(out, "{}\n", g.about);
        let _ = writeln!(
            out,
            "- **Licence:** [{}]({}). You accept it by installing.",
            g.licence, g.licence_url
        );
        let _ = writeln!(out, "- **The vendor's download:** <{}>", g.download);
        if !g.backends.is_empty() {
            let names: Vec<String> = g.backends.iter().map(|b| format!("`{b}`")).collect();
            let _ = writeln!(out, "- **Backends:** {}.", names.join(", "));
        }
        if !g.needs.is_empty() {
            let _ = writeln!(out, "- **Needs:** {}.", g.needs.join(", "));
        }
        out.push('\n');
        write_installs(&mut out, g.installs, g.unsupported);
    }
    out.push_str("## The libraries without a backend yet\n\n");
    out.push_str("How each installs, for the issue that adds its backend. photonoxide doesn't look for them yet.\n\n");
    for p in PLANNED {
        let _ = writeln!(out, "### {}\n", p.library);
        let _ = writeln!(
            out,
            "{} Its backend: [#{}](https://github.com/tachsin/photonoxide/issues/{}).\n",
            p.about, p.issue, p.issue
        );
        let _ = writeln!(out, "- **Licence:** {}.", p.licence);
        let _ = writeln!(out, "- **Home:** <{}>\n", p.home);
        write_installs(&mut out, p.installs, p.unsupported);
    }
    out.push_str("## Traps\n\n");
    out.push_str(
        "- **Debian's and Ubuntu's oneMKL is MKL 2020.4.** `apt` installs a library six years older than Intel's, conda-forge's or pip's. photonoxide finds it and its PARDISO passes the smoke test; prefer the others for a current library.\n\
         - **A conda environment must be the active one** when photonoxide starts: it is found by `CONDA_PREFIX`.\n\
         - **pip installs into the Python that runs it.** photonoxide looks in the active virtual environment and in the Python on `PATH`; a wheel installed into another Python isn't found unless the library's variable points at it.\n\
         - **NVIDIA's wheels for CUDA 13 keep their libraries in two folders** (`nvidia/cu13/bin` and `bin/x86_64` on Windows). photonoxide adds the second to the folders Windows searches when cuDSS needs it.\n\
         - **conda-forge's mumps-seq on Windows brings oneMKL with it,** and so the `pardiso` backend.\n\
         - **Every NVIDIA library needs NVIDIA's driver and a CUDA GPU** to do anything: found and loaded isn't yet a backend.\n",
    );
    out
}

/// A heading's anchor, as GitHub makes it.
fn anchor(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => Some(c),
            ' ' => Some('-'),
            _ => None,
        })
        .collect()
}

/// `photonoxide libraries [--json | --write <file>]`: finds every library, registers their
/// backends after their smoke tests, and prints what was found; or writes the install guides'
/// page.
pub fn run(args: &[String]) -> ExitCode {
    if let [flag, file] = args
        && flag == "--write"
    {
        return match std::fs::write(file, markdown()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => crate::fail(format!("{file}: {e}")),
        };
    }
    let json = match args {
        [] => false,
        [flag] if flag == "--json" => true,
        _ => return crate::usage(),
    };
    let probes = photonoxide_native::register_all();
    let r = report(
        probes
            .iter()
            .map(|p| library(&p.discovery, p.version.as_deref(), &p.details))
            .collect(),
        &backend::direct_solvers().unwrap_or_default(),
        &backend::iterative_solvers().unwrap_or_default(),
    );
    if json {
        match serde_json::to_string(&r) {
            Ok(text) => println!("{text}"),
            Err(e) => return crate::fail(e),
        }
    } else {
        for probe in &probes {
            println!("{probe}");
        }
        println!();
        for b in &r.backends {
            match (&b.version, &b.unavailable) {
                (Some(v), _) => println!("{} ({}): {v}, available", b.name, b.kind),
                (None, Some(why)) => println!("{} ({}): {why}", b.name, b.kind),
                _ => println!("{} ({})", b.name, b.kind),
            }
        }
    }
    ExitCode::SUCCESS
}

/// How long probing may take before it is given up: loading CUDA the first time can take a
/// while.
const PROBE_TIMEOUT: Duration = Duration::from_secs(120);

/// Asks a child process (`photonoxide libraries --json`) what is found here.
fn probe_in_a_child() -> Result<serde_json::Value, String> {
    let exe = std::env::current_exe().map_err(|e| format!("can't find the program: {e}"))?;
    let mut command = Command::new(exe);
    command
        .args(["libraries", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("can't start the probe: {e}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => break,
            None if start.elapsed() > PROBE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "finding the libraries took more than {} s and was stopped",
                    PROBE_TIMEOUT.as_secs()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr.lines().rev().find(|l| !l.trim().is_empty());
        return Err(format!(
            "finding the libraries ended the probe ({}): {}; a library that aborts on loading \
             can't be used",
            output.status,
            last.unwrap_or("no message")
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .ok_or("the probe printed no report")?;
    serde_json::from_str(line).map_err(|e| format!("the probe's report doesn't read: {e}"))
}

/// Every library and backend as found now, in a fresh process.
#[tauri::command(async)]
pub fn libraries() -> Result<serde_json::Value, String> {
    probe_in_a_child()
}

/// The install `id` of `library`'s guide, if it can be run on this system.
fn runnable(library: &str, id: &str) -> Result<&'static Install, String> {
    let guide = GUIDES
        .iter()
        .find(|g| g.library == library)
        .ok_or_else(|| format!("no library {library}"))?;
    let install = guide
        .installs
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| format!("{library} has no install {id}"))?;
    if install.manager != "winget" || !cfg!(windows) {
        return Err(format!(
            "{} is copied and run by you, in the environment it installs into",
            install.command
        ));
    }
    Ok(install)
}

/// Runs `library`'s install `id` in a terminal of its own, which stays open to show what the
/// package manager did. The window asks the user first, showing the licence and the command;
/// only the catalogue's winget commands run, on Windows, never a command the window sends.
#[tauri::command]
pub fn install(library: String, id: String) -> Result<String, String> {
    let install = runnable(&library, &id)?;
    let mut command = Command::new("cmd");
    // start opens a new console, whose cmd /k runs the command and stays open
    command.args(["/c", "start", "photonoxide: installing", "cmd", "/k"]);
    command.args(install.command.split(' '));
    command
        .spawn()
        .map_err(|e| format!("can't open a terminal: {e}"))?;
    Ok(install.command.to_owned())
}

/// No console window for a child of the window, on Windows.
pub(crate) fn no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::backend::Capabilities;
    use photonoxide_native::Source;
    use std::path::PathBuf;

    #[test]
    fn every_library_probed_has_a_guide_and_its_spec() {
        // what register_all reports on any machine, found or not
        for name in [
            nvidia::CUDA_RUNTIME.name,
            nvidia::CUSPARSE.name,
            nvidia::CUDSS.name,
            intel::MKL.name,
        ] {
            assert!(spec(name).is_some(), "{name}");
            assert!(GUIDES.iter().any(|g| g.library == name), "{name}");
        }
    }

    #[test]
    fn guides_are_whole() {
        let mut seen = std::collections::HashSet::new();
        for g in GUIDES {
            assert!(seen.insert(g.library), "{} twice", g.library);
            assert!(g.licence_url.starts_with("https://"), "{}", g.library);
            assert!(g.download.starts_with("https://"), "{}", g.library);
            assert!(!g.licence.is_empty() && !g.about.is_empty());
            assert_eq!(g.built_in, g.installs.is_empty(), "{}", g.library);
            for need in g.needs {
                assert!(GUIDES.iter().any(|n| n.library == *need), "{need}");
            }
            let mut ids = std::collections::HashSet::new();
            for i in g.installs {
                assert!(ids.insert(i.id), "{}: {} twice", g.library, i.id);
                assert!(!i.systems.is_empty());
                for s in i.systems {
                    assert!(["windows", "linux", "macos"].contains(s), "{s}");
                }
                // what the command line runs is what the page shows
                assert!(!i.command.contains("  ") && !i.command.contains(['&', '|', ';', '>']));
            }
        }
    }

    #[test]
    fn the_install_guides_page_is_the_one_the_program_writes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/libraries.md");
        let written = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        assert!(
            written == markdown(),
            "docs/libraries.md is out of date: photonoxide libraries --write docs/libraries.md"
        );
    }

    #[test]
    fn every_check_is_of_a_system_its_command_installs_on() {
        let installs = GUIDES
            .iter()
            .flat_map(|g| g.installs.iter().map(move |i| (g.library, i)))
            .chain(
                PLANNED
                    .iter()
                    .flat_map(|p| p.installs.iter().map(move |i| (p.library, i))),
            );
        let mut checked = 0;
        for (library, i) in installs {
            for c in i.checked {
                assert!(
                    i.systems.contains(&c.system),
                    "{library} {}: {}",
                    i.id,
                    c.system
                );
                assert_eq!(c.date.len(), 10, "{library} {}", i.id);
                assert!(!c.installed.is_empty() && !c.found.is_empty() && !c.image.is_empty());
                checked += 1;
            }
            assert!(!i.command.contains("  ") && !i.command.contains(['&', '|', ';', '>']));
        }
        assert!(checked >= 30, "{checked}");
        // every planned library names its issue, and has a method or says what to do instead
        for p in PLANNED {
            assert!(
                p.issue > 0 && p.home.starts_with("https://"),
                "{}",
                p.library
            );
            assert!(
                !p.installs.is_empty() || !p.unsupported.is_empty(),
                "{}",
                p.library
            );
            assert!(
                GUIDES.iter().all(|g| g.library != p.library),
                "{}",
                p.library
            );
        }
    }

    #[test]
    fn only_the_catalogues_winget_commands_run_and_only_on_windows() {
        assert!(runnable("oneMKL", "nothing").is_err());
        assert!(runnable("nothing", "winget").is_err());
        // conda and pip install into an environment the window can't know
        assert!(runnable("oneMKL", "conda").is_err());
        assert!(runnable("cuDSS", "pip").is_err());
        let winget = runnable("oneMKL", "winget");
        if cfg!(windows) {
            assert_eq!(
                winget.unwrap().command,
                "winget install --id Intel.oneMKL --exact"
            );
        } else {
            assert!(winget.is_err());
        }
    }

    #[test]
    fn a_report_says_what_was_found_and_why_not() {
        let mkl = library(
            &Discovery {
                library: "oneMKL",
                candidates: vec![
                    Candidate {
                        path: PathBuf::from("a/mkl_rt.3.dll"),
                        source: Source::Conda,
                        status: Status::Failed("a broken copy".into()),
                    },
                    Candidate {
                        path: PathBuf::from("b/mkl_rt.3.dll"),
                        source: Source::InstallFolder,
                        status: Status::Used,
                    },
                ],
            },
            Some("2026.1"),
            &["8 threads".into()],
        );
        let missing = library(
            &Discovery {
                library: "cuDSS",
                candidates: Vec::new(),
            },
            None,
            &[],
        );
        let direct = vec![
            Listed {
                name: "photonoxide".into(),
                capabilities: Some(Capabilities::new("photonoxide", "0.4", "MIT OR Apache-2.0")),
                unavailable: None,
            },
            Listed {
                name: "cudss".into(),
                capabilities: None,
                unavailable: Some("cuDSS wasn't found".into()),
            },
        ];
        let r = report(vec![mkl, missing], &direct, &[]);
        let mkl = &r.libraries[0];
        assert!(mkl.found);
        assert_eq!(mkl.version.as_deref(), Some("2026.1"));
        assert_eq!(
            mkl.path.as_deref(),
            Some(
                PathBuf::from("b/mkl_rt.3.dll")
                    .display()
                    .to_string()
                    .as_str()
            )
        );
        assert_eq!(mkl.source.as_deref(), Some("the install folder"));
        assert_eq!(mkl.candidates[0].status, "failed");
        assert_eq!(mkl.candidates[0].reason.as_deref(), Some("a broken copy"));
        assert_eq!(mkl.variables[0].0, "PHOTONOXIDE_MKL");
        assert!(mkl.reason.is_none());
        let cudss = &r.libraries[1];
        assert!(!cudss.found);
        assert_eq!(cudss.reason.as_deref(), Some("cuDSS wasn't found"));
        assert_eq!(cudss.wheel.as_deref(), Some("site-packages/nvidia"));
        assert_eq!(r.backends.len(), 2);
        assert!(r.backends[0].available && r.backends[0].kind == "direct");
        assert_eq!(
            r.backends[1].unavailable.as_deref(),
            Some("cuDSS wasn't found")
        );
        // and it serializes as the page reads it
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["libraries"][0]["candidates"][1]["status"], "used");
        assert_eq!(json["guides"][2]["installs"][0]["manager"], "winget");
    }
}
