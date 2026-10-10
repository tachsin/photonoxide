//! The Academy's lessons (academy/*.md, built into the program): front matter read, the body cut
//! into sections at its `##` headings, each with its depth (intuition, theory, research), and each
//! section into blocks: Markdown, diagrams of the device (crate::diagrams), charts
//! (crate::charts), examples, validation cases, the timeline of the lesson's papers, and answers
//! to reveal. The format is academy/README.md's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::charts::{self, ChartSpec};
use crate::diagrams::{self, DiagramSpec};

/// Each lesson's file and text, as the program was built with them.
macro_rules! lessons {
    ($($file:literal),* $(,)?) => {
        const LESSONS: &[(&str, &str)] = &[$(($file, include_str!(concat!("../../../academy/", $file)))),*];
    };
}

lessons!("ring-resonator.md", "bragg-gratings.md");

/// How far into a subject a lesson goes.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Introductory,
    Intermediate,
    Advanced,
}

/// A paper's place in a lesson's history.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Where the idea began.
    Origin,
    /// A step that changed what was possible.
    Milestone,
    /// A survey of the field.
    Review,
    /// Today's state of the art.
    Current,
}

/// A paper a lesson cites, for its timeline.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Paper {
    /// The citation: authors, journal, volume, page (year).
    pub cite: String,
    pub title: String,
    /// Its DOI, checked on Crossref when the lesson was written.
    pub doi: String,
    pub year: u32,
    pub role: Role,
    /// What it did, in a sentence of the lesson's own words.
    pub note: String,
}

/// A lesson's front matter.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Front {
    pub title: String,
    pub summary: String,
    /// The topic it is listed under, e.g. "Rings".
    pub topic: String,
    pub level: Level,
    /// About how long it takes to read, minutes.
    pub minutes: u32,
    /// Lessons to read first, by file name without `.md`.
    #[serde(default)]
    pub prerequisites: Vec<String>,
    /// Examples it uses (examples/<name>.rs).
    #[serde(default)]
    pub examples: Vec<String>,
    /// Jobs it opens in the builder (jobs/<file>).
    #[serde(default)]
    pub jobs: Vec<String>,
    /// Circuits it opens on the chip (circuits/<file>).
    #[serde(default)]
    pub circuits: Vec<String>,
    /// Method write-ups it explains (docs/methods/<file>).
    #[serde(default)]
    pub methods: Vec<String>,
    /// Validation cases it quotes, by id.
    #[serde(default)]
    pub validation: Vec<String>,
    /// Charts it shows, by id (crate::charts).
    #[serde(default)]
    pub charts: Vec<String>,
    pub papers: Vec<Paper>,
}

/// How deep a section goes: intuition is always open, theory and research open on demand.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Depth {
    Intuition,
    Theory,
    Research,
}

/// A piece of a section.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Block {
    /// Markdown with TeX math.
    Text { markdown: String },
    /// `::diagram <id>`: the device drawn, a labelled schematic and, where a job builds it, a 3D
    /// view (crate::diagrams).
    Diagram { diagram: String },
    /// `::chart <id>{key=value, ...}`: a chart, its parameters set as given and the rest at
    /// their defaults.
    Chart {
        chart: String,
        values: BTreeMap<String, f64>,
    },
    /// `::example <name>`: an example, what it printed when the release was checked, and a
    /// button to run it.
    Example { name: String },
    /// `::validation <id> <id> ...`: validation cases as the published report has them.
    Validation { cases: Vec<String> },
    /// `::timeline`: the lesson's papers by year.
    Timeline,
    /// `:::answer` ... `:::`: Markdown hidden until asked for.
    Answer { markdown: String },
}

/// A section: its heading, its depth, and its blocks.
#[derive(Serialize, Clone, Debug)]
pub struct Section {
    pub title: String,
    /// Its anchor, from the title (a part's prefixed by its section's).
    pub id: String,
    pub depth: Depth,
    /// 2 for a `##` section, 3 for a part of one: a `###` heading with a depth, which runs to
    /// the next part or section.
    pub level: u8,
    pub blocks: Vec<Block>,
}

/// A validation case's row in the published report (docs/validation.md).
#[derive(Serialize, Clone, Debug)]
pub struct CaseRow {
    pub id: String,
    pub tier: String,
    pub what: String,
    pub against: String,
    pub measured: String,
    pub expected: String,
    pub tolerance: String,
    pub pass: bool,
}

/// A lesson, ready to show.
#[derive(Serialize, Clone, Debug)]
pub struct Lesson {
    /// Its file name without `.md`, e.g. `ring-resonator`.
    pub id: String,
    #[serde(flatten)]
    pub front: Front,
    pub sections: Vec<Section>,
    /// Its validation cases' rows from the published report.
    pub cases: Vec<CaseRow>,
}

/// The lessons, and the diagrams and charts they can show.
#[derive(Serialize)]
pub struct Academy {
    pub lessons: Vec<Lesson>,
    pub diagrams: Vec<DiagramSpec>,
    pub charts: Vec<ChartSpec>,
}

/// Every lesson, parsed, and every diagram's and chart's spec. A lesson that doesn't parse is left out here;
/// the tests refuse it.
#[tauri::command]
pub fn academy() -> Academy {
    Academy {
        lessons: LESSONS
            .iter()
            .filter_map(|(file, text)| parse(file, text).ok())
            .collect(),
        diagrams: diagrams::specs(),
        charts: charts::specs(),
    }
}

/// A chart's curves and figures at `values` (each in its parameter's unit; those left out at
/// their defaults).
#[tauri::command]
pub fn academy_chart(
    chart: String,
    values: BTreeMap<String, f64>,
) -> Result<charts::ChartData, String> {
    charts::evaluate(&chart, &values)
}

/// The anchor for a heading: lower case, words joined by hyphens.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// A lesson's file as a Lesson: its front matter (YAML between `---` lines), then its body.
pub fn parse(file: &str, text: &str) -> Result<Lesson, String> {
    let text = text.replace("\r\n", "\n");
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| format!("{file}: no front matter (a --- line first)"))?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| format!("{file}: the front matter has no closing --- line"))?;
    let front: Front =
        serde_saphyr::from_str(&rest[..end]).map_err(|e| format!("{file}: front matter: {e}"))?;
    let body = &rest[end + 5..];
    let id = file.trim_end_matches(".md").to_owned();
    let sections = sections(file, body)?;
    let cases = front
        .validation
        .iter()
        .filter_map(|id| case_row(id))
        .collect();
    Ok(Lesson {
        id,
        front,
        sections,
        cases,
    })
}

/// The body cut into sections and blocks; `file` names it in errors.
fn sections(file: &str, body: &str) -> Result<Vec<Section>, String> {
    let mut out: Vec<Section> = Vec::new();
    let mut current = Section {
        title: String::new(),
        id: String::new(),
        depth: Depth::Intuition,
        level: 2,
        blocks: Vec::new(),
    };
    // the `##` section the next part belongs to
    let mut parent: Option<String> = None;
    let mut text = String::new();
    let mut answer: Option<String> = None;
    let mut code = false;
    let flush = |text: &mut String, blocks: &mut Vec<Block>| {
        if !text.trim().is_empty() {
            blocks.push(Block::Text {
                markdown: text.trim_matches('\n').to_owned(),
            });
        }
        text.clear();
    };
    for (k, line) in body.lines().enumerate() {
        let at = || format!("{file}, line {} of the body", k + 1);
        if line.starts_with("```") {
            code = !code;
        }
        let into = answer.as_mut().unwrap_or(&mut text);
        if code || line.starts_with("```") {
            into.push_str(line);
            into.push('\n');
            continue;
        }
        let trimmed = line.trim();
        if trimmed == ":::" {
            let Some(markdown) = answer.take() else {
                return Err(format!("{}: ::: closes no answer", at()));
            };
            current.blocks.push(Block::Answer {
                markdown: markdown.trim_matches('\n').to_owned(),
            });
            continue;
        }
        if trimmed == ":::answer" {
            if answer.is_some() {
                return Err(format!("{}: an answer inside an answer", at()));
            }
            flush(&mut text, &mut current.blocks);
            answer = Some(String::new());
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            if answer.is_some() {
                return Err(format!("{}: a heading inside an answer", at()));
            }
            flush(&mut text, &mut current.blocks);
            if !current.title.is_empty() || !current.blocks.is_empty() {
                out.push(current);
            }
            let (title, depth) = heading_depth(heading).map_err(|e| format!("{}: {e}", at()))?;
            parent = Some(slug(&title));
            current = Section {
                id: slug(&title),
                title,
                depth,
                level: 2,
                blocks: Vec::new(),
            };
            continue;
        }
        // a part: `### Title {depth}`; a `###` heading without a depth stays in the text
        if let Some(heading) = line
            .strip_prefix("### ")
            .filter(|h| h.trim_end().ends_with('}') && h.contains(" {"))
        {
            if answer.is_some() {
                return Err(format!("{}: a heading inside an answer", at()));
            }
            let Some(section) = &parent else {
                return Err(format!(
                    "{}: a ### part belongs to a ## section, and none has begun",
                    at()
                ));
            };
            flush(&mut text, &mut current.blocks);
            if !current.title.is_empty() || !current.blocks.is_empty() {
                out.push(current);
            }
            let (title, depth) = heading_depth(heading).map_err(|e| format!("{}: {e}", at()))?;
            current = Section {
                id: format!("{section}--{}", slug(&title)),
                title,
                depth,
                level: 3,
                blocks: Vec::new(),
            };
            continue;
        }
        if line.starts_with("# ") {
            return Err(format!(
                "{}: the title is the front matter's; sections are ## headings",
                at()
            ));
        }
        if let Some(directive) = trimmed.strip_prefix("::") {
            if answer.is_some() {
                return Err(format!("{}: a block inside an answer", at()));
            }
            flush(&mut text, &mut current.blocks);
            current
                .blocks
                .push(block(directive).map_err(|e| format!("{}: {e}", at()))?);
            continue;
        }
        let into = answer.as_mut().unwrap_or(&mut text);
        into.push_str(line);
        into.push('\n');
    }
    if answer.is_some() {
        return Err(format!("{file}: an answer is never closed (:::)"));
    }
    if code {
        return Err(format!("{file}: a code block is never closed"));
    }
    flush(&mut text, &mut current.blocks);
    if !current.title.is_empty() || !current.blocks.is_empty() {
        out.push(current);
    }
    Ok(out)
}

/// `The physics {theory}` as its title and depth; no tag is intuition.
fn heading_depth(heading: &str) -> Result<(String, Depth), String> {
    let heading = heading.trim();
    let Some(open) = heading.rfind(" {").filter(|_| heading.ends_with('}')) else {
        return Ok((heading.to_owned(), Depth::Intuition));
    };
    let depth = match &heading[open + 2..heading.len() - 1] {
        "intuition" => Depth::Intuition,
        "theory" => Depth::Theory,
        "research" => Depth::Research,
        other => {
            return Err(format!(
                "{other:?} isn't a depth: intuition, theory or research"
            ));
        }
    };
    Ok((heading[..open].trim().to_owned(), depth))
}

/// A block line, without its leading `::`.
fn block(directive: &str) -> Result<Block, String> {
    let (name, rest) = directive.split_once(' ').unwrap_or((directive, ""));
    let rest = rest.trim();
    match name {
        "chart" => {
            let (id, values) = match rest.split_once('{') {
                Some((id, values)) => (
                    id.trim(),
                    values
                        .strip_suffix('}')
                        .ok_or_else(|| format!("::chart {rest}: no closing }}"))?,
                ),
                None => (rest, ""),
            };
            let spec = charts::spec(id).ok_or_else(|| format!("::chart: no chart {id:?}"))?;
            Ok(Block::Chart {
                chart: id.to_owned(),
                values: charts::parse_values(&spec, values)?,
            })
        }
        "diagram" => {
            let d =
                diagrams::spec(rest).ok_or_else(|| format!("::diagram: no diagram {rest:?}"))?;
            Ok(Block::Diagram {
                diagram: d.id.to_owned(),
            })
        }
        "example" if !rest.is_empty() && !rest.contains(' ') => Ok(Block::Example {
            name: rest.to_owned(),
        }),
        "validation" if !rest.is_empty() => Ok(Block::Validation {
            cases: rest
                .split([' ', ','])
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        }),
        "timeline" if rest.is_empty() => Ok(Block::Timeline),
        _ => Err(format!(
            "::{directive} isn't a block: ::diagram <id>, ::chart <id>{{...}}, ::example <name>, ::validation <id> ..., ::timeline"
        )),
    }
}

/// The published report's row for the case `id`.
fn case_row(id: &str) -> Option<CaseRow> {
    let report = include_str!("../../../docs/validation.md");
    let prefix = format!("| `{id}` |");
    let line = report.lines().find(|l| l.starts_with(&prefix))?;
    let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
    if cells.len() != 8 {
        return None;
    }
    Some(CaseRow {
        id: id.to_owned(),
        tier: cells[1].to_owned(),
        what: cells[2].to_owned(),
        against: cells[3].to_owned(),
        measured: cells[4].to_owned(),
        expected: cells[5].to_owned(),
        tolerance: cells[6].to_owned(),
        pass: cells[7] == "pass",
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    fn all() -> Vec<Lesson> {
        LESSONS
            .iter()
            .map(|(file, text)| parse(file, text).unwrap_or_else(|e| panic!("{e}")))
            .collect()
    }

    /// The Markdown a lesson holds, its answers included.
    fn markdown(lesson: &Lesson) -> String {
        let mut out = String::new();
        for s in &lesson.sections {
            out.push_str(&s.title);
            out.push('\n');
            for b in &s.blocks {
                if let Block::Text { markdown } | Block::Answer { markdown } = b {
                    out.push_str(markdown);
                    out.push('\n');
                }
            }
        }
        out
    }

    /// The targets of a Markdown text's links, `[text](target)`.
    fn links(text: &str) -> Vec<String> {
        text.match_indices("](")
            .filter_map(|(k, _)| {
                let tail = &text[k + 2..];
                tail.find(')').map(|end| tail[..end].to_owned())
            })
            .collect()
    }

    #[test]
    fn every_lesson_is_built_in() {
        let mut files: Vec<String> = std::fs::read_dir(root().join("academy"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|f| f.ends_with(".md") && f != "README.md")
            .collect();
        files.sort();
        let mut built: Vec<String> = LESSONS.iter().map(|(f, _)| (*f).to_owned()).collect();
        built.sort();
        assert_eq!(built, files, "add the new lesson to lessons!()");
        assert_eq!(academy().lessons.len(), LESSONS.len());
    }

    #[test]
    fn every_lesson_has_its_parts() {
        for l in all() {
            let f = &l.front;
            assert!(
                !f.title.is_empty() && !f.summary.is_empty() && !f.topic.is_empty(),
                "{}",
                l.id
            );
            assert!(f.minutes > 0, "{}", l.id);
            let depths: Vec<Depth> = l.sections.iter().map(|s| s.depth).collect();
            for d in [Depth::Intuition, Depth::Theory, Depth::Research] {
                assert!(depths.contains(&d), "{}: no {d:?} section", l.id);
            }
            // the opening, before the first heading, is the only section without a title
            assert!(l.sections[0].depth == Depth::Intuition, "{}", l.id);
            for (k, s) in l.sections.iter().enumerate() {
                assert!(
                    (k == 0 || !s.title.is_empty()) && !s.blocks.is_empty(),
                    "{}: an empty section",
                    l.id
                );
            }
            let ids: Vec<&str> = l.sections.iter().map(|s| s.id.as_str()).collect();
            for (k, id) in ids.iter().enumerate() {
                assert!(!ids[..k].contains(id), "{}: two sections named {id}", l.id);
            }
            let blocks = || l.sections.iter().flat_map(|s| &s.blocks);
            assert!(
                blocks().any(|b| *b == Block::Timeline),
                "{}: no ::timeline",
                l.id
            );
            // the device drawn first: a diagram before any chart
            let diagram = blocks().position(|b| matches!(b, Block::Diagram { .. }));
            let chart = blocks().position(|b| matches!(b, Block::Chart { .. }));
            assert!(diagram.is_some(), "{}: no ::diagram", l.id);
            assert!(
                chart.is_none_or(|c| diagram < Some(c)),
                "{}: a chart before the lesson's diagram",
                l.id
            );
        }
    }

    /// Its examples, jobs, circuits, method write-ups, validation cases, diagrams, charts and
    /// prerequisites exist, and its blocks name only what its front matter lists; a diagram's
    /// 3D view is of one of its jobs.
    #[test]
    fn everything_a_lesson_names_exists() {
        let lessons = all();
        let cases: Vec<&str> = photonoxide::validation::cases()
            .iter()
            .map(|c| c.id)
            .collect();
        for l in &lessons {
            let f = &l.front;
            for e in &f.examples {
                assert!(
                    root().join(format!("examples/{e}.rs")).is_file(),
                    "{}: no example {e}",
                    l.id
                );
            }
            for j in &f.jobs {
                assert!(
                    crate::examples::jobs().iter().any(|x| &x.file == j),
                    "{}: no built-in job {j}",
                    l.id
                );
            }
            for c in &f.circuits {
                assert!(
                    root().join("circuits").join(c).is_file(),
                    "{}: no circuit {c}",
                    l.id
                );
            }
            for m in &f.methods {
                assert!(
                    root().join("docs/methods").join(m).is_file(),
                    "{}: no method {m}",
                    l.id
                );
                assert!(
                    crate::studio::METHOD_DOCS.iter().any(|(file, _)| file == m),
                    "{}: {m} isn't built into the program",
                    l.id
                );
            }
            for c in &f.validation {
                assert!(
                    cases.contains(&c.as_str()),
                    "{}: no validation case {c}",
                    l.id
                );
                assert!(
                    l.cases.iter().any(|r| &r.id == c),
                    "{}: {c} isn't in docs/validation.md",
                    l.id
                );
            }
            for c in &f.charts {
                assert!(charts::spec(c).is_some(), "{}: no chart {c}", l.id);
            }
            for p in &f.prerequisites {
                assert!(
                    lessons.iter().any(|o| &o.id == p) && p != &l.id,
                    "{}: no lesson {p}",
                    l.id
                );
            }
            for b in l.sections.iter().flat_map(|s| &s.blocks) {
                match b {
                    Block::Chart { chart, values } => {
                        assert!(
                            f.charts.contains(chart),
                            "{}: {chart} isn't in its charts",
                            l.id
                        );
                        charts::evaluate(chart, values).unwrap_or_else(|e| panic!("{}: {e}", l.id));
                    }
                    Block::Diagram { diagram } => {
                        let d = diagrams::spec(diagram)
                            .unwrap_or_else(|| panic!("{}: no diagram {diagram}", l.id));
                        if let Some(job) = d.job {
                            assert!(
                                f.jobs.iter().any(|j| j == job),
                                "{}: the diagram {diagram} shows {job}, which isn't in its jobs",
                                l.id
                            );
                        }
                    }
                    Block::Example { name } => assert!(
                        f.examples.contains(name),
                        "{}: {name} isn't in its examples",
                        l.id
                    ),
                    Block::Validation { cases } => {
                        for c in cases {
                            assert!(
                                f.validation.contains(c),
                                "{}: {c} isn't in its validation",
                                l.id
                            );
                        }
                    }
                    _ => {}
                }
            }
            let used = |c: &String| {
                l.sections
                    .iter()
                    .flat_map(|s| &s.blocks)
                    .any(|b| matches!(b, Block::Chart { chart, .. } if chart == c))
            };
            assert!(
                f.charts.iter().all(used),
                "{}: a chart listed but never shown",
                l.id
            );
        }
    }

    /// Its papers have DOIs, each once, and every DOI its text links to is one of them; its
    /// relative links lead to files that exist.
    #[test]
    fn every_paper_has_a_doi_and_every_link_leads_somewhere() {
        for l in all() {
            let papers = &l.front.papers;
            assert!(papers.len() >= 3, "{}: a history needs papers", l.id);
            for (k, p) in papers.iter().enumerate() {
                assert!(
                    p.doi.starts_with("10.") && p.doi.contains('/') && !p.doi.contains(' '),
                    "{}: {:?}",
                    l.id,
                    p.doi
                );
                assert!(
                    papers[..k]
                        .iter()
                        .all(|o| o.doi.to_lowercase() != p.doi.to_lowercase()),
                    "{}: {} twice",
                    l.id,
                    p.doi
                );
                assert!((1800..=2100).contains(&p.year), "{}: {}", l.id, p.year);
                assert!(
                    !p.cite.is_empty() && !p.title.is_empty() && !p.note.is_empty(),
                    "{}: {}",
                    l.id,
                    p.doi
                );
            }
            for target in links(&markdown(&l)) {
                if let Some(doi) = target.strip_prefix("https://doi.org/") {
                    assert!(
                        papers.iter().any(|p| p.doi.eq_ignore_ascii_case(doi)),
                        "{}: the text cites {doi}, which its papers don't list",
                        l.id
                    );
                } else if target.starts_with("https://") {
                    continue;
                } else {
                    let path = target.split('#').next().unwrap_or("");
                    assert!(
                        root().join("academy").join(path).exists(),
                        "{}: a link to {target}, which doesn't exist",
                        l.id
                    );
                }
            }
        }
    }

    /// Display math ($$ on lines of their own) opens and closes, and inline math pairs its
    /// dollars; the window's build renders every formula with KaTeX and fails on an error
    /// (studio/vite.config.ts).
    #[test]
    fn math_is_delimited() {
        for l in all() {
            for s in &l.sections {
                for b in &s.blocks {
                    let (Block::Text { markdown } | Block::Answer { markdown }) = b else {
                        continue;
                    };
                    let mut display = false;
                    for line in markdown.lines() {
                        if line.trim() == "$$" {
                            display = !display;
                        } else if !display && !line.starts_with("    ") {
                            let dollars = line.matches('$').count() - line.matches("\\$").count();
                            assert!(
                                dollars % 2 == 0,
                                "{}, {}: unpaired $ in {line:?}",
                                l.id,
                                s.title
                            );
                        }
                    }
                    assert!(!display, "{}, {}: $$ never closed", l.id, s.title);
                }
            }
        }
    }

    #[test]
    fn the_format_is_read_as_documented() {
        let text = "---\ntitle: T\nsummary: S\ntopic: Rings\nlevel: introductory\nminutes: 5\ncharts: [ring-spectrum]\npapers:\n  - cite: C\n    title: P\n    doi: 10.1/x\n    year: 1997\n    role: origin\n    note: N\n---\nIntro $a$.\n::diagram ring\n\n## The physics {theory}\nText.\n::chart ring-spectrum{radius=20um, loss=2dB/cm}\n:::answer\nHidden.\n:::\n```\n::not a block\n```\n## History {research}\n::timeline\n::validation a/b c/d\n::example ring_q_factor\n";
        let l = parse("x.md", text).unwrap();
        assert_eq!(l.id, "x");
        assert_eq!(l.front.level, Level::Introductory);
        assert_eq!(l.sections.len(), 3);
        assert_eq!(l.sections[0].title, "");
        assert_eq!(
            l.sections[0].blocks[1],
            Block::Diagram {
                diagram: "ring".to_owned()
            }
        );
        assert_eq!(l.sections[1].title, "The physics");
        assert_eq!(l.sections[1].id, "the-physics");
        assert_eq!(l.sections[1].depth, Depth::Theory);
        let b = &l.sections[1].blocks;
        assert_eq!(
            b[0],
            Block::Text {
                markdown: "Text.".to_owned()
            }
        );
        assert_eq!(
            b[1],
            Block::Chart {
                chart: "ring-spectrum".to_owned(),
                values: [("loss".to_owned(), 2.0), ("radius".to_owned(), 20.0)].into(),
            }
        );
        assert_eq!(
            b[2],
            Block::Answer {
                markdown: "Hidden.".to_owned()
            }
        );
        assert_eq!(
            b[3],
            Block::Text {
                markdown: "```\n::not a block\n```".to_owned()
            }
        );
        assert_eq!(l.sections[2].depth, Depth::Research);
        assert_eq!(l.sections[2].blocks[0], Block::Timeline);
        assert_eq!(
            l.sections[2].blocks[1],
            Block::Validation {
                cases: vec!["a/b".to_owned(), "c/d".to_owned()]
            }
        );
        assert_eq!(
            l.sections[2].blocks[2],
            Block::Example {
                name: "ring_q_factor".to_owned()
            }
        );

        let broken = |body: &str| parse("x.md", &text.replace("::timeline\n", body)).is_err();
        assert!(broken("::chart no-such{}\n"));
        assert!(
            broken("::diagram no-such\n"),
            "a diagram that doesn't exist"
        );
        assert!(broken("::diagram\n"));
        assert!(
            broken("::chart ring-spectrum{radius=1mm}\n"),
            "out of its range"
        );
        assert!(broken("::chart ring-spectrum{colour=red}\n"));
        assert!(broken("::graph x\n"));
        assert!(broken(":::answer\nnever closed\n"));
        assert!(broken(":::\n"));
        assert!(broken("## Deep {deeper}\n"));
        assert!(broken("# A second title\n"));
        assert!(
            parse(
                "x.md",
                &text.replace("minutes: 5", "minutes: 5\ncolour: red")
            )
            .is_err(),
            "an unknown field"
        );
        assert!(parse("x.md", &text.replace("role: origin", "role: founding")).is_err());
    }

    /// A `###` heading with a depth is a part of its `##` section, at that depth, to the next
    /// part or section; one without stays in the text.
    #[test]
    fn parts_belong_to_their_section() {
        let front = "---\ntitle: T\nsummary: S\ntopic: Rings\nlevel: introductory\nminutes: 5\npapers: []\n---\n";
        let body = "Intro.\n## 1. Resonance\nWhat it does.\n### A heading in the text\nMore.\n### The derivation {theory}\nDerived.\n### Its history {research}\nOld.\n## 2. Width\nNext.\n";
        let l = parse("x.md", &format!("{front}{body}")).unwrap();
        let shape: Vec<(&str, &str, Depth, u8)> = l
            .sections
            .iter()
            .map(|s| (s.title.as_str(), s.id.as_str(), s.depth, s.level))
            .collect();
        assert_eq!(
            shape,
            [
                ("", "", Depth::Intuition, 2),
                ("1. Resonance", "1-resonance", Depth::Intuition, 2),
                (
                    "The derivation",
                    "1-resonance--the-derivation",
                    Depth::Theory,
                    3
                ),
                (
                    "Its history",
                    "1-resonance--its-history",
                    Depth::Research,
                    3
                ),
                ("2. Width", "2-width", Depth::Intuition, 2),
            ]
        );
        assert_eq!(
            l.sections[1].blocks,
            [Block::Text {
                markdown: "What it does.\n### A heading in the text\nMore.".to_owned()
            }]
        );
        assert!(
            parse(
                "x.md",
                &format!("{front}Intro.\n### Early {{theory}}\nNo.\n")
            )
            .is_err(),
            "a part before any section"
        );
        assert!(parse("x.md", &format!("{front}## A\n### B {{deeper}}\nNo.\n")).is_err());
    }

    #[test]
    fn case_rows_come_from_the_report() {
        let row = case_row("mode/multilayer-bragg").unwrap();
        assert_eq!(row.tier, "analytic");
        assert_eq!(row.measured, "0.999258");
        assert!(row.pass);
        assert!(case_row("no/such-case").is_none());
    }

    #[test]
    fn headings_make_anchors() {
        assert_eq!(
            slug("What our example reproduces"),
            "what-our-example-reproduces"
        );
        assert_eq!(slug("Q, FSR and finesse"), "q-fsr-and-finesse");
    }
}
