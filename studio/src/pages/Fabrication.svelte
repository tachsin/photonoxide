<script lang="ts">
  // The Fabrication page (#622): how a design is made, from the bare wafer, step by step. A design
  // picked (the one open on its page, any device of the library at its defaults, a template, or a
  // job file with a [device] or a structure task), the platform it is made on, and the flow
  // photonoxide::fabrication's rules give it: each step with its reason, its numbers, each
  // number's source or "not specified", and what the rules flag. The wafer at each moment is drawn
  // as a cross-section (2D) and as a chip (3D), from the same frames, each film in one colour in
  // both; one timeline plays them, a lithography as its resist spun on, the pattern written and
  // the resist developed. The first frame is the bare wafer; the last is the design, and the
  // round trip (the flow run on the library's geometry against the design) says it is to the bit.
  // A design on silicon on insulator can start from a bought SOI wafer or from bare silicon, the
  // SOI wafer made first by Smart Cut or SIMOX: as the design's job says (`start: smart_cut`,
  // `start: simox`), as the
  // component's Fabricate button sent it, or as picked here.
  import { open } from "@tauri-apps/plugin-dialog";
  import { CircleCheck, FileUp, Layers, Play, TriangleAlert } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import FabPlayer from "../components/fabrication/FabPlayer.svelte";
  import LithoTool from "../components/fabrication/LithoTool.svelte";
  import StartChoice from "../components/fabrication/StartChoice.svelte";
  import Hint from "../components/Hint.svelte";
  import { api, formatOf, formatOfText, type FabDesign, type FabricationPage, type FabStart } from "../lib/api";
  import { app, CUSTOM, startRun } from "../lib/app.svelte";
  import { catalog, loadCatalog } from "../lib/catalog.svelte";
  import { deviceOf, devices, drafts, loadDevices } from "../lib/device.svelte";
  import { plain } from "../lib/deviceview";
  import type { FlowSource } from "../lib/events";
  import { cited, num, PHASE, quantity, RULE, STEP, value, VERDICT } from "../lib/flow";

  /** A design the picker offers. */
  interface Choice {
    id: string;
    group: string;
    label: string;
    design: FabDesign;
  }

  const capital = (s: string) => (s ? s[0].toUpperCase() + s.slice(1) : s);
  /** A device of the library at its defaults, on its own stack, as a job with only its kind draws it. */
  const libraryDesign = (kind: string): FabDesign => ({ kind: "job", text: `name = "fabrication"\n\n[device]\nkind = ${JSON.stringify(kind)}\n`, format: "toml", path: null });

  /** The design open elsewhere: the one sent here, else the component shown on Components (its page's design). */
  function current(): Choice | null {
    if (app.fabricate) return { id: "sent", group: "Open", label: app.fabricate.label, design: app.fabricate.design };
    const d = deviceOf(app.component);
    const draft = d ? drafts[d.id] : undefined;
    if (d && draft) return { id: `page:${d.id}`, group: "Open", label: `${capital(d.kind)}, as on its page`, design: { kind: "device", spec: $state.snapshot(draft.spec) } };
    if (app.component === CUSTOM && app.customDraft) {
      const c = app.customDraft;
      return { id: "custom", group: "Open", label: `The Custom device's job${c.path ? ` (${c.path.split(/[\\/]/).pop()})` : ""}`, design: { kind: "job", text: c.text, format: c.format, path: c.path } };
    }
    return null;
  }

  /** A file opened here, kept in the picker. */
  let opened = $state<Choice | null>(null);
  let here = $state<Choice | null>(null);
  const choices = $derived.by((): Choice[] => {
    const out: Choice[] = [];
    if (here) out.push(here);
    if (opened) out.push(opened);
    for (const d of devices.list) out.push({ id: `lib:${d.id}`, group: "The library's devices, at their defaults", label: capital(d.kind), design: libraryDesign(d.kind) });
    for (const j of catalog.data?.jobs ?? []) {
      if (j.fabricates) {
        out.push({ id: `job:${j.file}`, group: "Templates", label: j.name, design: { kind: "job", text: j.text, format: formatOfText(j.text), path: null } });
      }
    }
    return out;
  });
  const groups = $derived([...new Set(choices.map((c) => c.group))]);
  let chosenId = $state("");
  const chosen = $derived(choices.find((c) => c.id === chosenId) ?? null);

  /** Process ids to prefer where several qualify: the alternatives picked with "Use instead". */
  let prefer = $state<string[]>([]);
  /** Where the flow starts, when picked (or sent with the design); null: as the design says. */
  let start = $state<FabStart | null>(null);
  let page = $state<FabricationPage | null>(null);
  let problem = $state("");
  let busy = $state(false);

  onMount(() => {
    loadDevices();
    loadCatalog();
  });

  // opens on the design open elsewhere, and on one sent here while the page is shown
  $effect(() => {
    void app.fabricate;
    untrack(() => {
      here = current();
      if (here) {
        chosenId = here.id;
        start = here.id === "sent" ? (app.fabricate?.start ?? null) : null;
      }
    });
  });
  $effect(() => {
    if (!chosenId && devices.list.length) untrack(() => (chosenId = `lib:${devices.list[0].id}`));
  });

  $effect(() => {
    const c = chosen;
    const p = [...prefer];
    const from = start;
    if (!c) return;
    let live = true;
    busy = true;
    api
      .fabricationPage($state.snapshot(c.design) as FabDesign, p, [], from)
      .then((got) => {
        if (!live) return;
        page = got;
        shownDesign = c.id;
        problem = "";
      })
      .catch((e) => {
        if (live) problem = String(e);
      })
      .finally(() => {
        if (live) busy = false;
      });
    return () => {
      live = false;
    };
  });

  function pick(id: string) {
    prefer = [];
    start = null;
    chosenId = id;
  }

  /** A new start: the steps change, so the player goes back to the first frame. */
  function pickStart(s: FabStart) {
    start = s;
    frame = 0;
  }

  async function openFile() {
    const path = await open({ title: "Open a job file to fabricate", multiple: false, directory: false, defaultPath: app.state?.workspace, filters: [{ name: "Jobs", extensions: ["yaml", "yml", "toml", "json"] }] });
    if (typeof path !== "string") return;
    try {
      const text = await api.readJob(path);
      opened = { id: `file:${path}`, group: "Opened", label: path.split(/[\\/]/).pop() ?? path, design: { kind: "job", text, format: formatOf(path), path } };
      pick(opened.id);
    } catch (e) {
      problem = String(e);
    }
  }

  // ---- the frame shown: the player's (FabPlayer: the 2D and 3D views and their timeline) ----

  /** The design whose flow is shown, by its choice's id. */
  let shownDesign = $state("");
  let frame = $state(0);
  let player: FabPlayer | undefined = $state();
  const frames = $derived(page?.view.frames ?? []);

  // ---- the step shown ----

  const shownFrame = $derived(frames[frame] ?? null);
  const stepNow = $derived(shownFrame?.step ?? 0);
  const record = $derived(page?.view.flow.steps[stepNow] ?? null);
  /** Each step with its frames, in order. */
  const steps = $derived.by(() => {
    if (!page) return [];
    return page.view.flow.steps.map((s, k) => ({ s, k, frames: frames.map((f, i) => ({ f, i })).filter((x) => x.f.step === k) }));
  });
  /** The step list: where it scrolls on its own (a wide window), the step shown is kept in sight. */
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void stepNow;
    const el = list?.querySelector<HTMLElement>('[aria-current="step"]');
    if (!list || !el || list.scrollHeight <= list.clientHeight) return;
    // where the step is in the list's scrolled content
    const top = el.getBoundingClientRect().top - list.getBoundingClientRect().top + list.scrollTop;
    if (top < list.scrollTop) list.scrollTop = top - 8;
    else if (top + el.offsetHeight > list.scrollTop + list.clientHeight) list.scrollTop = top + el.offsetHeight - list.clientHeight + 8;
  });
  /** Each kind of step in a colour of its own, in the steps' list. */
  const TYPE_TONE: Record<string, string> = {
    wafer: "bg-slate-500/15 text-slate-500",
    deposition: "bg-sky-500/15 text-sky-500",
    lithography: "bg-violet-500/15 text-violet-500",
    pattern_transfer: "bg-amber-500/15 text-amber-500",
    planarization: "bg-teal-500/15 text-teal-500",
    anneal: "bg-orange-500/15 text-orange-500",
    implant: "bg-pink-500/15 text-pink-500",
    bond: "bg-emerald-500/15 text-emerald-500",
    split: "bg-rose-500/15 text-rose-500",
  };
  /** A step's short phrase as a sentence: its first letter capital. */
  const sentence = (s: string) => (s ? s[0].toUpperCase() + s.slice(1) : s);
  const checksOf = (k: number) => page?.view.assessment.checks.filter((c) => c.step === k) ?? [];
  const flagged = (k: number) => checksOf(k).filter((c) => c.verdict !== "within").length;
  const ranked = $derived(page?.view.assessment.ranking.find((r) => r.step === stepNow)?.ranked ?? []);
  const source = (s: FlowSource) => cited(page?.view.flow.references ?? [], s);
  /** The glossary's words the step's narration uses. */
  const words = $derived.by(() => {
    if (!page || !record) return [];
    const text = `${record.plain} ${record.does} ${record.reason}`.toLowerCase();
    return page.view.flow.glossary.filter((g) => text.includes(g.term.toLowerCase()));
  });
  const trip = $derived(page?.view.flow.round_trip ?? null);
  const exact = $derived(!!trip && trip.structure_equal && trip.films_equal && trip.differing === 0);

  /** The frame's layers, from the top down, the substrate last: an etched layer's bands joined. */
  const legend = $derived.by(() => {
    const scene = shownFrame?.scene;
    if (!scene) return [];
    const rows: { name: string; film: string; around: string; um: number | null }[] = [];
    const z: [number, number][] = [];
    for (const l of scene.layers) {
      const base = l.name.replace(/ \d+$/, "");
      const k = rows.findIndex((r) => r.name === base && base !== l.name);
      if (k >= 0) {
        z[k] = [Math.min(z[k][0], l.z_um[0]), Math.max(z[k][1], l.z_um[1])];
        if (l.background.material !== l.material.material) rows[k].around = l.background.material;
      } else {
        rows.push({ name: l.name, film: l.material.material, around: l.background.material, um: null });
        z.push([l.z_um[0], l.z_um[1]]);
      }
    }
    rows.forEach((r, k) => (r.um = z[k][1] - z[k][0]));
    // the films over the topography (the resist, a conformal film), on top: each once, by its layer and film
    const over: typeof rows = [];
    for (const f of shownFrame?.films ?? []) {
      if (!over.some((r) => r.name === f.layer && r.film === f.film)) over.push({ name: f.layer, film: f.film, around: f.film, um: null });
    }
    const out = [...over, ...rows.reverse(), { name: "substrate", film: scene.substrate.material, around: scene.substrate.material, um: null }];
    if (scene.cladding.material !== "air") out.unshift({ name: "cladding", film: scene.cladding.material, around: scene.cladding.material, um: null });
    return out;
  });
  const colour = (film: string) => (film === "air" ? "transparent" : (page?.colours[film] ?? "#b0b0b0"));

  function useInstead(process: string) {
    prefer = [...prefer.filter((p) => p !== process), process];
  }

  /** The parameters of the step shown, those it has. */
  const parameters = $derived.by(() => {
    const r = record;
    if (!r) return [];
    const out: [string, string][] = [["process", `${r.name} (${r.process})`]];
    if (r.layer) out.push(["layer", r.layer]);
    if (r.film) out.push(["film", r.film]);
    // (a cladding has none: it is above everything)
    if (r.thickness_um) out.push(["thickness", `${num(r.thickness_um)} µm`]);
    if (r.depth_um !== null) out.push(["depth", `${num(r.depth_um)} µm${r.timed ? ", timed" : ", to the film below"}`]);
    if (r.sidewall_deg !== null) out.push(["sidewall", `${num(r.sidewall_deg)}° from the plane`]);
    if (r.tone) out.push(["resist", r.tone === "keep" ? "kept on the shapes (an etch mask)" : "opened over the shapes (a stencil)"]);
    return out;
  });

  // ---- the step's details, a tab at a time ----

  type Tab = "overview" | "machine" | "numbers" | "checks" | "alternatives" | "terms";
  let tab = $state<Tab>("overview");
  /** Whether the step shown is a lithography: its machine can be shown. */
  const lithography = $derived(record?.step === "lithography");
  /** The tab shown: the machine only for a lithography. */
  const tabShown = $derived(tab === "machine" && !lithography ? "overview" : tab);
  /** The shapes the lithography patterns, as the finished design has them. */
  const maskShapes = $derived.by(() => {
    const last = frames[frames.length - 1]?.scene;
    const l = record?.layer;
    return last && l ? last.shapes.filter((s) => s.layer === l) : [];
  });
  const tabs = $derived.by((): { id: Tab; label: string; count: number | null; flag?: boolean }[] => [
    { id: "overview", label: "Overview", count: null },
    ...(lithography ? [{ id: "machine" as Tab, label: "Machine", count: null }] : []),
    { id: "numbers", label: "Numbers", count: parameters.length + (record?.capabilities.length ?? 0) },
    { id: "checks", label: "Checks", count: checksOf(stepNow).length + (record?.uncertainties.length ?? 0), flag: flagged(stepNow) > 0 },
    { id: "alternatives", label: "Alternatives", count: ranked.length > 1 ? ranked.length - 1 : 0 },
    { id: "terms", label: "Terms", count: words.length },
  ]);

  /** The flow's notes, a tab at a time: how accurately each dimension is made, its warnings, what no
   *  source states, its references. */
  type Note = "accuracy" | "warnings" | "unstated" | "references";
  let note = $state<Note>("accuracy");
  const notes = $derived.by((): { id: Note; label: string; count: number }[] =>
    page
      ? [
          { id: "accuracy" as Note, label: "Accuracy", count: page.view.assessment.accuracy.length },
          { id: "warnings" as Note, label: "Warnings", count: page.view.flow.warnings.length },
          { id: "unstated" as Note, label: "Not specified", count: page.view.flow.not_specified.length },
          { id: "references" as Note, label: "References", count: page.view.flow.references.length },
        ].filter((n) => n.count > 0)
      : [],
  );
  const noteShown = $derived(notes.some((n) => n.id === note) ? note : (notes[0]?.id ?? "references"));
  /** A dimension's value or bound in its unit. */
  const dim = (v: number, unit: string) => (unit === "degree" ? `${num(v)}°` : `${num(v)} nm`);

  // the top bar names the design shown (Fabrication › Waveguide), with the others to switch to
  $effect(() => {
    const c = chosen;
    app.crumb = c
      ? {
          page: "fabrication",
          label: c.label,
          about: page?.platform ? `Made on ${page.platform.name}, from the bare wafer` : "Its flow from the bare wafer",
          current: c.id,
          groups: groups.map((g) => ({ title: g, items: choices.filter((x) => x.group === g).map((x) => ({ id: x.id, label: x.label })) })),
          pick: pick,
        }
      : null;
  });
  $effect(() => () => {
    if (app.crumb?.page === "fabrication") app.crumb = null;
  });

  /** Runs a component's design's flow as a fabrication job, from the start shown: its run records
   *  the flow, its frames and the round trip, and replays them. */
  function runAsJob() {
    const d = chosen?.design;
    if (d?.kind !== "device") return;
    const from = page?.view.start ?? null;
    const name = `${d.spec.kind.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")}-flow${from === "smart_cut" ? "-smart-cut" : from === "simox" ? "-simox" : ""}`;
    startRun(async () => api.runText(await api.fabricationJob(d.spec, name, from), "yaml"), `Running how the ${d.spec.kind} is made`);
  }

  const PANEL = "rounded-2xl border border-base-content/8 bg-base-100";
</script>

<div class="h-full overflow-x-hidden overflow-y-auto">
  <div class="mx-auto max-w-[1500px] space-y-4 p-4 sm:p-5">
    <!-- the design, what it is made on, and whether the flow makes it -->
    <section class="{PANEL} space-y-3 p-3">
      <div class="flex flex-wrap items-center gap-2">
        <span class="flex items-center gap-1 text-xs font-medium text-base-content/70">
          Design
          <Hint text="A suggested flow from published processes, not a foundry's: it starts from the bare wafer, and each step says why it is there, what it does, and where every number comes from." />
        </span>
        <select class="select select-sm w-auto min-w-0 flex-1 sm:w-72 sm:flex-none" value={chosenId} onchange={(e) => pick((e.currentTarget as HTMLSelectElement).value)} aria-label="The design to fabricate">
          {#each groups as g (g)}
            <optgroup label={g}>
              {#each choices.filter((c) => c.group === g) as c (c.id)}
                <option value={c.id}>{c.label}</option>
              {/each}
            </optgroup>
          {/each}
        </select>
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={openFile} title="A job file with a [device] table, or a structure task's layers and shapes"><FileUp size={15} /> Open…</button>
        {#if page?.platform}
          <span class="inline-flex items-center gap-1 rounded-lg bg-base-content/6 px-2 py-1 text-xs">
            <Layers size={13} class="faint" /> {page.platform.name}
            <Hint text="A design has one platform in the process library: its films decide it (its single-crystal film, or the guide film deposited on a bare silicon wafer)." />
          </span>
        {/if}
        {#if page?.view.indices_elsewhere.length}
          <span class="inline-flex items-center gap-1 rounded-lg bg-base-content/6 px-2 py-1 text-xs">
            {page.view.indices_elsewhere.map(([film, at]) => `${film} at ${at} µm`).join(", ")}
            <Hint text="The pictures take each film's permittivity at 1.55 µm, only to tell the films apart. These films' index models stop short of it, so theirs is taken at the end of the model's range nearest 1.55 µm." />
          </span>
        {/if}
        {#if prefer.length}
          <span class="inline-flex items-center gap-1 rounded-lg bg-primary/10 px-2 py-1 text-xs text-primary" title={prefer.join(", ")}>
            preferring {prefer.length} process{prefer.length === 1 ? "" : "es"}
            <button class="link text-xs" onclick={() => (prefer = [])}>undo</button>
          </span>
        {/if}
        <span class="flex-1"></span>
        {#if busy}<span class="loading loading-spinner loading-xs text-primary" aria-label="Working out the flow"></span>{/if}
        {#if trip}
          <span class="inline-flex items-center gap-1.5 rounded-lg px-2 py-1 text-xs font-medium {exact ? 'bg-success/12 text-success' : 'bg-warning/15 text-warning'}" role="status">
            {#if exact}<CircleCheck size={14} /> Makes the design{:else}<TriangleAlert size={14} /> Differs from the design{/if}
            <Hint text={exact ? `The flow run on photonoxide's own geometry gives the design to the bit: 0 of ${trip.points} points through every layer differ.` : `The flow run on photonoxide's own geometry, against the design: ${trip.differing} of ${trip.points} points through every layer differ.`} side="left" />
          </span>
        {/if}
        {#if chosen?.design.kind === "device"}
          <button class="btn ms-auto gap-1.5 btn-sm" onclick={runAsJob} title="Run this flow as a fabrication job: its run records the flow, the frames and the round trip, and replays them"><Play size={14} /> Run as a job</button>
        {/if}
      </div>
      {#if page}
        <StartChoice starts={page.view.starts} start={page.view.start} route={page.view.smart_cut} simox={page.view.simox} refused={page.view.simox_refused} onpick={pickStart} />
      {/if}
    </section>

    {#if problem}
      <div role="alert" class="alert alert-warning alert-soft text-sm selectable"><TriangleAlert size={16} class="shrink-0" /><span>{plain(problem)}</span></div>
    {/if}
    {#if !page}
      {#if !problem}<div class="skeleton h-112 w-full rounded-2xl"></div>{/if}
    {:else}
      <div class="grid gap-4 lg:grid-cols-[21rem_minmax(0,1fr)]">
        <!-- the steps, from the bare wafer, as a timeline; under them the stack at the step shown -->
        <aside class="order-2 flex min-h-0 flex-col gap-4 lg:order-1 lg:sticky lg:top-0 lg:max-h-[calc(100vh-120px)] lg:self-start">
          <section class="{PANEL} flex min-h-0 flex-col" aria-label="The steps">
            <div class="border-b border-base-content/8 px-3 pt-3 pb-2.5">
              <div class="flex items-baseline justify-between gap-2">
                <p class="text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">Process flow</p>
                <p class="text-xs faint num">step {stepNow + 1} of {steps.length}</p>
              </div>
              <!-- one segment a step: done, shown, to come -->
              <div class="mt-2 flex gap-0.5" aria-hidden="true">
                {#each steps as st (st.k)}
                  <span class="h-1 flex-1 rounded-full {st.k < stepNow ? 'bg-primary/45' : st.k === stepNow ? 'bg-primary' : 'bg-base-content/10'}"></span>
                {/each}
              </div>
            </div>
            <ol bind:this={list} class="relative min-h-0 overflow-y-auto p-2">
              {#each steps as st, n (st.k)}
                {@const on = st.k === stepNow}
                {@const done = st.k < stepNow}
                {@const flags = flagged(st.k)}
                <li class="relative pl-8">
                  <!-- the line joining the steps, and the step's dot on it -->
                  {#if n < steps.length - 1}<span class="absolute top-8 bottom-0 left-2.75 w-px {done ? 'bg-primary/40' : 'bg-base-content/12'}"></span>{/if}
                  <span class="absolute top-2 left-0 grid size-6 place-items-center rounded-full text-[11px] font-semibold num {on ? 'bg-primary text-primary-content ring-4 ring-primary/15' : done ? 'bg-primary/15 text-primary' : 'bg-base-content/8 text-base-content/60'}">{st.k + 1}</span>
                  <button class="mb-1 w-full rounded-lg px-2.5 py-2 text-left transition-colors {on ? 'bg-primary/8' : 'hover:bg-base-content/5'}" onclick={() => player?.jump(st.frames[0]?.i ?? 0)} aria-current={on ? "step" : undefined}>
                    <span class="flex items-start gap-1.5">
                      <span class="min-w-0 flex-1 text-[13px] leading-snug font-medium {on ? 'text-primary' : ''}">{st.s.name}</span>
                      {#if flags}<span class="mt-1 size-2 shrink-0 rounded-full bg-warning" title="{flags} of the rules' checks flag this step"></span>{/if}
                    </span>
                    <span class="mt-1 flex items-start gap-1.5">
                      <span class="mt-px shrink-0 rounded px-1 py-px text-[10px] font-medium {TYPE_TONE[st.s.step] ?? 'bg-base-content/8 text-base-content/70'}">{STEP[st.s.step] ?? st.s.step}</span>
                      <span class="min-w-0 text-[11.5px] leading-snug text-base-content/65">{sentence(st.s.does)}</span>
                    </span>
                  </button>
                  {#if on && st.frames.length > 1}
                    <div class="-mt-0.5 flex flex-wrap gap-1 px-2.5 pb-2">
                      {#each st.frames as x (x.i)}
                        <button class="rounded-md px-1.5 py-0.5 text-[11px] transition-colors {x.i === frame ? 'bg-primary text-primary-content' : 'bg-base-content/6 hover:bg-base-content/12'}" onclick={() => player?.jump(x.i)} title={x.f.label}>{PHASE[x.f.phase] ?? x.f.phase}</button>
                      {/each}
                    </div>
                  {/if}
                </li>
              {/each}
            </ol>
          </section>

          <!-- the layers at this moment, from the top down, the substrate last -->
          <section class="{PANEL} shrink-0 p-3" aria-label="The layers at this step">
            <p class="mb-2 flex items-center gap-1 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">
              The layers now <Hint text="After the step shown, from the top down, the substrate last (the first made); the steps go the other way, from the bottom up." side="right" />
            </p>
            <ul class="space-y-1.5 text-xs">
              {#each legend as r, i (i)}
                <li class="flex items-center gap-2">
                  <span class="inline-flex h-4 w-6 shrink-0 overflow-hidden rounded-sm border border-base-content/20" aria-hidden="true">
                    <span class="h-full flex-1" style="background: {colour(r.film)}"></span>
                    {#if r.around !== r.film}<span class="h-full flex-1" style="background: {colour(r.around)}"></span>{/if}
                  </span>
                  <span class="min-w-0 flex-1 leading-tight"><span class="font-medium">{r.name}</span> <span class="faint">{r.film}{r.around !== r.film ? ` in ${r.around}` : ""}</span></span>
                  <span class="shrink-0 num faint">{r.um === null ? "" : `${num(r.um)} µm`}</span>
                </li>
              {/each}
            </ul>
          </section>
        </aside>

        <div class="order-1 min-w-0 space-y-4 lg:order-2">
          <!-- the views and the timeline -->
          <FabPlayer bind:this={player} bind:frame {page} design={shownDesign} active={app.page === "fabrication"} />

          <!-- the step shown, a tab at a time -->
          {#if record}
            <section class="{PANEL} p-4" aria-label="Step {stepNow + 1}">
              <div class="flex flex-wrap items-center gap-2">
                <span class="grid size-7 place-items-center rounded-full bg-primary text-xs font-semibold text-primary-content num">{stepNow + 1}</span>
                <h2 class="min-w-0 text-base font-semibold">{record.name}</h2>
                <span class="rounded-md bg-base-content/6 px-1.5 py-0.5 text-[11px] faint">{STEP[record.step] ?? record.step}</span>
              </div>
              <div class="mt-3 flex flex-wrap gap-1 border-b border-base-content/8" role="tablist">
                {#each tabs as t (t.id)}
                  <button class="-mb-px flex items-center gap-1.5 border-b-2 px-3 py-1.5 text-sm transition-colors {tabShown === t.id ? 'border-primary font-medium text-primary' : 'border-transparent text-base-content/60 hover:text-base-content'}" role="tab" aria-selected={tabShown === t.id} onclick={() => (tab = t.id)}>
                    {t.label}
                    {#if t.count}<span class="rounded-full px-1.5 text-[10.5px] num {t.flag ? 'bg-warning/20 text-warning' : 'bg-base-content/8'}">{t.count}</span>{/if}
                  </button>
                {/each}
              </div>

              <div class="pt-4 text-sm" role="tabpanel">
                {#if tabShown === "overview"}
                  <div class="grid gap-3 md:grid-cols-2">
                    <div class="rounded-xl bg-base-200/50 p-3">
                      <p class="mb-1 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">Why</p>
                      <p class="leading-relaxed selectable">{record.reason}</p>
                    </div>
                    <div class="rounded-xl bg-base-200/50 p-3">
                      <p class="mb-1 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">What it does</p>
                      <p class="leading-relaxed selectable">{record.plain}</p>
                    </div>
                  </div>
                {:else if tabShown === "machine" && record}
                  <LithoTool process={record.process} tone={record.tone} layer={record.layer} shapes={maskShapes} optics={page?.view.assessment.optics ?? []} />
                {:else if tabShown === "numbers"}
                  <div class="grid gap-4 xl:grid-cols-2">
                    <table class="table table-sm h-fit">
                      <tbody>
                        {#each parameters as [k, v] (k)}
                          <tr><td class="w-24 text-xs faint">{k}</td><td class="selectable">{v}</td></tr>
                        {/each}
                      </tbody>
                    </table>
                    <div>
                      <p class="mb-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">What it can achieve</p>
                      {#if record.capabilities.length}
                        <ul class="space-y-1.5">
                          {#each record.capabilities as c, i (i)}
                            <li class="rounded-lg bg-base-200/50 px-3 py-2 selectable">
                              <span class="flex flex-wrap items-baseline justify-between gap-x-2">
                                <span>{quantity(c)} <span class="text-xs faint">({c.conditions})</span></span>
                                <span class={c.value.kind === "not_specified" ? "text-warning" : "font-medium num"}>{value(c.value)}</span>
                              </span>
                              <span class="block text-xs faint">{c.value.kind === "not_specified" ? "Not specified: no source in hand states it." : source(c.value.source)}</span>
                            </li>
                          {/each}
                        </ul>
                      {:else}
                        <p class="faint">Not specified: the step relies on no stated number.</p>
                      {/if}
                      {#if record.facts.length}
                        <p class="mt-3 mb-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">What its sources say</p>
                        <ul class="space-y-1">
                          {#each record.facts as f, i (i)}<li class="selectable">{f.text} <span class="text-xs faint">({source(f.source)})</span></li>{/each}
                        </ul>
                      {/if}
                    </div>
                  </div>
                {:else if tabShown === "checks"}
                  {#if !checksOf(stepNow).length && !record.uncertainties.length}
                    <p class="faint">The rules check nothing at this step, and nothing in it is uncertain.</p>
                  {/if}
                  <ul class="space-y-1.5">
                    {#each checksOf(stepNow) as c, i (i)}
                      <li class="flex items-start gap-2 rounded-lg px-3 py-2 selectable {c.verdict === 'within' ? 'bg-success/8' : 'bg-warning/10'}">
                        {#if c.verdict === "within"}<CircleCheck size={15} class="mt-0.5 shrink-0 text-success" />{:else}<TriangleAlert size={15} class="mt-0.5 shrink-0 text-warning" />{/if}
                        <span><span class="font-medium">{RULE[c.rule] ?? c.rule}</span> <span class="text-xs faint">· {VERDICT[c.verdict] ?? c.verdict}</span><span class="block">{c.text}</span></span>
                      </li>
                    {/each}
                  </ul>
                  {#if record.uncertainties.length}
                    <p class="mt-3 mb-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">Not certain</p>
                    <ul class="ml-4 list-disc space-y-1">
                      {#each record.uncertainties as u, i (i)}<li class="selectable">{u}</li>{/each}
                    </ul>
                  {/if}
                {:else if tabShown === "alternatives"}
                  {#if ranked.length > 1}
                    <ol class="space-y-2">
                      {#each ranked as r, n (r.process)}
                        <li class="rounded-xl border p-3 {r.chosen ? 'border-primary/40 bg-primary/5' : 'border-base-content/8'} {r.feasible ? '' : 'opacity-60'}">
                          <div class="flex flex-wrap items-center gap-2">
                            <span class="text-xs num faint">{n + 1}</span>
                            <span class="font-medium">{r.name}</span>
                            {#if r.chosen}<span class="badge badge-primary badge-sm">chosen</span>{:else if !r.feasible}<span class="badge badge-ghost badge-sm">can't here</span>{/if}
                            <span class="flex-1"></span>
                            {#if !r.chosen}<span class="text-xs faint">{r.cost}</span>{/if}
                            {#if !r.chosen && r.feasible}<button class="btn btn-primary btn-soft btn-xs" onclick={() => useInstead(r.process)} title="Make the flow with this process here">Use instead</button>{/if}
                          </div>
                          {#if r.better.length || r.worse.length}
                            <ul class="mt-1.5 space-y-0.5 text-[13px]">
                              {#each r.better as b, i (i)}<li class="selectable"><span class="font-semibold text-success" aria-label="better">+</span> {b}</li>{/each}
                              {#each r.worse as w, i (i)}<li class="selectable"><span class="font-semibold text-warning" aria-label="worse">−</span> {w}</li>{/each}
                            </ul>
                          {/if}
                        </li>
                      {/each}
                    </ol>
                  {:else}
                    <p class="faint">Only this process can do this step on this platform.</p>
                  {/if}
                {:else}
                  {#if words.length}
                    <dl class="grid gap-x-4 gap-y-2 md:grid-cols-[auto_minmax(0,1fr)]">
                      {#each words as g (g.term)}
                        <dt class="font-medium">{g.term}</dt>
                        <dd class="selectable">{g.definition} <span class="text-xs faint">({source(g.source)})</span></dd>
                      {/each}
                    </dl>
                  {:else}
                    <p class="faint">No term of the glossary in this step's words.</p>
                  {/if}
                {/if}
              </div>
            </section>
          {/if}

          <!-- the flow's notes, a tab at a time -->
          {#if notes.length}
            <section class="{PANEL} p-4">
              <div class="flex flex-wrap gap-1 border-b border-base-content/8" role="tablist">
                {#each notes as n (n.id)}
                  <button class="-mb-px flex items-center gap-1.5 border-b-2 px-3 py-1.5 text-sm transition-colors {noteShown === n.id ? 'border-primary font-medium text-primary' : 'border-transparent text-base-content/60 hover:text-base-content'}" role="tab" aria-selected={noteShown === n.id} onclick={() => (note = n.id)}>
                    {n.label}
                    <span class="rounded-full px-1.5 text-[10.5px] num {n.id === 'warnings' ? 'bg-warning/20 text-warning' : 'bg-base-content/8'}">{n.count}</span>
                  </button>
                {/each}
              </div>
              <div class="max-h-72 overflow-y-auto pt-3 text-[13px]" role="tabpanel">
                {#if noteShown === "accuracy"}
                  <p class="mb-1.5 text-xs faint">How accurately each dimension is made: hover a row for its contributions and their sources. {page.view.assessment.combination}</p>
                  <table class="table table-sm">
                    <thead>
                      <tr><th>Dimension</th><th class="text-right">Design</th><th class="text-right">Made to</th><th class="text-right">At worst</th></tr>
                    </thead>
                    <tbody>
                      {#each page.view.assessment.accuracy as d, i (i)}
                        <tr title={d.contributions.map((c) => `${c.what}: ${c.bound === null ? "not specified" : "± " + dim(c.bound, d.unit)} (${c.conditions}; ${c.source ? source(c.source) : "no source in hand states it"})`).join("\n")}>
                          <td class="selectable">{d.what}</td>
                          <td class="text-right whitespace-nowrap num">{dim(d.nominal, d.unit)}</td>
                          <td class="text-right whitespace-nowrap num {d.complete ? '' : 'text-warning'}">{d.combined === null ? "not specified" : `± ${dim(d.combined, d.unit)}${d.complete ? "" : " +?"}`}</td>
                          <td class="text-right whitespace-nowrap num">{d.worst === null ? "" : `± ${dim(d.worst, d.unit)}`}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                {:else if noteShown === "warnings"}
                  <ul class="space-y-1.5">
                    {#each page.view.flow.warnings as w, i (i)}<li class="flex items-start gap-2 selectable"><TriangleAlert size={14} class="mt-0.5 shrink-0 text-warning" />{w}</li>{/each}
                  </ul>
                {:else if noteShown === "unstated"}
                  <p class="mb-1.5 text-xs faint">Numbers the flow needs that no source in hand states.</p>
                  <ul class="ml-4 list-disc space-y-1">
                    {#each page.view.flow.not_specified as n, i (i)}<li class="selectable">{n}</li>{/each}
                  </ul>
                {:else}
                  <ol class="ml-4 list-decimal space-y-1">
                    {#each page.view.flow.references as r (r.key)}
                      <li class="selectable">{r.citation}, “{r.title}”{r.doi ? `, doi:${r.doi}` : ""}{r.isbn ? `, ISBN ${r.isbn}` : ""}</li>
                    {/each}
                  </ol>
                  <p class="mt-2 text-xs faint">Process library {page.view.flow.library}. The 3D view draws at most {page.triangles.toLocaleString("en")} triangles a frame here (its cap is 200,000).</p>
                {/if}
              </div>
            </section>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
