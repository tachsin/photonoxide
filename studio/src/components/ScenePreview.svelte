<script lang="ts">
  // A job's structure in 3D, turning slowly: what its run will draw, before it runs.
  import { Box } from "@lucide/svelte";

  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import type { Scene } from "../lib/events";
  import { Preview3D } from "../lib/view3d";

  let { text, height = 150 }: { text: string; height?: number } = $props();

  let host: HTMLDivElement;
  let failed = $state("");
  let loading = $state(true);
  let view: Preview3D | null = null;

  /** Scenes already built, by job text: the cards share them across pages. */
  const cache = (globalThis as { __scenes?: Map<string, Scene> }).__scenes ??= new Map();

  $effect(() => {
    const t = text;
    let gone = false;
    loading = true;
    failed = "";
    const ready = cache.has(t) ? Promise.resolve(cache.get(t)!) : api.previewScene(t);
    ready
      .then((scene) => {
        cache.set(t, scene);
        if (gone) return;
        view?.destroy();
        view = new Preview3D(host, scene, app.dark);
        loading = false;
      })
      .catch((e) => {
        if (!gone) {
          failed = String(e);
          loading = false;
        }
      });
    return () => {
      gone = true;
    };
  });

  $effect(() => view?.setDark(app.dark));
  $effect(() => () => view?.destroy());
</script>

<div class="relative w-full" style="height: {height}px">
  <div bind:this={host} class="absolute inset-0"></div>
  {#if loading}<div class="skeleton absolute inset-0"></div>{/if}
  {#if failed}
    <div class="absolute inset-0 grid place-items-center p-3 text-center text-xs faint" title={failed}>
      <span class="flex flex-col items-center gap-1"><Box size={18} /> no preview: the job isn't valid yet</span>
    </div>
  {/if}
</div>
