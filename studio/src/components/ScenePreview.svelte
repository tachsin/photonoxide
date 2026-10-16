<script lang="ts">
  // A job's structure in 3D, turning slowly (and by a drag, when turnable): what its run will
  // draw, before it runs. A modes
  // job's is shown in front of its cut too, with the cut drawn where the cross-section is taken.
  import { Box } from "@lucide/svelte";

  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import type { Scene, ScenePort } from "../lib/events";
  import { Preview3D } from "../lib/view3d";

  let { text, height = 150, turnable = false }: { text: string; height?: number; turnable?: boolean } = $props();

  let host: HTMLDivElement;
  let failed = $state("");
  let loading = $state(true);
  /** Whether a scene is on show: while the next one loads, the last one stays. */
  let shown = $state(false);
  let view: Preview3D | null = null;

  /** Scenes already built, with their components' named ports, by job text: the cards share them across pages. */
  const cache = (globalThis as { __scenes?: Map<string, { scene: Scene; ports: ScenePort[] }> }).__scenes ??= new Map();

  /** A job's scene and its ports (none when they can't be found: the scene still shows). */
  const load = (t: string) =>
    Promise.all([api.previewScene(t), /\[\[task\.component\]\]|\[device\]/.test(t) ? api.previewPorts(t).catch(() => []) : Promise.resolve([])]).then(
      ([scene, ports]) => ({ scene, ports }),
    );

  /** A modes job's scene ends at its cut (photonoxide::job::preview): normal to x when the job says its modes travel along x, else to y. */
  const cutOf = (t: string, scene: Scene): { cut?: number; normal?: "x" | "y" } => {
    if (!/^\s*kind\s*=\s*"modes"/m.test(t)) return {};
    return /^\s*propagation\s*=\s*"x"/m.test(t) ? { cut: scene.x_um[1], normal: "x" } : { cut: scene.y_um[1], normal: "y" };
  };

  $effect(() => {
    const t = text;
    let gone = false;
    loading = true;
    failed = "";
    const ready = cache.has(t) ? Promise.resolve(cache.get(t)!) : load(t);
    ready
      .then((built) => {
        cache.set(t, built);
        if (gone) return;
        view ??= new Preview3D(host, app.dark, turnable);
        view.setScene(built.scene, { ...cutOf(t, built.scene), ports: built.ports });
        shown = true;
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
  $effect(() => () => {
    view?.destroy();
    view = null;
  });
</script>

<div class="relative w-full" style="height: {height}px">
  <div bind:this={host} class="absolute inset-0"></div>
  {#if loading && !shown}<div class="skeleton absolute inset-0"></div>{/if}
  {#if failed}
    <div class="absolute inset-0 grid place-items-center bg-base-100/70 p-3 text-center text-xs faint" title={failed}>
      <span class="flex flex-col items-center gap-1"><Box size={18} /> no preview: the job isn't valid yet</span>
    </div>
  {/if}
</div>
