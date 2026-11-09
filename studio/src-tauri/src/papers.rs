//! The paper engine: Home's feed of the latest photonics papers, a paper's own page (its
//! abstract, the references it cites and the works that cite it), the reading list and search.
//!
//! **Sources**, public and keyless: Crossref (the journals of `papers/journals.json`, by ISSN,
//! newest first; a paper's record and its reference list), arXiv (the newest of physics.optics;
//! a preprint by its id) and OpenAlex (a paper's abstract when Crossref has none, its
//! open-access link, keywords, the titles of its references and the works that cite it).
//!
//! **Manners:** the User-Agent names the program and its repository, nothing of the user's
//! (no email, no name, no machine); requests go one at a time, at most four a second to Crossref
//! (it asks for five at most), one every three seconds to arXiv (as its API terms ask) and six a
//! second to OpenAlex; each has a 30 s timeout; a validator a source sent (an ETag or a
//! Last-Modified) is sent back, and an unchanged answer (304) keeps the cache. The feed asks each
//! source once per refresh, and the window refreshes it every four hours while it is open (half
//! an hour after a failed one), only while Settings' "Paper feed" is on; nothing runs while the
//! studio is closed. A paper the user opens is fetched then, switch or no switch.
//!
//! **The journal picker:** Home's feed shows and asks only the sources the user keeps on
//! (Settings' `paper_sources_off` lists those turned off); a source turned on that was never
//! asked is asked on its own ([`Papers::refresh`]'s `only_new`).
//!
//! **Images** (`images.rs`, off unless Settings' "Images in the feed" is on): an arXiv preprint's
//! first figure, from arxiv.org's HTML version of it; a journal's paper keeps its journal's
//! initials, drawn by the window.
//!
//! **Cache**, under the studio's local data folder (`papers/`): each source's last answer, read
//! at once when Home opens (stale while it revalidates: the window shows it and refreshes
//! behind it), and each paper opened, so it opens again offline. The reading list is the
//! user's, in the workspace (`papers/reading-list.json`), and never leaves the computer.

pub mod images;
#[cfg(test)]
mod ours;
pub mod parse;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::channels::github::{Http, Web};

/// A paper as the feed, a list and the cache keep it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Paper {
    /// `doi:<doi>` or `arxiv:<id>` (parse::paper_id).
    pub id: String,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    /// The journal's name, or "arXiv".
    pub venue: String,
    /// `YYYY-MM-DD`, or as much of it as the source gives.
    pub date: String,
    /// Plain text; empty when the source has none.
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    /// The feed's topics its title and abstract name ([`TOPICS`]).
    pub topics: Vec<String>,
}

/// A reference a paper cites: its DOI when it has one, and what is known of it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reference {
    pub doi: Option<String>,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub venue: Option<String>,
    pub year: Option<String>,
    /// The reference as the publisher wrote it, when it gave no parts.
    pub text: Option<String>,
}

/// Where photonoxide cites a paper: a kind of place and the places (or their count).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cited {
    pub place: String,
    pub names: Vec<String>,
}

/// A paper's page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaperPage {
    pub paper: Paper,
    /// Subjects (Crossref) and keywords (OpenAlex).
    pub keywords: Vec<String>,
    /// A free full text (OpenAlex), or the preprint's PDF.
    pub oa_url: Option<String>,
    /// The journal version of a preprint, by DOI.
    pub journal_doi: Option<String>,
    pub references: Vec<Reference>,
    /// How many works cite it, and the newest of them.
    pub cited_by: u64,
    pub citing: Vec<Paper>,
    /// Its OpenAlex id, `W2075689277`.
    pub openalex: Option<String>,
    /// Where photonoxide cites it (docs/references.md).
    pub ours: Vec<Cited>,
    /// When it was fetched (s since 1970).
    pub fetched_at: u64,
    /// Shown from the cache because the sources couldn't be reached: why.
    pub offline: Option<String>,
}

/// A journal of the feed (`papers/journals.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct Journal {
    pub name: String,
    pub short: String,
    pub publisher: String,
    pub issn: Vec<String>,
    /// Asked for its optics papers and kept to those that name a photonics word.
    pub filter: bool,
}

#[derive(Deserialize)]
struct JournalFile {
    journals: Vec<Journal>,
}

/// The feed's journals, from the data file built in.
pub fn journals() -> Vec<Journal> {
    serde_json::from_str::<JournalFile>(include_str!("papers/journals.json"))
        .map(|f| f.journals)
        .unwrap_or_default()
}

/// The feed's topics and the words that put a paper in one: a word in its title or abstract,
/// in any case, anywhere in a word ("modulat" finds modulator and modulation).
pub const TOPICS: [(&str, &[&str]); 7] = [
    (
        "integrated",
        &[
            "integrated photonic",
            "photonic integrated",
            "silicon photonic",
            "on-chip",
            "on chip",
            "waveguide",
            "microring",
            "ring resonator",
            "silicon nitride",
            "thin-film lithium niobate",
            "lithium niobate on insulator",
            "photonic circuit",
            "grating coupler",
            "directional coupler",
            "multimode interference",
        ],
    ),
    (
        "nonlinear",
        &[
            "nonlinear",
            "harmonic generation",
            "kerr",
            "four-wave mixing",
            "frequency comb",
            "microcomb",
            "soliton",
            "parametric",
            "supercontinuum",
            "phase matching",
            "phase-matching",
            "χ(2)",
            "χ(3)",
        ],
    ),
    (
        "quantum",
        &[
            "quantum",
            "single-photon",
            "single photon",
            "entangle",
            "squeez",
            "qubit",
            "photon pair",
            "photon-pair",
        ],
    ),
    (
        "metasurfaces",
        &[
            "metasurface",
            "metalens",
            "metamaterial",
            "meta-optic",
            "nanoantenna",
            "metagrating",
        ],
    ),
    (
        "lasers",
        &[
            "laser",
            "lasing",
            "vcsel",
            "optical amplifier",
            "optical gain",
            "mode-locked",
            "mode locking",
        ],
    ),
    (
        "modulators",
        &[
            "modulator",
            "electro-optic",
            "pockels",
            "phase shifter",
            "thermo-optic",
            "optical switch",
            "modulation bandwidth",
        ],
    ),
    (
        "fabrication",
        &[
            "fabricat",
            "lithograph",
            "etching",
            "deposition",
            "wafer",
            "foundry",
            "process variation",
            "packaging",
            "wafer bonding",
            "nanoimprint",
        ],
    ),
];

/// A filtered journal's paper is kept when its title or abstract names one of these.
pub const PHOTONICS: [&str; 22] = [
    "photon",
    "optic",
    "laser",
    "lasing",
    "waveguide",
    "plasmon",
    "metasurface",
    "metamaterial",
    "metalens",
    "resonator",
    "microcavit",
    "optical cavit",
    "fiber",
    "fibre",
    "terahertz",
    "polariton",
    "frequency comb",
    "microcomb",
    "interferomet",
    "lidar",
    "light-matter",
    "electro-optic",
];

/// The topics `paper` falls in.
pub fn topics_of(title: &str, abstract_text: &str) -> Vec<String> {
    let text = format!("{title} {abstract_text}").to_lowercase();
    TOPICS
        .iter()
        .filter(|(_, words)| words.iter().any(|w| text.contains(w)))
        .map(|(t, _)| (*t).to_owned())
        .collect()
}

/// Whether a filtered journal's paper is about photonics.
pub fn photonic(title: &str, abstract_text: &str) -> bool {
    let text = format!("{title} {abstract_text}").to_lowercase();
    PHOTONICS.iter().any(|w| text.contains(w))
}

/// The User-Agent: the program and its repository, nothing about the user.
pub fn user_agent() -> String {
    format!(
        "photonoxide-studio/{} (https://github.com/tachsin/photonoxide)",
        photonoxide::VERSION
    )
}

/// How long a feed stays fresh, and how soon a failed refresh is tried again.
pub const FRESH: Duration = Duration::from_secs(4 * 3600);
pub const RETRY: Duration = Duration::from_secs(30 * 60);
/// How long a paper's page stays fresh (its citations change slowly).
pub const PAPER_FRESH: Duration = Duration::from_secs(7 * 24 * 3600);

/// The hosts asked, each with the least time between two requests to it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Host {
    Crossref,
    Arxiv,
    OpenAlex,
    /// arxiv.org itself, for the figures: its robots.txt asks for 15 s between requests.
    ArxivWeb,
}

impl Host {
    fn gap(self) -> Duration {
        match self {
            Host::Crossref => Duration::from_millis(250),
            Host::Arxiv => Duration::from_secs(3),
            Host::OpenAlex => Duration::from_millis(170),
            Host::ArxivWeb => Duration::from_secs(15),
        }
    }
}

/// Spaces the requests to each host ([`Host::gap`]); the tests make it wait for nothing.
pub struct Polite {
    last: Mutex<HashMap<Host, Instant>>,
    scale: u32,
}

impl Default for Polite {
    fn default() -> Polite {
        Polite::new()
    }
}

impl Polite {
    pub fn new() -> Polite {
        Polite {
            last: Mutex::new(HashMap::new()),
            scale: 1,
        }
    }

    #[cfg(test)]
    pub fn none() -> Polite {
        Polite {
            last: Mutex::new(HashMap::new()),
            scale: 0,
        }
    }

    /// Waits until a request to `host` keeps its gap, and books it.
    fn wait(&self, host: Host) {
        loop {
            let wait = {
                let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                let next = last
                    .get(&host)
                    .map_or(now, |&t| t + host.gap() * self.scale);
                if next <= now {
                    last.insert(host, now);
                    return;
                }
                next - now
            };
            std::thread::sleep(wait);
        }
    }
}

/// A source of the feed: a journal through Crossref, or arXiv.
#[derive(Clone, Debug)]
pub enum Source {
    Journal(Journal),
    Arxiv,
}

impl Source {
    /// Its id in the journal picker and the settings: a journal's first ISSN, or `arxiv`.
    pub fn id(&self) -> &str {
        match self {
            Source::Journal(j) => &j.issn[0],
            Source::Arxiv => "arxiv",
        }
    }

    /// The file its cache is kept in.
    fn key(&self) -> String {
        match self {
            Source::Journal(j) => format!("crossref-{}", j.issn[0]),
            Source::Arxiv => "arxiv-physics.optics".into(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Source::Journal(j) => &j.name,
            Source::Arxiv => "arXiv",
        }
    }

    fn host(&self) -> Host {
        match self {
            Source::Journal(_) => Host::Crossref,
            Source::Arxiv => Host::Arxiv,
        }
    }

    /// The request for its newest papers, up to `today` (`YYYY-MM-DD`): a journal's issues are
    /// dated ahead of their release, and those wait for their day.
    pub fn url(&self, today: &str) -> String {
        match self {
            Source::Journal(j) => {
                let (rows, query) = if j.filter {
                    (
                        25,
                        "&query.bibliographic=photonic+photonics+optical+optics+laser+waveguide",
                    )
                } else {
                    (10, "")
                };
                format!(
                    "https://api.crossref.org/journals/{}/works?sort=published&order=desc&rows={rows}\
                     &filter=type:journal-article,until-pub-date:{today}{query}\
                     &select=DOI,title,author,container-title,published,abstract,subject,type",
                    j.issn[0]
                )
            }
            Source::Arxiv => "https://export.arxiv.org/api/query?search_query=cat:physics.optics\
                              &sortBy=submittedDate&sortOrder=descending&start=0&max_results=30"
                .into(),
        }
    }

    /// Its answer read into papers, a filtered journal's kept to photonics.
    fn read(&self, body: &[u8]) -> Result<Vec<Paper>, String> {
        let mut papers = match self {
            Source::Journal(j) => {
                // the journal's short name: the feed's lines are narrow
                let mut papers = parse::crossref_list(body, &j.short)?;
                if j.filter {
                    papers.retain(|p| photonic(&p.title, &p.abstract_text));
                }
                papers
            }
            Source::Arxiv => parse::arxiv_feed(body)?
                .into_iter()
                .map(|e| e.paper)
                .collect(),
        };
        for p in &mut papers {
            p.topics = topics_of(&p.title, &p.abstract_text);
        }
        Ok(papers)
    }
}

/// Every source, the journals of the data file then arXiv.
pub fn sources() -> Vec<Source> {
    journals()
        .into_iter()
        .filter(|j| !j.issn.is_empty())
        .map(Source::Journal)
        .chain([Source::Arxiv])
        .collect()
}

/// The sources the user keeps on: every source but those `off` names by id.
pub fn chosen(off: &[String]) -> Vec<Source> {
    sources()
        .into_iter()
        .filter(|s| !off.iter().any(|o| o == s.id()))
        .collect()
}

/// A source as the journal picker shows it.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Choice {
    /// [`Source::id`].
    pub id: String,
    pub name: String,
    /// The name the feed's lines carry ("Opt. Express").
    pub short: String,
    pub publisher: String,
    pub on: bool,
}

impl Choice {
    fn of(s: &Source, off: &[String]) -> Choice {
        let (name, short, publisher) = match s {
            Source::Journal(j) => (j.name.clone(), j.short.clone(), j.publisher.clone()),
            Source::Arxiv => (
                "arXiv, physics.optics".to_owned(),
                "arXiv".to_owned(),
                "Cornell University".to_owned(),
            ),
        };
        Choice {
            id: s.id().to_owned(),
            name,
            short,
            publisher,
            on: !off.iter().any(|o| o == s.id()),
        }
    }
}

/// A source's last answer, kept on disk.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SourceCache {
    fetched_at: u64,
    etag: Option<String>,
    last_modified: Option<String>,
    papers: Vec<Paper>,
}

/// How the feed's checks went.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedState {
    /// The last check, and the last that reached a source (s since 1970; 0: never).
    pub checked_at: u64,
    pub updated_at: u64,
    /// Checks in a row that reached no source, and why the last one didn't.
    pub failures: u32,
    pub error: String,
    /// Sources the last check couldn't read, by name, with why.
    pub source_errors: Vec<String>,
}

/// A paper as a list shows it: no abstract, three authors and how many there are.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Summary {
    pub id: String,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub author_count: usize,
    pub venue: String,
    pub date: String,
    pub topics: Vec<String>,
}

impl From<&Paper> for Summary {
    fn from(p: &Paper) -> Summary {
        Summary {
            id: p.id.clone(),
            doi: p.doi.clone(),
            arxiv: p.arxiv.clone(),
            title: p.title.clone(),
            authors: p.authors.iter().take(3).cloned().collect(),
            author_count: p.authors.len(),
            venue: p.venue.clone(),
            date: p.date.clone(),
            topics: p.topics.clone(),
        }
    }
}

/// The feed as Home shows it.
#[derive(Clone, Debug, Serialize)]
pub struct Feed {
    pub papers: Vec<Summary>,
    pub state: FeedState,
    /// Whether a check is due (the window starts one when the switch is on).
    pub due: bool,
    /// The sources, each a journal and its publisher, for Settings' text.
    pub sources: Vec<String>,
    /// Every source, on or off, for the journal picker.
    pub choices: Vec<Choice>,
    /// The sources on that were never asked (turned on since the last check).
    pub missing: usize,
}

/// A starred paper and the user's note on it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Starred {
    pub paper: Paper,
    pub note: String,
    /// When it was starred (s since 1970).
    pub added: u64,
}

/// The engine: the network, its manners, and the cache's folder.
pub struct Papers {
    dir: PathBuf,
    http: Arc<dyn Http>,
    polite: Polite,
    /// Held through a refresh, so two never run at once.
    refreshing: Mutex<()>,
    /// The figures kept on disk (`images/`).
    pub images: images::Images,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `secs` since 1970 as a UTC date, `YYYY-MM-DD` (Howard Hinnant's days-to-civil).
pub fn day(secs: u64) -> String {
    let z = (secs / 86_400) as i64 + 719_468;
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

fn read_json<T: for<'de> Deserialize<'de>>(file: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(file).ok()?).ok()
}

/// Writes `value` to `file` whole: to a file beside it, then renamed over it.
fn write_json<T: Serialize>(file: &Path, value: &T) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = file.with_extension("json.tmp");
    let json = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, file).map_err(|e| format!("{}: {e}", file.display()))
}

/// A paper id as a file name: what isn't a letter, digit, `.` or `-` becomes `_`.
fn file_name(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(150)
        .collect();
    format!("{safe}.json")
}

/// A DOI as a URL's path: what a path may not hold, percent-encoded.
fn doi_path(doi: &str) -> String {
    doi.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~/:()".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

impl Papers {
    pub fn new(dir: PathBuf, http: Arc<dyn Http>, polite: Polite) -> Papers {
        Papers {
            images: images::Images::new(dir.join("images")),
            dir,
            http,
            polite,
            refreshing: Mutex::new(()),
        }
    }

    /// The engine on the network, its cache in `dir`.
    pub fn on_the_web(dir: PathBuf) -> Papers {
        Papers::new(
            dir,
            Arc::new(Web {
                user_agent: user_agent(),
            }),
            Polite::new(),
        )
    }

    fn get(&self, host: Host, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, String> {
        self.polite.wait(host);
        let r = self.http.get(url, headers).map_err(|e| unreached(&e))?;
        match r.status {
            200 => Ok(r.body),
            404 => Err("not found".into()),
            429 => Err("asked to slow down (429)".into()),
            s => Err(format!("answered {s}")),
        }
    }

    fn state_file(&self) -> PathBuf {
        self.dir.join("feed.json")
    }

    fn source_file(&self, s: &Source) -> PathBuf {
        self.dir.join("feed").join(format!("{}.json", s.key()))
    }

    pub fn state(&self) -> FeedState {
        read_json(&self.state_file()).unwrap_or_default()
    }

    /// Every paper of the caches of the sources kept on, newest first, each once.
    fn cached(&self, off: &[String]) -> Vec<Paper> {
        let mut papers: Vec<Paper> = chosen(off)
            .iter()
            .filter_map(|s| read_json::<SourceCache>(&self.source_file(s)))
            .flat_map(|c| c.papers)
            .collect();
        papers.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.title.cmp(&b.title)));
        let mut seen = std::collections::HashSet::new();
        papers.retain(|p| seen.insert(p.id.clone()));
        papers
    }

    /// The feed of the sources kept on (all but `off`), from the cache, at once; `due` when a
    /// check should run.
    pub fn feed(&self, now: u64, off: &[String]) -> Feed {
        let state = self.state();
        let wait = if state.failures > 0 { RETRY } else { FRESH };
        let all = sources();
        Feed {
            papers: self.cached(off).iter().map(Summary::from).collect(),
            due: now.saturating_sub(state.checked_at) >= wait.as_secs(),
            state,
            sources: all
                .iter()
                .map(|s| match s {
                    Source::Journal(j) => format!("{} ({})", j.name, j.publisher),
                    Source::Arxiv => "arXiv, physics.optics".to_owned(),
                })
                .collect(),
            choices: all.iter().map(|s| Choice::of(s, off)).collect(),
            missing: chosen(off)
                .iter()
                .filter(|s| !self.source_file(s).exists())
                .count(),
        }
    }

    /// Asks the sources kept on (all but `off`) for their newest papers, keeping each answer,
    /// and returns the feed. A source that fails keeps its last answer; when none answers, the
    /// check counts as failed. Two network failures before any answer stop the check (offline:
    /// no point in waiting on every source's timeout). A refresh already running makes this one
    /// return at once. `only_new` asks only the sources never asked (just turned on in the
    /// picker), and leaves the check's state as it was; with no source to ask, nothing is asked.
    pub fn refresh(&self, now: u64, off: &[String], only_new: bool) -> Feed {
        let asked: Vec<Source> = chosen(off)
            .into_iter()
            .filter(|s| !only_new || !self.source_file(s).exists())
            .collect();
        if asked.is_empty() {
            return self.feed(now, off);
        }
        let Ok(running) = self.refreshing.try_lock() else {
            return self.feed(now, off);
        };
        let mut state = self.state();
        let today = day(now);
        let (mut reached, mut unreachable) = (0, 0);
        let mut errors = Vec::new();
        for s in asked {
            let file = self.source_file(&s);
            let mut cache: SourceCache = read_json(&file).unwrap_or_default();
            let mut headers = Vec::new();
            if let Some(e) = &cache.etag {
                headers.push(("If-None-Match", e.as_str()));
            }
            if let Some(m) = &cache.last_modified {
                headers.push(("If-Modified-Since", m.as_str()));
            }
            self.polite.wait(s.host());
            let answer = self.http.get(&s.url(&today), &headers);
            let result = match answer {
                Err(e) => {
                    unreachable += 1;
                    Err(unreached(&e))
                }
                Ok(r) if r.status == 304 => {
                    reached += 1;
                    Ok(None)
                }
                Ok(r) if r.status == 200 => {
                    reached += 1;
                    let etag = r.header("etag").map(str::to_owned);
                    let modified = r.header("last-modified").map(str::to_owned);
                    s.read(&r.body).map(|papers| Some((papers, etag, modified)))
                }
                Ok(r) => {
                    reached += 1;
                    Err(format!("answered {}", r.status))
                }
            };
            match result {
                Ok(fresh) => {
                    cache.fetched_at = now;
                    if let Some((papers, etag, modified)) = fresh {
                        cache.papers = papers;
                        cache.etag = etag;
                        cache.last_modified = modified;
                    }
                    let _ = write_json(&file, &cache);
                }
                Err(e) => errors.push(format!("{}: {e}", s.name())),
            }
            if reached == 0 && unreachable >= 2 {
                break;
            }
        }
        if only_new {
            drop(running);
            return self.feed(now, off);
        }
        state.checked_at = now;
        state.source_errors = errors;
        if reached == 0 {
            state.failures += 1;
            state.error = state
                .source_errors
                .first()
                .cloned()
                .unwrap_or_else(|| "no source answered".into());
        } else {
            state.failures = 0;
            state.error.clear();
            state.updated_at = now;
        }
        let _ = write_json(&self.state_file(), &state);
        drop(running);
        self.feed(now, off)
    }

    fn paper_file(&self, id: &str) -> PathBuf {
        self.dir.join("paper").join(file_name(id))
    }

    /// A paper's page: from the cache when it is fresh, else from the sources (and kept); the
    /// cache, marked offline, when the sources can't be reached.
    pub fn open(&self, id: &str, now: u64) -> Result<PaperPage, String> {
        let id = parse::read_id(id).ok_or_else(|| format!("{id} is not a DOI or an arXiv id"))?;
        let file = self.paper_file(&id);
        let cached: Option<PaperPage> = read_json(&file);
        if let Some(c) = &cached
            && now.saturating_sub(c.fetched_at) < PAPER_FRESH.as_secs()
        {
            let mut c = c.clone();
            c.ours = ours_of(c.paper.doi.as_deref());
            return Ok(c);
        }
        match self.fetch(&id, now) {
            Ok(page) => {
                let _ = write_json(&file, &page);
                Ok(page)
            }
            Err(e) => match cached {
                Some(mut c) => {
                    c.offline = Some(e);
                    c.ours = ours_of(c.paper.doi.as_deref());
                    Ok(c)
                }
                None => Err(e),
            },
        }
    }

    fn fetch(&self, id: &str, now: u64) -> Result<PaperPage, String> {
        let mut page = if let Some(doi) = id.strip_prefix("doi:") {
            self.fetch_doi(doi)?
        } else {
            self.fetch_arxiv(id.trim_start_matches("arxiv:"))?
        };
        page.paper.topics = topics_of(&page.paper.title, &page.paper.abstract_text);
        page.ours = ours_of(page.paper.doi.as_deref());
        page.fetched_at = now;
        Ok(page)
    }

    fn openalex_by_doi(&self, doi: &str) -> Result<parse::OpenAlexWork, String> {
        let url = format!(
            "https://api.openalex.org/works/doi:{}?select=id,doi,display_name,cited_by_count,open_access,abstract_inverted_index,keywords,primary_location,publication_date,authorships,referenced_works",
            doi_path(doi)
        );
        parse::openalex_work(&self.get(Host::OpenAlex, &url, &[])?)
    }

    fn fetch_doi(&self, doi: &str) -> Result<PaperPage, String> {
        // Crossref and OpenAlex side by side: each is a separate host, each keeps its own gap
        let (crossref, openalex) = std::thread::scope(|s| {
            let c = s.spawn(|| {
                let url = format!("https://api.crossref.org/works/{}", doi_path(doi));
                parse::crossref_work(&self.get(Host::Crossref, &url, &[])?)
            });
            let o = s.spawn(|| self.openalex_by_doi(doi));
            (
                c.join().unwrap_or_else(|_| Err("panicked".into())),
                o.join().unwrap_or_else(|_| Err("panicked".into())),
            )
        });
        let mut page = PaperPage::default();
        match (&crossref, &openalex) {
            (Ok(c), _) => {
                page.paper = c.paper.clone();
                page.keywords = c.subjects.clone();
                page.references = c.references.clone();
                page.cited_by = c.cited_by;
            }
            (Err(_), Ok(o)) => {
                let mut p = o.paper();
                if p.doi.is_none() {
                    p.doi = Some(doi.to_owned());
                    p.id = parse::paper_id(Some(doi), None);
                }
                page.paper = p;
            }
            (Err(ce), Err(oe)) => {
                return Err(format!(
                    "Crossref: {ce}; OpenAlex: {oe} (offline, or not a DOI either knows)"
                ));
            }
        }
        if let Ok(o) = &openalex {
            if page.paper.abstract_text.is_empty() {
                page.paper.abstract_text = o.abstract_text.clone();
            }
            page.oa_url = o.oa_url.clone();
            page.cited_by = page.cited_by.max(o.cited_by);
            for k in &o.keywords {
                if !page.keywords.iter().any(|x| x.eq_ignore_ascii_case(k)) {
                    page.keywords.push(k.clone());
                }
            }
            page.openalex = Some(o.id.clone()).filter(|s| !s.is_empty());
            if page.references.is_empty() {
                page.references = self.references_by_id(&o.referenced);
            }
        }
        self.links(&mut page);
        Ok(page)
    }

    fn fetch_arxiv(&self, arxiv: &str) -> Result<PaperPage, String> {
        let url = format!("https://export.arxiv.org/api/query?id_list={arxiv}");
        let entry = parse::arxiv_feed(&self.get(Host::Arxiv, &url, &[])?)?
            .into_iter()
            .next()
            .ok_or_else(|| format!("arXiv has no {arxiv}"))?;
        let mut page = PaperPage {
            paper: entry.paper,
            oa_url: entry.pdf,
            journal_doi: entry.journal_doi,
            ..PaperPage::default()
        };
        // OpenAlex knows a preprint by arXiv's DataCite DOI
        if let Ok(o) = self.openalex_by_doi(&format!("10.48550/arxiv.{arxiv}")) {
            page.cited_by = o.cited_by;
            page.keywords = o.keywords.clone();
            page.openalex = Some(o.id.clone()).filter(|s| !s.is_empty());
            page.references = self.references_by_id(&o.referenced);
        }
        self.links(&mut page);
        Ok(page)
    }

    /// References OpenAlex lists by its own ids, resolved to titles (50 at a time).
    fn references_by_id(&self, ids: &[String]) -> Vec<Reference> {
        let mut refs = Vec::new();
        for chunk in ids.chunks(50) {
            let url = format!(
                "https://api.openalex.org/works?filter=openalex:{}&select=id,doi,display_name,publication_year,primary_location,authorships&per-page=50",
                chunk.join("|")
            );
            let Ok((_, works)) = self
                .get(Host::OpenAlex, &url, &[])
                .and_then(|b| parse::openalex_list(&b))
            else {
                break;
            };
            refs.extend(works.iter().map(reference_of));
        }
        refs
    }

    /// The references' titles (those with a DOI, through OpenAlex, 50 at a time) and the newest
    /// works that cite the paper, side by side. Either may fail and leave what Crossref gave.
    fn links(&self, page: &mut PaperPage) {
        let dois: Vec<String> = page
            .references
            .iter()
            .filter(|r| r.title.is_none())
            .filter_map(|r| r.doi.clone())
            .collect();
        let openalex = page.openalex.clone();
        let (titles, citing) = std::thread::scope(|s| {
            let t = s.spawn(|| {
                let mut found: HashMap<String, parse::OpenAlexWork> = HashMap::new();
                for chunk in dois.chunks(50) {
                    let url = format!(
                        "https://api.openalex.org/works?filter=doi:{}&select=id,doi,display_name,publication_year,primary_location,authorships&per-page=50",
                        chunk.iter().map(|d| doi_path(d)).collect::<Vec<_>>().join("|")
                    );
                    match self
                        .get(Host::OpenAlex, &url, &[])
                        .and_then(|b| parse::openalex_list(&b))
                    {
                        Ok((_, works)) => {
                            for w in works {
                                if let Some(d) = w.doi.clone() {
                                    found.insert(d, w);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                found
            });
            let c = s.spawn(|| {
                let id = openalex.as_deref()?;
                let url = format!(
                    "https://api.openalex.org/works?filter=cites:{id}&sort=publication_date:desc&per-page=10&select=id,doi,display_name,publication_date,primary_location,authorships"
                );
                self.get(Host::OpenAlex, &url, &[])
                    .and_then(|b| parse::openalex_list(&b))
                    .ok()
            });
            (t.join().unwrap_or_default(), c.join().ok().flatten())
        });
        for r in &mut page.references {
            if let Some(w) = r.doi.as_ref().and_then(|d| titles.get(d)) {
                let found = reference_of(w);
                r.title = r.title.take().or(found.title);
                r.authors = r.authors.take().or(found.authors);
                r.venue = r.venue.take().or(found.venue);
                r.year = r.year.take().or(found.year);
            }
        }
        if let Some((count, works)) = citing {
            page.cited_by = page.cited_by.max(count);
            page.citing = works.iter().map(parse::OpenAlexWork::paper).collect();
        }
    }

    /// Papers of the feed's cache, the cached pages and `extra` (the reading list) whose title,
    /// authors or abstract hold every word of `query`, newest first, at most 60.
    pub fn search(&self, query: &str, extra: &[Paper]) -> Vec<Summary> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        let pages = std::fs::read_dir(self.dir.join("paper"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| read_json::<PaperPage>(&e.path()))
            .map(|p| p.paper);
        let mut seen = std::collections::HashSet::new();
        let mut found: Vec<Paper> = extra
            .iter()
            .cloned()
            // every source's papers, those the picker hides too: a search looks through all kept
            .chain(self.cached(&[]))
            .chain(pages)
            .filter(|p| seen.insert(p.id.clone()))
            .filter(|p| {
                let text = format!("{} {} {}", p.title, p.authors.join(" "), p.abstract_text)
                    .to_lowercase();
                words.iter().all(|w| text.contains(w.as_str()))
            })
            .collect();
        found.sort_by(|a, b| b.date.cmp(&a.date));
        found.iter().take(60).map(Summary::from).collect()
    }
}

/// A request that reached no server, in a few words for the window: the client's own message
/// names the whole URL and its causes.
fn unreached(e: &str) -> String {
    let e = e.to_lowercase();
    if e.contains("timed out") || e.contains("timeout") {
        "timed out".into()
    } else if e.contains("dns") || e.contains("resolve") || e.contains("no such host") {
        "no network: the name lookup failed".into()
    } else if e.contains("certificate") || e.contains("tls") {
        "the secure connection failed".into()
    } else if e.contains("no recording") {
        // the tests' canned network
        "no recording".into()
    } else {
        "couldn't connect".into()
    }
}

fn reference_of(w: &parse::OpenAlexWork) -> Reference {
    let authors = match w.authors.as_slice() {
        [] => None,
        [a] => Some(a.clone()),
        [a, b] => Some(format!("{a}, {b}")),
        [a, ..] => Some(format!("{a} et al.")),
    };
    Reference {
        doi: w.doi.clone(),
        title: Some(w.title.clone()).filter(|t| !t.is_empty()),
        authors,
        venue: Some(w.venue.clone()).filter(|v| !v.is_empty()),
        year: w.date.get(..4).map(str::to_owned),
        text: None,
    }
}

/// Where photonoxide cites the paper with `doi`, from the index build.rs wrote.
pub fn ours_of(doi: Option<&str>) -> Vec<Cited> {
    let Some(doi) = doi.map(str::to_lowercase) else {
        return Vec::new();
    };
    ours_in(include_str!(concat!(env!("OUT_DIR"), "/ours.tsv")), &doi)
}

fn ours_in(index: &str, doi: &str) -> Vec<Cited> {
    index
        .lines()
        .find_map(|l| l.strip_prefix(doi)?.strip_prefix('\t'))
        .map(|places| {
            places
                .split(';')
                .filter_map(|p| {
                    let (place, names) = p.split_once('=')?;
                    Some(Cited {
                        place: place.to_owned(),
                        names: names.split(',').map(str::to_owned).collect(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---- the reading list, in the workspace ----

fn list_file(workspace: &Path) -> PathBuf {
    workspace.join("papers").join("reading-list.json")
}

pub fn read_list(workspace: &Path) -> Vec<Starred> {
    read_json(&list_file(workspace)).unwrap_or_default()
}

pub fn write_list(workspace: &Path, list: &[Starred]) -> Result<(), String> {
    write_json(&list_file(workspace), &list)
}

// ---- the window's commands; each runs off the window's thread ----

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}

/// The feed from the cache, at once: the sources the picker keeps on.
#[tauri::command]
pub async fn papers_feed(
    papers: tauri::State<'_, Arc<Papers>>,
    studio: tauri::State<'_, crate::studio::Studio>,
) -> Result<Feed, String> {
    let p = Arc::clone(&papers);
    let off = studio.settings().paper_sources_off;
    blocking(move || p.feed(now(), &off)).await
}

/// Asks the sources the picker keeps on for their newest papers (with `only_new`, those never
/// asked), and returns the feed.
#[tauri::command]
pub async fn papers_refresh(
    papers: tauri::State<'_, Arc<Papers>>,
    studio: tauri::State<'_, crate::studio::Studio>,
    only_new: Option<bool>,
) -> Result<Feed, String> {
    let p = Arc::clone(&papers);
    let off = studio.settings().paper_sources_off;
    blocking(move || p.refresh(now(), &off, only_new.unwrap_or(false))).await
}

/// A preprint's figure for the feed, by paper id: the name `paperimg:` serves it by, or none
/// (not a preprint, no figure, or Settings' "Images in the feed" off: then nothing is asked).
#[tauri::command]
pub async fn paper_image(
    papers: tauri::State<'_, Arc<Papers>>,
    studio: tauri::State<'_, crate::studio::Studio>,
    id: String,
) -> Result<Option<String>, String> {
    if !studio.settings().paper_images {
        return Ok(None);
    }
    let p = Arc::clone(&papers);
    blocking(move || p.image(&id, now())).await?
}

/// The figures kept on disk: how many, and their bytes.
#[tauri::command]
pub async fn paper_images_kept(
    papers: tauri::State<'_, Arc<Papers>>,
) -> Result<(usize, u64), String> {
    let p = Arc::clone(&papers);
    blocking(move || p.images.kept()).await
}

/// Removes the figures kept on disk.
#[tauri::command]
pub async fn paper_images_clear(papers: tauri::State<'_, Arc<Papers>>) -> Result<(), String> {
    let p = Arc::clone(&papers);
    blocking(move || p.images.clear()).await?
}

/// A paper's page, by DOI or arXiv id (as typed or pasted).
#[tauri::command]
pub async fn paper_open(
    papers: tauri::State<'_, Arc<Papers>>,
    id: String,
) -> Result<PaperPage, String> {
    let p = Arc::clone(&papers);
    blocking(move || p.open(&id, now())).await?
}

/// Searches the cached papers and the reading list.
#[tauri::command]
pub async fn papers_search(
    papers: tauri::State<'_, Arc<Papers>>,
    studio: tauri::State<'_, crate::studio::Studio>,
    query: String,
) -> Result<Vec<Summary>, String> {
    let p = Arc::clone(&papers);
    let workspace = studio.workspace();
    blocking(move || {
        let list: Vec<Paper> = read_list(&workspace).into_iter().map(|s| s.paper).collect();
        p.search(&query, &list)
    })
    .await
}

/// The reading list.
#[tauri::command]
pub fn reading_list(studio: tauri::State<'_, crate::studio::Studio>) -> Vec<Starred> {
    read_list(&studio.workspace())
}

/// Saves the reading list.
#[tauri::command]
pub fn reading_list_save(
    studio: tauri::State<'_, crate::studio::Studio>,
    list: Vec<Starred>,
) -> Result<(), String> {
    write_list(&studio.workspace(), &list)
}

#[cfg(test)]
mod tests;
