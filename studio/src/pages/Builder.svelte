<script lang="ts">
  // The job builder: a job as a form, with its device drawn as you type and its TOML beside it,
  // checked by the library as it changes; saved to the workspace's jobs/ and run from here.
  import {
    ChevronDown,
    CircleAlert,
    CircleCheck,
    Code,
    Copy,
    FileText,
    Plus,
    Play,
    Save,
    Trash2,
    Eye,
    LoaderCircle,
    Box,
  } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";

  import GeometryPreview, { type Selection } from "../components/GeometryPreview.svelte";
  import NumField from "../components/NumField.svelte";
  import OptField from "../components/OptField.svelte";
  import ScenePreview from "../components/ScenePreview.svelte";
  import Tip from "../components/Tip.svelte";
  import TomlEditor from "../components/TomlEditor.svelte";
  import { ago, api, KINDS, type JobCheck, type JobItem } from "../lib/api";
  import { app, startRun, toast } from "../lib/app.svelte";
  import { catalog, loadCatalog, modelOf } from "../lib/catalog.svelte";
  import { cells, fromModel, STACKS, template, toToml, type JobModel, type Kind } from "../lib/job";

  let model = $state<JobModel>(template("modes"));
  let text = $state(toToml(template("modes")));
  let path = $state<string | null>(null);
  let saved = $state(toToml(template("modes")));
  let check = $state<JobCheck | null>(null);
  let checking = $state(false);
  let selection = $state<Selection>(null);
  let right = $state<"preview" | "3d" | "toml">("preview");
  /** The text the 3D preview shows: the last one that checked out, so it doesn't rebuild on every key. */
  let shown = $state("");
  let jobs = $state<JobItem[]>([]);
  /** The model's TOML when it was last set from the text: a form that writes back the same
   * values (a select on mount, say) leaves the text, and its comments, as they are. */
  let baseline = toToml(template("modes"));

  loadCatalog();
  queueMicrotask(() => validate());

  $effect(() => {
    void app.workspaceVersion;
    api.home().then((h) => (jobs = h.jobs)).catch(() => {});
  });

  // whoever sent the user here asked for a job to be opened
  $effect(() => {
    const o = app.builderOpen;
    if (o) {
      app.builderOpen = null;
      load(o.text, o.path);
    }
  });

  // the form changed: the text follows
  $effect(() => {
    const t = toToml($state.snapshot(model) as JobModel);
    if (t === baseline) return;
    baseline = t;
    text = t;
    validate();
  });

  let timer: ReturnType<typeof setTimeout> | undefined;
  function validate() {
    checking = true;
    clearTimeout(timer);
    timer = setTimeout(async () => {
      const t = text;
      try {
        const c = await api.checkJob(t);
        if (t === text) {
          check = c;
          if (c.ok) shown = t;
        }
      } catch {
        check = null;
      }
      checking = false;
    }, 250);
  }

  async function load(t: string, p: string | null) {
    if (dirty && !(await ask("Discard the changes to this job?", { title: "Unsaved changes", kind: "warning" }))) return;
    const m = await modelOf(t);
    baseline = toToml(m);
    model = m;
    text = t;
    path = p;
    saved = t;
    selection = null;
    validate();
  }

  /** The TOML was edited: the form follows, when it parses. */
  let editTimer: ReturnType<typeof setTimeout> | undefined;
  function edited(t: string) {
    text = t;
    clearTimeout(editTimer);
    checking = true;
    editTimer = setTimeout(async () => {
      const c = await api.checkJob(t).catch(() => null);
      if (t !== text) return;
      check = c;
      checking = false;
      if (c?.ok) shown = t;
      if (c?.model) {
        const m = fromModel(c.model, t);
        baseline = toToml(m);
        model = m;
      }
    }, 300);
  }

  const dirty = $derived(text !== saved);
  const nameOk = $derived(/^[A-Za-z0-9._-]+$/.test(model.name));
  const grid = $derived(cells(model));
  const layers = $derived(STACKS[model.stack].layers);

  async function save() {
    try {
      const same = path !== null && path.replace(/\\/g, "/").endsWith(`/${model.name}.toml`);
      let where: string;
      try {
        where = await api.saveJob(text, same);
      } catch (e) {
        if (!String(e).startsWith("exists:")) throw e;
        if (!(await ask(`jobs/${model.name}.toml exists. Replace it?`, { title: "Replace the job?", kind: "warning" }))) return;
        where = await api.saveJob(text, true);
      }
      path = where;
      saved = text;
      app.workspaceVersion++;
      toast(`Saved jobs/${model.name}.toml`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function runIt() {
    if (!check?.ok) {
      toast(check?.error ?? "The job isn't valid yet.", "error");
      return;
    }
    startRun(() => api.runText(text), `Running ${model.name}`);
  }

  async function remove() {
    if (!path) return;
    if (!(await ask(`Delete jobs/${model.name}.toml from the workspace? Its runs stay.`, { title: "Delete the job?", kind: "warning" }))) return;
    try {
      await api.deleteJob(path);
      app.workspaceVersion++;
      saved = text;
      await load(toToml(template(model.kind)), null);
      toast("Deleted the job", "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function setKind(kind: Kind) {
    if (kind === model.kind) return;
    const t = template(kind);
    model.kind = kind;
    if (kind === "fdfd" && !model.port.length) model.port = t.port;
    if (kind === "structure") model.sweep = null;
    if (kind === "fdfd" && model.sweep) model.sweep.parameter = "wavelength";
    if (kind !== "modes" && model.y_um[0] === model.y_um[1]) model.y_um = t.y_um;
  }

  function addRect() {
    const c = model.x_um.map((v, k) => (v + [model.x_um[1], model.y_um[1]][k]) / 2);
    model.rect.push({ layer: layers[0], center_um: [Number(c[0].toFixed(3)) || 0, 0], size_um: [1, 0.5] });
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

  const isSel = (kind: "rect" | "circle" | "ring" | "port", index: number) => selection?.kind === kind && selection.index === index;
</script>

<svelte:window
  onkeydown={(e) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key.toLowerCase() === "s") {
      e.preventDefault();
      if (nameOk) save();
    } else if (e.key === "Enter") {
      e.preventDefault();
      runIt();
    }
  }}
/>

<div class="grid h-full grid-cols-[250px_minmax(420px,1fr)_minmax(380px,0.95fr)]">
  <!-- the workspace's jobs -->
  <aside class="flex min-h-0 flex-col border-r border-base-content/8 bg-base-100/40">
    <div class="p-3">
      <div class="dropdown w-full">
        <div tabindex="0" role="button" class="btn btn-primary btn-sm w-full gap-1.5"><Plus size={15} /> New job <ChevronDown size={14} /></div>
        <ul tabindex="-1" class="dropdown-content menu z-30 mt-1 w-64 rounded-box border border-base-content/10 bg-base-100 p-2 shadow-xl">
          {#each ["modes", "fdfd", "structure"] as const as kind (kind)}
            <li>
              <button class="flex flex-col items-start gap-0" onclick={() => load(toToml(template(kind)), null)}>
                <span class="font-medium">{KINDS[kind].label}</span><span class="text-xs faint">{KINDS[kind].about}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    </div>
    <div class="flex-1 overflow-y-auto px-2 pb-3">
      <p class="px-2 pt-1 pb-1.5 text-[10.5px] font-semibold tracking-wider uppercase faint">In the workspace</p>
      {#each jobs as j (j.path)}
        <button
          class="w-full rounded-lg px-3 py-2 text-left {path === j.path ? 'bg-primary/12' : 'hover:bg-base-content/4'}"
          onclick={() => api.readJob(j.path).then((t) => load(t, j.path))}
        >
          <span class="flex items-center gap-2">
            <FileText size={14} class={path === j.path ? "text-primary" : "faint"} />
            <span class="flex-1 truncate text-sm {path === j.path ? 'font-medium text-primary' : ''}">{j.name}</span>
          </span>
          <span class="block truncate pl-6 text-[11px] faint">{KINDS[j.kind]?.label ?? j.kind} · {ago(j.modified)}</span>
        </button>
      {:else}
        <p class="px-3 py-2 text-xs faint">No jobs saved yet. Save one and it appears here.</p>
      {/each}
      <p class="px-2 pt-4 pb-1.5 text-[10.5px] font-semibold tracking-wider uppercase faint">Start from an example</p>
      {#each catalog.data?.jobs ?? [] as j (j.file)}
        <button class="flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-left text-sm hover:bg-base-content/4" onclick={() => load(j.text, null)}>
          <Copy size={13} class="faint" /><span class="truncate">{j.name}</span>
        </button>
      {/each}
    </div>
  </aside>

  <!-- the form -->
  <section class="flex min-h-0 flex-col">
    <div class="flex items-center gap-3 border-b border-base-content/8 px-6 py-3">
      <div class="min-w-0 flex-1">
        <p class="truncate font-semibold">{model.name}{dirty ? " •" : ""}</p>
        <p class="truncate text-xs faint">{path ? path : "not saved yet"}</p>
      </div>
      {#if path}
        <div class="tooltip tooltip-bottom" data-tip="Delete this job file">
          <button class="btn btn-ghost btn-sm btn-square" aria-label="Delete the job" onclick={remove}><Trash2 size={16} /></button>
        </div>
      {/if}
      <button class="btn btn-sm gap-1.5" disabled={!nameOk} onclick={save} title="Save to jobs/{model.name}.toml in the workspace (Ctrl+S)"><Save size={15} /> Save</button>
      <button class="btn btn-primary btn-sm gap-1.5" disabled={!check?.ok} onclick={runIt} title="Run it now and watch it live (Ctrl+Enter)"><Play size={15} /> Run</button>
    </div>

    <div class="flex-1 space-y-6 overflow-x-hidden overflow-y-auto px-6 py-5">
      <Tip id="builder-intro" title="A job is a TOML file">
        Fill the form and the device is drawn on the right as you type; the library checks the job as it changes. Click a shape in the drawing to edit it. Prefer text? Switch the right panel to TOML: edits there update the form.
      </Tip>

      <fieldset class="space-y-3">
        <legend class="panel-title mb-2">Kind</legend>
        <div class="grid grid-cols-3 gap-2">
          {#each ["modes", "fdfd", "structure"] as const as kind (kind)}
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
          <textarea class="textarea textarea-sm min-h-16 w-full" bind:value={model.about} placeholder="One or two sentences: the device and what the run shows"></textarea>
        </label>
      </fieldset>

      <fieldset class="grid grid-cols-2 gap-3">
        <legend class="panel-title mb-2">Material stack and light</legend>
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Stack</span>
          <select class="select select-sm w-full" bind:value={model.stack} title={STACKS[model.stack].about}>
            {#each Object.entries(STACKS) as [key, s] (key)}<option value={key}>{s.label}</option>{/each}
          </select>
        </label>
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Layer</span>
          <select class="select select-sm w-full" bind:value={model.layer}>
            {#each layers as l (l)}<option value={l}>{l}</option>{/each}
          </select>
        </label>
        {#if model.stack !== "soi_220"}
          <OptField label="Core" unit="nm" bind:value={model.core_nm} step={10} hint="The guiding layer's thickness" />
          <OptField label="Bottom oxide" unit="µm" bind:value={model.bottom_oxide_um} step={0.1} hint="The buried oxide under the core" />
        {/if}
        <NumField label="Wavelength" unit="µm" bind:value={model.wavelength_um} step={0.01} hint="In vacuum; 1.55 µm is the C band" />
        {#if model.kind === "fdfd"}
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Polarization</span>
            <select class="select select-sm w-full" bind:value={model.polarization} title="The slab mode's polarization: TE is H along z in the 2D problem">
              <option value="te">TE (H along z)</option>
              <option value="tm">TM (E along z)</option>
            </select>
          </label>
        {/if}
      </fieldset>

      <fieldset class="grid grid-cols-2 gap-3">
        <legend class="panel-title mb-2">Window and grid</legend>
        <NumField label="x from" unit="µm" bind:value={model.x_um[0]} step={0.1} />
        <NumField label="x to" unit="µm" bind:value={model.x_um[1]} step={0.1} />
        {#if model.kind !== "modes"}
          <NumField label="y from" unit="µm" bind:value={model.y_um[0]} step={0.1} />
          <NumField label="y to" unit="µm" bind:value={model.y_um[1]} step={0.1} />
        {:else}
          <OptField label="Cut at y" unit="µm" bind:value={model.cut_y_um} step={0.1} hint="Where the cross-section is taken; the modes travel along y" placeholder="0" />
          <NumField label="Modes" bind:value={model.modes} integer step={1} min={1} hint="How many, from the highest effective index" />
        {/if}
        <NumField label="Grid step" unit="nm" bind:value={model.step_nm} step={5} min={1} hint="Smaller is more accurate and slower; the error falls as the square of the step" />
        {#if model.kind === "fdfd"}
          <OptField label="PML" unit="cells" bind:value={model.pml_cells} step={1} hint="Absorbing cells on each side, inside the window (20 by default)" placeholder="20" />
          <OptField label="Field at" unit="µm" bind:value={model.field_um} step={0.001} hint="The field is recorded at the swept wavelength nearest this: a resonance, say. The first wavelength by default." placeholder="first" />
        {:else if model.kind === "structure"}
          <OptField label="Side view at y" unit="µm" bind:value={model.side_y_um} step={0.1} placeholder="0" hint="Where the side view cuts" />
        {/if}
        <div class="col-span-2 flex items-center gap-2 text-xs faint">
          about <span class="num text-base-content/80">{grid.toLocaleString()}</span> cells
          {#if grid > 400_000}<span class="badge badge-warning badge-soft badge-xs">large: this may take minutes</span>{/if}
        </div>
      </fieldset>

      <fieldset class="space-y-2">
        <legend class="panel-title mb-2 flex w-full items-center">Shapes</legend>
        {#each model.rect as r, k (k)}
          <div class="rounded-xl border p-3 transition-colors {isSel('rect', k) ? 'border-accent/60 bg-accent/5' : 'border-base-content/8'}">
            <div class="mb-2 flex items-center gap-2">
              <button class="text-sm font-medium" onclick={() => (selection = { kind: "rect", index: k })}>Rectangle {k + 1}</button>
              {#if model.sweep?.parameter === "width" && (model.sweep.rect ?? 0) === k}<span class="badge badge-accent badge-soft badge-xs">swept</span>{/if}
              <span class="flex-1"></span>
              <select class="select select-xs w-24" bind:value={r.layer}>{#each layers as l (l)}<option value={l}>{l}</option>{/each}</select>
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.rect.splice(k, 1)}><Trash2 size={13} /></button>
            </div>
            <div class="grid grid-cols-4 gap-2">
              <NumField label="centre x" unit="µm" bind:value={r.center_um[0]} step={0.05} />
              <NumField label="centre y" unit="µm" bind:value={r.center_um[1]} step={0.05} />
              <NumField label="width (x)" unit="µm" bind:value={r.size_um[0]} step={0.05} />
              <NumField label="length (y)" unit="µm" bind:value={r.size_um[1]} step={0.05} />
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
              <NumField label="centre x" unit="µm" bind:value={c.center_um[0]} step={0.05} />
              <NumField label="centre y" unit="µm" bind:value={c.center_um[1]} step={0.05} />
              <NumField label="radius" unit="µm" bind:value={c.radius_um} step={0.05} />
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
            <div class="grid grid-cols-4 gap-2">
              <NumField label="centre x" unit="µm" bind:value={r.center_um[0]} step={0.05} />
              <NumField label="centre y" unit="µm" bind:value={r.center_um[1]} step={0.05} />
              <NumField label="radius" unit="µm" bind:value={r.radius_um} step={0.05} hint="To the waveguide's centre line, as ring resonators are specified" />
              <NumField label="width" unit="µm" bind:value={r.width_um} step={0.05} hint="The ring waveguide's width" />
            </div>
          </div>
        {/each}
        <div class="flex gap-2">
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={addRect}><Plus size={14} /> Rectangle</button>
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={addCircle}><Plus size={14} /> Disk</button>
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={addRing} title="A ring resonator's waveguide: a radius and a width"><Plus size={14} /> Ring</button>
        </div>
      </fieldset>

      {#if model.kind === "fdfd"}
        <fieldset class="space-y-2">
          <legend class="panel-title mb-2">Ports</legend>
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
                    onchange={(e) => (p.y_um = (e.currentTarget as HTMLInputElement).checked ? [0, model.y_um[1]] : null)}
                  /> own window
                </label>
                <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => model.port.splice(k, 1)}><Trash2 size={13} /></button>
              </div>
              <div class="grid grid-cols-4 gap-2">
                <NumField label="at x" unit="µm" bind:value={p.x_um} step={0.05} />
                <label class="flex flex-col gap-1">
                  <span class="text-xs font-medium text-base-content/70">Side</span>
                  <select class="select select-sm w-full" bind:value={p.side}><option value="left">left</option><option value="right">right</option></select>
                </label>
                {#if p.y_um}
                  <NumField label="y from" unit="µm" bind:value={p.y_um[0]} step={0.05} hint="One guide of several: the port's window" />
                  <NumField label="y to" unit="µm" bind:value={p.y_um[1]} step={0.05} />
                {/if}
              </div>
            </div>
          {/each}
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={addPort}><Plus size={14} /> Port</button>
        </fieldset>
      {/if}

      {#if model.kind !== "structure"}
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
            <div class="grid grid-cols-4 gap-2">
              <label class="flex flex-col gap-1">
                <span class="text-xs font-medium text-base-content/70">Over</span>
                <select class="select select-sm w-full" bind:value={model.sweep.parameter} disabled={model.kind === "fdfd"}>
                  <option value="wavelength">wavelength</option>
                  {#if model.kind === "modes"}<option value="width">a width</option>{/if}
                </select>
              </label>
              <NumField label="from" unit="µm" bind:value={model.sweep.from} step={0.01} />
              <NumField label="to" unit="µm" bind:value={model.sweep.to} step={0.01} />
              <NumField label="points" bind:value={model.sweep.points} integer step={1} min={1} />
              {#if model.sweep.parameter === "width"}
                <OptField label="Rectangle" bind:value={model.sweep.rect} integer step={1} hint="Which rectangle's width, counting from 0" placeholder="0" />
              {/if}
            </div>
          {:else}
            <p class="text-xs faint">One wavelength. Turn the sweep on to trace the effective indices{model.kind === "fdfd" ? " or the spectrum" : ""} over a range.</p>
          {/if}
        </fieldset>
      {/if}
    </div>
  </section>

  <!-- the device, and the TOML -->
  <section class="flex min-h-0 flex-col border-l border-base-content/8 bg-base-100/40">
    <div class="flex items-center gap-2 border-b border-base-content/8 px-4 py-2.5">
      <div class="join">
        <button class="btn join-item btn-sm gap-1.5 {right === 'preview' ? 'btn-primary btn-soft' : ''}" onclick={() => (right = "preview")} title="Seen from above: click a shape to edit it"><Eye size={14} /> Top view</button>
        <button class="btn join-item btn-sm gap-1.5 {right === '3d' ? 'btn-primary btn-soft' : ''}" onclick={() => (right = "3d")} title="The structure as the run will draw it"><Box size={14} /> 3D</button>
        <button class="btn join-item btn-sm gap-1.5 {right === 'toml' ? 'btn-primary btn-soft' : ''}" onclick={() => (right = "toml")}><Code size={14} /> TOML</button>
      </div>
      <span class="flex-1"></span>
      {#if checking}
        <span class="flex items-center gap-1.5 text-xs faint"><LoaderCircle size={14} class="animate-spin" /> checking</span>
      {:else if check?.ok}
        <span class="flex items-center gap-1.5 text-xs text-success"><CircleCheck size={14} /> ready to run</span>
      {:else if check}
        <span class="flex items-center gap-1.5 text-xs text-error"><CircleAlert size={14} /> not valid</span>
      {/if}
    </div>
    {#if check && !check.ok && check.error}
      <div class="mx-4 mt-3 rounded-lg border border-error/30 bg-error/8 px-3 py-2 text-xs text-error selectable">{check.error}</div>
    {/if}
    {#if right === "preview"}
      <div class="flex-1 overflow-y-auto p-4">
        <div class="panel p-3">
          <GeometryPreview {model} height={420} selected={selection} onselect={(s) => (selection = s)} />
        </div>
        <div class="mt-3 flex flex-wrap gap-x-4 gap-y-1.5 px-1 text-xs faint">
          <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm bg-primary/65"></span>silicon</span>
          <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm bg-secondary/70"></span>nitride</span>
          {#if model.kind === "fdfd"}
            <span class="flex items-center gap-1.5"><span class="h-2.5 w-0.5 bg-success"></span>ports</span>
            <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm border border-warning/50 bg-warning/20"></span>PML</span>
          {/if}
          {#if model.kind === "modes"}<span class="flex items-center gap-1.5"><span class="h-0.5 w-3 bg-accent"></span>cross-section</span>{/if}
          <span>seen from above, x across, y up</span>
        </div>
      </div>
    {:else if right === "3d"}
      <div class="flex-1 overflow-y-auto p-4">
        <div class="glow panel overflow-hidden">
          {#if shown}
            <ScenePreview text={shown} height={440} />
          {:else}
            <div class="grid h-[440px] place-items-center text-sm faint">The 3D view appears once the job is valid.</div>
          {/if}
        </div>
        <p class="mt-3 px-1 text-xs faint">The structure as its run will draw it, over the job's window{model.kind === "modes" ? ", behind the cut" : ""}.</p>
      </div>
    {:else}
      <div class="m-4 min-h-0 flex-1 overflow-hidden rounded-xl border border-base-content/8 bg-base-300/50 py-2">
        <TomlEditor value={text} onchange={edited} />
      </div>
    {/if}
  </section>
</div>
