//! The setup's payload and its Windows resources.
//!
//! `PHOTONOXIDE_SETUP_PAYLOAD` names the program to carry: the `photonoxide.exe` Tauri built
//! (`pnpm tauri build --bundles nsis`, which marks it as installed by a setup, for the updater,
//! and signs it). It is compressed here by 7-Zip's BCJ2 and xz at its slowest setting. BCJ2
//! takes the targets of the x86 calls and jumps out of the code into streams of their own, as
//! absolute addresses, so the same target reads the same wherever it's called from and the code
//! around it repeats more; each stream is then compressed alone. On the program (52.2 MiB), 10.9
//! MiB, against 11.6 for xz behind the plain x86 filter and 12.8 for NSIS's whole setup; both
//! unpack in half a second. Without the variable the payload is empty, and the setup says it was
//! built without one (CI's lints build it so).

use std::io::Write;
use std::path::PathBuf;

use lzma_rust2::filter::bcj2::{Bcj2Options, Bcj2Writer};
use lzma_rust2::{EncodeMode, LzmaOptions, MfType, XzOptions, XzWriter};

/// Asks for no administrator (a per-user install needs none, and Windows would otherwise ask
/// for one, because the name has "setup" in it), and draws at the screen's own resolution.
const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application><supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/></application>
  </compatibility>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-env-changed=PHOTONOXIDE_SETUP_PAYLOAD");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let payload = match std::env::var_os("PHOTONOXIDE_SETUP_PAYLOAD").filter(|p| !p.is_empty()) {
        Some(path) => {
            // a relative path is the repository's, where the release workflow runs
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            let path = root.join(path);
            println!("cargo:rerun-if-changed={}", path.display());
            let program = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("PHOTONOXIDE_SETUP_PAYLOAD, {}: {e}", path.display()));
            compress(&program)
        }
        None => Vec::new(),
    };
    std::fs::write(out.join("photonoxide.exe.bcj2"), payload).expect("a writable OUT_DIR");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = tauri_winres::WindowsResource::new();
        res.set_icon("../src-tauri/icons/icon.ico")
            .set_manifest(MANIFEST)
            .set("ProductName", "photonoxide")
            .set("FileDescription", "photonoxide setup")
            .set("CompanyName", "tachsin")
            .set("LegalCopyright", "MIT OR Apache-2.0");
        res.compile().expect("the setup's icon and manifest");
    }
}

/// The payload: BCJ2's four streams, the first three each as xz, the range coder's as it is,
/// after a header of the program's length (8 bytes) and the four streams' (4 bytes each), all
/// little-endian. main.rs reads it back.
fn compress(program: &[u8]) -> Vec<u8> {
    let mut bcj2 = Bcj2Writer::new(
        [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        &Bcj2Options {
            uncompressed_size: Some(program.len() as u64),
            ..Bcj2Options::default()
        },
    )
    .expect("BCJ2's options");
    bcj2.write_all(program).expect("split in memory");
    let [code, calls, jumps, ranges] = bcj2.finish().expect("split in memory");
    // the code with xz's defaults for it; the targets are 4-byte numbers: no literal context, by
    // their position in the 4 bytes
    let streams = [
        xz(&code, LzmaOptions::LC_DEFAULT, LzmaOptions::LP_DEFAULT),
        xz(&calls, 0, 2),
        xz(&jumps, 0, 2),
        ranges,
    ];
    let mut out = (program.len() as u64).to_le_bytes().to_vec();
    for s in &streams {
        let n = u32::try_from(s.len()).expect("a stream under 4 GiB");
        out.extend_from_slice(&n.to_le_bytes());
    }
    for s in streams {
        out.extend_from_slice(&s);
    }
    out
}

/// xz -9e: a 64 MiB dictionary, the longest matches, the deepest search.
fn xz(data: &[u8], lc: u32, lp: u32) -> Vec<u8> {
    let mut options = XzOptions::with_preset(9);
    options.lzma_options = LzmaOptions::new(
        64 << 20,
        lc,
        lp,
        LzmaOptions::PB_DEFAULT,
        EncodeMode::Normal,
        LzmaOptions::NICE_LEN_MAX,
        MfType::Bt4,
        512,
    );
    let mut writer = XzWriter::new(Vec::new(), options).expect("the xz options");
    writer.write_all(data).expect("compressed in memory");
    writer.finish().expect("compressed in memory")
}
