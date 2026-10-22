//! Building main at a commit, here: its tarball downloaded from GitHub and unpacked, the
//! prerequisites checked against what that source asks for, then `pnpm install
//! --frozen-lockfile` and `pnpm tauri build` (always the Tauri CLI, never plain cargo) with the
//! updater's artifacts off (a local build has no signing key) and only the bundle the update
//! needs. It runs on a thread of its own, its log kept line by line for the window to poll,
//! and stops when cancelled or at its hard timeout.
//!
//! The source goes to one folder, `source/`, emptied before each commit is unpacked there, and
//! Cargo's target folder, `target/`, is kept: the next build compiles only what changed. The
//! files are unpacked with the time of unpacking, not the commit's, so Cargo never takes a
//! newer commit's file for one it built before.

use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use super::compat::{self, TooOld};
use super::github::{self, Http, short};
use super::prereqs::{self, Needs};

/// The nightly channel's folder, in the app's local data folder, and what it holds.
#[derive(Clone, Debug)]
pub struct Dirs {
    pub root: PathBuf,
}

impl Dirs {
    /// The source being built: one commit's at a time.
    pub fn source(&self) -> PathBuf {
        self.root.join("source")
    }
    /// Cargo's target folder, kept between builds.
    pub fn target(&self) -> PathBuf {
        self.root.join("target")
    }
    pub fn downloads(&self) -> PathBuf {
        self.root.join("download")
    }
    /// The build waiting to be installed, and what it is.
    pub fn ready(&self) -> PathBuf {
        self.root.join("ready")
    }
    pub fn ready_file(&self) -> PathBuf {
        self.root.join("ready.json")
    }
    /// A folder for a build of `commit` made at `when` (milliseconds since 1970), in
    /// [`Dirs::ready`]: each build its own, so none is in the way of another.
    pub fn ready_for(&self, commit: &str, when: u128) -> PathBuf {
        self.ready().join(format!("{}-{when}", short(commit)))
    }
    /// The build that was running before the last install, to go back to.
    pub fn previous(&self) -> PathBuf {
        self.root.join("previous")
    }
    pub fn previous_file(&self) -> PathBuf {
        self.root.join("previous.json")
    }
    /// The install in progress, which the new build confirms by starting.
    pub fn handover(&self) -> PathBuf {
        self.root.join("handover.json")
    }
    /// What the last rollback or failed install did, for the window to say once.
    pub fn rollback_note(&self) -> PathBuf {
        self.root.join("rollback.json")
    }
    /// Each install's steps, as the watchdog took them.
    pub fn install_log(&self) -> PathBuf {
        self.root.join("install.log")
    }
    /// The check's memory.
    pub fn check_file(&self) -> PathBuf {
        self.root.join("check.json")
    }
    /// The last build's log.
    pub fn log(&self) -> PathBuf {
        self.root.join("build.log")
    }
    /// How many crates the last build compiled, for the next one's progress.
    pub fn last_build(&self) -> PathBuf {
        self.root.join("last-build.json")
    }
}

/// Reads `T` from the JSON in `file`, if there is one that reads.
pub fn read_json<T: for<'de> Deserialize<'de>>(file: &Path) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

/// Writes `value` as JSON to `file`, through a file beside it, so a reader never sees half.
pub fn write_json<T: Serialize>(file: &Path, value: &T) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let partial = file.with_extension("json.partial");
    std::fs::write(&partial, text).map_err(|e| format!("{}: {e}", partial.display()))?;
    std::fs::rename(&partial, file).map_err(|e| format!("{}: {e}", file.display()))
}

/// The name, identifier and binary the build takes: this program's, so the build replaces
/// this copy and keeps its settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Identity {
    pub identifier: String,
    pub product_name: Option<String>,
    pub main_binary_name: Option<String>,
}

/// What to build.
pub struct Plan {
    /// main's head, as the user saw it.
    pub commit: String,
    /// Its date, `2026-10-09`, when known.
    pub date: Option<String>,
    /// The bundle the update needs: `nsis`, `app`, `appimage`, `deb` or `rpm`.
    pub bundle: String,
    pub identity: Identity,
    /// PATH for the build's processes.
    pub path: OsString,
    /// The hard timeout of the whole build.
    pub timeout: Duration,
}

/// A build ready to install.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ready {
    pub commit: String,
    pub date: Option<String>,
    /// Its version, e.g. `0.5.4+nightly.20261017.4cc3c39`.
    pub version: String,
    /// What Settings calls it, e.g.
    /// `0.5.4+nightly.20261017.4cc3c39 (main @ 4cc3c39, 2026-10-17)`.
    pub label: String,
    pub bundle: String,
    /// The installer, app bundle, AppImage or package.
    pub artifact: PathBuf,
    /// When it was built, seconds since 1970, and how long it took.
    pub built_at: u64,
    pub seconds: f64,
}

/// The build's state, as the window polls it.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Progress {
    pub commit: String,
    /// The commit's date, `2026-10-09`, when known.
    pub date: Option<String>,
    /// `download`, `unpack`, `check`, `install`, `frontend`, `compile`, `bundle`, then `done`,
    /// `failed` or `cancelled`.
    pub phase: String,
    /// What it is doing, in words.
    pub doing: String,
    /// The log's lines from the `from`-th on, and how many there are in all.
    pub lines: Vec<String>,
    pub line_count: usize,
    pub seconds: f64,
    pub downloaded: u64,
    pub size: Option<u64>,
    /// Crates compiled so far, and how many the last build compiled, when there was one.
    pub compiled: u32,
    pub expected: Option<u32>,
    pub timeout_minutes: u64,
    pub finished: bool,
    pub error: Option<String>,
    /// The build failed because this copy's builder is too old for main, and what to do.
    pub too_old: Option<TooOld>,
    pub ready: Option<Ready>,
}

struct Shared {
    progress: Progress,
    lines: Vec<String>,
    started: Instant,
    log: Option<std::fs::File>,
}

impl Shared {
    fn phase(&mut self, phase: &str, doing: String) {
        self.progress.phase = phase.into();
        self.say(format!("== {doing}"));
        self.progress.doing = doing;
    }

    fn say(&mut self, line: String) {
        if let Some(log) = &mut self.log {
            let _ = writeln!(log, "{line}");
        }
        self.lines.push(line);
    }

    /// A line of a step's output, which also tells where the build is.
    fn line(&mut self, raw: &str) {
        let line = plain(raw);
        let t = line.trim_start();
        if t.starts_with("Running beforeBuildCommand") {
            self.progress.phase = "frontend".into();
            self.progress.doing = "Building the window: svelte-check and Vite".into();
        } else if let Some(krate) = t.strip_prefix("Compiling ") {
            self.progress.phase = "compile".into();
            self.progress.compiled += 1;
            self.progress.doing = format!("Compiling {}", krate.split(' ').next().unwrap_or(""));
        } else if t.starts_with("Finished ")
            || t.starts_with("Bundling ")
            || t.starts_with("Running makensis")
        {
            self.progress.phase = "bundle".into();
            self.progress.doing = "Bundling".into();
        }
        self.say(line);
    }
}

/// `text` without its terminal colours.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c != '\r' {
            out.push(c);
        }
    }
    out
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Checks the prerequisites against what the unpacked source asks for: pnpm's path, or what
/// is missing.
pub type Check = Box<dyn Fn(&Needs) -> Result<PathBuf, String> + Send>;

/// A build running, or finished.
pub struct Build {
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
}

impl Build {
    /// Starts building `plan` on a thread of its own.
    pub fn start(plan: Plan, dirs: Dirs, http: Arc<dyn Http>, check: Check) -> Build {
        let _ = std::fs::create_dir_all(&dirs.root);
        let log = std::fs::File::create(dirs.log()).ok();
        let expected = read_json::<u32>(&dirs.last_build());
        let shared = Arc::new(Mutex::new(Shared {
            progress: Progress {
                commit: plan.commit.clone(),
                date: plan.date.clone(),
                phase: "download".into(),
                doing: "Starting".into(),
                expected,
                timeout_minutes: plan.timeout.as_secs() / 60,
                ..Progress::default()
            },
            lines: Vec::new(),
            started: Instant::now(),
            log,
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let (shared, cancel) = (shared.clone(), cancel.clone());
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    build(&plan, &dirs, http.as_ref(), &check, &shared, &cancel)
                }))
                .unwrap_or_else(|_| Err("the build stopped on a bug in photonoxide".into()));
                let mut s = lock(&shared);
                s.progress.seconds = s.started.elapsed().as_secs_f64();
                match result {
                    Ok(ready) => {
                        s.progress.phase = "done".into();
                        s.progress.doing =
                            format!("Built {} in {:.0} min", ready.label, ready.seconds / 60.0);
                        let line = format!("== {}", s.progress.doing);
                        s.say(line);
                        s.progress.ready = Some(ready);
                    }
                    Err(e) => {
                        let cancelled = cancel.load(Ordering::SeqCst);
                        if !cancelled {
                            // main's own list of builder breaks, from the source just unpacked
                            let listed = std::fs::read_to_string(
                                dirs.source().join("studio/src-tauri/builder-breaks.json"),
                            )
                            .ok();
                            let me = super::this_build();
                            let tail = s.lines.len().saturating_sub(200);
                            let too_old = compat::diagnose(
                                &e,
                                &s.progress.phase,
                                &s.lines[tail..],
                                &compat::known(listed.as_deref()),
                                &compat::Builder {
                                    label: &me.label,
                                    version: &me.version,
                                },
                                &plan.commit,
                            );
                            if let Some(t) = &too_old {
                                let line = format!("== {}", t.message);
                                s.say(line);
                            }
                            s.progress.too_old = too_old;
                        }
                        s.progress.phase = if cancelled { "cancelled" } else { "failed" }.into();
                        s.progress.doing = if cancelled {
                            "Cancelled".into()
                        } else {
                            "Failed".into()
                        };
                        s.say(format!("== {e}"));
                        s.progress.error = Some(e);
                    }
                }
                s.progress.finished = true;
            });
        }
        Build { shared, cancel }
    }

    /// The state, with the log's lines from the `from`-th on.
    pub fn progress(&self, from: usize) -> Progress {
        let s = lock(&self.shared);
        let mut p = s.progress.clone();
        p.lines = s.lines.get(from..).unwrap_or_default().to_vec();
        p.line_count = s.lines.len();
        if !p.finished {
            p.seconds = s.started.elapsed().as_secs_f64();
        }
        p
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn finished(&self) -> bool {
        lock(&self.shared).progress.finished
    }
}

/// A nightly's version: main's, with the commit's date and short hash as build metadata. Main
/// at `0.5.4`, commit `4cc3c39` of 2026-10-17, is `0.5.4+nightly.20261017.4cc3c39`; without the
/// date, `0.5.4+nightly.4cc3c39`; a pre-release stays one, `0.6.0-rc.1+nightly.20261017.4cc3c39`.
/// It guesses no next release: semver's precedence ignores build metadata, so a nightly ranks
/// with the version main has, and is never equal to its release (the semver crate's `==` and
/// `Ord` do look at build metadata).
pub fn nightly_version(version: &str, commit: &str, date: Option<&str>) -> Result<String, String> {
    let mut v = semver::Version::parse(version.trim())
        .map_err(|e| format!("the version {version} doesn't read: {e}"))?;
    let date = date
        .map(|d| d.replace('-', ""))
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
    let build = match date {
        Some(d) => format!("nightly.{d}.{}", short(commit)),
        None => format!("nightly.{}", short(commit)),
    };
    v.build = semver::BuildMetadata::new(&build)
        .map_err(|e| format!("the build metadata {build} doesn't read: {e}"))?;
    Ok(v.to_string())
}

/// The version a nightly's bundle carries: main's own, without build metadata (`0.5.4`). The
/// bundles' versions are made for releases: Tauri's NSIS installer can only put a number after
/// the patch in its file version (it warns and writes 0 for `nightly.…`), and macOS reads
/// CFBundleShortVersionString as numbers. The nightly's full version reaches the program itself
/// through `PHOTONOXIDE_NIGHTLY_VERSION` instead (`build.rs`, `channels::this_build`).
pub fn bundle_version(version: &str) -> Result<String, String> {
    let mut v = semver::Version::parse(version.trim())
        .map_err(|e| format!("the version {version} doesn't read: {e}"))?;
    v.build = semver::BuildMetadata::EMPTY;
    Ok(v.to_string())
}

/// What Settings calls a nightly:
/// `0.5.4+nightly.20261017.4cc3c39 (main @ 4cc3c39, 2026-10-17)`.
pub fn nightly_label(version: &str, commit: &str, date: Option<&str>) -> String {
    match date {
        Some(d) => format!("{version} (main @ {}, {d})", short(commit)),
        None => format!("{version} (main @ {})", short(commit)),
    }
}

/// The folder, under Tauri's `bundle`, that a bundle goes to.
pub fn bundle_folder(bundle: &str) -> &'static str {
    match bundle {
        "nsis" => "nsis",
        "app" => "macos",
        "appimage" => "appimage",
        "deb" => "deb",
        _ => "rpm",
    }
}

fn is_artifact(bundle: &str, name: &str) -> bool {
    match bundle {
        "nsis" => name.ends_with("-setup.exe"),
        "app" => name.ends_with(".app"),
        "appimage" => name.ends_with(".AppImage"),
        "deb" => name.ends_with(".deb"),
        _ => name.ends_with(".rpm"),
    }
}

/// The bundle the build made: the newest of its kind made since `since`.
fn find_artifact(target: &Path, bundle: &str, since: SystemTime) -> Option<PathBuf> {
    let dir = target
        .join("release")
        .join("bundle")
        .join(bundle_folder(bundle));
    let since = since.checked_sub(Duration::from_secs(5)).unwrap_or(since);
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| is_artifact(bundle, &e.file_name().to_string_lossy()))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(t, _)| *t >= since)
        .max()
        .map(|(_, p)| p)
}

/// Unpacks GitHub's tarball of `sha` into `to`, without its one top folder
/// (`photonoxide-<sha>`), which must be there: it is how the archive says which commit it holds.
pub fn unpack(archive: &Path, to: &Path, sha: &str) -> Result<usize, String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("{}: {e}", archive.display()))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.set_preserve_mtime(false);
    let top = format!("photonoxide-{sha}");
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let mut files = 0;
    for entry in tar
        .entries()
        .map_err(|e| format!("the tarball doesn't read: {e}"))?
    {
        let mut entry = entry.map_err(|e| format!("the tarball doesn't read: {e}"))?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            // git's pax header (the commit, as a comment), and links, which the source hasn't
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| format!("a path in the tarball doesn't read: {e}"))?
            .into_owned();
        let mut parts = path.components();
        match parts.next() {
            Some(Component::Normal(first)) if first.to_string_lossy() == top => {}
            _ => {
                return Err(format!(
                    "the tarball isn't main at {}: it holds {}",
                    short(sha),
                    path.display()
                ));
            }
        }
        let rest: PathBuf = parts.as_path().to_path_buf();
        if rest.as_os_str().is_empty() {
            continue;
        }
        if !rest.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err(format!(
                "the tarball has a path outside its folder: {}",
                path.display()
            ));
        }
        let dest = to.join(&rest);
        if kind.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        } else {
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            entry
                .unpack(&dest)
                .map_err(|e| format!("{}: {e}", dest.display()))?;
            files += 1;
        }
    }
    if files == 0 {
        return Err("the tarball is empty".into());
    }
    Ok(files)
}

/// Removes `dir`, trying again for up to 10 s while a file in it is still held: a build
/// cancelled just before left its processes killed, and Windows lets go of a killed process's
/// files only some time after it is gone (a rebuild right after a cancel failed with "being used
/// by another process" when the machine was busy).
fn remove_dir_patiently(dir: &Path) -> std::io::Result<()> {
    let mut tries = 0;
    loop {
        match std::fs::remove_dir_all(dir) {
            Err(_) if tries < 100 && dir.exists() => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            done => return done,
        }
    }
}

fn stopped(cancel: &AtomicBool, deadline: Instant, timeout: Duration) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".into());
    }
    if Instant::now() > deadline {
        let limit = match timeout.as_secs() {
            s if s >= 60 => format!("{} minutes", s / 60),
            s => format!("{s} s"),
        };
        return Err(format!(
            "the build took longer than its {limit} and was stopped"
        ));
    }
    Ok(())
}

/// One process of the build.
struct Step<'a> {
    what: &'a str,
    program: &'a Path,
    args: Vec<OsString>,
    cwd: &'a Path,
    env: &'a [(OsString, OsString)],
}

/// Runs `step`, its output into the log, until it ends, is cancelled or runs past `deadline`.
fn run(
    step: &Step,
    shared: &Arc<Mutex<Shared>>,
    cancel: &AtomicBool,
    deadline: Instant,
    timeout: Duration,
) -> Result<(), String> {
    let mut command = Command::new(step.program);
    command
        .args(&step.args)
        .current_dir(step.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in step.env {
        command.env(k, v);
    }
    // a local build signs nothing
    command.env_remove("TAURI_SIGNING_PRIVATE_KEY");
    command.env_remove("TAURI_SIGNING_PRIVATE_KEY_PASSWORD");
    crate::libraries::no_window(&mut command);
    #[cfg(unix)]
    {
        // a group of its own, so cancelling stops pnpm, node, cargo and rustc together
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    {
        let shown: Vec<String> = step
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        lock(shared).say(format!("$ {} {}", step.program.display(), shown.join(" ")));
    }
    // a build cancelled before this step starts no process (one started only to be killed was
    // killed while it was starting its own)
    stopped(cancel, deadline, timeout)?;
    let mut child = command
        .spawn()
        .map_err(|e| format!("can't start {}: {e}", step.program.display()))?;
    #[cfg(windows)]
    let job = job::Job::of(&child);
    let pipes = [
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    ];
    let readers: Vec<_> = pipes
        .into_iter()
        .flatten()
        .map(|pipe| {
            let shared = shared.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(pipe).split(b'\n') {
                    let Ok(line) = line else { break };
                    lock(&shared).line(&String::from_utf8_lossy(&line));
                }
            })
        })
        .collect();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                for r in readers {
                    let _ = r.join();
                }
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "{} failed ({status}): the log's last lines say why",
                        step.what
                    ))
                };
            }
            Ok(None) => {}
            Err(e) => return Err(format!("{}: {e}", step.what)),
        }
        if let Err(e) = stopped(cancel, deadline, timeout) {
            #[cfg(windows)]
            if let Some(job) = &job {
                job.end(Duration::from_secs(10));
            }
            crate::tasks::kill_tree(&mut child);
            let _ = child.wait();
            return Err(e);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A step's processes on Windows, in a job object of their own. A process started by one in a
/// job is in it from its start, so ending the job ends pnpm, node, cargo and rustc together,
/// and a child being started at that moment too. `taskkill /T` walks the tree by parent and
/// misses such a child: a cancel just after a step started left its `ping` running for its
/// whole minute, holding the source folder, which the next build then couldn't empty.
#[cfg(windows)]
mod job {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use std::time::{Duration, Instant};

    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *mut c_void, name: *const u16) -> *mut c_void;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
        fn TerminateJobObject(job: *mut c_void, exit_code: u32) -> i32;
        fn QueryInformationJobObject(
            job: *mut c_void,
            class: i32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    /// `JobObjectBasicAccountingInformation`, and its struct.
    const BASIC_ACCOUNTING: i32 = 1;
    #[repr(C)]
    #[derive(Default)]
    struct Accounting {
        _total_user_time: i64,
        _total_kernel_time: i64,
        _this_period_total_user_time: i64,
        _this_period_total_kernel_time: i64,
        _total_page_fault_count: u32,
        _total_processes: u32,
        active_processes: u32,
        _total_terminated_processes: u32,
    }

    pub struct Job(*mut c_void);

    impl Job {
        /// A job holding `child` (and whatever it starts from now on), if Windows gives one.
        pub fn of(child: &Child) -> Option<Job> {
            // SAFETY: plain Win32 calls on a handle this function owns, closed if not kept;
            // the child's handle is valid while `child` is
            unsafe {
                let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
                if job.is_null() {
                    return None;
                }
                if AssignProcessToJobObject(job, child.as_raw_handle().cast()) == 0 {
                    CloseHandle(job);
                    return None;
                }
                Some(Job(job))
            }
        }

        /// Ends every process in the job, and waits up to `within` for them all to be gone.
        pub fn end(&self, within: Duration) {
            // SAFETY: the job's handle is valid until the job is dropped
            unsafe { TerminateJobObject(self.0, 1) };
            let until = Instant::now() + within;
            while self.active() > 0 && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        /// How many of its processes are still running (0 if Windows won't say).
        fn active(&self) -> u32 {
            let mut info = Accounting::default();
            let size = u32::try_from(std::mem::size_of::<Accounting>()).unwrap_or(0);
            // SAFETY: `info` is the struct this class asks for, `size` its size
            let ok = unsafe {
                QueryInformationJobObject(
                    self.0,
                    BASIC_ACCOUNTING,
                    (&raw mut info).cast(),
                    size,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 { 0 } else { info.active_processes }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: the handle is the job's own, closed once
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// main's version, from its `Cargo.toml` (`text`): the package's, or the workspace's that it
/// takes ([`prereqs::manifest_field`], which reads the lines when the TOML doesn't parse); and
/// failing both, the version main's latest release implies (main is at it or past it). Only a
/// version that reads, and a note when it wasn't the manifest's own. A manifest whose format
/// moves on doesn't stop the build.
pub fn main_version(text: &str, http: &dyn Http) -> Result<(String, Option<String>), String> {
    let reads = |v: &str| semver::Version::parse(v.trim()).is_ok();
    if let Some(v) = prereqs::manifest_field(text, "version").filter(|v| reads(v)) {
        return Ok((v, None));
    }
    let release = github::latest_release(http).map_err(|e| {
        format!("main's Cargo.toml gives no version that reads, and neither does GitHub: {e}")
    })?;
    let note = format!(
        "main's Cargo.toml gives no version that reads; main's latest release is {release}, so \
         main is {release} or later"
    );
    Ok((release, Some(note)))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn build(
    plan: &Plan,
    dirs: &Dirs,
    http: &dyn Http,
    check: &Check,
    shared: &Arc<Mutex<Shared>>,
    cancel: &AtomicBool,
) -> Result<Ready, String> {
    let begun = SystemTime::now();
    let deadline = Instant::now() + plan.timeout;
    let sha = plan.commit.as_str();
    if !github::is_sha(sha) {
        return Err(format!("{sha} isn't a commit"));
    }
    let say = |phase: &str, doing: String| lock(shared).phase(phase, doing);

    say(
        "download",
        format!("Downloading {} at {} from GitHub", github::REPO, short(sha)),
    );
    let downloads = dirs.downloads();
    let _ = std::fs::remove_dir_all(&downloads);
    std::fs::create_dir_all(&downloads).map_err(|e| format!("{}: {e}", downloads.display()))?;
    let archive = downloads.join(format!("{sha}.tar.gz"));
    http.download(
        &github::tarball(sha),
        &archive,
        &mut |done, size| {
            let mut s = lock(shared);
            s.progress.downloaded = done;
            s.progress.size = size;
        },
        &|| cancel.load(Ordering::SeqCst) || Instant::now() > deadline,
    )
    .map_err(|e| stopped(cancel, deadline, plan.timeout).err().unwrap_or(e))?;
    stopped(cancel, deadline, plan.timeout)?;

    say("unpack", "Unpacking the source".into());
    let source = dirs.source();
    if source.exists() {
        remove_dir_patiently(&source)
            .map_err(|e| format!("can't empty {}: {e}", source.display()))?;
    }
    let files = unpack(&archive, &source, sha)?;
    let _ = std::fs::remove_dir_all(&downloads);
    lock(shared).say(format!("{files} files in {}", source.display()));
    stopped(cancel, deadline, plan.timeout)?;

    say(
        "check",
        "Checking the prerequisites against what this source asks for".into(),
    );
    let studio = source.join("studio");
    let read =
        |p: PathBuf| std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()));
    let needs = Needs::from_source(
        &[
            &read(source.join("Cargo.toml"))?,
            &read(studio.join("src-tauri").join("Cargo.toml"))?,
        ],
        &read(studio.join("package.json"))?,
        &plan.bundle,
    )?;
    let pnpm = check(&needs)?;
    let (version, note) = main_version(&read(source.join("Cargo.toml"))?, http)?;
    if let Some(note) = note {
        lock(shared).say(note);
    }
    let bundled = bundle_version(&version)?;
    let version = nightly_version(&version, sha, plan.date.as_deref())?;
    let label = nightly_label(&version, sha, plan.date.as_deref());
    lock(shared).say(format!("pnpm: {}; building {label}", pnpm.display()));

    // this copy's name and identifier, main's plain version for the bundle (the nightly's own is
    // the program's, through PHOTONOXIDE_NIGHTLY_VERSION), the bundle the update needs, and no
    // updater artifacts: the Tauri CLI merges it over tauri.conf.json
    let mut config = serde_json::json!({
        "version": bundled,
        "identifier": plan.identity.identifier,
        "bundle": { "active": true, "createUpdaterArtifacts": false },
    });
    if let Some(name) = &plan.identity.product_name {
        config["productName"] = name.clone().into();
    }
    if let Some(name) = &plan.identity.main_binary_name {
        config["mainBinaryName"] = name.clone().into();
    }
    let config_file = studio.join("src-tauri").join("nightly.conf.json");
    std::fs::write(&config_file, config.to_string())
        .map_err(|e| format!("{}: {e}", config_file.display()))?;

    let env: Vec<(OsString, OsString)> = [
        ("PATH", plan.path.clone()),
        ("CARGO_TARGET_DIR", dirs.target().into_os_string()),
        ("CI", "true".into()),
        ("PHOTONOXIDE_COMMIT", sha.into()),
        (
            "PHOTONOXIDE_COMMIT_DATE",
            plan.date.clone().unwrap_or_default().into(),
        ),
        ("PHOTONOXIDE_DIRTY", "false".into()),
        ("PHOTONOXIDE_CHANNEL", "nightly".into()),
        ("PHOTONOXIDE_NIGHTLY_VERSION", version.clone().into()),
    ]
    .into_iter()
    .map(|(k, v)| (OsString::from(k), v))
    .collect();

    say(
        "install",
        "Installing the window's packages: pnpm install --frozen-lockfile".into(),
    );
    let install = Step {
        what: "pnpm install",
        program: &pnpm,
        args: vec!["install".into(), "--frozen-lockfile".into()],
        cwd: &studio,
        env: &env,
    };
    run(&install, shared, cancel, deadline, plan.timeout)?;

    say(
        "frontend",
        format!(
            "Building with the Tauri CLI: pnpm tauri build --bundles {}",
            plan.bundle
        ),
    );
    let tauri = Step {
        what: "pnpm tauri build",
        program: &pnpm,
        args: vec![
            "tauri".into(),
            "build".into(),
            "--bundles".into(),
            plan.bundle.clone().into(),
            "--config".into(),
            config_file.into_os_string(),
        ],
        cwd: &studio,
        env: &env,
    };
    run(&tauri, shared, cancel, deadline, plan.timeout)?;

    let made = find_artifact(&dirs.target(), &plan.bundle, begun).ok_or_else(|| {
        format!(
            "the build finished, but made no {} bundle in {}",
            plan.bundle,
            dirs.target().join("release").join("bundle").display()
        )
    })?;
    // each commit's build in a folder of its own: the build waiting until now stays whole (and
    // installable) until this one's is in place and `ready.json` names it
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let ready_dir = dirs.ready_for(sha, millis);
    std::fs::create_dir_all(&ready_dir).map_err(|e| format!("{}: {e}", ready_dir.display()))?;
    let artifact = ready_dir.join(made.file_name().unwrap_or_default());
    std::fs::rename(&made, &artifact).map_err(|e| {
        format!(
            "can't move {} to {}: {e}",
            made.display(),
            artifact.display()
        )
    })?;
    let (compiled, seconds) = {
        let s = lock(shared);
        (s.progress.compiled, s.started.elapsed().as_secs_f64())
    };
    let ready = Ready {
        commit: sha.to_owned(),
        date: plan.date.clone(),
        version,
        label,
        bundle: plan.bundle.clone(),
        artifact,
        built_at: now_unix(),
        seconds,
    };
    write_json(&dirs.ready_file(), &ready)?;
    // only now is the build it replaces removed
    remove_ready_except(dirs, Some(&ready_dir));
    if compiled > 0 {
        let _ = write_json(&dirs.last_build(), &compiled);
    }
    Ok(ready)
}

/// Removes what [`Dirs::ready`] holds but `keep`: older builds' folders, and files an earlier
/// version left there loose. What can't be removed now (a file in use) goes next time.
pub fn remove_ready_except(dirs: &Dirs, keep: Option<&Path>) {
    let Ok(entries) = std::fs::read_dir(dirs.ready()) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if Some(path.as_path()) == keep {
            continue;
        }
        let _ = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
    }
}

/// Sets aside the build waiting (`ready.json` and its files), when it is no use: older than
/// the build running, or the build running itself.
pub fn discard_ready(dirs: &Dirs, ready: &Ready) {
    // only if ready.json still names it: a build that just finished may have replaced it
    if read_json::<Ready>(&dirs.ready_file()).is_some_and(|r| r.commit == ready.commit) {
        let _ = std::fs::remove_file(dirs.ready_file());
    }
    let folder = ready.artifact.parent().filter(|p| *p != dirs.ready());
    let _ = match folder {
        Some(f) if f.starts_with(dirs.ready()) => std::fs::remove_dir_all(f),
        _ => std::fs::remove_file(&ready.artifact),
    };
}

/// Waits for `build` to finish, telling `line` each new line of its log: for the command line.
pub fn follow(build: &Build, line: &mut dyn FnMut(&str)) -> Progress {
    let mut from = 0;
    loop {
        let p = build.progress(from);
        for l in &p.lines {
            line(l);
        }
        from = p.line_count;
        if p.finished {
            return p;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::github::tests::{Canned, HEAD};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("photonoxide-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A tarball as GitHub makes them: one folder, `photonoxide-<sha>`, holding the source.
    fn tarball(top: &str) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        let files = [
            (
                "Cargo.toml",
                "[package]\nname = \"photonoxide\"\nversion = \"0.5.0\"\nrust-version = \"1.95\"\n",
            ),
            ("studio/package.json", r#"{"packageManager":"pnpm@12.4.1"}"#),
            (
                "studio/src-tauri/Cargo.toml",
                "[package]\nname = \"photonoxide-studio\"\nrust-version = \"1.95\"\n",
            ),
            ("src/lib.rs", "//! photonoxide\n"),
        ];
        for (path, text) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(text.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, format!("{top}/{path}"), text.as_bytes())
                .unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    /// The bundle a fake build makes here, as `tauri build` would name it.
    fn fake_bundle() -> (&'static str, &'static str) {
        if cfg!(windows) {
            ("nsis", "photonoxide_0.5.0_x64-setup.exe")
        } else {
            ("deb", "photonoxide_0.5.0_amd64.deb")
        }
    }

    /// A pnpm that installs nothing and, asked to `tauri build`, prints a few of the Tauri
    /// CLI's lines and makes the bundle; or fails, or never ends, as `mode` says.
    fn fake_pnpm(dir: &Path, mode: &str) -> PathBuf {
        let (folder, file) = fake_bundle();
        if cfg!(windows) {
            let path = dir.join("pnpm.cmd");
            let mut s = String::from(
                "@echo off\r\necho fake pnpm %*\r\nif \"%1\"==\"install\" exit /b 0\r\n",
            );
            match mode {
                "fail" => s.push_str("echo error: the linker broke\r\nexit /b 3\r\n"),
                "hang" => s.push_str("ping -n 60 127.0.0.1 >nul\r\n"),
                _ => {}
            }
            s.push_str(&format!(
                "echo     Running beforeBuildCommand `pnpm build`\r\necho nightly version %PHOTONOXIDE_NIGHTLY_VERSION%\r\necho    Compiling serde v1.0.228\r\necho    Compiling photonoxide v0.5.0\r\necho     Finished `release` profile\r\nmkdir \"%CARGO_TARGET_DIR%\\release\\bundle\\{folder}\" 2>nul\r\necho fake> \"%CARGO_TARGET_DIR%\\release\\bundle\\{folder}\\{file}\"\r\nexit /b 0\r\n"
            ));
            std::fs::write(&path, s).unwrap();
            path
        } else {
            let path = dir.join("pnpm");
            let mut s =
                String::from("#!/bin/sh\necho \"fake pnpm $*\"\n[ \"$1\" = install ] && exit 0\n");
            match mode {
                "fail" => s.push_str("echo 'error: the linker broke'\nexit 3\n"),
                "hang" => s.push_str("sleep 60\n"),
                _ => {}
            }
            s.push_str(&format!(
                "echo '    Running beforeBuildCommand `pnpm build`'\necho \"nightly version $PHOTONOXIDE_NIGHTLY_VERSION\"\necho '   Compiling serde v1.0.228'\necho '   Compiling photonoxide v0.5.0'\necho '    Finished `release` profile'\nmkdir -p \"$CARGO_TARGET_DIR/release/bundle/{folder}\"\necho fake > \"$CARGO_TARGET_DIR/release/bundle/{folder}/{file}\"\n"
            ));
            std::fs::write(&path, s).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            path
        }
    }

    fn plan(timeout: Duration) -> Plan {
        Plan {
            commit: HEAD.into(),
            date: Some("2026-10-09".into()),
            bundle: fake_bundle().0.into(),
            identity: Identity {
                identifier: "gr.tachsin.photonoxide".into(),
                product_name: Some("photonoxide".into()),
                main_binary_name: None,
            },
            path: std::env::var_os("PATH").unwrap_or_default(),
            timeout,
        }
    }

    fn start(name: &str, mode: &str, timeout: Duration) -> (Build, Dirs, PathBuf) {
        let dir = temp(name);
        let dirs = Dirs {
            root: dir.join("nightly"),
        };
        let web = Canned::default();
        web.answer(
            &github::tarball(HEAD),
            200,
            &[],
            &tarball(&format!("photonoxide-{HEAD}")),
        );
        let pnpm = fake_pnpm(&dir, mode);
        let check: Check = Box::new(move |needs: &Needs| {
            if (needs.rust, needs.pnpm.as_str()) == ((1, 95, 0), "12.4.1") {
                Ok(pnpm.clone())
            } else {
                Err(format!("the needs read wrong: {needs:?}"))
            }
        });
        (
            Build::start(plan(timeout), dirs.clone(), Arc::new(web), check),
            dirs,
            dir,
        )
    }

    #[test]
    fn versions_and_labels() {
        let day = Some("2026-10-09");
        assert_eq!(
            nightly_version("0.5.4", HEAD, day).unwrap(),
            "0.5.4+nightly.20261009.c9cae4c"
        );
        // no date, or one that isn't a date: the commit alone
        assert_eq!(
            nightly_version("0.5.4", HEAD, None).unwrap(),
            "0.5.4+nightly.c9cae4c"
        );
        assert_eq!(
            nightly_version("0.5.4", HEAD, Some("")).unwrap(),
            "0.5.4+nightly.c9cae4c"
        );
        assert_eq!(
            nightly_version("0.5.4", HEAD, Some("yesterday")).unwrap(),
            "0.5.4+nightly.c9cae4c"
        );
        // a pre-release stays one; build metadata already there goes
        assert_eq!(
            nightly_version("0.6.0-rc.1", HEAD, day).unwrap(),
            "0.6.0-rc.1+nightly.20261009.c9cae4c"
        );
        assert_eq!(
            nightly_version(" 1.2.3+meta.7 ", HEAD, day).unwrap(),
            "1.2.3+nightly.20261009.c9cae4c"
        );
        // a short hash of digits alone, leading zero and all, is build metadata semver takes
        assert_eq!(
            nightly_version("0.5.4", "0123456789012345678901234567890123456789", day).unwrap(),
            "0.5.4+nightly.20261009.0123456"
        );
        assert!(nightly_version("main", HEAD, day).is_err());
        // the bundles carry main's plain version
        assert_eq!(bundle_version("0.5.4").unwrap(), "0.5.4");
        assert_eq!(bundle_version("1.2.3+meta").unwrap(), "1.2.3");
        assert_eq!(bundle_version("0.6.0-rc.1").unwrap(), "0.6.0-rc.1");
        assert!(bundle_version("main").is_err());
        // no guess at the next release: a nightly ranks with main's version, ahead of none of
        // the releases after it, and isn't its release (`==` and `Ord` see build metadata)
        let v = |s: &str| semver::Version::parse(s).unwrap();
        let nightly = v(&nightly_version("0.5.4", HEAD, day).unwrap());
        assert_eq!(
            nightly.cmp_precedence(&v("0.5.4")),
            std::cmp::Ordering::Equal
        );
        assert!(nightly != v("0.5.4") && nightly < v("0.5.5") && nightly < v("0.6.0"));
        // two nightlies of one version: the later day's sorts after
        let next_day = v(&nightly_version("0.5.4", HEAD, Some("2026-10-10")).unwrap());
        assert!(next_day > nightly);
        assert_eq!(
            nightly_label("0.5.4+nightly.20261009.c9cae4c", HEAD, day),
            "0.5.4+nightly.20261009.c9cae4c (main @ c9cae4c, 2026-10-09)"
        );
        assert_eq!(
            nightly_label("0.5.4+nightly.c9cae4c", HEAD, None),
            "0.5.4+nightly.c9cae4c (main @ c9cae4c)"
        );
        assert_eq!(
            plain("\u{1b}[1m\u{1b}[32m   Compiling\u{1b}[0m x v1\r"),
            "   Compiling x v1"
        );
    }

    #[test]
    fn a_tarball_unpacks_without_its_folder_and_must_be_the_commit_asked_for() {
        let dir = temp("unpack");
        let archive = dir.join("main.tar.gz");
        std::fs::write(&archive, tarball(&format!("photonoxide-{HEAD}"))).unwrap();
        assert_eq!(unpack(&archive, &dir.join("src"), HEAD).unwrap(), 4);
        assert!(
            dir.join("src")
                .join("studio")
                .join("package.json")
                .is_file()
        );
        // another commit's tarball is refused
        std::fs::write(
            &archive,
            tarball("photonoxide-0000000000000000000000000000000000000000"),
        )
        .unwrap();
        let e = unpack(&archive, &dir.join("other"), HEAD).unwrap_err();
        assert!(e.contains("isn't main at c9cae4c"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_build_downloads_unpacks_checks_runs_pnpm_and_keeps_the_bundle() {
        let (build, dirs, dir) = start("build-ok", "ok", Duration::from_secs(600));
        let mut log = Vec::new();
        let p = follow(&build, &mut |l| log.push(l.to_owned()));
        assert_eq!(p.phase, "done", "{:?}\n{}", p.error, log.join("\n"));
        assert_eq!((p.compiled, p.downloaded > 0), (2, true));
        let ready = p.ready.unwrap();
        assert_eq!(ready.version, "0.5.0+nightly.20261009.c9cae4c");
        assert_eq!(
            ready.label,
            "0.5.0+nightly.20261009.c9cae4c (main @ c9cae4c, 2026-10-09)"
        );
        assert!(ready.artifact.starts_with(dirs.ready()) && ready.artifact.is_file());
        assert_eq!(read_json::<Ready>(&dirs.ready_file()), Some(ready));
        assert_eq!(read_json::<u32>(&dirs.last_build()), Some(2));
        // the steps, in order, with the Tauri CLI and the bundle the update needs
        let commands: Vec<&String> = log.iter().filter(|l| l.starts_with("fake pnpm")).collect();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0], "fake pnpm install --frozen-lockfile");
        assert!(commands[1].starts_with(&format!(
            "fake pnpm tauri build --bundles {} --config",
            fake_bundle().0
        )));
        // the config the CLI merged: this copy's identity, main's plain version for the bundle,
        // no updater artifacts; the nightly's own version goes to the program
        let config: serde_json::Value =
            read_json(&dirs.source().join("studio/src-tauri/nightly.conf.json")).unwrap();
        assert_eq!(config["version"], "0.5.0");
        assert!(
            log.iter()
                .any(|l| l == "nightly version 0.5.0+nightly.20261009.c9cae4c"),
            "{}",
            log.join("\n")
        );
        assert_eq!(config["bundle"]["createUpdaterArtifacts"], false);
        assert_eq!(config["identifier"], "gr.tachsin.photonoxide");
        // the log was kept on disk too
        assert!(
            std::fs::read_to_string(dirs.log())
                .unwrap()
                .contains("Compiling photonoxide")
        );
        // the next build knows how many crates to expect
        let web = Canned::default();
        web.answer(
            &github::tarball(HEAD),
            200,
            &[],
            &tarball(&format!("photonoxide-{HEAD}")),
        );
        let pnpm = fake_pnpm(&dir, "ok");
        let again = Build::start(
            plan(Duration::from_secs(600)),
            dirs,
            Arc::new(web),
            Box::new(move |_| Ok(pnpm.clone())),
        );
        assert_eq!(again.progress(0).expected, Some(2));
        assert_eq!(follow(&again, &mut |_| {}).phase, "done");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Builds again in `dirs`, the fake pnpm doing as `mode` says.
    fn again(dirs: &Dirs, dir: &Path, mode: &str) -> Progress {
        let web = Canned::default();
        web.answer(
            &github::tarball(HEAD),
            200,
            &[],
            &tarball(&format!("photonoxide-{HEAD}")),
        );
        let pnpm = fake_pnpm(dir, mode);
        let build = Build::start(
            plan(Duration::from_secs(600)),
            dirs.clone(),
            Arc::new(web),
            Box::new(move |_| Ok(pnpm.clone())),
        );
        follow(&build, &mut |_| {})
    }

    #[test]
    fn the_build_waiting_is_replaced_only_once_the_new_one_is_ready() {
        let (build, dirs, dir) = start("build-replace", "ok", Duration::from_secs(600));
        let first = follow(&build, &mut |_| {}).ready.unwrap();
        // a file an earlier version left loose in ready/
        std::fs::write(dirs.ready().join("photonoxide_0.4.9_x64-setup.exe"), "old").unwrap();
        // a rebuild that fails: the build waiting is still there, whole and named
        let failed = again(&dirs, &dir, "fail");
        assert_eq!(failed.phase, "failed");
        assert_eq!(read_json::<Ready>(&dirs.ready_file()), Some(first.clone()));
        assert!(first.artifact.is_file());
        // a cancelled one too
        let web = Canned::default();
        web.answer(
            &github::tarball(HEAD),
            200,
            &[],
            &tarball(&format!("photonoxide-{HEAD}")),
        );
        let pnpm = fake_pnpm(&dir, "hang");
        let hung = Build::start(
            plan(Duration::from_secs(600)),
            dirs.clone(),
            Arc::new(web),
            Box::new(move |_| Ok(pnpm.clone())),
        );
        while !matches!(hung.progress(0).phase.as_str(), "frontend" | "compile") && !hung.finished()
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        hung.cancel();
        assert_eq!(follow(&hung, &mut |_| {}).phase, "cancelled");
        assert_eq!(read_json::<Ready>(&dirs.ready_file()), Some(first.clone()));
        assert!(first.artifact.is_file());
        // one that succeeds replaces it, and only then is the old one removed
        let rebuilt = again(&dirs, &dir, "ok");
        let second = rebuilt
            .ready
            .unwrap_or_else(|| panic!("the rebuild failed: {:?}", rebuilt.error));
        assert_ne!(second.artifact, first.artifact);
        assert_eq!(read_json::<Ready>(&dirs.ready_file()), Some(second.clone()));
        assert!(second.artifact.is_file());
        assert!(!first.artifact.exists() && !first.artifact.parent().unwrap().exists());
        let left: Vec<_> = std::fs::read_dir(dirs.ready()).unwrap().flatten().collect();
        assert_eq!(left.len(), 1, "only the new build's folder");
        // a build set aside goes with its folder, and ready.json with it
        discard_ready(&dirs, &second);
        assert!(!dirs.ready_file().exists() && !second.artifact.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn main_s_version_is_read_however_its_manifest_gives_it() {
        let web = Canned::default();
        let latest = "https://api.github.com/repos/tachsin/photonoxide/releases/latest";
        // the package's own
        let own = "[package]\nname = \"photonoxide\"\nversion = \"0.5.0\"\n";
        assert_eq!(main_version(own, &web).unwrap(), ("0.5.0".into(), None));
        // the workspace's, as main has had it since #290
        let workspace = "[package]\nname = \"photonoxide\"\nversion.workspace = true\n\n\
                         [workspace.package]\nversion = \"0.5.2\"\n";
        assert_eq!(main_version(workspace, &web).unwrap().0, "0.5.2");
        // a manifest this build's TOML parser can't read: its lines
        let newer = "[package]\nname = \"photonoxide\"\nversion.workspace = true\n\
                     edition = 2027 @ \"future syntax\"\n\n[workspace.package]\n\
                     version = \"0.6.0\"  # the workspace's\nrust-version = \"1.99\"\n";
        assert!(toml::from_str::<toml::Value>(newer).is_err());
        assert_eq!(main_version(newer, &web).unwrap().0, "0.6.0");
        assert_eq!(
            prereqs::manifest_field(newer, "rust-version").as_deref(),
            Some("1.99")
        );
        // no version anywhere: the latest release's, said so
        let none = "[package]\nname = \"photonoxide\"\nversion = { from = \"somewhere\" }\n";
        assert!(
            main_version(none, &web)
                .unwrap_err()
                .contains("neither does GitHub")
        );
        web.answer(latest, 200, &[], br#"{"tag_name":"v0.5.2","name":"0.5.2"}"#);
        let (v, note) = main_version(none, &web).unwrap();
        assert_eq!(v, "0.5.2");
        assert!(note.unwrap().contains("latest release is 0.5.2"));
        assert_eq!(
            nightly_version(&v, HEAD, None).unwrap(),
            "0.5.2+nightly.c9cae4c"
        );
        // a version that isn't one isn't taken
        let bad = "[package]\nversion = \"main\"\n";
        assert_eq!(main_version(bad, &web).unwrap().0, "0.5.2");
        // and the prerequisites read past a manifest that doesn't parse, or says no
        // rust-version: this build's own then
        let needs =
            Needs::from_source(&[newer], r#"{"packageManager":"pnpm@12.4.1"}"#, "nsis").unwrap();
        assert_eq!(needs.rust, (1, 99, 0));
        let needs =
            Needs::from_source(&[none], r#"{"packageManager":"pnpm@12.4.1"}"#, "nsis").unwrap();
        assert_eq!(needs.rust, Needs::of_this_build("nsis").rust);
        // the workspace's own manifests, as main has them: the version is photonoxide's, and the
        // program's manifest, which takes it from the workspace, still says its rust-version
        let [workspace, studio] = prereqs::OWN_MANIFESTS;
        assert_eq!(
            main_version(workspace, &web).unwrap(),
            (photonoxide::VERSION.to_owned(), None)
        );
        assert!(prereqs::manifest_field(studio, "rust-version").is_some());
    }

    #[test]
    fn a_failing_step_stops_the_build_and_says_which() {
        let (build, dirs, dir) = start("build-fail", "fail", Duration::from_secs(600));
        let p = follow(&build, &mut |_| {});
        assert_eq!(p.phase, "failed");
        assert!(p.error.unwrap().starts_with("pnpm tauri build failed"));
        assert!(p.ready.is_none() && !dirs.ready_file().exists());
        let all = build.progress(0).lines.join("\n");
        assert!(all.contains("error: the linker broke"), "{all}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_build_can_be_cancelled_and_has_a_hard_timeout() {
        let (build, _, dir) = start("build-cancel", "hang", Duration::from_secs(600));
        while build.progress(0).phase != "compile"
            && build.progress(0).phase != "frontend"
            && !build.finished()
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        let asked = Instant::now();
        build.cancel();
        let p = follow(&build, &mut |_| {});
        assert_eq!(
            (p.phase.as_str(), p.error.as_deref()),
            ("cancelled", Some("cancelled"))
        );
        assert!(asked.elapsed() < Duration::from_secs(20));
        let _ = std::fs::remove_dir_all(&dir);

        // a hard timeout of 1 s: what is checked is that it fires and says so, not how long
        let (build, _, dir) = start("build-timeout", "hang", Duration::from_secs(1));
        let p = follow(&build, &mut |_| {});
        assert_eq!(p.phase, "failed");
        assert!(p.error.unwrap().contains("took longer than"), "timeout");
        assert!(p.seconds < 30.0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
