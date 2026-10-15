//! Installing a nightly build in place of this copy, and going back when it doesn't start.
//!
//! Before replacing anything, the running build is copied to `previous/<install>/`, a folder of
//! its own for each install, so that nothing an earlier install left running (its watchdog) is
//! ever in the way. A watchdog runs from that copy (`photonoxide nightly watch`) and follows the
//! install in `handover.json`, writing each step to `install.log`:
//!
//! - **Windows:** the watchdog waits for this program to exit (its process, up to
//!   [`EXIT_WITHIN`] seconds), then runs the build's NSIS per-user installer as Tauri's updater
//!   does, passive and as an update (`/P /UPDATE`), into this copy's folder (`/D=`), and waits
//!   for it (up to [`INSTALL_WITHIN`] seconds). It then asks the program installed what it is
//!   (`--version`), and opens it only when it is the new build. Otherwise the install failed:
//!   nothing was replaced (or what was, is put back), the build that was running opens again,
//!   and it says why.
//! - **macOS:** the `.app` bundle is replaced (copied beside it, then renamed into place),
//!   checked the same way, and opened again.
//! - **Linux:** an AppImage is swapped in place the same way. A `.deb` or `.rpm` install is the
//!   package manager's: the user installs the built package, or lets `pkexec` do it after
//!   confirming.
//!
//! The new build then has [`START_WITHIN`] seconds to say it started (its window calls
//! [`started`]); if it hasn't, the watchdog puts the previous build back and opens it, and that
//! build says what happened. A program that starts at the install's target as another build
//! knows the install didn't take ([`at_start`]): it says so, and the watchdog stands down. A
//! later install supersedes an earlier one still under way ([`supersede`]). Settings can roll
//! back by hand as well (`photonoxide nightly restore`).
//!
//! Children get no standard handles (`Stdio::null`): a program opened from Explorer has let go
//! of its console, and Windows refuses to start a child given the handles it had.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::builder::{Dirs, Ready, read_json, write_json};
use super::github::short;

/// How long a new build has to start before the previous one is put back, in seconds.
pub const START_WITHIN: u64 = 300;
/// How long the program being replaced has to exit before its installer runs, in seconds.
pub const EXIT_WITHIN: u64 = 60;
/// How long the installer may take, in seconds.
pub const INSTALL_WITHIN: u64 = 600;
/// How long a program asked for its version may take to answer.
const VERSION_WITHIN: Duration = Duration::from_secs(30);

/// This copy: its program, and the bundle it was installed from (Tauri's `bundle_type`).
pub struct Running {
    pub exe: PathBuf,
    /// `nsis`, `msi`, `app`, `dmg`, `appimage`, `deb` or `rpm`; none for a build from the
    /// repository, which isn't installed.
    pub bundle: Option<String>,
    /// The AppImage's path (`$APPIMAGE`), when it runs from one.
    pub appimage: Option<PathBuf>,
}

impl Running {
    /// This copy, as it is running.
    pub fn here() -> Running {
        use tauri::utils::config::BundleType;
        use tauri::utils::platform::bundle_type;
        let bundle = bundle_type().map(|b| {
            match b {
                BundleType::Deb => "deb",
                BundleType::Rpm => "rpm",
                BundleType::AppImage => "appimage",
                BundleType::Msi => "msi",
                BundleType::Nsis => "nsis",
                BundleType::App => "app",
                BundleType::Dmg => "dmg",
            }
            .to_owned()
        });
        let exe = std::env::current_exe().unwrap_or_default();
        // Tauri says "app" for any program on macOS: only one inside an app bundle is installed
        let in_app = exe
            .ancestors()
            .nth(3)
            .is_some_and(|p| p.extension().is_some_and(|e| e == "app"));
        let bundle = bundle.filter(|b| b != "app" || in_app);
        Running {
            exe,
            bundle: if cfg!(debug_assertions) { None } else { bundle },
            appimage: std::env::var_os("APPIMAGE").map(PathBuf::from),
        }
    }

    /// The bundle a nightly build makes to replace this copy.
    pub fn bundle_needed(&self) -> Result<&'static str, String> {
        match self.bundle.as_deref() {
            Some("nsis") => Ok("nsis"),
            Some("app" | "dmg") => Ok("app"),
            Some("appimage") => Ok("appimage"),
            Some("deb") => Ok("deb"),
            Some("rpm") => Ok("rpm"),
            Some(other) => Err(format!(
                "this copy was installed from a {other} package, which photonoxide doesn't make: \
                 install a release's, then switch to Nightly"
            )),
            None => Err(
                "this copy was built from the repository, not installed, so it can't \
                         replace itself: pull and rebuild, or install a release"
                    .into(),
            ),
        }
    }

    /// What an install replaces: the program (Windows, or a package's), the `.app` bundle,
    /// or the AppImage.
    pub fn target(&self) -> Result<PathBuf, String> {
        match self.bundle_needed()? {
            "app" => self
                .exe
                .ancestors()
                .nth(3)
                .filter(|p| p.extension().is_some_and(|e| e == "app"))
                .map(Path::to_path_buf)
                .ok_or_else(|| format!("{} isn't inside an app bundle", self.exe.display())),
            "appimage" => self
                .appimage
                .clone()
                .ok_or_else(|| "the AppImage's path ($APPIMAGE) isn't known".into()),
            _ => Ok(self.exe.clone()),
        }
    }
}

/// Where an install is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// The watchdog waits for the program being replaced to exit (Windows).
    #[default]
    Waiting,
    /// The installer runs (Windows).
    Installing,
    /// The new build is in place and was opened: it has until the deadline to say it started.
    Installed,
    /// It said so.
    Started,
    /// It didn't in time, or the user asked: the previous build was put back.
    RolledBack,
    /// The install didn't take: nothing was replaced, or what was is put back.
    Failed,
    /// A later install took over.
    Superseded,
}

impl Stage {
    /// The install is still under way: its watchdog has something to do.
    pub fn open(self) -> bool {
        matches!(self, Stage::Waiting | Stage::Installing | Stage::Installed)
    }
}

/// An install in progress: written before anything is replaced, followed by the watchdog,
/// confirmed by the new build.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handover {
    /// This install's own name: a later install's handover has another, and the watchdog of
    /// this one stands down. Empty in a handover written by a build before 0.5.2.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub stage: Stage,
    /// The new build's commit, and what it is called.
    pub to: String,
    pub to_label: String,
    /// What the build being replaced is called, and its commit.
    pub from_label: String,
    #[serde(default)]
    pub from_commit: Option<String>,
    /// The process being replaced, which the installer waits for, and the watchdog's.
    #[serde(default)]
    pub from_pid: Option<u32>,
    #[serde(default)]
    pub watchdog_pid: Option<u32>,
    /// The copy of the build being replaced, and where both are installed.
    pub previous: PathBuf,
    pub target: PathBuf,
    /// The installer the watchdog runs (Windows).
    #[serde(default)]
    pub installer: Option<PathBuf>,
    /// By when the new build must have started, seconds since 1970.
    pub deadline: u64,
    /// What builds before 0.5.2 read and write, kept from the stage: `started`, and
    /// `rolled_back` for an install over any other way, so that their watchdog stands down too.
    pub started: bool,
    pub rolled_back: bool,
    /// Why the install failed.
    #[serde(default)]
    pub reason: Option<String>,
}

impl Handover {
    /// Moves the install to `stage`.
    pub fn set(&mut self, stage: Stage) {
        self.stage = stage;
        self.started = stage == Stage::Started;
        self.rolled_back = matches!(stage, Stage::RolledBack | Stage::Failed | Stage::Superseded);
    }

    /// The install is under way, by this handover's own account and by what an older build
    /// may have written into it.
    pub fn open(&self) -> bool {
        !self.started && !self.rolled_back && (self.id.is_empty() || self.stage.open())
    }
}

/// The build kept to go back to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Previous {
    pub label: String,
    pub commit: Option<String>,
    /// The copy, and where it goes back to.
    pub copy: PathBuf,
    pub target: PathBuf,
    pub kept_at: u64,
}

/// What a rollback or a failed install did, for the window of the build running after it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// The build left (or not installed), and the one gone back to (or still running).
    pub left: String,
    pub back_to: String,
    /// The user asked for it; otherwise the new build didn't start in time, or wasn't
    /// installed.
    pub asked: bool,
    /// The install failed, and why.
    #[serde(default)]
    pub failed: Option<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Adds a line to `install.log` in the nightly folder: when, which install, what.
pub fn log(dirs: &Dirs, id: &str, line: &str) {
    use std::io::Write;
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64());
    let _ = std::fs::create_dir_all(&dirs.root);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dirs.install_log())
    {
        let _ = writeln!(file, "{at:.3} [{id}] {line}");
    }
}

/// `path` with `suffix` after its file name: `photonoxide.exe` → `photonoxide.exe.nightly-new`.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn remove(path: &Path) {
    let _ = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
}

/// No standard handles for a child: see the module's note.
fn quiet(command: &mut Command) -> &mut Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

/// Copies a file, or an app bundle's folder, keeping what makes it run (permissions; on macOS,
/// with `ditto`, its signature and links too).
pub fn copy(from: &Path, to: &Path) -> Result<(), String> {
    remove(to);
    if from.is_dir() {
        if cfg!(target_os = "macos") {
            let status = quiet(Command::new("ditto").arg(from).arg(to))
                .status()
                .map_err(|e| format!("can't run ditto: {e}"))?;
            return if status.success() {
                Ok(())
            } else {
                Err(format!("ditto {} {} failed", from.display(), to.display()))
            };
        }
        std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
        for entry in std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            copy(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to)
            .map(|_| ())
            .map_err(|e| format!("can't copy {} to {}: {e}", from.display(), to.display()))
    }
}

/// Where the program being replaced goes aside: `.nightly-old`, or when one is still there
/// (a program still running on Windows can't be removed), `.nightly-old-1` and on.
fn aside(target: &Path) -> PathBuf {
    let free = |p: &PathBuf| {
        remove(p);
        !p.exists()
    };
    std::iter::once(beside(target, ".nightly-old"))
        .chain((1..100).map(|n| beside(target, &format!(".nightly-old-{n}"))))
        .find(free)
        .unwrap_or_else(|| beside(target, &format!(".nightly-old-{}", now())))
}

/// Puts `new` where `target` is: copied beside it, then renamed into place, the old one
/// renamed aside first and removed after (a running program on Windows can be renamed but not
/// removed: it goes at the next install).
pub fn replace(target: &Path, new: &Path) -> Result<(), String> {
    let staged = beside(target, ".nightly-new");
    let old = aside(target);
    copy(new, &staged)?;
    #[cfg(unix)]
    if staged.is_file() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755));
    }
    if target.exists() {
        std::fs::rename(target, &old).map_err(|e| {
            remove(&staged);
            format!(
                "can't move {} aside ({e}); install {} by hand",
                target.display(),
                new.display()
            )
        })?;
    }
    if let Err(e) = std::fs::rename(&staged, target) {
        let _ = std::fs::rename(&old, target);
        return Err(format!(
            "can't put the new build at {}: {e}",
            target.display()
        ));
    }
    remove(&old);
    Ok(())
}

/// A name for an install, its folder in `previous/`: when, and by which process.
fn new_id() -> String {
    format!("{}-{}", now(), std::process::id())
}

/// Keeps a copy of what is installed at `target` (the build running now) in `previous/<id>/`,
/// and removes the copies of earlier installs, those it can: one still running (an earlier
/// install's watchdog) goes at a later install.
pub fn keep_previous(
    dirs: &Dirs,
    id: &str,
    target: &Path,
    label: &str,
    commit: Option<&str>,
) -> Result<Previous, String> {
    let dir = dirs.previous().join(id);
    remove(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // a program gets another name, so an installer closing the program by its name doesn't
    // close the watchdog running from the copy
    let name = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let name = match name.rsplit_once('.') {
        Some((stem, ext)) if ext != "app" => format!("{stem}-previous.{ext}"),
        Some(_) => name,
        None => format!("{name}-previous"),
    };
    let copied = dir.join(name);
    copy(target, &copied)?;
    let previous = Previous {
        label: label.to_owned(),
        commit: commit.map(str::to_owned),
        copy: copied,
        target: target.to_path_buf(),
        kept_at: now(),
    };
    write_json(&dirs.previous_file(), &previous)?;
    if let Ok(entries) = std::fs::read_dir(dirs.previous()) {
        for entry in entries.flatten() {
            if entry.file_name() != id {
                remove(&entry.path());
            }
        }
    }
    Ok(previous)
}

/// The program inside a kept copy: the copy itself, or an app bundle's executable.
pub fn program_of(copy: &Path) -> PathBuf {
    if copy.extension().is_some_and(|e| e == "app") {
        let macos = copy.join("Contents").join("MacOS");
        if let Some(first) = std::fs::read_dir(&macos)
            .ok()
            .and_then(|mut d| d.find_map(|e| e.ok().map(|e| e.path())))
        {
            return first;
        }
    }
    copy.to_path_buf()
}

/// Starts `program` with `args` on its own, to outlive this process; returns its process id.
pub fn launch(program: &Path, args: &[&std::ffi::OsStr]) -> Result<u32, String> {
    let mut command = if program.extension().is_some_and(|e| e == "app") {
        let mut open = Command::new("open");
        open.arg("-n").arg(program);
        if !args.is_empty() {
            open.arg("--args").args(args);
        }
        open
    } else {
        let mut c = Command::new(program);
        c.args(args);
        c
    };
    quiet(&mut command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .spawn()
        .map(|c| c.id())
        .map_err(|e| format!("can't start {}: {e}", program.display()))
}

/// Processes by their id: whether one is still running, waiting for it to end, ending it.
pub mod process {
    use std::time::Duration;

    #[cfg(windows)]
    mod win {
        use std::ffi::c_void;

        unsafe extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
            fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }
        const SYNCHRONIZE: u32 = 0x0010_0000;
        const WAIT_TIMEOUT: u32 = 0x0000_0102;

        /// Waits up to `ms` for process `pid` to end: true once it has, or when there is none.
        pub fn wait(pid: u32, ms: u32) -> bool {
            // SAFETY: plain Win32 calls; the handle is used only when OpenProcess gave one,
            // and closed once
            unsafe {
                let handle = OpenProcess(SYNCHRONIZE, 0, pid);
                if handle.is_null() {
                    return true;
                }
                let waited = WaitForSingleObject(handle, ms);
                CloseHandle(handle);
                waited != WAIT_TIMEOUT
            }
        }
    }

    /// Waits up to `within` for process `pid` to end: true once it has (or there is none).
    pub fn wait_exit(pid: u32, within: Duration) -> bool {
        #[cfg(windows)]
        {
            win::wait(
                pid,
                u32::try_from(within.as_millis()).unwrap_or(u32::MAX - 1),
            )
        }
        #[cfg(not(windows))]
        {
            let deadline = std::time::Instant::now() + within;
            loop {
                let running =
                    super::quiet(std::process::Command::new("kill").args(["-0", &pid.to_string()]))
                        .status()
                        .is_ok_and(|s| s.success());
                if !running {
                    return true;
                }
                if std::time::Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    pub fn alive(pid: u32) -> bool {
        !wait_exit(pid, Duration::ZERO)
    }

    /// Ends process `pid`, and waits a moment for it to go.
    pub fn kill(pid: u32) {
        let mut command = if cfg!(windows) {
            let mut c = std::process::Command::new("taskkill");
            c.args(["/PID", &pid.to_string(), "/F"]);
            c
        } else {
            let mut c = std::process::Command::new("kill");
            c.args(["-9", &pid.to_string()]);
            c
        };
        crate::libraries::no_window(super::quiet(&mut command));
        let _ = command.status();
        wait_exit(pid, Duration::from_secs(5));
    }
}

/// Waits up to `within` for `child` to end; ends it if it doesn't, and says `None`.
fn wait_for(child: &mut Child, within: Duration) -> Result<Option<ExitStatus>, String> {
    let deadline = Instant::now() + within;
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// What the program installed at `target` says it is (`--version`).
pub fn version_of(target: &Path) -> Result<String, String> {
    let program = program_of(target);
    let mut command = Command::new(&program);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::libraries::no_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("{} doesn't start: {e}", program.display()))?;
    // one short line, which the pipe holds until the program ends
    let status = wait_for(&mut child, VERSION_WITHIN)?.ok_or_else(|| {
        format!(
            "{} didn't answer --version within {} s",
            program.display(),
            VERSION_WITHIN.as_secs()
        )
    })?;
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut out);
    }
    if !status.success() {
        return Err(format!("{} --version failed ({status})", program.display()));
    }
    Ok(out.trim().to_owned())
}

/// Whether `version` (what `photonoxide --version` printed) is the build of `commit`: it names
/// the commit's first seven characters, as `0.5.1 (3064650)` or
/// `0.5.2-nightly (main @ 2a704d6, 2026-10-10)` does.
pub fn is_build(version: &str, commit: &str) -> bool {
    let short = short(commit);
    short.len() >= 7
        && version
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.eq_ignore_ascii_case(short))
}

/// Whether `a` and `b` are the same file (or `a` is inside the app bundle `b`).
pub fn same_place(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let fold = |p: PathBuf| {
        if cfg!(windows) {
            PathBuf::from(p.to_string_lossy().to_lowercase())
        } else {
            p
        }
    };
    let (a, b) = (fold(canon(a)), fold(canon(b)));
    a == b || a.starts_with(&b)
}

/// What happens after [`install`].
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// The new build is going in: this program exits now.
    Restart,
    /// A package for the package manager: the command that installs it, and the one `pkexec`
    /// would run, if there is pkexec.
    Package {
        command: String,
        pkexec: Option<String>,
    },
}

/// The package manager's command for a `.deb` or `.rpm`.
fn package_command(ready: &Ready) -> (String, Vec<String>) {
    let path = ready.artifact.display().to_string();
    if ready.bundle == "deb" {
        (
            format!("sudo apt install '{path}'"),
            vec!["dpkg".into(), "-i".into(), path],
        )
    } else {
        (
            format!("sudo dnf install '{path}'"),
            vec!["rpm".into(), "-U".into(), path],
        )
    }
}

/// Before a new install: an earlier one still under way stands down. Its watchdog is told so
/// in the handover, and stopped if it hasn't gone a moment later; a watchdog of a build before
/// 0.5.2, which reads only `started` and `rolled_back`, is given `rolled_back`, which it reads
/// within two seconds. An earlier install whose installer is running now is waited for.
pub fn supersede(dirs: &Dirs) -> Result<(), String> {
    let file = dirs.handover();
    let Some(mut h) = read_json::<Handover>(&file) else {
        return Ok(());
    };
    if !h.open() {
        return Ok(());
    }
    if h.id.is_empty() {
        h.rolled_back = true;
        write_json(&file, &h)?;
        log(
            dirs,
            "",
            "an earlier install (by a build before 0.5.2) stands down",
        );
        if now() < h.deadline {
            std::thread::sleep(Duration::from_millis(2500));
        }
        return Ok(());
    }
    let watchdog = h.watchdog_pid.filter(|&pid| process::alive(pid));
    if h.stage == Stage::Installing && watchdog.is_some() {
        return Err(format!(
            "photonoxide {} is being installed right now: wait for it to finish",
            h.to_label
        ));
    }
    h.set(Stage::Superseded);
    write_json(&file, &h)?;
    log(dirs, &h.id, "superseded by a new install");
    if let Some(pid) = watchdog
        && !process::wait_exit(pid, Duration::from_secs(3))
    {
        log(
            dirs,
            &h.id,
            &format!("stopping its watchdog (process {pid})"),
        );
        process::kill(pid);
    }
    Ok(())
}

/// Installs `ready` in place of this copy (`running`, called `from_label`). On Windows, macOS
/// and an AppImage it starts the watchdog (on Windows, it runs the installer once this program
/// has exited), and the caller exits; a package is left to the user (or [`install_package`]).
pub fn install(
    dirs: &Dirs,
    ready: &Ready,
    running: &Running,
    from_label: &str,
    from_commit: Option<&str>,
) -> Result<Outcome, String> {
    let needed = running.bundle_needed()?;
    if needed != ready.bundle {
        return Err(format!(
            "the build is a {} bundle, and this copy needs a {needed}: build it again",
            ready.bundle
        ));
    }
    if !ready.artifact.exists() {
        return Err(format!(
            "{} is gone: build it again",
            ready.artifact.display()
        ));
    }
    if needed == "deb" || needed == "rpm" {
        let (command, args) = package_command(ready);
        let pkexec = super::prereqs::System::here();
        let pkexec = super::prereqs::Probe::find(&pkexec, "pkexec")
            .map(|p| format!("{} {}", p.display(), args.join(" ")));
        return Ok(Outcome::Package { command, pkexec });
    }
    let target = running.target()?;
    supersede(dirs)?;
    let id = new_id();
    log(
        dirs,
        &id,
        &format!(
            "installing {} in place of {} at {}",
            ready.label,
            from_label,
            target.display()
        ),
    );
    let previous = keep_previous(dirs, &id, &target, from_label, from_commit)?;
    let nsis = needed == "nsis";
    let mut handover = Handover {
        id: id.clone(),
        stage: Stage::Waiting,
        to: ready.commit.clone(),
        to_label: ready.label.clone(),
        from_label: from_label.to_owned(),
        from_commit: from_commit.map(str::to_owned),
        from_pid: Some(std::process::id()),
        watchdog_pid: None,
        previous: previous.copy.clone(),
        target: target.clone(),
        installer: nsis.then(|| ready.artifact.clone()),
        // the watchdog sets the start's own deadline once the new build is in; until then,
        // one no watchdog of a build before 0.5.2 reaches while this install runs
        deadline: now() + EXIT_WITHIN + INSTALL_WITHIN + START_WITHIN,
        started: false,
        rolled_back: false,
        reason: None,
    };
    if !nsis {
        replace(&target, &ready.artifact)?;
        let found = version_of(&target);
        if !found.as_ref().is_ok_and(|v| is_build(v, &ready.commit)) {
            let put_back = replace(&target, &previous.copy);
            let found = found.unwrap_or_else(|e| e);
            log(dirs, &id, &format!("the new build isn't right: {found}"));
            return Err(match put_back {
                Ok(()) => format!(
                    "the new build didn't say it is {} ({found}): the build you have is back",
                    ready.label
                ),
                Err(e) => format!(
                    "the new build didn't say it is {} ({found}), and {e}",
                    ready.label
                ),
            });
        }
        handover.set(Stage::Installed);
        handover.deadline = now() + START_WITHIN;
    }
    write_json(&dirs.handover(), &handover)?;
    let _ = std::fs::remove_file(dirs.rollback_note());
    let watch = dirs.handover();
    let watchdog = launch(
        &program_of(&previous.copy),
        &["nightly".as_ref(), "watch".as_ref(), watch.as_os_str()],
    );
    let watchdog = match watchdog {
        Ok(pid) => pid,
        Err(e) => {
            // nothing goes on without the watchdog: on Windows nothing was replaced; elsewhere
            // the build that was running goes back
            if !nsis {
                let _ = replace(&target, &previous.copy);
            }
            handover.set(Stage::Failed);
            handover.reason = Some(e.clone());
            let _ = write_json(&dirs.handover(), &handover);
            log(dirs, &id, &e);
            return Err(e);
        }
    };
    log(dirs, &id, &format!("watchdog: process {watchdog}"));
    // the watchdog only reads the handover until this program has exited
    if let Some(mut h) = read_json::<Handover>(&dirs.handover()).filter(|h| h.id == id) {
        h.watchdog_pid = Some(watchdog);
        write_json(&dirs.handover(), &h)?;
    }
    if !nsis {
        launch(&target, &[])?;
    }
    Ok(Outcome::Restart)
}

/// The installer's arguments: passive (a progress bar, no questions) and as an update (no
/// uninstall first), as Tauri's updater runs it, into `dir` (`/D=`, which NSIS takes last and
/// unquoted). No `/R`: the watchdog opens the new build itself, once it has checked it.
pub fn installer_args(dir: &Path) -> String {
    format!("/P /UPDATE /D={}", dir.display())
}

/// Runs the NSIS installer and waits for it, up to `within`: its exit code, or none when it
/// didn't finish in time and was stopped. A per-user installer needs no elevation; one that
/// asks for it goes through the shell, which asks the user.
fn run_installer(installer: &Path, dir: &Path, within: Duration) -> Result<Option<i32>, String> {
    let args = installer_args(dir);
    #[cfg(windows)]
    let mut child = {
        use std::os::windows::process::CommandExt;
        const ELEVATION_REQUIRED: i32 = 740;
        let mut command = Command::new(installer);
        command.raw_arg(&args);
        quiet(&mut command);
        if let Some(folder) = installer.parent() {
            command.current_dir(folder);
        }
        match command.spawn() {
            Ok(child) => child,
            Err(e) if e.raw_os_error() == Some(ELEVATION_REQUIRED) => {
                // start /wait gives the installer's exit code back
                let mut shell = Command::new("cmd");
                shell.raw_arg(format!(
                    "/c start \"\" /wait \"{}\" {args}",
                    installer.display()
                ));
                crate::libraries::no_window(quiet(&mut shell));
                shell
                    .spawn()
                    .map_err(|e| format!("can't start {}: {e}", installer.display()))?
            }
            Err(e) => return Err(format!("can't start {}: {e}", installer.display())),
        }
    };
    #[cfg(not(windows))]
    let mut child = quiet(Command::new(installer).args(args.split(' ')))
        .spawn()
        .map_err(|e| format!("can't start {}: {e}", installer.display()))?;
    Ok(wait_for(&mut child, within)?.map(|s| s.code().unwrap_or(-1)))
}

/// What the installer's exit code says (NSIS: 0 done, 1 cancelled, 2 stopped on an error).
pub fn installer_says(code: Option<i32>) -> String {
    match code {
        None => format!("didn't finish within {} minutes", INSTALL_WITHIN / 60),
        Some(0) => "finished".into(),
        Some(1) => "was cancelled (exit code 1)".into(),
        Some(2) => "stopped on an error (exit code 2)".into(),
        Some(c) => format!("exited with code {c}"),
    }
}

/// Why an install didn't take: the installer's exit code, and what the program installed says
/// it is.
pub fn why_not(code: Option<i32>, target: &Path, found: &Result<String, String>) -> String {
    match found {
        Ok(v) => format!(
            "the installer {}, and {} is still {v}",
            installer_says(code),
            target.display()
        ),
        Err(e) => format!("the installer {}, and {e}", installer_says(code)),
    }
}

/// Reads the handover of install `id`, unless a later install took over.
fn this_install(file: &Path, id: &str) -> Option<Handover> {
    read_json::<Handover>(file).filter(|h| h.id == id)
}

/// The install failed: the handover and the note say why, and when `reopen`, the build that
/// was running opens again, to say it.
fn failed(dirs: &Dirs, file: &Path, id: &str, reason: String, reopen: bool) -> Result<(), String> {
    log(dirs, id, &format!("the install failed: {reason}"));
    let Some(mut h) = this_install(file, id) else {
        return Ok(());
    };
    h.set(Stage::Failed);
    h.reason = Some(reason.clone());
    write_json(file, &h)?;
    write_json(
        &dirs.rollback_note(),
        &Note {
            left: h.to_label.clone(),
            back_to: h.from_label.clone(),
            asked: false,
            failed: Some(reason),
        },
    )?;
    if reopen && let Err(e) = launch(&h.target, &[]) {
        log(dirs, id, &e);
    }
    Ok(())
}

/// Waits up to `within` for the program at `target` to be free, no process running from it.
fn wait_free(target: &Path, within: Duration) -> bool {
    // a program running on Windows can't be opened for writing (a sharing violation);
    // elsewhere it always can. Any other refusal (read-only, no rights) is the installer's
    // to meet, and to say
    const SHARING_VIOLATION: i32 = 32;
    let deadline = Instant::now() + within;
    loop {
        match std::fs::OpenOptions::new().append(true).open(target) {
            Err(e) if cfg!(windows) && e.raw_os_error() == Some(SHARING_VIOLATION) => {}
            _ => return true,
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// The watchdog's install, on Windows: once the program being replaced has exited, the
/// installer runs, and the program it installed is checked before it opens.
fn install_now(dirs: &Dirs, file: &Path, id: &str) -> Result<(), String> {
    let Some(h) = this_install(file, id) else {
        return Ok(());
    };
    let say = |line: &str| log(dirs, id, line);
    let Some(installer) = h.installer.clone() else {
        return Ok(());
    };
    if let Some(pid) = h.from_pid {
        say(&format!("waiting for photonoxide (process {pid}) to exit"));
        let begun = Instant::now();
        if !process::wait_exit(pid, Duration::from_secs(EXIT_WITHIN)) {
            return failed(
                dirs,
                file,
                id,
                format!(
                    "photonoxide (process {pid}) didn't exit within {EXIT_WITHIN} s, so nothing \
                     was installed"
                ),
                false,
            );
        }
        say(&format!(
            "it exited after {:.1} s",
            begun.elapsed().as_secs_f64()
        ));
    }
    if !wait_free(&h.target, Duration::from_secs(10)) {
        say(&format!(
            "{} is still in use (another photonoxide window?): the installer closes it",
            h.target.display()
        ));
    }
    let Some(mut h) = this_install(file, id).filter(|h| h.stage == Stage::Waiting) else {
        say("the install was taken over before the installer ran: standing down");
        return Ok(());
    };
    h.set(Stage::Installing);
    write_json(file, &h)?;
    let dir = h.target.parent().unwrap_or(Path::new("."));
    say(&format!(
        "running {} {}",
        installer.display(),
        installer_args(dir)
    ));
    let begun = Instant::now();
    let code = match run_installer(&installer, dir, Duration::from_secs(INSTALL_WITHIN)) {
        Ok(code) => code,
        Err(e) => return failed(dirs, file, id, e, true),
    };
    say(&format!(
        "the installer {} after {:.1} s",
        installer_says(code),
        begun.elapsed().as_secs_f64()
    ));
    let found = version_of(&h.target);
    say(&format!(
        "{} --version: {}",
        h.target.display(),
        match &found {
            Ok(v) => v.as_str(),
            Err(e) => e.as_str(),
        }
    ));
    if found.as_ref().is_ok_and(|v| is_build(v, &h.to)) {
        let Some(mut h) = this_install(file, id).filter(|h| h.stage == Stage::Installing) else {
            say("the new build is in, and started already");
            return Ok(());
        };
        h.set(Stage::Installed);
        h.deadline = now() + START_WITHIN;
        write_json(file, &h)?;
        match launch(&h.target, &[]) {
            Ok(pid) => say(&format!("the new build is in: opened it (process {pid})")),
            Err(e) => say(&format!("the new build is in, but {e}")),
        }
        return Ok(());
    }
    let reason = why_not(code, &h.target, &found);
    // the program there now isn't the build that was running either: that one goes back
    let unchanged = found
        .as_ref()
        .is_ok_and(|v| h.from_commit.as_deref().is_some_and(|c| is_build(v, c)));
    if !unchanged {
        match replace(&h.target, &h.previous) {
            Ok(()) => say("put the build that was running back"),
            Err(e) => say(&e),
        }
    }
    failed(dirs, file, id, reason, true)
}

/// Installs a `.deb` or `.rpm` with `pkexec` (which asks for the password), then restarts into
/// it. The window asked the user first.
pub fn install_package(ready: &Ready, running: &Running) -> Result<(), String> {
    let (_, args) = package_command(ready);
    let status = Command::new("pkexec")
        .args(&args)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("can't run pkexec: {e}"))?;
    if !status.success() {
        return Err(format!("pkexec {} failed ({status})", args.join(" ")));
    }
    launch(&running.exe, &[]).map(|_| ())
}

/// What the watchdog does next.
#[derive(Debug, PartialEq)]
pub enum Decision {
    Wait,
    /// The new build started, the install is over another way, or a later one took over:
    /// nothing to do.
    Done,
    Restore,
}

/// What the watchdog of install `id` does next, as the handover says now.
pub fn decide(h: &Handover, id: &str, now: u64) -> Decision {
    // a build before 0.5.2 confirms by `started` alone, and leaves out the rest
    if h.started || h.rolled_back || h.id != id {
        return Decision::Done;
    }
    match h.stage {
        Stage::Installed if now >= h.deadline => Decision::Restore,
        Stage::Waiting | Stage::Installing | Stage::Installed => Decision::Wait,
        _ => Decision::Done,
    }
}

/// The new build, `commit`, started: it confirms the install, so the watchdog stands down.
/// Returns whether there was an install to confirm.
pub fn started(dirs: &Dirs, commit: &str) -> bool {
    let file = dirs.handover();
    match read_json::<Handover>(&file) {
        Some(mut h) if h.to == commit && h.open() => {
            h.set(Stage::Started);
            log(dirs, &h.id, &format!("{} started", h.to_label));
            write_json(&file, &h).is_ok()
        }
        _ => false,
    }
}

/// At start, before the window: when an install is under way, this program is at its target
/// (`here`), and isn't the build being installed (`commit`), the install didn't take. The
/// watchdog stands down, and the note says so. An install whose watchdog is still waiting for
/// its installer is left to it.
pub fn at_start(
    dirs: &Dirs,
    commit: Option<&str>,
    here: Option<&Path>,
    label: &str,
) -> Option<Note> {
    let file = dirs.handover();
    let mut h = read_json::<Handover>(&file)?;
    if h.id.is_empty() || !h.open() || commit == Some(h.to.as_str()) {
        return None;
    }
    if !here.is_some_and(|p| same_place(p, &h.target)) {
        return None;
    }
    if matches!(h.stage, Stage::Waiting | Stage::Installing)
        && h.watchdog_pid.is_some_and(process::alive)
    {
        return None;
    }
    let reason = format!("photonoxide started as {label}, not as the new build");
    log(dirs, &h.id, &format!("the install didn't take: {reason}"));
    h.set(Stage::Failed);
    h.reason = Some(reason.clone());
    write_json(&file, &h).ok()?;
    Some(Note {
        left: h.to_label,
        back_to: label.to_owned(),
        asked: false,
        failed: Some(reason),
    })
}

/// Puts the previous build back at `target`, says so in the note, and opens it.
fn go_back(dirs: &Dirs, previous: &Path, target: &Path, note: &Note) -> Result<(), String> {
    if !note.asked {
        // the build that didn't confirm may still be running, its window stuck
        close(target);
    }
    replace(target, previous)?;
    write_json(&dirs.rollback_note(), note)?;
    launch(target, &[]).map(|_| ())
}

/// Closes the programs running from `target`: by its file name on Windows (the watchdog's copy
/// has another), by its path elsewhere.
fn close(target: &Path) {
    let mut command = if cfg!(windows) {
        let mut c = Command::new("taskkill");
        c.args(["/F", "/IM"])
            .arg(target.file_name().unwrap_or_default());
        c
    } else {
        let mut c = Command::new("pkill");
        c.arg("-f").arg(target);
        c
    };
    crate::libraries::no_window(quiet(&mut command));
    let _ = command.status();
    std::thread::sleep(Duration::from_secs(1));
}

/// `photonoxide nightly watch <handover>`: on Windows, installs once this program has exited;
/// then waits for the new build to start, and goes back to the previous one when it doesn't in
/// time.
pub fn watch(dirs: &Dirs, file: &Path) -> Result<Decision, String> {
    let first: Handover =
        read_json(file).ok_or_else(|| format!("{} doesn't read", file.display()))?;
    let id = first.id.clone();
    log(
        dirs,
        &id,
        &format!("watchdog started (process {})", std::process::id()),
    );
    if first.stage == Stage::Waiting && first.installer.is_some() {
        install_now(dirs, file, &id)?;
    }
    loop {
        let Some(mut h) = read_json::<Handover>(file) else {
            log(dirs, &id, "the handover is gone: standing down");
            return Ok(Decision::Done);
        };
        match decide(&h, &id, now()) {
            Decision::Wait => std::thread::sleep(Duration::from_secs(1)),
            Decision::Done => {
                let why = if h.started {
                    "the new build started".to_owned()
                } else if h.id != id {
                    "a later install took over".to_owned()
                } else {
                    serde_json::to_string(&h.stage).unwrap_or_default()
                };
                log(dirs, &id, &format!("watchdog done: {why}"));
                return Ok(Decision::Done);
            }
            Decision::Restore => {
                log(
                    dirs,
                    &id,
                    &format!(
                        "{} didn't start within {START_WITHIN} s: going back to {}",
                        h.to_label, h.from_label
                    ),
                );
                h.set(Stage::RolledBack);
                write_json(file, &h)?;
                let note = Note {
                    left: h.to_label.clone(),
                    back_to: h.from_label.clone(),
                    asked: false,
                    failed: None,
                };
                go_back(dirs, &h.previous, &h.target, &note)?;
                return Ok(Decision::Restore);
            }
        }
    }
}

/// `photonoxide nightly restore`: the user asked to go back to the previous build. Run from
/// the previous build's copy, a moment after the window closed.
pub fn restore(dirs: &Dirs, leaving: &str) -> Result<(), String> {
    let previous: Previous =
        read_json(&dirs.previous_file()).ok_or("there is no previous build to go back to")?;
    std::thread::sleep(Duration::from_secs(2));
    // an install still under way is over: its watchdog stands down
    let file = dirs.handover();
    if let Some(mut h) = read_json::<Handover>(&file).filter(Handover::open) {
        h.set(Stage::RolledBack);
        write_json(&file, &h)?;
        log(dirs, &h.id, "going back to the previous build, as asked");
    }
    let note = Note {
        left: leaving.to_owned(),
        back_to: previous.label.clone(),
        asked: true,
        failed: None,
    };
    go_back(dirs, &previous.copy, &previous.target, &note)
}

/// The previous build, if one is kept and still there.
pub fn previous(dirs: &Dirs) -> Option<Previous> {
    read_json::<Previous>(&dirs.previous_file()).filter(|p| p.copy.exists())
}

/// Starts going back to the previous build: the caller exits next.
pub fn roll_back(dirs: &Dirs, leaving: &str) -> Result<(), String> {
    let previous = previous(dirs).ok_or("there is no previous build to go back to")?;
    launch(
        &program_of(&previous.copy),
        &["nightly".as_ref(), "restore".as_ref(), leaving.as_ref()],
    )
    .map(|_| ())
}

/// What the last rollback or failed install did, once: the note is removed as it is read.
pub fn take_note(dirs: &Dirs) -> Option<Note> {
    let file = dirs.rollback_note();
    let note = read_json(&file);
    let _ = std::fs::remove_file(&file);
    note
}

#[cfg(test)]
mod tests {
    use super::*;

    const TO: &str = "2a704d6f45d84939b13483efc4314daa11c6fb6d";
    const FROM: &str = "3064650f81d2fa979713ad0ab9c21c9d61d7e639";

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-install-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn running(bundle: Option<&str>, exe: &str) -> Running {
        Running {
            exe: PathBuf::from(exe),
            bundle: bundle.map(str::to_owned),
            appimage: Some(PathBuf::from("/home/me/Apps/photonoxide.AppImage")),
        }
    }

    fn handover(id: &str, stage: Stage) -> Handover {
        let mut h = Handover {
            id: id.into(),
            stage,
            to: TO.into(),
            to_label: "0.5.2-nightly (main @ 2a704d6, 2026-10-10)".into(),
            from_label: "0.5.1".into(),
            from_commit: Some(FROM.into()),
            from_pid: None,
            watchdog_pid: None,
            previous: "p".into(),
            target: "t".into(),
            installer: None,
            deadline: 100,
            started: false,
            rolled_back: false,
            reason: None,
        };
        h.set(stage);
        h
    }

    /// A handover as a build before 0.5.2 writes it.
    fn old_handover(to: &str, target: &Path, started: bool) -> String {
        format!(
            r#"{{"to":"{to}","to_label":"","from_label":"","previous":"p","target":{},
                "deadline":0,"started":{started},"rolled_back":false}}"#,
            serde_json::to_string(target).unwrap()
        )
    }

    /// A child that runs for about `seconds`, reaped by a thread of its own once it ends, so
    /// it doesn't linger as a zombie on Unix.
    fn sleeper(seconds: u32) -> u32 {
        let mut command = if cfg!(windows) {
            let mut c = Command::new("ping");
            c.args(["-n", &(seconds + 1).to_string(), "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg(seconds.to_string());
            c
        };
        let mut child = quiet(&mut command).spawn().unwrap();
        let pid = child.id();
        std::thread::spawn(move || child.wait());
        pid
    }

    #[test]
    fn each_install_needs_its_own_bundle_and_replaces_its_own_thing() {
        let r = running(
            Some("nsis"),
            r"C:\Users\me\AppData\Local\photonoxide\photonoxide.exe",
        );
        assert_eq!(r.bundle_needed(), Ok("nsis"));
        assert_eq!(r.target().unwrap(), r.exe);
        let r = running(
            Some("app"),
            "/Applications/photonoxide.app/Contents/MacOS/photonoxide",
        );
        assert_eq!(
            r.target().unwrap(),
            PathBuf::from("/Applications/photonoxide.app")
        );
        assert_eq!(running(Some("dmg"), "/x").bundle_needed(), Ok("app"));
        assert!(
            running(Some("app"), "/usr/bin/photonoxide")
                .target()
                .is_err()
        );
        let r = running(Some("appimage"), "/tmp/.mount_x/usr/bin/photonoxide");
        assert_eq!(
            r.target().unwrap(),
            PathBuf::from("/home/me/Apps/photonoxide.AppImage")
        );
        assert_eq!(
            running(Some("deb"), "/usr/bin/photonoxide").bundle_needed(),
            Ok("deb")
        );
        assert!(
            running(Some("msi"), "x")
                .bundle_needed()
                .unwrap_err()
                .contains("msi package")
        );
        assert!(
            running(None, "x")
                .bundle_needed()
                .unwrap_err()
                .contains("built from the repository")
        );
    }

    #[test]
    fn the_installer_runs_passive_as_an_update_into_this_copy_s_folder() {
        let dir = Path::new(r"C:\Users\me\AppData\Local\photo noxide");
        // /D= last and unquoted, even with a space; no /R: the watchdog opens the build itself
        assert_eq!(
            installer_args(dir),
            r"/P /UPDATE /D=C:\Users\me\AppData\Local\photo noxide"
        );
        assert_eq!(installer_says(Some(0)), "finished");
        assert!(installer_says(Some(2)).contains("error (exit code 2)"));
        assert!(installer_says(Some(1)).contains("cancelled"));
        assert!(installer_says(None).contains("didn't finish within 10 minutes"));
    }

    #[test]
    fn a_build_is_known_by_the_commit_its_version_names() {
        assert!(is_build("photonoxide 0.5.1 (3064650)", FROM));
        assert!(is_build(
            "photonoxide 0.5.2-nightly (main @ 2a704d6, 2026-10-10)",
            TO
        ));
        assert!(!is_build("photonoxide 0.5.1 (3064650)", TO));
        // a release that doesn't know its commit, and a version that only begins with it
        assert!(!is_build("photonoxide 0.5.1", FROM));
        assert!(!is_build("photonoxide 0.5.1 (3064650f)", FROM));
        assert!(!is_build("anything", ""));
    }

    #[test]
    fn the_watchdog_waits_stands_down_or_goes_back() {
        let id = "1-2";
        // the installer's turn: the watchdog itself is at work, and there is no rollback yet
        assert_eq!(
            decide(&handover(id, Stage::Waiting), id, 1000),
            Decision::Wait
        );
        assert_eq!(
            decide(&handover(id, Stage::Installing), id, 1000),
            Decision::Wait
        );
        // installed: the new build has until the deadline
        let h = handover(id, Stage::Installed);
        assert_eq!(decide(&h, id, 99), Decision::Wait);
        assert_eq!(decide(&h, id, 100), Decision::Restore);
        // over: started, rolled back, failed, or taken over by a later install
        for stage in [
            Stage::Started,
            Stage::RolledBack,
            Stage::Failed,
            Stage::Superseded,
        ] {
            assert_eq!(decide(&handover(id, stage), id, 1000), Decision::Done);
        }
        assert_eq!(decide(&h, "3-4", 1000), Decision::Done);
        // a build before 0.5.2 confirms with `started` alone, leaving the rest out
        let old: Handover = serde_json::from_str(&old_handover(TO, Path::new("t"), true)).unwrap();
        assert_eq!(decide(&old, id, 1000), Decision::Done);
    }

    #[test]
    fn the_stage_keeps_what_older_builds_read() {
        let h = handover("1-2", Stage::Installed);
        assert!(!h.started && !h.rolled_back && h.open());
        let h = handover("1-2", Stage::Started);
        assert!(h.started && !h.rolled_back && !h.open());
        for stage in [Stage::RolledBack, Stage::Failed, Stage::Superseded] {
            let h = handover("1-2", stage);
            assert!(!h.started && h.rolled_back && !h.open(), "{stage:?}");
        }
        // the fields a build before 0.5.2 needs to read the handover are all there
        let json = serde_json::to_value(handover("1-2", Stage::Waiting)).unwrap();
        for key in [
            "to",
            "to_label",
            "from_label",
            "previous",
            "target",
            "deadline",
            "started",
            "rolled_back",
        ] {
            assert!(json.get(key).is_some(), "{key}");
        }
    }

    #[test]
    fn the_new_build_confirms_only_its_own_install() {
        let dir = temp("started");
        let dirs = Dirs { root: dir.clone() };
        assert!(!started(&dirs, TO));
        write_json(&dirs.handover(), &handover("1-2", Stage::Installed)).unwrap();
        assert!(!started(&dirs, FROM));
        assert!(started(&dirs, TO));
        let h = read_json::<Handover>(&dirs.handover()).unwrap();
        assert_eq!((h.stage, h.started), (Stage::Started, true));
        // once is enough
        assert!(!started(&dirs, TO));
        // a failed install isn't confirmed
        write_json(&dirs.handover(), &handover("1-2", Stage::Failed)).unwrap();
        assert!(!started(&dirs, TO));
        // an install by a build before 0.5.2 is confirmed as its watchdog expects
        std::fs::write(dirs.handover(), old_handover(TO, Path::new("t"), false)).unwrap();
        assert!(started(&dirs, TO));
        let old: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dirs.handover()).unwrap()).unwrap();
        assert_eq!(old["started"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_program_that_starts_as_another_build_knows_the_install_didn_t_take() {
        let dir = temp("at-start");
        let dirs = Dirs { root: dir.clone() };
        let target = dir.join("photonoxide.exe");
        std::fs::write(&target, "0.5.1").unwrap();
        let mut h = handover("1-2", Stage::Installed);
        h.target = target.clone();
        write_json(&dirs.handover(), &h).unwrap();
        // the new build: its window confirms
        assert_eq!(at_start(&dirs, Some(TO), Some(&target), "x"), None);
        // another copy, elsewhere
        assert_eq!(
            at_start(&dirs, Some(FROM), Some(&dir.join("other.exe")), "0.5.1"),
            None
        );
        // the installer still at work, its watchdog running: left to it
        let mut busy = h.clone();
        busy.set(Stage::Installing);
        busy.watchdog_pid = Some(std::process::id());
        write_json(&dirs.handover(), &busy).unwrap();
        assert_eq!(at_start(&dirs, Some(FROM), Some(&target), "0.5.1"), None);
        // the build that was there before, at the target: the install didn't take
        write_json(&dirs.handover(), &h).unwrap();
        let note = at_start(&dirs, Some(FROM), Some(&target), "0.5.1").unwrap();
        assert_eq!(note.back_to, "0.5.1");
        assert!(note.left.starts_with("0.5.2-nightly"));
        assert!(note.failed.unwrap().contains("started as 0.5.1"));
        let after = read_json::<Handover>(&dirs.handover()).unwrap();
        assert_eq!(after.stage, Stage::Failed);
        // so the watchdog stands down
        assert_eq!(decide(&after, "1-2", 0), Decision::Done);
        // and it's said once
        assert_eq!(at_start(&dirs, Some(FROM), Some(&target), "0.5.1"), None);
        // a handover a build before 0.5.2 left is its own watchdog's business
        std::fs::write(dirs.handover(), old_handover(TO, &target, false)).unwrap();
        assert_eq!(at_start(&dirs, Some(FROM), Some(&target), "0.5.1"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn processes_are_waited_for_and_stopped_by_their_id() {
        let pid = sleeper(30);
        assert!(process::alive(pid));
        let begun = Instant::now();
        assert!(!process::wait_exit(pid, Duration::from_millis(300)));
        assert!(begun.elapsed() >= Duration::from_millis(250));
        process::kill(pid);
        assert!(process::wait_exit(pid, Duration::from_secs(5)));
        assert!(!process::alive(pid));
        let short = sleeper(1);
        assert!(process::wait_exit(short, Duration::from_secs(10)));
    }

    #[test]
    fn a_new_install_supersedes_an_earlier_one_still_under_way() {
        let dir = temp("supersede");
        let dirs = Dirs { root: dir.clone() };
        // nothing under way
        supersede(&dirs).unwrap();
        // an earlier install's watchdog still running, which doesn't stand down by itself
        let watchdog = sleeper(60);
        let mut h = handover("1-2", Stage::Installed);
        h.watchdog_pid = Some(watchdog);
        write_json(&dirs.handover(), &h).unwrap();
        supersede(&dirs).unwrap();
        let after = read_json::<Handover>(&dirs.handover()).unwrap();
        assert_eq!(after.stage, Stage::Superseded);
        assert!(after.rolled_back, "a build before 0.5.2 reads this");
        assert!(!process::alive(watchdog), "the watchdog was stopped");
        // an installer running now is waited for
        let busy = sleeper(60);
        let mut h = handover("5-6", Stage::Installing);
        h.watchdog_pid = Some(busy);
        write_json(&dirs.handover(), &h).unwrap();
        assert!(
            supersede(&dirs)
                .unwrap_err()
                .contains("being installed right now")
        );
        // unless its watchdog is gone
        process::kill(busy);
        supersede(&dirs).unwrap();
        // an install by a build before 0.5.2: its watchdog reads `rolled_back`
        std::fs::write(dirs.handover(), old_handover(TO, Path::new("t"), false)).unwrap();
        supersede(&dirs).unwrap();
        let old: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dirs.handover()).unwrap()).unwrap();
        assert_eq!(old["rolled_back"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_install_keeps_its_own_copy_and_a_locked_earlier_one_is_no_obstacle() {
        let dir = temp("keep");
        let dirs = Dirs {
            root: dir.join("nightly"),
        };
        let target = dir.join("installed").join("photonoxide.exe");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "0.5.1").unwrap();
        let first = keep_previous(&dirs, "1-2", &target, "0.5.1", Some(FROM)).unwrap();
        assert_eq!(
            first.copy,
            dirs.previous().join("1-2").join("photonoxide-previous.exe")
        );
        // the earlier copy is in use, as by its watchdog (on Windows, a file open without
        // sharing is as locked as a program running from it)
        #[cfg(windows)]
        let lock = {
            use std::os::windows::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&first.copy)
                .unwrap()
        };
        std::fs::write(&target, "0.5.2-nightly").unwrap();
        let second = keep_previous(&dirs, "3-4", &target, "0.5.2-nightly", Some(TO)).unwrap();
        assert_eq!(
            std::fs::read_to_string(&second.copy).unwrap(),
            "0.5.2-nightly"
        );
        assert_eq!(previous(&dirs), Some(second.clone()));
        #[cfg(windows)]
        {
            assert!(first.copy.exists(), "still locked: it goes next time");
            drop(lock);
        }
        // the next install clears what it can
        let third = keep_previous(&dirs, "5-6", &target, "0.5.2-nightly", Some(TO)).unwrap();
        let left: Vec<_> = std::fs::read_dir(dirs.previous())
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left, ["5-6"]);
        assert!(third.copy.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_installer_waits_only_for_a_program_in_use() {
        let dir = temp("free");
        let program = dir.join("photonoxide.exe");
        std::fs::write(&program, "0.5.1").unwrap();
        assert!(wait_free(&program, Duration::ZERO));
        assert!(wait_free(&dir.join("gone.exe"), Duration::ZERO));
        // read-only: the installer's to meet, at once
        let mut permissions = std::fs::metadata(&program).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&program, permissions.clone()).unwrap();
        let begun = Instant::now();
        assert!(wait_free(&program, Duration::from_secs(5)));
        assert!(begun.elapsed() < Duration::from_secs(1));
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        std::fs::set_permissions(&program, permissions).unwrap();
        // in use, as a program running from it is on Windows
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let lock = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&program)
                .unwrap();
            assert!(!wait_free(&program, Duration::from_millis(300)));
            drop(lock);
            assert!(wait_free(&program, Duration::ZERO));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_program_and_an_app_folder_are_replaced_and_kept_and_put_back() {
        let dir = temp("replace");
        let dirs = Dirs {
            root: dir.join("nightly"),
        };
        // a program, as on Windows or an AppImage
        let target = dir.join("installed").join("photonoxide.exe");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "0.5.0").unwrap();
        let previous = keep_previous(&dirs, "1-2", &target, "0.5.0", None).unwrap();
        assert_eq!(
            previous.copy.file_name().unwrap(),
            "photonoxide-previous.exe"
        );
        assert_eq!(super::previous(&dirs), Some(previous.clone()));
        let new = dir.join("photonoxide_0.5.1-nightly.exe");
        std::fs::write(&new, "nightly").unwrap();
        replace(&target, &new).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "nightly");
        let leftovers: Vec<_> = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(leftovers, ["photonoxide.exe"]);
        // back to the previous one
        replace(&previous.target, &previous.copy).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "0.5.0");
        // an app bundle, a folder
        let app = dir.join("Applications").join("photonoxide.app");
        std::fs::create_dir_all(app.join("Contents").join("MacOS")).unwrap();
        std::fs::write(
            app.join("Contents").join("MacOS").join("photonoxide"),
            "0.5.0",
        )
        .unwrap();
        let kept = keep_previous(&dirs, "3-4", &app, "0.5.0", None).unwrap();
        assert_eq!(kept.copy.file_name().unwrap(), "photonoxide.app");
        assert_eq!(
            program_of(&kept.copy),
            kept.copy.join("Contents").join("MacOS").join("photonoxide")
        );
        assert!(same_place(
            &app.join("Contents").join("MacOS").join("photonoxide"),
            &app
        ));
        let built = dir.join("built").join("photonoxide.app");
        std::fs::create_dir_all(built.join("Contents").join("MacOS")).unwrap();
        std::fs::write(
            built.join("Contents").join("MacOS").join("photonoxide"),
            "nightly",
        )
        .unwrap();
        replace(&app, &built).unwrap();
        assert_eq!(
            std::fs::read_to_string(app.join("Contents/MacOS/photonoxide")).unwrap(),
            "nightly"
        );
        replace(&app, &kept.copy).unwrap();
        assert_eq!(
            std::fs::read_to_string(app.join("Contents/MacOS/photonoxide")).unwrap(),
            "0.5.0"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Makes `path` a script, runnable on Unix.
    fn script(path: &Path, text: &str) {
        std::fs::write(path, text).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    /// A program that says `photonoxide <version>` when asked.
    fn program(path: &Path, version: &str) {
        if cfg!(windows) {
            script(path, &format!("@echo photonoxide {version}\r\n"));
        } else {
            script(path, &format!("#!/bin/sh\necho 'photonoxide {version}'\n"));
        }
    }

    /// An installer that puts `new` at `target`, if given, and exits with `code`.
    fn installer(path: &Path, new: Option<(&Path, &Path)>, code: i32) {
        if cfg!(windows) {
            let copy = new.map_or(String::new(), |(new, target)| {
                format!(
                    "@copy /y \"{}\" \"{}\" >nul\r\n",
                    new.display(),
                    target.display()
                )
            });
            script(path, &format!("{copy}@exit /b {code}\r\n"));
        } else {
            let copy = new.map_or(String::new(), |(new, target)| {
                format!("cp '{}' '{}'\n", new.display(), target.display())
            });
            script(path, &format!("#!/bin/sh\n{copy}exit {code}\n"));
        }
    }

    /// The watchdog's install, with scripts for the program and its installer, as `case` has
    /// it: the new build put in place, nothing installed, or a broken program left there.
    fn watched_install(case: &str, code: i32) -> (Dirs, Handover, String) {
        let dir = temp(&format!("watch-{case}"));
        let dirs = Dirs {
            root: dir.join("nightly"),
        };
        let ext = if cfg!(windows) { "cmd" } else { "sh" };
        let target = dir.join("installed").join(format!("photonoxide.{ext}"));
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        program(&target, "0.5.1 (3064650)");
        let id = "1-2";
        let previous = keep_previous(&dirs, id, &target, "0.5.1", Some(FROM)).unwrap();
        let new = dir.join(format!("new.{ext}"));
        if case == "broken" {
            program(&new, "something else");
        } else {
            program(&new, "0.5.2-nightly (main @ 2a704d6, 2026-10-10)");
        }
        let setup = dir.join(format!("setup.{ext}"));
        installer(
            &setup,
            (case != "nothing").then_some((new.as_path(), target.as_path())),
            code,
        );
        let mut h = handover(id, Stage::Waiting);
        h.previous = previous.copy;
        h.target = target;
        h.installer = Some(setup);
        // the program being replaced, which exits a moment later
        h.from_pid = Some(sleeper(1));
        write_json(&dirs.handover(), &h).unwrap();
        install_now(&dirs, &dirs.handover(), id).unwrap();
        let after = read_json::<Handover>(&dirs.handover()).unwrap();
        let log = std::fs::read_to_string(dirs.install_log()).unwrap();
        (dirs, after, log)
    }

    #[test]
    fn the_watchdog_installs_after_the_program_exits_and_checks_the_build_before_opening_it() {
        let (dirs, h, log) = watched_install("installed", 0);
        assert_eq!(h.stage, Stage::Installed, "{log}");
        assert!(h.deadline + 5 >= now() + START_WITHIN);
        assert!(log.contains("waiting for photonoxide (process"), "{log}");
        assert!(log.contains("it exited after"), "{log}");
        assert!(log.contains("/P /UPDATE /D="), "{log}");
        assert!(log.contains("the installer finished"), "{log}");
        assert!(log.contains("the new build is in: opened it"), "{log}");
        assert_eq!(take_note(&dirs), None);
        let _ = std::fs::remove_dir_all(dirs.root.parent().unwrap());
    }

    #[test]
    fn a_failed_install_is_said_with_its_reason_and_nothing_pretends_it_updated() {
        // the installer stops on an error, having replaced nothing
        let (dirs, h, log) = watched_install("nothing", 2);
        assert_eq!(h.stage, Stage::Failed, "{log}");
        let reason = h.reason.clone().unwrap();
        assert!(
            reason.contains("stopped on an error (exit code 2)"),
            "{reason}"
        );
        assert!(
            reason.contains("is still photonoxide 0.5.1 (3064650)"),
            "{reason}"
        );
        assert!(
            !log.contains("put the build that was running back"),
            "{log}"
        );
        let note = take_note(&dirs).unwrap();
        assert_eq!(note.failed.as_deref(), Some(reason.as_str()));
        assert_eq!((note.back_to.as_str(), note.asked), ("0.5.1", false));
        assert!(
            h.rolled_back && !h.started,
            "older watchdogs stand down too"
        );
        assert_eq!(
            version_of(&h.target).unwrap(),
            "photonoxide 0.5.1 (3064650)"
        );
        let _ = std::fs::remove_dir_all(dirs.root.parent().unwrap());
        // the installer says it finished, but leaves a program that isn't the new build: the
        // build that was running goes back
        let (dirs, h, log) = watched_install("broken", 0);
        assert_eq!(h.stage, Stage::Failed, "{log}");
        assert!(h.reason.unwrap().contains("the installer finished, and"));
        assert!(log.contains("put the build that was running back"), "{log}");
        assert_eq!(
            version_of(&h.target).unwrap(),
            "photonoxide 0.5.1 (3064650)"
        );
        let _ = std::fs::remove_dir_all(dirs.root.parent().unwrap());
    }

    #[test]
    fn a_package_is_the_user_s_to_install() {
        let ready = Ready {
            commit: "c".into(),
            date: None,
            version: "0.5.1-nightly".into(),
            label: "l".into(),
            bundle: "deb".into(),
            artifact: "/data/ready/photonoxide_0.5.1-nightly_amd64.deb".into(),
            built_at: 0,
            seconds: 0.0,
        };
        let (command, args) = package_command(&ready);
        assert_eq!(
            command,
            "sudo apt install '/data/ready/photonoxide_0.5.1-nightly_amd64.deb'"
        );
        assert_eq!(args[..2], ["dpkg", "-i"]);
        let rpm = Ready {
            bundle: "rpm".into(),
            ..ready.clone()
        };
        assert!(package_command(&rpm).0.starts_with("sudo dnf install"));
        // a bundle of another kind than this copy needs is refused
        let dirs = Dirs {
            root: temp("mismatch"),
        };
        let e = install(
            &dirs,
            &ready,
            &running(Some("nsis"), "x.exe"),
            "0.5.0",
            None,
        )
        .unwrap_err();
        assert!(e.contains("needs a nsis"), "{e}");
        let _ = std::fs::remove_dir_all(&dirs.root);
    }

    #[test]
    fn a_rollback_note_is_said_once() {
        let dirs = Dirs { root: temp("note") };
        let note = Note {
            left: "nightly".into(),
            back_to: "0.5.0".into(),
            asked: false,
            failed: None,
        };
        write_json(&dirs.rollback_note(), &note).unwrap();
        assert_eq!(take_note(&dirs), Some(note));
        assert_eq!(take_note(&dirs), None);
        // a note an older build wrote reads
        std::fs::write(
            dirs.rollback_note(),
            r#"{"left":"a","back_to":"b","asked":true}"#,
        )
        .unwrap();
        assert_eq!(take_note(&dirs).unwrap().failed, None);
        let _ = std::fs::remove_dir_all(&dirs.root);
    }
}
