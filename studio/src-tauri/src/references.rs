//! The references index: every paper and book photonoxide cites, and every place that cites it.
//!
//! A citation is found by its form, wherever it stands (Markdown, Rust, YAML, TOML, Python):
//!
//! - `doi:10.…`, with or without a space after the colon and a quote before the DOI: prose and
//!   comments (`doi:10.1364/OE.21.001310`), front matter (`doi: 10.…`) and data fields in code
//!   (`doi: "10.…"`);
//! - a link to `https://doi.org/10.…`;
//! - a string literal holding a DOI and nothing else (`"10.1364/OE.21.001310"`), as the library's
//!   reference tables have;
//! - `ref:<key>` for a source without a DOI, its key the authors' names and the year
//!   (`ref:abramowitz-stegun-1964`).
//!
//! [`DATA`] holds each source once, with its authors, title, venue and year (Crossref's year of
//! publication: online, for a paper put online before its issue); [`PAGE`] is written from it
//! and from the citations, each source with every place that cites it, by file and line, and
//! [`INDEX`] with it, every citing file with its sources. [`problems`] is what CI checks: the
//! pages are current, every cited source has an entry, every entry is cited, every DOI is well
//! formed and written in one of the forms above.
//! [`crossref`] fills the entries of new DOIs from Crossref, on demand (it needs the network).
//!
//! The module uses the standard library alone (Crossref through `curl`), so CI's Docs job
//! compiles it by itself (`.github/scripts/references.rs`) and checks every pull request without
//! building the studio.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

/// The sources, one entry per DOI or key.
pub const DATA: &str = "docs/references.yaml";
/// The sources by first author, each with every place that cites it, written from [`DATA`] and
/// the citations.
pub const PAGE: &str = "docs/references.md";
/// Each citing file with the sources it cites, written with [`PAGE`].
pub const INDEX: &str = "docs/references-by-file.md";

/// Files the scan skips: the index itself, and this generator, whose tests hold sample citations.
const SKIP: &[&str] = &[
    DATA,
    PAGE,
    INDEX,
    "studio/src-tauri/src/references.rs",
    ".github/scripts/references.rs",
];
/// Folders the scan skips: the examples' printed output, which repeats their sources.
const SKIP_DIRS: &[&str] = &["examples/output/"];

/// What the Crossref requests say they are; no address of a person.
const AGENT: &str = "photonoxide-references/1 (+https://github.com/tachsin/photonoxide)";

/// A cited source: a DOI (lower case, as DOIs compare without case) or a key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Id {
    Doi(String),
    Key(String),
}

/// How a citation is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Form {
    /// `doi:10.…`, `doi: 10.…` or `doi: "10.…"`.
    Prefix,
    /// `https://doi.org/10.…`.
    Link,
    /// `"10.…"`, a string literal holding the DOI alone.
    Literal,
    /// `ref:<key>`.
    Key,
}

/// One place that cites a source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    pub id: Id,
    /// The DOI or key as written there.
    pub written: String,
    pub form: Form,
    /// The file, relative to the repository's root, with `/`.
    pub file: String,
    /// From 1.
    pub line: usize,
    /// The validation case a row of the validation report is, when the place is one.
    pub case: Option<String>,
}

/// Something the check fails on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Problem {
    /// `file:line`, or the file alone.
    pub place: String,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.place, self.message)
    }
}

/// The DOI at the start of `s`, if one is: `10.`, a registrant code of 4 to 9 digits, `/`, then
/// everything up to a space, a quote, a bracket, a brace, `|`, `*`, `\` or a `)` without its `(`;
/// a final `.`, `,`, `;` or `:` (and a `>` without its `<`) is the sentence's, not the DOI's.
pub fn doi(s: &str) -> Option<&str> {
    let rest = s.strip_prefix("10.")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if !(4..=9).contains(&digits) || rest.as_bytes().get(digits) != Some(&b'/') {
        return None;
    }
    let start = 3 + digits + 1;
    let mut end = s.len();
    let mut depth = 0usize;
    for (j, c) in s[start..].char_indices() {
        let close = c == ')' && depth == 0;
        if c.is_whitespace() || "\"'`[]{}|*\\".contains(c) || close {
            end = start + j;
            break;
        }
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }
    let mut t = &s[..end];
    loop {
        let suffix = &t[start..];
        match suffix.chars().last() {
            Some('.' | ',' | ';' | ':') => t = &t[..t.len() - 1],
            Some('>') if !suffix.contains('<') => t = &t[..t.len() - 1],
            _ => break,
        }
    }
    (t.len() > start).then_some(t)
}

/// The key at the start of `s`, if one is: lower-case letters and digits in words joined by `-`,
/// the last word a year (four digits from 1800), maybe with a letter after it.
pub fn key(s: &str) -> Option<&str> {
    let len = s
        .bytes()
        .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
        .count();
    let k = s[..len].trim_end_matches('-');
    let (head, last) = k.rsplit_once('-')?;
    let year = last
        .strip_suffix(|c: char| c.is_ascii_lowercase())
        .unwrap_or(last);
    let ok = !head.is_empty()
        && head.starts_with(|c: char| c.is_ascii_lowercase())
        && !head.contains("--")
        && year.len() == 4
        && year.bytes().all(|b| b.is_ascii_digit())
        && (year.starts_with("18") || year.starts_with("19") || year.starts_with("20"));
    ok.then_some(k)
}

/// Whether the byte before `at` ends a word, so `doi:` at `at` isn't the end of a longer name.
fn word_start(line: &str, at: usize) -> bool {
    line[..at]
        .chars()
        .last()
        .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
}

/// Whether `s` starts as a DOI does: `10.` and at least two digits. (`10.1/x` and a bare `10.`
/// are the placeholders tests write, not citations.)
fn attempt(s: &str) -> bool {
    s.strip_prefix("10.")
        .is_some_and(|r| r.bytes().take_while(u8::is_ascii_digit).count() >= 2)
}

/// `%XX` decoded, as a DOI link may write `<`, `>` and `;`.
fn percent_decoded(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && let (Some(h), Some(l)) = (
                b.get(i + 1).copied().and_then(hex),
                b.get(i + 2).copied().and_then(hex),
            )
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The citations in one file's text, and the DOIs in it that are malformed or written in none of
/// the forms (a DOI not written as `doi:` is allowed on a line that cites it in a form, as the
/// text of a link to it).
pub fn scan(file: &str, text: &str) -> (Vec<Citation>, Vec<Problem>) {
    let mut cites = Vec::new();
    let mut problems = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let place = || format!("{file}:{n}");
        let lower = line.to_ascii_lowercase();
        let mut found: Vec<(Id, String, Form)> = Vec::new();
        let mut add = |written: &str, form: Form| {
            let id = Id::Doi(written.to_ascii_lowercase());
            if !found.iter().any(|(f, _, _)| *f == id) {
                found.push((id, written.to_string(), form));
            }
        };
        for (at, _) in lower.match_indices("doi:") {
            if !word_start(line, at) {
                continue;
            }
            let rest = line[at + 4..].trim_start_matches(' ');
            let rest = rest.strip_prefix('"').unwrap_or(rest);
            if !attempt(rest) {
                continue;
            }
            match doi(rest) {
                Some(d) => add(d, Form::Prefix),
                None => problems.push(Problem {
                    place: place(),
                    message: format!("a malformed DOI: {}", first_word(rest)),
                }),
            }
        }
        for (at, _) in lower.match_indices("doi.org/") {
            let rest = percent_decoded(line[at + 8..].split(' ').next().unwrap_or(""));
            if !attempt(&rest) {
                continue;
            }
            match doi(&rest) {
                Some(d) => add(d, Form::Link),
                None => problems.push(Problem {
                    place: place(),
                    message: format!("a malformed DOI link: doi.org/{}", first_word(&rest)),
                }),
            }
        }
        for (at, _) in line.match_indices("\"10.") {
            let rest = &line[at + 1..];
            if let Some(d) = doi(rest)
                && rest[d.len()..].starts_with('"')
            {
                add(d, Form::Literal);
            }
        }
        for (at, _) in lower.match_indices("ref:") {
            if !word_start(line, at) {
                continue;
            }
            if let Some(k) = key(line[at + 4..].trim_start_matches(' ')) {
                let id = Id::Key(k.to_string());
                if !found.iter().any(|(f, _, _)| *f == id) {
                    found.push((id, k.to_string(), Form::Key));
                }
            }
        }
        for (at, _) in line.match_indices("10.") {
            let before = line[..at].chars().last();
            if before.is_some_and(|c| c.is_alphanumeric() || "./%:\"-_".contains(c)) {
                continue;
            }
            if let Some(d) = doi(&line[at..])
                && !found
                    .iter()
                    .any(|(f, _, _)| *f == Id::Doi(d.to_ascii_lowercase()))
            {
                problems.push(Problem {
                    place: place(),
                    message: format!("{d}: a DOI is written doi:{d}"),
                });
            }
        }
        let case = if file == "docs/validation.md" {
            line.strip_prefix("| `")
                .and_then(|row| row.split('`').next())
                .map(str::to_string)
        } else {
            None
        };
        for (id, written, form) in found {
            cites.push(Citation {
                id,
                written,
                form,
                file: file.to_string(),
                line: n,
                case: case.clone(),
            });
        }
    }
    (cites, problems)
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// One source of [`DATA`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    /// Its DOI, or empty when it has a key.
    pub doi: String,
    /// Its key, for a source without a DOI.
    pub key: String,
    /// `E. A. J. Marcatili, S. E. Miller`; more than ten end in `et al.`.
    pub authors: String,
    pub title: String,
    /// The journal with its volume and first page or article number, the proceedings, or the
    /// publisher of a book.
    pub venue: String,
    /// The year Crossref gives as published: online, for a paper put online before its issue;
    /// in print, for an archive put online after it was printed.
    pub year: String,
    /// A copy anyone can read: the DOI's own link when its licence is Creative Commons, or
    /// another address (a preprint, the publisher's free copy).
    pub open: String,
    /// The registration agency, for a DOI that isn't Crossref's (`datacite`, …).
    pub agency: String,
    /// Its line in [`DATA`].
    pub line: usize,
}

impl Source {
    pub fn id(&self) -> Id {
        if self.doi.is_empty() {
            Id::Key(self.key.clone())
        } else {
            Id::Doi(self.doi.to_ascii_lowercase())
        }
    }

    /// `Marcatili 1969`, `Snyder and Love 1983`, `Zhang et al. 2013`.
    fn label(&self) -> String {
        let names: Vec<&str> = self.authors.split(", ").collect();
        let family = |name: &str| {
            let words: Vec<&str> = name.split_whitespace().collect();
            let initials = words.iter().rposition(|w| w.ends_with('.'));
            match initials {
                Some(k) if k + 1 < words.len() => words[k + 1..].join(" "),
                _ => name.trim().to_string(),
            }
        };
        let first = family(names[0]);
        let who = match names.len() {
            1 => first,
            2 if names[1] != "et al." => format!("{first} and {}", family(names[1])),
            _ => format!("{first} et al."),
        };
        format!("{who} {}", self.year)
    }

    /// The full citation, in Markdown: authors, "title", venue (year), DOI link, open copy.
    fn citation(&self) -> String {
        let mut s = format!("{}, \"{}\"", md(&self.authors), md(&self.title));
        if !self.venue.is_empty() {
            let _ = write!(s, ", {}", md(&self.venue));
        }
        let _ = write!(s, " ({}).", self.year);
        if !self.doi.is_empty() {
            let _ = write!(s, " [doi:{}]({})", md(&self.doi), doi_url(&self.doi));
        }
        if self.open == doi_url(&self.doi) {
            s.push_str(", open access.");
        } else if !self.open.is_empty() {
            let _ = write!(s, " [Open copy]({}).", self.open);
        }
        s
    }
}

/// The DOI's link at doi.org, with the characters a Markdown link can't hold escaped.
fn doi_url(doi: &str) -> String {
    if doi.is_empty() {
        return String::new();
    }
    let mut url = String::from("https://doi.org/");
    for c in doi.chars() {
        match c {
            '<' => url.push_str("%3C"),
            '>' => url.push_str("%3E"),
            '#' => url.push_str("%23"),
            '[' => url.push_str("%5B"),
            ']' => url.push_str("%5D"),
            ' ' => url.push_str("%20"),
            _ => url.push(c),
        }
    }
    url
}

/// Text for Markdown: the characters that would start a link, an emphasis, a tag or math escaped.
fn md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if "\\`*_[]<>$|".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

const FIELDS: &[&str] = &[
    "doi", "key", "authors", "title", "venue", "year", "open", "agency",
];

/// [`DATA`]'s text: a YAML list of flat entries, each field on one line, a string in double
/// quotes (`\"` and `\\` escaped) or plain.
pub fn parse(text: &str) -> (Vec<Source>, Vec<Problem>) {
    let mut sources: Vec<Source> = Vec::new();
    let mut problems = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let place = format!("{DATA}:{n}");
        let line = raw.trim_end();
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let body = if let Some(body) = line.strip_prefix("- ") {
            sources.push(Source {
                line: n,
                ..Source::default()
            });
            body
        } else if let Some(body) = line.strip_prefix("  ")
            && !sources.is_empty()
        {
            body
        } else {
            problems.push(Problem {
                place,
                message: "an entry starts with `- `, its other fields indented by two spaces"
                    .into(),
            });
            continue;
        };
        let Some((name, value)) = body.split_once(':') else {
            problems.push(Problem {
                place,
                message: "a field is `name: value`".into(),
            });
            continue;
        };
        let value = match unquoted(value.trim()) {
            Ok(v) => v,
            Err(e) => {
                problems.push(Problem { place, message: e });
                continue;
            }
        };
        let s = sources.last_mut().expect("an entry was started");
        let field = match name.trim() {
            "doi" => &mut s.doi,
            "key" => &mut s.key,
            "authors" => &mut s.authors,
            "title" => &mut s.title,
            "venue" => &mut s.venue,
            "year" => &mut s.year,
            "open" => &mut s.open,
            "agency" => &mut s.agency,
            other => {
                problems.push(Problem {
                    place,
                    message: format!("`{other}` isn't a field: {}", FIELDS.join(", ")),
                });
                continue;
            }
        };
        *field = value;
    }
    (sources, problems)
}

/// A field's value: in double quotes, with `\"` and `\\`, or plain up to a ` #` comment.
fn unquoted(v: &str) -> Result<String, String> {
    let Some(rest) = v.strip_prefix('"') else {
        return Ok(v.split(" #").next().unwrap_or("").trim().to_string());
    };
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(e @ ('"' | '\\')) => out.push(e),
                _ => return Err("only \\\" and \\\\ are escapes".into()),
            },
            '"' => {
                let after = chars.as_str().trim();
                return if after.is_empty() || after.starts_with('#') {
                    Ok(out)
                } else {
                    Err("text after the closing quote".into())
                };
            }
            c => out.push(c),
        }
    }
    Err("a quote without its closing quote".into())
}

/// A value as [`parse`] reads it back: plain when it is safe so, else in double quotes.
fn quoted(v: &str) -> String {
    let plain = !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "./-_()<>;:".contains(c))
        && !v.contains(": ")
        && !v.starts_with(['-', ':', '<', '>', '(']);
    if plain {
        v.to_string()
    } else {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// [`DATA`]'s text for `sources`, in the page's order.
pub fn write_data(sources: &[Source]) -> String {
    let mut s = String::from(
        "# The sources photonoxide cites: one entry per DOI, or per key for a source without one.\n\
         # docs/references.md is written from this file and the citations in the repository\n\
         # (`photonoxide references --write`, or `just references`), with references-by-file.md;\n\
         # `photonoxide references --crossref` adds the entries of newly cited DOIs from Crossref.\n\
         # year: Crossref's year of publication, the earlier of online and print (a paper put\n\
         # online before its issue: the online year). open: a copy anyone can read.\n",
    );
    for src in ordered(sources) {
        s.push('\n');
        let fields = [
            ("doi", &src.doi),
            ("key", &src.key),
            ("authors", &src.authors),
            ("title", &src.title),
            ("venue", &src.venue),
            ("year", &src.year),
            ("open", &src.open),
            ("agency", &src.agency),
        ];
        let mut first = true;
        for (name, value) in fields {
            if value.is_empty() {
                continue;
            }
            s.push_str(if first { "- " } else { "  " });
            first = false;
            let _ = writeln!(s, "{name}: {}", quoted(value));
        }
    }
    s
}

/// The sources sorted by their labels (first author, then year), then titles, each letter as
/// its base letter (`Č` as `c`).
fn ordered(sources: &[Source]) -> Vec<&Source> {
    let mut v: Vec<&Source> = sources.iter().collect();
    v.sort_by_cached_key(|s| (folded(&s.label()), s.title.to_lowercase(), s.id()));
    v
}

/// Lower case, each accented Latin letter as its base letter, for sorting by name.
fn folded(s: &str) -> String {
    const BASES: &[(&str, char)] = &[
        ("àáâãäåāăą", 'a'),
        ("çćčĉċ", 'c'),
        ("ďđ", 'd'),
        ("èéêëēėęěĕ", 'e'),
        ("ğĝġģ", 'g'),
        ("ìíîïīįı", 'i'),
        ("ķ", 'k'),
        ("łľĺļ", 'l'),
        ("ñńňņ", 'n'),
        ("òóôõöøōőŏ", 'o'),
        ("řŕŗ", 'r'),
        ("śšşŝș", 's'),
        ("ťţț", 't'),
        ("ùúûüūůűų", 'u'),
        ("ýÿ", 'y'),
        ("źżž", 'z'),
    ];
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| {
            BASES
                .iter()
                .find(|(all, _)| all.contains(c))
                .map_or(c, |(_, base)| *base)
        })
        .collect()
}

/// What a citing file is, for the page's groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Methods,
    Academy,
    Validation,
    Examples,
    Code,
    Jobs,
    Docs,
}

impl Kind {
    fn of(file: &str) -> Kind {
        if file.starts_with("docs/methods/") {
            Kind::Methods
        } else if file.starts_with("academy/") {
            Kind::Academy
        } else if file == "docs/validation.md" {
            Kind::Validation
        } else if file.starts_with("examples/") || file.starts_with("python/examples/") {
            Kind::Examples
        } else if file.starts_with("jobs/") || file.starts_with("circuits/") {
            Kind::Jobs
        } else if file.ends_with(".md") {
            Kind::Docs
        } else {
            Kind::Code
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Methods => "Methods pages",
            Kind::Academy => "Academy lessons",
            Kind::Validation => "Validation cases",
            Kind::Examples => "Examples",
            Kind::Code => "Code",
            Kind::Jobs => "Jobs and circuits",
            Kind::Docs => "Other documents",
        }
    }
}

/// A file's name on the page: a methods page or a lesson by its file name, an example by its path
/// in `examples/`, anything else by its path.
fn shown(file: &str) -> &str {
    match Kind::of(file) {
        Kind::Methods | Kind::Academy => file.rsplit('/').next().unwrap_or(file),
        Kind::Examples => file.strip_prefix("examples/").unwrap_or(file),
        _ => file,
    }
}

/// A link from the page (in `docs/`) to a file.
fn file_link(file: &str) -> String {
    match file.strip_prefix("docs/") {
        Some(inside) => inside.to_string(),
        None => format!("../{file}"),
    }
}

/// A link from the page to a line: GitHub shows a Markdown file's lines with `?plain=1`.
fn line_link(file: &str, line: usize) -> String {
    let plain = if file.ends_with(".md") {
        "?plain=1"
    } else {
        ""
    };
    format!("{}{plain}#L{line}", file_link(file))
}

/// GitHub's anchor for a heading: lower case, spaces to `-`, punctuation but `-` and `_` dropped.
fn anchor(heading: &str) -> String {
    heading
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// Every citation in the repository and the problems the scan found, from the files git tracks.
pub fn scan_repository(root: &Path) -> Result<(Vec<Citation>, Vec<Problem>), String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git ls-files: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|f| !f.is_empty())
        .filter(|f| !SKIP.contains(f) && !SKIP_DIRS.iter().any(|d| f.starts_with(d)))
        .map(str::to_string)
        .collect();
    files.sort();
    let mut cites = Vec::new();
    let mut problems = Vec::new();
    for file in files {
        // a file removed but not yet staged, a binary file: nothing to read
        let Ok(bytes) = std::fs::read(root.join(&file)) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let (c, p) = scan(&file, &text);
        cites.extend(c);
        problems.extend(p);
    }
    Ok((cites, problems))
}

/// The two pages, [`PAGE`] and [`INDEX`], from the sources and the citations.
pub fn pages(sources: &[Source], cites: &[Citation]) -> [String; 2] {
    let order = ordered(sources);
    // labels made unique by a letter after the year, in the page's order
    let mut labels: BTreeMap<Id, String> = BTreeMap::new();
    let mut seen: BTreeMap<String, Vec<Id>> = BTreeMap::new();
    for s in &order {
        seen.entry(s.label()).or_default().push(s.id());
    }
    for (label, ids) in &seen {
        for (k, id) in ids.iter().enumerate() {
            let l = if ids.len() == 1 {
                label.clone()
            } else {
                format!("{label}{}", (b'a' + k as u8) as char)
            };
            labels.insert(id.clone(), l);
        }
    }
    let mut by_source: BTreeMap<&Id, Vec<&Citation>> = BTreeMap::new();
    for c in cites {
        by_source.entry(&c.id).or_default().push(c);
    }
    let files: BTreeSet<&str> = cites.iter().map(|c| c.file.as_str()).collect();
    let with_doi = sources.iter().filter(|s| !s.doi.is_empty()).count();

    let mut p = String::new();
    p.push_str("# References\n\n");
    p.push_str(WRITTEN);
    let _ = writeln!(
        p,
        "Every paper, book and data set photonoxide cites, with every place in the repository that \
         cites it, by file and line. {} sources, {} with a DOI and {} without, are cited at {} \
         places in {} files. The sources are listed by their first author; \
         [references-by-file.md](references-by-file.md) lists each methods page, lesson, example \
         and source file with the sources it cites, and the [validation report](validation.md) \
         gives each case's source on its row. How to cite a source is in \
         [CONTRIBUTING.md](../CONTRIBUTING.md#citing-a-source); the list of sources is \
         [references.yaml](references.yaml).\n",
        sources.len(),
        with_doi,
        sources.len() - with_doi,
        cites.len(),
        files.len()
    );
    let letter = |s: &Source| {
        folded(&labels[&s.id()])
            .chars()
            .find(|c| c.is_alphabetic())
            .map_or('#', |c| c.to_uppercase().next().unwrap_or(c))
    };
    let letters: Vec<char> = {
        let mut v: Vec<char> = order.iter().map(|s| letter(s)).collect();
        v.dedup();
        v
    };
    let jump: Vec<String> = letters
        .iter()
        .map(|l| format!("[{l}](#{})", anchor(&l.to_string())))
        .collect();
    let _ = writeln!(p, "{}\n", jump.join(" · "));
    p.push_str("## Sources\n");
    let mut current = None;
    for s in &order {
        let l = letter(s);
        if current != Some(l) {
            let _ = writeln!(p, "\n### {l}");
            current = Some(l);
        }
        let id = s.id();
        let _ = writeln!(p, "\n#### {}\n\n{}\n", labels[&id], s.citation());
        let places = by_source.get(&id).cloned().unwrap_or_default();
        let mut by_kind: BTreeMap<Kind, Vec<&Citation>> = BTreeMap::new();
        for c in places {
            by_kind.entry(Kind::of(&c.file)).or_default().push(c);
        }
        for (kind, cs) in by_kind {
            let _ = writeln!(p, "- {}: {}", kind.name(), places_text(&cs));
        }
    }
    let mut x = String::new();
    x.push_str("# References by citing file\n\n");
    x.push_str(WRITTEN);
    x.push_str(
        "Each methods page, lesson, example, job, document and source file that cites a source, \
         with the sources it cites; each leads to its entry in [References](references.md), with \
         the source's full citation and every other place that cites it. The \
         [validation report](validation.md) gives each validation case's source on its row.\n",
    );
    let mut units: BTreeMap<Kind, Files> = BTreeMap::new();
    for c in cites {
        let Some(label) = labels.get(&c.id) else {
            continue;
        };
        // the validation report gives each case's source on the case's row
        let kind = Kind::of(&c.file);
        if kind == Kind::Validation {
            continue;
        }
        units
            .entry(kind)
            .or_default()
            .entry((format!("`{}`", shown(&c.file)), file_link(&c.file)))
            .or_default()
            .insert((label.to_lowercase(), label.clone()));
    }
    for (kind, list) in units {
        let _ = writeln!(x, "\n## {}\n", kind.name());
        for ((text, link), labels) in list {
            let refs: Vec<String> = labels
                .iter()
                .map(|(_, l)| format!("[{l}](references.md#{})", anchor(l)))
                .collect();
            let _ = writeln!(x, "- [{text}]({link}): {}", refs.join(", "));
        }
    }
    [p, x]
}

/// Citing files, each by its name and link, with the labels of the sources it cites (each by its
/// sort key).
type Files = BTreeMap<(String, String), BTreeSet<(String, String)>>;

/// The pages' second paragraph: how they are written.
const WRITTEN: &str = "<!-- Written by `photonoxide references --write` (or `just references`) from \
     docs/references.yaml and the citations in the repository: edit those, not this page. -->\n\n";

/// A source's places of one kind: each file once, its first line `name:line`, the others `line`.
fn places_text(cs: &[&Citation]) -> String {
    let mut parts = Vec::new();
    let mut last: Option<&str> = None;
    let mut sorted: Vec<&&Citation> = cs.iter().collect();
    sorted.sort_by_key(|c| (c.file.as_str(), c.line));
    for c in sorted {
        let text = match (&c.case, Kind::of(&c.file)) {
            (Some(case), Kind::Validation) => format!("`{case}`"),
            _ if last == Some(c.file.as_str()) => c.line.to_string(),
            _ => format!("{}:{}", shown(&c.file), c.line),
        };
        parts.push(format!("[{text}]({})", line_link(&c.file, c.line)));
        last = Some(&c.file);
    }
    parts.join(", ")
}

/// Everything the check fails on, given the sources, the citations and the page as committed.
pub fn problems(
    sources: &[Source],
    cites: &[Citation],
    committed_pages: Option<[&str; 2]>,
) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut ids = BTreeSet::new();
    for s in sources {
        let place = format!("{DATA}:{}", s.line);
        let mut bad = |m: String| {
            out.push(Problem {
                place: place.clone(),
                message: m,
            })
        };
        match (s.doi.is_empty(), s.key.is_empty()) {
            (false, false) => bad("an entry has a DOI or a key, not both".into()),
            (true, true) => bad("an entry needs a DOI or a key".into()),
            (false, true) if doi(&s.doi) != Some(s.doi.as_str()) => {
                bad(format!("a malformed DOI: {}", s.doi))
            }
            (true, false) if key(&s.key) != Some(s.key.as_str()) => bad(format!(
                "a malformed key: {} (lower-case words joined by -, the last a year)",
                s.key
            )),
            _ => {}
        }
        for (name, value) in [
            ("authors", &s.authors),
            ("title", &s.title),
            ("year", &s.year),
        ] {
            if value.is_empty() {
                bad(format!("the entry has no {name}"));
            }
        }
        if !s.year.is_empty() && !(s.year.len() == 4 && s.year.bytes().all(|b| b.is_ascii_digit()))
        {
            bad(format!("a year is four digits: {}", s.year));
        }
        if !ids.insert(s.id()) {
            bad("a second entry for the same source".into());
        }
    }
    let cited: BTreeSet<&Id> = cites.iter().map(|c| &c.id).collect();
    for s in sources {
        if !cited.contains(&s.id()) {
            out.push(Problem {
                place: format!("{DATA}:{}", s.line),
                message: format!(
                    "{}: nothing cites this source; remove its entry",
                    s.id_text()
                ),
            });
        }
    }
    let mut missing: BTreeMap<&Id, &Citation> = BTreeMap::new();
    for c in cites {
        if !ids.contains(&c.id) {
            missing.entry(&c.id).or_insert(c);
        }
    }
    for c in missing.values() {
        let fix = match c.id {
            Id::Doi(_) => "add it (`photonoxide references --crossref`)",
            Id::Key(_) => "add its entry by hand",
        };
        out.push(Problem {
            place: format!("{}:{}", c.file, c.line),
            message: format!("{}: no entry in {DATA}; {fix}", c.written),
        });
    }
    if let Some(committed) = committed_pages {
        for ((path, now), was) in [PAGE, INDEX]
            .iter()
            .zip(pages(sources, cites))
            .zip(committed)
        {
            if was.replace("\r\n", "\n") != now {
                out.push(Problem {
                    place: (*path).into(),
                    message:
                        "out of date: run `photonoxide references --write` (or `just references`)"
                            .into(),
                });
            }
        }
    }
    out
}

impl Source {
    fn id_text(&self) -> &str {
        if self.doi.is_empty() {
            &self.key
        } else {
            &self.doi
        }
    }
}

/// The repository's root: the nearest folder up from the current one that holds [`DATA`].
pub fn root() -> Result<PathBuf, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    here.ancestors()
        .find(|d| d.join(DATA).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("no {DATA} in this folder or above it"))
}

/// Everything the check fails on, for the repository at `root`.
#[cfg(test)]
fn check(root: &Path) -> Result<Vec<Problem>, String> {
    let data = std::fs::read_to_string(root.join(DATA)).map_err(|e| format!("{DATA}: {e}"))?;
    let (sources, mut found) = parse(&data);
    let (cites, scan_problems) = scan_repository(root)?;
    found.extend(scan_problems);
    let committed = committed(root);
    found.extend(problems(
        &sources,
        &cites,
        Some([&committed[0], &committed[1]]),
    ));
    found.sort();
    Ok(found)
}

/// [`PAGE`] and [`INDEX`] as they are in the repository (empty when missing).
fn committed(root: &Path) -> [String; 2] {
    [PAGE, INDEX].map(|p| std::fs::read_to_string(root.join(p)).unwrap_or_default())
}

/// `photonoxide references`: with no arguments, the counts and the problems; `--write` writes
/// [`PAGE`] and [`INDEX`], `--check` fails unless they are current; `--crossref` adds the
/// entries of cited DOIs that have none, `--crossref --all` compares every entry with Crossref.
pub fn run(args: &[String]) -> ExitCode {
    let root = match root() {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let flag = match args {
        [] => "",
        [flag] if flag == "--crossref" => return crossref(&root, false),
        [flag, all] if flag == "--crossref" && all == "--all" => return crossref(&root, true),
        [flag] if flag == "--write" || flag == "--check" => flag.as_str(),
        _ => {
            eprintln!("usage: photonoxide references [--write | --check | --crossref [--all]]");
            return ExitCode::from(2);
        }
    };
    let data = match std::fs::read_to_string(root.join(DATA)) {
        Ok(d) => d,
        Err(e) => return fail(format!("{DATA}: {e}")),
    };
    let (sources, mut found) = parse(&data);
    let (cites, scan_problems) = match scan_repository(&root) {
        Ok(x) => x,
        Err(e) => return fail(e),
    };
    found.extend(scan_problems);
    if flag == "--write" {
        for (path, text) in [PAGE, INDEX].iter().zip(pages(&sources, &cites)) {
            if let Err(e) = std::fs::write(root.join(path), text) {
                return fail(format!("{path}: {e}"));
            }
        }
    }
    let now = committed(&root);
    let check = (flag == "--check").then(|| [now[0].as_str(), now[1].as_str()]);
    found.extend(problems(&sources, &cites, check));
    found.sort();
    summary(&sources, &cites);
    for p in &found {
        eprintln!("{p}");
    }
    if found.is_empty() {
        ExitCode::SUCCESS
    } else {
        fail(format!("{} problems", found.len()))
    }
}

/// The counts: sources, places, files, and the places by form.
fn summary(sources: &[Source], cites: &[Citation]) {
    let files: BTreeSet<&str> = cites.iter().map(|c| c.file.as_str()).collect();
    let ids: BTreeSet<&Id> = cites.iter().map(|c| &c.id).collect();
    let mut forms: BTreeMap<Form, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<Kind, BTreeSet<&str>> = BTreeMap::new();
    for c in cites {
        *forms.entry(c.form).or_default() += 1;
        kinds.entry(Kind::of(&c.file)).or_default().insert(&c.file);
    }
    println!(
        "{} sources in {DATA}; {} cited, at {} places in {} files",
        sources.len(),
        ids.len(),
        cites.len(),
        files.len()
    );
    for (form, n) in forms {
        println!("  {form:?}: {n} places");
    }
    for (kind, f) in kinds {
        println!("  {}: {} files", kind.name(), f.len());
    }
}

fn fail(e: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {e}");
    ExitCode::FAILURE
}

/// Adds the entries of the cited DOIs that have none, from Crossref (or, for a DOI another agency
/// registered, from doi.org); with `all`, fetches every entry's DOI again and prints each field
/// that differs, writing nothing.
pub fn crossref(root: &Path, all: bool) -> ExitCode {
    let data = std::fs::read_to_string(root.join(DATA)).unwrap_or_default();
    let (mut sources, problems) = parse(&data);
    if !problems.is_empty() {
        for p in &problems {
            eprintln!("{p}");
        }
        return fail(format!("{DATA} doesn't read"));
    }
    let (cites, _) = match scan_repository(root) {
        Ok(x) => x,
        Err(e) => return fail(e),
    };
    let have: BTreeSet<Id> = sources.iter().map(Source::id).collect();
    // each DOI as first written, in the files' order
    let mut wanted: BTreeMap<Id, String> = BTreeMap::new();
    if all {
        for s in sources.iter().filter(|s| !s.doi.is_empty()) {
            wanted.insert(s.id(), s.doi.clone());
        }
    } else {
        for c in cites.iter().filter(|c| matches!(c.id, Id::Doi(_))) {
            if !have.contains(&c.id) {
                wanted
                    .entry(c.id.clone())
                    .or_insert_with(|| c.written.clone());
            }
        }
    }
    let started = Instant::now();
    let total = wanted.len();
    let mut failed = 0;
    for (k, (id, doi)) in wanted.iter().enumerate() {
        let fetched = fetch(doi);
        let elapsed = started.elapsed().as_secs_f64();
        let eta = elapsed / (k + 1) as f64 * (total - k - 1) as f64;
        match fetched {
            Ok(new) if all => {
                let old = sources.iter().find(|s| s.id() == *id).expect("an entry");
                for (name, a, b) in [
                    ("authors", &old.authors, &new.authors),
                    ("title", &old.title, &new.title),
                    ("venue", &old.venue, &new.venue),
                    ("year", &old.year, &new.year),
                ] {
                    if a != b {
                        println!("{doi}: {name}: {a:?} here, {b:?} at Crossref");
                    }
                }
            }
            Ok(new) => {
                println!(
                    "[{}/{total}, {:.0}%, {elapsed:.0} s, {eta:.0} s left] {doi}: {}",
                    k + 1,
                    100.0 * (k + 1) as f64 / total as f64,
                    new.label()
                );
                sources.push(new);
                // saved as it goes, so a run cut short keeps what it found
                if (k + 1) % 25 == 0 {
                    let _ = std::fs::write(root.join(DATA), write_data(&sources));
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("[{}/{total}] {doi}: {e}", k + 1);
            }
        }
    }
    if !all {
        let path = root.join(DATA);
        if let Err(e) = std::fs::write(&path, write_data(&sources)) {
            return fail(format!("{DATA}: {e}"));
        }
        println!("{total} DOIs looked up, {failed} not found; {DATA} written");
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        fail(format!("{failed} DOIs not found"))
    }
}

/// `url`'s body and status, through curl.
fn get(url: &str, accept: &str) -> Result<(u32, String), String> {
    for attempt in 0..4 {
        let out = Command::new("curl")
            .args(["-sS", "-L", "--max-time", "40", "-A", AGENT, "-H"])
            .arg(format!("Accept: {accept}"))
            .args(["-w", "\n%{http_code}", url])
            .output()
            .map_err(|e| format!("curl: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let (body, code) = text.rsplit_once('\n').unwrap_or(("", &text));
        let code: u32 = code.trim().parse().unwrap_or(0);
        // too many requests, a server error or no answer: wait, then ask again
        if code == 429 || code >= 500 || code == 0 {
            std::thread::sleep(std::time::Duration::from_secs(2 << attempt));
            continue;
        }
        return Ok((code, body.to_string()));
    }
    Err(format!("{url}: no answer"))
}

/// A DOI for a URL's path: everything but letters, digits and `-._~/` percent-encoded.
fn encoded(doi: &str) -> String {
    let mut s = String::new();
    for b in doi.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            s.push(b as char);
        } else {
            let _ = write!(s, "%{b:02X}");
        }
    }
    s
}

/// The entry for `doi`: from Crossref's API, else, for a DOI another agency registered, from
/// doi.org's CSL JSON.
fn fetch(doi: &str) -> Result<Source, String> {
    let (code, body) = get(
        &format!("https://api.crossref.org/works/{}", encoded(doi)),
        "application/json",
    )?;
    if code == 200 {
        let json = Json::parse(&body).ok_or("Crossref's answer isn't JSON")?;
        let mut s = from_csl(json.get("message").ok_or("no message")?, true);
        s.doi = doi.to_string();
        if !s.open.is_empty() {
            s.open = doi_url(doi);
        }
        return Ok(s);
    }
    let (code, body) = get(
        &format!("https://api.crossref.org/works/{}/agency", encoded(doi)),
        "application/json",
    )?;
    let answer = if code == 200 {
        Json::parse(&body)
    } else {
        None
    };
    let agency = answer
        .and_then(|j| {
            j.get("message")?
                .get("agency")?
                .get("id")?
                .str()
                .map(str::to_string)
        })
        .ok_or("neither Crossref nor another agency knows it")?;
    let (code, body) = get(
        &format!("https://doi.org/{}", encoded(doi)),
        "application/vnd.citationstyles.csl+json",
    )?;
    if code != 200 {
        return Err(format!(
            "registered with {agency}, but doi.org answers {code}"
        ));
    }
    let json = Json::parse(&body).ok_or("doi.org's answer isn't JSON")?;
    let mut s = from_csl(&json, false);
    s.doi = doi.to_string();
    s.agency = agency;
    Ok(s)
}

/// An entry from Crossref's metadata (`crossref`) or CSL JSON, whose names are much the same.
fn from_csl(m: &Json, crossref: bool) -> Source {
    let text = |name: &str| -> String {
        match m.get(name) {
            Some(Json::Str(s)) => s.clone(),
            Some(Json::Arr(v)) => v.first().and_then(Json::str).unwrap_or("").to_string(),
            _ => String::new(),
        }
    };
    let people = m
        .get("author")
        .or_else(|| m.get("editor"))
        .map(Json::items)
        .unwrap_or_default();
    let mut names: Vec<String> = people.iter().filter_map(person).collect();
    if names.len() > 10 {
        names.truncate(10);
        names.push("et al.".into());
    }
    // Crossref's `published` is the earlier of print and online: the online year of a paper put
    // online before its issue, the print year of a journal's archive put online decades later
    let year = ["published", "published-online", "published-print", "issued"]
        .iter()
        .find_map(|k| {
            let y = m
                .get(k)?
                .get("date-parts")?
                .items()
                .first()?
                .items()
                .first()?;
            match y {
                Json::Num(n) => Some(n.clone()),
                Json::Str(s) => Some(s.clone()),
                _ => None,
            }
        })
        .unwrap_or_default();
    let kind = text("type");
    let short = text("short-container-title");
    let container = if short.is_empty() {
        text("container-title")
    } else {
        short
    };
    let page = text("page")
        .split(['-', '–'])
        .next()
        .unwrap_or("")
        .to_string();
    let page = if page.is_empty() {
        text("article-number")
    } else {
        page
    };
    let publisher = text("publisher");
    let venue = match kind.as_str() {
        "journal-article" | "article-journal" => {
            let mut v = container.clone();
            let volume = text("volume");
            if !volume.is_empty() {
                let _ = write!(v, " {volume}");
            }
            if !page.is_empty() {
                let _ = write!(v, ", {page}");
            }
            v
        }
        "proceedings-article" | "paper-conference" => container.clone(),
        "book-chapter" | "chapter" | "reference-entry" | "entry" => {
            [container.as_str(), publisher.as_str()]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        }
        _ if !container.is_empty() && !crossref => container.clone(),
        _ => publisher.clone(),
    };
    let mut title = clean(&text("title"));
    let subtitle = clean(&text("subtitle"));
    if !subtitle.is_empty() && !title.contains(&subtitle) {
        title = format!("{title}: {subtitle}");
    }
    let open = m
        .get("license")
        .map(Json::items)
        .unwrap_or_default()
        .iter()
        .any(|l| {
            l.get("URL")
                .and_then(Json::str)
                .is_some_and(|u| u.contains("creativecommons.org"))
        });
    let doi = text("DOI");
    Source {
        authors: names.join(", "),
        title,
        venue: clean(&venue),
        year,
        open: if open { doi_url(&doi) } else { String::new() },
        ..Source::default()
    }
}

/// `E. A. J. Marcatili` from a person's given and family names, or their name as one.
fn person(p: &Json) -> Option<String> {
    let family = p.get("family").and_then(Json::str).map(str::trim);
    let given = p.get("given").and_then(Json::str).unwrap_or("").trim();
    match family {
        Some(f) if !f.is_empty() => {
            let initials: Vec<String> = given
                .split_whitespace()
                .map(|part| {
                    part.split('-')
                        .filter_map(|w| w.chars().next())
                        .map(|c| format!("{}.", c.to_uppercase()))
                        .collect::<Vec<_>>()
                        .join("-")
                })
                .collect();
            Some(if initials.is_empty() {
                f.to_string()
            } else {
                format!("{} {f}", initials.join(" "))
            })
        }
        _ => p
            .get("name")
            .or_else(|| p.get("literal"))
            .and_then(Json::str)
            .map(|s| s.trim().to_string()),
    }
}

/// Text without the markup Crossref's titles may hold (`<i>`, `<sub>`, `&amp;`) and with its
/// spaces collapsed.
fn clean(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let tag_len = tail.find('>').map(|e| e + 1);
        let is_tag = tag_len.is_some_and(|e| {
            let inner = tail[1..e - 1].trim_start_matches('/');
            inner
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
                && !inner.contains('<')
        });
        if let (true, Some(e)) = (is_tag, tag_len) {
            rest = &tail[e..];
        } else {
            out.push('<');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// JSON, read as much as [`from_csl`] needs: numbers are kept as written.
#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn parse(s: &str) -> Option<Json> {
        let mut p = JsonReader {
            s: s.as_bytes(),
            i: 0,
        };
        let v = p.value()?;
        p.space();
        (p.i == p.s.len()).then_some(v)
    }

    fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(v) => v.iter().find(|(name, _)| name == k).map(|(_, j)| j),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    fn items(&self) -> &[Json] {
        match self {
            Json::Arr(v) => v,
            _ => &[],
        }
    }
}

struct JsonReader<'a> {
    s: &'a [u8],
    i: usize,
}

impl JsonReader<'_> {
    fn space(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Option<Json> {
        self.space();
        match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut v = Vec::new();
                self.space();
                if self.eat("}") {
                    return Some(Json::Obj(v));
                }
                loop {
                    self.space();
                    let Json::Str(k) = self.string()? else {
                        return None;
                    };
                    self.space();
                    if !self.eat(":") {
                        return None;
                    }
                    v.push((k, self.value()?));
                    self.space();
                    if self.eat("}") {
                        return Some(Json::Obj(v));
                    }
                    if !self.eat(",") {
                        return None;
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut v = Vec::new();
                self.space();
                if self.eat("]") {
                    return Some(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.space();
                    if self.eat("]") {
                        return Some(Json::Arr(v));
                    }
                    if !self.eat(",") {
                        return None;
                    }
                }
            }
            b'"' => self.string(),
            b't' => self.eat("true").then_some(Json::Bool(true)),
            b'f' => self.eat("false").then_some(Json::Bool(false)),
            b'n' => self.eat("null").then_some(Json::Null),
            _ => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|b| b.is_ascii_digit() || b"+-.eE".contains(b))
                {
                    self.i += 1;
                }
                (self.i > start).then(|| {
                    Json::Num(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
                })
            }
        }
    }

    fn string(&mut self) -> Option<Json> {
        if !self.eat("\"") {
            return None;
        }
        let mut out: Vec<u8> = Vec::new();
        loop {
            let b = *self.s.get(self.i)?;
            self.i += 1;
            match b {
                b'"' => return String::from_utf8(out).ok().map(Json::Str),
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    let c = match e {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) && self.eat("\\u") {
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.checked_sub(0xDC00)?)
                            } else {
                                hi
                            };
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        other => other as char,
                    };
                    let mut buf = [0; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                b => out.push(b),
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
        self.i += 4;
        u32::from_str_radix(h, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<(String, usize, Form)> {
        let (c, p) = scan("a.md", text);
        assert!(p.is_empty(), "{p:?}");
        c.into_iter().map(|c| (c.written, c.line, c.form)).collect()
    }

    #[test]
    fn every_form_is_found_with_its_line() {
        let text = "intro\n\
             Marcatili (1969), doi:10.1002/j.1538-7305.1969.tb01166.x.\n\
             \x20   doi: 10.1364/OE.21.001310\n\
             doi: \"10.1063/1.363679\",\n\
             [Zhang](https://doi.org/10.1364/OL.38.002810)\n\
             \x20       \"10.1016/j.mee.2015.02.042\",\n\
             see ref:abramowitz-stegun-1964 and ref: yariv-yeh-2007.\n";
        assert_eq!(
            ids(text),
            vec![
                ("10.1002/j.1538-7305.1969.tb01166.x".into(), 2, Form::Prefix),
                ("10.1364/OE.21.001310".into(), 3, Form::Prefix),
                ("10.1063/1.363679".into(), 4, Form::Prefix),
                ("10.1364/OL.38.002810".into(), 5, Form::Link),
                ("10.1016/j.mee.2015.02.042".into(), 6, Form::Literal),
                ("abramowitz-stegun-1964".into(), 7, Form::Key),
                ("yariv-yeh-2007".into(), 7, Form::Key),
            ]
        );
    }

    #[test]
    fn a_link_with_its_doi_as_text_is_one_citation() {
        let text = "[doi:10.1364/OE.21.001310](https://doi.org/10.1364/OE.21.001310) and \
                    [10.1364/OL.38.002810](https://doi.org/10.1364/OL.38.002810)";
        assert_eq!(
            ids(text),
            vec![
                ("10.1364/OE.21.001310".into(), 1, Form::Prefix),
                ("10.1364/OL.38.002810".into(), 1, Form::Link),
            ]
        );
    }

    #[test]
    fn a_doi_ends_where_the_sentence_resumes() {
        assert_eq!(
            doi("10.1016/0021-9991(94)90153-8), and"),
            Some("10.1016/0021-9991(94)90153-8")
        );
        assert_eq!(
            doi("10.1002/(SICI)1098-2760(19970820)15:6<363::AID-MOP8>3.0.CO;2-C."),
            Some("10.1002/(SICI)1098-2760(19970820)15:6<363::AID-MOP8>3.0.CO;2-C")
        );
        assert_eq!(
            doi("10.1023/A:1016545417478;"),
            Some("10.1023/A:1016545417478")
        );
        assert_eq!(doi("10.1364/OE.21.001310`"), Some("10.1364/OE.21.001310"));
        assert_eq!(doi("10.1364/OE.21.001310>"), Some("10.1364/OE.21.001310"));
        assert_eq!(doi("10.123/x"), None);
        assert_eq!(doi("10.1364"), None);
        assert_eq!(doi("10.1364/"), None);
        assert_eq!(
            ids("<https://doi.org/10.1002%2F%28SICI%29x>")[0].0,
            "10.1002/(SICI)x"
        );
    }

    #[test]
    fn a_doi_compares_without_case() {
        let (c, _) = scan("a.md", "doi:10.1364/OE.21.001310\ndoi:10.1364/oe.21.001310");
        assert_eq!(c[0].id, c[1].id);
    }

    #[test]
    fn malformed_and_bare_dois_are_problems() {
        let (_, p) = scan(
            "a.rs",
            "// doi:10.13/x\n// Zenodo 10.5281/zenodo.10246032\n",
        );
        assert_eq!(p.len(), 2, "{p:?}");
        assert_eq!(p[0].place, "a.rs:1");
        assert!(p[0].message.contains("malformed"));
        assert_eq!(p[1].place, "a.rs:2");
        assert!(p[1].message.contains("doi:10.5281/zenodo.10246032"));
    }

    #[test]
    fn what_isnt_a_citation_is_skipped() {
        let text = "doi: \"\"\npub doi: String,\nhref: x\nref: string\nsections 10.17 and 10.41\n\
                    https://example.org/pdf/10.1002/adfm.202002447\nlet x = \"10.5\";\n\
                    xdoi:10.1364/OE.21.001310\n";
        let (c, p) = scan("a.rs", text);
        assert!(c.is_empty(), "{c:?}");
        assert!(p.is_empty(), "{p:?}");
    }

    #[test]
    fn keys_are_words_then_a_year() {
        assert_eq!(key("calma-1987)"), Some("calma-1987"));
        assert_eq!(key("smith-2010b,"), Some("smith-2010b"));
        assert_eq!(key("string"), None);
        assert_eq!(key("abc-123"), None);
        assert_eq!(key("-1987"), None);
    }

    #[test]
    fn a_validation_report_row_names_its_case() {
        let (c, _) = scan(
            "docs/validation.md",
            "| `mode/slab-te-book` | x | doi:10.1017/CBO9781316084168 |",
        );
        assert_eq!(c[0].case.as_deref(), Some("mode/slab-te-book"));
    }

    fn sources() -> Vec<Source> {
        let (s, p) = parse(
            "# comment\n\
             - doi: 10.1364/OE.21.001310\n\
             \x20 authors: \"Y. Zhang, S. Yang, A. E.-J. Lim\"\n\
             \x20 title: \"A compact and low loss Y-junction: \\\"quoted\\\"\"\n\
             \x20 venue: \"Opt. Express 21, 1310\"\n\
             \x20 year: 2013\n\
             \x20 open: https://doi.org/10.1364/OE.21.001310\n\
             \n\
             - key: abramowitz-stegun-1964\n\
             \x20 authors: \"M. Abramowitz, I. A. Stegun\"\n\
             \x20 title: Handbook of Mathematical Functions\n\
             \x20 venue: \"National Bureau of Standards\"\n\
             \x20 year: 1964 # the first printing\n",
        );
        assert!(p.is_empty(), "{p:?}");
        s
    }

    #[test]
    fn the_data_file_reads_back_as_written() {
        let s = sources();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].title, "A compact and low loss Y-junction: \"quoted\"");
        assert_eq!(s[0].line, 2);
        assert_eq!(s[1].year, "1964");
        assert_eq!(s[0].label(), "Zhang et al. 2013");
        assert_eq!(s[1].label(), "Abramowitz and Stegun 1964");
        let again = parse(&write_data(&s)).0;
        let strip = |v: &[Source]| -> Vec<Source> {
            let mut v: Vec<Source> = v
                .iter()
                .map(|s| Source {
                    line: 0,
                    ..s.clone()
                })
                .collect();
            v.sort_by_key(Source::id);
            v
        };
        assert_eq!(strip(&again), strip(&s));
        let (_, p) = parse("- doi: 10.1364/x\n  colour: red\n");
        assert!(p[0].message.contains("isn't a field"));
    }

    fn cites() -> Vec<Citation> {
        let mut c = scan(
            "docs/methods/eme.md",
            "---\npapers:\n    doi: 10.1364/OE.21.001310\n",
        )
        .0;
        c.extend(
            scan(
                "src/mode/eme.rs",
                "// doi:10.1364/OE.21.001310\n\n// again doi:10.1364/oe.21.001310\n",
            )
            .0,
        );
        c.extend(
            scan(
                "docs/methods/geometry.md",
                "    ref: abramowitz-stegun-1964\n",
            )
            .0,
        );
        c
    }

    #[test]
    fn the_pages_list_each_source_with_its_places() {
        let [p, x] = pages(&sources(), &cites());
        assert!(p.contains("#### Zhang et al. 2013\n"), "{p}");
        assert!(
            p.contains(
                "- Methods pages: [eme.md:3](methods/eme.md?plain=1#L3)\n\
                 - Code: [src/mode/eme.rs:1](../src/mode/eme.rs#L1), [3](../src/mode/eme.rs#L3)\n"
            ),
            "{p}"
        );
        assert!(p.contains(
            "[doi:10.1364/OE.21.001310](https://doi.org/10.1364/OE.21.001310), open access."
        ));
        // A before Z, and Č under C
        assert!(p.find("### A").unwrap() < p.find("### Z").unwrap());
        assert_eq!(folded("Čtyroký"), "ctyroky");
        assert!(
            x.contains(
                "- [`geometry.md`](methods/geometry.md): \
                 [Abramowitz and Stegun 1964](references.md#abramowitz-and-stegun-1964)"
            ),
            "{x}"
        );
    }

    #[test]
    fn a_stale_page_fails_and_a_current_one_passes() {
        let (s, c) = (sources(), cites());
        let [p, x] = pages(&s, &c);
        assert!(problems(&s, &c, Some([&p, &x])).is_empty());
        let crlf = p.replace('\n', "\r\n");
        assert!(problems(&s, &c, Some([&crlf, &x])).is_empty());
        // a line moved: the page links to the old one, the index (by file) is the same
        let mut moved = c.clone();
        moved[1].line += 1;
        let found = problems(&s, &moved, Some([&p, &x]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].place, PAGE);
        // a source cited from one more file: both are stale
        let mut more = c.clone();
        more.extend(scan("src/x.rs", "// doi:10.1364/OE.21.001310").0);
        assert_eq!(problems(&s, &more, Some([&p, &x])).len(), 2);
    }

    #[test]
    fn a_missing_or_uncited_entry_fails() {
        let s = sources();
        let mut c = cites();
        c.retain(|c| matches!(c.id, Id::Doi(_)));
        c.extend(scan("a.md", "doi:10.1063/1.363679").0);
        let p: Vec<String> = problems(&s, &c, None)
            .iter()
            .map(|p| p.message.clone())
            .collect();
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(
            p.iter()
                .any(|m| m.starts_with("abramowitz-stegun-1964: nothing cites"))
        );
        assert!(
            p.iter()
                .any(|m| m.starts_with("10.1063/1.363679: no entry"))
        );
    }

    #[test]
    fn crossref_metadata_becomes_an_entry() {
        let json = r#"{"status":"ok","message":{"DOI":"10.1364/oe.21.001310","type":"journal-article",
            "title":["A compact and low loss Y-junction for submicron silicon waveguide"],
            "author":[{"given":"Yi","family":"Zhang"},{"given":"Ann-Marie","family":"Lim"},{"name":"Some Group"}],
            "short-container-title":["Opt. Express"],"volume":"21","page":"1310-1316",
            "published-online":{"date-parts":[[2013,1,14]]},"published-print":{"date-parts":[[2013,1,28]]},
            "license":[{"URL":"https://creativecommons.org/licenses/by/4.0/"}]}}"#;
        let j = Json::parse(json).unwrap();
        let s = from_csl(j.get("message").unwrap(), true);
        assert_eq!(s.authors, "Y. Zhang, A.-M. Lim, Some Group");
        assert_eq!(s.venue, "Opt. Express 21, 1310");
        assert_eq!(s.year, "2013");
        assert_eq!(s.open, "https://doi.org/10.1364/oe.21.001310");
        assert_eq!(
            clean("Si<sub>3</sub>N<sub>4</sub> &amp;  <i>x</i> < 2"),
            "Si3N4 & x < 2"
        );
        assert_eq!(
            Json::parse(r#"["é😀", 1.5e3, null]"#),
            Some(Json::Arr(vec![
                Json::Str("é😀".into()),
                Json::Num("1.5e3".into()),
                Json::Null
            ]))
        );
    }

    /// The repository's check, as CI runs it: the page is current, every cited source has an
    /// entry and every entry is cited, and every DOI is well formed and written in a form.
    #[test]
    fn the_repository_is_consistent() {
        let root = root().expect("the repository's root");
        let problems = check(&root).expect("the scan");
        assert!(
            problems.is_empty(),
            "{}",
            problems
                .iter()
                .map(Problem::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
