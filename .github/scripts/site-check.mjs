#!/usr/bin/env node
// Does tachsin.gr show the latest release? Compares the live project site with GitHub's latest
// release and the repository's ROADMAP.md at its tag, from outside, as anyone can see it:
//
//   - the overview names the release ("0.6.3 is the latest release on crates.io");
//   - the roadmap page (and the overview's milestone cards) show every milestone as ROADMAP.md at
//     the tag does, the release's own milestone done (site/lib/projects/photonoxide/milestones.js);
//   - the Python API reference is the release's (its `__version__`).
//
//   node .github/scripts/site-check.mjs [--wait <minutes>] [--site <url>] [--release vX.Y.Z]
//
// `just site-check` runs it; the Site check workflow runs it daily and after a release. --wait
// tries again every minute until the pages agree or the minutes are up (the site looks the
// release up every ten minutes). It never deploys anything: when the pages disagree it says what
// they show and what to do. It can't see the site's code (which commit tachsin.gr is built from),
// only what the pages say. GITHUB_TOKEN, when set, is sent to GitHub's API.
//
// Exit status: 0 the site agrees, 1 it doesn't, 2 GitHub or the site couldn't be read.

import { parseMilestones } from "../../site/lib/projects/photonoxide/milestones.js";
import { LABEL, shownStatuses, text } from "./site-check-parse.mjs";

const REPO = "tachsin/photonoxide";
const args = process.argv.slice(2);
const option = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};
const SITE = option("--site", "https://tachsin.gr/projects/photonoxide").replace(/\/$/, "");
const WAIT_MINUTES = Number(option("--wait", "0"));
const RELEASE = option("--release", null);

class Unreadable extends Error {}

async function get(url, { json = false, github = false } = {}) {
  const headers = { "User-Agent": "photonoxide site-check (+https://github.com/tachsin/photonoxide)" };
  if (github) {
    headers.Accept = "application/vnd.github+json";
    if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  }
  let res;
  try {
    res = await fetch(url, { headers, cache: "no-store", signal: AbortSignal.timeout(30000) });
  } catch (e) {
    throw new Unreadable(`${url}: ${e.cause?.code ?? e.message}`);
  }
  if (!res.ok) throw new Unreadable(`${url}: HTTP ${res.status}`);
  return json ? res.json() : res.text();
}

async function check() {
  const tag = RELEASE ?? (await get(`https://api.github.com/repos/${REPO}/releases/latest`, { json: true, github: true })).tag_name;
  const version = String(tag).replace(/^v/, "");
  const roadmap = await get(`https://raw.githubusercontent.com/${REPO}/${tag}/ROADMAP.md`);
  const expected = parseMilestones(roadmap, version);
  const [overview, roadmapPage, reference] = await Promise.all([
    get(SITE).then(text),
    get(`${SITE}/roadmap`).then(text),
    get(`${SITE}/docs/python/reference`).then(text),
  ]);
  const problems = [];

  // the overview's release
  const named = /(\d+\.\d+\.\d+) is the latest release on crates\.io/.exec(overview)?.[1] ?? null;
  if (named !== version) {
    problems.push(
      named
        ? `the overview still shows ${named} as the latest release; GitHub's latest is ${version}`
        : `the overview names no latest release (GitHub's latest is ${version})`,
    );
  }

  // the milestones, on the roadmap page and the overview's cards
  for (const [page, pageText, all] of [
    ["the roadmap page", roadmapPage, true],
    ["the overview", overview, false],
  ]) {
    const shown = shownStatuses(pageText, expected);
    const wrong = [];
    for (const m of expected) {
      const s = shown.get(m.version);
      if (!s) {
        if (all) wrong.push(`${m.version} ${m.title}: not shown (expected ${LABEL[m.status]})`);
      } else if (s !== LABEL[m.status]) {
        wrong.push(`${m.version} ${m.title}: shows ${s}, expected ${LABEL[m.status]}`);
      }
    }
    if (wrong.length) problems.push(`${page} disagrees with ROADMAP.md at ${tag}:\n      ${wrong.slice(0, 8).join("\n      ")}${wrong.length > 8 ? `\n      … and ${wrong.length - 8} more` : ""}`);
  }

  // the Python API reference
  const py = /__version__ ?: ?str ?= ?'([^']+)'/.exec(reference)?.[1] ?? null;
  if (!py || !(py === version || py.startsWith(`${version}.post`))) {
    problems.push(
      `the Python API reference is ${py ? `${py}'s` : "of no version"}, not ${version}'s (the tachsin repo's photonoxide-python-reference/ copy: \`pnpm run deploy\` brings it up to the latest release)`,
    );
  }

  return { version, expected, problems };
}

const started = Date.now();
for (let attempt = 1; ; attempt++) {
  let result;
  try {
    result = await check();
  } catch (e) {
    if (!(e instanceof Unreadable)) throw e;
    console.error(`site-check: couldn't read ${e.message}`);
    if (Date.now() - started < WAIT_MINUTES * 60000) {
      await new Promise((r) => setTimeout(r, 60000));
      continue;
    }
    process.exit(2);
  }
  const { version, expected, problems } = result;
  if (!problems.length) {
    const next = expected.find((m) => m.status === "next");
    console.log(
      `${SITE} shows ${version}, the latest release: the overview, the roadmap (${next ? `next: ${next.version} ${next.title}` : "nothing next"}) and the Python API reference agree.`,
    );
    process.exit(0);
  }
  const left = WAIT_MINUTES * 60000 - (Date.now() - started);
  if (left > 0) {
    console.log(`attempt ${attempt}: ${problems.length} disagreement(s); trying again in a minute (${Math.ceil(left / 60000)} min left)`);
    await new Promise((r) => setTimeout(r, 60000));
    continue;
  }
  console.error(`${SITE} doesn't show the latest release, ${version}:`);
  for (const p of problems) console.error(`  - ${p}`);
  console.error(
    [
      "",
      "The pages look the release up every ten minutes, and a deploy asks again at once. If this",
      "lasts longer, deploy tachsin.gr (`pnpm run deploy` in the tachsin repository, fiona/tachsin.gr),",
      "which builds photonoxide's site/ at its latest main and restarts the server; then run this again.",
      "When the roadmap is wrong at the tag itself, ROADMAP.md there is what the pages read",
      "(bash .github/scripts/release-marks.sh --at <tag>).",
    ].join("\n"),
  );
  process.exit(1);
}
