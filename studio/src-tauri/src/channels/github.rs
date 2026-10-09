//! main's head on GitHub and the commits since this build: the nightly channel's check, and
//! the download of main's source at a commit.
//!
//! Unauthenticated, so GitHub allows 60 requests an hour from an address. The head is asked
//! for as its SHA alone, with the ETag of the last answer: an unchanged main answers 304, which
//! doesn't count against the limit. The compare API, which lists the new commits, is asked only
//! when the head moved. A failure backs off, doubling from an hour up to a day, and a rate
//! limit waits for GitHub's reset time.

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// The repository the nightly channel follows: never another.
pub const REPO: &str = "tachsin/photonoxide";
/// Its branch.
pub const BRANCH: &str = "main";
const API: &str = "https://api.github.com";

/// An answer to a GET.
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// The header `name`, in any case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// What the check and the download need of the network; the tests answer from canned
/// responses.
pub trait Http: Send + Sync {
    /// A GET of `url` with `headers`, whatever its status.
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String>;

    /// Downloads `url` into `to`, telling `progress` the bytes so far and the size when known,
    /// and stopping when `stop` says so.
    fn download(
        &self,
        url: &str,
        to: &Path,
        progress: &mut dyn FnMut(u64, Option<u64>),
        stop: &dyn Fn() -> bool,
    ) -> Result<(), String>;
}

/// The network, through reqwest over rustls (ring), as Tauri's updater reaches it. Each
/// request runs on a thread of its own, so it can be made from inside the window's async
/// runtime.
pub struct Web {
    pub user_agent: String,
}

impl Web {
    fn client(user_agent: &str, timeout: Duration) -> Result<reqwest::blocking::Client, String> {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(30))
            .timeout(timeout)
            .build()
            .map_err(|e| format!("can't make an HTTP client: {e}"))
    }
}

impl Http for Web {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
        let (agent, url) = (self.user_agent.clone(), url.to_owned());
        let headers: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        std::thread::spawn(move || {
            let client = Web::client(&agent, Duration::from_secs(30))?;
            let mut request = client.get(&url);
            for (k, v) in &headers {
                request = request.header(k, v);
            }
            let response = request.send().map_err(|e| format!("{url}: {e}"))?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_owned()))
                .collect();
            let body = response
                .bytes()
                .map_err(|e| format!("{url}: {e}"))?
                .to_vec();
            Ok(Response {
                status,
                headers,
                body,
            })
        })
        .join()
        .map_err(|_| "the request's thread panicked".to_owned())?
    }

    fn download(
        &self,
        url: &str,
        to: &Path,
        progress: &mut dyn FnMut(u64, Option<u64>),
        stop: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        let client = Web::client(&self.user_agent, Duration::from_secs(1800))?;
        let mut response = client.get(url).send().map_err(|e| format!("{url}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("{url}: GitHub answered {}", response.status()));
        }
        let size = response.content_length();
        let mut file = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
        let mut buffer = vec![0u8; 1 << 16];
        let mut done = 0u64;
        loop {
            if stop() {
                return Err("cancelled".into());
            }
            let n = response
                .read(&mut buffer)
                .map_err(|e| format!("{url}: {e}"))?;
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n])
                .map_err(|e| format!("{}: {e}", to.display()))?;
            done += n as u64;
            progress(done, size);
        }
        file.flush().map_err(|e| format!("{}: {e}", to.display()))
    }
}

/// A commit since this build, as the window lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    /// The first line of its message.
    pub title: String,
    pub author: String,
    /// When it was committed, as GitHub gives it (`2026-10-09T08:15:00Z`).
    pub date: String,
}

/// What the check found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nightly {
    /// `"available"`: main has commits this build hasn't; `"current"`: this build is main's
    /// head; `"ahead"`: main has nothing this build hasn't (a build from a branch, or from
    /// commits not pushed yet).
    pub status: String,
    /// main's head: the commit a build would be made from.
    pub head: String,
    /// When the head was committed, `2026-10-09`, when the compare API said.
    pub head_date: Option<String>,
    /// The commit this build was made from, if known.
    pub built: Option<String>,
    /// The new commits, newest first (GitHub lists up to 250).
    pub commits: Vec<Commit>,
    /// How many commits main has that this build hasn't.
    pub new_commits: u32,
    /// Something to say about the list, when it couldn't be made.
    pub note: Option<String>,
    /// When it was checked, seconds since 1970.
    pub checked_at: u64,
}

/// The check's memory, kept between runs (`check.json` in the nightly folder), so a restart
/// keeps the ETag and the backoff.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Checker {
    /// The ETag of the last answer for main's head, and that head.
    pub etag: Option<String>,
    pub head: Option<String>,
    /// Failures in a row, and no quiet check before `retry_after` (seconds since 1970).
    pub failures: u32,
    pub retry_after: u64,
    /// GitHub's rate limit is reached: no check before this.
    pub limited_until: u64,
    /// The last result.
    pub last: Option<Nightly>,
}

/// The longest a failing check waits before the next.
const MAX_BACKOFF: u64 = 86_400;

/// The wait after the `failures`-th failure in a row: an hour, doubling, up to a day.
pub fn backoff(failures: u32) -> u64 {
    let doublings = failures.saturating_sub(1).min(16);
    (3600u64 << doublings).min(MAX_BACKOFF)
}

/// Whether `s` is a full commit SHA.
pub fn is_sha(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The first seven characters of a SHA.
pub fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

/// Seconds since 1970 as `HH:MM UTC`, for saying when a wait ends.
fn clock(unix: u64) -> String {
    let day = unix % 86_400;
    format!("{:02}:{:02} UTC", day / 3600, (day % 3600) / 60)
}

impl Checker {
    /// Looks at main's head and lists the commits this build hasn't. `built` is this build's
    /// commit, `version` its version (whose tag is the base when the commit is unknown), `now`
    /// seconds since 1970. A `quiet` check (on opening, hourly) is skipped while backing off
    /// (`Ok(None)`); one the user asked for goes ahead, unless GitHub's limit is reached.
    pub fn check(
        &mut self,
        http: &dyn Http,
        built: Option<&str>,
        version: &str,
        now: u64,
        quiet: bool,
    ) -> Result<Option<Nightly>, String> {
        if now < self.limited_until {
            return if quiet {
                Ok(None)
            } else {
                Err(format!(
                    "GitHub's limit of 60 requests an hour without signing in is reached here; \
                     photonoxide looks again after {}",
                    clock(self.limited_until)
                ))
            };
        }
        if quiet && now < self.retry_after {
            return Ok(None);
        }
        match self.look(http, built, version, now) {
            Ok(found) => {
                self.failures = 0;
                self.retry_after = 0;
                self.last = Some(found.clone());
                Ok(Some(found))
            }
            Err(Failure::Limited { until, why }) => {
                self.limited_until = until;
                Err(format!(
                    "{why}; photonoxide looks again after {}",
                    clock(until)
                ))
            }
            Err(Failure::Other(why)) => {
                self.failures += 1;
                self.retry_after = now + backoff(self.failures);
                Err(why)
            }
        }
    }

    fn look(
        &mut self,
        http: &dyn Http,
        built: Option<&str>,
        version: &str,
        now: u64,
    ) -> Result<Nightly, Failure> {
        let head = self.head(http, now)?;
        let built = built.filter(|b| is_sha(b));
        let mut found = Nightly {
            status: "available".into(),
            head: head.clone(),
            head_date: None,
            built: built.map(str::to_owned),
            commits: Vec::new(),
            new_commits: 0,
            note: None,
            checked_at: now,
        };
        if built == Some(head.as_str()) {
            found.status = "current".into();
            return Ok(found);
        }
        // the same head and build as last time: the list is the same
        if let Some(last) = &self.last
            && last.head == head
            && last.built.as_deref() == built
        {
            return Ok(Nightly {
                checked_at: now,
                ..last.clone()
            });
        }
        let base = built.map_or_else(|| format!("v{version}"), str::to_owned);
        let url = format!("{API}/repos/{REPO}/compare/{base}...{head}");
        let r = http
            .get(&url, &[("Accept", "application/vnd.github+json")])
            .map_err(Failure::Other)?;
        match r.status {
            200 => {}
            404 | 422 => {
                found.note = Some(match built {
                    Some(b) => format!(
                        "This build's commit, {}, isn't on GitHub (a local commit?), so the \
                         changes since it can't be listed.",
                        short(b)
                    ),
                    None => format!(
                        "This build doesn't know its commit, and GitHub has no tag v{version} \
                         to compare with, so the changes can't be listed."
                    ),
                });
                return Ok(found);
            }
            _ => return Err(failure(&r, now, "the compare API")),
        }
        let compare: Compare = serde_json::from_slice(&r.body)
            .map_err(|e| Failure::Other(format!("GitHub's compare doesn't read: {e}")))?;
        found.status = match compare.status.as_str() {
            "identical" => "current",
            "behind" => "ahead",
            _ => "available",
        }
        .into();
        found.new_commits = compare.ahead_by;
        found.head_date = compare
            .commits
            .last()
            .and_then(|c| c.commit.committer.as_ref())
            .and_then(|w| w.date.as_deref())
            .map(|d| d.get(..10).unwrap_or(d).to_owned());
        found.commits = compare
            .commits
            .into_iter()
            .rev()
            .map(|c| Commit {
                title: c.commit.message.lines().next().unwrap_or("").to_owned(),
                author: c
                    .commit
                    .author
                    .as_ref()
                    .and_then(|a| a.name.clone())
                    .unwrap_or_default(),
                date: c.commit.committer.and_then(|w| w.date).unwrap_or_default(),
                sha: c.sha,
            })
            .collect();
        if compare.ahead_by as usize > found.commits.len() {
            found.note = Some(format!(
                "GitHub lists {} of the {} new commits.",
                found.commits.len(),
                compare.ahead_by
            ));
        }
        Ok(found)
    }

    /// main's head, from GitHub or, when it answers that nothing changed, from the last time.
    fn head(&mut self, http: &dyn Http, now: u64) -> Result<String, Failure> {
        let url = format!("{API}/repos/{REPO}/commits/{BRANCH}");
        let mut headers = vec![("Accept", "application/vnd.github.sha")];
        let etag = self.etag.clone().filter(|_| self.head.is_some());
        if let Some(etag) = &etag {
            headers.push(("If-None-Match", etag.as_str()));
        }
        let r = http.get(&url, &headers).map_err(Failure::Other)?;
        match r.status {
            304 => self
                .head
                .clone()
                .ok_or_else(|| Failure::Other("GitHub said main hadn't changed".into())),
            200 => {
                let sha = String::from_utf8_lossy(&r.body).trim().to_owned();
                if !is_sha(&sha) {
                    return Err(Failure::Other(format!(
                        "GitHub's answer for main's head isn't a commit: {}",
                        sha.chars().take(60).collect::<String>()
                    )));
                }
                self.etag = r.header("etag").map(str::to_owned);
                self.head = Some(sha.clone());
                Ok(sha)
            }
            _ => Err(failure(&r, now, "main's head")),
        }
    }
}

enum Failure {
    /// GitHub's rate limit, until then.
    Limited {
        until: u64,
        why: String,
    },
    Other(String),
}

/// What a status other than success means: a rate limit (403 or 429 with no requests left, or
/// a Retry-After), or a failure to back off from.
fn failure(r: &Response, now: u64, what: &str) -> Failure {
    let remaining = r.header("x-ratelimit-remaining");
    let reset = r
        .header("x-ratelimit-reset")
        .and_then(|s| s.trim().parse::<u64>().ok());
    let retry = r
        .header("retry-after")
        .and_then(|s| s.trim().parse::<u64>().ok());
    if (r.status == 403 || r.status == 429) && (remaining == Some("0") || retry.is_some()) {
        let until = retry
            .map(|s| now + s)
            .or(reset)
            .unwrap_or(now + 3600)
            .max(now + 60);
        return Failure::Limited {
            until,
            why: "GitHub's limit of 60 requests an hour without signing in is reached here".into(),
        };
    }
    Failure::Other(format!("GitHub answered {} for {what}", r.status))
}

#[derive(Deserialize)]
struct Compare {
    status: String,
    ahead_by: u32,
    commits: Vec<CompareCommit>,
}

#[derive(Deserialize)]
struct CompareCommit {
    sha: String,
    commit: CommitBody,
}

#[derive(Deserialize)]
struct CommitBody {
    message: String,
    author: Option<Who>,
    committer: Option<Who>,
}

#[derive(Deserialize)]
struct Who {
    name: Option<String>,
    date: Option<String>,
}

/// main's source at `sha`, as GitHub serves it: a tarball whose one folder is
/// `photonoxide-<sha>`.
pub fn tarball(sha: &str) -> String {
    format!("https://codeload.github.com/{REPO}/tar.gz/{sha}")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    type Headers = Vec<(String, String)>;
    /// A status, its headers and its body.
    type Answer = (u16, Headers, Vec<u8>);

    /// Answers from canned responses, by URL, and remembers what was asked.
    #[derive(Default)]
    pub struct Canned {
        pub answers: Mutex<HashMap<String, Vec<Answer>>>,
        pub asked: Mutex<Vec<(String, Headers)>>,
    }

    impl Canned {
        /// Adds an answer for `url`; several are given in turn, the last one repeated.
        pub fn answer(&self, url: &str, status: u16, headers: &[(&str, &str)], body: &[u8]) {
            self.answers
                .lock()
                .unwrap()
                .entry(url.to_owned())
                .or_default()
                .push((
                    status,
                    headers
                        .iter()
                        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                        .collect(),
                    body.to_vec(),
                ));
        }

        pub fn asked(&self) -> usize {
            self.asked.lock().unwrap().len()
        }
    }

    impl Http for Canned {
        fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
            self.asked.lock().unwrap().push((
                url.to_owned(),
                headers
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            ));
            let mut answers = self.answers.lock().unwrap();
            let queue = answers
                .get_mut(url)
                .ok_or_else(|| format!("no network for {url}"))?;
            let (status, headers, body) = if queue.len() > 1 {
                queue.remove(0)
            } else {
                queue[0].clone()
            };
            Ok(Response {
                status,
                headers,
                body,
            })
        }

        fn download(
            &self,
            url: &str,
            to: &Path,
            progress: &mut dyn FnMut(u64, Option<u64>),
            _stop: &dyn Fn() -> bool,
        ) -> Result<(), String> {
            let r = self.get(url, &[])?;
            if r.status != 200 {
                return Err(format!("{url}: {}", r.status));
            }
            std::fs::write(to, &r.body).map_err(|e| e.to_string())?;
            progress(r.body.len() as u64, Some(r.body.len() as u64));
            Ok(())
        }
    }

    pub const HEAD: &str = "c9cae4c1aa1aa1aa1aa1aa1aa1aa1aa1aa1aa1aa";
    pub const BUILT: &str = "7d117d2bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEAD_URL: &str = "https://api.github.com/repos/tachsin/photonoxide/commits/main";

    fn compare_url(base: &str) -> String {
        format!("https://api.github.com/repos/tachsin/photonoxide/compare/{base}...{HEAD}")
    }

    /// GitHub's compare, cut to what is read: two commits, oldest first, as GitHub lists them.
    fn compare_json(status: &str, ahead: u32) -> String {
        format!(
            r#"{{"url":"…","status":"{status}","ahead_by":{ahead},"behind_by":0,"total_commits":{ahead},
            "commits":[
              {{"sha":"d99af3baaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","commit":{{"message":"docs: the Academy in the README's studio GIFs\n\nmore","author":{{"name":"tachsin","date":"2026-10-08T10:00:00Z"}},"committer":{{"name":"GitHub","date":"2026-10-08T10:01:00Z"}}}}}},
              {{"sha":"{HEAD}","commit":{{"message":"docs: plans in my own voice (#273)","author":{{"name":"tachsin","date":"2026-10-09T09:00:00Z"}},"committer":{{"name":"GitHub","date":"2026-10-09T09:02:00Z"}}}}}}
            ],"files":[]}}"#
        )
    }

    #[test]
    fn shas_and_backoff() {
        assert!(is_sha(HEAD) && !is_sha("c9cae4c") && !is_sha(&"z".repeat(40)));
        assert_eq!(short(HEAD), "c9cae4c");
        assert_eq!(
            [1, 2, 3, 5, 6, 40].map(backoff),
            [3600, 7200, 14_400, 57_600, 86_400, 86_400]
        );
        assert_eq!(clock(1_760_000_000), "08:53 UTC");
    }

    #[test]
    fn a_new_head_lists_its_commits_newest_first_and_an_unchanged_one_costs_nothing() {
        let web = Canned::default();
        web.answer(
            HEAD_URL,
            200,
            &[("ETag", "\"e1\"")],
            format!("{HEAD}\n").as_bytes(),
        );
        web.answer(HEAD_URL, 304, &[], b"");
        web.answer(
            &compare_url(BUILT),
            200,
            &[],
            compare_json("ahead", 2).as_bytes(),
        );
        let mut checker = Checker::default();
        let found = checker
            .check(&web, Some(BUILT), "0.5.0", 1000, true)
            .unwrap()
            .unwrap();
        assert_eq!(found.status, "available");
        assert_eq!((found.head.as_str(), found.new_commits), (HEAD, 2));
        assert_eq!(found.head_date.as_deref(), Some("2026-10-09"));
        let titles: Vec<&str> = found.commits.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "docs: plans in my own voice (#273)",
                "docs: the Academy in the README's studio GIFs"
            ]
        );
        assert_eq!(found.note, None);
        assert_eq!(checker.etag.as_deref(), Some("\"e1\""));
        assert_eq!(web.asked(), 2);
        // an hour later main hasn't moved: GitHub answers 304 to the ETag, and the list is
        // the one already made, with no compare
        let again = checker
            .check(&web, Some(BUILT), "0.5.0", 4600, true)
            .unwrap()
            .unwrap();
        assert_eq!(web.asked(), 3);
        let (url, headers) = web.asked.lock().unwrap()[2].clone();
        assert_eq!(url, HEAD_URL);
        assert!(headers.contains(&("If-None-Match".into(), "\"e1\"".into())));
        assert_eq!((again.commits, again.checked_at), (found.commits, 4600));
    }

    #[test]
    fn a_build_at_main_s_head_is_current_without_a_compare() {
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], HEAD.as_bytes());
        let mut checker = Checker::default();
        let found = checker
            .check(&web, Some(HEAD), "0.5.0", 0, false)
            .unwrap()
            .unwrap();
        assert_eq!((found.status.as_str(), web.asked()), ("current", 1));
    }

    #[test]
    fn a_build_ahead_of_main_or_off_github_or_without_a_commit() {
        // main is behind this build (a build from a branch)
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], HEAD.as_bytes());
        web.answer(
            &compare_url(BUILT),
            200,
            &[],
            compare_json("behind", 0).as_bytes(),
        );
        let found = Checker::default()
            .check(&web, Some(BUILT), "0.5.0", 0, false)
            .unwrap()
            .unwrap();
        assert_eq!(found.status, "ahead");
        // the commit isn't on GitHub: offered, without a list
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], HEAD.as_bytes());
        web.answer(&compare_url(BUILT), 404, &[], b"{}");
        let found = Checker::default()
            .check(&web, Some(BUILT), "0.5.0", 0, false)
            .unwrap()
            .unwrap();
        assert_eq!(found.status, "available");
        assert!(found.commits.is_empty() && found.note.unwrap().contains("7d117d2"));
        // no commit known: the release's tag is the base
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], HEAD.as_bytes());
        web.answer(
            &compare_url("v0.5.0"),
            200,
            &[],
            compare_json("ahead", 300).as_bytes(),
        );
        let found = Checker::default()
            .check(&web, None, "0.5.0", 0, false)
            .unwrap()
            .unwrap();
        assert_eq!(found.new_commits, 300);
        assert!(found.note.unwrap().contains("2 of the 300"));
    }

    #[test]
    fn failures_back_off_and_a_rate_limit_waits_for_github_s_reset() {
        let web = Canned::default();
        web.answer(HEAD_URL, 502, &[], b"bad gateway");
        let mut checker = Checker::default();
        let e = checker
            .check(&web, Some(BUILT), "0.5.0", 1000, true)
            .unwrap_err();
        assert!(e.contains("502"), "{e}");
        assert_eq!((checker.failures, checker.retry_after), (1, 4600));
        // the hourly check comes before the backoff ends: skipped, without asking GitHub
        assert_eq!(
            checker.check(&web, Some(BUILT), "0.5.0", 2000, true),
            Ok(None)
        );
        assert_eq!(web.asked(), 1);
        // the user asks: it goes ahead, fails again, and the wait doubles
        assert!(
            checker
                .check(&web, Some(BUILT), "0.5.0", 2000, false)
                .is_err()
        );
        assert_eq!((checker.failures, checker.retry_after), (2, 2000 + 7200));
        // a rate limit: nobody asks before GitHub's reset, not even the user
        let web = Canned::default();
        web.answer(
            HEAD_URL,
            403,
            &[
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", "50000"),
            ],
            b"{}",
        );
        let mut checker = Checker::default();
        let e = checker
            .check(&web, None, "0.5.0", 10_000, false)
            .unwrap_err();
        assert!(e.contains("60 requests") && e.contains("13:53 UTC"), "{e}");
        assert_eq!(checker.limited_until, 50_000);
        assert!(checker.check(&web, None, "0.5.0", 20_000, false).is_err());
        assert_eq!(checker.check(&web, None, "0.5.0", 20_000, true), Ok(None));
        assert_eq!(web.asked(), 1);
        // a success clears the backoff
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], HEAD.as_bytes());
        let mut checker = Checker {
            failures: 3,
            retry_after: 99,
            ..Checker::default()
        };
        checker.check(&web, Some(HEAD), "0.5.0", 100, true).unwrap();
        assert_eq!((checker.failures, checker.retry_after), (0, 0));
    }

    #[test]
    fn a_head_that_is_not_a_commit_is_refused() {
        let web = Canned::default();
        web.answer(HEAD_URL, 200, &[], b"<html>a captive portal</html>");
        let e = Checker::default()
            .check(&web, None, "0.5.0", 0, false)
            .unwrap_err();
        assert!(e.contains("isn't a commit"), "{e}");
    }

    #[test]
    fn the_checker_s_memory_round_trips() {
        let checker = Checker {
            etag: Some("W/\"x\"".into()),
            head: Some(HEAD.into()),
            failures: 2,
            ..Checker::default()
        };
        let json = serde_json::to_string(&checker).unwrap();
        assert_eq!(serde_json::from_str::<Checker>(&json).unwrap(), checker);
        assert_eq!(
            serde_json::from_str::<Checker>("{}").unwrap(),
            Checker::default()
        );
    }
}
