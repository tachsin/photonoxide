import { isReleased } from "./latest.js";

/**
 * photonoxide's milestones in ROADMAP.md. Plain functions with no React or app imports, so the
 * studio's tests run them (`studio/tests/site-latest.test.ts`); roadmap.js reads the file.
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

const HEADING = /^###\s+(\d+(?:\.(?:\d+|x))*):\s+(.+?)\s*$/;
const ITEM = /^- \[( |x|X)\]\s+(.*)$/;
const BOLD = /\*\*(.+?)\*\*/;

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
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
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
