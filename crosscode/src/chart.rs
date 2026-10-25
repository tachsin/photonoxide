//! The cross-code comparison chart: for each problem, each code on the same mesh, its time on one
//! log scale and beside it its time and its errors against photonoxide's converged result (the
//! reference), so the codes' accuracy and speed read across every problem at once.
//!
//! The studies write their points to `crosscode/results/*.tsv` ([`write_points`]); [`redraw`]
//! reads them all and writes `docs/cross-code-chart.svg` and its dark twin,
//! `docs/cross-code-chart-dark.svg`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One code's result on one mesh of one case.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    /// The case, as the chart names it (`the coplanar line`).
    pub case: String,
    /// The quantity (`n`, `Z₀`, `peak rise`).
    pub quantity: String,
    /// The code (`photonoxide`, `openEMS`, `Elmer`).
    pub code: String,
    /// The mesh's smallest cell, µm.
    pub smallest_um: f64,
    /// The value.
    pub value: f64,
    /// Its error against photonoxide's converged result, percent, signed.
    pub error_percent: f64,
    /// The code's own time for it, s.
    pub seconds: f64,
}

const HEADER: &str = "case\tquantity\tcode\tsmallest_um\tvalue\terror_percent\tseconds";

/// The results folder, `crosscode/results/`.
pub fn results() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("results")
}

/// Writes `points` to `crosscode/results/<study>.tsv`, replacing it.
///
/// # Errors
///
/// When the file can't be written.
pub fn write_points(study: &str, points: &[Point]) -> Result<(), String> {
    let mut text = format!("{HEADER}\n");
    for p in points {
        let _ = writeln!(
            text,
            "{}\t{}\t{}\t{}\t{:e}\t{:e}\t{:e}",
            p.case, p.quantity, p.code, p.smallest_um, p.value, p.error_percent, p.seconds
        );
    }
    let folder = results();
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let file = folder.join(format!("{study}.tsv"));
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))
}

/// Every point in `crosscode/results/*.tsv`, file by file in name order.
///
/// # Errors
///
/// When a file can't be read or a line isn't a point.
pub fn read_points() -> Result<Vec<Point>, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(results())
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "tsv"))
        .collect();
    files.sort();
    let mut points = Vec::new();
    for file in files {
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let f: Vec<&str> = line.split('\t').collect();
            let num = |i: usize| -> Result<f64, String> {
                f.get(i)
                    .ok_or_else(|| format!("{}: a short line: {line}", file.display()))?
                    .parse::<f64>()
                    .map_err(|e| format!("{}: {e} in {line}", file.display()))
            };
            points.push(Point {
                case: f[0].to_owned(),
                quantity: f[1].to_owned(),
                code: f[2].to_owned(),
                smallest_um: num(3)?,
                value: num(4)?,
                error_percent: num(5)?,
                seconds: num(6)?,
            });
        }
    }
    Ok(points)
}

/// Reads every study's points and writes the chart, light and dark, to `docs/`.
///
/// # Errors
///
/// When the points can't be read or the charts written.
pub fn redraw() -> Result<(), String> {
    let points = read_points()?;
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs");
    for (name, dark) in [
        ("cross-code-chart.svg", false),
        ("cross-code-chart-dark.svg", true),
    ] {
        let file = docs.join(name);
        std::fs::write(&file, svg(&points, dark))
            .map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

/// The chart's colours: the surface, its inks and lines, and each code's hue.
struct Theme {
    surface: &'static str,
    primary: &'static str,
    secondary: &'static str,
    muted: &'static str,
    grid: &'static str,
    axis: &'static str,
    codes: [(&'static str, &'static str); 3],
}

const LIGHT: Theme = Theme {
    surface: "#fcfcfb",
    primary: "#0b0b0b",
    secondary: "#52514e",
    muted: "#898781",
    grid: "#e1e0d9",
    axis: "#c3c2b7",
    codes: [
        ("photonoxide", "#2a78d6"),
        ("openEMS", "#eb6834"),
        ("Elmer", "#1baf7a"),
    ],
};

const DARK: Theme = Theme {
    surface: "#1a1a19",
    primary: "#f0efec",
    secondary: "#c3c2b7",
    muted: "#898781",
    grid: "#2c2c2a",
    axis: "#383835",
    codes: [
        ("photonoxide", "#3987e5"),
        ("openEMS", "#d95926"),
        ("Elmer", "#199e70"),
    ],
};

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A decade's label: `0.001`, `0.1`, `1`, `100`.
fn decade(e: i32) -> String {
    if e < 0 {
        format!("{:.*}", (-e) as usize, 10f64.powi(e))
    } else {
        format!("{}", 10f64.powi(e))
    }
}

/// One problem's rows: each code on the finest mesh every code in it ran (so all of them are
/// on the same mesh lines), its time and its errors by quantity.
struct Problem {
    case: String,
    smallest_um: f64,
    rows: Vec<Row>,
}

/// One code in a problem: its index among the codes, its time (s) and its errors by quantity
/// (percent).
struct Row {
    code: usize,
    seconds: f64,
    errors: Vec<(String, f64)>,
}

fn problems(points: &[Point], codes: &[(&str, &str); 3]) -> Vec<Problem> {
    let mut cases: Vec<String> = Vec::new();
    for p in points {
        if !cases.contains(&p.case) {
            cases.push(p.case.clone());
        }
    }
    let mut out = Vec::new();
    for case in cases {
        let here: Vec<&Point> = points.iter().filter(|p| p.case == case).collect();
        let present: Vec<usize> = (0..codes.len())
            .filter(|&c| here.iter().any(|p| p.code == codes[c].0))
            .collect();
        // the finest mesh that every code here has
        let mut meshes: Vec<f64> = here.iter().map(|p| p.smallest_um).collect();
        meshes.sort_by(f64::total_cmp);
        meshes.dedup();
        let Some(&smallest) = meshes.iter().find(|&&m| {
            present.iter().all(|&c| {
                here.iter()
                    .any(|p| p.code == codes[c].0 && p.smallest_um == m)
            })
        }) else {
            continue;
        };
        let rows = present
            .iter()
            .map(|&c| {
                let at: Vec<&&Point> = here
                    .iter()
                    .filter(|p| p.code == codes[c].0 && p.smallest_um == smallest)
                    .collect();
                let seconds = at.iter().map(|p| p.seconds).fold(0.0, f64::max);
                let errors = at
                    .iter()
                    .map(|p| (p.quantity.clone(), p.error_percent))
                    .collect();
                Row {
                    code: c,
                    seconds,
                    errors,
                }
            })
            .collect();
        out.push(Problem {
            case,
            smallest_um: smallest,
            rows,
        });
    }
    out
}

/// The chart as SVG: for each problem, each code's time on the same mesh as a dot on one log
/// scale, and beside it the time and its errors against photonoxide's converged result.
pub fn svg(points: &[Point], dark: bool) -> String {
    let t = if dark { &DARK } else { &LIGHT };
    let list = problems(points, &t.codes);
    let (w, row, head, top) = (1000.0_f64, 30.0_f64, 40.0_f64, 112.0_f64);
    let rows: usize = list.iter().map(|p| p.rows.len()).sum();
    let (name_x, plot_l, plot_r, time_x, err_x) = (24.0, 150.0, 560.0, 580.0, 670.0);
    let body = list.len() as f64 * head + rows as f64 * row;
    let h = top + body + 56.0;
    let times: Vec<f64> = list
        .iter()
        .flat_map(|p| p.rows.iter().map(|r| r.seconds))
        .filter(|&s| s > 0.0)
        .collect();
    let lo = times.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (x0, x1) = if times.is_empty() {
        (-2, 0)
    } else {
        let (a, b) = (lo.log10().floor() as i32, hi.log10().ceil() as i32);
        (a, b.max(a + 1))
    };
    let sx = |s: f64| plot_l + (s.log10() - f64::from(x0)) / f64::from(x1 - x0) * (plot_r - plot_l);
    let font = "font-family=\"system-ui, -apple-system, 'Segoe UI', sans-serif\"";
    let num = "style=\"font-variant-numeric: tabular-nums\"";

    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-labelledby=\"title desc\" {font}>"
    );
    let _ = writeln!(
        s,
        "<title id=\"title\">Cross-code comparison</title>\n<desc id=\"desc\">For each problem, each code's time on the same mesh (log scale) and its errors against photonoxide's converged result, in percent. The numbers are in docs/validation-openems.md and docs/validation-elmer.md.</desc>"
    );
    let _ = writeln!(
        s,
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>",
        t.surface
    );
    let _ = writeln!(
        s,
        "<text x=\"{name_x}\" y=\"36\" font-size=\"20\" font-weight=\"600\" fill=\"{}\">Cross-code comparison</text>",
        t.primary
    );
    let _ = writeln!(
        s,
        "<text x=\"{name_x}\" y=\"60\" font-size=\"13\" fill=\"{}\">Each code on the same mesh in each problem: its time, and its error against photonoxide's converged result.</text>",
        t.secondary
    );
    let _ = writeln!(
        s,
        "<text x=\"{name_x}\" y=\"78\" font-size=\"13\" fill=\"{}\">photonoxide and Elmer solve the same 2D problems; openEMS solves the line in 3D and in time.</text>",
        t.secondary
    );

    // the column heads and the time axis's decades, through the whole body
    let (y_body, y_end) = (top, top + body);
    for e in x0..=x1 {
        let x = sx(10f64.powi(e));
        let _ = writeln!(
            s,
            "<line x1=\"{x:.1}\" y1=\"{y_body}\" x2=\"{x:.1}\" y2=\"{y_end}\" stroke=\"{}\" stroke-width=\"1\"/>",
            t.grid
        );
        let _ = writeln!(
            s,
            "<text x=\"{x:.1}\" y=\"{:.1}\" font-size=\"12\" fill=\"{}\" text-anchor=\"middle\" {num}>{} s</text>",
            y_end + 20.0,
            t.muted,
            decade(e)
        );
    }
    let _ = writeln!(
        s,
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" fill=\"{}\" text-anchor=\"middle\">time, log scale</text>",
        0.5 * (plot_l + plot_r),
        y_end + 40.0,
        t.secondary
    );
    let _ = writeln!(
        s,
        "<line x1=\"{plot_l}\" y1=\"{y_end}\" x2=\"{plot_r}\" y2=\"{y_end}\" stroke=\"{}\" stroke-width=\"1\"/>",
        t.axis
    );
    for (x, label) in [(time_x, "Time"), (err_x, "Error")] {
        let _ = writeln!(
            s,
            "<text x=\"{x}\" y=\"{:.1}\" font-size=\"12\" font-weight=\"600\" fill=\"{}\">{label}</text>",
            top - 4.0,
            t.secondary
        );
    }

    let mut y = top;
    for p in &list {
        y += head;
        let _ = writeln!(
            s,
            "<text x=\"{name_x}\" y=\"{:.1}\" font-size=\"14\" font-weight=\"600\" fill=\"{}\">{} <tspan font-weight=\"400\" fill=\"{}\">(the same mesh: smallest cell {} µm)</tspan></text>",
            y - 12.0,
            t.primary,
            escape(&capitalised(&p.case)),
            t.secondary,
            p.smallest_um
        );
        for r in &p.rows {
            let (name, colour, seconds, errors) =
                (t.codes[r.code].0, t.codes[r.code].1, &r.seconds, &r.errors);
            let cy = y + 0.5 * row;
            let _ = writeln!(
                s,
                "<text x=\"{name_x}\" y=\"{:.1}\" font-size=\"13\" fill=\"{}\">{name}</text>",
                cy + 4.0,
                t.primary
            );
            let _ = writeln!(
                s,
                "<line x1=\"{plot_l}\" y1=\"{cy:.1}\" x2=\"{plot_r}\" y2=\"{cy:.1}\" stroke=\"{}\" stroke-width=\"1\"/><circle cx=\"{:.1}\" cy=\"{cy:.1}\" r=\"6\" fill=\"{colour}\" stroke=\"{}\" stroke-width=\"2\"/>",
                t.grid,
                sx(*seconds),
                t.surface
            );
            let _ = writeln!(
                s,
                "<text x=\"{time_x}\" y=\"{:.1}\" font-size=\"13\" fill=\"{}\" {num}>{}</text>",
                cy + 4.0,
                t.primary,
                crate::report::duration(*seconds)
            );
            let errs: Vec<String> = errors
                .iter()
                .map(|(q, e)| format!("{} {}", escape(q), crate::report::signed_percent(*e)))
                .collect();
            let _ = writeln!(
                s,
                "<text x=\"{err_x}\" y=\"{:.1}\" font-size=\"13\" fill=\"{}\" {num}>{}</text>",
                cy + 4.0,
                t.primary,
                errs.join("  ·  ")
            );
            y += row;
        }
    }
    let _ = writeln!(s, "</svg>");
    s
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(code: &str, quantity: &str, smallest: f64, error: f64, seconds: f64) -> Point {
        Point {
            case: "the line".into(),
            quantity: quantity.into(),
            code: code.into(),
            smallest_um: smallest,
            value: 1.0,
            error_percent: error,
            seconds,
        }
    }

    #[test]
    fn each_code_is_drawn_on_the_finest_mesh_every_code_ran() {
        let points = [
            point("photonoxide", "n", 0.5, 0.1, 0.01),
            point("photonoxide", "n", 0.25, 0.01, 0.02),
            point("photonoxide", "n", 0.125, 0.001, 0.04),
            point("openEMS", "n", 0.5, -0.5, 100.0),
            point("openEMS", "n", 0.25, -0.2, 300.0),
        ];
        let light = svg(&points, false);
        // one problem, two codes, on 0.25 µm (openEMS's finest), not photonoxide's 0.125
        assert_eq!(light.matches("<circle").count(), 2);
        assert!(light.contains("smallest cell 0.25 µm"));
        assert!(light.contains("n +0.010 %") && light.contains("n −0.20 %"));
        assert!(light.contains("5 min 0 s") && light.contains("0.020 s"));
        assert!(light.contains("#2a78d6") && light.contains("#eb6834"));
        assert!(svg(&points, true).contains("#1a1a19"));
    }

    #[test]
    fn decades_are_labelled_plainly() {
        assert_eq!(decade(-3), "0.001");
        assert_eq!(decade(0), "1");
        assert_eq!(decade(2), "100");
    }
}
