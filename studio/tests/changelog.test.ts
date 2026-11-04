// Home's "What's new" (src/lib/changelog.ts), run by `pnpm test`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { latestRelease } from "../src/lib/changelog.ts";

const SAMPLE = `# Changelog

Intro with a [link](https://example.org).

## [Unreleased]

## [0.6.2](https://github.com/tachsin/photonoxide/compare/v0.6.1...v0.6.2) - 2026-11-04

### <!-- 0 -->Added

- *(studio)* a Fabrication page on the rail ([#643](https://github.com/tachsin/photonoxide/pull/643))
- *(libraries)* MUMPS checked ([#312](https://x/312)) ([#647](https://x/647))
- plain line with \`code\` and [a link](https://example.org)

## [0.6.1](https://github.com/tachsin/photonoxide/compare/v0.6.0...v0.6.1) - 2026-11-01

- *(old)* not this one
`;

test("the latest release, not Unreleased, its lines as plain text", () => {
  const r = latestRelease(SAMPLE)!;
  assert.equal(r.version, "0.6.2");
  assert.equal(r.date, "2026-11-04");
  assert.deepEqual(r.lines, [
    { scope: "studio", text: "a Fabrication page on the rail" },
    { scope: "libraries", text: "MUMPS checked" },
    { scope: "", text: "plain line with code and a link" },
  ]);
});

test("no release: null", () => {
  assert.equal(latestRelease("# Changelog\n\n## [Unreleased]\n"), null);
});

test("the repository's own changelog reads", () => {
  const r = latestRelease(readFileSync(new URL("../../CHANGELOG.md", import.meta.url), "utf8"));
  assert.ok(r && /^\d+\.\d+\.\d+/.test(r.version) && r.lines.length > 0);
  for (const l of r.lines) assert.ok(!l.text.includes("](") && !l.text.includes("*("), l.text);
});
