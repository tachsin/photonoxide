<script lang="ts">
  // The welcome on the first start: what photonoxide is, in a sentence, and which channel it
  // follows, Stable or Nightly (Settings › Updates changes it later). Shown once.
  import { FlaskConical, ShieldCheck } from "@lucide/svelte";

  import { app, updateSettings } from "../lib/app.svelte";
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

  // what the settings say already, when the welcome opens (a nightly build follows Nightly)
  $effect(() => {
    if (app.welcome) chosen = app.state?.settings.channel === "nightly" ? "nightly" : "stable";
  });

  async function start() {
    busy = true;
    try {
      await updateSettings((x) => (x.welcomed = true));
      await switchChannel(chosen);
    } finally {
      busy = false;
      app.welcome = false;
    }
  }
</script>

<dialog class="modal" class:modal-open={app.welcome} aria-labelledby="welcome-title">
  <div class="modal-box max-w-2xl overflow-hidden p-0">
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
          <button
            class="choice {on ? 'on' : ''}"
            role="radio"
            aria-checked={on}
            onclick={() => (chosen = c.id)}
            ondblclick={() => {
              chosen = c.id;
              start();
            }}
          >
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
    <div class="flex items-center gap-3 px-8 pt-5 pb-7">
      <span class="flex-1 text-xs faint">Settings › Updates changes it whenever you like.</span>
      <button class="btn btn-primary px-6" onclick={start} disabled={busy}>
        {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
        Start with {chosen === "stable" ? "Stable" : "Nightly"}
      </button>
    </div>
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
  @media (prefers-reduced-motion: reduce) {
    .choice,
    .choice:hover {
      transition: none;
      transform: none;
    }
  }
</style>
