//! The stable channel for a copy that isn't installed (a build from the repository): Tauri's
//! updater replaces only an installed copy, so this one installs the latest release where a
//! first install goes, as a nightly does ([`super::install::first_install`]).
//!
//! It reads the release's update manifest (`latest.json`, the updater's own endpoint), takes this
//! system's file by the updater's names for it (`windows-x86_64-nsis`, then `windows-x86_64`, and
//! so on), downloads it, and checks it as the updater does: the minisign signature against the
//! release key in tauri.conf.json, and the version it was signed for against the one announced.
//! Only then is it installed, by the nightly's install ([`super::install::install`]): the setup
//! on Windows, the app bundle (unpacked from its `.app.tar.gz`) on macOS, the AppImage elsewhere.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use super::builder::{Dirs, Ready};
use super::github::{self, Http};

/// The latest release, for this system.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub date: String,
    /// The updater's name for this system's file, and the file.
    pub target: String,
    pub url: String,
    pub signature: String,
    /// Whether the stable channel takes it in place of this build (a later release; from a
    /// nightly, any other): otherwise it's offered only when asked.
    pub newer: bool,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    pub_date: String,
    platforms: std::collections::HashMap<String, Platform>,
}

#[derive(Deserialize)]
struct Platform {
    signature: String,
    url: String,
}

/// The updater's names for this system's file, the most particular first: `{os}-{arch}-{bundle}`,
/// then `{os}-{arch}`.
pub fn targets(os: &str, arch: &str) -> Vec<String> {
    let (os, bundle) = match os {
        "windows" => ("windows", "nsis"),
        "macos" => ("darwin", "app"),
        _ => ("linux", "appimage"),
    };
    vec![format!("{os}-{arch}-{bundle}"), format!("{os}-{arch}")]
}

/// The latest release in the manifest at `endpoint`, for this system, and whether it's later
/// than `current` (this build's version; `nightly`: this build is a nightly).
pub fn latest(
    http: &dyn Http,
    endpoint: &str,
    current: &str,
    nightly: bool,
) -> Result<Release, String> {
    let r = http.get(endpoint, &[])?;
    if r.status != 200 {
        return Err(format!(
            "the update manifest answered {} ({endpoint})",
            r.status
        ));
    }
    let m: Manifest = serde_json::from_slice(&r.body)
        .map_err(|e| format!("the update manifest doesn't read: {e}"))?;
    let (target, platform) = targets(std::env::consts::OS, std::env::consts::ARCH)
        .into_iter()
        .find_map(|t| m.platforms.get(&t).map(|p| (t, p)))
        .ok_or_else(|| {
            format!(
                "photonoxide {} has no file for {} on {}",
                m.version,
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })?;
    let release = semver::Version::parse(m.version.trim_start_matches('v'))
        .map_err(|e| format!("the manifest's version {} isn't one: {e}", m.version))?;
    let newer = semver::Version::parse(current)
        .map(|c| super::stable_accepts(nightly, &c, &release))
        .unwrap_or(true);
    Ok(Release {
        version: release.to_string(),
        notes: m.notes,
        date: m.pub_date,
        target,
        url: platform.url.clone(),
        signature: platform.signature.clone(),
        newer,
    })
}

/// Checks `data` as Tauri's updater does: the minisign signature (both base64, as the manifest
/// and tauri.conf.json carry them) against the release key, then the version it was signed for,
/// which only the checked signature makes trustworthy, against the one announced.
pub fn verify(data: &[u8], signature: &str, key: &str, announced: &str) -> Result<(), String> {
    let text = |b64: &str, what: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|e| format!("the {what} isn't base64: {e}"))?;
        String::from_utf8(bytes).map_err(|_| format!("the {what} isn't text"))
    };
    let key = minisign_verify::PublicKey::decode(&text(key, "release key")?)
        .map_err(|e| format!("the release key doesn't read: {e}"))?;
    let signature = minisign_verify::Signature::decode(&text(signature, "signature")?)
        .map_err(|e| format!("the signature doesn't read: {e}"))?;
    key.verify(data, &signature, true).map_err(|e| {
        format!("the download isn't signed by photonoxide's release key ({e}): it wasn't installed")
    })?;
    let signed = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"));
    match signed {
        Some(v) if v.trim_start_matches('v') != announced => Err(format!(
            "the download was signed as {v}, not {announced}: it wasn't installed"
        )),
        _ => Ok(()),
    }
}

/// Downloads `release` into `dirs`, checks it, and makes it ready to install: the setup, the
/// app bundle unpacked from its archive, or the AppImage. `commit` is the release's commit,
/// which the install checks the program it installs says it is.
pub fn fetch(
    http: &dyn Http,
    dirs: &Dirs,
    release: &Release,
    key: &str,
    commit: &str,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Ready, String> {
    let folder = dirs.root.join("releases").join(&release.version);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let name = release
        .url
        .rsplit('/')
        .next()
        .filter(|n| !n.is_empty())
        .unwrap_or("photonoxide-release");
    let file = folder.join(name);
    http.download(&release.url, &file, progress, &|| false)?;
    let data = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    verify(&data, &release.signature, key, &release.version)?;
    drop(data);
    let (bundle, artifact) = if release.target.starts_with("darwin") {
        ("app", unpack_app(&file, &folder.join("unpacked"))?)
    } else if release.target.starts_with("windows") {
        ("nsis", file)
    } else {
        ("appimage", file)
    };
    Ok(Ready {
        commit: commit.to_owned(),
        date: None,
        version: release.version.clone(),
        label: release.version.clone(),
        bundle: bundle.to_owned(),
        artifact,
        built_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        seconds: 0.0,
    })
}

/// The `.app` bundle inside a `.app.tar.gz`, unpacked into `into`.
fn unpack_app(archive: &Path, into: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("{}: {e}", archive.display()))?;
    let _ = std::fs::remove_dir_all(into);
    tar::Archive::new(flate2::read::GzDecoder::new(file))
        .unpack(into)
        .map_err(|e| format!("unpacking {}: {e}", archive.display()))?;
    std::fs::read_dir(into)
        .map_err(|e| format!("{}: {e}", into.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .ok_or_else(|| format!("{} holds no .app bundle", archive.display()))
}

/// The release's commit, by its tag (`v0.6.0`).
pub fn commit_of(http: &dyn Http, version: &str) -> Result<String, String> {
    github::tag_commit(http, &format!("v{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_system_s_file_is_taken_by_the_updater_s_names() {
        assert_eq!(
            targets("windows", "x86_64"),
            ["windows-x86_64-nsis", "windows-x86_64"]
        );
        assert_eq!(
            targets("macos", "aarch64"),
            ["darwin-aarch64-app", "darwin-aarch64"]
        );
        assert_eq!(
            targets("linux", "x86_64"),
            ["linux-x86_64-appimage", "linux-x86_64"]
        );
    }

    const ENDPOINT: &str = "https://example.test/latest.json";

    fn manifest(version: &str) -> Vec<u8> {
        let target = &targets(std::env::consts::OS, std::env::consts::ARCH)[0];
        serde_json::json!({
            "version": version,
            "notes": "the changes",
            "pub_date": "2026-10-28T12:00:00Z",
            "platforms": {
                target.as_str(): { "signature": "c2ln", "url": "https://example.test/photonoxide-setup.exe" },
                "some-other-system": { "signature": "x", "url": "https://example.test/other" }
            }
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn the_latest_release_is_this_system_s_file_and_later_only_when_it_is() {
        let http = github::tests::Canned::default();
        http.answer(ENDPOINT, 200, &[], &manifest("0.6.1"));
        let r = latest(&http, ENDPOINT, "0.6.0", false).unwrap();
        assert_eq!(r.version, "0.6.1");
        assert_eq!(r.url, "https://example.test/photonoxide-setup.exe");
        assert_eq!(
            r.target,
            targets(std::env::consts::OS, std::env::consts::ARCH)[0]
        );
        assert!(r.newer);
        // the same release isn't later; from a nightly, any release is taken
        assert!(!latest(&http, ENDPOINT, "0.6.1", false).unwrap().newer);
        assert!(
            latest(&http, ENDPOINT, "0.6.1+nightly.20261028.abcdef0", true)
                .unwrap()
                .newer
        );
        // no file for this system: said, not guessed
        let none = github::tests::Canned::default();
        none.answer(
            ENDPOINT,
            200,
            &[],
            br#"{"version":"0.6.1","platforms":{"some-other-system":{"signature":"x","url":"y"}}}"#,
        );
        assert!(
            latest(&none, ENDPOINT, "0.6.0", false)
                .unwrap_err()
                .contains("no file for")
        );
    }

    #[test]
    fn a_download_that_doesn_t_check_is_never_made_ready() {
        let http = github::tests::Canned::default();
        http.answer(ENDPOINT, 200, &[], &manifest("0.6.1"));
        http.answer(
            "https://example.test/photonoxide-setup.exe",
            200,
            &[],
            b"not the release",
        );
        let release = latest(&http, ENDPOINT, "0.6.0", false).unwrap();
        let root = std::env::temp_dir().join(format!("photonoxide-release-{}", std::process::id()));
        let dirs = Dirs { root: root.clone() };
        let key = base64::engine::general_purpose::STANDARD.encode(
            "untrusted comment: minisign public key 4840E1AF3997847E\n\
             RWR+hJc5r+FASLnEkCcdVVRuc2ghjLy/uJRdBt3TJO20alqMfTXo8amG\n",
        );
        let made = fetch(
            &http,
            &dirs,
            &release,
            &key,
            "0123456789abcdef0123456789abcdef01234567",
            &mut |_, _| {},
        );
        assert!(made.is_err(), "{made:?}");
        let _ = std::fs::remove_dir_all(root);
    }

    /// The latest real release for this system, downloaded and checked with tauri.conf.json's
    /// release key: the check takes what the release workflow signs. Needs the network.
    #[test]
    #[ignore = "downloads the latest release; cargo test -p photonoxide-studio release -- --ignored"]
    fn the_latest_real_release_checks_with_the_release_key() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let updater = &conf["plugins"]["updater"];
        let endpoint = updater["endpoints"][0].as_str().unwrap();
        let key = updater["pubkey"].as_str().unwrap();
        let web = github::Web {
            user_agent: "photonoxide-test".into(),
        };
        let release = latest(&web, endpoint, "0.0.1", false).unwrap();
        let root = std::env::temp_dir().join(format!("photonoxide-real-{}", std::process::id()));
        let commit = commit_of(&web, &release.version).unwrap();
        let ready = fetch(
            &web,
            &Dirs { root: root.clone() },
            &release,
            key,
            &commit,
            &mut |_, _| {},
        )
        .unwrap();
        assert!(ready.artifact.exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_download_not_signed_by_the_key_is_refused() {
        // a well-formed key and signature from another key: refused, and nothing is installed
        let key = base64::engine::general_purpose::STANDARD.encode(
            "untrusted comment: minisign public key 4840E1AF3997847E\n\
             RWR+hJc5r+FASLnEkCcdVVRuc2ghjLy/uJRdBt3TJO20alqMfTXo8amG\n",
        );
        assert!(verify(b"data", "bm90IGEgc2lnbmF0dXJl", &key, "0.6.1").is_err());
        assert!(
            verify(b"data", "!", &key, "0.6.1")
                .unwrap_err()
                .contains("base64")
        );
    }
}
