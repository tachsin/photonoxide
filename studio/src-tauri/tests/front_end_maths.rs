//! The front end's maths against the library's (#822): the window computes a few numbers it shows
//! as results itself (S-parameters and spectra in dB, their phases, an error's log₁₀), with
//! `studio/src/lib/portable.ts`, a port of `photonoxide::portable::log10` and `atan2`.
//! `studio/tests/portable-maths.json` holds a fixed set of arguments and the bits
//! `photonoxide::portable` gives for them; this test holds the library to the file and
//! `studio/tests/portable.test.ts` (`pnpm test`) holds the port to it, so the two give the same
//! bits or a test fails. Besides its arguments, the file holds a hash of each function's bits over
//! a grid of 200 000 random arguments that both tests draw alike (splitmix64 from fixed seeds).
//!
//! The file is written by this test with `PHOTONOXIDE_WRITE_PORTABLE_MATHS=1`, only when its
//! arguments change or `photonoxide::portable` changes on purpose.

use std::path::PathBuf;

use photonoxide::portable;

fn file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/portable-maths.json")
}

/// The number just above x (toward +∞), for x finite and not −0.
fn next_up(x: f64) -> f64 {
    if x >= 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else {
        f64::from_bits(x.to_bits() - 1)
    }
}

/// The number just below x (toward −∞), for x finite and positive.
fn next_down(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

/// log10's arguments: the special values, the subnormals, both ends of the range, around 1 and
/// around each power of ten from 10⁻³⁰⁰ to 10³⁰⁰, and a spread over the whole range and over the
/// powers an S-parameter's |S|² takes (10⁻³⁰ to 10).
fn log10_arguments() -> Vec<f64> {
    let mut out = vec![
        0.0,
        -0.0,
        -1.0,
        -1e-300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1.0,
        2.0,
        0.5,
        std::f64::consts::SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 3.0,
        f64::from_bits(1),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        1e-30,
        1e-300,
    ];
    for k in 1..=24u64 {
        out.push(f64::from_bits(1f64.to_bits() + k));
        out.push(f64::from_bits(1f64.to_bits() - k));
    }
    for n in -300..=300 {
        if n % 10 != 0 && !(-32..=32).contains(&n) {
            continue;
        }
        let p = format!("1e{n}").parse::<f64>().unwrap();
        out.extend([next_down(p), p, next_up(p)]);
    }
    for i in 0..400u32 {
        // a mantissa in [1, 2) and an exponent spread over the normal range, both scrambled
        let mantissa = 1.0 + f64::from((i * 7919) % 1000) / 1000.0;
        let exponent = -1020 + ((i * 104_729) % 2040) as i32;
        out.push(mantissa * portable::powi(2.0, exponent));
    }
    for i in 0..400u32 {
        let mantissa = 1.0 + f64::from((i * 6007) % 997) / 997.0;
        let exponent = -100 + (i % 104) as i32;
        out.push(mantissa * portable::powi(2.0, exponent));
    }
    out
}

/// atan2's arguments (y, x): every pair of the special values, x = 1 (atan alone), each quadrant
/// and both axes on a grid, ratios beyond 2⁶⁴ and below 2⁻⁶⁴, and points on the unit circle.
fn atan2_arguments() -> Vec<(f64, f64)> {
    let special = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        1e-300,
        -1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let mut out = Vec::new();
    for &y in &special {
        for &x in &special {
            out.push((y, x));
        }
    }
    for k in 0..=40 {
        let y = -4.0 + 0.2 * f64::from(k);
        out.push((y, 1.0));
    }
    for i in 0..=20 {
        for j in 0..=20 {
            let y = -3.7 + 0.37 * f64::from(i);
            let x = -3.7 + 0.37 * f64::from(j);
            out.push((y, x));
        }
    }
    for e in [60, 63, 64, 65, 66, 70, 200] {
        let big = portable::powi(2.0, e);
        out.extend([
            (big, 1.0),
            (1.0, big),
            (-1.0, -big),
            (1.0, -big),
            (-big, -3.0),
            (3.0, -big * 1.5),
        ]);
    }
    for k in 0..360 {
        let (s, c) = portable::sin_cos(f64::from(k) * std::f64::consts::PI / 180.0 + 1e-3);
        out.push((s, c));
    }
    out
}

/// How many arguments of each function the hashed grid has.
const GRID: u64 = 200_000;

/// splitmix64 (S. Vigna), from a seed: the grid's bits, the same sequence in the TypeScript test.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// A number from random bits, its biased exponent `exponent` and its sign and mantissa the bits'.
fn with_exponent(r: u64, exponent: u64) -> f64 {
    f64::from_bits((r & (1 << 63)) | (exponent << 52) | (r & ((1 << 52) - 1)))
}

/// FNV-1a over the results' bits, a NaN's as one pattern.
fn fnv(hash: u64, x: f64) -> u64 {
    let bits = if x.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        x.to_bits()
    };
    bits.to_le_bytes().iter().fold(hash, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;

/// The hash of log10 over GRID positive numbers: even ones anywhere in the normal range (one in
/// 256 subnormal), odd ones from 2⁻¹¹⁰ to 2¹⁰, where an S-parameter's |S|² lies.
fn log10_hash() -> String {
    let mut random = SplitMix(822);
    let mut hash = FNV_START;
    for k in 0..GRID {
        let r = random.next() & !(1 << 63);
        let exponent = if k % 2 == 1 {
            1023 - 110 + (r >> 52) % 121
        } else if r & 0xff == 0 {
            0
        } else {
            1 + (r >> 52) % 2046
        };
        hash = fnv(hash, portable::log10(with_exponent(r, exponent)));
    }
    format!("{hash:016x}")
}

/// The hash of atan2 over GRID points (y, x), each part of either sign and between 2⁻⁴⁰ and 2⁴⁰.
fn atan2_hash() -> String {
    let mut random = SplitMix(280);
    let mut hash = FNV_START;
    for _ in 0..GRID {
        let (a, b) = (random.next(), random.next());
        let y = with_exponent(a, 1023 - 40 + (a >> 52) % 81);
        let x = with_exponent(b, 1023 - 40 + (b >> 52) % 81);
        hash = fnv(hash, portable::atan2(y, x));
    }
    format!("{hash:016x}")
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn number(s: &serde_json::Value) -> f64 {
    f64::from_bits(u64::from_str_radix(s.as_str().unwrap(), 16).unwrap())
}

/// The same bits, or both NaN (a NaN's sign and payload are not part of the answer).
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[test]
fn the_front_ends_maths_file_holds_the_bits_of_photonoxides_portable_maths() {
    if std::env::var_os("PHOTONOXIDE_WRITE_PORTABLE_MATHS").is_some() {
        let log10: Vec<_> = log10_arguments()
            .into_iter()
            .map(|x| [hex(x), hex(portable::log10(x))])
            .collect();
        let atan2: Vec<_> = atan2_arguments()
            .into_iter()
            .map(|(y, x)| [hex(y), hex(x), hex(portable::atan2(y, x))])
            .collect();
        let text =
            serde_json::to_string(&serde_json::json!({ "log10": log10, "atan2": atan2, "grid": GRID, "log10_hash": log10_hash(), "atan2_hash": atan2_hash() })).unwrap();
        std::fs::write(file(), text + "\n").unwrap();
    }
    let text = std::fs::read_to_string(file()).expect("studio/tests/portable-maths.json");
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let log10 = json["log10"].as_array().unwrap();
    let atan2 = json["atan2"].as_array().unwrap();
    assert_eq!(
        log10.len(),
        log10_arguments().len(),
        "write the file again (PHOTONOXIDE_WRITE_PORTABLE_MATHS=1)"
    );
    assert_eq!(
        atan2.len(),
        atan2_arguments().len(),
        "write the file again (PHOTONOXIDE_WRITE_PORTABLE_MATHS=1)"
    );
    assert_eq!(
        json["grid"].as_u64(),
        Some(GRID),
        "write the file again (PHOTONOXIDE_WRITE_PORTABLE_MATHS=1)"
    );
    assert_eq!(
        json["log10_hash"].as_str(),
        Some(log10_hash().as_str()),
        "log10's bits over the grid"
    );
    assert_eq!(
        json["atan2_hash"].as_str(),
        Some(atan2_hash().as_str()),
        "atan2's bits over the grid"
    );
    for row in log10 {
        let (x, want) = (number(&row[0]), number(&row[1]));
        assert!(
            same(portable::log10(x), want),
            "log10({x:e}): {:e} here, {want:e} in the file",
            portable::log10(x)
        );
    }
    for row in atan2 {
        let (y, x, want) = (number(&row[0]), number(&row[1]), number(&row[2]));
        let got = portable::atan2(y, x);
        assert!(
            same(got, want),
            "atan2({y:e}, {x:e}): {got:e} here, {want:e} in the file"
        );
    }
}
