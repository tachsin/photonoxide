//! The studio window: a web view (studio/src, three.js) on a run's record.
//!
//! The window follows the run's `events.jsonl` as it grows, so a live run and a replay are the
//! same code path, and what the window shows is exactly what the record holds. Started on a run
//! from the command line, it closes itself a little after the run finishes, and closing it early
//! asks the run to stop. Started bare, it opens on its start page, where jobs are run and runs
//! reopened; jobs started there keep the window open.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::Duration;

use photonoxide::job::{self, Event};
use photonoxide::run::{Job, Meta, Run, Stop};
use photonoxide::units::Wavelength;
use serde::Serialize;
use tauri::Manager;

/// A run started from the command line: the window closes `linger` after `finished` fires, and
/// asks `stop` when it is closed first.
pub struct Live {
    pub linger: Duration,
    pub finished: Receiver<()>,
    pub stop: Stop,
}

/// Opens the window, on the run in `dir` or on the start page, and returns when it closes.
pub fn show(dir: Option<&Path>, live: Option<Live>) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    let title = dir.map_or_else(|| "photonoxide studio".to_owned(), window_title);
    let state = Studio {
        root,
        current: Mutex::new(Current {
            generation: 1,
            dir: dir.map(Path::to_path_buf),
            closes: live.is_some(),
            record: dir.map(|d| Record::new(d.join("events.jsonl"))),
        }),
        started: Mutex::new(Vec::new()),
    };
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            info,
            poll,
            home,
            open_run,
            run_job,
            group_index
        ])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "studio",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(title)
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
    let handle = app.handle().clone();
    app.run_return(move |_, event| {
        // closed before the run finished: ask it to stop
        if let (tauri::RunEvent::ExitRequested { .. }, Some(stop)) = (&event, &stop) {
            stop.request();
        }
    });
    // the jobs started from the window stop at their next check, and finish their records
    let started = std::mem::take(&mut *lock(&handle.state::<Studio>().started));
    for (stop, _) in &started {
        stop.request();
    }
    for (_, worker) in started {
        let _ = worker.join();
    }
    Ok(())
}

fn window_title(dir: &Path) -> String {
    format!("photonoxide studio: {}", name_of(dir))
}

fn name_of(dir: &Path) -> String {
    dir.file_name().map_or_else(
        || dir.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The window's state.
struct Studio {
    /// Where the start page looks for `jobs/` and `runs/`, and where new runs go.
    root: PathBuf,
    current: Mutex<Current>,
    /// The jobs started from the window, still running or finished.
    started: Mutex<Vec<(Stop, JoinHandle<()>)>>,
}

/// The run the window shows, if any.
struct Current {
    /// Changes whenever the run does, so the web view knows to start over.
    generation: u64,
    dir: Option<PathBuf>,
    /// A run from the command line: the window closes after it.
    closes: bool,
    record: Option<Record>,
}

/// What the window is showing.
#[derive(Serialize)]
struct Info {
    generation: u64,
    /// The run's directory and name, or none for the start page.
    dir: Option<String>,
    name: Option<String>,
    closes: bool,
    root: String,
}

fn info_of(studio: &Studio) -> Info {
    let current = lock(&studio.current);
    Info {
        generation: current.generation,
        dir: current.dir.as_ref().map(|d| d.display().to_string()),
        name: current.dir.as_deref().map(name_of),
        closes: current.closes,
        root: studio.root.display().to_string(),
    }
}

#[tauri::command]
fn info(state: tauri::State<'_, Studio>) -> Info {
    info_of(&state)
}

/// The current run's events from the `from`-th on, and the first line of its record that isn't
/// one.
#[derive(Serialize)]
struct Poll {
    generation: u64,
    events: Vec<Event>,
    problem: Option<String>,
}

#[tauri::command]
fn poll(state: tauri::State<'_, Studio>, from: usize) -> Poll {
    let mut current = lock(&state.current);
    let generation = current.generation;
    match current.record.as_mut() {
        Some(record) => {
            record.refresh();
            Poll {
                generation,
                events: record.events.get(from..).unwrap_or_default().to_vec(),
                problem: record.problem.clone(),
            }
        }
        None => Poll {
            generation,
            events: Vec::new(),
            problem: None,
        },
    }
}

/// A job file the start page offers.
#[derive(Serialize)]
struct JobItem {
    path: String,
    name: String,
    /// The task's kind, e.g. `"modes"`.
    kind: String,
    /// The job file's opening comment.
    about: String,
}

/// A run the start page offers.
#[derive(Serialize)]
struct RunItem {
    dir: String,
    name: String,
    job: String,
    /// When it started, UTC.
    started: String,
    /// Its record ends with the run finishing.
    finished: bool,
}

#[derive(Serialize)]
struct Home {
    root: String,
    jobs: Vec<JobItem>,
    runs: Vec<RunItem>,
}

/// The jobs in `jobs/` and the 40 newest runs in `runs/`, under the working directory.
#[tauri::command]
fn home(state: tauri::State<'_, Studio>) -> Home {
    Home {
        root: state.root.display().to_string(),
        jobs: jobs_in(&state.root.join("jobs")),
        runs: runs_in(&state.root.join("runs"), 40),
    }
}

fn jobs_in(dir: &Path) -> Vec<JobItem> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let job = Job::parse(&text).ok()?;
            let kind = job
                .task()
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_owned();
            Some(JobItem {
                path: path.display().to_string(),
                name: job.name().to_owned(),
                kind,
                about: about(&text),
            })
        })
        .collect()
}

/// A job file's opening comment, as one paragraph, without its usage lines.
fn about(text: &str) -> String {
    text.lines()
        .map_while(|l| l.strip_prefix('#'))
        .map(str::trim)
        .take_while(|l| !l.is_empty() && !l.starts_with("photonoxide "))
        .collect::<Vec<_>>()
        .join(" ")
}

fn runs_in(dir: &Path, most: usize) -> Vec<RunItem> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("events.jsonl").is_file())
        .collect();
    // run directories are named by their start time: newest first
    dirs.sort();
    dirs.reverse();
    dirs.truncate(most);
    dirs.into_iter()
        .map(|d| {
            let meta: Option<Meta> = std::fs::read_to_string(d.join("meta.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok());
            let finished = std::fs::read_to_string(d.join("events.jsonl"))
                .ok()
                .and_then(|t| {
                    t.lines()
                        .last()
                        .map(|l| l.contains("\"type\":\"finished\""))
                })
                .unwrap_or(false);
            RunItem {
                dir: d.display().to_string(),
                name: name_of(&d),
                job: meta.as_ref().map_or_else(String::new, |m| m.job.clone()),
                started: meta.map_or_else(String::new, |m| m.started),
                finished,
            }
        })
        .collect()
}

/// Shows the run in `dir` from now on.
fn switch(studio: &Studio, window: &tauri::WebviewWindow, dir: &Path) -> Info {
    {
        let mut current = lock(&studio.current);
        current.generation += 1;
        current.dir = Some(dir.to_path_buf());
        current.closes = false;
        current.record = Some(Record::new(dir.join("events.jsonl")));
    }
    let _ = window.set_title(&window_title(dir));
    info_of(studio)
}

#[tauri::command]
fn open_run(
    state: tauri::State<'_, Studio>,
    window: tauri::WebviewWindow,
    dir: String,
) -> Result<Info, String> {
    let dir = PathBuf::from(dir);
    if !dir.join("events.jsonl").is_file() {
        return Err(format!("{} has no events.jsonl", dir.display()));
    }
    Ok(switch(&state, &window, &dir))
}

/// Runs the job file at `path` (its run directory in `runs/`), and shows it.
#[tauri::command]
fn run_job(
    state: tauri::State<'_, Studio>,
    window: tauri::WebviewWindow,
    path: String,
) -> Result<Info, String> {
    let job = Job::load(Path::new(&path)).map_err(|e| e.to_string())?;
    let mut record = Run::create(&state.root.join("runs"), &job).map_err(|e| e.to_string())?;
    let dir = record.dir().to_path_buf();
    let stop = Stop::new(job.timeout());
    let worker = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            // a failure is in the record (or the record ends without finishing): the window
            // shows it
            let _ = job::execute(&job, &mut record, &stop);
        })
    };
    lock(&state.started).push((stop, worker));
    Ok(switch(&state, &window, &dir))
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

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("photonoxide-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_record_reads_complete_lines_as_they_arrive() {
        let dir = scratch("record");
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

    #[test]
    fn the_start_page_lists_the_repositorys_jobs() {
        let jobs = jobs_in(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jobs"));
        let modes = jobs.iter().find(|j| j.name == "strip-modes").unwrap();
        assert_eq!(modes.kind, "modes");
        assert!(
            modes
                .about
                .starts_with("The modes of a 500 x 220 nm silicon strip"),
            "{}",
            modes.about
        );
        assert!(!modes.about.contains("photonoxide run"));
    }

    #[test]
    fn the_start_page_lists_runs_newest_first_and_knows_which_finished() {
        let root = scratch("runs");
        for (name, last) in [
            (
                "20261001-120000-a",
                "{\"type\":\"finished\",\"stopped\":null,\"seconds\":1.0}",
            ),
            (
                "20261002-120000-b",
                "{\"type\":\"started\",\"job\":\"b\",\"kind\":\"modes\"}",
            ),
        ] {
            std::fs::create_dir_all(root.join(name)).unwrap();
            std::fs::write(root.join(name).join("events.jsonl"), format!("{last}\n")).unwrap();
        }
        std::fs::create_dir_all(root.join("not-a-run")).unwrap();
        let runs = runs_in(&root, 40);
        let names: Vec<&str> = runs.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["20261002-120000-b", "20261001-120000-a"]);
        assert!(!runs[0].finished && runs[1].finished);
        assert_eq!(runs_in(&root, 1).len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
