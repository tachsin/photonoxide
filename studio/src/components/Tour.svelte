<script lang="ts">
  // The welcome tour: six steps through what the studio does, following the rail, shown once
  // (the question mark brings it back).
  import { Boxes, History, House, ShieldCheck, Sparkles, SquarePen } from "@lucide/svelte";

  import { app, go, updateSettings } from "../lib/app.svelte";
  import Logo from "./Logo.svelte";

  const steps = [
    {
      icon: Sparkles,
      title: "Welcome to photonoxide",
      text: "A photonics toolkit whose every solver is checked against exact solutions and published papers. This studio is where you run it: build simulations, watch them live, overlay them, and see the evidence.",
    },
    {
      icon: House,
      title: "Start from a template",
      text: "Home holds ready-made simulations, built in: a strip's modes, a ring, an MMI splitter, each turning in 3D on its card. Run plays one live; Open takes it to the job builder.",
    },
    {
      icon: SquarePen,
      title: "Build your own jobs",
      text: "The job builder (Home's New job, or a template's Open) is a form with your device drawn live in 3D, and a top view of its shapes, ports, the PML and the cut. The TOML is written for you, checked as you type, and saved into your workspace.",
    },
    {
      icon: Boxes,
      title: "Components, the chip, and the physics",
      text: "The rail holds where work happens: Components, the parts with their models and spectra; Chip, where they are wired into a circuit and solved; the Academy's lessons, with charts the library computes as you move them; and Materials, each index from its paper.",
    },
    {
      icon: History,
      title: "Watch, then overlay",
      text: "Runs play live in 3D (drag to orbit, scroll to zoom) and in 2D plots. Runs, at the bottom left, opens every run in a drawer, the one going pinned on top: tick several and Overlay draws their sweeps and spectra together.",
    },
    {
      icon: ShieldCheck,
      title: "Trust, and stay current",
      text: "System, at the rail's foot, holds Validation: every check, and every published result reproduced, each runnable on your machine; and the libraries and benchmarks found here. New releases arrive by themselves: when one is out, an Update button appears at the top, and one click installs it.",
    },
  ];

  let step = $state(0);
  const s = $derived(steps[step]);

  function finish() {
    app.tour = false;
    step = 0;
    if (!app.state?.settings.tour_done) updateSettings((x) => (x.tour_done = true));
  }
</script>

<dialog class="modal" class:modal-open={app.tour}>
  <div class="modal-box max-w-xl overflow-hidden p-0">
    <div class="glow relative grid h-44 place-items-center border-b border-base-content/8">
      {#if step === 0}
        <Logo size={72} />
      {:else}
        {@const Icon = s.icon}
        <div class="grid size-20 place-items-center rounded-2xl bg-base-100/70 text-primary shadow-lg"><Icon size={40} strokeWidth={1.6} /></div>
      {/if}
    </div>
    <div class="px-8 pt-6 pb-2">
      <p class="mb-1 text-xs font-medium tracking-wider text-primary uppercase">Step {step + 1} of {steps.length}</p>
      <h3 class="text-xl font-semibold tracking-tight">{s.title}</h3>
      <p class="mt-2 leading-relaxed muted">{s.text}</p>
    </div>
    <div class="flex items-center gap-2 px-8 pt-4 pb-7">
      <div class="flex flex-1 gap-1.5">
        {#each steps as _, k (k)}
          <button class="h-1.5 rounded-full transition-all {k === step ? 'w-6 bg-primary' : 'w-1.5 bg-base-content/20'}" aria-label="Step {k + 1}" onclick={() => (step = k)}></button>
        {/each}
      </div>
      <button class="btn btn-ghost btn-sm" onclick={finish}>Skip</button>
      {#if step > 0}<button class="btn btn-sm" onclick={() => step--}>Back</button>{/if}
      {#if step < steps.length - 1}
        <button class="btn btn-primary btn-sm" onclick={() => step++}>Next</button>
      {:else}
        <button
          class="btn btn-primary btn-sm"
          onclick={() => {
            finish();
            go("home");
          }}>Show me the templates</button
        >
      {/if}
    </div>
  </div>
</dialog>
