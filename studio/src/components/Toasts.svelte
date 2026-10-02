<script lang="ts">
  // Short messages in the corner: what just happened, sometimes with an action.
  import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from "@lucide/svelte";
  import { fly } from "svelte/transition";

  import { closeToast, toasts } from "../lib/app.svelte";

  const icons = { info: Info, success: CircleCheck, warning: TriangleAlert, error: CircleAlert };
  const tone = { info: "text-info", success: "text-success", warning: "text-warning", error: "text-error" };
</script>

<div class="pointer-events-none fixed right-5 bottom-10 z-50 flex w-96 flex-col gap-2">
  {#each toasts as t (t.id)}
    {@const Icon = icons[t.kind]}
    <div
      class="pointer-events-auto flex items-start gap-3 rounded-xl border border-base-content/10 bg-base-100 p-3.5 shadow-xl shadow-black/20"
      transition:fly={{ x: 40, duration: 180 }}
    >
      <Icon size={18} class="mt-0.5 shrink-0 {tone[t.kind]}" />
      <div class="min-w-0 flex-1 text-sm">
        <p class="selectable break-words">{t.text}</p>
        {#if t.action}
          <button class="btn btn-link btn-xs mt-1 h-auto min-h-0 p-0 text-primary" onclick={() => { t.action!.run(); closeToast(t.id); }}>{t.action.label}</button>
        {/if}
      </div>
      <button class="btn btn-ghost btn-xs btn-square" aria-label="Close" onclick={() => closeToast(t.id)}><X size={14} /></button>
    </div>
  {/each}
</div>
