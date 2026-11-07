<script lang="ts">
  // A photon's loss budget as a waterfall: each stage a bar from the loss before it to the loss
  // after, in dB, the total from 0 under them, and the published thresholds on a photon's
  // transmission as dashed lines. Hover a row for where its number comes from.
  import type { BudgetRoute, BudgetView } from "../../lib/api";
  import { label, ticks } from "../../lib/plot";
  import { axisTo, dB, percent, waterfall } from "../../lib/quantum";

  let { route, thresholds }: { route: BudgetRoute; thresholds: BudgetView["thresholds"] } = $props();

  let width = $state(520);
  const LABEL = 150;
  const VALUE = 112;
  const ROW = 22;
  const TOP = 18;
  const fall = $derived(waterfall(route.stages));
  const most = $derived(axisTo(fall.total, thresholds.map((t) => t.loss_db)));
  const span = $derived(Math.max(40, width - LABEL - VALUE));
  const at = (db: number) => LABEL + (Math.max(0, db) / most) * span;
  const totalY = $derived(TOP + fall.falls.length * ROW);
  /** Under the total, a line for each threshold's name. */
  const height = $derived(totalY + ROW + 6 + thresholds.length * 13);
  const cut = (t: string, n: number) => (t.length > n ? `${t.slice(0, n - 1)}…` : t);
</script>

<div bind:clientWidth={width}>
  <svg {width} {height} class="block" role="img" aria-label="The loss budget, stage by stage: a total of {dB(route.loss_db)}">
    <!-- the axis: loss in dB, growing to the right -->
    {#each ticks(0, most, 5) as t (t)}
      <line x1={at(t)} x2={at(t)} y1={TOP - 4} y2={totalY + ROW} class="stroke-base-content/8" />
      <text x={at(t)} y={TOP - 7} text-anchor="middle" class="fill-base-content/45 text-[10px] num">{label(t)}</text>
    {/each}
    <text x={LABEL + span + 14} y={TOP - 7} class="fill-base-content/45 text-[10px]">dB</text>

    {#each fall.falls as f, k (k)}
      {@const y = TOP + k * ROW}
      <g>
        <title>{f.name}: keeps {percent(f.transmission)}, {dB(f.loss)}. {f.source}</title>
        <rect x="0" {y} {width} height={ROW} class="fill-transparent" />
        <text x="0" y={y + 15} class="fill-base-content/80 text-[11px] num">{cut(f.name, 24)}</text>
        <!-- the loss so far, carried down from the stage before -->
        {#if k > 0}<line x1={at(f.from)} x2={at(f.from)} y1={y - 5} y2={y + 5} class="stroke-base-content/35" stroke-dasharray="2 2" />{/if}
        <rect x={at(f.from)} y={y + 5} width={Math.max(1.5, at(f.to) - at(f.from))} height="12" rx="2" class={f.stated ? "fill-secondary/70" : "fill-primary/75"} />
        <text x={width} y={y + 15} text-anchor="end" class="fill-base-content/70 text-[11px] num">{dB(f.loss)} · {percent(f.transmission)}</text>
      </g>
    {/each}

    <!-- the total, from 0 -->
    <line x1="0" x2={width} y1={totalY + 1} y2={totalY + 1} class="stroke-base-content/15" />
    <text x="0" y={totalY + 16} class="fill-base-content text-[11px] font-semibold">Total</text>
    <rect x={at(0)} y={totalY + 6} width={Math.max(1.5, at(fall.total) - at(0))} height="12" rx="2" class="fill-base-content/55" />
    <text x={width} y={totalY + 16} text-anchor="end" class="fill-base-content text-[11px] font-semibold num">{dB(route.loss_db)} · {percent(route.transmission)}</text>

    <!-- the thresholds: met when the total is to their left -->
    {#each thresholds as t, k (t.name)}
      {@const met = route.transmission > t.transmission}
      <g>
        <title>{t.name}: a photon keeps more than {percent(t.transmission)} ({dB(t.loss_db)}), {met ? "met" : "not met"} here. {t.assumptions}</title>
        <line x1={at(t.loss_db)} x2={at(t.loss_db)} y1={TOP - 2} y2={totalY + ROW + 6 + k * 13} class={met ? "stroke-success" : "stroke-error"} stroke-width="1.4" stroke-dasharray="5 3" />
        <text x={at(t.loss_db) + 4} y={totalY + ROW + 15 + k * 13} class="{met ? 'fill-success' : 'fill-error'} text-[10px]">{t.name.split(/[ ,]/)[0]}: {dB(t.loss_db)}, {met ? "met" : "not met"}</text>
      </g>
    {/each}
  </svg>
</div>
