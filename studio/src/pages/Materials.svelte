<script lang="ts">
  // Materials: the catalogue. Each material's index models (plotted over their range, evaluated
  // at a wavelength, with their equation and coefficients as the paper prints them), its crystal,
  // its d and r tensors, and the paper every number comes from.
  import { Atom, BookOpen, Clock, ExternalLink, FileText, Search, Thermometer } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import katex from "katex";
  import "katex/dist/katex.min.css";

  import MathText from "../components/MathText.svelte";
  import Plot from "../components/Plot.svelte";
  import Tip from "../components/Tip.svelte";
  import UnitChip from "../components/UnitChip.svelte";
  import {
    api,
    type Axis,
    type Cell,
    type IndexModel,
    type MaterialCurve,
    type MaterialEntry,
    type MaterialPoint,
    type MaterialTag,
    type Source,
    type Tensor,
  } from "../lib/api";
  import type { Series } from "../lib/plot";
  import { len, lenUnit, shown as inUnit, stored } from "../lib/units";

  let entries = $state<MaterialEntry[]>([]);
  let problem = $state("");
  let query = $state("");
  let chosen = $state<string | null>(null);
  let modelId = $state<string | null>(null);
  let temperature = $state<number | null>(null);
  let composition = $state<number | null>(null);
  let wavelength = $state(1.55);
  let quantity = $state<"n" | "group" | "k">("n");
  let curves = $state<MaterialCurve[]>([]);
  let point = $state<MaterialPoint[] | null>(null);
  let pointProblem = $state("");
  /** Which of a material's tensor sets of each kind is shown. */
  let picked = $state<Record<string, number>>({});
  const KINDS = ["second-order", "electro-optic"] as const;
  let curveProblem = $state("");

  api
    .materials()
    .then((all) => (entries = all))
    .catch((e) => (problem = String(e)));

  const CATEGORIES: [MaterialEntry["category"], string][] = [
    ["dielectric", "Dielectrics"],
    ["semiconductor", "Semiconductors"],
    ["nonlinear-crystal", "Nonlinear crystals"],
  ];

  const shown = $derived(
    entries.filter((e) => {
      const q = query.trim().toLowerCase();
      return !q || `${e.name} ${e.formula} ${e.id} ${e.summary}`.toLowerCase().includes(q);
    }),
  );
  const entry = $derived(entries.find((e) => e.id === (chosen ?? entries[0]?.id)));
  const model = $derived<IndexModel | undefined>(
    entry ? (entry.index.find((m) => m.id === modelId) ?? entry.index.find((m) => m.default) ?? entry.index[0]) : undefined,
  );

  // a new material or model starts at that model's own defaults
  $effect(() => {
    const m = model;
    temperature = m?.temperature?.default ?? null;
    composition = m?.composition?.default ?? null;
  });

  function choose(id: string) {
    chosen = id;
    modelId = null;
    picked = {};
  }

  // the curves follow the model and its conditions
  $effect(() => {
    const e = entry;
    const m = model;
    const t = temperature;
    const x = composition;
    if (!e || !m) {
      curves = [];
      return;
    }
    api
      .materialCurves(e.id, m.id, t, x, 160)
      .then((c) => {
        curves = c;
        curveProblem = "";
      })
      .catch((err) => {
        curves = [];
        curveProblem = String(err);
      });
  });

  // and so does the readout at one wavelength
  $effect(() => {
    const e = entry;
    const m = model;
    const t = temperature;
    const x = composition;
    const w = wavelength;
    if (!e || !m || !(w > 0)) {
      point = null;
      return;
    }
    api
      .materialAt(e.id, m.id, t, x, w)
      .then((p) => {
        point = p;
        pointProblem = "";
      })
      .catch((err) => {
        point = null;
        pointProblem = String(err);
      });
  });

  const AXIS: Record<Axis, string> = { isotropic: "n", ordinary: "n_o", extraordinary: "n_e" };
  const AXIS_PLAIN: Record<Axis, string> = { isotropic: "n", ordinary: "nₒ", extraordinary: "nₑ" };

  const lossless = $derived(curves.every((c) => c.k.every((k) => k === 0)));
  const series = $derived<Series[]>(
    curves.map((c) => {
      const values = quantity === "n" ? c.n : quantity === "group" ? c.group : c.k;
      const name = quantity === "group" ? `${AXIS_PLAIN[c.axis]} (group)` : quantity === "k" ? `k (${AXIS_PLAIN[c.axis]})` : AXIS_PLAIN[c.axis];
      return { label: name, points: c.wavelength.map((w, i) => [w, values[i]] as [number, number]).filter((p) => Number.isFinite(p[1])) };
    }),
  );
  const yLabel = $derived(quantity === "n" ? "refractive index n" : quantity === "group" ? "group index n_g" : "extinction coefficient k");

  function display(tex: string): string {
    return katex.renderToString(tex, { displayMode: true, throwOnError: false, output: "html" });
  }

  function inline(tex: string): string {
    return katex.renderToString(tex, { throwOnError: false, output: "html" });
  }

  function reference(s: Source) {
    return entry?.references.find((r) => r.key === s.reference);
  }

  function cite(s: Source): string {
    const r = reference(s);
    return r ? `${r.citation.split(",")[0]}${r.citation.match(/\(\d{4}\)/)?.[0] ? " " + r.citation.match(/\((\d{4})\)/)![1] : ""}, ${s.location}` : s.location;
  }

  function doi(d: string) {
    openUrl(`https://doi.org/${d}`);
  }

  const kelvin = (t: number) => `${t.toFixed(1)} K (${(t - 273.15).toFixed(1)} °C)`;

  // a tensor's element, named the way papers name it: d₃₃, r₄₂
  const SUB = "₀₁₂₃₄₅₆₇₈₉";
  const sub = (n: number) => String(n).replace(/\d/g, (d) => SUB[+d]);
  function element(t: Tensor, row: number, col: number): string {
    return `${t.kind === "second-order" ? "d" : "r"}${sub(row)}${sub(col)}`;
  }

  function cellTitle(t: Tensor, i: number, j: number, c: Cell): string {
    const name = element(t, i + 1, j + 1);
    if (c.kind === "zero") return `${name} = 0 by the point group's symmetry`;
    if (c.kind === "unknown") return `${name}: independent, but this source doesn't give it`;
    if (c.kind === "same") {
      const v = valueOf(t, c);
      return `${name} = ${c.sign < 0 ? "−" : ""}${element(t, c.row, c.col)} by symmetry${v !== null ? ` (${fmt(v, 4)} pm/V)` : ""}`;
    }
    return `${name} = ${c.value} pm/V${c.uncertainty !== null ? ` ± ${c.uncertainty}` : ""}`;
  }

  function valueOf(t: Tensor, c: Cell): number | null {
    if (c.kind === "value") return c.value;
    if (c.kind === "same") {
      const other = t.cells[c.row - 1][c.col - 1];
      const v = valueOf(t, other);
      return v === null ? null : c.sign * v;
    }
    return c.kind === "zero" ? 0 : null;
  }

  /** The library's complaint, its wavelengths (in um) in the unit shown. */
  function complaint(text: string): string {
    const n = String.raw`(-?\d+(?:\.\d+)?(?:e-?\d+)?)`;
    return text
      .replace(/^.*?: /, "")
      .replace(new RegExp(`${n} to ${n} um\\b`, "g"), (_, a, b) => `${len(Number(a))} to ${lenUnit(Number(b))}`)
      .replace(new RegExp(`${n} um\\b`, "g"), (_, a) => lenUnit(Number(a)));
  }

  function fmt(v: number, digits = 6): string {
    return Number(v.toPrecision(digits)).toString();
  }

  const TAG_KIND: Record<MaterialTag["kind"], string> = {
    category: "Category",
    system: "Crystal system",
    "point-group": "Point group (Hermann–Mauguin, Schoenflies)",
    "space-group": "Space group",
    symmetry: "What the symmetry allows",
    optical: "Optical class",
  };

  /** The category outlined, a symmetry that allows χ⁽²⁾ in the accent colour, the rest quiet. */
  function tagStyle(t: MaterialTag): string {
    if (t.kind === "category") return "badge-outline";
    if (t.kind === "symmetry" && t.label.startsWith("non-")) return "badge-secondary badge-soft";
    return "badge-ghost";
  }

  const opticalLabel = (e: MaterialEntry) =>
    e.crystal.optical.kind === "isotropic" ? "isotropic" : `uniaxial, ${e.crystal.optical.positive ? "positive" : "negative"}`;
</script>

<div class="grid h-full grid-cols-[18rem_1fr]">
  <!-- the list -->
  <aside class="flex min-h-0 flex-col border-r border-base-content/8">
    <div class="p-4">
      <label class="input input-sm w-full"><Search size={14} class="faint" /><input bind:value={query} placeholder="Search the materials" /></label>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
      {#if problem}
        <p class="px-3 text-sm text-error">{problem}</p>
      {/if}
      {#each CATEGORIES as [category, title] (category)}
        {@const items = shown.filter((e) => e.category === category)}
        {#if items.length}
          <div class="px-3 pt-3 pb-1 text-[10.5px] font-semibold tracking-wider uppercase faint">{title}</div>
          {#each items as e (e.id)}
            {@const on = entry?.id === e.id}
            <button
              class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
              onclick={() => choose(e.id)}
            >
              <span class="w-20 shrink-0 truncate font-medium">{e.formula}</span>
              <span class="min-w-0 flex-1 truncate text-xs {on ? '' : 'muted'}">{e.name}</span>
              {#if !e.index.length}
                <span class="badge badge-xs badge-ghost" title="No index model yet">soon</span>
              {/if}
            </button>
          {/each}
        {/if}
      {/each}
    </div>
  </aside>

  <!-- the material -->
  <div class="min-h-0 overflow-y-auto">
    {#if entry}
      <div class="glow border-b border-base-content/8 px-8 py-6">
        <div class="flex flex-wrap items-start gap-6">
          <div class="grid size-14 place-items-center rounded-2xl bg-primary/12 text-primary"><Atom size={28} /></div>
          <div class="min-w-0 flex-1">
            <h2 class="text-2xl font-semibold tracking-tight">{entry.formula} <span class="ml-1 text-base font-normal muted">{entry.name}</span></h2>
            <p class="mt-1 max-w-3xl text-sm muted">{entry.summary}</p>
            <!-- the tags: each a statement, its physics and caveats on hover -->
            <div class="mt-3 flex flex-wrap gap-1.5 text-xs" data-tags>
              {#each entry.tags as t (t.kind + t.label)}
                <span class="group relative">
                  <span
                    class="badge badge-sm h-auto cursor-help py-0.5 {tagStyle(t)}"
                    tabindex="0"
                    role="button"
                    aria-label="{TAG_KIND[t.kind]}: {t.label}"
                    data-tag={t.kind}
                  >
                    <MathText text={t.label} />
                  </span>
                  <span
                    role="tooltip"
                    class="invisible absolute top-full left-0 z-30 w-[26rem] max-w-[70vw] pt-1.5 opacity-0 transition-opacity duration-100 group-focus-within:visible group-focus-within:opacity-100 group-hover:visible group-hover:opacity-100"
                  >
                    <span class="block rounded-box border border-base-content/10 bg-base-100 p-3.5 text-left text-xs leading-relaxed font-normal shadow-xl">
                      <span class="block text-[10.5px] font-semibold tracking-wider uppercase faint">{TAG_KIND[t.kind]}</span>
                      <span class="mt-1 block text-sm font-medium"><MathText text={t.label} /></span>
                      <span class="mt-1.5 block muted selectable"><MathText text={t.detail} /></span>
                      {#if t.source}
                        <span class="mt-2 flex gap-1.5 border-t border-base-content/8 pt-2 faint selectable"><BookOpen size={12} class="mt-0.5 shrink-0" /><span>{t.source}</span></span>
                      {/if}
                    </span>
                  </span>
                </span>
              {/each}
            </div>
          </div>
          <!-- the index ellipsoid's cross-section through the optic axis -->
          <div class="flex flex-col items-center gap-1" title="The index ellipsoid's section through the optic axis">
            <svg width="96" height="96" viewBox="-48 -48 96 96" class="text-primary">
              {#if entry.crystal.optical.kind === "isotropic"}
                <circle r="30" class="fill-primary/10 stroke-primary" stroke-width="1.5" />
                <text y="4" text-anchor="middle" class="fill-base-content/60 text-[11px]">n</text>
              {:else}
                {@const prolate = entry.crystal.optical.positive}
                <line x1="0" y1="-44" x2="0" y2="44" class="stroke-base-content/40" stroke-dasharray="3 3" />
                <ellipse rx={prolate ? 24 : 34} ry={prolate ? 36 : 24} class="fill-primary/10 stroke-primary" stroke-width="1.5" />
                <text x="4" y="-38" class="fill-base-content/60 text-[10px]">c</text>
                <text x={prolate ? 26 : 36} y="4" class="fill-base-content/60 text-[10px]">nₒ</text>
                <text x="4" y={prolate ? 33 : 21} class="fill-base-content/60 text-[10px]">nₑ</text>
              {/if}
            </svg>
            <span class="text-xs faint">{opticalLabel(entry)}</span>
          </div>
        </div>
        {#if entry.crystal.optical.kind === "uniaxial" || entry.crystal.notes}
          <p class="mt-3 text-xs faint">
            {#if entry.crystal.optical.kind === "uniaxial"}Optic axis {entry.crystal.optical.optic_axis}.{/if}
            {entry.crystal.notes}
          </p>
        {/if}
      </div>

      <div class="space-y-6 px-8 py-6">
        <Tip id="materials-intro" title="Every number from its paper">
          Each model is the paper's own equation and coefficients, valid over the range it states; outside it the library refuses, it doesn't extrapolate.
          The references at the bottom open the papers.
        </Tip>

        {#if entry.coming.length}
          <!-- what has no number yet: calm chips, the paper and what is in hand on hover -->
          <div class="flex flex-wrap items-center gap-2" data-coming>
            <span class="text-xs faint">Coming</span>
            {#each entry.coming as c (c.property)}
              <span class="group relative">
                <span class="badge badge-ghost h-auto cursor-help gap-1.5 py-1 text-left text-xs" tabindex="0" role="button" data-coming-chip>
                  <Clock size={12} class="faint" />
                  <span>{c.property}: {c.source ? `from ${c.source}, coming` : "coming"}</span>
                </span>
                <span
                  role="tooltip"
                  class="invisible absolute top-full left-0 z-30 w-[28rem] max-w-[70vw] pt-1.5 opacity-0 transition-opacity duration-100 group-focus-within:visible group-focus-within:opacity-100 group-hover:visible group-hover:opacity-100"
                >
                  <span class="block rounded-box border border-base-content/10 bg-base-100 p-3.5 text-left text-xs leading-relaxed shadow-xl">
                    <span class="block text-sm font-medium">{c.property}</span>
                    {#if c.citation}
                      <span class="mt-1 block selectable">From {c.citation}</span>
                      <span class="mt-1.5 flex flex-wrap items-center gap-1.5">
                        <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => doi(c.doi)}><ExternalLink size={12} /> doi:{c.doi}</button>
                        <span class="badge badge-xs badge-ghost">{c.open ? "open access" : "subscription"}</span>
                      </span>
                    {:else}
                      <span class="mt-1 block muted">No published measurement found yet.</span>
                    {/if}
                    <span class="mt-2 block border-t border-base-content/8 pt-2 muted selectable"><MathText text={c.detail} /></span>
                  </span>
                </span>
              </span>
            {/each}
          </div>
        {/if}

        {#if model}
          <!-- the index -->
          <section class="panel p-5">
            <div class="flex flex-wrap items-center gap-3">
              <h3 class="panel-title">Refractive index</h3>
              <div class="flex-1"></div>
              {#if entry.index.length > 1}
                <div class="flex flex-wrap justify-end gap-1">
                  {#each entry.index as m (m.id)}
                    <button class="btn btn-xs {m.id === model.id ? 'btn-primary' : 'btn-ghost'}" onclick={() => (modelId = m.id)} title={m.notes}>
                      {m.name}{m.default ? " ·  default" : ""}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="mt-4 flex flex-wrap items-end gap-5">
              <label class="flex flex-col gap-1 text-xs">
                <span class="muted">Wavelength (<UnitChip />)</span>
                <input
                  class="input input-sm w-32 num"
                  type="number"
                  step={inUnit(0.01)}
                  min={inUnit(0.01)}
                  value={inUnit(wavelength)}
                  oninput={(e) => {
                    const raw = e.currentTarget.value;
                    if (raw !== "" && Number.isFinite(Number(raw))) wavelength = stored(Number(raw));
                  }}
                />
              </label>
              {#if model.temperature}
                {@const p = model.temperature}
                <label class="flex flex-col gap-1 text-xs">
                  <span class="muted flex items-center gap-1"><Thermometer size={12} /> Temperature (K), {p.min}–{p.max}</span>
                  <input class="input input-sm w-32 num" type="number" step="1" min={p.min} max={p.max} bind:value={temperature} />
                </label>
              {/if}
              {#if model.composition}
                {@const p = model.composition}
                <label class="flex min-w-64 flex-col gap-1 text-xs">
                  <span class="muted">{p.name}, <i>{p.symbol}</i> = <span class="num">{composition?.toFixed(3)}</span></span>
                  <input class="range range-xs range-primary" type="range" min={p.min} max={p.max} step="0.001" bind:value={composition} />
                </label>
              {/if}
              <div class="flex-1"></div>
              <div class="join">
                <button class="btn btn-xs join-item {quantity === 'n' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (quantity = "n")}>n</button>
                <button class="btn btn-xs join-item {quantity === 'group' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (quantity = "group")}>group index</button>
                <button class="btn btn-xs join-item {quantity === 'k' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (quantity = "k")}>k</button>
              </div>
            </div>

            <!-- the readout at one wavelength -->
            <div class="mt-4 overflow-x-auto">
              {#if point}
                <table class="table table-sm w-auto">
                  <thead><tr><th></th><th>n</th><th>k</th><th>ε′</th><th>ε″</th><th>group index</th></tr></thead>
                  <tbody>
                    {#each point as p (p.axis)}
                      <tr class="num">
                        <td class="font-sans">{@html inline(AXIS[p.axis])}</td>
                        <td>{fmt(p.n)}</td>
                        <td>{fmt(p.k, 3)}</td>
                        <td>{fmt(p.eps_re)}</td>
                        <td>{fmt(p.eps_im, 3)}</td>
                        <td>{fmt(p.group)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {:else if pointProblem}
                <p class="text-sm text-warning">{complaint(pointProblem)}</p>
              {/if}
            </div>

            <div class="mt-3">
              {#if curveProblem}
                <p class="text-sm text-error">{curveProblem}</p>
              {:else if quantity === "k" && lossless}
                <p class="py-10 text-center text-sm faint">This model is lossless across its range: k = 0.</p>
              {:else if series.length}
                <Plot {series} xLabel="wavelength" xLength {yLabel} height={300} markers={false} name="{entry.id}-{model.id}-{quantity}" />
              {/if}
            </div>
            {#if curves.length}
              <p class="mt-1 text-xs faint">
                Valid from <span class="num">{fmt(inUnit(curves[0].range[0]), 4)}</span> to <span class="num">{fmt(inUnit(curves[0].range[1]), 4)}</span> <UnitChip />{#if curves.length > 1 && (curves[1].range[0] !== curves[0].range[0] || curves[1].range[1] !== curves[0].range[1])}
                  ({AXIS_PLAIN[curves[0].axis]}), <span class="num">{fmt(inUnit(curves[1].range[0]), 4)}</span> to <span class="num">{fmt(inUnit(curves[1].range[1]), 4)}</span> <UnitChip /> ({AXIS_PLAIN[curves[1].axis]}){/if}{#if temperature !== null}, at {kelvin(temperature)}{/if}.
              </p>
            {/if}
          </section>

          <!-- the model -->
          <section class="panel p-5">
            <h3 class="panel-title">The model: {model.name}</h3>
            <div class="mt-3 overflow-x-auto">{@html display(model.equation)}</div>
            <p class="mt-1 text-sm muted"><MathText text={model.symbols} /></p>
            <dl class="mt-4 grid grid-cols-[9rem_1fr] gap-x-4 gap-y-2 text-sm">
              <dt class="faint">Range</dt>
              <dd>
                <span class="num">{fmt(inUnit(model.wavelength[0]), 4)}–{fmt(inUnit(model.wavelength[1]), 4)}</span> <UnitChip />{#if model.temperature}; <span class="num">{model.temperature.min}–{model.temperature.max}</span> K{/if}{#if model.composition}; {model.composition.symbol} from <span class="num">{model.composition.min}</span> to <span class="num">{model.composition.max}</span>{/if}
              </dd>
              <dt class="faint">Accuracy</dt>
              <dd>{model.accuracy}</dd>
              {#if model.notes}
                <dt class="faint">Notes</dt>
                <dd class="muted">{model.notes}</dd>
              {/if}
              <dt class="faint">From</dt>
              <dd class="flex flex-wrap gap-2">
                {#each model.sources as s (s.reference + s.location)}
                  {@const r = reference(s)}
                  <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => r && doi(r.doi)} title={r?.title}><FileText size={12} /> {cite(s)}</button>
                {/each}
              </dd>
            </dl>
            {#each model.coefficients as t (t.caption)}
              <div class="mt-5">
                <p class="mb-1 text-xs font-medium muted">{t.caption}</p>
                <div class="overflow-x-auto">
                  <table class="table table-xs w-auto">
                    <thead><tr>{#each t.columns as c, i (i)}<th><MathText text={c} /></th>{/each}</tr></thead>
                    <tbody>
                      {#each t.rows as row, i (i)}
                        <tr>{#each row as c, j (j)}<td class={j ? "num" : "faint"}><MathText text={c} /></td>{/each}</tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              </div>
            {/each}
          </section>
        {:else}
          <section class="panel p-5 text-sm muted">No refractive-index model yet: see what each property waits for above.</section>
        {/if}

        <!-- the tensors -->
        {#if entry.tensors.length || entry.constants.length}
          <section class="panel p-5">
            <h3 class="panel-title">Second-order and electro-optic tensors</h3>
            <p class="mt-1 text-xs faint">
              In pm/V, Voigt notation; faint zeros vanish by the point group, linked elements equal another by symmetry, a "?" is independent but not given by this source.
            </p>
            <div class="mt-4 grid gap-6 2xl:grid-cols-2">
              {#each KINDS as kind (kind)}
                {@const sets = entry.tensors.filter((t) => t.kind === kind)}
                {#if sets.length}
                  {@const t = sets[Math.min(picked[kind] ?? 0, sets.length - 1)]}
                  {@const r = kind === "second-order" ? 3 : 6}
                  {@const c = kind === "second-order" ? 6 : 3}
                  <div class="min-w-0 rounded-box border border-base-content/8 p-4">
                    <div class="flex flex-wrap items-center gap-2">
                      <span class="text-sm font-semibold">{@html inline(kind === "second-order" ? "d_{il}" : "r_{ij}")}</span>
                      <span class="text-sm muted">{kind === "second-order" ? "second-order nonlinear" : "electro-optic (Pockels)"}</span>
                      <div class="flex-1"></div>
                      {#if t.clamping !== "none"}<span class="badge badge-xs {t.clamping === 'clamped' ? 'badge-info' : 'badge-accent'} badge-soft">{t.clamping}</span>{/if}
                    </div>
                    {#if sets.length > 1}
                      <div class="mt-3 flex flex-wrap gap-1">
                        {#each sets as s, k (k)}
                          <button class="btn btn-xs {s === t ? 'btn-primary' : 'btn-ghost'}" onclick={() => (picked[kind] = k)}>{s.label}</button>
                        {/each}
                      </div>
                    {:else}
                      <p class="mt-2 text-xs muted">{t.label}</p>
                    {/if}
                    <div class="mt-3 overflow-x-auto">
                      <div class="inline-grid gap-1 text-xs" style="grid-template-columns: 1.4rem repeat({c}, minmax(3.2rem, auto))">
                        <span></span>
                        {#each Array(c) as _, j (j)}<span class="text-center faint num">{j + 1}</span>{/each}
                        {#each Array(r) as _, i (i)}
                          <span class="self-center faint num">{i + 1}</span>
                          {#each t.cells[i] as cell, j (j)}
                            <span
                              class="rounded-md px-1.5 py-1 text-center whitespace-nowrap num
                                {cell.kind === 'value' ? 'bg-primary/12 font-semibold text-primary' : ''}
                                {cell.kind === 'same' ? 'bg-base-content/5 italic muted' : ''}
                                {cell.kind === 'unknown' ? 'bg-warning/10 text-warning' : ''}
                                {cell.kind === 'zero' ? 'faint opacity-50' : ''}"
                              title={cellTitle(t, i, j, cell)}
                            >
                              {#if cell.kind === "zero"}0{:else if cell.kind === "unknown"}?{:else if cell.kind === "value"}{cell.value}{#if cell.uncertainty !== null}<span class="font-normal opacity-70"> ±{cell.uncertainty}</span>{/if}{:else}{cell.sign < 0 ? "−" : ""}{element(t, cell.row, cell.col)}{/if}
                            </span>
                          {/each}
                        {/each}
                      </div>
                    </div>
                    <div class="mt-3 space-y-1 text-xs">
                      {#if t.source}
                        {@const ref = reference(t.source)}
                        <button class="btn btn-ghost btn-xs -ml-2 h-auto gap-1 py-0.5 text-left text-primary" onclick={() => ref && doi(ref.doi)} title={ref?.title}><FileText size={12} class="shrink-0" /> {cite(t.source)}</button>
                      {/if}
                      {#if t.notes}
                        <details class="group">
                          <summary class="cursor-pointer faint hover:text-base-content">About these values</summary>
                          <p class="mt-1 muted">{t.notes}</p>
                          <p class="mt-1 faint">{t.convention}</p>
                        </details>
                      {/if}
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
            {#if entry.constants.length}
              <table class="table table-sm mt-5 w-auto">
                <tbody>
                  {#each entry.constants as k (k.symbol + k.conditions)}
                    <tr>
                      <td>{@html inline(k.symbol)}</td>
                      <td class="whitespace-nowrap num font-semibold">{k.value}{#if k.uncertainty !== null} ± {k.uncertainty}{/if} {k.unit}</td>
                      <td class="text-xs muted">{k.name}; {k.conditions}</td>
                      <td>
                        {#if reference(k.source)}
                          {@const ref = reference(k.source)!}
                          <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => doi(ref.doi)} title={ref.title}><FileText size={12} /> {cite(k.source)}</button>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </section>
        {/if}

        <!-- the papers -->
        <section class="panel p-5">
          <h3 class="panel-title flex items-center gap-2"><BookOpen size={14} /> References</h3>
          <ul class="mt-3 space-y-3 text-sm">
            {#each entry.references as r (r.key)}
              <li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                <span class="min-w-0 flex-1">
                  <span>{r.citation}</span>
                  <span class="block text-xs muted italic">{r.title}</span>
                </span>
                <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => doi(r.doi)}><ExternalLink size={12} /> doi:{r.doi}</button>
                {#if r.open_access}
                  <button class="btn btn-ghost btn-xs gap-1 text-success" onclick={() => openUrl(r.open_access!)}><ExternalLink size={12} /> open access</button>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      </div>
    {:else if !problem}
      <div class="grid h-full place-items-center"><span class="loading loading-ring loading-lg text-primary"></span></div>
    {/if}
  </div>
</div>
