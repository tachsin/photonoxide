// Papers as the window shows and exports them (src/lib/paperText.ts), run by `pnpm test`.
import assert from "node:assert/strict";
import { test } from "node:test";

import { age, paperIdOf, paperUrl, safeMath, shortAuthors, shortDate, toBibtex, toCsv, type Starred } from "../src/lib/paperText.ts";

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
