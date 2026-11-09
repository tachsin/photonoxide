// The site's text pieces (site/lib/projects/photonoxide/inline.js and milestones.js): issue
// chips, short DOI links, a card's inline Markdown, a long title's head, and ROADMAP.md read into
// sections and milestones, on the repository's own files. And every formula the examples' doc
// comments write as `$ … $` renders with KaTeX, as the example pages render it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import katex from "katex";

import {
  INLINE_MATH,
  inlineMarkdown,
  plainText,
  refChip,
  shortenLinks,
  shortUrl,
  splitTitle,
  stashRefs,
  texToText,
} from "../../site/lib/projects/photonoxide/inline.js";
import { headingAnchor, milestoneAnchor, parseMilestones, splitRoadmap } from "../../site/lib/projects/photonoxide/milestones.js";

const repo = (path: string) => readFileSync(fileURLToPath(new URL(`../../${path}`, import.meta.url)), "utf8");

/** stashRefs with the stashed HTML kept beside the text, as markdown.js does. */
function refs(text: string) {
  const stashed: string[] = [];
  const out = stashRefs(text, (html: string) => {
    stashed.push(html);
    return `@${stashed.length - 1}@`;
  });
  return { out, stashed };
}

test("issue and pull request references become chips; a parenthesis of them loses its brackets", () => {
  const { out, stashed } = refs("done (#583: #584), and #338's P4 (#556, #565 to #567)");
  assert.equal(out, "done @0@, and @2@'s P4 @1@");
  assert.match(stashed[0], /^<span class="pnx-refs"><a class="pnx-ref" href="https:\/\/github\.com\/tachsin\/photonoxide\/issues\/583"/);
  assert.match(stashed[0], /<span class="pnx-ref-sep">:<\/span><a class="pnx-ref" [^>]*>#584<\/a><\/span>$/);
  assert.match(stashed[1], /#556<\/a><span class="pnx-ref-sep">,<\/span>.*#565<\/a><span class="pnx-ref-sep">to<\/span>.*#567<\/a>/);
  assert.equal(stashed[2], refChip(338));
});

test("references are left alone in links, anchors, words and other repositories", () => {
  for (const text of [
    "[the plan](docs/plans/backends.md#06-the-fronts)",
    "[#7](https://example.org)",
    "see docs/baselines.md#186-x",
    "tachsin#662 and abc#12",
    "#12a and #3-b",
  ]) {
    assert.equal(refs(text).out, text, text);
  }
});

test("a link whose text is a DOI or a bare URL shows a short label and keeps the address", () => {
  assert.equal(
    shortenLinks('<a href="https://doi.org/10.1109/TAP.1966.1138693">10.1109/TAP.1966.1138693</a>'),
    '<a href="https://doi.org/10.1109/TAP.1966.1138693" class="pnx-doi" title="doi:10.1109/TAP.1966.1138693">DOI</a>',
  );
  assert.match(shortenLinks('<a href="https://doi.org/10.1/x">doi:10.1038/s41598-021-04598-6</a>'), /class="pnx-doi" title="doi:10\.1038\/s41598-021-04598-6">DOI</);
  const url = "https://github.com/tachsin/genoxide/blob/main/README.md";
  assert.equal(
    shortenLinks(`<a href="${url}">${url}</a>`),
    `<a href="${url}" class="pnx-url" title="${url}">github.com/tachsin/genoxide/blob/…</a>`,
  );
  // a link's own words, and links already styled, stay as they are
  for (const html of ['<a href="/x">the report</a>', '<a href="https://github.com">github.com</a>', refChip(5)]) {
    assert.equal(shortenLinks(html), html);
  }
  assert.match(shortenLinks('<a href="https://github.com">https://github.com</a>'), />github\.com<\/a>$/);
  assert.equal(shortUrl("https://example.org/"), "example.org");
});

test("a card's inline Markdown: the DOI a short link, emphasis, code and math rendered, the rest escaped", () => {
  const card =
    "M. Wang, *Sci. Rep.* 12, 1482 (2022), [doi:10.1038/s41598-021-04598-6](https://doi.org/10.1038/s41598-021-04598-6). " +
    "`k_n < 1` and $w_{n+1}$ <b> (#12)";
  const html = inlineMarkdown(card, { refs: true, math: (tex: string) => `[${tex}]` });
  assert.equal(
    html,
    'M. Wang, <em>Sci. Rep.</em> 12, 1482 (2022), <a href="https://doi.org/10.1038/s41598-021-04598-6" class="pnx-doi" ' +
      'title="doi:10.1038/s41598-021-04598-6">DOI</a>. <code>k_n &lt; 1</code> and [w_{n+1}] &lt;b&gt; ' +
      `<span class="pnx-refs">${refChip(12)}</span>`,
  );
  // inside another link (a card's heading) a link keeps only its text
  assert.equal(inlineMarkdown("see [the paper](https://x.org)", { links: false }), "see the paper");
  assert.equal(inlineMarkdown("[x](javascript:alert(1))"), '<a href="#">x</a>');
  // without KaTeX the TeX is printed, escaped
  assert.equal(inlineMarkdown("$a<b$"), '<span class="pnx-tex">a&lt;b</span>');
  // a dollar as currency or in code is no math
  assert.equal(inlineMarkdown("costs $5 and $10, `$x$`"), "costs $5 and $10, <code>$x$</code>");
});

test("plain text for metadata: links to their words, math to readable characters", () => {
  assert.equal(plainText("How a PML's reflection falls, for $\\sigma$ graded as $(x/L)^d$, [doi](https://x)"), "How a PML's reflection falls, for σ graded as (x/L)^d, doi");
  assert.equal(texToText("\\varepsilon = 12"), "ε = 12");
  assert.equal(texToText("n_\\mathrm{eff}"), "n_eff");
});

test("a long title splits into a head and the rest, never inside math or code", () => {
  const wang =
    "Wang et al.'s double-ring-loaded Mach–Zehnder filter: the fourth-order elliptic low-passes it is synthesized for, derived here; its Fig. 4 circuit solved; and the circuit optimized towards them by Nelder–Mead, as in the paper";
  assert.deepEqual(splitTitle(wang), {
    head: "Wang et al.'s double-ring-loaded Mach–Zehnder filter",
    rest: "the fourth-order elliptic low-passes it is synthesized for, derived here; its Fig. 4 circuit solved; and the circuit optimized towards them by Nelder–Mead, as in the paper",
  });
  const strip = "The TE-like mode of a 500 × 220 nm silicon strip in oxide at 1550 nm, with the full-vector mode solver, on halving grids, and its grid study";
  assert.equal(splitTitle(strip).head, "The TE-like mode of a 500 × 220 nm silicon strip in oxide at 1550 nm");
  assert.deepEqual(splitTitle("A tunable splitter"), { head: "A tunable splitter", rest: "" });
  const math = `${"x".repeat(20)} $f(a, b): g$ ${"y".repeat(80)}`;
  assert.deepEqual(splitTitle(math), { head: math, rest: "" });
});

test("ROADMAP.md is read whole into sections, blocks and milestones", () => {
  const markdown = repo("ROADMAP.md");
  const { title, preface, sections } = splitRoadmap(markdown);
  assert.equal(title, "photonoxide roadmap");
  // every line but the title is in exactly one place
  const nonBlank = (s: string) => s.split("\n").filter((l) => l.trim()).length;
  let lines = 1 + nonBlank(preface);
  for (const s of sections) {
    lines += 1 + nonBlank(s.intro);
    for (const b of s.blocks) lines += 1 + nonBlank(b.body);
  }
  assert.equal(lines, markdown.split(/\r?\n/).filter((l) => l.trim()).length);

  const milestones = parseMilestones(markdown);
  const inBlocks = sections.flatMap((s) => s.blocks).filter((b) => b.milestone);
  assert.deepEqual(
    inBlocks.map((b) => b.milestone.version),
    milestones.map((m) => m.version),
  );
  assert.ok(sections.some((s) => s.heading === "Milestones"), "a Milestones section");
  assert.equal(milestones.filter((m) => m.status === "next").length, 1);
  const next = milestones.findIndex((m) => m.status === "next");
  assert.ok(milestones.slice(0, next).every((m) => m.status === "done"));
  // the anchors the page had when it printed ROADMAP.md whole
  for (const b of inBlocks) assert.equal(b.anchor, headingAnchor(b.heading));
  assert.equal(milestoneAnchor({ version: "0.5.4", title: "Geometry and devices" }), "054-geometry-and-devices");
});

test("the milestones' items and counts", () => {
  const md = [
    "## Milestones",
    "### 0.1: First ✅",
    "- [x] **One:** a",
    "- [ ] Two, b",
    "### Throughout",
    "- [ ] not counted",
    "### 0.2: Second",
    "```",
    "### 9.9: not a heading",
    "```",
    "- [x] **Three**",
  ].join("\n");
  const [first, second] = parseMilestones(md);
  assert.deepEqual(first, { version: "0.1", title: "First", status: "done", items: ["One", "Two, b"], done: 1, total: 2 });
  assert.equal(second.status, "next");
  assert.deepEqual(second.items, ["Three"]);
  assert.equal(parseMilestones(md).length, 2);
});

test("every formula in the examples' doc comments renders with KaTeX", () => {
  const dir = fileURLToPath(new URL("../../examples/", import.meta.url));
  let formulas = 0;
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".rs"))) {
    const lines = readFileSync(`${dir}${file}`, "utf8").split(/\r?\n/);
    for (let i = 0; i < lines.length && lines[i].startsWith("//!"); i++) {
      const text = lines[i].replace(/`[^`]*`/g, "");
      for (const m of text.matchAll(INLINE_MATH)) {
        formulas += 1;
        assert.doesNotThrow(() => katex.renderToString(m[1], { throwOnError: true }), `${file}:${i + 1}: ${m[1]}`);
      }
      // a dollar left over is a formula that lost its other half
      assert.ok(!text.replace(INLINE_MATH, "").includes("$"), `${file}:${i + 1}: an unpaired $`);
    }
  }
  assert.ok(formulas > 100, `${formulas} formulas`);
});
