//! The Windows install and uninstall, and their window.
//!
//! photonoxide's setup (studio/setup) unpacks this program and runs it as `photonoxide setup`;
//! `photonoxide uninstall` is what Apps & Features runs. Both take NSIS's flags, as Tauri's
//! updater and the nightly channel give them to a setup: `/P` shows only the progress, `/S`
//! nothing, `/UPDATE` and `/R` start the program again when it's done (with the arguments after
//! `/ARGS`), and `/D=<folder>` installs there. Without `/P` or `/S` the window asks first: the
//! folder, a shortcut on the Desktop, and for an uninstall, whether the settings and benchmark
//! records go too.
//!
//! The install is per user, in the place NSIS's per-user install had (`%LOCALAPPDATA%\photonoxide`,
//! or wherever the last install was), under the same Apps & Features key, with the same Start
//! menu shortcut, so it takes over from an NSIS install in place and removes its `uninstall.exe`.
//! The program installed before is renamed aside first (Windows lets a running program be
//! renamed, not replaced) and removed once it has exited ([`at_start`]). Each step is a line in
//! the window as it happens.
//!
//! The window is the studio's web view: it needs Microsoft's WebView2 runtime, which Windows 11
//! and an updated Windows 10 have; when it's missing, its installer is fetched from Microsoft and
//! run first, as Tauri's NSIS setup did.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, Manager};

/// The program's file, in its folder.
const PROGRAM: &str = "photonoxide.exe";
/// Apps & Features' key for it, under the user's registry (NSIS's key, by the product's name).
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\photonoxide";
/// NSIS's own record of the folder, under the user's registry (by the publisher and product).
const MAKER_KEY: &str = r"Software\tachsin\photonoxide";
const HOMEPAGE: &str = "https://tachsin.gr/projects/photonoxide";
/// The bundle identifier (tauri.conf.json): the folders the program's settings and records live
/// in, under the user's local and roaming app data.
const IDENTIFIER: &str = "gr.tachsin.photonoxide";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the setup shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Show {
    /// The window, which asks first.
    #[default]
    Window,
    /// The window with only the progress, which starts at once (`/P`).
    Progress,
    /// Nothing (`/S`).
    Nothing,
}

/// The command line, NSIS's flags.
#[derive(Debug, Default, PartialEq)]
pub struct Options {
    pub show: Show,
    /// `/UPDATE`: an update, which starts the program again when done.
    pub update: bool,
    /// `/R`: start the program again when done.
    pub restart: bool,
    /// `/D=<folder>`.
    pub dir: Option<PathBuf>,
    /// The setup that ran this (`--from`, from studio/setup), for what the window says.
    pub from: Option<PathBuf>,
    /// The uninstall's `--delete-data`: the settings and benchmark records go too.
    pub delete_data: bool,
    /// What to start the program with (everything after `/ARGS`).
    pub args: Vec<String>,
}

impl Options {
    /// Whether the program starts again by itself when the install is done.
    fn starts_again(&self) -> bool {
        self.update || self.restart
    }
}

/// Reads the flags; case doesn't matter, as in NSIS, and a flag it doesn't know is passed over,
/// as NSIS does, so a later updater's flag doesn't stop an update.
pub fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let upper = arg.to_ascii_uppercase();
        match upper.as_str() {
            "/P" => options.show = Show::Progress,
            "/S" => options.show = Show::Nothing,
            "/UPDATE" => options.update = true,
            "/R" => options.restart = true,
            "/ARGS" => options.args = args.by_ref().cloned().collect(),
            "--FROM" => {
                options.from = Some(args.next().ok_or("--from needs the setup's path")?.into());
            }
            "--DELETE-DATA" => options.delete_data = true,
            _ if upper.starts_with("/D=") => {
                let dir = &arg[3..];
                if dir.is_empty() {
                    return Err("/D= needs a folder".into());
                }
                options.dir = Some(PathBuf::from(dir));
            }
            _ => {}
        }
    }
    Ok(options)
}

/// Where everything goes.
#[derive(Clone, Debug)]
pub struct Target {
    /// The program's folder.
    pub dir: PathBuf,
    /// The Start menu's Programs folder.
    pub start_menu: PathBuf,
    pub desktop: PathBuf,
    /// The registry keys, under the user's (HKEY_CURRENT_USER).
    pub uninstall_key: String,
    pub maker_key: String,
    /// The program's settings and records: removed only when the uninstall is asked to.
    pub data: Vec<PathBuf>,
}

impl Target {
    /// The user's: in `dir`, or where the last install was, or NSIS's per-user default.
    pub fn user(dir: Option<PathBuf>) -> Result<Self, String> {
        let local = known(dirs::data_local_dir(), "local app data")?;
        let roaming = known(dirs::data_dir(), "roaming app data")?;
        let desktop = known(dirs::desktop_dir(), "Desktop")?;
        let dir = dir
            .or_else(|| system::installed_at(UNINSTALL_KEY, MAKER_KEY))
            .unwrap_or_else(|| local.join("photonoxide"));
        Ok(Self {
            dir,
            start_menu: roaming.join(r"Microsoft\Windows\Start Menu\Programs"),
            desktop,
            uninstall_key: UNINSTALL_KEY.into(),
            maker_key: MAKER_KEY.into(),
            data: vec![roaming.join(IDENTIFIER), local.join(IDENTIFIER)],
        })
    }

    pub fn program(&self) -> PathBuf {
        self.dir.join(PROGRAM)
    }

    /// The program installed before, renamed aside.
    fn aside(&self) -> PathBuf {
        self.dir.join("photonoxide.exe.old")
    }

    fn start_menu_link(&self) -> PathBuf {
        self.start_menu.join("photonoxide.lnk")
    }

    fn desktop_link(&self) -> PathBuf {
        self.desktop.join("photonoxide.lnk")
    }
}

fn known(dir: Option<PathBuf>, what: &str) -> Result<PathBuf, String> {
    dir.ok_or_else(|| format!("Windows didn't say where the user's {what} folder is"))
}

/// A step, for the window: a line when it's done, and how far the whole is.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Note {
    Line { text: String },
    Progress { done: f64 },
}

fn line(note: &mut dyn FnMut(Note), text: String) {
    note(Note::Line { text });
}

/// A registry value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    Number(u32),
}

/// Apps & Features' entry: NSIS's values, with photonoxide uninstalling itself.
pub fn entry(target: &Target, kib: u32) -> Vec<(&'static str, Value)> {
    let program = target.program();
    let quoted = format!("\"{}\"", program.display());
    vec![
        ("DisplayName", Value::Text("photonoxide".into())),
        ("DisplayIcon", Value::Text(quoted.clone())),
        ("DisplayVersion", Value::Text(VERSION.into())),
        ("Publisher", Value::Text("tachsin".into())),
        (
            "InstallLocation",
            Value::Text(format!("\"{}\"", target.dir.display())),
        ),
        (
            "UninstallString",
            Value::Text(format!("{quoted} uninstall")),
        ),
        (
            "QuietUninstallString",
            Value::Text(format!("{quoted} uninstall /S")),
        ),
        ("MainBinaryName", Value::Text(PROGRAM.into())),
        ("NoModify", Value::Number(1)),
        ("NoRepair", Value::Number(1)),
        ("EstimatedSize", Value::Number(kib)),
        ("URLInfoAbout", Value::Text(HOMEPAGE.into())),
        ("URLUpdateInfo", Value::Text(HOMEPAGE.into())),
        ("HelpLink", Value::Text(HOMEPAGE.into())),
    ]
}

/// Installs `program` (the one running, unpacked by the setup) into `target`.
pub fn install(
    target: &Target,
    program: &Path,
    desktop: bool,
    note: &mut dyn FnMut(Note),
) -> Result<(), String> {
    let shown = |p: &Path| p.display().to_string();
    std::fs::create_dir_all(&target.dir).map_err(|e| format!("{}: {e}", shown(&target.dir)))?;
    line(note, format!("Folder {}", shown(&target.dir)));
    note(Note::Progress { done: 0.04 });

    let to = target.program();
    let aside = target.aside();
    if to.exists() {
        let _ = std::fs::remove_file(&aside);
        std::fs::rename(&to, &aside)
            .map_err(|e| format!("the program installed before, {}: {e}", shown(&to)))?;
        line(note, "Set the program installed before aside".into());
    }
    let bytes = copy(program, &to, &mut |done| {
        note(Note::Progress {
            done: 0.05 + 0.8 * done,
        });
    })
    .inspect_err(|_| {
        // put back what was there
        if aside.exists() {
            let _ = std::fs::remove_file(&to);
            let _ = std::fs::rename(&aside, &to);
        }
    })?;
    line(
        note,
        format!(
            "Copied {PROGRAM} ({:.1} MiB) to {}",
            bytes as f64 / 1_048_576.0,
            shown(&to)
        ),
    );

    let nsis = target.dir.join("uninstall.exe");
    if nsis.exists() && std::fs::remove_file(&nsis).is_ok() {
        line(
            note,
            "Removed the NSIS setup's uninstall.exe: photonoxide uninstalls itself now".into(),
        );
    }
    note(Note::Progress { done: 0.88 });

    system::shortcut(&to, &target.start_menu_link())?;
    line(
        note,
        format!("Start menu shortcut {}", shown(&target.start_menu_link())),
    );
    if desktop || target.desktop_link().exists() {
        system::shortcut(&to, &target.desktop_link())?;
        line(
            note,
            format!("Desktop shortcut {}", shown(&target.desktop_link())),
        );
    }
    note(Note::Progress { done: 0.94 });

    let kib = u32::try_from(bytes.div_ceil(1024)).unwrap_or(u32::MAX);
    system::write_key(&target.uninstall_key, &entry(target, kib))?;
    system::write_key(&target.maker_key, &[("", Value::Text(shown(&target.dir)))])?;
    line(note, "Registered in Apps & Features".into());

    if aside.exists() {
        if std::fs::remove_file(&aside).is_ok() {
            line(note, "Removed the program installed before".into());
        } else {
            line(
                note,
                "The program installed before is still open: it's removed when photonoxide next starts"
                    .into(),
            );
        }
    }
    note(Note::Progress { done: 1.0 });
    Ok(())
}

/// Copies `from` to `to` in pieces, saying how far it is; returns the bytes copied.
fn copy(from: &Path, to: &Path, done: &mut dyn FnMut(f64)) -> Result<u64, String> {
    use std::io::{Read, Write};
    let mut input = std::fs::File::open(from).map_err(|e| format!("{}: {e}", from.display()))?;
    let total = input.metadata().map(|m| m.len()).unwrap_or(0).max(1);
    let mut output = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let mut buffer = vec![0u8; 1 << 20];
    let mut copied = 0u64;
    loop {
        let n = input
            .read(&mut buffer)
            .map_err(|e| format!("{}: {e}", from.display()))?;
        if n == 0 {
            break;
        }
        output
            .write_all(&buffer[..n])
            .map_err(|e| format!("{}: {e}", to.display()))?;
        copied += n as u64;
        done(copied as f64 / total as f64);
    }
    output
        .sync_all()
        .map_err(|e| format!("{}: {e}", to.display()))?;
    Ok(copied)
}

/// Removes the install in `target`; `running` is this program, which can't remove itself: it
/// is moved out of the folder and removed a moment after it exits.
pub fn uninstall(
    target: &Target,
    running: &Path,
    delete_data: bool,
    note: &mut dyn FnMut(Note),
) -> Result<(), String> {
    for link in [target.start_menu_link(), target.desktop_link()] {
        if link.exists() {
            std::fs::remove_file(&link).map_err(|e| format!("{}: {e}", link.display()))?;
            line(note, format!("Removed the shortcut {}", link.display()));
        }
    }
    note(Note::Progress { done: 0.2 });
    system::delete_key(&target.uninstall_key)?;
    system::delete_key(&target.maker_key)?;
    line(note, "Removed from Apps & Features".into());
    note(Note::Progress { done: 0.4 });
    if delete_data {
        for dir in &target.data {
            if dir.exists() {
                std::fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                line(
                    note,
                    format!("Removed the settings and records in {}", dir.display()),
                );
            }
        }
    }
    note(Note::Progress { done: 0.6 });

    let program = target.program();
    if program.exists() {
        if same_file(&program, running) {
            let moved = std::env::temp_dir().join(format!(
                "photonoxide-uninstalled-{}.exe",
                std::process::id()
            ));
            std::fs::rename(&program, &moved).map_err(|e| format!("{}: {e}", program.display()))?;
            system::remove_after_exit(&moved);
        } else {
            std::fs::remove_file(&program).map_err(|e| format!("{}: {e}", program.display()))?;
        }
        line(note, format!("Removed {}", program.display()));
    }
    for other in [target.aside(), target.dir.join("uninstall.exe")] {
        if other.exists() && std::fs::remove_file(&other).is_ok() {
            line(note, format!("Removed {}", other.display()));
        }
    }
    if std::fs::remove_dir(&target.dir).is_ok() {
        line(note, format!("Removed the folder {}", target.dir.display()));
    } else if target.dir.exists() {
        line(
            note,
            format!(
                "Left the folder {}: it holds files of your own",
                target.dir.display()
            ),
        );
    }
    note(Note::Progress { done: 1.0 });
    Ok(())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// At the program's start: the program an install set aside, once it has exited.
pub fn at_start() {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let aside = dir.join("photonoxide.exe.old");
        if aside.exists() {
            let _ = std::fs::remove_file(aside);
        }
    }
}

// ---- the command line ---------------------------------------------------------------------------

/// `photonoxide setup`.
pub fn run(args: &[String]) -> ExitCode {
    start(Kind::Install, args)
}

/// `photonoxide uninstall`.
pub fn remove(args: &[String]) -> ExitCode {
    start(Kind::Uninstall, args)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Install,
    Uninstall,
}

fn start(kind: Kind, args: &[String]) -> ExitCode {
    if !cfg!(windows) {
        eprintln!(
            "photonoxide's setup is for Windows: on macOS and Linux, the program installs as a disk image or a package"
        );
        return ExitCode::from(2);
    }
    let options = match parse(args) {
        Ok(o) => o,
        Err(e) => return fail(&e),
    };
    let running = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return fail(&format!("where this program is: {e}")),
    };
    // the uninstall removes the install it runs from
    let dir = match kind {
        Kind::Install => options.dir.clone(),
        Kind::Uninstall => options
            .dir
            .clone()
            .or_else(|| running.parent().map(Path::to_path_buf)),
    };
    let target = match Target::user(dir) {
        Ok(t) => t,
        Err(e) => return fail(&e),
    };
    if options.show == Show::Nothing {
        let mut quiet = |_: Note| {};
        let done = match kind {
            Kind::Install => install(&target, &running, false, &mut quiet),
            Kind::Uninstall => uninstall(&target, &running, options.delete_data, &mut quiet),
        };
        return match done {
            Ok(()) => {
                if kind == Kind::Install && options.starts_again() {
                    start_program(&target, &options.args);
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    if let Err(e) = system::web_view() {
        return fail(&e);
    }
    match window(kind, options, target, running) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => fail(&e),
    }
}

fn start_program(target: &Target, args: &[String]) {
    let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
    let _ = crate::channels::install::launch(&target.program(), &args);
}

/// Says what went wrong where nobody reads a console: in a message box.
fn fail(text: &str) -> ExitCode {
    eprintln!("{text}");
    #[cfg(windows)]
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("photonoxide setup")
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    ExitCode::FAILURE
}

// ---- the window ---------------------------------------------------------------------------------

struct Setup {
    kind: Kind,
    options: Options,
    target: Mutex<Target>,
    running: PathBuf,
    /// Whether the install or uninstall ran to its end.
    finished: Mutex<bool>,
    started: Mutex<bool>,
}

/// What the window shows before it starts.
#[derive(Serialize)]
struct Info {
    kind: Kind,
    show: Show,
    version: String,
    dir: String,
    /// The version installed now, if any.
    installed: Option<String>,
    /// Whether the Desktop has the shortcut already.
    desktop: bool,
    /// The program's size, in bytes.
    size: u64,
    update: bool,
    starts_again: bool,
    homepage: &'static str,
}

#[tauri::command]
fn setup_info(state: tauri::State<'_, Setup>) -> Info {
    let target = lock(&state.target).clone();
    Info {
        kind: state.kind,
        show: state.options.show,
        version: VERSION.into(),
        dir: target.dir.display().to_string(),
        installed: system::read_text(&target.uninstall_key, "DisplayVersion"),
        desktop: target.desktop_link().exists(),
        size: std::fs::metadata(&state.running)
            .map(|m| m.len())
            .unwrap_or(0),
        update: state.options.update,
        starts_again: state.options.starts_again(),
        homepage: HOMEPAGE,
    }
}

/// The outcome, at the end.
#[derive(Clone, Serialize)]
struct Done {
    ok: bool,
    error: Option<String>,
}

/// Starts the install (into `dir`, with or without the Desktop shortcut) or the uninstall; the
/// window follows `setup-note` and hears `setup-done` at the end.
#[tauri::command]
fn setup_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Setup>,
    dir: Option<String>,
    desktop: bool,
    delete_data: bool,
) -> Result<(), String> {
    {
        let mut started = lock(&state.started);
        if *started {
            return Err("it's under way already".into());
        }
        *started = true;
    }
    if let Some(dir) = dir.filter(|d| !d.trim().is_empty()) {
        lock(&state.target).dir = PathBuf::from(dir.trim());
    }
    let target = lock(&state.target).clone();
    let (kind, running) = (state.kind, state.running.clone());
    std::thread::spawn(move || {
        let emitter = app.clone();
        let mut note = move |n: Note| {
            let _ = emitter.emit("setup-note", n);
        };
        let outcome = match kind {
            Kind::Install => install(&target, &running, desktop, &mut note),
            Kind::Uninstall => uninstall(&target, &running, delete_data, &mut note),
        };
        let state = app.state::<Setup>();
        *lock(&state.finished) = outcome.is_ok();
        let _ = app.emit(
            "setup-done",
            Done {
                ok: outcome.is_ok(),
                error: outcome.err(),
            },
        );
    });
    Ok(())
}

/// Closes the window, starting the program first when `launch` (after an install).
#[tauri::command]
fn setup_finish(app: tauri::AppHandle, state: tauri::State<'_, Setup>, launch: bool) {
    if launch && state.kind == Kind::Install && *lock(&state.finished) {
        start_program(&lock(&state.target), &state.options.args);
    }
    app.exit(0);
}

/// Opens the window and returns once it closes: whether the install or uninstall finished.
fn window(kind: Kind, options: Options, target: Target, running: PathBuf) -> Result<bool, String> {
    // the web view's own files, in a folder of the setup's: none in the program's data
    let data = std::env::temp_dir().join(format!("photonoxide-setup-view-{}", std::process::id()));
    let script = format!(
        "window.__PHOTONOXIDE_SETUP__ = {};",
        serde_json::json!({ "kind": kind })
    );
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Setup {
            kind,
            options,
            target: Mutex::new(target),
            running,
            finished: Mutex::new(false),
            started: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            setup_info,
            setup_start,
            setup_finish
        ])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "setup",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(match kind {
                Kind::Install => "photonoxide setup",
                Kind::Uninstall => "Uninstall photonoxide",
            })
            .inner_size(820.0, 540.0)
            .resizable(false)
            .maximizable(false)
            .decorations(false)
            .shadow(true)
            .center()
            .data_directory(data.clone())
            .initialization_script(&script)
            .background_color(tauri::window::Color(22, 16, 16, 255))
            .build()?;
            Ok(())
        })
        .build(crate::studio::context())
        .map_err(|e| format!("can't open the setup's window: {e}"))?;
    let handle = app.handle().clone();
    app.run_return(|_, _| {});
    let finished = *lock(&handle.state::<Setup>().finished);
    Ok(finished)
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ---- Windows ------------------------------------------------------------------------------------

#[cfg(windows)]
mod system {
    use std::path::{Path, PathBuf};

    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    use super::Value;

    fn user() -> RegKey {
        RegKey::predef(HKEY_CURRENT_USER)
    }

    /// Where the last install was: Apps & Features' entry, or NSIS's own record.
    pub fn installed_at(uninstall_key: &str, maker_key: &str) -> Option<PathBuf> {
        read_text(uninstall_key, "InstallLocation")
            .or_else(|| read_text(maker_key, ""))
            .map(|d| PathBuf::from(d.trim_matches('"')))
            .filter(|d| !d.as_os_str().is_empty())
    }

    pub fn read_text(key: &str, name: &str) -> Option<String> {
        user().open_subkey(key).ok()?.get_value(name).ok()
    }

    pub fn write_key(key: &str, values: &[(&str, Value)]) -> Result<(), String> {
        let (k, _) = user()
            .create_subkey(key)
            .map_err(|e| format!("the registry key {key}: {e}"))?;
        for (name, value) in values {
            match value {
                Value::Text(t) => k.set_value(name, t),
                Value::Number(n) => k.set_value(name, n),
            }
            .map_err(|e| format!("the registry value {key}\\{name}: {e}"))?;
        }
        Ok(())
    }

    pub fn delete_key(key: &str) -> Result<(), String> {
        match user().delete_subkey_all(key) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("the registry key {key}: {e}")),
        }
    }

    pub fn shortcut(program: &Path, link: &Path) -> Result<(), String> {
        if let Some(dir) = link.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let mut l = mslnk::ShellLink::new(program)
            .map_err(|e| format!("the shortcut to {}: {e}", program.display()))?;
        l.set_name(Some(
            "photonoxide: photonics simulation, with a live studio".into(),
        ));
        l.set_working_dir(program.parent().map(|d| d.display().to_string()));
        l.set_icon_location(Some(program.display().to_string()));
        l.create_lnk(link)
            .map_err(|e| format!("the shortcut {}: {e}", link.display()))
    }

    /// Removes `file` a moment after this program exits, by a command prompt of its own.
    pub fn remove_after_exit(file: &Path) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        let _ = std::process::Command::new("cmd")
            .raw_arg(format!(
                "/C ping -n 3 127.0.0.1 > nul & del /F /Q \"{}\"",
                file.display()
            ))
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    /// The WebView2 runtime's client key (its "pv" is the version installed), per machine and
    /// per user, as Microsoft's distribution guide has it.
    const WEBVIEW2: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

    fn web_view_version() -> Option<String> {
        let machine = RegKey::predef(HKEY_LOCAL_MACHINE);
        let at = [
            (
                &machine,
                format!(r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{WEBVIEW2}"),
            ),
            (
                &machine,
                format!(r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{WEBVIEW2}"),
            ),
        ];
        let user = user();
        at.iter()
            .map(|(root, key)| (*root, key.clone()))
            .chain([(
                &user,
                format!(r"Software\Microsoft\EdgeUpdate\Clients\{WEBVIEW2}"),
            )])
            .filter_map(|(root, key)| {
                root.open_subkey_with_flags(key, KEY_READ)
                    .ok()?
                    .get_value::<String, _>("pv")
                    .ok()
            })
            .find(|v| !v.is_empty() && v != "0.0.0.0")
    }

    /// Makes sure the window can open: when WebView2 is missing, fetches Microsoft's evergreen
    /// bootstrapper and runs it, silently, for this user.
    pub fn web_view() -> Result<(), String> {
        if web_view_version().is_some() {
            return Ok(());
        }
        const BOOTSTRAPPER: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
        let file = std::env::temp_dir().join("MicrosoftEdgeWebview2Setup.exe");
        // over rustls with ring, as the channels' requests (channels/github.rs)
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let bytes = reqwest::blocking::Client::builder()
            .user_agent(format!("photonoxide-setup/{}", super::VERSION))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .and_then(|c| c.get(BOOTSTRAPPER).send())
            .and_then(reqwest::blocking::Response::error_for_status)
            .and_then(reqwest::blocking::Response::bytes)
            .map_err(|e| {
                format!(
                    "photonoxide's window needs Microsoft's WebView2 runtime, which isn't installed, \
                     and its installer couldn't be fetched ({e}). Install it from \
                     https://developer.microsoft.com/microsoft-edge/webview2 and run the setup again."
                )
            })?;
        std::fs::write(&file, &bytes).map_err(|e| format!("{}: {e}", file.display()))?;
        let status = std::process::Command::new(&file)
            .args(["/silent", "/install"])
            .status()
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let _ = std::fs::remove_file(&file);
        if web_view_version().is_some() {
            Ok(())
        } else {
            Err(format!(
                "Microsoft's WebView2 installer didn't install it ({status}). Install it from \
                 https://developer.microsoft.com/microsoft-edge/webview2 and run the setup again."
            ))
        }
    }
}

#[cfg(not(windows))]
mod system {
    use std::path::{Path, PathBuf};

    use super::Value;

    const ONLY: &str = "photonoxide's setup is for Windows";

    pub fn installed_at(_: &str, _: &str) -> Option<PathBuf> {
        None
    }
    pub fn read_text(_: &str, _: &str) -> Option<String> {
        None
    }
    pub fn write_key(_: &str, _: &[(&str, Value)]) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn delete_key(_: &str) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn shortcut(_: &Path, _: &Path) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn remove_after_exit(_: &Path) {}
    pub fn web_view() -> Result<(), String> {
        Err(ONLY.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn the_updaters_flags_are_read_as_nsis_reads_them() {
        // Tauri's updater, passive, an update, restarting with the program's arguments
        let o = parse(&args(&[
            "/P",
            "/UPDATE",
            "/R",
            "/ARGS",
            "run",
            "job one.yaml",
        ]))
        .unwrap();
        assert_eq!(o.show, Show::Progress);
        assert!(o.update && o.restart && o.starts_again());
        assert_eq!(o.args, ["run", "job one.yaml"]);
        // the nightly channel's, with the folder; case doesn't matter, unknown flags pass
        let o = parse(&args(&[
            "/p",
            "/update",
            "/NCRC",
            r"/D=C:\Users\me\My Apps\photonoxide",
        ]))
        .unwrap();
        assert_eq!(
            o.dir,
            Some(PathBuf::from(r"C:\Users\me\My Apps\photonoxide"))
        );
        assert!(o.update && !o.restart);
        let o = parse(&args(&["/S", "--from", "setup.exe"])).unwrap();
        assert_eq!(o.show, Show::Nothing);
        assert_eq!(o.from, Some(PathBuf::from("setup.exe")));
        assert!(!o.starts_again());
        assert_eq!(parse(&[]).unwrap(), Options::default());
        assert!(parse(&args(&["/D="])).is_err());
        assert!(parse(&args(&["--from"])).is_err());
    }

    fn scratch(name: &str) -> Target {
        let root = std::env::temp_dir().join(format!(
            "photonoxide-setup-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        // a registry key of the test's own, never the install's
        let key = format!(r"Software\photonoxide-tests\{name}-{}", std::process::id());
        Target {
            dir: root.join("install"),
            start_menu: root.join("start"),
            desktop: root.join("desktop"),
            uninstall_key: format!(r"{key}\uninstall"),
            maker_key: format!(r"{key}\maker"),
            data: vec![root.join("roaming"), root.join("local")],
        }
    }

    #[test]
    fn apps_and_features_runs_the_programs_own_uninstall() {
        let t = scratch("entry");
        let e = entry(&t, 51_200);
        let get = |n: &str| e.iter().find(|(k, _)| *k == n).map(|(_, v)| v.clone());
        let program = format!("\"{}\"", t.program().display());
        assert_eq!(
            get("UninstallString"),
            Some(Value::Text(format!("{program} uninstall")))
        );
        assert_eq!(
            get("QuietUninstallString"),
            Some(Value::Text(format!("{program} uninstall /S")))
        );
        assert_eq!(get("DisplayVersion"), Some(Value::Text(VERSION.into())));
        assert_eq!(get("EstimatedSize"), Some(Value::Number(51_200)));
    }

    #[cfg(windows)]
    #[test]
    fn an_install_takes_over_from_nsis_and_its_uninstall_leaves_nothing() {
        let t = scratch("round");
        let root = t.dir.parent().unwrap().to_path_buf();
        // what NSIS left: its program, its uninstaller, the Desktop shortcut
        std::fs::create_dir_all(&t.dir).unwrap();
        std::fs::write(t.program(), b"the program NSIS installed").unwrap();
        std::fs::write(t.dir.join("uninstall.exe"), b"NSIS's uninstaller").unwrap();
        std::fs::create_dir_all(&t.desktop).unwrap();
        std::fs::write(t.desktop.join("photonoxide.lnk"), b"").unwrap();
        // the new program: any file stands for it
        let new = root.join("new.exe");
        std::fs::write(&new, vec![7u8; 3 << 20]).unwrap();

        let mut notes = Vec::new();
        install(&t, &new, false, &mut |n| notes.push(n)).unwrap();
        assert_eq!(
            std::fs::read(t.program()).unwrap(),
            std::fs::read(&new).unwrap()
        );
        assert!(!t.dir.join("uninstall.exe").exists());
        assert!(!t.aside().exists());
        assert!(t.start_menu_link().exists());
        // the Desktop shortcut NSIS made is kept, though not asked for
        assert!(t.desktop_link().exists());
        assert_eq!(
            system::read_text(&t.uninstall_key, "DisplayVersion").as_deref(),
            Some(VERSION)
        );
        assert_eq!(
            system::installed_at(&t.uninstall_key, &t.maker_key),
            Some(t.dir.clone())
        );
        let progress: Vec<f64> = notes
            .iter()
            .filter_map(|n| match n {
                Note::Progress { done } => Some(*done),
                Note::Line { .. } => None,
            })
            .collect();
        assert!(progress.windows(2).all(|w| w[0] <= w[1]), "{progress:?}");
        assert_eq!(progress.last(), Some(&1.0));
        assert!(
            notes
                .iter()
                .any(|n| matches!(n, Note::Line { text } if text.contains("NSIS")))
        );

        // settings, kept unless asked
        std::fs::create_dir_all(&t.data[0]).unwrap();
        uninstall(&t, &new, false, &mut |_| {}).unwrap();
        assert!(!t.dir.exists() && !t.start_menu_link().exists() && !t.desktop_link().exists());
        assert!(system::read_text(&t.uninstall_key, "DisplayVersion").is_none());
        assert!(t.data[0].exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = system::delete_key(t.uninstall_key.rsplit_once('\\').unwrap().0);
        // and the tests' own key, once no other test's is in it
        let _ = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .delete_subkey(r"Software\photonoxide-tests");
    }
}
