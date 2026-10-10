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
pub mod github;
pub mod install;
pub mod prereqs;

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
    /// The program's version: a release's, or a nightly's (`0.5.1-nightly`).
    pub version: String,
    /// The commit, when known, and its first seven characters.
    pub commit: Option<String>,
    pub short: Option<String>,
    /// The commit's date, `2026-10-09`.
    pub date: Option<String>,
    /// The tree had changes not committed.
    pub dirty: bool,
    /// What Settings calls it: the version, or for a nightly,
    /// `0.5.1-nightly (main @ abc1234, 2026-10-09)`.
    pub label: String,
}

impl BuildInfo {
    pub fn nightly(&self) -> bool {
        self.channel == "nightly"
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
/// as always; but from a nightly, whatever the release is, even one that sorts before it (a
/// nightly of 0.5.0 is `0.5.1-nightly`, and the latest release may still be 0.5.0). Switching
/// back to Stable is a deliberate step down.
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

    /// Looks at main's head; see [`Checker::check`].
    pub fn check(&self, quiet: bool) -> Result<Option<Nightly>, String> {
        let build = this_build();
        let mut checker = lock(&self.checker);
        let result = checker.check(
            self.http.as_ref(),
            build.commit.as_deref(),
            photonoxide::VERSION,
            now(),
            quiet,
        );
        let _ = write_json(&self.dirs.check_file(), &*checker);
        result
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

    /// The build waiting to be installed, if it is still there and isn't this one.
    pub fn ready(&self) -> Option<Ready> {
        let build = this_build();
        read_json::<Ready>(&self.dirs.ready_file())
            .filter(|r| r.artifact.exists() && build.commit.as_deref() != Some(r.commit.as_str()))
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
    pub ready: Option<Ready>,
    pub previous: Option<Previous>,
    /// Why this copy can't install a nightly, if it can't.
    pub cannot_install: Option<String>,
    /// The bundle it would build.
    pub bundle: Option<String>,
    pub folder: String,
}

fn status_of(state: &State) -> Status {
    let running = Running::here();
    let needed = running.bundle_needed();
    Status {
        build: this_build(),
        last: lock(&state.checker).last.clone(),
        progress: state.progress(usize::MAX),
        ready: state.ready(),
        previous: install::previous(&state.dirs),
        cannot_install: needed.as_ref().err().cloned(),
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
                            println!("to build it here: photonoxide nightly build {}", n.head);
                        }
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
                None => fail(progress.error.unwrap_or_else(|| "the build failed".into())),
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
            "0.5.1-nightly",
            SHA,
            "2026-10-09",
            "false",
            "0.5.0",
        );
        assert!(nightly.nightly());
        assert_eq!(nightly.label, "0.5.1-nightly (main @ c9cae4c, 2026-10-09)");
        assert_eq!(
            (nightly.version.as_str(), nightly.short.as_deref()),
            ("0.5.1-nightly", Some("c9cae4c"))
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
            !BuildInfo::from_parts("nightly", "0.5.1-nightly", "", "", "false", "0.5.0").nightly()
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
        // a nightly of 0.5.0 sorts after the 0.5.0 release: back on Stable, it takes it anyway
        assert!(!stable_accepts(false, &v("0.5.1-nightly"), &v("0.5.0")));
        assert!(stable_accepts(true, &v("0.5.1-nightly"), &v("0.5.0")));
        // and the next release, of course
        assert!(stable_accepts(true, &v("0.5.1-nightly"), &v("0.5.1")));
        assert!(stable_accepts(true, &v("0.5.1-nightly"), &v("0.6.0")));
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
}
