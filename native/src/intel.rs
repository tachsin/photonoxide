//! Intel's oneMKL through its single dynamic library, `mkl_rt`: found, loaded once in the
//! process, its threading layer chosen and that layer's runtime loaded, and its conditional
//! numerical reproducibility set. Its backends are PARDISO ([`crate::Pardiso`], #175) and,
//! later, its dense kernels (#186).
//!
//! oneMKL is installed by the user under the Intel Simplified Software Licence and never
//! redistributed: Intel's oneAPI installer, winget (`Intel.oneMKL`), conda-forge (`mkl`), apt,
//! or pip's `mkl` wheel.
//!
//! - **The threading layer.** `mkl_rt` loads its threading layer at its first computation, and
//!   that layer's runtime (Intel's OpenMP, `libiomp5md.dll` or `libiomp5.so`; TBB; GNU's
//!   OpenMP) is another product's library, often not on the system's path: oneAPI keeps it in
//!   its compiler's folder (`compiler\<version>\bin`). If it can't be loaded then, oneMKL ends
//!   the process (with oneMKL 2026.1 on Windows, exit code 127 at PARDISO's first call). So the
//!   layer is chosen here, before any computation, and only one whose libraries are all found:
//!   `MKL_THREADING_LAYER` if set, otherwise Intel's OpenMP, then TBB, then sequential (on
//!   Linux GNU's OpenMP first: Debian's oneMKL 2020 failed PARDISO's analysis on Intel's
//!   OpenMP with two threads or more, and worked on GNU's; not yet checked here). Its runtime
//!   is loaded first, from beside `mkl_rt` (conda's and pip's layout) or where discovery finds
//!   it (oneAPI's folders), so that the layer's own dependency on it is that library; then
//!   `mkl_set_threading_layer` sets it.
//! - **Threads.** Intel's and GNU's OpenMP layers take the number of threads from
//!   `mkl_set_num_threads_local`; the TBB layer doesn't, and runs on oneTBB's own (all the
//!   processors); the sequential layer runs on one.
//! - **Reproducibility.** `mkl_cbwr_set(MKL_CBWR_AUTO)` before any computation, unless
//!   `MKL_CBWR` is set, in which case oneMKL reads it: the same code path on every run on this
//!   processor.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::Probe;
use crate::discovery::{Discovery, Spec, versioned};
use crate::library::{Library, load};

/// oneMKL's single dynamic library, `mkl_rt`.
pub const MKL: Spec = Spec {
    name: "oneMKL",
    files: if cfg!(windows) {
        &["mkl_rt.3.dll", "mkl_rt.2.dll"]
    } else if cfg!(target_os = "macos") {
        &["libmkl_rt.2.dylib", "libmkl_rt.dylib"]
    } else {
        &["libmkl_rt.so.3", "libmkl_rt.so.2", "libmkl_rt.so"]
    },
    variables: &["PHOTONOXIDE_MKL", "MKLROOT"],
    install: mkl_folders,
    // pip's `mkl` wheel installs beside Python (Library\bin, lib), not in site-packages: see
    // mkl_folders
    wheel: None,
};

/// Intel's OpenMP runtime, which oneMKL's Intel threading layer needs.
pub const INTEL_OPENMP: Spec = Spec {
    name: "Intel OpenMP",
    files: if cfg!(windows) {
        &["libiomp5md.dll"]
    } else if cfg!(target_os = "macos") {
        &["libiomp5.dylib"]
    } else {
        &["libiomp5.so"]
    },
    variables: &["PHOTONOXIDE_IOMP", "CMPLR_ROOT"],
    install: compiler_folders,
    wheel: None,
};

/// oneTBB, which oneMKL's TBB threading layer needs.
pub const TBB: Spec = Spec {
    name: "oneTBB",
    files: if cfg!(windows) {
        &["tbb12.dll"]
    } else if cfg!(target_os = "macos") {
        &["libtbb.12.dylib"]
    } else {
        &["libtbb.so.12"]
    },
    variables: &["PHOTONOXIDE_TBB", "TBBROOT"],
    install: tbb_folders,
    wheel: None,
};

/// GNU's OpenMP runtime, which oneMKL's GNU threading layer needs (Linux).
pub const GNU_OPENMP: Spec = Spec {
    name: "GNU OpenMP",
    files: if cfg!(target_os = "linux") {
        &["libgomp.so.1"]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_GOMP"],
    install: Vec::new,
    wheel: None,
};

/// oneAPI's roots: `ONEAPI_ROOT`, then the default install folders.
fn oneapi_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = std::env::var_os("ONEAPI_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(base) = std::env::var_os(var) {
                roots.push(PathBuf::from(base).join("Intel").join("oneAPI"));
            }
        }
    } else {
        roots.push(PathBuf::from("/opt/intel/oneapi"));
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join("intel").join("oneapi"));
        }
    }
    roots
}

/// A oneAPI component's folders under each root, the newest version first (`2026.1` before
/// `2025.3`; `latest` last, a link to one of them).
fn component(name: &str) -> Vec<PathBuf> {
    oneapi_roots()
        .iter()
        .flat_map(|root| versioned(&root.join(name), ""))
        .collect()
}

/// oneMKL's folders: oneAPI's, then Python's prefixes, where pip's `mkl` wheel puts it
/// (`Library\bin` on Windows, `lib` elsewhere; discovery looks in both below each).
fn mkl_folders() -> Vec<PathBuf> {
    let mut dirs = component("mkl");
    if let Some(venv) = std::env::var_os("VIRTUAL_ENV") {
        dirs.push(PathBuf::from(venv));
    }
    if cfg!(windows) {
        for var in ["LOCALAPPDATA", "APPDATA"] {
            if let Some(base) = std::env::var_os(var) {
                let base = PathBuf::from(base);
                dirs.extend(versioned(&base.join("Programs").join("Python"), "Python"));
                dirs.extend(versioned(&base.join("Python"), "Python"));
            }
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local"));
    }
    dirs
}

/// The oneAPI compiler's folders, which hold Intel's OpenMP runtime.
fn compiler_folders() -> Vec<PathBuf> {
    let mut dirs = component("compiler");
    // older oneAPI on Linux: compiler/<version>/linux/compiler/lib/intel64_lin
    let below: Vec<PathBuf> = dirs
        .iter()
        .map(|d| {
            d.join("linux")
                .join("compiler")
                .join("lib")
                .join("intel64_lin")
        })
        .collect();
    dirs.extend(below);
    dirs
}

/// oneTBB's folders.
fn tbb_folders() -> Vec<PathBuf> {
    let mut dirs = component("tbb");
    let below: Vec<PathBuf> = dirs
        .iter()
        .map(|d| d.join("lib").join("intel64").join("gcc4.8"))
        .collect();
    dirs.extend(below);
    dirs
}

/// oneMKL's threading layers, with `mkl_set_threading_layer`'s codes (`mkl_service.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Layer {
    /// Intel's OpenMP.
    Intel,
    /// One thread.
    Sequential,
    /// GNU's OpenMP (Linux).
    Gnu,
    /// oneTBB.
    Tbb,
}

impl Layer {
    fn code(self) -> c_int {
        match self {
            Layer::Intel => 0,
            Layer::Sequential => 1,
            Layer::Gnu => 3,
            Layer::Tbb => 4,
        }
    }

    /// Its name, as `MKL_THREADING_LAYER` takes it.
    pub fn name(self) -> &'static str {
        match self {
            Layer::Intel => "INTEL",
            Layer::Sequential => "SEQUENTIAL",
            Layer::Gnu => "GNU",
            Layer::Tbb => "TBB",
        }
    }

    fn parse(name: &str) -> Option<Layer> {
        match name.trim().to_ascii_uppercase().as_str() {
            "INTEL" => Some(Layer::Intel),
            "SEQUENTIAL" => Some(Layer::Sequential),
            "GNU" => Some(Layer::Gnu),
            "TBB" => Some(Layer::Tbb),
            _ => None,
        }
    }

    /// oneMKL's own library for the layer, as `mkl_rt`'s file names it (`mkl_rt.3.dll` →
    /// `mkl_intel_thread.3.dll`).
    fn library(self, rt: &str) -> String {
        let part = match self {
            Layer::Intel => "mkl_intel_thread",
            Layer::Sequential => "mkl_sequential",
            Layer::Gnu => "mkl_gnu_thread",
            Layer::Tbb => "mkl_tbb_thread",
        };
        rt.replacen("mkl_rt", part, 1)
    }

    /// The runtime it needs, another product's library.
    fn runtime(self) -> Option<&'static Spec> {
        match self {
            Layer::Intel => Some(&INTEL_OPENMP),
            Layer::Gnu => Some(&GNU_OPENMP),
            Layer::Tbb => Some(&TBB),
            Layer::Sequential => None,
        }
    }

    /// The layers to try where `MKL_THREADING_LAYER` isn't set, in order.
    fn preferred() -> &'static [Layer] {
        if cfg!(target_os = "linux") {
            &[Layer::Gnu, Layer::Intel, Layer::Tbb, Layer::Sequential]
        } else {
            &[Layer::Intel, Layer::Tbb, Layer::Sequential]
        }
    }
}

type SetInt = unsafe extern "C" fn(c_int) -> c_int;
type GetVersion = unsafe extern "C" fn(*mut c_void);

const CBWR_ALL: c_int = !0;
const CBWR_OFF: c_int = 0;
const CBWR_AUTO: c_int = 2;

/// oneMKL, loaded and set up: its threading layer, the layer's runtime, reproducibility.
#[derive(Debug)]
pub struct Mkl {
    /// Its version, as its products are named, from `mkl_get_version` (`2026.1`).
    pub version: String,
    /// Its threading layer.
    pub layer: Layer,
    /// `mkl_cbwr_get(MKL_CBWR_ALL)`: 0 for no reproducibility, else its branch.
    pub reproducibility: c_int,
    set_num_threads_local: SetInt,
    // last: the function above must not outlive it; the runtime is unloaded after oneMKL
    library: Library,
    _runtime: Option<Library>,
}

impl Mkl {
    /// The loaded `mkl_rt`, for its functions.
    pub(crate) fn library(&self) -> &Library {
        &self.library
    }

    /// Sets oneMKL's threads on this thread (`mkl_set_num_threads_local`), and returns the
    /// previous setting to restore (0: the global one).
    pub(crate) fn set_threads_here(&self, threads: usize) -> c_int {
        let threads = c_int::try_from(threads).unwrap_or(c_int::MAX);
        // SAFETY: `int mkl_set_num_threads_local(int nth)`: any int; 0 restores the global
        // setting
        unsafe { (self.set_num_threads_local)(threads) }
    }

    /// The threading layer and its runtime, in words.
    pub fn threads(&self) -> String {
        let how = match self.layer {
            Layer::Intel => {
                "Intel's OpenMP, on rayon's thread count (RAYON_NUM_THREADS) by \
                             mkl_set_num_threads_local"
            }
            Layer::Gnu => {
                "GNU's OpenMP, on rayon's thread count (RAYON_NUM_THREADS) by \
                           mkl_set_num_threads_local"
            }
            Layer::Tbb => "oneTBB, on all the processors: it doesn't take a thread count",
            Layer::Sequential => "one thread",
        };
        format!("oneMKL's {} layer: {how}", self.layer.name())
    }
}

/// oneMKL as found and set up in this process, once: where it was looked for, and it or why
/// not.
pub fn mkl() -> (&'static Discovery, Result<Arc<Mkl>, String>) {
    static MKL_ONCE: OnceLock<(Discovery, Result<Arc<Mkl>, String>)> = OnceLock::new();
    let (discovery, mkl) = MKL_ONCE.get_or_init(set_up);
    (discovery, mkl.clone())
}

fn set_up() -> (Discovery, Result<Arc<Mkl>, String>) {
    // only resolving its functions: nothing in oneMKL computes before the layer is set
    let (discovery, library) = load(&MKL, None, |l| {
        // SAFETY: only resolved, not called; the type is mkl_service.h's
        unsafe { l.function::<SetInt>("MKL_Set_Threading_Layer") }.map(drop)
    });
    let Some(library) = library else {
        let reason = discovery.reason();
        return (discovery, Err(reason));
    };
    let mkl = set_up_layer(library).map(Arc::new);
    (discovery, mkl)
}

/// The layer to use and its runtime, loaded; or why none.
fn choose(folder: &Path, rt: &str) -> Result<(Layer, Option<Library>), String> {
    let wanted = std::env::var("MKL_THREADING_LAYER").ok();
    let candidates: Vec<Layer> = match &wanted {
        Some(name) => vec![Layer::parse(name).ok_or_else(|| {
            format!("MKL_THREADING_LAYER is {name:?}: not INTEL, SEQUENTIAL, GNU or TBB")
        })?],
        None => Layer::preferred().to_vec(),
    };
    let core = rt.replacen("mkl_rt", "mkl_core", 1);
    if !folder.join(&core).is_file() {
        return Err(format!(
            "oneMKL's {core} isn't beside {rt} in {}",
            folder.display()
        ));
    }
    let mut why = Vec::new();
    for layer in candidates {
        let own = layer.library(rt);
        if !folder.join(&own).is_file() {
            why.push(format!("{}: {own} isn't beside {rt}", layer.name()));
            continue;
        }
        let Some(spec) = layer.runtime() else {
            return Ok((layer, None));
        };
        // beside mkl_rt first (conda's and pip's layout), then wherever discovery finds it
        let beside: Vec<PathBuf> = spec
            .files
            .iter()
            .map(|f| folder.join(f))
            .filter(|p| p.is_file())
            .collect();
        let (found, runtime) = load(spec, beside.first().map(PathBuf::as_path), |_| Ok(()));
        match runtime {
            Some(runtime) => return Ok((layer, Some(runtime))),
            None => why.push(format!("{}: {}", layer.name(), found.reason())),
        }
    }
    Err(format!(
        "no threading layer of oneMKL's can be loaded{}: {}",
        if wanted.is_some() {
            " (as MKL_THREADING_LAYER asks)"
        } else {
            ""
        },
        why.join("; ")
    ))
}

fn set_up_layer(library: Library) -> Result<Mkl, String> {
    let path = library.path().to_path_buf();
    let folder = path.parent().unwrap_or(Path::new("."));
    let rt = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (layer, runtime) = choose(folder, &rt)?;
    let functions = || -> photonoxide::Result<(SetInt, SetInt, SetInt, SetInt, GetVersion)> {
        // SAFETY: each type is the function's declaration in mkl_service.h: `int f(int)`, and
        // `void MKL_Get_Version(MKLVersion *)`
        unsafe {
            Ok((
                library.function("MKL_Set_Threading_Layer")?,
                library.function("MKL_CBWR_Set")?,
                library.function("MKL_CBWR_Get")?,
                library.function("MKL_Set_Num_Threads_Local")?,
                library.function("MKL_Get_Version")?,
            ))
        }
    };
    let (set_layer, cbwr_set, cbwr_get, set_num_threads_local, get_version) =
        functions().map_err(|e| e.to_string())?;
    // SAFETY: `int mkl_set_threading_layer(int code)`, with one of mkl_service.h's codes;
    // called before any computation, as it must be
    let set = unsafe { set_layer(layer.code()) };
    if set != layer.code() {
        return Err(format!(
            "oneMKL's threading layer is {set}, not {} as asked",
            layer.name()
        ));
    }
    if std::env::var_os("MKL_CBWR").is_none() {
        // SAFETY: `int mkl_cbwr_set(int settings)`, with MKL_CBWR_AUTO, before any
        // computation
        let status = unsafe { cbwr_set(CBWR_AUTO) };
        if status != 0 {
            return Err(format!(
                "mkl_cbwr_set(MKL_CBWR_AUTO) returned {status}: oneMKL computed before it was \
                 set up"
            ));
        }
    }
    // SAFETY: `int mkl_cbwr_get(int option)`, MKL_CBWR_ALL asking for the whole setting
    let reproducibility = unsafe { cbwr_get(CBWR_ALL) };
    // MKLVersion is four ints and four pointers in oneMKL 2021 and later, three ints and four
    // pointers before: a larger buffer, zeroed, and only the first three ints read
    let mut buffer = [0u64; 16];
    // SAFETY: the buffer is larger than either layout of MKLVersion, and aligned for it
    unsafe { get_version(buffer.as_mut_ptr().cast()) };
    let ints: [c_int; 3] = std::array::from_fn(|k| {
        let bytes = buffer[k / 2].to_ne_bytes();
        let half = if k % 2 == 0 { &bytes[..4] } else { &bytes[4..] };
        c_int::from_ne_bytes(half.try_into().expect("four bytes"))
    });
    // its products are named major.update (2026.1 is 2026, 0, 1; 2020.4 is 2020, 0, 4)
    let version = if ints[1] == 0 {
        format!("{}.{}", ints[0], ints[2])
    } else {
        format!("{}.{}.{}", ints[0], ints[1], ints[2])
    };
    Ok(Mkl {
        version,
        layer,
        reproducibility,
        set_num_threads_local,
        library,
        _runtime: runtime,
    })
}

/// oneMKL: where it was found, its version, its threading layer and its reproducibility.
pub fn probe() -> Probe {
    let (discovery, mkl) = mkl();
    let (version, details) = match mkl {
        Ok(mkl) => (
            Some(mkl.version.clone()),
            vec![
                mkl.threads(),
                if mkl.reproducibility == CBWR_OFF {
                    "conditional numerical reproducibility off".to_owned()
                } else {
                    format!(
                        "conditional numerical reproducibility on (mkl_cbwr_get: {})",
                        mkl.reproducibility
                    )
                },
            ],
        ),
        Err(reason) if discovery.used().is_some() => (None, vec![reason]),
        Err(_) => (None, Vec::new()),
    };
    Probe {
        discovery: discovery.clone(),
        version,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_layer_names_its_own_library_and_parses_as_mkl_does() {
        assert_eq!(
            Layer::Intel.library("mkl_rt.3.dll"),
            "mkl_intel_thread.3.dll"
        );
        assert_eq!(Layer::Tbb.library("mkl_rt.2.dll"), "mkl_tbb_thread.2.dll");
        assert_eq!(
            Layer::Gnu.library("libmkl_rt.so.2"),
            "libmkl_gnu_thread.so.2"
        );
        assert_eq!(
            Layer::Sequential.library("libmkl_rt.so"),
            "libmkl_sequential.so"
        );
        for layer in [Layer::Intel, Layer::Sequential, Layer::Gnu, Layer::Tbb] {
            assert_eq!(Layer::parse(layer.name()), Some(layer));
            assert_eq!(Layer::parse(&layer.name().to_lowercase()), Some(layer));
            assert_eq!(layer.runtime().is_none(), layer == Layer::Sequential);
        }
        assert_eq!(Layer::parse("PGI"), None);
        // sequential last, always there to fall back on
        assert_eq!(Layer::preferred().last(), Some(&Layer::Sequential));
    }

    #[test]
    fn a_folder_without_onemkl_has_no_layer() {
        let folder = std::env::temp_dir().join("photonoxide-no-onemkl-here");
        let why = choose(&folder, "mkl_rt.3.dll").unwrap_err();
        assert!(why.contains("mkl_core.3.dll"), "{why}");
    }
}
