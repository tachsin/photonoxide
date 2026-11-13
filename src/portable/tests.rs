//! The portable functions against mpmath's correctly rounded values
//! (`tests/data/canary/portable.txt`, written by `portable.py` beside it), each held to the bound
//! its doc states; their bits hashed over a fixed grid, the same on every system; fdlibm's exp and
//! ln against the `libm` crate's, bit for bit; their special values; and genoxide's fixed bits for
//! the functions it shares.
//!
//! A bound in ulps here is a count of binary64 numbers between the result and the correctly
//! rounded value: a function within e ulps of the true value is within ⌊e + ½⌋ of those.

use std::path::Path;

use num_complex::Complex64 as c64;

use super::complex as cx;
use super::*;

const U: f64 = f64::EPSILON / 2.0;

/// One line of `portable.txt`: the arguments and the correctly rounded result.
#[derive(Clone, Copy)]
struct Row {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    r: f64,
    i: f64,
}

/// The reference rows of `name`, in the file's order (LF or CRLF).
fn reference(name: &str) -> Vec<Row> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/canary/portable.txt");
    let text = std::fs::read_to_string(&path).expect("tests/data/canary/portable.txt");
    let value = |s: &str| -> f64 {
        if s == "-" {
            f64::NAN
        } else {
            f64::from_bits(u64::from_str_radix(s, 16).expect("a value's bits in hex"))
        }
    };
    let rows: Vec<Row> = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 7, "portable.txt: {l}");
            (f[0] == name).then(|| Row {
                a: value(f[1]),
                b: value(f[2]),
                c: value(f[3]),
                d: value(f[4]),
                r: value(f[5]),
                i: value(f[6]),
            })
        })
        .collect();
    assert!(!rows.is_empty(), "portable.txt has no line for {name}");
    rows
}

/// How many binary64 numbers apart `x` and `y` are (0 for the same number, ±0 alike).
fn ulps(x: f64, y: f64) -> u64 {
    let key = |v: f64| -> i128 {
        let b = v.to_bits();
        let m = i128::from(b & !(1u64 << 63));
        if b >> 63 == 1 { -m } else { m }
    };
    (key(x) - key(y)).unsigned_abs() as u64
}

/// The spacing of binary64 numbers at |x|.
fn ulp(x: f64) -> f64 {
    let x = x.abs();
    if x < f64::MIN_POSITIVE {
        return f64::from_bits(1);
    }
    f64::from_bits(x.to_bits() & 0x7ff0_0000_0000_0000) * f64::EPSILON
}

/// The bound in binary64 numbers from the correctly rounded value of a function within `e` ulps
/// of the true value.
fn steps(e: f64) -> u64 {
    (e + 0.5).floor() as u64
}

/// A function's worst error over its reference rows, against its rule; failures listed.
struct Measured {
    name: &'static str,
    inputs: usize,
    worst: f64,
    not_rounded: usize,
    rule: String,
    failures: Vec<String>,
}

impl Measured {
    fn new(name: &'static str, inputs: usize, rule: String) -> Self {
        Self {
            name,
            inputs,
            worst: 0.0,
            not_rounded: 0,
            rule,
            failures: Vec::new(),
        }
    }
}

/// A real function of one or two arguments held to `rule(a, c)` numbers of the correctly rounded
/// value.
fn real(
    name: &'static str,
    f: impl Fn(f64, f64) -> f64,
    rule: impl Fn(f64, f64) -> u64,
    text: &str,
) -> Measured {
    let rows = reference(name);
    let mut m = Measured::new(name, rows.len(), text.to_owned());
    for g in &rows {
        // the second argument, or the imaginary part of a complex one
        let second = if g.c.is_nan() { g.b } else { g.c };
        let ours = f(g.a, second);
        let d = ulps(ours, g.r);
        m.worst = m.worst.max(d as f64);
        m.not_rounded += usize::from(d != 0);
        if d > rule(g.a, second) {
            m.failures.push(format!(
                "{name}({:e}, {second:e}) = {ours:e}, correctly rounded {:e}: {d} ulps",
                g.a, g.r
            ));
        }
    }
    m
}

/// A complex function held part by part to `rule(z)` = (real part's, imaginary part's) numbers
/// of the correctly rounded values.
fn parts(
    name: &'static str,
    f: impl Fn(&Row) -> c64,
    rule: impl Fn(&Row) -> (u64, u64),
    text: &str,
) -> Measured {
    let rows = reference(name);
    let mut m = Measured::new(name, rows.len(), text.to_owned());
    for g in &rows {
        let ours = f(g);
        let (dr, di) = (ulps(ours.re, g.r), ulps(ours.im, g.i));
        let (rr, ri) = rule(g);
        m.worst = m.worst.max(dr.max(di) as f64);
        m.not_rounded += usize::from(dr + di != 0);
        if dr > rr || di > ri {
            m.failures.push(format!(
                "{name}({}{:+}i; {:e}{:+e}i) = {ours}, correctly rounded {}{:+}i: {dr} and {di} ulps",
                g.a, g.b, g.c, g.d, g.r, g.i
            ));
        }
    }
    m
}

/// A complex function held normwise: |ours − w| ≤ `bound(row, w)` + half an ulp of each of w's
/// parts (the reference's own rounding); its worst error reported as the fraction of the bound
/// it used.
fn normwise(
    name: &'static str,
    f: impl Fn(&Row) -> c64,
    bound: impl Fn(&Row, c64) -> f64,
    text: &str,
) -> Measured {
    let rows = reference(name);
    let mut m = Measured::new(name, rows.len(), text.to_owned());
    for g in &rows {
        let w = c64::new(g.r, g.i);
        let ours = f(g);
        let off = (ours - w).norm_sqr().sqrt();
        let allowed = bound(g, w) + 0.5 * (ulp(w.re) + ulp(w.im));
        m.worst = m.worst.max(off / allowed);
        m.not_rounded += usize::from(ulps(ours.re, w.re) + ulps(ours.im, w.im) != 0);
        if off.is_nan() || off > allowed {
            m.failures.push(format!(
                "{name}({}{:+}i; {:e}{:+e}i) = {ours}, correctly rounded {w}: off {off:e}, allowed {allowed:e}",
                g.a, g.b, g.c, g.d
            ));
        }
    }
    m
}

/// |w| of a complex number without the system's hypot.
fn modulus(w: c64) -> f64 {
    w.norm_sqr().sqrt()
}

/// Every portable function against mpmath, each held to the bound its doc states: the largest
/// error Gladman et al. (2026) know for the libm crate's sources, a first-order count of the
/// roundings for our own. The table it prints is what these functions do on every system.
#[test]
fn canary_portable_functions_within_their_stated_ulps() {
    let by = |e: f64| move |_: f64, _: f64| steps(e);
    let known = "Gladman et al.'s largest known error";
    let z = |g: &Row| c64::new(g.a, g.b);
    let table = vec![
        real("exp", |a, _| exp(a), by(0.949), known),
        real("exp2", |a, _| exp2(a), by(0.751), known),
        real("exp_m1", |a, _| exp_m1(a), by(0.909), known),
        real("ln", |a, _| ln(a), by(0.946), known),
        real("ln_1p", |a, _| ln_1p(a), by(0.900), known),
        real("log2", |a, _| log2(a), by(0.921), known),
        real("log10", |a, _| log10(a), by(0.814), known),
        real("sin", |a, _| sin(a), by(0.831), known),
        real("cos", |a, _| cos(a), by(0.834), known),
        real("tan", |a, _| tan(a), by(1.02), known),
        real("asin", |a, _| asin(a), by(0.981), known),
        real("acos", |a, _| acos(a), by(0.930), known),
        real("atan", |a, _| atan(a), by(0.861), known),
        real("sinh", |a, _| sinh(a), by(1.88), known),
        real("cosh", |a, _| cosh(a), by(1.47), known),
        real("tanh", |a, _| tanh(a), by(2.22), known),
        real("asinh", |a, _| asinh(a), by(1.92), known),
        real("acosh", |a, _| acosh(a), by(2.25), known),
        real("atanh", |a, _| atanh(a), by(1.81), known),
        real(
            "cbrt",
            |a, _| cbrt(a),
            |_, _| 0,
            "correctly rounded (CORE-MATH)",
        ),
        real("erf", |a, _| erf(a), by(1.02), known),
        real("erfc", |a, _| erfc(a), by(4.08), known),
        real(
            "erfcx",
            |a, _| erfcx(a),
            |a, _| {
                if a < 0.0 {
                    19
                } else if a < 26.0 {
                    12
                } else {
                    6
                }
            },
            "its doc's count: 12, 19 below 0, 6 above 26",
        ),
        real("atan2", atan2, by(1.55), known),
        real("hypot", hypot, by(1.21), known),
        real("powf", powf, |a, c| steps(powf_ulps(a, c)), POWF_RULE),
        real(
            "powi",
            |a, n| powi(a, n as i32),
            |_, n| {
                let n = n as i64;
                n.unsigned_abs().saturating_sub(1) + u64::from(n < 0)
            },
            "|n| − 1 + [n < 0]: its products and division",
        ),
        real("c_abs", |a, b| cx::abs(c64::new(a, b)), by(1.21), "hypot's"),
        real("c_arg", |a, b| cx::arg(c64::new(a, b)), by(1.55), "atan2's"),
        parts(
            "c_cis",
            |g| cx::cis(g.a),
            |_| (1, 1),
            "each part: cos's and sin's",
        ),
        parts(
            "c_from_polar",
            |g| cx::from_polar(g.a, g.c),
            |_| (3, 3),
            "each part 3: its doc's count",
        ),
        parts("c_exp", |g| cx::exp(z(g)), |_| (5, 5), "each part 5"),
        parts("c_sin", |g| cx::sin(z(g)), |_| (6, 6), "each part 6"),
        parts("c_cos", |g| cx::cos(z(g)), |_| (6, 6), "each part 6"),
        parts("c_sinh", |g| cx::sinh(z(g)), |_| (6, 6), "each part 6"),
        parts("c_cosh", |g| cx::cosh(z(g)), |_| (6, 6), "each part 6"),
        parts("c_tanh", |g| cx::tanh(z(g)), |_| (21, 16), "21 and 16"),
        parts("c_tan", |g| cx::tan(z(g)), |_| (16, 21), "16 and 21"),
        parts(
            "c_sqrt",
            |g| cx::sqrt(z(g)),
            |g| if g.a >= 0.0 { (3, 4) } else { (4, 3) },
            "3 for √((|a| + |z|)/2), 4 for the other",
        ),
        parts("c_asin", |g| cx::asin(z(g)), |_| (11, 14), "11 and 14"),
        normwise(
            "c_ln",
            |g| cx::ln(z(g)),
            |_, w| 15.0 * U * modulus(w),
            "normwise 15u|w|",
        ),
        normwise(
            "c_powf",
            |g| cx::powf(z(g), g.c),
            |g, w| {
                let p = g.c;
                let theta = cx::arg(z(g));
                let pow = 2.0 * powf_ulps(modulus(z(g)), p);
                (pow + 2.42 * p.abs() + 4.1 * (theta * p).abs() + 2.67) * U * modulus(w)
            },
            "its doc's normwise count",
        ),
        normwise(
            "c_powc",
            |g| cx::powc(z(g), c64::new(g.c, g.d)),
            |g, w| {
                let exponent = c64::new(g.c, g.d);
                let log = cx::ln(z(g));
                ((15.0 + 5f64.sqrt()) * modulus(exponent) * modulus(log) + 4.57) * U * modulus(w)
            },
            "its doc's normwise count",
        ),
    ];
    println!(
        "{:<13} {:>6} {:>10} {:>12}  rule",
        "function", "inputs", "worst", "not rounded"
    );
    let mut failures = Vec::new();
    for m in table {
        println!(
            "{:<13} {:>6} {:>10.2} {:>12}  {}",
            m.name, m.inputs, m.worst, m.not_rounded, m.rule
        );
        failures.extend(m.failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const POWF_RULE: &str = "1 (fdlibm's \"nearly rounded\"), 636 near base 1 (Gladman et al.)";

/// powf's stated bound in ulps at (base, exponent): fdlibm's "nearly rounded", an ulp, but
/// Gladman et al.'s 636 where base is within 2⁻¹⁰ of 1 and |exponent| above 10⁶, around their
/// worst case (base 1 + 1.65e-7, exponent 3.85e9), where `e_pow.c`'s log₂ of the base loses
/// digits that the exponent multiplies.
fn powf_ulps(base: f64, exponent: f64) -> f64 {
    if (base - 1.0).abs() < 1.0 / 1024.0 && exponent.abs() > 1e6 {
        636.0
    } else {
        1.0
    }
}

/// Folds a value's bits into a hash, order-sensitive; a NaN as one fixed NaN, since IEEE 754
/// leaves a NaN's sign and payload to the CPU (x86 makes its NaNs negative, ARM positive).
fn fold(h: &mut u64, v: f64) {
    let v = if v.is_nan() { f64::NAN } else { v };
    *h = h.rotate_left(7) ^ v.to_bits();
}

/// Functions with their names.
type Real = (&'static str, fn(f64) -> f64);
type Two = (&'static str, fn(f64, f64) -> f64);
type Complex = (&'static str, fn(c64) -> c64);

/// The grid of arguments: special values, the subnormals' edge, and 600 from a fixed LCG over
/// several scales.
fn grid() -> Vec<f64> {
    let mut xs = vec![
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(3),
        f64::MAX,
        -f64::MAX,
        1.0,
        -1.0,
        0.5,
        2.0,
        std::f64::consts::PI,
        std::f64::consts::FRAC_PI_2,
        1e22,
        -709.5,
        710.0,
        26.5,
    ];
    let mut s: u64 = 0x0822_2026_1113_0001;
    let mut unit = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (s >> 11) as f64 / 9_007_199_254_740_992.0
    };
    for scale in [1.0, 4.0, 30.0, 800.0, 1e6] {
        for _ in 0..120 {
            xs.push((unit() - 0.5) * 2.0 * scale);
        }
    }
    xs
}

/// The hash of each portable function's bits over the grid (pairs and complex arguments from
/// consecutive grid values), and of all of them: the same on Linux, Windows and macOS, x86-64
/// and ARM, as they are `+ − × ÷ √` and integer operations only. The hashes were taken on
/// Windows x86-64; a system that gives another has a primitive that isn't IEEE (a fused
/// multiply-add, a flushed subnormal), which is what this canary is for.
#[test]
fn canary_portable_bits_over_a_grid() {
    let xs = grid();
    let pairs: Vec<(f64, f64)> = xs
        .iter()
        .zip(xs.iter().rev())
        .map(|(&a, &b)| (a, b))
        .collect();
    let reals: [Real; 25] = [
        ("exp", exp),
        ("exp2", exp2),
        ("exp_m1", exp_m1),
        ("ln", ln),
        ("ln_1p", ln_1p),
        ("log2", log2),
        ("log10", log10),
        ("sin", sin),
        ("cos", cos),
        ("tan", tan),
        ("asin", asin),
        ("acos", acos),
        ("atan", atan),
        ("sinh", sinh),
        ("cosh", cosh),
        ("tanh", tanh),
        ("asinh", asinh),
        ("acosh", acosh),
        ("atanh", atanh),
        ("cbrt", cbrt),
        ("erf", erf),
        ("erfc", erfc),
        ("erfcx", erfcx),
        ("sin_cos.0", |x| sin_cos(x).0),
        ("sin_cos.1", |x| sin_cos(x).1),
    ];
    let twos: [Two; 4] = [
        ("atan2", atan2),
        ("hypot", hypot),
        ("powf", powf),
        ("powi", |x, n| powi(x, (n * 7.0) as i32 % 64)),
    ];
    let complexes: [Complex; 14] = [
        ("c_exp", cx::exp),
        ("c_ln", cx::ln),
        ("c_sqrt", cx::sqrt),
        ("c_sin", cx::sin),
        ("c_cos", cx::cos),
        ("c_tan", cx::tan),
        ("c_sinh", cx::sinh),
        ("c_cosh", cx::cosh),
        ("c_tanh", cx::tanh),
        ("c_asin", cx::asin),
        ("c_cis", |z| cx::cis(z.re)),
        ("c_from_polar", |z| cx::from_polar(z.re, z.im)),
        ("c_polar", |z| {
            let (r, t) = cx::to_polar(z);
            c64::new(r, t)
        }),
        ("c_powf", |z| cx::powf(z, z.im / 3.0)),
    ];
    let mut hashes: Vec<(&str, u64)> = Vec::new();
    for (name, f) in reals {
        let mut h = 0u64;
        xs.iter().for_each(|&x| fold(&mut h, f(x)));
        hashes.push((name, h));
    }
    for (name, f) in twos {
        let mut h = 0u64;
        pairs.iter().for_each(|&(a, b)| fold(&mut h, f(a, b)));
        hashes.push((name, h));
    }
    for (name, f) in complexes {
        let mut h = 0u64;
        for &(a, b) in &pairs {
            let w = f(c64::new(a, b));
            fold(&mut h, w.re);
            fold(&mut h, w.im);
        }
        hashes.push((name, h));
    }
    let mut h = 0u64;
    for w in pairs.windows(2) {
        let p = cx::powc(
            c64::new(w[0].0, w[0].1),
            c64::new(w[1].0 / 50.0, w[1].1 / 50.0),
        );
        fold(&mut h, p.re);
        fold(&mut h, p.im);
    }
    hashes.push(("c_powc", h));
    let mut all = 0u64;
    for (_, h) in &hashes {
        all = all.rotate_left(7) ^ h;
    }
    let mut changed = Vec::new();
    for ((name, h), (want_name, want)) in hashes.iter().zip(GRID_HASHES) {
        assert_eq!(name, want_name, "GRID_HASHES is in another order");
        if h != want {
            changed.push(format!("{name}: {h:#018x}, was {want:#018x}"));
        }
    }
    for (name, h) in &hashes {
        println!("(\"{name}\", {h:#018x}),");
    }
    println!("all: {all:#018x}");
    assert_eq!(hashes.len(), GRID_HASHES.len(), "a function without a hash");
    assert!(
        changed.is_empty(),
        "the portable functions' bits changed:\n{}",
        changed.join("\n")
    );
    assert_eq!(all, 0x1470_53dc_ba55_86e5, "{all:#018x}");
}

/// The grid hashes, taken on Windows x86-64 (Rust 1.100, libm 0.2.16); all of them folded together
/// in this order are 0x1470_53dc_ba55_86e5.
const GRID_HASHES: &[(&str, u64)] = &[
    ("exp", 0x413fbc04627e9685),
    ("exp2", 0x9d1a969c265cac84),
    ("exp_m1", 0x73190be6743b2d52),
    ("ln", 0x0bf1e9d3216efc53),
    ("ln_1p", 0x8df40320ac11090d),
    ("log2", 0xcb1de7f24e12948c),
    ("log10", 0x108e95a5b97f8f88),
    ("sin", 0xda525217fa01fbf0),
    ("cos", 0xda5c8775269b6b93),
    ("tan", 0xfc5ac4b85b72f239),
    ("asin", 0x691ee862ffa6959c),
    ("acos", 0xe9da63cec2870c8d),
    ("atan", 0x399728f358c0c9c1),
    ("sinh", 0xb8afd24875a5ef3b),
    ("cosh", 0x9ca08815778100d4),
    ("tanh", 0x866b49c89f66d53f),
    ("asinh", 0xe1dd727df1eb8581),
    ("acosh", 0xdab92ee03691324e),
    ("atanh", 0xacfcb0b29248ba7a),
    ("cbrt", 0xc3574e1416ac59ff),
    ("erf", 0x8a451d6966372d7e),
    ("erfc", 0xb1a60c7d9ccd9dcd),
    ("erfcx", 0x61fa5a0f4d3a8080),
    ("sin_cos.0", 0xda525217fa01fbf0),
    ("sin_cos.1", 0xda5c8775269b6b93),
    ("atan2", 0x5401923556e0ee65),
    ("hypot", 0xac2e5de82b7d5c15),
    ("powf", 0x9133a6eca7e0bb06),
    ("powi", 0x0193356f37480a41),
    ("c_exp", 0x9fc0eedeccb4ca6e),
    ("c_ln", 0x591636a61903229a),
    ("c_sqrt", 0xc84405d54e382130),
    ("c_sin", 0x48ecbbd8307e75d7),
    ("c_cos", 0xd7be7c1533a6ade2),
    ("c_tan", 0x1e42b83ec8bd89a1),
    ("c_sinh", 0x09a40766a282a08c),
    ("c_cosh", 0x55bfe4993ce04732),
    ("c_tanh", 0x9d59b17cfa390053),
    ("c_asin", 0x76f21b923d0bfc4f),
    ("c_cis", 0x3473b31bbcbe5479),
    ("c_from_polar", 0x09a50854d7a51ef5),
    ("c_polar", 0xb2a10e13b32679f9),
    ("c_powf", 0x3ed93e8697cc3f1f),
    ("c_powc", 0xc4ec6181006b7aa1),
];

/// fdlibm's exp, ported, gives the `libm` crate's bits (its `exp` is the same fdlibm code), and
/// fdlibm's ln is within one binary64 number of the crate's `log` (musl's shorter rewrite of
/// fdlibm's, whose bits differ at times: both are within an ulp of the true value, by fdlibm's
/// error analysis that both sources quote), over genoxide's arguments: (0, 1], every positive
/// finite double, subnormals included, [−745, 745] and [−2, 2], and the edges.
#[test]
fn exp_is_the_libm_crates_bit_for_bit_and_ln_within_one() {
    let mut s: u64 = 0x0822_0000_0000_0001;
    let mut next = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        s
    };
    let mut xs = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        2.0,
        1e-300,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        709.0,
        709.78,
        -745.0,
        -744.0,
        1e-30,
        -1e-30,
        powi(2.0, -29),
        1.0 - 1e-12,
        1.0 + 1e-12,
    ];
    for _ in 0..20_000 {
        let unit = (next() >> 11) as f64 / 9_007_199_254_740_992.0;
        xs.push(1.0 - unit);
        xs.push(f64::from_bits(next() % 0x7ff0_0000_0000_0000));
        xs.push((unit - 0.5) * 2.0 * 745.0);
        xs.push((unit - 0.5) * 4.0);
    }
    let same = |a: f64, b: f64| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
    let mut apart = 0;
    for &x in &xs {
        assert!(same(exp(x), libm::exp(x)), "exp({x:e})");
        let (ours, theirs) = (ln(x), libm::log(x));
        if !same(ours, theirs) {
            assert!(
                ulps(ours, theirs) <= 1,
                "ln({x:e}) = {ours:e}, libm's {theirs:e}"
            );
            apart += 1;
        }
    }
    println!("ln: {apart} of {} apart from libm's log by one", xs.len());
}

/// genoxide's fixed bits for the functions it shares with this module, from its own CI on the
/// three systems: the same code gives the same bits here.
#[test]
fn genoxides_bits() {
    let values = [
        exp(1.0),
        exp(-3.5),
        exp(0.1),
        sin(1.0),
        cos(1.0),
        sin(1e6),
        cos(-420.9687),
        tan(0.7),
        atan(9.0 * std::f64::consts::PI),
        atan2(1.0, -2.5),
        powf(2.0, 0.8),
        powf(0.3, 0.25),
        cbrt(3.0),
        sin_cos(2.0).0,
        sin_cos(2.0).1,
        ln_1p(1e-10),
        ln_1p(-0.5),
        ln_1p(-1e-3),
        ln_1p(2.5),
        exp_m1(1e-10),
        exp_m1(-0.021),
        exp_m1(-1.0),
        exp_m1(0.7),
        erf(0.5),
        erfc(0.5),
        erfcx(0.7),
        erfcx(-4.2),
        erfcx(31.0),
    ];
    assert_eq!(
        values.map(f64::to_bits),
        [
            4613303445314885482,  // 2.7182818284590455, 1 ulp above e
            4584361024591036262,  // 0.0301973834223185
            4607656066507473108,  // 1.1051709180756477
            4605754516372524270,  // 0.8414709848078965
            4603041830072026764,  // 0.5403023058681398
            13823348579196524647, // -0.34999350217129294
            4607182318654989009,  // 0.999988881668367
            4605761878818060462,  // 0.8422883804630794
            4609593841035272243,  // 1.5354432975345957
            4613399832362958176,  // 2.761086276477428
            4610520041557582112,  // 1.7411011265922482
            4604841292830345951,  // 0.7400828044922853
            4609174133800058614,  // 1.4422495703074083
            4606365442650518598,  // 0.9092974268256817
            13824540291724702213, // -0.4161468365471424
            4457293557087196819,  // 9.999999999500001e-11
            13827790571168217583, // -0.6931471805599453
            13785628853153536879, // -0.0010005003335835335
            4608320762010746208,  // 1.252762968495368
            4457293557087970531,  // 1.00000000005e-10
            13805018983922166456, // -0.020781035430540412
            13827240892226439268, // -0.6321205588285577
            4607244355488256781,  // 1.0137527074704764
            4602863465656806866,  // 0.5204998778130465
            4602309526204327004,  // 0.4795001221869535
            4602912378887895979,  // 0.525930337349441
            4725920843874906963,  // 91618811.94453172
            4580900192363489479,  // 0.018190209599233478
        ],
        "the portable maths moved from genoxide's bits"
    );
    let powers = [
        (1.1, 3, 4608673110276677044),                 // 1.3310000000000004
        (7.6268145870115305, -3, 4567343990406518369), // 0.002254085754944055
        (0.3, -2, 4622444617537217422),                // 11.11111111111111
        (2.5, 7, 4648579924938981376),                 // 610.3515625
        (-1.7, 5, 13847554739229683444),               // -14.198569999999997
        (3.3, 20, 4761950407933306278),                // 23457341881.036766
        (0.9, -31, 4628070934801639606),               // 26.21091652880633
    ];
    for (x, n, bits) in powers {
        assert_eq!(powi(x, n).to_bits(), bits, "powi({x}, {n})");
    }
}

/// The real functions' special values: C's for the libm ones, and those the docs state.
#[test]
fn the_real_functions_special_values() {
    assert_eq!(ln(0.0), f64::NEG_INFINITY);
    assert_eq!(ln(-0.0), f64::NEG_INFINITY);
    assert_eq!(ln(f64::INFINITY), f64::INFINITY);
    assert!(ln(-1.0).is_nan() && ln(f64::NAN).is_nan());
    assert_eq!(exp(710.0), f64::INFINITY);
    assert_eq!(exp(-746.0), 0.0);
    assert_eq!(exp(f64::NEG_INFINITY), 0.0);
    assert!(exp(f64::NAN).is_nan());
    assert_eq!(ln_1p(-1.0), f64::NEG_INFINITY);
    assert_eq!(exp_m1(f64::NEG_INFINITY), -1.0);
    assert_eq!(ln_1p(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(ln_1p(5e-324), 5e-324);
    assert_eq!(exp_m1(-5e-324), -5e-324);
    // musl's acosh gives 8.13 here (Gladman et al.); ours NaN, as below 1 everywhere
    assert!(acosh(f64::from_bits(0xc133_4e72_9fd0_8086)).is_nan());
    assert!(acosh(0.5).is_nan() && acosh(-1.0).is_nan() && acosh(f64::NAN).is_nan());
    assert_eq!(acosh(1.0), 0.0);
    assert!(asin(1.5).is_nan() && acos(-1.5).is_nan() && atanh(2.0).is_nan());
    assert_eq!(atanh(1.0), f64::INFINITY);
    assert_eq!(hypot(f64::INFINITY, f64::NAN), f64::INFINITY);
    assert_eq!(atan2(0.0, -1.0), std::f64::consts::PI);
    assert_eq!(atan2(-0.0, -1.0), -std::f64::consts::PI);
    assert_eq!(powf(0.0, 0.0), 1.0);
    assert!(powf(-8.0, 1.0 / 3.0).is_nan());
    assert_eq!(cbrt(-8.0), -2.0);
    // powi: 0⁰ = 1 for every x, NaN included; ±0 to negative powers
    assert_eq!(powi(0.0, 0), 1.0);
    assert_eq!(powi(f64::NAN, 0), 1.0);
    assert!(powi(f64::NAN, 2).is_nan());
    assert_eq!(powi(0.0, -1), f64::INFINITY);
    assert_eq!(powi(-0.0, -1), f64::NEG_INFINITY);
    assert_eq!(powi(-0.0, -2), f64::INFINITY);
    assert_eq!(powi(-2.0, 3), -8.0);
    assert_eq!(powi(2.0, -1022), f64::MIN_POSITIVE);
    assert_eq!(powi(2.0, i32::MIN), 0.0);
    for x in [0.3, -7.25, 1e-300, 3.1e200] {
        assert_eq!(powi(x, 1), x);
        assert_eq!(powi(x, 2), x * x);
        assert_eq!(powi(x, -1), 1.0 / x);
    }
    assert_eq!(erfcx(-27.0), f64::INFINITY);
    assert_eq!(erfcx(f64::INFINITY), 0.0);
    assert!(erfcx(f64::NAN).is_nan());
    // erfcx: decreasing, across its branch at 26 too
    let mut previous = erfcx(0.0);
    for k in 1..=3000 {
        let x = f64::from(k) / 50.0;
        let value = erfcx(x);
        assert!(value <= previous, "erfcx({x})");
        previous = value;
    }
}

/// Asserts the bits of a complex result, so that signed zeros and NaN count.
fn same(w: c64, want: (f64, f64)) -> bool {
    let eq = |a: f64, b: f64| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
    eq(w.re, want.0) && eq(w.im, want.1)
}

/// The complex functions' special values (C99's Annex G where their docs say so) and the sides
/// of the branch cuts.
#[test]
fn the_complex_functions_special_values_and_branch_cuts() {
    let (inf, nan) = (f64::INFINITY, f64::NAN);
    let c = c64::new;
    let checks: Vec<(&str, c64, (f64, f64))> = vec![
        ("exp", cx::exp(c(0.5, -0.0)), (exp(0.5), -0.0)),
        ("exp", cx::exp(c(0.5, 0.0)), (exp(0.5), 0.0)),
        ("exp", cx::exp(c(-inf, inf)), (0.0, 0.0)),
        ("exp", cx::exp(c(-inf, nan)), (0.0, 0.0)),
        ("exp", cx::exp(c(inf, 0.0)), (inf, 0.0)),
        ("exp", cx::exp(c(inf, -0.0)), (inf, -0.0)),
        ("exp", cx::exp(c(inf, inf)), (inf, nan)),
        ("exp", cx::exp(c(nan, -0.0)), (nan, -0.0)),
        ("ln", cx::ln(c(0.0, 0.0)), (-inf, 0.0)),
        ("ln", cx::ln(c(-0.0, 0.0)), (-inf, std::f64::consts::PI)),
        ("ln", cx::ln(c(-2.0, 0.0)), (ln(2.0), std::f64::consts::PI)),
        (
            "ln",
            cx::ln(c(-2.0, -0.0)),
            (ln(2.0), -std::f64::consts::PI),
        ),
        ("ln", cx::ln(c(inf, nan)), (inf, nan)),
        ("ln", cx::ln(c(1.0, 0.0)), (0.0, 0.0)),
        ("sqrt", cx::sqrt(c(-4.0, 0.0)), (0.0, 2.0)),
        ("sqrt", cx::sqrt(c(-4.0, -0.0)), (0.0, -2.0)),
        ("sqrt", cx::sqrt(c(4.0, -0.0)), (2.0, -0.0)),
        ("sqrt", cx::sqrt(c(0.0, 0.0)), (0.0, 0.0)),
        ("sqrt", cx::sqrt(c(-0.0, -0.0)), (0.0, -0.0)),
        ("sqrt", cx::sqrt(c(nan, inf)), (inf, inf)),
        ("sqrt", cx::sqrt(c(1.0, -inf)), (inf, -inf)),
        ("sqrt", cx::sqrt(c(inf, 1.0)), (inf, 0.0)),
        ("sqrt", cx::sqrt(c(-inf, -1.0)), (0.0, -inf)),
        ("sqrt", cx::sqrt(c(nan, 1.0)), (nan, nan)),
        ("sqrt", cx::sqrt(c(0.0, 2.0)), (1.0, 1.0)),
        ("sinh", cx::sinh(c(800.0, 0.0)), (inf, 0.0)),
        ("sinh", cx::sinh(c(-800.0, -0.0)), (-inf, -0.0)),
        ("cosh", cx::cosh(c(800.0, 0.0)), (inf, 0.0)),
        ("cosh", cx::cosh(c(-800.0, 0.0)), (inf, -0.0)),
        ("cosh", cx::cosh(c(inf, 0.0)), (inf, 0.0)),
        ("sin", cx::sin(c(0.0, 800.0)), (0.0, inf)),
        ("sin", cx::sin(c(-0.0, -800.0)), (-0.0, -inf)),
        ("cos", cx::cos(c(0.0, 800.0)), (inf, -0.0)),
        ("cos", cx::cos(c(-0.0, 800.0)), (inf, 0.0)),
        ("tanh", cx::tanh(c(inf, 1.0)), (1.0, 0.0)),
        ("tanh", cx::tanh(c(-inf, -2.0)), (-1.0, 0.0)),
        ("tanh", cx::tanh(c(0.0, inf)), (0.0, nan)),
        ("tanh", cx::tanh(c(1.0, inf)), (nan, nan)),
        ("tanh", cx::tanh(c(nan, 0.0)), (nan, 0.0)),
        ("tanh", cx::tanh(c(400.0, 1.0)), (1.0, 0.0)),
        ("tanh", cx::tanh(c(0.0, 0.0)), (0.0, 0.0)),
        ("tan", cx::tan(c(1.0, 400.0)), (0.0, 1.0)),
        ("tan", cx::tan(c(-0.0, -0.0)), (-0.0, -0.0)),
        ("asin", cx::asin(c(0.0, 0.0)), (0.0, 0.0)),
        ("asin", cx::asin(c(-0.0, -0.0)), (-0.0, -0.0)),
        ("asin", cx::asin(c(0.0, nan)), (0.0, nan)),
        ("asin", cx::asin(c(nan, inf)), (nan, inf)),
        ("asin", cx::asin(c(nan, 1.0)), (nan, nan)),
        (
            "asin",
            cx::asin(c(inf, 1.0)),
            (std::f64::consts::FRAC_PI_2, inf),
        ),
        (
            "asin",
            cx::asin(c(-inf, -inf)),
            (-std::f64::consts::FRAC_PI_4, -inf),
        ),
        ("asin", cx::asin(c(1.0, inf)), (0.0, inf)),
        (
            "asin",
            cx::asin(c(2.0, 0.0)),
            (std::f64::consts::FRAC_PI_2, acosh(2.0)),
        ),
        (
            "asin",
            cx::asin(c(2.0, -0.0)),
            (std::f64::consts::FRAC_PI_2, -acosh(2.0)),
        ),
        (
            "asin",
            cx::asin(c(-2.0, 0.0)),
            (-std::f64::consts::FRAC_PI_2, acosh(2.0)),
        ),
        ("asin", cx::asin(c(0.5, 0.0)), (asin(0.5), 0.0)),
        ("powf", cx::powf(c(nan, nan), 0.0), (1.0, 0.0)),
        ("powf", cx::powf(c(0.0, 0.0), 2.0), (0.0, 0.0)),
        ("powc", cx::powc(c(0.0, 0.0), c(0.5, 3.0)), (0.0, 0.0)),
        ("powc", cx::powc(c(nan, 1.0), c(0.0, 0.0)), (1.0, 0.0)),
        ("cis", cx::cis(0.0), (1.0, 0.0)),
        ("from_polar", cx::from_polar(2.0, 0.0), (2.0, 0.0)),
    ];
    let wrong: Vec<String> = checks
        .iter()
        .filter(|(_, w, want)| !same(*w, *want))
        .map(|(name, w, want)| format!("{name}: {w}, not {}{:+}i", want.0, want.1))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(cx::abs(c(3.0, -4.0)), 5.0);
    let big = cx::sqrt(c(f64::MAX, f64::MAX));
    assert!(
        big.re.is_finite() && big.im.is_finite() && big.re > big.im,
        "{big}"
    );
    assert_eq!(
        cx::to_polar(c(0.0, 2.0)),
        (2.0, std::f64::consts::FRAC_PI_2)
    );
    // the symmetries asin's first-quadrant algorithm is built on hold bit for bit
    for (a, b) in [
        (0.3, 0.7),
        (1.5, 1e-3),
        (4.0, 9.0),
        (0.9, 1e-200),
        (3e-9, 2e-9),
    ] {
        let w = cx::asin(c(a, b));
        assert!(same(cx::asin(c(-a, b)), (-w.re, w.im)));
        assert!(same(cx::asin(c(a, -b)), (w.re, -w.im)));
        assert!(same(cx::asin(c(-a, -b)), (-w.re, -w.im)));
    }
}

/// The portable exp, sin and cos (and their complex forms) within an ulp or two of the system's:
/// the system's functions as an independent reference, which the rule allows in a test.
#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "the system's functions, as an independent reference"
)]
fn they_are_within_an_ulp_or_two_of_the_systems() {
    let close = |a: f64, b: f64| (a - b).abs() <= 4.0 * f64::EPSILON * b.abs().max(1e-300);
    for x in arguments() {
        let (s, c) = sin_cos(x);
        assert!(close(s, x.sin()) || (s - x.sin()).abs() < 1e-15, "sin {x}");
        assert!(close(c, x.cos()) || (c - x.cos()).abs() < 1e-15, "cos {x}");
        let e = x / 2000.0 * 700.0;
        assert!(close(exp(e), e.exp()), "exp {e}");
        let z = c64::new(x / 2000.0, x / 300.0);
        for (ours, theirs) in [
            (cx::exp(z), z.exp()),
            (cx::cos(z), z.cos()),
            (cx::sin(z), z.sin()),
        ] {
            assert!(
                (ours - theirs).norm() <= 1e-15 * theirs.norm().max(1.0),
                "{z}"
            );
        }
    }
    assert_eq!(cx::exp(c64::new(0.5, 0.0)), c64::new(exp(0.5), 0.0));
}

/// 4000 arguments over the ranges the circuit's phases and the waveforms' envelopes take.
fn arguments() -> impl Iterator<Item = f64> {
    (0..4000).map(|k| {
        let t = f64::from(k) / 4000.0;
        (t - 0.5) * 2000.0 * (1.0 + 0.37 * (17.0 * t).fract())
    })
}

/// The bits of exp, sin_cos and the complex exp, cos and sin over [`arguments`], folded into one
/// word: the hash these had as `portable.rs`'s functions, before the module was a directory.
#[test]
fn they_give_the_same_bits_on_every_system() {
    let mut h = 0u64;
    let mut fold = |v: f64| h = h.rotate_left(7) ^ v.to_bits();
    for x in arguments() {
        let (s, c) = sin_cos(x);
        fold(s);
        fold(c);
        fold(exp(x / 2000.0 * 700.0));
        let z = c64::new(x / 2000.0, x / 300.0);
        for w in [cx::exp(z), cx::cos(z), cx::sin(z)] {
            fold(w.re);
            fold(w.im);
        }
    }
    assert_eq!(h, 0xbac4_8fd0_05df_2b48, "{h:#018x}");
}
