<script lang="ts">
  // A component's schematic symbol, centred on the origin in the chip view's units: its outline
  // and its guides, drawn in the theme's colours. Pins and labels are the caller's.
  import type { Symbol } from "../lib/api";

  let { symbol, selected = false, faded = false }: { symbol: Symbol; selected?: boolean; faded?: boolean } = $props();

  const w = $derived(symbol.width);
  const h = $derived(symbol.height);
</script>

<g class:opacity-40={faded}>
  <rect
    x={-w / 2}
    y={-h / 2}
    width={w}
    height={h}
    rx="5"
    class="{selected ? 'fill-primary/12 stroke-primary' : 'fill-base-100 stroke-base-content/25'} transition-colors"
    stroke-width={selected ? 1.6 : 1}
  />
  <g class="fill-none stroke-secondary" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    {#if symbol.glyph === "waveguide"}
      <line x1={-w / 2} y1="0" x2={w / 2} y2="0" />
    {:else if symbol.glyph === "phase-shifter"}
      <line x1={-w / 2} y1="0" x2={w / 2} y2="0" />
      <rect x="-14" y="-7" width="28" height="4" rx="1" class="fill-accent/60 stroke-accent" stroke-width="0.8" />
      <text x="0" y="8" text-anchor="middle" dominant-baseline="central" class="fill-accent stroke-none text-[8px] italic">φ</text>
    {:else if symbol.glyph === "bend"}
      <path d="M -20 10 A 30 30 0 0 0 10 -20" />
    {:else if symbol.glyph === "coupler" && symbol.pins.length === 4}
      <path d="M -40 -10 C -22 -10 -20 -3 -8 -3 L 8 -3 C 20 -3 22 -10 40 -10" />
      <path d="M -40 10 C -22 10 -20 3 -8 3 L 8 3 C 20 3 22 10 40 10" />
    {:else if symbol.glyph === "mmi"}
      <rect x="-14" y="-14" width="28" height="28" rx="1" class="fill-secondary/15" />
      {#each symbol.pins as p (p.port)}
        <line x1={p.x} y1={p.y} x2={p.x < 0 ? -14 : 14} y2={p.y} />
      {/each}
    {:else if symbol.glyph === "y-branch"}
      <path d="M -30 0 L -12 0 C 4 0 8 -10 30 -10" />
      <path d="M -12 0 C 4 0 8 10 30 10" />
    {:else if symbol.glyph === "ring-all-pass"}
      <line x1="-30" y1="20" x2="30" y2="20" />
      <circle cx="0" cy="-3" r="17" />
    {:else if symbol.glyph === "ring-add-drop"}
      <line x1="-30" y1="30" x2="30" y2="30" />
      <line x1="-30" y1="-30" x2="30" y2="-30" />
      <circle cx="0" cy="0" r="22" />
    {:else if symbol.glyph === "mzi"}
      {#if symbol.pins.length === 4}
        <path d="M -40 -10 L -28 -10 C -20 -10 -20 -16 -12 -16 L 12 -16 C 20 -16 20 -10 28 -10 L 40 -10" />
        <path d="M -40 10 L -28 10 C -20 10 -20 16 -12 16 L 12 16 C 20 16 20 10 28 10 L 40 10" />
      {:else}
        <path d="M -40 0 L -26 0 C -18 0 -18 -12 -10 -12 L 10 -12 C 18 -12 18 0 26 0 L 40 0" />
        <path d="M -26 0 C -18 0 -18 12 -10 12 L 10 12 C 18 12 18 0 26 0" />
      {/if}
    {:else if symbol.glyph === "terminator"}
      <line x1="-10" y1="0" x2="-1" y2="0" />
      <path d="M -1 -6 L 6 0 L -1 6 Z" class="fill-secondary/40" stroke-width="1.4" />
    {:else}
      <!-- a box: S-parameters from data, or a component without a drawing of its own -->
      {#each symbol.pins as p (p.port)}
        <line x1={p.x} y1={p.y} x2={p.x < 0 ? p.x + 8 : p.x - 8} y2={p.y} />
      {/each}
      <text x="0" y="0" text-anchor="middle" dominant-baseline="central" class="fill-secondary stroke-none text-[11px] font-semibold italic">
        {symbol.glyph === "measured" ? "S·data" : "S"}
      </text>
    {/if}
  </g>
</g>
