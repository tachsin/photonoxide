<script lang="ts">
  // The Custom device: any structure no device of the library describes, as a job. Its view
  // always on screen (the structure in 3D as its run will draw it, from above with its shapes,
  // ports, sources and monitors, or the job's text in TOML, JSON or YAML), and the form beside it:
  // the job's kind, its stack, its named parameters, its shapes, its ports, sources and monitors,
  // its window, grid and sweep. The program reads a job into the form and writes it back
  // (studio/src-tauri/src/form.rs): what the form doesn't show (components, paths, shapes defined
  // by functions, bodies, a stack of layers, a port found by name, a solver table) is kept as the
  // file has it, and said so; the task's parameters and a sweep of any of them are the form's.
  // Checked by the library as it changes, saved to the workspace's jobs/, run from here. A new
  // job is YAML. A click on a structure in the design (a shape, a layer, a port), beside the text
  // or in the 3D and Top views beside the form, opens its entry in a sidebar (and selects it in
  // the text), whose values, media and crystal orientation are written back into the text
  // (drawn.rs finds which entry draws which shape, media.rs the stack's media; lib/jobedit.ts
  // the text's spans). Outside "As text", the form's changes and the sidebar's are one history.
  import { Box, CircleAlert, CircleCheck, Code, Eye, LoaderCircle, Play, Plus, Save, Shapes, Trash2 } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { onDestroy, untrack } from "svelte";

  import { ago, api, duration, formatOf, KINDS, type JobCheck, type JobFormat, type RunItem } from "../../lib/api";
  import { customCompare } from "../../lib/device.svelte";
  import type { Event, Shape } from "../../lib/events";
  import { overlaidOf } from "../../lib/study";
  import Overlay, { type OverlayItem } from "../Overlay.svelte";
  import { app, startRun, toast } from "../../lib/app.svelte";
  import { modelOf } from "../../lib/catalog.svelte";
  import { crystalFrame, crystalNote, frameCaption, type Crystal, type Frame, type Section } from "../../lib/frames";
  import { along, cells, holds, layerSpan, numberOf, previewWindow, STACKS, template, turn, type JobModel, type Kind, type Num } from "../../lib/job";
  import { lenUnit } from "../../lib/units";
  import FdtdItems from "../FdtdItems.svelte";
  import GeometryPreview, { type Selection } from "../GeometryPreview.svelte";
  import NumField from "../NumField.svelte";
  import OptField from "../OptField.svelte";
  import ScenePreview from "../ScenePreview.svelte";
  import TomlEditor from "../TomlEditor.svelte";
  import EntryPanel from "./EntryPanel.svelte";
  import { drawerOf, entryAt, entrySpan, itemPaths, layerPath, samePath, type Drawing, type Medium, type Span } from "../../lib/jobedit";
  import type { Path } from "../../lib/jobschema";

  let model = $state<JobModel>(template("modes"));
  let text = $state("");
  /** The format `text` is written in, and its file saved in: the form's TOML, converted by the
   * library for the others, so every float comes back to the last bit. */
  let format = $state<JobFormat>("yaml");
  let path = $state<string | null>(null);
  let saved = $state("");
  let check = $state<JobCheck | null>(null);
  let checking = $state(false);
  let selection = $state<Selection>(null);
  let right = $state<"preview" | "3d" | "toml">("3d");
  /** Beside the text, the design: in 3D or from above. */
  let design = $state<"3d" | "preview">("3d");
  let designHeight = $state(0);
  /** The text the 3D preview shows: the last one that checked out, so it doesn't rebuild on every key. */
  let shown = $state("");
  /** The frame the views are in, and the shown job's cross-section that the cross-section's frame is (frames.rs): none for a job that isn't cut. */
  let frame = $state<Frame>("chip");
  let section = $state<Section | null>(null);
  /** The anisotropic materials the shown job uses and how it orients them (frames.rs): the crystal frame is the first one oriented. */
  let crystals = $state<Crystal[]>([]);
  const oriented = $derived(crystalFrame(crystals));
  $effect(() => {
    const t = shown;
    let gone = false;
    Promise.all([t ? api.jobSection(t).catch(() => null) : null, t ? api.jobCrystals(t).catch(() => []) : []]).then(([s, c]) => {
      if (gone) return;
      section = s;
      crystals = c;
      if (!s && frame === "section") frame = "chip";
      if (!crystalFrame(c) && frame === "crystal") frame = "chip";
    });
    return () => {
      gone = true;
    };
  });
  /** The page's width: narrower than about 1000 px, the form goes under the view. */
  let width = $state(0);
  const stacked = $derived(width > 0 && width < 1000);
  /** The view's height: the 3D and top views fill it. */
  let viewHeight = $state(0);
  /** The model as last set from the text: a form that writes back the same values (a select on
   * mount, say) leaves the text, and its comments, as they are. */
  let baseline = "";
  const snapshot = (): JobModel => $state.snapshot(model) as JobModel;

  // the job as it was left (a run started from here, another page looked at), or a new one
  // until someone asks for another
  const draft = app.customDraft;
  app.customDraft = null;
  if (draft && !app.jobOpen) {
    load(draft.text, draft.path, true, draft.format).then(() => (saved = draft.saved));
  } else if (!app.jobOpen) {
    fresh("modes")
      .then((t) => {
        if (!text) load(t, null, true, "yaml");
      })
      .catch(() => {});
  }

  /** A new job of `kind`, in YAML: the form's template, converted by the library. */
  async function fresh(kind: Kind): Promise<string> {
    return api.convertJob(await api.formText(template(kind)), "toml", "yaml");
  }

  onDestroy(() => {
    if (text) app.customDraft = { text, path, format, saved };
  });

  // whoever sent the user here asked for a job to be opened
  $effect(() => {
    const o = app.jobOpen;
    if (o) {
      app.jobOpen = null;
      load(o.text, o.path);
    }
  });

  // the form changed: the text follows, in its format, as the program writes it
  $effect(() => {
    const m = snapshot();
    const key = JSON.stringify(m);
    if (key === baseline) return;
    baseline = key;
    const f = format;
    api
      .formText(m)
      .then((t) => (f === "toml" ? t : api.convertJob(t, "toml", f)))
      .then((x) => {
        if (baseline === key && format === f) {
          remember(text, x);
          text = x;
          validate();
        }
      })
      // a job the library can't read yet (a name half typed) keeps its last text until it can
      .catch(() => {});
  });

  /** The 3D preview reads TOML: the text itself, or the program's TOML of the form. */
  async function asToml(t: string): Promise<string> {
    return format === "toml" ? t : api.formText(snapshot());
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  function validate() {
    checking = true;
    clearTimeout(timer);
    timer = setTimeout(async () => {
      const t = text;
      try {
        const c = await api.checkJob(t, format);
        if (t === text) {
          check = c;
          if (c.ok) shown = await asToml(t);
        }
      } catch {
        check = null;
      }
      checking = false;
    }, 250);
  }

  async function load(t: string, p: string | null, quiet = false, f: JobFormat = p ? formatOf(p) : "toml") {
    if (!quiet && dirty && !(await ask("Discard the changes to this job?", { title: "Unsaved changes", kind: "warning" }))) return;
    let m: JobModel;
    try {
      m = await modelOf(t, f);
    } catch (e) {
      toast(`Couldn't open the job: ${e}`, "error");
      return;
    }
    baseline = JSON.stringify(m);
    format = f;
    model = m;
    text = t;
    path = p;
    saved = t;
    selection = null;
    picked = null;
    past = [];
    future = [];
    validate();
  }

  /** The text was edited: the form follows, when it reads as a job. */
  let editTimer: ReturnType<typeof setTimeout> | undefined;
  function edited(t: string) {
    text = t;
    clearTimeout(editTimer);
    checking = true;
    editTimer = setTimeout(async () => {
      const c = await api.checkJob(t, format).catch(() => null);
      if (t !== text) return;
      check = c;
      checking = false;
      const m = await api.jobForm(t, format).catch(() => null);
      if (m && t === text) {
        baseline = JSON.stringify(m);
        model = m;
      }
      if (c?.ok) shown = await asToml(t);
    }, 300);
  }

  /** The same job in another format: converted by the library, the description kept but in JSON. */
  async function setFormat(f: JobFormat) {
    if (f === format) return;
    try {
      const x = await api.convertJob(text, format, f);
      format = f;
      text = x;
      validate();
    } catch (e) {
      toast(`Fix the job before switching it to ${f.toUpperCase()}: ${e}`, "error");
    }
  }

  /** Where the job is saved: jobs/<name>.<its format's extension>. */
  const file = $derived(`jobs/${model.name}.${format}`);

  const dirty = $derived(text !== saved);
  const nameOk = $derived(/^[A-Za-z0-9._-]+$/.test(model.name));
  const grid = $derived(cells(model));
  const layers = $derived(model.stack ? STACKS[model.stack].layers : [model.layer]);
  /** An fdtd job's axes with ends to choose: x and y, and z in 3D. */
  const sideAxes = $derived<readonly ("x" | "y" | "z")[]>(model.dimensions === 3 ? ["x", "y", "z"] : ["x", "y"]);
  /** The shapes the form doesn't hold (rectangles placed by expressions, components, paths…),
   * as the library draws them for the 3D view, of the last text that checked out: the top view
   * draws them too. */
  let keptShapes = $state<Shape[]>([]);
  /** Whether the form keeps anything that draws shapes. */
  const keepsShapes = $derived(model.kept.some((k) => /^task\.(rect|circle|ring|component|path|shape|body)$/.test(k)));
  $effect(() => {
    const t = shown;
    if (!t || !keepsShapes) {
      keptShapes = [];
      return;
    }
    let live = true;
    api
      .formKeptShapes(t, "toml")
      .then((s) => {
        if (live) keptShapes = s;
      })
      .catch(() => {
        if (live) keptShapes = [];
      });
    return () => (live = false);
  });
  /** What the form keeps as the file has it, in words. */
  const keptText = $derived(model.kept.map((k) => k.replace(/^task\./, "")).join(", "));

  async function save() {
    try {
      const same = path !== null && path.replace(/\\/g, "/").endsWith(`/${model.name}.${format}`);
      let where: string;
      try {
        where = await api.saveJob(text, format, same);
      } catch (e) {
        if (!String(e).startsWith("exists:")) throw e;
        if (!(await ask(`${file} exists. Replace it?`, { title: "Replace the job?", kind: "warning" }))) return;
        where = await api.saveJob(text, format, true);
      }
      path = where;
      saved = text;
      app.workspaceVersion++;
      toast(`Saved ${file}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function runIt() {
    if (!check?.ok) {
      toast(check?.error ?? "The job isn't valid yet.", "error");
      return;
    }
    startRun(() => api.runText(text, format), `Running ${model.name}`);
  }

  async function remove() {
    if (!path) return;
    const name = path.replace(/\\/g, "/").split("/").pop();
    if (!(await ask(`Delete jobs/${name} from the workspace? Its runs stay.`, { title: "Delete the job?", kind: "warning" }))) return;
    try {
      await api.deleteJob(path);
      app.workspaceVersion++;
      saved = text;
      await load(await fresh(model.kind), null, true, "yaml");
      toast("Deleted the job", "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function setKind(kind: Kind) {
    if (kind === model.kind) return;
    const t = template(kind);
    model.kind = kind;
    if (kind === "fdfd" && !model.port.length && holds(model, "port")) model.port = t.port;
    if (kind === "fdtd") {
      // a wavelength sweep becomes the spectrum; the sources and monitors start as the template's, in this window
      const s = model.sweep;
      const [from, to] = s ? [numberOf(s.from), numberOf(s.to)] : [null, null];
      if (s?.parameter === "wavelength" && from !== null && to !== null && !model.spectrum) model.spectrum = { from_um: from, to_um: to, points: s.points };
      if (!model.source.length && holds(model, "source")) model.source = t.source.map((s) => ({ ...s, at_um: Number((model.x_um[0] + 0.25 * (model.x_um[1] - model.x_um[0])).toFixed(3)) }));
      if (!model.monitor.length && holds(model, "monitor")) model.monitor = t.monitor.map((s) => (s.type === "mode" ? { ...s, at_um: Number((model.x_um[1] - 0.25 * (model.x_um[1] - model.x_um[0])).toFixed(3)) } : s));
      model.sweep = null;
    }
    if (kind === "structure") model.sweep = null;
    // an fdfd job sweeps only the wavelength: a width's or a parameter's values aren't wavelengths,
    // so it becomes a sweep of ±50 nm about the job's own
    if (kind === "fdfd" && model.sweep && model.sweep.parameter !== "wavelength") {
      const w = model.wavelength_um;
      model.sweep = { ...model.sweep, parameter: "wavelength", from: Number((w - 0.05).toFixed(4)), to: Number((w + 0.05).toFixed(4)), rect: null };
    }
    if (kind !== "modes" && model.y_um[0] === model.y_um[1]) model.y_um = t.y_um;
    if (kind !== "modes" && model.x_um[0] === model.x_um[1]) model.x_um = t.x_um;
    // the same device in the same box: the light goes along x in an fdfd job, so a modes job
    // along y turns to x on the way there (its modes stay the same), and a job that becomes a
    // modes job cuts its cross-section inside the box it had
    if ((kind === "fdfd" || kind === "fdtd") && along(model) === "y") turn(model, "x");
    if (kind === "modes") {
      model.propagation ??= "x";
      const [span, cut] = along(model) === "x" ? [model.x_um, model.cut_x_um] : [model.y_um, model.cut_y_um];
      if ((cut ?? 0) < span[0] || (cut ?? 0) > span[1]) {
        const mid = Number(((span[0] + span[1]) / 2).toFixed(3));
        if (along(model) === "x") model.cut_x_um = mid;
        else model.cut_y_um = mid;
      }
    }
  }

  function addRect() {
    const win = previewWindow(model);
    const cx = Number(((win.x[0] + win.x[1]) / 2).toFixed(3)) || 0;
    // a guide along the light
    model.rect.push({ layer: layers[0], center_um: [cx, 0], size_um: model.kind === "modes" && along(model) === "y" ? [0.5, 1] : [1, 0.5] });
    selection = { kind: "rect", index: model.rect.length - 1 };
  }
  function addCircle() {
    model.circle.push({ layer: layers[0], center_um: [0, 0], radius_um: 0.5 });
    selection = { kind: "circle", index: model.circle.length - 1 };
  }
  function addRing() {
    model.ring.push({ layer: layers[0], center_um: [0, 0], radius_um: 1.5, width_um: 0.5 });
    selection = { kind: "ring", index: model.ring.length - 1 };
  }
  function addPort() {
    const right = model.port.length % 2 === 1;
    const x = right ? model.x_um[1] - 0.6 : model.x_um[0] + 0.6;
    model.port.push({ x_um: Number(x.toFixed(3)), side: right ? "right" : "left", y_um: null });
    selection = { kind: "port", index: model.port.length - 1 };
  }
  function addParameter() {
    let k = model.parameters.length + 1;
    while (model.parameters.some(([n]) => n === `p${k}`)) k++;
    model.parameters.push([`p${k}`, "500 nm"]);
  }

  /** A sweep end's input: a number in the unit chosen, or an expression with its units. */
  function setEnd(which: "from" | "to", raw: string) {
    if (!model.sweep) return;
    const t = raw.trim();
    const n = Number(t);
    model.sweep[which] = (t !== "" && Number.isFinite(n) ? n : t) as Num;
  }

  // ---- the runs of jobs without a device, compared (in place of the Compare page) ----
  let panel = $state<"job" | "compare">("job");
  let others = $state<RunItem[]>([]);
  $effect(() => {
    void app.workspaceVersion;
    if (panel !== "compare") return;
    api
      .home()
      .then((h) => (others = h.runs.filter((r) => !r.device)))
      .catch(() => {});
  });
  const runItems = $derived<OverlayItem[]>(
    others.map((r) => ({
      id: `run ${r.dir}`,
      label: r.job || r.name,
      sub: `${KINDS[r.kind]?.label ?? r.kind} · ${ago(r.started)}${r.seconds !== null ? ` · ${duration(r.seconds)}` : ""}`,
      what: "run",
      version: `${r.finished} ${r.seconds}`,
      load: async () => overlaidOf((await api.runEvents(r.dir)) as Event[]),
    })),
  );

  const isSel = (kind: "rect" | "circle" | "ring" | "port", index: number) => selection?.kind === kind && selection.index === index;
  // ---- click-to-edit, beside the text and in the form's 3D and Top views ----
  let editor = $state<TomlEditor>();
  /** What each entry of the text draws, of its last valid version, and the stack's layers. */
  let drawing = $state<Drawing | null>(null);
  /** The media of the stack of the last valid version, with their indices and sources (media.rs). */
  let media = $state<Medium[]>([]);
  $effect(() => {
    const t = shown;
    if (!t) return;
    let live = true;
    api
      .jobMedia(t, "toml")
      .then((m) => {
        if (live) media = m;
      })
      .catch(() => {});
    return () => (live = false);
  });
  $effect(() => {
    const [t, f] = [shown, format];
    if (!t) return;
    let live = true;
    api
      .jobDrawing(
        untrack(() => text),
        f,
      )
      .then((d) => {
        if (live) drawing = d;
      })
      .catch(() => {});
    return () => (live = false);
  });
  /** The entry picked, by its path in the text. */
  let picked = $state<Path | null>(null);
  /** What the picked entry draws, outlined in both views. */
  const marked = $derived<Shape[]>(picked && drawing ? drawing.entries.filter((e) => samePath(e.path, picked!)).flatMap((e) => e.shapes) : []);

  /** A layer picked in the 3D view, by name: a preset's has no entry of its own. */
  let pickedSlab = $state<string | null>(null);
  /** The layer the picked entry is about: the picked shape's, or the layer's picked in 3D. */
  const pickedLayer = $derived.by(() => {
    const p = picked;
    if (!p) return null;
    const shape = drawing?.entries.find((e) => samePath(e.path, p))?.shapes[0];
    return shape ? { name: shape.layer, shape: true } : pickedSlab ? { name: pickedSlab, shape: false } : null;
  });

  /** Picks the entry at `p` (none: nothing), and selects it in the text. */
  function pick(p: Path | null, slab: string | null = null) {
    picked = p;
    pickedSlab = slab;
    const span = p && entrySpan(text, format, p);
    if (span) editor?.select(span);
  }
  /** A click in the 3D view: the entry that draws the shape, or the layer's in the stack. */
  function picked3d(what: { shape: Shape } | { layer: string } | null) {
    if (!what) return pick(null);
    if ("shape" in what) return pick(drawerOf(drawing, what.shape)?.path ?? null);
    pick(layerPath(text, format, what.layer), what.layer);
  }
  /** A click from above on the form's shapes, ports, sources and monitors. */
  function pickedTop(s: Selection) {
    if (!s) return pick(null);
    if (s.kind === "rect" || s.kind === "circle" || s.kind === "ring") {
      const e = drawing?.entries.find((e) => e.list === s.kind && e.index === s.index);
      return pick(e?.path ?? ["task", s.kind, s.index]);
    }
    pick(["task", s.kind, s.index]);
  }
  /** The cursor put in the text: the entry it is in, picked without moving the cursor. */
  function cursorAt(pos: number) {
    const paths = [...(drawing?.entries.map((e) => e.path) ?? []), ...itemPaths(text, format)];
    const p = entryAt(text, format, pos, paths);
    if (!(p && picked && samePath(p, picked))) {
      picked = p;
      pickedSlab = null;
    }
  }

  // Outside "As text" the text has no editor of its own: the form's changes and the sidebar's
  // are one history here, which the sidebar's undo and redo step through.
  let past: string[] = [];
  let future: string[] = [];
  /** The text goes from `before` to `after` by the form or the sidebar, outside "As text": a step to undo. */
  function remember(before: string, after: string) {
    if (right === "toml" || before === after || !before) return;
    past.push(before);
    if (past.length > 200) past.shift();
    future = [];
  }
  /** The sidebar changed a value: written over the old one, as a step undo takes back. */
  function write(span: Span, insert: string) {
    if (right === "toml") return editor?.replace(span, insert);
    const t = text.slice(0, span.from) + insert + text.slice(span.to);
    remember(text, t);
    edited(t);
  }
  function undo() {
    if (right === "toml") return editor?.undoChange();
    const t = past.pop();
    if (t === undefined) return;
    future.push(text);
    edited(t);
  }
  function redo() {
    if (right === "toml") return editor?.redoChange();
    const t = future.pop();
    if (t === undefined) return;
    past.push(text);
    edited(t);
  }

  /** The parameters a modes job's sweep can step through: the wavelength, a rectangle's width, or a named one. */
  const sweepable = $derived(model.kind === "modes" ? ["wavelength", "width", ...model.parameters.map(([n]) => n)] : ["wavelength"]);
</script>

<svelte:window
  onkeydown={(e) => {
    if (app.page !== "components" || app.palette || !(e.ctrlKey || e.metaKey)) return;
    if (e.key.toLowerCase() === "s") {
      e.preventDefault();
      if (nameOk) save();
    } else if (e.key === "Enter") {
      e.preventDefault();
      runIt();
    }
  }}
/>

<div class="flex h-full min-h-0 flex-col" bind:clientWidth={width}>
  <!-- the header, as a device's -->
  <div class="glow shrink-0 border-b border-base-content/8 px-6 {stacked ? 'py-2.5' : 'py-4'}">
    <div class="flex flex-wrap items-start gap-4">
      {#if !stacked}
        <div class="grid size-12 shrink-0 place-items-center rounded-2xl bg-primary/12 text-primary"><Shapes size={24} /></div>
      {/if}
      <div class="min-w-[14rem] flex-1">
        <h2 class="flex flex-wrap items-baseline gap-x-2 text-xl font-semibold tracking-tight">
          {model.name}
          {#if dirty}<span class="text-base text-warning" title="Changed since it was {path ? 'saved' : 'opened'}">●</span>{/if}
          <span class="text-sm font-normal muted">Custom device</span>
        </h2>
        {#if !stacked}<p class="mt-0.5 line-clamp-1 max-w-3xl text-sm muted">Any structure no device describes, as a job: its stack and shapes, its kind, ports, sources and monitors, its window, grid and sweep.</p>{/if}
        <div class="mt-2 flex flex-wrap items-center gap-1.5 text-xs">
          <span class="badge badge-sm badge-primary badge-soft" title={KINDS[model.kind]?.about}>{KINDS[model.kind]?.label ?? model.kind}</span>
          <span class="badge badge-sm badge-ghost max-w-72 truncate" title={path ?? "not saved yet"}>{path ? `jobs/${path.split(/[\\/]/).pop()}` : "not saved yet"}</span>
          {@render status()}
        </div>
      </div>
      <div class="flex flex-wrap gap-2">
        {#if path}
          <button class="btn btn-ghost btn-sm btn-square" aria-label="Delete the job" title="Delete this job file" onclick={remove}><Trash2 size={16} /></button>
        {/if}
        <button class="btn btn-sm gap-1.5" disabled={!nameOk} onclick={save} title="Save to {file} in the workspace (Ctrl+S)"><Save size={15} /> Save</button>
        <button class="btn btn-primary btn-sm gap-1.5" disabled={!check?.ok} onclick={runIt} title="Run it now and watch it live (Ctrl+Enter)"><Play size={15} /> Run</button>
      </div>
    </div>
  </div>

  <div class="grid min-h-0 flex-1 {stacked ? 'grid-rows-[minmax(13rem,1fr)_minmax(10rem,1fr)]' : 'grid-cols-[minmax(0,1fr)_minmax(24rem,38%)]'}">
    <!-- the view: always on screen -->
    <section class="flex min-h-0 min-w-0 flex-col border-base-content/8 {stacked ? 'border-b' : 'border-r'}">
      {@render view()}
    </section>

    <!-- the form, or the runs compared -->
    {#if right === "toml"}
    <!-- the text on the left, the design it describes on the right -->
    <aside class="flex min-h-0 min-w-0 flex-col">
      <div class="min-h-0 flex-1 overflow-hidden" bind:clientHeight={designHeight}>
        {@render picture(design, designHeight)}
      </div>
      {#if picked}
        <div class="flex max-h-[50%] min-h-0 shrink-0 flex-col border-t border-base-content/8">
          {@render sidebar(picked)}
        </div>
      {/if}
      <div class="flex shrink-0 items-center gap-2 border-t border-base-content/8 px-3 py-2">
        <div class="join" role="group" aria-label="What the design shows">
          <button class="btn btn-xs join-item gap-1 {design === '3d' ? 'btn-primary btn-soft' : ''}" onclick={() => (design = "3d")} title="The structure as the run will draw it"><Box size={12} /> 3D</button>
          <button class="btn btn-xs join-item gap-1 {design === 'preview' ? 'btn-primary btn-soft' : ''}" onclick={() => (design = "preview")} title="Seen from above"><Eye size={12} /> Top</button>
        </div>
        <span class="text-[11px] faint">{design === "3d" ? "as its run will draw it, from its last valid text" : "from above, as the text has it"}; click a structure to edit it</span>
      </div>
    </aside>
    {:else}
    <aside class="@container min-h-0 min-w-0 space-y-6 overflow-x-hidden overflow-y-auto px-5 py-4">
      <div class="join w-full" role="tablist" aria-label="The job, or its runs compared">
        <button role="tab" class="btn btn-sm join-item flex-1 {panel === 'job' ? 'btn-primary btn-soft' : ''}" onclick={() => (panel = "job")}>Job</button>
        <button role="tab" class="btn btn-sm join-item flex-1 {panel === 'compare' ? 'btn-primary btn-soft' : ''}" onclick={() => (panel = "compare")}>Compare runs</button>
      </div>
      {#if panel === "compare"}
        <section>
          <p class="mb-3 text-[11px] faint">The runs of jobs without a device, overlaid: their spectra, sweeps and FDTD spectra on shared axes, each curve with its fidelity and grid. A device's runs compare on its page's Study tab.</p>
          <Overlay items={runItems} bind:picked={customCompare.picked} empty="No runs of jobs without a device in this workspace yet." />
        </section>
      {:else}
      {#if check && !check.ok && check.error}{@render problem()}{/if}
      {#if model.kept.length}
        <div role="status" class="rounded-lg border border-info/30 bg-info/8 px-3 py-2 text-xs">
          Kept as the file has it: <span class="font-medium num">{keptText}</span>. The form doesn't show {model.kept.length === 1 ? "it" : "them"}: written back unchanged, edited in the view's <b>As text</b>.
        </div>
      {/if}

      <fieldset class="space-y-3">
        <legend class="panel-title mb-2">Kind</legend>
        <div class="grid grid-cols-2 gap-2">
          {#each ["modes", "fdfd", "fdtd", "structure"] as const as kind (kind)}
            <button
              class="rounded-xl border px-3 py-2.5 text-left transition-colors {model.kind === kind ? 'border-primary bg-primary/8' : 'border-base-content/10 hover:border-base-content/25'}"
              onclick={() => setKind(kind)}
            >
              <span class="block text-sm font-medium {model.kind === kind ? 'text-primary' : ''}">{KINDS[kind].label}</span>
              <span class="mt-0.5 block text-[11px] leading-snug faint">{KINDS[kind].about}</span>
            </button>
          {/each}
        </div>
      </fieldset>

      <fieldset class="grid grid-cols-[1fr_140px] gap-3">
        <legend class="panel-title mb-2">Job</legend>
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Name</span>
          <input class="input input-sm w-full {nameOk ? '' : 'input-error'}" bind:value={model.name} title="Letters, digits, '-', '_' and '.': it names the file and the runs" />
        </label>
        <OptField label="Time limit" unit="min" bind:value={model.timeout_minutes} hint="The run stops after this long; empty for no limit" step={1} />
        <label class="col-span-2 flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">What it is</span>
          <textarea
            class="textarea textarea-sm min-h-16 w-full"
            bind:value={model.about}
            disabled={format === "json"}
            placeholder={format === "json" ? "JSON has no comments, so a JSON job has no description" : "One or two sentences: the device and what the run shows"}
          ></textarea>
        </label>
      </fieldset>

      <fieldset class="grid grid-cols-2 gap-3">
        <legend class="panel-title mb-2">Material stack and light</legend>
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Stack</span>
          {#if model.stack}
            <select class="select select-sm w-full" bind:value={model.stack} title={STACKS[model.stack].about}>
              {#each Object.entries(STACKS) as [key, s] (key)}<option value={key}>{s.label}</option>{/each}
            </select>
          {:else}
            <span class="input input-sm w-full items-center text-xs faint" title="A [task.stack] of layers: kept as the file has it, edited in the text">layers of its own (in the text)</span>
          {/if}
        </label>
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Layer</span>
          {#if model.stack}
            <select class="select select-sm w-full" bind:value={model.layer}>
              {#each layers as l (l)}<option value={l}>{l}</option>{/each}
            </select>
          {:else}
            <input class="input input-sm w-full" bind:value={model.layer} title="The layer the job looks at, by its name in the stack's layers" />
          {/if}
        </label>
        {#if model.stack && model.stack !== "soi_220"}
          <OptField label="Core" length="nm" bind:value={model.core_nm} step={10} hint="The guiding layer's thickness" />
          <OptField label="Bottom oxide" length="um" bind:value={model.bottom_oxide_um} step={0.1} hint="The buried oxide under the core" />
        {/if}
        <NumField
          label="Wavelength"
          length="um"
          bind:value={model.wavelength_um}
          step={0.01}
          hint={model.kind === "fdtd" ? "The pulse's carrier, in vacuum; the materials are taken here (the run is non-dispersive)" : `In vacuum; ${lenUnit(1.55)} is the C band`}
        />
        {#if model.kind === "fdtd"}
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Dimensions</span>
            <select class="select select-sm w-full" bind:value={model.dimensions} title="2D: the layer's plane, each point's index its slab mode's (an estimate). 3D: the stack itself, much slower">
              <option value={2}>2D, the layer's plane</option>
              <option value={3}>3D, the stack</option>
            </select>
          </label>
        {/if}
        {#if model.kind === "fdfd" || (model.kind === "fdtd" && model.dimensions === 2)}
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Polarization</span>
            <select class="select select-sm w-full" bind:value={model.polarization} title="The slab mode's polarization: TE is H along z in the 2D problem">
              <option value="te">TE (H along z)</option>
              <option value="tm">TM (E along z)</option>
            </select>
          </label>
        {/if}
      </fieldset>

      <fieldset class="space-y-2">
        <legend class="panel-title mb-2">Parameters</legend>
        {#if holds(model, "parameters")}
          <p class="text-xs faint">Named values the shapes, the components and a sweep use: each a number or an expression with units (<span class="num">500 nm</span>, <span class="num">w/2.5</span>).</p>
          {#each model.parameters as p, k (k)}
            <div class="flex items-center gap-2">
              <input class="input input-xs w-24 num" aria-label="The parameter's name" spellcheck="false" bind:value={p[0]} />
              <span class="faint">=</span>
              <input
                class="input input-xs min-w-0 flex-1 num"
                aria-label="Its expression"
                spellcheck="false"
                value={String(p[1])}
                onchange={(e) => {
                  const t = (e.currentTarget as HTMLInputElement).value.trim();
                  const n = Number(t);
                  p[1] = t !== "" && Number.isFinite(n) && typeof p[1] === "number" ? n : t;
                }}
              />
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.parameters.splice(k, 1)}><Trash2 size={13} /></button>
            </div>
          {/each}
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={addParameter}><Plus size={14} /> Parameter</button>
        {:else}
          <p class="text-xs faint">Kept as the file has them: edit them in the text.</p>
        {/if}
      </fieldset>

      <fieldset class="grid grid-cols-2 gap-3">
        <legend class="panel-title mb-2">Window and grid</legend>
        {#if model.kind !== "modes"}
          <NumField label="x from" length="um" bind:value={model.x_um[0]} step={0.1} />
          <NumField label="x to" length="um" bind:value={model.x_um[1]} step={0.1} />
          <NumField label="y from" length="um" bind:value={model.y_um[0]} step={0.1} />
          <NumField label="y to" length="um" bind:value={model.y_um[1]} step={0.1} />
        {:else}
          <label class="col-span-2 flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Light along</span>
            <select
              class="select select-sm w-full"
              value={along(model)}
              title="The axis the modes travel along. x is the convention of every kind of job; changing it turns the device with it, so its modes stay the same"
              onchange={(e) => turn(model, (e.currentTarget as HTMLSelectElement).value === "y" ? "y" : "x")}
            >
              <option value="x">x, as an FDFD job's light</option>
              <option value="y">y, as older jobs</option>
            </select>
          </label>
          {#if along(model) === "x"}
            <NumField label="y from" length="um" bind:value={model.y_um[0]} step={0.1} hint="The window across the guide" />
            <NumField label="y to" length="um" bind:value={model.y_um[1]} step={0.1} />
            <OptField label="Cut at x" length="um" bind:value={model.cut_x_um} step={0.1} hint="Where the cross-section is taken; the modes travel along x" placeholder="0" />
          {:else}
            <NumField label="x from" length="um" bind:value={model.x_um[0]} step={0.1} hint="The window across the guide" />
            <NumField label="x to" length="um" bind:value={model.x_um[1]} step={0.1} />
            <OptField label="Cut at y" length="um" bind:value={model.cut_y_um} step={0.1} hint="Where the cross-section is taken; the modes travel along y" placeholder="0" />
          {/if}
          {#if model.modes_above === null}
            <NumField label="Modes" bind:value={model.modes} integer step={1} min={1} hint="How many, from the highest effective index: 1 to 50. Or give a threshold below for every mode above it" />
          {/if}
          <OptField label="Every mode above n_eff" bind:value={model.modes_above} step={0.01} hint="Every mode whose effective index is above this, with no count: the cladding's index (1.444 for oxide) gives every guided mode. By contour integrals; empty for a count of modes" placeholder="a count" />
        {/if}
        <NumField label="Grid step" length="nm" bind:value={model.step_nm} step={5} min={1} hint="Smaller is more accurate and slower; the error falls as the square of the step" />
        {#if model.kind === "fdfd"}
          <OptField label="PML" unit="cells" bind:value={model.pml_cells} step={1} hint="Absorbing cells on each side, inside the window (20 by default)" placeholder="20" />
          <OptField label="Field at" length="um" bind:value={model.field_um} step={0.001} hint="The field is recorded at the swept wavelength nearest this: a resonance, say. The first wavelength by default." placeholder="first" />
        {:else if model.kind === "fdtd"}
          <OptField label="CPML" unit="cells" bind:value={model.pml_cells} step={1} hint="The convolutional PML's cells at each end of an axis that has one, inside the window (20 by default)" placeholder="20" />
          <OptField label="Courant number" bind:value={model.courant} step={0.05} hint="The time step, C/√(Σ 1/Δ²): stable up to 1 (0.9 by default)" placeholder="0.9" />
          {#each sideAxes as a (a)}
            <label class="flex flex-col gap-1">
              <span class="text-xs font-medium text-base-content/70">Ends along {a}</span>
              <select class="select select-sm w-full" bind:value={model.boundaries[a]} title="What is beyond the window's ends along {a}">
                <option value={null}>CPML (absorbing)</option>
                <option value="wall">walls (perfect conductor)</option>
                <option value="periodic">periodic</option>
              </select>
            </label>
          {/each}
          {#if model.dimensions === 3}
            <label class="col-span-2 flex items-center gap-1.5 text-xs faint">
              <input
                type="checkbox"
                class="toggle toggle-xs"
                checked={model.z_um !== null}
                onchange={(e) => {
                  const [a, b] = layerSpan(model);
                  model.z_um = (e.currentTarget as HTMLInputElement).checked ? [Number((a - 1).toFixed(3)), Number((b + 1).toFixed(3))] : null;
                }}
              />
              own height (1 µm below and above the layer by default)
            </label>
            {#if model.z_um}
              <NumField label="z from" length="um" bind:value={model.z_um[0]} step={0.1} hint="Heights from the stack's bottom: the substrate is below 0" />
              <NumField label="z to" length="um" bind:value={model.z_um[1]} step={0.1} />
            {/if}
          {/if}
        {:else if model.kind === "structure"}
          <OptField label="Side view at y" length="um" bind:value={model.side_y_um} step={0.1} placeholder="0" hint="Where the side view cuts" />
        {/if}
        <div class="col-span-2 flex items-center gap-2 text-xs faint">
          about <span class="num text-base-content/80">{grid.toLocaleString()}</span> cells
          {#if grid > 400_000}<span class="badge badge-warning badge-soft badge-xs">large: this may take minutes</span>{/if}
        </div>
      </fieldset>

      <fieldset class="space-y-2">
        <legend class="panel-title mb-2 flex w-full items-center">Shapes</legend>
        {#if !holds(model, "rect")}<p class="text-xs faint">Rectangles kept as the file has them (their places or sizes are expressions): drawn in both views, edited in the text.</p>{/if}
        {#each model.rect as r, k (k)}
          <div class="rounded-xl border p-3 transition-colors {isSel('rect', k) ? 'border-accent/60 bg-accent/5' : 'border-base-content/8'}">
            <div class="mb-2 flex items-center gap-2">
              <button class="text-sm font-medium" onclick={() => (selection = { kind: "rect", index: k })}>Rectangle {k + 1}</button>
              {#if model.sweep?.parameter === "width" && (model.sweep.rect ?? 0) === k}<span class="badge badge-accent badge-soft badge-xs">swept</span>{/if}
              <span class="flex-1"></span>
              <select class="select select-xs w-24" bind:value={r.layer}>{#each layers as l (l)}<option value={l}>{l}</option>{/each}</select>
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.rect.splice(k, 1)}><Trash2 size={13} /></button>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <NumField label="centre x" length="um" bind:value={r.center_um[0]} step={0.05} />
              <NumField label="centre y" length="um" bind:value={r.center_um[1]} step={0.05} />
              <NumField label="width (x)" length="um" bind:value={r.size_um[0]} step={0.05} />
              <NumField label="length (y)" length="um" bind:value={r.size_um[1]} step={0.05} />
            </div>
          </div>
        {/each}
        {#each model.circle as c, k (k)}
          <div class="rounded-xl border p-3 transition-colors {isSel('circle', k) ? 'border-accent/60 bg-accent/5' : 'border-base-content/8'}">
            <div class="mb-2 flex items-center gap-2">
              <button class="text-sm font-medium" onclick={() => (selection = { kind: "circle", index: k })}>Disk {k + 1}</button>
              <span class="flex-1"></span>
              <select class="select select-xs w-24" bind:value={c.layer}>{#each layers as l (l)}<option value={l}>{l}</option>{/each}</select>
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.circle.splice(k, 1)}><Trash2 size={13} /></button>
            </div>
            <div class="grid grid-cols-3 gap-2">
              <NumField label="centre x" length="um" bind:value={c.center_um[0]} step={0.05} />
              <NumField label="centre y" length="um" bind:value={c.center_um[1]} step={0.05} />
              <NumField label="radius" length="um" bind:value={c.radius_um} step={0.05} />
            </div>
          </div>
        {/each}
        {#each model.ring as r, k (k)}
          <div class="rounded-xl border p-3 transition-colors {isSel('ring', k) ? 'border-accent/60 bg-accent/5' : 'border-base-content/8'}">
            <div class="mb-2 flex items-center gap-2">
              <button class="text-sm font-medium" onclick={() => (selection = { kind: "ring", index: k })}>Ring {k + 1}</button>
              <span class="flex-1"></span>
              <select class="select select-xs w-24" bind:value={r.layer}>{#each layers as l (l)}<option value={l}>{l}</option>{/each}</select>
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.ring.splice(k, 1)}><Trash2 size={13} /></button>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <NumField label="centre x" length="um" bind:value={r.center_um[0]} step={0.05} />
              <NumField label="centre y" length="um" bind:value={r.center_um[1]} step={0.05} />
              <NumField label="radius" length="um" bind:value={r.radius_um} step={0.05} hint="To the waveguide's centre line, as ring resonators are specified" />
              <NumField label="width" length="um" bind:value={r.width_um} step={0.05} hint="The ring waveguide's width" />
            </div>
          </div>
        {/each}
        <div class="flex flex-wrap gap-2">
          {#if holds(model, "rect")}<button class="btn btn-ghost btn-sm gap-1.5" onclick={addRect}><Plus size={14} /> Rectangle</button>{/if}
          {#if holds(model, "circle")}<button class="btn btn-ghost btn-sm gap-1.5" onclick={addCircle}><Plus size={14} /> Disk</button>{/if}
          {#if holds(model, "ring")}<button class="btn btn-ghost btn-sm gap-1.5" onclick={addRing} title="A ring resonator's waveguide: a radius and a width"><Plus size={14} /> Ring</button>{/if}
        </div>
        <p class="text-[11px] faint">Components by their generators, paths, shapes defined by functions, arrays and bodies are written in the text (<span class="num">[[task.component]]</span>, <span class="num">[[task.path]]</span>, <span class="num">[[task.shape]]</span>, <span class="num">[[task.body]]</span>): the form keeps them, and both views draw them.</p>
      </fieldset>

      {#if model.kind === "fdfd"}
        <fieldset class="space-y-2">
          <legend class="panel-title mb-2">Ports</legend>
          {#if holds(model, "port")}
            <p class="text-xs faint">Port 1 is where the light comes in. A left port launches towards +x; a right port receives from the left.</p>
            {#each model.port as p, k (k)}
              <div class="rounded-xl border p-3 transition-colors {isSel('port', k) ? 'border-accent/60 bg-accent/5' : 'border-base-content/8'}">
                <div class="mb-2 flex items-center gap-2">
                  <button class="text-sm font-medium" onclick={() => (selection = { kind: "port", index: k })}>Port {k + 1}</button>
                  <span class="flex-1"></span>
                  <label class="flex items-center gap-1.5 text-xs faint">
                    <input
                      type="checkbox"
                      class="toggle toggle-xs"
                      checked={p.y_um !== null}
                      onchange={(e) => {
                        p.y_um = (e.currentTarget as HTMLInputElement).checked ? [0, model.y_um[1]] : null;
                      }}
                    /> own window
                  </label>
                  <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.port.splice(k, 1)}><Trash2 size={13} /></button>
                </div>
                <div class="grid grid-cols-2 gap-2">
                  <NumField label="at x" length="um" bind:value={p.x_um} step={0.05} />
                  <label class="flex flex-col gap-1">
                    <span class="text-xs font-medium text-base-content/70">Side</span>
                    <select class="select select-sm w-full" bind:value={p.side}><option value="left">left</option><option value="right">right</option></select>
                  </label>
                  {#if p.y_um}
                    <NumField label="y from" length="um" bind:value={p.y_um[0]} step={0.05} hint="One guide of several: the port's window" />
                    <NumField label="y to" length="um" bind:value={p.y_um[1]} step={0.05} />
                  {/if}
                </div>
              </div>
            {/each}
            <button class="btn btn-ghost btn-sm gap-1.5" onclick={addPort}><Plus size={14} /> Port</button>
          {:else}
            <p class="text-xs faint">Kept as the file has them (a port found by a component's name, <span class="num">port = "ring.in"</span>): edited in the text.</p>
          {/if}
        </fieldset>
      {/if}

      {#if model.kind === "fdtd"}
        <fieldset class="space-y-2">
          <legend class="panel-title mb-2">Sources</legend>
          {#if holds(model, "source")}
            <p class="text-xs faint">Every source shares one Gaussian pulse on the carrier, wide enough to cover the spectrum. Each is drawn in the top view (orange).</p>
            <FdtdItems bind:model which="source" bind:selection />
          {:else}
            <p class="text-xs faint">Kept as the file has them: edited in the text.</p>
          {/if}
        </fieldset>
        <fieldset class="space-y-2">
          <legend class="panel-title mb-2">Monitors</legend>
          {#if holds(model, "monitor")}
            <p class="text-xs faint">What the run measures, referred to the sources' incident power; drawn in the top view (blue). The decay is judged at their middles.</p>
            <FdtdItems bind:model which="monitor" bind:selection />
          {:else}
            <p class="text-xs faint">Kept as the file has them: edited in the text.</p>
          {/if}
        </fieldset>
        <fieldset class="space-y-3">
          <legend class="panel-title mb-2 flex w-full items-center gap-2">
            Spectrum
            <input
              type="checkbox"
              class="toggle toggle-xs toggle-primary"
              checked={model.spectrum !== null}
              onchange={(e) =>
                (model.spectrum = (e.currentTarget as HTMLInputElement).checked ? { from_um: Number((model.wavelength_um - 0.05).toFixed(3)), to_um: Number((model.wavelength_um + 0.05).toFixed(3)), points: 21 } : null)}
            />
          </legend>
          {#if model.spectrum}
            <div class="grid grid-cols-3 gap-2">
              <NumField label="from" length="um" bind:value={model.spectrum.from_um} step={0.01} />
              <NumField label="to" length="um" bind:value={model.spectrum.to_um} step={0.01} />
              <NumField label="points" bind:value={model.spectrum.points} integer step={1} min={1} hint="The monitors' wavelengths, all from the one pulse: more points cost little" />
            </div>
          {:else}
            <p class="text-xs faint">The monitors report the carrier alone. Turn the spectrum on for a band, all from the one pulse.</p>
          {/if}
        </fieldset>
        <fieldset class="grid grid-cols-2 gap-3">
          <legend class="panel-title mb-2">Run length and view</legend>
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Runs until</span>
            <select class="select select-sm w-full" bind:value={model.stop.until}>
              <option value="decay">the field decays</option>
              <option value="time">a time</option>
            </select>
          </label>
          {#if model.stop.until === "decay"}
            <OptField label="Decayed below" bind:value={model.stop.fraction} step={1e-9} placeholder="1e-8" hint="|field|² at the monitors' middles, as a share of its peak, over ten carrier periods" />
            <OptField label="At most" unit="µm/c" bind:value={model.stop.limit_um} step={100} placeholder="2000" hint="The longest the run may go on, c·t in µm (1 µm/c is 3.34 fs)" />
          {:else}
            <OptField label="For" unit="µm/c" bind:value={model.stop.time_um} step={50} placeholder="500" hint="c·t in µm (1 µm/c is 3.34 fs)" />
          {/if}
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Frames show</span>
            <select class="select select-sm w-full" bind:value={model.view.field} title="The field the live frames show">
              <option value={null}>the default ({model.dimensions === 3 ? "E_y" : model.polarization === "te" ? "H_z" : "E_z"})</option>
              {#each model.dimensions === 3 ? ["Ex", "Ey", "Ez", "Hx", "Hy", "Hz"] : model.polarization === "te" ? ["Ex", "Ey", "Hz"] : ["Ez", "Hx", "Hy"] as c (c)}<option value={c}>{c[0]}_{c.slice(1)}</option>{/each}
              <option value="|E|^2">|E|²</option>
              <option value="|H|^2">|H|²</option>
            </select>
          </label>
          <OptField label="A frame every" unit="µm/c" bind:value={model.view.every_um} step={0.1} placeholder="auto" hint="At first; the interval doubles as the run goes on, and when frames take more than a tenth of its time" />
          {#if model.dimensions === 3}
            <OptField label="View at z" length="um" bind:value={model.view.z_um} step={0.01} placeholder="layer" hint="The plane the frames show, seen from above: the layer's middle by default" />
          {/if}
        </fieldset>
      {/if}

      {#if model.kind !== "structure" && model.kind !== "fdtd"}
        <fieldset class="space-y-3">
          <legend class="panel-title mb-2 flex w-full items-center gap-2">
            Sweep
            <input
              type="checkbox"
              class="toggle toggle-xs toggle-primary"
              checked={model.sweep !== null}
              onchange={(e) =>
                (model.sweep = (e.currentTarget as HTMLInputElement).checked
                  ? { parameter: "wavelength", from: Number((model.wavelength_um - 0.05).toFixed(3)), to: Number((model.wavelength_um + 0.05).toFixed(3)), points: 11, rect: null }
                  : null)}
            />
          </legend>
          {#if model.sweep}
            {@const s = model.sweep}
            <div class="grid grid-cols-2 gap-2">
              <label class="flex flex-col gap-1">
                <span class="text-xs font-medium text-base-content/70">Over</span>
                <select class="select select-sm w-full" bind:value={s.parameter} disabled={model.kind === "fdfd"} title="The wavelength, a rectangle's width, or one of the parameters above">
                  {#each sweepable as name (name)}<option value={name}>{name === "width" ? "a width" : name}</option>{/each}
                  {#if !sweepable.includes(s.parameter)}<option value={s.parameter}>{s.parameter}</option>{/if}
                </select>
              </label>
              <NumField label="points" bind:value={s.points} integer step={1} min={1} />
              {#each ["from", "to"] as const as end (end)}
                {#if typeof s[end] === "number" && (s.parameter === "wavelength" || s.parameter === "width")}
                  <NumField label={end} length="um" bind:value={() => s[end] as number, (v) => (s[end] = v)} step={0.01} />
                {:else}
                  <label class="flex flex-col gap-1">
                    <span class="text-xs font-medium text-base-content/70">{end}</span>
                    <input class="input input-sm w-full num" spellcheck="false" value={String(s[end])} title="A number (µm for a length, radians for an angle) or an expression with its units: 150 nm" onchange={(e) => setEnd(end, (e.currentTarget as HTMLInputElement).value)} />
                  </label>
                {/if}
              {/each}
              {#if s.parameter === "width"}
                <!-- numbered as the shapes above are, from 1; the job counts from 0 -->
                <OptField
                  label="Rectangle"
                  bind:value={
                    () => (model.sweep?.rect == null ? null : model.sweep.rect + 1),
                    (v) => {
                      if (model.sweep) model.sweep.rect = v == null ? null : Math.max(0, v - 1);
                    }
                  }
                  integer
                  step={1}
                  min={1}
                  hint="Which rectangle's width (its size across the guide): Rectangle 1, 2, … as numbered under Shapes"
                  placeholder="1"
                />
              {/if}
            </div>
            {#if s.parameter !== "wavelength" && s.parameter !== "width"}
              <p class="text-[11px] faint">A parameter's sweep: at each point the shapes and components that use <span class="num">{s.parameter}</span> follow it.</p>
            {/if}
          {:else}
            <p class="text-xs faint">One wavelength. Turn the sweep on to trace the effective indices{model.kind === "fdfd" ? " or the spectrum" : ", or step any parameter"} over a range.</p>
          {/if}
        </fieldset>
      {/if}
      {/if}
    </aside>
    {/if}
  </div>
</div>

{#snippet status()}
  {#if checking}
    <span class="flex shrink-0 items-center gap-1 faint"><LoaderCircle size={13} class="animate-spin" /> checking</span>
  {:else if check?.ok}
    <span class="flex shrink-0 items-center gap-1 text-success"><CircleCheck size={13} /> ready to run</span>
  {:else if check}
    <span class="flex shrink-0 items-center gap-1 text-error"><CircleAlert size={13} /> not valid</span>
  {/if}
{/snippet}

{#snippet sidebar(p: Path)}
  <EntryPanel
    {text}
    {format}
    path={p}
    model={check?.model}
    layers={drawing?.layers ?? []}
    {media}
    layer={pickedLayer}
    {crystals}
    onwrite={write}
    onundo={undo}
    onredo={redo}
    onclose={() => (picked = null)}
    onpick={(q) => pick(q)}
  />
{/snippet}

{#snippet problem()}
  {#if check && !check.ok && check.error}
    <div class="rounded-lg border border-error/30 bg-error/8 px-3 py-2 text-xs text-error selectable">{check.error}</div>
  {/if}
{/snippet}

{#snippet picture(kind: "3d" | "preview", h: number)}
  {#if kind === "preview"}
    <div class="h-full overflow-y-auto p-4">
      {#if right === "toml"}
        <GeometryPreview {model} kept={keptShapes} height={Math.max(140, h - (stacked ? 64 : 80))} onselect={pickedTop} onpickshape={(s) => pick(drawerOf(drawing, s)?.path ?? null)} {marked} {frame} {section} oriented={oriented?.oriented ?? null} />
      {:else}
        <GeometryPreview
          {model}
          kept={keptShapes}
          height={Math.max(140, h - (stacked ? 64 : 80))}
          selected={selection}
          onselect={(s) => {
            selection = s;
            pickedTop(s);
          }}
          onpickshape={(s) => pick(drawerOf(drawing, s)?.path ?? null)}
          {marked}
          {frame}
          {section}
          oriented={oriented?.oriented ?? null}
        />
      {/if}
      <div class="mt-3 flex flex-wrap gap-x-4 gap-y-1.5 px-1 text-xs faint">
        <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm bg-primary/65"></span>silicon</span>
        <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm bg-secondary/70"></span>nitride</span>
        {#if model.kind === "fdfd"}
          <span class="flex items-center gap-1.5"><span class="h-2.5 w-0.5 bg-success"></span>ports</span>
          <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm border border-warning/50 bg-warning/20"></span>PML</span>
        {/if}
        {#if model.kind === "fdtd"}
          <span class="flex items-center gap-1.5"><span class="h-2.5 w-0.5 bg-warning"></span>sources</span>
          <span class="flex items-center gap-1.5"><span class="h-2.5 w-0.5 bg-info"></span>monitors</span>
          <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm border border-warning/50 bg-warning/20"></span>CPML</span>
        {/if}
        {#if model.kind === "modes" || section}<span class="flex items-center gap-1.5"><span class="h-0.5 w-3 bg-accent"></span>cross-section, its across axis</span>{/if}
        <span>seen from above, x to the right, y up</span>
      </div>
    </div>
  {:else}
    <div class="relative h-full">
      {#if shown}
        <ScenePreview text={shown} height={Math.max(160, h)} turnable axes {frame} onpick={picked3d} {marked} />
        {#if check && !check.ok}
          <div class="absolute inset-x-3 top-3 rounded-lg border border-warning/30 bg-base-100/85 px-3 py-1.5 text-xs backdrop-blur">The job isn't valid now: this is its last valid version.</div>
        {/if}
      {:else}
        <div class="grid h-full place-items-center p-6 text-center">
          <div class="flex flex-col items-center gap-3 text-sm faint">
            <Box size={22} />
            <span>The 3D view appears once the job is valid{check && !check.ok ? (right === "toml" ? "; it isn't yet (see the text)" : "; it isn't yet (see the form)") : ""}.</span>
            <button class="btn btn-sm gap-1.5" onclick={() => (right === "toml" ? (design = "preview") : (right = "preview"))}><Eye size={14} /> Top view</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet view()}
  <div class="min-h-0 flex-1 overflow-hidden" bind:clientHeight={viewHeight}>
    {#if right !== "toml"}
      <div class="relative h-full">
        {@render picture(right, viewHeight)}
        {#if picked}
          <!-- the sidebar over the design, editing the same job's text -->
          <div class="absolute top-3 right-3 bottom-3 flex w-[min(22rem,calc(100%-1.5rem))] min-h-0 flex-col overflow-hidden rounded-xl border border-base-content/10 bg-base-100/95 shadow-lg backdrop-blur">
            {@render sidebar(picked)}
          </div>
        {/if}
      </div>
    {:else}
      <div class="flex h-full min-h-0 flex-col p-3">
        <div class="mb-2 flex flex-wrap items-center gap-2">
          <div class="join" role="group" aria-label="The job's format">
            {#each ["yaml", "toml", "json"] as const as f (f)}
              <button class="btn join-item btn-xs {format === f ? 'btn-primary btn-soft' : ''}" onclick={() => setFormat(f)} title="The same job in {f.toUpperCase()}, saved as jobs/{model.name}.{f}">{f.toUpperCase()}</button>
            {/each}
          </div>
          {#if format === "json"}<span class="text-[11px] faint">JSON has no comments: the description isn't kept.</span>{/if}
        </div>
        <div class="min-h-0 flex-1 overflow-hidden rounded-xl border border-base-content/8 bg-base-300/50 py-2">
          <TomlEditor bind:this={editor} value={text} onchange={edited} oncursor={cursorAt} {format} assist model={check?.model} problem={check && !check.ok ? check.error : null} />
        </div>
        <div class="mt-2 shrink-0">{@render problem()}</div>
      </div>
    {/if}
  </div>
  <div class="flex shrink-0 flex-wrap items-center gap-2 border-t border-base-content/8 px-3 py-2">
    <div class="join" role="group" aria-label="What the view shows">
      <button class="btn btn-xs join-item gap-1 {right === '3d' ? 'btn-primary btn-soft' : ''}" onclick={() => (right = "3d")} title="The structure as the run will draw it"><Box size={12} /> 3D</button>
      <button class="btn btn-xs join-item gap-1 {right === 'preview' ? 'btn-primary btn-soft' : ''}" onclick={() => (right = "preview")} title="Seen from above: click a shape to edit it"><Eye size={12} /> Top</button>
      <button class="btn btn-xs join-item gap-1 {right === 'toml' ? 'btn-primary btn-soft' : ''}" onclick={() => (right = "toml")} title="The job's text"><Code size={12} /> As text</button>
    </div>
    {#if right !== "toml"}
      <div class="join" role="group" aria-label="The frame the axes are named in">
        <button class="btn btn-xs join-item {frame === 'chip' ? 'btn-primary btn-soft' : ''}" onclick={() => (frame = "chip")} title="The chip's frame: x and y in the plane, z up">Chip</button>
        <button
          class="btn btn-xs join-item {frame === 'section' ? 'btn-primary btn-soft' : ''}"
          disabled={!section}
          onclick={() => (frame = "section")}
          title={section ? `The cross-section's frame: across the guide (the chip's ${section.across}) and z up; the 3D view looks along the guide` : "Only a modes or static job is cut"}>Cross-section</button
        >
        <button
          class="btn btn-xs join-item {frame === 'crystal' ? 'btn-primary btn-soft' : ''}"
          disabled={!oriented}
          onclick={() => (frame = "crystal")}
          title={oriented
            ? `${oriented.crystal.formula}'s frame (${oriented.oriented.label}): the chip's axes named by the crystal axis along each; the 3D view draws the crystal's axes on what it fills and looks along its c axis`
            : crystalNote(crystals)}>Crystal</button
        >
      </div>
      {#if crystals.length}
        <span class="badge badge-xs badge-outline border-[#b07fd6] text-[#b07fd6]" title={crystalNote(crystals)}>
          {crystals.map((c) => `${c.formula}: ${c.oriented.length ? c.oriented.map((o) => (o.label.startsWith("Euler") ? o.label : o.label.split(",")[0])).join(", ") : "its cut not given"}`).join("; ")}
        </span>
      {/if}
      <span class="hidden text-[11px] faint xl:inline">{frameCaption(frame, section, crystals)}</span>
    {/if}
    <span class="flex-1"></span>
    <span class="hidden text-[11px] faint lg:inline">{right === "3d" ? `as its run will draw it, over its window${model.kind === "modes" ? ", behind the cut" : ""}; click a structure to edit it` : right === "preview" ? "click a structure to edit it" : "Ctrl+Space: the keys and values that fit; hover a key for its docs"}</span>
  </div>
{/snippet}
