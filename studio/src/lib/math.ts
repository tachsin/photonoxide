// Text with TeX math between single dollar signs, as GitHub renders the repository's Markdown
// (the validation report's cases): each $…$ set by KaTeX, the text between escaped. The same rule
// for what counts as math as the project site's (site/lib/projects/photonoxide/markdown.js): no
// space just inside the dollars, and no letter, digit or dollar just outside.

import katex from "katex";

import { MATH } from "./formulas";

export { MATH };

const esc = (s: string) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

const cache = new Map<string, string>();

/** The text as HTML: math rendered by KaTeX (a TeX error shows the source in red), the rest escaped. */
export function mathHtml(text: string): string {
  const hit = cache.get(text);
  if (hit !== undefined) return hit;
  let html = "";
  let last = 0;
  for (const m of text.matchAll(MATH)) {
    html += esc(text.slice(last, m.index));
    html += katex.renderToString(m[1], { throwOnError: false, output: "html" });
    last = m.index + m[0].length;
  }
  html += esc(text.slice(last));
  cache.set(text, html);
  return html;
}

