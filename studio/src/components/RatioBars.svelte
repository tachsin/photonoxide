<script lang="ts" module>
  export interface Ratio {
    key: string;
    label: string;
    /** What the row is, under its label: its grid. */
    detail: string;
    value: number;
    colour: string;
    info: string[];
  }
</script>

<script lang="ts">
  // Ratios against 1 (photonoxide's own) as bars on a log scale: right of the line is faster
  // (or leaner), left is slower. Hover a bar for what it measured.
  let { rows, better = "faster", worse = "slower" }: { rows: Ratio[]; better?: string; worse?: string } = $props();

  let width = $state(520);
  let hover = $state<Ratio | null>(null);
  const LABEL = 230;
  const span = $derived(Math.max(1, ...rows.map((r) => Math.ceil(Math.abs(Math.log10(r.value)) * 10) / 10)));
  const mid = $derived(LABEL + (width - LABEL - 70) / 2);
  const scale = $derived((width - LABEL - 70) / 2 / span);
  const at = (v: number) => mid + Math.log10(v) * scale;
</script>

<div class="relative" bind:clientWidth={width}>
  <div class="mb-1 flex text-[11px] faint" style="padding-left: {LABEL}px; padding-right: 70px">
    <span class="flex-1">← {worse}</span><span>{better} →</span>
  </div>
  <svg {width} height={rows.length * 30 + 6} class="block" role="img" aria-label="Ratios against photonoxide's own">
    <line x1={mid} x2={mid} y1="0" y2={rows.length * 30 + 6} class="stroke-base-content/30" />
    {#each rows as r, k (r.key)}
      {@const y = k * 30 + 4}
      {@const x = at(r.value)}
      <g role="presentation" onpointerenter={() => (hover = r)} onpointerleave={() => (hover = null)}>
        <rect x="0" y={y - 2} {width} height="28" class="fill-transparent" />
        <text x="0" y={y + 9} class="fill-base-content/80 text-[11.5px] font-mono">{r.label.length > 34 ? `${r.label.slice(0, 33)}…` : r.label}</text>
        <text x="0" y={y + 22} class="fill-base-content/45 text-[10px]">{r.detail.length > 42 ? `${r.detail.slice(0, 41)}…` : r.detail}</text>
        <rect x={Math.min(mid, x)} y={y + 4} width={Math.max(2, Math.abs(x - mid))} height="16" rx="3" fill={r.colour} opacity={hover && hover !== r ? 0.45 : 1} />
        <text x={Math.max(mid, x) + 6} y={y + 16} class="fill-base-content/75 text-[11px] num">{r.value.toFixed(2)}×</text>
      </g>
    {/each}
  </svg>
  {#if hover}
    {@const k = rows.indexOf(hover)}
    <div class="pointer-events-none absolute z-10 max-w-80 rounded-lg border border-base-content/10 bg-base-100/95 px-3 py-2 text-xs shadow-lg" style="left: {Math.min(mid + 20, width - 300)}px; top: {k * 30 + 40}px">
      <div class="mb-1 font-medium">{hover.label}: {hover.value.toFixed(2)}×</div>
      {#each hover.info as line, j (j)}<div class="muted">{line}</div>{/each}
    </div>
  {/if}
</div>
