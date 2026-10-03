use super::*;

fn parse(text: &str) -> Touchstone {
    Touchstone::parse(text, None).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// The error's text, which must name `line`.
fn fails_at(text: &str, ports: Option<usize>, line: usize) -> String {
    let e = Touchstone::parse(text, ports).expect_err(text).to_string();
    assert!(
        e.contains(&format!("line {line}:")),
        "expected line {line}: {e}"
    );
    e
}

fn polar(m: f64, deg: f64) -> c64 {
    c64::from_polar(m, deg.to_radians())
}

fn close(a: c64, b: c64, tol: f64) -> bool {
    (a - b).norm() <= tol * b.norm().max(1.0)
}

// Rev. 1.1, Section 3, Example 1
#[test]
fn a_one_port_file_with_a_single_frequency() {
    let t = parse(
        "!1-port S-parameter file, single frequency point\n# MHz S MA R 50\n!freq magS11 angS11\n2.000 0.894 -12.136\n",
    );
    assert_eq!(t.version, Version::One);
    assert_eq!(
        (t.ports(), t.unit, t.parameter, t.format),
        (1, FrequencyUnit::MHz, Parameter::S, Format::MagnitudeAngle)
    );
    assert_eq!(t.reference, vec![50.0]);
    assert_eq!(t.frequencies_hz, vec![2e6]);
    assert!(close(t.matrices[0][0][0], polar(0.894, -12.136), 1e-15));
}

// Rev. 1.1, Example 2: Z-parameters normalized to 75 ohms
#[test]
fn a_one_port_z_file_over_five_frequencies() {
    let t = parse(
        "# MHz Z MA R 75\n100 0.99 -4\n200 0.80 -22\n300 0.707 -45\n400 0.40 -62\n500 0.01 -89\n",
    );
    assert_eq!(t.parameter, Parameter::Z);
    assert_eq!(t.reference, vec![75.0]);
    assert_eq!(t.frequencies_hz, vec![1e8, 2e8, 3e8, 4e8, 5e8]);
    assert!(close(t.matrices[4][0][0], polar(0.01, -89.0), 1e-15));
    // only S-parameters become S-matrices
    let e = t.spectrum(Convention::Physics).unwrap_err().to_string();
    assert!(e.contains("S-parameters"), "{e}");
}

// Rev. 1.1, Example 3, and Version 2.0 Example 12 (whose "[Version 2.0]" is a misprint)
#[test]
fn a_two_port_h_file() {
    let t = parse(
        "!2-port H-parameter file, single frequency point\n# KHz H MA R 1\n! freq magH11 angH11 magH21 angH21 magH12 angH12 magH22 angH22\n2 .95 -26 3.57 157 .04 76 .66 -14\n",
    );
    assert_eq!((t.parameter, t.unit), (Parameter::H, FrequencyUnit::KHz));
    // N11 N21 N12 N22
    assert!(close(t.matrices[0][1][0], polar(3.57, 157.0), 1e-15));
    assert!(close(t.matrices[0][0][1], polar(0.04, 76.0), 1e-15));
    // H-parameters are for 2 ports only
    fails_at("# KHz H MA R 1\n2 .95 -26\n", None, 1);
}

// Rev. 1.1, Example 4 (Version 2.0, Example 13)
const EXAMPLE_4: &str = "!2-port S-parameter file, three frequency points
# GHZ S RI R 50.0
!freq RelS11 ImS11 ReS21 ImS21 ReS12 ImS12 ReS22 ImS22
1.0000 0.3926 -0.1211 -0.0003 -0.0021 -0.0003 -0.0021 0.3926 -0.1211
2.0000 0.3517 -0.3054 -0.0096 -0.0298 -0.0096 -0.0298 0.3517 -0.3054
10.000 0.3419 0.3336 -0.0134 0.0379 -0.0134 0.0379 0.3419 0.3336
";

#[test]
fn a_two_port_s_file_in_real_and_imaginary_parts() {
    let t = parse(EXAMPLE_4);
    assert_eq!((t.ports(), t.format), (2, Format::RealImaginary));
    assert_eq!(t.frequencies_hz, vec![1e9, 2e9, 10e9]);
    assert_eq!(t.matrices[1][1][0], c64::new(-0.0096, -0.0298));
    assert_eq!(t.matrices[2][1][1], c64::new(0.3419, 0.3336));
    assert_eq!(t.noise_frequencies, 0);
}

// Rev. 1.1, Example 5 (Version 2.0, Example 14): a 4-port, a row per line
const EXAMPLE_5: &str = "! 4-port S-parameter data, taken at three frequency points
# GHZ S MA R 50
5.00000 0.60 161.24 0.40 -42.20 0.42 -66.58 0.53 -79.34 !row 1

              0.40 -42.20 0.60 161.20 0.53 -79.34 0.42 -66.58 !row 2
              0.42 -66.58 0.53 -79.34 0.60 161.24 0.40 -42.20 !row 3
              0.53 -79.34 0.42 -66.58 0.40 -42.20 0.60 161.24 !row 4

6.00000 0.57 150.37 0.40 -44.34 0.41 -81.24 0.57 -95.77 !row 1
              0.40 -44.34 0.57 150.37 0.57 -95.77 0.41 -81.24 !row 2
              0.41 -81.24 0.57 -95.77 0.57 150.37 0.40 -44.34 !row 3
              0.57 -95.77 0.41 -81.24 0.40 -44.34 0.57 150.37 !row 4

7.00000 0.50 136.69 0.45 -46.41 0.37 -99.09 0.62 -114.19 !row 1
0.45 -46.41 0.50 136.69 0.62 -114.19 0.37 -99.09 !row 2
0.37 -99.09 0.62 -114.19 0.50 136.69 0.45 -46.41 !row 3
0.62 -114.19 0.37 -99.09 0.45 -46.41 0.50 136.69 !row 4
";

#[test]
fn a_four_port_file_a_row_per_line() {
    // the number of ports from the layout, and from an extension
    for ports in [None, Some(4)] {
        let t = Touchstone::parse(EXAMPLE_5, ports).unwrap();
        assert_eq!(t.ports(), 4);
        assert_eq!(t.frequencies_hz, vec![5e9, 6e9, 7e9]);
        // row 1 is N11 N12 N13 N14
        assert!(close(t.matrices[0][0][1], polar(0.40, -42.20), 1e-15));
        assert!(close(t.matrices[0][1][1], polar(0.60, 161.20), 1e-15));
        assert!(close(t.matrices[2][3][2], polar(0.45, -46.41), 1e-15));
    }
    // a 4-port read as a 2-port: the second line isn't a frequency's line
    fails_at(EXAMPLE_5, Some(2), 5);
}

/// A 6-port file, Version 1, rows wrapped at four pairs, in RI: N_ij = i + j/10 + i(j/100).
fn six_port_text(break_a_row: bool) -> String {
    let mut s = String::from("# Hz S RI\n");
    for (k, f) in [1.0, 2.0].iter().enumerate() {
        for i in 1..=6 {
            let pairs: Vec<String> = (1..=6)
                .map(|j| {
                    format!(
                        "{} {}",
                        i as f64 + j as f64 / 10.0 + k as f64,
                        j as f64 / 100.0
                    )
                })
                .collect();
            if i == 1 {
                s.push_str(&format!("{f} "));
            }
            if break_a_row && i == 3 {
                // five pairs on one line
                s.push_str(&format!("{}\n{}\n", pairs[..5].join(" "), pairs[5]));
            } else {
                s.push_str(&format!(
                    "{} !row {i}\n{}\n",
                    pairs[..4].join(" "),
                    pairs[4..].join(" ")
                ));
            }
        }
    }
    s
}

// Rev. 1.1, "5-port and above networks"
#[test]
fn rows_of_more_than_four_pairs_wrap() {
    let t = parse(&six_port_text(false));
    assert_eq!(t.ports(), 6);
    assert_eq!(t.reference, vec![50.0; 6]);
    for (k, m) in t.matrices.iter().enumerate() {
        for (i, row) in m.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                let want = c64::new(
                    (i + 1) as f64 + (j + 1) as f64 / 10.0 + k as f64,
                    (j + 1) as f64 / 100.0,
                );
                assert_eq!(value, want, "{k} {i} {j}");
            }
        }
    }
    // five pairs on a line: rows 1 and 2 take 4 lines after the option line, so row 3 is line 6
    let e = fails_at(&six_port_text(true), Some(6), 6);
    assert!(e.contains("four pairs"), "{e}");
}

#[test]
fn a_row_must_start_on_a_new_line() {
    // a 3-port whose second row starts on the first row's line
    let text = "# GHz S RI\n1 1 0 2 0 3 0 4 0\n5 0 6 0\n7 0 8 0 9 0\n";
    let e = fails_at(text, Some(3), 2);
    assert!(e.contains("new line"), "{e}");
    // a row split over two lines with fewer than four pairs on the first
    let text = "# GHz S RI\n1 1 0 2 0\n3 0\n4 0 5 0 6 0\n7 0 8 0 9 0\n";
    fails_at(text, Some(3), 2);
}

// Rev. 1.1, Example 8 (Version 2.0, Example 18): noise parameters follow where the frequency
// stops increasing
const EXAMPLE_8: &str = "!2-port network, S-parameter and noise data
# GHZ S MA R 50
2 .95 -26 3.57 157 .04 76 .66 -14
22 .60 -144 1.30 40 .14 40 .56 -85
! NOISE PARAMETERS
4 .7 .64 69 .38
18 2.7 .46 -33 .40
";

#[test]
fn noise_parameters_are_skipped() {
    let t = parse(EXAMPLE_8);
    assert_eq!(t.frequencies_hz, vec![2e9, 22e9]);
    assert_eq!(t.noise_frequencies, 2);
    assert!(close(t.matrices[1][1][1], polar(0.56, -85.0), 1e-15));
    // a noise line of the wrong length
    let bad = EXAMPLE_8.replace("18 2.7 .46 -33 .40", "18 2.7 .46 -33");
    fails_at(&bad, None, 7);
    // only 2-port files have noise data: elsewhere a falling frequency is an error
    fails_at("# GHz S RI\n2 1 0\n1 1 0\n", None, 3);
}

// Version 2.0, general rule 2: LF, CR+LF or CR alone ends a line
#[test]
fn lines_end_in_lf_crlf_or_cr() {
    for ending in ["\n", "\r\n", "\r"] {
        let text = EXAMPLE_4.replace('\n', ending);
        let t = parse(&text);
        assert_eq!(t.frequencies_hz, vec![1e9, 2e9, 10e9], "{ending:?}");
        // and errors still name the right line
        fails_at(&text.replace("10.000 0.3419", "10.000 x"), None, 6);
    }
}

#[test]
fn the_option_line_has_defaults_and_any_order() {
    // "#" alone: GHz, S, MA, 50 ohms
    let t = parse("#\n1 0.5 90\n");
    assert_eq!(
        (t.unit, t.parameter, t.format),
        (FrequencyUnit::GHz, Parameter::S, Format::MagnitudeAngle)
    );
    assert_eq!(t.reference, vec![50.0]);
    assert!(close(t.matrices[0][0][0], c64::new(0.0, 0.5), 1e-15));
    // any order, any case
    let t = parse("! a comment first\n# ri r 75 khz y\n1 0.5 0.25\n");
    assert_eq!(
        (t.unit, t.parameter, t.format),
        (FrequencyUnit::KHz, Parameter::Y, Format::RealImaginary)
    );
    assert_eq!(t.reference, vec![75.0]);
    // decibels: 20 log10 |x|
    let t = parse("# Hz S DB\n1 -6.020599913279624 -90\n");
    assert!(close(t.matrices[0][0][0], c64::new(0.0, -0.5), 1e-14));
    // additional option lines are ignored
    let t = parse("# Hz S RI\n# GHz S MA\n1 0.5 0.25\n");
    assert_eq!(t.frequencies_hz, vec![1.0]);
    // a unit the specification doesn't have
    let e = fails_at("! optical\n# THz S RI\n193 1 0\n", None, 2);
    assert!(e.contains("THz"), "{e}");
    let e = fails_at("# GHz S RI R\n1 1 0\n", None, 1);
    assert!(e.contains("reference"), "{e}");
}

#[test]
fn errors_name_the_line() {
    // not a number
    let e = fails_at("# GHz S RI\n1 0.5 x\n", None, 2);
    assert!(e.contains("\"x\""), "{e}");
    // a missing value (a 2-port line with 8 values)
    fails_at("# GHz S RI\n1 1 0 0 0 0 0 1\n", Some(2), 2);
    // frequencies must increase
    let e = fails_at(&EXAMPLE_4.replace("10.000", "1.5"), None, 6);
    assert!(e.contains("increase"), "{e}");
    // data missing at the end
    let e = Touchstone::parse("# GHz S RI\n1 1 0 2 0 3 0\n4 0 5 0 6 0\n", Some(3))
        .unwrap_err()
        .to_string();
    assert!(e.contains("at its end") && e.contains("stops"), "{e}");
    // nothing but comments
    assert!(Touchstone::parse("! only a comment\n", None).is_err());
    // the option line must come first
    fails_at("\n1 1 0\n# GHz S RI\n", None, 2);
    // keywords need [Version] 2.0
    fails_at("# GHz S RI\n[Number of Ports] 1\n1 1 0\n", None, 2);
    // a layout no number of ports fits
    fails_at("# GHz S RI\n1 1 0 2 0\n", None, 2);
}

// Version 2.0, Example 5 (with Example 4's [Reference] on a line of its own)
const V2_FULL: &str = "! 4-port S-parameter data
[Version] 2.0
# GHz S MA R 50
[Number of Ports] 4
[Number of Frequencies] 1
[Reference]
50 75 0.01 0.01
[Matrix Format] Full
[Network Data]
5.00000 0.60 161.24 0.40 -42.20 0.42 -66.58 0.53 -79.34 !row 1

              0.40 -42.20 0.60 161.20 0.53 -79.34 0.42 -66.58 !row 2
              0.42 -66.58 0.53 -79.34 0.60 161.24 0.40 -42.20 !row 3
              0.53 -79.34 0.42 -66.58 0.40 -42.20 0.60 161.24 !row 4
";

// Version 2.0, Example 6: the lower triangle, [Reference] split over two lines
const V2_LOWER: &str = "[Version] 2.0
# GHz S MA R 50
[Number of Ports] 4
[Number of Frequencies] 1
[Reference] 50 75
0.01 0.01
[Matrix Format] Lower
[Network Data]
5.00000 0.60 161.24                                  !row 1
0.40 -42.20 0.60 161.20                              !row 2
0.42 -66.58 0.53 -79.34 0.60 161.24                  !row 3
0.53 -79.34 0.42 -66.58 0.40 -42.20 0.60 161.24 !row 4
";

#[test]
fn version_two_full_lower_and_upper_matrices() {
    let full = parse(V2_FULL);
    assert_eq!(full.version, Version::Two);
    assert_eq!(full.reference, vec![50.0, 75.0, 0.01, 0.01]);
    assert_eq!(full.matrix_format, MatrixFormat::Full);
    let lower = parse(V2_LOWER);
    assert_eq!(lower.matrix_format, MatrixFormat::Lower);
    assert_eq!(lower.reference, full.reference);
    // the example's matrix is symmetric, so the triangle gives the full matrix
    for i in 0..4 {
        for j in 0..4 {
            assert!(
                close(lower.matrices[0][i][j], full.matrices[0][i][j], 1e-15),
                "{i} {j}"
            );
        }
    }
    // the upper triangle, all on one line (Version 2.0 has no limit per line)
    let upper = parse(
        "[Version] 2.0\n# GHz S MA R 50\n[Number of Ports] 4\n[Number of Frequencies] 1\n[Matrix Format] Upper\n[Network Data]\n5 0.60 161.24 0.40 -42.20 0.42 -66.58 0.53 -79.34 0.60 161.20 0.53 -79.34 0.42 -66.58 0.60 161.24 0.40 -42.20 0.60 161.24\n[End]\n",
    );
    assert_eq!(upper.reference, vec![50.0; 4]);
    for i in 0..4 {
        for j in 0..4 {
            assert!(
                close(upper.matrices[0][i][j], full.matrices[0][i][j], 1e-15),
                "{i} {j}"
            );
        }
    }
}

// Version 2.0, Examples 17 and 20: two ports, the 21_12 order, noise data, [End]
const V2_NOISE: &str = "!2-port network, S-parameter and noise data
!Default MA format, GHz frequencies, 50 ohm reference, S-parameters
[Version] 2.0
#
[Number of Ports] 2
[Two-Port Data Order] 21_12
[Number of Frequencies] 2
[Number of Noise Frequencies] 2
[Reference] 50 25.0
[Network Data]
! NETWORK PARAMETERS
2 .95 -26 3.57 157 .04 76 .66 -14
22 .60 -144 1.30 40 .14 40 .56 -85
[Noise Data]
! NOISE PARAMETERS
4 .7 .64 69 19
18 2.7 .46 -33 20
[End]
";

#[test]
fn version_two_two_ports_with_noise() {
    let t = parse(V2_NOISE);
    assert_eq!(t.two_port_order, TwoPortOrder::N21N12);
    assert_eq!(t.reference, vec![50.0, 25.0]);
    assert_eq!(t.noise_frequencies, 2);
    assert!(close(t.matrices[0][1][0], polar(3.57, 157.0), 1e-15));
    // 12_21: the same numbers mean N12 first
    let swapped = parse(&V2_NOISE.replace("21_12", "12_21"));
    assert_eq!(swapped.matrices[0][0][1], t.matrices[0][1][0]);
    assert_eq!(swapped.matrices[0][1][0], t.matrices[0][0][1]);
    // Example 19: no [End]
    assert_eq!(parse(&V2_NOISE.replace("[End]\n", "")).noise_frequencies, 2);
    // the noise count must match
    let e = Touchstone::parse(&V2_NOISE.replace("18 2.7 .46 -33 20\n", ""), None)
        .unwrap_err()
        .to_string();
    assert!(e.contains("noise"), "{e}");
    fails_at(
        &V2_NOISE.replace(
            "[Number of Noise Frequencies] 2",
            "[Number of Noise Frequencies] 1",
        ),
        None,
        17,
    );
    // nothing after [End]
    fails_at(&format!("{V2_NOISE}1 2 3\n"), None, 19);
    // [Two-Port Data Order] is required for 2 ports, and refused for others
    let e = Touchstone::parse(&V2_NOISE.replace("[Two-Port Data Order] 21_12\n", ""), None)
        .unwrap_err()
        .to_string();
    assert!(e.contains("Two-Port Data Order"), "{e}");
    let e = fails_at(
        &V2_FULL.replace("[Matrix Format] Full", "[Two-Port Data Order] 12_21"),
        None,
        8,
    );
    assert!(e.contains("2 ports"), "{e}");
}

// Version 2.0, Example 7: a 1-port Z file with [Reference]
#[test]
fn version_two_one_port() {
    let t = parse(
        "!1-port Z-parameter file, multiple frequency points\n[Version] 2.0\n\n# MHz Z MA\n\n[Number of Ports] 1\n\n[Number of Frequencies] 5\n\n[Reference] 20.0\n\n[Network Data]\n\n!freq magZ11 angZ11\n\n100 74.25 -4\n\n200 60          -22\n\n300 53.025 -45\n\n400 30          -62\n\n500 0.75 -89\n",
    );
    assert_eq!((t.parameter, t.ports()), (Parameter::Z, 1));
    assert_eq!(t.reference, vec![20.0]);
    assert!(close(t.matrices[1][0][0], polar(60.0, -22.0), 1e-15));
}

#[test]
fn version_two_errors_name_the_line() {
    // mixed-mode data (Example 16) isn't supported
    let e = fails_at(
        "[Version] 2.0\n# MHz Y RI R 50\n[Number of Ports] 6\n[Number of Frequencies] 1\n[Mixed-Mode Order] D2,3 D6,5 C2,3 C6,5 S4 S1\n[Network Data]\n",
        None,
        5,
    );
    assert!(e.contains("mixed-mode"), "{e}");
    // the version
    fails_at("[Version] 3.0\n#\n", None, 1);
    // [Number of Ports] first after the option line
    fails_at(
        "[Version] 2.0\n# GHz S RI\n[Number of Frequencies] 1\n[Number of Ports] 1\n",
        None,
        3,
    );
    // [Number of Frequencies] is required
    let e = Touchstone::parse(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n[Network Data]\n1 1 0\n",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(e.contains("[Number of Frequencies]"), "{e}");
    // more data than declared
    fails_at(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n[Number of Frequencies] 1\n[Network Data]\n1 1 0\n2 1 0\n",
        None,
        7,
    );
    // less
    let e = Touchstone::parse("[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n[Number of Frequencies] 3\n[Network Data]\n1 1 0\n2 1 0\n[End]\n", None).unwrap_err().to_string();
    assert!(e.contains("stops after 2"), "{e}");
    // a frequency in the middle of a line
    fails_at(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n[Number of Frequencies] 2\n[Network Data]\n1 1 0 2 1 0\n",
        None,
        6,
    );
    // an unknown keyword
    let e = fails_at(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n[Colour] red\n",
        None,
        4,
    );
    assert!(e.contains("Colour"), "{e}");
    // a [Reference] short of a port
    fails_at(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 2\n[Reference] 50\n[Two-Port Data Order] 12_21\n",
        None,
        4,
    );
    // the extension's ports must agree
    fails_at(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n",
        Some(2),
        3,
    );
    // information is skipped
    let t = parse(
        "[Version] 2.0\n# GHz S RI\n[Number of Ports] 1\n[Begin Information]\nanything\n[End Information]\n[Number of Frequencies] 1\n[Network Data]\n1 0.5 0\n",
    );
    assert_eq!(t.matrices[0][0][0], c64::new(0.5, 0.0));
}

/// A deterministic pseudo-random number in [−1, 1).
fn random(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

fn random_data(
    n: usize,
    count: usize,
    symmetric: bool,
    seed: u64,
) -> (Vec<f64>, Vec<Vec<Vec<c64>>>) {
    let mut s = seed;
    let frequencies = (0..count)
        .map(|k| 1.93e14 + 1e11 * k as f64 + 3.7e7 * random(&mut s))
        .collect();
    let matrices = (0..count)
        .map(|_| {
            let mut m: Vec<Vec<c64>> = (0..n)
                .map(|_| {
                    (0..n)
                        .map(|_| c64::new(random(&mut s), random(&mut s)) * 0.7)
                        .collect()
                })
                .collect();
            if symmetric {
                for (i, j) in (0..n).flat_map(|i| (0..i).map(move |j| (i, j))) {
                    m[j][i] = m[i][j];
                }
            }
            m
        })
        .collect();
    (frequencies, matrices)
}

#[test]
fn every_variant_round_trips() {
    for n in 1..=6 {
        for version in [Version::One, Version::Two] {
            let formats: &[MatrixFormat] = match version {
                Version::One => &[MatrixFormat::Full],
                Version::Two => &[MatrixFormat::Full, MatrixFormat::Lower, MatrixFormat::Upper],
            };
            for &matrix_format in formats {
                for format in [
                    Format::RealImaginary,
                    Format::MagnitudeAngle,
                    Format::DecibelAngle,
                ] {
                    for unit in [
                        FrequencyUnit::Hz,
                        FrequencyUnit::KHz,
                        FrequencyUnit::MHz,
                        FrequencyUnit::GHz,
                    ] {
                        let (f, m) =
                            random_data(n, 3, matrix_format != MatrixFormat::Full, n as u64);
                        let mut t = Touchstone::s_parameters(f, m).unwrap();
                        t.version = version;
                        t.format = format;
                        t.unit = unit;
                        t.matrix_format = matrix_format;
                        if version == Version::One {
                            t.two_port_order = TwoPortOrder::N21N12;
                        }
                        if version == Version::Two && n > 1 {
                            t.reference = (0..n).map(|p| 25.0 + p as f64).collect();
                        }
                        let text = t.write(Precision::RoundTrip).unwrap();
                        let back = Touchstone::parse(&text, Some(n))
                            .unwrap_or_else(|e| panic!("{e}\n{text}"));
                        let what = format!(
                            "{n} ports, {version:?}, {matrix_format:?}, {format:?}, {unit:?}\n{text}"
                        );
                        assert_eq!(back.version, version, "{what}");
                        assert_eq!(
                            (back.format, back.unit, back.matrix_format),
                            (format, unit, matrix_format),
                            "{what}"
                        );
                        assert_eq!(back.reference, t.reference, "{what}");
                        for (a, b) in back.frequencies_hz.iter().zip(&t.frequencies_hz) {
                            // the shortest round-trip decimal of f/unit, times the unit
                            assert!((a - b).abs() <= 2.0 * f64::EPSILON * b, "{what}");
                        }
                        // RI is exact; MA and DB go through |x|, arg x and back
                        let tol = if format == Format::RealImaginary {
                            0.0
                        } else {
                            4.0 * f64::EPSILON
                        };
                        for (a, b) in back
                            .matrices
                            .iter()
                            .flatten()
                            .flatten()
                            .zip(t.matrices.iter().flatten().flatten())
                        {
                            assert!((a - b).norm() <= tol, "{a} {b}: {what}");
                        }
                        // and reading the version 1 text without the number of ports
                        if version == Version::One {
                            assert_eq!(parse(&text).ports(), n, "{what}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn printed_precision_bounds_the_round_trip_error() {
    let (f, m) = random_data(3, 4, false, 7);
    let t = Touchstone::s_parameters(f, m).unwrap();
    for digits in [3, 6, 10, 17] {
        let back =
            Touchstone::parse(&t.write(Precision::Significant(digits)).unwrap(), None).unwrap();
        // half a unit in the last printed digit, relative to each part
        let rel = 0.5 * 10f64.powi(1 - digits as i32);
        for (a, b) in back
            .matrices
            .iter()
            .flatten()
            .flatten()
            .zip(t.matrices.iter().flatten().flatten())
        {
            assert!(
                (a.re - b.re).abs() <= rel * b.re.abs() * (1.0 + 1e-12),
                "{digits}"
            );
            assert!(
                (a.im - b.im).abs() <= rel * b.im.abs() * (1.0 + 1e-12),
                "{digits}"
            );
        }
    }
    assert!(t.write(Precision::Significant(0)).is_err());
    assert!(t.write(Precision::Significant(18)).is_err());
}

#[test]
fn writing_checks_the_data() {
    let (f, m) = random_data(2, 2, false, 3);
    let mut t = Touchstone::s_parameters(f.clone(), m.clone()).unwrap();
    // Version 1 can't hold several reference resistances or the 12_21 order
    t.version = Version::One;
    assert!(t.write(Precision::RoundTrip).is_err());
    t.two_port_order = TwoPortOrder::N21N12;
    assert!(t.write(Precision::RoundTrip).is_ok());
    t.reference = vec![50.0, 75.0];
    assert!(t.write(Precision::RoundTrip).is_err());
    // a zero in decibels
    let mut z = m.clone();
    z[0][0][0] = c64::new(0.0, 0.0);
    let mut t = Touchstone::s_parameters(f.clone(), z).unwrap();
    t.format = Format::DecibelAngle;
    assert!(
        t.write(Precision::RoundTrip)
            .unwrap_err()
            .to_string()
            .contains("decibels")
    );
    // inconsistent data
    assert!(Touchstone::s_parameters(vec![2.0, 1.0], m.clone()).is_err());
    assert!(Touchstone::s_parameters(vec![1.0], m.clone()).is_err());
    assert!(
        Touchstone::s_parameters(f.clone(), vec![vec![vec![c64::new(1.0, 0.0)]]; 2])
            .map(|t| t.ports())
            .unwrap()
            == 1
    );
    assert!(Touchstone::s_parameters(f, vec![vec![vec![c64::new(f64::NAN, 0.0)]]; 2]).is_err());
}

#[test]
fn optical_spectra_convert_through_frequency() {
    use crate::circuit::{SMatrix, Spectrum};
    // an S-matrix spectrum at increasing wavelengths, in photonoxide's convention
    let wavelengths: Vec<Wavelength> = [1.50, 1.55, 1.60]
        .iter()
        .map(|&w| Wavelength::um(w).unwrap())
        .collect();
    let matrices: Vec<SMatrix> = (0..3)
        .map(|k| {
            SMatrix::from_rows(vec![
                vec![c64::new(0.1, 0.01 * k as f64), c64::new(0.0, 0.9)],
                vec![c64::new(0.0, 0.9), c64::new(0.2, -0.1)],
            ])
            .unwrap()
        })
        .collect();
    let names = vec!["in".to_owned(), "out".to_owned()];
    let spectrum = Spectrum::new(names, wavelengths.clone(), matrices.clone()).unwrap();
    let t = Touchstone::from_spectrum(&spectrum, Convention::Physics).unwrap();
    // increasing frequency: the longest wavelength first, at f = c/λ
    assert!((t.frequencies_hz[0] - SPEED_OF_LIGHT / 1.60e-6).abs() <= 1e-15 * t.frequencies_hz[0]);
    let back = t.spectrum(Convention::Physics).unwrap();
    assert_eq!(back.ports(), ["o1", "o2"]);
    for (a, b) in back.wavelengths().iter().zip(wavelengths.iter().rev()) {
        assert!((a.to_um() - b.to_um()).abs() <= 2.0 * f64::EPSILON * b.to_um());
    }
    assert_eq!(back.matrices()[0], matrices[2]);
    // in the engineering convention the file holds the conjugates
    let e = Touchstone::from_spectrum(&spectrum, Convention::Engineering).unwrap();
    assert_eq!(e.matrices[0][0][1], c64::new(0.0, -0.9));
    assert_eq!(
        e.spectrum(Convention::Engineering).unwrap().matrices()[2],
        matrices[0]
    );
    // through a file and back
    let text = e.write(Precision::RoundTrip).unwrap();
    let read = Touchstone::parse(&text, None).unwrap();
    assert_eq!(
        read.spectrum(Convention::Engineering).unwrap().matrices()[1],
        matrices[1]
    );
    // a 0 Hz point has no wavelength
    let dc =
        Touchstone::s_parameters(vec![0.0, 1.0], vec![vec![vec![c64::new(1.0, 0.0)]]; 2]).unwrap();
    assert!(dc.wavelengths().is_err());
    assert!(dc.spectrum(Convention::Physics).is_err());
}

#[test]
fn files_take_their_ports_from_the_extension() {
    assert_eq!(ports_of(Path::new("a/device.s4p")), Some(4));
    assert_eq!(ports_of(Path::new("DEVICE.S12P")), Some(12));
    assert_eq!(ports_of(Path::new("device.ts")), None);
    assert_eq!(ports_of(Path::new("device.s0p")), None);
    let dir = std::env::temp_dir().join(format!("photonoxide-touchstone-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // a 2-port line has 9 values, as a 4-port's first line has: the extension decides
    let path = dir.join("device.s4p");
    std::fs::write(&path, EXAMPLE_5).unwrap();
    assert_eq!(Touchstone::read(&path).unwrap().ports(), 4);
    let path = dir.join("device.s2p");
    std::fs::write(&path, EXAMPLE_5).unwrap();
    let e = Touchstone::read(&path).unwrap_err().to_string();
    assert!(e.contains("device.s2p, line 5"), "{e}");
    // written and read back
    let (f, m) = random_data(3, 2, false, 11);
    let t = Touchstone::s_parameters(f, m).unwrap();
    let path = dir.join("out.ts");
    t.write_file(&path, Precision::RoundTrip).unwrap();
    assert_eq!(Touchstone::read(&path).unwrap().matrices, t.matrices);
    assert!(matches!(
        Touchstone::read(&dir.join("missing.s1p")),
        Err(Error::Io { .. })
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}
