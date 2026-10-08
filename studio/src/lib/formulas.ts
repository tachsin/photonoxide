// Where the math is in Markdown, as the studio sets it: the rule for inline math, and the
// formulas of a text. No imports, so the build's own config can use it (vite.config.ts renders
// every formula of every Academy lesson, and fails on one KaTeX can't).

/**
 * Inline math, $…$, as GitHub renders the repository's Markdown, and with the same rule as the
 * project site (site/lib/projects/photonoxide/markdown.js): no space just inside the dollars, and
 * no letter, digit or dollar just outside.
 */
export const MATH = /(?<![\\$\w])\$(?!\s)([^$\n]+?)(?<!\s)\$(?![\w$])/g;

/**
 * The math `methodHtml` sets in a text, with the line each starts on (from 1): display math
 * between `$$` lines, and inline math on the lines outside it and outside code.
 */
export function formulas(text: string): { tex: string; display: boolean; line: number }[] {
  const out: { tex: string; display: boolean; line: number }[] = [];
  const lines = text.replaceAll("\r\n", "\n").split("\n");
  let display: { tex: string[]; line: number } | null = null;
  let code = false;
  lines.forEach((line, k) => {
    if (!display && line.startsWith("```")) code = !code;
    else if (code) return;
    else if (line.trim() === "$$") {
      if (display) out.push({ tex: display.tex.join("\n"), display: true, line: display.line });
      display = display ? null : { tex: [], line: k + 1 };
    } else if (display) display.tex.push(line);
    else for (const m of line.matchAll(MATH)) out.push({ tex: m[1], display: false, line: k + 1 });
  });
  return out;
}
