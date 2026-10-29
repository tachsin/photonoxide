<script lang="ts">
  // The welcome on the first start: what photonoxide is, in a sentence, and which channel it
  // follows, Stable or Nightly (Settings › Updates changes it later); then Express, which starts
  // at once, or Advanced, for the most performance on this machine: what it has (the CPU, an
  // NVIDIA GPU, the system), the libraries suggested for it, installed in one go into
  // photonoxide's own environment, their output shown as it runs (#564). The setup's Advanced
  // path opens it on that step.
  import { CircleCheck, CircleX, Cpu, FlaskConical, Gauge, MonitorSmartphone, ShieldCheck, Zap } from "@lucide/svelte";
  import { listen } from "@tauri-apps/api/event";
  import { tick } from "svelte";

  import { api, type HardwareAdvice } from "../lib/api";
  import { bytes } from "../lib/bench.svelte";

  /** The CPU's cores, caches and NUMA nodes, as far as the system said. */
  function shape(cpu: HardwareAdvice["hardware"]["cpu"]): string {
    const t = cpu.topology;
    const parts = [`${cpu.threads} threads`];
    if (t.cores) {
      parts.push(t.performance_cores ? `${t.performance_cores} P + ${t.efficiency_cores} E cores` : `${t.cores} cores`);
    }
    const caches = [["L1d", t.l1d], ["L2", t.l2], ["L3", t.l3]].filter(([, b]) => b);
    if (caches.length) parts.push(caches.map(([name, b]) => `${name} ${bytes(b as number)}`).join(", "));
    if (t.numa_nodes > 1) parts.push(`${t.numa_nodes} NUMA nodes`);
    if (cpu.features.length) parts.push(cpu.features.join(", "));
    return parts.join(" · ");
  }
  import { app, updateSettings } from "../lib/app.svelte";
  import { detect, libraries } from "../lib/libraries.svelte";
  import { openUrl } from "../lib/open";
  import { switchChannel } from "../lib/updater.svelte";
  import Logo from "./Logo.svelte";

  const channels = [
    {
      id: "stable",
      icon: ShieldCheck,
      name: "Stable",
      tag: "Recommended",
      text: "The signed releases, built and tested before they're published. When one is out, an Update button appears at the top, and one click installs it.",
    },
    {
      id: "nightly",
      icon: FlaskConical,
      name: "Nightly",
      tag: "For trying what's next",
      text: "main's latest commit, built here from its source, tested by CI and nothing more. It needs Rust, Node and the platform's build tools; nothing is built without your click.",
    },
  ] as const;

  let chosen = $state<"stable" | "nightly">("stable");
  let busy = $state(false);

  // the performance step
  let advice = $state<HardwareAdvice | null>(null);
  let adviceError = $state("");
  let picked = $state<Record<string, boolean>>({});
  let installing = $state<"idle" | "running" | "done">("idle");
  let lines = $state<string[]>([]);
  let outcomes = $state<{ library: string; ok: boolean; error: string }[]>([]);
  let log: HTMLElement | undefined = $state();

  // what the settings say already, when the welcome opens (a nightly build follows Nightly)
  $effect(() => {
    if (app.welcome) chosen = app.state?.settings.channel === "nightly" ? "nightly" : "stable";
  });
  $effect(() => {
    if (app.welcome && app.welcomeStep === "performance" && !advice) loadAdvice();
  });

  const found = (library: string) => !!libraries.report?.libraries.find((l) => l.name === library)?.found;

  async function loadAdvice() {
    try {
      if (!libraries.report) await detect();
      advice = await api.hardwareAdvice();
      picked = Object.fromEntries(advice.suggestions.map((s) => [s.library, !!s.install && !found(s.library)]));
    } catch (e) {
      adviceError = String(e);
    }
  }

  async function chooseChannel() {
    await updateSettings((x) => (x.welcomed = true));
    await switchChannel(chosen);
  }

  /** Express: the channel, then straight in. */
  async function start() {
    busy = true;
    try {
      await chooseChannel();
    } finally {
      busy = false;
      app.welcome = false;
    }
  }

  /** Advanced: the channel, then the performance step. */
  async function advanced() {
    busy = true;
    try {
      await chooseChannel();
      app.welcomeStep = "performance";
    } finally {
      busy = false;
    }
  }

  const chosenInstalls = $derived(advice?.suggestions.filter((s) => s.install && picked[s.library]) ?? []);

  async function installChosen() {
    installing = "running";
    lines = [];
    outcomes = [];
    const stop = await listen<{ library: string; line: string }>("library-install", async ({ payload }) => {
      lines.push(`${payload.library}: ${payload.line}`);
      await tick();
      log?.scrollTo({ top: log.scrollHeight });
    });
    try {
      // one at a time: they share photonoxide's own environment
      for (const s of chosenInstalls) {
        try {
          await api.libraryInstall(s.library, s.install!);
          outcomes.push({ library: s.library, ok: true, error: "" });
        } catch (e) {
          outcomes.push({ library: s.library, ok: false, error: String(e) });
        }
      }
      await detect();
    } finally {
      stop();
      installing = "done";
    }
  }

  function finish() {
    app.welcome = false;
    app.welcomeStep = "channel";
  }
</script>

<dialog class="modal" class:modal-open={app.welcome} aria-labelledby="welcome-title">
  <div class="modal-box max-w-2xl overflow-hidden p-0">
    {#if app.welcomeStep === "channel"}
      <div class="glow relative flex flex-col items-center gap-3 border-b border-base-content/8 px-8 pt-9 pb-7 text-center">
        <Logo size={76} />
        <h3 id="welcome-title" class="text-2xl font-semibold tracking-tight">Welcome to photonoxide</h3>
        <p class="max-w-lg leading-relaxed muted">
          Photonics simulation, from a waveguide's modes to whole circuits, heaters and modulators. Every solver is checked against exact solutions and published papers, and this studio runs your jobs live.
        </p>
      </div>
      <div class="px-8 pt-6">
        <p class="mb-3 text-sm font-medium">Which releases should photonoxide follow?</p>
        <div class="grid gap-3 sm:grid-cols-2" role="radiogroup" aria-label="The release channel">
          {#each channels as c (c.id)}
            {@const on = chosen === c.id}
            <button class="choice {on ? 'on' : ''}" role="radio" aria-checked={on} onclick={() => (chosen = c.id)}>
              <span class="flex items-center gap-2">
                <span class="grid size-8 place-items-center rounded-lg {on ? 'bg-primary text-primary-content' : 'bg-base-content/8'} transition-colors"><c.icon size={17} /></span>
                <span class="font-semibold">{c.name}</span>
                <span class="ml-auto text-[11px] {c.id === 'stable' ? 'text-primary' : 'faint'}">{c.tag}</span>
              </span>
              <span class="mt-2 block text-left text-xs leading-relaxed muted">{c.text}</span>
            </button>
          {/each}
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-3 px-8 pt-5 pb-7">
        <span class="flex-1 text-xs faint">Settings › Updates changes it whenever you like.</span>
        <button class="btn btn-ghost gap-1.5" onclick={advanced} disabled={busy} title="See what this machine has and install the libraries that make photonoxide fastest on it"><Gauge size={15} /> Set up for the most performance…</button>
        <button class="btn btn-primary px-6" onclick={start} disabled={busy}>
          {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
          Start with {chosen === "stable" ? "Stable" : "Nightly"}
        </button>
      </div>
    {:else}
      <div class="glow relative flex items-center gap-4 border-b border-base-content/8 px-8 pt-7 pb-5">
        <div class="grid size-12 place-items-center rounded-xl bg-primary/12 text-primary"><Zap size={24} /></div>
        <div>
          <h3 id="welcome-title" class="text-xl font-semibold tracking-tight">For the most performance here</h3>
          <p class="text-sm muted">photonoxide's own solvers need nothing more. These libraries are what make it fastest on this machine.</p>
        </div>
      </div>
      <div class="max-h-[60vh] space-y-4 overflow-y-auto px-8 pt-5">
        {#if adviceError}
          <p class="text-sm text-error selectable">{adviceError}</p>
        {:else if !advice}
          <p class="flex items-center gap-2 text-sm muted"><span class="loading loading-spinner loading-xs"></span> Looking at this machine…</p>
        {:else}
          <div class="grid gap-2 text-sm sm:grid-cols-2">
            <div class="hw"><Cpu size={15} class="text-primary" /><span><span class="font-medium">{advice.hardware.cpu.brand || advice.hardware.cpu.vendor}</span><span class="block text-xs faint">{shape(advice.hardware.cpu)}</span></span></div>
            <div class="hw"><MonitorSmartphone size={15} class="text-primary" /><span>{#if advice.hardware.gpu}<span class="font-medium">{advice.hardware.gpu.name}</span><span class="block text-xs faint">NVIDIA driver {advice.hardware.gpu.driver}</span>{:else}<span class="font-medium">No NVIDIA GPU found</span><span class="block text-xs faint">{advice.hardware.os} on {advice.hardware.arch}</span>{/if}</span></div>
          </div>
          <ul class="space-y-2">
            {#each advice.suggestions as s (s.library)}
              <li class="suggestion">
                {#if s.install && installing === "idle"}
                  <input type="checkbox" class="checkbox checkbox-sm checkbox-primary mt-0.5" bind:checked={picked[s.library]} aria-label="Install {s.library}" />
                {:else}
                  <span class="mt-0.5 size-5 shrink-0"></span>
                {/if}
                <div class="min-w-0 flex-1">
                  <p class="text-sm"><span class="font-medium">{s.library}</span>{#if found(s.library)}<span class="badge badge-success badge-soft badge-xs ml-2">found</span>{/if}</p>
                  <p class="text-xs muted">{s.why}</p>
                  {#if s.system}
                    <p class="text-xs faint">Part of the system: nothing to install.</p>
                  {:else if s.command}
                    <code class="mt-1 block truncate font-mono text-[11px] faint" title={s.command}>{s.command}</code>
                  {:else}
                    <p class="text-xs faint">Only its vendor's installer has it: <button class="link link-primary" onclick={() => openUrl(s.download)}>the download</button>.</p>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
          <p class="text-xs faint">Suggested by the hardware. Each is installed under its vendor's licence, a conda-forge one into photonoxide's own environment; the Benchmarks page then measures it against photonoxide's own here, and each stays optional.</p>
          {#if installing !== "idle"}
            <pre bind:this={log} class="max-h-48 overflow-auto rounded-lg bg-base-300/70 px-3 py-2 font-mono text-[11px] leading-relaxed whitespace-pre-wrap selectable">{lines.join("\n") || "Starting…"}</pre>
            {#each outcomes as o (o.library)}
              <p class="flex items-start gap-2 text-sm">{#if o.ok}<CircleCheck size={15} class="mt-0.5 text-success" /> {o.library}: installed{:else}<CircleX size={15} class="mt-0.5 text-error" /> <span class="selectable">{o.library}: {o.error}</span>{/if}</p>
            {/each}
          {/if}
        {/if}
      </div>
      <div class="flex items-center gap-3 px-8 pt-5 pb-7">
        <span class="flex-1 text-xs faint">The Libraries page installs and removes them later too.</span>
        {#if installing === "running"}
          <span class="flex items-center gap-2 text-sm muted"><span class="loading loading-spinner loading-xs"></span> Installing…</span>
        {:else if installing === "done"}
          <button class="btn btn-primary px-6" onclick={finish}>Start</button>
        {:else}
          <button class="btn btn-ghost" onclick={finish}>Skip</button>
          <button class="btn btn-primary px-6" disabled={!chosenInstalls.length} onclick={installChosen}>Install {chosenInstalls.length || ""} {chosenInstalls.length === 1 ? "library" : "libraries"}</button>
        {/if}
      </div>
    {/if}
  </div>
</dialog>

<style>
  .choice {
    display: block;
    border-radius: 14px;
    border: 1px solid color-mix(in oklch, var(--color-base-content) 12%, transparent);
    background: color-mix(in oklch, var(--color-base-content) 3%, transparent);
    padding: 14px;
    text-align: left;
    transition:
      border-color 160ms,
      background 160ms,
      transform 160ms,
      box-shadow 160ms;
  }
  .choice:hover {
    border-color: color-mix(in oklch, var(--color-primary) 45%, transparent);
    transform: translateY(-1px);
  }
  .choice.on {
    border-color: var(--color-primary);
    background: color-mix(in oklch, var(--color-primary) 9%, transparent);
    box-shadow: 0 6px 24px -12px var(--color-primary);
  }
  .hw,
  .suggestion {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    border-radius: 12px;
    border: 1px solid color-mix(in oklch, var(--color-base-content) 10%, transparent);
    padding: 10px 12px;
  }
  @media (prefers-reduced-motion: reduce) {
    .choice,
    .choice:hover {
      transition: none;
      transform: none;
    }
  }
</style>
