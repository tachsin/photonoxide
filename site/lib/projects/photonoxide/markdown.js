import katex from "katex";
import { renderMarkdown } from "@/lib/projects/highlight";
import { blobUrl, rawUrl } from "./github";
import { INLINE_MATH, inlineMarkdown, shortenLinks, stashRefs } from "./inline";
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

/** TeX to KaTeX's HTML; a TeX error shows the source in red rather than failing the page. */
function renderTex(tex, displayMode = false) {
  return katex.renderToString(String(tex).trim(), { displayMode, throwOnError: false, output: "html" });
}

/**
 * A line of inline Markdown (a card's title or summary) as HTML, its math by
 * KaTeX: inline.js's `inlineMarkdown`, which says what it takes.
 * @param {string} text
 * @param {Parameters<typeof inlineMarkdown>[1]} [options]
 */
export function renderInline(text, options = {}) {
  return inlineMarkdown(text, { math: (tex) => renderTex(tex), ...options });
}

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
    parts.push({ html: renderTex(m[1]) });
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
 * Options beyond the renderer's own (`resolveUrl`), each off unless asked:
 * - `refs`: issue and pull request references (`#584`, `(#583: #584)`) as
 *   small linked chips (inline.js, `stashRefs`);
 * - `compact`: a link whose text is a DOI or a bare URL shows "DOI" or its
 *   host, the whole address kept (inline.js, `shortenLinks`);
 * - `notes`: an italic parenthesis, `*( … )*`, the roadmap's notes on what
 *   was done, as a quieter note of its own (`pnx-note`) under its line.
 * The pages style the chips and notes with `RichText.module.css`.
 *
 * @param {string} source
 * @param {{
 *   resolveUrl?: (href: string, kind: "link" | "image") => string,
 *   refs?: boolean, compact?: boolean, notes?: boolean,
 * }} [options]
 * @returns {Promise<string>} HTML
 */
export async function renderMathMarkdown(source, options = {}) {
  const { refs = false, compact = false, notes = false, ...rendererOptions } = options;
  const stashed = [];
  const stashHtml = (html) => {
    stashed.push(html);
    return PLACEHOLDER(stashed.length - 1);
  };
  const stash = (tex, displayMode) => stashHtml(renderTex(tex, displayMode));
  const text = splitCode(String(source ?? ""))
    .map((part, i) => {
      if (i % 2 === 1) return part; // code
      const withMath = part
        .replace(/\$\$([\s\S]+?)\$\$/g, (_, tex) => `\n\n${stash(tex, true)}\n\n`)
        .replace(INLINE_MATH, (_, tex) => stash(tex, false));
      return refs ? stashRefs(withMath, stashHtml) : withMath;
    })
    .join("");
  let html = await renderMarkdown(text, rendererOptions);
  if (compact) html = shortenLinks(html);
  // a note is one emphasis, with none inside: a stray asterisk in the text ("φ*") can leave
  // an emphasis open, and the note must never reach across into the next one
  if (notes) html = html.replace(/<em>\(((?:(?!<\/?em>)[\s\S])*?)\)<\/em>/g, '<span class="pnx-note">$1</span>');
  return html
    .replace(new RegExp(`<p>${PLACEHOLDER("(\\d+)")}</p>`, "g"), (_, i) => stashed[Number(i)])
    .replace(new RegExp(PLACEHOLDER("(\\d+)"), "g"), (_, i) => stashed[Number(i)]);
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
    // the root's documents (folder "") resolve from "/", not "//"
    const resolved = new URL(target, `https://x/${folder ? `${folder}/` : ""}`).pathname.slice(1);
    const method = /^docs\/methods\/([\w-]+)\.md$/.exec(resolved);
    if (method) return `${METHODS_PATH}/${method[1]}${anchor ? `#${anchor}` : ""}`;
    if (kind === "image") return rawUrl(resolved);
    if (GUIDES[resolved]) return `${GUIDES[resolved]}${anchor ? `#${anchor}` : ""}`;
    return `${blobUrl(resolved)}${anchor ? `#${anchor}` : ""}`;
  };
}
