<script lang="ts">
  // A device's stack, drawn as the chip is built (the cladding on top, the substrate below): the
  // layers with their thickness, material and background. A preset (soi_220, soi, nitride) shows its layers; "Edit
  // the layers" turns it into layers of the catalogue's materials, each with its model and, where
  // the model takes them, its temperature and composition, as the Materials page has them. The
  // core's thickness is the device's `thickness` parameter.
  import { Layers, Plus, Trash2 } from "@lucide/svelte";

  import { api, type LayerSpec, type MaterialEntry, type MaterialSpec, type StackSpec } from "../../lib/api";
  import { presetLayers } from "../../lib/device.svelte";
  import { plain } from "../../lib/deviceview";
  import type { Scene } from "../../lib/events";
  import { len, shown, stored, unitText } from "../../lib/units";
  import UnitChip from "../UnitChip.svelte";

  let { stack = $bindable(), core, scene, problem = "" }: { stack: StackSpec; core: number | null; scene: Scene | null; problem?: string } = $props();

  // the catalogue's isotropic materials, loaded once and shared
  const shared = globalThis as { __isotropic?: Promise<MaterialEntry[]> };
  let materials = $state<MaterialEntry[]>([]);
  (shared.__isotropic ??= api.materials().then((all) => all.filter((m) => m.index.some((i) => i.axes.length === 1)))).then((m) => (materials = m)).catch(() => {});

  const PRESETS = [
    { name: "soi_220", label: "220 nm SOI (soi_220)" },
    { name: "soi", label: "SOI, its oxide chosen (soi)" },
    { name: "nitride", label: "Silicon nitride on oxide (nitride)" },
  ];

  function choose(value: string) {
    if (value === "layers") stack = presetLayers(stack, core ?? 0.22);
    else stack = { type: "preset", name: value, bottom_oxide_um: stack.type === "preset" ? stack.bottom_oxide_um : 2 };
  }

  const index = (eps: number | undefined) => (eps && eps > 0 ? Math.sqrt(eps).toFixed(3) : "–");
  const sceneLayer = (name: string) => scene?.layers.find((l) => l.name === name);

  function setLayer(k: number, change: Partial<LayerSpec>) {
    if (stack.type !== "layers") return;
    const layers = stack.layers.map((l, i) => (i === k ? { ...l, ...change } : l));
    const renamedCore = change.name !== undefined && stack.layers[k].name === stack.core;
    stack = { ...stack, layers, core: renamedCore ? change.name! : stack.core };
  }

  function addLayer() {
    if (stack.type !== "layers") return;
    let n = stack.layers.length + 1;
    while (stack.layers.some((l) => l.name === `layer${n}`)) n++;
    stack = { ...stack, layers: [...stack.layers, { name: `layer${n}`, thickness_um: 0.5, material: { id: "sio2" }, background: { id: "sio2" } }] };
  }

  function removeLayer(k: number) {
    if (stack.type !== "layers" || stack.layers[k].name === stack.core) return;
    stack = { ...stack, layers: stack.layers.filter((_, i) => i !== k) };
  }

  const entry = (id: string) => materials.find((m) => m.id === id);
  const model = (m: MaterialSpec) => {
    const e = entry(m.id);
    return e?.index.find((i) => i.id === m.model) ?? e?.index.find((i) => i.default) ?? e?.index[0];
  };
</script>

{#snippet pick(m: MaterialSpec, set: (m: MaterialSpec) => void, what: string)}
  {@const md = model(m)}
  {@const isotropic = entry(m.id)?.index.filter((i) => i.axes.length === 1) ?? []}
  <div class="flex flex-wrap items-center gap-1.5">
    <select class="select select-xs w-36" value={m.id} aria-label={what} onchange={(e) => set({ id: e.currentTarget.value })}>
      {#if !entry(m.id)}<option value={m.id}>{m.id}</option>{/if}
      {#each materials as e (e.id)}<option value={e.id}>{e.name}</option>{/each}
    </select>
    {#if isotropic.length > 1}
      <select class="select select-xs w-32" value={md?.id} aria-label="{what}'s model" onchange={(e) => set({ ...m, model: e.currentTarget.value, temperature: null, composition: null })}>
        {#each isotropic as i (i.id)}<option value={i.id}>{i.name}</option>{/each}
      </select>
    {:else if md}
      <span class="text-[11px] faint" title={md.accuracy}>{md.name}</span>
    {/if}
    {#if md?.temperature}
      {@const t = md.temperature}
      <label class="input input-xs w-24" title="{t.name}, {t.min} to {t.max} {t.unit}">
        <input type="number" class="num" min={t.min} max={t.max} step="any" value={m.temperature ?? t.default} onchange={(e) => set({ ...m, temperature: Number(e.currentTarget.value) })} aria-label="{what}'s temperature" />
        <span class="faint">{t.unit}</span>
      </label>
    {/if}
    {#if md?.composition}
      {@const c = md.composition}
      <label class="input input-xs w-24" title="{c.name}, {c.min} to {c.max}">
        <span class="faint">{c.symbol}</span>
        <input type="number" class="num" min={c.min} max={c.max} step="any" value={m.composition ?? c.default} onchange={(e) => set({ ...m, composition: Number(e.currentTarget.value) })} aria-label="{what}'s composition" />
      </label>
    {/if}
  </div>
{/snippet}

<div class="flex flex-wrap items-center gap-2">
  <select class="select select-sm w-64" value={stack.type === "preset" ? stack.name : "layers"} aria-label="The stack" onchange={(e) => choose(e.currentTarget.value)}>
    {#each PRESETS as p (p.name)}<option value={p.name}>{p.label}</option>{/each}
    <option value="layers">Layers of my own</option>
  </select>
  {#if stack.type === "preset"}
    {#if stack.name !== "soi_220"}
      <label class="input input-sm w-40" title="The oxide under the core">
        <span class="text-xs faint">oxide</span>
        <input
          type="number"
          class="num"
          min="0"
          step="any"
          value={shown(stack.bottom_oxide_um)}
          onchange={(e) => stack.type === "preset" && (stack = { ...stack, bottom_oxide_um: stored(Number(e.currentTarget.value)) })}
          aria-label="The oxide under the core"
        />
        <UnitChip />
      </label>
    {/if}
    <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => choose("layers")} title="Turn the preset into layers of the catalogue's materials, to change them"><Layers size={14} /> Edit the layers</button>
  {/if}
</div>

{#if problem}<p role="alert" class="mt-2 text-sm text-error selectable">{plain(problem)}</p>{/if}

<div class="mt-3 overflow-x-auto">
  <table class="table table-xs">
    <thead><tr><th>Layer</th><th>Thickness ({unitText()})</th><th>Material</th><th>Background</th><th class="text-right" title="The refractive index at the design's wavelength">n</th>{#if stack.type === "layers"}<th></th>{/if}</tr></thead>
    <tbody>
      <tr>
        <td class="faint">cladding</td>
        <td class="faint">above</td>
        <td colspan="2">
          {#if stack.type === "layers"}{@render pick(stack.cladding, (m) => stack.type === "layers" && (stack = { ...stack, cladding: m }), "the cladding")}{:else}{scene?.cladding.material ?? ""}{/if}
        </td>
        <td class="text-right num">{index(scene?.cladding.eps)}</td>
        {#if stack.type === "layers"}<td></td>{/if}
      </tr>
      {#if stack.type === "layers"}
        {@const s = stack}
        {#each [...s.layers.keys()].reverse() as k (k)}
          {@const l = s.layers[k]}
          {@const isCore = l.name === s.core}
          <tr class={isCore ? "bg-primary/6" : ""}>
            <td>
              <div class="flex items-center gap-1.5">
                <input type="radio" class="radio radio-xs radio-primary" checked={isCore} title="The guides are drawn on this layer" aria-label="{l.name} is the core" onchange={() => stack.type === "layers" && (stack = { ...stack, core: l.name })} />
                <input class="input input-xs w-20" value={l.name} aria-label="The layer's name" onchange={(e) => setLayer(k, { name: e.currentTarget.value.trim() || l.name })} />
              </div>
            </td>
            <td>
              {#if isCore}
                <span class="text-xs muted num" title="The core's thickness is the device's thickness parameter">{core !== null ? len(core) : "–"} <span class="faint">(thickness)</span></span>
              {:else}
                <input type="number" class="input input-xs w-24 num" min="0" step="any" value={shown(l.thickness_um)} aria-label="{l.name}'s thickness" onchange={(e) => setLayer(k, { thickness_um: stored(Number(e.currentTarget.value)) })} />
              {/if}
            </td>
            <td>{@render pick(l.material, (m) => setLayer(k, { material: m }), `${l.name}'s material`)}</td>
            <td>{@render pick(l.background, (m) => setLayer(k, { background: m }), `${l.name}'s background`)}</td>
            <td class="text-right num" title="drawn {index(sceneLayer(l.name)?.material.eps)}, elsewhere {index(sceneLayer(l.name)?.background.eps)}">{index(sceneLayer(l.name)?.material.eps)}</td>
            <td><button class="btn btn-ghost btn-xs btn-square" disabled={isCore} title={isCore ? "The core stays" : "Remove the layer"} aria-label="Remove {l.name}" onclick={() => removeLayer(k)}><Trash2 size={12} /></button></td>
          </tr>
        {/each}
      {:else}
        {#each [...(scene?.layers ?? [])].reverse() as l (l.name)}
          <tr>
            <td class="font-medium">{l.name}</td>
            <td class="num">{len(l.z_um[1] - l.z_um[0])}</td>
            <td>{l.material.material}</td>
            <td>{l.background.material}</td>
            <td class="text-right num">{index(l.material.eps)}</td>
          </tr>
        {/each}
      {/if}
      <tr>
        <td class="faint">substrate</td>
        <td class="faint">below</td>
        <td colspan="2">
          {#if stack.type === "layers"}{@render pick(stack.substrate, (m) => stack.type === "layers" && (stack = { ...stack, substrate: m }), "the substrate")}{:else}{scene?.substrate.material ?? ""}{/if}
        </td>
        <td class="text-right num">{index(scene?.substrate.eps)}</td>
        {#if stack.type === "layers"}<td></td>{/if}
      </tr>
    </tbody>
  </table>
</div>
{#if stack.type === "layers"}
  <button class="btn btn-ghost btn-xs mt-1 gap-1" onclick={addLayer}><Plus size={12} /> Add a layer on top</button>
{/if}
<p class="mt-2 text-xs faint">Top to bottom as the chip is built; each index at the design's wavelength, from the material's model in the catalogue.</p>
