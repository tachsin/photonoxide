<script lang="ts">
  // A device's Job tab (#668): the page as its job's text in the shared editor (JobEditor), the
  // text on the left with the schema's completions and the check's problems, the design in 3D
  // beside it. "The design" is the `device` section of the design's file, as Save writes it; "The
  // run" is the `task` section of the job the Simulate tab runs. The page and the text agree both
  // ways: an edit of the page writes the text again, and an edit of the text that the page can
  // hold moves the page. A text the page can't hold (one that doesn't read, a design of another
  // device) is kept as typed, and the tab says why. What the library refuses to write (a run too
  // big for its grid) is listed in Problems, in its words.
  import { FileCode2, RotateCcw } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, formatOfText, type DeviceField, type DeviceInfo, type DeviceSpec, type JobForm, type JobFormat } from "../../lib/api";
  import { toast } from "../../lib/app.svelte";
  import { openBuiltIn, openInDesign } from "../../lib/designpage.svelte";
  import { FORMATS, inOrder, sameParameters } from "../../lib/designs";
  import { dirty, rungOf, type Draft } from "../../lib/device.svelte";
  import type { Scene } from "../../lib/events";
  import JobEditor from "../job/JobEditor.svelte";
  import Device3D from "./Device3D.svelte";

  let { info, draft, scene, fields }: { info: DeviceInfo; draft: Draft; scene: Scene | null; fields: DeviceField[] } = $props();

  /** The design's file, or the job the Simulate tab runs. */
  let mode = $state<"design" | "run">("design");
  const format = $derived<JobFormat>(draft.jobFormat ?? "yaml");
  const name = $derived(draft.name ?? info.id);

  /**
   * The design as the form shows it: every field, a parameter left at its default written out
   * with its default, so that the text says every number (at its defaults, the device section of
   * its built-in file in jobs/components/).
   */
  const shown = $derived.by((): DeviceSpec => {
    const given = draft.spec.parameters;
    if (!fields.length) return draft.spec;
    const all: [string, string][] = fields.map((f) => [f.name, given.find(([n]) => n === f.name)?.[1] ?? f.expression]);
    for (const p of given) if (!all.some(([n]) => n === p[0])) all.push(p);
    return { ...draft.spec, parameters: inOrder(info, all) };
  });

  // ---- the design's file, as the page writes it ----
  let made = $state({ key: "", text: "" });
  const madeKey = $derived(JSON.stringify([shown, name, draft.about, format]));
  $effect(() => {
    const k = madeKey;
    const [spec, n, about, f] = [$state.snapshot(shown) as DeviceSpec, name, draft.about, format];
    let live = true;
    api
      .designText(spec, n, about, f)
      .then((text) => {
        if (live) made = { key: k, text };
      })
      .catch(() => {});
    return () => (live = false);
  });

  // ---- the run's job, as the Simulate tab writes it ----
  const how = $derived(draft.compute.fdtd ? `FDTD ${draft.compute.fdtd}D` : rungOf(draft.spec.fidelity, draft.compute.fdfd_2d));
  const runName = $derived(`${name}-${how.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`.replace(/-+$/, ""));
  let ran = $state({ key: "", text: "", refused: "" });
  const runKey = $derived(JSON.stringify([draft.spec, draft.compute, runName, format]));
  $effect(() => {
    if (mode !== "run") return;
    const k = runKey;
    const [spec, compute, n, f] = [$state.snapshot(draft.spec), $state.snapshot(draft.compute), runName, format];
    let live = true;
    api
      .deviceJob(spec, compute, n)
      .then((yaml) => api.convertJob(yaml, "yaml", f))
      .then((text) => {
        if (live) ran = { key: k, text, refused: "" };
      })
      .catch((e) => {
        if (live) ran = { key: k, text: "", refused: String(e) };
      });
    return () => (live = false);
  });

  // ---- the text shown: as typed while the page can't hold it, or as the page writes it ----
  /** Why the page can't take the text as typed, when it can't. */
  let why = $state("");
  /** The design the text last gave the page: a change of the page's design that isn't it is the page's own. */
  let fromText = JSON.stringify(untrack(() => draft.spec));
  $effect(() => {
    const now = JSON.stringify(draft.spec);
    untrack(() => {
      if (now === fromText) return;
      // the page's own edit: the text follows it
      fromText = now;
      draft.jobText = null;
      why = "";
    });
  });
  /** The text as typed, or as the page last wrote it (the one before, while the next is on its way). */
  const designText = $derived(draft.jobText ?? made.text);
  const runText = $derived(draft.text ?? (ran.key === runKey && !ran.refused ? ran.text : designText));
  const text = $derived(mode === "design" ? designText : runText);
  const textFormat = $derived<JobFormat>(mode === "run" && draft.text !== null ? formatOfText(draft.text) : format);
  const refusals = $derived(mode === "run" && draft.text === null && ran.key === runKey && ran.refused ? [`The run the Simulate tab sets isn't written: ${ran.refused}`] : []);

  let timer: ReturnType<typeof setTimeout> | undefined;
  function edited(t: string) {
    // the editor hands back what it was given: not an edit
    if (t === text) return;
    clearTimeout(timer);
    if (mode === "design") {
      draft.jobText = t;
      timer = setTimeout(() => readDesign(t), 300);
    } else {
      draft.text = t;
      timer = setTimeout(() => readRun(t), 300);
    }
  }

  async function readDesign(t: string) {
    const r = await api.designRead(t, format).catch((e) => ({ name: "", about: "", spec: null, beyond: [], error: String(e) }));
    if (draft.jobText !== t) return;
    if (!r.spec) {
      why = r.error ?? "it isn't a design";
      return;
    }
    if (r.spec.kind !== info.kind) {
      why = `it is a design of the ${r.spec.kind}, not of the ${info.kind}: open it on the Design page`;
      return;
    }
    if (r.beyond.length) {
      why = `the page holds a design alone, not ${r.beyond.join(", ")}`;
      return;
    }
    // the page follows; the check's problem, if any, is the editor's to show
    why = "";
    // the design the page shows, every field written out: nothing to move
    const same = r.spec.fidelity === shown.fidelity && JSON.stringify(r.spec.stack) === JSON.stringify(shown.stack) && sameParameters(r.spec.parameters, shown.parameters) && (r.spec.compact ?? null) === (shown.compact ?? null);
    if (!same) {
      // the text's design: the page takes it, and the text stays as typed
      fromText = JSON.stringify(r.spec);
      draft.spec = r.spec;
    }
  }

  async function readRun(t: string) {
    const form: JobForm = await api.deviceJobRead(t).catch((e) => ({ name: "", spec: null, compute: null, beyond: [], error: String(e), size: null, study: null, optimize: null }));
    if (draft.text !== t) return;
    // the form holds all of it: the page follows, and writes the text again
    if (!form.error && !form.beyond.length && form.spec && form.compute && form.spec.kind === info.kind && !form.study && !form.optimize) {
      why = "";
      draft.spec = form.spec;
      draft.compute = form.compute;
      draft.text = null;
    } else why = form.error ?? (form.beyond.length ? `the Simulate tab's form can't hold ${form.beyond.join(", ")}: the run uses the text as it is` : "");
  }

  /** Back to the text the page writes. */
  function backToPage() {
    if (mode === "design") draft.jobText = null;
    else draft.text = null;
    why = "";
  }

  async function setFormat(f: JobFormat) {
    if (f === format) return;
    try {
      if (draft.jobText != null) draft.jobText = await api.convertJob(draft.jobText, format, f);
      draft.jobFormat = f;
    } catch (e) {
      toast(`It can't be written in ${f.toUpperCase()} as it is: ${e}`, "error");
    }
  }

  /** The design's file on the Design page: a built-in component's own at its defaults, the text shown otherwise. */
  async function openAsJob() {
    if (!draft.name && !dirty(draft, info) && draft.jobText == null && (await openBuiltIn(info.id))) return;
    openInDesign({ text: designText, format, path: draft.path, builtIn: false, saved: draft.path && !dirty(draft, info) ? designText : "" });
  }

  let height = $state(0);
  const edits = $derived(mode === "design" ? draft.jobText != null : draft.text !== null);
</script>

<JobEditor {text} format={textFormat} onchange={edited} component="the {info.kind}" section={mode === "design" ? "device" : "task"} {refusals}>
  {#snippet toolbar()}
    <div class="mb-2 flex flex-wrap items-center gap-2">
      <div class="join" role="group" aria-label="The job shown">
        <button class="btn btn-xs join-item {mode === 'design' ? 'btn-primary btn-soft' : ''}" aria-pressed={mode === "design"} onclick={() => ((mode = "design"), (why = ""))} title="The design's file, as Save writes it: its device section">The design</button>
        <button class="btn btn-xs join-item {mode === 'run' ? 'btn-primary btn-soft' : ''}" aria-pressed={mode === "run"} onclick={() => ((mode = "run"), (why = ""))} title="The job the Simulate tab runs: its task section">The run</button>
      </div>
      <div class="join" role="group" aria-label="The format">
        {#each FORMATS as [f, label] (f)}
          <button class="btn btn-xs join-item {format === f ? 'btn-active' : 'btn-ghost'}" aria-pressed={format === f} onclick={() => setFormat(f)}>{label}</button>
        {/each}
      </div>
      <span class="text-[11px] faint">{edits ? "edited here" : "as the page writes it: type to edit, the page follows"}</span>
      <span class="flex-1"></span>
      {#if edits}<button class="btn btn-ghost btn-xs gap-1" onclick={backToPage} title="Drop the edits the page can't hold: the page writes the text again"><RotateCcw size={12} /> Back to the page</button>{/if}
      <button class="btn btn-ghost btn-xs gap-1" onclick={openAsJob} title="Open the design's file on the Design page: a built-in component's own file at its defaults, to read and to save a copy of"><FileCode2 size={12} /> Open as a job</button>
    </div>
    {#if why}<p role="status" class="mb-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-1.5 text-xs selectable">The page can't take this text: {why}. It is kept as typed.</p>{/if}
  {/snippet}
  {#snippet preview()}
    <div class="relative min-h-0 flex-1 overflow-hidden" bind:clientHeight={height}>
      {#if scene}
        <Device3D {scene} height={Math.max(160, height - 8)} />
      {:else}
        <div class="skeleton m-3 h-[calc(100%-1.5rem)]"></div>
      {/if}
    </div>
  {/snippet}
</JobEditor>
