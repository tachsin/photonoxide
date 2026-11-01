<script lang="ts">
  // A device's cross-section where the top view's cut is: the substrate, each layer with its
  // shapes cut across and its background around them, and the cladding, true to scale (a wide
  // cut, across a ring and its buses, has its height stretched and says by how much), from
  // 1 µm below the core to 1 µm above it.
  import type { FrameFilm } from "../../lib/api";
  import { cut, fillOf, tick } from "../../lib/deviceview";
  import type { Scene, SceneEtched } from "../../lib/events";
  import { ticks } from "../../lib/plot";
  import { unitText } from "../../lib/units";

  // `etched`, the etched layers' shapes with their etches (photonoxide::job::scene_etched), draws
  // each such shape exactly, a trapezoid from its top face down to its floor with its walls leaning
  // out, in place of the bands' staircase the scene draws them in (their slab stays the bands')
  // `quiet` leaves out "the cut meets no shape here", for a wafer with nothing drawn on it yet
  // `maxStretch` bounds how far the height may be stretched (8× by default); `names` false leaves
  // out the layers' names and the margin they take (a narrow picture, whose legend names them)
  // `films`, the films a flow's frame draws over its planar layers (photonoxide::job::FrameFilm:
  // the resist, a film conformal over an etched step), each piece cut across from its bottom
  // outline to its top, drawn over the layers' backgrounds and under their shapes
  let {
    scene,
    etched = [],
    films = [],
    normal,
    at,
    height = 300,
    quiet = false,
    maxStretch = 8,
    names = true,
    fills = {},
    colours = {},
  }: { scene: Scene; etched?: SceneEtched[]; films?: FrameFilm[]; normal: "x" | "y"; at: number; height?: number; quiet?: boolean; maxStretch?: number; names?: boolean; fills?: Record<string, string>; colours?: Record<string, string> } = $props();
  // `colours`, a film's colour by its name (the Fabrication page's, its 3D view's too), paints a
  // medium in place of the classes by permittivity; air stays unpainted
  /** A medium's fill: one `fills` names for its material (a metal the optical model draws as oxide), else by its permittivity. */
  const fill = (m: { material: string; eps: number }) => (paint(m) ? "" : (fills[m.material] ?? fillOf(m.eps)));
  /** A medium's paint from `colours`, as a style, or "" without one: the clear films (ε below 3, an oxide, a resist) lighter, a metal not, though drawn with an oxide's optics. */
  function paint(m: { material: string; eps: number }): string {
    const c = m.material === "air" ? undefined : colours[m.material];
    const clear = m.eps < 3 && !/^(TiN|Cu\/Au|Au|Cu|Al|Ti|Pt|Cr|NiCr|W)$/.test(m.material);
    return c ? `fill: ${c}; fill-opacity: ${clear ? 0.35 : 0.95}; stroke: ${c}` : "";
  }

  let width = $state(400);
  // the layers' names go at the right, beyond the window
  const pad = $derived({ left: 44, right: names ? 118 : 12, top: 10, bottom: 30 });
  const pieces = $derived(cut(scene, normal, at));
  /** Across the cut: around the shapes it meets with 1 µm to spare, or the whole window when it meets none. */
  const across = $derived.by((): [number, number] => {
    const win = normal === "x" ? scene.y_um : scene.x_um;
    // and the ends of the films' pieces inside the window (a conformal film reaching out past the shapes)
    const ends = films.flatMap((f) => cut({ ...scene, shapes: [{ layer: "", outline: f.top }, { layer: "", outline: f.bottom }] }, normal, at).flatMap((p) => [p.from, p.to])).filter((u) => u > win[0] + 1e-6 && u < win[1] - 1e-6);
    if (!pieces.length && !ends.length) return win;
    const lo = Math.min(...pieces.map((p) => p.from), ...ends) - 1;
    const hi = Math.max(...pieces.map((p) => p.to), ...ends) + 1;
    return [Math.max(win[0], lo), Math.min(win[1], hi)];
  });
  const z = $derived(scene.z_um);
  const fitX = $derived(Math.max(1e-6, (width - pad.left - pad.right) / (across[1] - across[0] || 1)));
  const fitZ = $derived(Math.max(1e-6, (height - pad.top - pad.bottom) / (z[1] - z[0] || 1)));
  /** True to scale where it fits; a wide cut (a ring's, across its buses) has its height stretched, at most 8 times, and says so. */
  const scale = $derived(Math.min(fitX, fitZ));
  const scaleZ = $derived(Math.min(fitZ, Math.max(scale, maxStretch * scale)));
  const stretch = $derived(scaleZ / scale);
  const w = $derived((across[1] - across[0]) * scale);
  const h = $derived((z[1] - z[0]) * scaleZ);
  const ox = $derived(pad.left + Math.max(0, (width - pad.left - pad.right - w) / 2));
  const oy = $derived(pad.top + Math.max(0, (height - pad.top - pad.bottom - h) / 2));
  const X = (u: number) => ox + (u - across[0]) * scale;
  const Z = (v: number) => oy + (z[1] - v) * scaleZ;
  const clip = `cut-${Math.random().toString(36).slice(2)}`;

  const top = $derived(scene.layers.length ? scene.layers[scene.layers.length - 1].z_um[1] : 0);
  /** The layers' names at the right: an etched layer's bands (`name 2`, `name 3`, … below it) under its one name, over its whole thickness. */
  const labels = $derived.by(() => {
    const out: { name: string; z_um: [number, number]; material: string; background: string }[] = [];
    for (const l of scene.layers) {
      const base = l.name.replace(/ \d+$/, "");
      const into = base !== l.name ? out.find((o) => o.name === base) : undefined;
      if (into) into.z_um = [Math.min(into.z_um[0], l.z_um[0]), Math.max(into.z_um[1], l.z_um[1])];
      else out.push({ name: l.name, z_um: [l.z_um[0], l.z_um[1]], material: l.material.material, background: l.background.material });
    }
    return out;
  });
  const band = (from: number, to: number) => ({ y: Z(Math.min(z[1], to)), height: Math.max(0, (Math.min(z[1], to) - Math.max(z[0], from)) * scaleZ) });
  /** The etched layers: the scene draws them in bands (`name 2`, `name 3`, … below), a staircase; their shapes are drawn exactly instead. */
  const etchedLayers = $derived(new Set(etched.map((e) => e.layer)));
  const banded = (name: string) => etchedLayers.has(name) || etchedLayers.has(name.replace(/ \d+$/, ""));
  /** The exact shapes the cut meets, each with its etch. */
  const exact = $derived(cut({ ...scene, shapes: etched.map((e) => ({ layer: e.layer, outline: e.outline })) }, normal, at).map((p) => ({ ...p, e: etched[p.shape] })));
  /** The intervals the cut meets of one outline. */
  const crossing = (outline: [number, number][]) => cut({ ...scene, shapes: [{ layer: "", outline }] }, normal, at);
  /** Each film piece the cut meets, as SVG polygons: an interval at its bottom joined to the same interval at its top (the top's outline is the bottom's moved in or out), or, where the two don't match up, the top's intervals over its height. */
  const filmCuts = $derived.by(() => {
    const out: { points: string; film: string; eps: number }[] = [];
    for (const f of films) {
      const [b, t] = [crossing(f.bottom), crossing(f.top)];
      const [z0, z1] = f.z_um;
      const pairs = b.length === t.length ? b.map((p, k) => [p, t[k]] as const) : t.map((p) => [p, p] as const);
      for (const [lo, hi] of pairs) {
        const pts = [[lo.from, z0], [hi.from, z1], [hi.to, z1], [lo.to, z0]];
        out.push({ points: pts.map(([u, v]) => `${X(u)},${Z(v)}`).join(" "), film: f.film, eps: f.eps });
      }
    }
    return out;
  });
  /** The films' names at the right, each layer's where it rises above the planar layers' top. */
  const filmLabels = $derived.by(() => {
    const out: { layer: string; films: string[]; z_um: [number, number] }[] = [];
    for (const f of films) {
      const o = out.find((x) => x.layer === f.layer);
      if (o) {
        o.z_um = [Math.min(o.z_um[0], f.z_um[0]), Math.max(o.z_um[1], f.z_um[1])];
        if (!o.films.includes(f.film)) o.films.push(f.film);
      } else out.push({ layer: f.layer, films: [f.film], z_um: [f.z_um[0], f.z_um[1]] });
    }
    return out.filter((o) => o.z_um[1] > top + 1e-9).map((o) => ({ ...o, z_um: [Math.max(top, o.z_um[0]), o.z_um[1]] as [number, number] }));
  });
  /** A layer whose shapes are air (trenches not yet filled): its background is drawn around them only, so the trenches show. */
  const around = (l: Scene["layers"][number]) => {
    if (l.material.eps >= 1.05 || l.material.material !== "air") return null;
    const hit = pieces.filter((p) => p.layer === l.name).sort((p, q) => p.from - q.from);
    if (!hit.length) return null;
    const out: [number, number][] = [];
    let from = across[0];
    for (const p of hit) {
      if (p.from > from) out.push([from, Math.min(p.from, across[1])]);
      from = Math.max(from, p.to);
    }
    if (from < across[1]) out.push([from, across[1]]);
    return out;
  };

  /** A cut piece etched by `e` on its layer `[z0, z1]`: its trapezoid's corners, as SVG points. */
  function trapezoid(from: number, to: number, z0: number, z1: number, e: SceneEtched): string {
    const depth = Math.min(e.depth_um, z1 - z0);
    const run = e.sidewall_deg >= 90 ? 0 : depth / Math.tan((e.sidewall_deg * Math.PI) / 180);
    const floor = z1 - depth;
    return [[from - run, floor], [from, z1], [to, z1], [to + run, floor]].map(([u, v]) => `${X(u)},${Z(v)}`).join(" ");
  }
</script>

<div bind:clientWidth={width} class="w-full">
  <svg {width} {height} class="block" role="img" aria-label="the cross-section at the cut">
    <defs><clipPath id={clip}><rect x={X(across[0])} y={Z(z[1])} width={w} height={h} /></clipPath></defs>
    <g clip-path="url(#{clip})">
      <!-- the substrate, the layers' backgrounds, the cladding -->
      <rect x={X(across[0])} width={w} {...band(z[0] - 1e3, 0)} class={fill(scene.substrate) ?? "fill-none"} style={paint(scene.substrate)} stroke-width="0" />
      {#each scene.layers as l (l.name)}
        {@const gaps = around(l)}
        {#if gaps}
          {#each gaps as [a, b], k (k)}
            <rect x={X(a)} width={Math.max(0, (b - a) * scale)} {...band(l.z_um[0], l.z_um[1])} class={fill(l.background) ?? "fill-none"} style={paint(l.background)} stroke-width="0" />
          {/each}
        {:else}
          <rect x={X(across[0])} width={w} {...band(l.z_um[0], l.z_um[1])} class={fill(l.background) ?? "fill-none"} style={paint(l.background)} stroke-width="0" />
        {/if}
        <line x1={X(across[0])} x2={X(across[1])} y1={Z(l.z_um[0])} y2={Z(l.z_um[0])} class="stroke-base-content/25" stroke-dasharray="3 3" />
      {/each}
      <rect x={X(across[0])} width={w} {...band(top, z[1] + 1e3)} class={fill(scene.cladding) ?? "fill-none"} style={paint(scene.cladding)} stroke-width="0" />
      <line x1={X(across[0])} x2={X(across[1])} y1={Z(top)} y2={Z(top)} class="stroke-base-content/25" stroke-dasharray="3 3" />
      <!-- the films over the topography, under the shapes they cover -->
      {#each filmCuts as c, k (k)}
        {@const m = { material: c.film, eps: c.eps }}
        <polygon points={c.points} class={fill(m) ?? "fill-none"} style={paint(m)} stroke-width="0.75" />
      {/each}
      <!-- the shapes the cut meets -->
      {#each pieces as p, k (k)}
        {@const l = scene.layers.find((x) => x.name === p.layer)}
        {#if l && !banded(l.name)}
          <rect x={X(p.from)} width={(p.to - p.from) * scale} {...band(l.z_um[0], l.z_um[1])} class={fill(l.material) ?? "fill-none"} style={paint(l.material)} stroke-width="1" />
        {/if}
      {/each}
      {#each exact as p, k (k)}
        {@const l = scene.layers.find((x) => x.name === p.layer)}
        {#if l}
          <polygon points={trapezoid(p.from, p.to, p.e.z_um[0], p.e.z_um[1], p.e)} class={fill(l.material) ?? "fill-none"} style={paint(l.material)} stroke-width="1" />
        {/if}
      {/each}
    </g>
    <rect x={X(across[0])} y={Z(z[1])} width={w} height={h} class="fill-none stroke-base-content/20" />
    <!-- the layers' names, at the right -->
    {#each names ? labels : [] as l (l.name)}
      {#if l.z_um[1] > z[0] && l.z_um[0] < z[1]}
        <text x={X(across[1]) + 6} y={(Z(Math.min(z[1], l.z_um[1])) + Z(Math.max(z[0], l.z_um[0]))) / 2} dominant-baseline="central" class="fill-base-content/60 text-[10px]">{l.name}: {l.material}{l.background !== l.material ? ` in ${l.background}` : ""}</text>
      {/if}
    {/each}
    {#each names ? filmLabels : [] as l (l.layer)}
      {#if l.z_um[0] < z[1]}
        <text x={X(across[1]) + 6} y={(Z(Math.min(z[1], l.z_um[1])) + Z(l.z_um[0])) / 2} dominant-baseline="central" class="fill-base-content/60 text-[10px]">{l.layer}: {l.films.join(", ")}</text>
      {/if}
    {/each}
    {#if names}
    <text x={X(across[1]) + 6} y={Z(z[1]) + 8} dominant-baseline="central" class="fill-base-content/60 text-[10px]">cladding: {scene.cladding.material}</text>
    <text x={X(across[1]) + 6} y={Z(z[0]) - 8} dominant-baseline="central" class="fill-base-content/60 text-[10px]">substrate: {scene.substrate.material}</text>
    {/if}
    {#each ticks(across[0], across[1], Math.max(2, Math.round(w / 90))) as t (t)}
      <line x1={X(t)} x2={X(t)} y1={Z(z[0])} y2={Z(z[0]) + 4} class="stroke-base-content/40" />
      <text x={X(t)} y={Z(z[0]) + 15} text-anchor="middle" class="fill-base-content/55 text-[10px] num">{tick(t)}</text>
    {/each}
    {#each ticks(z[0], z[1], Math.max(2, Math.round(h / 40))) as t (t)}
      <line x1={X(across[0]) - 4} x2={X(across[0])} y1={Z(t)} y2={Z(t)} class="stroke-base-content/40" />
      <text x={X(across[0]) - 6} y={Z(t)} text-anchor="end" dominant-baseline="central" class="fill-base-content/55 text-[10px] num">{tick(t)}</text>
    {/each}
    <text x={X(across[1])} y={height - 3} text-anchor="end" class="fill-base-content/45 text-[10px]">{normal === "x" ? "y" : "x"} ({unitText()}), z up{stretch > 1.05 ? `, z stretched ${stretch.toFixed(1)}×` : ""}</text>
    {#if !pieces.length && !quiet}
      <text x={X((across[0] + across[1]) / 2)} y={Z((z[0] + z[1]) / 2)} text-anchor="middle" class="fill-base-content/50 text-xs">the cut meets no shape here</text>
    {/if}
  </svg>
</div>
