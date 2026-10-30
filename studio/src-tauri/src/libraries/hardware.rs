//! What this machine has, for the most performance (#564): the setup's Advanced path and the
//! first start's performance step. The CPU (its vendor, model, cores and the instruction sets
//! the solvers use), an NVIDIA GPU and its driver, and the system; and the external libraries
//! suggested for them, each with the install that runs here ([`super::library_install`]).
//!
//! The suggestions go by the hardware, and say so; where this machine's benchmark records
//! (`photonoxide bench`) measured a library against photonoxide's own, they're ranked and said
//! by what was measured ([`super::measured`]), the hardware's order the fallback.

use serde::Serialize;

use super::measured::Measured;
use super::topology::{self, Topology};

#[derive(Clone, Debug, Serialize)]
pub struct Hardware {
    pub os: &'static str,
    pub arch: &'static str,
    pub cpu: Cpu,
    pub gpu: Option<Gpu>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Cpu {
    /// `GenuineIntel`, `AuthenticAMD`, `Apple`, or what the CPU says.
    pub vendor: String,
    pub brand: String,
    /// The threads the program can run at once.
    pub threads: usize,
    /// The instruction sets the solvers use where they're there: AVX2, FMA, AVX-512.
    pub features: Vec<&'static str>,
    /// Its cores, caches and NUMA nodes.
    pub topology: Topology,
}

#[derive(Clone, Debug, Serialize)]
pub struct Gpu {
    pub name: String,
    pub driver: String,
}

/// A library suggested for this machine.
#[derive(Clone, Debug, Serialize)]
pub struct Suggestion {
    pub library: &'static str,
    /// Why, for this machine.
    pub why: &'static str,
    /// What suggested it: `hardware`, or `records` for a library the hardware didn't suggest
    /// that this machine's benchmark records measured faster than photonoxide's own.
    pub by: &'static str,
    /// Its backends, by name: what the records measure of it.
    pub backends: &'static [&'static str],
    /// The install that runs here, by its id in the library's guide; none when its vendor's
    /// download is the only way (then `download`).
    pub install: Option<&'static str>,
    pub command: Option<&'static str>,
    pub download: &'static str,
    /// Part of the system (Accelerate, on macOS): nothing to install.
    pub system: bool,
    /// What this machine's benchmark records measured of it, if they did.
    pub measured: Option<Measured>,
}

/// This machine.
pub fn here() -> Hardware {
    Hardware {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        cpu: cpu(),
        gpu: nvidia(),
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn cpu() -> Cpu {
    let id = raw_cpuid::CpuId::new();
    let vendor = id
        .get_vendor_info()
        .map_or_else(String::new, |v| v.as_str().to_owned());
    let brand = id
        .get_processor_brand_string()
        .map_or_else(String::new, |b| b.as_str().trim().to_owned());
    let mut features = Vec::new();
    if std::arch::is_x86_feature_detected!("avx2") {
        features.push("AVX2");
    }
    if std::arch::is_x86_feature_detected!("fma") {
        features.push("FMA");
    }
    if std::arch::is_x86_feature_detected!("avx512f") {
        features.push("AVX-512");
    }
    Cpu {
        vendor,
        brand,
        threads: threads(),
        features,
        topology: topology::here(),
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn cpu() -> Cpu {
    // Arm: the brand as the system names it; NEON is always there on AArch64
    let brand = if cfg!(target_os = "macos") {
        std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default()
    } else {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|t| {
                t.lines()
                    .find(|l| l.starts_with("model name") || l.starts_with("Model"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().to_owned())
            })
            .unwrap_or_default()
    };
    Cpu {
        vendor: if cfg!(target_os = "macos") {
            "Apple".into()
        } else {
            "Arm".into()
        },
        brand,
        threads: threads(),
        features: vec!["NEON"],
        topology: topology::here(),
    }
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// An NVIDIA GPU, as its driver's `nvidia-smi` reports it: none without the driver.
fn nvidia() -> Option<Gpu> {
    let mut command = std::process::Command::new("nvidia-smi");
    command.args(["--query-gpu=name,driver_version", "--format=csv,noheader"]);
    super::no_window(&mut command);
    let out = command.output().ok().filter(|o| o.status.success())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (name, driver) = text.lines().next()?.split_once(',')?;
    Some(Gpu {
        name: name.trim().to_owned(),
        driver: driver.trim().to_owned(),
    })
}

/// The libraries suggested for `hardware`, each with its install here when one runs: oneMKL
/// on an Intel CPU, AMD's AOCL on an AMD one, Arm's libraries on Arm (Accelerate, built in, on
/// a Mac), cuDSS with the CUDA runtime and cuSPARSE beside an NVIDIA GPU, and MUMPS for the
/// direct solves on any.
pub fn suggest(hardware: &Hardware) -> Vec<Suggestion> {
    let mut wanted: Vec<(&'static str, &'static str)> = Vec::new();
    let vendor = hardware.cpu.vendor.as_str();
    if hardware.os == "macos" {
        wanted.push((
            "Accelerate",
            "Apple's BLAS, LAPACK and sparse solvers, built into macOS and tuned for its chips",
        ));
    } else if vendor == "GenuineIntel" {
        wanted.push((
            "oneMKL",
            "Intel's BLAS and LAPACK and its PARDISO direct solver, tuned for this Intel CPU",
        ));
    } else if vendor == "AuthenticAMD" {
        wanted.push((
            "AMD AOCL",
            "AMD's BLAS and LAPACK (BLIS and libFLAME), tuned for this AMD CPU",
        ));
    } else if hardware.arch == "aarch64" {
        wanted.push((
            "Arm Performance Libraries",
            "Arm's BLAS, LAPACK and sparse kernels, tuned for Arm cores",
        ));
    }
    if hardware.gpu.is_some() {
        wanted.push((
            "cuDSS",
            "NVIDIA's sparse direct solver on this GPU: the large 3D problems",
        ));
        wanted.push((
            "CUDA runtime",
            "what cuDSS and cuSPARSE run on, beside the GPU's driver",
        ));
        wanted.push((
            "cuSPARSE",
            "NVIDIA's sparse kernels, for the iterative solves on this GPU",
        ));
    }
    wanted.push((
        "MUMPS",
        "a multifrontal direct solver, sequential, against photonoxide's own on any CPU",
    ));
    wanted
        .into_iter()
        .filter_map(|(library, why)| {
            let guide = super::GUIDES.iter().find(|g| g.library == library)?;
            // conda-forge's first: it installs into photonoxide's own environment anywhere
            let install = guide
                .installs
                .iter()
                .filter(|i| i.systems.contains(&hardware.os) && super::runs_here(i))
                .min_by_key(|i| i.manager != "conda-forge");
            Some(Suggestion {
                library,
                why,
                by: "hardware",
                backends: guide.backends,
                install: install.map(|i| i.id),
                command: install.map(|i| i.command),
                download: guide.download,
                system: library == "Accelerate",
                measured: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(os: &'static str, arch: &'static str, vendor: &str, gpu: bool) -> Hardware {
        Hardware {
            os,
            arch,
            cpu: Cpu {
                vendor: vendor.into(),
                brand: String::new(),
                threads: 8,
                features: vec![],
                topology: Topology::default(),
            },
            gpu: gpu.then(|| Gpu {
                name: "a GPU".into(),
                driver: "1".into(),
            }),
        }
    }

    fn names(s: &[Suggestion]) -> Vec<&str> {
        s.iter().map(|s| s.library).collect()
    }

    #[test]
    fn each_machine_gets_its_vendor_s_libraries_and_the_gpu_s() {
        let intel = suggest(&machine("windows", "x86_64", "GenuineIntel", true));
        assert_eq!(
            names(&intel),
            ["oneMKL", "cuDSS", "CUDA runtime", "cuSPARSE", "MUMPS"]
        );
        // conda-forge first, where the library has one
        assert!(intel.iter().filter(|s| s.install.is_some()).all(|s| {
            super::super::GUIDES
                .iter()
                .find(|g| g.library == s.library)
                .and_then(|g| g.installs.iter().find(|i| Some(i.id) == s.install))
                .is_some_and(|i| {
                    i.manager == "conda-forge" || !s.command.unwrap_or("").starts_with("pip")
                })
        }));
        assert_eq!(
            names(&suggest(&machine("linux", "x86_64", "AuthenticAMD", false))),
            ["AMD AOCL", "MUMPS"]
        );
        let mac = suggest(&machine("macos", "aarch64", "Apple", false));
        assert_eq!(names(&mac), ["Accelerate", "MUMPS"]);
        assert!(mac[0].system && mac[0].install.is_none());
        assert_eq!(
            names(&suggest(&machine("linux", "aarch64", "Arm", false))),
            ["Arm Performance Libraries", "MUMPS"]
        );
    }

    #[test]
    fn this_machine_is_described() {
        let h = here();
        assert!(h.cpu.threads >= 1);
        assert_eq!(h.os, std::env::consts::OS);
    }
}
