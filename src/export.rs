//! Exporting a run's results, so they can be taken anywhere, not just drawn (issue #431).
//!
//! A `"modes"` job's `[task.export]` writes, into the run's directory, each solved mode's
//! complex fields as an `.npz` (`mode-<k of n>.npz`) and a CSV summary of every mode
//! (`modes.csv`). FDFD, FDTD and the static solvers' fields, and MAT (MATLAB) and VTK XML
//! writers, are left for later PRs under the same issue; `events.jsonl`'s own pictures
//! ([`crate::job::Event::Mode`], [`crate::job::Event::ModeField`]) stay as they are (`f32`,
//! scaled to a peak of 1, no phase): this is the fields themselves.
//!
//! **The `.npz`.** NumPy's own format: a ZIP (written from PKWARE's *APPNOTE*) of `.npy` arrays
//! (NEP 1), deflated at a selectable level, store (level 0) to smallest (9). Its arrays:
//!
//! - `x_um`, `y_um`: the cells' centres, as [`crate::mode::fields::Fields`] has them (not the
//!   solver's own nodes);
//! - `wavelength_um`, `effective_index` (complex), `te_fraction`, `power` (half the real part of
//!   ∫ E×H*·ẑ, [`Fields::power`]'s units): always `f64`, small enough that downcasting them
//!   would only lose precision for nothing;
//! - `Ex`, `Ey`, `Ez`, `Hx`, `Hy`, `Hz`: each `(ny, nx)` in C order, cell (i, j) at row j, column
//!   i, as [`crate::facade::vector_modes`] lays its own fields out; `f32` or `f64`, the
//!   solver's own units (E in units of Z₀ times H's: [`crate::mode::fields`]), not scaled to
//!   unit power;
//! - `meta.json`, a plain JSON member beside the arrays: `numpy.load`'s `NpzFile` only reads
//!   members that end in `.npy`, so it is never mistaken for one, and names photonoxide's
//!   version, the precision written, and the job and run.
//!
//! Nothing here reads a `.npy` or `.npz` file back: `write_mode_npz`'s own tests do, with the
//! `zip` crate (a dev-dependency only) unpacking the archive and a small reader of this module's
//! own checking the array bytes against the mode's fields, to the bit.

mod npy;
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
#[derive(Clone, Debug)]
pub struct Meta {
    /// The job's name.
    pub job: String,
    /// The run's id: its directory's name (`Run::dir`'s file name), empty for a run kept only in
    /// memory.
    pub run_id: String,
}

/// `meta.json`'s fields.
#[derive(Serialize)]
struct MetaFile<'a> {
    photonoxide: &'a str,
    job: &'a str,
    run_id: &'a str,
    precision: &'static str,
    grid: &'static str,
    units: &'static str,
}

fn meta_json(meta: &Meta, precision: Precision) -> String {
    let file = MetaFile {
        photonoxide: env!("CARGO_PKG_VERSION"),
        job: &meta.job,
        run_id: &meta.run_id,
        precision: match precision {
            Precision::F64 => "f64",
            Precision::F32 => "f32, downcast from f64",
        },
        grid: "x_um and y_um name each cell's centre (crate::mode::fields::Fields), not the \
               solver's own nodes",
        units: "E in units of Z0 (free space impedance) times H, the solver's own, not scaled \
                to unit power; power is in units of Z0 |H|^2 um^2 (Fields::power)",
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

/// A guided or leaky mode's complex fields, written as `path`, an `.npz`.
///
/// # Errors
///
/// [`Error::Io`] if `path` can't be written.
pub fn write_mode_npz(
    path: &Path,
    mode: &VectorMode,
    fields: &Fields,
    wavelength: Wavelength,
    meta: &Meta,
    precision: Precision,
    level: u8,
) -> Result<u64> {
    let (nx, ny) = (fields.x().len(), fields.y().len());
    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let mut put = |name: &str, data: &[u8]| -> Result<()> {
            writer.add(name, data, level).map_err(|e| io_error(path, e))
        };
        put("x_um.npy", &npy::real(&[nx], precision, fields.x()))?;
        put("y_um.npy", &npy::real(&[ny], precision, fields.y()))?;
        put(
            "wavelength_um.npy",
            &npy::real(&[], Precision::F64, &[wavelength.to_um()]),
        )?;
        put(
            "effective_index.npy",
            &npy::complex(&[], Precision::F64, &[mode.effective_index()]),
        )?;
        put(
            "te_fraction.npy",
            &npy::real(&[], Precision::F64, &[mode.te_fraction()]),
        )?;
        put(
            "power.npy",
            &npy::real(&[], Precision::F64, &[fields.power()]),
        )?;
        for (name, c) in [("Ex", 0), ("Ey", 1), ("Ez", 2)] {
            put(
                &format!("{name}.npy"),
                &npy::complex(&[ny, nx], precision, &component(fields, c, false)),
            )?;
        }
        for (name, c) in [("Hx", 0), ("Hy", 1), ("Hz", 2)] {
            put(
                &format!("{name}.npy"),
                &npy::complex(&[ny, nx], precision, &component(fields, c, true)),
            )?;
        }
        put("meta.json", meta_json(meta, precision).as_bytes())?;
        writer.finish().map_err(|e| io_error(path, e))?;
    }
    std::fs::write(path, &buf).map_err(|e| io_error(path, e))?;
    Ok(buf.len() as u64)
}

/// An upper bound on [`write_mode_npz`]'s file size before writing it, for a grid of `nx` by
/// `ny` cells at `precision`: every array's bytes (deflate only shrinks them further) plus each
/// `.npy`'s header and the zip's own overhead.
pub fn estimate_mode_npz_bytes(nx: usize, ny: usize, precision: Precision) -> u64 {
    let item = match precision {
        Precision::F32 => 4u64,
        Precision::F64 => 8,
    };
    let cells = (nx * ny) as u64;
    let real_bytes = |elements: u64| elements * item;
    let complex_bytes = |elements: u64| elements * item * 2;
    // x_um, y_um, Ex, Ey, Ez, Hx, Hy, Hz, wavelength_um, effective_index, te_fraction, power,
    // meta.json: 13 entries, each a `.npy` header (at most 128 bytes for these small, fixed
    // dicts) plus a local and a central directory record (about 90 bytes and the name)
    const ENTRIES: u64 = 13;
    const OVERHEAD_PER_ENTRY: u64 = 128 + 128;
    let data = real_bytes(nx as u64)
        + real_bytes(ny as u64)
        + 6 * complex_bytes(cells)
        + 4 * 32 // wavelength_um, effective_index, te_fraction, power: small f64 scalars
        + 1024; // meta.json, generously
    data + ENTRIES * OVERHEAD_PER_ENTRY
}

/// A `"modes"` job's mode, one line of [`write_modes_csv`]'s table.
pub struct ModeRow {
    /// What it is, e.g. `"mode 1 of 2"`.
    pub label: String,
    /// The vacuum wavelength, µm.
    pub wavelength_um: f64,
    /// The effective index.
    pub effective_index: c64,
    /// Its loss along the guide, dB/cm ([`crate::mode::dispersion::loss_db_per_cm`]).
    pub loss_db_per_cm: f64,
    /// The share of the transverse magnetic field in H_y.
    pub te_fraction: f64,
}

/// `rows` as `path`, a CSV with a header line and one line a row, each number at full `f64`
/// precision (17 significant digits: lossless, reading back to the same bits).
///
/// # Errors
///
/// [`Error::Io`] if `path` can't be written.
pub fn write_modes_csv(path: &Path, rows: &[ModeRow]) -> Result<()> {
    let mut text =
        String::from("label,wavelength_um,n_eff_re,n_eff_im,loss_db_per_cm,te_fraction\n");
    for r in rows {
        text.push_str(&format!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",
            r.label,
            r.wavelength_um,
            r.effective_index.re,
            r.effective_index.im,
            r.loss_db_per_cm,
            r.te_fraction
        ));
    }
    std::fs::write(path, text).map_err(|e| io_error(path, e))
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
        let meta = Meta {
            job: "export-test".into(),
            run_id: "run-1".into(),
        };
        write_mode_npz(&path, &m, &fields, w, &meta, Precision::F64, 6).unwrap();

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
        let meta = Meta {
            job: "export-test".into(),
            run_id: String::new(),
        };
        let full = write_mode_npz(&path, &m, &fields, w, &meta, Precision::F64, 0).unwrap();
        let half = write_mode_npz(&path, &m, &fields, w, &meta, Precision::F32, 0).unwrap();
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

    #[test]
    fn csv_round_trips_to_the_bit() {
        let (m, _fields, w) = test_mode();
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-export-test-csv-{:?}",
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("modes.csv");
        let loss = crate::mode::dispersion::loss_db_per_cm(m.effective_index(), w);
        write_modes_csv(
            &path,
            &[ModeRow {
                label: "mode 1 of 1".into(),
                wavelength_um: w.to_um(),
                effective_index: m.effective_index(),
                loss_db_per_cm: loss,
                te_fraction: m.te_fraction(),
            }],
        )
        .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        assert_eq!(
            lines.next().unwrap(),
            "label,wavelength_um,n_eff_re,n_eff_im,loss_db_per_cm,te_fraction"
        );
        let row = lines.next().unwrap();
        let fields: Vec<&str> = row.split(',').collect();
        assert_eq!(fields[0], "mode 1 of 1");
        assert_eq!(fields[1].parse::<f64>().unwrap(), w.to_um());
        assert_eq!(fields[2].parse::<f64>().unwrap(), m.effective_index().re);
        assert_eq!(fields[3].parse::<f64>().unwrap(), m.effective_index().im);
        assert_eq!(fields[4].parse::<f64>().unwrap(), loss);
        assert_eq!(fields[5].parse::<f64>().unwrap(), m.te_fraction());

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
        let meta = Meta {
            job: "x".into(),
            run_id: String::new(),
        };
        // level 0 (store): no compression, so the real file can't be larger than the estimate
        let written = write_mode_npz(&path, &m, &fields, w, &meta, Precision::F64, 0).unwrap();
        let estimated = estimate_mode_npz_bytes(nx, ny, Precision::F64);
        assert!(
            written <= estimated,
            "{written} should be at most {estimated}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
