//! A job's text read in the format it is written in: the studio's editor and the built-in jobs
//! read it so, and the headless program's examples and tests too.

use photonoxide::run::{Format, Job};

/// The format a job's text is written in, by its first line that isn't a comment: TOML for a
/// table's header or a `key = value`, JSON for an object, YAML otherwise. The built-in jobs are
/// YAML (#430) and the studio's own forms write TOML, and both reach the same commands.
pub fn job_format(text: &str) -> Format {
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'));
    let toml_key = |l: &str| {
        l.split_once('=').is_some_and(|(k, _)| {
            let k = k.trim();
            !k.is_empty()
                && k.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-.\"' ".contains(c))
        })
    };
    match first {
        Some(l) if l.starts_with('{') => Format::Json,
        Some(l) if l.starts_with('[') || toml_key(l) => Format::Toml,
        _ => Format::Yaml,
    }
}

/// A job from its text, in the format it is written in ([`job_format`]).
pub fn parse_job(text: &str) -> photonoxide::Result<Job> {
    Job::parse_as(text, job_format(text))
}
