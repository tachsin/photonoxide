/**
 * The small pieces of text the pages print outside a whole document: an
 * example's title and summary on its card, the roadmap's references to
 * issues and papers. Plain functions with no import at all, so the studio's
 * tests (studio/tests/site-inline.test.ts) run them as they are; KaTeX is
 * passed in (markdown.js, `renderInline`).
 *
 * - Inline Markdown: code spans, links, emphasis and `$ … $` math, the rest
 *   escaped, so a card prints what the Markdown says, not its asterisks and
 *   brackets.
 * - Issue and pull request references (`#584`, `(#583: #584)`) as small
 *   linked chips, to the repository's issues (GitHub redirects a pull
 *   request's number there to the pull request).
 * - Links whose text is a DOI or a URL shortened to a few letters, the whole
 *   address kept in the link and its title.
 */

export const ISSUES_URL = "https://github.com/tachsin/photonoxide/issues/";

/** Inline math, as GitHub finds it: no space just inside the dollars, no letter, digit or dollar just outside. */
export const INLINE_MATH = /(?<![\\$\w])\$(?!\s)([^$\n]+?)(?<!\s)\$(?![\w$])/g;

const UNSAFE_URL = /^\s*(javascript|vbscript|data):/i;

/** @param {unknown} text */
export function escapeHtml(text) {
  return String(text ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

// ---------- issue and pull request references ----------

/** `#123`, not in a word, a path, an anchor (`#06-the-…`) or a link's text. */
const REF = /(?<![\w/#&[\]-])#(\d+)(?![\w-])/g;
const SEP = String.raw`(?:, and |, |; |: | to | and )`;
/** A parenthesis holding only references: `(#584)`, `(#583: #584)`, `(#565 to #567, #570)`. */
const REF_GROUP = new RegExp(String.raw`\((#\d+(?:${SEP}#\d+)*)\)`, "g");
/** A Markdown link, `[text](url)`, a parenthesis deep in its URL at most. */
const MD_LINK = /(\[[^\]\n]*\]\((?:[^()\s]|\([^()\s]*\))*\))/g;

/** One reference as a chip. */
export function refChip(n) {
  const num = String(n);
  return `<a class="pnx-ref" href="${ISSUES_URL}${num}" title="Issue or pull request #${num} on GitHub">#${num}</a>`;
}

function refGroup(inner) {
  const html = inner.replace(/#(\d+)|(, and |, |; |: | to | and )/g, (_, n, sep) =>
    n ? refChip(n) : `<span class="pnx-ref-sep">${escapeHtml(sep.trim())}</span>`,
  );
  return `<span class="pnx-refs">${html}</span>`;
}

/**
 * Turn the references in Markdown text (no code in it) into chips, each set
 * aside by `stash` (which returns a placeholder the Markdown leaves alone) and
 * put back once the Markdown is HTML. A parenthesis of references only loses
 * its brackets: the chips are their own brackets. Links are left as they are.
 * @param {string} text
 * @param {(html: string) => string} stash
 */
export function stashRefs(text, stash) {
  return String(text)
    .split(MD_LINK)
    .map((part, i) => {
      if (i % 2 === 1) return part; // a link
      return part
        .replace(REF_GROUP, (whole, inner, offset, all) =>
          // `(#1)` right after a word is a reference in brackets, still
          /[\\[]/.test(all[offset - 1] ?? "") ? whole : stash(refGroup(inner)),
        )
        .replace(REF, (_, n) => stash(refChip(n)));
    })
    .join("");
}

// ---------- short links ----------

const DOI_TEXT = /^(?:doi:\s*|https?:\/\/(?:dx\.)?doi\.org\/)?(10\.\d{4,9}\/\S+)$/i;
const URL_TEXT = /^(?:https?:\/\/|www\.)\S+$/i;

function decodeEntities(s) {
  return s
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&");
}

/** A URL in a few characters: its host and as much of its path, folder by folder, as fits in 32. */
export function shortUrl(url, max = 32) {
  const bare = String(url).replace(/^https?:\/\//i, "").replace(/^www\./i, "").replace(/\/$/, "");
  if (bare.length <= max) return bare;
  const [host, ...path] = bare.split("/");
  let short = host;
  for (const segment of path) {
    if (`${short}/${segment}`.length > max) break;
    short = `${short}/${segment}`;
  }
  return `${short.length > max ? short.slice(0, max - 1) : short}/…`;
}

/**
 * In rendered HTML, a link whose text is a DOI or a bare URL keeps its
 * address but shows a short label: "DOI" for a DOI, the host for a URL. The
 * full text moves to the link's title, so nothing is lost.
 * @param {string} html
 */
export function shortenLinks(html) {
  return String(html).replace(/<a href="([^"]*)"([^>]*)>([^<]*)<\/a>/g, (whole, href, attrs, text) => {
    if (/\bclass="/.test(attrs)) return whole;
    const shown = decodeEntities(text).trim();
    const doi = DOI_TEXT.exec(shown);
    if (doi) {
      return `<a href="${href}"${attrs} class="pnx-doi" title="doi:${escapeHtml(doi[1])}">DOI</a>`;
    }
    if (URL_TEXT.test(shown)) {
      const short = shortUrl(shown);
      if (short === shown) return whole;
      return `<a href="${href}"${attrs} class="pnx-url" title="${escapeHtml(shown)}">${escapeHtml(short)}</a>`;
    }
    return whole;
  });
}

// ---------- inline Markdown ----------

/** Split text into code spans (odd indices) and the rest (even). */
function splitCodeSpans(text) {
  return String(text ?? "").split(/(`[^`\n]+`)/g);
}

/**
 * Inline Markdown to HTML: code spans, `[text](url)` links, `**strong**`,
 * `*em*` and `_em_`, `$ … $` math, the rest escaped. One line of a card, so
 * no blocks. With `links: false` a link prints its text only (inside another
 * link); with `refs`, issue references become chips; `resolveUrl` rewrites a
 * relative link; `math` renders TeX (KaTeX on the pages; without it the TeX
 * is printed as it is).
 * @param {string} text
 * @param {{
 *   links?: boolean, refs?: boolean, resolveUrl?: (href: string) => string,
 *   math?: (tex: string) => string,
 * }} [options]
 * @returns {string} HTML
 */
export function inlineMarkdown(text, options = {}) {
  const { links = true, refs = false, resolveUrl, math = (tex) => `<span class="pnx-tex">${escapeHtml(tex)}</span>` } = options;
  const stashed = [];
  const stash = (html) => {
    stashed.push(html);
    return `\u0000${stashed.length - 1}\u0000`;
  };
  const out = splitCodeSpans(text)
    .map((part, i) => {
      if (i % 2 === 1) return stash(`<code>${escapeHtml(part.slice(1, -1))}</code>`);
      let s = part.replace(INLINE_MATH, (_, tex) => stash(math(tex)));
      s = s.replace(/\[([^\]\n]+)\]\(([^()\s]*(?:\([^()\s]*\)[^()\s]*)*)\)/g, (_, label, href) => {
        const inner = inlineMarkdown(label, { links: false, math });
        if (!links) return stash(inner);
        let url = UNSAFE_URL.test(href) ? "#" : href;
        if (resolveUrl) url = resolveUrl(url);
        return stash(shortenLinks(`<a href="${escapeHtml(url)}">${inner}</a>`));
      });
      if (refs && links) s = stashRefs(s, stash);
      s = escapeHtml(s)
        .replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, "<strong>$1</strong>")
        .replace(/(?<![\w*])\*(?=\S)([^*]*?\S)\*(?![\w*])/g, "<em>$1</em>")
        .replace(/(?<![\w_])_(?=\S)([^_]*?\S)_(?![\w_])/g, "<em>$1</em>");
      return s;
    })
    .join("");
  // stashed pieces may hold placeholders of their own (a link's label)
  let html = out;
  for (let pass = 0; pass < 3 && html.includes("\u0000"); pass++) {
    html = html.replace(/\u0000(\d+)\u0000/g, (_, i) => stashed[Number(i)]);
  }
  return html;
}

const TEX_WORDS = {
  alpha: "α", beta: "β", gamma: "γ", Gamma: "Γ", delta: "δ", Delta: "Δ", epsilon: "ε", varepsilon: "ε",
  zeta: "ζ", eta: "η", theta: "θ", Theta: "Θ", kappa: "κ", lambda: "λ", Lambda: "Λ", mu: "µ", nu: "ν",
  xi: "ξ", pi: "π", Pi: "Π", rho: "ρ", sigma: "σ", Sigma: "Σ", tau: "τ", phi: "φ", varphi: "φ", Phi: "Φ",
  chi: "χ", psi: "ψ", Psi: "Ψ", omega: "ω", Omega: "Ω", partial: "∂", nabla: "∇", infty: "∞",
  times: "×", cdot: "·", pm: "±", mp: "∓", le: "≤", leq: "≤", ge: "≥", geq: "≥", ne: "≠", neq: "≠",
  approx: "≈", sim: "∼", propto: "∝", to: "→", rightarrow: "→", leftarrow: "←", sum: "Σ", prod: "Π",
  int: "∫", sqrt: "√", ell: "ℓ", hbar: "ħ", circ: "°", degree: "°",
};

/** TeX as readable plain text, for a page's title or description: Greek letters and operators as characters, the rest unbraced. */
export function texToText(tex) {
  return String(tex)
    .replace(/\\(?:mathrm|text|mathit|operatorname|mathbf|boldsymbol)\{([^{}]*)\}/g, "$1")
    .replace(/\\frac\{([^{}]*)\}\{([^{}]*)\}/g, "($1)/($2)")
    .replace(/\\([A-Za-z]+)/g, (whole, word) => TEX_WORDS[word] ?? word)
    .replace(/\\[,;:! ]/g, " ")
    .replace(/[{}]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/** Inline Markdown as plain text: for metadata, breadcrumbs and title attributes. */
export function plainText(text) {
  return splitCodeSpans(text)
    .map((part, i) => {
      if (i % 2 === 1) return part.slice(1, -1);
      return part
        .replace(INLINE_MATH, (_, tex) => texToText(tex))
        .replace(/\[([^\]\n]+)\]\([^)\s]*(?:\([^)\s]*\)[^)\s]*)*\)/g, "$1")
        .replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, "$1")
        .replace(/(?<![\w*])\*(?=\S)([^*]*?\S)\*(?![\w*])/g, "$1")
        .replace(/(?<![\w_])_(?=\S)([^_]*?\S)_(?![\w_])/g, "$1");
    })
    .join("")
    .replace(/\s+/g, " ")
    .trim();
}

/** Where `$ … $` math and code spans are in text, so a split never falls inside one. */
function protectedRanges(text) {
  const ranges = [];
  for (const re of [/`[^`\n]+`/g, new RegExp(INLINE_MATH.source, "g")]) {
    for (const m of text.matchAll(re)) ranges.push([m.index, m.index + m[0].length]);
  }
  return ranges;
}

/**
 * An example's title is its doc comment's first sentence, which can run to a
 * few lines. On a card and as a page's heading it reads better as a head and
 * the rest: a title of up to `max` characters stays whole; a longer one is
 * split at its first colon, or else its first comma, when what comes before
 * is a phrase of its own (`min` to `max` characters). Nothing is dropped:
 * the page prints the rest under its heading.
 * @param {string} title
 * @returns {{ head: string, rest: string }}
 */
export function splitTitle(title, { min = 15, max = 90 } = {}) {
  const t = String(title ?? "").trim();
  if (t.length <= max) return { head: t, rest: "" };
  const ranges = protectedRanges(t);
  const inside = (i) => ranges.some(([a, b]) => i > a && i < b);
  for (const sep of [": ", ", "]) {
    let at = t.indexOf(sep);
    while (at >= 0 && inside(at)) at = t.indexOf(sep, at + 1);
    if (at >= min && at <= max) return { head: t.slice(0, at), rest: t.slice(at + sep.length).trim() };
  }
  return { head: t, rest: "" };
}
