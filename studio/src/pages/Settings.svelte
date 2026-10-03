<script lang="ts">
  // Settings: appearance, the workspace, tips, updates, and what this copy is.
  import { Check, Download, ExternalLink, FolderOpen, Lightbulb, Monitor, Moon, RefreshCw, RotateCcw, Search, Sun } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

  import { app, toast, updateSettings } from "../lib/app.svelte";
  import { DAISY_THEMES } from "../lib/themes";
  import { checkForUpdate, updater } from "../lib/updater.svelte";

  const s = $derived(app.state!.settings);
  let query = $state("");

  async function chooseWorkspace() {
    const dir = await open({ title: "Choose the workspace folder", directory: true, defaultPath: app.state?.workspace });
    if (typeof dir === "string") {
      await updateSettings((x) => (x.workspace = dir));
      toast("Workspace changed: jobs and runs now live there.", "success");
    }
  }
</script>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-3xl space-y-6 p-8">
    <section class="panel p-6">
      <h3 class="font-semibold">Appearance</h3>
      <p class="mb-4 text-sm faint">The 3D view and the plots follow the theme.</p>
      <div class="grid grid-cols-3 gap-3">
        {#each [
          { v: "system", label: "System", icon: Monitor, tip: "photonoxide dark or light, as the system is" },
          { v: "dark", label: "photonoxide dark", icon: Moon, tip: "The studio's deep slate, for long sessions" },
          { v: "light", label: "photonoxide light", icon: Sun, tip: "The studio's paper white" },
        ] as t (t.v)}
          {@const Icon = t.icon}
          <button
            class="flex items-center gap-2.5 rounded-xl border px-4 py-3 text-sm transition-colors {s.theme === t.v ? 'border-primary bg-primary/8 text-primary' : 'border-base-content/10 hover:border-base-content/25'}"
            title={t.tip}
            onclick={() => updateSettings((x) => (x.theme = t.v))}
          >
            <Icon size={17} />{t.label}
          </button>
        {/each}
      </div>

      <div class="mt-5 flex items-center justify-between gap-4">
        <div>
          <p class="text-sm font-medium">daisyUI's themes</p>
          <p class="text-xs faint">Each card is drawn in its theme. The sun and moon at the top go back to photonoxide's.</p>
        </div>
        <label class="input input-sm w-48 shrink-0">
          <Search size={14} class="opacity-50" />
          <input type="search" placeholder="Search the themes" bind:value={query} />
        </label>
      </div>
      {#each [{ label: "Light", dark: false }, { label: "Dark", dark: true }] as group (group.label)}
        {@const themes = DAISY_THEMES.filter((t) => t.dark === group.dark && t.name.includes(query.trim().toLowerCase()))}
        {#if themes.length}
          <p class="panel-title mt-4 mb-2">{group.label}</p>
          <div class="grid grid-cols-4 gap-2.5">
            {#each themes as t (t.name)}
              {@const chosen = s.theme === t.setting}
              <button
                class="rounded-[calc(var(--radius-box)+3px)] p-0.5 text-left outline-2 transition-colors {chosen ? 'outline-primary' : 'outline-transparent hover:outline-base-content/25'}"
                aria-pressed={chosen}
                title="Use daisyUI's {t.name} theme"
                onclick={() => updateSettings((x) => (x.theme = t.setting))}
              >
                <div data-theme={t.name} class="rounded-box border border-base-content/10 bg-base-100 px-3 py-2.5 text-base-content">
                  <div class="flex items-center justify-between gap-1">
                    <span class="truncate text-[13px] font-medium">{t.name}</span>
                    {#if chosen}<Check size={14} class="shrink-0 text-primary" />{/if}
                  </div>
                  <div class="mt-2 flex gap-1">
                    {#each ["bg-base-100 border border-base-content/20", "bg-primary", "bg-secondary", "bg-accent", "bg-neutral"] as swatch, k (k)}
                      <span class="h-4 flex-1 rounded-selector {swatch}"></span>
                    {/each}
                  </div>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      {/each}
      <div class="mt-5 flex items-center justify-between">
        <div>
          <p class="text-sm font-medium">Runs open in</p>
          <p class="text-xs faint">The view a run shows first; the switch is in the viewer.</p>
        </div>
        <div class="join">
          <button class="btn join-item btn-sm {s.view === '3d' ? 'btn-primary btn-soft' : ''}" onclick={() => updateSettings((x) => (x.view = "3d"))}>3D</button>
          <button class="btn join-item btn-sm {s.view === '2d' ? 'btn-primary btn-soft' : ''}" onclick={() => updateSettings((x) => (x.view = "2d"))}>2D</button>
        </div>
      </div>
    </section>

    <section class="panel p-6">
      <h3 class="font-semibold">Workspace</h3>
      <p class="mb-4 text-sm faint">The folder that holds your job files (<code>jobs/</code>) and runs (<code>runs/</code>).</p>
      <div class="flex items-center gap-2 rounded-xl border border-base-content/10 bg-base-200/60 px-4 py-3">
        <FolderOpen size={17} class="shrink-0 text-primary" />
        <span class="selectable min-w-0 flex-1 truncate font-mono text-[12.5px]">{app.state?.workspace}</span>
        <button class="btn btn-ghost btn-xs" onclick={() => app.state && revealItemInDir(app.state.workspace)}>Show</button>
      </div>
      <div class="mt-3 flex gap-2">
        <button class="btn btn-sm" onclick={chooseWorkspace}>Choose another folder…</button>
        {#if s.workspace}
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => updateSettings((x) => (x.workspace = null))} title="Back to the default: a project folder the program was started in, else Documents/photonoxide">
            <RotateCcw size={14} /> Use the default
          </button>
        {/if}
      </div>
    </section>

    <section class="panel p-6">
      <h3 class="flex items-center gap-2 font-semibold"><Lightbulb size={17} class="text-primary" /> Tips</h3>
      <label class="mt-3 flex cursor-pointer items-center justify-between gap-4">
        <span>
          <span class="block text-sm font-medium">Show tips on the pages</span>
          <span class="block text-xs faint">Short hints at the top of each page, each closable on its own.</span>
        </span>
        <input type="checkbox" class="toggle toggle-primary" checked={s.hints} onchange={() => updateSettings((x) => (x.hints = !x.hints))} />
      </label>
      <div class="mt-4 flex gap-2">
        <button class="btn btn-sm" disabled={!s.dismissed.length} onclick={() => updateSettings((x) => (x.dismissed = []))}>Bring back the {s.dismissed.length} closed tip{s.dismissed.length === 1 ? "" : "s"}</button>
        <button class="btn btn-ghost btn-sm" onclick={() => (app.tour = true)}>Take the tour</button>
      </div>
    </section>

    <section class="panel p-6">
      <h3 class="flex items-center gap-2 font-semibold"><Download size={17} class="text-primary" /> Updates</h3>
      <label class="mt-3 flex cursor-pointer items-center justify-between gap-4">
        <span>
          <span class="block text-sm font-medium">Look for new releases</span>
          <span class="block text-xs faint">When photonoxide opens, and every hour while it stays open. When one is out, an Update button appears at the top; one click downloads, installs and restarts. Releases are signed, and the signature is checked before installing.</span>
        </span>
        <input type="checkbox" class="toggle toggle-primary" checked={s.check_updates} onchange={() => updateSettings((x) => (x.check_updates = !x.check_updates))} />
      </label>
      <div class="mt-4 flex flex-wrap items-center gap-3">
        <button class="btn btn-sm gap-1.5" onclick={() => checkForUpdate(false)} disabled={updater.status === "checking"}>
          <RefreshCw size={14} class={updater.status === "checking" ? "animate-spin" : ""} /> Check now
        </button>
        {#if updater.status === "available"}
          <button class="btn btn-primary btn-sm" onclick={() => (updater.dialog = true)}>Update to {updater.version}</button>
        {:else if updater.status === "publishing"}
          <span class="text-sm text-info">{updater.version} is being published: ready in a few minutes.</span>
        {:else if updater.status === "none"}
          <span class="text-sm text-success">You have the latest release.</span>
        {:else if updater.status === "error"}
          <span class="selectable text-xs text-error">{updater.error}</span>
        {/if}
        {#if !app.state?.updatable}
          <span class="text-xs faint">This copy was built from the repository; installed releases update themselves.</span>
        {/if}
      </div>
    </section>

    <section class="panel p-6">
      <h3 class="font-semibold">About</h3>
      <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 text-sm">
        <dt class="faint">Version</dt><dd class="num">photonoxide {app.state?.version}</dd>
        <dt class="faint">Platform</dt><dd class="num">{app.state?.platform}</dd>
        <dt class="faint">License</dt><dd>MIT or Apache-2.0</dd>
      </dl>
      <div class="mt-4 flex flex-wrap gap-2">
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => openUrl("https://github.com/tachsin/photonoxide")}><ExternalLink size={14} /> Source code</button>
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => openUrl("https://tachsin.gr/projects/photonoxide")}><ExternalLink size={14} /> Documentation</button>
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => openUrl("https://github.com/tachsin/photonoxide/issues/new")}><ExternalLink size={14} /> Report a problem</button>
      </div>
    </section>
  </div>
</div>
