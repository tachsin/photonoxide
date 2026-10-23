<script lang="ts">
  // A device in 3D, as its job draws it (photonoxide::job::preview of its [device] table):
  // turning slowly, and by a drag.
  import { app } from "../../lib/app.svelte";
  import type { Scene } from "../../lib/events";
  import { Preview3D } from "../../lib/view3d";

  let { scene, height = 320 }: { scene: Scene; height?: number } = $props();

  let host: HTMLDivElement;
  let view: Preview3D | null = null;

  $effect(() => {
    const s = scene;
    view ??= new Preview3D(host, app.dark, true, true);
    view.setScene(s);
  });
  $effect(() => view?.setDark(app.dark));
  $effect(() => () => {
    view?.destroy();
    view = null;
  });
</script>

<div class="relative w-full" style="height: {height}px">
  <div bind:this={host} class="absolute inset-0"></div>
</div>
