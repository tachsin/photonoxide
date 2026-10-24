//! HDF5's exported files against photonoxide's own, on this machine: a modes job's fields
//! written as an `.h5` (`container = "hdf5"`) and as a stored `.npz` (photonoxide's own path),
//! every array read back through libhdf5 and compared byte for byte, with its shape; and the
//! same arrays giving the same file twice. Skipped, and saying so, where libhdf5 isn't found;
//! with `PHOTONOXIDE_REQUIRE_HDF5` set (the Libraries workflow, after it installs libhdf5) not
//! finding it is a failure.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::export::codec::{self, Container};
use photonoxide::export::{Described, Meta, Npz, Precision};
use photonoxide::run::{Job, Run, Stop};
use photonoxide_native::{Hdf5, hdf5};

fn hdf5() -> Option<Arc<Hdf5>> {
    match Hdf5::load() {
        Ok(h) => Some(Arc::new(h)),
        Err(reason) => {
            assert!(
                std::env::var_os("PHOTONOXIDE_REQUIRE_HDF5").is_none(),
                "libhdf5 is required here and isn't found: {reason}"
            );
            println!("skipped: libhdf5 isn't here: {reason}");
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
         [task.export]\nformat = \"npz\"\nprecision = \"f32\"\n{extra}\n"
    ))
    .unwrap()
}

/// Runs `job` under `root` and returns its directory.
fn run(root: &Path, job: &Job) -> PathBuf {
    let mut run = Run::create(root, job).unwrap();
    photonoxide::job::execute(job, &mut run, &Stop::new(None)).unwrap();
    run.dir().to_path_buf()
}

/// Each `.npy` member of a stored `.npz`: its name (`.npy` left off), `descr`, shape and data.
fn arrays(path: &Path) -> Vec<(String, String, Vec<usize>, Vec<u8>)> {
    let bytes = std::fs::read(path).unwrap();
    let u16_at = |k: usize| u16::from_le_bytes([bytes[k], bytes[k + 1]]);
    let u32_at = |k: usize| u32::from_le_bytes(bytes[k..k + 4].try_into().unwrap());
    let mut out = Vec::new();
    let mut k = 0;
    while u32_at(k) == 0x0403_4b50 {
        let size = u32_at(k + 18) as usize;
        let (name_len, extra) = (u16_at(k + 26) as usize, u16_at(k + 28) as usize);
        let name = String::from_utf8(bytes[k + 30..k + 30 + name_len].to_vec()).unwrap();
        let start = k + 30 + name_len + extra;
        let member = &bytes[start..start + size];
        k = start + size;
        let Some(name) = name.strip_suffix(".npy") else {
            continue;
        };
        let len = 10 + usize::from(u16::from_le_bytes([member[8], member[9]]));
        let header = std::str::from_utf8(&member[10..len]).unwrap();
        let descr = header.split("'descr': '").nth(1).unwrap();
        let descr = descr.split('\'').next().unwrap().to_owned();
        let shape = header.split("'shape': (").nth(1).unwrap();
        let shape = shape.split(')').next().unwrap();
        let shape = shape
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        out.push((name.to_owned(), descr, shape, member[len..].to_vec()));
    }
    out
}

#[test]
fn it_finds_libhdf5_and_its_release() {
    let probe = hdf5::probe();
    println!("{probe}");
    let Some(h) = hdf5() else { return };
    assert_eq!(probe.version, Some(h.version()));
    assert!(h.deterministic());
    assert_eq!((h.name(), h.extension()), ("hdf5", "h5"));
}

#[test]
fn an_hdf5_file_reads_back_to_the_arrays_of_photonoxides_own() {
    let Some(h) = hdf5() else { return };
    codec::register_container(h.clone());
    let root = std::env::temp_dir().join(format!("photonoxide-hdf5-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let stored = run(&root, &job("level = 0")).join("mode-1-of-1.npz");
    let hdf5_job = job("container = \"hdf5\"\nlevel = 4");
    photonoxide::job::check(&hdf5_job).unwrap();
    let file = run(&root, &hdf5_job).join("mode-1-of-1.h5");
    let arrays = arrays(&stored);
    assert!(arrays.len() >= 7, "{}", arrays.len());
    for (name, descr, shape, data) in &arrays {
        let (precision, complex) = match descr.as_str() {
            "<f4" => (Precision::F32, false),
            "<f8" => (Precision::F64, false),
            "<c8" => (Precision::F32, true),
            "<c16" => (Precision::F64, true),
            other => panic!("{name}: {other}"),
        };
        let (read_shape, read) = h.read(&file, name, precision, complex).unwrap();
        assert_eq!(&read_shape, shape, "{name}");
        assert_eq!(&read, data, "{name}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_same_arrays_give_the_same_file() {
    let Some(h) = hdf5() else { return };
    let dir = std::env::temp_dir().join(format!("photonoxide-hdf5-same-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str| -> Vec<u8> {
        let mut npz = Npz::contained(Precision::F64, h.clone(), 6);
        npz.exact("x_um", &[3], &[0.0, 0.5, 1.0]);
        let field: Vec<c64> = (0..6).map(|k| c64::new(f64::from(k), -0.5)).collect();
        npz.complex_field("Ez", &[2, 3], &field);
        npz.meta(
            &Meta::new("same", "run"),
            &Described {
                grid: "x_um",
                units: "um",
                normalization: "none",
            },
        );
        let path = dir.join(name);
        npz.write(&path).unwrap();
        std::fs::read(path.with_extension("h5")).unwrap()
    };
    // a second apart: a file that kept its objects' times would differ
    let a = write("a.npz");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let b = write("b.npz");
    assert_eq!(a, b, "the same arrays gave different files");
    let (shape, data) = h
        .read(&dir.join("a.h5"), "Ez", Precision::F64, true)
        .unwrap();
    assert_eq!(shape, [2, 3]);
    assert_eq!(f64::from_le_bytes(data[16..24].try_into().unwrap()), 1.0);
    assert_eq!(f64::from_le_bytes(data[24..32].try_into().unwrap()), -0.5);
    let _ = std::fs::remove_dir_all(&dir);
}
