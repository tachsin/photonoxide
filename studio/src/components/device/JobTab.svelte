<script lang="ts">
  // A device's Job tab (#668): the page as its job's text in the shared editor (JobEditor), in the
  // page's panel like every tab, with the schema's completions, its Reference and the check's
  // problems; the design is in the page's view beside it. One text, the whole job the Simulate tab
  // runs: its `device` section (the design, every field written out, as Save writes it) and its
  // `task` (how it runs), the one text both tabs show and edit (`draft.text` while edited; #668
  // decision 2). The page and the text agree both ways: an edit of the page writes the text again,
  // and an edit of the text that the page can hold moves the page, the design and the run alike.
  // A text the page can't hold (one that doesn't read, a design of another device) is kept as
  // typed, and the tab says why. Where the library refuses to write the run (one too big for its
  // grid), its reason is listed in Problems and the design's file is shown alone, to edit.
  import { FileCode2, RotateCcw } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, formatOfText, type DeviceField, type DeviceInfo, type DeviceSpec, type JobForm, type JobFormat } from "../../lib/api";
  import { toast } from "../../lib/app.svelte";
  import { openBuiltIn, openInDesign } from "../../lib/designpage.svelte";
  import { FORMATS, inOrder, sameParameters } from "../../lib/designs";
  import { dirty, notHeld, rungOf, switchFormat, unread, type Draft } from "../../lib/device.svelte";
  import JobEditor from "../job/JobEditor.svelte";

  let { info, draft, fields }: { info: DeviceInfo; draft: Draft; fields: DeviceField[] } = $props();

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
  // every field written out, as in the design's file: the text says every number
  const runKey = $derived(JSON.stringify([shown, draft.compute, runName, format]));
  $effect(() => {
    const k = runKey;
    const [spec, compute, n, f] = [$state.snapshot(shown) as DeviceSpec, $state.snapshot(draft.compute), runName, format];
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
  /** The library won't write the run the Simulate tab sets: there is no whole job, and the design's file is shown instead. */
  const refused = $derived(draft.text === null && ran.key === runKey && !!ran.refused);
  /**
   * One text, the whole job: the design (its `device` section) and how it runs (its `task`), as
   * the Simulate tab runs it. The design's file alone only where there is no whole job: the
   * library won't write the run, or the design's file was being typed in when that happened.
   */
  const mode = $derived<"design" | "run">(refused || draft.jobText !== null ? "design" : "run");
  const text = $derived(mode === "design" ? designText : runText);
  const textFormat = $derived<JobFormat>(mode === "run" && draft.text !== null ? formatOfText(draft.text) : format);
  const refusals = $derived(refused ? [`The run the Simulate tab sets isn't written: ${ran.refused}`] : []);

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
    const form: JobForm = await api.deviceJobRead(t).catch(unread);
    if (draft.text !== t) return;
    why = notHeld(form, info.kind);
    // the form holds all of it: the page follows, and writes the text again
    if (!why && form.spec && form.compute) {
      // the design the page shows, every field written out: nothing to move
      const s = form.spec;
      const same = s.fidelity === shown.fidelity && JSON.stringify(s.stack) === JSON.stringify(shown.stack) && sameParameters(s.parameters, shown.parameters) && (s.compact ?? null) === (shown.compact ?? null);
      if (!same) {
        fromText = JSON.stringify(s);
        draft.spec = s;
      }
      if (JSON.stringify(form.compute) !== JSON.stringify(draft.compute)) draft.compute = form.compute;
      draft.text = null;
    }
  }

  /** Back to the text the page writes. */
  function backToPage() {
    if (mode === "design") draft.jobText = null;
    else draft.text = null;
    why = "";
  }

  async function setFormat(f: JobFormat) {
    try {
      await switchFormat(draft, f);
    } catch (e) {
      toast(`It can't be written in ${f.toUpperCase()} as it is: ${e}`, "error");
    }
  }

  /** The design's file on the Design page: a built-in component's own at its defaults, the text shown otherwise. */
  async function openAsJob() {
    if (!draft.name && !dirty(draft, info) && draft.jobText == null && (await openBuiltIn(info.id))) return;
    openInDesign({ text: designText, format, path: draft.path, builtIn: false, saved: draft.path && !dirty(draft, info) ? designText : "" });
  }

  const edits = $derived(mode === "design" ? draft.jobText != null : draft.text !== null);
</script>

<JobEditor {text} format={textFormat} onchange={edited} component="the {info.kind}" section={mode === "design" ? "device" : null} {refusals} sides={["problems", "reference"]} side="problems">
  {#snippet toolbar()}
    <div class="mb-2 flex flex-wrap items-center gap-2">
      <div class="join" role="group" aria-label="The format">
        {#each FORMATS as [f, label] (f)}
          <button class="btn btn-xs join-item {format === f ? 'btn-active' : 'btn-ghost'}" aria-pressed={format === f} onclick={() => setFormat(f)}>{label}</button>
        {/each}
      </div>
      <span class="text-[11px] faint">{edits ? "edited here" : refused ? "the design alone: the library won't write its run" : "the design and how it runs, as the page writes them: type to edit, the page follows"}</span>
      <span class="flex-1"></span>
      {#if edits}<button class="btn btn-ghost btn-xs gap-1" onclick={backToPage} title="Drop the edits the page can't hold: the page writes the text again"><RotateCcw size={12} /> Back to the page</button>{/if}
      <button class="btn btn-ghost btn-xs gap-1" onclick={openAsJob} title="Open the design's file on the Design page: a built-in component's own file at its defaults, to read and to save a copy of"><FileCode2 size={12} /> Open as a job</button>
    </div>
    {#if refused}<p role="status" class="mb-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-1.5 text-xs">The library won't write the run the Simulate tab sets (its reason is in Problems), so the design's file is shown alone, to edit. Change how it runs on the Simulate tab, and the whole job comes back.</p>{/if}
    {#if why}<p role="status" class="mb-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-1.5 text-xs selectable">The page can't take this text: {why}. It is kept as typed.</p>{/if}
  {/snippet}
</JobEditor>
