<script lang="ts">
  // A device's cross-section where the top view's cut is: the substrate, each layer with its
  // shapes cut across and its background around them, and the cladding, true to scale (a wide
  // cut, across a ring and its buses, has its height stretched and says by how much), from
  // 1 µm below the core to 1 µm above it.
  import { cut, fillOf, tick } from "../../lib/deviceview";
  import type { Scene } from "../../lib/events";
  import { ticks } from "../../lib/plot";
  import { unitText } from "../../lib/units";

  let { scene, normal, at, height = 300 }: { scene: Scene; normal: "x" | "y"; at: number; height?: number } = $props();

  let width = $state(400);
  // the layers' names go at the right, beyond the window
  const pad = { left: 44, right: 118, top: 10, bottom: 30 };
  const pieces = $derived(cut(scene, normal, at));
  /** Across the cut: around the shapes it meets with 1 µm to spare, or the whole window when it meets none. */
  const across = $derived.by((): [number, number] => {
    const win = normal === "x" ? scene.y_um : scene.x_um;
    if (!pieces.length) return win;
    const lo = Math.min(...pieces.map((p) => p.from)) - 1;
    const hi = Math.max(...pieces.map((p) => p.to)) + 1;
    return [Math.max(win[0], lo), Math.min(win[1], hi)];
  });
  const z = $derived(scene.z_um);
  const fitX = $derived(Math.max(1e-6, (width - pad.left - pad.right) / (across[1] - across[0] || 1)));
  const fitZ = $derived(Math.max(1e-6, (height - pad.top - pad.bottom) / (z[1] - z[0] || 1)));
  /** True to scale where it fits; a wide cut (a ring's, across its buses) has its height stretched, at most 8 times, and says so. */
  const scale = $derived(Math.min(fitX, fitZ));
  const scaleZ = $derived(Math.min(fitZ, Math.max(scale, 8 * scale)));
  const stretch = $derived(scaleZ / scale);
  const w = $derived((across[1] - across[0]) * scale);
  const h = $derived((z[1] - z[0]) * scaleZ);
  const ox = $derived(pad.left + Math.max(0, (width - pad.left - pad.right - w) / 2));
  const oy = $derived(pad.top + Math.max(0, (height - pad.top - pad.bottom - h) / 2));
  const X = (u: number) => ox + (u - across[0]) * scale;
  const Z = (v: number) => oy + (z[1] - v) * scaleZ;
  const clip = `cut-${Math.random().toString(36).slice(2)}`;

  const top = $derived(scene.layers.length ? scene.layers[scene.layers.length - 1].z_um[1] : 0);
  const band = (from: number, to: number) => ({ y: Z(Math.min(z[1], to)), height: Math.max(0, (Math.min(z[1], to) - Math.max(z[0], from)) * scaleZ) });
</script>

<div bind:clientWidth={width} class="w-full">
  <svg {width} {height} class="block" role="img" aria-label="the cross-section at the cut">
    <defs><clipPath id={clip}><rect x={X(across[0])} y={Z(z[1])} width={w} height={h} /></clipPath></defs>
    <g clip-path="url(#{clip})">
      <!-- the substrate, the layers' backgrounds, the cladding -->
      <rect x={X(across[0])} width={w} {...band(z[0] - 1e3, 0)} class={fillOf(scene.substrate.eps) ?? "fill-none"} stroke-width="0" />
      {#each scene.layers as l (l.name)}
        <rect x={X(across[0])} width={w} {...band(l.z_um[0], l.z_um[1])} class={fillOf(l.background.eps) ?? "fill-none"} stroke-width="0" />
        <line x1={X(across[0])} x2={X(across[1])} y1={Z(l.z_um[0])} y2={Z(l.z_um[0])} class="stroke-base-content/25" stroke-dasharray="3 3" />
      {/each}
      <rect x={X(across[0])} width={w} {...band(top, z[1] + 1e3)} class={fillOf(scene.cladding.eps) ?? "fill-none"} stroke-width="0" />
      <line x1={X(across[0])} x2={X(across[1])} y1={Z(top)} y2={Z(top)} class="stroke-base-content/25" stroke-dasharray="3 3" />
      <!-- the shapes the cut meets -->
      {#each pieces as p, k (k)}
        {@const l = scene.layers.find((x) => x.name === p.layer)}
        {#if l}
          <rect x={X(p.from)} width={(p.to - p.from) * scale} {...band(l.z_um[0], l.z_um[1])} class={fillOf(l.material.eps) ?? "fill-none"} stroke-width="1" />
        {/if}
      {/each}
    </g>
    <rect x={X(across[0])} y={Z(z[1])} width={w} height={h} class="fill-none stroke-base-content/20" />
    <!-- the layers' names, at the right -->
    {#each scene.layers as l (l.name)}
      {#if l.z_um[1] > z[0] && l.z_um[0] < z[1]}
        <text x={X(across[1]) + 6} y={(Z(Math.min(z[1], l.z_um[1])) + Z(Math.max(z[0], l.z_um[0]))) / 2} dominant-baseline="central" class="fill-base-content/60 text-[10px]">{l.name}: {l.material.material}{l.background.material !== l.material.material ? ` in ${l.background.material}` : ""}</text>
      {/if}
    {/each}
    <text x={X(across[1]) + 6} y={Z(z[1]) + 8} dominant-baseline="central" class="fill-base-content/60 text-[10px]">cladding: {scene.cladding.material}</text>
    <text x={X(across[1]) + 6} y={Z(z[0]) - 8} dominant-baseline="central" class="fill-base-content/60 text-[10px]">substrate: {scene.substrate.material}</text>
    {#each ticks(across[0], across[1], Math.max(2, Math.round(w / 90))) as t (t)}
      <line x1={X(t)} x2={X(t)} y1={Z(z[0])} y2={Z(z[0]) + 4} class="stroke-base-content/40" />
      <text x={X(t)} y={Z(z[0]) + 15} text-anchor="middle" class="fill-base-content/55 text-[10px] num">{tick(t)}</text>
    {/each}
    {#each ticks(z[0], z[1], Math.max(2, Math.round(h / 40))) as t (t)}
      <line x1={X(across[0]) - 4} x2={X(across[0])} y1={Z(t)} y2={Z(t)} class="stroke-base-content/40" />
      <text x={X(across[0]) - 6} y={Z(t)} text-anchor="end" dominant-baseline="central" class="fill-base-content/55 text-[10px] num">{tick(t)}</text>
    {/each}
    <text x={X(across[1])} y={height - 3} text-anchor="end" class="fill-base-content/45 text-[10px]">{normal === "x" ? "y" : "x"} ({unitText()}), z up{stretch > 1.05 ? `, z stretched ${stretch.toFixed(1)}×` : ""}</text>
    {#if !pieces.length}
      <text x={X((across[0] + across[1]) / 2)} y={Z((z[0] + z[1]) / 2)} text-anchor="middle" class="fill-base-content/50 text-xs">the cut meets no shape here</text>
    {/if}
  </svg>
</div>
