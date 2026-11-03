#!/usr/bin/env node
// Holds a release build of the studio to the limits in perf-limits.json, and exits non-zero with
// a table of what passed them. Run it before a release (`just perf`); it is not in CI (see
// docs/studio-performance.md, "A check before a release", for why).
//
//   cd studio && pnpm tauri build --no-bundle --config '{"identifier":"gr.tachsin.photonoxide.perf"}'
//   node scripts/perf-check.mjs                       # seed if needed, measure, compare
//   node scripts/perf-check.mjs --reuse               # compare the last measurement again
//   node scripts/perf-check.mjs --limits <json>       # other limits, e.g. a tightened copy
//
// It measures with measure-switches.mjs (the pages, the tabs, the WebGL contexts and the heap) and
// measure-loads.mjs (the first click on Components in an empty workspace and a seeded one), which
// it runs as they are and whose JSON it reads: their logic is theirs alone. Both drive the build
// over the Chrome DevTools Protocol with a temporary workspace and WebView2 profile of their own,
// and a build of the identifier gr.tachsin.photonoxide.perf, so an installed studio's settings and
// workspace are never read or written. Windows only, Node 24 or newer.
//
// The workspace they copy is the seed: 34 jobs (the repository's jobs/) and 204 runs, 34 of them
// made by the build and copied to six each, about 300 MB. The first run makes it in --seed
// (default target/perf/seed), which takes a few minutes; later runs reuse it.
//
// Options: --exe <path> (target/release/photonoxide.exe), --limits <json> (perf-limits.json beside
// this script), --seed <folder>, --out <folder> (target/perf), --label <name> (check: the files
// are measure-<label>.json and loads-<label>.json, and <label>.md is the table), --reps <n> (each
// ordered pair of pages, 3), --launches <n> (each workspace's launches for the loads, 3), --reuse
// (no measuring: compare the files a run left), --all (print every row, not the failures and the
// closest to the limit).

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..", "..");

const argv = process.argv.slice(2);
const opt = (name, fallback) => {
  const k = argv.indexOf(`--${name}`);
  return k >= 0 ? argv[k + 1] : fallback;
};
const EXE = path.resolve(opt("exe", path.join(repo, "target", "release", "photonoxide.exe")));
const LIMITS = path.resolve(opt("limits", path.join(here, "perf-limits.json")));
const OUT = path.resolve(opt("out", path.join(repo, "target", "perf")));
const SEED = path.resolve(opt("seed", path.join(OUT, "seed")));
const LABEL = opt("label", "check");
const REPS = opt("reps", "3");
const LAUNCHES = opt("launches", "3");
const REUSE = argv.includes("--reuse");
const ALL = argv.includes("--all");
/** The runs the seed holds: 34 jobs' runs, each in six copies (the studio lists the newest 200). */
const COPIES = 6;

const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
function fail(message) {
  console.error(`perf-check: ${message}`);
  process.exit(2);
}
if (process.platform !== "win32") fail("this measures the Windows build (WebView2); run it on Windows");
if (!fs.existsSync(LIMITS)) fail(`no limits file ${LIMITS}`);
const limits = JSON.parse(fs.readFileSync(LIMITS, "utf8"));
fs.mkdirSync(OUT, { recursive: true });

// ---- the seed ----

function seed() {
  const marker = path.join(SEED, "seeded.json");
  if (fs.existsSync(marker)) return log(`seed: ${SEED} (${JSON.parse(fs.readFileSync(marker, "utf8")).runs} runs)`);
  if (!fs.existsSync(EXE)) fail(`${EXE} is missing: build it first (see the top of this file)`);
  const jobs = fs.readdirSync(path.join(repo, "jobs")).filter((f) => /\.(ya?ml|toml|json)$/.test(f));
  const base = path.join(SEED, "runs-made");
  fs.rmSync(SEED, { recursive: true, force: true });
  fs.mkdirSync(path.join(SEED, "jobs"), { recursive: true });
  fs.mkdirSync(base, { recursive: true });
  const t0 = Date.now();
  jobs.forEach((job, k) => {
    fs.copyFileSync(path.join(repo, "jobs", job), path.join(SEED, "jobs", job));
    const r = spawnSync(EXE, ["run", path.join(repo, "jobs", job), "--headless", "--out", base], { stdio: "ignore", timeout: 20 * 60 * 1000 });
    log(`seed: ${k + 1}/${jobs.length} ${job} ${r.status === 0 ? "ran" : `FAILED (${r.status ?? r.error?.code})`}, ${((Date.now() - t0) / 60000).toFixed(1)} min`);
    if (r.status !== 0) fail(`${job} didn't run: the seed needs a run of every job`);
  });
  const made = fs.readdirSync(base).filter((d) => fs.existsSync(path.join(base, d, "events.jsonl")));
  const runs = path.join(SEED, "runs");
  fs.mkdirSync(runs, { recursive: true });
  // the first copy keeps the run's name; the others are named after it, so they sort beside it
  for (const d of made) for (let c = 0; c < COPIES; c++) fs.cpSync(path.join(base, d), path.join(runs, c ? `${d}-copy${c}` : d), { recursive: true });
  fs.rmSync(base, { recursive: true, force: true });
  const total = fs.readdirSync(runs).length;
  fs.writeFileSync(marker, JSON.stringify({ jobs: jobs.length, made: made.length, runs: total, date: new Date().toISOString() }));
  log(`seed: ${jobs.length} jobs, ${made.length} runs made, ${total} runs in all`);
}

// ---- measuring: the two scripts, as they are ----

function measure(script, args) {
  log(`running ${script} ${args.join(" ")}`);
  const r = spawnSync(process.execPath, [path.join(here, script), ...args], { stdio: "inherit" });
  if (r.status !== 0) fail(`${script} stopped with ${r.status ?? r.error}`);
}

if (!REUSE) {
  if (!fs.existsSync(EXE)) fail(`${EXE} is missing: build it first (see the top of this file)`);
  seed();
  const common = ["--exe", EXE, "--out", OUT, "--label", LABEL, "--seed", SEED];
  measure("measure-switches.mjs", [...common, "--only", "pages,tabs,contexts,heap", "--reps", REPS]);
  // one component opened in each launch: the limits are of the first click on Components
  measure("measure-loads.mjs", [...common, "--launches", LAUNCHES, "--components", "All-pass ring"]);
}

const readJson = (name) => {
  const file = path.join(OUT, name);
  if (!fs.existsSync(file)) fail(`${file} is missing: run without --reuse first`);
  return JSON.parse(fs.readFileSync(file, "utf8"));
};
const switches = readJson(`measure-${LABEL}.json`).summary;
const loads = readJson(`loads-${LABEL}.json`).summary;

// ---- the comparison ----

const rows = [];
const fmt = (x) => (typeof x === "number" ? (Number.isInteger(x) ? String(x) : x.toFixed(Math.abs(x) < 10 ? 2 : 1)) : String(x));

/** A measure that must not pass `limit`; `basis` says where the limit comes from. */
function atMost(name, observed, limit, unit, basis) {
  const missing = !Number.isFinite(observed);
  rows.push({ name, observed: missing ? "not measured" : `${fmt(observed)}${unit ? ` ${unit}` : ""}`, limit: `${fmt(limit)}${unit ? ` ${unit}` : ""}`, basis, ok: !missing && observed <= limit, share: missing ? Infinity : observed / (limit || 1) });
}
/** A measure that must be exactly `expected`. */
function equals(name, observed, expected, basis) {
  rows.push({ name, observed: observed === undefined ? "not measured" : String(observed), limit: String(expected), basis, ok: observed === expected, share: observed === expected ? 0 : Infinity });
}

/** The limit of a documented time, under the rule: the factor times it, and at least the floor. */
function ruled(documented, rule) {
  const r = limits.rules[rule];
  return Math.max(documented * r.factor, r.floorMs ?? r.floor ?? 0);
}
const basisOf = (documented, rule) => `${rule}: ${fmt(documented)} documented × ${limits.rules[rule].factor}${limits.rules[rule].floorMs || limits.rules[rule].floor ? `, at least ${limits.rules[rule].floorMs ?? limits.rules[rule].floor}` : ""}`;

for (const key of Object.keys(switches).filter((k) => k.endsWith("Problem"))) equals(`measure-switches: ${key.replace("Problem", "")}`, switches[key], "ran", "a measurement that stops is a failure");

for (const [group, from] of [["pages", switches.pages], ["tabs", switches.tabsByTab]]) {
  const g = limits[group];
  for (const [name, doc] of Object.entries(g).filter(([k]) => k !== "about" && k !== "rule")) {
    for (const what of ["frame", "settled"]) {
      atMost(`${group === "pages" ? "page" : "tab"} ${name}: ${what}`, from?.[name]?.[what]?.median, ruled(doc[what], g.rule), "ms", basisOf(doc[what], g.rule));
    }
  }
}

for (const kind of ["empty", "seeded"]) {
  for (const what of ["names", "settled"]) {
    const doc = limits.loads[kind][what];
    atMost(`first click on Components, ${kind} workspace: ${what}`, loads[kind]?.first[what]?.median, ruled(doc, limits.loads.rule), "ms", basisOf(doc, limits.loads.rule));
  }
}
for (const [cmd, doc] of Object.entries(limits.commands).filter(([k]) => k !== "about" && k !== "rule")) {
  const seen = loads.seeded?.first.byCommand?.[cmd];
  atMost(`first click on Components, seeded: ${cmd}`, seen?.ms.median, ruled(doc.ms, limits.commands.rule), "ms", basisOf(doc.ms, limits.commands.rule));
  if (doc.thread) equals(`first click on Components, seeded: ${cmd} runs on`, seen?.thread, doc.thread, "the thread it was moved to");
}

const h = switches.heap;
const growth = limits.rules.growth;
if (!h?.perSwitch) equals("heap: 200 page switches", undefined, "measured", "the heap's measurement");
else {
  atMost("DOM nodes gained per page switch", h.perSwitch.nodes, limits.heap.growth.nodesPerSwitch + growth.tolerance.nodesPerSwitch, "", `growth: ${limits.heap.growth.nodesPerSwitch} documented, within ${growth.tolerance.nodesPerSwitch}`);
  atMost("listeners gained per page switch", h.perSwitch.listeners, limits.heap.growth.listenersPerSwitch + growth.tolerance.listenersPerSwitch, "", `growth: ${limits.heap.growth.listenersPerSwitch} documented, within ${growth.tolerance.listenersPerSwitch}`);
  if (h.detached) atMost("detached nodes still held after 200 switches", h.detached.held, ruled(limits.heap.detachedHeld.documented, "count"), "", basisOf(limits.heap.detachedHeld.documented, "count"));
}

const rounds = switches.contexts?.rounds;
if (!rounds) equals("WebGL contexts: rounds", undefined, "measured", "the contexts' measurement");
else {
  rounds.forEach((r, i) => {
    for (const key of ["evicted", "lostOnPage", "leftToGc"]) equals(`WebGL contexts after round ${i + 1}: ${key}`, r[key], limits.contexts[key], "documented: none");
  });
}

// ---- the report ----

const failed = rows.filter((r) => !r.ok);
const table = (list) => {
  const head = ["Measure", "Observed", "Limit", "Basis"];
  const body = list.map((r) => [r.name, r.observed, r.limit, r.basis]);
  const width = head.map((_, c) => Math.max(head[c].length, ...body.map((b) => b[c].length)));
  const line = (cells) => cells.map((cell, c) => cell.padEnd(width[c])).join("  ");
  return [line(head), width.map((w) => "-".repeat(w)).join("  "), ...body.map(line)].join("\n");
};
const markdown = (list) => [`| Measure | Observed | Limit | Basis | |`, `| --- | ---: | ---: | --- | --- |`, ...list.map((r) => `| ${r.name} | ${r.observed} | ${r.limit} | ${r.basis} | ${r.ok ? "ok" : "**over**"} |`)].join("\n");

fs.writeFileSync(path.join(OUT, `${LABEL}.md`), `# Studio performance check: ${LABEL}\n\nLimits ${LIMITS}; ${failed.length} of ${rows.length} measures over their limits.\n\n${markdown(rows)}\n`);
console.log("");
if (failed.length) {
  console.log(`perf-check: ${failed.length} of ${rows.length} measures are over their limits:\n`);
  console.log(table(failed));
} else {
  console.log(`perf-check: all ${rows.length} measures are within their limits. The ten closest to theirs:\n`);
  console.log(table(ALL ? rows : [...rows].sort((a, b) => b.share - a.share).slice(0, 10)));
}
if (ALL && failed.length) console.log(`\nWithin their limits:\n\n${table(rows.filter((r) => r.ok))}`);
console.log(`\nEvery row: ${path.join(OUT, `${LABEL}.md`)}`);
process.exit(failed.length ? 1 : 0);
