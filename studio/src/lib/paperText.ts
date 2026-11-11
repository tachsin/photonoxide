// Papers as the window shows and exports them: the types of the paper engine (papers.rs), the
// feed's topics, short author lists, a DOI or arXiv id read from what is pasted, the journal
// picker's choices, a journal's initials, and the reading list as BibTeX or CSV. Plain
// functions, tested by tests/papers.test.ts.

/** A paper in a list: no abstract, its first three authors and how many there are. */
export interface PaperSummary {
  id: string;
  doi: string | null;
  arxiv: string | null;
  title: string;
  authors: string[];
  author_count: number;
  venue: string;
  date: string;
  topics: string[];
}

/** A paper whole, as the cache and the reading list keep it. */
export interface Paper {
  id: string;
  doi: string | null;
  arxiv: string | null;
  title: string;
  authors: string[];
  venue: string;
  date: string;
  abstract: string;
  topics: string[];
}

export interface FeedState {
  /** The last check, and the last that reached a source, in s since 1970 (0: never). */
  checked_at: number;
  updated_at: number;
  /** Checks in a row that reached no source, and why the last one didn't. */
  failures: number;
  error: string;
  source_errors: string[];
}

/** A source as the journal picker shows it; `id` is a journal's first ISSN, or "arxiv". */
export interface Choice {
  id: string;
  name: string;
  short: string;
  publisher: string;
  on: boolean;
}

export interface Feed {
  papers: PaperSummary[];
  state: FeedState;
  due: boolean;
  sources: string[];
  /** Every source, on or off, for the journal picker. */
  choices: Choice[];
  /** The sources on that were never asked. */
  missing: number;
}

export interface PaperReference {
  doi: string | null;
  title: string | null;
  authors: string | null;
  venue: string | null;
  year: string | null;
  text: string | null;
}

/** Where photonoxide cites a paper: a kind of place ("Examples") and the places, or their count. */
export interface Cited {
  place: string;
  names: string[];
}

export interface PaperPage {
  paper: Paper;
  keywords: string[];
  oa_url: string | null;
  journal_doi: string | null;
  references: PaperReference[];
  cited_by: number;
  citing: Paper[];
  openalex: string | null;
  ours: Cited[];
  fetched_at: number;
  /** Shown from the cache because the sources couldn't be reached: why. */
  offline: string | null;
}

export interface Starred {
  paper: Paper;
  note: string;
  added: number;
}

/** The feed's topics, in the filter's order; their words are papers.rs's TOPICS. */
export const PAPER_TOPICS: { id: string; label: string }[] = [
  { id: "integrated", label: "Integrated photonics" },
  { id: "nonlinear", label: "Nonlinear" },
  { id: "quantum", label: "Quantum" },
  { id: "metasurfaces", label: "Metasurfaces" },
  { id: "lasers", label: "Lasers" },
  { id: "modulators", label: "Modulators" },
  { id: "fabrication", label: "Fabrication" },
];

// ---- the journal picker: the settings keep the sources turned off, so a journal added later starts on ----

/** The ids turned off once `id` is turned on or off. */
export function withSource(off: string[], id: string, on: boolean): string[] {
  const rest = off.filter((x) => x !== id);
  return on ? rest : [...rest, id];
}

/** The ids turned off when every source is on (`on`), or none is. */
export function withAll(choices: Choice[], on: boolean): string[] {
  return on ? [] : choices.map((c) => c.id);
}

/** What the picker's button says: "All sources", "5 of 17", "None". */
export function pickerLabel(choices: Choice[]): string {
  const on = choices.filter((c) => c.on).length;
  if (on === choices.length) return "All sources";
  return on ? `${on} of ${choices.length}` : "None";
}

/** Words a journal's initials skip. */
const SMALL = new Set(["of", "and", "the", "&", "for", "in", "on"]);

/**
 * A journal's initials for its badge, from the feed's short name: "Nat. Photon." → "NP",
 * "Phys. Rev. Lett." → "PRL", "Optica" → "Op", "arXiv" → "aX". Three letters at most.
 */
export function monogram(venue: string): string {
  if (venue.trim().toLowerCase() === "arxiv") return "aX";
  const words = venue
    .split(/[\s.,:;/()-]+/)
    .filter((w) => w && !SMALL.has(w.toLowerCase()) && /\p{L}/u.test(w));
  if (!words.length) return "?";
  if (words.length === 1) {
    const [a, b = ""] = [...words[0]];
    return a.toUpperCase() + b.toLowerCase();
  }
  return words
    .slice(0, 3)
    .map((w) => [...w][0].toUpperCase())
    .join("");
}

/** A journal's badge hue, 0–359, the same for the same name on every start. */
export function monogramHue(venue: string): number {
  let h = 2166136261;
  for (const c of venue) h = Math.imul(h ^ c.codePointAt(0)!, 16777619) >>> 0;
  return h % 360;
}

/** "Smith", "Smith and Lee", "Smith et al." from the authors and how many there are. */
export function shortAuthors(authors: string[], count = authors.length): string {
  const last = (a: string) => a.trim().split(/\s+/).pop() ?? a;
  if (!authors.length) return "";
  if (count === 1) return last(authors[0]);
  if (count === 2 && authors.length >= 2) return `${last(authors[0])} and ${last(authors[1])}`;
  return `${last(authors[0])} et al.`;
}

/** "Nov 4, 2026" from "2026-11-04" (or "Nov 2026", "2026"). */
export function shortDate(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  if (!y) return "";
  const month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(m || 1) - 1];
  return m ? (d ? `${month} ${d}, ${y}` : `${month} ${y}`) : String(y);
}

/** "just now", "5 min ago", "3 h ago", "2 days ago", from seconds since 1970. */
export function age(secs: number, nowMs: number): string {
  const s = Math.max(0, nowMs / 1000 - secs);
  if (s < 90) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 48 * 3600) return `${Math.round(s / 3600)} h ago`;
  return `${Math.round(s / 86400)} days ago`;
}

/**
 * The paper `text` names, `doi:<doi>` or `arxiv:<id>`, as papers.rs's `read_id` reads it: a DOI
 * (bare, `doi:`, a doi.org link) or an arXiv id (bare, `arXiv:`, an arxiv.org link; the version
 * dropped). Null for anything else.
 */
export function paperIdOf(text: string): string | null {
  const t = text.trim().replace(/[.,;)]+$/, "");
  const doi = /^(?:https?:\/\/(?:dx\.)?doi\.org\/|doi\.org\/|doi:)?(10\.[\d.]{4,}\/\S+)$/i.exec(t);
  if (doi) return `doi:${doi[1].toLowerCase()}`;
  const ax = /^(?:https?:\/\/arxiv\.org\/(?:abs|pdf)\/|arxiv\.org\/abs\/|arxiv:)?(\d{4}\.\d{4,5}|[a-z-]+(?:\.[a-z-]+)?\/\d{7})(?:v\d+)?(?:\.pdf)?$/i.exec(t);
  return ax ? `arxiv:${ax[1].toLowerCase()}` : null;
}

/**
 * Whether a remote abstract's math may be set by KaTeX (MathText): every `$…$` short, in a plain
 * alphabet, with no command that links, embeds, styles with HTML or defines macros. Anything else
 * is shown as the plain text it is.
 */
export function safeMath(text: string): boolean {
  const parts = [...text.matchAll(/\$([^$]+)\$/g)].map((m) => m[1]);
  if (!parts.length) return false;
  const banned = /\\(href|url|html[a-z]*|includegraphics|def|gdef|edef|xdef|let|newcommand|renewcommand|providecommand|expandafter|csname|char|mathchoice|raisebox|rule|kern|hspace|vspace|color|textcolor|colorbox|fcolorbox|style|class|id|data|tag|label|ref|eqref|verb)(?![a-z])/i;
  return parts.every((p) => p.length <= 160 && /^[\w\s\\{}^_.,+\-*/=()|<>!'~:;[\]&%°±×·…−≤≥≈∼µΩα-ωΑ-Ω]*$/.test(p) && !banned.test(p));
}

/** Where a paper is read on the web: its DOI's page, or its arXiv page. */
export function paperUrl(p: { doi: string | null; arxiv: string | null }): string | null {
  return p.doi ? `https://doi.org/${p.doi}` : p.arxiv ? `https://arxiv.org/abs/${p.arxiv}` : null;
}

/** A BibTeX key: the first author's family name, the year and the title's first long word. */
function bibKey(p: Paper): string {
  const family = (p.authors[0] ?? "anon").trim().split(/\s+/).pop() ?? "anon";
  const word = p.title.split(/\s+/).find((w) => w.replace(/\W/g, "").length > 3) ?? "paper";
  return `${family}${p.date.slice(0, 4)}${word}`.normalize("NFKD").replace(/[^A-Za-z0-9]/g, "");
}

/** Braces and backslashes made safe inside a BibTeX value. */
function bibValue(s: string): string {
  return s.replace(/\\/g, "\\textbackslash{}").replace(/[{}]/g, (c) => `\\${c}`);
}

/** The reading list as BibTeX: an @article per paper (@misc for a preprint), its note as `note`. */
export function toBibtex(list: Starred[]): string {
  const keys = new Map<string, number>();
  return list
    .map(({ paper: p, note }) => {
      let key = bibKey(p);
      const n = keys.get(key) ?? 0;
      keys.set(key, n + 1);
      if (n) key += String.fromCharCode(97 + n);
      const fields: [string, string][] = [
        ["title", p.title],
        ["author", p.authors.join(" and ")],
        [p.arxiv && !p.doi ? "howpublished" : "journal", p.arxiv && !p.doi ? `arXiv:${p.arxiv}` : p.venue],
        ["year", p.date.slice(0, 4)],
        ["doi", p.doi ?? ""],
        ["eprint", p.arxiv ?? ""],
        ["note", note],
      ];
      const body = fields
        .filter(([, v]) => v)
        .map(([k, v]) => `  ${k} = {${bibValue(v)}}`)
        .join(",\n");
      return `@${p.arxiv && !p.doi ? "misc" : "article"}{${key},\n${body}\n}\n`;
    })
    .join("\n");
}

/** A CSV cell: quoted when it holds a comma, a quote or a line break; a leading =, +, - or @ guarded so a spreadsheet reads text, not a formula. */
function cell(s: string): string {
  const safe = /^[=+\-@]/.test(s) ? `'${s}` : s;
  return /[",\n\r]/.test(safe) ? `"${safe.replace(/"/g, '""')}"` : safe;
}

/** The reading list as CSV: title, authors, venue, date, DOI, arXiv, link, note, added. */
export function toCsv(list: Starred[]): string {
  const rows = [["title", "authors", "venue", "date", "doi", "arxiv", "link", "note", "added"]];
  for (const { paper: p, note, added } of list) {
    rows.push([p.title, p.authors.join("; "), p.venue, p.date, p.doi ?? "", p.arxiv ?? "", paperUrl(p) ?? "", note, new Date(added * 1000).toISOString().slice(0, 10)]);
  }
  return rows.map((r) => r.map(cell).join(",")).join("\r\n") + "\r\n";
}

/** A source photonoxide cites (docs/references.yaml), with every place it cites it. */
export interface CitedSource {
  /** `Marcatili 1969`, `Snyder and Love 1983`, `Zhang et al. 2013`. */
  label: string;
  doi: string | null;
  key: string | null;
  authors: string;
  title: string;
  venue: string;
  year: string;
  /** A copy anyone can read, when there is one. */
  open: string | null;
  ours: Cited[];
}
