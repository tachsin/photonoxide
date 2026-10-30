//! What the validation report and the tests check the GDSII reader on: a stream written here
//! byte by byte, record by record, so that every number in it is known, and read back.

use super::Gds;
use super::gds::real8;
use crate::validation::{Case, Outcome, Tier};

/// A number in GDSII's excess-64 form: the double's own value, exactly (its 53 bits fit the
/// 56-bit fraction).
pub(crate) fn excess64(v: f64) -> [u8; 8] {
    if v == 0.0 {
        return [0; 8];
    }
    let sign = if v < 0.0 { 0x80u8 } else { 0 };
    let a = v.abs();
    // 16^(e − 1) <= a < 16^e
    let mut e = (a.log2() / 4.0).floor() as i32 + 1;
    while a >= 2f64.powi(4 * e) {
        e += 1;
    }
    while a < 2f64.powi(4 * (e - 1)) {
        e -= 1;
    }
    // a / 16^e in [1/16, 1), times 2^56: a whole number below 2^56, exact
    let fraction = (a * 2f64.powi(56 - 4 * e)) as u64;
    let b = fraction.to_be_bytes();
    [
        sign | (e + 64) as u8,
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
    ]
}

/// A GDSII stream written record by record, for tests and checks.
pub(crate) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    /// A stream's HEADER, BGNLIB, LIBNAME and UNITS: `user` database units in a user unit, the
    /// database unit `unit_m` metres.
    pub(crate) fn new(user: f64, unit_m: f64) -> Writer {
        let mut w = Writer::without_units();
        w.reals(0x03, &[user, unit_m]);
        w
    }

    /// A stream's HEADER, BGNLIB and LIBNAME, without the UNITS a library needs.
    pub(crate) fn without_units() -> Writer {
        let mut w = Writer { bytes: Vec::new() };
        w.int2(0x00, &[600]);
        w.int2(0x01, &[2026, 10, 29, 12, 0, 0, 2026, 10, 29, 12, 0, 0]);
        w.text(0x02, "LIB");
        w
    }

    /// A record of `kind` holding `data` of `data_type`.
    pub(crate) fn record(&mut self, kind: u8, data_type: u8, data: &[u8]) {
        let length = u16::try_from(data.len() + 4).expect("a record fits 64 kB");
        self.bytes.extend(length.to_be_bytes());
        self.bytes.extend([kind, data_type]);
        self.bytes.extend(data);
    }

    pub(crate) fn int2(&mut self, kind: u8, v: &[i16]) {
        let data: Vec<u8> = v.iter().flat_map(|x| x.to_be_bytes()).collect();
        self.record(kind, 0x02, &data);
    }

    pub(crate) fn int4(&mut self, kind: u8, v: &[i32]) {
        let data: Vec<u8> = v.iter().flat_map(|x| x.to_be_bytes()).collect();
        self.record(kind, 0x03, &data);
    }

    pub(crate) fn reals(&mut self, kind: u8, v: &[f64]) {
        let data: Vec<u8> = v.iter().flat_map(|&x| excess64(x)).collect();
        self.record(kind, 0x05, &data);
    }

    pub(crate) fn text(&mut self, kind: u8, s: &str) {
        let mut data = s.as_bytes().to_vec();
        if !data.len().is_multiple_of(2) {
            data.push(0);
        }
        self.record(kind, 0x06, &data);
    }

    pub(crate) fn none(&mut self, kind: u8) {
        self.record(kind, 0x00, &[]);
    }

    fn xy(&mut self, points: &[[i32; 2]]) {
        let flat: Vec<i32> = points.iter().flatten().copied().collect();
        self.int4(0x10, &flat);
    }

    /// BGNSTR and STRNAME.
    pub(crate) fn begin(&mut self, name: &str) {
        self.int2(0x05, &[2026, 10, 29, 12, 0, 0, 2026, 10, 29, 12, 0, 0]);
        self.text(0x06, name);
    }

    /// ENDSTR.
    pub(crate) fn end(&mut self) {
        self.none(0x07);
    }

    /// A BOUNDARY through `points`, closed here.
    pub(crate) fn boundary(&mut self, layer: [i16; 2], points: &[[i32; 2]]) {
        self.none(0x08);
        self.int2(0x0D, &[layer[0]]);
        self.int2(0x0E, &[layer[1]]);
        let mut closed = points.to_vec();
        closed.push(points[0]);
        self.xy(&closed);
        self.none(0x11);
    }

    /// A BOX from `lo` to `hi`, its BOXTYPE `layer[1]`.
    pub(crate) fn rect(&mut self, layer: [i16; 2], lo: [i32; 2], hi: [i32; 2]) {
        self.none(0x2D);
        self.int2(0x0D, &[layer[0]]);
        self.int2(0x2E, &[layer[1]]);
        self.xy(&[lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo]);
        self.none(0x11);
    }

    /// A PATH of `width` and PATHTYPE `kind` through `points`; `ends` its BGNEXTN and ENDEXTN.
    pub(crate) fn path(
        &mut self,
        layer: [i16; 2],
        kind: i16,
        width: i32,
        ends: Option<[i32; 2]>,
        points: &[[i32; 2]],
    ) {
        self.none(0x09);
        self.int2(0x0D, &[layer[0]]);
        self.int2(0x0E, &[layer[1]]);
        self.int2(0x21, &[kind]);
        self.int4(0x0F, &[width]);
        if let Some([b, e]) = ends {
            self.int4(0x30, &[b]);
            self.int4(0x31, &[e]);
        }
        self.xy(points);
        self.none(0x11);
    }

    fn strans(&mut self, mirror: bool, mag: f64, angle: f64) {
        if mirror || mag != 1.0 || angle != 0.0 {
            self.int2(0x1A, &[if mirror { i16::MIN } else { 0 }]);
            if mag != 1.0 {
                self.reals(0x1B, &[mag]);
            }
            if angle != 0.0 {
                self.reals(0x1C, &[angle]);
            }
        }
    }

    /// An SREF of `cell` at `at`: reflected about x if `mirror`, magnified by `mag`, turned by
    /// `angle` degrees.
    pub(crate) fn sref(&mut self, cell: &str, mirror: bool, mag: f64, angle: f64, at: [i32; 2]) {
        self.none(0x0A);
        self.text(0x12, cell);
        self.strans(mirror, mag, angle);
        self.xy(&[at]);
        self.none(0x11);
    }

    /// An AREF of `cell`, `cols` × `rows`, its three points `xy`.
    pub(crate) fn aref(
        &mut self,
        cell: &str,
        transform: (bool, f64, f64),
        grid: [i16; 2],
        xy: [[i32; 2]; 3],
    ) {
        self.none(0x0B);
        self.text(0x12, cell);
        self.strans(transform.0, transform.1, transform.2);
        self.int2(0x13, &grid);
        self.xy(&xy);
        self.none(0x11);
    }

    /// A TEXT, which draws nothing.
    pub(crate) fn label(&mut self, layer: i16, at: [i32; 2], s: &str) {
        self.none(0x0C);
        self.int2(0x0D, &[layer]);
        self.int2(0x16, &[0]);
        self.xy(&[at]);
        self.text(0x19, s);
        self.none(0x11);
    }

    /// ENDLIB, and the stream.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.none(0x04);
        self.bytes
    }
}

/// Known excess-64 forms against their values: 1, 0.5 and −2.5 (exact), and 10⁻³ and 10⁻⁹ as
/// files hold them, their fractions the decimal's 2^56 × 16^−e rounded (10⁻³'s also truncated,
/// as some writers store it). (form, value).
pub(crate) fn known_numbers() -> Vec<([u8; 8], f64)> {
    // 10⁻³ = 0.256 × 16⁻²: its fraction 2^64 / 1000; 10⁻⁹ = 0.268… × 16⁻⁷: 2^84 / 10^9
    let rounded = |numerator: u128, d: u128| ((numerator + d / 2) / d) as u64;
    let with = |e: u8, f: u64| {
        let b = f.to_be_bytes();
        [e, b[1], b[2], b[3], b[4], b[5], b[6], b[7]]
    };
    vec![
        ([0x41, 0x10, 0, 0, 0, 0, 0, 0], 1.0),
        ([0x40, 0x80, 0, 0, 0, 0, 0, 0], 0.5),
        ([0xC1, 0x28, 0, 0, 0, 0, 0, 0], -2.5),
        (with(0x3E, rounded(1 << 64, 1000)), 1e-3),
        ([0x3E, 0x41, 0x89, 0x37, 0x4B, 0xC6, 0xA7, 0xEF], 1e-3),
        (with(0x39, rounded(1 << 84, 1_000_000_000)), 1e-9),
    ]
}

/// The largest relative difference of the known excess-64 forms from their values.
pub(crate) fn number_error() -> f64 {
    known_numbers()
        .into_iter()
        .map(|(b, v)| ((real8(b) - v) / v).abs())
        .fold(0.0, f64::max)
}

/// The layout the case reads, on a 1 nm database unit: a cell "unit" of an L-shaped boundary
/// (3 µm²), a box (1 × 0.5 µm) and a path 0.2 µm wide with flush ends turning a right angle
/// (0.32 µm²), all on layer 1/0, with a label; and a top cell placing it four ways (moved,
/// turned a quarter, reflected and turned 30°, magnified 1.5) and as a 3 × 2 array, none
/// overlapping, with a box of its own on layer 2/0 (4 µm²).
pub(crate) fn layout() -> Vec<u8> {
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("unit");
    // an L: 2 × 1 and 1 × 1 above its left end, 3 µm²
    w.boundary(
        [1, 0],
        &[
            [0, 0],
            [2000, 0],
            [2000, 1000],
            [1000, 1000],
            [1000, 2000],
            [0, 2000],
        ],
    );
    w.rect([1, 0], [2500, 0], [3500, 500]);
    // 0.2 µm wide, from (2.6, 1) right 0.8 µm then up 0.8 µm: two arms meeting in a mitred
    // corner, 0.2 × (0.8 + 0.1) + 0.2 × (0.8 − 0.1) = 0.32 µm²
    w.path(
        [1, 0],
        0,
        200,
        None,
        &[[2600, 1000], [3400, 1000], [3400, 1800]],
    );
    w.label(1, [0, 0], "unit");
    w.end();
    w.begin("top");
    w.sref("unit", false, 1.0, 0.0, [0, 0]);
    w.sref("unit", false, 1.0, 90.0, [-5000, 0]);
    w.sref("unit", true, 1.0, 30.0, [0, -10000]);
    w.sref("unit", false, 1.5, 0.0, [10000, 0]);
    w.aref(
        "unit",
        (false, 1.0, 0.0),
        [3, 2],
        [[0, 10000], [15000, 10000], [0, 16000]],
    );
    w.rect([2, 0], [-1000, -1000], [1000, 1000]);
    w.end();
    w.finish()
}

/// The area `layout()` draws on layer 1/0: the unit's, 3 + 0.5 + 0.32 µm², four times, 1.5²
/// times once more, and six times in the array.
pub(crate) fn expected() -> f64 {
    let unit = 3.0 + 0.5 + 0.32;
    unit * (3.0 + 1.5 * 1.5 + 6.0)
}

/// The largest relative difference of the layout's layers' areas, read back, from their closed
/// forms: layer 1/0's union and its polygons summed, and layer 2/0's box.
pub(crate) fn area_error() -> f64 {
    let Ok(gds) = Gds::parse(&layout()) else {
        return f64::NAN;
    };
    let rel = |a: f64, b: f64| ((a - b) / b).abs();
    let (Ok(one), Ok(polygons), Ok(two)) = (
        gds.shape("top", [1, 0], None),
        gds.polygons("top", [1, 0]),
        gds.shape("top", [2, 0], None),
    ) else {
        return f64::NAN;
    };
    let summed: f64 = polygons.iter().map(super::Polygon::area).sum();
    rel(one.area(), expected())
        .max(rel(summed, expected()))
        .max(rel(two.area(), 4.0))
}

/// The GDSII reader's validation cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![Case {
        id: "geometry/gds",
        title: r"A GDSII file written here byte by byte: a cell of an L-shaped boundary, a box and a mitred path 0.2 µm wide, placed moved, turned a quarter, reflected and turned 30°, magnified 1.5 and as a 3 × 2 array, on a 1 nm database unit; each layer's area read back (the union and its polygons summed) against the closed form, and the excess-64 numbers 1, 0.5, −2.5, 10⁻³ and 10⁻⁹ as files hold them against their values (largest relative difference shown)",
        tier: Tier::Analytic,
        source: r"the GDSII stream format (Calma, GDSII Stream Format Manual, release 6.0, 1987): records, excess-64 reals $(-1)^s\thinspace 0.f \times 16^{e - 64}$, references reflected, magnified, turned and moved in that order; the areas $3.82\thinspace(3 + 1.5^2 + 6)$ and $4$ µm²; measured 8.3e-16",
        run: || {
            let e = area_error().max(number_error());
            Outcome {
                measured: e,
                expected: 0.0,
                tolerance: 1e-12,
                error: e,
            }
        },
    }]
}
