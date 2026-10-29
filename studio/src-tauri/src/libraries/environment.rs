//! photonoxide's own environment for the external libraries (#564): a conda environment of the
//! user's, which the Libraries page installs into with one click, isolated from any other conda
//! or Python on the machine, as Pinokio keeps each app's.
//!
//! - **Where:** [`photonoxide_native::own_environment`], which the libraries' loader searches
//!   right after the user's settings and variables, so what is installed here is found by the
//!   program and by `photonoxide libraries` with nothing to set.
//! - **With:** micromamba (BSD-3-Clause, mamba-org), conda-forge's single-file installer,
//!   downloaded once into the program's `tools` folder beside it and checked against the
//!   SHA-256 its release publishes, kept here ([`MICROMAMBA`]). No Python or conda is needed on the
//!   machine; the user's own `.condarc` and package cache are left alone (`--no-rc`, a root of
//!   its own).
//! - **What:** a library's conda-forge packages, as its guide lists them (`conda install -c
//!   conda-forge …`), and only those: the window sends a library and an install id, never a
//!   command. Each install is recorded ([`installed`]), so the page offers to remove only what
//!   the program put there.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The micromamba release used, and each platform's file of it with the SHA-256 the release
/// publishes beside it (`micromamba-<platform>.sha256`), read 2026-10-29. A newer release is
/// taken by changing both here.
pub const MICROMAMBA: &str = "2.9.0-0";
const BUILDS: &[(&str, &str, &str, &str)] = &[
    (
        "windows",
        "x86_64",
        "win-64",
        "a6d804394b2418991c4e29562853eaace2f2ce9d9da661a98e74e02e8dbb44b0",
    ),
    (
        "linux",
        "x86_64",
        "linux-64",
        "366cd9cd8be14df1ab8ed50352a82111082a36686b2d389fdb79a92c3fafb3e3",
    ),
    (
        "linux",
        "aarch64",
        "linux-aarch64",
        "9f93b974adcb4d166996af969b6cd371287d1a3e52733704727884d9b74cb7a7",
    ),
    (
        "macos",
        "x86_64",
        "osx-64",
        "1e71054bb3ac9a076e21f7ec48acfef536f9b3f1408f371a942784bf5ef83d8a",
    ),
    (
        "macos",
        "aarch64",
        "osx-arm64",
        "ec2a072f028e1a7cf20f3e2e74d5a8127cf5a5f27636375b5359811565f4e5be",
    ),
];

/// This system's micromamba: its platform name and SHA-256.
fn build_here() -> Result<(&'static str, &'static str), String> {
    BUILDS
        .iter()
        .find(|(os, arch, _, _)| *os == std::env::consts::OS && *arch == std::env::consts::ARCH)
        .map(|(_, _, platform, sha)| (*platform, *sha))
        .ok_or_else(|| {
            format!(
                "conda-forge has no micromamba for {} on {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })
}

/// The environment's folder.
pub fn prefix() -> Result<PathBuf, String> {
    photonoxide_native::own_environment()
        .ok_or_else(|| "the user's local data folder isn't known".to_owned())
}

/// The program's tools, beside the environment: micromamba, its root and its package cache.
fn tools() -> Result<PathBuf, String> {
    let prefix = prefix()?;
    Ok(prefix
        .parent()
        .map_or_else(|| prefix.join("tools"), |p| p.join("tools")))
}

/// The marker the setup's Advanced path leaves for the first start: open on the performance
/// step. In the tools folder, beside the record, never in the program's settings.
fn performance_marker() -> Result<PathBuf, String> {
    Ok(tools()?.join("performance-on-start"))
}

/// Leaves the marker (the setup, on its Advanced path).
pub fn leave_performance_marker() -> Result<(), String> {
    let file = performance_marker()?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&file, b"").map_err(|e| format!("{}: {e}", file.display()))
}

/// Whether the marker was there; it's removed as it's read.
pub fn take_performance_marker() -> bool {
    performance_marker().is_ok_and(|f| std::fs::remove_file(f).is_ok())
}

/// What the program installed: by library, the install id and the packages.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Installed {
    pub libraries: BTreeMap<String, Record>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub manager: String,
    pub packages: Vec<String>,
    /// When, seconds since 1970.
    pub at: u64,
}

fn record_file() -> Result<PathBuf, String> {
    Ok(tools()?.join("installed.json"))
}

/// What the program installed, as recorded.
pub fn installed() -> Installed {
    record_file()
        .ok()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(installed: &Installed) -> Result<(), String> {
    let file = record_file()?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(installed).map_err(|e| e.to_string())?;
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))
}

/// A conda-forge guide's packages: what follows `-c conda-forge` in its command.
pub fn packages_of(command: &str) -> Vec<String> {
    let words: Vec<&str> = command.split_whitespace().collect();
    words
        .iter()
        .position(|w| *w == "conda-forge")
        .map(|at| {
            words[at + 1..]
                .iter()
                .filter(|w| !w.starts_with('-'))
                .map(|w| (*w).to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// micromamba, downloaded once and checked: every use checks the file against its SHA-256
/// again, so a file changed on disk isn't run.
pub fn micromamba(say: &mut dyn FnMut(&str)) -> Result<PathBuf, String> {
    let (platform, expected) = build_here()?;
    let dir = tools()?.join(format!("micromamba-{MICROMAMBA}"));
    let file = dir.join(if cfg!(windows) {
        "micromamba.exe"
    } else {
        "micromamba"
    });
    if let Ok(bytes) = std::fs::read(&file)
        && sha256(&bytes) == expected
    {
        return Ok(file);
    }
    let url = format!(
        "https://github.com/mamba-org/micromamba-releases/releases/download/{MICROMAMBA}/micromamba-{platform}"
    );
    say(&format!(
        "Downloading micromamba {MICROMAMBA} (conda-forge's installer, BSD-3-Clause) from {url}"
    ));
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let mut response = reqwest::blocking::Client::builder()
        .user_agent(format!("photonoxide/{}", photonoxide::VERSION))
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .and_then(|c| c.get(&url).send())
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|e| format!("micromamba couldn't be downloaded ({e}): check the connection"))?;
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .map_err(|e| format!("micromamba's download stopped: {e}"))?;
    let got = sha256(&bytes);
    if got != expected {
        return Err(format!(
            "micromamba's download isn't the release's file (SHA-256 {got}, the release's {expected}): nothing was run"
        ));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(&file, &bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("{}: {e}", file.display()))?;
    }
    say(&format!("micromamba checked (SHA-256 {got})"));
    Ok(file)
}

/// Runs `program` with `args`, each line of its output to `say` as it comes; its outcome.
pub fn run(
    program: &Path,
    args: &[String],
    envs: &[(&str, PathBuf)],
    say: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        command.env(k, v);
    }
    super::no_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("can't start {}: {e}", program.display()))?;
    // stderr on a thread of its own, so neither pipe fills while the other is read
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let err = child.stderr.take().map(|e| {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(e).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        })
    });
    let out = child.stdout.take().map(|o| {
        std::thread::spawn(move || {
            for line in BufReader::new(o).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        })
    });
    for line in rx {
        // progress bars redraw with carriage returns: the last state of each line
        let line = line.rsplit('\r').next().unwrap_or("").trim_end().to_owned();
        if !line.is_empty() {
            say(&line);
        }
    }
    for t in [err, out].into_iter().flatten() {
        let _ = t.join();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} {} ({status})",
            program.display(),
            args.first().map_or("", String::as_str)
        ))
    }
}

/// micromamba's arguments for the environment, isolated: its own root and cache, no `.condarc`,
/// conda-forge alone.
fn isolated(verb: &str, prefix: &Path, root: &Path, packages: &[String]) -> Vec<String> {
    let mut args: Vec<String> = [verb, "--yes", "--no-rc", "--root-prefix"]
        .map(str::to_owned)
        .to_vec();
    args.push(root.display().to_string());
    args.push("--prefix".into());
    args.push(prefix.display().to_string());
    if verb != "remove" {
        args.extend(["--override-channels", "--channel", "conda-forge"].map(str::to_owned));
    }
    args.extend(packages.iter().cloned());
    args
}

/// Installs `library`'s conda-forge `packages` (its guide's install `id`) into the environment,
/// creating it the first time, and records them.
pub fn install(
    library: &str,
    id: &str,
    packages: &[String],
    say: &mut dyn FnMut(&str),
) -> Result<(), String> {
    if packages.is_empty() {
        return Err(format!("{library}'s install {id} names no package"));
    }
    let mamba = micromamba(say)?;
    let prefix = prefix()?;
    let root = tools()?.join("mamba");
    let verb = if prefix.join("conda-meta").is_dir() {
        "install"
    } else {
        "create"
    };
    say(&format!(
        "Installing {} from conda-forge into photonoxide's own environment, {}",
        packages.join(", "),
        prefix.display()
    ));
    run(
        &mamba,
        &isolated(verb, &prefix, &root, packages),
        &[("MAMBA_ROOT_PREFIX", root.clone())],
        say,
    )?;
    remember(library, id, "conda-forge", packages)
}

/// Records that the program installed `library` by its guide's install `id`, with `manager`:
/// what the page offers to remove.
pub fn remember(library: &str, id: &str, manager: &str, packages: &[String]) -> Result<(), String> {
    let mut record = installed();
    record.libraries.insert(
        library.to_owned(),
        Record {
            id: id.to_owned(),
            manager: manager.to_owned(),
            packages: packages.to_vec(),
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        },
    );
    save(&record)
}

/// Forgets `library`'s install, once its own manager has removed it.
pub fn forget(library: &str) -> Result<(), String> {
    let mut record = installed();
    record.libraries.remove(library);
    save(&record)
}

/// Removes what the program installed for `library` from the environment.
pub fn remove(library: &str, say: &mut dyn FnMut(&str)) -> Result<(), String> {
    let mut record = installed();
    let Some(r) = record.libraries.get(library).cloned() else {
        return Err(format!("photonoxide didn't install {library}"));
    };
    let mamba = micromamba(say)?;
    let prefix = prefix()?;
    let root = tools()?.join("mamba");
    // a package another library installed too stays
    let shared: Vec<String> = record
        .libraries
        .iter()
        .filter(|(name, _)| name.as_str() != library)
        .flat_map(|(_, other)| other.packages.clone())
        .collect();
    let packages: Vec<String> = r
        .packages
        .into_iter()
        .filter(|p| !shared.contains(p))
        .collect();
    if !packages.is_empty() {
        say(&format!(
            "Removing {} from {}",
            packages.join(", "),
            prefix.display()
        ));
        run(
            &mamba,
            &isolated("remove", &prefix, &root, &packages),
            &[("MAMBA_ROOT_PREFIX", root)],
            say,
        )?;
    }
    record.libraries.remove(library);
    save(&record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guide_s_packages_are_what_follows_conda_forge() {
        assert_eq!(packages_of("conda install -c conda-forge mkl"), ["mkl"]);
        assert_eq!(
            packages_of("conda install -c conda-forge cuda-cudart libcusparse"),
            ["cuda-cudart", "libcusparse"]
        );
        assert_eq!(
            packages_of("conda install -c conda-forge libcudss=1.0"),
            ["libcudss=1.0"]
        );
        assert!(packages_of("pip install mkl").is_empty());
    }

    #[test]
    fn micromamba_runs_isolated_on_conda_forge_alone() {
        let args = isolated(
            "create",
            Path::new("/data/libraries"),
            Path::new("/data/tools/mamba"),
            &["openblas".into()],
        );
        for must in [
            "--yes",
            "--no-rc",
            "--override-channels",
            "conda-forge",
            "openblas",
        ] {
            assert!(args.iter().any(|a| a == must), "{must} in {args:?}");
        }
        let prefix_at = args.iter().position(|a| a == "--prefix").unwrap();
        assert!(args[prefix_at + 1].ends_with("libraries"));
        assert!(
            !isolated("remove", Path::new("p"), Path::new("r"), &[])
                .iter()
                .any(|a| a == "--channel")
        );
    }

    /// One click, end to end, in a folder of its own: micromamba downloaded and checked, zstd
    /// installed from conda-forge, and the libraries' loader finding it in photonoxide's own
    /// environment. Needs the network, and `PHOTONOXIDE_LIBRARIES_DIR` set to an empty folder.
    #[test]
    #[ignore = "installs zstd from conda-forge; set PHOTONOXIDE_LIBRARIES_DIR to a scratch folder"]
    fn a_library_installed_here_is_found_in_photonoxide_s_own_environment() {
        let dir = std::env::var_os("PHOTONOXIDE_LIBRARIES_DIR")
            .expect("PHOTONOXIDE_LIBRARIES_DIR, a scratch folder: never the program's own");
        let mut lines = Vec::new();
        install("Zstandard", "conda", &["zstd".into()], &mut |l| {
            println!("{l}");
            lines.push(l.to_owned());
        })
        .unwrap();
        let found = photonoxide_native::discover(&photonoxide_native::zstd::ZSTD, None);
        let own = found
            .candidates
            .iter()
            .find(|c| c.source == photonoxide_native::Source::Own)
            .unwrap_or_else(|| panic!("zstd not found in {}: {found:?}", dir.display()));
        assert!(own.path.starts_with(&dir), "{}", own.path.display());
        assert_eq!(installed().libraries["Zstandard"].packages, ["zstd"]);
        remove("Zstandard", &mut |l| println!("{l}")).unwrap();
        assert!(!installed().libraries.contains_key("Zstandard"));
    }

    #[test]
    fn every_system_photonoxide_ships_for_has_a_pinned_micromamba() {
        for (os, arch) in [
            ("windows", "x86_64"),
            ("linux", "x86_64"),
            ("linux", "aarch64"),
            ("macos", "x86_64"),
            ("macos", "aarch64"),
        ] {
            let b = BUILDS.iter().find(|(o, a, _, _)| *o == os && *a == arch);
            assert!(b.is_some_and(|b| b.3.len() == 64), "{os} {arch}");
        }
    }
}
