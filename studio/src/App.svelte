<script lang="ts">
  // The window: the navigation rail, the top bar, the page, and what floats over them.
  import { onMount } from "svelte";

  import CommandPalette from "./components/CommandPalette.svelte";
  import NavRail from "./components/NavRail.svelte";
  import RunsDrawer from "./components/RunsDrawer.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Welcome from "./components/Welcome.svelte";
  import TouchstoneImport from "./components/TouchstoneImport.svelte";
  import NightlyDialog from "./components/NightlyDialog.svelte";
  import UpdateDialog from "./components/UpdateDialog.svelte";
  import { app, boot } from "./lib/app.svelte";
  import { nightlyBoot } from "./lib/nightly.svelte";
  import { startSchedule } from "./lib/schedule";
  import { checkForUpdate, updater } from "./lib/updater.svelte";
  import Academy from "./pages/Academy.svelte";
  import Benchmarks from "./pages/Benchmarks.svelte";
  import ChipPage from "./pages/Chip.svelte";
  import Components from "./pages/Components.svelte";
  import Fabrication from "./pages/Fabrication.svelte";
  import Home from "./pages/Home.svelte";
  import Libraries from "./pages/Libraries.svelte";
  import Materials from "./pages/Materials.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import Validation from "./pages/Validation.svelte";
  import Viewer from "./pages/Viewer.svelte";

  onMount(() => {
    let stop = () => {};
    let gone = false;
    boot().then(() => {
      // a nightly build confirms it started, and a build running or waiting shows
      nightlyBoot();
      if (gone) return;
      // updates: at once (neither call waits for the other, nor holds up the window), when the
      // window comes back 15 minutes or more after the last check, and every hour; each only
      // while the setting is on
      stop = startSchedule({
        enabled: () => !!app.state?.settings.check_updates,
        checkedAt: () => updater.checkedAt,
        check: (why) => void checkForUpdate(true, why),
        now: Date.now,
        every: (run, ms) => {
          const t = setInterval(run, ms);
          return () => clearInterval(t);
        },
        onFocus: (run) => {
          const shown = () => document.visibilityState === "visible" && run();
          window.addEventListener("focus", run);
          document.addEventListener("visibilitychange", shown);
          return () => {
            window.removeEventListener("focus", run);
            document.removeEventListener("visibilitychange", shown);
          };
        },
      });
    });
    return () => {
      gone = true;
      stop();
    };
  });

  function keys(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      app.palette = !app.palette;
    }
  }
</script>

<svelte:window onkeydown={keys} />

{#if app.ready}
  <div class="grid h-full grid-cols-[auto_minmax(0,1fr)] grid-rows-[auto_1fr_auto]">
    <div class="row-span-3"><NavRail /></div>
    <TopBar />
    <main class="min-h-0 min-w-0 overflow-hidden">
      {#if app.page === "home"}
        <Home />
      {:else if app.page === "materials"}
        <Materials />
      {:else if app.page === "components"}
        <Components />
      {:else if app.page === "chip"}
        <ChipPage />
      {:else if app.page === "validation"}
        <Validation />
      {:else if app.page === "libraries"}
        <Libraries />
      {:else if app.page === "benchmarks"}
        <Benchmarks />
      {:else if app.page === "fabrication"}
        <Fabrication />
      {:else if app.page === "academy"}
        <Academy />
      {:else if app.page === "settings"}
        <SettingsPage />
      {/if}
      <!-- the run view stays mounted, so its 3D view keeps its camera between visits -->
      <div class="h-full" class:hidden={app.page !== "viewer"}><Viewer /></div>
    </main>
    <StatusBar />
  </div>
  <RunsDrawer />
  <CommandPalette />
  <UpdateDialog />
  <NightlyDialog />
  <TouchstoneImport />
  <Welcome />
  <Toasts />
{:else}
  <div class="grid h-full place-items-center">
    <span class="loading loading-ring loading-lg text-primary"></span>
  </div>
{/if}
