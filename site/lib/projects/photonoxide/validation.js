import { cache } from "react";
import { getRepoFile } from "./github";

/**
 * The validation report, docs/validation.md, read at the pinned commit (see
 * github.js). `photonoxide validate` writes it and CI checks it, so the
 * table is the library's own: one row per case, every column a cell.
 */

/**
 * @typedef {object} Case
 * @property {string} id         e.g. "mode/pml-soi-leakage-te"
 * @property {string} area       the id's first part: "units", "material", "mode", …
 * @property {string} tier       "analytic", "cross-code" or "published"
 * @property {string} what
 * @property {string} against
 * @property {string} measured
 * @property {string} expected
 * @property {string} tolerance
 * @property {boolean} passed
 */

const AREAS = {
  units: "Units and conventions",
  material: "Materials",
  mode: "Mode solvers",
};

/** The report's area names, for headings. */
export function areaTitle(area) {
  return AREAS[area] ?? area;
}

/** @param {string} markdown @returns {Case[]} */
export function parseReport(markdown) {
  const cases = [];
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
    if (!line.startsWith("| `")) continue;
    const cells = line
      .slice(1, -1)
      .split(" | ")
      .map((c) => c.trim());
    if (cells.length < 8) continue;
    const id = cells[0].replace(/`/g, "");
    cases.push({
      id,
      area: id.split("/")[0],
      tier: cells[1],
      what: cells[2],
      against: cells[3],
      measured: cells[4],
      expected: cells[5],
      tolerance: cells[6],
      passed: cells[7] === "pass",
    });
  }
  return cases;
}

/** @returns {Promise<{ ok: boolean, cases: Case[] }>} ok is false when GitHub can't be reached */
export const getReport = cache(async () => {
  const markdown = await getRepoFile("docs/validation.md");
  const cases = parseReport(markdown);
  return { ok: cases.length > 0, cases };
});

/**
 * A case's source with its DOIs as links: [text, doi | null] pieces.
 * @param {string} against
 * @returns {{ text: string, doi: string | null }[]}
 */
export function withDois(against) {
  const parts = [];
  const re = /doi:(10\.\d{4,9}\/[^\s,;)]+)/g;
  let last = 0;
  for (const m of against.matchAll(re)) {
    if (m.index > last) parts.push({ text: against.slice(last, m.index), doi: null });
    parts.push({ text: m[1], doi: m[1] });
    last = m.index + m[0].length;
  }
  if (last < against.length) parts.push({ text: against.slice(last), doi: null });
  return parts;
}
