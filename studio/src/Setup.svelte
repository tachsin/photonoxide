<script lang="ts">
  // The Windows setup's window (src-tauri/src/setup.rs): the install or the uninstall. It asks
  // first (the folder and a Desktop shortcut, or whether the settings go too), then shows each
  // step as a line as it happens, under the progress. Started with /P, it shows only the
  // progress and closes by itself.
  import { onMount, tick } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Check, CircleAlert, FolderOpen, HardDrive, Minus, Monitor, Trash2, User, X } from "@lucide/svelte";
  import Logo from "./components/Logo.svelte";

  type Info = {
    kind: "install" | "uninstall";
    show: "window" | "progress" | "nothing";
    version: string;
    dir: string;
    installed: string | null;
    desktop: boolean;
    size: number;
    update: boolean;
    starts_again: boolean;
    homepage: string;
  };
  type Note = { kind: "line"; text: string } | { kind: "progress"; done: number };

  let info = $state<Info | null>(null);
  let phase = $state<"choose" | "working" | "done" | "failed">("choose");
  let dir = $state("");
  let desktop = $state(false);
  let deleteData = $state(false);
  let launch = $state(true);
  let done = $state(0);
  let lines = $state<{ id: number; text: string }[]>([]);
  let error = $state("");
  let log: HTMLElement | undefined = $state();

  const still = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const motion = (ms: number) => (still ? 0 : ms);
  const installing = $derived(info?.kind === "install");
  const passive = $derived(info?.show === "progress");
  const mib = $derived(info ? (info.size / 1048576).toFixed(0) : "");

  const title = $derived.by(() => {
    if (!info) return "";
    if (!installing) {
      return { choose: `Uninstall photonoxide ${info.installed ?? ""}`.trim(), working: "Uninstalling", done: "photonoxide is uninstalled", failed: "The uninstall stopped" }[phase];
    }
    const verb = info.installed ? (info.installed === info.version ? "Reinstall" : "Update") : "Install";
    return {
      choose: `${verb} photonoxide ${info.version}`,
      working: info.installed ? `Updating to ${info.version}` : `Installing photonoxide ${info.version}`,
      done: info.installed ? `photonoxide ${info.version} is ready` : "photonoxide is installed",
      failed: "The install stopped",
    }[phase];
  });

  onMount(() => {
    const unlisten = [
      listen<Note>("setup-note", ({ payload }) => {
        if (payload.kind === "progress") done = payload.done;
        else {
          lines.push({ id: lines.length, text: payload.text });
          tick().then(() => log?.scrollTo({ top: log.scrollHeight, behavior: still ? "auto" : "smooth" }));
        }
      }),
      listen<{ ok: boolean; error: string | null }>("setup-done", ({ payload }) => {
        if (payload.ok) {
          phase = "done";
          done = 1;
          // an update starts the program again by itself, a moment after it's done
          if (passive) setTimeout(() => finish(info?.starts_again ?? false), 1200);
        } else {
          phase = "failed";
          error = payload.error ?? "it stopped without saying why";
        }
      }),
    ];
    invoke<Info>("setup_info").then((i) => {
      info = i;
      dir = i.dir;
      desktop = i.desktop;
      launch = true;
      if (i.show === "progress") start();
    });
    return () => unlisten.forEach((u) => u.then((f) => f()));
  });

  async function choose() {
    const picked = await open({ directory: true, defaultPath: dir, title: "Where to install photonoxide" });
    if (typeof picked === "string") dir = picked.replace(/[\\/]+$/, "").endsWith("photonoxide") ? picked : `${picked.replace(/[\\/]+$/, "")}\\photonoxide`;
  }

  async function start() {
    phase = "working";
    try {
      await invoke("setup_start", { dir, desktop, deleteData });
    } catch (e) {
      phase = "failed";
      error = String(e);
    }
  }

  function finish(start = false) {
    invoke("setup_finish", { launch: start });
  }

  function close() {
    if (phase !== "working") finish(false);
  }
</script>

<div class="setup">
  <aside class="brand" aria-hidden="true">
    <div class="glow"></div>
    <svg class="wave" viewBox="0 0 300 120" preserveAspectRatio="none">
      <defs>
        <linearGradient id="setup-fade" x1="0" x2="1">
          <stop offset="0" stop-color="#ee8a32" stop-opacity="0" />
          <stop offset="0.5" stop-color="#f7b955" stop-opacity="0.9" />
          <stop offset="1" stop-color="#ee8a32" stop-opacity="0" />
        </linearGradient>
      </defs>
      {#each [0, 1, 2] as k (k)}
        <path class="crest crest-{k}" d={Array.from({ length: 121 }, (_, i) => {
          const x = i * 5 - 150;
          const envelope = 26 * Math.exp(-(((x - 150) / 110) ** 2));
          return `${i ? "L" : "M"}${x} ${60 + envelope * Math.sin((2 * Math.PI * x) / 46 + (2 * Math.PI * k) / 3)}`;
        }).join(" ")} />
      {/each}
    </svg>
    {#each Array.from({ length: 18 }, (_, i) => i) as i (i)}
      <span class="photon" style="--x: {((i * 37) % 100) / 100}; --d: {(i % 6) * 0.7}s; --s: {2 + (i % 3)}px"></span>
    {/each}
    <div class="mark">
      <Logo size={104} />
      <div class="name"><span>photon</span><span class="oxide">oxide</span></div>
      {#if info}<div class="version">{info.version}</div>{/if}
    </div>
  </aside>

  <main>
    <header data-tauri-drag-region>
      <span class="caption" data-tauri-drag-region>{installing ? "Setup" : "Uninstall"}</span>
      <div class="controls">
        <button class="ctl" aria-label="Minimize" onclick={() => getCurrentWindow().minimize()}><Minus size={15} /></button>
        <button class="ctl close" aria-label="Close" disabled={phase === "working"} onclick={close}><X size={15} /></button>
      </div>
    </header>

    {#if info}
      {#key phase}
        <section class="body" in:fly={{ y: 10, duration: motion(260), delay: motion(80) }} out:fade={{ duration: motion(80) }}>
          <h1>{title}</h1>

          {#if phase === "choose" && installing}
            <p class="lead">
              Photonics simulation, from a waveguide's modes to whole circuits, heaters and modulators, with the studio that runs your jobs live, replays their records and reproduces published results.
            </p>
            <label class="box folder">
              <span class="label"><FolderOpen size={16} /> Folder</span>
              <span class="path" title={dir}>{dir}</span>
              <button class="btn btn-sm btn-ghost" onclick={choose}>Change…</button>
            </label>
            <label class="box option">
              <input type="checkbox" class="checkbox checkbox-sm checkbox-primary" bind:checked={desktop} />
              <Monitor size={16} /> A shortcut on the Desktop
            </label>
            <ul class="facts">
              <li><User size={14} /> For you only: no administrator needed</li>
              <li><HardDrive size={14} /> {mib} MiB on disk, one program</li>
            </ul>
          {:else if phase === "choose"}
            <p class="lead">This removes the program, its shortcuts and its entry in Apps &amp; Features. Your jobs and runs stay where they are.</p>
            <label class="box option">
              <input type="checkbox" class="checkbox checkbox-sm checkbox-error" bind:checked={deleteData} />
              <Trash2 size={16} /> Also remove its settings and benchmark records
            </label>
          {:else}
            <div class="progress-row">
              <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(done * 100)}>
                <div class="fill" class:finished={phase === "done"} class:stopped={phase === "failed"} style="width: {Math.max(done, 0.02) * 100}%"></div>
              </div>
              <span class="percent">{Math.round(done * 100)}%</span>
            </div>
            <ol class="log" bind:this={log}>
              {#each lines as l (l.id)}
                <li in:fly={{ x: -8, duration: motion(200) }}><Check size={13} class="tick" /> <span>{l.text}</span></li>
              {/each}
            </ol>
            {#if phase === "failed"}
              <div class="failure" in:fade={{ duration: motion(160) }}><CircleAlert size={16} /> <span class="selectable">{error}</span></div>
            {/if}
          {/if}
        </section>
      {/key}

      <footer>
        {#if phase === "choose"}
          <button class="btn btn-ghost" onclick={() => finish(false)}>Cancel</button>
          {#if installing}
            <button class="btn btn-primary px-6" onclick={start} disabled={!dir.trim()}>{info.installed ? (info.installed === info.version ? "Reinstall" : "Update") : "Install"}</button>
          {:else}
            <button class="btn btn-error px-6" onclick={start}>Uninstall</button>
          {/if}
        {:else if phase === "done"}
          {#if installing && !passive}
            <label class="start"><input type="checkbox" class="checkbox checkbox-sm checkbox-primary" bind:checked={launch} /> Start photonoxide</label>
          {/if}
          <button class="btn btn-ghost btn-sm" onclick={() => openUrl(info!.homepage).catch(() => {})}>photonoxide's page</button>
          <button class="btn btn-primary px-6" onclick={() => finish(installing && launch)}>{installing && launch ? "Open photonoxide" : "Close"}</button>
        {:else if phase === "failed"}
          <button class="btn px-6" onclick={() => finish(false)}>Close</button>
        {:else}
          <span class="quiet">{installing ? "Copying and registering…" : "Removing…"}</span>
        {/if}
      </footer>
    {/if}
  </main>
</div>

<style>
  /* the oxide's copper for the buttons and boxes, on either theme */
  .setup {
    --color-primary: #ee8a32;
    --color-primary-content: #1b120e;
    display: grid;
    grid-template-columns: 300px 1fr;
    height: 100vh;
    overflow: hidden;
    user-select: none;
    background: var(--color-base-100);
    color: var(--color-base-content);
  }

  /* the brand: the mark over the oxide, a guided mode's field moving through it */
  .brand {
    position: relative;
    overflow: hidden;
    background: linear-gradient(180deg, #2b1d17, #161010);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .glow {
    position: absolute;
    inset: -20% -40% auto;
    height: 70%;
    background: radial-gradient(closest-side, rgba(183, 65, 14, 0.38), transparent);
  }
  .mark {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    margin-top: -64px;
  }
  .name {
    font-weight: 700;
    font-size: 30px;
    letter-spacing: -0.02em;
    color: #f6ede5;
  }
  .oxide {
    background: linear-gradient(90deg, #b7410e, #ee8a32 55%, #f7b955);
    background-clip: text;
    color: transparent;
  }
  .version {
    font-size: 12px;
    color: #d8c3ae;
    border: 1px solid rgba(238, 138, 50, 0.35);
    border-radius: 999px;
    padding: 2px 10px;
  }
  .wave {
    position: absolute;
    left: 0;
    bottom: 56px;
    width: 100%;
    height: 120px;
  }
  .crest {
    fill: none;
    stroke: url(#setup-fade);
    stroke-width: 2;
    animation: travel 3.2s linear infinite;
  }
  .crest-1 {
    opacity: 0.55;
    animation-delay: -1.07s;
  }
  .crest-2 {
    opacity: 0.3;
    animation-delay: -2.13s;
  }
  @keyframes travel {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(46px);
    }
  }
  .photon {
    position: absolute;
    bottom: calc(80px + var(--x) * 70px);
    left: calc(10% + var(--x) * 80%);
    width: var(--s);
    height: var(--s);
    border-radius: 50%;
    background: #f7b955;
    box-shadow: 0 0 8px #ee8a32;
    opacity: 0;
    animation: glint 4.2s ease-in-out infinite;
    animation-delay: var(--d);
  }
  @keyframes glint {
    0%,
    100% {
      opacity: 0;
      transform: translateY(6px);
    }
    50% {
      opacity: 0.85;
      transform: translateY(-6px);
    }
  }

  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding-left: 28px;
  }
  .caption {
    font-size: 12px;
    opacity: 0.55;
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .ctl {
    width: 46px;
    display: grid;
    place-items: center;
    opacity: 0.7;
    transition: background 120ms, opacity 120ms;
  }
  .ctl:hover:not(:disabled) {
    opacity: 1;
    background: color-mix(in oklch, var(--color-base-content) 9%, transparent);
  }
  .ctl.close:hover:not(:disabled) {
    background: #c42b1c;
    color: white;
  }
  .ctl:disabled {
    opacity: 0.25;
  }

  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 12px 32px 0;
    min-height: 0;
  }
  h1 {
    font-size: 24px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .lead {
    font-size: 14px;
    line-height: 1.55;
    opacity: 0.75;
    margin-bottom: 4px;
  }
  .box {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid color-mix(in oklch, var(--color-base-content) 12%, transparent);
    background: color-mix(in oklch, var(--color-base-content) 3%, transparent);
    border-radius: 12px;
    padding: 10px 12px;
    font-size: 14px;
    transition: border-color 140ms;
  }
  .box:hover {
    border-color: color-mix(in oklch, var(--color-primary) 45%, transparent);
  }
  .option {
    cursor: pointer;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    opacity: 0.7;
    flex: none;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12.5px;
  }
  .facts {
    display: flex;
    gap: 18px;
    font-size: 12.5px;
    opacity: 0.6;
  }
  .facts li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .progress-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .bar {
    flex: 1;
    height: 8px;
    border-radius: 999px;
    overflow: hidden;
    background: color-mix(in oklch, var(--color-base-content) 10%, transparent);
  }
  .fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, #b7410e, #ee8a32, #f7b955, #ee8a32);
    background-size: 200% 100%;
    animation: shine 1.6s linear infinite;
    transition: width 220ms ease-out;
  }
  .fill.finished {
    animation: none;
    background: linear-gradient(90deg, #ee8a32, #f7b955);
  }
  .fill.stopped {
    animation: none;
    background: var(--color-error);
  }
  @keyframes shine {
    from {
      background-position: 200% 0;
    }
    to {
      background-position: 0 0;
    }
  }
  .percent {
    font-variant-numeric: tabular-nums;
    font-size: 13px;
    width: 3.2em;
    text-align: right;
    opacity: 0.7;
  }
  /* what is being done, as it is done: shown, not behind a button */
  .log {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border-radius: 12px;
    padding: 10px 12px;
    background: #161010;
    color: #fbe7c6;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    line-height: 1.7;
    user-select: text;
  }
  .log li {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .log :global(.tick) {
    color: #f7b955;
    flex: none;
    transform: translateY(2px);
  }
  .failure {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    color: var(--color-error);
    font-size: 13px;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    padding: 18px 32px 24px;
  }
  .start {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    margin-right: auto;
    cursor: pointer;
  }
  .quiet {
    font-size: 13px;
    opacity: 0.6;
  }

  @media (prefers-reduced-motion: reduce) {
    .crest,
    .photon,
    .fill {
      animation: none;
    }
    .photon {
      opacity: 0.5;
    }
  }
</style>
