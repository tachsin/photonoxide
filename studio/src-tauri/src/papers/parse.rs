//! The sources' answers read into papers: Crossref's JSON, arXiv's Atom and OpenAlex's JSON.
//! Every string that reaches the window is plain text: tags (JATS, MathML, HTML) are taken out
//! and entities decoded here, and the window shows the text as text, never as markup.

use serde_json::Value;

use super::{Paper, Reference};

/// A paper's id as the studio keys it: `doi:<doi, lower case>` or `arxiv:<id without version>`.
pub fn paper_id(doi: Option<&str>, arxiv: Option<&str>) -> String {
    match (doi, arxiv) {
        (Some(d), _) => format!("doi:{}", d.to_lowercase()),
        (None, Some(a)) => format!("arxiv:{a}"),
        (None, None) => String::new(),
    }
}

/// What the user typed or pasted, read as a paper: a DOI (bare, `doi:`, or a doi.org link) or an
/// arXiv id (bare, `arXiv:`, or an arxiv.org link; new style `2305.01234` or old `hep-th/9901001`,
/// the version dropped). `None` for anything else.
pub fn read_id(text: &str) -> Option<String> {
    let t = text.trim().trim_end_matches(['.', ',', ';', ')']);
    let lower = t.to_lowercase();
    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi.org/",
        "doi:",
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return doi_like(rest.trim()).map(|d| format!("doi:{d}"));
        }
    }
    if let Some(d) = doi_like(&lower) {
        return Some(format!("doi:{d}"));
    }
    let mut a = lower.as_str();
    for prefix in [
        "https://arxiv.org/abs/",
        "http://arxiv.org/abs/",
        "https://arxiv.org/pdf/",
        "http://arxiv.org/pdf/",
        "arxiv.org/abs/",
        "arxiv:",
    ] {
        if let Some(rest) = a.strip_prefix(prefix) {
            a = rest;
            break;
        }
    }
    let a = a.trim_end_matches(".pdf");
    let a = match a.rfind('v') {
        Some(k) if k > 0 && a[k + 1..].bytes().all(|b| b.is_ascii_digit()) && k + 1 < a.len() => {
            &a[..k]
        }
        _ => a,
    };
    arxiv_like(a).then(|| format!("arxiv:{a}"))
}

/// `10.<registrant>/<suffix>` with no spaces.
fn doi_like(s: &str) -> Option<String> {
    let (prefix, suffix) = s.split_once('/')?;
    let reg = prefix.strip_prefix("10.")?;
    (reg.len() >= 4
        && reg.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && !suffix.is_empty()
        && !s.chars().any(char::is_whitespace))
    .then(|| s.to_owned())
}

/// `YYMM.NNNNN` (4 or 5 digits) or an old-style `archive(.class)/YYMMNNN`.
fn arxiv_like(s: &str) -> bool {
    if let Some((yymm, n)) = s.split_once('.')
        && yymm.len() == 4
        && yymm.bytes().all(|b| b.is_ascii_digit())
        && (4..=5).contains(&n.len())
        && n.bytes().all(|b| b.is_ascii_digit())
    {
        return true;
    }
    if let Some((archive, n)) = s.split_once('/') {
        return !archive.is_empty()
            && archive
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'-' || b == b'.')
            && n.len() == 7
            && n.bytes().all(|b| b.is_ascii_digit());
    }
    false
}

/// Markup made plain text: tags dropped (a JATS `<jats:title>Abstract</jats:title>` with them),
/// entities decoded, runs of white space made one space.
pub fn plain(s: &str) -> String {
    let s = &layout(s);
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        // a tag opens with a letter, `/`, `!` or `?`; anything else ("x < y") is text
        if !after[1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?'))
        {
            out.push('<');
            rest = &after[1..];
            continue;
        }
        let Some(close) = after.find('>') else {
            // a lone '<' is text
            out.push_str(after);
            rest = "";
            break;
        };
        let tag = &after[1..close];
        // a heading inside an abstract ("Abstract") is no part of it
        let name = tag
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("");
        if !tag.starts_with('/') && name.ends_with("title") && !tag.ends_with('/') {
            let end = format!("</{name}>");
            if let Some(e) = after.find(&end) {
                rest = &after[e + end.len()..];
                out.push(' ');
                continue;
            }
        }
        // block tags part words
        if matches!(
            name.rsplit(':').next().unwrap_or(""),
            "p" | "sec" | "br" | "div" | "li" | "list-item"
        ) {
            out.push(' ');
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    let decoded = entities(&out);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The local name of the tag that starts at `s` (`<jats:sub>` and `</sub>` give `sub`).
fn tag_name(s: &str) -> &str {
    let s = s.trim_start_matches('<').trim_start_matches('/');
    let end = s
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(s.len());
    s[..end].rsplit(':').next().unwrap_or("")
}

/// Publishers pretty-print their markup, a line break and an indent around each inline tag
/// (`MXene/\n  <i>β</i>\n  ‐Ga\n  <sub>2</sub>\n  O`). Such a run is no space in the text when it
/// only sits between tags, joins a subscript or superscript to what it belongs to, or touches
/// punctuation; before a word it stays a space (`</sub>\n Heterojunction`).
fn layout(s: &str) -> String {
    let scripts = ["sub", "sup", "inf", "msub", "msup", "msubsup"];
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(k) = rest.find(|c: char| c.is_whitespace()) {
        let end = rest[k..]
            .find(|c: char| !c.is_whitespace())
            .map_or(rest.len(), |e| k + e);
        let run = &rest[k..end];
        let before = &rest[..k];
        let after = &rest[end..];
        out.push_str(before);
        rest = after;
        if !run.contains('\n') {
            out.push_str(run);
            continue;
        }
        let prev = out.chars().last();
        let next = after.chars().next();
        let after_tag = prev == Some('>');
        let before_tag = next == Some('<');
        // the tag just closed before the run, and the one that follows it
        let closed = after_tag
            .then(|| {
                out.rfind('<')
                    .map(|t| (out[t..].starts_with("</"), tag_name(&out[t..])))
            })
            .flatten();
        let opens = before_tag.then(|| (after.starts_with("</"), tag_name(after)));
        let punct = |c: Option<char>| c.is_some_and(|c| "/‐-–—()[],.;:".contains(c));
        let joined = if after_tag && before_tag {
            true
        } else if let Some((_, name)) = opens.filter(|_| !after_tag) {
            // text, then a tag: joined to a script, or after punctuation
            scripts.contains(&name) || punct(prev)
        } else if let Some((closing, name)) = closed.filter(|_| !before_tag) {
            // a tag, then text: after a script's end, joined to a symbol or punctuation
            // (`</sub>\n O`, `</sub>\n ‐`), else a word
            let word: String = after.chars().take_while(|c| c.is_alphanumeric()).collect();
            let short = word.chars().count() <= 2 && !word.is_empty();
            (closing && scripts.contains(&name) && (short || punct(next))) || punct(next)
        } else {
            false
        };
        if !joined {
            out.push(' ');
        }
    }
    out.push_str(rest);
    out
}

/// XML's and HTML's common entities, and numeric ones, decoded.
pub fn entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(k) = rest.find('&') {
        out.push_str(&rest[..k]);
        let after = &rest[k..];
        let end = after.find(';').filter(|&e| e <= 10);
        let decoded = end.and_then(|e| {
            let name = &after[1..e];
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => ' ',
                _ => {
                    let n = name
                        .strip_prefix("#x")
                        .or_else(|| name.strip_prefix("#X"))
                        .and_then(|h| u32::from_str_radix(h, 16).ok())
                        .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))?;
                    char::from_u32(n)?
                }
            };
            Some((c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &after[e + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// OpenAlex's `abstract_inverted_index` (each word and the positions it stands at) as the text.
pub fn inverted(index: &Value) -> String {
    let Some(map) = index.as_object() else {
        return String::new();
    };
    let mut words: Vec<(u64, &str)> = map
        .iter()
        .flat_map(|(w, at)| {
            at.as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
                .map(move |k| (k, w.as_str()))
        })
        .collect();
    words.sort_unstable_by_key(|&(k, _)| k);
    plain(&words.iter().map(|(_, w)| *w).collect::<Vec<_>>().join(" "))
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => plain(s),
        Value::Array(a) => a.first().map(text).unwrap_or_default(),
        _ => String::new(),
    }
}

/// Crossref's `{"date-parts": [[2026, 11, 4]]}` as `2026-11-04` (or `2026-11`, `2026`).
fn date_parts(v: &Value) -> String {
    let parts: Vec<u64> = v["date-parts"][0]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default();
    match parts.as_slice() {
        [y, m, d, ..] => format!("{y:04}-{m:02}-{d:02}"),
        [y, m] => format!("{y:04}-{m:02}"),
        [y] => format!("{y:04}"),
        [] => String::new(),
    }
}

fn crossref_authors(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let given = a["given"].as_str().unwrap_or("");
            let family = a["family"].as_str().or_else(|| a["name"].as_str())?;
            Some(plain(&format!("{given} {family}")))
        })
        .collect()
}

/// A Crossref work as a paper, in the journal `venue` when given (else its own container title).
fn crossref_paper(w: &Value, venue: Option<&str>) -> Option<Paper> {
    let doi = w["DOI"].as_str()?.to_lowercase();
    let title = text(&w["title"]);
    if title.is_empty() {
        return None;
    }
    let date = ["published", "published-online", "published-print", "issued"]
        .iter()
        .map(|k| date_parts(&w[*k]))
        .find(|d| !d.is_empty())
        .unwrap_or_default();
    Some(Paper {
        id: paper_id(Some(&doi), None),
        doi: Some(doi),
        arxiv: None,
        title,
        authors: crossref_authors(&w["author"]),
        venue: venue.map_or_else(|| text(&w["container-title"]), str::to_owned),
        date,
        abstract_text: w["abstract"].as_str().map(plain).unwrap_or_default(),
        topics: Vec::new(),
    })
}

fn json(body: &[u8], what: &str) -> Result<Value, String> {
    serde_json::from_slice(body)
        .map_err(|e| format!("{what} answered something that isn't JSON ({e})"))
}

/// A Crossref list of works (`/journals/<issn>/works`), each in `venue`.
pub fn crossref_list(body: &[u8], venue: &str) -> Result<Vec<Paper>, String> {
    let v = json(body, "Crossref")?;
    let items = v["message"]["items"]
        .as_array()
        .ok_or("Crossref's answer has no list of works")?;
    Ok(items
        .iter()
        .filter_map(|w| crossref_paper(w, Some(venue)))
        .collect())
}

/// What a paper's page takes from its Crossref record.
pub struct CrossrefWork {
    pub paper: Paper,
    pub subjects: Vec<String>,
    pub references: Vec<Reference>,
    pub cited_by: u64,
}

/// One Crossref work (`/works/<doi>`): the paper, its subjects, its references and how often it is cited.
pub fn crossref_work(body: &[u8]) -> Result<CrossrefWork, String> {
    let v = json(body, "Crossref")?;
    let w = &v["message"];
    let paper = crossref_paper(w, None).ok_or("Crossref's record has no DOI or title")?;
    let subjects = w["subject"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(plain)
        .collect();
    let references = w["reference"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let s = |k: &str| r[k].as_str().map(plain).filter(|s| !s.is_empty());
            Reference {
                doi: s("DOI").map(|d| d.to_lowercase()),
                title: s("article-title")
                    .or_else(|| s("volume-title"))
                    .or_else(|| s("series-title")),
                authors: s("author"),
                venue: s("journal-title"),
                year: s("year"),
                text: s("unstructured"),
            }
        })
        .collect();
    Ok(CrossrefWork {
        paper,
        subjects,
        references,
        cited_by: w["is-referenced-by-count"].as_u64().unwrap_or(0),
    })
}

/// What a paper's page takes from OpenAlex.
#[derive(Default)]
pub struct OpenAlexWork {
    /// Its OpenAlex id, `W2075689277`.
    pub id: String,
    pub doi: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub venue: String,
    pub date: String,
    pub abstract_text: String,
    pub cited_by: u64,
    pub oa_url: Option<String>,
    pub keywords: Vec<String>,
    /// The works it cites, by OpenAlex id.
    pub referenced: Vec<String>,
}

fn openalex_id(v: &Value) -> String {
    v.as_str()
        .and_then(|s| s.rsplit('/').next())
        .unwrap_or("")
        .to_owned()
}

fn openalex_work_of(w: &Value) -> OpenAlexWork {
    let year = w["publication_year"].as_u64().map(|y| y.to_string());
    OpenAlexWork {
        id: openalex_id(&w["id"]),
        doi: w["doi"]
            .as_str()
            .map(|d| d.trim_start_matches("https://doi.org/").to_lowercase()),
        title: w["display_name"]
            .as_str()
            .or_else(|| w["title"].as_str())
            .map(plain)
            .unwrap_or_default(),
        authors: w["authorships"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| a["author"]["display_name"].as_str().map(plain))
            .collect(),
        venue: w["primary_location"]["source"]["display_name"]
            .as_str()
            .map(plain)
            .unwrap_or_default(),
        date: w["publication_date"]
            .as_str()
            .map(str::to_owned)
            .or(year)
            .unwrap_or_default(),
        abstract_text: inverted(&w["abstract_inverted_index"]),
        cited_by: w["cited_by_count"].as_u64().unwrap_or(0),
        oa_url: w["open_access"]["oa_url"]
            .as_str()
            .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
            .map(str::to_owned),
        keywords: w["keywords"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|k| k["display_name"].as_str().map(plain))
            .collect(),
        referenced: w["referenced_works"]
            .as_array()
            .into_iter()
            .flatten()
            .map(openalex_id)
            .filter(|s| !s.is_empty())
            .collect(),
    }
}

/// One OpenAlex work (`/works/doi:<doi>`).
pub fn openalex_work(body: &[u8]) -> Result<OpenAlexWork, String> {
    let v = json(body, "OpenAlex")?;
    if v["id"].as_str().is_none() {
        return Err("OpenAlex's answer has no work".into());
    }
    Ok(openalex_work_of(&v))
}

/// An OpenAlex list (`/works?filter=...`): how many there are in all, and those on this page.
pub fn openalex_list(body: &[u8]) -> Result<(u64, Vec<OpenAlexWork>), String> {
    let v = json(body, "OpenAlex")?;
    let results = v["results"]
        .as_array()
        .ok_or("OpenAlex's answer has no list of works")?;
    Ok((
        v["meta"]["count"].as_u64().unwrap_or(results.len() as u64),
        results.iter().map(openalex_work_of).collect(),
    ))
}

impl OpenAlexWork {
    /// As a paper of a list (no abstract): a citing work, say.
    pub fn paper(&self) -> Paper {
        let arxiv = self
            .doi
            .as_deref()
            .and_then(|d| d.strip_prefix("10.48550/arxiv."))
            .map(str::to_owned);
        Paper {
            id: paper_id(self.doi.as_deref(), arxiv.as_deref()),
            doi: self.doi.clone(),
            arxiv,
            title: self.title.clone(),
            authors: self.authors.clone(),
            venue: self.venue.clone(),
            date: self.date.clone(),
            abstract_text: String::new(),
            topics: Vec::new(),
        }
    }
}

/// The text of the first `<tag>` in `s`, and what follows its end.
fn element<'a>(s: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let open = format!("<{tag}");
    let mut from = 0;
    loop {
        let k = s[from..].find(&open)? + from;
        let after = &s[k + open.len()..];
        // `<title` must not match `<titles`
        match after.chars().next() {
            Some('>') | Some(' ') | Some('/') => {}
            _ => {
                from = k + open.len();
                continue;
            }
        }
        let start = after.find('>')? + 1;
        if after[..start].ends_with("/>") {
            return Some(("", &after[start..]));
        }
        let body = &after[start..];
        let close = format!("</{tag}>");
        let end = body.find(&close)?;
        return Some((&body[..end], &body[end + close.len()..]));
    }
}

fn all<'a>(mut s: &'a str, tag: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    while let Some((body, rest)) = element(s, tag) {
        found.push(body);
        s = rest;
    }
    found
}

/// A preprint from arXiv's Atom: the paper, and its PDF's link.
pub struct ArxivEntry {
    pub paper: Paper,
    pub pdf: Option<String>,
    /// The journal version's DOI, when the authors gave one.
    pub journal_doi: Option<String>,
}

/// arXiv's Atom feed (`/api/query`), entry by entry.
pub fn arxiv_feed(body: &[u8]) -> Result<Vec<ArxivEntry>, String> {
    let s = std::str::from_utf8(body).map_err(|_| "arXiv answered something that isn't UTF-8")?;
    if !s.contains("<feed") {
        return Err("arXiv answered something that isn't an Atom feed".into());
    }
    let mut entries = Vec::new();
    for e in all(s, "entry") {
        let Some(id) = element(e, "id").map(|(t, _)| t.trim()) else {
            continue;
        };
        // http://arxiv.org/abs/2611.05355v1: an error is an entry too, with another id
        let Some(arxiv) = id.split("/abs/").nth(1).and_then(read_id) else {
            continue;
        };
        let arxiv = arxiv.trim_start_matches("arxiv:").to_owned();
        let title = element(e, "title")
            .map(|(t, _)| plain(t))
            .unwrap_or_default();
        if title.is_empty() {
            continue;
        }
        let journal_doi = element(e, "arxiv:doi").map(|(t, _)| t.trim().to_lowercase());
        let pdf = e
            .split("<link ")
            .skip(1)
            .find(|l| l.contains("title=\"pdf\""))
            .and_then(|l| l.split("href=\"").nth(1))
            .and_then(|l| l.split('"').next())
            .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
            .map(|u| entities(u).replace("http://", "https://"));
        entries.push(ArxivEntry {
            paper: Paper {
                id: paper_id(None, Some(&arxiv)),
                doi: None,
                arxiv: Some(arxiv),
                title,
                authors: all(e, "author")
                    .iter()
                    .filter_map(|a| element(a, "name").map(|(n, _)| plain(n)))
                    .collect(),
                venue: "arXiv".into(),
                date: element(e, "published")
                    .map(|(d, _)| d.trim().chars().take(10).collect())
                    .unwrap_or_default(),
                abstract_text: element(e, "summary")
                    .map(|(t, _)| plain(t))
                    .unwrap_or_default(),
                topics: Vec::new(),
            },
            pdf,
            journal_doi,
        });
    }
    Ok(entries)
}
