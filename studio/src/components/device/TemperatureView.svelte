<script lang="ts">
  // A thermo-optic shifter's temperature on its cross-section, by its 2D model at the design's
  // power, or at the power for a π shift while the design's is zero
  // (studio/src-tauri/src/devices/fields.rs): the rise above the ambient from black to pale
  // yellow, its isotherms, the media's edges, and where the guide's mode takes its phase from the
  // heat (its thermo-optic weights at half and 1/e² of their largest); with the drive per unit
  // length, the peak, and the rise the mode feels. Whatever model the page uses, the field is the
  // 2D model's: the analytic one gives the rise at the guide's centre only.
  //
  // In time: the same model turned on from the ambient at that power, by the transient its
  // figures take (ThermoShifter::turn_on): the rise the mode feels against time with its 10 % and
  // 90 % times and its 1/e time (the figure τ) marked, and the cross-section at a time the slider
  // picks, its isotherms at the steady picture's levels.
  import { api, type DeviceSpec, type Temperature, type TemperatureInTime as InTime } from "../../lib/api";
  import { range } from "../../lib/colours";
  import { plain } from "../../lib/deviceview";
  import RasterView from "../RasterView.svelte";
  import TemperatureInTime from "../TemperatureInTime.svelte";

  let { spec, height = 300 }: { spec: DeviceSpec; height?: number } = $props();

  /** The steady rise, or the rise in time from the turn-on. */
  let when = $state<"steady" | "time">("steady");

  /** The design at the 2D model: the temperature doesn't change with the model the page shows. */
  const asked = $derived(JSON.stringify({ ...spec, fidelity: "2D", compact: undefined }));
  let shown = $state<Temperature | null>(null);
  let problem = $state("");
  let busy = $state(false);
  $effect(() => {
    const s = JSON.parse(asked) as DeviceSpec;
    let current = true;
    busy = true;
    const timer = setTimeout(() => {
      api
        .deviceTemperature(s)
        .then((t) => {
          if (!current) return;
          shown = t;
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

  // in time: asked for only while shown, kept for the design it was computed for
  let timed = $state<{ key: string; data: InTime } | null>(null);
  let timeProblem = $state("");
  let timing = $state(false);
  $effect(() => {
    if (when !== "time") return;
    const key = asked;
    if (timed?.key === key) return;
    const s = JSON.parse(key) as DeviceSpec;
    let current = true;
    timing = true;
    const timer = setTimeout(() => {
      api
        .deviceTemperatureInTime(s)
        // done here, not in a finally: setting `timed` runs this effect again, which ends `current`
        .then((data) => {
          if (!current) return;
          timing = false;
          timed = { key, data };
          timeProblem = "";
        })
        .catch((e) => {
          if (!current) return;
          timing = false;
          timeProblem = String(e);
        });
    }, 300);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  });
  const t = $derived(timed?.data ?? null);
  const stale = $derived(!!timed && timed.key !== asked);

  const kelvin = (v: number) => `${Number(v.toPrecision(3))} K`;
  const milliwatts = (v: number) => `${Number(v.toPrecision(4))} mW`;
  const micro = (v: number) => `${Number(v.toPrecision(4))} µs`;
  const ends = $derived(shown ? range(shown.raster) : [0, 0]);
  const step = $derived(shown && shown.isotherms.length > 1 ? shown.isotherms[1].level - shown.isotherms[0].level : (shown?.isotherms[0]?.level ?? 0));
  const lines = $derived(
    shown
      ? [
          { lines: shown.isotherms.flatMap((i) => i.lines), tone: "level" as const },
          { lines: shown.mode[0]?.lines ?? [], tone: "mode" as const },
          { lines: shown.mode[1]?.lines ?? [], tone: "mode-faint" as const },
        ]
      : [],
  );
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
  <div class="join self-start" role="group" aria-label="The temperature steady or in time">
    <button class="btn btn-xs join-item {when === 'steady' ? 'btn-primary btn-soft' : ''}" aria-pressed={when === "steady"} onclick={() => (when = "steady")}>Steady</button>
    <button class="btn btn-xs join-item {when === 'time' ? 'btn-primary btn-soft' : ''}" aria-pressed={when === "time"} onclick={() => (when = "time")} title="The 2D model turned on from the ambient: a transient, seconds"
      >In time</button
    >
  </div>
  {#if problem}
    <p class="text-sm text-warning selectable">{plain(problem)}</p>
  {:else if when === "time"}
    {#if timeProblem}
      <p class="text-sm text-warning selectable">{plain(timeProblem)}</p>
    {:else if t}
      <p class="text-xs muted">
        The 2D model turned on from the ambient at <span class="num">{milliwatts(t.power_mw)}</span>{t.at_pi ? ", the power for a π shift (the design's is zero)" : ""}: the rise the mode feels goes to
        <span class="num">{kelvin(t.steady)}</span>, 10 % to 90 % of the way in <span class="num">{micro(t.rise_us)}</span> (from <span class="num">{micro(t.ten_us)}</span> to
        <span class="num">{micro(t.ninety_us)}</span>), 1 − 1/e of it at <span class="num">{micro(t.tau_us)}</span>, the figure τ.
        {#if timing || stale}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
      </p>
      <TemperatureInTime
        series={[{ label: "the mode's rise", points: t.times_us.map((x, k): [number, number] => [x, t.guide[k]]) }]}
        marks={[
          { x: t.ten_us, label: "10 %" },
          { x: t.tau_us, label: "τ" },
          { x: t.ninety_us, label: "90 %" },
        ]}
        frames={t.frames.map((f) => ({ time_us: f.time_us, raster: f.raster, lines: f.isotherms.flatMap((i) => i.lines) }))}
        scale={[0, t.peak]}
        ends={[kelvin(0), kelvin(t.peak)]}
        outline={{ shapes: t.outline, interfaces: [] }}
        over={[
          { lines: t.mode[0]?.lines ?? [], tone: "mode" },
          { lines: t.mode[1]?.lines ?? [], tone: "mode-faint" },
        ]}
        quantity="rise (K)"
        yLabel="rise at the mode (K)"
        name="temperature-in-time"
        height={Math.max(140, Math.round(0.36 * height))}
        maxHeight={Math.max(120, Math.round(0.64 * height) - 110)}
      />
      <p class="text-[11px] faint">
        The curve: the rise the mode feels, the mean of the rise weighted by where it takes its phase from, so its times are the phase's. The pictures: the rise from <span class="num">0 K</span>
        (black) to the steady peak, <span class="num">{kelvin(t.peak)}</span> (pale yellow), every picture on that scale; thin lines, isotherms at the steady picture's levels{#if t.levels.length > 1}, every
          <span class="num">{kelvin(t.levels[1] - t.levels[0])}</span>{/if}; teal, the mode as in the steady view. {t.grid}.
      </p>
    {:else}
      <div class="skeleton flex-1" aria-busy="true" aria-label="Turning the heater on"></div>
    {/if}
  {:else if shown}
    <p class="text-xs muted">
      The 2D model's temperature rise
      {#if shown.at_pi}at the power for a π shift, <span class="num">{milliwatts(shown.p_pi_mw)}</span> (the design's power is zero){:else}at the design's power, <span class="num"
          >{milliwatts(shown.power_mw)}</span
        > (a π shift takes <span class="num">{milliwatts(shown.p_pi_mw)}</span>){/if}: <span class="num">{Number(shown.watts_per_metre.toPrecision(4))} W/m</span> along the guide, at most
      <span class="num">{kelvin(shown.peak)}</span>. The mode feels <span class="num">{kelvin(shown.guide)}</span>, the mean of the rise weighted by where it takes its phase from.
      {#if busy}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
    </p>
    <RasterView
      raster={shown.raster}
      kind="intensity"
      axes={["x", "y"]}
      maxHeight={Math.max(120, height - 120)}
      quantity="rise (K)"
      outline={{ shapes: shown.outline, interfaces: [] }}
      {lines}
      ends={[kelvin(ends[0]), kelvin(ends[1])]}
    />
    <p class="text-[11px] faint">
      From <span class="num">{kelvin(ends[0])}</span> (black) to <span class="num">{kelvin(ends[1])}</span> (pale yellow) in this window; x across the guide from its centre, y up from the buried
      oxide's top. White: the media's edges{#if step > 0}; thin lines, isotherms every <span class="num">{kelvin(step)}</span>{/if}. Teal: where the mode takes its phase from the heat, its
      thermo-optic weight at half its largest (solid) and at 1/e² (dashed). {shown.grid}.
    </p>
  {:else}
    <div class="skeleton flex-1" aria-busy="true" aria-label="Solving the temperature"></div>
  {/if}
</div>
