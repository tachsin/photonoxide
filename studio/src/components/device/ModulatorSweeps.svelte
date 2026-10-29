<script lang="ts" module>
  /** The kind of device these sweeps are for. */
  export const MODULATOR = "TFLN Mach-Zehnder modulator";
</script>

<script lang="ts">
  // A TFLN modulator's sweeps on its Study tab (studio/src-tauri/src/devices/modulator.rs), each
  // number one the device's own model gives, at its analytic or 2D fidelity:
  //
  // - Vπ·L, Vπ at DC, or the 3 or 6 dB bandwidth against the electrode gap, the signal
  //   electrode's width, the rib's width or the length: each point the device's figures with the
  //   parameter there and the rest as designed, one point a call so the sweep shows as it goes
  //   and stops between points;
  // - the electro-optic response against frequency, 20 log10 |m(f)| less its value at 1 GHz (the
  //   reference the figures' bandwidths take), each point the figure m_dB at that frequency, the
  //   3 and 6 dB points marked.
  import { Play, Square } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, duration, type DeviceSpec, type EoResponse, type VpiPoint } from "../../lib/api";
  import { plain } from "../../lib/deviceview";
  import { label as tick } from "../../lib/plot";
  import { shown as inUnit, stored } from "../../lib/units";
  import Plot from "../Plot.svelte";
  import UnitChip from "../UnitChip.svelte";

  let { spec, values, designKey }: { spec: DeviceSpec; values: Record<string, number>; designKey: string } = $props();

  /** The parameters swept, µm, and a range for each to start from. */
  const PARAMETERS = [
    { name: "gap", label: "electrode gap", from: 3, to: 10 },
    { name: "signal", label: "signal electrode width", from: 5, to: 60 },
    { name: "width", label: "rib width", from: 0.8, to: 2 },
    { name: "length", label: "length", from: 2000, to: 20000 },
  ] as const;
  /** What the chart shows of each point. */
  const QUANTITIES = [
    { key: "vpi_l", label: "Vπ·L", axis: "Vπ·L, push–pull (V·cm)" },
    { key: "vpi", label: "Vπ", axis: "Vπ at DC (V)" },
    { key: "f_3db", label: "3 dB", axis: "3 dB bandwidth from 1 GHz (GHz)" },
    { key: "f_6db", label: "6 dB", axis: "6 dB bandwidth from 1 GHz (GHz)" },
  ] as const;

  /** The model the sweeps compute by: the design's when it is the analytic or 2D one. */
  let fidelity = $state<"analytic" | "2D">(untrack(() => (spec.fidelity === "2D" ? "2D" : "analytic")));
  let parameter = $state<(typeof PARAMETERS)[number]["name"]>("gap");
  const chosen = $derived(PARAMETERS.find((p) => p.name === parameter)!);
  let from = $state<number>(PARAMETERS[0].from);
  let to = $state<number>(PARAMETERS[0].to);
  let count = $state(9);
  let quantity = $state<(typeof QUANTITIES)[number]["key"]>("vpi_l");
  const shownQuantity = $derived(QUANTITIES.find((q) => q.key === quantity)!);

  function choose(name: string) {
    const p = PARAMETERS.find((q) => q.name === name)!;
    parameter = p.name;
    from = p.from;
    to = p.to;
  }

  // ---- the sweep: one point a call ----
  let sweep = $state<{ parameter: string; fidelity: string; key: string; points: VpiPoint[]; grid: string; total: number } | null>(null);
  let going = $state(false);
  let stopping = false;
  let started = $state(0);
  let now = $state(0);
  let problem = $state("");

  async function start() {
    const s = $state.snapshot(spec) as DeviceSpec;
    const n = Math.max(2, Math.round(count));
    const xs = Array.from({ length: n }, (_, k) => from + ((to - from) * k) / (n - 1));
    sweep = { parameter, fidelity, key: designKey, points: [], grid: "", total: n };
    going = true;
    stopping = false;
    problem = "";
    started = Date.now();
    const clock = setInterval(() => (now = Date.now()), 500);
    try {
      for (const x of xs) {
        if (stopping) break;
        const r = await api.deviceVpiLSweep(s, fidelity, parameter, [x]);
        sweep.points.push(...r.points);
        if (!sweep.grid && r.grid) sweep.grid = r.grid;
        now = Date.now();
      }
    } catch (e) {
      problem = String(e);
    } finally {
      clearInterval(clock);
      going = false;
    }
  }

  const done = $derived(sweep?.points.length ?? 0);
  const elapsed = $derived(going ? (now - started) / 1000 : 0);
  const left = $derived(going && done > 0 && sweep ? (elapsed / done) * (sweep.total - done) : null);
  const series = $derived(
    sweep
      ? [
          {
            label: `our ${sweep.fidelity} model`,
            points: sweep.points.filter((p) => p[quantity] !== null).map((p) => [p.x, p[quantity] as number] as [number, number]),
          },
        ]
      : [],
  );
  const refused = $derived(sweep?.points.filter((p) => p.problem) ?? []);

  // ---- the response against frequency: at once by the analytic model, on asking by the 2D ----
  let response = $state<{ key: string; data: EoResponse } | null>(null);
  let responseProblem = $state("");
  let responding = $state(false);
  let asked = 0;

  function respond() {
    const s = { ...($state.snapshot(spec) as DeviceSpec) };
    const key = `${designKey} ${fidelity}`;
    const mine = ++asked;
    responding = true;
    api
      .deviceEoResponse(s, fidelity, null, 120)
      .then((data) => {
        if (mine !== asked) return;
        response = { key, data };
        responseProblem = "";
      })
      .catch((e) => {
        if (mine === asked) responseProblem = String(e);
      })
      .finally(() => {
        if (mine === asked) responding = false;
      });
  }
  $effect(() => {
    // the analytic model follows the design
    if (fidelity !== "analytic") return;
    void designKey;
    const timer = setTimeout(() => untrack(respond), 300);
    return () => clearTimeout(timer);
  });
  const responseStale = $derived(!!response && response.key !== `${designKey} ${fidelity}`);
  const r = $derived(response?.data ?? null);
  /** A grid in words as a sentence of its own. */
  const sentence = (t: string) => (t ? t[0].toUpperCase() + t.slice(1) : t);
  const ghz = (v: number) => `${Number(v.toPrecision(4))} GHz`;
  const guides = $derived(
    r
      ? [
          { y: -3, label: "−3 dB" },
          { y: -6, label: "−6 dB" },
          ...(r.falls_3db ? [{ x: r.f_3db, label: `f₃dB ${ghz(r.f_3db)}` }] : []),
          ...(r.falls_6db ? [{ x: r.f_6db, label: `f₆dB ${ghz(r.f_6db)}` }] : []),
        ]
      : [],
  );
</script>

<div class="space-y-5">
  <section>
    <h3 class="panel-title">Model</h3>
    <div class="mt-2 flex flex-wrap items-center gap-2">
      <div class="join" role="group" aria-label="The model the sweeps compute by">
        {#each ["analytic", "2D"] as const as f (f)}
          <button class="btn btn-xs join-item {fidelity === f ? 'btn-primary btn-soft' : ''}" aria-pressed={fidelity === f} disabled={going} onclick={() => (fidelity = f)}>{f}</button>
        {/each}
      </div>
      <span class="min-w-0 flex-1 text-[11px] leading-snug faint">
        {fidelity === "analytic"
          ? "Closed forms: exact, at once."
          : "The static solver's field and the full-vector mode for Vπ·L, the quasi-TEM line from the static solver: a solve each design, seconds; kept, so a sweep of the length or the frequency solves once."}
      </span>
    </div>
  </section>

  <section>
    <h3 class="panel-title">Vπ·L, Vπ and the bandwidths against a parameter</h3>
    <div class="mt-2 flex flex-wrap items-end gap-3 text-xs">
      <label class="flex flex-col gap-1">
        <span class="muted">Parameter</span>
        <select class="select select-xs w-44" value={parameter} disabled={going} onchange={(e) => choose((e.currentTarget as HTMLSelectElement).value)}>
          {#each PARAMETERS as p (p.name)}<option value={p.name}>{p.label} ({p.name})</option>{/each}
        </select>
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted">From (<UnitChip />)</span>
        <input class="input input-xs w-24 num" type="number" value={inUnit(from)} disabled={going} onchange={(e) => (from = stored(Number((e.currentTarget as HTMLInputElement).value)))} />
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted">To (<UnitChip />)</span>
        <input class="input input-xs w-24 num" type="number" value={inUnit(to)} disabled={going} onchange={(e) => (to = stored(Number((e.currentTarget as HTMLInputElement).value)))} />
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted">Points</span>
        <input class="input input-xs w-16 num" type="number" min="2" max="201" bind:value={count} disabled={going} />
      </label>
      {#if going}
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => (stopping = true)} title="Stop after the point being computed: the points done stay"><Square size={13} /> Stop</button>
      {:else}
        <button class="btn btn-primary btn-sm gap-1.5" disabled={!(to > from) || !(count >= 2)} onclick={start} title="Each point the device's figures by its {fidelity} model with the {chosen.label} there"><Play size={13} /> Sweep</button>
      {/if}
    </div>
    {#if sweep}
      <div class="mt-3 flex flex-wrap items-center gap-2">
        <div class="join" role="group" aria-label="What the chart shows">
          {#each QUANTITIES as q (q.key)}
            <button class="btn btn-xs join-item {quantity === q.key ? 'btn-primary btn-soft' : ''}" aria-pressed={quantity === q.key} onclick={() => (quantity = q.key)}>{q.label}</button>
          {/each}
        </div>
        <span class="flex-1"></span>
        <span class="text-xs faint num" aria-live="polite">
          {#if going}point {done + 1} of {sweep.total} · {duration(elapsed)}{left !== null ? ` · about ${duration(left)} left` : ""}{:else}{done} of {sweep.total} points{/if}
        </span>
        {#if sweep.key !== designKey && !going}<span class="badge badge-warning badge-soft badge-sm" title="The design has changed since this sweep: sweep again for the design as it is">an earlier design</span>{/if}
      </div>
      {#if series[0]?.points.length}
        <div class="mt-2">
          <Plot {series} xLabel={PARAMETERS.find((p) => p.name === sweep!.parameter)?.label ?? sweep.parameter} xLength yLabel={shownQuantity.axis} height={240} name="modulator-{sweep.parameter}-{quantity}" marker={values[sweep.parameter] ?? null} />
        </div>
      {/if}
      <p class="mt-1 text-[11px] faint">
        Each point is our {sweep.fidelity} model's figures with the {PARAMETERS.find((p) => p.name === sweep!.parameter)?.label ?? sweep.parameter} there and the rest as designed (what uses it evaluated
        again): the same figures the Design tab shows. The line marks the design's value. Vπ·L is per length, so the length moves Vπ and the bandwidths alone. {#if sweep.grid}{sentence(sweep.grid)}.{/if}
      </p>
      {#if refused.length}
        <p class="mt-1 text-[11px] text-warning selectable">
          {refused.length} point{refused.length === 1 ? "" : "s"} the model refused, the first at {tick(inUnit(refused[0].x))} <UnitChip />: {plain(refused[0].problem ?? "")}
        </p>
      {/if}
    {/if}
    {#if problem}<p role="alert" class="mt-2 text-xs text-error selectable">{plain(problem)}</p>{/if}
  </section>

  <section>
    <div class="flex flex-wrap items-center gap-2">
      <h3 class="panel-title mr-auto">Electro-optic response against frequency</h3>
      {#if responding}<span class="loading loading-spinner loading-xs text-primary" aria-label="Computing the response"></span>{/if}
      {#if fidelity === "2D"}
        <button class="btn btn-primary btn-xs gap-1" disabled={responding} onclick={respond} title="The 2D model's response: a solve of the design if it isn't kept"><Play size={12} /> {response ? "Again" : "Compute"} at 2D</button>
      {/if}
      {#if responseStale && !responding}<span class="badge badge-warning badge-soft badge-sm" title="The design or the model has changed since">an earlier design</span>{/if}
    </div>
    {#if responseProblem}
      <p role="alert" class="mt-2 text-xs text-error selectable">{plain(responseProblem)}</p>
    {:else if r}
      <div class="mt-2">
        <Plot
          series={[{ label: `our ${r.fidelity} model`, points: r.points }]}
          xLabel="frequency (GHz)"
          yLabel="20 log₁₀ |m(f)| from 1 GHz (dB)"
          height={240}
          markers={false}
          name="modulator-eo-response"
          {guides}
        />
      </div>
      <p class="mt-1 text-[11px] faint">
        Our {r.fidelity} model's electro-optic response, 20 log₁₀ |m(f)| less its value at the reference, <span class="num">{r.reference_ghz} GHz</span>
        (<span class="num">{r.reference_db.toFixed(3)} dB</span> there): the reference the figures f_3dB and f_6dB take, as Zhang et al. refer theirs. Each point is the figure m_dB with the design's frequency
        there. It falls 3 dB at
        <span class="num">{r.falls_3db ? ghz(r.f_3db) : `none below ${ghz(r.f_3db)}`}</span>
        and 6 dB at <span class="num">{r.falls_6db ? ghz(r.f_6db) : `none below ${ghz(r.f_6db)}`}</span>.
        {#if r.fidelity === "2D"}The line is quasi-TEM: past about 100 GHz the curve is that line's.{:else}The analytic line's bandwidths have no stated error: its RF index runs high against the 2D line's, which
          puts them low.{/if}
        {sentence(r.grid)}.
      </p>
    {:else if fidelity === "2D"}
      <p class="mt-2 text-xs faint">At 2D the response comes from the design's solve: Compute starts it (seconds), or takes the one kept.</p>
    {:else}
      <div class="skeleton mt-2 h-60" aria-busy="true" aria-label="Computing the response"></div>
    {/if}
  </section>
</div>
