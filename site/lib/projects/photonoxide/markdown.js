import katex from "katex";
import { renderMarkdown } from "@/lib/projects/highlight";
import { blobUrl, rawUrl } from "./github";
import {
  CROSS_CODE_DOCS_PATH,
  DESIGN_DOCS_PATH,
  DOCS_PATH,
  LANGUAGES_DOCS_PATH,
  METHODS_PATH,
  PYTHON_DOCS_PATH,
} from "./meta";

/** The guides that are pages of their own here, by their file in the repository. */
const GUIDES = {
  "docs/getting-started.md": DOCS_PATH,
  "docs/python.md": PYTHON_DOCS_PATH,
  "docs/languages.md": LANGUAGES_DOCS_PATH,
  "docs/cross-code.md": CROSS_CODE_DOCS_PATH,
  "docs/design.md": DESIGN_DOCS_PATH,
};

/**
 * The app's Markdown renderer, with TeX math: `$$ … $$` on lines of their
 * own for a displayed equation, `$ … $` inline, as GitHub renders them, so
 * the repository's documents read the same in both places.
 *
 * The math is set aside before the Markdown is parsed (so `_` and `*` in it
 * aren't taken for emphasis), rendered to HTML by KaTeX on the server, and
 * put back afterwards. Code, fenced or inline, is left alone. The page needs
 * KaTeX's stylesheet (katex/dist/katex.min.css).
 */

const PLACEHOLDER = (i) => `PNXMATH${i}X`;

/** Inline math, as GitHub finds it: no space just inside the dollars, no letter, digit or dollar just outside. */
const INLINE_MATH = /(?<![\\$\w])\$(?!\s)([^$\n]+?)(?<!\s)\$(?![\w$])/g;

/**
 * Plain text with inline `$ … $` math, such as a validation case's
 * description, as pieces: the text as it is, the math as KaTeX's HTML.
 * @param {string} text
 * @returns {({ text: string } | { html: string })[]}
 */
export function inlineMath(text) {
  const s = String(text ?? "");
  const parts = [];
  let last = 0;
  for (const m of s.matchAll(INLINE_MATH)) {
    if (m.index > last) parts.push({ text: s.slice(last, m.index) });
    parts.push({ html: katex.renderToString(m[1], { throwOnError: false, output: "html" }) });
    last = m.index + m[0].length;
  }
  if (last < s.length) parts.push({ text: s.slice(last) });
  return parts;
}

/** Split the source into code (fenced blocks, inline spans) and text, keeping both. */
function splitCode(source) {
  return source.split(/(```[\s\S]*?```|`[^`\n]*`)/g);
}

/**
 * @param {string} source
 * @param {{ resolveUrl?: (href: string, kind: "link" | "image") => string }} [options]
 * @returns {Promise<string>} HTML
 */
export async function renderMathMarkdown(source, options = {}) {
  const math = [];
  const stash = (tex, displayMode) => {
    math.push(katex.renderToString(tex.trim(), { displayMode, throwOnError: false, output: "html" }));
    return PLACEHOLDER(math.length - 1);
  };
  const text = splitCode(String(source ?? ""))
    .map((part, i) => {
      if (i % 2 === 1) return part; // code
      return part
        .replace(/\$\$([\s\S]+?)\$\$/g, (_, tex) => `\n\n${stash(tex, true)}\n\n`)
        .replace(INLINE_MATH, (_, tex) => stash(tex, false));
    })
    .join("");
  const html = await renderMarkdown(text, options);
  return html
    .replace(new RegExp(`<p>${PLACEHOLDER("(\\d+)")}</p>`, "g"), (_, i) => math[Number(i)])
    .replace(new RegExp(PLACEHOLDER("(\\d+)"), "g"), (_, i) => math[Number(i)]);
}

/**
 * Where a link in one of the repository's documents goes: another method's
 * Markdown (`pml.md`) to its page here, a guide with a page of its own
 * (`GUIDES`) to that page, an image to the file itself, anything else relative
 * to the document's folder to GitHub, absolute links and anchors as they are.
 * @param {string} folder  the document's folder in the repository, e.g. "docs/methods"
 */
export function repoLinkResolver(folder) {
  return (href, kind) => {
    if (!href || /^[a-z]+:/i.test(href) || href.startsWith("#") || href.startsWith("/")) return href;
    const [target, anchor] = href.split("#");
    const resolved = new URL(target, `https://x/${folder}/`).pathname.slice(1);
    const method = /^docs\/methods\/([\w-]+)\.md$/.exec(resolved);
    if (method) return `${METHODS_PATH}/${method[1]}${anchor ? `#${anchor}` : ""}`;
    if (kind === "image") return rawUrl(resolved);
    if (GUIDES[resolved]) return `${GUIDES[resolved]}${anchor ? `#${anchor}` : ""}`;
    return `${blobUrl(resolved)}${anchor ? `#${anchor}` : ""}`;
  };
}
