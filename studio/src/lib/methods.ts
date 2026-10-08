// The methods' write-ups (docs/methods/*.md, built into the program): which ones a kind of job
// solves by, their front matter (title, module, papers), and their text as HTML with its math.

import katex from "katex";

import { MATH } from "./math";

export interface MethodDoc {
  /** The write-up's file, e.g. "vector.md". */
  file: string;
  title: string;
  /** The library module that implements it, e.g. "mode::vector". */
  module: string;
  summary: string;
  /** The papers it implements, each with its DOI. */
  papers: { cite: string; doi: string }[];
  /** The write-up itself, Markdown with TeX math. */
  body: string;
}

/** The write-ups a kind of job solves by, the solver's own first. */
export const METHODS: Record<string, string[]> = {
  modes: ["vector.md", "eigen.md", "walls.md"],
  fdfd: ["fdfd.md", "fdfd-ports.md", "eim.md", "slab.md", "pml.md"],
  structure: [],
};

/** A solver's module as its run records it, in words. */
export const SOLVERS: Record<string, string> = {
  "mode::vector": "full-vector finite differences",
  fdfd: "2D FDFD, direct",
};

const unquote = (s: string) => s.trim().replace(/^"(.*)"$/, "$1").replaceAll('\\"', '"');

/** A write-up's file as a MethodDoc: its front matter read, the rest its body. */
export function parseMethod(file: string, text: string): MethodDoc {
  const doc: MethodDoc = { file, title: file, module: "", summary: "", papers: [], body: text };
  const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(text);
  if (!m) return doc;
  doc.body = text.slice(m[0].length);
  let list = "";
  for (const line of m[1].split(/\r?\n/)) {
    const top = /^(\w+):\s*(.*)$/.exec(line);
    if (top) {
      list = top[2] ? "" : top[1];
      if (top[1] === "title") doc.title = unquote(top[2]);
      else if (top[1] === "module") doc.module = unquote(top[2]);
      else if (top[1] === "summary") doc.summary = unquote(top[2]);
      continue;
    }
    if (list !== "papers") continue;
    const cite = /^\s*-\s*cite:\s*(.*)$/.exec(line);
    const doi = /^\s*doi:\s*(.*)$/.exec(line);
    if (cite) doc.papers.push({ cite: unquote(cite[1]), doi: "" });
    else if (doi && doc.papers.length) doc.papers[doc.papers.length - 1].doi = unquote(doi[1]);
  }
  return doc;
}

const esc = (s: string) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

/**
 * Text between math: code, bold, and links. A paper's or a site's opens outside; another
 * write-up's, or a lesson's (the Academy's, beside its own: `ring-resonator.md`, and
 * `../docs/methods/components.md`), in place; an example's (`../examples/<name>.rs`) on the
 * Examples page.
 */
function marks(s: string): string {
  return esc(s)
    .replace(/`([^`]+)`/g, '<code class="rounded bg-base-content/8 px-1 font-mono text-[0.9em]">$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a class="link link-primary" data-href="$2">$1</a>')
    .replace(/\[([^\]]+)\]\((?:\.\.\/docs\/methods\/)?([\w-]+\.md)(?:#[^)]*)?\)/g, '<a class="link" data-doc="$2">$1</a>')
    .replace(/\[([^\]]+)\]\(\.\.\/examples\/(\w+)\.rs\)/g, '<a class="link" data-example="$2">$1</a>')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1");
}

/** A line of text as HTML: its $…$ math by KaTeX, the rest with its marks. */
function inline(s: string): string {
  let html = "";
  let last = 0;
  for (const m of s.matchAll(MATH)) {
    html += marks(s.slice(last, m.index));
    html += katex.renderToString(m[1], { throwOnError: false, output: "html" });
    last = m.index + m[0].length;
  }
  return html + marks(s.slice(last));
}

const cache = new Map<string, string>();

/**
 * A write-up's body as HTML: headings, paragraphs, bullets, tables, code, and math, inline
 * ($…$) and displayed ($$ on lines of their own), as the project site sets the same files.
 */
export function methodHtml(body: string): string {
  const hit = cache.get(body);
  if (hit !== undefined) return hit;
  const lines = body.replaceAll("\r\n", "\n").split("\n");
  const out: string[] = [];
  let k = 0;
  const until = (stop: (line: string) => boolean): string[] => {
    const block: string[] = [];
    while (k < lines.length && !stop(lines[k])) block.push(lines[k++]);
    return block;
  };
  while (k < lines.length) {
    const line = lines[k];
    if (!line.trim()) {
      k++;
    } else if (line.trim() === "$$") {
      k++;
      const tex = until((l) => l.trim() === "$$").join("\n");
      k++;
      out.push(`<div class="my-3 overflow-x-auto">${katex.renderToString(tex, { throwOnError: false, output: "html", displayMode: true })}</div>`);
    } else if (line.startsWith("```")) {
      k++;
      const code = until((l) => l.startsWith("```")).join("\n");
      k++;
      out.push(`<pre class="my-2 overflow-x-auto rounded-lg bg-base-300/60 p-3 font-mono text-xs">${esc(code)}</pre>`);
    } else if (/^#{1,4} /.test(line)) {
      const h = /^(#{1,4}) (.*)$/.exec(line)!;
      out.push(`<h4 class="${h[1].length <= 2 ? "text-base" : "text-sm"} mt-5 mb-1.5 font-semibold">${inline(h[2])}</h4>`);
      k++;
    } else if (line.startsWith("|")) {
      const rows = until((l) => !l.startsWith("|")).map((r) => r.replace(/^\||\|\s*$/g, "").split("|").map((c) => c.trim()));
      const body = rows.filter((r) => !r.every((c) => /^:?-+:?$/.test(c)));
      const cells = (r: string[], tag: string) => r.map((c) => `<${tag} class="px-2 py-1 text-left">${inline(c)}</${tag}>`).join("");
      out.push(
        `<div class="my-2 overflow-x-auto"><table class="table table-xs w-auto"><thead><tr>${cells(body[0] ?? [], "th")}</tr></thead><tbody>${body
          .slice(1)
          .map((r) => `<tr>${cells(r, "td")}</tr>`)
          .join("")}</tbody></table></div>`,
      );
    } else if (/^\s*[-*] /.test(line)) {
      // bullets, each with the lines that continue it
      const items: string[] = [];
      while (k < lines.length && (/^\s*[-*] /.test(lines[k]) || (/^\s+\S/.test(lines[k]) && items.length))) {
        const bullet = /^\s*[-*] (.*)$/.exec(lines[k]);
        if (bullet) items.push(bullet[1]);
        else items[items.length - 1] += ` ${lines[k].trim()}`;
        k++;
      }
      out.push(`<ul class="my-2 list-disc space-y-1 pl-5">${items.map((i) => `<li>${inline(i)}</li>`).join("")}</ul>`);
    } else {
      const text = until((l) => !l.trim() || l.trim() === "$$" || l.startsWith("```") || /^#{1,4} /.test(l) || l.startsWith("|") || /^\s*[-*] /.test(l)).join(" ");
      out.push(`<p class="my-2 leading-relaxed">${inline(text)}</p>`);
    }
  }
  const html = out.join("");
  cache.set(body, html);
  return html;
}
