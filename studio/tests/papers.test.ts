// Papers as the window shows and exports them (src/lib/paperText.ts), run by `pnpm test`.
import assert from "node:assert/strict";
import { test } from "node:test";

import { age, monogram, monogramHue, paperIdOf, paperUrl, pickerLabel, safeMath, shortAuthors, shortDate, toBibtex, toCsv, withAll, withSource, type Choice, type Starred } from "../src/lib/paperText.ts";

/** The picker's sources: three journals and arXiv, all on. */
const CHOICES: Choice[] = [
  { id: "1749-4885", name: "Nature Photonics", short: "Nat. Photon.", publisher: "Springer Nature", on: true },
  { id: "2334-2536", name: "Optica", short: "Optica", publisher: "Optica", on: true },
  { id: "0031-9007", name: "Physical Review Letters", short: "Phys. Rev. Lett.", publisher: "APS", on: true },
  { id: "arxiv", name: "arXiv, physics.optics", short: "arXiv", publisher: "Cornell University", on: true },
];
/** The choices with `off` turned off, as papers.rs lists them after the settings change. */
const after = (off: string[]): Choice[] => CHOICES.map((c) => ({ ...c, on: !off.includes(c.id) }));

test("the journal picker keeps the sources turned off, so a journal added later starts on", () => {
  // one off, then on again: each id once, the rest kept
  let off = withSource([], "arxiv", false);
  assert.deepEqual(off, ["arxiv"]);
  off = withSource(off, "2334-2536", false);
  assert.deepEqual(off, ["arxiv", "2334-2536"]);
  assert.deepEqual(withSource(off, "2334-2536", false), ["arxiv", "2334-2536"], "turned off twice, listed once");
  off = withSource(off, "arxiv", true);
  assert.deepEqual(off, ["2334-2536"]);
  assert.deepEqual(withSource(off, "0000-0000", true), ["2334-2536"], "turning on what was on changes nothing");
  // All and None
  assert.deepEqual(withAll(CHOICES, true), []);
  assert.deepEqual(withAll(CHOICES, false), ["1749-4885", "2334-2536", "0031-9007", "arxiv"]);
  // what the button says
  assert.equal(pickerLabel(CHOICES), "All sources");
  assert.equal(pickerLabel(after(off)), "3 of 4");
  assert.equal(pickerLabel(after(withAll(CHOICES, false))), "None");
  // a journal the settings never named (added to journals.json later) is on
  const later = [...after(["arxiv"]), { id: "2378-0967", name: "APL Photonics", short: "APL Photon.", publisher: "AIP", on: true }];
  assert.equal(pickerLabel(later), "4 of 5");
});

test("a journal's initials and tint, the same on every start", () => {
  assert.equal(monogram("Nat. Photon."), "NP");
  assert.equal(monogram("Phys. Rev. Lett."), "PRL");
  assert.equal(monogram("J. Lightw. Technol."), "JLT");
  assert.equal(monogram("Laser Photonics Rev."), "LPR");
  assert.equal(monogram("Optica"), "Op");
  assert.equal(monogram("Nature"), "Na");
  assert.equal(monogram("arXiv"), "aX");
  assert.equal(monogram("Light: Science & Applications"), "LSA", "the small words and the punctuation skipped");
  assert.equal(monogram(""), "?");
  assert.equal(monogramHue("Optica"), monogramHue("Optica"));
  assert.notEqual(monogramHue("Opt. Express"), monogramHue("Opt. Lett."));
  for (const v of ["", "Nature", "Nat. Photon.", "arXiv"]) {
    const h = monogramHue(v);
    assert.ok(Number.isInteger(h) && h >= 0 && h < 360, `${v}: ${h}`);
  }
});

test("a DOI or an arXiv id is read from what is pasted, as papers.rs reads it", () => {
  const cases: [string, string | null][] = [
    ["10.1364/OE.18.017186", "doi:10.1364/oe.18.017186"],
    ["doi:10.1103/PhysRevLett.58.2059", "doi:10.1103/physrevlett.58.2059"],
    ["https://doi.org/10.1038/s41566-020-0001-1.", "doi:10.1038/s41566-020-0001-1"],
    ["arXiv:2305.01234v3", "arxiv:2305.01234"],
    ["https://arxiv.org/abs/2611.05355v1", "arxiv:2611.05355"],
    ["https://arxiv.org/pdf/2611.05355.pdf", "arxiv:2611.05355"],
    ["hep-th/9901001v2", "arxiv:hep-th/9901001"],
    ["", null],
    ["strip waveguide", null],
    ["2611.05", null],
    ["10.1364/ OE.1", null],
  ];
  for (const [text, id] of cases) assert.equal(paperIdOf(text), id, text);
});

test("authors, dates and ages read short", () => {
  assert.equal(shortAuthors(["Eli Yablonovitch"]), "Yablonovitch");
  assert.equal(shortAuthors(["A. B. Smith", "C. Lee"]), "Smith and Lee");
  assert.equal(shortAuthors(["A. Smith", "B. Lee", "C. Kim"], 12), "Smith et al.");
  assert.equal(shortAuthors([]), "");
  assert.equal(shortDate("2026-11-04"), "Nov 4, 2026");
  assert.equal(shortDate("1987-05"), "May 1987");
  assert.equal(shortDate(""), "");
  const now = 1_793_793_600_000;
  assert.equal(age(now / 1000 - 30, now), "just now");
  assert.equal(age(now / 1000 - 3 * 3600, now), "3 h ago");
  assert.equal(age(now / 1000 - 5 * 86400, now), "5 days ago");
  assert.equal(paperUrl({ doi: "10.1/x", arxiv: null }), "https://doi.org/10.1/x");
  assert.equal(paperUrl({ doi: null, arxiv: "2305.01234" }), "https://arxiv.org/abs/2305.01234");
});

test("only a safe subset of an abstract's math goes to KaTeX", () => {
  assert.ok(safeMath("attention $|\\langle q|k\\rangle|^4$ to $10^{-11}$"));
  assert.ok(safeMath("Q of $1.2 \\times 10^{6}$ at $\\lambda = 1550$ nm"));
  assert.ok(!safeMath("no math here"));
  assert.ok(!safeMath("a $\\href{javascript:alert(1)}{x}$ link"));
  assert.ok(!safeMath("$\\def\\a{\\a\\a}\\a$"));
  assert.ok(!safeMath("$\\htmlClass{x}{y}$"));
  assert.ok(!safeMath(`$${"x".repeat(200)}$`));
});

const LIST: Starred[] = [
  {
    paper: { id: "doi:10.1103/physrevlett.58.2059", doi: "10.1103/physrevlett.58.2059", arxiv: null, title: "Inhibited Spontaneous Emission in {Solid-State} Physics", authors: ["Eli Yablonovitch"], venue: "Physical Review Letters", date: "1987-05-18", abstract: "", topics: [] },
    note: "the photonic band gap, first",
    added: 1_793_793_600,
  },
  {
    paper: { id: "arxiv:2305.01234", doi: null, arxiv: "2305.01234", title: "Generalized chiral instabilities", authors: ["A. Author", "B. Writer"], venue: "arXiv", date: "2023-05-02", abstract: "", topics: [] },
    note: "=SUM(A1), \"quoted\"",
    added: 1_793_793_600,
  },
];

test("the reading list as BibTeX", () => {
  const bib = toBibtex(LIST);
  assert.match(bib, /^@article\{Yablonovitch1987Inhibited,\n {2}title = \{Inhibited Spontaneous Emission in \\\{Solid-State\\\} Physics\},/);
  assert.match(bib, /doi = \{10\.1103\/physrevlett\.58\.2059\}/);
  assert.match(bib, /@misc\{Author2023Generalized,[\s\S]*howpublished = \{arXiv:2305\.01234\}[\s\S]*eprint = \{2305\.01234\}/);
  assert.equal((bib.match(/^@/gm) ?? []).length, 2);
});

test("the reading list as CSV, quoted and safe from formulas", () => {
  const lines = toCsv(LIST).split("\r\n");
  assert.equal(lines[0], "title,authors,venue,date,doi,arxiv,link,note,added");
  assert.ok(lines[1].startsWith("Inhibited Spontaneous Emission in {Solid-State} Physics,Eli Yablonovitch,Physical Review Letters,1987-05-18,"));
  assert.ok(lines[2].includes(`"'=SUM(A1), ""quoted"""`), lines[2]);
  assert.ok(lines[2].includes("https://arxiv.org/abs/2305.01234"));
  assert.equal(lines.length, 4);
});
