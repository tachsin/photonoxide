<script lang="ts">
  // A TFLN modulator's electrode field on its cross-section, by its 2D model
  // (studio/src-tauri/src/devices/modulator.rs, photonoxide's TflnModulator::electrode_field):
  // the static field |E| at 1 V on the signal electrode, the ground at 0, from black to pale
  // yellow (its colours up to where 99% of its pixels are, past the corners' singular field), its
  // direction as arrows, the rib and the electrodes, and the optical mode's |E|² at half and 1/e²
  // of its peak over it: where the two overlap is what sets Vπ·L. One arm's gap; the other arm is
  // its mirror. Whatever model the page uses, the field is the 2D model's: the same solves as its
  // Vπ·L. Its second channel is the Pockels change of the index that field makes for light
  // polarized across, Δn = −½ n_e³ r₃₃ E_x in the crystal, with the same outlines over it: its
  // overlap with the mode, with the tensor's elements it leaves out, is the Vπ·L shown.
  //
  // Its RF view is the travelling-wave line's own field at a frequency (TflnModulator::line_field):
  // the full-wave RF mode of the line's cross-section at 1 V (|E|, or |E_x|, the component the TE
  // mode meets), or the quasi-TEM line's static field, which is that mode's at low frequency, on
  // the same pixels and the same colours; the mode's numbers and the device's own line figures at
  // that frequency beside it. The frequency is a slider on a log scale, 1, 2 and 5 a decade from 1
  // GHz to the 2D model's response's top; each frequency is one full-wave solve, kept once made.
  import { api, type DeviceSpec, type ElectrodeField, type LineField } from "../../lib/api";
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

  /** What the picture shows: the static field |E|, or the Pockels Δn it makes. */
  let channel = $state<"field" | "index">("field");
  /** Which field: the 2D model's static one (the Pockels problem's), or the RF line's own at a frequency. */
  let view = $state<"static" | "rf">("static");

  // ---- the RF line: one full-wave solve a frequency, kept by design and frequency ----
  /** The frequencies the slider offers, GHz: 1, 2 and 5 a decade from 1 GHz, and the top. */
  const ladder = (top: number) => {
    const out: number[] = [];
    for (let decade = 1; decade < top; decade *= 10) for (const m of [1, 2, 5]) if (m * decade < top) out.push(m * decade);
    out.push(top);
    return out;
  };
  let lineTop = $state<number | null>(null);
  const steps = $derived(lineTop ? ladder(lineTop) : [1]);
  /** The slider's place, and the frequency asked (on letting go of it). */
  let place = $state(0);
  let lineGhz = $state(1);
  const kept = new Map<string, LineField>();
  let line = $state<LineField | null>(null);
  let lineProblem = $state("");
  let lineBusy = $state(false);
  $effect(() => {
    if (view !== "rf") return;
    const key = `${asked}|${lineGhz}`;
    const have = kept.get(key);
    if (have) {
      line = have;
      lineTop = have.top_ghz;
      lineProblem = "";
      lineBusy = false;
      return;
    }
    const s = JSON.parse(asked) as DeviceSpec;
    const ghz = lineGhz;
    let current = true;
    lineBusy = true;
    const timer = setTimeout(() => {
      api
        .deviceLineField(s, ghz)
        .then((l) => {
          kept.set(key, l);
          if (!current) return;
          line = l;
          lineTop = l.top_ghz;
          lineProblem = "";
        })
        .catch((e) => {
          if (current) lineProblem = String(e);
        })
        .finally(() => {
          if (current) lineBusy = false;
        });
    }, 300);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  });
  /** What the RF picture shows: the full-wave mode's |E|, |E_x| or its |E|'s change from 1 GHz, or the quasi-TEM line's |E|. */
  let lineChannel = $state<"field" | "across" | "change" | "quasi">("field");
  const lineRaster = $derived(
    line ? (lineChannel === "field" ? line.field : lineChannel === "across" ? line.across : lineChannel === "change" ? line.change : line.quasi_static) : null,
  );
  /** The change's colours: symmetric about 0, to where 99% of its pixels' sizes are. */
  const changeTop = $derived.by(() => {
    if (!line) return 0;
    const sizes = { ...line.change, values: line.change.values.map(Math.abs) };
    return Math.max(quantile(sizes, CAP), 1e-9);
  });
  /** Every channel's colours: the quasi-TEM field's, from its lowest to where 99% of its pixels are, so that each frequency is seen against it. */
  const lineEnds = $derived(line ? range(line.quasi_static) : [0, 0]);
  const lineTopColour = $derived(line ? Math.min(lineEnds[1], quantile(line.quasi_static, CAP)) : 0);
  const lineHighest = $derived(lineRaster ? range(lineRaster)[1] : 0);
  const lineOutline = $derived(
    line
      ? {
          shapes: line.shapes,
          interfaces: line.interfaces.map((y) => [
            [line!.field.x0, y],
            [line!.field.x1, y],
          ]) as [number, number][][],
        }
      : undefined,
  );
  const ghzText = (v: number) => `${Number(v.toPrecision(3))} GHz`;
  const ohms = ([re, im]: [number, number]) => `${re.toFixed(2)} ${im < 0 ? "−" : "+"} ${Math.abs(im).toFixed(2)}j Ω`;
  const sentence = (s: string) => s[0].toUpperCase() + s.slice(1);

  const ends = $derived(shown ? range(shown.raster) : [0, 0]);
  const top = $derived(shown ? Math.min(ends[1], quantile(shown.raster, CAP)) : 0);
  /** Δn's colours: from its lowest to where 99% of its pixels are (the electrodes' corners touch the slab). */
  const indexEnds = $derived(shown ? range(shown.index) : [0, 0]);
  const indexTop = $derived(shown ? Math.min(indexEnds[1], quantile(shown.index, CAP)) : 0);
  const fmt = (v: number) => Number(v.toPrecision(3)).toLocaleString("en", { maximumSignificantDigits: 3 });
  /** A small number (an index change) to three digits. */
  const sci = (v: number) => (v === 0 ? "0" : v.toExponential(2).replace("e", "×10^").replace(/\^\+?(-?\d+)$/, (_, p: string) => superscript(p)));
  const superscript = (p: string) => [...p].map((c) => "⁻⁰¹²³⁴⁵⁶⁷⁸⁹"["-0123456789".indexOf(c)]).join("");
  /** A colour bar's end: two digits. */
  const short = (v: number) => (Math.abs(v) >= 1e4 ? v.toExponential(1) : tick(v));
  const shortIndex = (v: number) => (v === 0 ? "0" : v.toExponential(1));
  /** The share of the mode's Δn_eff the Δn shown gives. */
  const share = $derived(shown ? shown.index_shown / (shown.index_shown + shown.index_rest) : 0);
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
  <div class="flex flex-wrap items-center gap-2">
    <div class="join" role="group" aria-label="Which field">
      <button class="btn btn-xs join-item {view === 'static' ? 'btn-primary btn-soft' : ''}" aria-pressed={view === "static"} onclick={() => (view = "static")} title="The 2D model's static field, the one its Vπ·L takes"
        >Static</button
      >
      <button class="btn btn-xs join-item {view === 'rf' ? 'btn-primary btn-soft' : ''}" aria-pressed={view === "rf"} onclick={() => (view = "rf")} title="The travelling-wave line's own field at a frequency"
        >RF line at f</button
      >
    </div>
    {#if view === "rf"}
      <label class="flex items-center gap-2 text-xs">
        <span class="muted">f</span>
        <input
          type="range"
          class="range range-xs w-40"
          min="0"
          max={steps.length - 1}
          step="1"
          bind:value={place}
          onchange={() => (lineGhz = steps[Math.min(place, steps.length - 1)])}
          aria-label="Frequency, on a log scale"
          aria-valuetext={ghzText(steps[Math.min(place, steps.length - 1)])}
        />
        <span class="num w-16">{ghzText(steps[Math.min(place, steps.length - 1)])}</span>
      </label>
      <div class="join" role="group" aria-label="What the RF picture shows">
        <button class="btn btn-xs join-item {lineChannel === 'field' ? 'btn-primary btn-soft' : ''}" aria-pressed={lineChannel === "field"} onclick={() => (lineChannel = "field")} title="The full-wave mode's |E|"
          >|E|</button
        >
        <button class="btn btn-xs join-item {lineChannel === 'across' ? 'btn-primary btn-soft' : ''}" aria-pressed={lineChannel === "across"} onclick={() => (lineChannel = "across")} title="The full-wave mode's field across, which the TE mode meets through r₃₃"
          >|E<sub>x</sub>|</button
        >
        <button class="btn btn-xs join-item {lineChannel === 'change' ? 'btn-primary btn-soft' : ''}" aria-pressed={lineChannel === "change"} onclick={() => (lineChannel = "change")} title="The full-wave mode's |E| less its |E| at 1 GHz: where the field has gone"
          >Δ from 1 GHz</button
        >
        <button class="btn btn-xs join-item {lineChannel === 'quasi' ? 'btn-primary btn-soft' : ''}" aria-pressed={lineChannel === "quasi"} onclick={() => (lineChannel = "quasi")} title="The quasi-TEM line's field: the line's static field, the same at every frequency"
          >quasi-TEM</button
        >
      </div>
      {#if lineBusy}<span class="loading loading-spinner loading-xs text-primary" aria-label="Solving the line's mode"></span>{/if}
    {/if}
  </div>
  {#if view === "rf"}
    {#if lineProblem}
      <p class="text-sm text-warning selectable">{plain(lineProblem)}</p>
    {:else if line && lineRaster}
      <p class="text-xs muted">
        The line's full-wave mode at <span class="num">{ghzText(line.frequency_ghz)}</span>, 1 V on the signal: n <span class="num">{line.index.toFixed(4)}</span>, Z₀
        <span class="num">{ohms(line.impedance)}</span>, loss <span class="num">{line.loss.toFixed(2)} dB/cm</span> (the metal <span class="num">{line.conductor.toFixed(2)}</span>, the
        dielectrics <span class="num">{line.dielectric.toFixed(2)}</span>, radiated into the silicon <span class="num">{Number(line.radiated.toPrecision(2))}</span>). The 2D model's quasi-TEM line
        there, the device's figures: n<sub>rf</sub> <span class="num">{line.n_rf.toFixed(4)}</span>, Z₀ <span class="num">{line.z0.toFixed(2)} Ω</span>, α
        <span class="num">{line.alpha_rf.toFixed(2)} dB/cm</span>.
      </p>
      {#if lineChannel === "change"}
        <RasterView
          raster={lineRaster}
          kind="signed"
          axes={["x", "y"]}
          maxHeight={Math.max(120, height - 120)}
          quantity="|E| − |E at 1 GHz|, at 1 V (V/m)"
          outline={lineOutline}
          lines={lines.slice(1)}
          scale={[-changeTop, changeTop]}
          ends={[`≤ −${fmt(changeTop)} V/m`, `≥ ${fmt(changeTop)} V/m`]}
        />
      {:else}
        <RasterView
          raster={lineRaster}
          kind="intensity"
          axes={["x", "y"]}
          maxHeight={Math.max(120, height - 120)}
          quantity={lineChannel === "across" ? "|E_x| at 1 V (V/m)" : "|E| at 1 V (V/m)"}
          outline={lineOutline}
          lines={lines.slice(1)}
          scale={[lineEnds[0], lineTopColour]}
          ends={[`${short(lineEnds[0])} V/m`, `≥ ${short(lineTopColour)} V/m`]}
        />
      {/if}
      <p class="text-[11px] faint">
        {#if lineChannel === "quasi"}The quasi-TEM line's field: the static field of the line's own cross-section at <span class="num">1 V</span> on the signal, the grounds at 0, on the full-wave
          lattice's grid. It is the same at every frequency; its γ and Z₀ come from it and the strips' internal impedance. Its colours run from
          <span class="num">{fmt(lineEnds[0])}</span>
          (black) to <span class="num">{fmt(lineTopColour)} V/m</span> (pale yellow), where 99% of its pixels are, and the full-wave channels take the same.
        {:else if lineChannel === "change"}The full-wave mode's |E| at this frequency less its |E| at 1 GHz, both at <span class="num">1 V</span> on the signal: red where the field has grown, blue where it
          has fallen, from <span class="num">−{fmt(changeTop)}</span> to <span class="num">{fmt(changeTop)} V/m</span> where 99% of the pixels' sizes are (the quasi-TEM field's colours top at
          <span class="num">{fmt(lineTopColour)} V/m</span>). Both are the same lattice's, so its own difference from the static solver's field (1.4% of |E| at the defaults) leaves the picture.
        {:else}The full-wave mode's {lineChannel === "across" ? "field across, |E_x|, which the TE mode meets through r₃₃" : "|E| (across, up and along)"}, scaled to
          <span class="num">1 V</span> on the signal by its voltage V = 2P/I*, P its power and I the signal's current (so Z₀ = V/I). At 1 GHz it is the quasi-TEM field within the grid's error; as f rises
          it draws into the silicon (Δ from 1 GHz shows where). The quasi-TEM field's colours, from <span class="num">{fmt(lineEnds[0])}</span> (black) to
          <span class="num">{fmt(lineTopColour)} V/m</span> (pale yellow) where 99% of its pixels are, so that each frequency is seen against it{#if lineHighest > lineTopColour}; this picture reaches
            <span class="num">{fmt(lineHighest)} V/m</span>{/if}.{/if}
        One arm's gap and a gap past each of its edges, down into the silicon as deep as the gap is wide; the ground on the left and the signal on the right. White: the rib, the electrodes and the layers' interfaces (dashed).
        Teal: the optical mode's |E|² at half its peak (solid) and at 1/e² (dashed). The device's response takes the static field's overlap with the mode and the quasi-TEM line's γ and Z₀: no overlap of the RF
        field with the mode is computed. {sentence(line.grid)}; the device's figures on {line.line_grid}.
      </p>
    {:else}
      <div class="skeleton flex-1" aria-busy="true" aria-label="Solving the line's mode"></div>
    {/if}
  {:else if problem}
    <p class="text-sm text-warning selectable">{plain(problem)}</p>
  {:else if shown}
    <div class="flex flex-wrap items-start gap-2">
      <div class="join" role="group" aria-label="What the picture shows">
        <button class="btn btn-xs join-item {channel === 'field' ? 'btn-primary btn-soft' : ''}" aria-pressed={channel === "field"} onclick={() => (channel = "field")} title="The static field's strength"
          >|E|</button
        >
        <button class="btn btn-xs join-item {channel === 'index' ? 'btn-primary btn-soft' : ''}" aria-pressed={channel === "index"} onclick={() => (channel = "index")} title="The Pockels change of the index the field makes"
          >Δn</button
        >
      </div>
      {#if channel === "field"}
        <p class="min-w-0 flex-1 text-xs muted">
          The 2D model's static field at <span class="num">1 V</span> on the signal electrode, the ground at 0: under the mode's peak
          <span class="num">{fmt(shown.at_mode)} V/m</span>. Its overlap with the mode gives Vπ·L <span class="num">{shown.vpi_l.toFixed(3)} V·cm</span> push–pull (n<sub>eff</sub>
          <span class="num">{shown.n_eff.toFixed(4)}</span>).
          {#if busy}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
        </p>
      {:else}
        <p class="min-w-0 flex-1 text-xs muted">
          The Pockels change of the index at <span class="num">1 V</span> for light polarized across, Δn = −½ n<sub>e</sub>³ r₃₃ E<sub>x</sub>: under the mode's peak
          <span class="num">{sci(shown.index_at_mode)}</span>. Over the mode it moves n<sub>eff</sub> by
          <span class="num">{sci(shown.index_shown + shown.index_rest)}</span> per volt on one arm, so Vπ·L = λ / (4 |Δn<sub>eff</sub>|) =
          <span class="num">{shown.vpi_l.toFixed(3)} V·cm</span> push–pull.
          {#if busy}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
        </p>
      {/if}
    </div>
    {#if channel === "field"}
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
      <RasterView
        raster={shown.index}
        kind="intensity"
        axes={["x", "y"]}
        maxHeight={Math.max(120, height - 100)}
        quantity="Δn at 1 V"
        {outline}
        lines={lines.slice(1)}
        scale={[indexEnds[0], indexTop]}
        ends={[shortIndex(indexEnds[0]), `${indexTop < indexEnds[1] ? "≥ " : ""}${shortIndex(indexTop)}`]}
      />
      <p class="text-[11px] faint">
        Δn from <span class="num">{sci(indexEnds[0])}</span> (black) to <span class="num">{sci(indexTop)}</span> (pale yellow){#if indexTop < indexEnds[1]}, where 99% of the pixels are: under the
          electrodes' corners it reaches <span class="num">{sci(indexEnds[1])}</span>{/if}; zero outside the crystal. It is the 2D model's own change of permittivity, −ε Δη ε with Δη<sub>ij</sub> = r<sub
          >ijk</sub
        >
        E<sub>k</sub> (Jazbinšek & Zgonik 2002, Eq. 8; their clamped tensor with the film's r₃₃), as Δε<sub>xx</sub> / (2n) in each optical cell. The x-cut film's c axis is across, so the TE mode meets the
        field through r₃₃: what Δn shows carries <span class="num">{(100 * share).toFixed(1)}%</span> of Δn<sub>eff</sub>; r₁₃ on the ordinary index and r₅₁ give the rest,
        <span class="num">{sci(shown.index_rest)}</span>. The other arm's Δn is this one's mirror with the opposite sign: push–pull. Teal: the optical mode's |E|² at half its peak (solid) and at 1/e²
        (dashed). {shown.grid[0].toUpperCase() + shown.grid.slice(1)}.
      </p>
    {/if}
  {:else}
    <div class="skeleton flex-1" aria-busy="true" aria-label="Solving the electrode field"></div>
  {/if}
</div>
