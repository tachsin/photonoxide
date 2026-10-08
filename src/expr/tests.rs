use std::f64::consts::{FRAC_PI_2, PI};

use super::special::erf;
use super::*;

fn value(text: &str) -> f64 {
    let e = Expr::parse(text, &Scope::new()).unwrap_or_else(|e| panic!("{text}: {e}"));
    e.try_eval(&[0.0; 7])
        .unwrap_or_else(|e| panic!("{text}: {e}"))
}

fn error(text: &str) -> ExprError {
    match Expr::parse(text, &Scope::new()) {
        Ok(e) => e
            .try_eval(&[0.0; 7])
            .err()
            .unwrap_or_else(|| panic!("{text} should fail")),
        Err(e) => e,
    }
}

#[test]
fn numbers_units_and_operators() {
    assert_eq!(value("500 nm"), 0.5);
    assert_eq!(value("500nm"), 0.5);
    assert_eq!(value("2.5 um"), 2.5);
    assert_eq!(value("2.5 µm"), 2.5);
    assert_eq!(value("3 mm"), 3000.0);
    assert_eq!(value("90 deg"), FRAC_PI_2);
    assert_eq!(value("1.5 rad"), 1.5);
    assert_eq!(value("1e3 nm"), 1.0);
    assert_eq!(value("2.5e-1"), 0.25);
    assert_eq!(value(".5"), 0.5);
    // nanometres convert by division, as Length::nm does: the same bits
    for v in [220.0, 450.0, 123.456, 0.1] {
        assert_eq!(
            value(&format!("{v} nm")),
            crate::units::Length::nm(v).to_um()
        );
    }
    // a unit alone is the unit: w in nanometres
    assert_eq!(value("1 um / nm"), 1000.0);
    assert_eq!(value("2*3 + 4"), 10.0);
    assert_eq!(value("2*(3 + 4)"), 14.0);
    assert_eq!(value("-2^2"), -4.0);
    assert_eq!(value("2^3^2"), 512.0);
    assert_eq!(value("2**3"), 8.0);
    assert_eq!(value("2^-1"), 0.5);
    assert_eq!(value("7 - 2 - 1"), 4.0);
    assert_eq!(value("8 / 4 / 2"), 1.0);
    assert_eq!(value("3 × 2 − 1"), 5.0);
    assert_eq!(value("1 < 2"), 1.0);
    assert_eq!(value("2 <= 1"), 0.0);
    assert_eq!(value("1 um == 1000 nm"), 1.0);
    assert_eq!(value("1 != 1"), 0.0);
    assert_eq!(value("if(1 > 2, 3 um, 4 um)"), 4.0);
    assert_eq!(value("piecewise(0, 1, 1, 2, 3)"), 2.0);
    assert_eq!(value("piecewise(0, 1, 0, 2, 3)"), 3.0);
    assert_eq!(value("pi"), PI);
    // a bare zero fits any dimension
    assert_eq!(value("0 + 1 um"), 1.0);
    assert_eq!(value("max(0, 2 nm)"), 0.002);
}

#[test]
fn every_function_is_its_value() {
    let cases: [(&str, f64); 22] = [
        ("sin(0.3)", 0.3f64.sin()),
        ("cos(30 deg)", 30f64.to_radians().cos()),
        ("tan(0.4)", 0.4f64.tan()),
        ("asin(0.3)", 0.3f64.asin()),
        ("acos(0.3)", 0.3f64.acos()),
        ("atan(2)", 2f64.atan()),
        ("atan2(1 um, -2 um)", 1f64.atan2(-2.0)),
        ("exp(1.5)", 1.5f64.exp()),
        ("ln(1.5)", 1.5f64.ln()),
        ("log10(1500)", 1500f64.log10()),
        ("sqrt(2)", 2f64.sqrt()),
        ("abs(-3 nm)", 0.003),
        ("min(1 um, 3 nm)", 0.003),
        ("max(1 um, 3 nm)", 1.0),
        ("clamp(5, 1, 3)", 3.0),
        ("floor(-1.5)", -2.0),
        ("ceil(-1.5)", -1.0),
        ("round(2.5)", 3.0),
        ("erf(0.5)", 0.520_499_877_813_046_5),
        ("tanh(0.7)", 0.7f64.tanh()),
        ("sinc(0.5)", 0.5f64.sin() / 0.5),
        ("smoothstep(0, 2, 0.5)", 0.156_25),
    ];
    for (text, expected) in cases {
        let v = value(text);
        assert!(
            (v - expected).abs() <= 1e-15 * expected.abs().max(1.0),
            "{text}: {v} against {expected}"
        );
    }
    assert_eq!(value("sinc(0)"), 1.0);
    assert_eq!(value("smoothstep(0, 1, -1)"), 0.0);
    assert_eq!(value("smoothstep(0, 1, 2)"), 1.0);
    assert_eq!(value("sqrt(4 um^2)"), 2.0);
    assert_eq!(value("2 um^2 / (1 um)^2"), 2.0);
    assert_eq!(value("(2 um)^2 / 1 um^2"), 4.0);
    assert_eq!(value("(3 um)^2 / (1 um)^2"), 9.0);
    // erf against tabulated values (DLMF 7.6, Abramowitz and Stegun Table 7.1)
    for (x, e) in [
        (0.1, 0.112_462_916_018_284_9),
        (1.0, 0.842_700_792_949_714_9),
        (2.0, 0.995_322_265_018_952_7),
        (3.0, 0.999_977_909_503_001_4),
        (3.5, 0.999_999_256_901_627_7),
        (5.0, 0.999_999_999_998_462_5),
    ] {
        assert!(
            (erf(x) - e).abs() < 5e-16,
            "erf({x}) = {} against {e}",
            erf(x)
        );
        assert!((erf(-x) + e).abs() < 5e-16);
    }
    assert_eq!(erf(0.0), 0.0);
    assert_eq!(erf(10.0), 1.0);
}

#[test]
fn errors_point_at_the_column() {
    let e = error("1 um + 30 deg");
    assert_eq!(e.column, 6, "{e}");
    assert!(e.message.contains("can't add an angle to a length"), "{e}");
    let e = error("2 * wdth");
    assert_eq!((e.column, e.message.as_str()), (5, "unknown name \"wdth\""));
    let e = error("sin(1 um)");
    assert_eq!(e.column, 5, "{e}");
    assert!(e.message.contains("an angle or a number"), "{e}");
    let e = error("exp(2 nm)");
    assert!(
        e.message.contains("exp takes a number, not a length"),
        "{e}"
    );
    let e = error("sqrt(1 um)");
    assert!(e.message.contains("sqrt of a length"), "{e}");
    let e = error("1 / 0");
    assert_eq!((e.column, e.message.as_str()), (3, "division by zero"));
    let e = error("1 / (1 - 1)");
    assert_eq!((e.column, e.message.as_str()), (3, "division by zero"));
    let e = error("sqrt(1 - 2)");
    assert!(e.message.contains("sqrt of a negative"), "{e}");
    assert_eq!(e.column, 1);
    let e = error("ln(0)");
    assert!(e.message.contains("logarithm"), "{e}");
    let e = error("2 +");
    assert_eq!(e.column, 4, "{e}");
    let e = error("(1 + 2");
    assert!(e.message.contains("expected \")\""), "{e}");
    let e = error("1 2");
    assert_eq!(e.column, 3, "{e}");
    let e = error("min(1)");
    assert!(e.message.contains("min takes 2 arguments, got 1"), "{e}");
    let e = error("1 # 2");
    assert_eq!((e.column, e.message.as_str()), (3, "unexpected \"#\""));
    let e = error("(1 um)^t");
    assert!(e.message.contains("constant power"), "{e}");
    let e = error("(1 um)^0.5");
    assert!(e.message.contains("whole power"), "{e}");
    let e = error("if(1, 1 um, 1 deg)");
    assert!(e.message.contains("of a kind"), "{e}");
    let e = error("1 um < 1");
    assert!(e.message.contains("compare a length with a number"), "{e}");
    let e = error("sin");
    assert!(e.message.contains("sin is a function"), "{e}");
    let e = error("");
    assert!(e.message.contains("empty"), "{e}");
    let deep = format!("{}1{}", "(".repeat(70), "+1)".repeat(70));
    assert!(
        Expr::parse(&deep, &Scope::new()).is_ok(),
        "parentheses alone don't deepen"
    );
    let deep = (0..70).fold("x".to_owned(), |acc, _| format!("1 um + ({acc}")) + &")".repeat(70);
    let e = Expr::parse(&deep, &Scope::new()).unwrap_err();
    assert!(e.message.contains("nested too deeply"), "{e}");
    assert_eq!(
        error("2 * wdth").to_string(),
        "column 5: unknown name \"wdth\""
    );
}

#[test]
fn coordinates_and_parameters() {
    let p = Parameters::new(&[
        ("gap".into(), "w/2.5".into()),
        ("w".into(), "500 nm".into()),
        ("radius".into(), "5 um".into()),
        ("y0".into(), "radius + w + gap".into()),
        ("n".into(), "3".into()),
        ("tilt".into(), "8 deg".into()),
    ])
    .unwrap();
    assert_eq!(p.value("w"), Some(0.5));
    assert_eq!(p.value("gap"), Some(0.2));
    assert_eq!(p.value("y0"), Some(5.0 + 0.5 + 0.2));
    assert_eq!(p.dim("y0"), Some(Some(Dim::LENGTH)));
    assert_eq!(p.dim("n"), Some(Some(Dim::NONE)));
    assert_eq!(p.dim("tilt"), Some(Some(Dim::ANGLE)));
    assert_eq!(p.names(), ["gap", "w", "radius", "y0", "n", "tilt"]);
    // an expression of the coordinates and the parameters
    let e = Expr::parse("y0 + x*sin(tilt) + i*w", p.scope()).unwrap();
    let mut v = p.values().to_vec();
    v[0] = 2.0;
    v[5] = 3.0;
    let expected = 5.7 + 2.0 * 8f64.to_radians().sin() + 1.5;
    assert!((e.eval(&v) - expected).abs() < 1e-15);
    assert_eq!(e.dim(), Some(Dim::LENGTH));
    // a sweep's point: w set, gap and y0 follow it
    let q = p.with("w", 0.6).unwrap();
    assert_eq!(q.value("gap"), Some(0.6 / 2.5));
    assert!((q.value("y0").unwrap() - (5.0 + 0.6 + 0.24)).abs() < 1e-15);
    // gap set: w stays, y0 follows
    let q = p.with("gap", 0.3).unwrap();
    assert_eq!(q.value("w"), Some(0.5));
    assert_eq!(q.value("y0"), Some(5.0 + 0.5 + 0.3));
    assert!(p.with("nothing", 1.0).is_err());
    assert!(p.with("w", f64::NAN).is_err());
}

#[test]
fn bad_parameter_tables_are_refused() {
    let table = |defs: &[(&str, &str)]| {
        Parameters::new(
            &defs
                .iter()
                .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
                .collect::<Vec<_>>(),
        )
    };
    let e = table(&[("a", "b + 1"), ("b", "c * 2"), ("c", "a"), ("d", "1")]).unwrap_err();
    assert!(e.message.contains("cycle: a -> b -> c -> a"), "{e}");
    let e = table(&[("a", "a + 1")]).unwrap_err();
    assert!(e.message.contains("uses itself"), "{e}");
    let e = table(&[("sin", "1")]).unwrap_err();
    assert!(e.message.contains("the expression language's own"), "{e}");
    let e = table(&[("x", "1")]).unwrap_err();
    assert!(e.message.contains("the expression language's own"), "{e}");
    let e = table(&[("2w", "1")]).unwrap_err();
    assert!(e.message.contains("isn't a name"), "{e}");
    let e = table(&[("w", "1 um"), ("v", "w + 1 deg")]).unwrap_err();
    assert_eq!(e.name, "v");
    assert!(
        e.message
            .contains("column 3: can't add an angle to a length"),
        "{e}"
    );
    let e = table(&[("w", "1 um"), ("v", "1 / (w - 1 um)")]).unwrap_err();
    assert_eq!(e.to_string(), "v: column 3: division by zero");
}

/// Every function's derivative against central differences.
#[test]
fn derivatives_are_the_finite_differences() {
    let texts = [
        "p^3 - 2*p",
        "sin(p) * cos(2*p) / tan(p)",
        "asin(p/2) + acos(p/3) + atan(p)",
        "atan2(p, 1 - p)",
        "exp(p) + ln(p) + log10(p) + sqrt(p)",
        "abs(p - 2) + min(p, 0.3) + max(p, 0.2)",
        "clamp(3*p, 0.5, 2)",
        "erf(p) + tanh(p) + sinc(3*p)",
        "smoothstep(0.1, 0.9 + p/10, p)",
        "if(p > 0.5, p^2, -p)",
        "piecewise(p < 0.2, 1, p < 0.6, 2*p^2, 3)",
        "p^p",
        "floor(10*p) + round(p) + p",
        "(p*1 um)^2 / (1 um)^2",
    ];
    let scope = {
        let mut s = Scope::new();
        s.define("p", Some(Dim::NONE)).unwrap();
        s
    };
    for text in texts {
        let e = Expr::parse(text, &scope).unwrap_or_else(|e| panic!("{text}: {e}"));
        for p in [0.37, 0.55, 0.81] {
            let mut v = [0.0; 8];
            let at = |q: f64, v: &mut [f64; 8]| {
                v[7] = q;
                e.eval(v)
            };
            let h = 1e-6;
            let fd = (at(p + h, &mut v) - at(p - h, &mut v)) / (2.0 * h);
            let mut duals = [Dual::default(); 8];
            duals[7] = Dual::variable(p);
            let d = e.eval_dual(&duals);
            assert_eq!(d.value, at(p, &mut v), "{text}");
            assert!(
                (d.derivative - fd).abs() < 1e-7 * fd.abs().max(1.0),
                "{text} at {p}: {} against {fd}",
                d.derivative
            );
        }
    }
    // through a parameter table: y0 = radius + w + gap, gap = w/2.5, so dy0/dw = 1.4
    let p = Parameters::new(&[
        ("w".into(), "500 nm".into()),
        ("gap".into(), "w/2.5".into()),
        ("radius".into(), "5 um".into()),
        ("y0".into(), "radius + w + gap".into()),
    ])
    .unwrap();
    let duals = p.derivatives("w").unwrap();
    let y0 = p.slot("y0").unwrap();
    assert!((duals[y0].derivative - 1.4).abs() < 1e-15);
    let e = Expr::parse("y0^2", p.scope()).unwrap();
    let d = e.eval_dual(&duals);
    assert!((d.derivative - 2.0 * 5.7 * 1.4).abs() < 1e-13);
    // with gap set, it no longer follows w
    let q = p.with("gap", 0.25).unwrap();
    assert!((q.derivatives("w").unwrap()[y0].derivative - 1.0).abs() < 1e-15);
}

#[test]
fn many_points_at_once_are_the_points_one_by_one() {
    let mut scope = Scope::new();
    let r = scope.define("r", Some(Dim::LENGTH)).unwrap();
    let e = Expr::parse(
        "if(x*x + y*y < r^2, sqrt(r^2 - x*x - y*y), 0) + r*smoothstep(0, r, abs(x))",
        &scope,
    )
    .unwrap();
    let n = 1000;
    let xs: Vec<f64> = (0..n).map(|k| -1.0 + 2.0 * f64::from(k) / 999.0).collect();
    let ys: Vec<f64> = (0..n).map(|k| (f64::from(k) * 0.37).sin()).collect();
    let mut values = vec![0.0; scope.len()];
    values[r] = 0.8;
    let mut out = vec![0.0; n as usize];
    e.eval_many(&values, &[(0, &xs), (1, &ys)], &mut out);
    for k in 0..n as usize {
        let mut v = values.clone();
        v[0] = xs[k];
        v[1] = ys[k];
        assert_eq!(out[k].to_bits(), e.eval(&v).to_bits(), "{k}");
    }
}

/// How fast: `cargo test --release --lib expression_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, not a check"]
fn expression_timing() {
    use std::time::Instant;
    let mut scope = Scope::new();
    let r = scope.define("r", Some(Dim::LENGTH)).unwrap();
    let w = scope.define("w", Some(Dim::LENGTH)).unwrap();
    // a deformed ring's radius at angle t: 14 instructions and two functions
    let e = Expr::parse("r*(1 + 0.05*cos(4*t)) + w/2*sin(3*t)^2", &scope).unwrap();
    let mut values = vec![0.0; scope.len()];
    values[r] = 5.0;
    values[w] = 0.5;
    let n = 10_000_000;
    let start = Instant::now();
    let mut sum = 0.0;
    for k in 0..n {
        values[3] = f64::from(k) * 1e-6;
        sum += e.eval(&values);
    }
    let one = start.elapsed().as_secs_f64();
    let ts: Vec<f64> = (0..n).map(|k| f64::from(k) * 1e-6).collect();
    let mut out = vec![0.0; ts.len()];
    let start = Instant::now();
    e.eval_many(&values, &[(3, &ts)], &mut out);
    let many = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut dsum = 0.0;
    let mut duals: Vec<Dual> = values.iter().map(|v| Dual::constant(*v)).collect();
    duals[r] = Dual::variable(5.0);
    for k in 0..n {
        duals[3] = Dual::constant(f64::from(k) * 1e-6);
        dsum += e.eval_dual(&duals).derivative;
    }
    let dual = start.elapsed().as_secs_f64();
    println!(
        "{} evaluations: one at a time {:.1} M/s, in blocks {:.1} M/s, with a derivative {:.1} M/s \
         ({sum:.3}, {:.3}, {dsum:.3})",
        n,
        f64::from(n) / one / 1e6,
        f64::from(n) / many / 1e6,
        f64::from(n) / dual / 1e6,
        out.iter().sum::<f64>()
    );
}
