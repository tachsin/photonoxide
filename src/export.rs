//! Exporting a run's results, so they can be taken anywhere, not just drawn (issue #431).
//!
//! `events.jsonl`'s own pictures ([`crate::job::Event::Mode`], [`crate::job::Event::Field`],
//! [`crate::job::Event::StaticField`]) stay as they are (`f32`, scaled to a peak of 1, no
//! phase): this module writes the numbers themselves, into the run's directory.
//!
//! - **Tables** ([`tables`]): the small numbers, as CSV, each number the shortest decimal that
//!   reads back to the same bits: the modes' effective indices, a sweep's, the S-parameters,
//!   FDTD's spectra and resonances, a study's points, an optimization's history, a static job's
//!   numbers and its transient. A run writes every table it has by default, whether its job
//!   asks or not ([`tables::write_tables`]).
//! - **Fields** ([`Npz`]): the complex fields on their grid, as NumPy's `.npz`, only when the
//!   job's `[task.export]` asks: a mode's E and H ([`write_mode_npz`]), FDFD's field along z,
//!   FDTD's field monitors' transforms of E and H, a static job's temperature and potential.
//!
//! **The `.npz`.** NumPy's own format: a ZIP (written from PKWARE's *APPNOTE*) of `.npy` arrays
//! (NEP 1), deflated at a selectable level, store (level 0) to smallest (9). Beside the arrays,
//! `meta.json`, a plain JSON member (`numpy.load`'s `NpzFile` only reads members that end in
//! `.npy`, so it is never mistaken for one): photonoxide's version, the job and its text as it
//! ran, the run's id, the precision written, and the grid, units and normalization of the
//! arrays.
//!
//! Nothing here reads a `.npy` or `.npz` file back: the tests do, with the `zip` crate (a
//! dev-dependency only) unpacking the archive and a small reader of their own checking the
//! array bytes against the solver's own values, to the bit.

pub mod codec;
mod npy;
pub mod tables;
mod zip;

use std::path::Path;

use num_complex::Complex64 as c64;
use serde::Serialize;

use crate::mode::fields::Fields;
use crate::mode::vector::VectorMode;
use crate::units::Wavelength;
use crate::{Error, Result};

/// The width an exported array's numbers are written at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Precision {
    /// `f64`, the solver's own: every result here is computed in it.
    #[default]
    F64,
    /// `f32`: half the size, each number downcast (`meta.json` says so).
    F32,
}

/// What an exported file names about where it came from.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Meta {
    /// The job's name.
    pub job: String,
    /// The run's id: its directory's name (`Run::dir`'s file name), empty for a run kept only in
    /// memory.
    pub run_id: String,
    /// The job's text as it ran (its `job.toml`, `job.json` or `job.yaml`), empty if unknown.
    pub job_text: String,
}

impl Meta {
    /// A file's provenance: its job's name and its run's id.
    pub fn new(job: impl Into<String>, run_id: impl Into<String>) -> Meta {
        Meta {
            job: job.into(),
            run_id: run_id.into(),
            job_text: String::new(),
        }
    }

    /// With the job's text as it ran.
    #[must_use]
    pub fn with_job_text(mut self, text: impl Into<String>) -> Meta {
        self.job_text = text.into();
        self
    }

    /// `job` as `run` runs it.
    pub(crate) fn of(job: &crate::run::Job, run: &crate::run::Run) -> Meta {
        Meta::new(
            job.name(),
            run.dir()
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
        .with_job_text(job.text())
    }
}

/// What an `.npz`'s arrays are: their grid, their units and their normalization, for its
/// `meta.json`.
#[derive(Clone, Copy, Debug)]
pub struct Described<'a> {
    /// Where each array's values sit.
    pub grid: &'a str,
    /// Their units.
    pub units: &'a str,
    /// What they are scaled to, if anything.
    pub normalization: &'a str,
}

/// `meta.json`'s fields.
#[derive(Serialize)]
struct MetaFile<'a> {
    photonoxide: &'a str,
    job: &'a str,
    run_id: &'a str,
    precision: &'static str,
    grid: &'a str,
    units: &'a str,
    normalization: &'a str,
    job_text: &'a str,
    compression: &'a str,
}

fn meta_json(
    meta: &Meta,
    precision: Precision,
    described: &Described<'_>,
    compression: &str,
) -> String {
    let file = MetaFile {
        compression,
        photonoxide: env!("CARGO_PKG_VERSION"),
        job: &meta.job,
        run_id: &meta.run_id,
        precision: match precision {
            Precision::F64 => "f64",
            Precision::F32 => "f32, downcast from f64",
        },
        grid: described.grid,
        units: described.units,
        normalization: described.normalization,
        job_text: &meta.job_text,
    };
    // every field is a plain string: this cannot fail
    serde_json::to_string_pretty(&file).expect("meta.json always serializes")
}

fn io_error(path: &Path, e: std::io::Error) -> Error {
    Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

/// An `.npz` being written: arrays added one by one, each deflated at the level it was made
/// with, then [`Npz::write`] puts it on disk.
pub struct Npz {
    zip: zip::ZipWriter<Vec<u8>>,
    precision: Precision,
    level: u8,
    /// An external library's compressor and its level, in place of deflate.
    codec: Option<(std::sync::Arc<dyn codec::Compressor>, i32)>,
    error: Option<std::io::Error>,
}

impl Npz {
    /// An empty archive whose fields are written at `precision`, deflated at `level` (0 stores
    /// them, 1 to 9 deflate; above 9 is 9).
    pub fn new(precision: Precision, level: u8) -> Npz {
        Npz {
            zip: zip::ZipWriter::new(Vec::new()),
            precision,
            level: level.min(9),
            codec: None,
            error: None,
        }
    }

    /// An empty archive whose members `compressor` compresses at `level` (Zstandard's, from
    /// [`codec::compressor`]), stored as its ZIP method: a reader needs that method, as
    /// Python's `zipfile` has Zstandard's from Python 3.14.
    pub fn compressed(
        precision: Precision,
        compressor: std::sync::Arc<dyn codec::Compressor>,
        level: i32,
    ) -> Npz {
        Npz {
            codec: Some((compressor, level)),
            ..Npz::new(precision, 0)
        }
    }

    /// How its members are compressed, for `meta.json`.
    fn compression(&self) -> String {
        match &self.codec {
            Some((c, level)) => format!("{} {}, level {level}", c.name(), c.version()),
            None if self.level == 0 => "stored".into(),
            None => format!("deflate, level {}", self.level),
        }
    }

    fn put(&mut self, name: &str, data: &[u8]) {
        if self.error.is_some() {
            return;
        }
        let added = match &self.codec {
            Some((c, level)) => match c.compress(data, *level) {
                Ok(payload) => self.zip.add_compressed(name, data, c.zip_method(), payload),
                Err(e) => Err(std::io::Error::other(e.to_string())),
            },
            None => self.zip.add(name, data, self.level),
        };
        if let Err(e) = added {
            self.error = Some(e);
        }
    }

    /// A real field `name` (`.npy` added), `values` in C order for `shape`, at the archive's
    /// precision.
    ///
    /// # Panics
    ///
    /// If `values.len()` isn't `shape`'s product.
    pub fn field(&mut self, name: &str, shape: &[usize], values: &[f64]) {
        let bytes = npy::real(shape, self.precision, values);
        self.put(&format!("{name}.npy"), &bytes);
    }

    /// A complex field `name`, `values` in C order for `shape`, at the archive's precision.
    ///
    /// # Panics
    ///
    /// If `values.len()` isn't `shape`'s product.
    pub fn complex_field(&mut self, name: &str, shape: &[usize], values: &[c64]) {
        let bytes = npy::complex(shape, self.precision, values);
        self.put(&format!("{name}.npy"), &bytes);
    }

    /// A real array `name` always in `f64`, whatever the archive's precision: the grid and the
    /// small numbers, which downcasting would only lose precision for nothing.
    ///
    /// # Panics
    ///
    /// If `values.len()` isn't `shape`'s product.
    pub fn exact(&mut self, name: &str, shape: &[usize], values: &[f64]) {
        let bytes = npy::real(shape, Precision::F64, values);
        self.put(&format!("{name}.npy"), &bytes);
    }

    /// A complex array `name` always in `f64` (`complex128`).
    ///
    /// # Panics
    ///
    /// If `values.len()` isn't `shape`'s product.
    pub fn exact_complex(&mut self, name: &str, shape: &[usize], values: &[c64]) {
        let bytes = npy::complex(shape, Precision::F64, values);
        self.put(&format!("{name}.npy"), &bytes);
    }

    /// `meta.json`: `meta`'s provenance, the precision, and what the arrays are.
    pub fn meta(&mut self, meta: &Meta, described: &Described<'_>) {
        let text = meta_json(meta, self.precision, described, &self.compression());
        self.put("meta.json", text.as_bytes());
    }

    /// Writes the archive as `path`, returning its size in bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if `path` can't be written.
    pub fn write(self, path: &Path) -> Result<u64> {
        if let Some(e) = self.error {
            return Err(io_error(path, e));
        }
        let bytes = self.zip.finish().map_err(|e| io_error(path, e))?;
        std::fs::write(path, &bytes).map_err(|e| io_error(path, e))?;
        Ok(bytes.len() as u64)
    }
}

/// The six components of E and H, as the files name them.
pub const COMPONENTS: [&str; 6] = ["Ex", "Ey", "Ez", "Hx", "Hy", "Hz"];

/// `fields`' component `c` (0 for x, 1 for y, 2 for z) of `e` or `h`, at every cell in C order
/// (row j = y, column i = x), as [`crate::facade::vector_modes`] lays its own arrays out.
fn component(fields: &Fields, c: usize, h: bool) -> Vec<c64> {
    let (nx, ny) = (fields.x().len(), fields.y().len());
    (0..ny)
        .flat_map(|j| (0..nx).map(move |i| (i, j)))
        .map(|(i, j)| {
            if h {
                fields.h(i, j)[c]
            } else {
                fields.e(i, j)[c]
            }
        })
        .collect()
}

/// A guided or leaky mode's complex fields, written as `path`, an `.npz`: the components `keep`
/// keeps of [`COMPONENTS`], each `(ny, nx)`, with `x_um`, `y_um` (the cells' centres),
/// `wavelength_um`, `effective_index`, `te_fraction` and `power`. Returns the file's size.
///
/// # Errors
///
/// [`Error::Io`] if `path` can't be written.
#[allow(clippy::too_many_arguments)]
pub fn write_mode_npz(
    path: &Path,
    mode: &VectorMode,
    fields: &Fields,
    wavelength: Wavelength,
    meta: &Meta,
    precision: Precision,
    level: u8,
    keep: &dyn Fn(&str) -> bool,
) -> Result<u64> {
    let npz = Npz::new(precision, level);
    write_mode_into(npz, path, mode, fields, wavelength, meta, keep)
}

/// [`write_mode_npz`] into `npz`, an archive made with its precision and compression
/// ([`Npz::new`], [`Npz::compressed`]).
///
/// # Errors
///
/// [`Error::Io`] if `path` can't be written.
pub fn write_mode_into(
    mut npz: Npz,
    path: &Path,
    mode: &VectorMode,
    fields: &Fields,
    wavelength: Wavelength,
    meta: &Meta,
    keep: &dyn Fn(&str) -> bool,
) -> Result<u64> {
    let (nx, ny) = (fields.x().len(), fields.y().len());
    npz.exact("x_um", &[nx], fields.x());
    npz.exact("y_um", &[ny], fields.y());
    npz.exact("wavelength_um", &[], &[wavelength.to_um()]);
    npz.exact_complex("effective_index", &[], &[mode.effective_index()]);
    npz.exact("te_fraction", &[], &[mode.te_fraction()]);
    npz.exact("power", &[], &[fields.power()]);
    for (k, name) in COMPONENTS.iter().enumerate() {
        if keep(name) {
            npz.complex_field(name, &[ny, nx], &component(fields, k % 3, k >= 3));
        }
    }
    npz.meta(
        meta,
        &Described {
            grid: "x_um and y_um name each cell's centre (crate::mode::fields::Fields), not the \
                   solver's own nodes; each field (ny, nx), row j at y_um[j], column i at x_um[i]",
            units: "um for lengths; E in units of Z0 (free space impedance) times H, the \
                    solver's own; power in units of Z0 |H|^2 um^2 (Fields::power)",
            normalization: "none: the solver's own amplitude, not scaled to unit power (divide \
                            by sqrt(power) for unit power)",
        },
    );
    npz.write(path)
}

/// An upper bound on [`write_mode_npz`]'s file size before writing it, for a grid of `nx` by
/// `ny` cells at `precision`, all six components kept: every array's bytes (deflate only
/// shrinks them further) plus each `.npy`'s header and the zip's own overhead.
pub fn estimate_mode_npz_bytes(nx: usize, ny: usize, precision: Precision) -> u64 {
    let item = match precision {
        Precision::F32 => 4u64,
        Precision::F64 => 8,
    };
    let cells = (nx * ny) as u64;
    // x_um, y_um, Ex, Ey, Ez, Hx, Hy, Hz, wavelength_um, effective_index, te_fraction, power,
    // meta.json: 13 entries, each a `.npy` header (at most 128 bytes for these small, fixed
    // dicts) plus a local and a central directory record (about 90 bytes and the name)
    const ENTRIES: u64 = 13;
    const OVERHEAD_PER_ENTRY: u64 = 128 + 128;
    let data = 8 * (nx as u64 + ny as u64) // the grid, always f64
        + 6 * cells * item * 2
        + 4 * 32 // wavelength_um, effective_index, te_fraction, power: small f64 scalars
        + 4096; // meta.json, generously, without the job's text
    data + ENTRIES * OVERHEAD_PER_ENTRY
}

/// A reader of the `.npz` files for the tests: the `zip` crate (a dev-dependency) unpacks the
/// archive, and each `.npy` is parsed here.
#[cfg(test)]
pub(crate) mod read {
    use std::collections::BTreeMap;
    use std::io::Read as _;

    use num_complex::Complex64 as c64;

    /// An array read back: its `descr`, its shape and its raw little-endian bytes.
    #[derive(Debug)]
    pub(crate) struct Array {
        pub(crate) descr: String,
        pub(crate) shape: Vec<usize>,
        pub(crate) data: Vec<u8>,
    }

    impl Array {
        pub(crate) fn f64s(&self) -> Vec<f64> {
            assert_eq!(self.descr, "<f8");
            let (words, _) = self.data.as_chunks::<8>();
            words.iter().map(|w| f64::from_le_bytes(*w)).collect()
        }

        pub(crate) fn c64s(&self) -> Vec<c64> {
            assert_eq!(self.descr, "<c16");
            self.f64s_raw()
                .chunks(2)
                .map(|z| c64::new(z[0], z[1]))
                .collect()
        }

        fn f64s_raw(&self) -> Vec<f64> {
            let (words, _) = self.data.as_chunks::<8>();
            words.iter().map(|w| f64::from_le_bytes(*w)).collect()
        }
    }

    /// Every member of the `.npz` at `path`: its arrays by name (`.npy` left off), and
    /// `meta.json`'s text.
    pub(crate) fn npz(path: &std::path::Path) -> (BTreeMap<String, Array>, String) {
        let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let mut archive = ::zip::ZipArchive::new(file).unwrap();
        let mut arrays = BTreeMap::new();
        let mut meta = String::new();
        for k in 0..archive.len() {
            let mut entry = archive.by_index(k).unwrap();
            let name = entry.name().to_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if name == "meta.json" {
                meta = String::from_utf8(bytes).unwrap();
                continue;
            }
            assert_eq!(&bytes[..6], b"\x93NUMPY");
            let len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
            let header = std::str::from_utf8(&bytes[10..10 + len]).unwrap();
            let descr = header.split("'descr': '").nth(1).unwrap();
            let descr = descr.split('\'').next().unwrap().to_owned();
            let shape = header.split("'shape': (").nth(1).unwrap();
            let shape = shape
                .split(')')
                .next()
                .unwrap()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.parse().unwrap())
                .collect();
            let name = name.strip_suffix(".npy").unwrap().to_owned();
            arrays.insert(
                name,
                Array {
                    descr,
                    shape,
                    data: bytes[10 + len..].to_vec(),
                },
            );
        }
        (arrays, meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::vector::{modes, strip};
    use std::io::Read as _;

    fn test_mode() -> (VectorMode, Fields, Wavelength) {
        let cs = strip(0.02);
        let w = Wavelength::from_um_unchecked(1.55);
        let m = modes(&cs, w, 1, None)
            .expect("the solver converges")
            .remove(0);
        let fields = m.fields(&cs).expect("the mode is on this grid");
        (m, fields, w)
    }

    /// Unpacks `.npy` bytes (as `write_mode_npz`'s own writer frames them): the header dict's
    /// `shape` and `descr`, and the raw data after it.
    fn parse_npy(bytes: &[u8]) -> (String, Vec<usize>, Vec<u8>) {
        assert_eq!(&bytes[..6], b"\x93NUMPY");
        let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        let header = std::str::from_utf8(&bytes[10..10 + header_len]).unwrap();
        let descr = header
            .split("'descr': '")
            .nth(1)
            .unwrap()
            .split('\'')
            .next()
            .unwrap()
            .to_owned();
        let shape_str = header
            .split("'shape': (")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        let shape = shape_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        (descr, shape, bytes[10 + header_len..].to_vec())
    }

    fn read_f64(data: &[u8]) -> Vec<f64> {
        data.as_chunks::<8>()
            .0
            .iter()
            .map(|c| f64::from_le_bytes(*c))
            .collect()
    }

    fn read_c64(data: &[u8]) -> Vec<c64> {
        data.as_chunks::<16>()
            .0
            .iter()
            .map(|c| {
                c64::new(
                    f64::from_le_bytes(c[0..8].try_into().unwrap()),
                    f64::from_le_bytes(c[8..16].try_into().unwrap()),
                )
            })
            .collect()
    }

    /// The `.npz` a real, if minimal, ZIP reader (the `zip` crate, a dev-dependency) opens, with
    /// every array read back to the bit of the solver's own [`Fields`] and [`VectorMode`].
    #[test]
    fn mode_npz_round_trips_to_the_bit() {
        let (m, fields, w) = test_mode();
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-export-test-{:?}",
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mode.npz");
        let meta = Meta::new("export-test", "run-1").with_job_text("name = \"export-test\"");
        write_mode_npz(&path, &m, &fields, w, &meta, Precision::F64, 6, &|_| true).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let mut archive = ::zip::ZipArchive::new(file).unwrap();
        let mut read_entry = |name: &str| -> Vec<u8> {
            let mut entry = archive.by_name(name).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            data
        };

        let (nx, ny) = (fields.x().len(), fields.y().len());
        let (descr, shape, data) = parse_npy(&read_entry("x_um.npy"));
        assert_eq!(descr, "<f8");
        assert_eq!(shape, vec![nx]);
        assert_eq!(read_f64(&data), fields.x());

        let (_, shape, data) = parse_npy(&read_entry("y_um.npy"));
        assert_eq!(shape, vec![ny]);
        assert_eq!(read_f64(&data), fields.y());

        let (descr, shape, data) = parse_npy(&read_entry("Ex.npy"));
        assert_eq!(descr, "<c16");
        assert_eq!(shape, vec![ny, nx]);
        assert_eq!(read_c64(&data), component(&fields, 0, false));

        let (_, _, data) = parse_npy(&read_entry("Hz.npy"));
        assert_eq!(read_c64(&data), component(&fields, 2, true));

        let (_, shape, data) = parse_npy(&read_entry("effective_index.npy"));
        assert!(shape.is_empty());
        assert_eq!(read_c64(&data)[0], m.effective_index());

        let (_, _, data) = parse_npy(&read_entry("te_fraction.npy"));
        assert_eq!(read_f64(&data)[0], m.te_fraction());

        let (_, _, data) = parse_npy(&read_entry("power.npy"));
        assert_eq!(read_f64(&data)[0], fields.power());

        let meta_bytes = read_entry("meta.json");
        let text = std::str::from_utf8(&meta_bytes).unwrap();
        assert!(text.contains("\"job\": \"export-test\""));
        assert!(text.contains("\"precision\": \"f64\""));
        assert!(text.contains("\"job_text\": \"name = \\\"export-test\\\"\""));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f32_precision_downcasts_and_says_so() {
        let (m, fields, w) = test_mode();
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-export-test-f32-{:?}",
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mode.npz");
        let meta = Meta::new("export-test", "");
        let full =
            write_mode_npz(&path, &m, &fields, w, &meta, Precision::F64, 0, &|_| true).unwrap();
        let half =
            write_mode_npz(&path, &m, &fields, w, &meta, Precision::F32, 0, &|_| true).unwrap();
        assert!(half < full, "{half} should be smaller than {full}");

        let file = std::fs::File::open(&path).unwrap();
        let mut archive = ::zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name("Ex.npy").unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        let (descr, _, _) = parse_npy(&data);
        assert_eq!(descr, "<c8");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The members of a ZIP as its local headers have them: each one's name, method and
    /// payload, walked from the start (this writer's archives have no data descriptors).
    fn members(bytes: &[u8]) -> Vec<(String, u16, Vec<u8>)> {
        let u16_at = |k: usize| u16::from_le_bytes([bytes[k], bytes[k + 1]]);
        let u32_at = |k: usize| u32::from_le_bytes(bytes[k..k + 4].try_into().unwrap());
        let mut out = Vec::new();
        let mut k = 0;
        while u32_at(k) == 0x0403_4b50 {
            let method = u16_at(k + 8);
            let size = u32_at(k + 18) as usize;
            let name_len = u16_at(k + 26) as usize;
            let extra = u16_at(k + 28) as usize;
            let name = String::from_utf8(bytes[k + 30..k + 30 + name_len].to_vec()).unwrap();
            let start = k + 30 + name_len + extra;
            out.push((name, method, bytes[start..start + size].to_vec()));
            k = start + size;
        }
        out
    }

    /// An external library's compressor, here one of the test's own that reverses the bytes:
    /// each member stored as its method, its payload the compressor's, of the same `.npy` bytes
    /// the stored archive holds; `meta.json` names it.
    #[test]
    fn a_compressor_of_a_library_writes_each_member_by_its_own_method() {
        struct Reverse;
        impl codec::Compressor for Reverse {
            fn name(&self) -> &str {
                "reverse"
            }
            fn version(&self) -> String {
                "0.1".into()
            }
            fn deterministic(&self) -> bool {
                true
            }
            fn levels(&self) -> (i32, i32) {
                (1, 1)
            }
            fn default_level(&self) -> i32 {
                1
            }
            fn compress(&self, data: &[u8], _: i32) -> Result<Vec<u8>> {
                Ok(data.iter().rev().copied().collect())
            }
            fn zip_method(&self) -> u16 {
                93
            }
        }
        let (m, fields, w) = test_mode();
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-export-codec-{:?}",
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let meta = Meta::new("codec", "");
        let (stored, reversed) = (dir.join("stored.npz"), dir.join("reversed.npz"));
        let npz = Npz::new(Precision::F64, 0);
        write_mode_into(npz, &stored, &m, &fields, w, &meta, &|_| true).unwrap();
        let npz = Npz::compressed(Precision::F64, std::sync::Arc::new(Reverse), 1);
        write_mode_into(npz, &reversed, &m, &fields, w, &meta, &|_| true).unwrap();
        let plain = members(&std::fs::read(&stored).unwrap());
        let coded = members(&std::fs::read(&reversed).unwrap());
        assert_eq!(plain.len(), coded.len());
        for ((name, method, data), (coded_name, coded_method, payload)) in plain.iter().zip(&coded)
        {
            assert_eq!((name, *method), (coded_name, 0));
            assert_eq!(*coded_method, 93, "{name}");
            if name == "meta.json" {
                let text: String =
                    String::from_utf8(payload.iter().rev().copied().collect()).unwrap();
                assert!(
                    text.contains("\"compression\": \"reverse 0.1, level 1\""),
                    "{text}"
                );
            } else {
                let back: Vec<u8> = payload.iter().rev().copied().collect();
                assert_eq!(&back, data, "{name}");
            }
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn size_estimate_is_an_upper_bound() {
        let (_, fields, _) = test_mode();
        let (nx, ny) = (fields.x().len(), fields.y().len());
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-export-test-size-{:?}",
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mode.npz");
        let (m, _, w) = test_mode();
        let meta = Meta::new("x", "");
        // level 0 (store): no compression, so the real file can't be larger than the estimate
        let written =
            write_mode_npz(&path, &m, &fields, w, &meta, Precision::F64, 0, &|_| true).unwrap();
        let estimated = estimate_mode_npz_bytes(nx, ny, Precision::F64);
        assert!(
            written <= estimated,
            "{written} should be at most {estimated}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
