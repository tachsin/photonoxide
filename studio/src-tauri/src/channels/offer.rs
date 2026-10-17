//! What the Nightly panel offers, from the last check, the build waiting and the one running.
//!
//! A build waiting to be installed (`ready.json`) is offered only once a check of main, made by
//! this very build, has placed it: newer than this build (never older, never this one), and
//! either main's head (**Install**) or behind it (**Revalidate**: build main's newest, the
//! waiting build kept until the new one is ready). A leftover from an earlier session, or one
//! the check hasn't seen yet, waits for the next check; one older than this build is set aside.

use serde::Serialize;

use super::builder::{Progress, Ready};
use super::github::{Commit, Nightly, is_sha, short};

/// What the panel offers.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Offer {
    /// - `"none"`: nothing (this build is main's head, or main has nothing it hasn't);
    /// - `"unchecked"`: main hasn't been looked at by this build yet, so nothing is offered;
    /// - `"build"`: main has commits this build hasn't, and nothing is built for them;
    /// - `"install"`: a build of main's head is waiting;
    /// - `"revalidate"`: a build is waiting, but main has moved past it: build main's newest;
    /// - `"building"`: a build is running (`behind` says whether main moved past it).
    pub kind: String,
    /// The build offered (waiting, running, or main's head to build): its commit and date, and
    /// how many commits main has that it hasn't (`None` when GitHub can't tell).
    pub commit: Option<String>,
    pub date: Option<String>,
    pub behind: Option<u32>,
    /// Those commits, newest first: main's new commits since the build offered, or for
    /// `"build"`, since this one.
    pub commits: Vec<Commit>,
    pub note: Option<String>,
    /// main's newest commit and its date, as last seen.
    pub head: Option<String>,
    pub head_date: Option<String>,
    /// The build waiting, when it is offered (`"install"`, `"revalidate"`, or kept through a
    /// `"building"`).
    pub ready: Option<Ready>,
    /// A build waiting that is no use, and why: it is set aside.
    #[serde(skip)]
    pub discard: Option<(Ready, String)>,
}

/// The last check, if this build made it: one made by another build (before an install, or
/// left from an earlier session) says nothing about this one.
pub fn own_check<'a>(built: Option<&str>, last: Option<&'a Nightly>) -> Option<&'a Nightly> {
    let built = built.filter(|b| is_sha(b));
    last.filter(|n| n.built.as_deref() == built)
}

/// What to offer. `built` is this build's commit, `last` the last check, `ready` the build
/// waiting (its file there), `progress` the build running or last finished.
pub fn decide(
    built: Option<&str>,
    last: Option<&Nightly>,
    ready: Option<&Ready>,
    progress: Option<&Progress>,
) -> Offer {
    let last = own_check(built, last);
    let mut offer = Offer {
        kind: "none".into(),
        head: last.map(|n| n.head.clone()),
        head_date: last.and_then(|n| n.head_date.clone()),
        ..Offer::default()
    };
    // the build waiting: this one's own commit is never offered again
    let mut ready = ready.cloned();
    if let Some(r) = ready.as_ref().filter(|r| Some(r.commit.as_str()) == built) {
        offer.discard = Some((r.clone(), "it is the build running".into()));
        ready = None;
    }
    let since = |commit: &str| last.and_then(|n| n.since(commit));
    if let Some(r) = &ready {
        match since(&r.commit) {
            // never older than the build running
            Some(s) if s.newer == Some(false) => {
                offer.discard = Some((
                    r.clone(),
                    format!(
                        "it is main @ {}, no newer than the build running",
                        short(&r.commit)
                    ),
                ));
                ready = None;
            }
            Some(_) => {}
            // not placed yet: neither offered nor set aside, until a check has seen it
            None => ready = None,
        }
    }
    if let Some(p) = progress.filter(|p| !p.finished) {
        let s = since(&p.commit);
        return Offer {
            kind: "building".into(),
            commit: Some(p.commit.clone()),
            date: p.date.clone(),
            behind: s.as_ref().and_then(|s| s.behind),
            commits: s.as_ref().map(|s| s.commits.clone()).unwrap_or_default(),
            note: s.and_then(|s| s.note),
            ready,
            ..offer
        };
    }
    let Some(last) = last else {
        offer.kind = "unchecked".into();
        return offer;
    };
    if let Some(r) = ready {
        let s = since(&r.commit).expect("placed above");
        let at_head = s.behind == Some(0);
        return Offer {
            kind: if at_head { "install" } else { "revalidate" }.into(),
            commit: Some(r.commit.clone()),
            date: r.date.clone(),
            behind: s.behind,
            commits: s.commits,
            note: s.note,
            ready: Some(r),
            ..offer
        };
    }
    if last.status == "available" {
        return Offer {
            kind: "build".into(),
            commit: Some(last.head.clone()),
            date: last.head_date.clone(),
            behind: Some(0),
            commits: last.commits.clone(),
            note: last.note.clone(),
            ..offer
        };
    }
    offer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::github::Checker;
    use crate::channels::github::tests::Canned;
    use std::path::PathBuf;

    /// main's history, oldest first, as in the real case: 2a704d6 (a nightly's), 6a581ad (the
    /// 0.5.2 release, installed), 896e64c (main's head), and a newer head, 9a9a9a9.
    const OLD: &str = "2a704d6f45d84939b13483efc4314daa11c6fb6d";
    const RELEASE: &str = "6a581ad0000000000000000000000000000000aa";
    const MID: &str = "896e64c0000000000000000000000000000000bb";
    const NEW: &str = "9a9a9a90000000000000000000000000000000cc";
    const HEAD_URL: &str = "https://api.github.com/repos/tachsin/photonoxide/commits/main";

    fn url(base: &str, head: &str) -> String {
        format!("https://api.github.com/repos/tachsin/photonoxide/compare/{base}...{head}")
    }

    /// GitHub's compare with `n` commits ahead, the newest dated 2026-10-11, titled by number.
    fn compare(n: u32) -> Vec<u8> {
        let commits: Vec<String> = (0..n)
            .map(|k| {
                format!(
                    r#"{{"sha":"{k:040}","commit":{{"message":"commit {k}","author":{{"name":"tachsin"}},"committer":{{"date":"2026-10-1{}T10:00:00Z"}}}}}}"#,
                    k.min(1)
                )
            })
            .collect();
        let status = if n == 0 { "behind" } else { "ahead" };
        format!(
            r#"{{"status":"{status}","ahead_by":{n},"commits":[{}]}}"#,
            commits.join(",")
        )
        .into_bytes()
    }

    fn ready(commit: &str) -> Ready {
        Ready {
            commit: commit.into(),
            date: Some("2026-10-10".into()),
            version: format!("0.5.2+nightly.20261010.{}", short(commit)),
            label: format!(
                "0.5.2+nightly.20261010.{0} (main @ {0}, 2026-10-10)",
                short(commit)
            ),
            bundle: "nsis".into(),
            artifact: PathBuf::from("ready/x/photonoxide_0.5.2_x64-setup.exe"),
            built_at: 0,
            seconds: 60.0,
        }
    }

    /// main at `head`; this build at `built`, main `ahead` of it; the build waiting at `waiting`,
    /// main `ahead_of_ready` of it.
    fn checked(head: &str, built: &str, ahead: u32, waiting: Option<(&str, u32)>) -> Nightly {
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], head.as_bytes());
        web.answer(&url(built, head), 200, &[], &compare(ahead));
        if let Some((w, n)) = waiting {
            web.answer(&url(w, head), 200, &[], &compare(n));
        }
        let others: Vec<&str> = waiting.map(|(w, _)| w).into_iter().collect();
        Checker::default()
            .check_with(&web, Some(built), "0.5.2", &others, 1000, false)
            .unwrap()
            .unwrap()
    }

    #[test]
    fn a_waiting_build_main_has_moved_past_is_revalidated_with_the_new_commits_listed() {
        // built and waiting at MID; this build is the release; main moved on by 2 commits
        let last = checked(NEW, RELEASE, 5, Some((MID, 2)));
        let o = decide(Some(RELEASE), Some(&last), Some(&ready(MID)), None);
        assert_eq!(o.kind, "revalidate");
        assert_eq!(
            (o.commit.as_deref(), o.date.as_deref(), o.behind),
            (Some(MID), Some("2026-10-10"), Some(2))
        );
        let titles: Vec<&str> = o.commits.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["commit 1", "commit 0"]);
        assert_eq!(
            (o.head.as_deref(), o.head_date.as_deref()),
            (Some(NEW), Some("2026-10-11"))
        );
        assert!(o.ready.is_some() && o.discard.is_none());
    }

    #[test]
    fn a_waiting_build_at_main_s_head_is_installed_with_no_new_commits() {
        let last = checked(MID, RELEASE, 3, None);
        let o = decide(Some(RELEASE), Some(&last), Some(&ready(MID)), None);
        assert_eq!((o.kind.as_str(), o.behind), ("install", Some(0)));
        assert_eq!(o.commit.as_deref(), Some(MID));
        assert!(o.commits.is_empty());
    }

    #[test]
    fn a_waiting_build_older_than_the_one_running_is_never_offered_but_set_aside() {
        // the real case: 0.5.2 (6a581ad) running, a leftover 2a704d6 waiting; main is 3 past
        // the release and 20 past the leftover
        let last = checked(MID, RELEASE, 3, Some((OLD, 20)));
        let o = decide(Some(RELEASE), Some(&last), Some(&ready(OLD)), None);
        assert_eq!(o.kind, "build", "main's head is offered instead");
        assert_eq!((o.commit.as_deref(), o.behind), (Some(MID), Some(0)));
        assert!(o.ready.is_none());
        let (r, why) = o.discard.unwrap();
        assert_eq!(r.commit, OLD);
        assert!(why.contains("no newer than the build running"), "{why}");
        // the same when this build is main's head: nothing to offer, the leftover set aside
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], MID.as_bytes());
        web.answer(&url(OLD, MID), 200, &[], &compare(20));
        let last = Checker::default()
            .check_with(&web, Some(MID), "0.5.2", &[OLD], 0, false)
            .unwrap()
            .unwrap();
        let o = decide(Some(MID), Some(&last), Some(&ready(OLD)), None);
        assert_eq!(o.kind, "none");
        assert!(o.discard.is_some());
        // a waiting build of this very commit: set aside too
        let o = decide(Some(MID), Some(&last), Some(&ready(MID)), None);
        assert!(o.discard.unwrap().1.contains("the build running"));
    }

    #[test]
    fn a_leftover_is_not_offered_before_this_build_has_checked_it() {
        // no check yet (just switched to Nightly), or a check made by another build
        let o = decide(Some(RELEASE), None, Some(&ready(OLD)), None);
        assert_eq!(o.kind, "unchecked");
        assert!(o.ready.is_none() && o.discard.is_none());
        let other = checked(MID, OLD, 20, None);
        let o = decide(Some(RELEASE), Some(&other), Some(&ready(OLD)), None);
        assert_eq!(o.kind, "unchecked");
        // checked by this build, but before the build waiting was there: it waits for the next
        let last = checked(NEW, RELEASE, 5, None);
        let o = decide(Some(RELEASE), Some(&last), Some(&ready(MID)), None);
        assert_eq!(o.kind, "build");
        assert!(o.ready.is_none() && o.discard.is_none());
    }

    #[test]
    fn with_no_new_commits_or_the_waiting_build_gone_there_is_nothing_or_a_build() {
        // this build is main's head
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], MID.as_bytes());
        let last = Checker::default()
            .check(&web, Some(MID), "0.5.2", 0, false)
            .unwrap()
            .unwrap();
        let o = decide(Some(MID), Some(&last), None, None);
        assert_eq!(o.kind, "none");
        // the waiting build's file is gone (State::ready finds none): main's head is offered
        let last = checked(NEW, RELEASE, 5, Some((MID, 2)));
        let o = decide(Some(RELEASE), Some(&last), None, None);
        assert_eq!(o.kind, "build");
        assert_eq!(
            (
                o.commit.as_deref(),
                o.date.as_deref(),
                o.behind,
                o.commits.len()
            ),
            (Some(NEW), Some("2026-10-11"), Some(0), 5)
        );
    }

    #[test]
    fn a_build_running_says_when_main_has_moved_past_it() {
        let running = |commit: &str| Progress {
            commit: commit.into(),
            date: Some("2026-10-11".into()),
            phase: "compile".into(),
            ..Progress::default()
        };
        // main moved 2 past the commit being built
        let last = checked(NEW, RELEASE, 5, Some((MID, 2)));
        let o = decide(Some(RELEASE), Some(&last), None, Some(&running(MID)));
        assert_eq!(o.kind, "building");
        assert_eq!(
            (o.commit.as_deref(), o.behind, o.commits.len()),
            (Some(MID), Some(2), 2)
        );
        assert_eq!(o.head.as_deref(), Some(NEW));
        // still main's head
        let o = decide(Some(RELEASE), Some(&last), None, Some(&running(NEW)));
        assert_eq!((o.kind.as_str(), o.behind), ("building", Some(0)));
        // a finished build is no longer running
        let mut done = running(NEW);
        done.finished = true;
        assert_eq!(
            decide(Some(RELEASE), Some(&last), None, Some(&done)).kind,
            "build"
        );
    }

    #[test]
    fn the_compare_for_a_waiting_build_is_made_once_per_head() {
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[("ETag", "\"a\"")], NEW.as_bytes());
        web.answer(HEAD_URL, 304, &[], b"");
        web.answer(&url(RELEASE, NEW), 200, &[], &compare(5));
        web.answer(&url(MID, NEW), 200, &[], &compare(2));
        let mut checker = Checker::default();
        checker
            .check_with(&web, Some(RELEASE), "0.5.2", &[MID], 0, false)
            .unwrap();
        assert_eq!(web.asked(), 3);
        let again = checker
            .check_with(&web, Some(RELEASE), "0.5.2", &[MID], 3600, true)
            .unwrap()
            .unwrap();
        assert_eq!(web.asked(), 4, "only the head, answered 304");
        assert_eq!(again.since(MID).unwrap().behind, Some(2));
        // a commit GitHub doesn't know: how far behind can't be told, and it isn't newer
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], NEW.as_bytes());
        web.answer(&url(RELEASE, NEW), 200, &[], &compare(5));
        web.answer(&url(MID, NEW), 404, &[], b"{}");
        let last = Checker::default()
            .check_with(&web, Some(RELEASE), "0.5.2", &[MID], 0, false)
            .unwrap()
            .unwrap();
        let s = last.since(MID).unwrap();
        assert_eq!((s.behind, s.newer), (None, None));
        assert!(s.note.unwrap().contains("isn't on GitHub"));
        // which is offered for revalidating, not for installing
        assert_eq!(
            decide(Some(RELEASE), Some(&last), Some(&ready(MID)), None).kind,
            "revalidate"
        );
    }
}
