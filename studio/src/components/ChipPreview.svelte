<script lang="ts">
  // A chip drawn small and still: its parts, wires and external ports, fitted to the box.
  import type { Chip } from "../lib/api";
  import { bounds, externalPin, pinsOf, tag, tagLabel, wirePath, type WorldPin } from "../lib/circuit";
  import { allParts, library, loadLibrary, partKey } from "../lib/chip.svelte";
  import Glyph from "./Glyph.svelte";

  let { chip, height = 160 }: { chip: Chip; height?: number } = $props();

  loadLibrary();

  const kinds = $derived(allParts());
  const box = $derived(bounds(chip, kinds) ?? [-50, -50, 50, 50]);
  const pins = $derived.by(() => {
    const all = new Map<string, WorldPin>();
    for (const inst of chip.instance) for (const p of pinsOf(inst, kinds.get(partKey(inst)))) all.set(p.ref, p);
    chip.port.forEach((e, k) => all.set(`#${k}`, externalPin(e, k)));
    return all;
  });
  const wires = $derived([
    ...chip.connections.map(([a, b]) => [pins.get(a), pins.get(b)]),
    ...chip.port.map((p, k) => [pins.get(`#${k}`), p.at ? pins.get(p.at) : undefined]),
  ].filter(([a, b]) => a && b) as [WorldPin, WorldPin][]);
</script>

<svg class="block w-full" {height} viewBox="{box[0] - 16} {box[1] - 16} {box[2] - box[0] + 32} {box[3] - box[1] + 32}" role="img" aria-label="{chip.name}'s schematic">
  {#if library.loaded}
    {#each wires as [a, b], k (k)}
      <path d={wirePath(a, b)} class="fill-none stroke-base-content/50" stroke-width="1.6" />
    {/each}
    {#each chip.instance as inst (inst.name)}
      {@const k = kinds.get(partKey(inst))}
      {#if k}
        <g transform="translate({inst.x} {inst.y}) rotate({inst.rotation ?? 0}) scale(1 {inst.mirror ? -1 : 1})"><Glyph symbol={k.symbol} /></g>
      {/if}
    {/each}
    {#each chip.port as p, i (i)}
      <g transform="translate({p.x} {p.y}) rotate({p.rotation ?? 0})">
        <path d={tag(p.name).d} class="fill-accent/15 stroke-accent" stroke-width="1.2" />
      </g>
      <text x={tagLabel(p)[0]} y={tagLabel(p)[1]} text-anchor="middle" dominant-baseline="central" class="fill-base-content/80 text-[8px] font-semibold num">{p.name}</text>
    {/each}
  {/if}
</svg>
