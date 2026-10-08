<script lang="ts">
  // A lesson's papers by year: where the idea began, the steps that changed it, the surveys, and
  // today's work, each opened by its DOI.
  import { ExternalLink } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import type { Paper, Role } from "../lib/academy.svelte";

  let { papers }: { papers: Paper[] } = $props();

  const ROLES: { role: Role; label: string; about: string; dot: string; badge: string }[] = [
    { role: "origin", label: "origin", about: "Where the idea began", dot: "bg-accent", badge: "badge-accent" },
    { role: "milestone", label: "milestone", about: "A step that changed what was possible", dot: "bg-primary", badge: "badge-primary" },
    { role: "review", label: "review", about: "A survey of the field", dot: "bg-secondary", badge: "badge-secondary" },
    { role: "current", label: "current", about: "Today's state of the art", dot: "bg-success", badge: "badge-success" },
  ];
  const look = (r: Role) => ROLES.find((x) => x.role === r) ?? ROLES[1];

  let only = $state<Role | null>(null);
  const shown = $derived([...papers].sort((a, b) => a.year - b.year).filter((p) => !only || p.role === only));
  const present = $derived(ROLES.filter((r) => papers.some((p) => p.role === r.role)));
</script>

<div class="my-6">
  <div class="mb-3 flex flex-wrap items-center gap-1.5">
    <button class="btn btn-xs {only === null ? 'btn-primary' : 'btn-ghost'}" onclick={() => (only = null)}>all {papers.length}</button>
    {#each present as r (r.role)}
      <button class="btn gap-1.5 btn-xs {only === r.role ? 'btn-primary' : 'btn-ghost'}" title={r.about} onclick={() => (only = only === r.role ? null : r.role)}>
        <span class="size-2 rounded-full {r.dot}"></span>{r.label}
      </button>
    {/each}
  </div>
  <ul class="timeline timeline-vertical timeline-compact">
    {#each shown as p, k (p.doi)}
      {@const r = look(p.role)}
      <!-- the dot level with the year (timeline-snap-icon would undo timeline-compact) -->
      <li style="--timeline-row-start: 0.45rem">
        {#if k > 0}<hr class="bg-base-content/12" />{/if}
        <div class="timeline-middle"><span class="block size-3 rounded-full ring-4 ring-base-100 {r.dot}"></span></div>
        <div class="timeline-end mb-5 ml-1 min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <span class="text-sm font-semibold num">{p.year}</span>
            <span class="badge badge-soft badge-xs {r.badge}" title={r.about}>{r.label}</span>
          </div>
          <p class="mt-0.5 text-sm leading-snug font-medium">{p.title}</p>
          <p class="mt-0.5 text-xs leading-snug faint">{p.cite}</p>
          <p class="mt-1 text-[13px] leading-relaxed muted">{p.note}</p>
          <button class="btn mt-1 -ml-2 gap-1 text-primary btn-ghost btn-xs" onclick={() => openUrl(`https://doi.org/${p.doi}`)} title="Open the paper at its publisher">
            <ExternalLink size={11} /> doi:{p.doi}
          </button>
        </div>
        {#if k < shown.length - 1}<hr class="bg-base-content/12" />{/if}
      </li>
    {/each}
  </ul>
</div>
