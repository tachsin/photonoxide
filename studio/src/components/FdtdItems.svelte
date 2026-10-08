<script lang="ts">
  // An fdtd job's sources or monitors in the builder: each one's type and the fields that type
  // reads, as photonoxide::job's fdtd module reads them; placed in the top view, where a click
  // picks one.
  import { Plus, Trash2 } from "@lucide/svelte";

  import { layerSpan, newItem, type FdtdItem, type JobModel } from "../lib/job";
  import NumField from "./NumField.svelte";
  import OptField from "./OptField.svelte";
  import type { Selection } from "./GeometryPreview.svelte";

  let {
    model = $bindable(),
    which,
    selection = $bindable(),
  }: {
    model: JobModel;
    which: "source" | "monitor";
    selection: Selection;
  } = $props();

  const TYPES: Record<"source" | "monitor", [string, string, string][]> = {
    source: [
      ["mode", "guide mode", "A guide's fundamental mode of the job's polarization, launched one way from a plane"],
      ["dipole", "dipole", "A point current: an electric (E) or magnetic (H) dipole"],
      ["plane_wave", "plane wave", "A plane wave on a total-field/scattered-field box: the total field inside, the scattered field outside"],
      ["beam", "Gaussian beam", "A Gaussian beam launched one way from a plane"],
    ],
    monitor: [
      ["mode", "guide mode", "The power in a guide's fundamental mode each way through a plane"],
      ["flux", "flux plane", "The flux through a plane"],
      ["flux_box", "flux box", "The flux out of a box: what a scatterer inside scatters or absorbs"],
      ["field", "field", "|E|² of the transforms on the view's plane at chosen wavelengths"],
      ["resonance", "resonance", "A point's field after the pulse, by harmonic inversion: resonances' wavelengths and Q"],
    ],
  };

  const list = $derived(which === "source" ? model.source : model.monitor);
  const three = $derived(model.dimensions === 3);
  /** A box's extents: along x and y, and z in 3D. */
  const boxKeys = $derived<readonly ("x_um" | "y_um" | "z_um")[]>(three ? ["x_um", "y_um", "z_um"] : ["x_um", "y_um"]);
  const components = $derived(three ? ["Ex", "Ey", "Ez", "Hx", "Hy", "Hz"] : model.polarization === "te" ? ["Ex", "Ey", "Hz"] : ["Ez", "Hx", "Hy"]);

  function add(type: string) {
    list.push(newItem(model, type, which === "monitor"));
    selection = { kind: which, index: list.length - 1 };
  }

  function retype(k: number, type: string) {
    list[k] = { ...newItem(model, type, which === "monitor"), name: list[k].name };
  }

  /** The window across a plane normal to x or y: y_um or x_um. */
  const across = (it: FdtdItem): "x_um" | "y_um" => (it.normal === "y" ? "x_um" : "y_um");

  /** A number field's two-way binding to an item's nullable number, 0 standing in for none. */
  const at = (it: FdtdItem, key: "at_um" | "waist_um") => ({
    get: () => it[key] ?? 0,
    set: (v: number) => (it[key] = v),
  });

  /** A point's coordinate k, the point made when there was none. */
  function coordinate(it: FdtdItem, key: "position_um" | "center_um", k: number) {
    return {
      get: () => it[key]?.[k] ?? 0,
      set: (v: number) => {
        const n = key === "position_um" ? (three ? 3 : 2) : three ? 2 : 1;
        const p = Array.from({ length: n }, (_, i) => it[key]?.[i] ?? 0);
        p[k] = v;
        it[key] = p;
      },
    };
  }

  /** End k of an item's window or box along an axis, made from `fallback` when there was none. */
  function end(it: FdtdItem, key: "x_um" | "y_um" | "z_um", k: 0 | 1, fallback: [number, number]) {
    return {
      get: () => (it[key] ?? fallback)[k],
      set: (v: number) => {
        const p: [number, number] = [...(it[key] ?? fallback)];
        p[k] = v;
        it[key] = p;
      },
    };
  }

  /** The window's span along an axis: a box's default. */
  const span = (key: "x_um" | "y_um" | "z_um"): [number, number] => {
    const [a, b] = key === "x_um" ? model.x_um : key === "y_um" ? model.y_um : (model.z_um ?? [layerSpan(model)[0] - 1, layerSpan(model)[1] + 1]);
    return [Number((a + (b - a) / 3).toFixed(3)), Number((b - (b - a) / 3).toFixed(3))];
  };

  /** A field monitor's wavelengths as text, "1.5205, 1.55". */
  function wavelengths(it: FdtdItem) {
    return {
      get: () => (it.wavelengths_um ?? []).join(", "),
      set: (t: string) => {
        const v = t
          .split(/[,\s]+/)
          .map(Number)
          .filter((x) => Number.isFinite(x) && x > 0);
        it.wavelengths_um = v.length ? v : null;
      },
    };
  }

  const isSel = (k: number) => selection?.kind === which && selection.index === k;
</script>

{#each list as it, k (k)}
  {@const w = across(it)}
  {@const atB = at(it, "at_um")}
  {@const waistB = at(it, "waist_um")}
  <div class="rounded-xl border p-3 transition-colors {isSel(k) ? 'border-accent/60 bg-accent/5' : 'border-base-content/8'}">
    <div class="mb-2 flex items-center gap-2">
      <button class="text-sm font-medium" onclick={() => (selection = { kind: which, index: k })}>{which === "source" ? "Source" : "Monitor"} {k + 1}</button>
      <select class="select select-xs w-32" value={it.type} onchange={(e) => retype(k, (e.currentTarget as HTMLSelectElement).value)} title={TYPES[which].find((t) => t[0] === it.type)?.[2]}>
        {#each TYPES[which] as [v, name] (v)}<option value={v}>{name}</option>{/each}
      </select>
      <span class="flex-1"></span>
      {#if which === "monitor" && it.type !== "field"}
        <input class="input input-xs w-28" placeholder="name" value={it.name ?? ""} oninput={(e) => (it.name = (e.currentTarget as HTMLInputElement).value || null)} title="The monitor's name, on its spectrum" />
      {/if}
      <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" title="Remove" onclick={() => list.splice(k, 1)}><Trash2 size={13} /></button>
    </div>
    <div class="grid grid-cols-2 gap-2 @xl:grid-cols-4">
      {#if it.type === "mode" || it.type === "beam" || it.type === "flux"}
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Plane normal to</span>
          <select
            class="select select-sm w-full"
            value={it.normal ?? "x"}
            onchange={(e) => {
              const n = (e.currentTarget as HTMLSelectElement).value as "x" | "y" | "z";
              // the window across moves to the other axis
              if (it.normal !== n) [it.x_um, it.y_um] = [null, null];
              it.normal = n;
            }}
          >
            <option value="x">x</option>
            <option value="y">y</option>
            {#if it.type === "flux" && three}<option value="z">z</option>{/if}
          </select>
        </label>
        <NumField label="at {it.normal ?? 'x'}" length="um" bind:value={atB.get, atB.set} step={0.05} hint="Where the plane crosses its normal; a mode's or a beam's plane must be clear of the CPMLs" />
      {/if}
      {#if (which === "source" && (it.type === "mode" || it.type === "beam"))}
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Towards</span>
          <select class="select select-sm w-full" value={it.direction ?? "+"} onchange={(e) => (it.direction = (e.currentTarget as HTMLSelectElement).value)}>
            <option value="+">+{it.normal ?? "x"}</option>
            <option value="-">−{it.normal ?? "x"}</option>
          </select>
        </label>
      {/if}
      {#if (it.type === "mode" || it.type === "flux") && it.normal !== "z"}
        <label class="flex items-end gap-1.5 pb-1.5 text-xs faint">
          <input
            type="checkbox"
            class="toggle toggle-xs"
            checked={it[w] !== null}
            onchange={(e) => {
              const span = it.normal === "y" ? model.x_um : model.y_um;
              it[w] = (e.currentTarget as HTMLInputElement).checked ? [span[0], Number(((span[0] + span[1]) / 2).toFixed(3))] : null;
            }}
          />
          own window across
        </label>
        {#if it[w]}
          {@const lo = end(it, w, 0, span(w))}
          {@const hi = end(it, w, 1, span(w))}
          <NumField label="{w[0]} from" length="um" bind:value={lo.get, lo.set} step={0.05} hint="One guide of several: the window across the plane" />
          <NumField label="{w[0]} to" length="um" bind:value={hi.get, hi.set} step={0.05} />
        {/if}
      {/if}
      {#if it.type === "dipole" || it.type === "resonance"}
        {#each three ? ["x", "y", "z"] : ["x", "y"] as a, i (a)}
          {@const b = coordinate(it, "position_um", i)}
          <NumField label="at {a}" length="um" bind:value={b.get, b.set} step={0.05} />
        {/each}
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Component</span>
          <select class="select select-sm w-full" value={it.component ?? (three ? "Ey" : model.polarization === "te" ? "Hz" : "Ez")} onchange={(e) => (it.component = (e.currentTarget as HTMLSelectElement).value)}>
            {#each components as c (c)}<option value={c}>{c[0]}_{c.slice(1)}</option>{/each}
          </select>
        </label>
      {/if}
      {#if it.type === "plane_wave" || it.type === "flux_box"}
        {#each boxKeys as key (key)}
          {@const lo = end(it, key, 0, span(key))}
          {@const hi = end(it, key, 1, span(key))}
          <NumField label="box {key[0]} from" length="um" bind:value={lo.get, lo.set} step={0.05} hint={it.type === "plane_wave" ? "The total-field box: the wave is inside it, the scattered field outside" : "The box the flux leaves"} />
          <NumField label="box {key[0]} to" length="um" bind:value={hi.get, hi.set} step={0.05} />
        {/each}
      {/if}
      {#if it.type === "plane_wave"}
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Direction</span>
          <select class="select select-sm w-full" bind:value={it.direction}>
            {#each three ? ["+x", "-x", "+y", "-y", "+z", "-z"] : ["+x", "-x", "+y", "-y"] as d (d)}<option value={d}>{d}</option>{/each}
          </select>
        </label>
        {#if three}
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">E along</span>
            <select class="select select-sm w-full" bind:value={it.polarization}>
              {#each ["x", "y", "z"].filter((a) => a !== it.direction?.slice(1)) as a (a)}<option value={a}>{a}</option>{/each}
            </select>
          </label>
        {/if}
      {/if}
      {#if it.type === "beam"}
        {#each three ? ["across", "z"] : ["across"] as a, i (a)}
          {@const b = coordinate(it, "center_um", i)}
          <NumField label="centre {a === 'across' ? (it.normal === 'y' ? 'x' : 'y') : 'z'}" length="um" bind:value={b.get, b.set} step={0.05} hint="Where the beam's axis crosses its plane" />
        {/each}
        <NumField label="waist" length="um" bind:value={waistB.get, waistB.set} step={0.1} hint="The field's 1/e radius at the focus" />
        <OptField label="focus ahead" length="um" bind:value={it.focus_um} step={0.1} placeholder="0" hint="The focus's distance from the plane along the beam" />
        <OptField label="tilt" unit="°" bind:value={it.angle_deg} step={1} placeholder="0" hint="From the plane's normal, in the layer's plane" />
        {#if three}
          <label class="flex flex-col gap-1">
            <span class="text-xs font-medium text-base-content/70">Polarization</span>
            <select class="select select-sm w-full" bind:value={it.polarization}>
              <option value={null}>s (default)</option>
              <option value="s">s</option>
              <option value="p">p</option>
            </select>
          </label>
        {/if}
      {/if}
      {#if it.type === "mode" && three}
        <label class="flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">Mode</span>
          <select class="select select-sm w-full" bind:value={it.polarization}>
            <option value={null}>TE-like (default)</option>
            <option value="te">TE-like</option>
            <option value="tm">TM-like</option>
          </select>
        </label>
      {/if}
      {#if it.type === "field"}
        {@const b = wavelengths(it)}
        <label class="col-span-2 flex flex-col gap-1">
          <span class="text-xs font-medium text-base-content/70">At wavelengths (µm)</span>
          <input class="input input-sm w-full" value={b.get()} onchange={(e) => b.set((e.currentTarget as HTMLInputElement).value)} placeholder="1.55, 1.5205" title="Within the pulse's band; up to 20" />
        </label>
      {/if}
    </div>
  </div>
{/each}
<div class="flex flex-wrap gap-1">
  {#each TYPES[which] as [type, name, about] (type)}
    <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => add(type)} title={about}><Plus size={14} /> {name}</button>
  {/each}
</div>
