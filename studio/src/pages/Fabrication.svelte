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
  import { open } from "@tauri-apps/plugin-dialog";
  import { CircleCheck, FileUp, TriangleAlert } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import FabPlayer from "../components/fabrication/FabPlayer.svelte";
  import { api, formatOf, formatOfText, type FabDesign, type FabricationPage } from "../lib/api";
  import { app, CUSTOM } from "../lib/app.svelte";
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
      if (/(^|\n)\s*(device:|\[device\])/.test(j.text) || j.kind === "structure") {
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
      if (here) chosenId = here.id;
    });
  });
  $effect(() => {
    if (!chosenId && devices.list.length) untrack(() => (chosenId = `lib:${devices.list[0].id}`));
  });

  $effect(() => {
    const c = chosen;
    const p = [...prefer];
    if (!c) return;
    let live = true;
    busy = true;
    api
      .fabricationPage($state.snapshot(c.design) as FabDesign, p, [])
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
    chosenId = id;
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
    if (el.offsetTop < list.scrollTop) list.scrollTop = el.offsetTop - 8;
    else if (el.offsetTop + el.offsetHeight > list.scrollTop + list.clientHeight) list.scrollTop = el.offsetTop + el.offsetHeight - list.clientHeight + 8;
  });
  const checksOf = (k: number) => page?.view.assessment.checks.filter((c) => c.step === k) ?? [];
  const flagged = (k: number) => checksOf(k).filter((c) => c.verdict !== "within").length;
  const ranked = $derived(page?.view.assessment.ranking.find((r) => r.step === stepNow)?.ranked ?? []);
  const source = (s: FlowSource) => cited(page?.view.flow.references ?? [], s);
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
</script>

<div class="h-full overflow-x-hidden overflow-y-auto">
  <div class="mx-auto max-w-[1500px] space-y-3 p-3 sm:p-4">
    <!-- the design and what it is made on -->
    <div class="flex flex-wrap items-center gap-2">
      <label class="flex w-full min-w-0 items-center gap-2 sm:w-auto">
        <span class="shrink-0 text-sm font-medium">Design</span>
        <select class="select select-sm w-auto min-w-0 flex-1 sm:w-80 sm:flex-none" value={chosenId} onchange={(e) => pick((e.currentTarget as HTMLSelectElement).value)} aria-label="The design to fabricate">
          {#each groups as g (g)}
            <optgroup label={g}>
              {#each choices.filter((c) => c.group === g) as c (c.id)}
                <option value={c.id}>{c.label}</option>
              {/each}
            </optgroup>
          {/each}
        </select>
      </label>
      <button class="btn btn-sm gap-1.5" onclick={openFile} title="A job file with a [device] table, or a structure task's layers and shapes"><FileUp size={15} /> Open a job file…</button>
      {#if page?.platform}
        <span class="badge badge-ghost gap-1" title="A design has one platform in the process library: its films decide it (its single-crystal film, or the guide film deposited on a bare silicon wafer)">Platform: {page.platform.name}</span>
      {/if}
      {#if prefer.length}
        <span class="badge badge-primary badge-outline gap-1" title={prefer.join(", ")}>preferring {prefer.length} process{prefer.length === 1 ? "" : "es"}</span>
        <button class="btn btn-ghost btn-xs" onclick={() => (prefer = [])}>The rules' choice again</button>
      {/if}
      <span class="flex-1"></span>
      {#if busy}<span class="loading loading-spinner loading-xs" aria-label="Working out the flow"></span>{/if}
      {#if trip}
        <span class="badge gap-1 {exact ? 'badge-success badge-soft' : 'badge-warning'}" role="status" title="The flow run on photonoxide's own geometry, against the design, on {trip.points} points through every layer">
          {#if exact}<CircleCheck size={13} /> The last step is the design: 0 of {trip.points} points differ{:else}<TriangleAlert size={13} /> The last step isn't the design: {trip.differing} of {trip.points} points differ{/if}
        </span>
      {/if}
    </div>
    <p class="text-xs faint">A suggested flow from published processes, not a foundry's process: it starts from the bare wafer, and each step says why it is there, what it does, and where every number comes from.</p>

    {#if problem}
      <p role="alert" class="rounded-box border border-warning/40 bg-warning/10 p-3 text-sm text-warning selectable">{plain(problem)}</p>
    {/if}
    {#if !page}
      {#if !problem}<div class="skeleton h-96 w-full"></div>{/if}
    {:else}
      <div class="grid gap-3 lg:grid-cols-[minmax(240px,320px)_minmax(0,1fr)]">
        <!-- the steps, from the bare wafer -->
        <section bind:this={list} class="relative order-2 rounded-box border border-base-content/10 bg-base-100 p-2 lg:order-1 lg:max-h-[calc(100vh-150px)] lg:overflow-y-auto" aria-label="The steps">
          <div class="px-1 pb-1 text-xs font-medium faint">{page.view.flow.steps.length} steps, from the bare wafer</div>
          <ol class="space-y-0.5">
            {#each steps as st (st.k)}
              {@const on = st.k === stepNow}
              {@const flags = flagged(st.k)}
              <li>
                <button class="flex w-full items-start gap-2 rounded-lg px-2 py-1.5 text-left text-xs transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}" onclick={() => player?.jump(st.frames[0]?.i ?? 0)} aria-current={on ? "step" : undefined}>
                  <span class="num w-5 shrink-0 text-right faint">{st.k + 1}</span>
                  <span class="min-w-0 flex-1">
                    <span class="block font-medium">{st.s.name}</span>
                    <span class="block faint">{STEP[st.s.step] ?? st.s.step} · {st.s.does}</span>
                  </span>
                  {#if flags}<span class="badge badge-warning badge-xs shrink-0" title="{flags} of the rules' checks flag this step">{flags}</span>{/if}
                </button>
                {#if st.frames.length > 1}
                  <div class="ml-9 flex flex-wrap gap-1 pb-1">
                    {#each st.frames as x (x.i)}
                      <button class="btn btn-xs {x.i === frame ? 'btn-primary' : 'btn-ghost'}" onclick={() => player?.jump(x.i)} title={x.f.label}>{PHASE[x.f.phase] ?? x.f.phase}</button>
                    {/each}
                  </div>
                {/if}
              </li>
            {/each}
          </ol>
        </section>

        <div class="order-1 min-w-0 space-y-3 lg:order-2">
          <!-- the views and the timeline -->
          <FabPlayer bind:this={player} bind:frame {page} design={shownDesign} active={app.page === "fabrication"} />

          <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(220px,300px)]">
            <!-- the step shown: why, what, its numbers and their sources, what the rules flag -->
            {#if record}
              <section class="space-y-2 rounded-box border border-base-content/10 bg-base-100 p-3 text-xs" aria-label="Step {stepNow + 1}">
                <h2 class="text-sm font-medium">{stepNow + 1}. {record.name}</h2>
                <p class="selectable"><span class="font-medium">Why.</span> {record.reason}</p>
                <p class="selectable"><span class="font-medium">What it does.</span> {record.plain}</p>
                <table class="table table-xs">
                  <tbody>
                    {#each parameters as [k, v] (k)}
                      <tr><td class="w-24 faint">{k}</td><td class="selectable">{v}</td></tr>
                    {/each}
                  </tbody>
                </table>
                <div>
                  <div class="font-medium">What it can achieve, and the source of each number</div>
                  {#if record.capabilities.length}
                    <ul class="ml-4 list-disc space-y-0.5">
                      {#each record.capabilities as c, i (i)}
                        <li class="selectable">
                          {quantity(c)}: <span class={c.value.kind === "not_specified" ? "text-warning" : "num"}>{value(c.value)}</span> <span class="faint">({c.conditions})</span>
                          <span class="block faint">{c.value.kind === "not_specified" ? "Not specified: no source in hand states it." : source(c.value.source)}</span>
                        </li>
                      {/each}
                    </ul>
                  {:else}
                    <p class="faint">Not specified: the step relies on no stated number.</p>
                  {/if}
                </div>
                {#if record.facts.length}
                  <div>
                    <div class="font-medium">What its sources say</div>
                    <ul class="ml-4 list-disc">
                      {#each record.facts as f, i (i)}<li class="selectable">{f.text} <span class="faint">({source(f.source)})</span></li>{/each}
                    </ul>
                  </div>
                {/if}
                {#if checksOf(stepNow).length}
                  <div>
                    <div class="font-medium">What the rules check here</div>
                    <ul class="ml-4 list-disc">
                      {#each checksOf(stepNow) as c, i (i)}
                        <li class="selectable"><span class={c.verdict === "within" ? "" : "text-warning"}>{RULE[c.rule] ?? c.rule}, {VERDICT[c.verdict] ?? c.verdict}:</span> {c.text}</li>
                      {/each}
                    </ul>
                  </div>
                {/if}
                {#if record.uncertainties.length}
                  <div>
                    <div class="font-medium">Not certain</div>
                    <ul class="ml-4 list-disc">
                      {#each record.uncertainties as u, i (i)}<li class="selectable">{u}</li>{/each}
                    </ul>
                  </div>
                {/if}
                {#if ranked.length > 1}
                  <div>
                    <div class="font-medium">The processes that could do it, best first</div>
                    <ol class="ml-4 list-decimal space-y-1">
                      {#each ranked as r (r.process)}
                        <li class={r.feasible ? "" : "faint"}>
                          <span class="font-medium">{r.name}</span>
                          {#if r.chosen}<span class="badge badge-primary badge-xs ml-1">chosen</span>{:else if !r.feasible}<span class="badge badge-ghost badge-xs ml-1">can't</span>{:else}<button class="btn btn-ghost btn-xs ml-1" onclick={() => useInstead(r.process)} title="Make the flow with this process here">Use instead</button>{/if}
                          {#if !r.chosen}<span class="faint"> · {r.cost}</span>{/if}
                          {#if r.better.length || r.worse.length}
                            <ul class="ml-3">
                              {#each r.better as b, i (i)}<li class="selectable"><span class="text-success" aria-label="better">+</span> {b}</li>{/each}
                              {#each r.worse as w, i (i)}<li class="selectable"><span class="text-warning" aria-label="worse">−</span> {w}</li>{/each}
                            </ul>
                          {/if}
                        </li>
                      {/each}
                    </ol>
                  </div>
                {/if}
              </section>
            {/if}

            <!-- the layers at this moment, from the top down, the substrate last -->
            <section class="space-y-1 rounded-box border border-base-content/10 bg-base-100 p-3 text-xs" aria-label="The layers at this step">
              <div class="font-medium">The layers now</div>
              <p class="faint">From the top down, the substrate last (the first made); the steps go the other way, from the bottom up.</p>
              <ul class="space-y-1">
                {#each legend as r, i (i)}
                  <li class="flex items-center gap-2">
                    <span class="inline-flex h-3.5 w-5 shrink-0 overflow-hidden rounded-sm border border-base-content/20" aria-hidden="true">
                      <span class="h-full flex-1" style="background: {colour(r.film)}"></span>
                      {#if r.around !== r.film}<span class="h-full flex-1" style="background: {colour(r.around)}"></span>{/if}
                    </span>
                    <span class="min-w-0 flex-1"><span class="font-medium">{r.name}</span>: {r.film}{r.around !== r.film ? ` in ${r.around}` : ""}</span>
                    <span class="num faint">{r.um === null ? "" : `${num(r.um)} µm`}</span>
                  </li>
                {/each}
              </ul>
            </section>
          </div>

          {#if page.view.flow.warnings.length}
            <section role="status" class="rounded-box border border-warning/40 bg-warning/5 p-3 text-xs">
              <div class="font-medium text-warning">The flow's warnings</div>
              <ul class="ml-4 list-disc">
                {#each page.view.flow.warnings as w, i (i)}<li class="selectable">{w}</li>{/each}
              </ul>
            </section>
          {/if}
          {#if page.view.flow.not_specified.length}
            <details class="rounded-box border border-base-content/10 bg-base-100 p-3 text-xs">
              <summary class="cursor-pointer font-medium">Not specified by any source in hand ({page.view.flow.not_specified.length})</summary>
              <ul class="mt-1 ml-4 list-disc">
                {#each page.view.flow.not_specified as n, i (i)}<li class="selectable">{n}</li>{/each}
              </ul>
            </details>
          {/if}
          <details class="rounded-box border border-base-content/10 bg-base-100 p-3 text-xs">
            <summary class="cursor-pointer font-medium">References ({page.view.flow.references.length})</summary>
            <ul class="mt-1 ml-4 list-disc">
              {#each page.view.flow.references as r (r.key)}
                <li class="selectable">{r.citation}, "{r.title}"{r.doi ? `, doi:${r.doi}` : ""}{r.isbn ? `, ISBN ${r.isbn}` : ""}</li>
              {/each}
            </ul>
            <p class="mt-1 faint">Process library {page.view.flow.library}. The 3D view draws at most {page.triangles.toLocaleString("en")} triangles a frame here (its cap is 200,000).</p>
          </details>
        </div>
      </div>
    {/if}
  </div>
</div>
