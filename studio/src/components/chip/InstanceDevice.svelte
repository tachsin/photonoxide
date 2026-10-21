<script lang="ts">
  // A chip instance's device design in the inspector: its kind and fidelity (a visible choice,
  // with its model's cost and error on the device's page), its job file if it names one, its
  // parameters' expressions, and its geometry seen from above; "Open device" goes to its page
  // with these values.
  import { ExternalLink } from "@lucide/svelte";

  import { api, type DeviceGeometry, type DeviceRef, type InstanceDevice, type Placed } from "../../lib/api";
  import { change, openInstanceDevice, resolve } from "../../lib/chip.svelte";
  import { deviceOf, devices, loadDevices } from "../../lib/device.svelte";
  import TopView from "../device/TopView.svelte";

  let { inst }: { inst: Placed } = $props();

  let shown = $state<InstanceDevice | null>(null);
  let problem = $state("");
  let geometry = $state<DeviceGeometry | null>(null);

  $effect(() => {
    const d = inst.device ? ($state.snapshot(inst.device) as DeviceRef) : null;
    if (!d) return;
    let live = true;
    loadDevices();
    resolve(d)
      .then(async (r) => {
        if (!live) return;
        shown = r;
        problem = "";
        const g = await api.deviceGeometry(r.spec);
        if (live) geometry = g;
      })
      .catch((e) => {
        if (!live) return;
        shown = null;
        geometry = null;
        problem = String(e);
      });
    return () => (live = false);
  });

  const info = $derived(shown && devices.loaded ? deviceOf(shown.id) : undefined);
  const fields = $derived(geometry?.fields.filter((f) => f.device) ?? []);

  /** Sets the design's parameter `name` to `expression`: on its own design, or as a change of its job file's. */
  function setParameter(name: string, expression: string) {
    const e = expression.trim();
    if (!e) return;
    change((chip) => {
      const i = chip.instance.find((x) => x.name === inst.name);
      if (!i?.device) return;
      i.device.parameters = { ...i.device.parameters, [name]: e };
    });
  }

  function setFidelity(f: string) {
    change((chip) => {
      const i = chip.instance.find((x) => x.name === inst.name);
      if (i?.device) i.device.fidelity = f;
    });
  }
</script>

<p class="panel-title mt-5">Device design</p>
{#if problem}
  <p class="mt-2 text-xs text-error">{problem}</p>
{:else if !shown}
  <div class="mt-2"><span class="loading loading-dots loading-xs"></span></div>
{:else}
  {@const s = shown.spec}
  <p class="mt-2 text-sm font-medium">{s.kind}</p>
  {#if info?.about}<p class="mt-0.5 text-xs faint">{info.about}</p>{/if}
  <div class="mt-2 flex items-center gap-2 text-sm">
    <select class="select select-xs w-32" value={s.fidelity} onchange={(e) => setFidelity(e.currentTarget.value)} aria-label="Its fidelity" title="The model the chip computes this instance by">
      {#each info?.fidelities ?? [] as m (m.fidelity)}<option value={m.fidelity}>{m.fidelity}</option>{/each}
    </select>
    <span class="flex-1"></span>
    <button class="btn btn-link btn-xs h-auto min-h-0 gap-1 p-0" onclick={() => openInstanceDevice(inst)} title="Open its page with these values; coming back applies the changes or keeps them as a variant"
      ><ExternalLink size={12} /> Open device</button
    >
  </div>
  {#if shown.job}<p class="mt-1 truncate text-xs num muted" title="The design is this job file's; the values below change it for this instance only">{shown.job}</p>{/if}
  {#if s.fidelity === "2D"}<p class="mt-1 text-xs text-warning">2D: an estimate, never the device's performance.</p>{/if}
  {#if geometry?.scene}
    <div class="mt-2 rounded-md border border-base-content/8"><TopView scene={geometry.scene} ports={geometry.ports} height={140} /></div>
  {:else if geometry?.problem}
    <p class="mt-2 text-xs text-error">{geometry.problem}</p>
  {/if}
  <div class="mt-2 space-y-1.5">
    {#each fields as f (f.name)}
      {@const p = info?.parameters.find((q) => q.name === f.name)}
      <label class="flex items-center gap-2 text-xs">
        <span class="w-24 shrink-0 truncate font-medium num" title={f.name}>{f.name}</span>
        <input
          class="input input-xs min-w-0 flex-1 num"
          value={f.expression}
          onchange={(e) => setParameter(f.name, e.currentTarget.value)}
          aria-label="{f.name}: an expression in {f.unit || 'its unit'}"
          title={p ? `${p.min} to ${p.max} ${p.unit}; ${p.default} by default` : f.unit}
        />
        <span class="w-11 shrink-0 faint">{f.unit}</span>
      </label>
      {#if f.error}<p class="text-xs text-error">{f.error.message}</p>{/if}
    {/each}
  </div>
{/if}
