//! Compressors an external library gives the exported files, beside deflate (issue #431).
//!
//! Deflate is photonoxide's own (`flate2`'s pure-Rust backend) and the default: everything
//! works with no library installed. Zstandard's main implementation is written in C, which the
//! library never links, so it is offered here at run time instead: `photonoxide-native` loads
//! the user's own libzstd and registers a [`Compressor`] by its name, and an export that asks
//! for `"zstd"` uses it, or fails saying why it isn't there ([`compressor`]). The same
//! registry's shape as the solvers' ([`crate::backend`]).
//!
//! HDF5 is a file format of its own, whose library is written in C too: a [`Container`]
//! holds the arrays an `.npz` would, `photonoxide-native` registers the user's own libhdf5 as
//! `"hdf5"`, and `[task.export] container = "hdf5"` asks for it ([`container`]).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use crate::Result;

/// A compressor of an external library's.
pub trait Compressor: Send + Sync {
    /// Its name, as a job's `[task.export] compression` gives it: `"zstd"`.
    fn name(&self) -> &str;

    /// The library's version, as it states it.
    fn version(&self) -> String;

    /// Whether it gives the same bytes on every run, for the same input, level and library.
    fn deterministic(&self) -> bool;

    /// The levels it takes, smallest first.
    fn levels(&self) -> (i32, i32);

    /// Its level when an export gives none.
    fn default_level(&self) -> i32;

    /// `data` compressed at `level`, as one frame of its own format.
    ///
    /// # Errors
    ///
    /// The library's own, as photonoxide's errors.
    fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>>;

    /// The ZIP compression method its frames are stored as (PKWARE's *APPNOTE* 4.4.5): 93 for
    /// Zstandard.
    fn zip_method(&self) -> u16;
}

type Registry = RwLock<BTreeMap<String, std::result::Result<Arc<dyn Compressor>, String>>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// Offers `compressor` to exports under its name, in place of any registered before.
pub fn register(compressor: Arc<dyn Compressor>) {
    let name = compressor.name().to_owned();
    if let Ok(mut r) = registry().write() {
        r.insert(name, Ok(compressor));
    }
}

/// Records that the compressor `name` isn't available here, and why (its library wasn't
/// found, or didn't load), for an export that asks for it to say so.
pub fn register_unavailable(name: &str, reason: impl Into<String>) {
    if let Ok(mut r) = registry().write() {
        r.insert(name.to_owned(), Err(reason.into()));
    }
}

/// The compressor `name`, or why there is none: its library's reason, or that nothing
/// registered it (a program that doesn't load external libraries, as the Python package).
///
/// # Errors
///
/// The reason, as text.
pub fn compressor(name: &str) -> std::result::Result<Arc<dyn Compressor>, String> {
    let r = registry()
        .read()
        .map_err(|_| "the compressors' registry is poisoned".to_owned())?;
    match r.get(name) {
        Some(Ok(c)) => Ok(Arc::clone(c)),
        Some(Err(reason)) => Err(format!("{name} isn't available here: {reason}")),
        None => Err(format!(
            "{name} isn't available here: it is loaded from the user's own library by \
             photonoxide-native, which the photonoxide program does at its start and this \
             program didn't (docs/libraries.md)"
        )),
    }
}

/// Every compressor registered, by name, with its library's version or why it is missing.
pub fn listed() -> Vec<(String, std::result::Result<String, String>)> {
    registry()
        .read()
        .map(|r| {
            r.iter()
                .map(|(name, c)| {
                    (
                        name.clone(),
                        c.as_ref().map(|c| c.version()).map_err(Clone::clone),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// An array a [`Container`] writes: its name, its shape (C order, the last index fastest), and
/// its values as little-endian bytes, `f32` or `f64`, a complex value as its real part then its
/// imaginary part.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Array<'a> {
    /// Its name, e.g. `"Ex"`.
    pub name: &'a str,
    /// Its shape; empty for a scalar.
    pub shape: &'a [usize],
    /// Whether its values are complex.
    pub complex: bool,
    /// The width of its numbers.
    pub precision: crate::export::Precision,
    /// Its values' bytes.
    pub data: &'a [u8],
}

/// A file format of an external library's, holding the exported arrays in place of an `.npz`:
/// HDF5's, from the user's own libhdf5.
pub trait Container: Send + Sync {
    /// Its name, as a job's `[task.export] container` gives it: `"hdf5"`.
    fn name(&self) -> &str;

    /// The library's version.
    fn version(&self) -> String;

    /// Whether the same arrays give the same file's bytes on every run.
    fn deterministic(&self) -> bool;

    /// Its files' extension, e.g. `"h5"`.
    fn extension(&self) -> &str;

    /// Writes `arrays` and `meta` (the text of an `.npz`'s `meta.json`) as `path`, compressed at
    /// deflate's `level` (0 for none), and returns the file's size.
    ///
    /// # Errors
    ///
    /// The library's own, as photonoxide's errors.
    fn write(&self, path: &Path, arrays: &[Array<'_>], meta: &str, level: u8) -> Result<u64>;
}

type Containers = RwLock<BTreeMap<String, std::result::Result<Arc<dyn Container>, String>>>;

fn containers() -> &'static Containers {
    static CONTAINERS: OnceLock<Containers> = OnceLock::new();
    CONTAINERS.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// Offers `container` to exports under its name.
pub fn register_container(container: Arc<dyn Container>) {
    let name = container.name().to_owned();
    if let Ok(mut r) = containers().write() {
        r.insert(name, Ok(container));
    }
}

/// Records that the container `name` isn't available here, and why.
pub fn register_container_unavailable(name: &str, reason: impl Into<String>) {
    if let Ok(mut r) = containers().write() {
        r.insert(name.to_owned(), Err(reason.into()));
    }
}

/// The container `name`, or why there is none.
///
/// # Errors
///
/// The reason, as text.
pub fn container(name: &str) -> std::result::Result<Arc<dyn Container>, String> {
    let r = containers()
        .read()
        .map_err(|_| "the containers' registry is poisoned".to_owned())?;
    match r.get(name) {
        Some(Ok(c)) => Ok(Arc::clone(c)),
        Some(Err(reason)) => Err(format!("{name} isn't available here: {reason}")),
        None => Err(format!(
            "{name} isn't available here: it is loaded from the user's own library by \
             photonoxide-native, which the photonoxide program does at its start and this \
             program didn't (docs/libraries.md)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A compressor of the tests' own: the data reversed, behind a marker.
    struct Reverse;

    impl Compressor for Reverse {
        fn name(&self) -> &str {
            "reverse-test"
        }
        fn version(&self) -> String {
            "1".into()
        }
        fn deterministic(&self) -> bool {
            true
        }
        fn levels(&self) -> (i32, i32) {
            (1, 3)
        }
        fn default_level(&self) -> i32 {
            2
        }
        fn compress(&self, data: &[u8], _level: i32) -> Result<Vec<u8>> {
            Ok(data.iter().rev().copied().collect())
        }
        fn zip_method(&self) -> u16 {
            99
        }
    }

    #[test]
    fn a_compressor_is_found_by_name_or_says_why_not() {
        assert!(
            compressor("nothing-here")
                .err()
                .unwrap()
                .contains("isn't available")
        );
        register_unavailable("missing-test", "libmissing wasn't found");
        assert!(
            compressor("missing-test")
                .err()
                .unwrap()
                .contains("libmissing wasn't found")
        );
        register(Arc::new(Reverse));
        let c = compressor("reverse-test").unwrap();
        assert_eq!(c.compress(b"abc", 1).unwrap(), b"cba");
        assert!(
            listed()
                .iter()
                .any(|(n, v)| n == "reverse-test" && v.as_deref() == Ok("1"))
        );
    }
}
