//! The studio: the native window that shows a run as it happens, or replays a finished one.
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

struct App {
    tail: Tail,
    options: Options,
    job: Option<(String, String)>,
    pictures: Vec<Picture>,
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        egui::TopBottomPanel::top("status").show(ctx, |ui| {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.pictures.is_empty() {
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
