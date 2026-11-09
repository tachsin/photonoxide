//! The real setup, run silently into a folder of the test's own (#649).
//!
//! It needs the setup built with the program in it, which CI's builds don't have (they carry no
//! payload), and Windows: so it is ignored, and runs by hand on a Windows machine, from the
//! repository's root, after a build of the program:
//!
//! ```text
//! cd studio && pnpm tauri build --no-bundle && cd ..
//! set PHOTONOXIDE_SETUP_PAYLOAD=target/release/photonoxide.exe
//! cargo test -p photonoxide-setup --release --test real_setup -- --ignored
//! ```
//!
//! (`cargo test -p photonoxide-studio` leaves a debug build of the program in
//! `target/debug/photonoxide.exe`, which serves as well.) Cargo builds the setup with that
//! payload for the test. The setup runs as `/S /D=<folder>`, with the program's
//! `PHOTONOXIDE_SETUP_TEST_ROOT` set (studio/src-tauri/src/setup.rs), so the Start menu's and
//! the Desktop's shortcuts, the settings' folders and the Apps & Features entry are the test's
//! own, never the installed studio's; the temporary folder the setup unpacks into is the test's
//! too. Then the installed program uninstalls itself the same way, and nothing is left.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const TEST_ROOT: &str = "PHOTONOXIDE_SETUP_TEST_ROOT";

/// The program the setup was built with (`PHOTONOXIDE_SETUP_PAYLOAD`, relative to the
/// repository's root as build.rs reads it).
fn payload() -> PathBuf {
    let path = std::env::var_os("PHOTONOXIDE_SETUP_PAYLOAD")
        .filter(|p| !p.is_empty())
        .expect(
            "PHOTONOXIDE_SETUP_PAYLOAD names the program the setup carries: see this file's \
             first lines for how to run the test",
        );
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// `reg query` of the value `name` of the user's key `key`: its text, if it is there.
fn registry(key: &str, name: &str) -> Option<String> {
    let out = Command::new("reg")
        .args(["query", &format!(r"HKCU\{key}"), "/v", name])
        .output()
        .expect("reg runs");
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Waits up to `seconds` for `done`.
fn eventually(seconds: u64, done: impl Fn() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    done()
}

#[test]
#[ignore = "needs the setup built with the program (PHOTONOXIDE_SETUP_PAYLOAD) and Windows"]
fn the_real_setup_installs_silently_and_the_program_uninstalls_itself() {
    let program = payload();
    let expected = std::fs::read(&program)
        .unwrap_or_else(|e| panic!("the payload, {}: {e}", program.display()));
    let name = format!("real-setup-{}", std::process::id());
    let root = std::env::temp_dir().join(format!("photonoxide-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let temp = root.join("temp");
    std::fs::create_dir_all(&temp).unwrap();
    // a folder with a space, given unquoted after /D= as the nightly channel gives it
    let dir = root.join("photo noxide");
    let key = format!(
        r"Software\photonoxide-tests\{}",
        root.file_name().unwrap().to_string_lossy()
    );
    let run = |command: &mut Command| {
        command
            .env(TEST_ROOT, &root)
            .env("TMP", &temp)
            .env("TEMP", &temp)
            .status()
            .expect("it starts")
    };

    let setup = env!("CARGO_BIN_EXE_photonoxide-setup");
    let status = run(Command::new(setup).args(["/S", &format!("/D={}", dir.display())]));
    assert!(status.success(), "the setup: {status}");
    let installed = dir.join("photonoxide.exe");
    assert!(
        std::fs::read(&installed).unwrap() == expected,
        "the installed program is the one the setup carries"
    );
    assert!(root.join(r"start\photonoxide.lnk").exists());
    // no Desktop shortcut unless asked
    assert!(!root.join(r"desktop\photonoxide.lnk").exists());
    let version = registry(&format!(r"{key}\uninstall"), "DisplayVersion").unwrap_or_default();
    assert!(version.contains(env!("CARGO_PKG_VERSION")), "{version}");
    let maker = Command::new("reg")
        .args(["query", &format!(r"HKCU\{key}\maker"), "/ve"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&maker.stdout).contains("photo noxide"),
        "the folder's record"
    );
    let uninstall =
        registry(&format!(r"{key}\uninstall"), "QuietUninstallString").unwrap_or_default();
    assert!(uninstall.contains("uninstall /S"), "{uninstall}");
    // the setup's own folder, where it unpacked the program, is gone
    let unpacked = || {
        std::fs::read_dir(&temp).unwrap().flatten().any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("photonoxide-setup-")
        })
    };
    assert!(
        eventually(15, || !unpacked()),
        "the setup left its folder in {}",
        temp.display()
    );
    // the installed program says its version
    let out = Command::new(&installed).arg("--version").output().unwrap();
    assert!(out.status.success());

    // what Apps & Features' quiet uninstall runs
    let status = run(Command::new(&installed).args(["uninstall", "/S"]));
    assert!(status.success(), "the uninstall: {status}");
    assert!(!root.join(r"start\photonoxide.lnk").exists());
    assert!(registry(&format!(r"{key}\uninstall"), "DisplayVersion").is_none());
    let maker = Command::new("reg")
        .args(["query", &format!(r"HKCU\{key}\maker")])
        .output()
        .unwrap();
    assert!(!maker.status.success(), "the folder's record is left");
    // the program moved itself into the temporary folder and is removed once it has exited
    assert!(
        eventually(30, || !dir.exists()),
        "{} is left",
        dir.display()
    );
    assert!(
        eventually(30, || std::fs::read_dir(&temp).unwrap().next().is_none()),
        "the uninstalled program is left in {}",
        temp.display()
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = Command::new("reg")
        .args(["delete", &format!(r"HKCU\{key}"), "/f"])
        .output();
}
