//! Loading a library and resolving its functions.

use std::path::{Path, PathBuf};

use photonoxide::{Error, Result};

use crate::discovery::{Discovery, Spec, Status, discover};

pub(crate) fn error(reason: impl Into<String>) -> Error {
    Error::InvalidValue {
        what: "native library",
        reason: reason.into(),
    }
}

/// A loaded library. It stays loaded while this lives, so a function taken from it must not
/// outlive it.
#[derive(Debug)]
pub struct Library {
    path: PathBuf,
    inner: libloading::Library,
}

impl Library {
    /// Loads the library in this file. On Windows, the libraries it needs are looked for
    /// beside it first, then on the usual search path. When that fails and a wheel's sibling
    /// folder (`x86_64` below the library's, or the one above when it is in `x86_64`) exists,
    /// the load is tried again with those folders added to the search by `AddDllDirectory`.
    /// That addition is process-wide and is never removed; it changes only loads that ask for
    /// the user's folders (`LOAD_LIBRARY_SEARCH_USER_DIRS`, or every load after a
    /// `SetDefaultDllDirectories`, which photonoxide never calls), and each folder is added once.
    ///
    /// # Errors
    ///
    /// The system's reason when the file can't be loaded.
    pub fn open(path: &Path) -> Result<Library> {
        // SAFETY: loading runs the library's initializers. The libraries photonoxide loads are
        // numerical libraries whose initializers only set up their own state.
        let inner = unsafe { open(path) }.map_err(|e| error(e.to_string()))?;
        Ok(Library {
            path: path.to_path_buf(),
            inner,
        })
    }

    /// The file it was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The function `name`, as the type `F`.
    ///
    /// # Safety
    ///
    /// `F` must be the function's C signature: an `unsafe extern "C" fn` with the argument and
    /// return types the library's header declares. It must not be called after this library
    /// is dropped.
    ///
    /// # Errors
    ///
    /// When the library has no such function.
    pub unsafe fn function<F: Copy>(&self, name: &str) -> Result<F> {
        // SAFETY: the caller promises F is the symbol's type
        let symbol = unsafe { self.inner.get::<F>(name.as_bytes()) }.map_err(|e| {
            error(format!(
                "{} has no function {name}: {e}",
                self.path.display()
            ))
        })?;
        Ok(*symbol)
    }
}

/// The folders beside `path`'s that a wheel keeps the rest of its libraries in: NVIDIA's CUDA 13
/// wheels put some in `bin` and some in `bin\x86_64`.
#[cfg(windows)]
fn beside(path: &Path) -> Vec<std::path::PathBuf> {
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let mut out = vec![dir.join("x86_64")];
    if dir.file_name().is_some_and(|n| n == "x86_64")
        && let Some(above) = dir.parent()
    {
        out.push(above.to_path_buf());
    }
    out.retain(|d| d.is_dir());
    out
}

#[cfg(windows)]
unsafe fn open(path: &Path) -> std::result::Result<libloading::Library, libloading::Error> {
    use libloading::os::windows::{
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
        LOAD_LIBRARY_SEARCH_USER_DIRS, LOAD_WITH_ALTERED_SEARCH_PATH, Library,
    };
    use std::os::windows::ffi::OsStrExt;
    unsafe extern "system" {
        fn AddDllDirectory(new_directory: *const u16) -> *mut std::ffi::c_void;
    }
    // SAFETY: as Library::open's
    let first = match unsafe { Library::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) } {
        Ok(library) => return Ok(library.into()),
        Err(e) => e,
    };
    // what it needs may be in a folder beside its own, where Windows doesn't look: again, with
    // those folders added to the ones searched
    // (LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR and AddDllDirectory both want an absolute path)
    let Ok(path) = std::path::absolute(path) else {
        return Err(first);
    };
    let path = path.as_path();
    let beside = beside(path);
    if beside.is_empty() {
        return Err(first);
    }
    // the folders already added: the addition lasts as long as the process, so once each
    static ADDED: std::sync::Mutex<Vec<std::path::PathBuf>> = std::sync::Mutex::new(Vec::new());
    let mut added = ADDED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for dir in beside {
        if added.contains(&dir) {
            continue;
        }
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: AddDllDirectory reads a NUL-terminated UTF-16 string, which `wide` is and
        // which outlives the call; it keeps no pointer to it. A path it refuses returns null,
        // and the retry below then fails as the first load did.
        if !unsafe { AddDllDirectory(wide.as_ptr()) }.is_null() {
            added.push(dir);
        }
    }
    drop(added);
    let flags = LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR
        | LOAD_LIBRARY_SEARCH_USER_DIRS
        | LOAD_LIBRARY_SEARCH_SYSTEM32;
    // SAFETY: as Library::open's
    unsafe { Library::load_with_flags(path, flags) }
        .map(Into::into)
        .map_err(|_| first)
}

#[cfg(not(windows))]
unsafe fn open(path: &Path) -> std::result::Result<libloading::Library, libloading::Error> {
    // SAFETY: as Library::open's
    unsafe { libloading::Library::new(path) }
}

/// Finds `spec`'s library and loads the first candidate that loads and passes `check` (a
/// version check, say), recording what became of every candidate.
pub fn load(
    spec: &Spec,
    setting: Option<&Path>,
    check: impl Fn(&Library) -> Result<()>,
) -> (Discovery, Option<Library>) {
    let mut discovery = discover(spec, setting);
    let mut loaded = None;
    for candidate in &mut discovery.candidates {
        if loaded.is_some() {
            break;
        }
        match Library::open(&candidate.path).and_then(|l| check(&l).map(|()| l)) {
            Ok(library) => {
                candidate.status = Status::Used;
                loaded = Some(library);
            }
            Err(e) => {
                candidate.status = Status::Failed(match e {
                    Error::InvalidValue { reason, .. } => reason,
                    other => other.to_string(),
                });
            }
        }
    }
    (discovery, loaded)
}
