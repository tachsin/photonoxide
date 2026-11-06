//! The paper engine on recorded answers (papers/recorded/: real answers of 2026-11-04, trimmed
//! to a few items): no network.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::*;
use crate::channels::github::Response;

const JOURNAL: &str = include_str!("recorded/crossref-journal.json");
const FILTERED: &str = include_str!("recorded/crossref-filtered.json");
const ARXIV_FEED: &str = include_str!("recorded/arxiv-feed.xml");
const WORK: &str = include_str!("recorded/crossref-work.json");
const OPENALEX: &str = include_str!("recorded/openalex-work.json");
const REFERENCES: &str = include_str!("recorded/openalex-references.json");
const CITES: &str = include_str!("recorded/openalex-cites.json");
const ARXIV_PAPER: &str = include_str!("recorded/arxiv-paper.xml");

/// A recorded answer: its status and body, or the network failing.
type Answer = Result<(u16, Vec<u8>), String>;
/// A request made: its URL and headers.
type Asked = (String, Vec<(String, String)>);

/// Answers from recordings, the first rule whose pattern the URL holds; and what was asked.
struct Canned {
    rules: Vec<(&'static str, Answer)>,
    asked: Mutex<Vec<Asked>>,
}

impl Canned {
    fn new(rules: Vec<(&'static str, Answer)>) -> Arc<Canned> {
        Arc::new(Canned {
            rules,
            asked: Mutex::new(Vec::new()),
        })
    }

    fn asked(&self) -> Vec<Asked> {
        self.asked.lock().unwrap().clone()
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
        let (_, answer) = self
            .rules
            .iter()
            .find(|(p, _)| url.contains(p))
            .ok_or_else(|| format!("{url}: no recording"))?;
        let (status, body) = answer.clone()?;
        Ok(Response {
            status,
            headers: vec![("ETag".into(), "\"v1\"".into())],
            body,
        })
    }

    fn download(
        &self,
        url: &str,
        _to: &Path,
        _progress: &mut dyn FnMut(u64, Option<u64>),
        _stop: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        Err(format!("{url}: no downloads here"))
    }
}

fn ok(body: &str) -> Answer {
    Ok((200, body.as_bytes().to_vec()))
}

/// A fresh cache folder, removed when dropped.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let d =
            std::env::temp_dir().join(format!("photonoxide-papers-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Dir(d)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn engine(dir: &Dir, http: Arc<Canned>) -> Papers {
    Papers::new(dir.0.clone(), http, Polite::none())
}

/// 2026-11-04, noon UTC.
const NOW: u64 = 1_793_793_600;

/// Every journal and arXiv answering from the recordings.
fn all_answer() -> Arc<Canned> {
    Canned::new(vec![
        ("query.bibliographic", ok(FILTERED)),
        ("api.crossref.org/journals/", ok(JOURNAL)),
        ("export.arxiv.org", ok(ARXIV_FEED)),
    ])
}

#[test]
fn the_journal_list_is_well_formed_and_every_issn_is_one() {
    let js = journals();
    assert_eq!(js.len(), 16);
    for j in &js {
        assert!(!j.name.is_empty() && !j.short.is_empty() && !j.publisher.is_empty());
        assert!(!j.issn.is_empty(), "{}", j.name);
        for i in &j.issn {
            let b = i.as_bytes();
            assert!(
                b.len() == 9
                    && b[4] == b'-'
                    && b[..4].iter().all(u8::is_ascii_digit)
                    && b[5..8].iter().all(u8::is_ascii_digit)
                    && (b[8].is_ascii_digit() || b[8] == b'X'),
                "{}: {i}",
                j.name
            );
        }
    }
    let filtered: Vec<&str> = js
        .iter()
        .filter(|j| j.filter)
        .map(|j| j.name.as_str())
        .collect();
    for general in [
        "Physical Review Applied",
        "Physical Review Letters",
        "Applied Physics Letters",
    ] {
        assert!(filtered.contains(&general), "{general} is filtered");
    }
    assert_eq!(sources().len(), 17);
}

#[test]
fn the_user_agent_carries_nothing_personal() {
    let ua = user_agent();
    assert!(ua.starts_with("photonoxide-studio/"));
    assert!(!ua.contains('@'), "{ua}");
    assert!(ua.contains("github.com/tachsin/photonoxide"));
}

#[test]
fn a_refresh_reads_every_source_keeps_it_and_the_feed_reads_back_offline() {
    let dir = Dir::new("refresh");
    let http = all_answer();
    let p = engine(&dir, http.clone());
    // nothing cached: an empty feed, and a check due
    let empty = p.feed(NOW, &[]);
    assert!(empty.papers.is_empty() && empty.due);
    let feed = p.refresh(NOW, &[], false);
    assert_eq!(http.asked().len(), 17, "one request per source");
    assert_eq!(feed.state.failures, 0);
    assert_eq!(feed.state.updated_at, NOW);
    assert!(!feed.due);
    // the journal recording's three papers (it answers for every unfiltered journal, so they
    // carry the first journal's name), arXiv's two, and the filtered journals' photonics papers
    let journal = parse::crossref_list(JOURNAL.as_bytes(), "x").unwrap();
    assert_eq!(journal.len(), 3);
    for p in &journal {
        assert!(feed.papers.iter().any(|s| s.id == p.id), "{}", p.title);
    }
    assert_eq!(feed.papers.iter().filter(|s| s.venue == "arXiv").count(), 2);
    assert!(
        feed.papers
            .iter()
            .all(|s| !s.title.contains("Any-light-particle")),
        "a particle physics paper of a filtered journal is dropped"
    );
    assert!(
        feed.papers
            .iter()
            .any(|s| s.title.contains("ion-channel laser")),
        "a laser paper of a filtered journal is kept"
    );
    // newest first, each paper once (the journal recording answers for every unfiltered journal)
    assert!(feed.papers.windows(2).all(|w| w[0].date >= w[1].date));
    let mut ids: Vec<&str> = feed.papers.iter().map(|s| s.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), feed.papers.len());
    // the requests: up to today, sorted by date, filtered journals asked for their optics
    let (url, _) = &http.asked()[0];
    assert!(url.contains("until-pub-date:2026-11-04") && url.contains("sort=published"));
    assert!(
        http.asked()
            .iter()
            .any(|(u, _)| u.contains("0031-9007") && u.contains("query.bibliographic"))
    );
    // offline: the same papers from the cache, the failure counted, a retry due in half an hour
    let none = Canned::new(vec![]);
    let offline = engine(&dir, none.clone());
    let again = offline.refresh(NOW + 5 * 3600, &[], false);
    assert_eq!(again.papers, feed.papers);
    assert_eq!(again.state.failures, 1);
    assert_eq!(again.state.updated_at, NOW);
    assert!(again.state.error.contains("no recording"));
    assert!(!offline.feed(NOW + 5 * 3600 + 60, &[]).due);
    assert!(offline.feed(NOW + 5 * 3600 + 31 * 60, &[]).due);
    // and it stopped after two sources rather than waiting on all seventeen
    assert_eq!(none.asked().len(), 2);
}

#[test]
fn a_validator_is_sent_back_and_an_unchanged_answer_keeps_the_cache() {
    let dir = Dir::new("conditional");
    let first = all_answer();
    let feed = engine(&dir, first).refresh(NOW, &[], false);
    let unchanged = Canned::new(vec![("", Ok((304, Vec::new())))]);
    let again = engine(&dir, unchanged.clone()).refresh(NOW + FRESH.as_secs(), &[], false);
    assert_eq!(again.papers, feed.papers);
    assert_eq!(again.state.failures, 0);
    assert_eq!(again.state.updated_at, NOW + FRESH.as_secs());
    for (url, headers) in unchanged.asked() {
        assert!(
            headers
                .iter()
                .any(|(k, v)| k == "If-None-Match" && v == "\"v1\""),
            "{url}"
        );
    }
}

#[test]
fn a_malformed_answer_is_one_sources_error_and_keeps_its_last_papers() {
    let dir = Dir::new("malformed");
    let feed = engine(&dir, all_answer()).refresh(NOW, &[], false);
    let broken = Canned::new(vec![
        ("query.bibliographic", ok(FILTERED)),
        ("api.crossref.org/journals/", ok(JOURNAL)),
        ("export.arxiv.org", ok("<html>Service unavailable</html>")),
    ]);
    let again = engine(&dir, broken).refresh(NOW + FRESH.as_secs(), &[], false);
    assert_eq!(again.state.failures, 0, "the journals answered");
    assert_eq!(again.state.source_errors.len(), 1);
    assert!(again.state.source_errors[0].starts_with("arXiv:"));
    // arXiv's papers are still there, from its last answer
    assert_eq!(
        again.papers.iter().filter(|s| s.venue == "arXiv").count(),
        feed.papers.iter().filter(|s| s.venue == "arXiv").count()
    );
    // a JSON that isn't Crossref's list, and one that isn't JSON
    assert!(parse::crossref_list(b"{\"message\":{}}", "x").is_err());
    assert!(parse::crossref_list(b"not json", "x").is_err());
    assert!(parse::arxiv_feed(b"\xff\xfe").is_err());
}

#[test]
fn a_papers_page_from_crossref_and_openalex_with_titled_references_and_citing_works() {
    let dir = Dir::new("page");
    let http = Canned::new(vec![
        ("api.crossref.org/works/10.1103", ok(WORK)),
        ("api.openalex.org/works/doi:", ok(OPENALEX)),
        ("filter=doi:", ok(REFERENCES)),
        ("filter=cites:", ok(CITES)),
    ]);
    let p = engine(&dir, http.clone());
    let page = p
        .open("https://doi.org/10.1103/PhysRevLett.58.2059", NOW)
        .unwrap();
    assert_eq!(page.paper.id, "doi:10.1103/physrevlett.58.2059");
    assert!(
        page.paper
            .title
            .starts_with("Inhibited Spontaneous Emission")
    );
    assert_eq!(page.paper.authors, vec!["Eli Yablonovitch"]);
    assert_eq!(page.paper.venue, "Physical Review Letters");
    assert_eq!(page.paper.date.get(..4), Some("1987"));
    // Crossref has no abstract for it: OpenAlex's, put back together from its index
    assert!(
        page.paper.abstract_text.len() > 200,
        "{}",
        page.paper.abstract_text
    );
    assert!(!page.paper.abstract_text.contains('<'));
    assert!(
        page.oa_url
            .as_deref()
            .is_some_and(|u| u.starts_with("http"))
    );
    assert_eq!(page.openalex.as_deref(), Some("W2075689277"));
    // every reference with a DOI has its title now
    assert_eq!(page.references.len(), 13);
    let with_doi: Vec<&Reference> = page.references.iter().filter(|r| r.doi.is_some()).collect();
    assert!(!with_doi.is_empty() && with_doi.iter().all(|r| r.title.is_some()));
    // cited: the count and the newest citing works
    assert!(page.cited_by > 10_000);
    assert_eq!(page.citing.len(), 3);
    assert!(page.citing.windows(2).all(|w| w[0].date >= w[1].date));
    assert!(!page.keywords.is_empty());
    // kept: opened again offline within the week, it comes from the cache without a request
    let n = http.asked().len();
    let offline = engine(&dir, Canned::new(vec![]));
    assert_eq!(
        offline
            .open("10.1103/physrevlett.58.2059", NOW + 3600)
            .unwrap()
            .paper,
        page.paper
    );
    // after the week, offline, the cache marked offline
    let stale = offline
        .open(
            "doi:10.1103/PhysRevLett.58.2059",
            NOW + PAPER_FRESH.as_secs() + 1,
        )
        .unwrap();
    assert!(stale.offline.is_some());
    assert_eq!(stale.references, page.references);
    assert_eq!(http.asked().len(), n);
    // a paper never opened, offline: an error that says so
    assert!(offline.open("10.1364/oe.1.000001", NOW).is_err());
    // nonsense is no paper
    assert!(offline.open("not a paper", NOW).is_err());
}

#[test]
fn a_preprints_page_from_arxiv() {
    let dir = Dir::new("preprint");
    let http = Canned::new(vec![(
        "export.arxiv.org/api/query?id_list=2305.01234",
        ok(ARXIV_PAPER),
    )]);
    let page = engine(&dir, http).open("arXiv:2305.01234v2", NOW).unwrap();
    assert_eq!(page.paper.id, "arxiv:2305.01234");
    assert_eq!(page.paper.venue, "arXiv");
    assert!(
        page.paper
            .title
            .starts_with("Generalized chiral instabilities")
    );
    assert!(page.paper.abstract_text.len() > 100);
    assert!(
        page.oa_url
            .as_deref()
            .is_some_and(|u| u.starts_with("https://arxiv.org/pdf/"))
    );
    // OpenAlex unreachable: no citations, and no failure
    assert_eq!(page.cited_by, 0);
}

#[test]
fn search_finds_every_word_in_titles_authors_and_abstracts() {
    let dir = Dir::new("search");
    let p = engine(&dir, all_answer());
    let feed = p.refresh(NOW, &[], false);
    let first = &feed.papers[0];
    let word = first
        .title
        .split_whitespace()
        .max_by_key(|w| w.len())
        .unwrap();
    let found = p.search(word, &[]);
    assert!(found.iter().any(|s| s.id == first.id), "{word}");
    assert!(p.search("", &[]).is_empty());
    assert!(p.search("zzzz-no-such-word", &[]).is_empty());
    // the reading list is searched too
    let starred = Paper {
        id: "doi:10.1/x".into(),
        title: "A starred paper on qqqq".into(),
        ..Paper::default()
    };
    assert_eq!(
        p.search("QQQQ", std::slice::from_ref(&starred))[0].id,
        starred.id
    );
}

#[test]
fn the_reading_list_round_trips_in_the_workspace() {
    let dir = Dir::new("list");
    assert!(read_list(&dir.0).is_empty());
    let list = vec![Starred {
        paper: Paper {
            id: "arxiv:2305.01234".into(),
            title: "t".into(),
            ..Paper::default()
        },
        note: "read section 3".into(),
        added: NOW,
    }];
    write_list(&dir.0, &list).unwrap();
    assert!(dir.0.join("papers").join("reading-list.json").is_file());
    assert_eq!(read_list(&dir.0), list);
}

#[test]
fn ids_are_read_from_what_is_pasted() {
    use parse::read_id;
    for (text, id) in [
        ("10.1364/OE.18.017186", "doi:10.1364/oe.18.017186"),
        (
            "doi:10.1103/PhysRevLett.58.2059",
            "doi:10.1103/physrevlett.58.2059",
        ),
        (
            "https://doi.org/10.1038/s41566-020-0001-1.",
            "doi:10.1038/s41566-020-0001-1",
        ),
        ("arXiv:2305.01234v3", "arxiv:2305.01234"),
        ("https://arxiv.org/abs/2611.05355v1", "arxiv:2611.05355"),
        ("https://arxiv.org/pdf/2611.05355.pdf", "arxiv:2611.05355"),
        ("2611.0535", "arxiv:2611.0535"),
        ("hep-th/9901001v2", "arxiv:hep-th/9901001"),
        ("physics.optics/0601001", "arxiv:physics.optics/0601001"),
    ] {
        assert_eq!(read_id(text).as_deref(), Some(id), "{text}");
    }
    for text in [
        "",
        "strip waveguide",
        "10.12/x",
        "2611.05",
        "12345.6789",
        "10.1364/ OE.1",
    ] {
        assert_eq!(read_id(text), None, "{text}");
    }
}

#[test]
fn markup_becomes_plain_text() {
    use parse::{entities, plain};
    assert_eq!(
        plain(
            "<jats:title>Abstract</jats:title><jats:p>Light in a <jats:italic>ring</jats:italic>, Q &gt; 10<jats:sup>6</jats:sup>.</jats:p><jats:p>Next.</jats:p>"
        ),
        "Light in a ring, Q > 106. Next."
    );
    assert_eq!(plain("a <script>alert(1)</script> b"), "a alert(1) b");
    // a publisher's pretty-printed title (Laser & Photonics Reviews, as Crossref had it on 2026-11-04)
    assert_eq!(
        plain(
            "A Solar‐Blind Photodetector Based on MXene/\n                    <i>β</i>\n                    ‐Ga\n                    <sub>2</sub>\n                    O\n                    <sub>3</sub>\n                    Heterojunction With Superlinear Photoresponse"
        ),
        "A Solar‐Blind Photodetector Based on MXene/β‐Ga2O3 Heterojunction With Superlinear Photoresponse"
    );
    assert_eq!(
        plain("collisions at\n  <mml:math>\n  <mml:mi>s</mml:mi>\n  </mml:math>\n  = 13 TeV"),
        "collisions at s = 13 TeV"
    );
    assert_eq!(plain("a\n  <i>Kerr</i>\n  comb"), "a Kerr comb");
    assert_eq!(plain("x < y and y > z"), "x < y and y > z");
    assert_eq!(
        entities("&#955; = 1.55 &#x3bc;m &amp; &unknown; &"),
        "λ = 1.55 μm & &unknown; &"
    );
    let index: serde_json::Value =
        serde_json::from_str(r#"{"light":[0,3],"guided":[1],"by":[2]}"#).unwrap();
    assert_eq!(parse::inverted(&index), "light guided by light");
}

#[test]
fn topics_and_the_photonics_filter_go_by_their_words() {
    assert_eq!(
        topics_of("A thin-film lithium niobate electro-optic modulator", ""),
        vec!["integrated", "modulators"]
    );
    assert_eq!(topics_of("Kerr microcomb solitons", ""), vec!["nonlinear"]);
    assert!(topics_of("Dark matter limits", "").is_empty());
    assert!(photonic("Ion-channel laser onset", ""));
    assert!(!photonic("Search for light bosons", "in proton collisions"));
}

#[test]
fn the_index_of_our_references_names_the_places_that_cite_a_paper() {
    let md = "### A\n\n#### Someone 2010\n\nS. One, \"T\", J (2010). [doi:10.1364/OE.18.1](https://doi.org/x)\n\n- Methods pages: [devices.md:42](methods/devices.md#L42), [979](methods/devices.md#L979)\n- Examples: [l3_akahane.rs:5](../examples/l3_akahane.rs#L5)\n- Jobs and circuits: [jobs/chip-fit.yaml:2](../jobs/chip-fit.yaml#L2)\n- Code: [src/a.rs:1](x), [src/b.rs:2](y)\n\n#### No DOI 1999\n\nA book.\n\n- Code: [src/c.rs:1](z)\n";
    let index = ours::index(md);
    assert_eq!(
        index,
        "10.1364/oe.18.1\tMethods pages=devices.md;Examples=l3_akahane.rs;Jobs and circuits=chip-fit.yaml;Code=2\n"
    );
    let cited = ours_in(&index, "10.1364/oe.18.1");
    assert_eq!(cited.len(), 4);
    assert_eq!(
        cited[1],
        Cited {
            place: "Examples".into(),
            names: vec!["l3_akahane.rs".into()]
        }
    );
    // the index built from docs/references.md knows the papers the examples reproduce
    let built = include_str!(concat!(env!("OUT_DIR"), "/ours.tsv"));
    assert!(
        built.lines().count() > 500,
        "docs/references.md has 551 DOIs"
    );
    let (doi, _) = built
        .lines()
        .find(|l| l.contains("Examples="))
        .unwrap()
        .split_once('\t')
        .unwrap();
    assert!(
        ours_of(Some(&doi.to_uppercase()))
            .iter()
            .any(|c| c.place == "Examples")
    );
}

#[test]
fn days_are_utc_dates() {
    assert_eq!(day(0), "1970-01-01");
    assert_eq!(day(NOW), "2026-11-04");
    assert_eq!(day(951_782_400), "2000-02-29");
}

// ---- the journal picker ----

#[test]
fn a_source_turned_off_is_neither_asked_nor_shown_and_turned_on_is_asked_alone() {
    let dir = Dir::new("picker");
    let http = all_answer();
    let p = engine(&dir, http.clone());
    let off = vec!["arxiv".to_owned(), "1749-4885".to_owned()];
    let feed = p.refresh(NOW, &off, false);
    // fifteen requests, none to arXiv or to Nature Photonics
    assert_eq!(http.asked().len(), 15);
    assert!(
        http.asked()
            .iter()
            .all(|(u, _)| !u.contains("export.arxiv.org") && !u.contains("1749-4885"))
    );
    assert!(feed.papers.iter().all(|s| s.venue != "arXiv"));
    assert!(!feed.papers.is_empty());
    // the picker lists all seventeen, the two turned off as off
    assert_eq!(feed.choices.len(), 17);
    let offs: Vec<&str> = feed
        .choices
        .iter()
        .filter(|c| !c.on)
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(offs, ["1749-4885", "arxiv"]);
    let arxiv = feed.choices.iter().find(|c| c.id == "arxiv").unwrap();
    assert_eq!(arxiv.short, "arXiv");
    assert_eq!(feed.missing, 0);
    // both on again: shown at once from the cache (neither has one yet), and two missing
    let on = p.feed(NOW + 60, &[]);
    assert_eq!(on.missing, 2);
    assert!(on.choices.iter().all(|c| c.on));
    // asked alone: just those two, and the check's state as it was
    let only = p.refresh(NOW + 60, &[], true);
    assert_eq!(http.asked().len(), 17);
    assert_eq!(only.missing, 0);
    assert_eq!(only.state.checked_at, NOW);
    assert_eq!(only.papers.iter().filter(|s| s.venue == "arXiv").count(), 2);
    // nothing new to ask: nothing asked
    p.refresh(NOW + 120, &[], true);
    assert_eq!(http.asked().len(), 17);
    // the papers of a source turned off stay kept, and come back when it is on
    assert!(
        p.feed(NOW + 180, &["arxiv".to_owned()])
            .papers
            .iter()
            .all(|s| s.venue != "arXiv")
    );
    assert_eq!(
        p.feed(NOW + 180, &[])
            .papers
            .iter()
            .filter(|s| s.venue == "arXiv")
            .count(),
        2
    );
    // a search still looks through every source kept
    let title = &only
        .papers
        .iter()
        .find(|s| s.venue == "arXiv")
        .unwrap()
        .title;
    assert!(!p.search(title, &[]).is_empty());
}

#[test]
fn with_every_source_off_nothing_is_asked_and_nothing_fails() {
    let dir = Dir::new("picker-none");
    let http = all_answer();
    let p = engine(&dir, http.clone());
    let off: Vec<String> = sources().iter().map(|s| s.id().to_owned()).collect();
    let feed = p.refresh(NOW, &off, false);
    assert!(http.asked().is_empty());
    assert!(feed.papers.is_empty());
    assert_eq!(feed.state.failures, 0);
    assert!(feed.choices.iter().all(|c| !c.on));
    assert_eq!(feed.missing, 0);
    // an id no source has turns nothing off
    assert_eq!(chosen(&["0000-0000".to_owned()]).len(), 17);
    // every id is a source's own, once
    let mut ids: Vec<&str> = off.iter().map(String::as_str).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 17);
}

// ---- the feed's images ----

const ARXIV_HTML: &str = include_str!("recorded/arxiv-html.html");

/// A PNG's first bytes and a little more: what the cache reads an image by.
fn png(len: usize) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n".to_vec();
    b.resize(len, 7);
    b
}

#[test]
fn a_preprints_first_figure_is_its_first_raster_image() {
    // the recording's first figure is three SVG panels; the second's first panel is a PNG
    assert_eq!(
        images::first_figure(ARXIV_HTML).as_deref(),
        Some("https://arxiv.org/html/2611.07038v1/4bandModelNoEPs_3D.png")
    );
    // a table is not a figure; a link elsewhere, a path up or an SVG is never fetched
    let table = r#"<figure class="ltx_table"><img src="2611.1v1/t.png"></figure>"#;
    assert_eq!(images::first_figure(table), None);
    for src in [
        "https://example.org/x.png",
        "//example.org/x.png",
        "2611.1v1/../../x.png",
        "2611.1v1/x.svg",
        "javascript:x.png",
        "/static/x.png",
    ] {
        let html = format!(r#"<figure class="ltx_figure"><img src="{src}"></figure>"#);
        assert_eq!(images::first_figure(&html), None, "{src}");
    }
    let html = r#"<figure class="ltx_figure"><img alt="x" src="/html/2611.1v2/x1.JPG"></figure>"#;
    assert_eq!(
        images::first_figure(html).as_deref(),
        Some("https://arxiv.org/html/2611.1v2/x1.JPG")
    );
    // which ids have an HTML version to ask: arXiv's present form only
    assert_eq!(images::preprint("arxiv:2611.07038"), Some("2611.07038"));
    assert_eq!(images::preprint("arxiv:2611.0703v2"), Some("2611.0703v2"));
    for id in [
        "doi:10.1364/oe.1",
        "arxiv:hep-th/9901001",
        "arxiv:2611.07038/x",
        "arxiv:2611.07",
        "2611.07038",
    ] {
        assert_eq!(images::preprint(id), None, "{id}");
    }
    // an image is known by its bytes, never by its name
    assert_eq!(images::sniff(&png(16)), Some(("png", "image/png")));
    assert_eq!(
        images::sniff(b"\xff\xd8\xff\xe0"),
        Some(("jpg", "image/jpeg"))
    );
    assert_eq!(
        images::sniff(b"RIFF\0\0\0\0WEBPVP8 "),
        Some(("webp", "image/webp"))
    );
    assert_eq!(
        images::sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
        None
    );
}

#[test]
fn a_figure_is_asked_once_kept_and_served_by_its_name_only() {
    let dir = Dir::new("images");
    let http = Canned::new(vec![
        ("arxiv.org/html/2611.07038v1/", Ok((206, png(2000)))),
        (
            "arxiv.org/html/2611.07038",
            Ok((206, ARXIV_HTML.as_bytes().to_vec())),
        ),
        ("arxiv.org/html/2611.00001", Ok((404, Vec::new()))),
    ]);
    let p = engine(&dir, http.clone());
    // a journal's paper: nothing asked
    assert_eq!(p.image("doi:10.1364/oe.1", NOW), Ok(None));
    assert!(http.asked().is_empty());
    let name = p.image("arxiv:2611.07038", NOW).unwrap().unwrap();
    assert_eq!(name, "arxiv-2611.07038.png");
    let asked = http.asked();
    assert_eq!(asked.len(), 2, "the page's head, then the figure");
    assert_eq!(asked[0].0, "https://arxiv.org/html/2611.07038");
    assert_eq!(
        asked[0].1,
        [(
            "Range".to_owned(),
            format!("bytes=0-{}", images::HTML_HEAD - 1)
        )]
    );
    assert_eq!(
        asked[1].1,
        [("Range".to_owned(), format!("bytes=0-{}", images::MAX_IMAGE))]
    );
    // kept: asked again, nothing goes out
    assert_eq!(
        p.image("arxiv:2611.07038", NOW + 3600),
        Ok(Some(name.clone()))
    );
    assert_eq!(http.asked().len(), 2);
    let (bytes, mime) = p.images.read(&name).unwrap();
    assert_eq!((bytes.len(), mime), (2000, "image/png"));
    assert_eq!(p.images.kept(), (1, 2000));
    // the scheme serves files of the folder by name, nothing else
    for bad in [
        "../feed.json",
        "index.json",
        ".index",
        "",
        "a/b.png",
        "arxiv-x.png",
    ] {
        assert!(p.images.read(bad).is_none(), "{bad}");
    }
    // no HTML version yet: remembered for a day, then asked again
    assert_eq!(p.image("arxiv:2611.00001", NOW), Ok(None));
    assert_eq!(p.image("arxiv:2611.00001", NOW + 3600), Ok(None));
    assert_eq!(http.asked().len(), 3);
    p.image("arxiv:2611.00001", NOW + images::NO_HTML_AGAIN.as_secs())
        .unwrap();
    assert_eq!(http.asked().len(), 4);
    // offline: an error, and nothing remembered
    let offline = engine(&dir, Canned::new(vec![]));
    assert!(offline.image("arxiv:2611.09999", NOW).is_err());
    assert!(offline.image("arxiv:2611.09999", NOW).is_err());
    // a figure is kept thirty days, then asked again
    let later = NOW + images::KEEP.as_secs();
    let again = Canned::new(vec![]);
    assert!(
        engine(&dir, again.clone())
            .image("arxiv:2611.07038", later)
            .is_err()
    );
    assert_eq!(again.asked().len(), 1);
    // cleared: nothing kept
    p.images.clear().unwrap();
    assert_eq!(p.images.kept(), (0, 0));
    assert!(p.images.read(&name).is_none());
}

#[test]
fn a_figure_too_large_is_left_out_and_the_cache_keeps_to_its_cap() {
    let dir = Dir::new("images-cap");
    let http = Canned::new(vec![
        (
            "arxiv.org/html/2611.07038v1/",
            Ok((206, png(images::MAX_IMAGE + 1))),
        ),
        (
            "arxiv.org/html/2611.07038",
            Ok((206, ARXIV_HTML.as_bytes().to_vec())),
        ),
        (
            "arxiv.org/html/2611.11111",
            Ok((200, b"<p>no figures</p>".to_vec())),
        ),
    ]);
    let p = engine(&dir, http.clone());
    assert_eq!(p.image("arxiv:2611.07038", NOW), Ok(None));
    assert_eq!(p.images.kept(), (0, 0));
    // remembered: not asked again
    assert_eq!(p.image("arxiv:2611.07038", NOW + 60), Ok(None));
    assert_eq!(p.image("arxiv:2611.11111", NOW), Ok(None));
    assert_eq!(p.image("arxiv:2611.11111", NOW + 60), Ok(None));
    assert_eq!(http.asked().len(), 3);
    // over the cap, the oldest figures go first
    let each = images::MAX_IMAGE;
    let fits = (images::CAP / each as u64) as usize;
    for k in 0..=fits {
        let key = format!("arxiv-2611.{k:05}");
        let kept = p.images.keep(&key, NOW + k as u64, Ok((&png(each), "png")));
        assert!(kept.unwrap().is_some());
    }
    let (count, bytes) = p.images.kept();
    assert_eq!(count, fits);
    assert!(bytes <= images::CAP);
    assert_eq!(
        p.images.lookup("arxiv-2611.00000", NOW + 99),
        images::Lookup::Unknown
    );
    assert!(matches!(
        p.images.lookup(&format!("arxiv-2611.{fits:05}"), NOW + 99),
        images::Lookup::Kept(_)
    ));
}
