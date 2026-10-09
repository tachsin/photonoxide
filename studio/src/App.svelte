<script lang="ts">
  // The window: the navigation rail, the top bar, the page, and what floats over them.
  import { onMount } from "svelte";

  import CommandPalette from "./components/CommandPalette.svelte";
  import NavRail from "./components/NavRail.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Tour from "./components/Tour.svelte";
  import TouchstoneImport from "./components/TouchstoneImport.svelte";
  import NightlyDialog from "./components/NightlyDialog.svelte";
  import UpdateDialog from "./components/UpdateDialog.svelte";
  import { app, boot } from "./lib/app.svelte";
  import { nightlyBoot } from "./lib/nightly.svelte";
  import { checkForUpdate } from "./lib/updater.svelte";
  import Academy from "./pages/Academy.svelte";
  import Benchmarks from "./pages/Benchmarks.svelte";
  import Builder from "./pages/Builder.svelte";
  import ChipPage from "./pages/Chip.svelte";
  import Compare from "./pages/Compare.svelte";
  import Components from "./pages/Components.svelte";
  import Examples from "./pages/Examples.svelte";
  import Home from "./pages/Home.svelte";
  import Libraries from "./pages/Libraries.svelte";
  import Materials from "./pages/Materials.svelte";
  import Runs from "./pages/Runs.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import Validation from "./pages/Validation.svelte";
  import Viewer from "./pages/Viewer.svelte";

  onMount(() => {
    boot().then(() => {
      // a nightly build confirms it started, and a build running or waiting shows
      nightlyBoot();
      // a moment after the window opens, so the first paint isn't held up
      if (app.state?.settings.check_updates) setTimeout(() => checkForUpdate(true), 2500);
    });
    // and every hour while the window stays open, as long as the setting is on
    const hourly = setInterval(() => {
      if (app.state?.settings.check_updates) checkForUpdate(true);
    }, 3_600_000);
    return () => clearInterval(hourly);
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
      {:else if app.page === "examples"}
        <Examples />
      {:else if app.page === "builder"}
        <Builder />
      {:else if app.page === "runs"}
        <Runs />
      {:else if app.page === "compare"}
        <Compare />
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
      {:else if app.page === "academy"}
        <Academy />
      {:else if app.page === "settings"}
        <SettingsPage />
      {/if}
      <!-- the viewer stays mounted, so its 3D view keeps its camera between visits -->
      <div class="h-full" class:hidden={app.page !== "viewer"}><Viewer /></div>
    </main>
    <StatusBar />
  </div>
  <CommandPalette />
  <UpdateDialog />
  <NightlyDialog />
  <TouchstoneImport />
  <Tour />
  <Toasts />
{:else}
  <div class="grid h-full place-items-center">
    <span class="loading loading-ring loading-lg text-primary"></span>
  </div>
{/if}
