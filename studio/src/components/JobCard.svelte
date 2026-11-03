<script lang="ts">
  // A template as a row on Home: its name, its kind, what it is in a line, and Run (live, in the
  // run view) or Open (on its device's page, the Chip or the Custom device, to change it); and, for
  // one that draws a design, Fabricate (its flow from the bare wafer, on the Fabrication page).
  import { Factory, Play, SquarePen } from "@lucide/svelte";

  import { KINDS } from "../lib/api";

  let {
    name,
    kind,
    about,
    onrun,
    onopen,
    onfabricate,
  }: {
    name: string;
    kind: string;
    about: string;
    onrun: () => void;
    onopen: () => void;
    /** Shows how its design is made, if it draws one. */
    onfabricate?: () => void;
  } = $props();

  const tone: Record<string, string> = { modes: "badge-primary", fdfd: "badge-secondary", fdtd: "badge-info", structure: "badge-accent", static: "badge-warning", device: "badge-success" };
</script>

<div class="group flex items-center gap-3 px-4 py-2.5 transition-colors hover:bg-base-content/3">
  <span class="badge badge-soft badge-sm w-[4.5rem] shrink-0 justify-center {tone[kind] ?? ''}" title={KINDS[kind]?.about}>{KINDS[kind]?.label ?? kind}</span>
  <span class="min-w-0 flex-1">
    <span class="block truncate text-sm font-medium">{name}</span>
    <span class="block truncate text-xs faint" title={about || KINDS[kind]?.about}>{about || KINDS[kind]?.about}</span>
  </span>
  {#if onfabricate}
    <button class="btn btn-ghost btn-xs gap-1.5" onclick={onfabricate} title="How its design is made, from the bare wafer: the steps, each with its reason and sources, played on the Fabrication page"><Factory size={14} /> Fabricate</button>
  {/if}
  <button class="btn btn-ghost btn-xs gap-1.5" onclick={onopen} title="Open it, to change it and save your own"><SquarePen size={14} /> Open</button>
  <button class="btn btn-primary btn-xs gap-1.5" onclick={onrun} title="Run it now and watch it live"><Play size={14} /> Run</button>
</div>
