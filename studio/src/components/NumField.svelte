<script lang="ts">
  // A number in the builder's form: a label with its hint, the value, its unit.
  import { Info } from "@lucide/svelte";

  import { shown, stored, type LengthUnit } from "../lib/units";
  import UnitChip from "./UnitChip.svelte";

  let {
    label,
    value = $bindable(),
    unit = "",
    hint = "",
    step = 0.01,
    min,
    integer = false,
    placeholder = "",
    length,
  }: {
    label: string;
    value: number;
    unit?: string;
    hint?: string;
    step?: number;
    min?: number;
    integer?: boolean;
    placeholder?: string;
    /** A length kept in this unit: the field shows and takes it in the unit chosen app-wide, with a chip to change that. */
    length?: LengthUnit;
  } = $props();

  /** A number as the field shows it: a length in the unit chosen. */
  const view = <T extends number | null | undefined>(v: T): T => (typeof v === "number" && length ? (shown(v, length) as T) : v);

  function input(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    if (raw === "") return;
    const v = Number(raw);
    if (Number.isFinite(v)) value = integer ? Math.round(v) : length ? stored(v, length) : v;
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
    <input type="number" class="num" step={view(step)} min={view(min)} {placeholder} value={view(value) ?? ""} oninput={input} />
    {#if length}<span class="text-xs faint"><UnitChip tip="bottom" /></span>{:else if unit}<span class="text-xs faint">{unit}</span>{/if}
  </span>
</label>
