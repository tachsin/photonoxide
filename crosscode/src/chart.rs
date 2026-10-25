//! The cross-code chart: every case and every code on one plot, each code's error against
//! photonoxide's converged result (the reference) against its own solve time, both on log
//! scales. Each line is one code on one case through its meshes, finer meshes further right:
//! a work–precision chart, where lower is more accurate and further left is faster.
//!
//! The studies write their points to `crosscode/results/*.tsv` ([`write_points`]); [`redraw`]
//! reads them all and writes `docs/cross-code-chart.svg` and its dark twin,
//! `docs/cross-code-chart-dark.svg`.

use std::collections::BTreeMap;
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

/// The quantities' markers, by the case's quantity.
const MARKERS: [(&str, &str); 3] = [("n", "circle"), ("Z₀", "square"), ("peak rise", "triangle")];

fn marker(shape: &str, x: f64, y: f64, fill: &str, ring: &str) -> String {
    let r = 4.5;
    match shape {
        "square" => format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"1\" fill=\"{fill}\" stroke=\"{ring}\" stroke-width=\"2\"/>",
            x - r,
            y - r,
            2.0 * r,
            2.0 * r
        ),
        "triangle" => format!(
            "<path d=\"M{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} Z\" fill=\"{fill}\" stroke=\"{ring}\" stroke-width=\"2\" stroke-linejoin=\"round\"/>",
            x,
            y - 1.2 * r,
            x + 1.1 * r,
            y + 0.8 * r,
            x - 1.1 * r,
            y + 0.8 * r
        ),
        _ => format!(
            "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{r}\" fill=\"{fill}\" stroke=\"{ring}\" stroke-width=\"2\"/>"
        ),
    }
}

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

/// The chart as SVG.
pub fn svg(points: &[Point], dark: bool) -> String {
    let t = if dark { &DARK } else { &LIGHT };
    let (w, h) = (1000.0_f64, 640.0_f64);
    let (left, right, top, bottom) = (84.0, 250.0, 112.0, 64.0);
    let (pw, ph) = (w - left - right, h - top - bottom);
    let pts: Vec<&Point> = points
        .iter()
        .filter(|p| p.seconds > 0.0 && p.error_percent != 0.0)
        .collect();
    let span = |v: Vec<f64>| -> (i32, i32) {
        let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (lo.log10().floor() as i32, hi.log10().ceil() as i32)
    };
    let (x0, x1) = span(pts.iter().map(|p| p.seconds).collect());
    let (y0, y1) = span(pts.iter().map(|p| p.error_percent.abs()).collect());
    let (x1, y1) = (x1.max(x0 + 1), y1.max(y0 + 1));
    let sx = |s: f64| left + (s.log10() - f64::from(x0)) / f64::from(x1 - x0) * pw;
    let sy = |e: f64| top + ph - (e.abs().log10() - f64::from(y0)) / f64::from(y1 - y0) * ph;
    let font = "font-family=\"system-ui, -apple-system, 'Segoe UI', sans-serif\"";

    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-labelledby=\"title desc\" {font}>"
    );
    let _ = writeln!(
        s,
        "<title id=\"title\">Cross-code check: each code's error against its time</title>\n<desc id=\"desc\">Each line is one code on one case through its meshes. The y axis is the error against photonoxide's converged result, in percent, on a log scale; the x axis is the code's own solve time, in seconds, on a log scale. The numbers are in docs/validation-openems.md and docs/validation-elmer.md.</desc>"
    );
    let _ = writeln!(
        s,
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>",
        t.surface
    );
    let _ = writeln!(
        s,
        "<text x=\"{left}\" y=\"34\" font-size=\"19\" font-weight=\"600\" fill=\"{}\">Cross-code check: error against time</text>",
        t.primary
    );
    let _ = writeln!(
        s,
        "<text x=\"{left}\" y=\"58\" font-size=\"13\" fill=\"{}\">Each line is one code on one case, through its meshes (finer meshes to the right). Lower is more accurate, left is faster.</text>",
        t.secondary
    );
    let _ = writeln!(
        s,
        "<text x=\"{left}\" y=\"76\" font-size=\"13\" fill=\"{}\">Error against photonoxide's converged result; time is each code's own solve. openEMS solves the line in 3D and in time, the others in 2D.</text>",
        t.secondary
    );
    let _ = writeln!(
        s,
        "<text x=\"{left}\" y=\"94\" font-size=\"13\" fill=\"{}\">Where an error changes sign from one mesh to the next its size dips (n). Elmer reports its heat solver's time to 0.01 s.</text>",
        t.secondary
    );

    // the grid and the axes, at each decade
    for e in x0..=x1 {
        let x = sx(10f64.powi(e));
        let _ = writeln!(
            s,
            "<line x1=\"{x:.1}\" y1=\"{top}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
            top + ph,
            t.grid
        );
        let _ = writeln!(
            s,
            "<text x=\"{x:.1}\" y=\"{:.1}\" font-size=\"12\" fill=\"{}\" text-anchor=\"middle\" style=\"font-variant-numeric: tabular-nums\">{} s</text>",
            top + ph + 20.0,
            t.muted,
            decade(e)
        );
    }
    for e in y0..=y1 {
        let y = sy(10f64.powi(e));
        let _ = writeln!(
            s,
            "<line x1=\"{left}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
            left + pw,
            t.grid
        );
        let _ = writeln!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" fill=\"{}\" text-anchor=\"end\" style=\"font-variant-numeric: tabular-nums\">{} %</text>",
            left - 10.0,
            y + 4.0,
            t.muted,
            decade(e)
        );
    }
    let _ = writeln!(
        s,
        "<line x1=\"{left}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
        top + ph,
        left + pw,
        top + ph,
        t.axis
    );
    let _ = writeln!(
        s,
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"13\" fill=\"{}\" text-anchor=\"middle\">the code's own solve time (log scale)</text>",
        left + 0.5 * pw,
        h - 16.0,
        t.secondary
    );
    let _ = writeln!(
        s,
        "<text transform=\"translate(22 {:.1}) rotate(-90)\" font-size=\"13\" fill=\"{}\" text-anchor=\"middle\">error, % (log scale)</text>",
        top + 0.5 * ph,
        t.secondary
    );

    // the series: one code on one case's quantity, its meshes in order
    let mut series: BTreeMap<(usize, usize, String), Vec<&Point>> = BTreeMap::new();
    for p in &pts {
        let code = t.codes.iter().position(|c| c.0 == p.code).unwrap_or(0);
        let quantity = MARKERS.iter().position(|m| m.0 == p.quantity).unwrap_or(0);
        series
            .entry((quantity, code, p.case.clone()))
            .or_default()
            .push(p);
    }
    for ((quantity, code, _), list) in &mut series {
        list.sort_by(|a, b| b.smallest_um.total_cmp(&a.smallest_um));
        let colour = t.codes[*code].1;
        let path: Vec<String> = list
            .iter()
            .map(|p| format!("{:.1},{:.1}", sx(p.seconds), sy(p.error_percent)))
            .collect();
        let _ = writeln!(
            s,
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{colour}\" stroke-width=\"2\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
            path.join(" ")
        );
        for p in list.iter() {
            let _ = writeln!(
                s,
                "<g>{}<title>{}: {} on {}, {} µm mesh: {} (error {:+.4} %), {:.3} s</title></g>",
                marker(
                    MARKERS[*quantity].1,
                    sx(p.seconds),
                    sy(p.error_percent),
                    colour,
                    t.surface
                ),
                escape(&p.code),
                escape(&p.quantity),
                escape(&p.case),
                p.smallest_um,
                p.value,
                p.error_percent,
                p.seconds
            );
        }
    }

    // the legend: the codes by colour, the quantities by marker
    let lx = left + pw + 28.0;
    let mut ly = top + 6.0;
    let _ = writeln!(
        s,
        "<text x=\"{lx}\" y=\"{ly}\" font-size=\"12\" font-weight=\"600\" fill=\"{}\">Code</text>",
        t.secondary
    );
    for (name, colour) in t.codes {
        if !pts.iter().any(|p| p.code == name) {
            continue;
        }
        ly += 24.0;
        let _ = writeln!(
            s,
            "<line x1=\"{lx}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{colour}\" stroke-width=\"2\"/>{}<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"13\" fill=\"{}\">{name}</text>",
            ly - 4.0,
            lx + 28.0,
            ly - 4.0,
            marker("circle", lx + 14.0, ly - 4.0, colour, t.surface),
            lx + 38.0,
            ly,
            t.primary
        );
    }
    ly += 40.0;
    let _ = writeln!(
        s,
        "<text x=\"{lx}\" y=\"{ly}\" font-size=\"12\" font-weight=\"600\" fill=\"{}\">Case and quantity</text>",
        t.secondary
    );
    let mut cases: Vec<(usize, String)> = series
        .keys()
        .map(|(q, _, case)| (*q, case.clone()))
        .collect();
    cases.dedup();
    cases.sort();
    cases.dedup();
    for (q, case) in cases {
        ly += 24.0;
        let _ = writeln!(
            s,
            "{}<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"13\" fill=\"{}\">{}, {}</text>",
            marker(MARKERS[q].1, lx + 14.0, ly - 4.0, t.muted, t.surface),
            lx + 38.0,
            ly,
            t.primary,
            escape(&case),
            escape(MARKERS[q].0)
        );
    }
    let _ = writeln!(s, "</svg>");
    s
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
    fn the_chart_has_a_line_per_code_and_quantity_and_a_marker_per_point() {
        let points = [
            point("photonoxide", "n", 1.0, 0.1, 0.01),
            point("photonoxide", "n", 0.5, 0.01, 0.02),
            point("Elmer", "Z₀", 1.0, -0.5, 0.1),
            point("Elmer", "Z₀", 0.5, -0.2, 0.3),
        ];
        let light = svg(&points, false);
        assert_eq!(light.matches("<polyline").count(), 2);
        assert_eq!(light.matches("<title>").count(), 4);
        assert!(light.contains("#2a78d6") && light.contains("#1baf7a"));
        assert!(light.contains("0.01 s") && light.contains("1 %"));
        assert!(
            !light.contains("openEMS</text>"),
            "a code without points has no legend entry"
        );
        assert!(svg(&points, true).contains("#1a1a19"));
    }

    #[test]
    fn decades_are_labelled_plainly() {
        assert_eq!(decade(-3), "0.001");
        assert_eq!(decade(0), "1");
        assert_eq!(decade(2), "100");
    }
}
