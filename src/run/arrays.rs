//! A run's heavy arrays, kept in `events.bin` beside `events.jsonl` (issue #431), so that a
//! large run's record stays small to read and quick to open.
//!
//! **What moves.** A line of the record of 4 KiB or more is looked through, and each of its
//! long values goes into `events.bin`:
//!
//! - a flat array of at least 256 numbers (a picture's values, a spectrum's), as `<f4` when
//!   every number is an `f32`'s shortest decimal, as `<f8` when it is an `f64`'s;
//! - a string of at least 1024 characters in base64 (RFC 4648, padded: an FDTD frame's
//!   pixels), as its bytes, `|u1`.
//!
//! Each is written as a NumPy `.npy` array (NEP 1, version 1.0, one dimension), one after
//! another, and its place in the line holds `{"$npy":<offset>}`, the byte at which its `.npy`
//! starts. A value is moved only when writing it back gives the very text the line had, which
//! is checked as it is moved: the line read back ([`inline`]) is the line the run recorded, to
//! the last byte, so a replayed run is the same run to the last bit of every float. The array
//! is written before the line that names it, so a live reader always finds it.
//!
//! From Python: `f = open(run / "events.bin", "rb"); f.seek(offset); a = numpy.load(f)`.
//!
//! **Older runs** have every array in the line itself, and read as they always did: a line
//! without a `{"$npy":` is left as it is. The text `{"$npy":` can't be inside a JSON string
//! (its quote would be escaped there), so only a reference reads as one.

use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// The file beside `events.jsonl` holding the record's heavy arrays.
pub const FILE: &str = "events.bin";

/// What a reference to an array starts with: `{"$npy":` and then its offset in [`FILE`].
const MARK: &str = "{\"$npy\":";

/// Lines shorter than this are written as they are, without looking through them.
const LINE: usize = 4096;

/// The fewest numbers a flat array has to be moved.
const NUMBERS: usize = 256;

/// The fewest characters a base64 string has to be moved.
const CHARACTERS: usize = 1024;

/// A value taken out of a line, as its array's element type and its elements.
enum Payload {
    F32(Vec<f32>),
    F64(Vec<f64>),
    Bytes(Vec<u8>),
}

impl Payload {
    /// The `.npy` bytes of the array.
    fn npy(&self) -> Vec<u8> {
        let (descr, n, data): (&str, usize, Vec<u8>) = match self {
            Payload::F32(v) => (
                "<f4",
                v.len(),
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Payload::F64(v) => (
                "<f8",
                v.len(),
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Payload::Bytes(v) => ("|u1", v.len(), v.clone()),
        };
        let mut out = crate::export::npy::header_for(descr, &[n]);
        out.extend_from_slice(&data);
        out
    }

    /// The JSON text the value had in the line.
    fn text(&self) -> String {
        match self {
            // a Vec of plain numbers always serializes
            Payload::F32(v) => serde_json::to_string(v).expect("numbers serialize"),
            Payload::F64(v) => serde_json::to_string(v).expect("numbers serialize"),
            Payload::Bytes(v) => format!("\"{}\"", base64(v)),
        }
    }
}

/// `line` with its heavy values taken out, and their `.npy` bytes, the first at `offset` in
/// [`FILE`]; `None` when it has none (a short line, or nothing long in it).
pub(super) fn split(line: &str, offset: u64) -> Option<(String, Vec<u8>)> {
    if line.len() < LINE {
        return None;
    }
    let spans = spans(line)?;
    if spans.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(line.len() / 4);
    let mut bytes = Vec::new();
    let mut last = 0;
    for (start, end, payload) in spans {
        out.push_str(&line[last..start]);
        out.push_str(MARK);
        out.push_str(&(offset + bytes.len() as u64).to_string());
        out.push('}');
        bytes.extend(payload.npy());
        last = end;
    }
    out.push_str(&line[last..]);
    Some((out, bytes))
}

/// The heavy values of `line`, each its byte range and what it holds, in order; `None` for a
/// line that isn't well-formed JSON as far as this reads it.
fn spans(line: &str) -> Option<Vec<(usize, usize, Payload)>> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                let end = string_end(b, i)?;
                let key = b[end + 1..]
                    .iter()
                    .find(|c| !c.is_ascii_whitespace())
                    .is_some_and(|&c| c == b':');
                let inner = &line[i + 1..end];
                if !key
                    && inner.len() >= CHARACTERS
                    && let Some(bytes) = unbase64(inner)
                    && base64(&bytes) == inner
                {
                    out.push((i, end + 1, Payload::Bytes(bytes)));
                }
                i = end + 1;
            }
            b'[' => match flat_numbers(b, i) {
                Some((end, count)) => {
                    let text = &line[i..=end];
                    if count >= NUMBERS
                        && let Some(p) = numbers(text)
                    {
                        out.push((i, end + 1, p));
                    }
                    i = end + 1;
                }
                None => i += 1,
            },
            _ => i += 1,
        }
    }
    Some(out)
}

/// The index of the quote that closes the string opened at `start`.
fn string_end(b: &[u8], start: usize) -> Option<usize> {
    let mut j = start + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return Some(j),
            _ => j += 1,
        }
    }
    None
}

/// Where the array opened at `start` closes and how many numbers it has, if it is a flat array
/// of numbers (nothing but numbers, commas and spaces).
fn flat_numbers(b: &[u8], start: usize) -> Option<(usize, usize)> {
    let number = |c: u8| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E');
    let skip = |mut j: usize| {
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        j
    };
    let mut j = skip(start + 1);
    if *b.get(j)? == b']' {
        return Some((j, 0));
    }
    let mut count = 0;
    loop {
        let first = j;
        while j < b.len() && number(b[j]) {
            j += 1;
        }
        if j == first {
            return None;
        }
        count += 1;
        j = skip(j);
        match *b.get(j)? {
            b',' => j = skip(j + 1),
            b']' => return Some((j, count)),
            _ => return None,
        }
    }
}

/// The numbers of `text`, a flat JSON array, as `f32`s if they write back as the same text,
/// else as `f64`s if they do; `None` if neither does (integers, say).
fn numbers(text: &str) -> Option<Payload> {
    if let Ok(v) = serde_json::from_str::<Vec<f32>>(text) {
        let p = Payload::F32(v);
        if p.text() == text {
            return Some(p);
        }
    }
    if let Ok(v) = serde_json::from_str::<Vec<f64>>(text) {
        let p = Payload::F64(v);
        if p.text() == text {
            return Some(p);
        }
    }
    None
}

const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The base64 of `bytes` (RFC 4648, with padding).
pub(crate) fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (k, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if k <= chunk.len() {
                out.push(TABLE[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes of `text` in base64 (RFC 4648, padded), if it is that.
fn unbase64(text: &str) -> Option<Vec<u8>> {
    let b = text.as_bytes();
    if !b.len().is_multiple_of(4) {
        return None;
    }
    let value = |c: u8| TABLE.iter().position(|&t| t == c).map(|v| v as u32);
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    for (k, quad) in b.chunks(4).enumerate() {
        let last = k + 1 == b.len() / 4;
        let pad = quad.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for &c in &quad[..4 - pad] {
            n = (n << 6) | value(c)?;
        }
        n <<= 6 * pad as u32;
        let three = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&three[..3 - pad]);
    }
    Some(out)
}

/// Reads the arrays a run's record refers to, from its [`FILE`], opened when first needed.
#[derive(Debug)]
pub struct Arrays {
    path: PathBuf,
    file: Option<File>,
}

impl Arrays {
    /// The arrays of the run in `dir`.
    pub fn new(dir: &Path) -> Arrays {
        Arrays {
            path: dir.join(FILE),
            file: None,
        }
    }

    /// `line` as the run recorded it: each `{"$npy":<offset>}` replaced by the value it stands
    /// for, read from [`FILE`]; a line without one, as it is.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if [`FILE`] can't be read, and [`Error::Parse`] for a reference that
    /// isn't one or an array that isn't one this module writes.
    pub fn inline<'a>(&mut self, line: &'a str) -> Result<Cow<'a, str>> {
        if !line.contains(MARK) {
            return Ok(Cow::Borrowed(line));
        }
        let mut out = String::with_capacity(line.len() * 8);
        let mut rest = line;
        while let Some(at) = rest.find(MARK) {
            out.push_str(&rest[..at]);
            let after = &rest[at + MARK.len()..];
            let close = after
                .find('}')
                .ok_or_else(|| self.bad("a reference isn't closed"))?;
            let offset: u64 = after[..close]
                .parse()
                .map_err(|_| self.bad(&format!("{:?} isn't an offset", &after[..close])))?;
            out.push_str(&self.read(offset)?.text());
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        Ok(Cow::Owned(out))
    }

    fn bad(&self, reason: &str) -> Error {
        Error::Parse {
            what: self.path.display().to_string(),
            reason: reason.to_owned(),
        }
    }

    /// The array whose `.npy` starts at `offset`.
    fn read(&mut self, offset: u64) -> Result<Payload> {
        let path = self.path.clone();
        let io = |e: std::io::Error| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        };
        if self.file.is_none() {
            self.file = Some(File::open(&self.path).map_err(io)?);
        }
        let file = self.file.as_mut().expect("just opened");
        file.seek(SeekFrom::Start(offset)).map_err(io)?;
        let mut start = [0u8; 10];
        file.read_exact(&mut start).map_err(io)?;
        if &start[..6] != b"\x93NUMPY" || start[6] != 1 {
            return Err(self.bad(&format!("no .npy (version 1) at byte {offset}")));
        }
        let mut header = vec![0u8; usize::from(u16::from_le_bytes([start[8], start[9]]))];
        file.read_exact(&mut header).map_err(io)?;
        let header = String::from_utf8_lossy(&header).into_owned();
        let field = |key: &str, close: char| {
            header
                .split(key)
                .nth(1)
                .and_then(|s| s.split(close).next())
                .map(str::to_owned)
        };
        let descr = field("'descr': '", '\'');
        let n: Option<usize> =
            field("'shape': (", ')').and_then(|s| s.trim_end_matches(',').trim().parse().ok());
        let (Some(descr), Some(n)) = (descr, n) else {
            return Err(self.bad(&format!("the .npy at byte {offset} has no descr or shape")));
        };
        let size = match descr.as_str() {
            "<f4" => 4,
            "<f8" => 8,
            "|u1" => 1,
            other => return Err(self.bad(&format!("an array of {other} at byte {offset}"))),
        };
        let file = self.file.as_mut().expect("opened above");
        let mut data = vec![0u8; n * size];
        file.read_exact(&mut data).map_err(io)?;
        Ok(match size {
            4 => Payload::F32(
                data.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|w| f32::from_le_bytes(*w))
                    .collect(),
            ),
            8 => Payload::F64(
                data.as_chunks::<8>()
                    .0
                    .iter()
                    .map(|w| f64::from_le_bytes(*w))
                    .collect(),
            ),
            _ => Payload::Bytes(data),
        })
    }
}

/// `line` of the run in `dir` as the run recorded it: [`Arrays::inline`], opening the run's
/// [`FILE`] for this line alone.
///
/// # Errors
///
/// As [`Arrays::inline`].
pub fn inline<'a>(dir: &Path, line: &'a str) -> Result<Cow<'a, str>> {
    Arrays::new(dir).inline(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_reads_back_what_it_writes() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0x80, 0x7f]), "/4B/");
        for n in 0..20 {
            let bytes: Vec<u8> = (0..n).map(|k| (k * 37 + 11) as u8).collect();
            assert_eq!(unbase64(&base64(&bytes)).unwrap(), bytes);
        }
        assert!(unbase64("Zg=").is_none());
        assert!(unbase64("Zg==Zm8=").is_none());
        assert!(unbase64("Z!==").is_none());
    }

    /// A line in, the line out and its arrays, written to a file and read back: the same line.
    fn round_trip(line: &str) -> (String, usize) {
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-arrays-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (short, bytes) = split(line, 0).unwrap_or((line.to_owned(), Vec::new()));
        std::fs::write(dir.join(FILE), &bytes).unwrap();
        let back = inline(&dir, &short).unwrap().into_owned();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(back, line);
        (short, bytes.len())
    }

    #[test]
    fn long_arrays_move_and_come_back_as_the_same_text() {
        let f32s: Vec<f32> = (0..1000).map(|k| (k as f32 * 0.37).sin() / 3.0).collect();
        let f64s: Vec<f64> = (0..1000)
            .map(|k| (f64::from(k) * 0.37).sin() / 3.0)
            .collect();
        let ints: Vec<u32> = (0..1000).collect();
        let bytes: Vec<u8> = (0..3000u32).map(|k| (k * 7 % 256) as u8).collect();
        let line = format!(
            "{{\"type\":\"x\",\"a\":{},\"note\":\"[1,2,3] {{\\\"$npy\\\":5}}\",\"b\":{{\"values\":{}}},\"n\":{},\"data\":\"{}\",\"short\":[1.5,2.5]}}",
            serde_json::to_string(&f32s).unwrap(),
            serde_json::to_string(&f64s).unwrap(),
            serde_json::to_string(&ints).unwrap(),
            base64(&bytes),
        );
        let (short, written) = round_trip(&line);
        // the f32s, the f64s and the bytes moved; the integers, the note and the short array
        // stayed
        assert_eq!(short.matches(MARK).count(), 3, "{short}");
        assert!(short.contains("\"short\":[1.5,2.5]"));
        assert!(short.contains(&serde_json::to_string(&ints).unwrap()));
        assert!(
            short.len() < line.len() / 4,
            "{} of {}",
            short.len(),
            line.len()
        );
        // f32s, f64s and bytes, each behind a header of 128 bytes
        assert_eq!(written, 1000 * 4 + 1000 * 8 + 3000 + 3 * 128);
    }

    #[test]
    fn a_short_line_or_one_without_long_values_stays() {
        assert!(split("{\"type\":\"started\"}", 0).is_none());
        let ints: Vec<u32> = (0..2000).collect();
        let line = format!("{{\"n\":{}}}", serde_json::to_string(&ints).unwrap());
        assert!(split(&line, 0).is_none());
        // a string key is never moved, nor a string that isn't base64
        let text = "x".repeat(5000);
        let line = format!("{{\"{text}\":1,\"t\":\"{text}!\"}}");
        assert!(split(&line, 0).is_none());
    }

    #[test]
    fn every_float_comes_back_to_the_bit() {
        let tricky: Vec<f32> = [
            0.1f32,
            -0.0,
            f32::MIN_POSITIVE,
            f32::MAX,
            1e-45,
            3.402_823_5e38,
            1.0 / 3.0,
        ]
        .into_iter()
        .cycle()
        .take(700)
        .collect();
        let line = format!("{{\"v\":{}}}", serde_json::to_string(&tricky).unwrap());
        let (short, _) = round_trip(&line);
        assert!(short.contains(MARK));
        let back: serde_json::Value = serde_json::from_str(&line).unwrap();
        let v: Vec<f32> = serde_json::from_value(back["v"].clone()).unwrap();
        assert!(
            v.iter()
                .zip(&tricky)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
}
