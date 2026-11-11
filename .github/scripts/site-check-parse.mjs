// What site-check.mjs reads from the live pages: their visible text, and each milestone's status
// in it. Plain functions with no network, so studio/tests/site-check.test.ts runs them on text
// copied from the live roadmap page.

const ENTITIES = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };

/** A page's visible text: no scripts or styles, no tags, entities decoded, spaces collapsed. */
export function text(html) {
  return html
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]*>/g, " ")
    .replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (m, e) =>
      e[0] === "#"
        ? String.fromCodePoint(e[1] === "x" || e[1] === "X" ? parseInt(e.slice(2), 16) : Number(e.slice(1)))
        : ENTITIES[e.toLowerCase()] ?? m,
    )
    .replace(/\s+/g, " ");
}

const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
export const LABEL = { done: "Done", next: "Next", planned: "Planned" };

/**
 * Each milestone's status as a page shows it: "<version> <title> [N of M done] Done|Next|Planned".
 * The roadmap's timeline shows an open milestone's checked items between its title and its status
 * ("0.7.1 Distributed memory 0 of 7 done Planned"); the overview's cards have none.
 */
export function shownStatuses(pageText, milestones) {
  const shown = new Map();
  for (const m of milestones) {
    const match = new RegExp(`(?:^| )${escape(m.version)} ${escape(m.title)} (?:\\d+ of \\d+ (?:items )?done )?(Done|Next|Planned)\\b`).exec(pageText);
    if (match) shown.set(m.version, match[1]);
  }
  return shown;
}
