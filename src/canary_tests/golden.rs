//! The platform canaries' golden values, `tests/data/canary/golden.txt`: exact results correctly
//! rounded to binary64, written once by `tests/data/canary/golden.py` (mpmath at 256 bits), and
//! the distance between two numbers in ulps. Shared by `src/canary_tests.rs` and
//! `tests/canary.rs` (which takes this file by `#[path]`).

use std::path::Path;

/// One line of the file: a function's arguments and its result.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Golden {
    /// The (first) argument, or the real part of a complex one.
    pub(crate) a: f64,
    /// The second argument, or the imaginary part; `NAN` when there is none.
    pub(crate) b: f64,
    /// The result, or its real part.
    pub(crate) r: f64,
    /// The result's imaginary part; `NAN` when it is real.
    pub(crate) i: f64,
}

/// The golden values of `name`, in the file's order. The file may have LF or CRLF line endings
/// (git's `core.autocrlf` writes CRLF on Windows checkouts).
///
/// # Panics
///
/// If the file can't be read, a line doesn't parse, or `name` has no line.
pub(crate) fn golden(name: &str) -> Vec<Golden> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/canary/golden.txt");
    let text = std::fs::read_to_string(&path).expect("tests/data/canary/golden.txt");
    let value = |s: &str| -> f64 {
        if s == "-" {
            f64::NAN
        } else {
            f64::from_bits(u64::from_str_radix(s, 16).expect("a value's bits in hex"))
        }
    };
    let rows: Vec<Golden> = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 5, "golden.txt: {l}");
            (f[0] == name).then(|| Golden {
                a: value(f[1]),
                b: value(f[2]),
                r: value(f[3]),
                i: value(f[4]),
            })
        })
        .collect();
    assert!(!rows.is_empty(), "golden.txt has no line for {name}");
    rows
}

/// The spacing of binary64 numbers at |x|: 2^(e − 52) for 2^e ≤ |x| < 2^(e+1), the smallest
/// subnormal's below.
pub(crate) fn ulp(x: f64) -> f64 {
    let x = x.abs();
    if x < f64::MIN_POSITIVE {
        return f64::from_bits(1);
    }
    let e = (x.to_bits() >> 52) as i32 - 1023;
    // 2^p written into the bits: exact, no maths function
    let p = e - 52;
    if p >= -1022 {
        f64::from_bits(((p + 1023) as u64) << 52)
    } else {
        f64::from_bits(1u64 << (p + 1074))
    }
}

/// How many binary64 numbers apart `x` and `y` are (0 for the same number, ±0 alike): the
/// distance in ulps of a result from the correctly rounded one.
pub(crate) fn ulps(x: f64, y: f64) -> u64 {
    let key = |v: f64| -> i128 {
        let b = v.to_bits();
        let m = i128::from(b & !(1u64 << 63));
        if b >> 63 == 1 { -m } else { m }
    };
    (key(x) - key(y)).unsigned_abs() as u64
}
