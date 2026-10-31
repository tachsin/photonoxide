<script lang="ts">
  // Every catalogue material's transparency window as a band over wavelength (a log axis), every
  // edge with its source, or drawn faint where no paper in hand specifies it; a process's
  // wavelengths marked on top, so which material can carry it shows at a glance.
  import type { TransparencyEdge, TransparencyWindow } from "../../lib/api";
  import { SERIES } from "../../lib/plot";

  let {
    windows,
    marks = [],
  }: {
    windows: TransparencyWindow[];
    /** The process's waves: each role at its wavelength, µm. */
    marks?: { label: string; um: number }[];
  } = $props();

  const LO = 0.2;
  const HI = 12;
  const L = 150;
  const R = 12;
  const ROW = 20;
  const T = 26;
  const B = 28;
  let width = $state(700);
  const height = $derived(T + B + ROW * windows.length);
  const X = (um: number) => L + (Math.log(Math.min(Math.max(um, LO), HI) / LO) / Math.log(HI / LO)) * (width - L - R);
  const TICKS = [0.2, 0.3, 0.5, 1, 2, 3, 5, 10];

  /** What a row's edges say, for its tooltip. */
  function about(w: TransparencyWindow): string {
    const edge = (what: string, e: TransparencyEdge) =>
      `${what}: ${e.wavelength === null ? "" : `${e.wavelength.toFixed(3)} µm, `}${e.how}${e.source ? ` (${e.source.reference}, ${e.source.location})` : ""}`;
    return [
      `${w.name}${w.composition !== null ? `, x = ${w.composition}` : ""}`,
      edge("band edge", w.band_edge),
      edge("two-photon absorption below", w.two_photon),
      edge("multiphonon edge", w.multiphonon),
      `index data: ${w.index_range[0]} to ${w.index_range[1]} µm`,
    ].join("\n");
  }

  /** Whether every wave lies inside the material's specified window. */
  function carries(w: TransparencyWindow): boolean | null {
    if (!marks.length) return null;
    const lo = w.band_edge.wavelength;
    const hi = w.multiphonon.wavelength;
    return marks.every((m) => (lo === null || m.um > lo) && (hi === null || m.um < hi) && m.um >= w.index_range[0] && m.um <= w.index_range[1]);
  }
</script>

<div class="w-full" bind:clientWidth={width}>
  <svg {width} {height} class="block" role="img" aria-label="transparency windows over wavelength">
    {#each TICKS as t (t)}
      <line x1={X(t)} x2={X(t)} y1={T - 4} y2={height - B} class="stroke-base-content/10" />
      <text x={X(t)} y={height - B + 14} text-anchor="middle" class="fill-base-content/70 text-[10px]">{t}</text>
    {/each}
    <text x={(L + width - R) / 2} y={height - 4} text-anchor="middle" class="fill-base-content/80 text-[11px]">vacuum wavelength (µm)</text>
    {#each windows as w, k (w.id)}
      {@const y = T + k * ROW}
      {@const lo = w.band_edge.wavelength}
      {@const hi = w.multiphonon.wavelength}
      {@const ok = carries(w)}
      <g>
        <title>{about(w)}</title>
        <text x={L - 8} y={y + 13} text-anchor="end" class="text-[11px] {ok === false ? 'fill-base-content/40' : 'fill-base-content/85'}">{w.formula}{w.composition !== null ? ` (x ${w.composition})` : ""}</text>
        <!-- the index data's range, a thin line -->
        <line x1={X(w.index_range[0])} x2={X(w.index_range[1])} y1={y + 17} y2={y + 17} class="stroke-base-content/40" stroke-width="1.5" />
        <!-- the window: solid between specified edges, faint where an edge isn't specified -->
        <rect x={X(lo ?? w.index_range[0])} y={y + 4} width={Math.max(0, X(hi ?? w.index_range[1]) - X(lo ?? w.index_range[0]))} height="10" rx="2" class={lo !== null && hi !== null ? "fill-success/60" : "fill-success/25"} />
        {#if lo !== null && w.two_photon.wavelength !== null}
          <!-- two-photon absorption: from the band edge to twice its wavelength -->
          <rect x={X(lo)} y={y + 4} width={Math.max(0, X(w.two_photon.wavelength) - X(lo))} height="10" class="fill-warning/60" />
        {/if}
        {#if lo !== null}<line x1={X(lo)} x2={X(lo)} y1={y + 2} y2={y + 16} class="stroke-error" stroke-width="2" />{/if}
        {#if hi !== null}<line x1={X(hi)} x2={X(hi)} y1={y + 2} y2={y + 16} class="stroke-error" stroke-width="2" />{/if}
      </g>
    {/each}
    {#each marks as m, k (k)}
      <line x1={X(m.um)} x2={X(m.um)} y1={T - 6} y2={height - B} stroke={SERIES[k % SERIES.length]} stroke-width="1.5" stroke-dasharray="5 3" />
      <text x={X(m.um)} y={10 + (k % 2) * 9} text-anchor="middle" fill={SERIES[k % SERIES.length]} class="text-[10px]">{m.label} {m.um.toFixed(3)}</text>
    {/each}
  </svg>
  <p class="mt-1 text-xs faint">
    Green: transparent (faint where an edge has no paper in hand); amber: two-photon absorption, below twice the band edge's wavelength; red lines: the band and multiphonon edges; grey line: the range the index data covers. Hover a row for each edge's source.
  </p>
</div>
