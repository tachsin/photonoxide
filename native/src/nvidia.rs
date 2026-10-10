//! NVIDIA's libraries: the CUDA runtime, cuSPARSE and cuDSS, found and their versions read.
//! Their backends are #188 (cuDSS) and #190 (cuSPARSE).
//!
//! They are installed by the user under NVIDIA's licence and never redistributed: the CUDA
//! toolkit's installer, cuDSS's installer, conda-forge, or NVIDIA's pip wheels.

use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use photonoxide::Result;

use crate::Probe;
use crate::discovery::{Spec, versioned};
use crate::library::{Library, error, load};

type GetProperty = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;
type GetInt = unsafe extern "C" fn(*mut c_int) -> c_int;
type MemGetInfo = unsafe extern "C" fn(*mut usize, *mut usize) -> c_int;

/// The CUDA toolkit's folders, the newest first.
fn cuda_toolkits() -> Vec<PathBuf> {
    if cfg!(windows) {
        versioned(
            Path::new(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA"),
            "v",
        )
    } else {
        let mut dirs = vec![PathBuf::from("/usr/local/cuda")];
        dirs.extend(versioned(Path::new("/usr/local"), "cuda-"));
        dirs
    }
}

/// cuDSS's folders, the newest first: a build for each CUDA major version, 13 before 12.
fn cudss_folders() -> Vec<PathBuf> {
    let roots = if cfg!(windows) {
        versioned(Path::new(r"C:\Program Files\NVIDIA cuDSS"), "v")
            .into_iter()
            .map(|v| v.join("bin"))
            .collect()
    } else {
        vec![PathBuf::from("/usr/lib/x86_64-linux-gnu/libcudss")]
    };
    roots
        .iter()
        .flat_map(|r| [r.join("13"), r.join("12")])
        .collect()
}

/// The CUDA runtime's major releases photonoxide looks for, by its files' names
/// (`cudart64_13.dll`, `libcudart.so.12`): another's file isn't looked for.
pub const CUDA_RUNTIME_RELEASES: &[u32] = &[13, 12];
/// cuSPARSE's, likewise (`cusparse64_12.dll`).
pub const CUSPARSE_RELEASES: &[u32] = &[12];
/// cuDSS's, likewise (`cudss64_0.dll`).
pub const CUDSS_RELEASES: &[u32] = &[0];

/// The CUDA runtime.
pub const CUDA_RUNTIME: Spec = Spec {
    name: "CUDA runtime",
    files: if cfg!(windows) {
        &["cudart64_13.dll", "cudart64_12.dll"]
    } else if cfg!(target_os = "linux") {
        &["libcudart.so.13", "libcudart.so.12"]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_CUDART", "CUDA_PATH", "CUDA_HOME"],
    install: cuda_toolkits,
    wheel: Some("nvidia"),
};

/// cuSPARSE, the CUDA toolkit's sparse kernels.
pub const CUSPARSE: Spec = Spec {
    name: "cuSPARSE",
    files: if cfg!(windows) {
        &["cusparse64_12.dll"]
    } else if cfg!(target_os = "linux") {
        &["libcusparse.so.12"]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_CUSPARSE", "CUDA_PATH", "CUDA_HOME"],
    install: cuda_toolkits,
    wheel: Some("nvidia"),
};

/// cuDSS, NVIDIA's sparse direct solver.
pub const CUDSS: Spec = Spec {
    name: "cuDSS",
    files: if cfg!(windows) {
        &["cudss64_0.dll"]
    } else if cfg!(target_os = "linux") {
        &["libcudss.so.0"]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_CUDSS"],
    install: cudss_folders,
    wheel: Some("nvidia"),
};

/// `name`'s version from a `libraryPropertyType` call (cuSPARSE's, cuDSS's).
pub(crate) fn property_version(library: &Library, name: &str) -> Result<String> {
    // SAFETY: cusparseGetProperty and cudssGetProperty are
    // `status f(libraryPropertyType, int *value)`, the enum an int
    let get: GetProperty = unsafe { library.function(name)? };
    let mut parts = [0; 3];
    for (kind, part) in parts.iter_mut().enumerate() {
        // SAFETY: 0, 1 and 2 are MAJOR_VERSION, MINOR_VERSION and PATCH_LEVEL; part is a
        // writable int
        let status = unsafe { get(kind as c_int, part) };
        if status != 0 {
            return Err(error(format!("{name} returned status {status}")));
        }
    }
    Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
}

fn runtime_version(library: &Library) -> Result<String> {
    // SAFETY: `cudaError_t cudaRuntimeGetVersion(int *runtimeVersion)`
    let get: GetInt = unsafe { library.function("cudaRuntimeGetVersion")? };
    let mut v = 0;
    // SAFETY: v is a writable int
    let status = unsafe { get(&mut v) };
    if status != 0 {
        return Err(error(format!("cudaRuntimeGetVersion returned {status}")));
    }
    Ok(format!("{}.{}", v / 1000, v % 1000 / 10))
}

/// The GPUs the runtime sees, and the first one's free and total memory.
fn devices(library: &Library) -> Result<String> {
    // SAFETY: `cudaError_t cudaGetDeviceCount(int *count)`
    let count: GetInt = unsafe { library.function("cudaGetDeviceCount")? };
    // SAFETY: `cudaError_t cudaMemGetInfo(size_t *free, size_t *total)`
    let memory: MemGetInfo = unsafe { library.function("cudaMemGetInfo")? };
    let mut n = 0;
    // SAFETY: n is a writable int
    let status = unsafe { count(&mut n) };
    if status != 0 {
        return Err(error(format!(
            "cudaGetDeviceCount returned {status}: no GPU, or no driver"
        )));
    }
    let (mut free, mut total) = (0usize, 0usize);
    // SAFETY: two writable size_t; on device 0, the current one by default
    let status = unsafe { memory(&mut free, &mut total) };
    if status != 0 {
        return Err(error(format!("cudaMemGetInfo returned {status}")));
    }
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    Ok(format!(
        "{n} GPU{}; the first has {:.1} of {:.1} GiB free",
        if n == 1 { "" } else { "s" },
        free as f64 / GIB,
        total as f64 / GIB
    ))
}

/// The CUDA runtime, cuSPARSE and cuDSS: where each was found, its version, and the GPUs.
pub fn probe() -> Vec<Probe> {
    let (discovery, runtime) = load(&CUDA_RUNTIME, None, |l| runtime_version(l).map(drop));
    let mut details = Vec::new();
    if let Some(runtime) = &runtime {
        details.push(devices(runtime).unwrap_or_else(|e| e.to_string()));
    }
    let mut probes = vec![Probe {
        discovery,
        version: runtime.as_ref().and_then(|l| runtime_version(l).ok()),
        details,
    }];
    for (spec, function) in [
        (&CUSPARSE, "cusparseGetProperty"),
        (&CUDSS, "cudssGetProperty"),
    ] {
        let (discovery, library) = load(spec, None, |l| property_version(l, function).map(drop));
        probes.push(Probe {
            discovery,
            version: library
                .as_ref()
                .and_then(|l| property_version(l, function).ok()),
            details: Vec::new(),
        });
    }
    probes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_releases_looked_for_are_the_files_names() {
        for (spec, releases) in [
            (&CUDA_RUNTIME, CUDA_RUNTIME_RELEASES),
            (&CUSPARSE, CUSPARSE_RELEASES),
            (&CUDSS, CUDSS_RELEASES),
        ] {
            if !cfg!(any(windows, target_os = "linux")) {
                assert!(spec.files.is_empty());
                continue;
            }
            assert_eq!(spec.files.len(), releases.len(), "{}", spec.name);
            for (file, major) in spec.files.iter().zip(releases) {
                let named = if cfg!(windows) {
                    file.ends_with(&format!("_{major}.dll"))
                } else {
                    file.ends_with(&format!(".so.{major}"))
                };
                assert!(named, "{file}: {major}");
            }
        }
    }
}
