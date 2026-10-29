<script lang="ts">
  // A TFLN modulator's electrode field on its cross-section, by its 2D model
  // (studio/src-tauri/src/devices/modulator.rs, photonoxide's TflnModulator::electrode_field):
  // the static field |E| at 1 V on the signal electrode, the ground at 0, from black to pale
  // yellow (its colours up to where 99% of its pixels are, past the corners' singular field), its
  // direction as arrows, the rib and the electrodes, and the optical mode's |E|² at half and 1/e²
  // of its peak over it: where the two overlap is what sets Vπ·L. One arm's gap; the other arm is
  // its mirror. Whatever model the page uses, the field is the 2D model's: the same solves as its
  // Vπ·L.
  import { api, type DeviceSpec, type ElectrodeField } from "../../lib/api";
  import { quantile, range } from "../../lib/colours";
  import { plain } from "../../lib/deviceview";
  import { label as tick } from "../../lib/plot";
  import RasterView from "../RasterView.svelte";

  let { spec, height = 300 }: { spec: DeviceSpec; height?: number } = $props();

  /** The share of |E|'s pixels its colours span: the electrodes' corners are singular, and would leave the gap dark. */
  const CAP = 0.99;
  /** The arrows drawn: where |E| is at least this share of the colours' top. */
  const FAINTEST = 0.03;

  /** The design at the 2D model: the field doesn't change with the model the page shows. */
  const asked = $derived(JSON.stringify({ ...spec, fidelity: "2D", compact: undefined }));
  let shown = $state<ElectrodeField | null>(null);
  let problem = $state("");
  let busy = $state(false);
  $effect(() => {
    const s = JSON.parse(asked) as DeviceSpec;
    let current = true;
    busy = true;
    const timer = setTimeout(() => {
      api
        .deviceElectrodeField(s)
        .then((e) => {
          if (!current) return;
          shown = e;
          problem = "";
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

  const ends = $derived(shown ? range(shown.raster) : [0, 0]);
  const top = $derived(shown ? Math.min(ends[1], quantile(shown.raster, CAP)) : 0);
  const fmt = (v: number) => Number(v.toPrecision(3)).toLocaleString("en", { maximumSignificantDigits: 3 });
  /** A colour bar's end: two digits. */
  const short = (v: number) => (Math.abs(v) >= 1e4 ? v.toExponential(1) : tick(v));
  /** The arrows as lines: a shaft along E, its length a share of the arrows' spacing, and a head. */
  const arrows = $derived.by(() => {
    if (!shown || shown.arrows.length < 2) return [];
    const r = shown.raster;
    const spacing = (r.x1 - r.x0) / Math.max(1, new Set(shown.arrows.map((a) => a[0][0])).size);
    const length = 0.7 * spacing;
    const out: [number, number][][] = [];
    for (const [[x, y], [ex, ey]] of shown.arrows) {
      const size = Math.hypot(ex, ey);
      if (!(size >= FAINTEST * top) || size === 0) continue;
      const [ux, uy] = [ex / size, ey / size];
      const tail: [number, number] = [x - (0.5 * length) * ux, y - (0.5 * length) * uy];
      const tip: [number, number] = [x + (0.5 * length) * ux, y + (0.5 * length) * uy];
      const head = 0.35 * length;
      const side = (s: number): [number, number] => [tip[0] - head * (ux * Math.cos(0.5) - s * uy * Math.sin(0.5)), tip[1] - head * (uy * Math.cos(0.5) + s * ux * Math.sin(0.5))];
      out.push([tail, tip], [side(1), tip, side(-1)]);
    }
    return out;
  });
  const lines = $derived(
    shown
      ? [
          { lines: arrows, tone: "level" as const },
          { lines: shown.mode[0]?.lines ?? [], tone: "mode" as const },
          { lines: shown.mode[1]?.lines ?? [], tone: "mode-faint" as const },
        ]
      : [],
  );
  const outline = $derived(
    shown
      ? {
          shapes: shown.shapes,
          interfaces: shown.interfaces.map((y) => [
            [shown!.raster.x0, y],
            [shown!.raster.x1, y],
          ]) as [number, number][][],
        }
      : undefined,
  );
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
  {#if problem}
    <p class="text-sm text-warning selectable">{plain(problem)}</p>
  {:else if shown}
    <p class="text-xs muted">
      The 2D model's static field at <span class="num">1 V</span> on the signal electrode, the ground at 0: under the mode's peak
      <span class="num">{fmt(shown.at_mode)} V/m</span>. Its overlap with the mode gives Vπ·L <span class="num">{shown.vpi_l.toFixed(3)} V·cm</span> push–pull (n<sub>eff</sub>
      <span class="num">{shown.n_eff.toFixed(4)}</span>).
      {#if busy}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
    </p>
    <RasterView
      raster={shown.raster}
      kind="intensity"
      axes={["x", "y"]}
      maxHeight={Math.max(120, height - 100)}
      quantity="|E| at 1 V (V/m)"
      {outline}
      {lines}
      scale={[ends[0], top]}
      ends={[`${short(ends[0])} V/m`, `${top < ends[1] ? "≥ " : ""}${short(top)} V/m`]}
    />
    <p class="text-[11px] faint">
      |E| from <span class="num">{fmt(ends[0])}</span> (black) to <span class="num">{fmt(top)} V/m</span> (pale yellow){#if top < ends[1]}, where 99% of the pixels are: the electrodes'
        corners reach <span class="num">{fmt(ends[1])} V/m</span>{/if}; zero inside a conductor. One arm's gap, the ground on the left and the signal on the right; the other arm is its mirror. x across
      from the rib's centre, y up from the film's bottom. White: the rib, the electrodes and the layers' interfaces (dashed); small arrows, E's direction where it is over {FAINTEST * 100}% of the
      top. Teal: the optical mode's |E|² at half its peak (solid) and at 1/e² (dashed). {shown.grid[0].toUpperCase() + shown.grid.slice(1)}.
    </p>
  {:else}
    <div class="skeleton flex-1" aria-busy="true" aria-label="Solving the electrode field"></div>
  {/if}
</div>
