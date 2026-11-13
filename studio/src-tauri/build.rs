use std::path::Path;
use std::process::Command;

#[path = "src/papers/ours.rs"]
mod ours;

fn main() {
    build_info();
    references_index();
    // the window's build: its config, capabilities and icons (not in a build without it)
    #[cfg(feature = "window")]
    tauri_build::build();
}

/// The DOIs photonoxide cites and where (src/papers/ours.rs), from docs/references.md, for a
/// paper's page; empty when the index isn't there (a source tree without docs/).
fn references_index() {
    let here = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let file = Path::new(&here).join("../../docs/references.md");
    println!("cargo:rerun-if-changed={}", file.display());
    let index = std::fs::read_to_string(&file)
        .map(|md| ours::index(&md))
        .unwrap_or_default();
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    std::fs::write(Path::new(&out).join("ours.tsv"), index).expect("OUT_DIR is writable");
}

/// What the program is built from, for Settings › About and the nightly channel
/// (`src/channels.rs`): the commit, its date, whether the tree had changes not committed, and
/// for a nightly, its channel and version. The nightly builder sets them in the environment (it
/// builds from a tarball, which has no git); otherwise they come from git, when there is a
/// checkout and git is found. Each is set, empty when unknown.
fn build_info() {
    const SET: [&str; 5] = [
        "PHOTONOXIDE_COMMIT",
        "PHOTONOXIDE_COMMIT_DATE",
        "PHOTONOXIDE_DIRTY",
        "PHOTONOXIDE_CHANNEL",
        "PHOTONOXIDE_NIGHTLY_VERSION",
    ];
    for v in SET {
        println!("cargo:rerun-if-env-changed={v}");
    }
    let given = |v: &str| std::env::var(v).ok().filter(|s| !s.is_empty());
    let here = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&here)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };
    let (commit, date, dirty) = match given("PHOTONOXIDE_COMMIT") {
        Some(commit) => (
            Some(commit),
            given("PHOTONOXIDE_COMMIT_DATE"),
            given("PHOTONOXIDE_DIRTY").is_some_and(|d| d == "true"),
        ),
        None => {
            // built again when the checkout moves to another commit or its index changes
            for p in ["HEAD", "index", "logs/HEAD"] {
                if let Some(path) = git(&["rev-parse", "--path-format=absolute", "--git-path", p])
                    && Path::new(&path).exists()
                {
                    println!("cargo:rerun-if-changed={path}");
                }
            }
            let commit = git(&["rev-parse", "HEAD"]);
            let date = commit
                .as_ref()
                .and_then(|_| git(&["log", "-1", "--format=%cs"]));
            // without optional locks: a plain `git status` may refresh and rewrite the index,
            // which is newer than this run, so Cargo ran the script and built the program again
            // on the next build with nothing changed
            let dirty = commit.is_some()
                && git(&[
                    "--no-optional-locks",
                    "status",
                    "--porcelain",
                    "--untracked-files=no",
                ])
                .is_some_and(|s| !s.is_empty());
            (commit, date, dirty)
        }
    };
    let pairs = [
        ("PHOTONOXIDE_COMMIT", commit.unwrap_or_default()),
        ("PHOTONOXIDE_COMMIT_DATE", date.unwrap_or_default()),
        ("PHOTONOXIDE_DIRTY", dirty.to_string()),
        (
            "PHOTONOXIDE_CHANNEL",
            given("PHOTONOXIDE_CHANNEL").unwrap_or_default(),
        ),
        (
            "PHOTONOXIDE_NIGHTLY_VERSION",
            given("PHOTONOXIDE_NIGHTLY_VERSION").unwrap_or_default(),
        ),
    ];
    for (name, value) in pairs {
        println!("cargo:rustc-env={name}={value}");
    }
}
