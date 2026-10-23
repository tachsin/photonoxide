//! Zstandard's exported files against photonoxide's own, on this machine: a modes job's `.npz`
//! with `compression = "zstd"`, each member read back by libzstd and compared, byte for byte,
//! with the same job's stored `.npz` (photonoxide's own path, no compression), and the same
//! frames from a second run. Skipped, and saying so, where libzstd isn't found; with
//! `PHOTONOXIDE_REQUIRE_ZSTD` set (the Libraries workflow, after it installs libzstd) not
//! finding it is a failure.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use photonoxide::export::codec::{self, Compressor};
use photonoxide::run::{Job, Run, Stop};
use photonoxide_native::{Zstd, zstd};

fn zstd() -> Option<Arc<Zstd>> {
    match Zstd::load() {
        Ok(z) => Some(Arc::new(z)),
        Err(reason) => {
            assert!(
                std::env::var_os("PHOTONOXIDE_REQUIRE_ZSTD").is_none(),
                "libzstd is required here and isn't found: {reason}"
            );
            println!("skipped: libzstd isn't here: {reason}");
            None
        }
    }
}

/// A strip's one mode, its fields exported with `extra`.
fn job(extra: &str) -> Job {
    Job::parse(&format!(
        "name = \"strip\"\n[task]\nkind = \"modes\"\nstack = \"soi_220\"\nwavelength_um = 1.55\n\
         layer = \"Si\"\npropagation = \"x\"\ny_um = [-1.0, 1.0]\nstep_nm = 50.0\nmodes = 1\n\
         [[task.rect]]\nlayer = \"Si\"\ncenter_um = [0.0, 0.0]\nsize_um = [10.0, 0.5]\n\
         [task.export]\nformat = \"npz\"\n{extra}\n"
    ))
    .unwrap()
}

/// Runs `job` under `root` and returns its mode's `.npz`.
fn run(root: &Path, job: &Job) -> PathBuf {
    let mut run = Run::create(root, job).unwrap();
    photonoxide::job::execute(job, &mut run, &Stop::new(None)).unwrap();
    run.dir().join("mode-1-of-1.npz")
}

/// Each member's name, ZIP method and payload, from its local header on.
fn members(path: &Path) -> Vec<(String, u16, Vec<u8>)> {
    let bytes = std::fs::read(path).unwrap();
    let u16_at = |k: usize| u16::from_le_bytes([bytes[k], bytes[k + 1]]);
    let u32_at = |k: usize| u32::from_le_bytes(bytes[k..k + 4].try_into().unwrap());
    let mut out = Vec::new();
    let mut k = 0;
    while u32_at(k) == 0x0403_4b50 {
        let (method, size) = (u16_at(k + 8), u32_at(k + 18) as usize);
        let (name_len, extra) = (u16_at(k + 26) as usize, u16_at(k + 28) as usize);
        let name = String::from_utf8(bytes[k + 30..k + 30 + name_len].to_vec()).unwrap();
        let start = k + 30 + name_len + extra;
        out.push((name, method, bytes[start..start + size].to_vec()));
        k = start + size;
    }
    out
}

#[test]
fn it_finds_libzstd_and_its_release() {
    let probe = zstd::probe();
    println!("{probe}");
    let Some(z) = zstd() else { return };
    assert_eq!(probe.version, Some(z.version()));
    assert!(z.deterministic());
    assert_eq!((z.levels().0, z.zip_method()), (1, 93));
    let data = b"photonoxide photonoxide photonoxide".repeat(40);
    let frame = z.compress(&data, 19).unwrap();
    assert!(frame.len() < data.len() / 4);
    assert_eq!(z.decompress(&frame).unwrap(), data);
    assert!(z.compress(&data, 99).is_err());
}

#[test]
fn a_zstd_npz_reads_back_to_the_bytes_of_photonoxides_own() {
    let Some(z) = zstd() else { return };
    codec::register(z.clone());
    let root = std::env::temp_dir().join(format!("photonoxide-zstd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let stored = members(&run(&root, &job("level = 0")));
    let zstd_job = job("compression = \"zstd\"\nlevel = 9");
    photonoxide::job::check(&zstd_job).unwrap();
    let first = members(&run(&root, &zstd_job));
    let second = members(&run(&root, &zstd_job));
    assert_eq!(stored.len(), first.len());
    for ((name, method, data), (zname, zmethod, frame)) in stored.iter().zip(&first) {
        assert_eq!((name, *method, *zmethod), (zname, 0, 93));
        let back = z.decompress(frame).unwrap();
        if name == "meta.json" {
            // the same but for the compression it names
            let text = String::from_utf8(back).unwrap();
            assert!(
                text.contains(&format!("zstd {}, level 9", z.version())),
                "{text}"
            );
        } else {
            assert_eq!(&back, data, "{name}");
        }
    }
    // the same frames on a second run (but meta.json's, which names its own run): libzstd's
    // single-threaded compression is deterministic
    let arrays = |m: &[(String, u16, Vec<u8>)]| -> Vec<(String, u16, Vec<u8>)> {
        m.iter()
            .filter(|(n, ..)| n != "meta.json")
            .cloned()
            .collect()
    };
    assert_eq!(arrays(&first), arrays(&second));
    let _ = std::fs::remove_dir_all(&root);
}
