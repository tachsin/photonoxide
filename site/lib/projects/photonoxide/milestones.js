import { isReleased } from "./latest.js";

/**
 * photonoxide's milestones in ROADMAP.md. Plain functions with no React or app imports, so the
 * studio's tests run them (`studio/tests/site-latest.test.ts`, and `site-inline.test.ts` on the
 * repository's own ROADMAP.md); roadmap.js reads the file. The overview's milestones and the
 * roadmap page both come from here.
 *
 * A milestone is a `### <version>: <title>` heading (a version like "0.4" or "0.3.x"). Its items are the
 * `- [ ]` / `- [x]` lines under it (top level only), their bold lead-in
 * being the item's name. A milestone is done when its heading ends in ✅, or when the latest
 * release has it (its version is the release's or older), whatever its boxes say: a release's
 * tag then shows its own milestone done even when the ✅ was added after the tag. The first
 * milestone not done is "next".
 */

/**
 * @typedef {object} Milestone
 * @property {string} version   e.g. "0.4"
 * @property {string} title     e.g. "Finite-difference time-domain (FDTD)"
 * @property {"done" | "next" | "planned"} status
 * @property {string[]} items   the bold lead-ins of its checklist, in order
 * @property {number} done      checked items
 * @property {number} total     all items
 */

export const HEADING = /^###\s+(\d+(?:\.(?:\d+|x))*):\s+(.+?)\s*$/;
const ITEM = /^- \[( |x|X)\]\s+(.*)$/;
const BOLD = /\*\*(.+?)\*\*/;
const FENCE = /^\s*(```|~~~)/;

/** A milestone from its heading's version and title (the title may end in ✅). */
function milestone(version, title, release) {
  const done = title.endsWith("✅") || isReleased(version, release);
  return {
    version,
    title: title.replace(/\s*✅$/, ""),
    status: done ? "done" : "planned",
    items: [],
    done: 0,
    total: 0,
  };
}

/** The first milestone not done is next. */
function markNext(milestones) {
  const next = milestones.find((m) => m.status !== "done");
  if (next) next.status = "next";
  return milestones;
}

/**
 * @param {string} markdown  ROADMAP.md
 * @param {string | null} [release]  the latest release, e.g. "0.6.3"
 * @returns {Milestone[]}
 */
export function parseMilestones(markdown, release = null) {
  /** @type {Milestone[]} */
  const milestones = [];
  let current = null;
  let fenced = false;
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
    // a fenced block's lines are its text, never a heading or an item
    if (FENCE.test(line)) fenced = !fenced;
    if (fenced) continue;
    if (line.startsWith("## ")) {
      current = null;
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      current = milestone(heading[1], heading[2], release);
      milestones.push(current);
      continue;
    }
    if (line.startsWith("### ")) {
      current = null; // "### Throughout" and the like
      continue;
    }
    const item = current ? ITEM.exec(line) : null;
    if (item) {
      current.total += 1;
      if (item[1] !== " ") current.done += 1;
      const bold = BOLD.exec(item[2]);
      const name = (bold ? bold[1] : item[2]).replace(/[:,.]$/, "").trim();
      if (name) current.items.push(name);
    }
  }
  return markNext(milestones);
}

/**
 * The milestones from headings alone, `<version>: <title>` (meta.js's FALLBACK_MILESTONES).
 * @param {string[]} headings
 * @param {string | null} [release]
 * @returns {Milestone[]}
 */
export function milestonesFromHeadings(headings, release = null) {
  return markNext(
    headings.map((heading) => {
      const [version, ...rest] = heading.split(": ");
      return milestone(version, rest.join(": "), release);
    }),
  );
}

/** A heading's anchor, as the app's Markdown renderer makes it: "0.5.4: Geometry and devices ✅" is "054-geometry-and-devices". */
export function headingAnchor(text) {
  return String(text)
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .trim()
    .replace(/\s+/g, "-");
}

/** A milestone's anchor on the roadmap page, the one its heading had when the page printed ROADMAP.md whole, so old links still land. */
export function milestoneAnchor(m) {
  return headingAnchor(`${m.version}: ${m.title}`);
}

/**
 * @typedef {object} RoadmapBlock     a `### ` heading and what follows it
 * @property {string} heading
 * @property {string} anchor
 * @property {string} body            Markdown, without the heading
 * @property {Milestone | null} milestone
 *
 * @typedef {object} RoadmapSection   a `## ` heading and what follows it
 * @property {string} heading
 * @property {string} anchor
 * @property {string} intro           Markdown before its first `### `
 * @property {RoadmapBlock[]} blocks
 */

/**
 * ROADMAP.md in its sections and their `### ` blocks, each milestone's block
 * with its parsed milestone (its status as parseMilestones gives it for the
 * same release). Every line of the file but its `# ` title is in exactly one
 * place, so a page built from this prints all of it.
 * @param {string} markdown
 * @param {string | null} [release]  the latest release, e.g. "0.6.3"
 * @returns {{ title: string, preface: string, sections: RoadmapSection[] }}
 */
export function splitRoadmap(markdown, release = null) {
  const parsed = parseMilestones(markdown, release);
  let next = 0;
  let title = "";
  const preface = [];
  /** @type {RoadmapSection[]} */
  const sections = [];
  let section = null;
  let lines = preface;
  let fenced = false;
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
    if (FENCE.test(line)) fenced = !fenced;
    if (!fenced && !title && !section && /^#\s+/.test(line)) {
      title = line.replace(/^#\s+/, "").trim();
      continue;
    }
    if (!fenced && line.startsWith("## ")) {
      const heading = line.slice(3).trim();
      section = { heading, anchor: headingAnchor(heading), intro: "", lines: [], blocks: [] };
      sections.push(section);
      lines = section.lines;
      continue;
    }
    if (!fenced && section && line.startsWith("### ")) {
      const heading = line.slice(4).trim();
      const m = HEADING.test(line) ? (parsed[next++] ?? null) : null;
      const block = { heading, anchor: m ? milestoneAnchor(m) : headingAnchor(heading), body: "", lines: [], milestone: m };
      section.blocks.push(block);
      lines = block.lines;
      continue;
    }
    lines.push(line);
  }
  const out = sections.map(({ lines: intro, blocks, ...s }) => ({
    ...s,
    intro: intro.join("\n").trim(),
    blocks: blocks.map(({ lines: body, ...b }) => ({ ...b, body: body.join("\n").trim() })),
  }));
  return { title, preface: preface.join("\n").trim(), sections: out };
}
