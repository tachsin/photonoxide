import { cache } from "react";
import { getRepoFile, getRepoFiles } from "./github";

/**
 * photonoxide's examples, read from the repository at the pinned commit
 * (see github.js). Each is one file, examples/<name>.rs, that reproduces a
 * published result and fails when it disagrees (examples/README.md):
 *
 *   examples/<name>.rs          its doc comment (`//!` lines) is its text:
 *                               the first sentence the title, the rest the
 *                               paper and what is checked; then the code
 *   examples/output/<name>.txt  what it prints, which CI checks
 *
 * examples/common/ is the code they share, not an example.
 */

/**
 * @typedef {object} Example
 * @property {string} slug     URL segment, e.g. "strip-waveguide"
 * @property {string} name     the cargo example's name, e.g. "strip_waveguide"
 * @property {string} path     examples/<name>.rs
 * @property {string} title
 * @property {string} summary  the doc comment's first paragraph after the title
 * @property {string} doc      the doc comment as Markdown, without its title line
 * @property {string} code     the file without its doc comment
 * @property {string | null} output
 */

const slugFor = (name) => name.replace(/_/g, "-");

/** Split a Rust file's leading `//!` doc comment from its code. */
function splitDoc(source) {
  const lines = String(source ?? "").split(/\r?\n/);
  let end = 0;
  while (end < lines.length && (lines[end].startsWith("//!") || (end > 0 && lines[end].trim() === "" && lines[end + 1]?.startsWith("//!")))) end++;
  const doc = lines
    .slice(0, end)
    .map((l) => l.replace(/^\/\/! ?/, ""))
    .join("\n");
  const code = lines.slice(end).join("\n").replace(/^\s*\n/, "");
  return { doc, code };
}

function parse(name, source, output) {
  const { doc, code } = splitDoc(source);
  const [first, ...rest] = doc.split(/\n\s*\n/);
  const title = (first ?? name).replace(/\s+/g, " ").trim().replace(/\.$/, "");
  const body = rest.join("\n\n");
  const summary = (rest[0] ?? "").replace(/\s+/g, " ").trim();
  return { slug: slugFor(name), name, path: `examples/${name}.rs`, title, summary, doc: body, code, output };
}

/** @returns {Promise<{ ok: boolean, examples: Example[] }>} ok is false when GitHub can't be reached */
export const getExamples = cache(async () => {
  const files = await getRepoFiles();
  if (!files) return { ok: false, examples: [] };
  const names = [...files]
    .map((p) => /^examples\/([\w-]+)\.rs$/.exec(p)?.[1])
    .filter(Boolean)
    .sort();
  const loaded = await Promise.all(
    names.map(async (name) => {
      const out = `examples/output/${name}.txt`;
      const [source, output] = await Promise.all([
        getRepoFile(`examples/${name}.rs`),
        files.has(out) ? getRepoFile(out) : Promise.resolve(null),
      ]);
      return source == null ? null : parse(name, source, output);
    }),
  );
  const examples = loaded.filter(Boolean);
  return { ok: examples.length > 0, examples };
});

/** @param {string} slug @returns {Promise<Example | null>} */
export async function getExample(slug) {
  const { examples } = await getExamples();
  return examples.find((e) => e.slug === slug) ?? null;
}

/** The command that runs an example. */
export function runCommand(example) {
  return `cargo run --release --example ${example.name}`;
}
