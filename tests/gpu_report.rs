//! The GPU's validation report, docs/validation-gpu.md: written on a machine with a GPU before
//! each release (GitHub's runners have none, so CI doesn't run it), in release:
//!
//! ```sh
//! cargo test --release --features gpu --test gpu_report -- --ignored --nocapture
//! ```
//!
//! It skips, saying so, where there is no GPU.

#![cfg(feature = "gpu")]

#[test]
#[ignore = "runs every FDTD case on the GPU: slow, on a machine with a GPU, in release"]
fn the_gpu_report_passes() {
    let (report, passed) = match photonoxide::validation::gpu_report() {
        Ok(r) => r,
        Err(e) => {
            println!("skipped: {e}");
            return;
        }
    };
    print!("{report}");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/validation-gpu.md");
    std::fs::write(path, &report).expect("docs/validation-gpu.md");
    assert!(
        passed,
        "some of the GPU's cases failed: see docs/validation-gpu.md"
    );
}
