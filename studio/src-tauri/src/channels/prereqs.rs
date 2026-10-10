//! What building the studio here needs, found or not, and how to install what isn't: Rust (at
//! the MSRV), Node and pnpm, and Tauri 2's platform prerequisites
//! (<https://v2.tauri.app/start/prerequisites/>).
//!
//! As on the Libraries page (`libraries.rs`), nothing installs by itself. Each missing tool
//! comes with the command that installs it, to copy; where a terminal can be opened (Windows
//! and macOS), the window can run the catalogue's command there, and only after the user
//! confirms ([`run_fix`]). The window names a fix by its id, never sends a command.
//!
//! Tools are looked for on PATH and in the folders their installers use (`~/.cargo/bin`,
//! nvm's, Homebrew's, npm's and pnpm's), so one installed since the studio opened is found,
//! and the build is given the same search path.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

/// A version as three numbers.
pub type Version = (u64, u64, u64);

/// The oldest Node the studio's frontend builds with: Vite 8 needs 20.19 or 22.12, and CI
/// builds with 24.
pub const NODE: Version = (22, 12, 0);
/// The Node version CI builds with, which the install commands install.
pub const NODE_CI: u64 = 24;

/// What the source to be built asks for.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Needs {
    /// The newest `rust-version` of the workspace's and the studio's `Cargo.toml`.
    pub rust: Version,
    pub node: Version,
    /// The pnpm `studio/package.json` pins (its `packageManager`), e.g. `12.4.1`.
    pub pnpm: String,
    /// The bundle the update needs: `nsis`, `app`, `appimage`, `deb` or `rpm`.
    pub bundle: String,
}

/// The first `a.b[.c]` in `text`.
pub fn version(text: &str) -> Option<Version> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let mut parts = text[start..]
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .split('.')
        .map(|p| p.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next()??;
    let patch = parts.next().flatten().unwrap_or(0);
    Some((major, minor, patch))
}

/// `1.95` or `1.95.1`.
pub fn shown(v: Version) -> String {
    if v.2 == 0 {
        format!("{}.{}", v.0, v.1)
    } else {
        format!("{}.{}.{}", v.0, v.1, v.2)
    }
}

impl Needs {
    /// What the source asks for, from its `Cargo.toml` files and `studio/package.json`.
    pub fn from_source(
        cargo_tomls: &[&str],
        package_json: &str,
        bundle: &str,
    ) -> Result<Needs, String> {
        let mut rust = None;
        for text in cargo_tomls {
            let toml: toml::Value =
                toml::from_str(text).map_err(|e| format!("a Cargo.toml doesn't read: {e}"))?;
            let declared = toml
                .get("package")
                .and_then(|p| p.get("rust-version"))
                .or_else(|| {
                    toml.get("workspace")
                        .and_then(|w| w.get("package"))
                        .and_then(|p| p.get("rust-version"))
                })
                .and_then(|v| v.as_str())
                .and_then(version);
            rust = rust.max(declared);
        }
        let package: serde_json::Value = serde_json::from_str(package_json)
            .map_err(|e| format!("studio/package.json doesn't read: {e}"))?;
        let pnpm = package
            .get("packageManager")
            .and_then(|p| p.as_str())
            .and_then(|p| p.strip_prefix("pnpm@"))
            .map(|p| p.split('+').next().unwrap_or(p).to_owned())
            .ok_or("studio/package.json doesn't pin pnpm (packageManager)")?;
        Ok(Needs {
            rust: rust.ok_or("no Cargo.toml declares a rust-version")?,
            node: NODE,
            pnpm,
            bundle: bundle.to_owned(),
        })
    }

    /// What this build's own source asked for: the best guess before main's is downloaded.
    pub fn of_this_build(bundle: &str) -> Needs {
        Needs::from_source(
            &[
                include_str!("../../../../Cargo.toml"),
                include_str!("../../Cargo.toml"),
            ],
            include_str!("../../../package.json"),
            bundle,
        )
        .expect("the program's own Cargo.toml and package.json read")
    }

    fn pnpm_major(&self) -> u64 {
        version(&self.pnpm).map_or(0, |v| v.0)
    }
}

/// How to install a missing tool.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fix {
    /// Its id, for [`run_fix`].
    pub id: &'static str,
    /// The command, exactly as it is copied or run; empty when there is only a page to read.
    pub command: String,
    /// The window can run it in a terminal of its own (after the user confirms).
    pub runs_here: bool,
    /// What to know before running it.
    pub note: String,
    /// The tool's own install page.
    pub url: &'static str,
}

/// One prerequisite, as found.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Tool {
    pub id: &'static str,
    /// What is needed, e.g. "Rust 1.95 or newer".
    pub name: String,
    pub ok: bool,
    /// What was found and where.
    pub found: Option<String>,
    /// What is wrong, when it isn't ok.
    pub problem: Option<String>,
    pub fix: Option<Fix>,
}

/// Every prerequisite on this system.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Report {
    pub ok: bool,
    /// `windows`, `linux` or `macos`.
    pub system: String,
    pub tools: Vec<Tool>,
    pub needs: Needs,
}

impl Report {
    /// The missing tools, in a sentence.
    pub fn missing(&self) -> String {
        self.tools
            .iter()
            .filter(|t| !t.ok)
            .map(|t| {
                format!(
                    "{}: {}",
                    t.name,
                    t.problem.as_deref().unwrap_or("not found")
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// What the checks need of the system; the tests give a made-up one.
pub trait Probe {
    /// `program` on the search path.
    fn find(&self, program: &str) -> Option<PathBuf>;
    /// Runs `program` with `args`: whether it succeeded, and its output and errors.
    fn run(&self, program: &Path, args: &[&str]) -> Option<(bool, String)>;
    fn exists(&self, path: &Path) -> bool;
    fn var(&self, name: &str) -> Option<String>;
    fn read(&self, path: &Path) -> Option<String>;
}

/// This machine, searching `dirs` for programs.
pub struct System {
    pub dirs: Vec<PathBuf>,
}

impl System {
    /// PATH, with the install folders of rustup, nvm, Volta, pnpm, npm and Homebrew around it
    /// (and on Windows, the PATH the registry holds now, which an installer may have changed
    /// since the studio opened).
    pub fn here() -> System {
        let var = |v: &str| std::env::var_os(v).map(PathBuf::from);
        let home = var("HOME").or_else(|| var("USERPROFILE"));
        let mut first = Vec::new();
        first.push(
            var("CARGO_HOME")
                .or_else(|| home.as_ref().map(|h| h.join(".cargo")))
                .map(|c| c.join("bin")),
        );
        if let Some(home) = &home {
            first.push(newest_nvm_node(
                &home.join(".nvm").join("versions").join("node"),
            ));
            first.push(Some(home.join(".volta").join("bin")));
        }
        first.push(var("PNPM_HOME"));
        let mut last: Vec<Option<PathBuf>> = Vec::new();
        if cfg!(windows) {
            first.push(var("APPDATA").map(|a| a.join("npm")));
            first.push(var("LOCALAPPDATA").map(|a| a.join("pnpm")));
            last.extend(registry_path().into_iter().map(Some));
            last.push(var("ProgramFiles").map(|p| p.join("nodejs")));
            last.push(
                var("LOCALAPPDATA").map(|a| a.join("Microsoft").join("WinGet").join("Links")),
            );
        } else {
            if let Some(home) = &home {
                last.push(Some(home.join("Library").join("pnpm")));
                last.push(Some(home.join(".local").join("share").join("pnpm")));
                last.push(Some(home.join(".local").join("bin")));
            }
            for d in [
                "/opt/homebrew/bin",
                "/usr/local/bin",
                "/home/linuxbrew/.linuxbrew/bin",
                "/usr/bin",
                "/bin",
            ] {
                last.push(Some(PathBuf::from(d)));
            }
        }
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut dirs: Vec<PathBuf> = first.into_iter().flatten().collect();
        dirs.extend(std::env::split_paths(&path));
        dirs.extend(last.into_iter().flatten());
        System::with_dirs(dirs)
    }

    /// Searching only `dirs`, each once, in order.
    pub fn with_dirs(dirs: Vec<PathBuf>) -> System {
        let mut seen = Vec::<PathBuf>::new();
        for d in dirs {
            if !d.as_os_str().is_empty() && !seen.contains(&d) {
                seen.push(d);
            }
        }
        System { dirs: seen }
    }

    /// The search path, as PATH, for the build's processes.
    pub fn path_var(&self) -> OsString {
        std::env::join_paths(self.dirs.iter().filter(|d| d.is_dir())).unwrap_or_default()
    }
}

/// The newest Node nvm installed, its `bin`.
fn newest_nvm_node(versions: &Path) -> Option<PathBuf> {
    std::fs::read_dir(versions)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            Some((version(&name)?, e.path().join("bin")))
        })
        .max()
        .map(|(_, bin)| bin)
}

/// The user's and the machine's PATH as the registry holds them now, their `%VAR%`s expanded.
fn registry_path() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let keys = [
        r"HKCU\Environment",
        r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
    ];
    let mut dirs = Vec::new();
    for key in keys {
        let mut command = Command::new("reg");
        command.args(["query", key, "/v", "Path"]);
        let Some((true, text)) = output(command) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            let value = line
                .strip_prefix("Path")
                .or_else(|| line.strip_prefix("PATH"))
                .map(str::trim_start)
                .and_then(|r| r.split_once(char::is_whitespace))
                .map(|(_, v)| v.trim());
            if let Some(value) = value {
                dirs.extend(value.split(';').map(|d| PathBuf::from(expand(d))));
            }
        }
    }
    dirs
}

/// `%VAR%`s replaced by their values.
fn expand(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn output(mut command: Command) -> Option<(bool, String)> {
    command.stdin(Stdio::null());
    crate::libraries::no_window(&mut command);
    let out = command.output().ok()?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Some((out.status.success(), text))
}

impl Probe for System {
    fn find(&self, program: &str) -> Option<PathBuf> {
        let exts: Vec<String> = if cfg!(windows) && Path::new(program).extension().is_none() {
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
                .split(';')
                .filter(|e| !e.is_empty())
                .map(str::to_ascii_lowercase)
                .collect()
        } else {
            vec![String::new()]
        };
        self.dirs.iter().find_map(|dir| {
            exts.iter()
                .map(|e| dir.join(format!("{program}{e}")))
                .find(|p| runnable(p))
        })
    }

    fn run(&self, program: &Path, args: &[&str]) -> Option<(bool, String)> {
        let mut command = Command::new(program);
        command.args(args).env("PATH", self.path_var());
        output(command)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

fn runnable(path: &Path) -> bool {
    let Ok(meta) = path.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    meta.is_file()
}

const RUSTUP_URL: &str = "https://rustup.rs";
const NODE_URL: &str = "https://nodejs.org/en/download";
const PNPM_URL: &str = "https://pnpm.io/installation";
const TAURI_URL: &str = "https://v2.tauri.app/start/prerequisites/";

/// The `system`'s prerequisites as `probe` finds them, for the source that `needs` describes.
pub fn check(system: &str, probe: &dyn Probe, needs: &Needs) -> Report {
    let mut tools = vec![rust(system, probe, needs), node(system, probe, needs)];
    tools.push(pnpm(system, probe, needs));
    match system {
        "windows" => {
            tools.push(msvc(probe));
            tools.push(webview2(probe));
        }
        "macos" => tools.push(xcode(probe)),
        _ => tools.extend(linux(probe, needs)),
    }
    Report {
        ok: tools.iter().all(|t| t.ok),
        system: system.to_owned(),
        tools,
        needs: needs.clone(),
    }
}

fn tool(id: &'static str, name: String) -> Tool {
    Tool {
        id,
        name,
        ok: false,
        found: None,
        problem: None,
        fix: None,
    }
}

fn rustup_fix(system: &str) -> Fix {
    if system == "windows" {
        Fix {
            id: "rustup",
            command: "winget install --id Rustlang.Rustup --source winget".into(),
            runs_here: true,
            note:
                "rustup installs Rust and keeps it up to date. It uses the MSVC build tools, below."
                    .into(),
            url: RUSTUP_URL,
        }
    } else {
        Fix {
            id: "rustup",
            command: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".into(),
            runs_here: system == "macos",
            note: "rustup's own installer, from rustup.rs: it installs Rust in ~/.cargo, for you alone."
                .into(),
            url: RUSTUP_URL,
        }
    }
}

fn rust(system: &str, probe: &dyn Probe, needs: &Needs) -> Tool {
    let mut t = tool("rust", format!("Rust {} or newer", shown(needs.rust)));
    let rustup = probe.find("rustup");
    let (Some(rustc), Some(_cargo)) = (probe.find("rustc"), probe.find("cargo")) else {
        t.problem = Some("rustc and cargo aren't found".into());
        t.fix = Some(rustup_fix(system));
        return t;
    };
    let Some((_, text)) = probe.run(&rustc, &["-vV"]) else {
        t.problem = Some(format!("{} doesn't run", rustc.display()));
        t.fix = Some(rustup_fix(system));
        return t;
    };
    let line = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .map(|v| v.trim().to_owned())
    };
    let found = line("release:").as_deref().and_then(version);
    let host = line("host:").unwrap_or_default();
    t.found = Some(format!(
        "rustc {} ({host}) at {}",
        found.map_or_else(|| "?".into(), shown),
        rustc.display()
    ));
    let update = || {
        match &rustup {
        Some(_) => Fix {
            id: "rustup-update",
            command: "rustup update stable".into(),
            runs_here: system != "linux",
            note: "Updates the stable toolchain in place.".into(),
            url: RUSTUP_URL,
        },
        None => Fix {
            note: "This Rust didn't come from rustup (a system package?): rustup installs a newer one beside it, first on the path.".into(),
            ..rustup_fix(system)
        },
    }
    };
    match found {
        Some(v) if v >= needs.rust => {}
        Some(v) => {
            t.problem = Some(format!(
                "rustc {} is older than {}",
                shown(v),
                shown(needs.rust)
            ));
            t.fix = Some(update());
            return t;
        }
        None => {
            t.problem = Some("rustc doesn't say its version".into());
            t.fix = Some(update());
            return t;
        }
    }
    if system == "windows" && !host.contains("msvc") {
        t.problem = Some(format!(
            "the default toolchain is {host}; Tauri builds with MSVC's"
        ));
        t.fix = Some(Fix {
            id: "rustup-msvc",
            command: "rustup default stable-msvc".into(),
            runs_here: true,
            note: "Makes the MSVC toolchain the default, installing it if it isn't.".into(),
            url: RUSTUP_URL,
        });
        return t;
    }
    t.ok = true;
    t
}

fn node_fix(system: &str, probe: &dyn Probe) -> Fix {
    match system {
        "windows" => Fix {
            id: "node",
            command: "winget install --id OpenJS.NodeJS.LTS --source winget".into(),
            runs_here: true,
            note: "Node's long-term-support release, from the Node.js project.".into(),
            url: NODE_URL,
        },
        "macos" if probe.find("brew").is_some() => Fix {
            id: "node",
            command: format!("brew install node@{NODE_CI}"),
            runs_here: true,
            note: "Homebrew's Node; Homebrew may ask you to link it onto the path.".into(),
            url: NODE_URL,
        },
        _ => Fix {
            id: "node",
            command: format!(
                "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash && \\. \"$HOME/.nvm/nvm.sh\" && nvm install {NODE_CI}"
            ),
            runs_here: system == "macos",
            note: "nvm, as nodejs.org's download page suggests: Node in your home folder, newer than most distributions package.".into(),
            url: NODE_URL,
        },
    }
}

fn node(system: &str, probe: &dyn Probe, needs: &Needs) -> Tool {
    let mut t = tool("node", format!("Node {} or newer", shown(needs.node)));
    let Some(node) = probe.find("node") else {
        t.problem = Some("not found".into());
        t.fix = Some(node_fix(system, probe));
        return t;
    };
    let found = probe
        .run(&node, &["--version"])
        .and_then(|(_, text)| version(&text));
    t.found = Some(format!(
        "Node {} at {}",
        found.map_or_else(|| "?".into(), shown),
        node.display()
    ));
    match found {
        Some(v) if v >= needs.node => t.ok = true,
        Some(v) => {
            t.problem = Some(format!(
                "Node {} is older than {}",
                shown(v),
                shown(needs.node)
            ));
            t.fix = Some(node_fix(system, probe));
        }
        None => {
            t.problem = Some("node doesn't say its version".into());
            t.fix = Some(node_fix(system, probe));
        }
    }
    t
}

fn pnpm(system: &str, probe: &dyn Probe, needs: &Needs) -> Tool {
    let major = needs.pnpm_major();
    let mut t = tool("pnpm", format!("pnpm {major} or newer"));
    let fix = Fix {
        id: "pnpm",
        command: format!("npm install --global pnpm@{}", needs.pnpm),
        runs_here: system != "linux",
        note: format!(
            "With Node's npm. pnpm {} is the one studio/package.json pins.",
            needs.pnpm
        ),
        url: PNPM_URL,
    };
    let Some(pnpm) = probe.find("pnpm") else {
        t.problem = Some("not found".into());
        t.fix = Some(fix);
        return t;
    };
    let found = probe
        .run(&pnpm, &["--version"])
        .and_then(|(_, text)| version(&text));
    t.found = Some(format!(
        "pnpm {} at {}",
        found.map_or_else(|| "?".into(), shown),
        pnpm.display()
    ));
    match found {
        Some(v) if v.0 >= major => t.ok = true,
        Some(v) => {
            t.problem = Some(format!(
                "pnpm {}; the lockfile needs pnpm {major}",
                shown(v)
            ));
            t.fix = Some(fix);
        }
        None => {
            t.problem = Some("pnpm doesn't say its version".into());
            t.fix = Some(fix);
        }
    }
    t
}

fn msvc(probe: &dyn Probe) -> Tool {
    let mut t = tool(
        "msvc",
        "The MSVC build tools (C++ and the Windows SDK)".into(),
    );
    let vswhere = probe
        .var("ProgramFiles(x86)")
        .map(|p| Path::new(&p).join(r"Microsoft Visual Studio\Installer\vswhere.exe"))
        .filter(|p| probe.exists(p));
    let found = vswhere.and_then(|w| {
        probe.run(
            &w,
            &[
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ],
        )
    });
    match found {
        Some((true, text)) if !text.trim().is_empty() => {
            t.ok = true;
            t.found = Some(text.lines().next().unwrap_or("").trim().to_owned());
        }
        _ => {
            t.problem = Some("Visual Studio's C++ build tools aren't found".into());
            t.fix = Some(Fix {
                id: "msvc",
                command: "winget install --id Microsoft.VisualStudio.BuildTools --source winget --override \"--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\"".into(),
                runs_here: true,
                note: "Microsoft's Build Tools for Visual Studio, with \"Desktop development with C++\": several gigabytes, and the installer asks for administrator rights. Its licence is Microsoft's.".into(),
                url: TAURI_URL,
            });
        }
    }
    t
}

fn webview2(probe: &dyn Probe) -> Tool {
    let mut t = tool("webview2", "Microsoft Edge WebView2".into());
    const CLIENT: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let keys = [
        format!(r"HKLM\SOFTWARE\WOW6432Node\{CLIENT}"),
        format!(r"HKLM\SOFTWARE\{CLIENT}"),
        format!(r"HKCU\Software\{CLIENT}"),
    ];
    let found = probe.find("reg").and_then(|reg| {
        keys.iter()
            .find_map(|k| match probe.run(&reg, &["query", k, "/v", "pv"]) {
                Some((true, text)) => text
                    .lines()
                    .find(|l| l.trim_start().starts_with("pv"))
                    .and_then(|l| l.split_whitespace().last())
                    .map(str::to_owned),
                _ => None,
            })
    });
    match found {
        Some(v) => {
            t.ok = true;
            t.found = Some(format!("WebView2 {v}"));
        }
        None => {
            t.problem = Some("the WebView2 runtime isn't found".into());
            t.fix = Some(Fix {
                id: "webview2",
                command: "winget install --id Microsoft.EdgeWebView2Runtime --source winget".into(),
                runs_here: true,
                note: "Windows 10 and 11 normally have it; the studio runs in it.".into(),
                url: TAURI_URL,
            });
        }
    }
    t
}

fn xcode(probe: &dyn Probe) -> Tool {
    let mut t = tool("xcode", "Xcode's command line tools".into());
    let select = probe
        .find("xcode-select")
        .unwrap_or_else(|| PathBuf::from("/usr/bin/xcode-select"));
    match probe.run(&select, &["-p"]) {
        Some((true, text)) if !text.trim().is_empty() => {
            t.ok = true;
            t.found = Some(text.trim().to_owned());
        }
        _ => {
            t.problem = Some("not installed".into());
            t.fix = Some(Fix {
                id: "xcode",
                command: "xcode-select --install".into(),
                runs_here: true,
                note: "Apple's installer opens and asks to confirm; it needs no Xcode.".into(),
                url: TAURI_URL,
            });
        }
    }
    t
}

/// The distribution's package manager, from /etc/os-release.
fn distribution(probe: &dyn Probe) -> &'static str {
    let text = probe.read(Path::new("/etc/os-release")).unwrap_or_default();
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .map(|v| v.trim_matches('"').to_ascii_lowercase())
            .unwrap_or_default()
    };
    let ids = format!("{} {}", field("ID="), field("ID_LIKE="));
    let has = |id: &str| ids.split_whitespace().any(|w| w == id);
    if has("debian") || has("ubuntu") {
        "apt"
    } else if has("fedora") || has("rhel") || has("centos") {
        "dnf"
    } else if has("arch") {
        "pacman"
    } else if has("suse") || has("opensuse") || ids.contains("opensuse") {
        "zypper"
    } else {
        ""
    }
}

/// Tauri 2's Linux prerequisites, as its page gives them for each distribution.
fn linux_fix(probe: &dyn Probe) -> Fix {
    let (command, manager) = match distribution(probe) {
        "apt" => (
            "sudo apt update && sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config",
            "apt",
        ),
        "dnf" => (
            "sudo dnf check-update; sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel libxdo-devel pkgconf-pkg-config && sudo dnf group install \"c-development\"",
            "dnf",
        ),
        "pacman" => (
            "sudo pacman -Syu && sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool",
            "pacman",
        ),
        "zypper" => (
            "sudo zypper up && sudo zypper in webkit2gtk3-devel libopenssl-devel curl wget file libappindicator3-1 librsvg-devel && sudo zypper in -t pattern devel_basis",
            "zypper",
        ),
        _ => ("", ""),
    };
    Fix {
        id: "linux-packages",
        command: command.into(),
        runs_here: false,
        note: if manager.is_empty() {
            "Tauri's page lists the packages for each distribution.".into()
        } else {
            format!(
                "Tauri's list for {manager}: run it in a terminal; sudo asks for your password."
            )
        },
        url: TAURI_URL,
    }
}

fn linux(probe: &dyn Probe, needs: &Needs) -> Vec<Tool> {
    let fix = linux_fix(probe);
    let mut tools = Vec::new();
    let mut build = tool("build-essential", "A C compiler and make".into());
    match (probe.find("cc"), probe.find("make")) {
        (Some(cc), Some(make)) => {
            build.ok = true;
            build.found = Some(format!("{} and {}", cc.display(), make.display()));
        }
        (cc, _) => {
            build.problem = Some(format!(
                "{} isn't found",
                if cc.is_none() { "cc" } else { "make" }
            ));
            build.fix = Some(fix.clone());
        }
    }
    tools.push(build);
    let pkg_config = probe.find("pkg-config").or_else(|| probe.find("pkgconf"));
    // the libraries, by their pkg-config names; the first of several that is found counts
    let libraries: [(&'static str, &str, &[&str]); 3] = [
        ("webkit2gtk", "webkit2gtk-4.1", &["webkit2gtk-4.1"]),
        (
            "appindicator",
            "libayatana-appindicator or libappindicator",
            &["ayatana-appindicator3-0.1", "appindicator3-0.1"],
        ),
        ("librsvg", "librsvg", &["librsvg-2.0"]),
    ];
    for (id, name, modules) in libraries {
        let mut t = tool(id, name.into());
        match &pkg_config {
            None => t.problem = Some("pkg-config isn't found, to look for it".into()),
            Some(pc) => {
                let found =
                    modules
                        .iter()
                        .find_map(|m| match probe.run(pc, &["--modversion", m]) {
                            Some((true, v)) => Some(format!("{m} {}", v.trim())),
                            _ => None,
                        });
                match found {
                    Some(f) => {
                        t.ok = true;
                        t.found = Some(f);
                    }
                    None => t.problem = Some("its development files aren't found".into()),
                }
            }
        }
        if !t.ok {
            t.fix = Some(fix.clone());
        }
        tools.push(t);
    }
    if needs.bundle == "appimage" {
        let mut t = tool("file", "file (for the AppImage)".into());
        match probe.find("file") {
            Some(f) => {
                t.ok = true;
                t.found = Some(f.display().to_string());
            }
            None => {
                t.problem = Some("not found".into());
                t.fix = Some(fix);
            }
        }
        tools.push(t);
    }
    tools
}

/// Runs the fix `id` of a missing prerequisite in a terminal of its own, which stays open to
/// show what it did; returns the command. Only a fix of the report made now, and only one the
/// window may run here; the window asked the user first.
pub fn run_fix(report: &Report, id: &str) -> Result<String, String> {
    let fix = report
        .tools
        .iter()
        .filter(|t| !t.ok)
        .filter_map(|t| t.fix.as_ref())
        .find(|f| f.id == id)
        .ok_or_else(|| format!("nothing missing here is installed by {id}"))?;
    if !fix.runs_here || fix.command.is_empty() {
        return Err(format!("{} is copied and run by you", fix.command));
    }
    in_a_terminal(&fix.command)?;
    Ok(fix.command.clone())
}

/// Opens a terminal running `command`, left open afterwards.
fn in_a_terminal(command: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // start opens a console of its own, whose cmd /k runs the line and stays open; the line
        // goes as it is, its quotes for winget's --override intact
        Command::new("cmd")
            .raw_arg(format!(
                "/c start \"photonoxide: installing\" cmd /k {command}"
            ))
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("can't open a terminal: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"\ntell application \"Terminal\" to activate",
            command.replace('\\', "\\\\").replace('"', "\\\"")
        );
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("can't open Terminal: {e}"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(format!("copy it into a terminal: {command}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A made-up system: the programs it has, what each answers, and its files.
    #[derive(Default)]
    struct Fake {
        programs: HashMap<&'static str, PathBuf>,
        answers: HashMap<String, (bool, String)>,
        files: HashMap<PathBuf, String>,
        vars: HashMap<&'static str, String>,
    }

    impl Fake {
        fn with(mut self, program: &'static str, args: &str, answer: &str) -> Fake {
            let path = PathBuf::from(format!("/fake/bin/{program}"));
            self.programs.insert(program, path.clone());
            if !args.is_empty() {
                self.answers.insert(
                    format!("{} {args}", path.display()),
                    (true, answer.to_owned()),
                );
            }
            self
        }
    }

    impl Probe for Fake {
        fn find(&self, program: &str) -> Option<PathBuf> {
            self.programs.get(program).cloned()
        }
        fn run(&self, program: &Path, args: &[&str]) -> Option<(bool, String)> {
            let key = format!("{} {}", program.display(), args.join(" "));
            Some(
                self.answers
                    .get(&key)
                    .cloned()
                    .unwrap_or((false, String::new())),
            )
        }
        fn exists(&self, path: &Path) -> bool {
            self.files.contains_key(path)
        }
        fn var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }
        fn read(&self, path: &Path) -> Option<String> {
            self.files.get(path).cloned()
        }
    }

    fn needs(bundle: &str) -> Needs {
        Needs {
            rust: (1, 95, 0),
            node: NODE,
            pnpm: "12.4.1".into(),
            bundle: bundle.into(),
        }
    }

    fn rust_and_node(fake: Fake, rustc: &str, host: &str, node: &str, pnpm: &str) -> Fake {
        fake.with("rustup", "", "")
            .with("cargo", "", "")
            .with(
                "rustc",
                "-vV",
                &format!("rustc {rustc} (abc 2026-09-28)\nbinary: rustc\nhost: {host}\nrelease: {rustc}\n"),
            )
            .with("node", "--version", node)
            .with("pnpm", "--version", pnpm)
    }

    #[test]
    fn versions_read() {
        assert_eq!(
            version("rustc 1.99.0 (b940084d7 2026-09-28)"),
            Some((1, 99, 0))
        );
        assert_eq!(version("v24.11.0\n"), Some((24, 11, 0)));
        assert_eq!(version("1.95"), Some((1, 95, 0)));
        assert_eq!(version("pnpm"), None);
        assert_eq!(shown((1, 95, 0)), "1.95");
        assert_eq!(shown((22, 12, 1)), "22.12.1");
    }

    #[test]
    fn needs_come_from_the_source() {
        let root =
            "[package]\nname = \"photonoxide\"\nversion = \"0.5.0\"\nrust-version = \"1.95\"\n";
        let studio = "[package]\nname = \"s\"\nrust-version = \"1.97\"\n";
        let package = r#"{"name":"s","packageManager":"pnpm@12.4.1+sha512.abc"}"#;
        let n = Needs::from_source(&[root, studio], package, "nsis").unwrap();
        assert_eq!((n.rust, n.pnpm.as_str()), ((1, 97, 0), "12.4.1"));
        assert!(Needs::from_source(&[root], "{}", "nsis").is_err());
        // this build's own: the repository's
        let own = Needs::of_this_build("nsis");
        assert!(own.rust >= (1, 95, 0) && own.pnpm_major() >= 12, "{own:?}");
    }

    #[test]
    fn windows_with_everything() {
        let fake = rust_and_node(
            Fake::default(),
            "1.99.0",
            "x86_64-pc-windows-msvc",
            "v24.11.0",
            "12.4.1",
        )
        .with("reg", "", "");
        let mut fake = fake;
        let vswhere =
            PathBuf::from(r"C:\PF86").join(r"Microsoft Visual Studio\Installer\vswhere.exe");
        fake.vars.insert("ProgramFiles(x86)", r"C:\PF86".into());
        fake.files.insert(vswhere.clone(), String::new());
        fake.answers.insert(
            format!("{} -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath", vswhere.display()),
            (true, "C:\\Program Files\\Microsoft Visual Studio\\18\\Community\r\n".into()),
        );
        fake.answers.insert(
            r"/fake/bin/reg query HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5} /v pv".into(),
            (true, "\r\nHKEY_LOCAL_MACHINE\\...\r\n    pv    REG_SZ    141.0.3537.71\r\n".into()),
        );
        let r = check("windows", &fake, &needs("nsis"));
        assert!(r.ok, "{}", r.missing());
        let ids: Vec<&str> = r.tools.iter().map(|t| t.id).collect();
        assert_eq!(ids, ["rust", "node", "pnpm", "msvc", "webview2"]);
        assert_eq!(r.tools[4].found.as_deref(), Some("WebView2 141.0.3537.71"));
    }

    #[test]
    fn windows_missing_and_old_tools_say_what_installs_them() {
        let fake = rust_and_node(
            Fake::default(),
            "1.90.0",
            "x86_64-pc-windows-gnu",
            "v20.11.0",
            "9.15.0",
        );
        let r = check("windows", &fake, &needs("nsis"));
        assert!(!r.ok);
        let by = |id: &str| r.tools.iter().find(|t| t.id == id).unwrap();
        assert_eq!(
            by("rust").problem.as_deref(),
            Some("rustc 1.90 is older than 1.95")
        );
        assert_eq!(
            by("rust").fix.as_ref().unwrap().command,
            "rustup update stable"
        );
        assert!(
            by("node")
                .problem
                .as_ref()
                .unwrap()
                .contains("older than 22.12")
        );
        assert_eq!(
            by("pnpm").fix.as_ref().unwrap().command,
            "npm install --global pnpm@12.4.1"
        );
        let msvc = by("msvc").fix.as_ref().unwrap();
        assert!(msvc.runs_here && msvc.command.contains("Microsoft.VisualStudio.BuildTools"));
        assert_eq!(by("webview2").fix.as_ref().unwrap().id, "webview2");
        // a new enough GNU toolchain: the MSVC one is asked for
        let fake = rust_and_node(
            Fake::default(),
            "1.99.0",
            "x86_64-pc-windows-gnu",
            "v24.0.0",
            "12.4.1",
        );
        let r = check("windows", &fake, &needs("nsis"));
        assert_eq!(
            r.tools[0].fix.as_ref().unwrap().command,
            "rustup default stable-msvc"
        );
        // nothing at all
        let r = check("windows", &Fake::default(), &needs("nsis"));
        assert!(r.tools.iter().all(|t| !t.ok && t.fix.is_some()));
        assert_eq!(
            r.tools[0].fix.as_ref().unwrap().command,
            "winget install --id Rustlang.Rustup --source winget"
        );
        assert!(
            r.missing()
                .starts_with("Rust 1.95 or newer: rustc and cargo aren't found; Node")
        );
    }

    #[test]
    fn linux_checks_tauri_s_libraries_and_gives_the_distribution_s_command() {
        let mut fake = rust_and_node(
            Fake::default(),
            "1.99.0",
            "x86_64-unknown-linux-gnu",
            "v24.1.0",
            "12.4.1",
        )
        .with("cc", "", "")
        .with("make", "", "")
        .with("pkg-config", "--modversion webkit2gtk-4.1", "2.48.0\n")
        .with("pkg-config", "--modversion appindicator3-0.1", "12.10.0\n");
        fake.files.insert(
            "/etc/os-release".into(),
            "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n".into(),
        );
        let r = check("linux", &fake, &needs("appimage"));
        let by = |id: &str| r.tools.iter().find(|t| t.id == id).unwrap();
        assert!(by("webkit2gtk").ok && by("build-essential").ok);
        // libappindicator counts when libayatana-appindicator isn't there
        assert_eq!(
            by("appindicator").found.as_deref(),
            Some("appindicator3-0.1 12.10.0")
        );
        assert!(!by("librsvg").ok && !by("file").ok && !r.ok);
        let fix = by("librsvg").fix.as_ref().unwrap();
        assert!(
            fix.command
                .starts_with("sudo apt update && sudo apt install libwebkit2gtk-4.1-dev")
        );
        assert!(!fix.runs_here);
        // Fedora; no file check for a .deb or .rpm
        fake.files
            .insert("/etc/os-release".into(), "ID=fedora\n".into());
        let r = check("linux", &fake, &needs("rpm"));
        assert!(r.tools.iter().all(|t| t.id != "file"));
        assert!(
            r.tools
                .iter()
                .find(|t| t.id == "librsvg")
                .unwrap()
                .fix
                .as_ref()
                .unwrap()
                .command
                .contains("dnf install webkit2gtk4.1-devel")
        );
        // no pkg-config: the libraries can't be looked for
        let fake = rust_and_node(
            Fake::default(),
            "1.99.0",
            "x86_64-unknown-linux-gnu",
            "v24.1.0",
            "12.4.1",
        );
        let r = check("linux", &fake, &needs("deb"));
        assert_eq!(
            r.tools
                .iter()
                .find(|t| t.id == "webkit2gtk")
                .unwrap()
                .problem
                .as_deref(),
            Some("pkg-config isn't found, to look for it")
        );
    }

    #[test]
    fn macos_needs_the_command_line_tools() {
        let fake = rust_and_node(
            Fake::default(),
            "1.99.0",
            "aarch64-apple-darwin",
            "v24.1.0",
            "12.4.1",
        );
        let r = check("macos", &fake, &needs("app"));
        let xcode = r.tools.last().unwrap();
        assert_eq!((xcode.id, xcode.ok), ("xcode", false));
        assert_eq!(
            xcode.fix.as_ref().unwrap().command,
            "xcode-select --install"
        );
        let fake = rust_and_node(
            Fake::default(),
            "1.99.0",
            "aarch64-apple-darwin",
            "v24.1.0",
            "12.4.1",
        )
        .with(
            "xcode-select",
            "-p",
            "/Library/Developer/CommandLineTools\n",
        );
        assert!(check("macos", &fake, &needs("app")).ok);
        // Node missing: Homebrew's when there is Homebrew, nvm's otherwise
        let fake = Fake::default().with("brew", "", "");
        let node = check("macos", &fake, &needs("app")).tools[1].clone();
        assert_eq!(node.fix.unwrap().command, "brew install node@24");
        let node = check("macos", &Fake::default(), &needs("app")).tools[1].clone();
        assert!(node.fix.unwrap().command.contains("nvm install 24"));
    }

    #[test]
    fn only_a_fix_of_something_missing_and_runnable_here_runs() {
        let r = check("linux", &Fake::default(), &needs("deb"));
        // Linux's are copied, not run
        assert!(
            run_fix(&r, "linux-packages")
                .unwrap_err()
                .contains("copied and run by you")
        );
        assert!(
            run_fix(&r, "no-such-fix")
                .unwrap_err()
                .contains("nothing missing")
        );
        let ok = rust_and_node(
            Fake::default(),
            "1.99.0",
            "x86_64-pc-windows-msvc",
            "v24.0.0",
            "12.4.1",
        );
        let r = check("windows", &ok, &needs("nsis"));
        // Rust is fine here: its fix isn't offered, so it can't be run
        assert!(
            run_fix(&r, "rustup")
                .unwrap_err()
                .contains("nothing missing")
        );
    }

    /// Fake programs in a folder of their own, found through it alone, as PATH would be.
    #[test]
    fn programs_are_found_on_a_given_path_and_answer() {
        let dir = std::env::temp_dir().join(format!("photonoxide-prereqs-{}", std::process::id()));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let script = |name: &str, says: &str| {
            if cfg!(windows) {
                std::fs::write(bin.join(format!("{name}.cmd")), format!("@echo {says}\r\n"))
                    .unwrap();
            } else {
                let path = bin.join(name);
                std::fs::write(&path, format!("#!/bin/sh\necho '{says}'\n")).unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                        .unwrap();
                }
            }
        };
        script("node", "v24.11.0");
        script("pnpm", "12.4.1");
        // not executable on Unix, so not a program; on Windows, no extension PATHEXT knows
        std::fs::write(bin.join("rustc"), "not a program").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(bin.join("rustc"), std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
        let system = System::with_dirs(vec![dir.join("missing"), bin.clone(), bin.clone()]);
        assert_eq!(system.dirs.len(), 2);
        assert_eq!(system.find("rustc"), None);
        let node = system.find("node").unwrap();
        assert!(node.starts_with(&bin));
        assert_eq!(
            system.run(&node, &["--version"]).unwrap().1.trim(),
            "v24.11.0"
        );
        // the build's PATH is the folders that exist
        assert_eq!(system.path_var(), std::env::join_paths([&bin]).unwrap());
        let n = needs(if cfg!(windows) { "nsis" } else { "deb" });
        let os = if cfg!(windows) { "windows" } else { "linux" };
        let r = check(os, &system, &n);
        let by = |id: &str| r.tools.iter().find(|t| t.id == id).unwrap().clone();
        assert!(by("node").ok && by("pnpm").ok, "{}", r.missing());
        assert!(!by("rust").ok);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn registry_paths_expand_their_variables() {
        let path = std::env::var("PATH").unwrap();
        assert_eq!(
            expand("%PATH%\\.cargo\\bin"),
            format!("{path}\\.cargo\\bin")
        );
        assert_eq!(expand("%NO_SUCH_VAR_HERE%\\x"), "%NO_SUCH_VAR_HERE%\\x");
        assert_eq!(expand("50%"), "50%");
    }
}
