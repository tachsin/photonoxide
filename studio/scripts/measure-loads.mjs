#!/usr/bin/env node
// Measures what the studio loads the first time: from launching it to its window, the first
// click on Components after a launch (the list and the component it shows), and the first time
// each of several components opens. Every Tauri command each load calls is logged: when it
// starts and answers, how much it sends and gets back, whether it runs on the window's main
// thread and how many ran at once. docs/studio-performance.md holds its results.
//
//   cd studio && pnpm tauri build --config '{"identifier":"gr.tachsin.photonoxide.perf"}'
//   node scripts/measure-loads.mjs                                  # an empty workspace
//   node scripts/measure-loads.mjs --seed <a workspace with jobs/ and runs/>   # that one, and an empty one
//   node scripts/measure-loads.mjs --label after --compare <an earlier run's JSON>
//
// Windows only, Node 24 or newer: it drives the window's WebView2 over the Chrome DevTools
// Protocol, as measure-switches.mjs does, with a build of its own identifier
// (gr.tachsin.photonoxide.perf), so an installed studio's settings are never read or written.
// Each launch is a new process: what the program computes and keeps is computed again. The
// WebView2 profile is the same across one workspace's launches, as an installed studio's is, and
// fresh for its first launch.
//
// Each load is a click on what a person would click, once the window has been idle for a second.
// Its times are from the click: frame (the first frame painted after it), names (the list's rows
// shown), view (the component's page shown) and settled (the page's last change, once it has been
// quiet for 500 ms; capped at 60 s). The window asks for reduced motion, as measure-switches does.
//
// Options: --exe <path> (target/release/photonoxide.exe), --port <n> (9233), --out <folder>
// (target/perf), --label <name> (loads-<label>.json/.md), --launches <n> (5 for each workspace),
// --click-at <idle|window> (the first click once the boot is idle, or as soon as the window shows),
// --seed <folder>, --components <titles> (comma-separated: the components opened, each once a
// launch), --compare <json>, --keep.

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
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
const PORT = Number(opt("port", "9233"));
const OUT = path.resolve(opt("out", path.join(repo, "target", "perf")));
const LABEL = opt("label", "run");
const LAUNCHES = Number(opt("launches", "5"));
const SEED = opt("seed", null);
const COMPARE = opt("compare", null);
const KEEP = argv.includes("--keep");
/** When the first click comes: once the boot is idle for a second (idle), or as the window shows (window). */
const CLICK_AT = opt("click-at", "idle");
const COMPONENTS = opt(
  "components",
  "All-pass ring,Add-drop ring,Mach–Zehnder modulator,Mach–Zehnder interferometer,Directional coupler,MMI 1 × 2,Y-branch,Bend,Coupler,Terminator,Custom device",
).split(",");
const QUIET = 500;
const CAP = 60000;
const ID = "gr.tachsin.photonoxide.perf";
const LIST = 'nav [aria-label="Components: its list"]';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const started = Date.now();
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
function fail(message) {
  console.error(`measure-loads: ${message}`);
  process.exit(1);
}
if (process.platform !== "win32") fail("this measures the Windows build (WebView2); run it on Windows");
if (typeof WebSocket === "undefined") fail("Node 24 or newer is needed, for its WebSocket");
if (!fs.existsSync(EXE)) fail(`${EXE} is missing: build it first (see the top of this file)`);

// ---- which commands run on the window's main thread: a command that is neither async nor
// marked #[tauri::command(async)] does ----

const THREAD = new Map();
for (const file of fs.readdirSync(path.join(here, "..", "src-tauri", "src"), { recursive: true })) {
  if (!String(file).endsWith(".rs")) continue;
  const text = fs.readFileSync(path.join(here, "..", "src-tauri", "src", String(file)), "utf8");
  for (const m of text.matchAll(/#\[tauri::command(\(async\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(async\s+)?fn\s+(\w+)/g)) {
    THREAD.set(m[3], m[1] || m[2] ? "pool" : "main");
  }
}

// ---- the settings, the workspaces and the profiles: the run's own ----

const CONFIG = path.join(process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming"), ID);
const configExisted = fs.existsSync(CONFIG);
const configBackup = new Map();
if (configExisted) for (const f of fs.readdirSync(CONFIG, { withFileTypes: true })) if (f.isFile()) configBackup.set(f.name, fs.readFileSync(path.join(CONFIG, f.name)));
const TEMP = fs.mkdtempSync(path.join(os.tmpdir(), "photonoxide-loads-"));
const SETTINGS = { theme: "dark", workspace: null, check_updates: false, hints: false, welcomed: true, dismissed: [], view: "3d", length_unit: "um" };

let child = null;
let cdp = null;
function closeProgram() {
  if (child?.pid && child.exitCode === null) spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  child = null;
  try {
    cdp?.ws.close();
  } catch {}
  cdp = null;
}
function remove(dir) {
  for (let i = 0; i < 20; i++) {
    try {
      fs.rmSync(dir, { recursive: true, force: true });
      return;
    } catch {
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
    }
  }
  console.error(`measure-loads: couldn't remove ${dir}`);
}
let cleaned = false;
function cleanUp() {
  if (cleaned) return;
  cleaned = true;
  closeProgram();
  try {
    if (!configExisted) remove(CONFIG);
    else {
      for (const f of fs.readdirSync(CONFIG, { withFileTypes: true })) if (f.isFile() && !configBackup.has(f.name)) fs.rmSync(path.join(CONFIG, f.name));
      for (const [name, bytes] of configBackup) fs.writeFileSync(path.join(CONFIG, name), bytes);
    }
  } catch (e) {
    console.error(`measure-loads: couldn't put back ${CONFIG}: ${e}`);
  }
  if (!KEEP) remove(TEMP);
}
process.on("exit", cleanUp);
process.on("SIGINT", () => process.exit(130));
process.on("uncaughtException", (e) => {
  console.error(e);
  process.exit(1);
});
process.on("unhandledRejection", (e) => {
  console.error(e);
  process.exit(1);
});

// ---- the DevTools protocol ----

class Cdp {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.id = 0;
    this.pending = new Map();
    this.ws.addEventListener("message", (m) => {
      const d = JSON.parse(m.data);
      if (d.id === undefined) return;
      const p = this.pending.get(d.id);
      this.pending.delete(d.id);
      if (d.error) p?.reject(new Error(`${p.method}: ${d.error.message}`));
      else p?.resolve(d.result);
    });
  }
  open() {
    return new Promise((resolve, reject) => {
      this.ws.addEventListener("open", resolve, { once: true });
      this.ws.addEventListener("error", reject, { once: true });
    });
  }
  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      this.pending.set(id, { resolve, reject, method });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
}

async function js(body, ...args) {
  const expression = `(async (...args) => { ${body} })(...${JSON.stringify(args)})`;
  const r = await cdp.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(`in the page: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`);
  return r.result.value;
}
async function until(body, what, ms = 30000, ...args) {
  const end = Date.now() + ms;
  for (;;) {
    const v = await js(body, ...args).catch(() => null);
    if (v) return v;
    if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
    await sleep(50);
  }
}

// ---- what the page is given: the counters, installed once the window shows ----

const INSTRUMENT = String.raw`
(() => {
  if (window.__load) return;
  const now = () => performance.now();
  const P = (window.__load = { ipc: [], long: [], gl: 0 });
  // the Tauri commands: the window's invoke asks over http://ipc.localhost/<command>
  const fetch0 = window.fetch;
  window.fetch = function (input, init) {
    const url = typeof input === "string" ? input : input?.url ?? "";
    const m = /^https?:\/\/ipc\.localhost\/([^?]+)/.exec(url);
    const p = fetch0.call(this, input, init);
    if (m) {
      const body = init?.body;
      const e = { cmd: decodeURIComponent(m[1]), t: now(), end: null, sent: typeof body === "string" ? body.length : body?.byteLength ?? 0, got: null };
      P.ipc.push(e);
      p.then(
        (r) => {
          e.end = now();
          r.clone().arrayBuffer().then((b) => (e.got = b.byteLength), () => {});
        },
        () => (e.end = now()),
      );
    }
    return p;
  };
  const getContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
    const fresh = (type === "webgl" || type === "webgl2") && !this.__loadGl;
    const ctx = getContext.call(this, type, ...rest);
    if (fresh && ctx) {
      this.__loadGl = true;
      P.gl++;
    }
    return ctx;
  };
  try {
    new PerformanceObserver((list) => {
      for (const e of list.getEntries()) P.long.push({ start: e.startTime, ms: e.duration });
    }).observe({ type: "longtask", buffered: true });
  } catch {}
  const visible = (e) => {
    const r = e.getBoundingClientRect();
    if (!r.width || !r.height) return false;
    const s = getComputedStyle(e);
    return s.visibility !== "hidden" && s.display !== "none";
  };
  const norm = (s) => s.replace(/\s+/g, " ").trim();
  P.find = (selector, text) => {
    let all = [...document.querySelectorAll(selector)].filter(visible);
    if (text != null) {
      const exact = all.filter((e) => norm(e.textContent) === text);
      all = exact.length ? exact : all.filter((e) => norm(e.textContent).includes(text));
    }
    return all[0] ?? null;
  };
  /** Waits until no command has been pending for ms milliseconds. */
  P.idle = async (ms) => {
    let since = now();
    for (;;) {
      await new Promise((r) => setTimeout(r, 50));
      if (P.ipc.some((e) => e.end === null)) since = now();
      else if (now() - since >= ms) return;
    }
  };
  const frame = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
  /**
   * Clicks and times the load: the first frame after the click, the first time each mark's
   * selector matches (marks: [name, selector, at least how many]), and the page's last change
   * before quiet ms of quiet (cap ms at most). Every command it called, from the click.
   */
  P.measure = async (selector, text, marks, quiet, cap) => {
    const el = P.find(selector, text);
    if (!el) throw new Error("nothing to click: " + selector + " " + (text ?? ""));
    let last = -1;
    const MOVING = ".cm-cursorLayer, .cm-selectionLayer";
    const changes = new Map();
    const mo = new MutationObserver((records) => {
      const seen = new Set();
      for (const r of records) {
        const e = r.target.nodeType === 1 ? r.target : r.target.parentElement;
        if (e?.closest(MOVING)) continue;
        if (e && !seen.has(e)) {
          seen.add(e);
          changes.set(e, (changes.get(e) ?? 0) + 1);
        }
        if (e && changes.get(e) > 6) continue;
        last = now();
      }
    });
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true });
    const ipc0 = P.ipc.length;
    const gl0 = P.gl;
    const at = {};
    const check = (t) => {
      for (const [name, sel, least] of marks) if (at[name] === undefined && [...document.querySelectorAll(sel)].filter(visible).length >= least) at[name] = t;
    };
    const t0 = now();
    el.click();
    await frame();
    const painted = now() - t0;
    check(painted);
    let capped = false;
    for (;;) {
      await new Promise((r) => setTimeout(r, 20));
      const t = now();
      check(t - t0);
      // any command still pending, those asked for before the click too (a page can wait on them)
      const pending = P.ipc.some((e) => e.end === null);
      // and every mark shown: a page can wait on a command asked for before the counters were in
      const shown = marks.every(([name]) => at[name] !== undefined);
      if (!pending && shown && t - Math.max(last, t0 + painted) >= quiet) break;
      if (t - t0 > cap) {
        capped = true;
        break;
      }
    }
    mo.disconnect();
    const settled = Math.max(painted, last - t0, ...P.ipc.slice(ipc0).map((e) => (e.end ?? now()) - t0));
    const long = P.long.filter((l) => l.start + l.ms > t0 && l.start < t0 + settled);
    performance.measure("load", { start: t0, end: t0 + settled });
    return {
      frame: painted,
      marks: at,
      settled,
      capped,
      longMs: long.reduce((s, l) => s + l.ms, 0),
      longMax: long.reduce((s, l) => Math.max(s, l.ms), 0),
      glMade: P.gl - gl0,
      ipc: P.ipc.slice(ipc0).map((e) => ({ cmd: e.cmd, at: e.t - t0, ms: e.end === null ? null : e.end - e.t, sent: e.sent, got: e.got })),
    };
  };
})();
`;

// ---- statistics ----

function quantile(sorted, q) {
  if (!sorted.length) return NaN;
  const i = (sorted.length - 1) * q;
  const lo = Math.floor(i);
  const hi = Math.ceil(i);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (i - lo);
}
function stats(values) {
  const v = values.filter((x) => Number.isFinite(x)).sort((a, b) => a - b);
  return { n: v.length, median: quantile(v, 0.5), p10: quantile(v, 0.1), p90: quantile(v, 0.9), min: v[0], max: v.at(-1) };
}
const ms = (x) => (Number.isFinite(x) ? (x >= 100 ? x.toFixed(0) : x.toFixed(1)) : "–");
const spread = (s) => (s.n > 1 ? `${ms(s.median)} (${ms(s.min)}–${ms(s.max)})` : ms(s.median));
const kb = (b) => (Number.isFinite(b) ? (b >= 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${(b / 1e3).toFixed(1)} kB`) : "–");

// ---- a launch ----

async function metrics() {
  const { metrics: m } = await cdp.send("Performance.getMetrics");
  return Object.fromEntries(m.map((x) => [x.name, x.value]));
}

/** How many commands ran at once with each: the most that overlapped it, itself included. */
function overlaps(calls) {
  return calls.map((c) => {
    const end = c.at + (c.ms ?? 0);
    return calls.filter((o) => o.at < end && o.at + (o.ms ?? 0) > c.at).length;
  });
}

async function timed(selector, text, marks) {
  const before = await metrics();
  const r = await js(`return __load.measure(args[0], args[1], args[2], args[3], args[4]);`, selector, text, marks, QUIET, CAP);
  const after = await metrics();
  const d = (k) => 1000 * (after[k] - before[k]);
  const together = overlaps(r.ipc);
  r.ipc = r.ipc.map((c, i) => ({ ...c, thread: THREAD.get(c.cmd) ?? "?", together: together[i] }));
  return { ...r, taskMs: d("TaskDuration"), scriptMs: d("ScriptDuration"), styleMs: d("RecalcStyleDuration"), layoutMs: d("LayoutDuration"), nodes: after.Nodes - before.Nodes };
}

async function launch(workspace, profile) {
  if (await fetch(`http://127.0.0.1:${PORT}/json`).then(() => true).catch(() => false)) fail(`something already answers on port ${PORT}: close it, or pass --port`);
  fs.mkdirSync(CONFIG, { recursive: true });
  fs.writeFileSync(path.join(CONFIG, "settings.json"), JSON.stringify(SETTINGS, null, 2));
  const t0 = Date.now();
  child = spawn(EXE, [], {
    cwd: workspace,
    stdio: "ignore",
    env: {
      ...process.env,
      WEBVIEW2_USER_DATA_FOLDER: profile,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT} --disable-features=CalculateNativeWinOcclusion --disable-renderer-backgrounding --disable-background-timer-throttling --disable-backgrounding-occluded-windows --force-prefers-reduced-motion`,
    },
  });
  let target = null;
  for (let i = 0; i < 600 && !target; i++) {
    await sleep(50);
    const list = await fetch(`http://127.0.0.1:${PORT}/json`).then((r) => r.json()).catch(() => []);
    target = list.find((t) => t.type === "page" && t.url.includes("tauri.localhost"));
  }
  if (!target) throw new Error(`no studio window on port ${PORT}`);
  cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  const id = await until(`return window.__TAURI_INTERNALS__ ? await window.__TAURI_INTERNALS__.invoke("plugin:app|identifier") : null;`, "the identifier", 30000);
  if (id !== ID) {
    closeProgram();
    fail(`${EXE} is built as ${id}, not ${ID}: build it with --config '{"identifier":"${ID}"}'`);
  }
  await until(`return document.querySelector("nav button[aria-label='Components']");`, "the window", 60000);
  const ready = Date.now() - t0;
  await js(INSTRUMENT);
  await cdp.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  await cdp.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await cdp.send("Performance.enable");
  const workspaceShown = await js(`return document.querySelector("footer span.truncate")?.textContent;`);
  if (workspaceShown !== workspace) throw new Error(`the workspace is ${workspaceShown}, not ${workspace}`);
  // the boot's own commands done, and a second of nothing, before the first click; or the click
  // as soon as the window shows (--click-at window)
  if (CLICK_AT !== "window") await js(`await __load.idle(1000);`);
  const settledBoot = Date.now() - t0;
  return { ready, idle: settledBoot };
}

const MARKS = [
  ["names", `${LIST} button svg + span`, 4],
  ["view", "main [role='tablist'], main h2", 1],
];

async function onePass(kind, workspace, profile, k) {
  const boot = await launch(workspace, profile);
  log(`${kind} launch ${k + 1}/${LAUNCHES}: window in ${boot.ready} ms, idle at ${boot.idle} ms`);
  const first = await timed(`nav button[aria-label='Components']`, null, MARKS);
  log(`  first Components: frame ${ms(first.frame)}, names ${ms(first.marks.names)}, view ${ms(first.marks.view)}, settled ${ms(first.settled)} ms, ${first.ipc.length} commands`);
  // the components list open, every component's first opening, each once
  const opens = [];
  for (const c of COMPONENTS) {
    if (!(await js(`return !!__load.find(args[0] + " button", args[1]);`, LIST, c))) {
      opens.push({ component: c, missing: true });
      continue;
    }
    const r = await timed(`${LIST} button`, c, [["view", "main [role='tablist'], main h2", 1]]);
    opens.push({ component: c, ...r });
  }
  log(`  opened ${opens.filter((o) => !o.missing).length} components; slowest settled ${ms(Math.max(...opens.map((o) => o.settled ?? 0)))} ms`);
  closeProgram();
  await sleep(1500);
  return { boot, first, opens };
}

// ---- the report ----

function summarise(passes) {
  const firsts = passes.map((p) => p.first);
  const commands = new Map();
  for (const f of firsts) {
    for (const c of f.ipc) {
      if (!commands.has(c.cmd)) commands.set(c.cmd, { calls: [], at: [], ms: [], got: [], thread: c.thread, together: [] });
      const x = commands.get(c.cmd);
      x.calls.push(1);
      x.at.push(c.at);
      x.ms.push(c.ms);
      x.got.push(c.got);
      x.together.push(c.together);
    }
  }
  const opens = new Map();
  for (const p of passes) for (const o of p.opens) if (!o.missing) opens.set(o.component, [...(opens.get(o.component) ?? []), o]);
  return {
    ready: stats(passes.map((p) => p.boot.ready)),
    idle: stats(passes.map((p) => p.boot.idle)),
    first: {
      frame: stats(firsts.map((f) => f.frame)),
      names: stats(firsts.map((f) => f.marks.names)),
      view: stats(firsts.map((f) => f.marks.view)),
      settled: stats(firsts.map((f) => f.settled)),
      script: stats(firsts.map((f) => f.scriptMs)),
      styleLayout: stats(firsts.map((f) => f.styleMs + f.layoutMs)),
      long: stats(firsts.map((f) => f.longMs)),
      longMax: stats(firsts.map((f) => f.longMax)),
      nodes: stats(firsts.map((f) => f.nodes)),
      gl: stats(firsts.map((f) => f.glMade)),
      commands: stats(firsts.map((f) => f.ipc.length)),
      byCommand: Object.fromEntries(
        [...commands].map(([cmd, x]) => [cmd, { perLoad: x.calls.length / firsts.length, thread: x.thread, at: stats(x.at), ms: stats(x.ms), got: stats(x.got), together: stats(x.together) }]),
      ),
    },
    opens: Object.fromEntries(
      [...opens].map(([c, list]) => [
        c,
        {
          frame: stats(list.map((o) => o.frame)),
          view: stats(list.map((o) => o.marks.view)),
          settled: stats(list.map((o) => o.settled)),
          script: stats(list.map((o) => o.scriptMs)),
          long: stats(list.map((o) => o.longMs)),
          gl: stats(list.map((o) => o.glMade)),
          slowest: Object.entries(
            list
              .flatMap((o) => o.ipc)
              .reduce((m, c) => ((m[c.cmd] = [...(m[c.cmd] ?? []), c.ms]), m), {}),
          )
            .map(([cmd, v]) => ({ cmd, ms: stats(v).median, perLoad: v.length / list.length, thread: THREAD.get(cmd) ?? "?" }))
            .sort((a, b) => b.ms - a.ms)
            .slice(0, 3),
        },
      ]),
    ),
  };
}

function table(head, rows) {
  return [`| ${head.join(" | ")} |`, `| ${head.map((h, i) => (i ? "---:" : "---")).join(" | ")} |`, ...rows.map((r) => `| ${r.join(" | ")} |`)].join("\n");
}

function report(results, before) {
  const lines = [`# Studio first loads: ${LABEL}`, "", `Build ${EXE}; ${os.cpus()[0]?.model.trim()}, ${os.cpus().length} threads; ${LAUNCHES} launches a workspace. Times in ms from the click: the median, and the least and most in brackets.`, ""];
  for (const [kind, s] of Object.entries(results.summary)) {
    const b = before?.summary?.[kind];
    const was = (v, w) => (w && Number.isFinite(w.median) ? `${ms(w.median)} → ${ms(v.median)}` : spread(v));
    lines.push(`## ${kind}`, "");
    lines.push(table(["", "ms"], [
      ["launch to the window", was(s.ready, b?.ready)],
      ["first Components: first frame", was(s.first.frame, b?.first.frame)],
      ["first Components: the list's names", was(s.first.names, b?.first.names)],
      ["first Components: the component's page", was(s.first.view, b?.first.view)],
      ["first Components: settled", was(s.first.settled, b?.first.settled)],
      ["… main thread: script; style + layout; long tasks", `${ms(s.first.script.median)}; ${ms(s.first.styleLayout.median)}; ${ms(s.first.long.median)} (longest ${ms(s.first.longMax.median)})`],
      ["… commands; WebGL contexts made", `${ms(s.first.commands.median)}; ${ms(s.first.gl.median)}`],
    ]), "");
    lines.push("The first click on Components, every command it called (each launch's median):", "");
    lines.push(table(["Command", "calls", "starts at", "takes", "returns", "thread", "at once"], Object.entries(s.first.byCommand).sort((x, y) => x[1].at.median - y[1].at.median).map(([cmd, x]) => [`\`${cmd}\``, x.perLoad.toFixed(1), ms(x.at.median), spread(x.ms), kb(x.got.median), x.thread, ms(x.together.median)])), "");
    lines.push("Each component opened for the first time after a launch:", "");
    lines.push(table(["Component", "first frame", "page", "settled", "script", "long tasks", "WebGL", "slowest commands"], Object.entries(s.opens).map(([c, o]) => [c, was(o.frame, b?.opens?.[c]?.frame), ms(o.view.median), was(o.settled, b?.opens?.[c]?.settled), ms(o.script.median), ms(o.long.median), ms(o.gl.median), o.slowest.map((x) => `\`${x.cmd}\` ${ms(x.ms)} (${x.thread})`).join(", ")])), "");
  }
  return lines.join("\n");
}

// ---- the run ----

fs.mkdirSync(OUT, { recursive: true });
const kinds = [["empty", null], ...(SEED ? [["seeded", SEED]] : [])];
const results = { label: LABEL, exe: EXE, date: new Date().toISOString(), passes: {}, summary: {} };
for (const [kind, seed] of kinds) {
  const workspace = path.join(TEMP, kind, "workspace");
  for (const d of ["jobs", "runs", "circuits"]) {
    fs.mkdirSync(path.join(workspace, d), { recursive: true });
    if (seed && fs.existsSync(path.join(seed, d))) fs.cpSync(path.join(seed, d), path.join(workspace, d), { recursive: true });
  }
  const profile = path.join(TEMP, kind, "webview2");
  results.passes[kind] = [];
  for (let k = 0; k < LAUNCHES; k++) {
    results.passes[kind].push(await onePass(kind, workspace, profile, k));
    log(`${kind}: ${k + 1}/${LAUNCHES} launches, ${((Date.now() - started) / 60000).toFixed(1)} min in all`);
  }
  results.summary[kind] = summarise(results.passes[kind]);
  fs.writeFileSync(path.join(OUT, `loads-${LABEL}.json`), JSON.stringify(results, null, 1));
}
const before = COMPARE ? JSON.parse(fs.readFileSync(COMPARE, "utf8")) : null;
fs.writeFileSync(path.join(OUT, `loads-${LABEL}.md`), report(results, before));
log(`done in ${((Date.now() - started) / 60000).toFixed(1)} min: ${path.join(OUT, `loads-${LABEL}.md`)}`);
process.exit(0);
