// The studio window: follows the run's record (through the `poll` command) and shows it in a 3D
// view (the default) or a 2D one, with the run, its layers, modes and sweep in the sidebar. With
// no run open it shows the start page, where jobs are run and runs reopened.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import { mediumLook } from "./colours";
import { modeKind, type Event, type Mode, type Permittivity, type Scene, type SweepPoint } from "./events";
import { renderHome, type Home } from "./home";
import { render2d } from "./view2d";
import { View3D } from "./view3d";
import "./style.css";

interface Info {
  generation: number;
  /** The run's directory and name; none for the start page. */
  dir: string | null;
  name: string | null;
  /** A run from the command line: the window closes after it. */
  closes: boolean;
  root: string;
}

const state = {
  info: null as Info | null,
  page: "run" as "run" | "home",
  job: null as { job: string; kind: string } | null,
  scene: null as Scene | null,
  pictures: [] as Permittivity[],
  modes: [] as Mode[],
  sweep: null as { parameter: string; points: SweepPoint[] } | null,
  finished: null as { stopped: string | null; seconds: number } | null,
  problem: null as string | null,
  count: 0,
  view: "3d" as "3d" | "2d",
  hidden: new Set<string>(),
  selected: 0,
  opened: performance.now(),
};

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

const view3d = new View3D($("view3d"), document.getElementById("gizmo") as unknown as SVGSVGElement);
const dirty = { scene: false, field: false, side: true, twoD: true };

function take(e: Event) {
  switch (e.type) {
    case "started":
      state.job = { job: e.job, kind: e.kind };
      break;
    case "scene":
      state.scene = e;
      dirty.scene = true;
      break;
    case "permittivity":
      state.pictures.push(e);
      break;
    case "mode":
      state.modes.push(e);
      if (state.modes.length === state.selected + 1) dirty.field = true;
      break;
    case "sweep_point":
      state.sweep ??= { parameter: e.parameter, points: [] };
      state.sweep.points.push(e);
      break;
    case "finished":
      state.finished = { stopped: e.stopped, seconds: e.seconds };
      break;
  }
  dirty.side = true;
  dirty.twoD = true;
}

/** Starts over on the run `info` describes (or none). */
function reset(info: Info) {
  Object.assign(state, {
    info,
    job: null,
    scene: null,
    pictures: [],
    modes: [],
    sweep: null,
    finished: null,
    problem: null,
    count: 0,
    hidden: new Set<string>(),
    selected: 0,
    opened: performance.now(),
  });
  view3d.clear();
  $("view2d").replaceChildren();
  Object.assign(dirty, { scene: false, field: false, side: true, twoD: true });
  setPage(info.dir ? "run" : "home");
}

function topBar() {
  const hasRun = !!state.info?.dir;
  $("job").textContent = !hasRun ? "" : state.job ? `${state.job.job} (${state.job.kind})` : "waiting for the run…";
  const f = state.finished;
  $("status").textContent = !hasRun
    ? ""
    : !f
    ? `running · ${((performance.now() - state.opened) / 1000).toFixed(0)} s`
    : f.stopped
      ? `stopped (${f.stopped}) after ${f.seconds.toFixed(2)} s`
      : `finished in ${f.seconds.toFixed(2)} s`;
  $("closes").hidden = !(state.info?.closes && f);
  $("switch").hidden = state.page === "home";
  $("run-bar").hidden = !hasRun;
  $("hint").textContent =
    state.page === "home"
      ? "run a job, or open a run"
      : state.view === "3d"
      ? "drag: rotate · right-drag: pan · scroll: zoom · double-click: reset"
      : "pictures, modes and sweeps as the run records them";
  $("problem").textContent = state.problem ?? "";
  $("dir").textContent = state.info?.name ?? "";
  $("dir").title = state.info?.dir ?? "";
}

function sideBar() {
  const s = state.scene;
  const rows: string[] = [];
  if (state.job) rows.push(`<dt>job</dt><dd>${esc(state.job.job)}</dd><dt>kind</dt><dd>${esc(state.job.kind)}</dd>`);
  if (s) {
    rows.push(`<dt>wavelength</dt><dd>${s.wavelength_um} µm</dd>`);
    for (const [axis, w] of [
      ["x", s.x_um],
      ["y", s.y_um],
      ["z", s.z_um],
    ] as const) {
      rows.push(`<dt>${axis}</dt><dd>${w[0].toFixed(3)} to ${w[1].toFixed(3)} µm</dd>`);
    }
  }
  let html = `<details open><summary>Run</summary>${
    state.info?.dir ? `<dl>${rows.join("")}</dl>` : `<p class="weak small">No run open: start one, or open one.</p>`
  }</details>`;
  if (s) {
    // top to bottom, as they stand
    const media = [
      ["cladding", s.cladding] as const,
      ...[...s.layers].reverse().map((l) => [l.name, l.material] as const),
      ["substrate", s.substrate] as const,
    ];
    const items = media.map(([name, m]) => {
      const look = mediumLook(m.eps);
      const swatch = look
        ? `<span class="swatch" style="background:${look.colour}${look.solid ? "" : ";opacity:.6"}"></span>`
        : `<span class="swatch hollow"></span>`;
      return `<label class="layer"><input type="checkbox" data-layer="${esc(name)}" ${state.hidden.has(name) ? "" : "checked"}/>
        ${swatch}<span>${esc(name)}</span><span class="weak">${esc(m.material)}, ε ${m.eps.toFixed(3)}</span></label>`;
    });
    const n = s.shapes.length;
    html += `<details open><summary>Layers</summary>${items.join("")}
      <p class="weak small">${n} shape${n === 1 ? "" : "s"} · silicon blue, nitride teal, oxides clear</p></details>`;
  }
  if (state.modes.length) {
    const items = state.modes.map(
      (m, k) =>
        `<button class="mode${k === state.selected ? " on" : ""}" data-mode="${k}">
          <span>${esc(m.label)}</span><span class="weak">${modeKind(m)} · n_eff ${m.effective_index[0].toFixed(5)}</span></button>`,
    );
    html += `<details open><summary>Modes</summary>${items.join("")}
      <p class="weak small">the selected mode's |E|² is painted on the cut</p></details>`;
  }
  if (state.sweep) {
    const { parameter, points } = state.sweep;
    const first = points[0]?.value;
    const last = points[points.length - 1]?.value;
    html += `<details open><summary>Sweep</summary><p>over the ${esc(parameter)}: ${points.length} points</p>
      <p class="weak small">from ${first} to ${last}</p><a href="#" data-goto="2d">plots in the 2D view</a></details>`;
  }
  // keep the sections the user folded
  const closed = new Set(
    [...document.querySelectorAll("#sidebar details:not([open]) > summary")].map((e) => e.textContent),
  );
  $("sidebar").innerHTML = html;
  for (const d of document.querySelectorAll<HTMLDetailsElement>("#sidebar details")) {
    if (closed.has(d.querySelector("summary")!.textContent)) d.open = false;
  }
}

function overlay() {
  const m = state.modes[state.selected];
  $("overlay").innerHTML = m
    ? `<strong>${esc(m.label)}</strong> · ${modeKind(m)} · n_eff = ${m.effective_index[0].toFixed(6)} at ${m.wavelength_um} µm<br/>
       <span class="weak">|E|² on the cut at y = ${m.cut_y_um.toFixed(3)} µm, from zero (black) to its peak (pale yellow)</span>`
    : "";
  const empty = $("empty3d");
  empty.hidden = !!state.scene;
  if (!state.scene) {
    empty.innerHTML = state.finished
      ? `this run has no 3D scene (an older photonoxide recorded it): <a href="#" data-goto="2d">show the 2D view</a>`
      : "waiting for the structure…";
  }
}

function setView(v: "3d" | "2d") {
  state.view = v;
  for (const b of document.querySelectorAll<HTMLButtonElement>(".switch button")) {
    b.classList.toggle("on", b.dataset.view === v);
  }
  setPage("run");
}

function setPage(p: "run" | "home") {
  state.page = p;
  $("home").hidden = p !== "home";
  $("view3d").hidden = p !== "run" || state.view !== "3d";
  $("view2d").hidden = p !== "run" || state.view !== "2d";
  if (p === "run" && state.view === "3d") view3d.resize();
  if (p === "home") refreshHome();
  update();
}

async function refreshHome() {
  try {
    const h = await invoke<Home>("home");
    if (state.page === "home") renderHome($("home"), h, state.info?.dir ? state.info.name : null);
  } catch (e) {
    state.problem = String(e);
  }
}

/** Calls `command` (open_run or run_job), and shows the run it opens. */
async function start(command: string, args: Record<string, string>) {
  try {
    const info = await invoke<Info>(command, args);
    state.view = "3d";
    for (const b of document.querySelectorAll<HTMLButtonElement>(".switch button")) {
      b.classList.toggle("on", b.dataset.view === "3d");
    }
    reset(info);
  } catch (e) {
    state.problem = String(e);
    topBar();
  }
}

let drawing2d = false;
function update() {
  if (dirty.scene && state.scene) {
    view3d.setScene(state.scene, state.hidden);
    dirty.scene = false;
  }
  if (dirty.field) {
    view3d.setField(state.modes[state.selected] ?? null);
    dirty.field = false;
  }
  if (dirty.side) {
    sideBar();
    dirty.side = false;
  }
  if (dirty.twoD && state.view === "2d" && !drawing2d) {
    dirty.twoD = false;
    drawing2d = true;
    render2d($("view2d"), state, (wavelengths_um, n) => invoke<number[]>("group_index", { wavelengthsUm: wavelengths_um, n }))
      .catch((e) => (state.problem = String(e)))
      .finally(() => (drawing2d = false));
  }
  topBar();
  overlay();
}

async function poll() {
  try {
    const p = await invoke<{ generation: number; events: Event[]; problem: string | null }>("poll", {
      from: state.count,
    });
    if (p.generation !== state.info?.generation) {
      // the window moved to another run: start over on it
      reset(await invoke<Info>("info"));
    } else {
      for (const e of p.events) take(e);
      state.count += p.events.length;
      state.problem = p.problem ?? state.problem;
      update();
    }
  } catch (e) {
    state.problem = `can't read the record: ${e}`;
    topBar();
  }
  setTimeout(poll, state.finished || !state.info?.dir ? 1000 : 150);
}

document.addEventListener("click", (e) => {
  const t = e.target as HTMLElement;
  const view = t.closest<HTMLElement>("[data-view]")?.dataset.view;
  if (view === "3d" || view === "2d") setView(view);
  const goto = t.closest<HTMLElement>("[data-goto]")?.dataset.goto;
  if (goto === "2d" || goto === "run") {
    e.preventDefault();
    if (goto === "2d") setView("2d");
    else setPage("run");
  }
  if (t.closest("#home-button")) setPage("home");
  const run = t.closest<HTMLElement>("[data-run]")?.dataset.run;
  if (run !== undefined) start("run_job", { path: run });
  const dir = t.closest<HTMLElement>("[data-open]")?.dataset.open;
  if (dir !== undefined) start("open_run", { dir });
  const pick = t.closest<HTMLElement>("[data-pick]")?.dataset.pick;
  if (pick === "job" || pick === "run") {
    const root = state.info?.root ?? "";
    const chosen =
      pick === "job"
        ? open({ title: "Run a job file", filters: [{ name: "photonoxide job", extensions: ["toml"] }], defaultPath: `${root}/jobs` })
        : open({ title: "Open a run folder", directory: true, defaultPath: `${root}/runs` });
    chosen.then((path) => {
      if (typeof path === "string") start(pick === "job" ? "run_job" : "open_run", pick === "job" ? { path } : { dir: path });
    });
  }
  const mode = t.closest<HTMLElement>("[data-mode]")?.dataset.mode;
  if (mode !== undefined) {
    state.selected = Number(mode);
    dirty.field = true;
    dirty.side = true;
    update();
  }
});

document.addEventListener("change", (e) => {
  const t = e.target as HTMLInputElement;
  const layer = t.dataset.layer;
  if (layer === undefined) return;
  if (t.checked) state.hidden.delete(layer);
  else state.hidden.add(layer);
  dirty.scene = true;
  update();
});

invoke<Info>("info")
  .then((info) => reset(info))
  .catch(() => {})
  .finally(() => poll());
