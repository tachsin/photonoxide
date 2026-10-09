<script lang="ts">
  // Settings: appearance, the workspace, tips, updates, and what this copy is.
  import { Check, ChevronDown, Download, ExternalLink, FolderOpen, GitBranch, Hammer, Lightbulb, Monitor, Moon, RefreshCw, RotateCcw, Search, Sun, Trash2, Undo2 } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

  import { app, toast, updateSettings } from "../lib/app.svelte";
  import { MORE_THEMES } from "../lib/themes";
  import { lengthUnit } from "../lib/units";
  import { available, building, clearCache, nightly, refresh, rollBack } from "../lib/nightly.svelte";
  import { checkForUpdate, updater } from "../lib/updater.svelte";

  const s = $derived(app.state!.settings);
  let query = $state("");
  /** Whether the further themes are listed: closed until asked for. */
  let more = $state(false);
  /** The theme in use, by name: the studio's own, or one of the further ones. */
  const themeLabel = $derived(
    s.theme === "dark" || s.theme === "light"
      ? `photonoxide ${s.theme}`
      : (MORE_THEMES.find((t) => t.setting === s.theme)?.name ?? `photonoxide ${app.dark ? "dark" : "light"}, as the system is`),
  );

  const build = $derived(app.state!.build);
  const nightlyOn = $derived(s.channel === "nightly");
  /** Going back to the previous build, waiting for the user's confirmation. */
  let confirmingBack = $state(false);

  /** Switches the channel, and looks for an update on the new one. */
  async function switchChannel(channel: "stable" | "nightly") {
    if (s.channel === channel) return;
    await updateSettings((x) => (x.channel = channel));
    if (channel === "nightly") {
      toast("Nightly: photonoxide now follows main, and offers to build its new commits here.", "info", undefined, 7000);
      await refresh();
    } else if (build.channel === "nightly") {
      toast("Stable: the latest release replaces this nightly build when you update, even if its version number is lower.", "info", undefined, 9000);
    }
    checkForUpdate(false);
  }

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

      <!-- the theme in use, as it looks: its surfaces, its text and its colours -->
      <div class="mt-4 overflow-hidden rounded-box border border-base-content/10" aria-label="The theme in use">
        <div class="flex items-center gap-2 border-b border-base-content/10 bg-base-200 px-4 py-2">
          <span class="size-2.5 rounded-full bg-error/70"></span><span class="size-2.5 rounded-full bg-warning/70"></span><span class="size-2.5 rounded-full bg-success/70"></span>
          <span class="ml-2 text-xs faint">In use</span>
          <span class="text-xs font-medium">{themeLabel}</span>
        </div>
        <div class="grid grid-cols-[1fr_auto] gap-5 bg-base-100 px-4 py-4">
          <div class="min-w-0 space-y-2.5">
            <p class="text-sm font-semibold">A strip waveguide's modes</p>
            <p class="text-xs leading-relaxed muted">Text, panels, buttons and plots take these colours; the 3D view's backdrop is the darker surface.</p>
            <div class="flex flex-wrap items-center gap-2">
              <span class="btn btn-primary btn-xs">Run</span>
              <span class="btn btn-xs">Edit</span>
              <span class="badge badge-soft badge-primary badge-sm">modes</span>
              <span class="badge badge-soft badge-secondary badge-sm">TM-like</span>
              <span class="badge badge-soft badge-success badge-sm">passed</span>
            </div>
          </div>
          <div class="grid grid-cols-4 content-start gap-1.5">
            {#each [["bg-base-100", "surface"], ["bg-base-200", "backdrop"], ["bg-base-300", "inset"], ["bg-neutral", "neutral"], ["bg-primary", "primary"], ["bg-secondary", "secondary"], ["bg-accent", "accent"], ["bg-info", "info"]] as [swatch, name] (name)}
              <span class="size-7 rounded-selector border border-base-content/15 {swatch}" title={name}></span>
            {/each}
          </div>
        </div>
      </div>

      <div class="mt-4 flex items-center justify-between gap-4">
        <button class="btn btn-ghost btn-sm -ml-2 gap-1.5" aria-expanded={more} onclick={() => (more = !more)} title="Further themes to choose from; the three at the top are photonoxide's own">
          <ChevronDown size={15} class="transition-transform {more ? 'rotate-180' : ''}" />
          {more ? "Hide the other themes" : `More themes (${MORE_THEMES.length})`}
        </button>
        {#if more}
          <label class="input input-sm w-48 shrink-0">
            <Search size={14} class="opacity-50" />
            <input type="search" placeholder="Search the themes" bind:value={query} />
          </label>
        {/if}
      </div>
      {#if more}
        <p class="mt-1 text-xs faint">Each card is drawn in its theme. The sun and moon at the top go back to photonoxide's.</p>
      {/if}
      {#each more ? [{ label: "Light", dark: false }, { label: "Dark", dark: true }] : [] as group (group.label)}
        {@const themes = MORE_THEMES.filter((t) => t.dark === group.dark && t.name.includes(query.trim().toLowerCase()))}
        {#if themes.length}
          <p class="panel-title mt-4 mb-2">{group.label}</p>
          <div class="grid grid-cols-4 gap-2.5">
            {#each themes as t (t.name)}
              {@const chosen = s.theme === t.setting}
              <button
                class="rounded-[calc(var(--radius-box)+3px)] p-0.5 text-left outline-2 transition-colors {chosen ? 'outline-primary' : 'outline-transparent hover:outline-base-content/25'}"
                aria-pressed={chosen}
                title="Use the {t.name} theme"
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
      <div class="mt-5 flex items-center justify-between gap-4">
        <div>
          <p class="text-sm font-medium">Lengths in</p>
          <p class="text-xs faint">Every length and wavelength the studio shows or asks for, on every page. A unit beside a value switches it too, wherever it is. Job and chip files keep their own units.</p>
        </div>
        <div class="join shrink-0" role="radiogroup" aria-label="The unit of lengths">
          {#each [["um", "µm", "Micrometres: 1.55 µm, 0.22 µm"], ["nm", "nm", "Nanometres: 1550 nm, 220 nm"]] as [v, text, tip] (v)}
            <button
              class="btn join-item btn-sm {lengthUnit() === v ? 'btn-primary btn-soft' : ''}"
              role="radio"
              aria-checked={lengthUnit() === v}
              title={tip}
              onclick={() => updateSettings((x) => (x.length_unit = v))}>{text}</button
            >
          {/each}
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
        <button class="btn btn-sm" disabled={!s.dismissed.length} onclick={() =>
            updateSettings((x) => {
              x.dismissed = [];
            })}>Bring back the {s.dismissed.length} closed tip{s.dismissed.length === 1 ? "" : "s"}</button>
        <button class="btn btn-ghost btn-sm" onclick={() => (app.tour = true)}>Take the tour</button>
      </div>
    </section>

    <section class="panel p-6">
      <h3 class="flex items-center gap-2 font-semibold"><Download size={17} class="text-primary" /> Updates</h3>
      <div class="mt-3 flex items-center justify-between gap-4">
        <span>
          <span class="block text-sm font-medium">Channel</span>
          <span class="block text-xs faint">
            {#if nightlyOn}Nightly: built here from main's latest code, tested by CI and nothing more; it needs Rust, Node and the platform's build tools.
            {:else}Stable: the signed releases, built and tested before they are published.{/if}
          </span>
        </span>
        <div class="join shrink-0" role="radiogroup" aria-label="The release channel">
          {#each [["stable", "Stable", "The signed releases from GitHub"], ["nightly", "Nightly", "main's latest commit, built here from its source"]] as [v, text, tip] (v)}
            <button
              class="btn join-item btn-sm {(nightlyOn ? 'nightly' : 'stable') === v ? 'btn-primary btn-soft' : ''}"
              role="radio"
              aria-checked={(nightlyOn ? "nightly" : "stable") === v}
              title={tip}
              onclick={() => switchChannel(v as "stable" | "nightly")}>{text}</button
            >
          {/each}
        </div>
      </div>
      <label class="mt-4 flex cursor-pointer items-center justify-between gap-4">
        <span>
          <span class="block text-sm font-medium">{nightlyOn ? "Look at main for new commits" : "Look for new releases"}</span>
          <span class="block text-xs faint">
            {#if nightlyOn}When photonoxide opens, and every hour while it stays open, it asks GitHub for main's latest commit. When main moved, a button at the top lists the new commits and offers to build them here; nothing is built or installed without your click.
            {:else}When photonoxide opens, and every hour while it stays open. When one is out, an Update button appears at the top; one click downloads, installs and restarts. Releases are signed, and the signature is checked before installing.{/if}
          </span>
        </span>
        <input type="checkbox" class="toggle toggle-primary" checked={s.check_updates} onchange={() => updateSettings((x) => (x.check_updates = !x.check_updates))} />
      </label>
      {#if nightlyOn}
        <div class="mt-4 flex flex-wrap items-center gap-3">
          <button class="btn btn-sm gap-1.5" onclick={() => checkForUpdate(false)} disabled={nightly.checking}>
            <RefreshCw size={14} class={nightly.checking ? "animate-spin" : ""} /> Check now
          </button>
          {#if building()}
            <button class="btn btn-sm btn-soft gap-1.5" onclick={() => (nightly.dialog = true)}><span class="loading loading-spinner loading-xs"></span> Building… see the log</button>
          {:else if nightly.status?.ready}
            <button class="btn btn-primary btn-sm gap-1.5" onclick={() => (nightly.dialog = true)}><Hammer size={14} /> Install {nightly.status.ready.label}</button>
          {:else if available()}
            <button class="btn btn-primary btn-sm btn-soft gap-1.5" onclick={() => (nightly.dialog = true)}><GitBranch size={14} /> {nightly.status?.last?.new_commits} new on main: see them</button>
          {:else if nightly.status?.last?.status === "current"}
            <span class="text-sm text-success">You have main's latest commit.</span>
          {:else if nightly.status?.last}
            <button class="btn btn-ghost btn-sm" onclick={() => (nightly.dialog = true)}>main's latest commit…</button>
          {/if}
          {#if nightly.error}<span class="selectable text-xs text-error">{nightly.error}</span>{/if}
        </div>
        {#if nightly.status?.cannot_install}
          <p class="mt-3 text-xs text-warning">{nightly.status.cannot_install}</p>
        {/if}
        <div class="mt-4 space-y-2 rounded-xl border border-base-content/10 bg-base-200/60 px-4 py-3 text-xs">
          {#if nightly.status?.previous}
            <div class="flex items-center justify-between gap-3">
              <span><span class="faint">The build before the last install, kept to go back to:</span> photonoxide {nightly.status.previous.label}</span>
              {#if confirmingBack}
                <span class="flex shrink-0 gap-1">
                  <button class="btn btn-ghost btn-xs" onclick={() => (confirmingBack = false)}>Cancel</button>
                  <button class="btn btn-warning btn-xs" onclick={() => { confirmingBack = false; rollBack(); }}>Go back and restart</button>
                </span>
              {:else}
                <button class="btn btn-ghost btn-xs shrink-0 gap-1" onclick={() => (confirmingBack = true)}><Undo2 size={12} /> Go back to it…</button>
              {/if}
            </div>
          {/if}
          <div class="flex items-center justify-between gap-3">
            <span class="min-w-0 truncate"><span class="faint">Builds, their cache and log:</span> <span class="font-mono">{nightly.status?.folder}</span></span>
            <span class="flex shrink-0 gap-1">
              <button class="btn btn-ghost btn-xs" onclick={() => nightly.status && revealItemInDir(nightly.status.folder)}>Show</button>
              <button class="btn btn-ghost btn-xs gap-1" disabled={building()} title="Remove main's source and Cargo's target folder (several gigabytes): the next build compiles everything again" onclick={clearCache}><Trash2 size={12} /> Remove the cache</button>
            </span>
          </div>
        </div>
      {:else}
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
      {#if build.channel === "nightly"}
        <p class="mt-3 text-xs faint">This copy is a nightly build: the latest release replaces it when you update, even if its version number is lower.</p>
      {/if}
      {/if}
    </section>

    <section class="panel p-6">
      <h3 class="font-semibold">About</h3>
      <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 text-sm">
        <dt class="faint">Version</dt><dd class="num selectable">photonoxide {build.label}</dd>
        <dt class="faint">Built from</dt>
        <dd class="num selectable">
          {#if build.commit}
            <button class="link link-hover font-mono" title={build.commit} onclick={() => openUrl(`https://github.com/tachsin/photonoxide/commit/${build.commit}`)}>{build.short}</button>{build.date ? `, ${build.date}` : ""}{build.dirty ? ", with changes not committed" : ""}
          {:else}
            <span class="faint">a commit not known to this build</span>
          {/if}
        </dd>
        <dt class="faint">Channel</dt><dd>{build.channel === "nightly" ? "Nightly, built here from main" : "Stable"}</dd>
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
