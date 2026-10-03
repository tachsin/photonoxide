<script lang="ts">
  // Importing a Touchstone file: the file's time convention, which the format doesn't record,
  // chosen before it is read as a measured component.
  import { FileUp } from "@lucide/svelte";

  import { finishImport, library } from "../lib/chip.svelte";
  import MathText from "./MathText.svelte";

  const name = $derived(library.importing?.path.split(/[\/]/).pop() ?? "");
</script>

<dialog class="modal" class:modal-open={!!library.importing}>
  {#if library.importing}
    {@const job = library.importing}
    <div class="modal-box max-w-lg">
      <h3 class="flex items-center gap-2 text-lg font-semibold"><FileUp size={18} class="text-primary" /> Import {name}</h3>
      <p class="mt-2 text-sm muted">
        Touchstone files don't say which time convention their complex values are in, and the two are each other's complex conjugates. Which one was this file written in?
      </p>
      <div class="mt-4 space-y-2">
        <label class="flex cursor-pointer items-start gap-3 rounded-lg border border-base-content/10 p-3 {job.convention === 'engineering' ? 'border-primary/50 bg-primary/6' : ''}">
          <input type="radio" class="radio radio-sm radio-primary mt-0.5" value="engineering" bind:group={job.convention} />
          <span class="text-sm">
            <span class="font-medium"><MathText text={"$e^{+j\omega t}$"} />, engineering</span>
            <span class="block text-xs faint">Network analysers and most RF and photonics design tools.</span>
          </span>
        </label>
        <label class="flex cursor-pointer items-start gap-3 rounded-lg border border-base-content/10 p-3 {job.convention === 'physics' ? 'border-primary/50 bg-primary/6' : ''}">
          <input type="radio" class="radio radio-sm radio-primary mt-0.5" value="physics" bind:group={job.convention} />
          <span class="text-sm">
            <span class="font-medium"><MathText text={"$e^{-i\omega t}$"} />, physics</span>
            <span class="block text-xs faint">photonoxide's own: what its Touchstone export writes.</span>
          </span>
        </label>
      </div>
      <p class="mt-3 text-xs faint">Its ports become o1, o2, … in the file's order, and its S-parameters are interpolated between the file's wavelengths, never beyond them.</p>
      <div class="modal-action">
        <button class="btn btn-ghost btn-sm" onclick={() => finishImport(false)}>Cancel</button>
        <button class="btn btn-primary btn-sm" disabled={job.busy} onclick={() => finishImport(true)}>
          {#if job.busy}<span class="loading loading-spinner loading-xs"></span>{/if} Import
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button onclick={() => finishImport(false)}>close</button></form>
  {/if}
</dialog>
