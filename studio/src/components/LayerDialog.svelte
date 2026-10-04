<script lang="ts">
  // A medium of the run's scene in detail: what it is, what fills it around its shapes, what is
  // under and over it, and how the viewer draws it (visible, colour, opacity: viewer-only).
  import { Eye, Layers, RotateCcw } from "@lucide/svelte";

  import { run } from "../lib/app.svelte";
  import { defaultLook, effectiveLook, explain, index, lookTarget, neighbours, outside, rows } from "../lib/layers";
  import { len } from "../lib/units";
  import UnitChip from "./UnitChip.svelte";

  let { name, onclose }: { name: string | null; onclose: () => void } = $props();

  const all = $derived(run.scene ? rows(run.scene) : []);
  const k = $derived(all.findIndex((r) => r.name === name));
  const row = $derived(k >= 0 ? all[k] : null);
  const near = $derived(k >= 0 ? neighbours(all, k) : { below: null, above: null });
  const away = $derived(run.scene && row ? outside(run.scene, row) : null);
  const base = $derived(row ? defaultLook(row) : null);
  const look = $derived(row ? effectiveLook(row, run.looks[row.name]) : null);
  const target = $derived(row ? lookTarget(row) : null);
  const title = $derived(row ? (row.kind === "layer" ? `Layer ${row.name}` : row.kind === "substrate" ? "Substrate" : "Cladding") : "");

  /** A few colours that read on both themes: the media's own, then some to tell layers apart. */
  const SWATCHES = ["#4a6fa8", "#33968c", "#9fc0de", "#d9604f", "#e0a43a", "#8e6cc9", "#5aa65e", "#9aa3ad"];

  function setLook(change: { colour?: string; opacity?: number }) {
    if (!row) return;
    run.looks[row.name] = { ...run.looks[row.name], ...change };
  }

  function resetLook() {
    if (!row) return;
    delete run.looks[row.name];
  }

  function toggle() {
    if (!row) return;
    run.hidden = run.hidden.includes(row.name) ? run.hidden.filter((h) => h !== row.name) : [...run.hidden, row.name];
  }

  const span = (z: [number, number]) =>
    z[0] === -Infinity ? `below ${len(z[1])}` : z[1] === Infinity ? `above ${len(z[0])}` : `${len(z[0])} → ${len(z[1])}`;
</script>

<dialog class="modal" class:modal-open={!!row}>
  {#if row && run.scene}
    <div class="modal-box max-w-lg p-0">
      <div class="glow rounded-t-box border-b border-base-content/8 px-6 pt-5 pb-4">
        <div class="flex items-center gap-3">
          <div class="grid size-10 place-items-center rounded-xl bg-primary/15 text-primary"><Layers size={20} /></div>
          <div class="min-w-0">
            <h3 class="text-lg font-semibold">{title}</h3>
            <p class="text-sm muted">
              {row.material.material}{row.background && row.background.material !== row.material.material ? ` in ${row.background.material}` : ""} ·
              <span class="num">z {span(row.z)} <UnitChip /></span>
            </p>
          </div>
        </div>
      </div>

      <div class="max-h-[62vh] space-y-5 overflow-y-auto px-6 py-4">
        <p class="text-sm leading-relaxed selectable">{explain(all, k)}</p>

        <section>
          <h4 class="panel-title mb-2">What it is</h4>
          <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
            <dt class="faint">{row.kind === "layer" ? "shapes are" : "material"}</dt>
            <dd class="num">{row.material.material} · ε {row.material.eps.toFixed(3)} · n {index(row.material.eps)}</dd>
            {#if row.background}
              <dt class="faint">around them</dt>
              <dd class="num">{row.background.material} · ε {row.background.eps.toFixed(3)} · n {index(row.background.eps)}</dd>
            {/if}
            <dt class="faint">thickness</dt>
            <dd class="num">{#if Number.isFinite(row.z[1] - row.z[0])}{len(row.z[1] - row.z[0])} <UnitChip />{:else}without end (modelled as infinitely thick){/if}</dd>
            <dt class="faint">z</dt>
            <dd class="num">{span(row.z)} <UnitChip /></dd>
            {#if row.kind === "layer"}
              <dt class="faint">shapes</dt>
              <dd>{row.shapes === 0 ? "none drawn" : `${row.shapes} drawn on it`}</dd>
            {/if}
            <dt class="faint">below</dt>
            <dd>{near.below ? near.below.name : "nothing: the stack's bottom"}</dd>
            <dt class="faint">above</dt>
            <dd>{near.above ? near.above.name : "nothing: the stack's top"}</dd>
          </dl>
          <p class="mt-2 text-[11px] faint">ε is the real part of the permittivity at the scene's wavelength, {len(run.scene.wavelength_um)} <UnitChip />, and n = √ε.</p>
        </section>

        <section>
          <h4 class="panel-title mb-2 flex items-center gap-1.5"><Eye size={13} /> In the 3D view</h4>
          {#if away}
            <p class="mb-2 rounded-lg border border-base-content/10 bg-base-content/4 px-3 py-2 text-xs muted">
              It lies wholly {away} the run's window (z {len(run.scene.z_um[0])} → {len(run.scene.z_um[1])} <UnitChip />), so nothing of it is drawn.
            </p>
          {/if}
          <label class="flex items-center gap-2.5 text-sm" class:opacity-50={!!away}>
            <input type="checkbox" class="checkbox checkbox-sm" checked={!run.hidden.includes(row.name)} disabled={!!away} onchange={toggle} />
            visible
          </label>

          <div class="mt-3 flex flex-wrap items-center gap-1.5">
            <span class="w-16 text-xs faint">colour</span>
            {#each SWATCHES as c (c)}
              <button
                class="size-6 rounded-md border transition-transform hover:scale-110 {look?.colour === c ? 'border-base-content ring-2 ring-primary/50' : 'border-base-content/20'}"
                style="background:{c}"
                aria-label="Colour {c}"
                title={c}
                onclick={() => setLook({ colour: c })}
              ></button>
            {/each}
            <input
              type="color"
              class="h-6 w-8 cursor-pointer rounded-md border border-base-content/20 bg-transparent"
              value={look?.colour ?? "#9aa3ad"}
              title="Any colour"
              oninput={(e) => setLook({ colour: (e.currentTarget as HTMLInputElement).value })}
            />
          </div>
          <label class="mt-3 flex items-center gap-2">
            <span class="w-16 text-xs faint">opacity</span>
            <input
              type="range"
              class="range range-xs range-primary flex-1"
              min="0"
              max="1"
              step="0.01"
              value={look?.opacity ?? base?.opacity ?? 0.13}
              oninput={(e) => setLook({ opacity: Number((e.currentTarget as HTMLInputElement).value) })}
            />
            <span class="w-10 text-right text-xs num">{Math.round(100 * (look?.opacity ?? base?.opacity ?? 0.13))}%</span>
          </label>
          <p class="mt-2 text-[11px] faint">
            {#if row.kind === "layer" && row.shapes > 0}
              Colour and opacity apply to the layer's {target?.material} shapes; the {row.background?.material} around them keeps its look.
            {:else if base}
              Colour and opacity apply to the {target?.material} {row.kind === "layer" ? "layer" : row.kind}.
            {:else}
              {target?.material} isn't drawn by default; pick a colour to draw it.
            {/if}
            Only the viewer changes: the run and its record stay as they are.
          </p>
        </section>
      </div>

      <div class="flex items-center justify-between gap-2 border-t border-base-content/8 px-6 py-3">
        <button class="btn btn-ghost btn-sm gap-1.5" disabled={!run.looks[row.name]} onclick={resetLook}><RotateCcw size={14} /> Default look</button>
        <button class="btn btn-sm" onclick={onclose}>Close</button>
      </div>
    </div>
  {/if}
  <form method="dialog" class="modal-backdrop"><button onclick={onclose}>close</button></form>
</dialog>
