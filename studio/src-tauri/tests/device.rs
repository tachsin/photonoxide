//! `photonoxide run` on a job that computes a device, as a process: it runs headless, records
//! the device's model and its S-parameters, and exits.

use std::path::PathBuf;
use std::process::Command;

const PROGRAM: &str = env!("CARGO_BIN_EXE_photonoxide");

/// A 1 × 2 MMI by its 2D model (guided modes, labelled 2D) over five wavelengths.
const JOB: &str = r#"
name = "mmi-2d"
timeout_minutes = 5.0

[device]
kind = "MMI 1x2"
fidelity = "2D"

[task]
kind = "spectrum"
from_um = 1.53
to_um = 1.57
points = 5
"#;

#[test]
fn a_device_job_runs_from_the_command_line_and_records_its_model() {
    let dir =
        std::env::temp_dir().join(format!("photonoxide-device-program-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let job = dir.join("mmi-2d.toml");
    std::fs::write(&job, JOB).unwrap();
    let output = Command::new(PROGRAM)
        .arg("run")
        .arg(&job)
        .args(["--headless", "--out"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = PathBuf::from(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap()
            .trim(),
    );
    let record = std::fs::read_to_string(run.join("events.jsonl")).unwrap();
    let types: Vec<String> = record
        .lines()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["type"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(
        types,
        [
            "started",
            "scene",
            "ports",
            "device_model",
            "s_parameters",
            "s_parameters",
            "s_parameters",
            "s_parameters",
            "s_parameters",
            "finished"
        ]
    );
    assert!(record.contains("\"fidelity\":\"2D\""), "{record}");
    let _ = std::fs::remove_dir_all(&dir);
}
