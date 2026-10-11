//! When the Nightly builder in this copy is too old for main: main's source moved on (its
//! manifest's format, say) in a way only a newer builder reads, so the build stops in the
//! builder's own steps, not in main's code.
//!
//! The breaks known so far are listed in `studio/src-tauri/builder-breaks.json`: what a too-old
//! builder says, what changed, and the pull request, commit and release that fixed it. A build
//! that fails reads the list compiled into this copy and the one in the source it just
//! downloaded, so main can tell an older builder about a break found after that builder was
//! made. When no listed break matches but the build stopped while reading main's source, the
//! message says the builder may be behind. Either way the way out is the same: the latest
//! release, through the Stable channel, then back to Nightly.

use serde::{Deserialize, Serialize};

use super::github::{REPO, short};

/// The breaks this copy knows of.
const OWN: &str = include_str!("../../builder-breaks.json");

/// A known break between main and older builders.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Break {
    /// What a too-old builder says when it hits it, as it appears in its error or its log.
    pub symptom: String,
    /// What changed, in words.
    pub what: String,
    /// The pull request that fixed the builder, its commit on main, and the first release with it.
    pub pr: Option<u32>,
    pub fixed_in: Option<String>,
    pub release: Option<String>,
}

/// A build stopped because the builder is too old for main.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TooOld {
    /// What the window says.
    pub message: String,
    /// The known break, when it is one.
    pub known: Option<Break>,
    /// The release that has the fix, when one is out and newer than this copy: the way out.
    pub release: Option<String>,
}

/// The breaks listed in `text`, or none when it doesn't read.
pub fn breaks(text: &str) -> Vec<Break> {
    serde_json::from_str(text).unwrap_or_default()
}

/// This copy's list and, when given, the downloaded source's (its `builder-breaks.json`).
pub fn known(source: Option<&str>) -> Vec<Break> {
    let mut all = breaks(OWN);
    for b in source.map(breaks).unwrap_or_default() {
        if !all.iter().any(|a| a.symptom == b.symptom) {
            all.push(b);
        }
    }
    all
}

/// This copy, as the message names it.
pub struct Builder<'a> {
    /// Its label, `0.5.2-nightly (main @ 2a704d6, 2026-10-10)`, and its version.
    pub label: &'a str,
    pub version: &'a str,
}

/// The phases that are the builder's own: reading main's source before running any of its code.
const BUILDER_PHASES: [&str; 2] = ["unpack", "check"];

/// Whether a build that failed with `error`, in `phase`, with `log` its last lines, failed
/// because this copy's builder is too old for main at `commit`; and what to say if so.
pub fn diagnose(
    error: &str,
    phase: &str,
    log: &[String],
    breaks: &[Break],
    builder: &Builder,
    commit: &str,
) -> Option<TooOld> {
    let way_out = "switch to Stable in Settings › Updates and update to the latest release, \
                   then switch back to Nightly";
    let seen = |s: &str| error.contains(s) || log.iter().any(|l| l.contains(s));
    if let Some(b) = breaks
        .iter()
        .find(|b| !b.symptom.is_empty() && seen(&b.symptom))
    {
        let fixed = match (b.pr, &b.fixed_in) {
            (Some(pr), Some(c)) => format!(" It was fixed in #{pr} (main @ {})", short(c)),
            (Some(pr), None) => format!(" It was fixed in #{pr}"),
            (None, Some(c)) => format!(" It was fixed at main @ {}", short(c)),
            (None, None) => " It was fixed on main".into(),
        };
        let mine = semver::Version::parse(builder.version).ok();
        let release = b
            .release
            .clone()
            .filter(|r| match (semver::Version::parse(r).ok(), &mine) {
                (Some(r), Some(m)) => r > *m,
                _ => true,
            });
        let message = match &release {
            Some(r) => format!(
                "This copy of photonoxide, {}, is too old to build main @ {}: {}.{fixed}, \
                 released in {r}. To get past it, {way_out}.",
                builder.label,
                short(commit),
                b.what
            ),
            None => format!(
                "This copy of photonoxide, {}, is too old to build main @ {}: {}.{fixed}, \
                 which isn't in a release newer than this copy yet. Build main once by hand \
                 (studio/README.md, Channels, says how) or wait for the next release; then \
                 Nightly builds again from the window.",
                builder.label,
                short(commit),
                b.what
            ),
        };
        return Some(TooOld {
            message,
            known: Some(b.clone()),
            release,
        });
    }
    let main_s = error.starts_with("this commit needs")
        || error == "cancelled"
        || error.contains("took longer than");
    let builder_s =
        BUILDER_PHASES.contains(&phase) || error.starts_with("the build finished, but made no");
    if builder_s && !main_s {
        return Some(TooOld {
            message: format!(
                "The build stopped while reading main @ {}'s source, before running any of its \
                 code: main may have changed in a way this copy's builder ({}) doesn't know. If \
                 a release newer than this copy is out, {way_out}. Otherwise see \
                 https://github.com/{REPO}/issues for a fix.",
                short(commit),
                builder.label
            ),
            known: None,
            release: None,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: &str = "896e64c9b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0";

    fn old() -> Builder<'static> {
        Builder {
            label: "0.5.2-nightly (main @ 2a704d6, 2026-10-10)",
            version: "0.5.2-nightly",
        }
    }

    #[test]
    fn the_list_compiled_in_reads_and_names_the_version_workspace_break() {
        let all = known(None);
        let b = all
            .iter()
            .find(|b| b.symptom == "Cargo.toml has no version")
            .expect("the version.workspace break is listed");
        assert_eq!((b.pr, b.release.as_deref()), (Some(293), Some("0.5.2")));
        assert!(
            b.fixed_in
                .as_deref()
                .is_some_and(|c| c.starts_with("860f599"))
        );
        // a list from the source adds what this copy doesn't know, once
        let source = r#"[{"symptom":"Cargo.toml has no version","what":"x"},
                         {"symptom":"package.json has no workspace","what":"y","pr":400}]"#;
        let all = known(Some(source));
        assert_eq!(all.len(), breaks(OWN).len() + 1);
        assert!(known(Some("not json")).len() == breaks(OWN).len());
    }

    #[test]
    fn a_too_old_builder_says_so_names_the_fix_and_the_way_out() {
        // the real case: a nightly from before #293 building main after #290
        let error = r"C:\Users\x\AppData\Local\gr.tachsin.photonoxide\nightly\source\Cargo.toml has no version";
        let too_old = diagnose(error, "check", &[], &known(None), &old(), MAIN).unwrap();
        assert_eq!(too_old.release.as_deref(), Some("0.5.2"));
        let m = &too_old.message;
        assert!(
            m.contains("0.5.2-nightly (main @ 2a704d6, 2026-10-10)"),
            "{m}"
        );
        assert!(m.contains("too old to build main @ 896e64c"), "{m}");
        assert!(
            m.contains("#293 (main @ 860f599), released in 0.5.2"),
            "{m}"
        );
        assert!(
            m.contains("switch to Stable") && m.contains("back to Nightly"),
            "{m}"
        );
        // the symptom found in the log rather than the error
        let log = vec!["== the version doesn't read: Cargo.toml has no version".to_owned()];
        assert!(diagnose("failed", "check", &log, &known(None), &old(), MAIN).is_some());
        // a break whose fix isn't released yet, or released before this copy: no Stable advice
        let unreleased = Break {
            symptom: "frobnicate".into(),
            what: "the manifest frobnicates".into(),
            pr: Some(400),
            fixed_in: None,
            release: None,
        };
        let t = diagnose("frobnicate", "check", &[], &[unreleased], &old(), MAIN).unwrap();
        assert!(
            t.release.is_none() && t.message.contains("by hand"),
            "{}",
            t.message
        );
        let new = Builder {
            label: "0.6.0",
            version: "0.6.0",
        };
        let t = diagnose(error, "check", &[], &known(None), &new, MAIN).unwrap();
        assert!(t.release.is_none(), "{}", t.message);
    }

    #[test]
    fn an_unknown_failure_reading_main_s_source_may_be_the_builder_but_main_s_own_isn_t() {
        let e = "a Cargo.toml doesn't read: expected `=`";
        let t = diagnose(e, "check", &[], &known(None), &old(), MAIN).unwrap();
        assert!(
            t.known.is_none() && t.message.contains("may have changed"),
            "{}",
            t.message
        );
        // main's code failing to compile is main's, not the builder's
        assert!(
            diagnose(
                "pnpm tauri build failed (exit code: 1)",
                "compile",
                &[],
                &known(None),
                &old(),
                MAIN
            )
            .is_none()
        );
        // a missing tool, a cancel, the timeout: not the builder
        for e in [
            "this commit needs what isn't here: Rust 1.99",
            "cancelled",
            "the build took longer than its 120 minutes and was stopped",
        ] {
            assert!(
                diagnose(e, "check", &[], &known(None), &old(), MAIN).is_none(),
                "{e}"
            );
        }
    }
}
