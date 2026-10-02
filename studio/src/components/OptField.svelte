<script lang="ts">
  // An optional number in the builder's form: empty means the job's default.
  import { Info } from "@lucide/svelte";

  let {
    label,
    value = $bindable(),
    unit = "",
    hint = "",
    step = 0.01,
    min,
    integer = false,
    placeholder = "",
  }: {
    label: string;
    value: number | null;
    unit?: string;
    hint?: string;
    step?: number;
    min?: number;
    integer?: boolean;
    placeholder?: string;
  } = $props();

  function input(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    if (raw === "") {
      value = null;
      return;
    }
    const v = Number(raw);
    if (Number.isFinite(v)) value = integer ? Math.round(v) : v;
  }
</script>

<label class="flex flex-col gap-1">
  <span class="flex items-center gap-1 text-xs font-medium text-base-content/70">
    {label}
    {#if hint}
      <span class="tooltip tooltip-right z-30 font-normal" data-tip={hint}><Info size={12} class="faint" /></span>
    {/if}
  </span>
  <span class="input input-sm w-full">
    <input type="number" class="num" {step} {min} {placeholder} value={value ?? ""} oninput={input} />
    {#if unit}<span class="text-xs faint">{unit}</span>{/if}
  </span>
</label>
