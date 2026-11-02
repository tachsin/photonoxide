#!/usr/bin/env node
// Measures how fast the studio's window answers: switching pages, a component's tabs and
// components, scrolling the components list, the WebGL contexts alive and the heap's growth.
// docs/studio-performance.md holds its results and says how to read them.
//
//   cd studio && pnpm tauri build --config '{"identifier":"gr.tachsin.photonoxide.perf"}'
//   node scripts/measure-switches.mjs                          # every measurement
//   node scripts/measure-switches.mjs --only pages,tabs        # some of them
//   node scripts/measure-switches.mjs --label after --compare <an earlier run's JSON>
//   node scripts/measure-switches.mjs --discover               # list the components list's rows and exit
//
// Windows only: it drives the window's WebView2 over the Chrome DevTools Protocol, as
// record-gifs.mjs does. It needs Node 24 or newer (its global WebSocket).
//
// The build it measures is built with its own identifier, gr.tachsin.photonoxide.perf, so its
// settings are in a folder of their own (%APPDATA%\gr.tachsin.photonoxide.perf, written for the
// run and removed after it) and an installed studio's settings are never read or written. The
// script refuses a build that reports the installed studio's identifier, before it does anything.
// The workspace and the WebView2 profile are fresh temporary folders, removed at the end.
//
// What it measures, each switch a click on the element a person would click:
// - frame:   from the click to the first frame painted after it (rAF, then a task after the frame);
// - settled: from the click to the last change of the page's DOM, once it has been quiet for
//            300 ms (data loaded and drawn); capped at 8 s;
// - the renderer's main-thread time in the switch (Performance.getMetrics: script, style
//   and layout, and all tasks), its long tasks (over 50 ms), the Tauri commands it called (each
//   timed from its request to its answer), and the WebGL contexts made and the time spent making
//   them and compiling their shaders.
// One more pass of the pages and the tabs is traced (Tracing), and the trace's main-thread time
// in each switch is split into script, style, layout, paint, garbage collection and GPU waits.
//
// Options: --exe <path> (default target/release/photonoxide.exe), --port <n> (9231),
// --out <folder> (target/perf), --label <name> (the files' name: measure-<label>.json/.md),
// --reps <n> (each ordered pair of pages, 10), --tab-reps <n> (each ordered pair of tabs, 2),
// --open-reps <n> (each component opened, 10), --scroll-reps <n> (10), --only <names>
// (pages, tabs, open, scroll, contexts, heap, idle, trace), --components <titles> (comma-separated,
// the tabs' and the opening's components), --compare <json> (an earlier run: a before/after table),
// --size <w>x<h> (the page's size in CSS pixels, 1440x900), --seed <folder> (a workspace whose
// jobs/, runs/ and circuits/ are copied into the fresh one, so the lists are as long as in use),
// --keep (keep the profile and workspace).

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..", "..");

// ---- options ----

const argv = process.argv.slice(2);
const opt = (name, fallback) => {
  const k = argv.indexOf(`--${name}`);
  return k >= 0 ? argv[k + 1] : fallback;
};
const EXE = path.resolve(opt("exe", path.join(repo, "target", "release", "photonoxide.exe")));
const PORT = Number(opt("port", "9231"));
const OUT = path.resolve(opt("out", path.join(repo, "target", "perf")));
const LABEL = opt("label", "run");
const REPS = Number(opt("reps", "10"));
const TAB_REPS = Number(opt("tab-reps", "2"));
const OPEN_REPS = Number(opt("open-reps", "10"));
const SCROLL_REPS = Number(opt("scroll-reps", "10"));
const ALL = ["pages", "tabs", "open", "scroll", "contexts", "heap", "idle", "trace"];
const ONLY = new Set(opt("only", ALL.join(",")).split(","));
const COMPARE = opt("compare", null);
const KEEP = argv.includes("--keep");
const DISCOVER = argv.includes("--discover");
const [VIEW_W, VIEW_H] = opt("size", "1440x900").split("x").map(Number);
/** The components the tabs and the opening use: one drawn in 3D, a heavy one, a plain one. */
const COMPONENTS = opt("components", "All-pass ring,Mach–Zehnder modulator,Waveguide").split(",");
const QUIET = 300;
const CAP = 8000;

const ID = "gr.tachsin.photonoxide.perf";
const INSTALLED = "gr.tachsin.photonoxide";

for (const name of ONLY) if (!ALL.includes(name)) fail(`no measurement called ${name}: ${ALL.join(", ")}`);

function fail(message) {
  console.error(`measure-switches: ${message}`);
  process.exit(1);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const started = Date.now();
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

/** Logs step k of n of a stage, with how far along, how long so far and how long to go. */
function progress(stage, k, n) {
  if (k !== n && k % Math.max(1, Math.floor(n / 10)) !== 0) return;
  const spent = (Date.now() - stageStart) / 1000;
  const eta = k ? (spent / k) * (n - k) : 0;
  log(`${stage}: ${k}/${n} (${((100 * k) / n).toFixed(0)}%), ${spent.toFixed(0)} s in this stage, about ${eta.toFixed(0)} s to go; ${((Date.now() - started) / 60000).toFixed(1)} min in all`);
}
let stageStart = Date.now();
function stage(name) {
  stageStart = Date.now();
  log(`---- ${name}`);
}

if (process.platform !== "win32") fail("this measures the Windows build (WebView2); run it on Windows");
if (typeof WebSocket === "undefined") fail("Node 24 or newer is needed, for its WebSocket");
if (!fs.existsSync(EXE)) fail(`${EXE} is missing: build it first (see the top of this file)`);

// ---- the settings, the workspace and the profile: the run's own ----

const CONFIG = path.join(process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming"), ID);
if (path.basename(CONFIG) === INSTALLED) fail("refusing the installed studio's settings folder");
const configExisted = fs.existsSync(CONFIG);
const configBackup = new Map();
if (configExisted) {
  for (const f of fs.readdirSync(CONFIG, { withFileTypes: true })) if (f.isFile()) configBackup.set(f.name, fs.readFileSync(path.join(CONFIG, f.name)));
}
const TEMP = fs.mkdtempSync(path.join(os.tmpdir(), "photonoxide-perf-"));
const WORKSPACE = path.join(TEMP, "workspace");
const PROFILE = path.join(TEMP, "webview2");
for (const d of ["jobs", "runs", "circuits"]) fs.mkdirSync(path.join(WORKSPACE, d), { recursive: true });
// a workspace as one in use has it: its jobs, runs and circuits copied in (the seed is left as it is)
const SEED = opt("seed", null);
if (SEED) {
  if (!fs.existsSync(SEED)) fail(`no seed folder ${SEED}`);
  for (const d of ["jobs", "runs", "circuits"]) if (fs.existsSync(path.join(SEED, d))) fs.cpSync(path.join(SEED, d), path.join(WORKSPACE, d), { recursive: true });
}
const SETTINGS = { theme: "dark", workspace: null, check_updates: false, hints: false, welcomed: true, dismissed: [], view: "3d", length_unit: "um" };

let child = null;
function closeProgram() {
  if (child?.pid && child.exitCode === null) spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  child = null;
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
  console.error(`measure-switches: couldn't remove ${dir}`);
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
    console.error(`measure-switches: couldn't put back ${CONFIG}: ${e}`);
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
    this.listeners = new Map();
    this.ws.addEventListener("message", (m) => {
      const d = JSON.parse(m.data);
      if (d.id !== undefined) {
        const p = this.pending.get(d.id);
        this.pending.delete(d.id);
        if (d.error) p?.reject(new Error(`${p.method}: ${d.error.message}`));
        else p?.resolve(d.result);
      } else {
        for (const f of this.listeners.get(d.method) ?? []) f(d.params);
      }
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
  on(method, f) {
    this.listeners.set(method, [...(this.listeners.get(method) ?? []), f]);
  }
}

let cdp;

/** Runs `body` (the inside of an async function) in the page and returns its value. */
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
    await sleep(150);
  }
}

// ---- what the page is given before it loads: the counters ----

const INSTRUMENT = String.raw`
(() => {
  if (window.__perf) return;
  const now = () => performance.now();
  const P = (window.__perf = { gl: { made: 0, released: 0, lost: 0, makeMs: 0, compileMs: 0, list: [] }, ipc: [], long: [] });

  // WebGL: each context made (and how long that took), released (WEBGL_lose_context, as
  // three.js's forceContextLoss does) or lost, and the time in shader compiling and linking,
  // which the window waits for when three.js checks a program
  const getContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
    const webgl = type === "webgl" || type === "webgl2" || type === "experimental-webgl";
    const had = webgl && this.__perfGl;
    const t = now();
    const ctx = getContext.call(this, type, ...rest);
    if (webgl && ctx && !had) {
      P.gl.makeMs += now() - t;
      this.__perfGl = true;
      P.gl.made++;
      const entry = { canvas: new WeakRef(this), lost: false, released: false };
      P.gl.list.push(entry);
      this.addEventListener("webglcontextlost", () => {
        if (!entry.lost) {
          entry.lost = true;
          P.gl.lost++;
        }
      });
      this.addEventListener("webglcontextrestored", () => {
        if (entry.lost) {
          entry.lost = false;
          P.gl.lost--;
        }
      });
      const getExtension = ctx.getExtension;
      ctx.getExtension = function (name) {
        const ext = getExtension.call(this, name);
        if (ext && name === "WEBGL_lose_context" && !ext.__perf) {
          ext.__perf = true;
          const lose = ext.loseContext;
          ext.loseContext = function () {
            if (!entry.released) {
              entry.released = true;
              P.gl.released++;
            }
            return lose.call(this);
          };
        }
        return ext;
      };
    }
    return ctx;
  };
  for (const C of [window.WebGLRenderingContext, window.WebGL2RenderingContext]) {
    if (!C) continue;
    for (const m of ["compileShader", "linkProgram", "getProgramParameter", "getShaderParameter", "getProgramInfoLog", "getShaderInfoLog"]) {
      const f = C.prototype[m];
      C.prototype[m] = function (...a) {
        const t = now();
        try {
          return f.apply(this, a);
        } finally {
          P.gl.compileMs += now() - t;
        }
      };
    }
  }
  /**
   * The contexts: made; released by the studio (WEBGL_lose_context); evicted by the browser (lost
   * without being released: past its limit of live contexts it drops the oldest); on the page;
   * and left to the garbage collector (neither lost nor on the page: alive until their canvas is
   * collected, whenever that is).
   */
  P.contexts = () => {
    let onPage = 0;
    let lostOnPage = 0;
    for (const e of P.gl.list) {
      const c = e.canvas.deref();
      if (!c?.isConnected) continue;
      if (e.lost && !e.released) lostOnPage++;
      else if (!e.lost) onPage++;
    }
    // lostOnPage: a view still on the page (the run view, kept while hidden) whose context the browser took
    return { made: P.gl.made, released: P.gl.released, evicted: Math.max(0, P.gl.lost - P.gl.released), onPage, lostOnPage, leftToGc: P.gl.made - P.gl.lost - onPage };
  };

  // the Tauri commands: the window's invoke asks over http://ipc.localhost/<command>
  const fetch0 = window.fetch;
  window.fetch = function (input, init) {
    const url = typeof input === "string" ? input : input?.url ?? "";
    const m = /^https?:\/\/ipc\.localhost\/([^?]+)/.exec(url);
    const p = fetch0.call(this, input, init);
    if (m) {
      const e = { cmd: decodeURIComponent(m[1]), t: now(), ms: null };
      P.ipc.push(e);
      p.then(() => (e.ms = now() - e.t), () => (e.ms = now() - e.t));
    }
    return p;
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
  /** The visible element matching a selector, by its text when given (an exact match first). */
  P.find = (selector, text, nth = 0) => {
    let all = [...document.querySelectorAll(selector)].filter(visible);
    if (text != null) {
      const exact = all.filter((e) => norm(e.textContent) === text);
      all = exact.length ? exact : all.filter((e) => norm(e.textContent).includes(text));
    }
    return all[nth] ?? null;
  };

  const frame = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
  /**
   * Clicks an element and times the window's answer: the first frame painted after the click, and
   * when the page's DOM last changed before staying quiet for quiet ms (capped at cap ms).
   */
  P.measure = async (selector, text, quiet, cap, name) => {
    const el = P.find(selector, text);
    if (!el) throw new Error("nothing to click: " + selector + " " + (text ?? ""));
    let last = -1;
    let lastWhat = "";
    // a text editor's blinking cursor changes the DOM twice a second, and a turning 3D view's axes
    // every frame: not the page changing. An element changed in more than six batches is moving.
    const MOVING = ".cm-cursorLayer, .cm-selectionLayer";
    const changes = new Map();
    const mo = new MutationObserver((records) => {
      const seen = new Set();
      for (const r of records) {
        const el = r.target.nodeType === 1 ? r.target : r.target.parentElement;
        if (el?.closest(MOVING)) continue;
        if (el && !seen.has(el)) {
          seen.add(el);
          changes.set(el, (changes.get(el) ?? 0) + 1);
        }
        if (el && changes.get(el) > 6) continue;
        last = now();
        lastWhat = el ? (el.tagName.toLowerCase() + "." + String(el.className?.baseVal ?? el.className ?? "").split(/\s+/).slice(0, 3).join(".")).slice(0, 80) : "";
      }
    });
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true });
    const gl0 = { made: P.gl.made, makeMs: P.gl.makeMs, compileMs: P.gl.compileMs };
    const ipc0 = P.ipc.length;
    const t0 = now();
    el.click();
    const handler = now() - t0;
    await frame();
    const painted = now() - t0;
    let capped = false;
    for (;;) {
      await new Promise((r) => setTimeout(r, 25));
      const t = now();
      if (t - Math.max(last, t0 + painted) >= quiet) break;
      if (t - t0 > cap) {
        capped = true;
        break;
      }
    }
    mo.disconnect();
    const settled = Math.max(painted, last - t0);
    const long = P.long.filter((l) => l.start + l.ms > t0 && l.start < t0 + settled);
    if (name) performance.measure("switch:" + name, { start: t0, end: t0 + settled });
    return {
      handler,
      frame: painted,
      settled,
      capped,
      lastWhat,
      longMs: long.reduce((s, l) => s + l.ms, 0),
      longMax: long.reduce((s, l) => Math.max(s, l.ms), 0),
      ipc: P.ipc.slice(ipc0).map((e) => ({ cmd: e.cmd, at: e.t - t0, ms: e.ms })),
      glMade: P.gl.made - gl0.made,
      glMakeMs: P.gl.makeMs - gl0.makeMs,
      glCompileMs: P.gl.compileMs - gl0.compileMs,
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
const spread = (s) => `${ms(s.median)} (${ms(s.p10)}–${ms(s.p90)})`;

/** A circuit through every ordered pair of `nodes` once (an Eulerian circuit of the complete digraph). */
function everyPair(nodes) {
  const out = new Map(nodes.map((a) => [a, nodes.filter((b) => b !== a)]));
  const stack = [nodes[0]];
  const circuit = [];
  while (stack.length) {
    const v = stack.at(-1);
    const next = out.get(v);
    if (next.length) stack.push(next.shift());
    else circuit.push(stack.pop());
  }
  return circuit.reverse();
}

// ---- the window ----

const PAGES = ["Home", "Components", "Chip", "Fabrication", "Academy", "Materials", "Validation", "Libraries", "Benchmarks", "Settings"];
const TABS = ["Design", "Overview", "Model", "Simulate", "Study", "Optimize", "Fabrication"];
const LIST = 'nav [aria-label="Components: its list"]';

async function metrics() {
  const { metrics: m } = await cdp.send("Performance.getMetrics");
  return Object.fromEntries(m.map((x) => [x.name, x.value]));
}

/** Clicks and times it; the main thread's time from Performance.getMetrics around it. */
async function timed(selector, text, name) {
  const before = await metrics();
  const r = await js(`return __perf.measure(args[0], args[1], args[2], args[3], args[4]);`, selector, text, QUIET, CAP, name);
  const after = await metrics();
  const d = (k) => 1000 * (after[k] - before[k]);
  return { ...r, taskMs: d("TaskDuration"), scriptMs: d("ScriptDuration"), styleMs: d("RecalcStyleDuration"), layoutMs: d("LayoutDuration") };
}

const pageButton = (label) => `nav button[aria-label='${label}']`;
async function goPage(label, name = `page ${label}`) {
  await openSystem();
  return timed(pageButton(label), null, name);
}
async function openSystem() {
  const shown = await js(`return !!__perf.find("nav button[aria-label='Validation']");`);
  if (!shown) {
    await js(`__perf.find("nav button[aria-label='System']").click();`);
    await sleep(300);
  }
}
/** Shows the Components page with its list open. */
async function componentsPage() {
  if (!(await js(`return document.querySelector("nav button[aria-label='Components']")?.getAttribute("aria-current") === "page";`))) await goPage("Components");
  const open = await js(`return !!document.querySelector(args[0]);`, LIST);
  if (!open) {
    await js(`document.querySelector("nav button[aria-label='Components']").click();`);
    await until(`return !!document.querySelector(args[0]);`, "the components list", 5000, LIST);
  }
  // the library's components listed, not only the Custom device and the groups
  await until(`return document.querySelectorAll(args[0] + " button svg + span").length > 3;`, "the library in the list", 30000, LIST);
  await sleep(400);
}
async function openComponent(title, name) {
  await componentsPage();
  return timed(`${LIST} button`, title, name);
}
async function settle(ms = 600) {
  await sleep(ms);
}

async function launch() {
  // another window on the port would be measured in place of this one
  if (await fetch(`http://127.0.0.1:${PORT}/json`).then(() => true).catch(() => false)) fail(`something already answers on port ${PORT}: close it, or pass --port`);
  fs.mkdirSync(CONFIG, { recursive: true });
  fs.writeFileSync(path.join(CONFIG, "settings.json"), JSON.stringify(SETTINGS, null, 2));
  log(`starting ${EXE} in ${WORKSPACE}`);
  child = spawn(EXE, [], {
    cwd: WORKSPACE,
    stdio: "ignore",
    env: {
      ...process.env,
      WEBVIEW2_USER_DATA_FOLDER: PROFILE,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT} --disable-features=CalculateNativeWinOcclusion --disable-renderer-backgrounding --disable-background-timer-throttling --disable-backgrounding-occluded-windows`,
    },
  });
  let target = null;
  for (let i = 0; i < 100 && !target; i++) {
    await sleep(300);
    const list = await fetch(`http://127.0.0.1:${PORT}/json`).then((r) => r.json()).catch(() => []);
    target = list.find((t) => t.type === "page" && t.url.includes("tauri.localhost"));
  }
  if (!target) throw new Error(`no studio window on port ${PORT}`);
  cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  // the build's identifier first: the installed studio's means its settings, so stop at once
  const id = await until(`return window.__TAURI_INTERNALS__ ? await window.__TAURI_INTERNALS__.invoke("plugin:app|identifier") : null;`, "the identifier", 30000);
  if (id !== ID) {
    closeProgram();
    fail(`${EXE} is built as ${id}, not ${ID}: build it with --config '{"identifier":"${ID}"}'`);
  }
  await cdp.send("Page.enable");
  await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: INSTRUMENT });
  await cdp.send("Emulation.setDeviceMetricsOverride", { width: VIEW_W, height: VIEW_H, deviceScaleFactor: 1, mobile: false });
  await cdp.send("Emulation.setFocusEmulationEnabled", { enabled: true });
  await cdp.send("Page.reload");
  await sleep(500);
  await until(`return window.__perf && document.querySelector("nav button[aria-label='Home']");`, "the window", 60000);
  const workspace = await js(`return document.querySelector("footer span.truncate")?.textContent;`);
  if (workspace !== WORKSPACE) throw new Error(`the workspace is ${workspace}, not ${WORKSPACE}`);
  await cdp.send("Performance.enable", { timeDomain: "threadTicks" }).catch(() => cdp.send("Performance.enable"));
  await sleep(2000);
}

// ---- the measurements ----

const results = { label: LABEL, samples: {}, summary: {} };

function summarise(samples, key) {
  const groups = new Map();
  for (const s of samples) groups.set(s[key], [...(groups.get(s[key]) ?? []), s]);
  const out = {};
  for (const [k, g] of groups) {
    const ipc = new Map();
    for (const s of g) for (const c of s.ipc) ipc.set(c.cmd, [...(ipc.get(c.cmd) ?? []), c.ms]);
    out[k] = {
      frame: stats(g.map((s) => s.frame)),
      settled: stats(g.map((s) => s.settled)),
      capped: g.filter((s) => s.capped).length,
      handler: stats(g.map((s) => s.handler)),
      task: stats(g.map((s) => s.taskMs)),
      script: stats(g.map((s) => s.scriptMs)),
      styleLayout: stats(g.map((s) => s.styleMs + s.layoutMs)),
      long: stats(g.map((s) => s.longMs)),
      glMade: g.reduce((a, s) => a + s.glMade, 0) / g.length,
      glMs: stats(g.map((s) => s.glMakeMs + s.glCompileMs)),
      ipc: Object.fromEntries([...ipc].map(([c, v]) => [c, { perSwitch: v.length / g.length, ms: stats(v) }])),
      // what changed last, most often: what the settled time waits for
      lastChange: Object.entries(g.reduce((m, s) => ((m[s.lastWhat] = (m[s.lastWhat] ?? 0) + 1), m), {})).sort((a, b) => b[1] - a[1])[0]?.[0] ?? "",
    };
  }
  return out;
}

async function measurePages() {
  stage("pages: every ordered pair");
  const tour = everyPair(PAGES);
  await goPage(tour[0]);
  await settle();
  const samples = [];
  const n = (tour.length - 1) * REPS;
  for (let r = 0; r < REPS; r++) {
    for (let i = 1; i < tour.length; i++) {
      const s = await goPage(tour[i]);
      samples.push({ from: tour[i - 1], to: tour[i], contexts: await js(`return __perf.contexts();`), ...s });
      progress("pages", samples.length, n);
    }
  }
  results.samples.pages = samples;
  results.summary.pages = summarise(samples, "to");
  results.summary.pagesContexts = { final: samples.at(-1).contexts };
}

async function measureTabs() {
  stage("tabs: every ordered pair, on each component");
  const samples = [];
  const tour = everyPair(TABS);
  const n = COMPONENTS.length * (tour.length - 1) * TAB_REPS;
  for (const c of COMPONENTS) {
    await openComponent(c);
    await until(`return !!__perf.find('[role="tablist"] [role="tab"]', "Overview");`, `${c}'s tabs`, 15000);
    await timed('[role="tablist"] [role="tab"]', tour[0]);
    await settle();
    for (let r = 0; r < TAB_REPS; r++) {
      for (let i = 1; i < tour.length; i++) {
        const s = await timed('[role="tablist"] [role="tab"]', tour[i], `tab ${c} ${tour[i]}`);
        samples.push({ component: c, from: tour[i - 1], to: tour[i], key: `${c} › ${tour[i]}`, ...s });
        progress("tabs", samples.length, n);
      }
    }
    // back to Design, as a person leaves it
    await timed('[role="tablist"] [role="tab"]', "Design");
  }
  results.samples.tabs = samples;
  results.summary.tabs = summarise(samples, "key");
  results.summary.tabsByTab = summarise(samples, "to");
}

async function measureOpen() {
  stage("components opened from the list");
  const samples = [];
  const list = [...COMPONENTS, "Coupler", "Custom device"];
  await openComponent(list.at(-1));
  await settle();
  const n = list.length * OPEN_REPS;
  for (let r = 0; r < OPEN_REPS; r++) {
    for (const c of list) {
      const s = await openComponent(c, `open ${c}`);
      samples.push({ to: c, ...s });
      progress("open", samples.length, n);
    }
  }
  results.samples.open = samples;
  results.summary.open = summarise(samples, "to");
}

async function measureScroll() {
  stage("scrolling the components list");
  await componentsPage();
  // every group open (the folded ones too: Templates, Your jobs), the list at its longest
  await js(`for (const b of document.querySelectorAll(args[0] + " button[aria-expanded='false']")) b.click();`, LIST);
  await sleep(600);
  const rows = await js(`return document.querySelectorAll(args[0] + " button").length;`, LIST);
  const box = await js(`const r = document.querySelector(args[0]).getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2, h: r.height };`, LIST);
  const startFrames = `window.__frames = []; let last = performance.now(); window.__framing = true;
      const tick = (t) => { window.__frames.push(t - last); last = t; if (window.__framing) requestAnimationFrame(tick); };
      requestAnimationFrame((t) => { last = t; requestAnimationFrame(tick); });
      window.__long0 = __perf.long.length;`;
  // the frame interval at rest, nothing scrolled: what the window paints at anyway
  await js(startFrames);
  await sleep(1500);
  const rest = stats((await js(`window.__framing = false; return window.__frames;`)).slice(1));
  const passes = [];
  for (let r = 0; r < SCROLL_REPS; r++) {
    await js(`document.querySelector(args[0]).scrollTop = 0;`, LIST);
    await sleep(300);
    await js(startFrames);
    // down and back up, a notch (100 px) every 16 ms, as a wheel turned briskly
    for (const dir of [1, -1]) {
      for (let k = 0; k < 40; k++) {
        await cdp.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: box.x, y: box.y, deltaX: 0, deltaY: 100 * dir });
        await sleep(16);
      }
      await sleep(300);
    }
    const f = await js(`window.__framing = false; const l = __perf.long.slice(window.__long0); return { frames: window.__frames, longMs: l.reduce((s, x) => s + x.ms, 0), height: document.querySelector(args[0]).scrollHeight };`, LIST);
    const frames = f.frames.slice(1);
    passes.push({ frames: frames.length, median: stats(frames).median, p95: quantile([...frames].sort((a, b) => a - b), 0.95), max: Math.max(...frames), slow: frames.filter((x) => x > 25).length, longMs: f.longMs, height: f.height });
    progress("scroll", r + 1, SCROLL_REPS);
  }
  results.samples.scroll = passes;
  results.summary.scroll = {
    rows,
    listHeight: passes[0]?.height,
    viewHeight: Math.round(box.h),
    rest,
    median: stats(passes.map((p) => p.median)),
    p95: stats(passes.map((p) => p.p95)),
    max: stats(passes.map((p) => p.max)),
    slowShare: stats(passes.map((p) => (100 * p.slow) / p.frames)),
    long: stats(passes.map((p) => p.longMs)),
  };
}

async function collect() {
  await cdp.send("HeapProfiler.collectGarbage");
  await sleep(300);
  await cdp.send("HeapProfiler.collectGarbage");
  await sleep(300);
}

async function measureContexts() {
  stage("WebGL contexts after a round of switches");
  const order = ["Home", "Components", "Fabrication", "Chip", "Academy", "Materials", "Home"];
  await collect();
  const before = await js(`return __perf.contexts();`);
  const rounds = [];
  for (let r = 0; r < 3; r++) {
    for (const p of order) {
      await goPage(p);
      if (p === "Components") for (const c of COMPONENTS) await openComponent(c);
    }
    await collect();
    rounds.push(await js(`return __perf.contexts();`));
    progress("contexts", r + 1, 3);
  }
  results.summary.contexts = { before, rounds, round: order.join(" → ") + `, opening ${COMPONENTS.join(", ")} on Components` };
}

async function measureHeap() {
  stage("the heap over 50 + 50 page switches");
  const order = ["Home", "Components", "Chip", "Fabrication", "Academy", "Materials", "Validation", "Libraries", "Benchmarks", "Settings"];
  const take = async () => {
    await collect();
    const m = await metrics();
    return { heapMB: m.JSHeapUsedSize / 1e6, nodes: m.Nodes, listeners: m.JSEventListeners, documents: m.Documents };
  };
  await goPage("Home");
  const points = [await take()];
  for (let half = 0; half < 2; half++) {
    for (let k = 0; k < 50; k++) {
      await goPage(order[k % order.length]);
      progress("heap", half * 50 + k + 1, 100);
    }
    await goPage("Home");
    points.push(await take());
  }
  results.summary.heap = { points, first50MB: points[1].heapMB - points[0].heapMB, second50MB: points[2].heapMB - points[1].heapMB };
}

async function measureIdle() {
  stage("each page at rest: the main thread's load");
  const out = {};
  for (const p of PAGES) {
    await goPage(p);
    await sleep(1500);
    const a = await metrics();
    await sleep(3000);
    const b = await metrics();
    out[p] = { busyPct: (100 * (b.TaskDuration - a.TaskDuration)) / 3, scriptPct: (100 * (b.ScriptDuration - a.ScriptDuration)) / 3 };
  }
  await openComponent(COMPONENTS[0]);
  await sleep(1500);
  const a = await metrics();
  await sleep(3000);
  const b = await metrics();
  out[`Components › ${COMPONENTS[0]}`] = { busyPct: (100 * (b.TaskDuration - a.TaskDuration)) / 3, scriptPct: (100 * (b.ScriptDuration - a.ScriptDuration)) / 3 };
  results.summary.idle = out;
}

// ---- a traced pass: where the main thread's time goes ----

const BUCKETS = [
  ["gc", /GC|Gc|Scavenge|Sweep|Mark(Compact|ing)|BlinkGC/],
  ["gpu wait", /CommandBuffer|GLES2|WebGL|gpu::|GpuChannel|WaitFor|SharedImage/],
  ["style", /recalcStyle|UpdateLayoutTree|RecalculateStyles|RecalcStyle|StyleEngine|StyleResolver|ParseAuthorStyleSheet|ScheduleStyleRecalculation/],
  ["layout", /Layout|InlineNode|ShapeText|Shaping|performLayout/],
  ["paint", /Paint|Layerize|CompositeLayers|UpdateLayer|Commit|Raster|Decode|PrePaint|UpdateLayerTree|Animation|Composit/],
  ["script", /FunctionCall|EvaluateScript|v8\.|V8\.|TimerFire|FireAnimationFrame|EventDispatch|RunMicrotasks|Compile|ParseHTML|XHR|HandlePostMessage|MessagePort|ResourceReceive|Promise|IdleCallback|Microtask/],
];
const WRAPPERS = /^(RunTask|ThreadControllerImpl::RunTask|ThreadController::Task|SequenceManager|TaskQueueImpl|MessageLoop|ScheduledAction|toplevel|Scheduler::|PageAnimator::serviceScriptedAnimations|ProxyMain::BeginMainFrame|ThreadProxy|WidgetBase::|BeginMainFrame)/;

async function trace(name, body) {
  const events = [];
  cdp.on("Tracing.dataCollected", (p) => events.push(...p.value));
  const done = new Promise((r) => cdp.on("Tracing.tracingComplete", r));
  await cdp.send("Tracing.start", {
    transferMode: "ReportEvents",
    traceConfig: {
      includedCategories: ["devtools.timeline", "disabled-by-default-devtools.timeline", "toplevel", "v8.execute", "blink.user_timing", "gpu", "disabled-by-default-v8.gc"],
      excludedCategories: ["*"],
    },
  });
  await body();
  await cdp.send("Tracing.end");
  await done;
  cdp.listeners.delete("Tracing.dataCollected");
  cdp.listeners.delete("Tracing.tracingComplete");
  // gzipped: Perfetto and the browser's DevTools open it as it is
  const file = path.join(OUT, `trace-${LABEL}-${name}.json.gz`);
  fs.writeFileSync(file, zlib.gzipSync(JSON.stringify({ traceEvents: events })));
  log(`${name}: ${events.length} trace events in ${path.relative(repo, file)}`);
  return analyse(events);
}

/** The renderer main thread's self time in each `switch:` window, by bucket, and the GPU process's busy time. */
function analyse(events) {
  const names = new Map();
  for (const e of events) if (e.ph === "M" && e.name === "thread_name") names.set(`${e.pid}:${e.tid}`, e.args.name);
  const main = [...names].filter(([, n]) => n === "CrRendererMain").map(([k]) => k);
  const gpuMain = [...names].filter(([, n]) => n === "CrGpuMain" || n === "VizCompositorThread").map(([k]) => k);
  // the windows, from the page's performance.measure calls
  const opens = new Map();
  const windows = [];
  for (const e of events) {
    if (!e.cat?.includes("blink.user_timing") || !e.name?.startsWith("switch:")) continue;
    if (e.ph === "b") opens.set(`${e.name}:${e.id ?? e.id2?.local}`, e.ts);
    else if (e.ph === "e") {
      const k = `${e.name}:${e.id ?? e.id2?.local}`;
      if (opens.has(k)) windows.push({ name: e.name.slice(7), from: opens.get(k), to: e.ts });
    }
  }
  windows.sort((a, b) => a.from - b.from);
  // self times on the main threads: complete events nested by time
  const byThread = new Map();
  for (const e of events) {
    if (e.ph !== "X" || !e.dur) continue;
    const k = `${e.pid}:${e.tid}`;
    if (!main.includes(k) && !gpuMain.includes(k)) continue;
    if (!byThread.has(k)) byThread.set(k, []);
    byThread.get(k).push(e);
  }
  const selfs = [];
  for (const [k, list] of byThread) {
    list.sort((a, b) => a.ts - b.ts || b.dur - a.dur);
    const stack = [];
    for (const e of list) {
      while (stack.length && stack.at(-1).ts + stack.at(-1).dur <= e.ts) stack.pop();
      const parent = stack.at(-1);
      const item = { name: e.name, ts: e.ts, dur: e.dur, self: e.dur, gpu: gpuMain.includes(k), top: !parent };
      if (parent) parent.item.self -= e.dur;
      stack.push({ ts: e.ts, dur: e.dur, item });
      selfs.push(item);
    }
  }
  const bucketOf = (name) => {
    for (const [b, re] of BUCKETS) if (re.test(name)) return b;
    return WRAPPERS.test(name) ? "tasks (other)" : "other";
  };
  const per = new Map();
  let w = 0;
  const sorted = selfs.sort((a, b) => a.ts - b.ts);
  for (const it of sorted) {
    while (w < windows.length && windows[w].to < it.ts) w++;
    const win = windows[w];
    if (!win || it.ts < win.from) continue;
    const target = win.name;
    if (!per.has(target)) per.set(target, { n: 0, buckets: {}, gpuProcess: 0, top: {} });
    const p = per.get(target);
    if (it.gpu) {
      if (it.top) p.gpuProcess += it.dur / 1000;
      continue;
    }
    const b = bucketOf(it.name);
    p.buckets[b] = (p.buckets[b] ?? 0) + Math.max(0, it.self) / 1000;
    p.top[it.name] = (p.top[it.name] ?? 0) + Math.max(0, it.self) / 1000;
  }
  for (const win of windows) if (per.has(win.name)) per.get(win.name).n++;
  const out = {};
  for (const [k, p] of per) {
    const n = Math.max(1, p.n);
    out[k] = {
      n: p.n,
      perSwitch: Object.fromEntries(Object.entries(p.buckets).map(([b, v]) => [b, v / n])),
      gpuProcess: p.gpuProcess / n,
      topEvents: Object.entries(p.top)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 6)
        .map(([name, v]) => ({ name, ms: v / n })),
    };
  }
  return out;
}

async function measureTrace() {
  stage("a traced pass of the pages and the tabs");
  const tour = everyPair(PAGES);
  await goPage(tour[0]);
  await settle();
  results.summary.tracePages = await trace("pages", async () => {
    for (let i = 1; i < tour.length; i++) {
      await goPage(tour[i]);
      progress("traced pages", i, tour.length - 1);
    }
  });
  const tabs = everyPair(TABS);
  await openComponent(COMPONENTS[0]);
  await settle();
  results.summary.traceTabs = await trace("tabs", async () => {
    for (const c of COMPONENTS) {
      await openComponent(c, `open ${c}`);
      await sleep(400);
      for (let i = 1; i < tabs.length; i++) await timed('[role="tablist"] [role="tab"]', tabs[i], `tab ${tabs[i]}`);
      await timed('[role="tablist"] [role="tab"]', "Design");
    }
  });
}

// ---- the report ----

function environment() {
  const commit = spawnSync("git", ["-C", repo, "rev-parse", "--short", "HEAD"], { encoding: "utf8" }).stdout.trim();
  const stat = fs.statSync(EXE);
  return {
    exe: EXE,
    exeBuilt: stat.mtime.toISOString(),
    exeMB: +(stat.size / 1e6).toFixed(1),
    commit,
    cpu: os.cpus()[0]?.model.trim(),
    threads: os.cpus().length,
    memoryGB: +(os.totalmem() / 2 ** 30).toFixed(0),
    os: `${os.type()} ${os.release()}`,
    view: `${VIEW_W} x ${VIEW_H} CSS px at scale 1`,
    workspace: SEED ? `seeded: ${["jobs", "runs", "circuits"].map((d) => `${fs.readdirSync(path.join(WORKSPACE, d)).length} ${d}`).join(", ")}` : "empty",
    date: new Date().toISOString(),
  };
}

function table(head, rows) {
  return [`| ${head.join(" | ")} |`, `| ${head.map((h, i) => (i ? "---:" : "---")).join(" | ")} |`, ...rows.map((r) => `| ${r.join(" | ")} |`)].join("\n");
}

function report(before) {
  const S = results.summary;
  const lines = [`# Studio switch times: ${LABEL}`, ""];
  const e = results.environment;
  lines.push(`Build ${e.exe} (${e.exeMB} MB, built ${e.exeBuilt}), commit ${e.commit}; ${e.cpu}, ${e.threads} threads, ${e.memoryGB} GB; ${e.os}; ${e.view}; WebView2 ${e.webview}, ${e.webgl}; the workspace ${e.workspace}.`, "");
  lines.push("Times in ms: the median, and in brackets the 10th to the 90th percentile. *frame*: the click to the first frame painted after it; *settled*: the click to the page's last change (data loaded and drawn).", "");
  const was = (group, key, field) => before?.summary?.[group]?.[key]?.[field]?.median;
  const change = (now, then) => (Number.isFinite(then) && then > 0 ? `${ms(then)} → ${ms(now)} (${(((now - then) / then) * 100).toFixed(0)}%)` : "");
  const rowsOf = (group) =>
    Object.entries(S[group] ?? {}).map(([k, v]) => {
      const cmds = Object.entries(v.ipc)
        .sort((a, b) => b[1].ms.median - a[1].ms.median)
        .slice(0, 2)
        .map(([c, x]) => `${c} ${ms(x.ms.median)}`)
        .join(", ");
      const row = [k, v.frame.n, spread(v.frame), spread(v.settled) + (v.capped ? ` ${v.capped} capped` : ""), ms(v.script.median), ms(v.styleLayout.median), ms(v.long.median), v.glMade ? `${v.glMade.toFixed(1)} (${ms(v.glMs.median)} ms)` : "0", cmds || "–"];
      if (before) row.push(change(v.frame.median, was(group, k, "frame")), change(v.settled.median, was(group, k, "settled")));
      return row;
    });
  const head = ["To", "n", "frame", "settled", "script", "style + layout", "long tasks", "WebGL contexts made", "slowest commands"];
  if (before) head.push("frame before → after", "settled before → after");
  if (S.pages) lines.push("## Pages", "", `Each of the ${PAGES.length} pages from each of the others, ${REPS} times; by the page switched to.`, "", table(head, rowsOf("pages")), "");
  if (S.tabs) lines.push("## A component's tabs", "", `Each tab from each of the others, ${TAB_REPS} times, on each component.`, "", table(head, rowsOf("tabs")), "");
  if (S.open) lines.push("## Opening a component from the list", "", `Each, ${OPEN_REPS} times, from the one before it.`, "", table(head, rowsOf("open")), "");
  if (S.scroll) {
    const s = S.scroll;
    lines.push("## Scrolling the components list", "", `${s.rows} rows, every group open: ${s.listHeight} px of list in a ${s.viewHeight} px view; 40 wheel notches down and 40 up, ${SCROLL_REPS} times. Frame intervals in ms; a slow frame takes over 25 ms. At rest, nothing scrolled, the window paints every ${ms(s.rest.median)} ms (${ms(s.rest.p10)}–${ms(s.rest.p90)}).`, "");
    lines.push(table(["", "median", "p10–p90 of the passes"], [["median frame interval", ms(s.median.median), `${ms(s.median.p10)}–${ms(s.median.p90)}`], ["95th percentile frame", ms(s.p95.median), `${ms(s.p95.p10)}–${ms(s.p95.p90)}`], ["longest frame", ms(s.max.median), `${ms(s.max.p10)}–${ms(s.max.p90)}`], ["slow frames (%)", ms(s.slowShare.median), `${ms(s.slowShare.p10)}–${ms(s.slowShare.p90)}`], ["long tasks (ms per pass)", ms(s.long.median), `${ms(s.long.p10)}–${ms(s.long.p90)}`]]), "");
  }
  if (S.contexts) {
    const c = S.contexts;
    lines.push("## WebGL contexts", "", `A round: ${c.round}. After each round and a garbage collection:`, "");
    const keys = ["made", "released", "evicted", "onPage", "lostOnPage", "leftToGc"];
    lines.push(table(["", "made", "released by the studio", "evicted by the browser", "on the page", "on the page, evicted", "off the page, not released (one is the cards' shared context)"], [["before", ...keys.map((k) => c.before[k])], ...c.rounds.map((r, i) => [`after round ${i + 1}`, ...keys.map((k) => r[k])])]), "");
  }
  if (S.heap) {
    const h = S.heap;
    lines.push("## The heap over page switches", "", "After a garbage collection, on Home:", "");
    lines.push(table(["", "JS heap (MB)", "DOM nodes", "listeners"], h.points.map((p, i) => [["at the start", "after 50 switches", "after 100 switches"][i], p.heapMB.toFixed(1), p.nodes, p.listeners])), "");
  }
  if (S.idle) {
    lines.push("## Each page at rest", "", "The renderer's main thread busy over 3 s with nothing done:", "");
    lines.push(table(["Page", "busy (%)", "script (%)"], Object.entries(S.idle).map(([k, v]) => [k, v.busyPct.toFixed(1), v.scriptPct.toFixed(1)])), "");
  }
  for (const [name, t] of [["pages", S.tracePages], ["tabs", S.traceTabs]]) {
    if (!t) continue;
    const buckets = ["script", "style", "layout", "paint", "gc", "gpu wait", "tasks (other)", "other"];
    lines.push(`## Traced: the ${name}, the main thread's time per switch (ms)`, "");
    lines.push(table(["Switch", "n", ...buckets, "GPU process", "largest events"], Object.entries(t).map(([k, v]) => [k, v.n, ...buckets.map((b) => ms(v.perSwitch[b] ?? 0)), ms(v.gpuProcess), v.topEvents.slice(0, 3).map((x) => `${x.name} ${ms(x.ms)}`).join(", ")])), "");
  }
  return lines.join("\n");
}

// ---- the run ----

fs.mkdirSync(OUT, { recursive: true });
await launch();
if (DISCOVER) {
  await componentsPage();
  const rows = await js(`return [...document.querySelectorAll(args[0] + " button")].map((b) => b.textContent.replace(/\\s+/g, " ").trim());`, LIST);
  console.log(rows.join("\n"));
  process.exit(0);
}
results.environment = environment();
results.environment.webgl = await js(`const g = document.createElement("canvas").getContext("webgl2"); const x = g?.getExtension("WEBGL_debug_renderer_info"); const r = x ? g.getParameter(x.UNMASKED_RENDERER_WEBGL) : null; g?.getExtension("WEBGL_lose_context")?.loseContext(); return r;`);
results.environment.webview = (await cdp.send("Browser.getVersion").catch(() => ({}))).product ?? null;
const run = { pages: measurePages, tabs: measureTabs, open: measureOpen, scroll: measureScroll, idle: measureIdle, trace: measureTrace, contexts: measureContexts, heap: measureHeap };
for (const name of ALL) {
  if (!ONLY.has(name)) continue;
  try {
    await run[name]();
  } catch (e) {
    log(`${name} failed: ${e.message}`);
    results.summary[`${name}Problem`] = String(e.message);
  }
  fs.writeFileSync(path.join(OUT, `measure-${LABEL}.json`), JSON.stringify(results, null, 1));
}
const before = COMPARE ? JSON.parse(fs.readFileSync(COMPARE, "utf8")) : null;
fs.writeFileSync(path.join(OUT, `measure-${LABEL}.md`), report(before));
log(`done in ${((Date.now() - started) / 60000).toFixed(1)} min: ${path.join(OUT, `measure-${LABEL}.json`)} and .md`);
// the DevTools socket would keep Node running: close the program and put everything back now
cdp.ws.close();
process.exit(0);
