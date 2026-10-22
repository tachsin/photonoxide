//! The frames a job's pictures are in (#435). The chip's, seen from above: x and y in the
//! plane, z up. A cross-section's: the axis across the guide and z up, the light along the
//! third. Which chip axis is across depends on the job:
//!
//! - a modes job with `propagation = "x"` is cut at `cut_x_um`, across y;
//! - one with `"y"`, or none (an older job), is cut at `cut_y_um`, across x;
//! - a static job is always cut at `cut_x_um`, across y.
//!
//! The design view's frame switch relabels its axes by this; no number in the job changes.

use photonoxide::run::Job;
use serde::Serialize;

/// A job's cross-section, in the chip's frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    /// The job's kind, "modes" or "static": a modes job's scene ends at its cut, a static
    /// job's reaches as far on either side.
    pub kind: &'static str,
    /// The chip's axis the light travels along, normal to the cut: "x" or "y".
    pub along: &'static str,
    /// The chip's axis across the guide, the cross-section's horizontal one: "x" or "y".
    pub across: &'static str,
    /// What the chip's x, y and z are called in the cross-section's frame.
    pub names: [&'static str; 3],
}

impl Section {
    fn along(kind: &'static str, along: &'static str) -> Section {
        let across = if along == "x" { "y" } else { "x" };
        let name = |axis: &str| if axis == along { "along" } else { "across" };
        Section {
            kind,
            along,
            across,
            names: [name("x"), name("y"), "z"],
        }
    }
}

/// The cross-section `job` is cut on: a modes or static job's; none for the others, which are
/// in the chip's frame only.
pub fn section(job: &Job) -> Option<Section> {
    if job.device().is_some() {
        return None;
    }
    let task = job.task();
    match task.get("kind").and_then(|k| k.as_str())? {
        "modes" => Some(Section::along(
            "modes",
            match task.get("propagation").and_then(|p| p.as_str()) {
                Some("x") => "x",
                _ => "y",
            },
        )),
        "static" => Some(Section::along("static", "x")),
        _ => None,
    }
}

/// The cross-section of the job in `text`, for the design view's frame switch: none for a job
/// that isn't cut.
#[tauri::command]
pub fn job_section(text: String) -> Result<Option<Section>, String> {
    let job = Job::parse(&text).map_err(|e| e.to_string())?;
    Ok(section(&job))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(task: &str) -> Option<Section> {
        section(&Job::parse(&format!("name = \"frames\"\n\n[task]\n{task}")).unwrap())
    }

    #[test]
    fn a_modes_job_along_x_is_across_y() {
        let s = of("kind = \"modes\"\npropagation = \"x\"\ncut_x_um = 1.0").unwrap();
        assert_eq!((s.along, s.across), ("x", "y"));
        assert_eq!(s.names, ["along", "across", "z"]);
    }

    #[test]
    fn a_modes_job_along_y_or_an_older_one_is_across_x() {
        for task in ["kind = \"modes\"\npropagation = \"y\"", "kind = \"modes\""] {
            let s = of(task).unwrap();
            assert_eq!((s.along, s.across), ("y", "x"), "{task}");
            assert_eq!(s.names, ["across", "along", "z"]);
        }
    }

    #[test]
    fn a_static_job_is_across_y_and_the_others_are_not_cut() {
        let s = of("kind = \"static\"").unwrap();
        assert_eq!((s.kind, s.along, s.across), ("static", "x", "y"));
        assert_eq!(s.names, ["along", "across", "z"]);
        for kind in ["structure", "fdfd", "fdtd"] {
            assert_eq!(of(&format!("kind = \"{kind}\"")), None, "{kind}");
        }
    }

    #[test]
    fn the_built_in_modes_jobs_name_their_propagation() {
        let text = include_str!("../../../jobs/strip-modes.toml");
        let s = job_section(text.to_owned()).unwrap().unwrap();
        assert_eq!(s.across, "y");
    }
}
