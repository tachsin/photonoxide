//! The installers' files (studio/src-tauri/installer) against what reads them: every file
//! tauri.conf.json names exists, the licence page carries both licences word for word, the
//! pictures are the uncompressed 24-bit bitmaps NSIS's pages take at the sizes they're shown at,
//! and the hooks file is plain ASCII, which NSIS reads the same in every code page.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).to_path_buf()
}

fn studio(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

/// A text file without its carriage returns, as git may check it out with or without them.
fn text(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace('\r', "")
}

#[test]
fn every_installer_file_the_configuration_names_exists() {
    let config: serde_json::Value =
        serde_json::from_str(&text(&studio("tauri.conf.json"))).expect("tauri.conf.json is JSON");
    let bundle = &config["bundle"];
    let nsis = &bundle["windows"]["nsis"];
    let named = [
        &bundle["licenseFile"],
        &nsis["installerIcon"],
        &nsis["headerImage"],
        &nsis["uninstallerHeaderImage"],
        &nsis["sidebarImage"],
        &nsis["installerHooks"],
        &bundle["macOS"]["dmg"]["background"],
    ];
    for path in named {
        let path = path.as_str().expect("a path in tauri.conf.json");
        assert!(studio(path).is_file(), "{path} doesn't exist");
    }
}

#[test]
fn the_licence_page_carries_both_licences() {
    let page = text(&studio("installer/license.txt"));
    for licence in ["LICENSE-MIT", "LICENSE-APACHE"] {
        assert!(
            page.contains(text(&root().join(licence)).trim()),
            "installer/license.txt doesn't carry {licence} as it is"
        );
    }
}

#[test]
fn the_pictures_are_bitmaps_at_their_pages_sizes() {
    // the welcome and finish pages' sidebar, and the other pages' header
    for (name, width, height) in [("sidebar.bmp", 164, 314), ("header.bmp", 150, 57)] {
        let b = std::fs::read(studio(&format!("installer/{name}"))).expect("the picture");
        let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        assert_eq!(&b[..2], b"BM", "{name} isn't a bitmap");
        assert_eq!((u32_at(18), u32_at(22)), (width, height), "{name}'s size");
        assert_eq!(
            (u16_at(28), u32_at(30)),
            (24, 0),
            "{name}: 24 bits, uncompressed"
        );
        assert_eq!(u32_at(2) as usize, b.len(), "{name}'s length");
    }
}

#[test]
fn the_hooks_file_is_ascii() {
    let hooks = std::fs::read(studio("installer/hooks.nsh")).expect("the hooks file");
    assert!(
        hooks.is_ascii(),
        "installer/hooks.nsh has a character beyond ASCII"
    );
}
