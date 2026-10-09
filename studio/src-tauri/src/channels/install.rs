//! Installing a nightly build in place of this copy, and going back when it doesn't start.
//!
//! - **Windows:** the build's NSIS per-user installer runs as the program exits, with the
//!   arguments Tauri's updater gives it (`/P /UPDATE /R`: passive, as an update, restarting the
//!   program after).
//! - **macOS:** the `.app` bundle is replaced (copied beside it, then renamed into place) and
//!   opened again.
//! - **Linux:** an AppImage is swapped in place the same way. A `.deb` or `.rpm` install is the
//!   package manager's: the user installs the built package, or lets `pkexec` do it after
//!   confirming.
//!
//! Before replacing anything, the running build is copied to `previous/`. A watchdog, run from
//! that copy (`photonoxide nightly watch`), waits for the new build to say it started (its
//! window calls [`started`]); if it hasn't within [`START_WITHIN`] seconds, the watchdog puts
//! the previous build back and opens it, and that build says what happened. Settings can roll
//! back by hand as well (`photonoxide nightly restore`).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::builder::{Dirs, Ready, read_json, write_json};

/// How long a new build has to start before the previous one is put back, in seconds.
pub const START_WITHIN: u64 = 300;

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

/// An install in progress: written before anything is replaced, confirmed by the new build.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handover {
    /// The new build's commit, and what it is called.
    pub to: String,
    pub to_label: String,
    /// What the build being replaced is called.
    pub from_label: String,
    /// The copy of the build being replaced, and where both are installed.
    pub previous: PathBuf,
    pub target: PathBuf,
    /// By when the new build must have started, seconds since 1970.
    pub deadline: u64,
    pub started: bool,
    pub rolled_back: bool,
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

/// What a rollback did, for the window of the build it went back to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// The build left, and the one gone back to.
    pub left: String,
    pub back_to: String,
    /// The user asked for it; otherwise the new build didn't start in time.
    pub asked: bool,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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

/// Copies a file, or an app bundle's folder, keeping what makes it run (permissions; on macOS,
/// with `ditto`, its signature and links too).
pub fn copy(from: &Path, to: &Path) -> Result<(), String> {
    remove(to);
    if from.is_dir() {
        if cfg!(target_os = "macos") {
            let status = Command::new("ditto")
                .arg(from)
                .arg(to)
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

/// Keeps a copy of what is installed at `target` (the build running now) in `previous/`.
pub fn keep_previous(
    dirs: &Dirs,
    target: &Path,
    label: &str,
    commit: Option<&str>,
) -> Result<Previous, String> {
    let dir = dirs.previous();
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

/// Starts `program` with `args` on its own, to outlive this process.
pub fn launch(program: &Path, args: &[&std::ffi::OsStr]) -> Result<(), String> {
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
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
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
        .map(|_| ())
        .map_err(|e| format!("can't start {}: {e}", program.display()))
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

/// Installs `ready` in place of this copy (`running`, called `from_label`). On Windows, macOS
/// and an AppImage it starts the watchdog and the new build, and the caller exits; a package
/// is left to the user (or [`install_package`]).
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
    let previous = keep_previous(dirs, &target, from_label, from_commit)?;
    let handover = Handover {
        to: ready.commit.clone(),
        to_label: ready.label.clone(),
        from_label: from_label.to_owned(),
        previous: previous.copy.clone(),
        target: target.clone(),
        deadline: now() + START_WITHIN,
        started: false,
        rolled_back: false,
    };
    write_json(&dirs.handover(), &handover)?;
    let _ = std::fs::remove_file(dirs.rollback_note());
    if needed != "nsis" {
        replace(&target, &ready.artifact)?;
    }
    let watch = dirs.handover();
    launch(
        &program_of(&previous.copy),
        &["nightly".as_ref(), "watch".as_ref(), watch.as_os_str()],
    )?;
    if needed == "nsis" {
        run_installer(&ready.artifact)?;
    } else {
        launch(&target, &[])?;
    }
    Ok(Outcome::Restart)
}

/// Runs the NSIS installer as Tauri's updater does: passive, as an update, restarting the
/// program after (`/P /UPDATE /R`). A per-user installer needs no elevation; one that asks for
/// it goes through the shell, which asks the user.
fn run_installer(installer: &Path) -> Result<(), String> {
    const ARGS: [&str; 3] = ["/P", "/UPDATE", "/R"];
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const ELEVATION_REQUIRED: i32 = 740;
        let mut command = Command::new(installer);
        command
            .args(ARGS)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        match command.spawn() {
            Ok(_) => Ok(()),
            Err(e) if e.raw_os_error() == Some(ELEVATION_REQUIRED) => {
                let mut shell = Command::new("cmd");
                shell.raw_arg(format!(
                    "/c start \"\" \"{}\" {}",
                    installer.display(),
                    ARGS.join(" ")
                ));
                crate::libraries::no_window(&mut shell);
                shell
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("can't start {}: {e}", installer.display()))
            }
            Err(e) => Err(format!("can't start {}: {e}", installer.display())),
        }
    }
    #[cfg(not(windows))]
    {
        let args: Vec<&std::ffi::OsStr> = ARGS.iter().map(std::ffi::OsStr::new).collect();
        launch(installer, &args)
    }
}

/// Installs a `.deb` or `.rpm` with `pkexec` (which asks for the password), then restarts into
/// it. The window asked the user first.
pub fn install_package(ready: &Ready, running: &Running) -> Result<(), String> {
    let (_, args) = package_command(ready);
    let status = Command::new("pkexec")
        .args(&args)
        .status()
        .map_err(|e| format!("can't run pkexec: {e}"))?;
    if !status.success() {
        return Err(format!("pkexec {} failed ({status})", args.join(" ")));
    }
    launch(&running.exe, &[])
}

/// What the watchdog does next.
#[derive(Debug, PartialEq)]
pub enum Decision {
    Wait,
    /// The new build started, or a rollback was already made: nothing to do.
    Done,
    Restore,
}

pub fn decide(h: &Handover, now: u64) -> Decision {
    if h.started || h.rolled_back {
        Decision::Done
    } else if now >= h.deadline {
        Decision::Restore
    } else {
        Decision::Wait
    }
}

/// The new build, `commit`, started: it confirms the install, so the watchdog stands down.
/// Returns whether there was an install to confirm.
pub fn started(dirs: &Dirs, commit: &str) -> bool {
    let file = dirs.handover();
    match read_json::<Handover>(&file) {
        Some(mut h) if h.to == commit && !h.started && !h.rolled_back => {
            h.started = true;
            write_json(&file, &h).is_ok()
        }
        _ => false,
    }
}

/// Puts the previous build back at `target`, says so in the note, and opens it.
fn go_back(dirs: &Dirs, previous: &Path, target: &Path, note: &Note) -> Result<(), String> {
    if !note.asked {
        // the build that didn't confirm may still be running, its window stuck
        close(target);
    }
    replace(target, previous)?;
    write_json(&dirs.rollback_note(), note)?;
    launch(target, &[])
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
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::libraries::no_window(&mut command);
    let _ = command.status();
    std::thread::sleep(Duration::from_secs(1));
}

/// `photonoxide nightly watch <handover>`: waits for the new build to start, and goes back to
/// the previous one when it doesn't in time.
pub fn watch(dirs: &Dirs, file: &Path) -> Result<Decision, String> {
    loop {
        let mut h: Handover =
            read_json(file).ok_or_else(|| format!("{} doesn't read", file.display()))?;
        match decide(&h, now()) {
            Decision::Wait => std::thread::sleep(Duration::from_secs(2)),
            Decision::Done => return Ok(Decision::Done),
            Decision::Restore => {
                h.rolled_back = true;
                write_json(file, &h)?;
                let note = Note {
                    left: h.to_label.clone(),
                    back_to: h.from_label.clone(),
                    asked: false,
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
    let note = Note {
        left: leaving.to_owned(),
        back_to: previous.label.clone(),
        asked: true,
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
}

/// What the last rollback did, once: the note is removed as it is read.
pub fn take_note(dirs: &Dirs) -> Option<Note> {
    let file = dirs.rollback_note();
    let note = read_json(&file);
    let _ = std::fs::remove_file(&file);
    note
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_watchdog_waits_stands_down_or_goes_back() {
        let mut h = Handover {
            to: "c9cae4c".into(),
            to_label: "0.5.1-nightly (main @ c9cae4c)".into(),
            from_label: "0.5.0".into(),
            previous: "p".into(),
            target: "t".into(),
            deadline: 100,
            started: false,
            rolled_back: false,
        };
        assert_eq!(decide(&h, 99), Decision::Wait);
        assert_eq!(decide(&h, 100), Decision::Restore);
        h.started = true;
        assert_eq!(decide(&h, 1000), Decision::Done);
        h.started = false;
        h.rolled_back = true;
        assert_eq!(decide(&h, 1000), Decision::Done);
    }

    #[test]
    fn the_new_build_confirms_only_its_own_install() {
        let dir = temp("started");
        let dirs = Dirs { root: dir.clone() };
        assert!(!started(&dirs, "c9cae4c"));
        let h = Handover {
            to: "c9cae4c".into(),
            to_label: String::new(),
            from_label: String::new(),
            previous: "p".into(),
            target: "t".into(),
            deadline: u64::MAX,
            started: false,
            rolled_back: false,
        };
        write_json(&dirs.handover(), &h).unwrap();
        assert!(!started(&dirs, "7d117d2"));
        assert!(started(&dirs, "c9cae4c"));
        assert!(read_json::<Handover>(&dirs.handover()).unwrap().started);
        // once is enough
        assert!(!started(&dirs, "c9cae4c"));
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
        let previous = keep_previous(&dirs, &target, "0.5.0", None).unwrap();
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
        let kept = keep_previous(&dirs, &app, "0.5.0", None).unwrap();
        assert_eq!(kept.copy.file_name().unwrap(), "photonoxide.app");
        assert_eq!(
            program_of(&kept.copy),
            kept.copy.join("Contents").join("MacOS").join("photonoxide")
        );
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
        };
        write_json(&dirs.rollback_note(), &note).unwrap();
        assert_eq!(take_note(&dirs), Some(note));
        assert_eq!(take_note(&dirs), None);
        let _ = std::fs::remove_dir_all(&dirs.root);
    }
}
