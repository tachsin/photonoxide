//! The GDSII reader: every record it reads, written here byte by byte.

use super::gds::real8;
use super::gds_checks::{Writer, excess64, known_numbers, layout};
use super::*;

/// A one-cell library on a 1 nm database unit, `draw` writing its elements.
fn one_cell(draw: impl FnOnce(&mut Writer)) -> Gds {
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    draw(&mut w);
    w.end();
    Gds::parse(&w.finish()).unwrap()
}

fn area_on(gds: &Gds, layer: [u16; 2]) -> f64 {
    gds.shape("top", layer, None).unwrap().area()
}

fn close(a: f64, b: f64) {
    assert!(((a - b) / b).abs() < 1e-13, "{a} against {b}");
}

#[test]
fn excess_64_numbers_decode_to_their_values() {
    for (bytes, value) in known_numbers() {
        assert_eq!(real8(bytes), value, "{bytes:02x?}");
    }
    // the exact ones' forms are the writer's too
    assert_eq!(excess64(1.0), [0x41, 0x10, 0, 0, 0, 0, 0, 0]);
    assert_eq!(excess64(0.5), [0x40, 0x80, 0, 0, 0, 0, 0, 0]);
    assert_eq!(excess64(-2.5), [0xC1, 0x28, 0, 0, 0, 0, 0, 0]);
    assert_eq!(real8([0; 8]), 0.0);
    // every double's form decodes to it, to the bit
    for v in [
        1e-3,
        1e-9,
        -1e-9,
        0.1,
        1.0 / 3.0,
        90.0,
        -270.0,
        1.5,
        6.02e23,
        1e-70,
        7e75,
        -4.4e-16,
    ] {
        assert_eq!(real8(excess64(v)), v, "{v}");
    }
}

#[test]
fn units_give_micrometres() {
    let gds = one_cell(|w| w.rect([1, 0], [0, 0], [1500, 250]));
    assert_eq!(gds.database_unit().to_um(), 1e-3);
    let p = gds.polygons("top", [1, 0]).unwrap();
    let xs: Vec<f64> = p[0].vertices().iter().map(|q| q.x.to_um()).collect();
    assert!(xs.contains(&1.5) && xs.contains(&0.0), "{xs:?}");
    assert_eq!(gds.library(), "LIB");
    // a 10 nm unit: 150 of them are 1.5 µm
    let mut w = Writer::new(1e-2, 1e-8);
    w.begin("top");
    w.rect([1, 0], [0, 0], [150, 25]);
    w.end();
    let coarse = Gds::parse(&w.finish()).unwrap();
    assert_eq!(
        coarse
            .shape("top", [1, 0], None)
            .unwrap()
            .bounds()
            .max
            .x
            .to_um(),
        1.5
    );
}

#[test]
fn boundaries_boxes_and_texts() {
    let gds = one_cell(|w| {
        // a triangle with a repeated point, in clockwise order
        w.boundary([1, 0], &[[0, 0], [0, 3000], [0, 3000], [4000, 0]]);
        w.rect([2, 7], [0, 0], [2000, 1000]);
        w.label(1, [500, 500], "a label draws nothing");
    });
    close(area_on(&gds, [1, 0]), 6.0);
    close(area_on(&gds, [2, 7]), 2.0);
    assert_eq!(gds.layers("top").unwrap(), vec![[1, 0], [2, 7]]);
    let e = gds.shape("top", [3, 0], None).unwrap_err().to_string();
    assert!(
        e.contains("draws nothing on 3/0") && e.contains("1/0, 2/7"),
        "{e}"
    );
}

/// The number of segments a half circle of radius `h` database units is drawn with, as the
/// reader chooses: chords straying by at most half a unit.
fn segments(h: f64) -> f64 {
    (std::f64::consts::PI / (2.0 * (1.0 - 0.5 / h).acos())).ceil()
}

#[test]
fn paths_of_every_end() {
    // 4 µm long, 0.5 µm wide, each on a layer of its own
    let (l, w, h) = (4.0, 0.5, 0.25);
    let gds = one_cell(|g| {
        for (layer, kind) in [(0, 0), (1, 1), (2, 2), (4, 4)] {
            let ends = (kind == 4).then_some([100, 300]);
            g.path([layer, 0], kind, 500, ends, &[[0, 0], [4000, 0]]);
        }
        // no width: nothing
        g.path([5, 0], 0, 0, None, &[[0, 0], [4000, 0]]);
    });
    close(area_on(&gds, [0, 0]), w * l);
    close(area_on(&gds, [2, 0]), w * (l + w));
    close(area_on(&gds, [4, 0]), w * (l + 0.4));
    // round: two half circles of m chords each, their vertices on the circle
    let m = segments(250.0);
    let round = w * l + m * h * h * (std::f64::consts::PI / m).sin();
    close(area_on(&gds, [1, 0]), round);
    let disk = std::f64::consts::PI * h * h;
    // within half a database unit of the circle all round
    assert!(round < w * l + disk && w * l + disk - round < 0.5e-3 * 2.0 * std::f64::consts::PI * h);
    assert!(gds.shape("top", [5, 0], None).is_err());
}

#[test]
fn a_path_s_corners_are_mitred() {
    // a right angle and a 45° turn, 0.2 µm wide
    let gds = one_cell(|g| {
        g.path([1, 0], 0, 200, None, &[[0, 0], [1000, 0], [1000, 1000]]);
        g.path([2, 0], 0, 200, None, &[[0, 0], [1000, 0], [2000, 1000]]);
    });
    close(area_on(&gds, [1, 0]), 0.2 * 1.1 + 0.2 * 0.9);
    // each side's offset line meets the next's: the centre line's lengths times the width
    close(area_on(&gds, [2, 0]), 0.2 * (1.0 + 2f64.sqrt()));
    let turned = one_cell(|g| g.path([1, 0], 0, 200, None, &[[0, 0], [1000, 0], [0, 0]]));
    let e = turned.shape("top", [1, 0], None).unwrap_err().to_string();
    assert!(e.contains("turns back on itself"), "{e}");
}

/// A cell "t" of a right triangle (0, 0), (2, 0), (0, 1) µm on layer 1/0, placed by `place`.
fn placed(place: impl FnOnce(&mut Writer)) -> Gds {
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("t");
    w.boundary([1, 0], &[[0, 0], [2000, 0], [0, 1000]]);
    w.end();
    w.begin("top");
    place(&mut w);
    w.end();
    Gds::parse(&w.finish()).unwrap()
}

fn vertices(gds: &Gds) -> Vec<Vec<[f64; 2]>> {
    gds.polygons("top", [1, 0])
        .unwrap()
        .iter()
        .map(|p| {
            p.vertices()
                .iter()
                .map(|q| [q.x.to_um(), q.y.to_um()])
                .collect()
        })
        .collect()
}

fn has(rings: &[Vec<[f64; 2]>], ring: usize, point: [f64; 2]) {
    assert!(
        rings[ring]
            .iter()
            .any(|q| (q[0] - point[0]).abs() < 1e-12 && (q[1] - point[1]).abs() < 1e-12),
        "{point:?} not in {:?}",
        rings[ring]
    );
}

#[test]
fn references_reflect_magnify_turn_then_move() {
    // reflected about x, magnified 2, turned a quarter, moved to (10, 5): (2, 0) goes to
    // (4, 0), then (0, 4), then (10, 9); (0, 1) to (0, −1), (0, −2), (2, 0), (12, 5)
    let gds = placed(|w| w.sref("t", true, 2.0, 90.0, [10000, 5000]));
    let r = vertices(&gds);
    assert_eq!(r.len(), 1);
    for p in [[10.0, 5.0], [10.0, 9.0], [12.0, 5.0]] {
        has(&r, 0, p);
    }
    // a quarter turn is exact: no rounding in its vertices
    assert!(
        r[0].iter()
            .all(|q| q[0].fract() == 0.0 && q[1].fract() == 0.0)
    );
    close(gds.shape("top", [1, 0], None).unwrap().area(), 4.0);
    // 30°, unmagnified: the vertex (2, 0) at 2 (cos 30°, sin 30°)
    let turned = placed(|w| w.sref("t", false, 1.0, 30.0, [0, 0]));
    let c = 30f64.to_radians();
    has(&vertices(&turned), 0, [2.0 * c.cos(), 2.0 * c.sin()]);
}

#[test]
fn arrays_of_references() {
    // 3 columns 5 µm apart, 2 rows 3 µm apart, each turned half a turn
    let gds = placed(|w| {
        w.aref(
            "t",
            (false, 1.0, 180.0),
            [3, 2],
            [[0, 0], [15000, 0], [0, 6000]],
        );
    });
    let r = vertices(&gds);
    assert_eq!(r.len(), 6);
    // each copy's right angle at its place, its long leg along −x
    let mut k = 0;
    for j in 0..2 {
        for i in 0..3 {
            let (x, y) = (5.0 * f64::from(i), 3.0 * f64::from(j));
            has(&r, k, [x, y]);
            has(&r, k, [x - 2.0, y]);
            has(&r, k, [x, y - 1.0]);
            k += 1;
        }
    }
    close(gds.shape("top", [1, 0], None).unwrap().area(), 6.0);
}

#[test]
fn nested_references_flatten() {
    // a cell placing the triangle twice, placed itself three times, two levels down
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("t");
    w.boundary([1, 0], &[[0, 0], [2000, 0], [0, 1000]]);
    w.end();
    w.begin("pair");
    w.sref("t", false, 1.0, 0.0, [0, 0]);
    w.sref("t", true, 1.0, 0.0, [0, -500]);
    w.end();
    w.begin("top");
    w.aref(
        "pair",
        (false, 0.5, 0.0),
        [3, 1],
        [[0, 0], [9000, 0], [0, 1000]],
    );
    w.end();
    let gds = Gds::parse(&w.finish()).unwrap();
    assert_eq!(gds.cells(), vec!["t", "pair", "top"]);
    assert_eq!(gds.top_cells(), vec!["top"]);
    assert_eq!(gds.top_cell().unwrap(), "top");
    assert_eq!(gds.polygons("top", [1, 0]).unwrap().len(), 6);
    close(gds.shape("top", [1, 0], None).unwrap().area(), 6.0 * 0.25);
}

#[test]
fn a_layer_is_a_union_and_keyholes_are_holes() {
    let gds = one_cell(|w| {
        // two squares overlapping by 1 × 2 µm: 4 + 4 − 2
        w.boundary([1, 0], &[[0, 0], [2000, 0], [2000, 2000], [0, 2000]]);
        w.boundary([1, 0], &[[1000, 0], [3000, 0], [3000, 2000], [1000, 2000]]);
        // a square inside a square: not a hole, a union
        w.boundary([2, 0], &[[0, 0], [4000, 0], [4000, 4000], [0, 4000]]);
        w.boundary(
            [2, 0],
            &[[1000, 1000], [3000, 1000], [3000, 3000], [1000, 3000]],
        );
        // a keyhole: a 4 µm square less a 2 µm one, cut along y = 0 from x = 0 to 1
        w.boundary(
            [3, 0],
            &[
                [0, 0],
                [1000, 0],
                [1000, 1000],
                [3000, 1000],
                [3000, -1000],
                [1000, -1000],
                [1000, 0],
                [0, 0],
                [0, -2000],
                [4000, -2000],
                [4000, 2000],
                [0, 2000],
            ],
        );
    });
    close(area_on(&gds, [1, 0]), 6.0);
    close(area_on(&gds, [2, 0]), 16.0);
    let keyhole = gds.shape("top", [3, 0], None).unwrap();
    close(keyhole.area(), 16.0 - 4.0);
    assert!(matches!(keyhole, Shape::Boolean(_)), "{keyhole:?}");
    assert!(!keyhole.contains(Point::um(2.0, 0.0)));
    assert!(keyhole.contains(Point::um(0.5, 0.0)), "the cut is inside");
    // its distance at the cut is the hole's, 0.5 µm away, not the cut's
    let d = keyhole.distance(Point::um(0.5, 0.0)).signed.to_um();
    assert!((d + 0.5).abs() < 1e-9, "{d}");
}

#[test]
fn a_polygon_s_area_round_trips() {
    // an irregular heptagon's area, from its integer coordinates by the shoelace formula in
    // integers, read back
    let points = [
        [-1234, -987],
        [4321, -555],
        [6789, 2345],
        [3000, 5432],
        [-500, 4999],
        [-2222, 3333],
        [-3001, 17],
    ];
    let twice: i64 = (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            i64::from(a[0]) * i64::from(b[1]) - i64::from(b[0]) * i64::from(a[1])
        })
        .sum();
    let exact = twice as f64 / 2.0 * 1e-6;
    let gds = one_cell(|w| w.boundary([9, 3], &points));
    close(area_on(&gds, [9, 3]), exact);
    close(gds.polygons("top", [9, 3]).unwrap()[0].area(), exact);
}

#[test]
fn a_window_clips() {
    let gds = one_cell(|w| {
        w.rect([1, 0], [0, 0], [10000, 1000]);
        w.rect([1, 0], [20000, 0], [21000, 1000]);
    });
    let window = Bounds::new(Point::um(2.0, -1.0), Point::um(5.0, 0.5));
    let s = gds.shape("top", [1, 0], Some(window)).unwrap();
    close(s.area(), 3.0 * 0.5);
    let far = Bounds::new(Point::um(12.0, -1.0), Point::um(15.0, 2.0));
    let e = gds.shape("top", [1, 0], Some(far)).unwrap_err().to_string();
    assert!(e.contains("inside the window"), "{e}");
    // two pieces: an array of them
    let both = gds.shape("top", [1, 0], None).unwrap();
    assert!(matches!(both, Shape::Array(_)));
    close(both.area(), 11.0);
}

#[test]
fn the_case_s_layout_reads_back() {
    let e = super::gds_checks::area_error().max(super::gds_checks::number_error());
    assert!(e < 1e-13, "{e}");
}

/// The error parsing `bytes`, which must be one.
fn refused(bytes: &[u8]) -> String {
    Gds::parse(bytes).unwrap_err().to_string()
}

#[test]
fn a_file_cut_short_anywhere_is_an_error() {
    let bytes = layout();
    for n in 0..bytes.len() {
        let e = refused(&bytes[..n]);
        assert!(
            e.contains("cut short") || e.contains("HEADER"),
            "{n} bytes: {e}"
        );
    }
    assert!(Gds::parse(&bytes).is_ok());
}

#[test]
fn malformed_files_are_errors_not_panics() {
    let e = refused(b"not a GDSII file at all");
    assert!(e.contains("HEADER"), "{e}");
    // a record that says it is 2 bytes long
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    let mut bytes = w.finish();
    // before the ENDLIB, which ends the reading
    bytes.truncate(bytes.len() - 4);
    bytes.extend([0, 2, 0x08, 0]);
    assert!(refused(&bytes).contains("a record 2 bytes long"));
    // an odd length
    let mut w = Writer::new(1e-3, 1e-9);
    w.record(0x05, 0x02, &[0, 1, 2]);
    assert!(refused(&w.finish()).contains("even number"));
    // XY of 6 bytes
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    w.none(0x08);
    w.int2(0x0D, &[1]);
    w.int2(0x0E, &[0]);
    w.record(0x10, 0x03, &[0; 6]);
    assert!(refused(&w.finish()).contains("6 bytes"));
    // an element without its ENDEL, and one without its layer
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    w.none(0x08);
    w.int2(0x0D, &[1]);
    w.end();
    assert!(refused(&w.finish()).contains("has no ENDEL"));
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    w.none(0x08);
    w.int2(0x0E, &[0]);
    w.int4(0x10, &[0, 0, 1, 0, 1, 1, 0, 0]);
    w.none(0x11);
    w.end();
    assert!(refused(&w.finish()).contains("has no LAYER"));
    // no UNITS
    let mut w = Writer::without_units();
    w.begin("top");
    w.end();
    assert!(refused(&w.finish()).contains("no UNITS"));
    // a cell without its ENDSTR, and two of one name
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("a");
    w.begin("b");
    assert!(refused(&w.finish()).contains("no ENDSTR"));
    let mut w = Writer::new(1e-3, 1e-9);
    for _ in 0..2 {
        w.begin("a");
        w.end();
    }
    assert!(refused(&w.finish()).contains("second cell named \"a\""));
    // absolute magnification, a negative width, an AREF of no columns
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    w.none(0x0A);
    w.text(0x12, "x");
    w.int2(0x1A, &[0x0004]);
    w.int4(0x10, &[0, 0]);
    w.none(0x11);
    w.end();
    assert!(refused(&w.finish()).contains("absolute magnification"));
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    w.path([1, 0], 0, -100, None, &[[0, 0], [10, 0]]);
    w.end();
    assert!(refused(&w.finish()).contains("absolute width"));
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("top");
    w.aref("x", (false, 1.0, 0.0), [0, 1], [[0, 0], [0, 0], [0, 0]]);
    w.end();
    assert!(refused(&w.finish()).contains("0 columns"));
}

#[test]
fn what_flattening_refuses() {
    // a cell placing one the file doesn't have
    let gds = placed(|w| w.sref("missing", false, 1.0, 0.0, [0, 0]));
    let e = gds.polygons("top", [1, 0]).unwrap_err().to_string();
    assert!(e.contains("which the file doesn't have"), "{e}");
    // a cell placing itself, through another
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("a");
    w.sref("b", false, 1.0, 0.0, [0, 0]);
    w.end();
    w.begin("b");
    w.sref("a", false, 1.0, 0.0, [0, 0]);
    w.end();
    let gds = Gds::parse(&w.finish()).unwrap();
    let e = gds.polygons("a", [1, 0]).unwrap_err().to_string();
    assert!(e.contains("a cell places itself: a → b → a"), "{e}");
    assert!(
        gds.top_cell()
            .unwrap_err()
            .to_string()
            .contains("no top cell")
    );
    // too many: 32767 × 32767 copies, counted, not made
    let gds = placed(|w| {
        w.aref(
            "t",
            (false, 1.0, 0.0),
            [i16::MAX, i16::MAX],
            [[0, 0], [i32::MAX, 0], [0, i32::MAX]],
        );
    });
    let e = gds.polygons("top", [1, 0]).unwrap_err().to_string();
    assert!(e.contains("more than"), "{e}");
    // a cell the file doesn't have, and two top cells
    let mut w = Writer::new(1e-3, 1e-9);
    for name in ["a", "b"] {
        w.begin(name);
        w.end();
    }
    let gds = Gds::parse(&w.finish()).unwrap();
    assert!(
        gds.polygons("c", [1, 0])
            .unwrap_err()
            .to_string()
            .contains("no cell \"c\"")
    );
    assert!(
        gds.top_cell()
            .unwrap_err()
            .to_string()
            .contains("2 top cells (a, b)")
    );
}

#[test]
fn a_deep_hierarchy_is_refused() {
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("c0");
    w.rect([1, 0], [0, 0], [10, 10]);
    w.end();
    for k in 1..=MAX_DEPTH_TEST {
        w.begin(&format!("c{k}"));
        w.sref(&format!("c{}", k - 1), false, 1.0, 0.0, [0, 0]);
        w.end();
    }
    let gds = Gds::parse(&w.finish()).unwrap();
    // as deep as allowed: read
    assert_eq!(
        gds.polygons(&format!("c{}", GDS_MAX_DEPTH - 1), [1, 0])
            .unwrap()
            .len(),
        1
    );
    let e = gds
        .polygons(&format!("c{MAX_DEPTH_TEST}"), [1, 0])
        .unwrap_err()
        .to_string();
    assert!(e.contains("deeper than 64 levels"), "{e}");
}

const MAX_DEPTH_TEST: usize = GDS_MAX_DEPTH + 5;

#[test]
fn every_byte_changed_is_read_or_refused() {
    // each byte of the case's layout flipped in turn: never a panic, flattening included
    let bytes = layout();
    for k in 0..bytes.len() {
        for flip in [0x01u8, 0x80] {
            let mut b = bytes.clone();
            b[k] ^= flip;
            if let Ok(gds) = Gds::parse(&b) {
                for cell in gds.cells() {
                    for layer in gds.layers(cell).unwrap() {
                        let _ = gds.polygons(cell, layer);
                    }
                }
            }
        }
    }
}
