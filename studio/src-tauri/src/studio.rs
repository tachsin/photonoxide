//! The studio window: a web view (studio/src, three.js) on a run's record.
//!
//! The window follows the run's `events.jsonl` as it grows, so a live run and a replay are the
//! same code path, and what the window shows is exactly what the record holds. Live, it closes
//! itself a little after the run finishes; closing it early asks the run to stop.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use photonoxide::job::Event;
use photonoxide::run::Stop;
use photonoxide::units::Wavelength;
use serde::Serialize;

/// A live run: the window closes `linger` after `finished` fires, and asks `stop` when it is
/// closed first.
pub struct Live {
    pub linger: Duration,
    pub finished: Receiver<()>,
    pub stop: Stop,
}

/// Opens the window on the run in `dir` and returns when it closes.
pub fn show(dir: &Path, live: Option<Live>) -> Result<(), String> {
    let name = dir.file_name().map_or_else(
        || dir.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let state = Studio {
        dir: dir.to_path_buf(),
        live: live.as_ref().map(|l| l.linger.as_secs_f64()),
        record: Mutex::new(Record::new(dir.join("events.jsonl"))),
    };
    let app = tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![info, poll, group_index])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "studio",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(format!("photonoxide studio: {name}"))
            .inner_size(1280.0, 840.0)
            .min_inner_size(720.0, 480.0)
            .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| format!("can't open the studio window: {e}"))?;
    let stop = live.as_ref().map(|l| l.stop.clone());
    if let Some(Live {
        linger, finished, ..
    }) = live
    {
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            if finished.recv().is_ok() {
                std::thread::sleep(linger);
                handle.exit(0);
            }
        });
    }
    app.run_return(move |_, event| {
        // closed before the run finished: ask it to stop
        if let (tauri::RunEvent::ExitRequested { .. }, Some(stop)) = (&event, &stop) {
            stop.request();
        }
    });
    Ok(())
}

/// The window's state: the run and its record so far.
struct Studio {
    dir: PathBuf,
    /// For a live run, how long the window stays after it, seconds.
    live: Option<f64>,
    record: Mutex<Record>,
}

/// What the window is showing.
#[derive(Serialize)]
struct Info {
    dir: String,
    name: String,
    live: bool,
    linger_s: f64,
}

#[tauri::command]
fn info(state: tauri::State<'_, Studio>) -> Info {
    Info {
        dir: state.dir.display().to_string(),
        name: state
            .dir
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        live: state.live.is_some(),
        linger_s: state.live.unwrap_or(0.0),
    }
}

/// The events from the `from`-th on, and the first line of the record that isn't one.
#[derive(Serialize)]
struct Poll {
    events: Vec<Event>,
    problem: Option<String>,
}

#[tauri::command]
fn poll(state: tauri::State<'_, Studio>, from: usize) -> Poll {
    let mut record = state
        .record
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    record.refresh();
    Poll {
        events: record.events.get(from..).unwrap_or_default().to_vec(),
        problem: record.problem.clone(),
    }
}

/// Group indices n_g = n − λ dn/dλ of a mode's effective indices `n` at increasing
/// `wavelengths_um`, by [`photonoxide::mode::dispersion::group_index`].
#[tauri::command]
fn group_index(wavelengths_um: Vec<f64>, n: Vec<f64>) -> Result<Vec<f64>, String> {
    let w = wavelengths_um
        .iter()
        .map(|&l| Wavelength::um(l))
        .collect::<photonoxide::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    photonoxide::mode::dispersion::group_index(&w, &n).map_err(|e| e.to_string())
}

/// A run's record as it grows: the complete lines read so far, as events.
struct Record {
    path: PathBuf,
    offset: u64,
    partial: String,
    events: Vec<Event>,
    problem: Option<String>,
}

impl Record {
    fn new(path: PathBuf) -> Record {
        Record {
            path,
            offset: 0,
            partial: String::new(),
            events: Vec::new(),
            problem: None,
        }
    }

    /// Reads the lines appended since the last call.
    fn refresh(&mut self) {
        let mut text = String::new();
        let read = File::open(&self.path).and_then(|mut f| {
            f.seek(SeekFrom::Start(self.offset))?;
            f.read_to_string(&mut text)
        });
        let Ok(n) = read else {
            return; // not created yet
        };
        self.offset += n as u64;
        self.partial.push_str(&text);
        while let Some(end) = self.partial.find('\n') {
            let line: String = self.partial.drain(..=end).collect();
            match serde_json::from_str::<Event>(line.trim_end()) {
                Ok(e) => self.events.push(e),
                Err(e) => {
                    self.problem = self
                        .problem
                        .take()
                        .or(Some(format!("a line of the record isn't an event: {e}")));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_reads_complete_lines_as_they_arrive() {
        let dir = std::env::temp_dir().join(format!("photonoxide-record-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        let mut record = Record::new(path.clone());
        record.refresh();
        assert!(record.events.is_empty()); // no file yet
        std::fs::write(
            &path,
            "{\"type\":\"started\",\"job\":\"a\",\"kind\":\"structure\"}\n{\"type\":\"fin",
        )
        .unwrap();
        record.refresh();
        assert_eq!(record.events.len(), 1);
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(b"ished\",\"stopped\":null,\"seconds\":0.5}\nnot an event\n")
            .unwrap();
        record.refresh();
        assert_eq!(record.events.len(), 2);
        assert!(matches!(record.events[1], Event::Finished { .. }));
        assert!(record.problem.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn group_indices_come_from_the_library() {
        // n = 2.5 − 0.6 (λ − 1.55) gives n_g = n − λ dn/dλ = 2.5 + 0.6 · 1.55 at 1.55 µm
        let w = vec![1.5, 1.55, 1.6];
        let n: Vec<f64> = w.iter().map(|l| 2.5 - 0.6 * (l - 1.55)).collect();
        let ng = group_index(w, n).unwrap();
        assert!((ng[1] - (2.5 + 0.6 * 1.55)).abs() < 1e-12, "{ng:?}");
        assert!(group_index(vec![1.5], vec![2.0]).is_err());
    }
}
