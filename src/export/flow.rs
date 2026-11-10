//! A design's process flow, exported (F7 of #622): its data as JSON with stable keys
//! ([`FlowFile`], [`FlowFile::json`]), a process traveller in Markdown ([`FlowFile::markdown`]),
//! the steps as a CSV table ([`FlowFile::csv`]), and the wafer's cross-section after each step
//! as a standalone SVG ([`FlowExport::svg`]). A run of a `"fabrication"` job writes them all
//! into its folder ([`FlowExport::write`]); the façade gives them to Python
//! ([`crate::facade::device_fabrication`]).
//!
//! The JSON holds the flow and the assessment exactly as the run's `flow` and `flow_assessment`
//! events record them, each number the shortest decimal that reads back to the same bits, so
//! [`FlowFile::read`] gives back the same flow. The traveller and the cross-sections are made
//! from the same data: the traveller from the file alone, the cross-sections from the wafer as
//! the flow leaves it after each step ([`crate::fabrication::stages`]), cut across y at the
//! middle of the design's shapes in x, as the studio's design view starts its cut, or on a plane of
//! one's choosing ([`FlowExport::cut_at`]). Each draws the wafer as the studio's Fabrication page
//! draws the step's last frame: the planar layers, and over them the resist, a film deposited
//! over an etched step and a film evaporated onto a stencil ([`crate::job::FrameFilm`]).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::device::Spec;
use crate::fabrication::{
    Assessment, Choices, Dimension, FlowRecord, Kind, Made, Source, Start, StepRecord, Unit,
    Verdict, assess, generate_from, round_trip, stages,
};
use crate::geometry::Point;
use crate::job::FrameFilm;
use crate::units::Wavelength;
use crate::{Error, Result};

/// A parameter of the design, at its value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowParameter {
    /// Its name, e.g. `"width"`.
    pub name: String,
    /// Its value, in `unit`.
    pub value: f64,
    /// Its unit, e.g. `"µm"`, or `""` for a pure number.
    pub unit: String,
}

/// Where the cross-sections are cut, and the window they show.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowCut {
    /// The plane x = `x_um` the wafer is cut on, µm: the middle of the design's shapes in x,
    /// unless [`FlowExport::cut_at`] put it elsewhere.
    pub x_um: f64,
    /// Across it, from and to, µm: the shapes' extent in y and 1 µm beyond it.
    pub y_um: [f64; 2],
    /// Up, from and to, µm: from 0.5 µm into the substrate to 1 µm above the highest the wafer
    /// stands at any step, its resist and films included.
    pub z_um: [f64; 2],
}

/// A layer of the wafer after a step, as a cross-section shows it from the top down.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StackRow {
    /// The layer's name, as the design's stack has it; `"cladding"` above everything and
    /// `"substrate"` below.
    pub name: String,
    /// The film there: the drawn shapes' in a patterned layer, the whole layer's otherwise.
    pub film: String,
    /// What fills a patterned layer where its shapes aren't, when that is another film: the
    /// film deposited into its etched regions, or `"air"` where nothing fills them yet.
    pub elsewhere: Option<String>,
    /// Its thickness, µm; `None` for the cladding and the substrate.
    pub thickness_um: Option<f64>,
}

/// A flow's export, as `flow.json` holds it: the design, the flow and what the rules find in it,
/// and the wafer's layers after each step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowFile {
    /// The device's kind, e.g. `"waveguide"`.
    pub device: String,
    /// Its parameters at the design's values, in the device's order.
    pub parameters: Vec<FlowParameter>,
    /// Where the cross-sections are cut.
    pub cut: FlowCut,
    /// The flow, as a run's `flow` event records it: its steps with their reasons,
    /// alternatives, capabilities and sources, the warnings, what is not specified, the
    /// library's version, the round trip, the references and the glossary.
    pub flow: FlowRecord,
    /// What the rules find, as a run's `flow_assessment` event records it: the checks, each
    /// step's processes ranked, and the accuracy of each fabricated dimension.
    pub assessment: Assessment,
    /// The wafer after each step, in order: its layers from the top down, the cladding (air
    /// until one is deposited) first and the substrate last.
    pub stacks: Vec<Vec<StackRow>>,
}

/// A flow's export: its file, and the wafer after each step, which the cross-sections draw.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowExport {
    /// What `flow.json` holds.
    pub file: FlowFile,
    /// The wafer as each step leaves it, as the studio's Fabrication page draws its last frame:
    /// the planar layers, and the films over them (the resist, a film deposited over an etched
    /// step, a film evaporated onto a stencil).
    drawn: Vec<(Made, Vec<FrameFilm>)>,
    /// The window's extent in x, µm, that a cut may be placed in: the shapes' and 1 µm beyond.
    x_um: [f64; 2],
}

/// The convention every list of layers here follows.
pub const LAYERS_TOP_DOWN: &str =
    "Layers are listed from the top down, the substrate last, as a cross-section is drawn.";

fn io_error(path: &Path, e: &std::io::Error) -> Error {
    Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

/// A number as the traveller writes it: to four significant digits, without trailing zeros.
fn number(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v == 0.0 { "0".into() } else { v.to_string() };
    }
    let digits = (3 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// `v` in `unit`, as the narration writes it: `500 nm`, `82°`.
fn with_unit(v: f64, unit: Unit) -> String {
    match unit {
        Unit::Degree | Unit::Celsius | Unit::Ratio | Unit::Percent | Unit::Number => {
            format!("{}{}", number(v), unit.symbol())
        }
        _ => format!("{} {}", number(v), unit.symbol()),
    }
}

/// The layers of `made`, from the top down.
fn rows(made: &Made) -> Vec<StackRow> {
    let s = &made.structure;
    let mut rows = vec![StackRow {
        name: "cladding".into(),
        film: made.cladding.clone(),
        elsewhere: None,
        thickness_um: None,
    }];
    for (layer, (drawn, background)) in s.stack().layers().iter().zip(&made.layers).rev() {
        let patterned = !s.shapes(&layer.name).is_empty();
        rows.push(StackRow {
            name: layer.name.clone(),
            film: drawn.clone(),
            elsewhere: (patterned && drawn != background).then(|| background.clone()),
            thickness_um: Some(layer.thickness.to_um()),
        });
    }
    rows.push(StackRow {
        name: "substrate".into(),
        film: made.substrate.clone(),
        elsewhere: None,
        thickness_um: None,
    });
    rows
}

impl FlowExport {
    /// The flow of `spec`'s design as `choices` ask, its round trip and what the rules find in
    /// it ([`crate::fabrication::fabricate`], [`assess`]), with the wafer after each step: what a
    /// `"fabrication"` job's run records, to the bit, and the files it writes.
    ///
    /// # Errors
    ///
    /// The design's ([`Spec::build`], [`crate::device::Device::physical`]), a process
    /// `choices` names that the library hasn't, and [`crate::fabrication::generate`]'s and
    /// [`round_trip`]'s.
    pub fn of_design(spec: &Spec, choices: &Choices) -> Result<FlowExport> {
        FlowExport::of_design_from(spec, choices, Start::Bought)
    }

    /// The export [`FlowExport::of_design`] gives, the flow starting from `start`
    /// ([`generate_from`]): for an SOI design, from bare silicon with the Smart Cut steps.
    ///
    /// # Errors
    ///
    /// [`FlowExport::of_design`]'s, and [`generate_from`]'s.
    pub fn of_design_from(spec: &Spec, choices: &Choices, start: Start) -> Result<FlowExport> {
        choices.check()?;
        let (device, design) = spec.build()?;
        let values = design.values();
        let physical = device.physical(values)?;
        let flow = generate_from(&physical, choices, start)?;
        let trip = round_trip(&flow, &physical)?;
        let assessment = assess(&physical, &flow)?;
        let made = stages(&flow)?;
        let s = physical.structure();
        let (mut x, mut y) = (
            [f64::INFINITY, f64::NEG_INFINITY],
            [f64::INFINITY, f64::NEG_INFINITY],
        );
        for l in s.stack().layers() {
            for shape in s.shapes(&l.name) {
                let b = shape.bounds();
                x = [x[0].min(b.min.x.to_um()), x[1].max(b.max.x.to_um())];
                y = [y[0].min(b.min.y.to_um()), y[1].max(b.max.y.to_um())];
            }
        }
        if !x[0].is_finite() {
            (x, y) = ([0.0, 0.0], [0.0, 0.0]);
        }
        // each step's last frame, as the Fabrication page draws it, over the same window
        let window = ([x[0] - 1.0, x[1] + 1.0], [y[0] - 1.0, y[1] + 1.0]);
        let (moments, _, _) =
            crate::job::moments(&flow, &made, window.0, window.1, Wavelength::um(1.55)?)?;
        let top = made
            .iter()
            .map(|m| m.structure.stack().thickness().to_um())
            .chain(moments.iter().map(crate::job::Moment::top))
            .fold(s.stack().thickness().to_um(), f64::max);
        let mut drawn: Vec<(Made, Vec<FrameFilm>)> = Vec::with_capacity(made.len());
        for m in moments {
            if m.step < drawn.len() {
                drawn[m.step] = (m.made, m.films);
            } else {
                drawn.push((m.made, m.films));
            }
        }
        let cut = FlowCut {
            x_um: 0.5 * (x[0] + x[1]),
            y_um: window.1,
            z_um: [-0.5, top + 1.0],
        };
        let parameters = device
            .parameters()
            .iter()
            .filter_map(|p| {
                values.get(&p.name).map(|value| FlowParameter {
                    name: p.name.clone(),
                    value,
                    unit: p.unit.clone(),
                })
            })
            .collect();
        Ok(FlowExport {
            file: FlowFile {
                device: device.kind().to_owned(),
                parameters,
                cut,
                flow: flow.record(Some(trip)),
                assessment,
                stacks: made.iter().map(rows).collect(),
            },
            drawn,
            x_um: window.0,
        })
    }

    /// The same export with its cross-sections cut on the plane x = `x_um` in place of the
    /// middle of the design's shapes: anywhere across the window the cut shows, the shapes'
    /// extent in x and 1 µm beyond it. `flow.json`'s `cut` records it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a plane outside the window, or not a number.
    pub fn cut_at(mut self, x_um: f64) -> Result<FlowExport> {
        let [lo, hi] = self.x_um;
        if !(x_um >= lo && x_um <= hi) {
            return Err(Error::invalid(
                "cross-section's cut",
                format!(
                    "is a plane x from {} to {} µm (the design's shapes and 1 µm beyond), not {x_um}",
                    number(lo),
                    number(hi)
                ),
            ));
        }
        self.file.cut.x_um = x_um;
        Ok(self)
    }

    /// The name of step `k`'s (from 0) cross-section among `n` steps: `flow-01.svg`, ...
    pub fn svg_name(k: usize, n: usize) -> String {
        let width = n.to_string().len().max(2);
        format!("flow-{:0width$}.svg", k + 1)
    }

    /// Writes `flow.json`, `flow.md`, `flow.csv` and each step's cross-section (`flow-01.svg`,
    /// ...) into `dir`, and returns the files written, in that order.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if `dir` can't be made or a file written.
    pub fn write(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dir).map_err(|e| io_error(dir, &e))?;
        let n = self.file.flow.steps.len();
        let mut files = vec![
            (dir.join("flow.json"), self.file.json()),
            (dir.join("flow.md"), self.file.markdown()),
            (dir.join("flow.csv"), self.file.csv()),
        ];
        for k in 0..n {
            files.push((dir.join(Self::svg_name(k, n)), self.svg(k)));
        }
        for (path, text) in &files {
            std::fs::write(path, text).map_err(|e| io_error(path, &e))?;
        }
        Ok(files.into_iter().map(|(p, _)| p).collect())
    }

    /// Every step's cross-section, in order ([`FlowExport::svg`]).
    pub fn svgs(&self) -> Vec<String> {
        (0..self.drawn.len()).map(|k| self.svg(k)).collect()
    }

    /// The wafer's cross-section after step `k` (from 0), as a standalone SVG: the layers drawn
    /// across the cut with each etch's floor and walls exact, each film in its colour, and
    /// beside them the layers from the top down, the substrate last, with their films and
    /// thicknesses. y and z are drawn to scales of their own. Over the layers, the films the
    /// Fabrication page draws at the step's last frame: the resist, a film deposited over an etched
    /// step (conformal), a film evaporated onto a stencil; each cut exactly, and named in the
    /// legend.
    ///
    /// # Panics
    ///
    /// If `k` is not a step of the flow.
    pub fn svg(&self, k: usize) -> String {
        Section::new(&self.file, &self.drawn[k], k).svg()
    }
}

/// The SVG's layout, in pixels.
const PLOT_X: f64 = 64.0;
const PLOT_Y: f64 = 72.0;
const PLOT_W: f64 = 480.0;
const PLOT_H: f64 = 300.0;
const LEGEND_X: f64 = 580.0;
const WIDTH: f64 = 960.0;
/// The samples across the cut a patterned layer's profile is read at.
const SAMPLES: usize = 2001;

/// A film's colour in the cross-sections, as `#rrggbb`: the exported SVGs' and the studio's
/// Fabrication page's, in 2D and 3D alike. A resist (a film named `… resist` or `photoresist`)
/// is magenta, and a light pink where the light or the beam falls on it (`…, exposed`).
pub fn film_colour(film: &str) -> &'static str {
    if film.ends_with(", exposed") {
        return "#f2bde8";
    }
    if film.ends_with("resist") {
        return "#c45ab3";
    }
    match film {
        "air" => "#ffffff",
        "Si" => "#7f8fa6",
        "SiO2" => "#cfe6f5",
        "LiNbO3" => "#e7a9c4",
        "TiN" => "#c8a03c",
        "Cu/Au" => "#d9a520",
        "Au" | "Cu" | "Al" => "#d0a63a",
        "Si3N4" | "SiN" => "#9ccf9f",
        // what a flow that makes its own SOI wafer draws in its frames
        "implanted hydrogen" => "#e8673c",
        "bond interface" => "#2f3644",
        "Si, implant-damaged" => "#a99bb8",
        _ => {
            const OTHERS: [&str; 6] = [
                "#b7a1d6", "#f0c27a", "#9fd3c7", "#e38b8b", "#a3b86c", "#d7b5a0",
            ];
            let h = film.bytes().fold(0usize, |h, b| {
                h.wrapping_mul(31).wrapping_add(usize::from(b))
            });
            OTHERS[h % OTHERS.len()]
        }
    }
}

/// Text made safe for XML.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A stack row as the legend and the traveller write it.
fn describe(row: &StackRow) -> String {
    let mut s = format!("{}: {}", row.name, row.film);
    if let Some(e) = &row.elsewhere {
        let _ = write!(s, ", {e} where etched");
    }
    match (row.thickness_um, row.name.as_str()) {
        (Some(t), _) => {
            let _ = write!(s, ", {} µm", number(t));
        }
        (None, "cladding") => s.push_str(", above everything"),
        (None, _) => s.push_str(", below everything"),
    }
    s
}

/// One step's cross-section, being drawn: the wafer's planar layers as the step leaves them,
/// and the films over them.
struct Section<'a> {
    file: &'a FlowFile,
    made: &'a Made,
    films: &'a [FrameFilm],
    k: usize,
}

impl Section<'_> {
    fn new<'a>(file: &'a FlowFile, drawn: &'a (Made, Vec<FrameFilm>), k: usize) -> Section<'a> {
        Section {
            file,
            made: &drawn.0,
            films: &drawn.1,
            k,
        }
    }

    /// The films drawn over the layers, each once, in the order drawn.
    fn over(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for f in self.films {
            if !names.contains(&f.film.as_str()) && !cut_film(f, self.file.cut.x_um).is_empty() {
                names.push(&f.film);
            }
        }
        names
    }

    fn px(&self, y: f64) -> f64 {
        let [y0, y1] = self.file.cut.y_um;
        PLOT_X + (y - y0) / (y1 - y0) * PLOT_W
    }

    fn pz(&self, z: f64) -> f64 {
        let [z0, z1] = self.file.cut.z_um;
        PLOT_Y + PLOT_H - (z - z0) / (z1 - z0) * PLOT_H
    }

    fn rect(&self, out: &mut String, z: [f64; 2], film: &str) {
        let (top, bottom) = (self.pz(z[1]), self.pz(z[0]));
        let _ = writeln!(
            out,
            r#"<rect x="{PLOT_X:.3}" y="{top:.3}" width="{PLOT_W:.3}" height="{:.3}" fill="{}"/>"#,
            (bottom - top).max(0.0),
            film_colour(film)
        );
    }

    /// The layer `name`'s drawn film, from its bottom `bottom` up to where it stands at each
    /// sample across the cut, as a polygon in pixels with collinear points dropped.
    fn profile(&self, name: &str, bottom: f64) -> Vec<(f64, f64)> {
        let [y0, y1] = self.file.cut.y_um;
        let x = self.file.cut.x_um;
        let mut points: Vec<(f64, f64)> = (0..SAMPLES)
            .map(|i| {
                let y = y0 + (y1 - y0) * i as f64 / (SAMPLES - 1) as f64;
                let kept = self
                    .made
                    .structure
                    .kept(name, Point::um(x, y))
                    .map_or(0.0, |h| h.to_um());
                (self.px(y), self.pz(bottom + kept))
            })
            .collect();
        points.push((self.px(y1), self.pz(bottom)));
        points.push((self.px(y0), self.pz(bottom)));
        let mut kept: Vec<(f64, f64)> = Vec::with_capacity(points.len());
        for p in points {
            while kept.len() >= 2 {
                let (a, b) = (kept[kept.len() - 2], kept[kept.len() - 1]);
                let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
                if cross.abs() < 1e-9 {
                    kept.pop();
                } else {
                    break;
                }
            }
            kept.push(p);
        }
        kept
    }

    fn svg(&self) -> String {
        let steps = &self.file.flow.steps;
        let step = &steps[self.k];
        let rows = &self.file.stacks[self.k];
        let cut = self.file.cut;
        let over = self.over();
        let legend_bottom = PLOT_Y + 24.0 + 22.0 * (rows.len() + over.len()) as f64;
        let height = (PLOT_Y + PLOT_H + 70.0).max(legend_bottom + 16.0);
        let title = format!("Step {} of {}: {}", self.k + 1, steps.len(), step.name);
        let mut out = String::new();
        let _ = writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = writeln!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" viewBox="0 0 {WIDTH} {height}" font-family="sans-serif" font-size="13">"#
        );
        let _ = writeln!(out, "<title>{}</title>", escape(&title));
        let _ = writeln!(
            out,
            "<desc>{}</desc>",
            escape(&format!(
                "The wafer after this step, cut at x = {} µm. {LAYERS_TOP_DOWN}",
                number(cut.x_um)
            ))
        );
        let _ = writeln!(
            out,
            r##"<rect x="0" y="0" width="{WIDTH}" height="{height}" fill="#ffffff"/>"##
        );
        let _ = writeln!(
            out,
            r#"<text x="16" y="26" font-size="16" font-weight="bold">{}</text>"#,
            escape(&title)
        );
        let _ = writeln!(
            out,
            r#"<text x="16" y="48">{}</text>"#,
            escape(&format!("{}: {}", step.process, step.does))
        );
        // the drawing: the substrate, each layer's background, the cladding, the films over the
        // layers (the resist, a film over an etched step), then the layers' drawn shapes, as the
        // Fabrication page draws a frame
        let _ = writeln!(
            out,
            r#"<defs><clipPath id="plot"><rect x="{PLOT_X:.3}" y="{PLOT_Y:.3}" width="{PLOT_W:.3}" height="{PLOT_H:.3}"/></clipPath></defs>"#
        );
        let _ = writeln!(out, r#"<g id="cross-section">"#);
        let [z0, z1] = cut.z_um;
        self.rect(&mut out, [z0, 0.0], &self.made.substrate);
        let mut bottom = 0.0;
        let mut tops = Vec::new();
        let mut shapes = Vec::new();
        for (layer, (drawn, background)) in self
            .made
            .structure
            .stack()
            .layers()
            .iter()
            .zip(&self.made.layers)
        {
            let top = bottom + layer.thickness.to_um();
            let patterned = !self.made.structure.shapes(&layer.name).is_empty();
            self.rect(
                &mut out,
                [bottom, top],
                if patterned { background } else { drawn },
            );
            if patterned {
                let points = self
                    .profile(&layer.name, bottom)
                    .iter()
                    .map(|(a, b)| format!("{a:.3},{b:.3}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                shapes.push(format!(
                    r#"<polygon points="{points}" fill="{}"/>"#,
                    film_colour(drawn)
                ));
            }
            tops.push(top);
            bottom = top;
        }
        self.rect(&mut out, [bottom, z1], &self.made.cladding);
        let _ = writeln!(out, r#"<g id="films" clip-path="url(#plot)">"#);
        for film in self.films {
            for piece in cut_film(film, cut.x_um) {
                let points = piece
                    .iter()
                    .map(|&(y, z)| format!("{:.3},{:.3}", self.px(y), self.pz(z)))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = writeln!(
                    out,
                    r#"<polygon points="{points}" fill="{}"/>"#,
                    film_colour(&film.film)
                );
            }
        }
        let _ = writeln!(out, "</g>");
        for shape in shapes {
            let _ = writeln!(out, "{shape}");
        }
        for z in std::iter::once(0.0).chain(tops.iter().copied()) {
            let _ = writeln!(
                out,
                r##"<line x1="{PLOT_X:.3}" y1="{0:.3}" x2="{1:.3}" y2="{0:.3}" stroke="#555555" stroke-width="0.5"/>"##,
                self.pz(z),
                PLOT_X + PLOT_W
            );
        }
        let _ = writeln!(
            out,
            r##"<rect x="{PLOT_X:.3}" y="{PLOT_Y:.3}" width="{PLOT_W:.3}" height="{PLOT_H:.3}" fill="none" stroke="#333333"/>"##
        );
        let _ = writeln!(out, "</g>");
        // the axes' ends
        let below = PLOT_Y + PLOT_H + 18.0;
        let _ = writeln!(
            out,
            r#"<text x="{PLOT_X:.3}" y="{below:.3}">y = {} µm</text>"#,
            number(cut.y_um[0])
        );
        let _ = writeln!(
            out,
            r#"<text x="{:.3}" y="{below:.3}" text-anchor="end">y = {} µm</text>"#,
            PLOT_X + PLOT_W,
            number(cut.y_um[1])
        );
        for z in [0.0, bottom] {
            let _ = writeln!(
                out,
                r#"<text x="{:.3}" y="{:.3}" text-anchor="end">z = {}</text>"#,
                PLOT_X - 6.0,
                self.pz(z) + 4.0,
                number(z)
            );
        }
        let _ = writeln!(
            out,
            r#"<text x="16" y="{:.3}">{}</text>"#,
            below + 22.0,
            escape(&format!(
                "Cut at x = {} µm, across y; z in µm; y and z each to its own scale.{}",
                number(cut.x_um),
                if resist_on(steps, self.k) {
                    " The resist is on the wafer."
                } else {
                    ""
                }
            ))
        );
        // the layers, from the top down
        let _ = writeln!(out, r#"<g id="layers">"#);
        let _ = writeln!(
            out,
            r#"<text x="{LEGEND_X}" y="{PLOT_Y:.3}" font-weight="bold">{}</text>"#,
            escape("Layers, from the top down, the substrate last")
        );
        for (i, row) in rows.iter().enumerate() {
            let y = PLOT_Y + 24.0 + 22.0 * i as f64;
            let _ = writeln!(
                out,
                r##"<rect x="{LEGEND_X}" y="{:.3}" width="14" height="14" fill="{}" stroke="#333333" stroke-width="0.5"/>"##,
                y - 12.0,
                film_colour(&row.film)
            );
            if let Some(e) = &row.elsewhere {
                let _ = writeln!(
                    out,
                    r##"<rect x="{}" y="{:.3}" width="14" height="14" fill="{}" stroke="#333333" stroke-width="0.5"/>"##,
                    LEGEND_X + 16.0,
                    y - 12.0,
                    film_colour(e)
                );
            }
            let _ = writeln!(
                out,
                r#"<text x="{}" y="{y:.3}">{}</text>"#,
                LEGEND_X + 38.0,
                escape(&describe(row))
            );
        }
        for (i, film) in over.iter().enumerate() {
            let y = PLOT_Y + 24.0 + 22.0 * (rows.len() + i) as f64;
            let _ = writeln!(
                out,
                r##"<rect x="{LEGEND_X}" y="{:.3}" width="14" height="14" fill="{}" stroke="#333333" stroke-width="0.5"/>"##,
                y - 12.0,
                film_colour(film)
            );
            let _ = writeln!(
                out,
                r#"<text x="{}" y="{y:.3}">{}</text>"#,
                LEGEND_X + 38.0,
                escape(&format!("over the layers: {film}"))
            );
        }
        let _ = writeln!(out, "</g>");
        let _ = writeln!(out, "</svg>");
        out
    }
}

/// A film's piece ([`FrameFilm`]) cut by the plane x = `x`: the quadrilaterals, (y, z) in µm,
/// that make up its section, each from its bottom left round. The piece is the loft between its
/// bottom and top outlines, each vertex moving straight from the one to the other, its faces
/// planar (each edge stays parallel to itself); between the heights where a vertex crosses the
/// plane, the same edges cross it, each where it does moving linearly in height, so the section
/// there is exactly the trapezoids between the crossings paired in order along y (even-odd, the
/// holes included).
fn cut_film(film: &FrameFilm, x: f64) -> Vec<[(f64, f64); 4]> {
    let (bottom, top) = (&film.bottom, &film.top);
    let [z0, z1] = film.z_um;
    if bottom.len() != top.len() || z1 <= z0 {
        return Vec::new();
    }
    // the rings, as index ranges: each back to its first vertex, which isn't repeated here
    let mut rings: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    while start < bottom.len() {
        let first = bottom[start];
        let end = (start + 1..bottom.len())
            .find(|&k| bottom[k] == first)
            .unwrap_or(bottom.len());
        if end - start >= 3 {
            rings.push((start, end));
        }
        start = end + 1;
    }
    let at = |k: usize, s: f64| {
        let (b, t) = (bottom[k], top[k]);
        (b[0] + s * (t[0] - b[0]), b[1] + s * (t[1] - b[1]))
    };
    let mut breaks = vec![0.0, 1.0];
    for &(a, b) in &rings {
        for k in a..b {
            let dx = top[k][0] - bottom[k][0];
            if dx != 0.0 {
                let s = (x - bottom[k][0]) / dx;
                if s > 0.0 && s < 1.0 {
                    breaks.push(s);
                }
            }
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    let mut pieces = Vec::new();
    for w in breaks.windows(2) {
        let (sa, sb) = (w[0], w[1]);
        if sb - sa < 1e-12 {
            continue;
        }
        let sm = 0.5 * (sa + sb);
        // where each edge crossing the plane crosses it, at the middle and at the ends
        let mut crossings: Vec<(f64, f64, f64)> = Vec::new();
        for &(a, b) in &rings {
            for i in a..b {
                let j = if i + 1 == b { a } else { i + 1 };
                let (p, q) = (at(i, sm), at(j, sm));
                if (p.0 <= x) == (q.0 <= x) {
                    continue;
                }
                let y_at = |s: f64| {
                    let (p, q) = (at(i, s), at(j, s));
                    if q.0 == p.0 {
                        p.1
                    } else {
                        p.1 + (x - p.0) * (q.1 - p.1) / (q.0 - p.0)
                    }
                };
                crossings.push((y_at(sm), y_at(sa), y_at(sb)));
            }
        }
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (za, zb) = (z0 + sa * (z1 - z0), z0 + sb * (z1 - z0));
        for [lo, hi] in crossings.as_chunks::<2>().0 {
            let (lo, hi) = (*lo, *hi);
            pieces.push([(lo.1, za), (hi.1, za), (hi.2, zb), (lo.2, zb)]);
        }
    }
    pieces
}

/// Whether resist is on the wafer after step `k`: patterned and not yet removed or lifted off.
fn resist_on(steps: &[StepRecord], k: usize) -> bool {
    let mut on = false;
    for s in &steps[..=k] {
        match s.operation.as_str() {
            "pattern" => on = true,
            "strip" | "lift_off" => on = false,
            _ => {}
        }
    }
    on
}

impl FlowFile {
    /// Its JSON, as `flow.json` holds it: pretty-printed, the keys in a fixed order (the
    /// fields' own), each number the shortest decimal that reads back to the same bits.
    ///
    /// # Panics
    ///
    /// Never: the file is plain data (strings, numbers, lists and records), which always
    /// serializes.
    pub fn json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a flow always serializes");
        text.push('\n');
        text
    }

    /// The steps as a CSV table, as `flow.csv` holds it: a header line, then one line a step in
    /// order, for a spreadsheet or any CSV reader. Its columns: `step` (from 1), `process` (the
    /// id), `name`, `step_type`, `operation`, `does`, `layer`, `film`, `thickness_um`,
    /// `depth_um`, `timed`, `sidewall_deg`, `tone`, `reason`, and the counts of the step's
    /// `capabilities` and `uncertainties` (their text and sources are in `flow.json` and the
    /// traveller). Each number is the shortest decimal that reads back to the same bits; an empty
    /// field is one the step doesn't have; a field with a comma, a quote or a line break is
    /// quoted, its quotes doubled (RFC 4180), as the run's other tables
    /// ([`crate::export::tables`]).
    pub fn csv(&self) -> String {
        use super::tables::{num, quote};
        let opt = |v: Option<f64>| v.map(num).unwrap_or_default();
        let mut text = String::from(
            "step,process,name,step_type,operation,does,layer,film,thickness_um,depth_um,timed,\
             sidewall_deg,tone,reason,capabilities,uncertainties\n",
        );
        for (k, s) in self.flow.steps.iter().enumerate() {
            let tone = match s.tone {
                Some(crate::fabrication::Tone::Keep) => "keep",
                Some(crate::fabrication::Tone::Open) => "open",
                None => "",
            };
            let fields = [
                (k + 1).to_string(),
                s.process.clone(),
                s.name.clone(),
                s.step.name().to_owned(),
                s.operation.clone(),
                s.does.clone(),
                s.layer.clone().unwrap_or_default(),
                s.film.clone().unwrap_or_default(),
                opt(s.thickness_um),
                opt(s.depth_um),
                s.timed.to_string(),
                opt(s.sidewall_deg),
                tone.to_owned(),
                s.reason.clone(),
                s.capabilities.len().to_string(),
                s.uncertainties.len().to_string(),
            ];
            let line = fields.iter().map(|f| quote(f)).collect::<Vec<_>>();
            text.push_str(&line.join(","));
            text.push('\n');
        }
        text
    }

    /// The file `text` holds, read back: [`FlowFile::json`]'s text gives the same file.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for text that isn't a flow's file.
    pub fn read(text: &str) -> Result<FlowFile> {
        serde_json::from_str(text).map_err(|e| Error::Parse {
            what: "flow.json".into(),
            reason: e.to_string(),
        })
    }

    /// Where `source` is: its reference's citation and the place in it.
    fn cite(&self, source: &Source) -> String {
        let citation = self
            .flow
            .references
            .iter()
            .find(|r| r.key == source.reference)
            .map_or(source.reference.as_str(), |r| r.citation.as_str());
        format!("{citation}, {}", source.location)
    }

    /// The layers step `k` (from 0) adds to the wafer.
    fn new_layers(&self, k: usize) -> Vec<&str> {
        let names = |k: usize| -> Vec<&str> {
            self.stacks[k]
                .iter()
                .filter(|r| r.thickness_um.is_some())
                .map(|r| r.name.as_str())
                .collect()
        };
        let before = if k == 0 { Vec::new() } else { names(k - 1) };
        names(k)
            .into_iter()
            .filter(|n| !before.contains(n))
            .collect()
    }

    /// The step (from 0) that sets dimension `d`: a line's width, floor, depth and walls its
    /// layer's etch (or lift-off); a layer's placement the lithography that patterns it; a
    /// film's thickness the etch that stops on it (its over-etch takes from it), else the polish
    /// that flattens it, else the step that lays it down (an evaporation, for a film lifted off).
    fn owner(&self, d: &Dimension) -> Option<usize> {
        let steps = &self.flow.steps;
        let on = |k: usize| steps[k].layer.as_deref() == Some(d.layer.as_str());
        let last = |op: &str| {
            (0..steps.len())
                .rev()
                .find(|&k| steps[k].operation == op && on(k))
        };
        let shaped = || last("etch").or_else(|| last("lift_off"));
        match d.kind {
            Kind::Width | Kind::FloorWidth | Kind::Depth | Kind::Sidewall => shaped(),
            Kind::Placement => {
                shaped().and_then(|j| (0..j).rev().find(|&k| steps[k].operation == "pattern"))
            }
            _ => {
                // the step that lays it down: by its layer, or by its film (an evaporation's,
                // which a lift-off makes a layer of)
                let made = (0..steps.len())
                    .find(|&k| self.new_layers(k).contains(&d.layer.as_str()))
                    .or_else(|| {
                        (0..steps.len()).find(|&k| {
                            matches!(
                                steps[k].operation.as_str(),
                                "evaporate" | "sputter" | "deposit"
                            ) && steps[k].film.as_deref() == Some(d.layer.as_str())
                        })
                    });
                let stopped = (0..steps.len()).find(|&k| {
                    let s = &steps[k];
                    let rows = &self.stacks[k];
                    s.operation == "etch"
                        && !s.timed
                        && rows
                            .iter()
                            .position(|r| Some(r.name.as_str()) == s.layer.as_deref())
                            .and_then(|i| rows.get(i + 1))
                            .is_some_and(|below| below.name == d.layer)
                });
                // a film laid over it (a deposit, Smart Cut's bond of the donor's film, or
                // SIMOX's anneal, which draws the film it heals) keeps the polish off it
                let over = |j: usize| {
                    matches!(steps[j].operation.as_str(), "deposit" | "bond")
                        || (matches!(steps[j].operation.as_str(), "implant" | "anneal")
                            && steps[j].film.is_some())
                };
                let polished = made.and_then(|m| {
                    // SIMOX's film is thinned by its sacrificial oxidation alone, with no polish
                    let first = (m + 1..steps.len()).find(|&k| {
                        matches!(steps[k].operation.as_str(), "planarize" | "oxidize")
                            && !(m + 1..k).any(over)
                    })?;
                    // Smart Cut's sacrificial oxidation right after the polish thins it last
                    Some(
                        (first + 1..steps.len())
                            .take_while(|&k| steps[k].operation == "oxidize")
                            .last()
                            .unwrap_or(first),
                    )
                });
                stopped.or(polished).or(made)
            }
        }
    }

    /// The dimensions step `k` (from 0) sets ([`FlowFile::owner`]).
    fn results(&self, k: usize) -> Vec<&Dimension> {
        self.assessment
            .accuracy
            .iter()
            .filter(|d| self.owner(d) == Some(k))
            .collect()
    }

    /// The process traveller, in Markdown: the design and its layers from the top down, then
    /// each step in order (what it does, why, the process, the capabilities it relies on with
    /// their sources, the alternatives, what isn't certain, the rules' checks, the dimensions it
    /// makes and how accurately, and the wafer after it), the accuracy of every dimension, the
    /// warnings, what is not specified, the round trip and the references.
    pub fn markdown(&self) -> String {
        let flow = &self.flow;
        let n = flow.steps.len();
        let mut md = String::new();
        let _ = writeln!(md, "# Process traveller: {}\n", self.device);
        let _ = writeln!(
            md,
            "A flow that makes this design from published processes, every number with its \
             source, from photonoxide's process library (version {}). It is one sound way to \
             make the design, to learn and plan from, alongside a foundry's own process and \
             design rules.\n",
            flow.library
        );
        let _ = writeln!(md, "## The design\n");
        let _ = writeln!(md, "| Parameter | Value | Unit |\n|---|---|---|");
        for p in &self.parameters {
            let _ = writeln!(md, "| {} | {} | {} |", p.name, number(p.value), p.unit);
        }
        let _ = writeln!(md, "\n{LAYERS_TOP_DOWN}\n");
        let _ = writeln!(md, "| Layer | Film | Thickness (µm) |\n|---|---|---|");
        if let Some(last) = self.stacks.last() {
            for row in last {
                let film = match &row.elsewhere {
                    Some(e) => format!("{}, {e} where etched", row.film),
                    None => row.film.clone(),
                };
                let thickness = match (row.thickness_um, row.name.as_str()) {
                    (Some(t), _) => number(t),
                    (None, "cladding") => "above everything".into(),
                    (None, _) => "below everything".into(),
                };
                let _ = writeln!(md, "| {} | {film} | {thickness} |", row.name);
            }
        }
        let _ = writeln!(md, "\n## The steps\n");
        for (k, s) in flow.steps.iter().enumerate() {
            let _ = writeln!(md, "### Step {} of {n}: {}\n", k + 1, s.name);
            let _ = writeln!(md, "- **What:** {}.", s.does);
            let _ = writeln!(md, "- **Why:** {}", s.reason);
            let _ = writeln!(
                md,
                "- **Process:** {} (`{}`, {}): {}",
                s.name,
                s.process,
                s.step.name(),
                s.plain
            );
            if !s.capabilities.is_empty() {
                let _ = writeln!(md, "- **Cited capabilities:**");
                for c in &s.capabilities {
                    let source = c
                        .value
                        .source()
                        .map_or_else(|| "no source in hand".into(), |src| self.cite(src));
                    let conditions = if c.conditions.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", c.conditions)
                    };
                    let _ = writeln!(
                        md,
                        "  - {}: {}{conditions}. Source: {source}.",
                        c.quantity.name(),
                        c.value
                    );
                }
            }
            if !s.facts.is_empty() {
                let _ = writeln!(md, "- **What the sources say:**");
                for f in &s.facts {
                    let _ = writeln!(md, "  - {} Source: {}.", f.text, self.cite(&f.source));
                }
            }
            if !s.alternatives.is_empty() {
                let _ = writeln!(md, "- **Alternatives:**");
                for a in &s.alternatives {
                    let _ = writeln!(md, "  - {} (`{}`): {}", a.name, a.process, a.why_not);
                }
            }
            if !s.uncertainties.is_empty() {
                let _ = writeln!(md, "- **Not certain:**");
                for u in &s.uncertainties {
                    let _ = writeln!(md, "  - {u}");
                }
            }
            let checks: Vec<_> = self
                .assessment
                .checks
                .iter()
                .filter(|c| c.step == Some(k))
                .collect();
            if !checks.is_empty() {
                let _ = writeln!(md, "- **The rules' checks:**");
                for c in checks {
                    let verdict = match c.verdict {
                        Verdict::Within => "within",
                        Verdict::Warning => "warning",
                        _ => "not specified",
                    };
                    let _ = writeln!(md, "  - {verdict}: {}", c.text);
                }
            }
            let results = self.results(k);
            if !results.is_empty() {
                let _ = writeln!(md, "- **Expected result:**");
                for d in results {
                    let _ = writeln!(md, "  - {}", d.text);
                }
            }
            let _ = writeln!(
                md,
                "- **After it, from the top down:** {}.",
                self.stacks[k]
                    .iter()
                    .map(describe)
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            let _ = writeln!(
                md,
                "- **Cross-section:** [{0}]({0})\n",
                FlowExport::svg_name(k, n)
            );
        }
        let _ = writeln!(md, "## The accuracy of each dimension\n");
        let _ = writeln!(md, "{}\n", self.assessment.combination);
        let _ = writeln!(
            md,
            "| Dimension | Layer | Design value | Made to ± | At worst ± | Not specified |\n\
             |---|---|---|---|---|---|"
        );
        for d in &self.assessment.accuracy {
            let with = |v: Option<f64>| v.map_or_else(String::new, |v| with_unit(v, d.unit));
            let missing: Vec<&str> = d
                .contributions
                .iter()
                .filter(|c| c.bound.is_none())
                .map(|c| c.what.as_str())
                .collect();
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} |",
                d.what,
                d.layer,
                with_unit(d.nominal, d.unit),
                with(d.combined),
                with(d.worst),
                if missing.is_empty() {
                    "none".into()
                } else {
                    missing.join("; ")
                }
            );
        }
        let _ = writeln!(md);
        for d in &self.assessment.accuracy {
            let _ = writeln!(md, "- {}", d.text);
            for c in &d.contributions {
                let bound = c.bound.map_or_else(
                    || "not specified".into(),
                    |b| format!("± {}", with_unit(b, d.unit)),
                );
                let source = c
                    .source
                    .as_ref()
                    .map_or_else(String::new, |s| format!(" Source: {}.", self.cite(s)));
                let _ = writeln!(md, "  - {}: {bound} ({}).{source}", c.what, c.conditions);
            }
        }
        if !flow.warnings.is_empty() {
            let _ = writeln!(md, "\n## Warnings\n");
            for w in &flow.warnings {
                let _ = writeln!(md, "- {w}");
            }
        }
        if !flow.not_specified.is_empty() {
            let _ = writeln!(md, "\n## Not specified by the sources in hand\n");
            for w in &flow.not_specified {
                let _ = writeln!(md, "- {w}");
            }
        }
        if let Some(t) = flow.round_trip {
            let _ = writeln!(md, "\n## The round trip\n");
            let _ = writeln!(
                md,
                "The flow, run on the library's own geometry, {} the design: the structures {}, \
                 the films {}, {} of the {} points compared differing.",
                if t.exact() {
                    "gives back"
                } else {
                    "does not give back"
                },
                if t.structure_equal { "equal" } else { "differ" },
                if t.films_equal {
                    "the same"
                } else {
                    "different"
                },
                t.differing,
                t.points
            );
        }
        let _ = writeln!(md, "\n## References\n");
        for r in &flow.references {
            let id = if r.doi.is_empty() {
                format!("ISBN {}", r.isbn)
            } else {
                format!("[doi:{0}](https://doi.org/{0})", r.doi)
            };
            let _ = writeln!(md, "- {}, \"{}\", {id}", r.citation, r.title);
        }
        md
    }
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests;
