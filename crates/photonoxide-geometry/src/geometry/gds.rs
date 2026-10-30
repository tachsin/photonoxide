//! GDSII stream files, read: a layout's cells, and the polygons a cell draws on a (layer,
//! datatype) pair with every cell it places flattened into it, as a [`Shape`] for a structure.
//! Read only: writing layouts is the layout module's (0.9).
//!
//! **What is read.** The records that make polygons: `BOUNDARY`, `BOX` (its `BOXTYPE` taken as
//! the datatype), `PATH` (its `WIDTH`, and its ends by `PATHTYPE`: 0 flush, 1 round, 2 out by
//! half the width, 4 out by `BGNEXTN` and `ENDEXTN`, its corners mitred), and the references
//! that place cells, `SREF` and `AREF` (`STRANS`'s reflection about x, `MAG` and `ANGLE`, in that
//! order, then the move). `UNITS` gives the database unit in metres; every coordinate is a whole
//! number of it, and comes out in µm. `TEXT` and `NODE` draw nothing and are passed over, as are
//! properties and the library's other records.
//!
//! **What isn't.** A path's or a reference's absolute width, magnification or angle (a negative
//! `WIDTH`, `STRANS`'s absolute bits) are refused, naming the cell: they would ignore the
//! placements above them. Writing GDSII, and OASIS, are 0.9's.
//!
//! **Holes.** GDSII has no polygons with holes: a layer is the union of everything drawn on it,
//! as a mask is written, so a boundary inside another is not a hole. A hole is drawn as GDSII
//! draws one, a boundary cut to it along a line it goes out and back on (a keyhole), and the
//! union finds it. A hole drawn on a layer of its own is cut from the other by a job's
//! `"difference"`.
//!
//! **Limits.** A cell placed within itself is refused, and so is a hierarchy deeper than
//! [`MAX_DEPTH`] or one that flattens to more than [`MAX_POLYGONS`] polygons, counted before any
//! is made. A malformed or cut-short file is an [`Error::Parse`] naming the byte, never a panic.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use super::clip::{self, Operation, Part};
use super::{Array, Bounds, Element, Point, Polygon, Shape};
use crate::units::Length;
use crate::{Error, Result};

/// The deepest hierarchy flattened: a cell placing a cell placing … this many levels down.
/// Real layouts are a handful of levels deep; the limit keeps a malformed file from exhausting
/// the stack.
pub const MAX_DEPTH: usize = 64;

/// The most polygons a cell flattens to on one layer, as many as an [`Array`] may have elements
/// ([`super::MAX_SITES`]): a reference array of 10⁵ × 10⁵ is refused before it is made.
pub const MAX_POLYGONS: usize = super::MAX_SITES;

/// The most vertices a round path end's half circle is drawn with: GDSII's own limit on a
/// boundary's points (8191) halved and rounded down to a power of two.
const MAX_CAP: usize = 4096;

// record types
const HEADER: u8 = 0x00;
const LIBNAME: u8 = 0x02;
const UNITS: u8 = 0x03;
const ENDLIB: u8 = 0x04;
const BGNSTR: u8 = 0x05;
const STRNAME: u8 = 0x06;
const ENDSTR: u8 = 0x07;
const BOUNDARY: u8 = 0x08;
const PATH: u8 = 0x09;
const SREF: u8 = 0x0A;
const AREF: u8 = 0x0B;
const TEXT: u8 = 0x0C;
const LAYER: u8 = 0x0D;
const DATATYPE: u8 = 0x0E;
const WIDTH: u8 = 0x0F;
const XY: u8 = 0x10;
const ENDEL: u8 = 0x11;
const SNAME: u8 = 0x12;
const COLROW: u8 = 0x13;
const NODE: u8 = 0x15;
const STRANS: u8 = 0x1A;
const MAG: u8 = 0x1B;
const ANGLE: u8 = 0x1C;
const PATHTYPE: u8 = 0x21;
const BOX: u8 = 0x2D;
const BOXTYPE: u8 = 0x2E;
const BGNEXTN: u8 = 0x30;
const ENDEXTN: u8 = 0x31;

/// A GDSII number of 8 bytes, excess-64: a sign bit, a 7-bit exponent of 16 biased by 64, and a
/// 56-bit fraction, (−1)ˢ × 0.f × 16^(e − 64). Rounded once, to the nearest double.
pub(crate) fn real8(b: [u8; 8]) -> f64 {
    let sign = if b[0] & 0x80 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from(b[0] & 0x7f) - 64;
    let fraction = u64::from_be_bytes([0, b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    // the fraction to the nearest double, then an exact power of two (2^-312 to 2^196)
    sign * fraction as f64 * 2f64.powi(4 * exponent - 56)
}

/// A cell's element that draws, or places another cell.
#[derive(Clone, Debug)]
enum Item {
    /// A boundary or a box: its points, closed or not.
    Polygon { layer: [u16; 2], xy: Vec<[i32; 2]> },
    Path {
        layer: [u16; 2],
        width: i32,
        kind: i16,
        /// How far it goes on past its first and last points, in database units.
        ends: [i32; 2],
        xy: Vec<[i32; 2]>,
    },
    /// An `SREF` (one place) or an `AREF` (`grid`: columns and rows; `xy` its three points).
    Reference {
        cell: String,
        mirror: bool,
        mag: f64,
        angle_deg: f64,
        grid: Option<[u16; 2]>,
        xy: Vec<[i32; 2]>,
    },
}

#[derive(Clone, Debug)]
struct Cell {
    name: String,
    items: Vec<Item>,
}

/// A GDSII stream file, read: its cells and its database unit. See the module.
#[derive(Clone, Debug)]
pub struct Gds {
    library: String,
    /// The database unit, m.
    unit_m: f64,
    /// Database units in a µm.
    per_um: f64,
    cells: Vec<Cell>,
    by_name: HashMap<String, usize>,
}

/// A record: its type, its data, and the byte it starts at.
struct Record<'a> {
    kind: u8,
    data: &'a [u8],
    at: usize,
}

impl Record<'_> {
    fn sized(&self, size: usize, what: &str) -> std::result::Result<(), String> {
        if self.data.len().is_multiple_of(size) {
            Ok(())
        } else {
            Err(format!(
                "at byte {}: {what} holds {} bytes, not a whole number of {size}-byte values",
                self.at,
                self.data.len()
            ))
        }
    }

    fn int2(&self, what: &str) -> std::result::Result<Vec<i16>, String> {
        self.sized(2, what)?;
        Ok(self
            .data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_be_bytes(*c))
            .collect())
    }

    fn int4(&self, what: &str) -> std::result::Result<Vec<i32>, String> {
        self.sized(4, what)?;
        Ok(self
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i32::from_be_bytes(*c))
            .collect())
    }

    fn real8(&self, what: &str) -> std::result::Result<Vec<f64>, String> {
        self.sized(8, what)?;
        Ok(self
            .data
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| real8(*c))
            .collect())
    }

    /// One value of a record that holds one (or more: the first).
    fn one<T: Copy>(
        &self,
        what: &str,
        read: impl Fn(&Self, &str) -> std::result::Result<Vec<T>, String>,
    ) -> std::result::Result<T, String> {
        read(self, what)?
            .first()
            .copied()
            .ok_or_else(|| format!("at byte {}: {what} holds no value", self.at))
    }

    fn text(&self) -> String {
        let end = self.data.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
        String::from_utf8_lossy(&self.data[..end]).into_owned()
    }

    fn points(&self) -> std::result::Result<Vec<[i32; 2]>, String> {
        let v = self.int4("XY")?;
        if !v.len().is_multiple_of(2) {
            return Err(format!(
                "at byte {}: XY holds {} numbers, not pairs",
                self.at,
                v.len()
            ));
        }
        Ok(v.as_chunks::<2>().0.to_vec())
    }
}

/// The records of a stream, one after another.
struct Records<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Records<'a> {
    fn next(&mut self) -> std::result::Result<Record<'a>, String> {
        let rest = &self.bytes[self.at.min(self.bytes.len())..];
        if rest.is_empty() {
            return Err(format!(
                "the file ends at byte {} before its ENDLIB record: it is cut short",
                self.at
            ));
        }
        if rest.len() < 4 {
            return Err(format!(
                "at byte {}: {} bytes are left, fewer than a record's 4-byte header: the file is \
                 cut short",
                self.at,
                rest.len()
            ));
        }
        let length = usize::from(u16::from_be_bytes([rest[0], rest[1]]));
        if length < 4 || !length.is_multiple_of(2) {
            return Err(format!(
                "at byte {}: a record {length} bytes long: a record is an even number of bytes, \
                 4 or more",
                self.at
            ));
        }
        if length > rest.len() {
            return Err(format!(
                "at byte {}: a record {length} bytes long, but the file ends {} bytes on: it is \
                 cut short",
                self.at,
                rest.len()
            ));
        }
        let r = Record {
            kind: rest[2],
            data: &rest[4..length],
            at: self.at,
        };
        self.at += length;
        Ok(r)
    }
}

/// The fields an element's records give.
#[derive(Default)]
struct Fields {
    layer: Option<u16>,
    datatype: Option<u16>,
    width: Option<i32>,
    kind: Option<i16>,
    ends: [Option<i32>; 2],
    xy: Option<Vec<[i32; 2]>>,
    sname: Option<String>,
    strans: Option<u16>,
    mag: Option<f64>,
    angle: Option<f64>,
    colrow: Option<[i16; 2]>,
}

/// The element that `start` begins in `cell`, read to its ENDEL; `None` for one that draws
/// nothing (text, a node).
fn element(
    start: &Record,
    records: &mut Records,
    cell: &str,
) -> std::result::Result<Option<Item>, String> {
    let name = match start.kind {
        BOUNDARY => "BOUNDARY",
        PATH => "PATH",
        SREF => "SREF",
        AREF => "AREF",
        BOX => "BOX",
        TEXT => "TEXT",
        _ => "NODE",
    };
    let at = start.at;
    let mut f = Fields::default();
    loop {
        let r = records.next()?;
        match r.kind {
            ENDEL => break,
            LAYER => f.layer = Some(r.one("LAYER", Record::int2)? as u16),
            DATATYPE | BOXTYPE => f.datatype = Some(r.one("DATATYPE", Record::int2)? as u16),
            WIDTH => f.width = Some(r.one("WIDTH", Record::int4)?),
            PATHTYPE => f.kind = Some(r.one("PATHTYPE", Record::int2)?),
            BGNEXTN => f.ends[0] = Some(r.one("BGNEXTN", Record::int4)?),
            ENDEXTN => f.ends[1] = Some(r.one("ENDEXTN", Record::int4)?),
            XY => f.xy = Some(r.points()?),
            SNAME => f.sname = Some(r.text()),
            STRANS => f.strans = Some(r.one("STRANS", Record::int2)? as u16),
            MAG => f.mag = Some(r.one("MAG", Record::real8)?),
            ANGLE => f.angle = Some(r.one("ANGLE", Record::real8)?),
            COLROW => {
                let v = r.int2("COLROW")?;
                if v.len() != 2 {
                    return Err(format!(
                        "at byte {}: COLROW holds {} numbers, not 2",
                        r.at,
                        v.len()
                    ));
                }
                f.colrow = Some([v[0], v[1]]);
            }
            HEADER | ENDLIB | BGNSTR | ENDSTR | BOUNDARY | PATH | SREF | AREF | TEXT | NODE
            | BOX => {
                return Err(format!(
                    "at byte {}: the {name} at byte {at} in cell \"{cell}\" has no ENDEL",
                    r.at
                ));
            }
            // properties, flags, a text's own records
            _ => {}
        }
    }
    if matches!(start.kind, TEXT | NODE) {
        return Ok(None);
    }
    let what = format!("the {name} at byte {at} in cell \"{cell}\"");
    let need = |field: &str| format!("{what} has no {field}");
    let layer = || -> std::result::Result<[u16; 2], String> {
        Ok([
            f.layer.ok_or_else(|| need("LAYER"))?,
            f.datatype.ok_or_else(|| {
                need(if start.kind == BOX {
                    "BOXTYPE"
                } else {
                    "DATATYPE"
                })
            })?,
        ])
    };
    let xy = f.xy.clone().ok_or_else(|| need("XY"))?;
    let item = match start.kind {
        BOUNDARY | BOX => {
            if xy.len() < 4 {
                return Err(format!(
                    "{what} has {} points: a closed boundary needs 4 or more",
                    xy.len()
                ));
            }
            Item::Polygon {
                layer: layer()?,
                xy,
            }
        }
        PATH => {
            let width = f.width.unwrap_or(0);
            if width < 0 {
                return Err(format!(
                    "{what} has an absolute width ({width}, negative), which isn't read: it \
                     would ignore the magnification of the references above it"
                ));
            }
            let kind = f.kind.unwrap_or(0);
            if ![0, 1, 2, 4].contains(&kind) {
                return Err(format!(
                    "{what} has PATHTYPE {kind}: GDSII's are 0 (flush), 1 (round), 2 (out by \
                     half the width) and 4 (out by BGNEXTN and ENDEXTN)"
                ));
            }
            let half = width / 2;
            let ends = match kind {
                2 => [half, half],
                4 => [f.ends[0].unwrap_or(0), f.ends[1].unwrap_or(0)],
                _ => [0, 0],
            };
            if xy.len() < 2 {
                return Err(format!("{what} has {} point: a path needs two", xy.len()));
            }
            Item::Path {
                layer: layer()?,
                width,
                kind,
                ends,
                xy,
            }
        }
        SREF | AREF => {
            let cell = f.sname.clone().ok_or_else(|| need("SNAME"))?;
            let strans = f.strans.unwrap_or(0);
            if strans & 0x0006 != 0 {
                return Err(format!(
                    "{what} has an absolute magnification or angle (STRANS bits 13 and 14), \
                     which isn't read: it would ignore the references above it"
                ));
            }
            let mag = f.mag.unwrap_or(1.0);
            if !(mag.is_finite() && mag > 0.0) {
                return Err(format!("{what} has MAG {mag}: it must be positive"));
            }
            let angle_deg = f.angle.unwrap_or(0.0);
            if !angle_deg.is_finite() {
                return Err(format!("{what} has an ANGLE that isn't finite"));
            }
            let grid = if start.kind == AREF {
                let [c, r] = f.colrow.ok_or_else(|| need("COLROW"))?;
                if c < 1 || r < 1 {
                    return Err(format!(
                        "{what} has {c} columns and {r} rows: each must be 1 or more"
                    ));
                }
                if xy.len() != 3 {
                    return Err(format!(
                        "{what} has {} points: an AREF has 3 (its origin, and its last column's \
                         and last row's ends)",
                        xy.len()
                    ));
                }
                Some([c as u16, r as u16])
            } else {
                if xy.len() != 1 {
                    return Err(format!("{what} has {} points: an SREF has 1", xy.len()));
                }
                None
            };
            Item::Reference {
                cell,
                mirror: strans & 0x8000 != 0,
                mag,
                angle_deg,
                grid,
                xy,
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(item))
}

/// An affine map of the plane: (x, y) to (a x + b y + e, c x + d y + f), `m` = [a, b, c, d].
#[derive(Clone, Copy, Debug)]
struct Affine {
    m: [f64; 4],
    t: [f64; 2],
}

impl Affine {
    const IDENTITY: Affine = Affine {
        m: [1.0, 0.0, 0.0, 1.0],
        t: [0.0, 0.0],
    };

    fn apply(&self, p: [f64; 2]) -> [f64; 2] {
        let m = &self.m;
        [
            m[0] * p[0] + m[1] * p[1] + self.t[0],
            m[2] * p[0] + m[3] * p[1] + self.t[1],
        ]
    }

    /// `inner` first, then `self`.
    fn after(&self, inner: &Affine) -> Affine {
        let (a, b) = (&self.m, &inner.m);
        Affine {
            m: [
                a[0] * b[0] + a[1] * b[2],
                a[0] * b[1] + a[1] * b[3],
                a[2] * b[0] + a[3] * b[2],
                a[2] * b[1] + a[3] * b[3],
            ],
            t: self.apply(inner.t),
        }
    }

    /// A reference's placement: reflected about x, magnified, turned by `angle_deg`
    /// counterclockwise, then moved to `at`. Quarter turns are exact.
    fn placing(mirror: bool, mag: f64, angle_deg: f64, at: [f64; 2]) -> Affine {
        let quarter = angle_deg / 90.0;
        let (c, s) = if quarter.fract() == 0.0 && quarter.abs() < 1e15 {
            match (quarter as i64).rem_euclid(4) {
                0 => (1.0, 0.0),
                1 => (0.0, 1.0),
                2 => (-1.0, 0.0),
                _ => (0.0, -1.0),
            }
        } else {
            let r = angle_deg.to_radians();
            (r.cos(), r.sin())
        };
        let f = if mirror { -1.0 } else { 1.0 };
        Affine {
            m: [mag * c, -mag * s * f, mag * s, mag * c * f],
            t: at,
        }
    }
}

/// `xy` without consecutive repeats.
fn unrepeated(xy: impl Iterator<Item = [f64; 2]>) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::new();
    for p in xy {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    out
}

/// A ring's points without consecutive repeats, and without a last point that closes it.
fn distinct(xy: impl Iterator<Item = [f64; 2]>) -> Vec<[f64; 2]> {
    let mut out = unrepeated(xy);
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// A path's outline in its cell's database units: its two sides offset by half its width,
/// mitred at its corners, its ends out by `ends` or, for PATHTYPE 1, round (half circles
/// within half a database unit, their vertices on the circle). `None` for a path of no width.
fn outline(
    xy: &[[i32; 2]],
    width: i32,
    kind: i16,
    ends: [i32; 2],
) -> std::result::Result<Option<Vec<[f64; 2]>>, String> {
    // a path's last point may be its first (a loop of path): it is kept
    let p = unrepeated(xy.iter().map(|q| [f64::from(q[0]), f64::from(q[1])]));
    if width == 0 {
        return Ok(None);
    }
    if p.len() < 2 {
        return Err("has no two distinct points".into());
    }
    let h = f64::from(width) / 2.0;
    let n = p.len();
    let dirs: Vec<[f64; 2]> = (0..n - 1)
        .map(|k| {
            let d = [p[k + 1][0] - p[k][0], p[k + 1][1] - p[k][1]];
            let l = d[0].hypot(d[1]);
            [d[0] / l, d[1] / l]
        })
        .collect();
    let normal = |d: [f64; 2]| [-d[1], d[0]];
    let mut first = p[0];
    let mut last = p[n - 1];
    let (d0, dn) = (dirs[0], dirs[n - 2]);
    if kind != 1 {
        let (b, e) = (f64::from(ends[0]), f64::from(ends[1]));
        first = [first[0] - d0[0] * b, first[1] - d0[1] * b];
        last = [last[0] + dn[0] * e, last[1] + dn[1] * e];
    }
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    let (n0, nn) = (normal(d0), normal(dn));
    left.push([first[0] + h * n0[0], first[1] + h * n0[1]]);
    right.push([first[0] - h * n0[0], first[1] - h * n0[1]]);
    for k in 1..n - 1 {
        let (a, b) = (normal(dirs[k - 1]), normal(dirs[k]));
        let dot = a[0] * b[0] + a[1] * b[1];
        if 1.0 + dot <= 1e-12 {
            return Err(format!(
                "turns back on itself at its point {}: a mitre there has no end",
                k + 1
            ));
        }
        // the two sides' offset lines meet at the mitre's corner
        let m = [
            h * (a[0] + b[0]) / (1.0 + dot),
            h * (a[1] + b[1]) / (1.0 + dot),
        ];
        left.push([p[k][0] + m[0], p[k][1] + m[1]]);
        right.push([p[k][0] - m[0], p[k][1] - m[1]]);
    }
    left.push([last[0] + h * nn[0], last[1] + h * nn[1]]);
    right.push([last[0] - h * nn[0], last[1] - h * nn[1]]);
    let mut ring = left;
    // a half circle about `c` from the side `from` (a unit normal) clockwise to its opposite
    let cap = |c: [f64; 2], from: [f64; 2], ring: &mut Vec<[f64; 2]>| {
        let segments = cap_segments(h);
        let start = from[1].atan2(from[0]);
        for j in 1..segments {
            let t = start - std::f64::consts::PI * j as f64 / segments as f64;
            ring.push([c[0] + h * t.cos(), c[1] + h * t.sin()]);
        }
    };
    if kind == 1 {
        cap(last, nn, &mut ring);
    }
    ring.extend(right.into_iter().rev());
    if kind == 1 {
        cap(first, [-n0[0], -n0[1]], &mut ring);
    }
    Ok(Some(ring))
}

/// The segments of a half circle of radius `h` database units whose chords stray from it by at
/// most half a database unit: the file can't place a point closer than that.
fn cap_segments(h: f64) -> usize {
    let sagitta = 0.5 / h;
    if sagitta >= 1.0 {
        return 2;
    }
    // a chord over the angle φ strays by h (1 − cos(φ/2)); a half circle is π
    let phi = 2.0 * (1.0 - sagitta).acos();
    ((std::f64::consts::PI / phi).ceil() as usize).clamp(2, MAX_CAP)
}

fn parse_error(reason: String) -> Error {
    Error::Parse {
        what: "GDSII file".into(),
        reason,
    }
}

fn layer_name(l: [u16; 2]) -> String {
    format!("{}/{}", l[0], l[1])
}

impl Gds {
    /// The GDSII file at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it can't be read, and [`Gds::parse`]'s errors, naming the path.
    pub fn read(path: &Path) -> Result<Gds> {
        let bytes = std::fs::read(path).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        Gds::parse(&bytes).map_err(|e| match e {
            Error::Parse { reason, .. } => Error::Parse {
                what: format!("GDSII file {}", path.display()),
                reason,
            },
            other => other,
        })
    }

    /// A GDSII stream from its bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`], naming the byte, for a stream that doesn't start with a HEADER record,
    /// a record whose length isn't one or whose data doesn't fit its type, a stream cut short
    /// (before its ENDLIB), an element without the fields it needs or without its ENDEL, a cell
    /// without a name or its ENDSTR, two cells of one name, no UNITS or a database unit that
    /// isn't positive, and the absolute widths, magnifications and angles it doesn't read.
    pub fn parse(bytes: &[u8]) -> Result<Gds> {
        Gds::records(bytes).map_err(parse_error)
    }

    fn records(bytes: &[u8]) -> std::result::Result<Gds, String> {
        let mut records = Records { bytes, at: 0 };
        let head = records.next().map_err(|_| {
            "it isn't a GDSII stream file: it doesn't start with a HEADER record".to_owned()
        })?;
        if head.kind != HEADER {
            return Err(
                "it isn't a GDSII stream file: it doesn't start with a HEADER record".into(),
            );
        }
        let mut library = String::new();
        let mut unit_m = None;
        let mut cells: Vec<Cell> = Vec::new();
        let mut by_name = HashMap::new();
        // the cell being read: its name (once read), its items, and where it began
        let mut open: Option<(Option<String>, Vec<Item>, usize)> = None;
        loop {
            let r = records.next()?;
            match r.kind {
                LIBNAME => library = r.text(),
                UNITS => {
                    let v = r.real8("UNITS")?;
                    let Some(&m) = v.get(1) else {
                        return Err(format!(
                            "at byte {}: UNITS holds {} numbers, not 2",
                            r.at,
                            v.len()
                        ));
                    };
                    if !(m.is_finite() && m > 0.0) {
                        return Err(format!(
                            "at byte {}: the database unit is {m} m: it must be positive",
                            r.at
                        ));
                    }
                    unit_m = Some(m);
                }
                BGNSTR => {
                    if let Some((name, _, at)) = &open {
                        return Err(format!(
                            "at byte {}: a cell begins inside cell \"{}\" (from byte {at}), \
                             which has no ENDSTR",
                            r.at,
                            name.as_deref().unwrap_or("")
                        ));
                    }
                    open = Some((None, Vec::new(), r.at));
                }
                STRNAME => match &mut open {
                    Some((name, _, _)) => *name = Some(r.text()),
                    None => {
                        return Err(format!("at byte {}: a STRNAME outside a cell", r.at));
                    }
                },
                ENDSTR => {
                    let Some((name, items, at)) = open.take() else {
                        return Err(format!("at byte {}: an ENDSTR outside a cell", r.at));
                    };
                    let name = name
                        .filter(|n| !n.is_empty())
                        .ok_or_else(|| format!("the cell at byte {at} has no name (STRNAME)"))?;
                    if by_name.insert(name.clone(), cells.len()).is_some() {
                        return Err(format!("at byte {at}: a second cell named \"{name}\""));
                    }
                    cells.push(Cell { name, items });
                }
                BOUNDARY | PATH | SREF | AREF | TEXT | NODE | BOX => {
                    let Some((name, items, _)) = &mut open else {
                        return Err(format!("at byte {}: an element outside a cell", r.at));
                    };
                    let cell = name.clone().unwrap_or_default();
                    if let Some(item) = element(&r, &mut records, &cell)? {
                        items.push(item);
                    }
                }
                ENDLIB => {
                    if let Some((name, _, at)) = open {
                        return Err(format!(
                            "at byte {}: the library ends inside cell \"{}\" (from byte {at}), \
                             which has no ENDSTR",
                            r.at,
                            name.unwrap_or_default()
                        ));
                    }
                    break;
                }
                // BGNLIB, the library's other records, and anything newer
                _ => {}
            }
        }
        let unit_m = unit_m.ok_or("it has no UNITS record: its database unit isn't known")?;
        // database units in a µm. 10⁻⁹ m has no exact excess-64 form, nor a double: a ratio
        // that is a whole number to the precision the two hold (53 bits at least, each rounded
        // once, and the division once more) is taken as that number, so that a nanometre unit
        // gives 1000 and a coordinate of 1500 gives 1.5 µm exactly
        let mut per_um = 1e-6 / unit_m;
        if (per_um - per_um.round()).abs() <= 4.0 * f64::EPSILON * per_um {
            per_um = per_um.round();
        }
        Ok(Gds {
            library,
            unit_m,
            per_um,
            cells,
            by_name,
        })
    }

    /// The library's name (LIBNAME).
    pub fn library(&self) -> &str {
        &self.library
    }

    /// The database unit: every coordinate is a whole number of it.
    pub fn database_unit(&self) -> Length {
        Length::um(self.unit_m * 1e6)
    }

    /// The cells' names, in the file's order.
    pub fn cells(&self) -> Vec<&str> {
        self.cells.iter().map(|c| c.name.as_str()).collect()
    }

    /// The cells no other cell places, in the file's order.
    pub fn top_cells(&self) -> Vec<&str> {
        let mut placed = vec![false; self.cells.len()];
        for c in &self.cells {
            for item in &c.items {
                if let Item::Reference { cell, .. } = item
                    && let Some(&k) = self.by_name.get(cell)
                {
                    placed[k] = true;
                }
            }
        }
        self.cells
            .iter()
            .zip(placed)
            .filter(|(_, p)| !p)
            .map(|(c, _)| c.name.as_str())
            .collect()
    }

    /// The file's one top cell.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when it has none (no cells, or every cell placed by another) or
    /// several, naming them.
    pub fn top_cell(&self) -> Result<&str> {
        match self.top_cells().as_slice() {
            [one] => Ok(one),
            [] => Err(Error::invalid(
                "GDSII cell",
                "the file has no top cell (a cell no other places): name the cell to read",
            )),
            many => Err(Error::invalid(
                "GDSII cell",
                format!(
                    "the file has {} top cells ({}): name the one to read",
                    many.len(),
                    many.join(", ")
                ),
            )),
        }
    }

    fn index(&self, cell: &str) -> Result<usize> {
        self.by_name.get(cell).copied().ok_or_else(|| {
            Error::invalid(
                "GDSII cell",
                format!(
                    "the file has no cell \"{cell}\": its cells are {}",
                    self.cells().join(", ")
                ),
            )
        })
    }

    /// The (layer, datatype) pairs `cell` draws on, the cells it places included, in order.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a cell the file doesn't have.
    pub fn layers(&self, cell: &str) -> Result<Vec<[u16; 2]>> {
        let start = self.index(cell)?;
        let mut seen = vec![false; self.cells.len()];
        let mut stack = vec![start];
        let mut layers = BTreeSet::new();
        seen[start] = true;
        while let Some(k) = stack.pop() {
            for item in &self.cells[k].items {
                match item {
                    Item::Polygon { layer, .. } | Item::Path { layer, .. } => {
                        layers.insert(*layer);
                    }
                    Item::Reference { cell, .. } => {
                        if let Some(&c) = self.by_name.get(cell)
                            && !seen[c]
                        {
                            seen[c] = true;
                            stack.push(c);
                        }
                    }
                }
            }
        }
        Ok(layers.into_iter().collect())
    }

    /// How many polygons cell `k` flattens to on `layer` (at most a little over
    /// [`MAX_POLYGONS`]), refusing a cell that places itself, a hierarchy deeper than
    /// [`MAX_DEPTH`] and a cell the file doesn't have.
    fn count(
        &self,
        k: usize,
        layer: [u16; 2],
        chain: &mut Vec<usize>,
        memo: &mut [Option<u128>],
    ) -> std::result::Result<u128, String> {
        if let Some(n) = memo[k] {
            return Ok(n);
        }
        if chain.contains(&k) {
            let names: Vec<&str> = chain
                .iter()
                .chain(std::iter::once(&k))
                .map(|&c| self.cells[c].name.as_str())
                .collect();
            return Err(format!("a cell places itself: {}", names.join(" → ")));
        }
        if chain.len() >= MAX_DEPTH {
            return Err(format!(
                "the hierarchy below cell \"{}\" is deeper than {MAX_DEPTH} levels",
                self.cells[chain[0]].name
            ));
        }
        chain.push(k);
        let mut n: u128 = 0;
        for item in &self.cells[k].items {
            n = n.saturating_add(match item {
                Item::Polygon { layer: l, .. } | Item::Path { layer: l, .. } => {
                    u128::from(*l == layer)
                }
                Item::Reference { cell, grid, .. } => {
                    let c = *self.by_name.get(cell).ok_or_else(|| {
                        format!(
                            "cell \"{}\" places cell \"{cell}\", which the file doesn't have",
                            self.cells[k].name
                        )
                    })?;
                    let copies = grid.map_or(1, |[a, b]| u128::from(a) * u128::from(b));
                    self.count(c, layer, chain, memo)?.saturating_mul(copies)
                }
            });
            // stop counting once over the limit: the answer is already no
            n = n.min(MAX_POLYGONS as u128 + 1);
        }
        chain.pop();
        memo[k] = Some(n);
        Ok(n)
    }

    /// Cell `k`'s polygons on `layer` under the placement `t`, in database units, added to
    /// `out`.
    fn collect(
        &self,
        k: usize,
        layer: [u16; 2],
        t: &Affine,
        out: &mut Vec<Vec<[f64; 2]>>,
    ) -> std::result::Result<(), String> {
        let cell = &self.cells[k];
        for item in &cell.items {
            match item {
                Item::Polygon { layer: l, xy } if *l == layer => {
                    out.push(
                        xy.iter()
                            .map(|q| t.apply([f64::from(q[0]), f64::from(q[1])]))
                            .collect(),
                    );
                }
                Item::Path {
                    layer: l,
                    width,
                    kind,
                    ends,
                    xy,
                } if *l == layer => {
                    let ring = outline(xy, *width, *kind, *ends).map_err(|m| {
                        format!(
                            "a path on {} in cell \"{}\" {m}",
                            layer_name(layer),
                            cell.name
                        )
                    })?;
                    if let Some(ring) = ring {
                        out.push(ring.into_iter().map(|q| t.apply(q)).collect());
                    }
                }
                Item::Reference {
                    cell: name,
                    mirror,
                    mag,
                    angle_deg,
                    grid,
                    xy,
                } => {
                    let c = self.by_name[name];
                    let at = |q: [i32; 2]| [f64::from(q[0]), f64::from(q[1])];
                    let p0 = at(xy[0]);
                    match grid {
                        None => {
                            let place = Affine::placing(*mirror, *mag, *angle_deg, p0);
                            self.collect(c, layer, &t.after(&place), out)?;
                        }
                        Some([cols, rows]) => {
                            let (p1, p2) = (at(xy[1]), at(xy[2]));
                            let (nc, nr) = (f64::from(*cols), f64::from(*rows));
                            let col = [(p1[0] - p0[0]) / nc, (p1[1] - p0[1]) / nc];
                            let row = [(p2[0] - p0[0]) / nr, (p2[1] - p0[1]) / nr];
                            for j in 0..*rows {
                                for i in 0..*cols {
                                    let (fi, fj) = (f64::from(i), f64::from(j));
                                    let o = [
                                        p0[0] + fi * col[0] + fj * row[0],
                                        p0[1] + fi * col[1] + fj * row[1],
                                    ];
                                    let place = Affine::placing(*mirror, *mag, *angle_deg, o);
                                    self.collect(c, layer, &t.after(&place), out)?;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// `cell`'s polygons on `layer`, every cell it places flattened into it, in µm: each
    /// counterclockwise, without repeated points; boundaries of no area left out.
    fn rings(&self, cell: &str, layer: [u16; 2]) -> Result<Vec<Vec<[f64; 2]>>> {
        let k = self.index(cell)?;
        let what = |m: String| Error::invalid("GDSII cell", m);
        let mut memo = vec![None; self.cells.len()];
        let n = self
            .count(k, layer, &mut Vec::new(), &mut memo)
            .map_err(what)?;
        if n > MAX_POLYGONS as u128 {
            return Err(what(format!(
                "cell \"{cell}\" flattens to more than {MAX_POLYGONS} polygons on {}",
                layer_name(layer)
            )));
        }
        let mut raw = Vec::with_capacity(n as usize);
        self.collect(k, layer, &Affine::IDENTITY, &mut raw)
            .map_err(what)?;
        let s = self.per_um;
        Ok(raw
            .into_iter()
            .filter_map(|ring| {
                let mut r = distinct(ring.into_iter().map(|q| [q[0] / s, q[1] / s]));
                if r.len() < 3 {
                    return None;
                }
                let area = clip::shoelace(&r);
                if area == 0.0 || !area.is_finite() {
                    return None;
                }
                if area < 0.0 {
                    r.reverse();
                }
                Some(r)
            })
            .collect())
    }

    /// `cell`'s polygons on `layer` (a [layer, datatype] pair), every cell it places flattened
    /// into it, in µm, as the file draws them: overlapping, a keyhole as its one polygon.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a cell the file doesn't have, a cell that places one it
    /// doesn't have or places itself, a hierarchy deeper than [`MAX_DEPTH`], more than
    /// [`MAX_POLYGONS`] polygons, or a path that turns back on itself.
    pub fn polygons(&self, cell: &str, layer: [u16; 2]) -> Result<Vec<Polygon>> {
        self.rings(cell, layer)?
            .into_iter()
            .map(|r| Polygon::new(r.into_iter().map(|q| Point::um(q[0], q[1])).collect()))
            .collect()
    }

    /// What `cell` draws on `layer`, every cell it places flattened into it, as one shape in
    /// µm: the union of its polygons (holes found where a keyhole draws them), within `window`
    /// if one is given. A piece without holes is a [`Polygon`], one with them the polygon less
    /// its holes ([`super::Boolean`]); several pieces are an [`Array`], their union.
    ///
    /// # Errors
    ///
    /// As [`Gds::polygons`], and [`Error::InvalidValue`] when nothing is drawn on `layer` (or
    /// inside `window`), naming the layers the cell does draw on.
    pub fn shape(&self, cell: &str, layer: [u16; 2], window: Option<Bounds>) -> Result<Shape> {
        let rings = self.rings(cell, layer)?;
        let nothing = |inside: &str| -> Result<Shape> {
            let drawn: Vec<String> = self.layers(cell)?.into_iter().map(layer_name).collect();
            Err(Error::invalid(
                "GDSII layer",
                format!(
                    "cell \"{cell}\" draws nothing on {}{inside}: it draws on {}",
                    layer_name(layer),
                    if drawn.is_empty() {
                        "no layer".to_owned()
                    } else {
                        drawn.join(", ")
                    }
                ),
            ))
        };
        if rings.is_empty() {
            return nothing("");
        }
        let parts: Vec<Part> = rings.into_iter().map(|r| (r, Vec::new())).collect();
        let mut merged = clip::simplified(&parts, 0.0);
        if let Some(w) = window {
            let [lo, hi] = w.corners();
            let rect = vec![
                [lo[0], lo[1]],
                [hi[0], lo[1]],
                [hi[0], hi[1]],
                [lo[0], hi[1]],
            ];
            merged = clip::boolean(
                &merged,
                &[(rect, Vec::new())],
                Operation::Intersection,
                0.0,
                0.0,
            );
            if merged.is_empty() {
                return nothing(&format!(
                    " inside the window x {} to {} µm, y {} to {} µm",
                    lo[0], hi[0], lo[1], hi[1]
                ));
            }
        }
        let mut shapes = Vec::with_capacity(merged.len());
        for part in merged {
            shapes.push(piece(part)?);
        }
        if shapes.len() == 1 {
            return Ok(shapes.remove(0));
        }
        Ok(Shape::Array(Array::new(
            shapes
                .into_iter()
                .enumerate()
                .map(|(k, s)| Element::new([k as i64, 0], s))
                .collect(),
        )?))
    }
}

/// A polygon with holes as a shape: the polygon, less its holes.
fn piece((outer, holes): Part) -> Result<Shape> {
    let polygon = |r: Vec<[f64; 2]>| {
        Polygon::new(r.into_iter().map(|q| Point::um(q[0], q[1])).collect()).map(Shape::Polygon)
    };
    let outer = polygon(outer)?;
    let mut cut: Vec<Shape> = Vec::with_capacity(holes.len());
    for h in holes {
        cut.push(polygon(h)?);
    }
    let cut = match cut.len() {
        0 => return Ok(outer),
        1 => cut.remove(0),
        _ => Shape::Array(Array::new(
            cut.into_iter()
                .enumerate()
                .map(|(k, s)| Element::new([k as i64, 0], s))
                .collect(),
        )?),
    };
    outer.boolean(Operation::Difference, &cut)
}
