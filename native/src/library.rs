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
    /// beside it first, then on the usual search path.
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

#[cfg(windows)]
unsafe fn open(path: &Path) -> std::result::Result<libloading::Library, libloading::Error> {
    use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library};
    // SAFETY: as Library::open's
    unsafe { Library::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) }.map(Into::into)
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
