<script lang="ts">
  // A job as a card: its device in 3D, turning, what it is, and what can be done with it.
  import { Pencil, Play } from "@lucide/svelte";

  import { KINDS } from "../lib/api";
  import ScenePreview from "./ScenePreview.svelte";

  let {
    name,
    kind,
    about,
    text,
    onrun,
    onedit,
    badge,
  }: {
    name: string;
    kind: string;
    about: string;
    text: string;
    onrun: () => void;
    onedit: () => void;
    badge?: string;
  } = $props();

  const tone: Record<string, string> = { modes: "badge-primary", fdfd: "badge-secondary", fdtd: "badge-info", structure: "badge-accent" };
</script>

<article class="panel group flex flex-col overflow-hidden transition-all hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-xl hover:shadow-black/10">
  <div class="glow border-b border-base-content/8 bg-base-200/60" title="What its run will draw, before it runs">
    <ScenePreview {text} height={160} />
  </div>
  <div class="flex flex-1 flex-col gap-2 p-4">
    <div class="flex items-center gap-2">
      <h3 class="truncate font-semibold">{name}</h3>
      <span class="badge badge-soft badge-sm {tone[kind] ?? ''}" title={KINDS[kind]?.about}>{KINDS[kind]?.label ?? kind}</span>
      {#if badge}<span class="badge badge-ghost badge-sm">{badge}</span>{/if}
    </div>
    <p class="line-clamp-3 flex-1 text-[13px] leading-relaxed muted">{about || KINDS[kind]?.about}</p>
    <div class="mt-1 flex gap-2">
      <button class="btn btn-primary btn-sm flex-1 gap-1.5" onclick={onrun} title="Run it now and watch it live"><Play size={14} /> Run</button>
      <button class="btn btn-ghost btn-sm gap-1.5" onclick={onedit} title="Open it in the job builder, to change it"><Pencil size={14} /> Edit</button>
    </div>
  </div>
</article>
