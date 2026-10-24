//! A run's `.npz` written again at another precision and deflate level, and its size estimated
//! before writing it (issue #431): what the studio's Export dialog, `photonoxide export` and the
//! façade's `export_fields` take a finished run's fields with.
//!
//! The arrays are read from the file photonoxide wrote (its members stored or deflated, read
//! with `flate2`'s pure-Rust inflate) and written by the same writer: the grid and the small
//! numbers, the arrays of no or one dimension, stay `f64`; the fields, of two dimensions or
//! more, are downcast to `f32` when asked. A field already `f32` stays so (nothing brings its
//! digits back), and `meta.json` says what each file holds. Written again at the precision and
//! level it has, a file is the same file, byte for byte.
//!
//! **The estimate.** Stored (level 0), the size is known exactly before writing: each member's
//! `.npy` bytes and the ZIP's own records. Deflated, a member up to 1 MiB is deflated in memory
//! as the writer would, so its size is exact too; a larger one is estimated from eight slices of
//! 64 KiB spread over it, each deflated at the level asked, their ratio taken for the whole.

use std::io::Read as _;
use std::path::Path;

use super::{MetaFile, Precision, npy, precision_text, zip};
use crate::{Error, Result};

/// A member deflated whole for the estimate, up to this size.
const WHOLE: usize = 1 << 20;
/// The slices a larger member's estimate deflates, and each one's size.
const SLICES: usize = 8;
const SLICE: usize = 64 << 10;

/// A file's size, written again, as estimated before writing it ([`estimate`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Estimate {
    /// Its size, bytes.
    pub bytes: u64,
    /// Whether that is the size the file will have, to the byte (every member stored, or
    /// deflated whole for the estimate), rather than an estimate from slices of it.
    pub exact: bool,
    /// The precision its fields will have: the one asked, or `f32` for a file whose fields
    /// already are.
    pub precision: Precision,
}

/// A member of an `.npz`: its name and its bytes, uncompressed.
struct Member {
    name: String,
    data: Vec<u8>,
}

fn io(path: &Path, e: impl std::fmt::Display) -> Error {
    Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

fn bad(path: &Path, reason: impl Into<String>) -> Error {
    Error::Parse {
        what: path.display().to_string(),
        reason: reason.into(),
    }
}

/// The members of the `.npz` at `path`, from its local headers (photonoxide's writer puts no
/// data descriptor after a member), each inflated.
fn members(path: &Path) -> Result<Vec<Member>> {
    let bytes = std::fs::read(path).map_err(|e| io(path, e))?;
    let u16_at = |k: usize| {
        bytes
            .get(k..k + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |k: usize| {
        bytes
            .get(k..k + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut out = Vec::new();
    let mut k = 0;
    while u32_at(k) == Some(0x0403_4b50) {
        let short = || bad(path, "a ZIP member is cut short");
        let flags = u16_at(k + 6).ok_or_else(short)?;
        let method = u16_at(k + 8).ok_or_else(short)?;
        let size = u32_at(k + 18).ok_or_else(short)? as usize;
        let full = u32_at(k + 22).ok_or_else(short)? as usize;
        let name_len = usize::from(u16_at(k + 26).ok_or_else(short)?);
        let extra = usize::from(u16_at(k + 28).ok_or_else(short)?);
        if flags & 0x08 != 0 {
            return Err(bad(
                path,
                "a ZIP member with a data descriptor isn't photonoxide's",
            ));
        }
        let name = bytes
            .get(k + 30..k + 30 + name_len)
            .ok_or_else(short)
            .map(|b| String::from_utf8_lossy(b).into_owned())?;
        let start = k + 30 + name_len + extra;
        let payload = bytes.get(start..start + size).ok_or_else(short)?;
        let data = match method {
            0 => payload.to_vec(),
            8 => {
                let mut data = Vec::with_capacity(full);
                flate2::read::DeflateDecoder::new(payload)
                    .read_to_end(&mut data)
                    .map_err(|e| io(path, e))?;
                data
            }
            93 => {
                return Err(Error::invalid(
                    "field file",
                    format!(
                        "{} is compressed with Zstandard, which photonoxide reads only through \
                         libzstd: it can be copied as it is, not written again",
                        path.display()
                    ),
                ));
            }
            m => {
                return Err(bad(
                    path,
                    format!("ZIP method {m} isn't one photonoxide writes"),
                ));
            }
        };
        if crc32fast::hash(&data) != u32_at(k + 14).ok_or_else(short)? {
            return Err(bad(path, format!("{name}'s CRC-32 doesn't match")));
        }
        out.push(Member { name, data });
        k = start + size;
    }
    if out.is_empty() {
        return Err(bad(path, "not an .npz of photonoxide's"));
    }
    Ok(out)
}

/// A `.npy`'s `descr`, shape and where its data starts.
fn npy_header(data: &[u8]) -> Option<(String, Vec<usize>, usize)> {
    if data.get(..6)? != b"\x93NUMPY" || *data.get(6)? != 1 {
        return None;
    }
    let len = usize::from(u16::from_le_bytes([*data.get(8)?, *data.get(9)?]));
    let header = std::str::from_utf8(data.get(10..10 + len)?).ok()?;
    let descr = header.split("'descr': '").nth(1)?.split('\'').next()?;
    let shape = header.split("'shape': (").nth(1)?.split(')').next()?;
    let shape = shape
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().ok())
        .collect::<Option<Vec<usize>>>()?;
    Some((descr.to_owned(), shape, 10 + len))
}

/// The members of the `.npz` at `path` as they are written again at `precision` (its fields
/// downcast if asked), with `meta.json` saying so and naming `compression`; and the precision
/// its fields then have.
fn converted(
    path: &Path,
    precision: Precision,
    compression: &str,
) -> Result<(Vec<Member>, Precision)> {
    let mut members = members(path)?;
    // a file whose fields are all f32 already stays f32 whatever is asked
    let mut has = Precision::F64;
    for m in &members {
        if let Some((descr, shape, _)) = npy_header(&m.data)
            && shape.len() >= 2
            && matches!(descr.as_str(), "<f4" | "<c8")
        {
            has = Precision::F32;
        }
    }
    let to = if has == Precision::F32 {
        Precision::F32
    } else {
        precision
    };
    for m in &mut members {
        if m.name == "meta.json" {
            let text = String::from_utf8_lossy(&m.data).into_owned();
            let mut meta: MetaFile<'_> = serde_json::from_str(&text)
                .map_err(|e| bad(path, format!("its meta.json isn't photonoxide's: {e}")))?;
            meta.precision = precision_text(to).into();
            meta.compression = compression.to_owned().into();
            m.data = meta.text().into_bytes();
            continue;
        }
        let Some((descr, shape, start)) = npy_header(&m.data) else {
            return Err(bad(path, format!("{} isn't a .npy", m.name)));
        };
        if to == Precision::F64 || shape.len() < 2 {
            continue;
        }
        let narrow = match descr.as_str() {
            "<f8" => "<f4",
            "<c16" => "<c8",
            _ => continue,
        };
        let (words, _) = m.data[start..].as_chunks::<8>();
        let mut out = npy::header_for(narrow, &shape);
        out.reserve(words.len() * 4);
        for w in words {
            out.extend_from_slice(&(f64::from_le_bytes(*w) as f32).to_le_bytes());
        }
        m.data = out;
    }
    Ok((members, to))
}

/// What `meta.json` says of members deflated at `level`, as [`super::Npz`] writes it.
fn compression(level: u8) -> String {
    if level == 0 {
        "stored".into()
    } else {
        format!("deflate, level {level}")
    }
}

/// The ZIP's own bytes for a member named `name`: its local and central records.
fn records(name: &str) -> u64 {
    30 + 46 + 2 * name.len() as u64
}

/// The bytes `data` takes deflated at `level`, as the writer deflates it.
fn deflated(data: &[u8], level: u8) -> u64 {
    use std::io::Write as _;
    let mut enc =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(u32::from(level)));
    // writing into memory cannot fail
    enc.write_all(data).expect("deflate into memory");
    enc.finish().expect("deflate into memory").len() as u64
}

/// The size the `.npz` at `path` has written again at `precision` and deflate `level` (0
/// stores, 1 to 9 deflate; above 9 is 9), before writing it: exact when stored, and for members
/// up to 1 MiB; from slices of the larger ones.
///
/// # Errors
///
/// [`Error::Io`] if the file can't be read, [`Error::Parse`] if it isn't an `.npz` photonoxide
/// wrote, and [`Error::InvalidValue`] for one compressed with Zstandard.
pub fn estimate(path: &Path, precision: Precision, level: u8) -> Result<Estimate> {
    let level = level.min(9);
    let (members, to) = converted(path, precision, &compression(level))?;
    let mut bytes = 22; // the end of the central directory
    let mut exact = true;
    for m in &members {
        bytes += records(&m.name);
        bytes += if level == 0 {
            m.data.len() as u64
        } else if m.data.len() <= WHOLE {
            deflated(&m.data, level)
        } else {
            exact = false;
            let gap = (m.data.len() - SLICE) / (SLICES - 1);
            let (raw, small) = (0..SLICES)
                .map(|k| &m.data[k * gap..k * gap + SLICE])
                .fold((0u64, 0u64), |(r, s), slice| {
                    (r + slice.len() as u64, s + deflated(slice, level))
                });
            (m.data.len() as u64 * small).div_ceil(raw)
        };
    }
    Ok(Estimate {
        bytes,
        exact,
        precision: to,
    })
}

/// Writes the `.npz` at `path` again as `to`, at `precision` and deflate `level` (0 stores, 1
/// to 9 deflate; above 9 is 9), returning its size: the same arrays in the same order, the
/// fields downcast when `f32` is asked, `meta.json` saying so. `to` must be another file.
///
/// # Errors
///
/// As [`estimate`]; [`Error::InvalidValue`] if `to` is `path`; [`Error::Io`] if `to` can't be
/// written.
pub fn rewrite(path: &Path, to: &Path, precision: Precision, level: u8) -> Result<u64> {
    let level = level.min(9);
    let same = |a: &Path| std::fs::canonicalize(a).ok();
    if path == to || (same(to).is_some() && same(path) == same(to)) {
        return Err(Error::invalid(
            "field file",
            format!("{} would be written over itself", path.display()),
        ));
    }
    let (members, _) = converted(path, precision, &compression(level))?;
    let mut zip = zip::ZipWriter::new(Vec::new());
    for m in &members {
        zip.add(&m.name, &m.data, level).map_err(|e| io(to, e))?;
    }
    let bytes = zip.finish().map_err(|e| io(to, e))?;
    std::fs::write(to, &bytes).map_err(|e| io(to, e))?;
    Ok(bytes.len() as u64)
}

/// For the jobs' tests: the `.npz` a job wrote, estimated and written again at each precision
/// and at levels 0, 1, 6 and 9, its estimate the written size to the byte where it says it is
/// exact and within 2 % where it doesn't. Returns each estimate and size.
#[cfg(test)]
pub(crate) fn assert_estimates(path: &Path) -> Vec<(Precision, u8, Estimate, u64)> {
    let to = path.with_extension("again.npz");
    let mut out = Vec::new();
    for precision in [Precision::F64, Precision::F32] {
        for level in [0, 1, 6, 9] {
            let e = estimate(path, precision, level).unwrap();
            let written = rewrite(path, &to, precision, level).unwrap();
            assert_eq!(written, std::fs::metadata(&to).unwrap().len());
            if e.exact {
                assert_eq!(e.bytes, written, "{path:?} {precision:?} level {level}");
            } else {
                let off = (e.bytes as f64 / written as f64 - 1.0).abs();
                assert!(
                    off < 0.02,
                    "{path:?} {precision:?} {level}: {e:?}, {written}"
                );
            }
            assert!(e.exact || level > 0, "stored, the size is known");
            out.push((precision, level, e, written));
        }
    }
    std::fs::remove_file(&to).ok();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{Described, Meta, Npz};

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-rewrite-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// An `.npz` of a smooth complex field of `ny` by `nx` (the size of a 3D run's monitor when
    /// large) with its grid, as the jobs write theirs.
    fn field_npz(path: &Path, nx: usize, ny: usize, precision: Precision, level: u8) -> u64 {
        let mut npz = Npz::new(precision, level);
        let x: Vec<f64> = (0..nx).map(|i| i as f64 * 0.02).collect();
        let y: Vec<f64> = (0..ny).map(|j| j as f64 * 0.02).collect();
        npz.exact("x_um", &[nx], &x);
        npz.exact("y_um", &[ny], &y);
        npz.exact("wavelength_um", &[], &[1.55]);
        let field: Vec<num_complex::Complex64> = (0..nx * ny)
            .map(|k| {
                let (i, j) = ((k % nx) as f64, (k / nx) as f64);
                let envelope = (-((i - nx as f64 / 2.0).powi(2) + (j - ny as f64 / 3.0).powi(2))
                    / (nx as f64 * 4.0))
                    .exp();
                num_complex::Complex64::from_polar(envelope, 0.31 * i + 0.17 * j)
            })
            .collect();
        npz.complex_field("Ez", &[ny, nx], &field);
        npz.meta(
            &Meta::new("rewrite", "run-1").with_job_text("name: rewrite"),
            &Described {
                grid: "cell centres",
                units: "um",
                normalization: "none",
            },
        );
        npz.write(path).unwrap()
    }

    #[test]
    fn written_again_as_it_was_it_is_the_same_file() {
        let dir = scratch("same");
        for (precision, level) in [
            (Precision::F64, 6),
            (Precision::F32, 0),
            (Precision::F64, 9),
        ] {
            let (a, b) = (dir.join("a.npz"), dir.join("b.npz"));
            field_npz(&a, 40, 30, precision, level);
            let n = rewrite(&a, &b, precision, level).unwrap();
            assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
            assert_eq!(n, std::fs::metadata(&b).unwrap().len());
        }
        assert!(rewrite(&dir.join("a.npz"), &dir.join("a.npz"), Precision::F32, 1).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Downcast, the fields are their `f32`s, the grid stays `f64`, and `meta.json` says so;
    /// asked for `f64` again, an `f32` file stays `f32`.
    #[test]
    fn downcast_fields_are_the_f32s_of_the_f64s() {
        let dir = scratch("downcast");
        let (a, b, c) = (dir.join("a.npz"), dir.join("b.npz"), dir.join("c.npz"));
        field_npz(&a, 50, 20, Precision::F64, 6);
        rewrite(&a, &b, Precision::F32, 3).unwrap();
        let (full, _) = crate::export::read::npz(&a);
        let (half, meta) = crate::export::read::npz(&b);
        assert_eq!(half["Ez"].descr, "<c8");
        assert_eq!(half["x_um"].descr, "<f8");
        assert_eq!(half["x_um"].data, full["x_um"].data);
        let narrow: Vec<u8> = full["Ez"]
            .c64s()
            .iter()
            .flat_map(|z| [z.re as f32, z.im as f32])
            .flat_map(f32::to_le_bytes)
            .collect();
        assert_eq!(half["Ez"].data, narrow);
        assert!(
            meta.contains("\"precision\": \"f32, downcast from f64\""),
            "{meta}"
        );
        assert!(
            meta.contains("\"compression\": \"deflate, level 3\""),
            "{meta}"
        );
        assert!(meta.contains("\"job_text\": \"name: rewrite\""), "{meta}");
        let e = estimate(&b, Precision::F64, 0).unwrap();
        assert_eq!(e.precision, Precision::F32);
        rewrite(&b, &c, Precision::F64, 0).unwrap();
        assert_eq!(crate::export::read::npz(&c).0["Ez"].descr, "<c8");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Stored, and deflated when every member is deflated whole, the estimate is the written
    /// file's size to the byte; a member of more than 1 MiB is estimated from its slices, here
    /// within 2 % of what is written.
    #[test]
    fn the_estimate_is_the_written_size() {
        let dir = scratch("estimate");
        let (a, b) = (dir.join("a.npz"), dir.join("b.npz"));
        field_npz(&a, 120, 80, Precision::F64, 6);
        for precision in [Precision::F64, Precision::F32] {
            for level in [0, 1, 6, 9] {
                let e = estimate(&a, precision, level).unwrap();
                let written = rewrite(&a, &b, precision, level).unwrap();
                assert!(e.exact);
                assert_eq!(e.bytes, written, "{precision:?} level {level}");
            }
        }
        // 600 by 400 complex f64s: 3.8 MB, sliced
        field_npz(&a, 600, 400, Precision::F64, 0);
        for (precision, level) in [
            (Precision::F64, 6),
            (Precision::F32, 9),
            (Precision::F64, 1),
        ] {
            let e = estimate(&a, precision, level).unwrap();
            let written = rewrite(&a, &b, precision, level).unwrap();
            assert!(!e.exact);
            let off = (e.bytes as f64 / written as f64 - 1.0).abs();
            assert!(
                off < 0.02,
                "{precision:?} level {level}: {} for {written}",
                e.bytes
            );
        }
        let e = estimate(&a, Precision::F32, 0).unwrap();
        assert!(e.exact);
        assert_eq!(e.bytes, rewrite(&a, &b, Precision::F32, 0).unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_file_that_isnt_photonoxides_npz_is_refused() {
        let dir = scratch("refused");
        let a = dir.join("a.npz");
        std::fs::write(&a, b"not a zip").unwrap();
        assert!(estimate(&a, Precision::F64, 6).is_err());
        assert!(estimate(&dir.join("none.npz"), Precision::F64, 6).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
