//! The feed's images: an arXiv preprint's first figure, read from arxiv.org's HTML version of
//! the preprint. Asked only while Settings' "Images in the feed" is on, and only for the
//! preprints the window shows; a journal's paper keeps its journal's initials, drawn by the
//! window, since no open source gives a journal paper's figure (studio/README.md, "The paper
//! feed's images", says what was looked at).
//!
//! **What is asked, and how:** for a preprint, the first 256 kB of `arxiv.org/html/<id>` (a
//! `Range` request; the first figure comes well before that), then that figure, at most 1.5 MB
//! (a `Range` request too: a larger one is left out). arxiv.org's robots.txt allows `/html` and
//! asks for 15 s between requests, which [`Host::ArxivWeb`] keeps. The User-Agent is the feed's:
//! the program and its repository, nothing about the user.
//!
//! **Kept** in the cache's `images/`: each figure as its file, and an index of what was found
//! for each preprint (a figure, or why none: arXiv has no HTML version yet, which is asked again
//! after a day; no figure in it, or one too large, kept for the 30 days a figure is). At most
//! [`CAP`] bytes of figures: the oldest go first. The window shows them through the `paperimg:`
//! scheme (studio.rs), which serves only files of this folder, by name.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{Host, Papers, read_json, unreached, write_json};

/// How long a figure, or the knowledge that a preprint has none, is kept.
pub const KEEP: Duration = Duration::from_secs(30 * 24 * 3600);
/// How soon a preprint arXiv had no HTML version of is asked again (arXiv makes it once the
/// preprint is announced).
pub const NO_HTML_AGAIN: Duration = Duration::from_secs(24 * 3600);
/// The figures' bytes kept at most.
pub const CAP: u64 = 32 * 1024 * 1024;
/// A figure larger than this is left out.
pub const MAX_IMAGE: usize = 1_500_000;
/// How much of a preprint's HTML is read for its first figure.
pub const HTML_HEAD: usize = 256 * 1024;

/// Why a preprint has no figure kept.
const NO_HTML: &str = "no HTML version";
const NO_FIGURE: &str = "no figure";
const TOO_BIG: &str = "too large";
const NOT_AN_IMAGE: &str = "not an image";

/// What was found for a preprint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Entry {
    /// When it was asked (s since 1970).
    fetched_at: u64,
    /// Its figure's file in the folder, or none.
    file: Option<String>,
    bytes: u64,
    /// Why there is no figure.
    why: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Index {
    entries: BTreeMap<String, Entry>,
}

/// What the cache knows of a preprint.
#[derive(Debug, PartialEq)]
pub enum Lookup {
    /// Its figure, by file name.
    Kept(String),
    /// It has none (for now).
    Nothing,
    /// Never asked, or asked too long ago.
    Unknown,
}

/// The figures on disk and their index.
pub struct Images {
    dir: PathBuf,
    /// Held while the index is read and written.
    lock: Mutex<()>,
}

fn expired(e: &Entry, now: u64) -> bool {
    let life = if e.file.is_none() && e.why == NO_HTML {
        NO_HTML_AGAIN
    } else {
        KEEP
    };
    now.saturating_sub(e.fetched_at) >= life.as_secs()
}

impl Images {
    pub fn new(dir: PathBuf) -> Images {
        Images {
            dir,
            lock: Mutex::new(()),
        }
    }

    fn index_file(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    fn index(&self) -> Index {
        read_json(&self.index_file()).unwrap_or_default()
    }

    /// What is known of `key`'s figure at `now`.
    pub fn lookup(&self, key: &str, now: u64) -> Lookup {
        let _held = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        match self.index().entries.get(key) {
            Some(e) if !expired(e, now) => match &e.file {
                Some(f) if self.dir.join(f).is_file() => Lookup::Kept(f.clone()),
                Some(_) => Lookup::Unknown,
                None => Lookup::Nothing,
            },
            _ => Lookup::Unknown,
        }
    }

    /// Keeps `found`, a figure's bytes and its extension, as `key`'s figure, or remembers why
    /// it has none; then drops what is past its time and the oldest figures over [`CAP`].
    /// Returns the figure's file name.
    pub fn keep(
        &self,
        key: &str,
        now: u64,
        found: Result<(&[u8], &str), &str>,
    ) -> Result<Option<String>, String> {
        let _held = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut index = self.index();
        if let Some(old) = index.entries.remove(key).and_then(|e| e.file) {
            let _ = std::fs::remove_file(self.dir.join(old));
        }
        let entry = match found {
            Ok((bytes, ext)) => {
                let name = format!("{key}.{ext}");
                std::fs::create_dir_all(&self.dir)
                    .map_err(|e| format!("{}: {e}", self.dir.display()))?;
                let file = self.dir.join(&name);
                let tmp = self.dir.join(format!("{name}.tmp"));
                std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
                std::fs::rename(&tmp, &file).map_err(|e| format!("{}: {e}", file.display()))?;
                Entry {
                    fetched_at: now,
                    file: Some(name),
                    bytes: bytes.len() as u64,
                    why: String::new(),
                }
            }
            Err(why) => Entry {
                fetched_at: now,
                file: None,
                bytes: 0,
                why: why.to_owned(),
            },
        };
        let name = entry.file.clone();
        index.entries.insert(key.to_owned(), entry);
        self.trim(&mut index, now);
        write_json(&self.index_file(), &index)?;
        // the figure may have been the one trimmed, were it alone larger than the cap
        Ok(name.filter(|n| self.dir.join(n).is_file()))
    }

    /// Drops the entries past their time, then the oldest figures until the rest fit [`CAP`].
    fn trim(&self, index: &mut Index, now: u64) {
        let gone: Vec<String> = index
            .entries
            .iter()
            .filter(|(_, e)| expired(e, now))
            .map(|(k, _)| k.clone())
            .collect();
        for k in gone {
            if let Some(f) = index.entries.remove(&k).and_then(|e| e.file) {
                let _ = std::fs::remove_file(self.dir.join(f));
            }
        }
        let mut figures: Vec<(u64, String, u64)> = index
            .entries
            .iter()
            .filter(|(_, e)| e.file.is_some())
            .map(|(k, e)| (e.fetched_at, k.clone(), e.bytes))
            .collect();
        let mut total: u64 = figures.iter().map(|f| f.2).sum();
        figures.sort();
        for (_, k, bytes) in figures {
            if total <= CAP {
                break;
            }
            if let Some(f) = index.entries.remove(&k).and_then(|e| e.file) {
                let _ = std::fs::remove_file(self.dir.join(f));
            }
            total -= bytes;
        }
    }

    /// A kept figure's bytes and media type, by its file name; none for any other name.
    pub fn read(&self, name: &str) -> Option<(Vec<u8>, &'static str)> {
        let safe = !name.is_empty()
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        if !safe {
            return None;
        }
        let bytes = std::fs::read(self.dir.join(name)).ok()?;
        let (_, mime) = sniff(&bytes)?;
        Some((bytes, mime))
    }

    /// How many figures are kept, and their bytes.
    pub fn kept(&self) -> (usize, u64) {
        let _held = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let index = self.index();
        let figures = index.entries.values().filter(|e| e.file.is_some());
        (figures.clone().count(), figures.map(|e| e.bytes).sum())
    }

    /// Removes every figure kept, and the index.
    pub fn clear(&self) -> Result<(), String> {
        let _held = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        match std::fs::remove_dir_all(&self.dir) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("{}: {e}", self.dir.display()))
            }
            _ => Ok(()),
        }
    }
}

/// An image's extension and media type, read from its first bytes (never from what a server
/// says): PNG, JPEG, GIF or WebP; none for anything else, SVG too.
pub fn sniff(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpg", "image/jpeg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("gif", "image/gif"))
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

/// The arXiv id of a paper id (`arxiv:2611.07038`, as the feed gives it) in arXiv's present
/// form (`YYMM.NNNNN`, with a version or not): the form arXiv's HTML versions exist for.
pub fn preprint(id: &str) -> Option<&str> {
    let a = id.strip_prefix("arxiv:")?;
    let (number, version) = a.split_once('v').unwrap_or((a, "1"));
    let (month, serial) = number.split_once('.')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (month.len() == 4
        && (4..=5).contains(&serial.len())
        && digits(month)
        && digits(serial)
        && digits(version))
    .then_some(a)
}

/// The URL of the first figure in an arXiv HTML page (LaTeXML's `<figure class="ltx_figure">`,
/// not its tables), on arxiv.org, when it is a PNG, JPEG, GIF or WebP.
pub fn first_figure(html: &str) -> Option<String> {
    let mut rest = html;
    while let Some(k) = rest.find("<figure") {
        let figure = &rest[k..];
        let end = figure.find("</figure>").unwrap_or(figure.len());
        let body = &figure[..end];
        let open = &body[..body.find('>').unwrap_or(body.len())];
        if open.contains("ltx_figure")
            && !open.contains("ltx_table")
            && let Some(url) = img_src(body).and_then(|s| figure_url(&s))
        {
            return Some(url);
        }
        rest = &figure["<figure".len()..];
    }
    None
}

/// The `src` of the first `<img>` in `html`.
fn img_src(html: &str) -> Option<String> {
    let tag = &html[html.find("<img")?..];
    let tag = &tag[..tag.find('>')?];
    let at = tag.find(" src=\"")? + " src=\"".len();
    let src = &tag[at..];
    Some(src[..src.find('"')?].replace("&amp;", "&"))
}

/// A figure's `src` as a URL on arxiv.org: relative to `arxiv.org/html/`, as LaTeXML writes it
/// (`2611.07038v1/x1.png`), or already there; nothing else, and only an image's extension.
fn figure_url(src: &str) -> Option<String> {
    let image = [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|e| src.to_ascii_lowercase().ends_with(e));
    let path = if let Some(p) = src.strip_prefix("https://arxiv.org/html/") {
        p
    } else if let Some(p) = src.strip_prefix("/html/") {
        p
    } else if src.contains(':') || src.starts_with('/') {
        return None;
    } else {
        src
    };
    let plain = !path.is_empty()
        && !path.contains("..")
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/+".contains(c));
    (image && plain).then(|| format!("https://arxiv.org/html/{path}"))
}

impl Papers {
    /// A preprint's first figure, by paper id: kept, or asked of arxiv.org and kept; its file
    /// name, or none (a journal's paper, a preprint of arXiv's older form, or no figure). A
    /// failure that says nothing of the preprint (offline, a server error) is not remembered.
    pub fn image(&self, id: &str, now: u64) -> Result<Option<String>, String> {
        let Some(arxiv) = preprint(id) else {
            return Ok(None);
        };
        let key = format!("arxiv-{arxiv}");
        match self.images.lookup(&key, now) {
            Lookup::Kept(f) => return Ok(Some(f)),
            Lookup::Nothing => return Ok(None),
            Lookup::Unknown => {}
        }
        let range = format!("bytes=0-{}", HTML_HEAD - 1);
        self.polite.wait(Host::ArxivWeb);
        let page = self
            .http
            .get(
                &format!("https://arxiv.org/html/{arxiv}"),
                &[("Range", &range)],
            )
            .map_err(|e| unreached(&e))?;
        let url = match page.status {
            200 | 206 => first_figure(&String::from_utf8_lossy(&page.body)),
            404 => return self.images.keep(&key, now, Err(NO_HTML)),
            s => return Err(format!("arxiv.org answered {s}")),
        };
        let Some(url) = url else {
            return self.images.keep(&key, now, Err(NO_FIGURE));
        };
        // one byte past the most kept: a figure that fills it is too large
        let range = format!("bytes=0-{MAX_IMAGE}");
        self.polite.wait(Host::ArxivWeb);
        let figure = self
            .http
            .get(&url, &[("Range", &range)])
            .map_err(|e| unreached(&e))?;
        match figure.status {
            200 | 206 => {}
            404 => return self.images.keep(&key, now, Err(NO_FIGURE)),
            s => return Err(format!("arxiv.org answered {s}")),
        }
        if figure.body.len() > MAX_IMAGE {
            return self.images.keep(&key, now, Err(TOO_BIG));
        }
        match sniff(&figure.body) {
            Some((ext, _)) => self.images.keep(&key, now, Ok((&figure.body, ext))),
            None => self.images.keep(&key, now, Err(NOT_AN_IMAGE)),
        }
    }
}
