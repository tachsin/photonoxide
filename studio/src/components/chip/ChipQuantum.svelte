<script lang="ts">
  // The chip's quantum view: photons on its ports through its S-matrix, the Hong–Ou–Mandel dip
  // through its S(ω), and the loss budget of each photon's route, all computed by the library
  // (photonoxide::quantum, through circuits/quantum.rs).
  import { Play } from "@lucide/svelte";

  import { api, type Chip } from "../../lib/api";
  import { go, startRun, toast } from "../../lib/app.svelte";
  import { editor, panels } from "../../lib/chip.svelte";
  import { sweepProblem } from "../../lib/circuit";
  import { materials, PAIR_SOURCES } from "../../lib/materials.svelte";
  import { defaultPhotons, defaultSqueezed, photonsOn, quantumTask } from "../../lib/quantum";
  import QuantumBudget from "./QuantumBudget.svelte";
  import QuantumDip from "./QuantumDip.svelte";
  import QuantumGaussian from "./QuantumGaussian.svelte";
  import QuantumPhotons from "./QuantumPhotons.svelte";
  import QuantumSampling from "./QuantumSampling.svelte";

  let { chip }: { chip: Chip } = $props();

  const q = panels.quantum;
  const VIEWS = [
    ["photons", "Photons", "Photons on the ports at one wavelength: the output patterns, each port's photon numbers and bunching"],
    ["sampling", "Sampling", "The same photons sampled live by the exact sampler, against their exact distribution, under a hard time limit"],
    ["dip", "HOM dip", "Two photons' coincidences against their delay, through the chip's S(ω) over its sweep"],
    ["budget", "Loss budget", "Each photon's route through the chip, stage by stage, against published thresholds"],
    ["gaussian", "Squeezed light", "Squeezed light on the ports through the chip: the squeezing left after its loss, the covariance, a port's photon numbers and Wigner function"],
  ] as const;

  /** Why nothing can be computed yet, or "". */
  const blocked = $derived.by(() => {
    if (!chip.port.length) return "Expose a port first: drag from a component's port into empty space. Photons go in and come out at the chip's external ports.";
    const sweep = sweepProblem(chip.sweep);
    if (sweep) return sweep;
    const n = editor.problems.length;
    if (n) return `Finish the chip first: ${n} ${n === 1 ? "thing is" : "things are"} left, listed in the side panel.`;
    return "";
  });
  /** The wavelength chosen, or the sweep's middle when none is or it is outside the sweep. */
  const wavelength = $derived(
    q.wavelength !== null && q.wavelength >= chip.sweep.from_um && q.wavelength <= chip.sweep.to_um ? q.wavelength : Number(((chip.sweep.from_um + chip.sweep.to_um) / 2).toPrecision(12)),
  );

  /** The pairs' sources are designed beside the nonlinear processes, on the Materials page. */
  function designSource() {
    materials.chosen = PAIR_SOURCES;
    go("materials");
  }

  /** The forms as they stand, run as a "quantum" job: the same numbers, recorded, and the job runs from the command line too. */
  async function runAsJob() {
    const c = $state.snapshot(chip) as Chip;
    const ports = c.port.map((p) => p.name);
    const photons = Object.keys(q.photons).some((k) => ports.includes(k)) ? photonsOn(ports, q.photons) : defaultPhotons(ports);
    const task = quantumTask({
      ports,
      wavelength,
      photons,
      samples: 10000,
      seed: 1,
      dip: $state.snapshot(q.dip) as typeof q.dip,
      band: { from_um: c.sweep.from_um, to_um: c.sweep.to_um, points: c.sweep.points },
      budget: q.budget.photons.map(({ from, to }) => ({ from, to })),
      squeezed: q.gaussian.inputs.length ? ($state.snapshot(q.gaussian.inputs) as typeof q.gaussian.inputs) : defaultSqueezed(ports),
    });
    try {
      const name = `${c.name}-quantum`;
      const text = await api.chipQuantumJob(c, { name, timeout_minutes: 10, task });
      await startRun(() => api.runText(text), `Running ${name}`);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function keys(e: KeyboardEvent) {
    const k = VIEWS.findIndex(([id]) => id === q.view);
    const next = e.key === "ArrowRight" ? k + 1 : e.key === "ArrowLeft" ? k - 1 : e.key === "Home" ? 0 : e.key === "End" ? VIEWS.length - 1 : null;
    if (next === null) return;
    e.preventDefault();
    q.view = VIEWS[(next + VIEWS.length) % VIEWS.length][0];
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-view="${q.view}"]`)?.focus();
  }
</script>

<div class="flex items-center gap-2">
  <div role="tablist" tabindex="-1" aria-label="The quantum view's parts" class="join" onkeydown={keys}>
    {#each VIEWS as [id, name, about] (id)}
      <button
        role="tab"
        data-view={id}
        aria-selected={q.view === id}
        tabindex={q.view === id ? 0 : -1}
        class="btn join-item btn-xs {q.view === id ? 'btn-active' : 'btn-ghost'}"
        title={about}
        onclick={() => (q.view = id)}>{name}</button
      >
    {/each}
  </div>
  <span class="min-w-0 flex-1 truncate text-xs faint">{VIEWS.find(([id]) => id === q.view)?.[2]}</span>
  <button class="btn btn-ghost btn-xs shrink-0" disabled={!!blocked} title="Run every part's computation as a job: the photons and 10 000 samples of them, the dip over the sweep, the budget and the squeezed light, recorded in the runs drawer; photonoxide run runs the same file" onclick={runAsJob}><Play size={12} /> Run as a job</button>
  <button class="btn btn-ghost btn-xs shrink-0" title="Materials › Photon-pair sources: a source's joint spectrum, Schmidt modes, brightness and heralding" onclick={designSource}>Pair sources →</button>
</div>
<div class="mt-3">
  {#if q.view === "photons"}
    <QuantumPhotons {chip} {blocked} {wavelength} />
  {:else if q.view === "sampling"}
    <QuantumSampling {chip} {blocked} {wavelength} />
  {:else if q.view === "dip"}
    <QuantumDip {chip} {blocked} {wavelength} />
  {:else if q.view === "budget"}
    <QuantumBudget {chip} {blocked} {wavelength} />
  {:else}
    <QuantumGaussian {chip} {blocked} {wavelength} />
  {/if}
</div>
