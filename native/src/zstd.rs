//! Zstandard (Collet and Kucherawy, RFC 8878, doi:10.17487/RFC8878): the user's own libzstd,
//! loaded at run time, as a compressor of the exported files ([`photonoxide::export::codec`]).
//!
//! Deflate stays photonoxide's own and the default; this is the choice of a job's
//! `[task.export] compression = "zstd"`, which then needs the library. Each member of an `.npz`
//! is one Zstandard frame (`ZSTD_compress`, the simple one-shot call, one thread), stored as
//! ZIP method 93. The same input at the same level gives the same frame from the same library
//! on every run: its single-threaded compression has no randomness and no dependence on
//! timing, so [`Compressor::deterministic`] is true.
//!
//! libzstd is BSD-3-Clause or GPL-2.0, at the user's choice; it is installed by the user and
//! never linked or redistributed.

use std::ffi::{CStr, c_char, c_int, c_uint, c_ulonglong, c_void};
use std::path::{Path, PathBuf};

use photonoxide::Result;
use photonoxide::export::codec::Compressor;

use crate::Probe;
use crate::discovery::Spec;
use crate::library::{Library, error, load};

/// libzstd's files: its release 1 (the only one there has been, `libzstd.so.1`), and the names
/// Windows' builds give it (conda-forge's `libzstd.dll`, others' `zstd.dll`).
pub const ZSTD: Spec = Spec {
    name: "Zstandard",
    files: if cfg!(target_os = "linux") {
        &["libzstd.so.1"]
    } else if cfg!(target_os = "macos") {
        &["libzstd.1.dylib"]
    } else {
        &["libzstd.dll", "zstd.dll"]
    },
    variables: &["PHOTONOXIDE_ZSTD"],
    install: install_folders,
    wheel: None,
};

/// Homebrew's folders on macOS, where its libraries aren't on a system path.
fn install_folders() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/opt/homebrew/lib"),
            PathBuf::from("/usr/local/lib"),
        ]
    } else {
        Vec::new()
    }
}

/// The oldest release taken: 1.4, the first whose levels run to 22 with negative ones beside,
/// and which Debian and Ubuntu have shipped since 2020.
const OLDEST: c_uint = 10400;

type Compress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize;
type Decompress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
type Bound = unsafe extern "C" fn(usize) -> usize;
type IsError = unsafe extern "C" fn(usize) -> c_uint;
type ErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type VersionNumber = unsafe extern "C" fn() -> c_uint;
type ContentSize = unsafe extern "C" fn(*const c_void, usize) -> c_ulonglong;
type MaxLevel = unsafe extern "C" fn() -> c_int;

/// libzstd, loaded, with the functions photonoxide calls.
pub struct Zstd {
    // keeps the functions below valid
    _library: Library,
    path: PathBuf,
    version: String,
    max_level: c_int,
    compress: Compress,
    decompress: Decompress,
    bound: Bound,
    is_error: IsError,
    error_name: ErrorName,
    content_size: ContentSize,
}

impl std::fmt::Debug for Zstd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Zstd({} at {})", self.version, self.path.display())
    }
}

/// The version as `1.5.7`, from `ZSTD_versionNumber`'s 10507.
fn version_text(n: c_uint) -> String {
    format!("{}.{}.{}", n / 10000, n / 100 % 100, n % 100)
}

/// The library's release, refused if older than [`OLDEST`].
fn identify(library: &Library) -> Result<(c_uint, String)> {
    // SAFETY: ZSTD_versionNumber(void) -> unsigned, as zstd.h declares it
    let version: VersionNumber = unsafe { library.function("ZSTD_versionNumber") }?;
    // SAFETY: it takes nothing and returns a number
    let n = unsafe { version() };
    if n < OLDEST {
        return Err(error(format!(
            "libzstd {} is older than the 1.4 photonoxide takes",
            version_text(n)
        )));
    }
    Ok((n, version_text(n)))
}

impl Zstd {
    /// Finds and loads libzstd ([`ZSTD`]), or says why not.
    ///
    /// # Errors
    ///
    /// The discovery's reason, as text: not found, or each candidate's failure.
    pub fn load() -> std::result::Result<Zstd, String> {
        let (discovery, library) = load(&ZSTD, None, |l| identify(l).map(drop));
        let library = library.ok_or_else(|| discovery.reason())?;
        Zstd::open(library).map_err(|e| e.to_string())
    }

    fn open(library: Library) -> Result<Zstd> {
        let (_, version) = identify(&library)?;
        // SAFETY: each type is the function's signature in zstd.h (size_t is usize, unsigned
        // is c_uint, unsigned long long is c_ulonglong), and each is called only while
        // `_library` keeps the library loaded
        unsafe {
            let max: MaxLevel = library.function("ZSTD_maxCLevel")?;
            Ok(Zstd {
                path: library.path().to_path_buf(),
                version,
                max_level: max(),
                compress: library.function("ZSTD_compress")?,
                decompress: library.function("ZSTD_decompress")?,
                bound: library.function("ZSTD_compressBound")?,
                is_error: library.function("ZSTD_isError")?,
                error_name: library.function("ZSTD_getErrorName")?,
                content_size: library.function("ZSTD_getFrameContentSize")?,
                _library: library,
            })
        }
    }

    /// The file it was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The error a returned size stands for, if it is one.
    fn check(&self, code: usize, what: &str) -> Result<usize> {
        // SAFETY: ZSTD_isError takes any size_t
        if unsafe { (self.is_error)(code) } == 0 {
            return Ok(code);
        }
        // SAFETY: ZSTD_getErrorName returns a static, NUL-terminated string for any code
        let name = unsafe { CStr::from_ptr((self.error_name)(code)) };
        Err(error(format!(
            "libzstd's {what}: {}",
            name.to_string_lossy()
        )))
    }

    /// One frame's content, back: for the tests' read-back, and any reader of photonoxide's.
    ///
    /// # Errors
    ///
    /// When `frame` isn't a whole Zstandard frame that states its size.
    pub fn decompress(&self, frame: &[u8]) -> Result<Vec<u8>> {
        // SAFETY: reads at most frame.len() bytes of the frame's header
        let size = unsafe { (self.content_size)(frame.as_ptr().cast(), frame.len()) };
        // ZSTD_CONTENTSIZE_UNKNOWN and _ERROR are the two largest values
        if size >= c_ulonglong::MAX - 1 {
            return Err(error("the frame doesn't state its content's size"));
        }
        let size = usize::try_from(size).map_err(|_| error("the frame is too large"))?;
        let mut out = vec![0u8; size];
        // SAFETY: `out` holds `size` bytes for the library to write, `frame` is read whole
        let written = unsafe {
            (self.decompress)(
                out.as_mut_ptr().cast(),
                out.len(),
                frame.as_ptr().cast(),
                frame.len(),
            )
        };
        let written = self.check(written, "decompression")?;
        out.truncate(written);
        Ok(out)
    }
}

impl Compressor for Zstd {
    fn name(&self) -> &str {
        "zstd"
    }

    fn version(&self) -> String {
        self.version.clone()
    }

    fn deterministic(&self) -> bool {
        true
    }

    fn levels(&self) -> (i32, i32) {
        (1, self.max_level)
    }

    fn default_level(&self) -> i32 {
        // ZSTD_CLEVEL_DEFAULT
        3
    }

    fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>> {
        let (low, high) = self.levels();
        if !(low..=high).contains(&level) {
            return Err(error(format!(
                "zstd's level is {low} to {high}, not {level}"
            )));
        }
        // SAFETY: ZSTD_compressBound takes any size
        let capacity = unsafe { (self.bound)(data.len()) };
        let capacity = self.check(capacity, "bound")?;
        let mut out = vec![0u8; capacity];
        // SAFETY: `out` holds `capacity` bytes, the bound for `data`'s length, and `data` is
        // read whole; the call keeps no pointer to either
        let written = unsafe {
            (self.compress)(
                out.as_mut_ptr().cast(),
                out.len(),
                data.as_ptr().cast(),
                data.len(),
                level,
            )
        };
        let written = self.check(written, "compression")?;
        out.truncate(written);
        Ok(out)
    }

    fn zip_method(&self) -> u16 {
        93
    }
}

/// What is found of libzstd here.
pub fn probe() -> Probe {
    let (discovery, library) = load(&ZSTD, None, |l| identify(l).map(drop));
    let version = library
        .as_ref()
        .and_then(|l| identify(l).ok())
        .map(|(_, v)| v);
    Probe {
        discovery,
        version,
        details: vec!["compresses exported .npz files (compression = \"zstd\")".into()],
    }
}

/// Offers libzstd to exports, or records why it isn't here.
pub fn register() {
    match Zstd::load() {
        Ok(zstd) => photonoxide::export::codec::register(std::sync::Arc::new(zstd)),
        Err(reason) => photonoxide::export::codec::register_unavailable("zstd", reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_read_as_their_releases() {
        assert_eq!(version_text(10507), "1.5.7");
        assert_eq!(version_text(10400), "1.4.0");
    }
}
