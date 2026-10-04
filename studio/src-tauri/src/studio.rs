//! The studio window: a web view (studio/src: Svelte, Tailwind and daisyUI, three.js) on the
//! workspace, the built-in examples, and a run's record.
//!
//! The window follows the run's `events.jsonl` as it grows, so a live run and a replay are the
//! same code path, and what the window shows is exactly what the record holds. Started on a run
//! from the command line, it closes itself a little after the run finishes, and closing it early
//! asks the run to stop. Started bare, it opens on its home page; jobs started from the window
//! keep it open.

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

use crate::examples;
use crate::settings::{self, Settings};
use crate::tasks::{Progress, Tasks};

/// A run started from the command line: the window closes `linger` after `finished` fires, and
/// asks `stop` when it is closed first.
pub struct Live {
    pub linger: Duration,
    pub finished: Receiver<()>,
    pub stop: Stop,
}

/// Opens the window, on the run in `dir` or on the home page, and returns when it closes.
pub fn show(dir: Option<&Path>, live: Option<Live>) -> Result<(), String> {
    let started_in = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    let title = dir.map_or_else(|| "photonoxide".to_owned(), window_title);
    let first = dir.map(Path::to_path_buf);
    let closes = live.is_some();
    let live_stop = live.as_ref().map(|l| l.stop.clone());
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // the window opens where it was left, at the size it had
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            info,
            poll,
            app_state,
            save_settings,
            home,
            catalog,
            open_run,
            run_job,
            run_text,
            stop_run,
            check_job,
            save_job,
            read_job,
            delete_job,
            delete_run,
            run_events,
            group_index,
            start_example,
            start_validation,
            task_output,
            stop_task,
            changelog,
            published_report,
            method_docs,
            preview_scene,
            save_text,
            crate::materials::materials,
            crate::materials::material_curves,
            crate::materials::material_at,
            crate::circuits::component_library,
            crate::circuits::component_spectrum,
            crate::circuits::measured_component,
            crate::circuits::circuit_check,
            crate::circuits::circuit_simulate,
            crate::circuits::circuit_touchstone,
            crate::circuits::component_touchstone,
            crate::circuits::circuit_text,
            crate::circuits::circuit_parse,
            crate::circuits::circuit_examples,
            crate::circuits::circuits,
            crate::circuits::save_circuit,
            crate::circuits::read_circuit,
            crate::circuits::delete_circuit
        ])
        .setup(move |app| {
            let paths = app.path();
            let settings_file = paths
                .app_config_dir()
                .unwrap_or_else(|_| started_in.join(".photonoxide"))
                .join("settings.json");
            let documents = paths.document_dir().ok();
            let settings = Settings::load(&settings_file);
            let workspace = settings::workspace(&settings, &started_in, documents.as_deref());
            let mut started = Vec::new();
            if let (Some(dir), Some(stop)) = (&first, live_stop) {
                started.push(Started {
                    dir: dir.clone(),
                    stop,
                    worker: None,
                });
            }
            app.manage(Studio {
                started_in,
                documents,
                settings_file,
                settings: Mutex::new(settings),
                workspace: Mutex::new(workspace),
                current: Mutex::new(Current {
                    generation: 1,
                    dir: first.clone(),
                    closes,
                    record: first.as_ref().map(|d| Record::new(d.join("events.jsonl"))),
                }),
                started: Mutex::new(started),
                tasks: Tasks::default(),
            });
            tauri::WebviewWindowBuilder::new(
                app,
                "studio",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(title)
            .inner_size(1440.0, 900.0)
            .min_inner_size(960.0, 600.0)
            .background_color(tauri::window::Color(15, 17, 21, 255))
            .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| format!("can't open the studio window: {e}"))?;
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
    app.run_return(move |app, event| {
        // closed before the run finished: ask it to stop
        if let tauri::RunEvent::ExitRequested { .. } = &event
            && closes
            && let Some(studio) = app.try_state::<Studio>()
        {
            for s in lock(&studio.started).iter() {
                s.stop.request();
            }
        }
    });
    // the jobs started from the window stop at their next check, and finish their records
    let studio = handle.state::<Studio>();
    studio.tasks.stop_all();
    let started = std::mem::take(&mut *lock(&studio.started));
    for s in &started {
        s.stop.request();
    }
    for s in started {
        if let Some(worker) = s.worker {
            let _ = worker.join();
        }
    }
    Ok(())
}

fn window_title(dir: &Path) -> String {
    format!("photonoxide: {}", name_of(dir))
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
pub(crate) struct Studio {
    /// The folder the program was started in.
    started_in: PathBuf,
    documents: Option<PathBuf>,
    settings_file: PathBuf,
    settings: Mutex<Settings>,
    /// Where `jobs/` and `runs/` are: see [`settings::workspace`].
    workspace: Mutex<PathBuf>,
    current: Mutex<Current>,
    /// The runs started here (or the one from the command line), running or finished.
    started: Mutex<Vec<Started>>,
    /// Examples and validation reports, each in a process of its own.
    tasks: Tasks,
}

struct Started {
    dir: PathBuf,
    stop: Stop,
    /// None for the run from the command line, which `main` waits for.
    worker: Option<JoinHandle<()>>,
}

impl Studio {
    pub(crate) fn workspace(&self) -> PathBuf {
        lock(&self.workspace).clone()
    }
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
    /// The run's directory and name, or none.
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
        root: studio.workspace().display().to_string(),
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
    /// The run was started here and is still going: the window offers to stop it.
    stoppable: bool,
}

#[tauri::command]
fn poll(state: tauri::State<'_, Studio>, from: usize) -> Poll {
    let mut current = lock(&state.current);
    let generation = current.generation;
    let stoppable = current.dir.as_ref().is_some_and(|d| {
        lock(&state.started)
            .iter()
            .any(|s| &s.dir == d && s.worker.as_ref().is_none_or(|w| !w.is_finished()))
    });
    match current.record.as_mut() {
        Some(record) => {
            record.refresh();
            let finished = matches!(record.events.last(), Some(Event::Finished { .. }));
            Poll {
                generation,
                events: record.events.get(from..).unwrap_or_default().to_vec(),
                problem: record.problem.clone(),
                stoppable: stoppable && !finished,
            }
        }
        None => Poll {
            generation,
            events: Vec::new(),
            problem: None,
            stoppable: false,
        },
    }
}

/// What the window needs to know at the start.
#[derive(Serialize)]
struct AppState {
    settings: Settings,
    workspace: String,
    /// The library's version, which is the program's.
    version: String,
    /// The platform, e.g. `"windows-x86_64"`.
    platform: String,
    /// This copy was installed from a release (an installer, AppImage, package or app), so it
    /// can update itself; a build from the repository can't.
    updatable: bool,
}

fn app_state_of(studio: &Studio) -> AppState {
    AppState {
        settings: lock(&studio.settings).clone(),
        workspace: studio.workspace().display().to_string(),
        version: photonoxide::VERSION.to_owned(),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        updatable: !cfg!(debug_assertions) && tauri::utils::platform::bundle_type().is_some(),
    }
}

#[tauri::command]
fn app_state(state: tauri::State<'_, Studio>) -> AppState {
    app_state_of(&state)
}

/// Saves `settings`, and moves to the workspace they name.
#[tauri::command]
fn save_settings(state: tauri::State<'_, Studio>, settings: Settings) -> Result<AppState, String> {
    settings.save(&state.settings_file)?;
    *lock(&state.workspace) =
        settings::workspace(&settings, &state.started_in, state.documents.as_deref());
    *lock(&state.settings) = settings;
    Ok(app_state_of(&state))
}

/// A job file in the workspace.
#[derive(Serialize)]
struct JobItem {
    path: String,
    name: String,
    /// The task's kind, e.g. `"modes"`.
    kind: String,
    /// The job file's opening comment.
    about: String,
    /// When it was last changed, seconds since 1970.
    modified: f64,
}

/// A run in the workspace.
#[derive(Serialize)]
struct RunItem {
    dir: String,
    name: String,
    job: String,
    /// The task's kind, from the start of its record.
    kind: String,
    /// When it started, UTC.
    started: String,
    /// Its record ends with the run finishing.
    finished: bool,
    /// How long it took, from its record.
    seconds: Option<f64>,
    /// Why it stopped early, if it did.
    stopped: Option<String>,
}

#[derive(Serialize)]
struct Home {
    root: String,
    jobs: Vec<JobItem>,
    runs: Vec<RunItem>,
}

/// The jobs in the workspace's `jobs/` and its 200 newest runs.
#[tauri::command]
fn home(state: tauri::State<'_, Studio>) -> Home {
    let root = state.workspace();
    Home {
        root: root.display().to_string(),
        jobs: jobs_in(&root.join("jobs")),
        runs: runs_in(&root.join("runs"), 200),
    }
}

fn kind_of(job: &Job) -> String {
    job.task()
        .get("kind")
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_owned()
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
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0.0, |d| d.as_secs_f64());
            Some(JobItem {
                path: path.display().to_string(),
                name: job.name().to_owned(),
                kind: kind_of(&job),
                about: about(&text),
                modified,
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
            let text = std::fs::read_to_string(d.join("events.jsonl")).unwrap_or_default();
            let event =
                |line: Option<&str>| line.and_then(|l| serde_json::from_str::<Event>(l).ok());
            let kind = match event(text.lines().next()) {
                Some(Event::Started { kind, .. }) => kind,
                _ => String::new(),
            };
            let (finished, seconds, stopped) = match event(text.lines().last()) {
                Some(Event::Finished { stopped, seconds }) => (true, Some(seconds), stopped),
                _ => (false, None, None),
            };
            RunItem {
                dir: d.display().to_string(),
                name: name_of(&d),
                job: meta.as_ref().map_or_else(String::new, |m| m.job.clone()),
                kind,
                started: meta.map_or_else(String::new, |m| m.started),
                finished,
                seconds,
                stopped,
            }
        })
        .collect()
}

/// A built-in job, as the gallery shows it.
#[derive(Serialize)]
struct JobExample {
    file: String,
    name: String,
    kind: String,
    about: String,
    text: String,
}

/// What ships inside the program: the examples and the job files.
#[derive(Serialize)]
struct Catalog {
    examples: Vec<examples::Example>,
    jobs: Vec<JobExample>,
}

#[tauri::command]
fn catalog() -> Catalog {
    Catalog {
        examples: examples::list(),
        jobs: examples::jobs()
            .into_iter()
            .filter_map(|j| {
                let job = Job::parse(&j.text).ok()?;
                Some(JobExample {
                    name: job.name().to_owned(),
                    kind: kind_of(&job),
                    about: about(&j.text),
                    file: j.file,
                    text: j.text,
                })
            })
            .collect(),
    }
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

/// Runs `job` (its run directory in the workspace's `runs/`), and shows it.
fn start(studio: &Studio, window: &tauri::WebviewWindow, job: Job) -> Result<Info, String> {
    job::check(&job).map_err(|e| e.to_string())?;
    let mut record =
        Run::create(&studio.workspace().join("runs"), &job).map_err(|e| e.to_string())?;
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
    lock(&studio.started).push(Started {
        dir: dir.clone(),
        stop,
        worker: Some(worker),
    });
    Ok(switch(studio, window, &dir))
}

/// Runs the job file at `path`, and shows it.
#[tauri::command]
fn run_job(
    state: tauri::State<'_, Studio>,
    window: tauri::WebviewWindow,
    path: String,
) -> Result<Info, String> {
    let job = Job::load(Path::new(&path)).map_err(|e| e.to_string())?;
    start(&state, &window, job)
}

/// Runs the job in `text` (from the builder, or a built-in one), and shows it.
#[tauri::command]
fn run_text(
    state: tauri::State<'_, Studio>,
    window: tauri::WebviewWindow,
    text: String,
) -> Result<Info, String> {
    let job = Job::parse(&text).map_err(|e| e.to_string())?;
    start(&state, &window, job)
}

/// Asks the run in `dir` to stop at its next check.
#[tauri::command]
fn stop_run(state: tauri::State<'_, Studio>, dir: String) {
    let dir = PathBuf::from(dir);
    for s in lock(&state.started).iter().filter(|s| s.dir == dir) {
        s.stop.request();
    }
}

/// A job's text checked: its parts for the builder's form, or why it isn't a job.
#[derive(Serialize)]
struct JobCheck {
    /// The text is a job file and its task fits its kind ([`job::check`]).
    ok: bool,
    error: Option<String>,
    /// The whole file as JSON, when it is TOML at all.
    model: Option<serde_json::Value>,
}

#[tauri::command]
fn check_job(text: String) -> JobCheck {
    let model = toml::from_str::<toml::Table>(&text)
        .ok()
        .and_then(|t| serde_json::to_value(t).ok());
    let error = Job::parse(&text)
        .and_then(|job| job::check(&job))
        .err()
        .map(|e| e.to_string());
    JobCheck {
        ok: error.is_none(),
        error,
        model,
    }
}

/// `path` inside `dir` (after resolving both), or an error.
pub(crate) fn inside(dir: &Path, path: &Path) -> Result<PathBuf, String> {
    let dir = dir
        .canonicalize()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if path.starts_with(&dir) && path != dir {
        Ok(path)
    } else {
        Err(format!(
            "{} isn't in the workspace's {}",
            path.display(),
            dir.display()
        ))
    }
}

/// Saves the job in `text` as `jobs/<its name>.toml` in the workspace; refuses to replace
/// another file unless `replace`.
#[tauri::command]
fn save_job(
    state: tauri::State<'_, Studio>,
    text: String,
    replace: bool,
) -> Result<String, String> {
    let job = Job::parse(&text).map_err(|e| e.to_string())?;
    let dir = state.workspace().join("jobs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.toml", job.name()));
    if path.exists() && !replace {
        return Err(format!("exists: {}", path.display()));
    }
    std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn read_job(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path)
        .map(|t| t.replace("\r\n", "\n"))
        .map_err(|e| format!("{path}: {e}"))
}

/// Deletes a job file of the workspace.
#[tauri::command]
fn delete_job(state: tauri::State<'_, Studio>, path: String) -> Result<(), String> {
    let path = inside(&state.workspace().join("jobs"), Path::new(&path))?;
    if path.extension().is_none_or(|x| x != "toml") {
        return Err(format!("{} isn't a job file", path.display()));
    }
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Deletes a run of the workspace, its folder and all. A run still going is stopped first, and
/// the window closes the run if it shows it.
#[tauri::command(async)]
fn delete_run(
    state: tauri::State<'_, Studio>,
    window: tauri::WebviewWindow,
    dir: String,
) -> Result<(), String> {
    let dir = inside(&state.workspace().join("runs"), Path::new(&dir))?;
    if !dir.join("events.jsonl").is_file() {
        return Err(format!("{} isn't a run", dir.display()));
    }
    let same = |d: &Path| d.canonicalize().is_ok_and(|d| d == dir);
    // a run started here: stop it, and wait for its record to close
    let running = |studio: &Studio| {
        lock(&studio.started)
            .iter()
            .any(|s| same(&s.dir) && s.worker.as_ref().is_none_or(|w| !w.is_finished()))
    };
    if running(&state) {
        for s in lock(&state.started).iter().filter(|s| same(&s.dir)) {
            s.stop.request();
        }
        let waited = std::time::Instant::now();
        while running(&state) {
            if waited.elapsed() > Duration::from_secs(30) {
                return Err(
                    "the run is still stopping (it stops at its next check): try again in a moment"
                        .into(),
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    {
        let mut current = lock(&state.current);
        if current.dir.as_deref().is_some_and(same) {
            current.generation += 1;
            current.dir = None;
            current.closes = false;
            current.record = None;
            let _ = window.set_title("photonoxide");
        }
    }
    lock(&state.started).retain(|s| !same(&s.dir));
    std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))
}

/// Writes `text` to `path`, a file the user chose in a save dialog (a plot's data as CSV).
#[tauri::command]
fn save_text(path: String, text: String) -> Result<(), String> {
    std::fs::write(&path, text).map_err(|e| format!("{path}: {e}"))
}

/// The scene the job in `text` records first, for the studio to draw before it runs.
#[tauri::command]
fn preview_scene(text: String) -> Result<Event, String> {
    let job = Job::parse(&text).map_err(|e| e.to_string())?;
    job::preview(&job).map_err(|e| e.to_string())
}

/// Every event of the run in `dir`, for comparing runs.
#[tauri::command]
fn run_events(dir: String) -> Result<Vec<Event>, String> {
    let mut record = Record::new(Path::new(&dir).join("events.jsonl"));
    record.refresh();
    if record.offset == 0 {
        return Err(format!("{dir} has no record"));
    }
    Ok(record.events)
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

/// Starts the built-in example `name`; returns its task.
#[tauri::command]
fn start_example(state: tauri::State<'_, Studio>, name: String) -> Result<u64, String> {
    if !examples::list().iter().any(|e| e.name == name) {
        return Err(format!("no example {name}"));
    }
    state.tasks.start(&["example", &name])
}

/// Starts the validation report; returns its task.
#[tauri::command]
fn start_validation(state: tauri::State<'_, Studio>) -> Result<u64, String> {
    state.tasks.start(&["validate"])
}

#[tauri::command]
fn task_output(state: tauri::State<'_, Studio>, id: u64, from: usize) -> Result<Progress, String> {
    state.tasks.output(id, from)
}

#[tauri::command]
fn stop_task(state: tauri::State<'_, Studio>, id: u64) {
    state.tasks.stop(id);
}

/// The changelog of every release up to this one.
#[tauri::command]
fn changelog() -> &'static str {
    include_str!("../../../CHANGELOG.md")
}

/// The validation report as this release's CI wrote it (docs/validation.md).
#[tauri::command]
fn published_report() -> &'static str {
    include_str!("../../../docs/validation.md")
}

/// A method's write-up: its file in docs/methods and its text, front matter included.
#[derive(Serialize)]
struct MethodDoc {
    file: &'static str,
    text: &'static str,
}

/// The write-ups of the methods the jobs solve by (docs/methods), for the viewer's account of
/// how a run was solved.
#[tauri::command]
fn method_docs() -> Vec<MethodDoc> {
    macro_rules! docs {
        ($($file:literal),* $(,)?) => {
            vec![$(MethodDoc {
                file: $file,
                text: include_str!(concat!("../../../docs/methods/", $file)),
            }),*]
        };
    }
    docs![
        "vector.md",
        "eigen.md",
        "walls.md",
        "fdfd.md",
        "fdfd-ports.md",
        "eim.md",
        "slab.md",
        "pml.md",
    ]
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
