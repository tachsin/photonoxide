<script lang="ts">
  // The job editor, one for every page (#668): a job's text on the left, with the schema's
  // completions, hover docs and the check's problem in place (TomlEditor's `assist`), and beside
  // it the design it describes, the schema's Reference for the job and its Problems (the library
  // check's and the schema's), each a click from its place in the text. It takes the whole job and
  // the component it belongs to; given a `section` (a top-level key: `device`, `task`), it shows
  // that section alone, with the paths it has in the whole job, and writes an edit back over that
  // section only (lib/jobsection.ts), the rest of the text as it was, to the byte. The check, the
  // Reference and the preview are always of the whole job.
  //
  // Its owner gives the text and hears each edit; it may give the check it runs (the Custom
  // device's, which its header and Run button read) or leave the editor to run the library's
  // own; and it may give its own design beside the text (`preview`), what goes under it
  // (`below`, `previewbar`), a bar over the text (`toolbar`) and one under it (`footer`).
  // Without a `preview`, the design is the job in 3D as its run will draw it, of its last valid text.
  // A page whose own view shows the design (a device's Simulate, Study and Optimize tabs, beside
  // its main view) leaves the preview out (`sides`).
  import { Box, CircleAlert, CircleCheck } from "@lucide/svelte";
  import { untrack, type Snippet } from "svelte";

  import { api, type JobCheck, type JobFormat } from "../../lib/api";
  import { intoSection, messageIn, outOfSection, sectionOf, withSection, type Section } from "../../lib/jobsection";
  import { entryOf, locate, schemaProblems, type Path, type Schema } from "../../lib/jobschema";
  import JobReference from "../custom/JobReference.svelte";
  import MethodDocDialog from "../MethodDocDialog.svelte";
  import ScenePreview from "../ScenePreview.svelte";
  import TomlEditor, { loadSchema } from "../TomlEditor.svelte";

  let {
    text,
    format,
    onchange,
    component,
    section = null,
    check = undefined,
    checking = false,
    layers = [],
    side = $bindable("preview"),
    sides = ["preview", "reference", "problems"],
    oncursor,
    readonly = false,
    refusals = [],
    toolbar,
    preview,
    below,
    previewbar,
    footer,
  }: {
    /** The whole job's text. */
    text: string;
    format: JobFormat;
    /** The whole job's text, edited. */
    onchange: (text: string) => void;
    /** The component the job belongs to, as the page names it: "Custom device", "directional coupler". */
    component: string;
    /** A top-level key of the job to show and edit alone; null: the whole job. */
    section?: string | null;
    /** The owner's check of the whole text; left out, the editor runs the library's own. */
    check?: JobCheck | null;
    checking?: boolean;
    /** The stack's layers' names, offered for a `layer`. */
    layers?: string[];
    /** What is beside the text. */
    side?: "preview" | "reference" | "problems";
    /** The tabs beside the text: a page whose own view shows the design already leaves out the preview. */
    sides?: ("preview" | "reference" | "problems")[];
    /** The user put the cursor at this offset of the whole text. */
    oncursor?: (pos: number) => void;
    /** Shown to read, not to edit (a built-in component's file). */
    readonly?: boolean;
    /** What the library refuses of what the page asks for (a job it won't write), in its words: listed in Problems. */
    refusals?: string[];
    toolbar?: Snippet;
    preview?: Snippet;
    below?: Snippet;
    previewbar?: Snippet;
    footer?: Snippet;
  } = $props();

  let editor = $state<TomlEditor>();
  /** The method page a key's hover opened. */
  let methodPage = $state<string | null>(null);
  let schema = $state<Schema | null>(null);
  loadSchema().then((s) => (schema = s));

  // ---- the check: the owner's, or the library's run here on each edit ----
  const own = $derived(check === undefined);
  let ownCheck = $state<JobCheck | null>(null);
  let ownChecking = $state(false);
  $effect(() => {
    if (!own) return;
    const [t, f] = [text, format];
    ownChecking = true;
    const timer = setTimeout(async () => {
      const c = await api.checkJob(t, f).catch(() => null);
      if (t === text && f === format) {
        ownCheck = c;
        ownChecking = false;
      }
    }, 250);
    return () => clearTimeout(timer);
  });
  const current = $derived(own ? ownCheck : (check ?? null));
  const busy = $derived(own ? ownChecking : checking);

  // ---- the section shown: found again when the text changes from outside ----
  let sec = $state<Section | null>(null);
  /** The whole text as this editor last wrote it: its own edits keep the span they left. */
  let written: string | null = null;
  $effect(() => {
    const [t, f, key] = [text, format, section];
    const s = untrack(() => sec);
    if (t === written && s && s.key === key) return;
    written = null;
    sec = key ? sectionOf(t, f, key) : null;
  });
  const shownText = $derived(sec ? sec.shown : text);

  function edited(t: string) {
    if (!sec) return onchange(t);
    // over the text as last written here, should the owner not have taken it yet
    const r = withSection(written ?? text, sec, t, format);
    sec = r.section;
    written = r.text;
    onchange(r.text);
  }

  /** A span of the whole text, in the editor's text; null outside the section. */
  function inEditor(span: { from: number; to: number }): { from: number; to: number } | null {
    if (!sec) return span;
    const [from, to] = [intoSection(sec, span.from), intoSection(sec, span.to)];
    return from === null || to === null ? null : { from, to };
  }

  /** Selects a span of the whole text, and scrolls it into view. */
  export function select(span: { from: number; to: number }) {
    const s = inEditor(span);
    if (s) editor?.select(s);
  }
  /** Writes `insert` over a span of the whole text: one step, which undo takes back. */
  export function replace(span: { from: number; to: number }, insert: string) {
    const s = inEditor(span);
    if (s) editor?.replace(s, insert);
  }
  /** Undoes the last change, the text's or the sidebar's. */
  export const undoChange = () => editor?.undoChange() ?? false;
  /** Redoes the last change undone. */
  export const redoChange = () => editor?.redoChange() ?? false;

  /** Every problem of the text: the library check's (it stops at its first) and the schema's, each where it is. */
  const problems = $derived.by(() => {
    const out: { message: string; source: "check" | "schema" | "library"; from: number; to: number; line: number }[] = [];
    const put = (message: string, source: "check" | "schema" | "library", at: { from: number; to: number } | null) => {
      const from = at?.from ?? 0;
      out.push({ message, source, from, to: at?.to ?? from, line: at ? text.slice(0, from).split("\n").length : 0 });
    };
    if (current && !current.ok && current.error) put(current.error, "check", locate(text, format, current.error));
    if (schema && current?.model) for (const p of schemaProblems(schema, current.model)) put(p.message, "schema", entryOf(text, format, p.path));
    for (const r of refusals) put(r, "library", null);
    return out;
  });
  /** Shows the key at `p` in the text. */
  function jump(p: Path) {
    const e = entryOf(text, format, p);
    if (e) select(e);
  }
  /** The check's error as the editor underlines it: its line counted in the section shown. */
  const problem = $derived(current && !current.ok ? (sec && current.error ? messageIn(text, sec, current.error) : current.error) : null);

  // ---- the design beside the text, when the owner gives none: the job in 3D, of its last valid text ----
  let drawn = $state("");
  $effect(() => {
    if (preview || !sides.includes("preview") || !current?.ok) return;
    // on each check that passes, not on each key
    const [t, f] = untrack(() => [text, format] as const);
    let live = true;
    (f === "toml" ? Promise.resolve(t) : api.convertJob(t, f, "toml"))
      .then((x) => {
        if (live) drawn = x;
      })
      .catch(() => {});
    return () => (live = false);
  });
  let width = $state(0);
  const stacked = $derived(width > 0 && width < 1000);
  let height = $state(0);
  /** The tab beside the text shown: the one asked for, if the page offers it. */
  const beside = $derived(sides.includes(side) ? side : sides[0]);
  const labels = { preview: "Preview", reference: "Reference", problems: "Problems" } as const;
</script>

<div class="grid min-h-0 flex-1 {stacked ? 'grid-rows-[minmax(13rem,3fr)_minmax(10rem,2fr)]' : 'grid-cols-[minmax(0,1fr)_minmax(24rem,38%)]'}" bind:clientWidth={width}>
  <!-- the text -->
  <section class="flex min-h-0 min-w-0 flex-col border-base-content/8 {stacked ? 'border-b' : 'border-r'}">
    <div class="min-h-0 flex-1 overflow-hidden">
      <div class="flex h-full min-h-0 flex-col p-3">
        {@render toolbar?.()}
        {#if section}
          <p class="mb-2 text-[11px] faint">
            {#if sec}The <code>{section}</code> section of {component}'s job: an edit here changes it alone, the rest of the file stays as it is.{:else}This job has no <code>{section}</code> section in one piece: the whole job is shown.{/if}
          </p>
        {/if}
        <div class="min-h-0 flex-1 overflow-hidden rounded-xl border border-base-content/8 bg-base-300/50 py-2">
          <TomlEditor bind:this={editor} value={shownText} onchange={edited} oncursor={(pos) => oncursor?.(sec ? outOfSection(sec, pos) : pos)} {format} assist model={current?.model} {problem} {layers} {readonly} onmethod={(f) => (methodPage = f)} />
        </div>
        <div class="mt-2 shrink-0">
          {#if current && !current.ok && current.error}
            <!-- at most a few lines, so the text keeps its room in a narrow panel: the whole message is in Problems too -->
            <div class="max-h-20 overflow-y-auto rounded-lg border border-error/30 bg-error/8 px-3 py-2 text-xs whitespace-pre-wrap text-error selectable">{current.error}</div>
          {/if}
        </div>
        <MethodDocDialog file={methodPage} onclose={() => (methodPage = null)} />
      </div>
    </div>
    {@render footer?.()}
  </section>

  <!-- beside the text: the design it describes, the schema's Reference, its Problems -->
  <aside class="flex min-h-0 min-w-0 flex-col">
    <div class="flex shrink-0 items-center gap-1 border-b border-base-content/8 px-3 py-1.5" role="tablist" aria-label="Beside the text">
      {#each sides as s (s)}
        <button role="tab" aria-selected={beside === s} class="btn btn-ghost btn-xs gap-1.5 {beside === s ? 'btn-active' : ''}" onclick={() => (side = s)}>
          {labels[s]}
          {#if s === "problems" && problems.length}<span class="badge badge-xs badge-error">{problems.length}</span>{/if}
        </button>
      {/each}
    </div>
    {#if beside === "reference"}
      <div class="min-h-0 flex-1 overflow-hidden"><JobReference {schema} model={current?.model ?? null} onjump={jump} /></div>
    {:else if beside === "problems"}
      <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2">
        {#if busy && !current}
          <p class="text-xs faint">Checking.</p>
        {:else if !problems.length}
          <p class="flex items-center gap-1.5 text-xs text-success"><CircleCheck size={13} /> No problems: the library's check passes, and the schema finds nothing.</p>
        {:else}
          <ul class="space-y-1.5">
            {#each problems as p, k (k)}
              {@const outside = !!sec && p.source !== "library" && !inEditor(p)}
              <li>
                <button class="flex w-full items-start gap-2 rounded-lg px-2 py-1.5 text-left text-xs hover:bg-base-content/5" onclick={() => select(p)} title={outside ? `In another section than ${section}` : "Show it in the text"}>
                  <CircleAlert size={13} class="mt-0.5 shrink-0 text-error" />
                  <span class="min-w-0 flex-1 break-words whitespace-pre-wrap selectable">{p.message}</span>
                  <span class="shrink-0 text-[10.5px] faint">{p.source === "check" ? "the check" : p.source === "schema" ? "the schema" : "the library, of the run asked for"}{p.line ? `, line ${p.line}` : ""}{outside ? ", outside this section" : ""}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else if preview}
      {@render preview()}
    {:else}
      <div class="relative min-h-0 flex-1 overflow-hidden" bind:clientHeight={height}>
        {#if drawn}
          <ScenePreview text={drawn} height={Math.max(160, height)} turnable own axes />
          {#if current && !current.ok}
            <div class="absolute inset-x-3 top-3 rounded-lg border border-warning/30 bg-base-100/85 px-3 py-1.5 text-xs backdrop-blur">The job isn't valid now: this is its last valid version.</div>
          {/if}
        {:else}
          <div class="grid h-full place-items-center p-6 text-center">
            <span class="flex flex-col items-center gap-3 text-sm faint"><Box size={22} /> The 3D view appears once the job is valid{current && !current.ok ? "; it isn't yet (see the text)" : ""}.</span>
          </div>
        {/if}
      </div>
    {/if}
    {@render below?.()}
    {#if beside === "preview"}{@render previewbar?.()}{/if}
  </aside>
</div>
