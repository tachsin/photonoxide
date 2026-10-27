//! photonoxide's Windows setup.
//!
//! It carries the program (`photonoxide.exe`, compressed by build.rs), unpacks it into a folder
//! of its own in the user's temporary folder, and runs it as `photonoxide setup`, with this
//! setup's own arguments: the window that program opens is the install, in the studio's style
//! (studio/src-tauri/src/setup.rs). Tauri's updater runs a setup with NSIS's flags (`/P` for a
//! window with only the progress, `/S` for none, `/UPDATE`, `/R` to start the program again,
//! `/ARGS` and the arguments to start it with), and so does the nightly channel (with `/D=` for
//! the folder): the program takes them as NSIS did. When it ends, the folder goes and the setup
//! exits with its code.

#![cfg_attr(windows, windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Duration;

/// The program, as BCJ2's streams compressed (build.rs).
const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/photonoxide.exe.bcj2"));
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            say(&format!("The setup couldn't start: {e}"));
            ExitCode::FAILURE
        }
    }
}

/// A window program has no console to write to: it says so in a message box.
#[cfg(windows)]
fn say(text: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("photonoxide setup")
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[cfg(not(windows))]
fn say(text: &str) {
    eprintln!("{text}");
}

fn run() -> Result<ExitCode, String> {
    if PAYLOAD.is_empty() {
        return Err("it was built without the program (PHOTONOXIDE_SETUP_PAYLOAD)".into());
    }
    let dir = std::env::temp_dir().join(format!(
        "photonoxide-setup-{VERSION}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let program = dir.join("photonoxide.exe");
    let unpacked = unpack(PAYLOAD, &program);
    let status = unpacked.and_then(|()| {
        let me = std::env::current_exe().map_err(|e| format!("where this setup is: {e}"))?;
        let mut command = Command::new(&program);
        command
            .arg("setup")
            .arg("--from")
            .arg(me)
            .args(std::env::args_os().skip(1));
        no_console(&mut command);
        command
            .status()
            .map_err(|e| format!("{}: {e}", program.display()))
    });
    remove(&dir);
    let status = status?;
    Ok(status
        .code()
        .and_then(|c| u8::try_from(c).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}

/// Writes the program out of its xz stream.
/// Writes the program out of its streams: the header (its length, then the four streams'),
/// the code and the calls' and jumps' targets as xz, the range coder's as it is, joined by BCJ2.
fn unpack(payload: &[u8], to: &Path) -> Result<(), String> {
    use std::io::{Cursor, Read};
    let damaged = || "its copy of the program is damaged: download the setup again".to_string();
    let number = |at: usize, n: usize| -> Result<u64, String> {
        let bytes = payload.get(at..at + n).ok_or_else(damaged)?;
        Ok(bytes
            .iter()
            .rev()
            .fold(0u64, |v, &b| (v << 8) | u64::from(b)))
    };
    let size = number(0, 8)?;
    let mut at = 24;
    let mut streams = Vec::with_capacity(4);
    for k in 0..4 {
        let n = usize::try_from(number(8 + 4 * k, 4)?).map_err(|_| damaged())?;
        streams.push(payload.get(at..at + n).ok_or_else(damaged)?);
        at += n;
    }
    let unxz = |stream: &[u8]| -> Result<Cursor<Vec<u8>>, String> {
        let mut out = Vec::new();
        lzma_rust2::XzReader::new(stream, false)
            .read_to_end(&mut out)
            .map_err(|_| damaged())?;
        Ok(Cursor::new(out))
    };
    let inputs = vec![
        unxz(streams[0])?,
        unxz(streams[1])?,
        unxz(streams[2])?,
        Cursor::new(streams[3].to_vec()),
    ];
    let mut reader = lzma_rust2::filter::bcj2::Bcj2Reader::new(inputs, size);
    let mut file = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let written = std::io::copy(&mut reader, &mut file)
        .map_err(|e| format!("unpacking the program into {}: {e}", to.display()))?;
    if written == size {
        Ok(())
    } else {
        Err(damaged())
    }
}

/// The folder and what's in it; the web view's helpers may hold their files a moment after the
/// window closes, so it tries again for a few seconds, and leaves it to the system's own
/// cleaning of the temporary folder after that.
fn remove(dir: &Path) {
    for _ in 0..40 {
        if std::fs::remove_dir_all(dir).is_ok() || !dir.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The program is a console program, which lets go of the console it was given alone; started
/// from this window program, it is given none, so no console flashes up.
#[cfg(windows)]
fn no_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_console(_: &mut Command) {}
