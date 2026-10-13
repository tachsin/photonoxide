<script lang="ts">
  // A device's geometry as a form: each of its parameters a field that takes a number in its unit
  // or an expression of named parameters (`gap = "w/2.5"`, crate photonoxide::expr), with its
  // slider and range, what it uses and what uses it, and its error in place, pointing at the
  // column. Named parameters can be added for the others to use.
  import { ArrowRight, Plus, RotateCcw, X } from "@lucide/svelte";

  import type { DeviceField, DeviceInfo } from "../../lib/api";
  import { literal, literalAt, plain, showExpression, takeExpression, valueText } from "../../lib/deviceview";
  import { label } from "../../lib/plot";
  import { isMicrometres, showIn } from "../../lib/units";
  import UnitChip from "../UnitChip.svelte";

  let {
    info,
    fields,
    parameters = $bindable(),
  }: { info: DeviceInfo; fields: DeviceField[]; parameters: [string, string][] } = $props();

  /** What a field's input holds while it is being typed in, so a change of unit doesn't move the caret. */
  let editing = $state<Record<string, string>>({});

  const given = (name: string) => parameters.find(([n]) => n === name)?.[1];
  /** The fields to show: the evaluated ones, and any named parameter added since. */
  const rows = $derived.by((): DeviceField[] => {
    const out = fields.map((f) => ({ ...f, expression: given(f.name) ?? f.expression }));
    for (const [name, expression] of parameters) {
      if (!out.some((f) => f.name === name)) out.push({ name, expression, value: null, unit: "", device: false, uses: [], used_by: [], error: null });
    }
    return out;
  });
  const deviceRows = $derived(rows.filter((f) => f.device));
  const named = $derived(rows.filter((f) => !f.device));

  function set(name: string, expression: string) {
    const k = parameters.findIndex(([n]) => n === name);
    parameters = k >= 0 ? parameters.map((p, i) => (i === k ? [name, expression] : p)) : [...parameters, [name, expression]];
  }

  function rename(from: string, to: string) {
    to = to.trim();
    if (!to || to === from || parameters.some(([n]) => n === to)) return;
    parameters = parameters.map(([n, e]) => (n === from ? [to, e] : [n, e]));
  }

  function remove(name: string) {
    parameters = parameters.filter(([n]) => n !== name);
  }

  function add() {
    let k = 1;
    while (rows.some((f) => f.name === `p${k}`)) k++;
    parameters = [...parameters, [`p${k}`, "0"]];
  }

  const parameter = (name: string) => info.parameters.find((p) => p.name === name);
  /** The top of a parameter's slider: its range when that is modest, else a span around its default. */
  function sliderMax(p: DeviceInfo["parameters"][number]): number {
    if (p.max - p.min <= 100) return p.max;
    return Math.min(p.max, Math.max(p.min + 1, 4 * Math.abs(p.default) || 1));
  }
  const step = (p: DeviceInfo["parameters"][number]) => {
    const s = (sliderMax(p) - p.min) / 1000;
    return Number(s.toPrecision(1));
  };
</script>

{#snippet error(f: DeviceField)}
  {#if f.error}
    <div role="alert" class="mt-1 text-xs text-error">
      {#if f.error.column}
        <pre class="font-mono leading-tight whitespace-pre">{f.expression}
{" ".repeat(Math.max(0, f.error.column - 1))}^</pre>
      {/if}
      <span class="selectable">{plain(f.error.message)}</span>
    </div>
  {/if}
{/snippet}

{#snippet links(f: DeviceField)}
  {#if f.uses.length || f.used_by.length}
    <div class="mt-1 flex flex-wrap items-center gap-1 text-[11px] faint">
      {#if f.uses.length}<span>uses</span>{#each f.uses as u (u)}<span class="badge badge-xs badge-ghost num">{u}</span>{/each}{/if}
      {#if f.used_by.length}<span class="inline-flex items-center gap-0.5 {f.uses.length ? 'ml-2' : ''}">used by <ArrowRight size={10} /></span>{#each f.used_by as u (u)}<span class="badge badge-xs badge-primary badge-soft num">{u}</span>{/each}{/if}
    </div>
  {/if}
{/snippet}

<div class="flex items-center gap-2">
  <p class="mr-auto text-xs faint">A number in its unit, or an expression: <code>450 nm</code>, <code>w/2.5</code>, <code>2*radius</code>.</p>
  <div class="tooltip tooltip-left" data-tip="Every parameter back to its default">
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Back to the defaults" onclick={() => (parameters = [])}><RotateCcw size={13} /></button>
  </div>
</div>

<div class="mt-3 space-y-4">
  {#each deviceRows as f (f.name)}
    {@const p = parameter(f.name)}
    {@const lit = literal(f)}
    <div>
      <div class="flex items-baseline gap-2 text-sm">
        <label for="field-{f.name}" class="font-medium num">{f.name}</label>
        {#if isMicrometres(f.unit)}<span class="text-xs faint"><UnitChip /></span>{:else if f.unit}<span class="text-xs faint">{f.unit}</span>{/if}
        <span class="flex-1"></span>
        {#if p}<span class="text-xs faint num" title="Its range, and its default">{label(showIn(p.unit, p.min))} – {label(showIn(p.unit, p.max))} · default {label(showIn(p.unit, p.default))}</span>{/if}
      </div>
      <div class="mt-1.5 flex items-center gap-3">
        {#if p}
          <input
            class="range range-xs range-primary flex-1"
            type="range"
            min={p.min}
            max={sliderMax(p)}
            step={step(p)}
            value={lit ? lit.value : (f.value ?? p.default)}
            disabled={!lit}
            title={lit ? "" : "Set by its expression"}
            oninput={(e) => set(f.name, literalAt(Number(e.currentTarget.value), lit?.unit ?? ""))}
            aria-label={f.name}
          />
        {/if}
        <input
          id="field-{f.name}"
          class="input input-xs w-32 num {f.error ? 'input-error' : ''}"
          value={editing[f.name] ?? showExpression(f)}
          spellcheck="false"
          autocomplete="off"
          oninput={(e) => {
            editing[f.name] = e.currentTarget.value;
            set(f.name, takeExpression(f, e.currentTarget.value));
          }}
          onblur={() => delete editing[f.name]}
          aria-label="{f.name}: a number or an expression"
        />
      </div>
      {#if !lit && f.value !== null && !f.error}<div class="mt-1 text-xs muted num">= {valueText(f.value, f.unit)}</div>{/if}
      {@render error(f)}
      {@render links(f)}
    </div>
  {/each}
</div>

<div class="mt-6 flex items-center gap-2">
  <h4 class="text-sm font-medium">Named parameters</h4>
  <span class="text-xs faint">for the others to use</span>
  <span class="flex-1"></span>
  <button class="btn btn-ghost btn-xs gap-1" onclick={add}><Plus size={12} /> Add</button>
</div>
<div class="mt-2 space-y-3">
  {#each named as f (f.name)}
    <div>
      <div class="flex items-center gap-2">
        <input class="input input-xs w-20 shrink-0 num" value={f.name} aria-label="The parameter's name" spellcheck="false" onchange={(e) => rename(f.name, e.currentTarget.value)} />
        <span class="faint">=</span>
        <input
          class="input input-xs min-w-0 flex-1 num {f.error ? 'input-error' : ''}"
          value={editing[f.name] ?? f.expression}
          spellcheck="false"
          autocomplete="off"
          oninput={(e) => {
            editing[f.name] = e.currentTarget.value;
            set(f.name, e.currentTarget.value);
          }}
          onblur={() => delete editing[f.name]}
          aria-label="{f.name}'s expression"
        />
        <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove {f.name}" title="Remove it" onclick={() => remove(f.name)}><X size={12} /></button>
      </div>
      {#if !f.error && f.value !== null}<div class="mt-1 text-xs muted num">= {valueText(f.value, f.unit)}</div>{/if}
      {@render error(f)}
      {@render links(f)}
    </div>
  {:else}
    <p class="text-xs faint">None yet: add one, e.g. <code>w = 450 nm</code>, and set <code>width</code> to <code>w</code>.</p>
  {/each}
</div>
