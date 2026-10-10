//! Every validation case passes, and docs/validation.md is exactly the report the code writes.
//! Slow in a debug build: CI runs it in release (`cargo test --release --test validation_report
//! -- --ignored`). After a change, rewrite the report with `photonoxide validate --write
//! docs/validation.md`. With `PHOTONOXIDE_VALIDATION_OUT` set, it writes the report it made
//! there too, so that a run on another system can be compared (the Systems workflow).

#[test]
#[ignore = "runs every validation case: slow, run in release by the Validation report job"]
fn the_validation_report_passes_and_is_up_to_date() {
    let (report, passed) = photonoxide::validation::report();
    if let Some(out) = std::env::var_os("PHOTONOXIDE_VALIDATION_OUT") {
        std::fs::write(out, &report).expect("the report is written");
    }
    assert!(passed, "some validation cases failed:\n{report}");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/validation.md");
    let current = std::fs::read_to_string(path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        current == report,
        "docs/validation.md is out of date: run `photonoxide validate --write docs/validation.md`"
    );
}
