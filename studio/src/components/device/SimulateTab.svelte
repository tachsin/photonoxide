<script lang="ts">
  // A device's Simulate tab: the design run as a job (photonoxide::job::from_device), by its own
  // model at a fidelity (a closed form, its cross-section's modes, 2D FDFD labelled 2D, 3D FDFD)
  // or by FDTD of its drawn structure; checked by the library, recorded in runs/, replayable. The
  // run shows in place: on the device in the page's main view (its Run view), its spectra and
  // plots here, with the past runs of this device. "As text" is the whole job file in the shared
  // job editor (#668: the schema's completions and hover docs, its Reference and Problems), in
  // the page's format: an edit the form can hold moves the form; one it can't says why and is
  // kept, the run then using the text as it is. The text is the page's one run text
  // (`draft.text`), the one the Job tab's "The run" shows: an edit in either is the other's.
  import { Code, Eye, FileText, History, Play, RotateCcw, Save, Square, TriangleAlert } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";

  import { ago, api, duration, formatOfText, type DeviceInfo, type DeviceSpec, type JobForm, type JobFormat, type RunItem } from "../../lib/api";
  import { app, run, startRunHere, toast } from "../../lib/app.svelte";
  import { sci } from "../../lib/bench.svelte";
  import { FORMATS } from "../../lib/designs";
  import { keep, notHeld, pending, rungOf, rungs, shownRun, switchFormat, unread, type Draft } from "../../lib/device.svelte";
  import { plain } from "../../lib/deviceview";
  import { heldRun } from "../../lib/typing";
  import { shown, stored, unitText } from "../../lib/units";
  import JobEditor from "../job/JobEditor.svelte";
  import RunPlots from "../RunPlots.svelte";
  import UnitChip from "../UnitChip.svelte";

  let {
    info,
    draft,
    designKey,
    values,
    onshow,
  }: {
    info: DeviceInfo;
    draft: Draft;
    designKey: string;
    values: Record<string, number>;
    /** Shows the run on the device in the page's main view. */
    onshow: () => void;
  } = $props();

  const snapshot = (): DeviceSpec => $state.snapshot(draft.spec) as DeviceSpec;
  const ladder = $derived(rungs(info));
  const rung = $derived(rungOf(draft.spec.fidelity, draft.compute.fdfd_2d));
  const how = $derived(draft.compute.fdtd ? `FDTD ${draft.compute.fdtd}D` : rung);
  const method = $derived(ladder.find((r) => r.key === rung)?.method);
  const fdfd = $derived(!draft.compute.fdtd && !!method && /FDFD/.test(method.name));
  const solver = $derived(!!draft.compute.fdtd || draft.spec.fidelity !== "analytic");
  /** A name for the job: the design's, and how it is computed. */
  const jobName = $derived(`${draft.name ?? info.id}-${how.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`.replace(/-+$/, ""));

  // ---- the job the form makes, and its size ----
  /** The page's format for its job texts, the Job tab's too. */
  const format = $derived<JobFormat>(draft.jobFormat ?? "yaml");
  let made = $state<{ key: string; text: string; problem: string; form: JobForm | null }>({ key: "", text: "", problem: "", form: null });
  const madeKey = $derived(JSON.stringify([designKey, draft.compute, jobName, format]));
  $effect(() => {
    const k = madeKey;
    const spec = snapshot();
    const compute = $state.snapshot(draft.compute);
    const [name, f] = [jobName, format];
    const timer = setTimeout(async () => {
      try {
        const yaml = await api.deviceJob(spec, compute, name);
        const form = await api.deviceJobRead(yaml);
        const text = await api.convertJob(yaml, "yaml", f);
        if (k === madeKey) made = { key: k, text, problem: "", form };
      } catch (e) {
        if (k === madeKey) made = { key: k, text: "", problem: String(e), form: null };
      }
    }, 150);
    return () => clearTimeout(timer);
  });

  // ---- the text, as edited ----
  /** The text as typed, read back: what the form can't hold, or why it isn't a job. */
  let typed = $state<JobForm | null>(null);
  let editTimer: ReturnType<typeof setTimeout> | undefined;
  function edited(t: string) {
    // the editor hands back what it was given: not an edit
    if (t === text) return;
    draft.text = t;
    clearTimeout(editTimer);
    editTimer = setTimeout(async () => {
      const form = await api.deviceJobRead(t).catch(unread);
      if (draft.text !== t) return;
      typed = form;
      // the form holds all of it: the form follows, and writes the text again
      if (!notHeld(form, info.kind) && form.spec && form.compute) {
        draft.spec = form.spec;
        draft.compute = form.compute;
        draft.text = null;
        typed = null;
      }
    }, 400);
  }
  async function setFormat(f: JobFormat) {
    try {
      await switchFormat(draft, f);
    } catch (e) {
      toast(`It can't be written in ${f.toUpperCase()} as it is: ${e}`, "error");
    }
  }
  // a text opened from a file, read once
  $effect(() => {
    const t = draft.text;
    if (t !== null && typed === null) api.deviceJobRead(t).then((f) => draft.text === t && (typed = f)).catch(() => {});
  });

  function backToForm() {
    draft.text = null;
    typed = null;
  }

  const text = $derived(draft.text ?? (made.key === madeKey ? made.text : ""));
  /** The form's job is being made again, from a value just typed: Run waits for it rather than refusing the click. */
  const making = $derived(draft.text === null && made.key !== madeKey);
  const textFormat = $derived<JobFormat>(draft.text !== null ? formatOfText(draft.text) : format);
  // the last job's problem stays while the next is made: its line doesn't vanish under the pointer on its way to Run
  const problem = $derived(draft.text !== null ? (typed?.error ?? "") : made.problem);
  const otherDevice = $derived(draft.text !== null && !!typed?.spec && typed.spec.kind !== info.kind ? typed.spec.kind : null);
  /** Why the form can't take the text as typed, when it can't (the problem and another device's said above the buttons). */
  const why = $derived(draft.text !== null && typed && !typed.error && !otherDevice ? notHeld(typed, info.kind) : "");
  const size = $derived(draft.text !== null ? (typed?.size ?? null) : made.key === madeKey ? (made.form?.size ?? null) : null);
  let asText = $state(false);
  // the text, opened, in view: the panel's form is above it
  let box = $state<HTMLElement>();
  $effect(() => box?.scrollIntoView({ block: "nearest" }));

  // ---- running it ----
  let runs = $state<RunItem[]>([]);
  $effect(() => {
    void app.workspaceVersion;
    api
      .home()
      .then((h) => (runs = h.runs.filter((r) => r.device === info.kind)))
      .catch(() => {});
  });
  /** The run started here last: its directory, and the design and rung it computes. */
  let started = $state<{ dir: string; design: string; rung: string } | null>(null);
  const mine = $derived(!!run.info?.dir && run.info.dir === shownRun[info.id]);
  const going = $derived(mine && !run.finished);
  // a run's clock, while it goes
  let now = $state(performance.now());
  $effect(() => {
    if (!going) return;
    const t = setInterval(() => (now = performance.now()), 500);
    return () => clearInterval(t);
  });

  async function open(r: RunItem) {
    if (await startRunHere(() => api.openRun(r.dir))) {
      shownRun[info.id] = r.dir;
      onshow();
    }
  }

  // a click on Run while the job is made from a value just typed: it runs once the job is made
  const runClick = heldRun(() => making, () => start());
  $effect(() => {
    if (!making) runClick.made();
  });

  async function start() {
    if (!text || problem || otherDevice) {
      toast(problem ? plain(problem) : otherDevice ? `This text is a ${otherDevice}'s job: open it on its page.` : "The job isn't ready yet.", "error");
      return;
    }
    const design = designKey;
    const r = draft.compute.fdtd ? `FDTD ${draft.compute.fdtd}D` : rung;
    if (await startRunHere(() => api.runText(text))) {
      started = { dir: run.info?.dir ?? "", design, rung: r };
      shownRun[info.id] = started.dir;
      onshow();
      toast(`Running ${jobName}: it shows here, and keeps going if you leave the page`, "success", undefined, 2500);
    }
  }

  // the header's Run asked for one
  $effect(() => {
    if (pending[info.id] && text && !problem) {
      pending[info.id] = false;
      start();
    }
  });

  // a model's run, finished: its figures are this design's at that rung, for the Model tab
  $effect(() => {
    const d = run.device;
    if (d && run.finished && started && run.info?.dir === started.dir && !started.rung.startsWith("FDTD")) {
      keep(info, started.design, started.rung, { figures: d.figures.map((f) => ({ ...f, fidelity: f.fidelity as "analytic" })), seconds: d.seconds, at: Date.now(), how: "run" });
    }
  });

  async function stop() {
    if (run.info?.dir) await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
  }

  async function save() {
    // in the format the text is written in: the page's, or the one it was typed in
    const f = formatOfText(text);
    try {
      let path: string;
      try {
        path = await api.saveJob(text, f, false);
      } catch (e) {
        if (!String(e).startsWith("exists:")) throw e;
        if (!(await ask(`${String(e).slice("exists:".length).trim().split(/[\\/]/).slice(-2).join("/")} exists. Replace it?`, { title: "Replace the job?", kind: "warning" }))) return;
        path = await api.saveJob(text, f, true);
      }
      app.workspaceVersion++;
      toast(`Saved ${path.split(/[\\/]/).pop()} to jobs/: photonoxide run runs it, and it reopens here`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  // ---- the form's fields ----
  const band = $derived<[number, number, number]>(draft.compute.band ?? [(values.wavelength ?? 1.55) - (values.span ?? 0.1) / 2, (values.wavelength ?? 1.55) + (values.span ?? 0.1) / 2, draft.compute.fdtd ? 21 : 101]);
  function setBand(k: 0 | 1 | 2, v: number) {
    if (!Number.isFinite(v)) return;
    const b: [number, number, number] = [...band];
    b[k] = k === 2 ? Math.round(v) : stored(v);
    draft.compute = { ...draft.compute, band: b };
  }
  type GridKey = "step_nm" | "step_z_nm" | "margin_um" | "samples" | "pml_cells" | "timeout_minutes";
  /** What was typed in each grid field, kept in view when the form holds none for it: the device's own. */
  let typedGrid = $state<Partial<Record<GridKey, string>>>({});
  // another model or FDTD: another grid, and what was typed for the last one goes
  $effect(() => {
    void how;
    typedGrid = {};
  });
  function setNumber(key: GridKey, raw: string) {
    const v = raw.trim() === "" ? null : Number(raw);
    if (v !== null && !Number.isFinite(v)) return;
    typedGrid[key] = raw.trim();
    draft.compute = { ...draft.compute, [key]: key === "samples" || key === "pml_cells" ? (v === null ? null : Math.round(v)) : v };
  }
  /** A grid field's text: the form's value, or, when it holds none (the device's own), what was typed. */
  const gridText = (key: GridKey) => (draft.compute[key] !== null ? val(draft.compute[key]) : (typedGrid[key] ?? ""));
  /** The fields typed in whose value the form holds as the device's own, said under the grid. */
  const ownTyped = $derived((["step_nm", "step_z_nm", "margin_um", "samples", "pml_cells"] as const).filter((k) => draft.compute[k] === null && !!typedGrid[k]));
  function choose(r: string) {
    if (r === "FDTD 2D" || r === "FDTD 3D") {
      draft.compute = { ...draft.compute, fdtd: r === "FDTD 2D" ? 2 : 3, step_z_nm: null, samples: null, fdfd_2d: false };
      return;
    }
    const to = ladder.find((x) => x.key === r);
    if (!to) return;
    draft.spec = { ...draft.spec, fidelity: to.fidelity };
    draft.compute = { ...draft.compute, fdtd: null, fdfd_2d: to.fdfd2d };
  }
  const choices = $derived([...ladder.map((r) => r.key), "FDTD 2D", "FDTD 3D"]);
  const about: Record<string, string> = {
    analytic: "its closed form: no grid, milliseconds",
    "2D": "its 2D model, labelled 2D: an estimate, never the device's performance",
    "2D FDFD": "2D FDFD of its drawn geometry by the effective index method, labelled 2D",
    "3D": "its 3D model: a solve on its grid",
    "FDTD 2D": "its drawn structure stepped in time in the core's plane (the effective index method): a 2D estimate",
    "FDTD 3D": "its drawn structure stepped in time on its stack",
  };
  const val = (v: number | null) => (v === null ? "" : String(v));
</script>

<div class="space-y-5">
  <section>
    <h3 class="panel-title">How</h3>
    <div class="mt-2 flex flex-col gap-0.5">
      {#each choices as c (c)}
        {@const m = ladder.find((r) => r.key === c)?.method}
        <label class="flex cursor-pointer items-start gap-2.5 rounded-lg px-2 py-1.5 hover:bg-base-content/4">
          <input type="radio" class="radio radio-xs radio-primary mt-0.5" name="how-{info.id}" checked={how === c} onchange={() => choose(c)} />
          <span class="min-w-0 flex-1">
            <span class="text-sm font-medium">{c}</span>
            {#if c === rung && !draft.compute.fdtd}<span class="ml-1 text-[11px] text-primary">the design's model</span>{/if}
            <span class="block text-[11px] leading-snug faint">{m?.name ?? about[c]}</span>
          </span>
        </label>
      {/each}
    </div>
    <p class="mt-1 text-[11px] faint">The model chosen here is the design's: it is saved with it, and the Model tab shows the same choice.</p>
  </section>

  <section>
    <h3 class="panel-title">Band</h3>
    <div class="mt-2 flex flex-wrap items-end gap-3">
      <label class="flex flex-col gap-1 text-xs">
        <span class="muted">From (<UnitChip />)</span>
        <span class="input input-xs w-28"><input class="num" type="number" step={shown(0.01)} value={shown(band[0])} onchange={(e) => setBand(0, Number((e.currentTarget as HTMLInputElement).value))} /><span class="faint">{unitText()}</span></span>
      </label>
      <label class="flex flex-col gap-1 text-xs">
        <span class="muted">To (<UnitChip />)</span>
        <span class="input input-xs w-28"><input class="num" type="number" step={shown(0.01)} value={shown(band[1])} onchange={(e) => setBand(1, Number((e.currentTarget as HTMLInputElement).value))} /><span class="faint">{unitText()}</span></span>
      </label>
      <label class="flex flex-col gap-1 text-xs">
        <span class="muted">Points</span>
        <input class="input input-xs w-20 num" type="number" min="1" value={band[2]} onchange={(e) => setBand(2, Number((e.currentTarget as HTMLInputElement).value))} />
      </label>
      {#if draft.compute.band}<button class="btn btn-ghost btn-xs" onclick={() => (draft.compute = { ...draft.compute, band: null })} title="Back to the device's own band: its wavelength ± span/2">its own</button>{/if}
    </div>
  </section>

  <section>
    <h3 class="panel-title">Grid</h3>
    {#if !solver}
      <p class="mt-2 text-sm muted">A closed form: exact, no grid.</p>
    {:else}
      <div class="mt-2 grid grid-cols-2 gap-3">
        <label class="flex flex-col gap-1 text-xs">
          <span class="muted">{draft.compute.fdtd ? "Cells, nm" : fdfd ? "Cells in the plane, nm" : "Mesh step, nm"}</span>
          <input class="input input-xs num" type="number" min="1" placeholder={draft.compute.fdtd ? (draft.compute.fdtd === 3 ? "50" : "40") : "its own"} value={gridText("step_nm")} onchange={(e) => setNumber("step_nm", (e.currentTarget as HTMLInputElement).value)} />
        </label>
        {#if fdfd && draft.spec.fidelity === "3D"}
          <label class="flex flex-col gap-1 text-xs">
            <span class="muted">Cells' height, nm</span>
            <input class="input input-xs num" type="number" min="1" placeholder="its own" value={gridText("step_z_nm")} onchange={(e) => setNumber("step_z_nm", (e.currentTarget as HTMLInputElement).value)} />
          </label>
        {/if}
        {#if !draft.compute.fdtd}
          <label class="flex flex-col gap-1 text-xs">
            <span class="muted">Cladding beside, µm</span>
            <input class="input input-xs num" type="number" min="0" step="0.1" placeholder="its own" value={gridText("margin_um")} onchange={(e) => setNumber("margin_um", (e.currentTarget as HTMLInputElement).value)} />
          </label>
        {/if}
        {#if fdfd}
          <label class="flex flex-col gap-1 text-xs">
            <span class="muted">Wavelengths solved</span>
            <input class="input input-xs num" type="number" min="1" placeholder="5" value={gridText("samples")} onchange={(e) => setNumber("samples", (e.currentTarget as HTMLInputElement).value)} />
          </label>
        {/if}
        {#if fdfd || draft.compute.fdtd}
          <label class="flex flex-col gap-1 text-xs">
            <span class="muted">PML cells</span>
            <input class="input input-xs num" type="number" min="4" placeholder={draft.compute.fdtd ? "20" : "12"} value={gridText("pml_cells")} onchange={(e) => setNumber("pml_cells", (e.currentTarget as HTMLInputElement).value)} />
          </label>
        {/if}
      </div>
      {#if size}
        <p class="mt-2 text-xs num">{size[0]} × {size[1]} cells in the plane, <span class="font-medium">{sci(size[2])}</span> unknowns a solve</p>
      {/if}
      {#if ownTyped.length}
        <p role="status" class="mt-1 text-xs muted">{ownTyped.map((k) => `${typedGrid[k]}${k === "margin_um" ? " µm" : k.endsWith("_nm") ? " nm" : ""}`).join(", ")}: the device's own grid, which the run uses.</p>
      {/if}
      <p class="mt-1 text-[11px] faint">Empty fields are the device's own grid, as its Model tab states it; the view's "As solved" shows these cells.</p>
    {/if}
    <label class="mt-3 flex items-center gap-2 text-xs">
      <span class="muted">Stop it after</span>
      <input class="input input-xs w-20 num" type="number" min="1" placeholder="30" value={gridText("timeout_minutes")} onchange={(e) => setNumber("timeout_minutes", (e.currentTarget as HTMLInputElement).value)} />
      <span class="muted">minutes</span>
    </label>
  </section>

  {#if problem || otherDevice}
    <div role="alert" class="alert alert-warning alert-soft text-sm selectable">
      <TriangleAlert size={16} />
      <span>{otherDevice ? `This text is a ${otherDevice}'s job: open it on its page.` : plain(problem)}</span>
    </div>
  {/if}

  <div class="flex flex-wrap gap-2">
    <button class="btn btn-primary btn-sm gap-1.5" disabled={!making && (!text || !!problem || !!otherDevice)} onclick={() => runClick.click()} title="Run the job: checked by the library, recorded in runs/, shown here live"><Play size={14} /> Run {how}</button>
    <button class="btn btn-sm gap-1.5 {asText ? 'btn-active' : ''}" onclick={() => (asText = !asText)} title="The job file, editable, with the schema's completions, Reference and Problems"><Code size={14} /> As text</button>
    <button class="btn btn-ghost btn-sm gap-1.5" disabled={!text} onclick={save} title="Save the job to the workspace's jobs/ as {jobName}.{textFormat}"><Save size={14} /> Save job</button>
  </div>

  {#if asText}
    <!-- the whole job, as Run runs it and Save writes it, in the shared editor; the design is in the page's view beside it -->
    <section class="flex {text ? 'h-[36rem]' : ''} flex-col overflow-hidden rounded-xl border border-base-content/10" bind:this={box}>
      {#if text}
        <JobEditor {text} format={textFormat} onchange={edited} component="the {info.kind}" sides={["problems", "reference"]} side="problems">
          {#snippet toolbar()}
            <div class="mb-2 flex flex-wrap items-center gap-2">
              <FileText size={14} class="faint" />
              <span class="min-w-0 truncate text-sm font-medium">jobs/{jobName}.{textFormat}</span>
              <div class="join" role="group" aria-label="The format">
                {#each FORMATS as [f, label] (f)}
                  <button class="btn btn-xs join-item {textFormat === f ? 'btn-active' : 'btn-ghost'}" aria-pressed={textFormat === f} onclick={() => setFormat(f)}>{label}</button>
                {/each}
              </div>
              <span class="flex-1"></span>
              {#if draft.text !== null}<button class="btn btn-ghost btn-xs gap-1" onclick={backToForm} title="Drop the edits: the form writes the text again"><RotateCcw size={12} /> Back to the form</button>{/if}
            </div>
            <p class="mb-2 text-[11px] faint">{draft.text === null ? "As the form writes it: type to edit, and the form follows. The Job tab's \"The run\" is this text." : "Edited: the run uses this text."}</p>
            {#if why}<p role="status" class="mb-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-1.5 text-xs selectable">The form can't take this text: {why}.</p>{/if}
          {/snippet}
        </JobEditor>
      {:else}
        <p class="px-3 py-6 text-center text-xs faint">{problem ? "No job: the form's settings make none (above)." : "Making the job…"}</p>
      {/if}
    </section>
  {/if}

  {#if mine}
    <section>
      <div class="flex flex-wrap items-center gap-2">
        <h3 class="panel-title mr-auto">The run</h3>
        {#if going}
          <span class="badge badge-success badge-soft gap-1.5"><span class="status status-success animate-pulse"></span> running · {duration(Math.max(0, (now - run.opened) / 1000))}</span>
          {#if run.stoppable}<button class="btn btn-ghost btn-xs gap-1" onclick={stop} title="Stop the run: what it recorded stays"><Square size={12} /> Stop</button>{/if}
        {:else if run.finished?.stopped}
          <span class="badge badge-warning badge-soft">stopped: {run.finished.stopped}</span>
        {:else if run.finished}
          <span class="badge badge-ghost">finished in {duration(run.finished.seconds)}</span>
        {/if}
        <button class="btn btn-ghost btn-xs gap-1" onclick={onshow} title="Show the run on the device in the main view"><Eye size={12} /> On the device</button>
      </div>
      <p class="mt-1 truncate text-xs faint" title={run.info?.dir ?? ""}>{run.job?.job ?? ""} · {run.info?.name ?? ""}</p>
      {#if going && run.device === null && !run.frames.length}
        <p class="mt-2 text-xs faint">The model is being built{size ? `: ${sci(size[2])} unknowns a solve` : ""}. It goes on in the background if you leave the page; Stop ends it, and what it recorded stays.</p>
      {/if}
      {#if run.device || run.sparams.length || run.frames.length || run.progress}
        <div class="mt-3 [&_.max-w-6xl]:max-w-none [&_.max-w-6xl]:p-0">
          <RunPlots />
        </div>
      {/if}
    </section>
  {/if}

  <section>
    <div class="flex items-center gap-2">
      <h3 class="panel-title mr-auto flex items-center gap-1.5"><History size={13} /> Runs of this device</h3>
      <span class="text-xs faint">{runs.length}</span>
    </div>
    <div class="mt-2 max-h-72 space-y-0.5 overflow-y-auto">
      {#each runs as r (r.dir)}
        <button class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-base-content/4 {run.info?.dir === r.dir ? 'bg-primary/8' : ''}" onclick={() => open(r)} title="Show it here: {r.name}">
          <span class="badge badge-xs badge-soft {r.computes === 'analytic' ? 'badge-secondary' : 'badge-primary'} shrink-0">{r.computes ?? "design"}</span>
          <span class="min-w-0 flex-1 truncate">{r.job}</span>
          <span class="shrink-0 text-[11px] faint">{ago(r.started)}{r.seconds !== null ? ` · ${duration(r.seconds)}` : r.finished ? "" : " · running"}{r.stopped ? ` · ${r.stopped}` : ""}</span>
        </button>
      {:else}
        <p class="px-2 py-3 text-xs faint">None yet: run it, and each run is kept in runs/, replayable.</p>
      {/each}
    </div>
  </section>
</div>
