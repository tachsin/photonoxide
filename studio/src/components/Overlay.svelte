<script lang="ts" module>
  import type { Overlaid } from "../lib/study";

  /** Something an overlay can draw: a run, or a design computed at its fidelity. */
  export interface OverlayItem {
    id: string;
    label: string;
    /** A line under it: when, how. */
    sub: string;
    /** "run", "design" or "now" (the design as it is being edited). */
    what: string;
    /** What it was loaded at: a change (a run that has finished since) loads it again. */
    version?: string;
    load: () => Promise<Overlaid>;
  }
</script>

<script lang="ts">
  // Overlays in place of the Compare page: tick any of a device's designs and runs and draw them
  // on shared axes, with any quantity they share (S-parameters over the wavelength, a sweep's
  // quantity over its parameter, effective indices, FDTD spectra), and their figures side by
  // side. Each curve's legend names its fidelity and grid; a 2D one is said to be 2D.
  import { TriangleAlert } from "@lucide/svelte";

  import { SERIES, type Series } from "../lib/plot";
  import { caveat } from "../lib/study";
  import Plot from "./Plot.svelte";

  let { items, picked = $bindable([]), empty }: { items: OverlayItem[]; picked?: string[]; empty: string } = $props();

  /** What each thing ticked gave, by its id: loaded once. */
  let loaded = $state<Record<string, { data: Overlaid | null; error: string; loading: boolean; version: string }>>({});
  /** Each thing's colour, kept while it stays ticked, so ticking another repaints nothing. */
  let colours = $state<Record<string, string>>({});

  $effect(() => {
    for (const id of picked) {
      const item = items.find((i) => i.id === id);
      if (!item) continue;
      const version = item.version ?? "";
      if (loaded[id]?.version === version) continue;
      loaded[id] = { data: loaded[id]?.data ?? null, error: "", loading: true, version };
      item
        .load()
        .then((data) => loaded[id]?.version === version && (loaded[id] = { data, error: "", loading: false, version }))
        .catch((e) => loaded[id]?.version === version && (loaded[id] = { data: null, error: String(e), loading: false, version }));
    }
  });

  function toggle(id: string) {
    if (picked.includes(id)) {
      picked = picked.filter((p) => p !== id);
      delete colours[id];
    } else {
      const used = new Set(Object.values(colours));
      colours[id] = SERIES.find((c) => !used.has(c)) ?? SERIES[picked.length % SERIES.length];
      picked = [...picked, id];
    }
  }

  const shown = $derived(picked.map((id) => ({ item: items.find((i) => i.id === id), state: loaded[id], id })).filter((x) => x.item && x.state?.data));
  /** The x axes the things ticked share, by how many have curves on them. */
  const axes = $derived.by(() => {
    const count = new Map<string, { n: number; unit: string }>();
    for (const s of shown) {
      for (const x of new Set(s.state!.data!.curves.map((c) => c.x))) {
        const unit = s.state!.data!.curves.find((c) => c.x === x)!.xUnit;
        count.set(x, { n: (count.get(x)?.n ?? 0) + 1, unit });
      }
    }
    return [...count.entries()].sort((a, b) => b[1].n - a[1].n || (a[0] === "wavelength" ? -1 : 1));
  });
  let xChosen = $state("");
  const x = $derived(axes.find(([k]) => k === xChosen)?.[0] ?? axes[0]?.[0] ?? "");
  const xUnit = $derived(axes.find(([k]) => k === x)?.[1].unit ?? "");
  const ys = $derived.by(() => {
    const count = new Map<string, number>();
    for (const s of shown) for (const y of new Set(s.state!.data!.curves.filter((c) => c.x === x).map((c) => c.y))) count.set(y, (count.get(y) ?? 0) + 1);
    return [...count.entries()].sort((a, b) => b[1] - a[1]);
  });
  let yChosen = $state("");
  /** Whether a quantity is the power from one port into another, not in dB. */
  const transmission = (k: string) => {
    const m = /^\|S(\d)(\d)/.exec(k);
    return !!m && m[1] !== m[2] && !k.includes("(dB)");
  };
  /** The quantity drawn: the one chosen, or of those most things share, the first power (not dB) between two ports. */
  const y = $derived(ys.find(([k]) => k === yChosen)?.[0] ?? ys.find(([k, n]) => n === ys[0]?.[1] && transmission(k))?.[0] ?? ys[0]?.[0] ?? "");

  const series = $derived.by<Series[]>(() => {
    const out: Series[] = [];
    for (const s of shown) {
      const d = s.state!.data!;
      const curves = d.curves.filter((c) => c.x === x && c.y === y);
      curves.forEach((c, k) => {
        out.push({
          label: `${s.item!.label}${c.tag ? ` · ${c.tag}` : ""} · ${d.fidelity}${d.grid ? ` · ${d.grid}` : ""}`,
          points: c.points.filter(([, v]) => Number.isFinite(v)),
          colour: colours[s.id],
          dashed: k > 0,
        });
      });
    }
    return out;
  });
  const figures = $derived.by(() => {
    const symbols = new Map<string, { name: string; unit: string }>();
    for (const s of shown) for (const f of s.state!.data!.figures) if (!symbols.has(f.symbol)) symbols.set(f.symbol, { name: f.name, unit: f.unit });
    return [...symbols.entries()];
  });
  const warnings = $derived([...new Set(shown.map((s) => caveat(s.state!.data!.fidelity)).filter(Boolean))]);
  const show = (v: number | undefined) => (v === undefined ? "—" : Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-3 && v !== 0) ? v.toExponential(4) : v.toPrecision(6));
</script>

<div class="space-y-4">
  <ul class="max-h-64 divide-y divide-base-content/6 overflow-y-auto rounded-lg border border-base-content/8">
    {#each items as it (it.id)}
      {@const st = loaded[it.id]}
      <li class="flex items-center gap-2.5 px-2.5 py-1.5">
        <input type="checkbox" class="checkbox checkbox-xs" checked={picked.includes(it.id)} aria-label="Overlay {it.label}" onchange={() => toggle(it.id)} />
        {#if picked.includes(it.id)}<span class="block h-1 w-4 shrink-0 rounded" style="background:{colours[it.id]}"></span>{/if}
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm">{it.label} <span class="badge badge-ghost badge-xs">{it.what}</span></span>
          <span class="block truncate text-[11px] faint">{it.sub}</span>
        </span>
        {#if st?.loading}<span class="loading loading-spinner loading-xs text-primary" title="Loading{it.what === 'run' ? '' : ': a 2D or 3D design is a solve'}"></span>{/if}
        {#if st?.error}<span class="text-[11px] text-error" title={st.error}>failed</span>{/if}
      </li>
    {:else}
      <li class="px-3 py-6 text-center text-xs faint">{empty}</li>
    {/each}
  </ul>

  {#if shown.length}
    {#each warnings as w (w)}
      <div role="note" class="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-2 text-xs"><TriangleAlert size={14} class="mt-0.5 shrink-0 text-warning" /> {w}</div>
    {/each}
    {#if axes.length}
      <div class="flex flex-wrap items-end gap-3 text-xs">
        <label class="flex flex-col gap-1">
          <span class="muted">Against</span>
          <select class="select select-xs w-40" value={x} onchange={(e) => (xChosen = (e.currentTarget as HTMLSelectElement).value)}>
            {#each axes as [k, v] (k)}<option value={k}>{k} ({v.n} of {shown.length})</option>{/each}
          </select>
        </label>
        <label class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="muted">Quantity</span>
          <select class="select select-xs w-full" value={y} onchange={(e) => (yChosen = (e.currentTarget as HTMLSelectElement).value)}>
            {#each ys as [k, n] (k)}<option value={k}>{k} ({n} of {shown.length})</option>{/each}
          </select>
        </label>
      </div>
      {#if series.length}
        <Plot {series} xLabel={xUnit && xUnit !== "µm" ? `${x} (${xUnit})` : x} xLength={xUnit === "µm"} yLabel={y} markers={series.every((s) => s.points.length <= 60)} name="overlay" />
      {/if}
    {:else}
      <p class="text-xs faint">These share no curve: a spectrum, a sweep or an FDTD run's spectra overlay here.</p>
    {/if}
    {#if figures.length}
      <div class="overflow-x-auto">
        <table class="table table-xs">
          <thead>
            <tr>
              <th>Figure</th>
              {#each shown as s (s.id)}<th class="text-right"><span class="inline-flex items-center gap-1"><span class="h-1 w-3 rounded" style="background:{colours[s.id]}"></span>{s.item?.label}</span><span class="block text-[10px] font-normal faint">{s.state?.data?.fidelity}</span></th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each figures as [symbol, f] (symbol)}
              <tr>
                <td title={f.name}><span class="num">{symbol}</span>{f.unit ? ` (${f.unit})` : ""}</td>
                {#each shown as s (s.id)}<td class="text-right num">{show(s.state?.data?.figures.find((g) => g.symbol === symbol)?.value)}</td>{/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {:else if picked.length}
    <div class="skeleton h-40"></div>
  {/if}
</div>
