<script lang="ts">
  // The Fabrication tab (#622): how the design could be made, read-only. The flow
  // photonoxide::fabrication's rules give for the design as it stands (studio/src-tauri's
  // device_fabrication): its steps in order, the wafer's cross-section after the step picked (at
  // the design view's cut), and for that step what the process does in plain words, why it was
  // chosen for this design, what else could do it and why not, what it can achieve with each
  // number's source (on hover), and what isn't certain; then the flow's warnings, what no source
  // in hand states, the round trip (the flow run on the library's geometry against the design)
  // and the references. A suggested flow from published processes, not a foundry's process.
  import { api, type DeviceFabrication, type DeviceSpec } from "../../lib/api";
  import { plain } from "../../lib/deviceview";
  import type { FlowCapability, FlowSource, Stated } from "../../lib/events";
  import ProcessAnimation from "./ProcessAnimation.svelte";

  let { spec, designKey, normal, at }: { spec: DeviceSpec; designKey: unknown; normal: "x" | "y"; at: number } = $props();

  let shown = $state<DeviceFabrication | null>(null);
  let problem = $state("");
  let busy = $state(false);
  /** The step picked, from 0. */
  let picked = $state(0);

  $effect(() => {
    void designKey;
    const s = $state.snapshot(spec) as DeviceSpec;
    let current = true;
    busy = true;
    const timer = setTimeout(() => {
      api
        .deviceFabrication(s)
        .then((f) => {
          if (!current) return;
          shown = airy(f);
          problem = "";
          if (picked >= f.flow.steps.length) picked = 0;
        })
        .catch((e) => {
          if (current) problem = String(e);
        })
        .finally(() => {
          if (current) busy = false;
        });
    }, 300);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  });

  /** The stages with the vacuum where nothing is yet named "air", as the narration calls it. */
  function airy(f: DeviceFabrication): DeviceFabrication {
    const air = <T extends { material: string }>(m: T): T => (m.material.toLowerCase() === "vacuum" ? { ...m, material: "air" } : m);
    return {
      ...f,
      stages: f.stages.map((s) => ({
        ...s,
        scene: { ...s.scene, cladding: air(s.scene.cladding), substrate: air(s.scene.substrate), layers: s.scene.layers.map((l) => ({ ...l, material: air(l.material), background: air(l.background) })) },
      })),
    };
  }

  const step = $derived(shown?.flow.steps[picked] ?? null);
  /** The wafer's layers after the step shown, from the top down, the substrate last: the scene's bands of an etched layer joined. */
  const stack = $derived.by(() => {
    const scene = shown?.stages[picked]?.scene;
    if (!scene) return [];
    const air = (m: string) => m;
    const rows: { name: string; film: string; around: string; um: number | null }[] = [];
    const z: [number, number][] = [];
    for (const l of scene.layers) {
      const base = l.name.replace(/ \d+$/, "");
      const k = rows.findIndex((r) => r.name === base && base !== l.name);
      if (k >= 0) z[k] = [Math.min(z[k][0], l.z_um[0]), Math.max(z[k][1], l.z_um[1])];
      else {
        rows.push({ name: l.name, film: air(l.material.material), around: air(l.background.material), um: null });
        z.push([l.z_um[0], l.z_um[1]]);
      }
    }
    rows.forEach((r, k) => (r.um = z[k][1] - z[k][0]));
    const top = air(scene.cladding.material);
    const bottom = air(scene.substrate.material);
    return [{ name: "above", film: top, around: top, um: null }, ...rows.reverse(), { name: "substrate", film: bottom, around: bottom, um: null }];
  });
  /** The glossary's words the step's narration uses. */
  const words = $derived.by(() => {
    if (!shown || !step) return [];
    const text = `${step.plain} ${step.does} ${step.reason}`.toLowerCase();
    return shown.flow.glossary.filter((g) => text.includes(g.term.toLowerCase()));
  });

  const UNIT: Record<string, string> = { nm: " nm", um: " µm", degree: "°", celsius: " °C", ratio: ":1", number: "", percent: " %" };
  /** A number to four significant digits, without trailing zeros. */
  const num = (v: number) => String(Number(v.toPrecision(4)));
  function value(v: Stated): string {
    switch (v.kind) {
      case "range":
        return v.low === v.high ? `${num(v.low)}${UNIT[v.unit]}` : `${num(v.low)} to ${num(v.high)}${UNIT[v.unit]}`;
      case "at_most":
        return `at most ${num(v.value)}${UNIT[v.unit]}`;
      case "at_least":
        return `at least ${num(v.value)}${UNIT[v.unit]}`;
      default:
        return "not specified";
    }
  }
  function source(s: FlowSource): string {
    const r = shown?.flow.references.find((x: { key: string }) => x.key === s.reference);
    return r ? `${r.citation}, ${s.location}${r.doi ? `, doi:${r.doi}` : r.isbn ? `, ISBN ${r.isbn}` : ""}` : `${s.reference}, ${s.location}`;
  }
  function quantity(c: FlowCapability): string {
    const names: Record<string, string> = {
      min_feature: "minimum feature",
      overlay: "overlay",
      sidewall: "sidewall angle",
      uniformity: "uniformity",
      roughness: "roughness",
      temperature: "temperature",
      thickness: "thickness",
    };
    return c.quantity.kind === "selectivity" ? `selectivity over ${c.quantity.over ?? ""}` : (names[c.quantity.kind] ?? c.quantity.kind);
  }
  /** The picked step's processes ranked, the chosen first (photonoxide::fabrication::assess). */
  const ranked = $derived(shown?.assessment.ranking.find((r) => r.step === picked)?.ranked ?? []);
  /** The rules' checks of the picked step. */
  const stepChecks = $derived(shown?.assessment.checks.filter((c) => c.step === picked) ?? []);
  /** The optics of the picked step's lithography, when it is a projection one. */
  const optics = $derived(step ? (shown?.assessment.optics.find((o) => o.process === step.process) ?? null) : null);
  const VERDICT: Record<string, string> = { within: "within", warning: "warning", not_specified: "not specified" };
  const RULE: Record<string, string> = {
    resolution: "resolution",
    aspect_ratio: "aspect ratio",
    sidewall: "walls",
    selectivity: "selectivity",
    thermal: "thermal budget",
    overlay: "registration",
    planarization: "planarization",
    thickness: "thickness",
  };
  /** A dimension's value or bound in its unit. */
  const dim = (v: number, unit: string) => (unit === "degree" ? `${num(v)}°` : `${num(v)} nm`);
  /** A contribution's source, cited. */
  const cite = (s: FlowSource | null) => (s ? source(s) : "No source in hand states it");
  const STEP: Record<string, string> = {
    wafer: "wafer",
    deposition: "deposition",
    lithography: "lithography",
    pattern_transfer: "pattern transfer",
    planarization: "planarization",
    anneal: "anneal",
  };
</script>

<div class="space-y-3 text-xs">
  <p class="faint">A suggested flow from published processes, not a foundry's process: each step's reason, what else could do it, and every number's source (hover it).</p>
  {#if problem}
    <p role="alert" class="text-warning selectable">{plain(problem)}</p>
  {:else if !shown}
    <div class="skeleton h-40 w-full"></div>
  {:else}
    {#if busy}<p class="faint">Updating…</p>{/if}
    <ol class="space-y-0.5" aria-label="The steps">
      {#each shown.flow.steps as s, k (k)}
        <li>
          <button class="btn btn-ghost btn-xs h-auto w-full justify-start py-1 text-left font-normal {k === picked ? 'btn-active' : ''}" onclick={() => (picked = k)} aria-current={k === picked ? "step" : undefined}>
            <span class="num w-5 shrink-0 text-right faint">{k + 1}</span>
            <span class="w-24 shrink-0 faint">{STEP[s.step] ?? s.step}</span>
            <span>{s.name}{s.operation === "undrawn" ? " (not drawn)" : ""}</span>
          </button>
        </li>
      {/each}
    </ol>

    {#if step}
      <section class="space-y-2 rounded-box border border-base-content/10 p-3" aria-label="Step {picked + 1}">
        <h3 class="text-sm font-medium">{picked + 1}. {step.name}</h3>
        <div>
          <div class="mb-1 faint">The wafer as this step leaves it, at the design view's cut ({normal} = <span class="num">{num(at)}</span> µm)</div>
          <ProcessAnimation stages={shown.stages} steps={shown.flow.steps} bind:picked {normal} {at} />
        </div>
        <div>
          <div class="faint">The layers, from the top down, the substrate last (a job's stack lists them from the bottom up):</div>
          <table class="table table-xs mt-1">
            <tbody>
              {#each stack as r, i (i)}
                <tr>
                  <td class="font-medium">{r.name}</td>
                  <td>{r.film}{r.around !== r.film ? ` in ${r.around}` : ""}</td>
                  <td class="num text-right">{r.um === null ? "" : `${num(r.um)} µm`}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="selectable"><span class="font-medium">What it does.</span> {step.plain}</p>
        <p class="selectable"><span class="font-medium">Here.</span> {step.does}.</p>
        <p class="selectable"><span class="font-medium">Why.</span> {step.reason}</p>
        {#if words.length}
          <div>
            <div class="font-medium">The words</div>
            <dl class="ml-1 space-y-0.5">
              {#each words as g (g.term)}
                <div title={source(g.source)}><dt class="inline font-medium">{g.term}:</dt> <dd class="inline">{g.definition}</dd></div>
              {/each}
            </dl>
          </div>
        {/if}
        {#if step.capabilities.length}
          <div>
            <div class="font-medium">What it can achieve</div>
            <ul class="ml-4 list-disc">
              {#each step.capabilities as c, i (i)}
                <li title={c.value.kind === "not_specified" ? "No source in hand states it" : source(c.value.source)}>
                  {quantity(c)}: <span class={c.value.kind === "not_specified" ? "text-warning" : "num"}>{value(c.value)}</span> <span class="faint">({c.conditions})</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if step.facts.length}
          <div>
            <div class="font-medium">What its sources say</div>
            <ul class="ml-4 list-disc">
              {#each step.facts as f, i (i)}
                <li title={source(f.source)}>{f.text}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if optics}
          <p class="selectable">
            <span class="font-medium">Its optics.</span>
            <span class="num">{num(optics.wavelength_nm)}</span> nm light, numerical aperture <span class="num">{num(optics.na[0])}</span> to
            <span class="num">{num(optics.na[1])}</span>: by the Rayleigh criterion W = k₁λ/NA it prints lines down to
            <span class="num">{num(optics.typical_nm)}</span> nm as a matter of course (k₁ 0.4) and <span class="num">{num(optics.limit_nm)}</span> nm at the limit
            (k₁ 0.25).
          </p>
        {/if}
        {#if ranked.length}
          <div>
            <div class="font-medium">The processes that could do it, best first</div>
            <ol class="ml-4 list-decimal space-y-1" aria-label="Alternatives, ranked">
              {#each ranked as r (r.process)}
                <li class={r.feasible ? "" : "faint"}>
                  <span class="font-medium">{r.name}</span>{#if r.chosen}<span class="badge badge-primary badge-xs ml-1">chosen</span>{:else if !r.feasible}<span class="badge badge-ghost badge-xs ml-1">can't</span>{/if}
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
        {:else if step.alternatives.length}
          <div>
            <div class="font-medium">What else could do it</div>
            <ul class="ml-4 list-disc">
              {#each step.alternatives as a, i (i)}
                <li><span class="font-medium">{a.name}:</span> {a.why_not}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if stepChecks.length}
          <div>
            <div class="font-medium">What the rules check here</div>
            <ul class="ml-4 list-disc">
              {#each stepChecks as c, i (i)}
                <li class="selectable"><span class={c.verdict === "within" ? "" : "text-warning"}>{RULE[c.rule] ?? c.rule}, {VERDICT[c.verdict] ?? c.verdict}:</span> {c.text}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if step.uncertainties.length}
          <div>
            <div class="font-medium">Not certain</div>
            <ul class="ml-4 list-disc">
              {#each step.uncertainties as u, i (i)}<li>{u}</li>{/each}
            </ul>
          </div>
        {/if}
      </section>
    {/if}

    {#if shown.assessment.accuracy.length}
      <section aria-label="How accurately each dimension is made">
        <div class="font-medium">How accurately each dimension is made</div>
        <p class="faint">Hover a row for its contributions and their sources. {shown.assessment.combination}</p>
        <table class="table table-xs mt-1">
          <thead>
            <tr><th>Dimension</th><th class="text-right">Design</th><th class="text-right">Made to</th><th class="text-right">At worst</th></tr>
          </thead>
          <tbody>
            {#each shown.assessment.accuracy as d, i (i)}
              <tr title={d.contributions.map((c) => `${c.what}: ${c.bound === null ? "not specified" : "± " + dim(c.bound, d.unit)} (${c.conditions}; ${cite(c.source)})`).join("\n")}>
                <td>{d.what}</td>
                <td class="num whitespace-nowrap text-right">{dim(d.nominal, d.unit)}</td>
                <td class="num whitespace-nowrap text-right {d.complete ? '' : 'text-warning'}">{d.combined === null ? "not specified" : `± ${dim(d.combined, d.unit)}${d.complete ? "" : " +?"}`}</td>
                <td class="num whitespace-nowrap text-right">{d.worst === null ? "" : `± ${dim(d.worst, d.unit)}`}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="faint mt-1">“+?”: a contribution no source in hand states is left out, so the true spread is larger.</p>
      </section>
    {/if}
    {#if shown.assessment.checks.some((c) => c.verdict !== "within")}
      <div>
        <div class="font-medium">What the rules flag</div>
        <ul class="ml-4 list-disc">
          {#each shown.assessment.checks.filter((c) => c.verdict !== "within") as c, i (i)}
            <li class="selectable"><span class="text-warning">{RULE[c.rule] ?? c.rule}, {VERDICT[c.verdict] ?? c.verdict}:</span> {c.text}</li>
          {/each}
        </ul>
      </div>
    {/if}
    {#if shown.flow.warnings.length}
      <div role="status">
        <div class="font-medium text-warning">Warnings</div>
        <ul class="ml-4 list-disc">
          {#each shown.flow.warnings as w, i (i)}<li class="selectable">{w}</li>{/each}
        </ul>
      </div>
    {/if}
    {#if shown.flow.not_specified.length}
      <div>
        <div class="font-medium">Not specified by any source in hand</div>
        <ul class="ml-4 list-disc">
          {#each shown.flow.not_specified as n, i (i)}<li class="selectable">{n}</li>{/each}
        </ul>
      </div>
    {/if}
    {#if shown.flow.round_trip}
      {@const t = shown.flow.round_trip}
      <p class="selectable">
        <span class="font-medium">The round trip.</span>
        {#if t.structure_equal && t.films_equal && t.differing === 0}
          Run on photonoxide's own geometry, the flow gives the design back to the bit: <span class="num">0</span> of <span class="num">{t.points}</span> points differ.
        {:else}
          <span class="text-warning">The flow doesn't give the design back: <span class="num">{t.differing}</span> of <span class="num">{t.points}</span> points differ{t.structure_equal ? "" : ", the structure differs"}{t.films_equal ? "" : ", a film differs"}.</span>
        {/if}
        Process library {shown.flow.library}.
      </p>
    {/if}
    <details>
      <summary class="cursor-pointer font-medium">References ({shown.flow.references.length})</summary>
      <ul class="ml-4 mt-1 list-disc">
        {#each shown.flow.references as r (r.key)}
          <li class="selectable">{r.citation}, "{r.title}"{r.doi ? `, doi:${r.doi}` : ""}{r.isbn ? `, ISBN ${r.isbn}` : ""}</li>
        {/each}
      </ul>
    </details>
  {/if}
</div>
