//! The studio: the native window that shows a run as it happens, or replays a finished one.
//!
//! It shows what the run records: pictures of the permittivity; the modes of a cross-section,
//! each as a picture of |E|² with its effective index; and a sweep's effective indices (and,
//! for a wavelength sweep, group indices) as plots that grow as the points arrive.
//!
//! The window follows the run's `events.jsonl` as it grows, so a live run and a replay are the
//! same code path, and what the window shows is exactly what the record holds. Live, it closes
//! itself a little after the run finishes; closing it early asks the run to stop.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::job::Event;
use crate::raster::Raster;
use crate::run::Stop;
use crate::{Error, Result};

/// How the window behaves.
#[derive(Clone, Debug)]
pub struct Options {
    /// The run is in progress: close the window `linger` after it finishes.
    pub live: bool,
    /// How long a live window stays open after its run finished.
    pub linger: Duration,
    /// Told to stop when the window is closed before the run finished.
    pub stop: Option<Stop>,
}

/// Opens the window on the run in `dir` and returns when it closes.
///
/// # Errors
///
/// [`Error::Io`] if the window can't be opened (no display, no graphics driver).
pub fn show(dir: &Path, options: Options) -> Result<()> {
    let title = format!(
        "photonoxide studio: {}",
        dir.file_name().map_or_else(
            || dir.display().to_string(),
            |n| n.to_string_lossy().into_owned()
        )
    );
    let viewport = egui::ViewportBuilder::default()
        .with_title(&title)
        .with_inner_size([1100.0, 820.0]);
    let native = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let app = App::new(dir.to_path_buf(), options);
    eframe::run_native(&title, native, Box::new(|_| Ok(Box::new(app)))).map_err(|e| Error::Io {
        path: dir.display().to_string(),
        reason: format!("can't open the studio window: {e}"),
    })
}

/// Reads the complete lines appended to a file since the last call.
struct Tail {
    path: PathBuf,
    offset: u64,
    partial: String,
}

impl Tail {
    fn new(path: PathBuf) -> Tail {
        Tail {
            path,
            offset: 0,
            partial: String::new(),
        }
    }

    /// The new events, and the first line that isn't one, if any.
    fn poll(&mut self) -> (Vec<Event>, Option<String>) {
        let mut text = String::new();
        let read = File::open(&self.path).and_then(|mut f| {
            f.seek(SeekFrom::Start(self.offset))?;
            f.read_to_string(&mut text)
        });
        let Ok(n) = read else {
            return (Vec::new(), None); // not created yet
        };
        self.offset += n as u64;
        self.partial.push_str(&text);
        let mut events = Vec::new();
        let mut bad = None;
        while let Some(end) = self.partial.find('\n') {
            let line: String = self.partial.drain(..=end).collect();
            match serde_json::from_str::<Event>(line.trim_end()) {
                Ok(e) => events.push(e),
                Err(e) => bad = bad.or(Some(e.to_string())),
            }
        }
        (events, bad)
    }
}

/// A picture of a raster, with what it shows.
struct Picture {
    view: String,
    axes: [String; 2],
    wavelength_um: f64,
    raster: Raster,
    texture: Option<egui::TextureHandle>,
}

/// A mode, with its picture.
struct ModeView {
    label: String,
    wavelength_um: f64,
    effective_index: [f64; 2],
    te_fraction: f64,
    intensity: Raster,
    texture: Option<egui::TextureHandle>,
}

/// One point of a sweep.
struct SweepPoint {
    value: f64,
    effective_indices: Vec<[f64; 2]>,
}

struct App {
    tail: Tail,
    options: Options,
    job: Option<(String, String)>,
    pictures: Vec<Picture>,
    modes: Vec<ModeView>,
    sweep: Option<(String, Vec<SweepPoint>)>,
    finished: Option<(Instant, Option<String>, f64)>,
    problem: Option<String>,
    opened: Instant,
}

impl App {
    fn new(dir: PathBuf, options: Options) -> App {
        App {
            tail: Tail::new(dir.join("events.jsonl")),
            options,
            job: None,
            pictures: Vec::new(),
            modes: Vec::new(),
            sweep: None,
            finished: None,
            problem: None,
            opened: Instant::now(),
        }
    }

    fn take(&mut self, event: Event) {
        match event {
            Event::Started { job, kind } => self.job = Some((job, kind)),
            Event::Permittivity {
                view,
                axes,
                wavelength_um,
                raster,
            } => self.pictures.push(Picture {
                view,
                axes,
                wavelength_um,
                raster,
                texture: None,
            }),
            Event::Mode {
                label,
                wavelength_um,
                effective_index,
                te_fraction,
                intensity,
            } => self.modes.push(ModeView {
                label,
                wavelength_um,
                effective_index,
                te_fraction,
                intensity,
                texture: None,
            }),
            Event::SweepPoint {
                parameter,
                value,
                effective_indices,
                ..
            } => self
                .sweep
                .get_or_insert_with(|| (parameter, Vec::new()))
                .1
                .push(SweepPoint {
                    value,
                    effective_indices,
                }),
            Event::Finished { stopped, seconds } => {
                self.finished = Some((Instant::now(), stopped, seconds));
            }
        }
    }
}

/// The colour of a permittivity: light for low (oxide), dark blue for high (silicon), a
/// sequential ramp through three stops.
fn colour(t: f32) -> [u8; 3] {
    const STOPS: [[f32; 3]; 3] = [
        [247.0, 251.0, 255.0],
        [107.0, 174.0, 214.0],
        [8.0, 48.0, 107.0],
    ];
    let t = t.clamp(0.0, 1.0) * 2.0;
    let (a, b, f) = if t < 1.0 {
        (STOPS[0], STOPS[1], t)
    } else {
        (STOPS[1], STOPS[2], t - 1.0)
    };
    [0, 1, 2].map(|c| (a[c] + (b[c] - a[c]) * f).round() as u8)
}

/// The colour of a field's intensity: black at zero through purple and orange to pale yellow at
/// its peak, a perceptually ordered ramp that reads on a light or a dark background.
fn intensity_colour(t: f32) -> [u8; 3] {
    const STOPS: [[f32; 3]; 4] = [
        [0.0, 0.0, 4.0],
        [120.0, 28.0, 109.0],
        [237.0, 105.0, 37.0],
        [252.0, 255.0, 164.0],
    ];
    let t = t.clamp(0.0, 1.0) * 3.0;
    let k = (t.floor() as usize).min(2);
    let (a, b, f) = (STOPS[k], STOPS[k + 1], t - k as f32);
    [0, 1, 2].map(|c| (a[c] + (b[c] - a[c]) * f).round() as u8)
}

/// A raster as an image, its rows top first, coloured by `ramp` from its smallest value to
/// its largest.
fn image_with(raster: &Raster, ramp: fn(f32) -> [u8; 3]) -> egui::ColorImage {
    let (lo, hi) = raster.range();
    let span = (hi - lo).max(f32::EPSILON);
    let mut rgb = Vec::with_capacity(raster.nx * raster.ny * 3);
    for j in (0..raster.ny).rev() {
        for i in 0..raster.nx {
            rgb.extend(ramp((raster.at(i, j) - lo) / span));
        }
    }
    egui::ColorImage::from_rgb([raster.nx, raster.ny], &rgb)
}

/// The series of a sweep, one per mode (by rank): (value, effective index) points.
fn sweep_series(points: &[SweepPoint]) -> Vec<Vec<(f64, f64)>> {
    let count = points
        .iter()
        .map(|p| p.effective_indices.len())
        .max()
        .unwrap_or(0);
    (0..count)
        .map(|m| {
            points
                .iter()
                .filter_map(|p| p.effective_indices.get(m).map(|n| (p.value, n[0])))
                .collect()
        })
        .collect()
}

/// For a wavelength sweep, each series' group index n_g = n − λ dn/dλ
/// ([`crate::mode::dispersion::group_index`]), once it has 3 or more increasing wavelengths.
fn group_series(series: &[Vec<(f64, f64)>]) -> Vec<Vec<(f64, f64)>> {
    series
        .iter()
        .filter_map(|s| {
            let mut s = s.clone();
            s.sort_by(|a, b| a.0.total_cmp(&b.0));
            s.dedup_by(|a, b| a.0 == b.0);
            let w: Vec<crate::units::Wavelength> = s
                .iter()
                .map(|p| crate::units::Wavelength::um(p.0))
                .collect::<Result<_>>()
                .ok()?;
            let n: Vec<f64> = s.iter().map(|p| p.1).collect();
            let ng = crate::mode::dispersion::group_index(&w, &n).ok()?;
            Some(s.iter().zip(ng).map(|(p, g)| (p.0, g)).collect())
        })
        .collect()
}

/// The range a plot's axis covers: the data's, with a margin, and never empty.
fn padded(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let (lo, hi) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    if !lo.is_finite() {
        return (0.0, 1.0);
    }
    let span = (hi - lo).max(1e-9 * hi.abs().max(1.0));
    (lo - 0.05 * span, hi + 0.05 * span)
}

/// The series' colours: a categorical set, distinct on light and dark backgrounds.
const SERIES: [egui::Color32; 4] = [
    egui::Color32::from_rgb(31, 119, 180),
    egui::Color32::from_rgb(230, 85, 13),
    egui::Color32::from_rgb(44, 160, 44),
    egui::Color32::from_rgb(148, 103, 189),
];

/// A line plot of `series` (each its points, in order), with its axis ranges marked.
fn plot(ui: &mut egui::Ui, title: &str, x_label: &str, y_label: &str, series: &[Vec<(f64, f64)>]) {
    ui.strong(title);
    let (x0, x1) = padded(series.iter().flatten().map(|p| p.0));
    let (y0, y1) = padded(series.iter().flatten().map(|p| p.1));
    let size = egui::vec2(ui.available_width().min(640.0), 220.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let frame = rect
        .shrink2(egui::vec2(56.0, 22.0))
        .translate(egui::vec2(24.0, -6.0));
    let painter = ui.painter_at(rect);
    let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
    painter.rect_stroke(
        frame,
        0.0,
        egui::Stroke::new(1.0, stroke.color.gamma_multiply(0.4)),
        egui::StrokeKind::Inside,
    );
    let at = |x: f64, y: f64| {
        egui::pos2(
            frame.left() + ((x - x0) / (x1 - x0)) as f32 * frame.width(),
            frame.bottom() - ((y - y0) / (y1 - y0)) as f32 * frame.height(),
        )
    };
    for (k, s) in series.iter().enumerate() {
        let colour = SERIES[k % SERIES.len()];
        let points: Vec<egui::Pos2> = s.iter().map(|&(x, y)| at(x, y)).collect();
        if points.len() > 1 {
            painter.add(egui::Shape::line(
                points.clone(),
                egui::Stroke::new(2.0, colour),
            ));
        }
        for p in points {
            painter.circle_filled(p, 3.0, colour);
        }
    }
    let font = egui::FontId::proportional(11.0);
    let text = stroke.color;
    painter.text(
        egui::pos2(frame.left(), frame.bottom() + 4.0),
        egui::Align2::LEFT_TOP,
        format!("{x0:.4}"),
        font.clone(),
        text,
    );
    painter.text(
        egui::pos2(frame.right(), frame.bottom() + 4.0),
        egui::Align2::RIGHT_TOP,
        format!("{x1:.4}"),
        font.clone(),
        text,
    );
    painter.text(
        egui::pos2(frame.center().x, frame.bottom() + 4.0),
        egui::Align2::CENTER_TOP,
        x_label,
        font.clone(),
        text,
    );
    painter.text(
        egui::pos2(frame.left() - 4.0, frame.bottom()),
        egui::Align2::RIGHT_BOTTOM,
        format!("{y0:.4}"),
        font.clone(),
        text,
    );
    painter.text(
        egui::pos2(frame.left() - 4.0, frame.top()),
        egui::Align2::RIGHT_TOP,
        format!("{y1:.4}"),
        font.clone(),
        text,
    );
    painter.text(
        egui::pos2(frame.left() - 4.0, frame.center().y),
        egui::Align2::RIGHT_CENTER,
        y_label,
        font,
        text,
    );
    ui.horizontal(|ui| {
        for k in 0..series.len() {
            ui.colored_label(SERIES[k % SERIES.len()], format!("● mode {}", k + 1));
        }
    });
}

fn image(raster: &Raster) -> egui::ColorImage {
    let (lo, hi) = raster.range();
    let span = (hi - lo).max(f32::EPSILON);
    let mut rgb = Vec::with_capacity(raster.nx * raster.ny * 3);
    // image rows run top to bottom, the raster's from y0 up
    for j in (0..raster.ny).rev() {
        for i in 0..raster.nx {
            rgb.extend(colour((raster.at(i, j) - lo) / span));
        }
    }
    egui::ColorImage::from_rgb([raster.nx, raster.ny], &rgb)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let (events, bad) = self.tail.poll();
        for e in events {
            self.take(e);
        }
        if let Some(bad) = bad {
            self.problem = Some(format!("a line of the record isn't an event: {bad}"));
        }
        if ctx.input(|i| i.viewport().close_requested())
            && self.finished.is_none()
            && let Some(stop) = &self.options.stop
        {
            stop.request();
        }
        if self.options.live
            && let Some((at, _, _)) = self.finished
            && at.elapsed() >= self.options.linger
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::Panel::top("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let name = self
                    .job
                    .as_ref()
                    .map_or("waiting for the run...".to_owned(), |(job, kind)| {
                        format!("{job} ({kind})")
                    });
                ui.strong(name);
                ui.separator();
                match &self.finished {
                    None => ui.label(format!(
                        "running · {:.0} s",
                        self.opened.elapsed().as_secs_f64()
                    )),
                    Some((_, None, s)) => ui.label(format!("finished in {s:.2} s")),
                    Some((_, Some(why), s)) => ui.label(format!("stopped ({why}) after {s:.2} s")),
                };
                if self.options.live && self.finished.is_some() {
                    ui.separator();
                    ui.weak("closes by itself");
                }
            });
            if let Some(p) = &self.problem {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 40), p);
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.pictures.is_empty() && self.modes.is_empty() {
                    ui.weak("no pictures yet");
                }
                for (k, p) in self.pictures.iter_mut().enumerate() {
                    let texture = p.texture.get_or_insert_with(|| {
                        ctx.load_texture(
                            format!("picture-{k}"),
                            image(&p.raster),
                            egui::TextureOptions::NEAREST,
                        )
                    });
                    let r = &p.raster;
                    let (lo, hi) = r.range();
                    ui.add_space(8.0);
                    ui.strong(format!("{} · Re ε at {} µm", p.view, p.wavelength_um));
                    ui.label(format!(
                        "{} from {} to {} µm, {} from {} to {} µm · ε from {lo:.3} (light) to {hi:.3} (dark)",
                        p.axes[0], r.x0, r.x1, p.axes[1], r.y0, r.y1
                    ));
                    let aspect = ((r.x1 - r.x0) / (r.y1 - r.y0)) as f32;
                    let width = ui.available_width().min(1000.0);
                    let size = egui::vec2(width, width / aspect.max(1e-3));
                    ui.image((texture.id(), size));
                }
                if !self.modes.is_empty() {
                    ui.add_space(12.0);
                    ui.heading("Modes");
                    ui.label("|E|² from zero (black) to its peak (pale yellow)");
                }
                for (k, m) in self.modes.iter_mut().enumerate() {
                    let texture = m.texture.get_or_insert_with(|| {
                        ctx.load_texture(format!("mode-{k}"), image_with(&m.intensity, intensity_colour), egui::TextureOptions::LINEAR)
                    });
                    let r = &m.intensity;
                    let kind = if m.te_fraction > 0.5 { "TE-like" } else { "TM-like" };
                    ui.add_space(8.0);
                    ui.strong(format!(
                        "{} · {kind} (TE fraction {:.3}) · n_eff = {:.6}{} at {} µm",
                        m.label,
                        m.te_fraction,
                        m.effective_index[0],
                        if m.effective_index[1].abs() > 0.0 { format!(" + {:.3e}i", m.effective_index[1]) } else { String::new() },
                        m.wavelength_um
                    ));
                    let aspect = ((r.x1 - r.x0) / (r.y1 - r.y0)) as f32;
                    let width = ui.available_width().min(700.0);
                    ui.image((texture.id(), egui::vec2(width, width / aspect.max(1e-3))));
                }
                if let Some((parameter, points)) = &self.sweep {
                    ui.add_space(12.0);
                    let unit = if parameter == "wavelength" { "wavelength (µm)" } else { "width (µm)" };
                    let series = sweep_series(points);
                    ui.heading(format!("Sweep over the {parameter}: {} points", points.len()));
                    plot(ui, "effective index", unit, "n_eff", &series);
                    if parameter == "wavelength" {
                        let groups = group_series(&series);
                        if !groups.is_empty() {
                            ui.add_space(8.0);
                            plot(ui, "group index, n − λ dn/dλ", unit, "n_g", &groups);
                        }
                    }
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_ramp_runs_from_light_to_dark() {
        assert_eq!(colour(0.0), [247, 251, 255]);
        assert_eq!(colour(0.5), [107, 174, 214]);
        assert_eq!(colour(1.0), [8, 48, 107]);
        assert_eq!(colour(-1.0), colour(0.0));
    }

    #[test]
    fn the_image_puts_the_top_of_the_raster_first() {
        let r = Raster {
            nx: 1,
            ny: 2,
            x0: 0.0,
            x1: 1.0,
            y0: 0.0,
            y1: 2.0,
            values: vec![1.0, 12.0],
        };
        let img = image(&r);
        // the high value (the raster's top row) is the image's first, dark row
        assert_eq!(img.pixels[0], egui::Color32::from_rgb(8, 48, 107));
        assert_eq!(img.pixels[1], egui::Color32::from_rgb(247, 251, 255));
    }

    #[test]
    fn the_intensity_ramp_runs_from_black_to_pale_yellow() {
        assert_eq!(intensity_colour(0.0), [0, 0, 4]);
        assert_eq!(intensity_colour(1.0), [252, 255, 164]);
        assert_eq!(intensity_colour(2.0), intensity_colour(1.0));
        // brighter as it rises
        let lum = |c: [u8; 3]| {
            0.2126 * f32::from(c[0]) + 0.7152 * f32::from(c[1]) + 0.0722 * f32::from(c[2])
        };
        assert!((0..10).all(|k| lum(intensity_colour(k as f32 / 10.0))
            < lum(intensity_colour((k + 1) as f32 / 10.0))));
    }

    #[test]
    fn a_wavelength_sweep_gives_each_modes_group_index() {
        // n = 2.5 − 0.6 (λ − 1.55) gives n_g = n − λ dn/dλ = 2.5 + 0.6 · 1.55 at 1.55 µm
        let points: Vec<SweepPoint> = [1.5, 1.55, 1.6]
            .iter()
            .map(|&l| SweepPoint {
                value: l,
                effective_indices: vec![[2.5 - 0.6 * (l - 1.55), 0.0], [1.9, 0.0]],
            })
            .collect();
        let series = sweep_series(&points);
        assert_eq!(series.len(), 2);
        let groups = group_series(&series);
        assert!(
            (groups[0][1].1 - (2.5 + 0.6 * 1.55)).abs() < 1e-12,
            "{:?}",
            groups[0]
        );
        assert!((groups[1][1].1 - 1.9).abs() < 1e-12);
        // too few points for a group index
        assert!(group_series(&sweep_series(&points[..2])).is_empty());
    }

    #[test]
    fn a_plots_range_has_a_margin_and_is_never_empty() {
        assert_eq!(padded([1.0, 3.0].into_iter()), (0.9, 3.1));
        let (lo, hi) = padded([2.0, 2.0].into_iter());
        assert!(lo < 2.0 && hi > 2.0);
        assert_eq!(padded(std::iter::empty()), (0.0, 1.0));
    }

    #[test]
    fn the_tail_reads_complete_lines_as_they_arrive() {
        let dir = std::env::temp_dir().join(format!("photonoxide-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        let mut tail = Tail::new(path.clone());
        assert!(tail.poll().0.is_empty()); // no file yet
        std::fs::write(
            &path,
            "{\"type\":\"started\",\"job\":\"a\",\"kind\":\"structure\"}\n{\"type\":\"fin",
        )
        .unwrap();
        assert_eq!(tail.poll().0.len(), 1);
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(b"ished\",\"stopped\":null,\"seconds\":0.5}\n")
            .unwrap();
        let (events, bad) = tail.poll();
        assert_eq!(events.len(), 1);
        assert!(bad.is_none());
        assert!(matches!(events[0], Event::Finished { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
