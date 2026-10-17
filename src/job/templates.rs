//! A short job of each kind, in YAML, to start a new one from: `photonoxide new <name>` writes
//! one (the job editor of #430 will start from them too). Each says what it computes, gives the
//! fields its kind needs and a few it may take, with a comment on what they mean, and runs as it
//! is, in seconds: a test checks each with [`super::check`], in YAML, TOML and JSON alike, and
//! runs it. The texts are the files in `src/job/templates/`.

/// A job to start from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Template {
    /// What `photonoxide new` takes: `"modes"`, `"fdfd"`, `"device"`.
    pub name: &'static str,
    /// What the job computes, in a line.
    pub about: &'static str,
    /// The job, in YAML with its comments.
    pub text: &'static str,
}

/// Every template, in the order `photonoxide new --list` gives them: the solvers' kinds, then a
/// device's and a chip's.
pub const TEMPLATES: [Template; 9] = [
    Template {
        name: "structure",
        about: "a design drawn and nothing solved: a ring beside its bus, as pictures",
        text: include_str!("templates/structure.yaml"),
    },
    Template {
        name: "modes",
        about: "a strip's guided modes by the full-vector mode solver",
        text: include_str!("templates/modes.yaml"),
    },
    Template {
        name: "fdfd",
        about: "a straight guide between two ports by 2D FDFD: S-parameters and its field",
        text: include_str!("templates/fdfd.yaml"),
    },
    Template {
        name: "fdtd",
        about: "a pulse through a straight guide by 2D FDTD: its transmission over the band",
        text: include_str!("templates/fdtd.yaml"),
    },
    Template {
        name: "static",
        about: "a heater over a strip by the static solver: the temperature on the cross-section",
        text: include_str!("templates/static.yaml"),
    },
    Template {
        name: "device",
        about: "a directional coupler of the device library: its S-parameters at a fidelity",
        text: include_str!("templates/device.yaml"),
    },
    Template {
        name: "sweep",
        about: "a device's study: a directional coupler's gap swept, with the band at each",
        text: include_str!("templates/sweep.yaml"),
    },
    Template {
        name: "optimize",
        about: "a device optimized: a directional coupler's length to an even split",
        text: include_str!("templates/optimize.yaml"),
    },
    Template {
        name: "chip",
        about: "a chip of instances: an all-pass ring of a coupler and a strip",
        text: include_str!("templates/chip.yaml"),
    },
];

/// The template called `name`, if there is one.
pub fn template(name: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{Event, check, execute};
    use crate::run::{Format, Job, Run, Stop};

    fn parse(t: &Template) -> Job {
        Job::parse_as(t.text, Format::Yaml).unwrap_or_else(|e| panic!("{}: {e}", t.name))
    }

    #[test]
    fn every_template_is_named_for_itself_and_checks() {
        for t in &TEMPLATES {
            let job = parse(t);
            assert_eq!(job.name(), format!("new-{}", t.name));
            assert!(
                job.timeout().is_some(),
                "{}: a template sets a timeout",
                t.name
            );
            check(&job).unwrap_or_else(|e| panic!("{}: {e}", t.name));
            assert_eq!(template(t.name), Some(t));
            // the same job in TOML and JSON
            for format in Format::ALL {
                let again = Job::parse_as(&job.to_text(format).unwrap(), format).unwrap();
                assert_eq!(again.task(), job.task(), "{} as {format:?}", t.name);
                check(&again).unwrap_or_else(|e| panic!("{} as {format:?}: {e}", t.name));
            }
        }
        assert_eq!(template("nothing"), None);
    }

    /// Every template but `optimize`, whose optimizer is genoxide's and runs in the program
    /// (the studio's tests run it).
    #[test]
    fn every_template_runs_to_its_end() {
        for t in TEMPLATES.iter().filter(|t| t.name != "optimize") {
            let job = parse(t);
            let mut run = Run::in_memory();
            execute(&job, &mut run, &Stop::new(job.timeout()))
                .unwrap_or_else(|e| panic!("{}: {e}", t.name));
            let last: Option<Event> = run.lines().last().map(|l| serde_json::from_str(l).unwrap());
            assert!(
                matches!(last, Some(Event::Finished { stopped: None, .. })),
                "{}: {last:?}",
                t.name
            );
        }
    }
}
