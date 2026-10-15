//! What the Releases workflow's two issues share: GETs, versions compared by their numbers, the
//! record an issue keeps of the versions it last showed, and the command line that writes an
//! issue's body and the comment on what is new (`photonoxide libraries --releases`,
//! `photonoxide dependencies`).

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::process::ExitCode;

use serde_json::Value;

use crate::channels::github::{Http, Web};

/// GETs over HTTP, each URL once.
pub struct Fetch<'a> {
    http: &'a dyn Http,
    /// The workflow's token, sent to GitHub's API alone, for its higher rate limit.
    token: Option<String>,
    seen: HashMap<String, Result<Vec<u8>, String>>,
}

impl<'a> Fetch<'a> {
    pub fn new(http: &'a dyn Http, token: Option<String>) -> Fetch<'a> {
        Fetch {
            http,
            token,
            seen: HashMap::new(),
        }
    }

    /// The body of a 200 answer to a GET of `url`.
    pub fn bytes(&mut self, url: &str) -> Result<Vec<u8>, String> {
        if let Some(r) = self.seen.get(url) {
            return r.clone();
        }
        let mut headers = Vec::new();
        let auth;
        if url.starts_with("https://api.github.com/") {
            headers.push(("Accept", "application/vnd.github+json"));
            if let Some(t) = &self.token {
                auth = format!("Bearer {t}");
                headers.push(("Authorization", auth.as_str()));
            }
        }
        let r = self.http.get(url, &headers).and_then(|r| match r.status {
            200 => Ok(r.body),
            s => Err(format!("{url}: HTTP {s}")),
        });
        self.seen.insert(url.to_owned(), r.clone());
        r
    }

    pub fn text(&mut self, url: &str) -> Result<String, String> {
        self.bytes(url)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    pub fn json(&mut self, url: &str) -> Result<Value, String> {
        serde_json::from_slice(&self.bytes(url)?).map_err(|e| format!("{url}: {e}"))
    }
}

/// A version's numbers, up to the first part that isn't one: `v0.3.34` is 0, 3, 34;
/// `2020.4.304-2ubuntu3` 2020, 4, 304.
pub fn numbers(version: &str) -> Vec<u64> {
    let v = version.trim().trim_start_matches(['v', 'V']);
    let v = v.split(['-', '+', ' ', '_']).next().unwrap_or("");
    v.split('.').map_while(|p| p.parse().ok()).collect()
}

/// Two versions by their numbers, a missing number as 0: `26` is `26.0`.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (a, b) = (numbers(a), numbers(b));
    for i in 0..a.len().max(b.len()) {
        let o = a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0));
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

/// The first ten characters of a timestamp: its day.
pub fn day(timestamp: &str) -> Option<String> {
    let d = timestamp.get(..10)?;
    (d.len() == 10 && d.as_bytes()[4] == b'-').then(|| d.to_owned())
}

/// Days since 1970 as a day of the Gregorian calendar (Hinnant's `civil_from_days`).
pub fn date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    date(i64::try_from(secs / 86_400).unwrap_or(0))
}

/// A Markdown table's cell, its pipes escaped.
pub fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

const MARKER: &str = "<!-- newest releases";

/// The record an issue's body starts with, of the version each row showed: hidden on GitHub,
/// read back the next week by [`recorded`].
pub fn record<'a>(rows: impl IntoIterator<Item = (String, &'a str)>) -> String {
    let mut out = format!("{MARKER}\n");
    for (key, version) in rows {
        out.push_str(&format!("{key} = {version}\n"));
    }
    out.push_str("-->\n");
    out
}

/// The versions an earlier body recorded, by row: none if it has no record.
pub fn recorded(body: &str) -> Option<BTreeMap<String, String>> {
    let start = body.find(MARKER)? + MARKER.len();
    let end = start + body[start..].find("-->")?;
    Some(
        body[start..end]
            .lines()
            .filter_map(|l| l.rsplit_once(" = "))
            .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
            .collect(),
    )
}

/// Whether a row's version is news: the earlier record had the row at another version. A row
/// the record didn't have, or no record at all (the issue's first run), isn't.
pub fn is_new(previous: Option<&BTreeMap<String, String>>, key: &str, version: &str) -> bool {
    previous
        .and_then(|p| p.get(key))
        .is_some_and(|old| old != version)
}

/// What a run writes: the issue's body, the comment on what is new if anything is, and what
/// couldn't be read.
pub struct Report {
    pub body: String,
    pub news: Option<String>,
    pub failures: Vec<String>,
}

/// `[--previous <file>] [--write <file>] [--news <file>]`: reads the earlier body, has `make`
/// write the report, prints the body or writes it, and writes the comment (an empty file when
/// nothing is new). Fails, after writing, when something couldn't be read.
pub fn run(
    args: &[String],
    make: impl FnOnce(&mut Fetch, Option<&BTreeMap<String, String>>, &str) -> Report,
) -> ExitCode {
    let (mut previous, mut write, mut news) = (None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let slot = match a.as_str() {
            "--previous" => &mut previous,
            "--write" => &mut write,
            "--news" => &mut news,
            _ => return crate::usage(),
        };
        match it.next() {
            Some(file) => *slot = Some(file.clone()),
            None => return crate::usage(),
        }
    }
    let previous = match &previous {
        Some(file) => match std::fs::read_to_string(file) {
            Ok(text) => recorded(&text),
            Err(e) => return crate::fail(format!("{file}: {e}")),
        },
        None => None,
    };
    let web = Web {
        user_agent: format!(
            "photonoxide/{} (https://github.com/tachsin/photonoxide)",
            photonoxide::VERSION
        ),
    };
    let token = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty());
    let mut fetch = Fetch::new(&web, token);
    let report = make(&mut fetch, previous.as_ref(), &today());
    match &write {
        Some(file) => {
            if let Err(e) = std::fs::write(file, &report.body) {
                return crate::fail(format!("{file}: {e}"));
            }
        }
        None => print!("{}", report.body),
    }
    if let Some(file) = &news
        && let Err(e) = std::fs::write(file, report.news.unwrap_or_default())
    {
        return crate::fail(format!("{file}: {e}"));
    }
    for f in &report.failures {
        eprintln!("{f}");
    }
    if report.failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{} couldn't be read", report.failures.len());
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_their_numbers() {
        assert_eq!(numbers("v0.3.34"), [0, 3, 34]);
        assert_eq!(numbers("2020.4.304-2ubuntu3"), [2020, 4, 304]);
        assert_eq!(numbers("0.8.0.10"), [0, 8, 0, 10]);
        assert_eq!(numbers("7.0.0rc1"), [7, 0]);
        assert_eq!(compare("5.10.0", "5.9.1"), Ordering::Greater);
        assert_eq!(compare("26", "26.0"), Ordering::Equal);
        assert_eq!(compare("27.0.1", "26"), Ordering::Greater);
        assert_eq!(compare("13.4.92", "13.4.92"), Ordering::Equal);
        assert_eq!(compare("12.8.6.49", "12.8.6.72"), Ordering::Less);
    }

    #[test]
    fn days_are_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(11_016), "2000-02-29");
        assert_eq!(date(20_736), "2026-10-10");
        assert_eq!(day("2026-07-16T19:58:36Z").as_deref(), Some("2026-07-16"));
        assert_eq!(day("July"), None);
    }

    #[test]
    fn a_record_reads_back_and_says_what_is_new() {
        let body = format!(
            "{}The table.\n",
            record([("a | b".to_owned(), "1.0"), ("c".to_owned(), "2.0")])
        );
        let r = recorded(&body).unwrap();
        assert_eq!((r["a | b"].as_str(), r["c"].as_str()), ("1.0", "2.0"));
        assert!(is_new(Some(&r), "a | b", "1.1"));
        assert!(!is_new(Some(&r), "a | b", "1.0"));
        // a row it didn't have, and a first run, are no news
        assert!(!is_new(Some(&r), "d", "1.0"));
        assert!(!is_new(None, "a | b", "1.1"));
        assert_eq!(recorded("an issue without a record"), None);
        assert_eq!(cell("a|b\nc"), "a\\|b c");
    }
}
