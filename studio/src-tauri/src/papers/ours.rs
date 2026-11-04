//! The papers photonoxide cites, as a compact index the program carries: built at build time
//! (build.rs includes this file) from docs/references.md, the index of every source and every
//! place that cites it, so a paper's page can say which methods pages, lessons, examples, jobs
//! and validation cases cite it. Self-contained, as a build script needs.

/// The places kept by name; the others ("Code", "Other documents") are kept as a count.
const NAMED: [&str; 5] = [
    "Methods pages",
    "Academy lessons",
    "Examples",
    "Jobs and circuits",
    "Validation cases",
];

/// docs/references.md as lines of `doi<TAB>Place=name,name;Place=count`, one per source with a
/// DOI, the DOI in lower case.
pub fn index(markdown: &str) -> String {
    let mut out = String::new();
    let mut doi: Option<String> = None;
    let mut places: Vec<String> = Vec::new();
    let mut flush = |doi: &mut Option<String>, places: &mut Vec<String>| {
        if let Some(d) = doi.take()
            && !places.is_empty()
        {
            out.push_str(&d);
            out.push('\t');
            out.push_str(&places.join(";"));
            out.push('\n');
        }
        places.clear();
    };
    for line in markdown.lines() {
        if line.starts_with("#### ") || line.starts_with("## ") || line.starts_with("### ") {
            flush(&mut doi, &mut places);
            continue;
        }
        if doi.is_none()
            && let Some(k) = line.find("[doi:")
        {
            let rest = &line[k + 5..];
            if let Some(end) = rest.find(']') {
                doi = Some(rest[..end].trim().to_lowercase());
            }
            continue;
        }
        let Some(item) = line.strip_prefix("- ") else {
            continue;
        };
        let Some((place, links)) = item.split_once(": ") else {
            continue;
        };
        let mut names: Vec<String> = Vec::new();
        // each link's text ends a part, and its target begins the next: the last part is no text
        let parts: Vec<&str> = links.split("](").collect();
        for part in &parts[..parts.len().saturating_sub(1)] {
            let Some(open) = part.rfind('[') else {
                continue;
            };
            let text = part[open + 1..].trim_matches('`');
            // `devices.md:42` is the file, `979` a further line in the same one
            let name = text.split(':').next().unwrap_or(text);
            if name.is_empty() || name.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let name = name.trim_start_matches("jobs/").to_owned();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        if names.is_empty() {
            continue;
        }
        let place = place.trim();
        if NAMED.contains(&place) {
            places.push(format!("{place}={}", names.join(",")));
        } else {
            places.push(format!("{place}={}", names.len()));
        }
    }
    flush(&mut doi, &mut places);
    out
}
