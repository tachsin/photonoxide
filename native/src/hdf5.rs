//! HDF5 (The HDF Group's Hierarchical Data Format 5): the user's own libhdf5, loaded at run
//! time, as a container of the exported fields ([`photonoxide::export::codec::Container`]).
//!
//! A job's `[task.export] container = "hdf5"` writes each `.npz` it would write as an `.h5`
//! instead, beside the run's record: one dataset an array, by the same name, in the same C
//! order, `f32` or `f64` as IEEE little-endian; a complex array as the compound `{r, i}` that
//! h5py reads as complex; `meta.json`'s text as the root's string attribute `meta`. Deflated by
//! libhdf5's own filter at the export's level (chunked whole), stored at level 0. Objects carry
//! no modification times (`H5Pset_obj_track_times` off), so the same arrays from the same
//! library give the same file.
//!
//! libhdf5 is BSD-3-Clause; it is installed by the user and never linked or redistributed.

use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};

use photonoxide::Result;
use photonoxide::export::Precision;
use photonoxide::export::codec::{Array, Container};

use crate::Probe;
use crate::discovery::Spec;
use crate::library::{Library, error, load};

/// libhdf5's files, by the names its packages give them: conda-forge's and the HDF Group's
/// `hdf5.dll` on Windows, the sonames of 1.10 to 1.14 and 2.0 on Linux and macOS.
pub const HDF5: Spec = Spec {
    name: "HDF5",
    files: if cfg!(target_os = "linux") {
        &[
            "libhdf5.so.320",
            "libhdf5.so.310",
            "libhdf5.so.300",
            "libhdf5.so.200",
            "libhdf5.so.103",
            "libhdf5_serial.so.310",
            "libhdf5_serial.so.103",
            "libhdf5.so",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "libhdf5.320.dylib",
            "libhdf5.310.dylib",
            "libhdf5.300.dylib",
            "libhdf5.200.dylib",
            "libhdf5.103.dylib",
            "libhdf5.dylib",
        ]
    } else {
        &["hdf5.dll", "libhdf5.dll"]
    },
    variables: &["PHOTONOXIDE_HDF5", "HDF5_DIR"],
    install: install_folders,
    wheel: None,
};

/// The HDF Group's installers' folders on Windows, Homebrew's on macOS, Debian's serial build's
/// on Linux.
fn install_folders() -> Vec<PathBuf> {
    if cfg!(windows) {
        let root = std::env::var_os("ProgramFiles")
            .map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from);
        let group = root.join("HDF_Group").join("HDF5");
        let mut found: Vec<PathBuf> = std::fs::read_dir(&group)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .map(|e| e.path().join("bin"))
                    .collect()
            })
            .unwrap_or_default();
        // the newest first
        found.sort();
        found.reverse();
        found
    } else if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/opt/homebrew/lib"),
            PathBuf::from("/usr/local/lib"),
        ]
    } else {
        vec![PathBuf::from("/usr/lib/x86_64-linux-gnu/hdf5/serial")]
    }
}

type Hid = i64;
type Herr = c_int;
type Hsize = u64;

const H5P_DEFAULT: Hid = 0;
const H5S_ALL: Hid = 0;
const H5S_SCALAR: c_int = 0;
const H5F_ACC_RDONLY: c_uint = 0;
const H5F_ACC_TRUNC: c_uint = 2;
const H5T_COMPOUND: c_int = 6;

/// One call into libhdf5 at a time: its global state is shared unless it was built thread-safe.
static CALLS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The oldest release taken: 1.10, the first whose identifiers are 64-bit (`hid_t`).
const OLDEST: (c_uint, c_uint) = (1, 10);

/// The functions photonoxide calls, by their signatures in `hdf5.h`.
#[derive(Clone, Copy)]
#[allow(clippy::type_complexity)]
struct Api {
    open: unsafe extern "C" fn() -> Herr,
    eset_auto: unsafe extern "C" fn(Hid, *const c_void, *mut c_void) -> Herr,
    fcreate: unsafe extern "C" fn(*const c_char, c_uint, Hid, Hid) -> Hid,
    fopen: unsafe extern "C" fn(*const c_char, c_uint, Hid) -> Hid,
    fclose: unsafe extern "C" fn(Hid) -> Herr,
    screate: unsafe extern "C" fn(c_int) -> Hid,
    screate_simple: unsafe extern "C" fn(c_int, *const Hsize, *const Hsize) -> Hid,
    sget_ndims: unsafe extern "C" fn(Hid) -> c_int,
    sget_dims: unsafe extern "C" fn(Hid, *mut Hsize, *mut Hsize) -> c_int,
    sclose: unsafe extern "C" fn(Hid) -> Herr,
    pcreate: unsafe extern "C" fn(Hid) -> Hid,
    pset_chunk: unsafe extern "C" fn(Hid, c_int, *const Hsize) -> Herr,
    pset_deflate: unsafe extern "C" fn(Hid, c_uint) -> Herr,
    pset_track_times: unsafe extern "C" fn(Hid, c_uint) -> Herr,
    pclose: unsafe extern "C" fn(Hid) -> Herr,
    dcreate: unsafe extern "C" fn(Hid, *const c_char, Hid, Hid, Hid, Hid, Hid) -> Hid,
    dopen: unsafe extern "C" fn(Hid, *const c_char, Hid) -> Hid,
    dget_space: unsafe extern "C" fn(Hid) -> Hid,
    dwrite: unsafe extern "C" fn(Hid, Hid, Hid, Hid, Hid, *const c_void) -> Herr,
    dread: unsafe extern "C" fn(Hid, Hid, Hid, Hid, Hid, *mut c_void) -> Herr,
    dclose: unsafe extern "C" fn(Hid) -> Herr,
    tcreate: unsafe extern "C" fn(c_int, usize) -> Hid,
    tinsert: unsafe extern "C" fn(Hid, *const c_char, usize, Hid) -> Herr,
    tcopy: unsafe extern "C" fn(Hid) -> Hid,
    tset_size: unsafe extern "C" fn(Hid, usize) -> Herr,
    tclose: unsafe extern "C" fn(Hid) -> Herr,
    acreate: unsafe extern "C" fn(Hid, *const c_char, Hid, Hid, Hid, Hid) -> Hid,
    awrite: unsafe extern "C" fn(Hid, Hid, *const c_void) -> Herr,
    aclose: unsafe extern "C" fn(Hid) -> Herr,
    /// The library's global type and class identifiers, read after `H5open`.
    f64_le: Hid,
    f32_le: Hid,
    c_s1: Hid,
    dataset_create: Hid,
    file_create: Hid,
}

/// libhdf5, loaded.
pub struct Hdf5 {
    // keeps the functions valid
    _library: Library,
    api: Api,
    path: PathBuf,
    version: String,
}

impl std::fmt::Debug for Hdf5 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hdf5({} at {})", self.version, self.path.display())
    }
}

/// The library's release as (major, minor, release), refused below [`OLDEST`].
fn identify(library: &Library) -> Result<(c_uint, c_uint, c_uint)> {
    type Version = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *mut c_uint) -> Herr;
    // SAFETY: H5get_libversion(unsigned*, unsigned*, unsigned*) -> herr_t, as hdf5.h declares
    let version: Version = unsafe { library.function("H5get_libversion") }?;
    let (mut major, mut minor, mut release) = (0, 0, 0);
    // SAFETY: it writes the three numbers and nothing else
    if unsafe { version(&mut major, &mut minor, &mut release) } < 0 {
        return Err(error("H5get_libversion failed"));
    }
    if (major, minor) < OLDEST {
        return Err(error(format!(
            "libhdf5 {major}.{minor}.{release} is older than the 1.10 photonoxide takes"
        )));
    }
    Ok((major, minor, release))
}

/// A global identifier of the library's (a `hid_t` variable), read after `H5open`.
///
/// # Safety
///
/// `name` must be a `hid_t` variable the library exports, and `H5open` must have been called.
unsafe fn global(library: &Library, name: &str) -> Result<Hid> {
    // SAFETY: the symbol's address, as a pointer to a hid_t, per the caller
    let at: *const Hid = unsafe { library.function(name) }?;
    // SAFETY: the variable is initialized by H5open and lives as long as the library
    Ok(unsafe { at.read() })
}

/// An identifier the library closes when it is dropped.
struct Id {
    id: Hid,
    close: unsafe extern "C" fn(Hid) -> Herr,
}

impl Drop for Id {
    fn drop(&mut self) {
        // SAFETY: `id` was returned open by the library, and is closed once, here
        unsafe { (self.close)(self.id) };
    }
}

/// `id` as one to close with `close`, or the library's failure at `what`.
fn opened(id: Hid, close: unsafe extern "C" fn(Hid) -> Herr, what: &str) -> Result<Id> {
    if id < 0 {
        return Err(error(format!("libhdf5 failed: {what}")));
    }
    Ok(Id { id, close })
}

fn status(code: Herr, what: &str) -> Result<()> {
    if code < 0 {
        return Err(error(format!("libhdf5 failed: {what}")));
    }
    Ok(())
}

fn c_text(text: &str) -> Result<CString> {
    CString::new(text).map_err(|_| error(format!("{text:?} holds a NUL")))
}

impl Hdf5 {
    /// Finds and loads libhdf5 ([`HDF5`]), or says why not.
    ///
    /// # Errors
    ///
    /// The discovery's reason, as text.
    pub fn load() -> std::result::Result<Hdf5, String> {
        let (discovery, library) = load(&HDF5, None, |l| identify(l).map(drop));
        let library = library.ok_or_else(|| discovery.reason())?;
        Hdf5::open(library).map_err(|e| e.to_string())
    }

    fn open(library: Library) -> Result<Hdf5> {
        let (major, minor, release) = identify(&library)?;
        // SAFETY: each type is the function's signature in hdf5.h (hid_t 64-bit from 1.10,
        // herr_t int, hsize_t 64-bit unsigned), each called only while `_library` lives; the
        // globals are read after H5open, which initializes them
        let api = unsafe {
            let open: unsafe extern "C" fn() -> Herr = library.function("H5open")?;
            status(open(), "H5open")?;
            Api {
                open,
                eset_auto: library.function("H5Eset_auto2")?,
                fcreate: library.function("H5Fcreate")?,
                fopen: library.function("H5Fopen")?,
                fclose: library.function("H5Fclose")?,
                screate: library.function("H5Screate")?,
                screate_simple: library.function("H5Screate_simple")?,
                sget_ndims: library.function("H5Sget_simple_extent_ndims")?,
                sget_dims: library.function("H5Sget_simple_extent_dims")?,
                sclose: library.function("H5Sclose")?,
                pcreate: library.function("H5Pcreate")?,
                pset_chunk: library.function("H5Pset_chunk")?,
                pset_deflate: library.function("H5Pset_deflate")?,
                pset_track_times: library.function("H5Pset_obj_track_times")?,
                pclose: library.function("H5Pclose")?,
                dcreate: library.function("H5Dcreate2")?,
                dopen: library.function("H5Dopen2")?,
                dget_space: library.function("H5Dget_space")?,
                dwrite: library.function("H5Dwrite")?,
                dread: library.function("H5Dread")?,
                dclose: library.function("H5Dclose")?,
                tcreate: library.function("H5Tcreate")?,
                tinsert: library.function("H5Tinsert")?,
                tcopy: library.function("H5Tcopy")?,
                tset_size: library.function("H5Tset_size")?,
                tclose: library.function("H5Tclose")?,
                acreate: library.function("H5Acreate2")?,
                awrite: library.function("H5Awrite")?,
                aclose: library.function("H5Aclose")?,
                f64_le: global(&library, "H5T_IEEE_F64LE_g")?,
                f32_le: global(&library, "H5T_IEEE_F32LE_g")?,
                c_s1: global(&library, "H5T_C_S1_g")?,
                dataset_create: global(&library, "H5P_CLS_DATASET_CREATE_ID_g")?,
                file_create: global(&library, "H5P_CLS_FILE_CREATE_ID_g")?,
            }
        };
        // its errors are photonoxide's, not a stack printed on stderr
        // SAFETY: H5Eset_auto2(H5E_DEFAULT, NULL, NULL) turns the printing off
        unsafe { (api.eset_auto)(0, std::ptr::null(), std::ptr::null_mut()) };
        Ok(Hdf5 {
            path: library.path().to_path_buf(),
            version: format!("{major}.{minor}.{release}"),
            _library: library,
            api,
        })
    }

    /// The file it was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The type of an element: IEEE little-endian at `precision`, or the compound `{r, i}` of
    /// two of them.
    fn element(&self, precision: Precision, complex: bool) -> Result<Id> {
        let (real, size) = match precision {
            Precision::F32 => (self.api.f32_le, 4),
            _ => (self.api.f64_le, 8),
        };
        // SAFETY: copying a predefined type gives a new one to close
        let t = opened(
            unsafe { (self.api.tcopy)(real) },
            self.api.tclose,
            "H5Tcopy",
        )?;
        if !complex {
            return Ok(t);
        }
        drop(t);
        let compound = opened(
            // SAFETY: a compound of 2 × size bytes, its two members inserted at 0 and size
            unsafe { (self.api.tcreate)(H5T_COMPOUND, 2 * size) },
            self.api.tclose,
            "H5Tcreate",
        )?;
        for (name, offset) in [("r", 0), ("i", size)] {
            let name = c_text(name)?;
            status(
                // SAFETY: a NUL-terminated name, an offset inside the compound, a predefined type
                unsafe { (self.api.tinsert)(compound.id, name.as_ptr(), offset, real) },
                "H5Tinsert",
            )?;
        }
        Ok(compound)
    }

    /// An array of the file at `path`: its shape and its values' bytes as they are stored, for
    /// the tests' read-back.
    ///
    /// # Errors
    ///
    /// When the file or the dataset can't be read.
    pub fn read(
        &self,
        path: &Path,
        name: &str,
        precision: Precision,
        complex: bool,
    ) -> Result<(Vec<usize>, Vec<u8>)> {
        let _one = CALLS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let file_name = c_text(&path.display().to_string())?;
        let file = opened(
            // SAFETY: a NUL-terminated path, read only
            unsafe { (self.api.fopen)(file_name.as_ptr(), H5F_ACC_RDONLY, H5P_DEFAULT) },
            self.api.fclose,
            "H5Fopen",
        )?;
        let name = c_text(name)?;
        let set = opened(
            // SAFETY: a dataset by its NUL-terminated name in the open file
            unsafe { (self.api.dopen)(file.id, name.as_ptr(), H5P_DEFAULT) },
            self.api.dclose,
            "H5Dopen2",
        )?;
        let space = opened(
            // SAFETY: the dataset's space, to close
            unsafe { (self.api.dget_space)(set.id) },
            self.api.sclose,
            "H5Dget_space",
        )?;
        // SAFETY: the rank of the open space
        let rank = unsafe { (self.api.sget_ndims)(space.id) };
        let rank = usize::try_from(rank).map_err(|_| error("H5Sget_simple_extent_ndims"))?;
        let mut dims = vec![0 as Hsize; rank];
        status(
            // SAFETY: `dims` holds `rank` sizes for the library to write
            unsafe { (self.api.sget_dims)(space.id, dims.as_mut_ptr(), std::ptr::null_mut()) },
            "H5Sget_simple_extent_dims",
        )?;
        let shape: Vec<usize> = dims.iter().map(|&d| d as usize).collect();
        let element = self.element(precision, complex)?;
        let item = match precision {
            Precision::F32 => 4,
            _ => 8,
        } * if complex { 2 } else { 1 };
        let mut data = vec![0u8; shape.iter().product::<usize>() * item];
        status(
            // SAFETY: `data` holds every element of the dataset at the memory type `element`
            unsafe {
                (self.api.dread)(
                    set.id,
                    element.id,
                    H5S_ALL,
                    H5S_ALL,
                    H5P_DEFAULT,
                    data.as_mut_ptr().cast(),
                )
            },
            "H5Dread",
        )?;
        Ok((shape, data))
    }
}

impl Container for Hdf5 {
    fn name(&self) -> &str {
        "hdf5"
    }

    fn version(&self) -> String {
        self.version.clone()
    }

    fn deterministic(&self) -> bool {
        true
    }

    fn extension(&self) -> &str {
        "h5"
    }

    fn write(&self, path: &Path, arrays: &[Array<'_>], meta: &str, level: u8) -> Result<u64> {
        // libhdf5 is thread-safe only when built so: one call into it at a time
        let _one = CALLS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let api = &self.api;
        // the library's state is process-wide: H5open again, harmless, in case it was closed
        // SAFETY: H5open takes nothing
        status(unsafe { (api.open)() }, "H5open")?;
        let file_name = c_text(&path.display().to_string())?;
        // the root group keeps no times either: its creation list is the file's
        let fcpl = opened(
            // SAFETY: a file creation list, to close
            unsafe { (api.pcreate)(api.file_create) },
            api.pclose,
            "H5Pcreate",
        )?;
        status(
            // SAFETY: setting a property of the open list
            unsafe { (api.pset_track_times)(fcpl.id, 0) },
            "H5Pset_obj_track_times",
        )?;
        let file = opened(
            // SAFETY: a NUL-terminated path, created or truncated, with the list above
            unsafe { (api.fcreate)(file_name.as_ptr(), H5F_ACC_TRUNC, fcpl.id, H5P_DEFAULT) },
            api.fclose,
            "H5Fcreate",
        )?;
        for a in arrays {
            let dims: Vec<Hsize> = a.shape.iter().map(|&n| n as Hsize).collect();
            let space = if dims.is_empty() {
                opened(
                    // SAFETY: a scalar space
                    unsafe { (api.screate)(H5S_SCALAR) },
                    api.sclose,
                    "H5Screate",
                )?
            } else {
                opened(
                    // SAFETY: `dims` holds the rank's sizes; no maximum
                    unsafe {
                        (api.screate_simple)(dims.len() as c_int, dims.as_ptr(), std::ptr::null())
                    },
                    api.sclose,
                    "H5Screate_simple",
                )?
            };
            let dcpl = opened(
                // SAFETY: a dataset creation list, to close
                unsafe { (api.pcreate)(api.dataset_create) },
                api.pclose,
                "H5Pcreate",
            )?;
            status(
                // SAFETY: setting properties of the open list
                unsafe { (api.pset_track_times)(dcpl.id, 0) },
                "H5Pset_obj_track_times",
            )?;
            let elements: usize = a.shape.iter().product();
            if level > 0 && !dims.is_empty() && elements > 0 {
                status(
                    // SAFETY: one chunk the whole array, then deflate at `level`
                    unsafe { (api.pset_chunk)(dcpl.id, dims.len() as c_int, dims.as_ptr()) },
                    "H5Pset_chunk",
                )?;
                status(
                    // SAFETY: as above
                    unsafe { (api.pset_deflate)(dcpl.id, c_uint::from(level.min(9))) },
                    "H5Pset_deflate",
                )?;
            }
            let element = self.element(a.precision, a.complex)?;
            let name = c_text(a.name)?;
            let set = opened(
                // SAFETY: a new dataset by its NUL-terminated name, of the type and space above
                unsafe {
                    (api.dcreate)(
                        file.id,
                        name.as_ptr(),
                        element.id,
                        space.id,
                        H5P_DEFAULT,
                        dcpl.id,
                        H5P_DEFAULT,
                    )
                },
                api.dclose,
                "H5Dcreate2",
            )?;
            if elements > 0 {
                status(
                    // SAFETY: `a.data` holds every element in C order at `element`'s layout,
                    // little-endian as the file type is
                    unsafe {
                        (api.dwrite)(
                            set.id,
                            element.id,
                            H5S_ALL,
                            H5S_ALL,
                            H5P_DEFAULT,
                            a.data.as_ptr().cast(),
                        )
                    },
                    "H5Dwrite",
                )?;
            }
        }
        // meta.json's text, as the root's string attribute `meta`, NUL-terminated
        let mut bytes = meta.as_bytes().to_vec();
        bytes.push(0);
        // SAFETY: a copy of the C string type, sized to the text and its NUL
        let text = opened(unsafe { (api.tcopy)(api.c_s1) }, api.tclose, "H5Tcopy")?;
        status(
            // SAFETY: setting the size of the open type
            unsafe { (api.tset_size)(text.id, bytes.len()) },
            "H5Tset_size",
        )?;
        // SAFETY: a scalar space
        let scalar = opened(
            unsafe { (api.screate)(H5S_SCALAR) },
            api.sclose,
            "H5Screate",
        )?;
        let name = c_text("meta")?;
        let attribute = opened(
            // SAFETY: an attribute of the root by its NUL-terminated name
            unsafe {
                (api.acreate)(
                    file.id,
                    name.as_ptr(),
                    text.id,
                    scalar.id,
                    H5P_DEFAULT,
                    H5P_DEFAULT,
                )
            },
            api.aclose,
            "H5Acreate2",
        )?;
        status(
            // SAFETY: the text and its NUL, as long as the type's size
            unsafe { (api.awrite)(attribute.id, text.id, bytes.as_ptr().cast()) },
            "H5Awrite",
        )?;
        drop(attribute);
        drop(scalar);
        drop(text);
        drop(file);
        Ok(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))
    }
}

/// What is found of libhdf5 here.
pub fn probe() -> Probe {
    let (discovery, library) = load(&HDF5, None, |l| identify(l).map(drop));
    let version = library
        .as_ref()
        .and_then(|l| identify(l).ok())
        .map(|(a, b, c)| format!("{a}.{b}.{c}"));
    Probe {
        discovery,
        version,
        details: vec!["writes exported fields as .h5 files (container = \"hdf5\")".into()],
    }
}

/// Offers libhdf5 to exports, or records why it isn't here.
pub fn register() {
    match Hdf5::load() {
        Ok(h) => photonoxide::export::codec::register_container(std::sync::Arc::new(h)),
        Err(reason) => photonoxide::export::codec::register_container_unavailable("hdf5", reason),
    }
}
