//! Release channels. **Stable** is the signed releases, which Tauri's updater takes from GitHub
//! (`studio/src/lib/updater.svelte.ts`). **Nightly** follows main: the studio looks at main's
//! head on GitHub ([`github`]), and when it moved, offers to build that commit here, from its
//! source ([`builder`]), once the prerequisites are there ([`prereqs`]), then installs the build
//! in place of this copy, keeping the previous one to go back to ([`install`]).
//!
//! Nothing runs without a click: the check only reads GitHub, and building (which runs main's
//! code) and installing each wait for the user. Only tachsin/photonoxide's main is built, at the
//! commit the window showed.
//!
//! Each build knows what it was built from (`build.rs`): its commit, the commit's date, and
//! whether the tree had changes; a nightly also its channel and version ([`this_build`]).

pub mod builder;
pub mod compat;
pub mod github;
pub mod install;
pub mod offer;
pub mod prereqs;
pub mod release;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

use builder::{Build, Dirs, Identity, Plan, Progress, Ready, read_json, write_json};
use github::{Checker, Http, Nightly, Web, short};
use install::{Note, Outcome, Previous, Running};
use prereqs::{Needs, Probe, Report, System};

/// How long a nightly build may take, all in: a first build compiles every crate.
pub const BUILD_TIMEOUT: Duration = Duration::from_secs(2 * 3600);

/// What this program was built from.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BuildInfo {
    /// `"nightly"` for a build the nightly channel made, `"stable"` otherwise (a release, or a
    /// build from the repository).
    pub channel: String,
    /// The program's version: a release's, or a nightly's (`0.5.4+nightly.20261017.4cc3c39`).
    pub version: String,
    /// The commit, when known, and its first seven characters.
    pub commit: Option<String>,
    pub short: Option<String>,
    /// The commit's date, `2026-10-09`.
    pub date: Option<String>,
    /// The tree had changes not committed.
    pub dirty: bool,
    /// What Settings calls it: the version, or for a nightly,
    /// `0.5.4+nightly.20261017.4cc3c39 (main @ 4cc3c39, 2026-10-17)`.
    pub label: String,
}

impl BuildInfo {
    pub fn nightly(&self) -> bool {
        self.channel == "nightly"
    }

    /// The version the stable channel compares a release with. Tauri's updater gives the
    /// bundle's (`bundled`), which for a nightly is main's plain version (`0.5.4`, see
    /// [`builder::bundle_version`]): a nightly's is its own, `0.5.4+nightly.…`, never equal to
    /// a release.
    pub fn updating_from(&self, bundled: &semver::Version) -> semver::Version {
        self.nightly()
            .then(|| semver::Version::parse(&self.version).ok())
            .flatten()
            .unwrap_or_else(|| bundled.clone())
    }

    /// From what `build.rs` gave: each is empty when unknown.
    pub fn from_parts(
        channel: &str,
        nightly_version: &str,
        commit: &str,
        date: &str,
        dirty: &str,
        version: &str,
    ) -> BuildInfo {
        let some = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        let commit = some(commit).filter(|c| github::is_sha(c));
        let nightly = channel == "nightly" && commit.is_some() && !nightly_version.is_empty();
        let version = if nightly { nightly_version } else { version }.to_owned();
        let label = match (&commit, nightly) {
            (Some(c), true) => builder::nightly_label(&version, c, some(date).as_deref()),
            _ => version.clone(),
        };
        BuildInfo {
            channel: if nightly { "nightly" } else { "stable" }.into(),
            short: commit.as_deref().map(|c| short(c).to_owned()),
            commit,
            date: some(date),
            dirty: dirty == "true",
            label,
            version,
        }
    }
}

/// This program's build.
pub fn this_build() -> BuildInfo {
    BuildInfo::from_parts(
        env!("PHOTONOXIDE_CHANNEL"),
        env!("PHOTONOXIDE_NIGHTLY_VERSION"),
        env!("PHOTONOXIDE_COMMIT"),
        env!("PHOTONOXIDE_COMMIT_DATE"),
        env!("PHOTONOXIDE_DIRTY"),
        photonoxide::VERSION,
    )
}

/// Whether the stable channel takes `release` in place of this build (`current`): a later one,
/// as always; but from a nightly, whatever the release is, even the one it was made after (a
/// nightly of main at 0.5.4 is `0.5.4+nightly.20261017.4cc3c39`, and the latest release may
/// still be 0.5.4). Switching back to Stable is a deliberate step down. The semver crate's `!=`
/// sees build metadata, so a nightly is never its release.
pub fn stable_accepts(
    nightly_build: bool,
    current: &semver::Version,
    release: &semver::Version,
) -> bool {
    if nightly_build {
        release != current
    } else {
        release > current
    }
}

/// The channel the settings name: `"nightly"`, or `"stable"` for anything else.
pub fn channel_of(setting: &str) -> &'static str {
    if setting == "nightly" {
        "nightly"
    } else {
        "stable"
    }
}

/// The nightly folder: `$PHOTONOXIDE_NIGHTLY_DIR` when set (to try the channel without
/// touching an installed copy's), else `nightly` in the app's local data folder.
pub fn nightly_dir(local_data: Option<PathBuf>) -> PathBuf {
    std::env::var_os("PHOTONOXIDE_NIGHTLY_DIR")
        .map(PathBuf::from)
        .or_else(|| local_data.map(|d| d.join("nightly")))
        .unwrap_or_else(|| std::env::temp_dir().join("photonoxide-nightly"))
}

/// The app's local data folder for `identifier`, as Tauri finds it, for the command line.
pub fn local_data(identifier: &str) -> Option<PathBuf> {
    let var = |v: &str| std::env::var_os(v).map(PathBuf::from);
    Some(match std::env::consts::OS {
        "windows" => var("LOCALAPPDATA")?.join(identifier),
        "macos" => var("HOME")?
            .join("Library")
            .join("Application Support")
            .join(identifier),
        _ => var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local").join("share")))?
            .join(identifier),
    })
}

/// This program's name, identifier and binary, which a nightly build takes.
pub fn identity_of(config: &tauri::Config) -> Identity {
    Identity {
        identifier: config.identifier.clone(),
        product_name: config.product_name.clone(),
        main_binary_name: config.main_binary_name.clone(),
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The nightly channel's state, in the window or on the command line.
pub struct State {
    pub dirs: Dirs,
    identity: Identity,
    checker: Mutex<Checker>,
    build: Mutex<Option<Build>>,
    http: Arc<dyn Http>,
}

impl State {
    pub fn new(root: PathBuf, identity: Identity) -> State {
        let dirs = Dirs { root };
        let checker = read_json(&dirs.check_file()).unwrap_or_default();
        State {
            dirs,
            identity,
            checker: Mutex::new(checker),
            build: Mutex::new(None),
            http: Arc::new(Web {
                user_agent: format!("photonoxide-studio/{}", photonoxide::VERSION),
            }),
        }
    }

    /// Looks at main's head, and places the build waiting and the one running against it and
    /// this build; see [`Checker::check_with`]. A build waiting that the check finds no newer
    /// than this one is set aside.
    pub fn check(&self, quiet: bool) -> Result<Option<Nightly>, String> {
        let build = this_build();
        let waiting = self.waiting().map(|r| r.commit);
        let running = lock(&self.build)
            .as_ref()
            .filter(|b| !b.finished())
            .map(|b| b.progress(usize::MAX).commit);
        let others: Vec<&str> = [waiting.as_deref(), running.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        let result = {
            let mut checker = lock(&self.checker);
            let result = checker.check_with(
                self.http.as_ref(),
                build.commit.as_deref(),
                photonoxide::VERSION,
                &others,
                now(),
                quiet,
            );
            let _ = write_json(&self.dirs.check_file(), &*checker);
            result
        };
        self.offer();
        result
    }

    /// What the panel offers ([`offer::decide`]); a build waiting that is no use is set aside
    /// here.
    pub fn offer(&self) -> offer::Offer {
        let build = this_build();
        let last = lock(&self.checker).last.clone();
        let progress = self.progress(usize::MAX);
        let waiting = self.waiting();
        let mut offer = offer::decide(
            build.commit.as_deref(),
            last.as_ref(),
            waiting.as_ref(),
            progress.as_ref(),
        );
        if let Some((ready, why)) = offer.discard.take() {
            install::log(
                &self.dirs,
                "",
                &format!("set aside the build waiting, {}: {why}", ready.label),
            );
            builder::discard_ready(&self.dirs, &ready);
        }
        offer
    }

    /// The prerequisites here, for what main asks for (as this build's source did, until a
    /// build has main's).
    pub fn prerequisites(&self) -> Report {
        let os = std::env::consts::OS;
        let bundle = Running::here().bundle_needed().unwrap_or(match os {
            "windows" => "nsis",
            "macos" => "app",
            _ => "appimage",
        });
        prereqs::check(os, &System::here(), &Needs::of_this_build(bundle))
    }

    /// Starts building `commit`, which must be main's head as the last check found it.
    pub fn start_build(&self, commit: &str) -> Result<(), String> {
        let shown = lock(&self.checker).last.clone();
        let shown = shown.filter(|n| n.head == commit).ok_or_else(|| {
            format!(
                "{} isn't main's head as photonoxide last saw it: check again first",
                short(commit)
            )
        })?;
        if !github::is_sha(commit) {
            return Err(format!("{commit} isn't a commit"));
        }
        let bundle = Running::here().bundle_needed()?;
        let mut build = lock(&self.build);
        if build.as_ref().is_some_and(|b| !b.finished()) {
            return Err("a build is already running".into());
        }
        let report = self.prerequisites();
        if !report.ok {
            return Err(format!("missing to build here: {}", report.missing()));
        }
        let system = System::here();
        let plan = Plan {
            commit: commit.to_owned(),
            date: shown.head_date.clone(),
            bundle: bundle.to_owned(),
            identity: self.identity.clone(),
            path: system.path_var(),
            timeout: BUILD_TIMEOUT,
        };
        let check: builder::Check = Box::new(move |needs: &Needs| {
            let system = System::here();
            let report = prereqs::check(std::env::consts::OS, &system, needs);
            if !report.ok {
                return Err(format!(
                    "this commit needs what isn't here: {}",
                    report.missing()
                ));
            }
            system
                .find("pnpm")
                .ok_or_else(|| "pnpm isn't found".to_owned())
        });
        *build = Some(Build::start(
            plan,
            self.dirs.clone(),
            self.http.clone(),
            check,
        ));
        Ok(())
    }

    pub fn progress(&self, from: usize) -> Option<Progress> {
        lock(&self.build).as_ref().map(|b| b.progress(from))
    }

    pub fn cancel(&self) {
        if let Some(b) = lock(&self.build).as_ref() {
            b.cancel();
        }
    }

    fn building(&self) -> bool {
        lock(&self.build).as_ref().is_some_and(|b| !b.finished())
    }

    /// Stops the build running and starts again at `commit`, main's newest as the last check
    /// found it: for a build main has moved past.
    pub fn restart_build(&self, commit: &str) -> Result<(), String> {
        if lock(&self.checker).last.as_ref().map(|n| n.head.as_str()) != Some(commit) {
            return Err(format!(
                "{} isn't main's head as photonoxide last saw it: check again first",
                short(commit)
            ));
        }
        self.cancel();
        // the build's processes are stopped within a second or two; its folder is then free
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while self.building() {
            if std::time::Instant::now() > deadline {
                return Err("the build running didn't stop within a minute".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        self.start_build(commit)
    }

    /// The build waiting in the nightly folder, its file there: whatever it is, before the
    /// check has placed it.
    fn waiting(&self) -> Option<Ready> {
        read_json::<Ready>(&self.dirs.ready_file()).filter(|r| r.artifact.exists())
    }

    /// The build waiting to be installed, when it is offered: still there, checked by this
    /// build against main, and newer than this build ([`offer::decide`]).
    pub fn ready(&self) -> Option<Ready> {
        self.offer().ready
    }
}

/// What the window needs of the nightly channel when it opens, and after each change.
#[derive(Serialize)]
pub struct Status {
    pub build: BuildInfo,
    /// The last check's result.
    pub last: Option<Nightly>,
    /// A build running (its progress) or finished and waiting.
    pub progress: Option<Progress>,
    /// The build waiting, when it is offered.
    pub ready: Option<Ready>,
    /// What the panel offers: the build, its commit, date and how far behind main it is.
    pub offer: offer::Offer,
    pub previous: Option<Previous>,
    /// Why this copy can't install a nightly, if it can't.
    pub cannot_install: Option<String>,
    /// Where a nightly goes when this copy isn't installed (a build from the repository): its
    /// first install, which then updates itself.
    pub first_install: Option<String>,
    /// The bundle it would build.
    pub bundle: Option<String>,
    pub folder: String,
}

fn status_of(state: &State) -> Status {
    let running = Running::here();
    let needed = running.bundle_needed();
    let offer = state.offer();
    Status {
        build: this_build(),
        last: lock(&state.checker).last.clone(),
        progress: state.progress(usize::MAX),
        ready: offer.ready.clone(),
        offer,
        previous: install::previous(&state.dirs),
        cannot_install: needed.as_ref().err().cloned(),
        first_install: (!running.installed())
            .then(|| running.target().ok())
            .flatten()
            .map(|t| t.display().to_string()),
        bundle: needed.ok().map(str::to_owned),
        folder: state.dirs.root.display().to_string(),
    }
}

#[tauri::command]
pub fn nightly_status(state: tauri::State<'_, State>) -> Status {
    status_of(&state)
}

/// At start, before the window: an install that didn't take is known as such (see
/// [`install::at_start`]), its watchdog stands down, and the window says so.
pub fn at_start(state: &State) {
    let build = this_build();
    let here = Running::here().target().ok();
    if let Some(note) = install::at_start(
        &state.dirs,
        build.commit.as_deref(),
        here.as_deref(),
        &build.label,
    ) {
        let _ = write_json(&state.dirs.rollback_note(), &note);
    }
}

/// The new build's window is up: confirms its install, and returns what the last rollback or
/// failed install did, once.
#[tauri::command]
pub fn nightly_started(state: tauri::State<'_, State>) -> Option<Note> {
    if let Some(commit) = this_build().commit {
        install::started(&state.dirs, &commit);
    }
    install::take_note(&state.dirs)
}

#[tauri::command(async)]
pub fn nightly_check(
    state: tauri::State<'_, State>,
    quiet: bool,
) -> Result<Option<Nightly>, String> {
    state.check(quiet)
}

#[tauri::command(async)]
pub fn nightly_prerequisites(state: tauri::State<'_, State>) -> Report {
    state.prerequisites()
}

/// Runs a missing prerequisite's install command, by its id, in a terminal of its own. The
/// window asked the user first.
#[tauri::command(async)]
pub fn nightly_install_tool(state: tauri::State<'_, State>, id: String) -> Result<String, String> {
    prereqs::run_fix(&state.prerequisites(), &id)
}

#[tauri::command(async)]
pub fn nightly_build(state: tauri::State<'_, State>, commit: String) -> Result<Status, String> {
    state.start_build(&commit)?;
    Ok(status_of(&state))
}

/// Stops the build running and starts again at main's newest, `commit`. The user chose it.
#[tauri::command(async)]
pub fn nightly_restart(state: tauri::State<'_, State>, commit: String) -> Result<Status, String> {
    state.restart_build(&commit)?;
    Ok(status_of(&state))
}

#[tauri::command]
pub fn nightly_progress(state: tauri::State<'_, State>, from: usize) -> Option<Progress> {
    state.progress(from)
}

#[tauri::command]
pub fn nightly_cancel(state: tauri::State<'_, State>) {
    state.cancel();
}

/// Exits a moment after the window hears back, for an install or a rollback to go ahead.
fn exit_soon(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        app.exit(0);
    });
}

/// Installs the build waiting; on Windows, macOS and an AppImage the program then exits, and
/// the new build opens. The window asked the user first.
#[tauri::command(async)]
pub fn nightly_install(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<Outcome, String> {
    let ready = state
        .ready()
        .ok_or("there is no build waiting to be installed")?;
    let build = this_build();
    let outcome = install::install(
        &state.dirs,
        &ready,
        &Running::here(),
        &build.label,
        build.commit.as_deref(),
    )?;
    if outcome == Outcome::Restart {
        exit_soon(&app);
    }
    Ok(outcome)
}

/// The updater's endpoint (its first) and release key, as tauri.conf.json gives them: the stable
/// channel of a copy that isn't installed reads and checks releases as the updater does.
fn updater_config(app: &tauri::AppHandle) -> Result<(String, String), String> {
    let updater = app
        .config()
        .plugins
        .0
        .get("updater")
        .ok_or("tauri.conf.json has no updater settings")?;
    let endpoint = updater["endpoints"][0]
        .as_str()
        .ok_or("tauri.conf.json's updater has no endpoint")?;
    let key = updater["pubkey"]
        .as_str()
        .ok_or("tauri.conf.json's updater has no release key")?;
    Ok((endpoint.to_owned(), key.to_owned()))
}

/// The latest release for this system, for a copy that isn't installed, which Tauri's updater
/// doesn't update ([`release`]): whether it's later than this build, and what it says.
#[tauri::command(async)]
pub fn release_check(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<release::Release, String> {
    let (endpoint, _) = updater_config(&app)?;
    let build = this_build();
    release::latest(&*state.http, &endpoint, &build.version, build.nightly())
}

/// Downloads the latest release, checks its signature as the updater does, and installs it
/// where a first install goes (or over an older one there); the program then exits, and the
/// release opens. The window follows `release-progress` (bytes, and the size when known).
#[tauri::command(async)]
pub fn release_install(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<Outcome, String> {
    use tauri::Emitter;
    let (endpoint, key) = updater_config(&app)?;
    let build = this_build();
    let latest = release::latest(&*state.http, &endpoint, &build.version, build.nightly())?;
    let commit = release::commit_of(&*state.http, &latest.version)?;
    let emitter = app.clone();
    let ready = release::fetch(
        &*state.http,
        &state.dirs,
        &latest,
        &key,
        &commit,
        &mut |done, total| {
            let _ = emitter.emit("release-progress", (done, total));
        },
    )?;
    let outcome = install::install(
        &state.dirs,
        &ready,
        &Running::here(),
        &build.label,
        build.commit.as_deref(),
    )?;
    if outcome == Outcome::Restart {
        exit_soon(&app);
    }
    Ok(outcome)
}

/// Installs the waiting `.deb` or `.rpm` with pkexec, then restarts into it. The window asked
/// the user first.
#[tauri::command(async)]
pub fn nightly_install_package(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<(), String> {
    let ready = state
        .ready()
        .ok_or("there is no build waiting to be installed")?;
    install::install_package(&ready, &Running::here())?;
    exit_soon(&app);
    Ok(())
}

/// Goes back to the previous build: the program exits, and the previous one opens.
#[tauri::command]
pub fn nightly_roll_back(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<(), String> {
    install::roll_back(&state.dirs, &this_build().label)?;
    exit_soon(&app);
    Ok(())
}

/// Removes the build cache (the source and Cargo's target folder), keeping a build waiting and
/// the previous one.
#[tauri::command(async)]
pub fn nightly_clear_cache(state: tauri::State<'_, State>) -> Result<(), String> {
    if state.building() {
        return Err("a build is running: cancel it first".into());
    }
    for dir in [
        state.dirs.source(),
        state.dirs.target(),
        state.dirs.downloads(),
    ] {
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
    }
    let _ = std::fs::remove_file(state.dirs.last_build());
    Ok(())
}

/// Adds `e` to `install.log` in the nightly folder, and returns it.
fn logged(dirs: &Dirs, e: String) -> String {
    install::log(dirs, "", &format!("error: {e}"));
    e
}

const USAGE: &str =
    "usage: photonoxide nightly <check | prerequisites | build <commit> | install | rollback>
  check            main's head on GitHub, and the commits this build hasn't
  prerequisites    what building here needs, found or not, and how to install what isn't
  build <commit>   build main at <commit>, which must be its head now (it runs main's code)
  install          install the build made, in place of this copy (the program restarts)
  rollback         go back to the build installed before the last install";

/// `photonoxide nightly ...`: the nightly channel from the command line, each step typed by the
/// user. `watch` and `restore` are the install's own, run from the previous build's copy.
pub fn run(args: &[String]) -> ExitCode {
    let config = crate::studio::context().config().clone();
    let identity = identity_of(&config);
    let state = State::new(nightly_dir(local_data(&identity.identifier)), identity);
    let fail = |e: String| {
        eprintln!("error: {e}");
        ExitCode::FAILURE
    };
    let build = this_build();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["check"] => {
            println!(
                "this build: {} ({})",
                build.label,
                build.commit.as_deref().map_or("commit unknown", short)
            );
            match state.check(false) {
                Ok(Some(n)) => {
                    println!(
                        "main: {} ({})",
                        n.head,
                        n.head_date.as_deref().unwrap_or("date unknown")
                    );
                    match n.status.as_str() {
                        "current" => println!("this build is main's head"),
                        "ahead" => println!("main has nothing this build hasn't"),
                        _ => {
                            println!("{} new commits:", n.new_commits);
                            for c in &n.commits {
                                println!("  {} {}", short(&c.sha), c.title);
                            }
                            if let Some(note) = &n.note {
                                println!("{note}");
                            }
                        }
                    }
                    let offer = state.offer();
                    let shown = |c: &Option<String>| c.as_deref().map_or("?", short).to_owned();
                    match (offer.kind.as_str(), &offer.ready) {
                        ("install", Some(r)) => {
                            println!("built and waiting: {} (main's head)", r.label);
                            println!("to install it: photonoxide nightly install");
                        }
                        ("revalidate", Some(r)) => {
                            println!(
                                "built and waiting: {}, but main has {} commits it hasn't",
                                r.label,
                                offer.behind.map_or("some".into(), |b| b.to_string())
                            );
                            for c in &offer.commits {
                                println!("  {} {}", short(&c.sha), c.title);
                            }
                            println!(
                                "to revalidate (build main's newest; the build waiting stays \
                                 until the new one is ready): photonoxide nightly build {}",
                                n.head
                            );
                        }
                        ("build", _) => {
                            println!("to build it here: photonoxide nightly build {}", n.head)
                        }
                        ("building", _) => println!(
                            "building main @ {}, {} commits behind main",
                            shown(&offer.commit),
                            offer.behind.map_or("?".into(), |b| b.to_string())
                        ),
                        _ => {}
                    }
                    ExitCode::SUCCESS
                }
                Ok(None) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            }
        }
        ["prerequisites"] => {
            let report = state.prerequisites();
            for t in &report.tools {
                println!("{} {}", if t.ok { "ok     " } else { "MISSING" }, t.name);
                if let Some(f) = &t.found {
                    println!("        {f}");
                }
                if let Some(p) = &t.problem {
                    println!("        {p}");
                }
                if let Some(fix) = &t.fix {
                    println!(
                        "        install: {}",
                        if fix.command.is_empty() {
                            fix.url
                        } else {
                            &fix.command
                        }
                    );
                }
            }
            if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        ["build", commit] => {
            if let Err(e) = state.check(false) {
                return fail(e);
            }
            if let Err(e) = state.start_build(commit) {
                return fail(e);
            }
            println!(
                "building main at {} in {} (it runs main's code)",
                short(commit),
                state.dirs.root.display()
            );
            let guard = lock(&state.build);
            let progress =
                builder::follow(guard.as_ref().expect("started"), &mut |l| println!("{l}"));
            match progress.ready {
                Some(r) => {
                    println!(
                        "built {} in {:.1} min: {}",
                        r.label,
                        r.seconds / 60.0,
                        r.artifact.display()
                    );
                    println!("to install it: photonoxide nightly install");
                    ExitCode::SUCCESS
                }
                None => {
                    if let Some(t) = &progress.too_old {
                        eprintln!("{}", t.message);
                    }
                    fail(progress.error.unwrap_or_else(|| "the build failed".into()))
                }
            }
        }
        ["install"] => {
            let Some(ready) = state.ready() else {
                return fail("there is no build waiting to be installed".into());
            };
            match install::install(
                &state.dirs,
                &ready,
                &Running::here(),
                &build.label,
                build.commit.as_deref(),
            ) {
                Ok(Outcome::Restart) => {
                    println!(
                        "installing {}: photonoxide opens the new build once it is in, or the \
                         one you have, saying why it isn't (the steps: {})",
                        ready.label,
                        state.dirs.install_log().display()
                    );
                    ExitCode::SUCCESS
                }
                Ok(Outcome::Package { command, pkexec }) => {
                    println!("install the package with: {command}");
                    if let Some(p) = pkexec {
                        println!("or: {p}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        ["rollback"] => match install::roll_back(&state.dirs, &build.label) {
            Ok(()) => {
                println!("going back to the previous build");
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        // run detached, with no terminal: a failure goes to install.log too
        ["watch", file] => match install::watch(&state.dirs, std::path::Path::new(file)) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => fail(logged(&state.dirs, e)),
        },
        ["restore", leaving] => match install::restore(&state.dirs, leaving) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(logged(&state.dirs, e)),
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    const SHA: &str = "c9cae4c1aa1aa1aa1aa1aa1aa1aa1aa1aa1aa1aa";

    #[test]
    fn a_build_says_what_it_was_built_from() {
        let nightly = BuildInfo::from_parts(
            "nightly",
            "0.5.0+nightly.20261009.c9cae4c",
            SHA,
            "2026-10-09",
            "false",
            "0.5.0",
        );
        assert!(nightly.nightly());
        assert_eq!(
            nightly.label,
            "0.5.0+nightly.20261009.c9cae4c (main @ c9cae4c, 2026-10-09)"
        );
        assert_eq!(
            (nightly.version.as_str(), nightly.short.as_deref()),
            ("0.5.0+nightly.20261009.c9cae4c", Some("c9cae4c"))
        );
        // a release, from a checkout of its tag
        let release = BuildInfo::from_parts("", "", SHA, "2026-10-09", "false", "0.5.0");
        assert!(!release.nightly());
        assert_eq!(
            (release.label.as_str(), release.version.as_str()),
            ("0.5.0", "0.5.0")
        );
        // a build from a checkout with changes, and one without git
        assert!(BuildInfo::from_parts("", "", SHA, "", "true", "0.5.0").dirty);
        let bare = BuildInfo::from_parts("", "", "", "", "false", "0.5.0");
        assert_eq!((bare.commit, bare.label.as_str()), (None, "0.5.0"));
        // a nightly that doesn't know its commit isn't taken for one
        assert!(
            !BuildInfo::from_parts("nightly", "0.5.0+nightly.c9cae4c", "", "", "false", "0.5.0")
                .nightly()
        );
        // this program's own
        let own = this_build();
        assert!(own.label.starts_with(&own.version));
    }

    #[test]
    fn the_stable_channel_steps_down_from_a_nightly_but_never_from_a_release() {
        let v = |s: &str| semver::Version::parse(s).unwrap();
        // a release takes only later releases
        assert!(stable_accepts(false, &v("0.5.0"), &v("0.5.1")));
        assert!(!stable_accepts(false, &v("0.5.0"), &v("0.5.0")));
        assert!(!stable_accepts(false, &v("0.5.1"), &v("0.5.0")));
        // a nightly of main at 0.5.0 is not the 0.5.0 release (semver's `==` and `Ord` see the
        // build metadata, though its precedence doesn't): back on Stable, it takes that release
        let nightly = v("0.5.0+nightly.20261009.c9cae4c");
        assert!(nightly != v("0.5.0") && nightly > v("0.5.0"));
        assert!(!stable_accepts(false, &nightly, &v("0.5.0")));
        assert!(stable_accepts(true, &nightly, &v("0.5.0")));
        // an older release too, and the next ones, of course
        assert!(stable_accepts(true, &nightly, &v("0.4.9")));
        assert!(stable_accepts(true, &nightly, &v("0.5.1")));
        assert!(stable_accepts(true, &nightly, &v("0.6.0")));
        // Tauri's updater gives the bundle's version, main's plain 0.5.0: a nightly compares
        // with its own, a release with the bundle's
        let built = |channel, version| {
            BuildInfo::from_parts(channel, version, SHA, "2026-10-09", "false", "0.5.0")
        };
        let bundled = v("0.5.0");
        let nightly_build = built("nightly", "0.5.0+nightly.20261009.c9cae4c");
        assert_eq!(nightly_build.updating_from(&bundled), nightly);
        assert!(stable_accepts(
            nightly_build.nightly(),
            &nightly_build.updating_from(&bundled),
            &v("0.5.0")
        ));
        let release = built("", "");
        assert_eq!(release.updating_from(&bundled), bundled);
        assert!(!stable_accepts(
            release.nightly(),
            &release.updating_from(&bundled),
            &v("0.5.0")
        ));
    }

    #[test]
    fn the_program_has_photonoxide_s_version_and_a_nightly_has_main_s() {
        // the program's crate takes the workspace's version, and tauri.conf.json sets none, so
        // Tauri's (the installer's, and the one the stable updater compares) is photonoxide's
        assert_eq!(env!("CARGO_PKG_VERSION"), photonoxide::VERSION);
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert!(
            config.get("version").is_none(),
            "tauri.conf.json sets a version"
        );
        let v = |s: &str| semver::Version::parse(s).unwrap();
        let release = v(photonoxide::VERSION);
        let mut next = release.clone();
        next.patch += 1;
        next.pre = semver::Prerelease::EMPTY;
        let nightly =
            v(&builder::nightly_version(photonoxide::VERSION, SHA, Some("2026-10-09")).unwrap());
        // a nightly of main has main's version, the release it follows, with build metadata:
        // the same precedence, and before the next release, whatever that turns out to be
        assert_eq!(nightly.cmp_precedence(&release), std::cmp::Ordering::Equal);
        assert!(nightly != release && nightly < next);
        // its bundle has main's version, plain
        assert_eq!(
            v(&builder::bundle_version(photonoxide::VERSION).unwrap()),
            release
        );
        // a build from a checkout of main has the release's version: Stable doesn't offer that
        // release again (it would be a step back), and offers the next
        assert!(!stable_accepts(false, &release, &release));
        assert!(stable_accepts(false, &release, &next));
        // from a nightly, back on Stable, the release it was made after is taken on purpose
        assert!(stable_accepts(true, &nightly, &release));
    }

    #[test]
    fn the_channel_switch_is_a_setting_that_defaults_to_stable() {
        assert_eq!(Settings::default().channel, "stable");
        let old: Settings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(channel_of(&old.channel), "stable");
        let s: Settings = serde_json::from_str(r#"{"channel":"nightly"}"#).unwrap();
        assert_eq!(channel_of(&s.channel), "nightly");
        let back = serde_json::to_string(&s).unwrap();
        assert!(back.contains(r#""channel":"nightly""#));
        // a channel this version doesn't know is Stable
        let s: Settings = serde_json::from_str(r#"{"channel":"beta"}"#).unwrap();
        assert_eq!(channel_of(&s.channel), "stable");
    }

    #[test]
    fn the_nightly_folder_is_the_app_s_unless_another_is_given() {
        if std::env::var_os("PHOTONOXIDE_NIGHTLY_DIR").is_none() {
            assert_eq!(
                nightly_dir(Some(PathBuf::from("/data/app"))),
                PathBuf::from("/data/app/nightly")
            );
        }
        let data = local_data("gr.tachsin.photonoxide").unwrap();
        assert!(data.ends_with("gr.tachsin.photonoxide"));
    }

    #[test]
    fn only_main_s_head_as_last_seen_is_built() {
        let dir = std::env::temp_dir().join(format!("photonoxide-channels-{}", std::process::id()));
        let identity = Identity {
            identifier: "gr.tachsin.photonoxide.test".into(),
            product_name: None,
            main_binary_name: None,
        };
        let state = State::new(dir.clone(), identity);
        // no check yet
        let e = state.start_build(SHA).unwrap_err();
        assert!(e.contains("isn't main's head"), "{e}");
        // a check saw another head
        lock(&state.checker).last = Some(Nightly {
            status: "available".into(),
            head: "7d117d2bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            head_date: None,
            built: None,
            commits: Vec::new(),
            new_commits: 1,
            note: None,
            checked_at: 0,
            placed: true,
            others: Vec::new(),
        });
        assert!(
            state
                .start_build(SHA)
                .unwrap_err()
                .contains("isn't main's head")
        );
        assert!(state.progress(0).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_leftover_build_is_checked_before_it_is_offered_and_set_aside_when_older() {
        // this program's own commit is what the check compares from
        let Some(built) = this_build().commit else {
            eprintln!("skipped: this build doesn't know its commit");
            return;
        };
        let dir = std::env::temp_dir().join(format!("photonoxide-leftover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = Identity {
            identifier: "gr.tachsin.photonoxide.test".into(),
            product_name: None,
            main_binary_name: None,
        };
        let mut state = State::new(dir.clone(), identity);
        // the real case: a build of 2a704d6 left waiting by an earlier session
        let old = "2a704d6f45d84939b13483efc4314daa11c6fb6d";
        let folder = state.dirs.ready_for(old, 1);
        std::fs::create_dir_all(&folder).unwrap();
        let artifact = folder.join("photonoxide_0.5.1_x64-setup.exe");
        std::fs::write(&artifact, "old").unwrap();
        let leftover = Ready {
            commit: old.into(),
            date: Some("2026-10-10".into()),
            version: "0.5.1+nightly.20261010.2a704d6".into(),
            label: "0.5.1+nightly.20261010.2a704d6 (main @ 2a704d6, 2026-10-10)".into(),
            bundle: "nsis".into(),
            artifact: artifact.clone(),
            built_at: 0,
            seconds: 0.0,
        };
        write_json(&state.dirs.ready_file(), &leftover).unwrap();
        // just switched to Nightly, nothing checked: not offered, and not touched either
        assert_eq!(state.offer().kind, "unchecked");
        assert!(state.ready().is_none() && artifact.exists());
        // the check finds main 3 past this build and 40 past the leftover
        let head = "9a9a9a90000000000000000000000000000000cc";
        let api = "https://api.github.com/repos/tachsin/photonoxide";
        let web = github::tests::Canned::default();
        web.answer(&format!("{api}/commits/main"), 200, &[], head.as_bytes());
        let ahead = |n: u32| format!(r#"{{"status":"ahead","ahead_by":{n},"commits":[]}}"#);
        web.answer(
            &format!("{api}/compare/{built}...{head}"),
            200,
            &[],
            ahead(3).as_bytes(),
        );
        web.answer(
            &format!("{api}/compare/{old}...{head}"),
            200,
            &[],
            ahead(40).as_bytes(),
        );
        state.http = Arc::new(web);
        state.check(false).unwrap();
        // never offered, and set aside: main's head is offered instead
        assert!(state.ready().is_none());
        assert!(!state.dirs.ready_file().exists() && !artifact.exists());
        let offer = state.offer();
        assert_eq!(
            (offer.kind.as_str(), offer.commit.as_deref()),
            ("build", Some(head))
        );
        let log = std::fs::read_to_string(state.dirs.install_log()).unwrap();
        assert!(log.contains("no newer than the build running"), "{log}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
