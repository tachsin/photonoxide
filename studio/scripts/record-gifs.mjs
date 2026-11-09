#!/usr/bin/env node
// Records the README's GIFs of the studio (assets/studio/*.gif) from the built program.
//
//   cd studio && pnpm tauri build                   # target/release/photonoxide.exe
//   node scripts/record-gifs.mjs                     # every GIF
//   node scripts/record-gifs.mjs --only hero,academy # some of them
//   node scripts/record-gifs.mjs --exe <a build> --config <its settings folder>   # another build, as its own app
//
// Windows only: it drives the window's WebView2 over the Chrome DevTools Protocol. It needs
// Node 24 or newer (its global WebSocket), and ffmpeg and gifski on the PATH
// (`winget install ffmpeg`, `cargo install gifski`).
//
// What it does:
// 1. Backs up the studio's settings folder (%APPDATA%\gr.tachsin.photonoxide) and writes the
//    recording's settings: photonoxide dark, no tips, no tour, no update check. Everything is put
//    back as it was at the end, also when the recording fails or is interrupted.
// 2. Starts the program in a fresh workspace, C:\photonoxide-demo (empty jobs/, runs/, circuits/),
//    so the status bar shows no one's home folder, with a WebView2 profile of its own in a temporary
//    folder and the DevTools port 9228 (--port changes it).
// 3. Plays each scene with a drawn pointer that moves smoothly, screencasting the page at
//    1440 x 900 (the window sized and placed to fit the screen's work area, behind the others).
//    A long run is recorded as a time-lapse, with a badge saying how much faster.
// 4. Closes the program (by its process id), then resamples each scene to a steady frame rate,
//    crossfades between takes and from the end back to the start (so each GIF loops seamlessly),
//    scales with Lanczos (ffmpeg) and encodes with gifski.
//
// Options: --only <names>, --exe <path>, --config <folder> (the settings folder of the build --exe
// names: another identifier's, so a check build leaves the installed studio's alone), --port <n>,
// --workspace <folder>, --keep (keep the frames),
// --frames <folder> (compose frames an earlier --keep kept, without recording).

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..", "..");

// ---- options ----

const argv = process.argv.slice(2);
const opt = (name, fallback) => {
  const k = argv.indexOf(`--${name}`);
  return k >= 0 ? argv[k + 1] : fallback;
};
const OUT = path.join(repo, "assets", "studio");
const EXE = path.resolve(opt("exe", path.join(repo, "target", "release", "photonoxide.exe")));
const PORT = Number(opt("port", "9228"));
const WORKSPACE = path.resolve(opt("workspace", `${process.env.SystemDrive ?? "C:"}\\photonoxide-demo`));
const KEEP = argv.includes("--keep");
/** A folder kept by an earlier --keep: compose its takes again without recording. */
const REUSE = opt("frames", null);
/** The page's size while recording, in CSS pixels. */
const VIEW = { width: 1440, height: 900 };

const GIFS = {
  // the README's top, under 35 s, eight scenes of at least 3.5 s each, crossfades left out, and no captions: an MMI's
  // FDTD run; a ring's spectrum building up, then the ring lit at resonance (two takes, crossfaded);
  // a directional coupler's and a Y-branch's design moved by their sliders; an MZI wired on the
  // chip; a heater switched on, its heat spreading; the ring's fabrication flow; a lesson's live
  // chart taken to critical coupling
  hero: {
    width: 1000,
    fps: 12,
    fade: 5,
    // full motion quality: with less, a page left lingers faintly under the next one's text
    quality: [72, 100, 50],
    takes: ["fdtd", "ring", "ring3d", "coupler", "ybranch", "chip", "heater", "fab", "lesson"],
    // the takes that make one scene, for the seconds each scene is on screen
    // the ring's sweep: its time-lapse's last seconds only, however long this machine took
    lapse: { ring: 2.6 },
    scenes: [["fdtd"], ["ring", "ring3d"], ["coupler"], ["ybranch"], ["chip"], ["heater"], ["fab"], ["lesson"]],
  },
  device: { width: 860, fps: 12, fade: 8, quality: [85, 75, 70], takes: ["device"] },
  materials: { width: 860, fps: 12, fade: 8, quality: [85, 75, 70], takes: ["materials"] },
  validation: { width: 860, fps: 12, fade: 8, quality: [85, 75, 70], takes: ["validation"] },
  // full motion quality: with less, the lesson's text scrolled past lingers faintly under the chart
  academy: { width: 860, fps: 12, fade: 8, quality: [85, 100, 70], takes: ["academy"] },
};
const only = opt("only", Object.keys(GIFS).join(",")).split(",");
for (const name of only) if (!GIFS[name]) fail(`no GIF called ${name}: ${Object.keys(GIFS).join(", ")}`);
const wanted = new Set(only.flatMap((g) => GIFS[g].takes));

function fail(message) {
  console.error(`record-gifs: ${message}`);
  process.exit(1);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

// ---- the tools ----

if (process.platform !== "win32") fail("this records the Windows build (WebView2); run it on Windows");
if (typeof WebSocket === "undefined") fail("Node 24 or newer is needed, for its WebSocket");
if (!fs.existsSync(EXE)) fail(`${EXE} is missing: build it first (cd studio && pnpm tauri build)`);
for (const tool of ["ffmpeg", "gifski"]) {
  if (spawnSync(tool, ["--version"], { stdio: "ignore" }).error && spawnSync(tool, ["-version"], { stdio: "ignore" }).error) {
    fail(`${tool} isn't on the PATH (winget install ffmpeg; cargo install gifski)`);
  }
}

// ---- the settings, backed up and restored ----

const CONFIG = path.resolve(opt("config", path.join(process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming"), "gr.tachsin.photonoxide")));
const backup = REUSE ? null : { existed: fs.existsSync(CONFIG), files: new Map() };
if (backup?.existed) {
  for (const f of fs.readdirSync(CONFIG, { withFileTypes: true })) {
    if (f.isFile()) backup.files.set(f.name, fs.readFileSync(path.join(CONFIG, f.name)));
  }
}

function restoreSettings() {
  if (!backup) return;
  if (!backup.existed) {
    fs.rmSync(CONFIG, { recursive: true, force: true });
    return;
  }
  for (const f of fs.readdirSync(CONFIG, { withFileTypes: true })) {
    if (f.isFile() && !backup.files.has(f.name)) fs.rmSync(path.join(CONFIG, f.name));
  }
  for (const [name, bytes] of backup.files) fs.writeFileSync(path.join(CONFIG, name), bytes);
}

const RECORDING_SETTINGS = { theme: "dark", workspace: null, check_updates: false, hints: false, welcomed: true, dismissed: [], view: "3d", length_unit: "um", paper_feed: false };

// ---- the run's own folders ----

const TEMP = REUSE ? path.resolve(REUSE) : fs.mkdtempSync(path.join(os.tmpdir(), "photonoxide-gifs-"));
const FRAMES = path.join(TEMP, "frames");
const PROFILE = path.join(TEMP, "webview2");
fs.mkdirSync(FRAMES, { recursive: true });
const madeWorkspace = !REUSE && !fs.existsSync(WORKSPACE);
if (!REUSE && !madeWorkspace && fs.readdirSync(WORKSPACE).some((f) => !["jobs", "runs", "circuits"].includes(f) || fs.readdirSync(path.join(WORKSPACE, f)).length)) {
  fail(`${WORKSPACE} exists and isn't empty: remove it, or pass --workspace <an empty folder>`);
}
if (!REUSE) for (const d of ["jobs", "runs", "circuits"]) fs.mkdirSync(path.join(WORKSPACE, d), { recursive: true });

let child = null;

function closeProgram() {
  if (child?.pid && child.exitCode === null) {
    // this program and its web view's processes, by its process id only
    spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  }
  child = null;
}

let cleaned = false;
function cleanUp() {
  if (cleaned) return;
  cleaned = true;
  closeProgram();
  try {
    restoreSettings();
  } catch (e) {
    console.error(`record-gifs: couldn't restore the settings in ${CONFIG}: ${e}`);
  }
  if (madeWorkspace) remove(WORKSPACE);
  if (!KEEP && !REUSE) remove(TEMP);
}

/** Removes a folder, waiting for a program just killed to let go of it. */
function remove(dir) {
  for (let i = 0; i < 20; i++) {
    try {
      fs.rmSync(dir, { recursive: true, force: true });
      return;
    } catch {
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
    }
  }
  console.error(`record-gifs: couldn't remove ${dir}`);
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

/** Waits until `body` returns something truthy in the page, and returns it. */
async function until(body, what, ms = 30000, ...args) {
  const end = Date.now() + ms;
  for (;;) {
    const v = await js(body, ...args).catch(() => null);
    if (v) return v;
    if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
    await sleep(150);
  }
}

// what the page gets: a drawn pointer, element lookup, and a badge for time-lapses
const PAGE_HELPERS = String.raw`
if (!window.__rec) {
  const style = document.createElement("style");
  style.textContent = ${"`"}
    #rec-cursor { position: fixed; left: 0; top: 0; z-index: 2147483647; pointer-events: none; transform: translate(-200px, -200px); }
    #rec-cursor svg { display: block; filter: drop-shadow(0 1px 1.5px rgb(0 0 0 / 0.55)); }
    #rec-ripple { position: fixed; left: 0; top: 0; width: 30px; height: 30px; margin: -15px 0 0 -15px; border-radius: 50%;
      z-index: 2147483646; pointer-events: none; border: 2px solid #6cb4ff; background: rgb(108 180 255 / 0.18); opacity: 0; }
    #rec-badge { position: fixed; top: 64px; left: 50%; transform: translateX(-50%); z-index: 2147483645; pointer-events: none;
      display: none; align-items: center; gap: 6px; padding: 5px 12px 5px 10px; border-radius: 999px;
      font: 600 13px/1 Inter Variable, Inter, system-ui, sans-serif; color: #e8edf5; letter-spacing: 0.01em;
      background: rgb(15 17 21 / 0.82); border: 1px solid rgb(255 255 255 / 0.14); box-shadow: 0 6px 20px rgb(0 0 0 / 0.35); }
    /* no toasts in the recordings, and no busy spinner flickering by a chart while its slider moves */
    [aria-label=Computing] { visibility: hidden !important; }
    div.pointer-events-none.fixed.z-50.w-96 { display: none !important; }
  ${"`"};
  document.head.append(style);
  const cursor = document.createElement("div");
  cursor.id = "rec-cursor";
  cursor.innerHTML = '<svg width="20" height="24" viewBox="0 0 20 24"><path d="M2 1.5 L2 19.5 L6.6 15.3 L9.6 22 L12.6 20.7 L9.7 14.2 L16 14.2 Z" fill="#fff" stroke="#111" stroke-width="1.4" stroke-linejoin="round"/></svg>';
  const ripple = document.createElement("div");
  ripple.id = "rec-ripple";
  const badge = document.createElement("div");
  badge.id = "rec-badge";
  document.body.append(ripple, cursor, badge);
  let shown = false;
  const at = (x, y) => (cursor.style.transform = "translate(" + (x - 2) + "px, " + (y - 1.5) + "px)");
  addEventListener("pointermove", (e) => shown && at(e.clientX, e.clientY), true);
  addEventListener("pointerdown", (e) => {
    if (!shown) return;
    ripple.style.left = e.clientX + "px";
    ripple.style.top = e.clientY + "px";
    ripple.animate([{ opacity: 0.9, transform: "scale(0.35)" }, { opacity: 0, transform: "scale(1.25)" }], { duration: 480, easing: "ease-out" });
  }, true);
  const visible = (e) => {
    const r = e.getBoundingClientRect();
    if (!r.width || !r.height) return false;
    const s = getComputedStyle(e);
    return s.visibility !== "hidden" && s.display !== "none";
  };
  const norm = (s) => s.replace(/\s+/g, " ").trim();
  window.__rec = {
    cursor(on, x, y) {
      shown = on;
      cursor.style.display = on ? "" : "none";
      if (on && x !== undefined) at(x, y);
    },
    badge(text, top = 64) {
      badge.style.display = text ? "flex" : "none";
      badge.style.top = top + "px";
      badge.innerHTML = text ? '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#6cb4ff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 19 22 12 13 5 13 19"/><polygon points="2 19 11 12 2 5 2 19"/></svg>' + text : "";
    },
    /** The visible element matching a selector, by its text when given (an exact match first), and which of them. */
    find(selector, text, nth = 0) {
      let all = [...document.querySelectorAll(selector)].filter(visible);
      if (text != null) {
        const exact = all.filter((e) => norm(e.textContent) === text);
        all = exact.length ? exact : all.filter((e) => norm(e.textContent).includes(text));
      }
      return all[nth] ?? null;
    },
    box(selector, text, nth) {
      const e = this.find(selector, text, nth);
      if (!e) return null;
      const r = e.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2, left: r.left, top: r.top, width: r.width, height: r.height };
    },
  };
}
`;

// ---- the pointer ----

const mouse = { x: VIEW.width / 2, y: VIEW.height / 2, down: false };
const ease = (t) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);

async function mouseEvent(type, x, y, extra = {}) {
  await cdp.send("Input.dispatchMouseEvent", {
    type,
    x,
    y,
    button: mouse.down || type !== "mouseMoved" ? "left" : "none",
    buttons: mouse.down ? 1 : 0,
    clickCount: type === "mouseMoved" ? 0 : 1,
    ...extra,
  });
}

/** Moves the pointer to (x, y) along an eased path over `ms`, as a hand would. */
async function moveTo(x, y, ms = 600, curve = ease) {
  const from = { ...mouse };
  const start = performance.now();
  // a slight arc, so moves don't look ruled
  const bend = Math.min(40, Math.hypot(x - from.x, y - from.y) * 0.08);
  for (;;) {
    const t = Math.min(1, (performance.now() - start) / ms);
    const e = curve(t);
    const arc = Math.sin(Math.PI * e) * bend;
    mouse.x = from.x + (x - from.x) * e;
    mouse.y = from.y + (y - from.y) * e - arc;
    await mouseEvent("mouseMoved", mouse.x, mouse.y);
    if (t >= 1) break;
    await sleep(14);
  }
}

async function press() {
  mouse.down = true;
  await mouseEvent("mousePressed", mouse.x, mouse.y);
}
async function release() {
  await mouseEvent("mouseReleased", mouse.x, mouse.y);
  mouse.down = false;
}

/** Finds an element (see `__rec.find`), waiting for it to show. */
async function box(selector, text, nth = 0, ms = 20000) {
  return until(`return __rec.box(args[0], args[1], args[2]);`, `${selector}${text ? ` "${text}"` : ""}`, ms, selector, text ?? null, nth);
}

/** Moves to an element and clicks it. */
async function click(selector, text, { nth = 0, ms = 650, pause = 140, dx = 0, dy = 0 } = {}) {
  await box(selector, text, nth);
  // scrolled into its list's view first, when it is out of it
  const moved = await js(
    `const e = __rec.find(args[0], args[1], args[2]);
     const before = e.getBoundingClientRect().top;
     e.scrollIntoView({ block: "nearest", behavior: "smooth" });
     await new Promise((r) => setTimeout(r, 50));
     return e.getBoundingClientRect().top !== before;`,
    selector,
    text ?? null,
    nth,
  );
  if (moved) await sleep(600);
  const b = await box(selector, text, nth);
  await moveTo(b.x + dx, b.y + dy, ms);
  await sleep(pause);
  await press();
  await sleep(70);
  await release();
}

/** Presses the left button at one point and lets go at another, moving smoothly between. */
async function drag(from, to, ms = 900, curve = ease, approach = 500) {
  if (Math.hypot(from.x - mouse.x, from.y - mouse.y) > 2) {
    await moveTo(from.x, from.y, approach);
    await sleep(120);
  }
  await press();
  await moveTo(to.x, to.y, ms, curve);
  await sleep(100);
  await release();
}

async function key(keyName, { code = keyName, keyCode = 0, modifiers = 0, text } = {}) {
  await cdp.send("Input.dispatchKeyEvent", { type: text ? "keyDown" : "rawKeyDown", key: keyName, code, windowsVirtualKeyCode: keyCode, modifiers, text });
  await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: keyName, code, windowsVirtualKeyCode: keyCode, modifiers });
}
const KEYS = { ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Delete: 46, Escape: 27, Enter: 13 };
const pressKey = (k) => key(k, { keyCode: KEYS[k] });

/** Shows (or hides) the drawn pointer, at the pointer's place. */
const cursor = (on) => js(`__rec.cursor(args[0], args[1], args[2]);`, on, mouse.x, mouse.y);
/** Goes to a page by the navigation rail, without being recorded; the System group's opened first when it is folded. */
const goTo = (label) =>
  js(`const find = () => document.querySelector("nav button[aria-label='" + args[0] + "']");
      if (!find()) document.querySelector("nav button[aria-label='System']").click();
      await new Promise((r) => setTimeout(r, 50));
      find().click();`, label);

/** The page right of the navigation rail, above the status bar. */
const railless = () =>
  js(`const nav = document.querySelector("nav").getBoundingClientRect();
      const foot = document.querySelector("footer").getBoundingClientRect();
      return { x: nav.right, y: 0, width: innerWidth - nav.right, height: foot.top };`);
/** Folds the navigation rail to its icons, or opens it again. */
async function fold(folded) {
  const now = await js(`return document.querySelector("nav").getBoundingClientRect().width < 120;`);
  // the rail's Collapse button folds it (its label goes when folded: the one before the status then)
  if (now !== folded)
    await js(`const all = [...document.querySelectorAll("nav button")];
              (all.find((b) => b.textContent.trim() === "Collapse") ?? all.at(-2)).click();`);
  await sleep(300);
}

// ---- recording ----

/**
 * A take: the page's frames as it plays, with their times, and the marks where its speed
 * changes (a speed of n plays n times faster; Infinity cuts).
 */
class Recorder {
  constructor() {
    this.take = null;
    this.lapse = null;
    cdp.on("Page.screencastFrame", ({ data, metadata, sessionId }) => {
      cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
      if (this.take && !this.lapse) this.keep(data, metadata.timestamp);
    });
  }
  keep(data, t) {
    const take = this.take;
    const file = path.join(take.dir, `${String(take.frames.length).padStart(5, "0")}.png`);
    fs.writeFileSync(file, Buffer.from(data, "base64"));
    take.frames.push({ t, file });
  }
  async begin(name) {
    const dir = path.join(FRAMES, name);
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir, { recursive: true });
    this.take = { name, dir, frames: [], marks: [{ t: Date.now() / 1000, speed: 1 }], crop: null };
    await cdp.send("Page.startScreencast", { format: "png", everyNthFrame: 2 });
    log(`recording ${name}`);
  }
  /** From now on, play `speed` times faster; a speed over 3 is a time-lapse of screenshots. */
  async speed(speed, fps = 15) {
    this.take.marks.push({ t: Date.now() / 1000, speed });
    if (speed > 3 && Number.isFinite(speed) && !this.lapse) {
      const every = (speed / fps) * 1000;
      this.lapse = { on: true };
      const lapse = this.lapse;
      (async () => {
        while (lapse.on) {
          const t0 = Date.now();
          const shot = await cdp.send("Page.captureScreenshot", { format: "png" }).catch(() => null);
          if (shot && lapse.on) this.keep(shot.data, t0 / 1000);
          await sleep(Math.max(0, every - (Date.now() - t0)));
        }
      })();
    } else if (!(speed > 3 && Number.isFinite(speed)) && this.lapse) {
      this.lapse.on = false;
      this.lapse = null;
    }
  }
  /** Keeps only this part of the page (CSS pixels, whole ones). */
  crop(box) {
    const r = (v) => Math.round(v);
    this.take.crop = { x: r(box.x), y: r(box.y), width: r(box.width) & ~1, height: r(box.height) & ~1 };
  }
  /** Leaves out what follows, until the next `speed`. */
  cut() {
    return this.speed(Infinity);
  }
  async end() {
    if (this.lapse) this.lapse.on = false;
    this.lapse = null;
    const take = this.take;
    take.end = Date.now() / 1000;
    await cdp.send("Page.stopScreencast");
    await sleep(100);
    this.take = null;
    fs.writeFileSync(path.join(take.dir, "take.json"), JSON.stringify({ ...take, dir: undefined }));
    log(`  ${take.frames.length} frames over ${(take.end - take.marks[0].t).toFixed(1)} s`);
    return take;
  }
}

// ---- the scenes ----

/** Scrolls the material's index panel smoothly so its plot shows whole, and its inputs above it. */
async function showIndexPlot() {
  await js(`const section = __rec.find("h3", "Refractive index").closest("section");
     const plot = section.querySelector("svg[role=img]").getBoundingClientRect();
     const box = section.closest(".overflow-y-auto");
     const bottom = document.querySelector("footer").getBoundingClientRect().top - 20;
     let by = plot.bottom + 30 - bottom;
     by = Math.min(by, section.getBoundingClientRect().top - 66);
     box.scrollTo({ top: box.scrollTop + by, behavior: "smooth" });`);
  await sleep(700);
}

let rec;

/** Runs a template from Home's examples (every topic of its accordion opened), unrecorded. */
async function runExample(name) {
  await goTo("Home");
  await until(`return document.querySelector("[id^=ex-head-]");`, "the examples", 20000);
  await js(`for (const b of document.querySelectorAll("[id^=ex-head-]")) if (b.getAttribute("aria-expanded") === "false") b.click();`);
  await until(
    `const li = [...document.querySelectorAll("li")].find((l) => l.querySelector("span.font-medium")?.textContent.trim() === args[0]);
     return li && [...li.querySelectorAll("button")].some((b) => b.textContent.trim() === "Run");`,
    `the ${name} example`,
    20000,
    name,
  );
  await js(
    `const li = [...document.querySelectorAll("li")].find((l) => l.querySelector("span.font-medium")?.textContent.trim() === args[0]);
     [...li.querySelectorAll("button")].find((b) => b.textContent.trim() === "Run").click();`,
    name,
  );
  // its run's page, open on it
  await until(`return [...document.querySelectorAll("h1")].some((h) => h.textContent.replace(/\\s+/g, " ").trim() === "Run · " + args[0]);`, `the ${name} run`, 30000, name);
  await sleep(300);
}

/**
 * What the hero keeps of every take, the same for all: the page down to the top of the rail's
 * Collapse row, measured once with the rail open. The rail's last two rows (Collapse, and the runs'
 * status) and the status bar beside them (the scratch workspace's path) are left out whole, so no
 * line is cut at the foot.
 */
let heroCrop = null;
const measureHeroCrop = async () => {
  heroCrop = await js(`const all = [...document.querySelectorAll("nav button")];
     const collapse = all.find((b) => b.textContent.trim() === "Collapse") ?? all.at(-2);
     return { x: 0, y: 0, width: innerWidth, height: Math.floor(collapse.getBoundingClientRect().top) - 2 };`);
  log(`  the hero keeps the page's top ${heroCrop.height} px`);
};

/**
 * Scrolls each of the page's scrolling panes a few pixels, the least that does it, so no line of
 * text is cut through by the hero's foot or by the pane's own top.
 */
const tidyFoot = () =>
  js(
    `const foot = args[0];
     const boxes = [...document.querySelectorAll("main *")].filter((b) => {
       const s = getComputedStyle(b);
       return b.scrollHeight > b.clientHeight + 4 && /auto|scroll/.test(s.overflowY) && b.getBoundingClientRect().bottom > foot;
     });
     const said = [];
     for (const box of boxes) {
       const r = box.getBoundingClientRect();
       // the lines of text inside it, each line of a paragraph on its own, and its fields
       const lines = [...box.querySelectorAll("input, select, textarea")].map((e) => e.getBoundingClientRect());
       const walk = document.createTreeWalker(box, NodeFilter.SHOW_TEXT);
       const range = document.createRange();
       for (let n = walk.nextNode(); n; n = walk.nextNode()) {
         if (!n.textContent.trim()) continue;
         range.selectNodeContents(n);
         lines.push(...range.getClientRects());
       }
       const near = lines.filter((q) => q.height > 0 && q.width > 0 && q.bottom > r.top - 60 && q.top < foot + 60);
       const cuts = (d) => near.filter((q) => {
         const top = q.top - d, bottom = q.bottom - d;
         return (top < foot + 3 && bottom > foot - 3) || (top < r.top + 2 && bottom > r.top - 2);
       }).length;
       if (!cuts(0)) continue;
       let best = null;
       for (let k = 1; k <= 48 && best === null; k++) {
         for (const d of [k, -k]) {
           const to = box.scrollTop + d;
           if (to < 0 || to > box.scrollHeight - box.clientHeight) continue;
           if (!cuts(d)) { best = d; break; }
         }
       }
       if (best !== null) { box.scrollTop += best; said.push(best); }
     }
     return said;`,
    // the crop's height as the take keeps it (even)
    heroCrop.height & ~1,
  );

async function tidy() {
  const moved = await tidyFoot();
  if (moved.length) {
    log(`  panes scrolled ${moved.join(", ")} px so no line is cut`);
    await sleep(250);
  }
}

/**
 * Begins a take of the hero, cropped as every other, its panes tidied at the foot first (a scene
 * that puts the pointer on something tidies before it, so the pointer stays on it).
 */
async function heroTake(name) {
  await tidy();
  await rec.begin(name);
  rec.crop(heroCrop);
}
async function heroEnd() {
  await rec.end();
  await cursor(false);
}

/** Opens a component from the Components page's list, unrecorded (the list unfolded first when it is folded). */
async function openComponent(name) {
  await goTo("Components");
  await sleep(600);
  // pressing Components again, on its page, unfolds its list
  await js(`if (!__rec.find("nav button, aside button", args[0])) document.querySelector("nav button[aria-label='Components']").click();`, name);
  await until(`return __rec.find("nav button, aside button", args[0]);`, `${name} in the list`, 20000, name);
  await js(`__rec.find("nav button, aside button", args[0]).click();`, name);
  await until(`return [...document.querySelectorAll("main h1, main h2")].some((h) => h.textContent.replace(/\\s+/g, " ").trim().startsWith(args[0]));`, `the ${name}'s page`, 20000, name);
  await sleep(1200);
}

/** Scrolls a design's panel so the field `name` sits `margin` below the panel's top. */
const fieldIntoView = (name, margin = 80) =>
  js(
    `const e = document.getElementById("field-" + args[0]);
     let box = e.parentElement;
     while (box && !(box.scrollHeight > box.clientHeight + 4 && /auto|scroll/.test(getComputedStyle(box).overflowY))) box = box.parentElement;
     if (!box) return false;
     box.scrollTo({ top: box.scrollTop + e.getBoundingClientRect().top - box.getBoundingClientRect().top - args[1] });
     return true;`,
    name,
    margin,
  );

/** A design's slider, by its field's name: where it lies, its range and its value. */
const designSlider = (name) =>
  js(
    `const e = document.getElementById("field-" + args[0]).closest("div").parentElement.querySelector("input[type=range]");
     const r = e.getBoundingClientRect();
     return { left: r.left + r.height / 2, width: r.width - r.height, y: r.top + r.height / 2, min: Number(e.min), max: Number(e.max), value: Number(e.value) };`,
    name,
  );

/** Where a design's slider's thumb is. */
const designThumb = async (name) => {
  const s = await designSlider(name);
  return { x: s.left + (s.width * (s.value - s.min)) / (s.max - s.min), y: s.y };
};

/** Drags a design's slider from its value to a fraction of its range. */
async function dragDesign(name, to, ms, approach = 450) {
  const s = await designSlider(name);
  await drag(await designThumb(name), { x: s.left + s.width * to, y: s.y }, ms, ease, approach);
}

/** Scrolls the nearest scrolling box around the element `selector` finds, so the element's top sits `margin` below the box's. */
const scrollTo = (selector, text, margin = 12, smooth = true) =>
  js(
    `const e = __rec.find(args[0], args[1]);
     let box = e.parentElement;
     while (box && !(box.scrollHeight > box.clientHeight + 4 && /auto|scroll/.test(getComputedStyle(box).overflowY))) box = box.parentElement;
     if (!box) return false;
     box.scrollTo({ top: box.scrollTop + e.getBoundingClientRect().top - box.getBoundingClientRect().top - args[2], behavior: args[3] ? "smooth" : "instant" });
     return true;`,
    selector,
    text ?? null,
    margin,
    smooth,
  );

const finished = () => until(`return __rec.find("span.badge", "finished in");`, "the run to finish", 20 * 60 * 1000);

const SCENES = {
  /**
   * A ring's spectrum building up live (a time-lapse); its take ends on the spectrum. The 3D view
   * is then opened and drawn off the record, and `ring3d` begins on it, so the GIF crossfades from
   * the spectrum straight to the ring: the field's 2D picture, hidden while the sweep runs, never
   * shows between them.
   */
  async ring() {
    await moveTo(VIEW.width / 2, VIEW.height / 2, 10);
    await runExample("ring-fdfd");
    await until(`return [...document.querySelectorAll("main button")].some((b) => b.textContent.trim() === "2D" && b.getBoundingClientRect().width);`, "the viewer");
    await js(`__rec.find("main button", "2D").click();`);
    // the spectrum alone while the run plays: the field's picture, which arrives above it, hidden
    await until(`return __rec.find("h3", "S-parameters");`, "the first S-parameters", 120000);
    await js(
      `clearInterval(window.__pin);
       window.__pin = setInterval(() => {
         const s = __rec.find("h3", "S-parameters")?.closest("section");
         for (let p = s?.previousElementSibling; p; p = p.previousElementSibling) p.style.display = "none";
       }, 50);`,
    );
    // the time-lapse plays RING_SPEED times faster and fills about LAPSE seconds of the GIF: its
    // take begins once the sweep's last LAPSE × RING_SPEED seconds are left, by the pace of the
    // last few seconds (so a machine busy with something else still gives a take of that length)
    const solved = () => js(`return Number(__rec.find("h3", "S-parameters")?.textContent.match(/(\\d+) wavelength/)?.[1] ?? 0);`);
    const total = Number(/\n  sweep:\n(?:    .*\n)*?    points:\s*(\d+)/.exec(fs.readFileSync(path.join(repo, "jobs", "ring-fdfd.yaml"), "utf8").replace(/\r\n/g, "\n"))[1]);
    const speed = RING_SPEED;
    const seen = [{ t: Date.now(), n: await solved() }];
    for (;;) {
      await sleep(500);
      seen.push({ t: Date.now(), n: await solved() });
      const a = seen.find((s) => s.t >= Date.now() - 8000) ?? seen[0];
      const b = seen.at(-1);
      if (b.t - a.t < 3000 || b.n === a.n) continue;
      const left = ((total - b.n) / (b.n - a.n)) * ((b.t - a.t) / 1000);
      // begun a little early: the GIF keeps the time-lapse's last LAPSE seconds (spec.lapse)
      if (left <= 1.3 * LAPSE * speed) {
        log(`  ${b.n} of ${total} wavelengths, about ${left.toFixed(0)} s to go: played ${speed}× faster`);
        break;
      }
    }
    await heroTake("ring");
    // the badge under the run's bar, where it hides none of the sweep's progress
    await js(`__rec.badge(args[0], 102);`, `${speed}× faster`);
    await rec.speed(speed, GIFS.hero.fps);
    await finished();
    await rec.speed(1);
    // the whole spectrum held a moment, the badge gone; the take ends before anything else shows
    await js(`__rec.badge(null);`);
    await sleep(500);
    await rec.end();
    // off the record: the hidden sections back, the 3D view opened and drawn
    await js(`clearInterval(window.__pin);
              for (const s of document.querySelectorAll("main section")) s.style.display = "";
              __rec.find("main button", "3D").click();`);
    await until(`return __rec.find("main canvas");`, "the ring in 3D", 20000);
    await js(`await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));`);
    await sleep(1800);
    const view = await box("main canvas");
    await moveTo(view.x - 120, view.y + 110, 10);
    await cursor(true);
    await heroTake("ring3d");
    await sleep(300);
    // turned slowly, then held: the light circling the ring at resonance is what this shows
    await drag({ x: view.x - 120, y: view.y + 110 }, { x: view.x + 10, y: view.y + 100 }, 1800, (t) => t * t * (3 - 2 * t));
    await moveTo(view.x + 120, view.y + 190, 500);
    await sleep(400);
    await heroEnd();
  },

  /** A directional coupler designed: its gap opened, then its coupled length shortened, the 3D view following. */
  async coupler() {
    await openComponent("Directional coupler");
    await js(`__rec.find("main button", "Design")?.click();`);
    await sleep(400);
    await fieldIntoView("gap", 60);
    await sleep(1500);
    // the pointer on the gap's thumb as the take begins
    await tidy();
    const gap = await designThumb("gap");
    await moveTo(gap.x, gap.y, 10);
    await cursor(true);
    await heroTake("coupler");
    await sleep(250);
    await dragDesign("gap", 0.85, 1300);
    await sleep(250);
    await dragDesign("length", 0.3, 1200);
    await sleep(450);
    await heroEnd();
  },

  /** A Y-branch designed: its arms spread apart, then lengthened, the 3D view following. */
  async ybranch() {
    await openComponent("Y-branch");
    await js(`__rec.find("main button", "Design")?.click();`);
    await sleep(400);
    await fieldIntoView("arm_length", 60);
    await sleep(1500);
    // the pointer on the separation's thumb as the take begins
    await tidy();
    const sep = await designThumb("separation");
    await moveTo(sep.x, sep.y, 10);
    await cursor(true);
    await heroTake("ybranch");
    await sleep(250);
    await dragDesign("separation", 0.55, 1300);
    await sleep(250);
    await dragDesign("arm_length", 0.5, 1200);
    await sleep(450);
    await heroEnd();
  },

  /** An MZI's last two wires drawn on the chip, then simulated: its spectrum and its checks. */
  async chip() {
    // the example MZI, its arms not yet joined to the second coupler
    const mzi = fs
      .readFileSync(path.join(repo, "circuits", "mzi.toml"), "utf8")
      .replace(/\r\n/g, "\n")
      .replace('    ["upper.o2", "combine.o2"],\n', "")
      .replace('    ["lower.o2", "combine.o1"],\n', "");
    fs.writeFileSync(path.join(WORKSPACE, "circuits", "mzi.toml"), mzi);
    await goTo("Chip");
    await until(`return __rec.find("nav button", "mzi");`, "the saved MZI");
    await js(`__rec.find("nav button", "mzi").click();`);
    await until(`return document.querySelector('[data-pin="upper.o2"]');`, "the MZI on the chip");
    // more room for the canvas, and the chip a little closer
    await fold(true);
    await sleep(400);
    await js(`__rec.find("button[aria-label='Fit']").click();`);
    await sleep(400);
    const canvas = await box("svg[role=application]");
    await moveTo(canvas.x, canvas.y, 10);
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: canvas.x, y: canvas.y, deltaX: 0, deltaY: -150 });
    await sleep(500);
    await cursor(true);
    const pin = (ref) => js(`const r = document.querySelector('[data-pin="' + args[0] + '"]').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`, ref);
    const start = await pin("upper.o2");
    await moveTo(start.x - 60, start.y + 90, 10);
    await heroTake("chip");
    await rec.speed(1.3);
    await drag(await pin("upper.o2"), await pin("combine.o2"), 650, ease, 300);
    await sleep(120);
    await drag(await pin("lower.o2"), await pin("combine.o1"), 650, ease, 300);
    await sleep(200);
    await click("button", "Simulate", { ms: 550 });
    await until(`return __rec.find("span.badge", "σ");`, "the spectrum", 20000);
    await moveTo(mouse.x - 40, mouse.y + 330, 600);
    await sleep(1200);
    await heroEnd();
    await fold(false);
  },

  /** An MMI's FDTD run, live: the pulse entering, filling the MMI and leaving by its two arms (a time-lapse). */
  async fdtd() {
    await moveTo(VIEW.width / 2, VIEW.height / 2, 10);
    await runExample("mmi-fdtd");
    await until(`return [...document.querySelectorAll("main h3")].find((h) => h.textContent.trim().startsWith("Field"));`, "the FDTD field", 60000);
    await until(`return Number(document.querySelector("main").innerText.match(/(\\d+) \\/ \\d+\\s/)?.[1] ?? 0) >= 2;`, "the first frames", 60000);
    await heroTake("fdtd");
    const speed = FDTD_SPEED;
    await js(`__rec.badge(args[0]);`, `${speed}× faster`);
    await rec.speed(speed, GIFS.hero.fps);
    await sleep(FDTD_SECONDS * 1000);
    await rec.speed(1);
    await rec.end();
    await js(`__rec.badge(null); document.querySelector("button[aria-label='Stop the run']")?.click();`);
    await sleep(800);
  },

  /**
   * A thermo-optic phase shifter switched on, at a pace to follow: its 2D model's cross-section
   * first, cold (the heater and the guide outlined), then played from the turn-on, the heat
   * spreading down to the guide as the curve of the rise the guide feels fills in, the time read
   * beside it.
   */
  async heater() {
    await openComponent("Thermo-optic phase shifter");
    await js(`__rec.find("main button", "Temperature").click();`);
    await until(`return __rec.find("main button", "In time");`, "the temperature view", 60000);
    await js(`__rec.find("main button", "In time").click();`);
    await until(`return __rec.find("main button[aria-label=Play]") && !document.querySelector("[aria-label='Turning the heater on']");`, "the heater turned on", 120000);
    await sleep(800);
    // the first picture: the turn-on, nothing heated yet
    await js(`const r = document.querySelector("main input[aria-label='The time of the picture']");
              Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(r, "0");
              r.dispatchEvent(new Event("input", { bubbles: true }));`);
    await sleep(1200);
    // the view scrolled to the curve's top, so the cross-section under it shows whole above the
    // hero's foot (the sentence above the curve, its numbers, scrolled out)
    await js(
      `const r = document.querySelector("main input[aria-label='The time of the picture']");
       const curve = r.closest("div.space-y-3");
       let box = curve.parentElement;
       while (box && !(box.scrollHeight > box.clientHeight + 4 && /auto|scroll/.test(getComputedStyle(box).overflowY))) box = box.parentElement;
       if (box) box.scrollTo({ top: box.scrollTop + curve.getBoundingClientRect().top - box.getBoundingClientRect().top - 6 });`,
    );
    await sleep(600);
    // the pointer by the time's slider, off the curve (whose read-out would follow it)
    await tidy();
    const play = await box("main button[aria-label=Play]");
    await moveTo(play.x + 110, play.y + 2, 10);
    await cursor(true);
    await heroTake("heater");
    await sleep(700);
    await click("main button[aria-label=Play]", null, { ms: 450 });
    // the pictures play a few a second; a little brisker here, still several seconds of heating
    await rec.speed(HEATER_SPEED);
    // to the time read beside the slider
    const reading = await box("main span[aria-live=polite]", " of ");
    await moveTo(reading.left - 40, reading.y + 1, 900);
    await until(`return __rec.find("main button[aria-label=Play]");`, "the end of the turn-on", 30000);
    await rec.speed(1);
    await sleep(450);
    await heroEnd();
  },

  /** An all-pass ring made from the bare wafer, step by step in 3D. */
  async fab() {
    await goTo("Fabrication");
    await until(`return document.querySelector("main select");`, "the design picker", 30000);
    await js(`const s = document.querySelector("main select");
              s.value = "lib:ring-all-pass";
              s.dispatchEvent(new Event("change", { bubbles: true }));`);
    await until(`return __rec.find("main div", "5 steps, from the bare wafer") && __rec.find("main button", "3D");`, "the ring's flow", 60000);
    await js(`__rec.find("main button", "3D").click();
              const sp = document.querySelector("main select[aria-label=Speed]");
              sp.value = "2";
              sp.dispatchEvent(new Event("change", { bubbles: true }));
              document.querySelector("main button[aria-label='Back to the bare wafer']").click();`);
    await sleep(1500);
    await scrollTo("main div", "5 steps, from the bare wafer", 22, false);
    await sleep(800);
    const play = await box("main button[aria-label=Play]");
    await moveTo(play.x + 120, play.y - 160, 10);
    await cursor(true);
    await heroTake("fab");
    await rec.speed(FAB_SPEED);
    await sleep(200);
    await click("main button[aria-label=Play]", null, { ms: 500 });
    await moveTo(play.x + 420, play.y - 260, 900);
    await until(`return __rec.find("main button[aria-label=Play]") && /7 \\/ 7/.test(document.querySelector("main").innerText);`, "the flow played", 60000);
    await sleep(600);
    await heroEnd();
  },

  /** The ring resonator's lesson: its live chart brought to critical coupling. */
  async lesson() {
    await goTo("Academy");
    await until(`return __rec.find("nav button, aside button", "The ring resonator");`, "the lessons");
    await js(`__rec.find("nav button, aside button", "The ring resonator").click();`);
    await until(`return __rec.find("figure.panel:not([data-diagram])");`, "the ring's chart");
    await sleep(800);
    await js(`const fig = __rec.find("figure.panel:not([data-diagram])");
       const art = fig.closest("article");
       const head = fig.firstElementChild.getBoundingClientRect();
       art.scrollTo({ top: art.scrollTop + head.bottom - art.getBoundingClientRect().top });`);
    await sleep(900);
    await cursor(true);
    await slideTo("radius", RING_RADIUS, 200);
    await slideTo("window", 50, 200);
    await slideTo("coupling", 730, 200);
    await sleep(900);
    const critical = await js(
      `const label = [...document.querySelectorAll("figure.panel p")].find((p) => p.textContent.includes("Critical coupling"));
       const sup = "⁰¹²³⁴⁵⁶⁷⁸⁹";
       const text = label.nextElementSibling.textContent.replace(/\\s/g, "").replace(/[⁰¹²³⁴⁵⁶⁷⁸⁹⁻]/g, (c) => (c === "⁻" ? "-" : String(sup.indexOf(c))));
       const m = /^([\\d.]+)(?:×10(-?\\d+))?/.exec(text);
       return Number(m[1]) * 10 ** Number(m[2] ?? 0);`,
    );
    const at = Math.round((1000 * Math.log(critical / 1e-4)) / Math.log(0.5 / 1e-4));
    // the chart whole: its title, plot, sliders and figures
    await js(`const fig = __rec.find("figure.panel:not([data-diagram])");
       const art = fig.closest("article");
       art.scrollTo({ top: art.scrollTop + fig.getBoundingClientRect().top - art.getBoundingClientRect().top - 8 });`);
    await sleep(700);
    await tidy();
    const thumb = await slider("coupling");
    await moveTo(thumb.left + ((thumb.value - thumb.min) / (thumb.max - thumb.min)) * thumb.width + 40, thumb.y + 70, 10);
    await heroTake("lesson");
    await sleep(500);
    // the input coupling slid down to critical: the dip deepens to zero, the figures following
    await slideTo("coupling", at, 1900, (t) => t * t * (3 - 2 * t));
    await moveTo(mouse.x + 60, mouse.y + 90, 400);
    await sleep(700);
    await heroEnd();
  },

  /** A directional coupler's page: its gap and length moved, the view following; then its cut. */
  async device() {
    await goTo("Components");
    // a component already open has folded the list: its breadcrumb opens it again
    await sleep(600);
    await js(`if (!__rec.find("button", "Directional coupler")) document.querySelector("button[title^='The components']")?.click();`);
    await until(`return __rec.find("button", "Directional coupler");`, "the components");
    // opening it folds the list away, so the design's view and its panel get the width
    await js(`__rec.find("button", "Directional coupler").click();`);
    await until(`return document.getElementById("field-gap");`, "the coupler's page");
    await fold(true);
    await sleep(1500);
    const slider = (name) =>
      js(`const r = document.getElementById("field-" + args[0]).closest("div").parentElement.querySelector("input[type=range]").getBoundingClientRect();
          return { left: r.left, width: r.width, y: r.top + r.height / 2 };`, name);
    const gap = await slider("gap");
    await moveTo(gap.left + gap.width * 0.4, gap.y - 80, 10);
    await cursor(true);
    await rec.begin("device");
    rec.crop(await railless());
    await sleep(500);
    // the gap opened, then the coupled length shortened: the 3D view redraws at every step
    const at = (s, v) => ({ x: s.left + s.width * v, y: s.y });
    const now = await js(`return Number(document.getElementById("field-gap").closest("div").parentElement.querySelector("input[type=range]").value);`);
    const g0 = (now - 0.1) / (0.5 - 0.1);
    await drag(at(gap, Math.min(0.95, Math.max(0.05, g0))), at(gap, 0.9), 1600, ease, 600);
    await sleep(400);
    const length = await slider("length");
    await drag(at(length, 0.6), at(length, 0.25), 1400, ease, 600);
    await sleep(500);
    await click("button", "Cross-section", { ms: 700 });
    await sleep(1600);
    await rec.end();
    await cursor(false);
    await fold(false);
  },

  /** Lithium niobate's two indices read off the plot; AlGaAs's as its aluminium fraction moves. */
  async materials() {
    await goTo("Materials");
    await until(`return __rec.find("aside button", "LiNbO₃");`, "the materials list");
    await moveTo(600, 400, 10);
    await cursor(true);
    await rec.begin("materials");
    rec.crop(await railless());
    // a touch brisker than life, to keep the clip short
    await rec.speed(1.15);
    await sleep(100);
    await click("aside button", "LiNbO₃", { ms: 700 });
    await sleep(200);
    await showIndexPlot();
    await sleep(900);
    // the pointer along the curves: the read-out follows it
    let plot = await box("section.panel svg[role=img]");
    await moveTo(plot.left + plot.width * 0.12, plot.top + plot.height * 0.45, 400);
    await moveTo(plot.left + plot.width * 0.8, plot.top + plot.height * 0.55, 1700, (t) => t);
    await sleep(250);
    await click("aside button", "AlₓGa₁₋ₓAs", { ms: 800 });
    await js(`__rec.find("h3", "Refractive index").closest(".overflow-y-auto").scrollTo({ top: 0 });`);
    await sleep(200);
    await showIndexPlot();
    await sleep(300);
    const slider = await box("section.panel input[type=range]");
    await drag({ x: slider.left + slider.width * 0.03, y: slider.y }, { x: slider.left + slider.width * 0.85, y: slider.y }, 2000, (t) => t * t * (3 - 2 * t));
    await sleep(1000);
    await rec.end();
    await cursor(false);
  },

  /** The validation report: cases found and opened, their math rendered. */
  async validation() {
    await goTo("Validation");
    await until(`return __rec.find("h2", "cases pass");`, "the report");
    await sleep(500);
    await moveTo(700, 300, 10);
    await cursor(true);
    await rec.begin("validation");
    rec.crop(await railless());
    await sleep(400);
    // a case looked up and opened: what it computes and what it is checked against
    const open = async (query, id) => {
      await click("input[placeholder='Search the cases']", null, { ms: 600 });
      await cdp.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "a", code: "KeyA", windowsVirtualKeyCode: 65, modifiers: 2, commands: ["selectAll"] });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: "a", code: "KeyA", windowsVirtualKeyCode: 65, modifiers: 2 });
      for (const ch of query) {
        await cdp.send("Input.insertText", { text: ch });
        await sleep(55);
      }
      await sleep(400);
      await click("div.panel > div > button", id, { ms: 600, dx: -150 });
    };
    await open("hadley-interface", "mode/hadley-interface");
    await sleep(2300);
    await open("mzi-closed", "circuit/mzi-closed-form");
    await sleep(2300);
    await rec.end();
    await cursor(false);
  },

  /**
   * The ring resonator's lesson opened on its diagram, the ring drawn from above and then turned
   * in 3D, and its live chart: the radius set so a resonance sits at 1.55 µm, the window narrowed
   * onto it, then the coupling raised and lowered to critical coupling, where the dip reaches zero.
   */
  async academy() {
    await goTo("Academy");
    await until(`return __rec.find("aside button", "Bragg gratings and mirrors");`, "the lessons");
    // the other lesson open first, so the take opens the ring's
    await js(`__rec.find("aside button", "Bragg gratings and mirrors").click();`);
    await until(`return __rec.find("article h2", "Bragg gratings and mirrors");`, "the Bragg lesson");
    await sleep(600);
    await moveTo(760, 330, 10);
    await cursor(true);
    await rec.begin("academy");
    rec.crop(await railless());
    await sleep(500);
    await click("aside button", "The ring resonator", { ms: 700 });
    await until(`return __rec.find("figure[data-diagram] svg[role=img]");`, "the ring's diagram");
    await sleep(700);
    // first the device: its diagram whole, the schematic, then the same ring in 3D, turned a little
    await js(`const fig = __rec.find("figure[data-diagram]");
       const art = fig.closest("article");
       art.scrollTo({ top: art.scrollTop + fig.getBoundingClientRect().top - art.getBoundingClientRect().top - 12, behavior: "smooth" });`);
    await sleep(2800);
    await click("figure[data-diagram] button", "3D", { ms: 700 });
    await until(`return __rec.find("figure[data-diagram] canvas");`, "the ring in 3D");
    await sleep(1200);
    const view = await box("figure[data-diagram] canvas");
    await drag({ x: view.x - 90, y: view.y - 30 }, { x: view.x + 70, y: view.y + 20 }, 1600, (t) => t * t * (3 - 2 * t), 600);
    await sleep(1400);
    // then down to the chart: its plot at the top, then its sliders and the numbers it computes
    await js(`const fig = __rec.find("figure.panel:not([data-diagram])");
       const art = fig.closest("article");
       const head = fig.firstElementChild.getBoundingClientRect();
       art.scrollTo({ top: art.scrollTop + head.bottom - art.getBoundingClientRect().top, behavior: "smooth" });`);
    await sleep(1100);
    await slideTo("radius", RING_RADIUS, 1100);
    await sleep(500);
    await slideTo("window", 50, 1100);
    await sleep(700);
    // strongly over-coupled: a wide, shallow dip; then down to critical, where it reaches zero
    await slideTo("coupling", 730, 1000);
    await sleep(700);
    // the chart's figure for it, e.g. "2.721×10⁻³"
    const critical = await js(
      `const label = [...document.querySelectorAll("figure.panel p")].find((p) => p.textContent.includes("Critical coupling"));
       const sup = "⁰¹²³⁴⁵⁶⁷⁸⁹";
       const text = label.nextElementSibling.textContent.replace(/\\s/g, "").replace(/[⁰¹²³⁴⁵⁶⁷⁸⁹⁻]/g, (c) => (c === "⁻" ? "-" : String(sup.indexOf(c))));
       const m = /^([\\d.]+)(?:×10(-?\\d+))?/.exec(text);
       return Number(m[1]) * 10 ** Number(m[2] ?? 0);`,
    );
    const at = Math.round((1000 * Math.log(critical / 1e-4)) / Math.log(0.5 / 1e-4));
    log(`  critical coupling ${critical}: the slider at ${at}`);
    await slideTo("coupling", at, 2200, (t) => t * t * (3 - 2 * t));
    await moveTo(mouse.x + 60, mouse.y + 110, 600);
    await sleep(2000);
    await rec.end();
    await cursor(false);
  },
};

/**
 * The ring lesson's radius slider's place (of 1000, from 2 to 200 µm on a log scale) at which a
 * resonance sits within a few pm of 1.55 µm: 6.27 µm, n_eff 2.4, order 61.
 */
const RING_RADIUS = 248;

/** A chart slider's thumb, and where a place on it (of its own range) is. */
const slider = (key) =>
  js(
    `const e = __rec.find("figure.panel input[aria-label='" + args[0] + "']");
     const r = e.getBoundingClientRect();
     const [min, max, value] = [Number(e.min), Number(e.max), Number(e.value)];
     return { min, max, value, left: r.left + r.height / 2, width: r.width - r.height, y: r.top + r.height / 2 };`,
    key,
  );

/** Drags a chart slider's thumb to a place, then steps it there with the arrow keys, exactly. */
async function slideTo(key, to, ms = 1000, curve = ease) {
  let s = await slider(key);
  const x = (v) => s.left + ((v - s.min) / (s.max - s.min)) * s.width;
  await drag({ x: x(s.value), y: s.y }, { x: x(to), y: s.y }, ms, curve, 600);
  for (let i = 0; ; i++) {
    s = await slider(key);
    if (s.value === to) return;
    if (i === 40) throw new Error(`the ${key} slider is at ${s.value}, not ${to}`);
    await pressKey(s.value < to ? "ArrowRight" : "ArrowLeft");
    await sleep(60);
  }
}

/** The ring's sweep in the hero: played this many times faster, for about this many seconds of the GIF. */
const RING_SPEED = 15;
const LAPSE = GIFS.hero.lapse.ring;

/** The MMI's FDTD run in the hero: played this many times faster, for this many seconds of the run. */
const FDTD_SPEED = 6;
const FDTD_SECONDS = 26;

/** The heater's pictures (a few a second in the studio), played this many times faster in the hero. */
const HEATER_SPEED = 1.9;

/** The fabrication flow (at the player's own speed of 2), played this many times faster in the hero. */
const FAB_SPEED = 1.75;

// ---- composing the GIFs ----

/**
 * The take's frames resampled to `fps`, following its speed marks: the files to show, in order.
 * A time-lapse longer than `lapse` seconds (of the GIF) keeps only its last `lapse` seconds, at
 * its speed: its start is left out.
 */
function resampleTake(take, fps, lapse = Infinity) {
  const marks = [...take.marks, { t: take.end, speed: 1 }].map((m) => ({ ...m }));
  for (let i = 0; i + 1 < marks.length; i++) {
    const a = marks[i];
    const b = marks[i + 1];
    if (a.speed > 3 && Number.isFinite(a.speed) && (b.t - a.t) / a.speed > lapse) {
      // what comes before it, and its start, cut
      for (let k = 0; k < i; k++) marks[k].speed = Infinity;
      a.t = b.t - lapse * a.speed;
      break;
    }
  }
  const frames = take.frames;
  const pick = (t) => {
    let lo = 0;
    let hi = frames.length - 1;
    if (t <= frames[0].t) return frames[0].file;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (frames[mid].t <= t) lo = mid;
      else hi = mid - 1;
    }
    return frames[lo].file;
  };
  const out = [];
  let virtual = 0;
  let next = 0;
  for (let i = 0; i + 1 < marks.length; i++) {
    const a = marks[i];
    const b = marks[i + 1];
    if (!Number.isFinite(a.speed) || b.t <= a.t) continue;
    const span = (b.t - a.t) / a.speed;
    while (next / fps < virtual + span) {
      out.push(pick(a.t + (next / fps - virtual) * a.speed));
      next++;
    }
    virtual += span;
  }
  return out;
}

/** Decodes the files at `width` (Lanczos), as raw RGB frames. */
function decode(files, width, dir, crop) {
  const list = path.join(dir, "list.ffconcat");
  const esc = (f) => f.replace(/\\/g, "/").replace(/'/g, "'\\''");
  fs.writeFileSync(list, "ffconcat version 1.0\n" + files.map((f) => `file '${esc(f)}'\nduration 0.04\n`).join(""));
  const c = crop ?? { x: 0, y: 0, width: VIEW.width, height: VIEW.height };
  const height = Math.round((c.height * width) / c.width / 2) * 2;
  const filter = `crop=${c.width}:${c.height}:${c.x}:${c.y},scale=${width}:${height}:flags=lanczos`;
  const r = spawnSync(
    "ffmpeg",
    ["-v", "error", "-f", "concat", "-safe", "0", "-i", list, "-vf", filter, "-fps_mode", "passthrough", "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"],
    { maxBuffer: 2 ** 31 },
  );
  if (r.status !== 0) throw new Error(`ffmpeg: ${r.stderr}`);
  const size = width * height * 3;
  const frames = [];
  for (let o = 0; o + size <= r.stdout.length; o += size) frames.push(r.stdout.subarray(o, o + size));
  if (frames.length !== files.length) log(`  (decoded ${frames.length} frames of ${files.length})`);
  while (frames.length < files.length) frames.push(frames[frames.length - 1]);
  return { frames: frames.slice(0, files.length), width, height };
}

function blend(a, b, alpha) {
  const out = Buffer.allocUnsafe(a.length);
  const w = Math.round(alpha * 256);
  for (let i = 0; i < a.length; i++) out[i] = (a[i] * (256 - w) + b[i] * w) >> 8;
  return out;
}
const smooth = (t) => t * t * (3 - 2 * t);

/** Joins the takes with crossfades, and crossfades the end back into the start. */
function compose(clips, fade) {
  let frames = [...clips[0]];
  for (const next of clips.slice(1)) {
    const n = Math.min(fade, frames.length, next.length);
    const head = frames.slice(0, frames.length - n);
    const mixed = Array.from({ length: n }, (_, i) => blend(frames[frames.length - n + i], next[i], smooth((i + 1) / (n + 1))));
    frames = [...head, ...mixed, ...next.slice(n)];
  }
  const n = Math.min(fade, Math.floor(frames.length / 3));
  const loop = Array.from({ length: n }, (_, i) => blend(frames[frames.length - n + i], frames[i], smooth((i + 1) / (n + 1))));
  return [...frames.slice(n, frames.length - n), ...loop];
}

function encode(name, spec, frames, width, height) {
  const dir = path.join(TEMP, "out", name);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  const png = spawnSync(
    "ffmpeg",
    ["-v", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", `${width}x${height}`, "-r", String(spec.fps), "-i", "pipe:0", path.join(dir, "%05d.png")],
    { input: Buffer.concat(frames), maxBuffer: 1 << 30 },
  );
  if (png.status !== 0) throw new Error(`ffmpeg: ${png.stderr}`);
  const files = fs.readdirSync(dir).filter((f) => f.endsWith(".png")).sort();
  const gif = path.join(OUT, `${name}.gif`);
  const r = spawnSync("gifski", ["--fps", String(spec.fps), "--quality", String(spec.quality[0]), "--motion-quality", String(spec.quality[1]), "--lossy-quality", String(spec.quality[2]), "--width", String(width), "--output", gif, ...files], { cwd: dir, encoding: "utf8" });
  if (r.status !== 0) throw new Error(`gifski: ${r.stderr}`);
  const mb = fs.statSync(gif).size / 1e6;
  log(`${path.relative(repo, gif)}: ${(frames.length / spec.fps).toFixed(1)} s, ${frames.length} frames, ${width} x ${height}, ${mb.toFixed(2)} MB`);
}

// ---- the window ----

/**
 * Sizes the studio's window so its page is VIEW in CSS pixels at the window's display scale, and
 * puts it in the bottom-right corner of the primary display's work area, behind the other windows
 * and without taking the focus (it keeps rendering there: occlusion is off). A window that can't
 * hold VIEW at that scale is made as large as the work area allows; the page is still laid out at
 * VIEW by the emulation, so the recording is the same and only the window on the screen shows less.
 */
function placeWindow(pid) {
  const script = String.raw`
$ErrorActionPreference = "Stop"
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class HeroWin {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Ri, B; }
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SystemParametersInfo(uint a, uint b, out R r, uint c);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
}
"@
[void][HeroWin]::SetThreadDpiAwarenessContext([IntPtr](-4))
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 50 -and $h -eq [IntPtr]::Zero; $i++) { $h = (Get-Process -Id ${pid}).MainWindowHandle; if ($h -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200 } }
if ($h -eq [IntPtr]::Zero) { "no window"; exit 1 }
$w = New-Object HeroWin+R; $c = New-Object HeroWin+R; $a = New-Object HeroWin+R
[void][HeroWin]::GetWindowRect($h, [ref]$w); [void][HeroWin]::GetClientRect($h, [ref]$c)
[void][HeroWin]::SystemParametersInfo(0x30, 0, [ref]$a, 0)
$scale = [HeroWin]::GetDpiForWindow($h) / 96.0
$fw = ($w.Ri - $w.L) - ($c.Ri - $c.L); $fh = ($w.B - $w.T) - ($c.B - $c.T)
$ow = [Math]::Min([int][Math]::Round(${VIEW.width} * $scale) + $fw, $a.Ri - $a.L)
$oh = [Math]::Min([int][Math]::Round(${VIEW.height} * $scale) + $fh, $a.B - $a.T)
# HWND_BOTTOM, SWP_NOACTIVATE
[void][HeroWin]::SetWindowPos($h, [IntPtr]1, $a.Ri - $ow, $a.B - $oh, $ow, $oh, 0x10)
[void][HeroWin]::GetClientRect($h, [ref]$c)
"scale {0}, page {1} x {2} CSS px in a work area of {3} x {4} px" -f $scale, [Math]::Round(($c.Ri - $c.L) / $scale), [Math]::Round(($c.B - $c.T) / $scale), ($a.Ri - $a.L), ($a.B - $a.T)
`;
  const r = spawnSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], { encoding: "utf8" });
  const said = `${r.stdout ?? ""}${r.stderr ?? ""}`.trim();
  if (r.status !== 0) {
    log(`  couldn't place the window: ${said}`);
    return;
  }
  log(`  the window: ${said}${/page (\d+) x (\d+)/.exec(said)?.slice(1).join("x") === `${VIEW.width}x${VIEW.height}` ? "" : ` (the recording is laid out at ${VIEW.width} x ${VIEW.height} all the same)`}`);
}

// ---- the session ----

async function record() {
  fs.mkdirSync(CONFIG, { recursive: true });
  fs.writeFileSync(path.join(CONFIG, "settings.json"), JSON.stringify(RECORDING_SETTINGS, null, 2));
  // no window size or place kept from an earlier session (put back at the end with the settings)
  fs.rmSync(path.join(CONFIG, ".window-state.json"), { force: true });
  log(`starting ${path.relative(repo, EXE)} in ${WORKSPACE}`);
  child = spawn(EXE, [], {
    cwd: WORKSPACE,
    stdio: "ignore",
    env: {
      ...process.env,
      WEBVIEW2_USER_DATA_FOLDER: PROFILE,
      // keep rendering when other windows cover it
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
  placeWindow(child.pid);
  cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  // the page laid out at VIEW whatever the window's size, so every recording is the same
  await cdp.send("Emulation.setDeviceMetricsOverride", { ...VIEW, deviceScaleFactor: 1, mobile: false });
  await until(`return document.querySelector("nav button");`, "the window");
  await js(PAGE_HELPERS);
  await cursor(false);
  const workspace = await js(`return document.querySelector("footer span.truncate")?.textContent;`);
  if (workspace !== WORKSPACE) throw new Error(`the workspace is ${workspace}, not ${WORKSPACE}`);
  await measureHeroCrop();
  rec = new Recorder();

  const order = ["fdtd", "ring", "ring3d", "coupler", "ybranch", "chip", "heater", "fab", "lesson", "device", "materials", "validation", "academy"];
  for (const name of order) {
    // a take another scene records (ring3d, the ring's) has no scene of its own
    if (wanted.has(name) && SCENES[name]) await SCENES[name]();
  }
  closeProgram();
  restoreSettings();
  return loadTakes();
}

function loadTakes() {
  const takes = {};
  for (const name of wanted) {
    const file = path.join(FRAMES, name, "take.json");
    if (fs.existsSync(file)) takes[name] = { ...JSON.parse(fs.readFileSync(file, "utf8")), dir: path.join(FRAMES, name) };
  }
  return takes;
}

const takes = REUSE ? loadTakes() : await record();
for (const name of wanted) if (!takes[name]) fail(`no take called ${name} in ${FRAMES}`);
fs.mkdirSync(OUT, { recursive: true });
for (const name of only) {
  const spec = GIFS[name];
  log(`composing ${name}`);
  let size = null;
  const clips = spec.takes.map((t) => {
    const files = resampleTake(takes[t], spec.fps, spec.lapse?.[t]);
    const d = decode(files, spec.width, path.join(FRAMES, t), takes[t].crop);
    size = d;
    return d.frames;
  });
  // each scene's seconds in the GIF: its takes less the crossfades, half of each at either end
  // (so the scenes add up to the whole), and the time it is on screen alone, the fades left out
  if (spec.scenes) {
    const frames = Object.fromEntries(spec.takes.map((t, k) => [t, clips[k].length]));
    for (const scene of spec.scenes) {
      const all = scene.reduce((sum, t) => sum + frames[t], 0);
      const shared = (all - spec.fade * scene.length) / spec.fps;
      const alone = (all - spec.fade * (scene.length + 1)) / spec.fps;
      log(`  ${scene.join(" + ")}: ${shared.toFixed(2)} s, ${alone.toFixed(2)} s without the crossfades`);
    }
  }
  encode(name, spec, compose(clips, spec.fade), size.width, size.height);
}
log(REUSE ? "done" : `done; the settings in ${CONFIG} are as they were`);
