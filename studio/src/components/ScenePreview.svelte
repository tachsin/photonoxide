<script lang="ts">
  // A job's structure in 3D, turning slowly (and by a drag, when turnable): what its run will
  // draw, before it runs. A modes job's is shown in front of its cut too, and a static job's
  // around it, with the cut drawn where the cross-section is taken. With `axes`, its axes in the
  // corner, named in `frame`; a cross-section's frame turns the camera to look along the guide.
  import { Box } from "@lucide/svelte";

  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import type { Scene, ScenePort } from "../lib/events";
  import { axisNames, type Frame, type Section } from "../lib/frames";
  import { Preview3D } from "../lib/view3d";

  let {
    text,
    height = 150,
    turnable = false,
    axes = false,
    frame = "chip",
  }: { text: string; height?: number; turnable?: boolean; axes?: boolean; frame?: Frame } = $props();

  let host: HTMLDivElement;
  let gizmo = $state<SVGSVGElement>();
  let failed = $state("");
  let loading = $state(true);
  /** Whether a scene is on show: while the next one loads, the last one stays. */
  let shown = $state(false);
  /** The cross-section the job is cut on, in the chip's frame (frames.rs); none for a job that isn't cut. */
  let section = $state<Section | null>(null);
  let view = $state.raw<Preview3D | null>(null);

  type Built = { scene: Scene; ports: ScenePort[]; section: Section | null };
  /** Scenes already built, with their components' named ports and their cuts, by job text: the cards share them across pages. */
  const cache = (globalThis as { __scenes?: Map<string, Built> }).__scenes ??= new Map();

  /** A job's scene, its ports and its cut (no ports or cut when they can't be found: the scene still shows). */
  const load = (t: string): Promise<Built> =>
    Promise.all([
      api.previewScene(t),
      /\[\[task\.component\]\]|\[device\]/.test(t) ? api.previewPorts(t).catch(() => []) : Promise.resolve([]),
      api.jobSection(t).catch(() => null),
    ]).then(([scene, ports, section]) => ({ scene, ports, section }));

  /**
   * Where the scene's cut is drawn: a modes job's scene ends at its cut (photonoxide::job::preview),
   * normal to the axis its modes travel along; a static job's is cut at x in the middle of it.
   */
  const cutOf = (scene: Scene, s: Section | null): { cut?: number; normal?: "x" | "y"; within?: boolean } => {
    if (!s) return {};
    if (s.kind === "static") return { cut: (scene.x_um[0] + scene.x_um[1]) / 2, normal: "x", within: true };
    return { cut: s.along === "x" ? scene.x_um[1] : scene.y_um[1], normal: s.along };
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
        view.setScene(built.scene, { ...cutOf(built.scene, built.section), ports: built.ports });
        section = built.section;
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

  $effect(() => {
    view?.setAxes(axes && gizmo ? gizmo : null, axisNames(frame, section));
    view?.lookAlong(frame === "section" && section ? section.along : null);
  });
  $effect(() => view?.setDark(app.dark));
  $effect(() => () => {
    view?.destroy();
    view = null;
  });
</script>

<div class="relative w-full" style="height: {height}px">
  <div bind:this={host} class="absolute inset-0"></div>
  {#if axes}
    <svg bind:this={gizmo} class="pointer-events-none absolute bottom-2 left-2 text-[11px] font-semibold" width="110" height="96" viewBox="-55 -48 110 96" role="img" aria-label="the axes"></svg>
  {/if}
  {#if loading && !shown}<div class="skeleton absolute inset-0"></div>{/if}
  {#if failed}
    <div class="absolute inset-0 grid place-items-center bg-base-100/70 p-3 text-center text-xs faint" title={failed}>
      <span class="flex flex-col items-center gap-1"><Box size={18} /> no preview: the job isn't valid yet</span>
    </div>
  {/if}
</div>
