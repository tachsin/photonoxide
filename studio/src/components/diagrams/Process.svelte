<script lang="ts">
  // A silicon strip made, in five cross-sections from left to right: the SOI wafer (a silicon
  // film t thick on a buried oxide h thick over the silicon handle); resist spun on and exposed
  // through a mask, light of wavelength λ through a lens of numerical aperture NA; the resist
  // developed, left only where the mask's bar shaded it, w wide; the silicon etched where the
  // resist doesn't cover it, the etch stopping on the oxide; the resist stripped and oxide
  // deposited over everything, leaving a strip w by t clad in oxide. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";

  const [W, H] = [760, 300];
  const PANEL = 136;
  const GAP = 20;
  const x0 = (k: number) => 12 + k * (PANEL + GAP);
  /** The layers' faces, from the top: the resist's top, the film's top, the oxide's top, the handle's top, the bottom. */
  const [resist, film, box, handle, bottom] = [158, 172, 192, 236, 276];
  /** The strip: its middle and half its width, in a panel. */
  const mid = PANEL / 2;
  const half = 16;
  const steps = ["1. the wafer", "2. coat, expose", "3. develop", "4. etch", "5. strip, clad"];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A silicon strip made in five cross-sections: the SOI wafer, a silicon film t thick on a buried oxide h thick over the silicon handle; resist coated and exposed through a mask by light of wavelength λ through a lens of numerical aperture NA; the resist developed, left w wide where the mask shaded it; the silicon etched down to the oxide where the resist leaves it bare; the resist stripped and oxide deposited over everything, leaving a strip w by t clad in oxide">
  {#each steps as title, k (k)}
    {@const x = x0(k)}
    <Label x={x + mid} y={18} size={14} text={title} width={PANEL} />
    <!-- the handle and the buried oxide, the same in every panel -->
    <rect {x} y={handle} width={PANEL} height={bottom - handle} class="fill-base-content/20" />
    <rect {x} y={box} width={PANEL} height={handle - box} class="fill-info/15 stroke-info/40" stroke-width="1" />
    <!-- the silicon film: whole until the etch, then the strip -->
    {#if k < 3}
      <rect {x} y={film} width={PANEL} height={box - film} class="fill-primary/35 stroke-primary/70" stroke-width="1" />
    {:else}
      <rect x={x + mid - half} y={film} width={2 * half} height={box - film} class="fill-primary/35 stroke-primary/70" stroke-width="1" />
    {/if}
    <!-- the resist: everywhere when coated, under the bar once developed, gone once stripped -->
    {#if k === 1}
      <rect {x} y={resist} width={PANEL} height={film - resist} class="fill-warning/45" />
      <rect x={x + mid - half} y={resist} width={2 * half} height={film - resist} class="fill-warning/80" />
    {:else if k === 2 || k === 3}
      <rect x={x + mid - half} y={resist} width={2 * half} height={film - resist} class="fill-warning/80" />
    {/if}
    <!-- the cladding deposited over everything -->
    {#if k === 4}
      <rect {x} y={128} width={PANEL} height={film - 128} class="fill-info/15" />
      <rect {x} y={film} width={mid - half} height={box - film} class="fill-info/15" />
      <rect x={x + mid + half} y={film} width={PANEL - mid - half} height={box - film} class="fill-info/15" />
    {/if}
  {/each}

  <!-- 1: the wafer's layers -->
  <Label x={x0(0) + 4} y={(film + box) / 2} anchor="start" size={12} text="Si" width={40} />
  <Label x={x0(0) + 4} y={(box + handle) / 2} anchor="start" size={12} text="buried oxide" width={100} />
  <Label x={x0(0) + 4} y={(handle + bottom) / 2} anchor="start" size={12} text="Si handle" width={100} />
  <Dim x1={x0(0) + PANEL - 14} y1={film} x2={x0(0) + PANEL - 14} y2={box} outside />
  <Label x={x0(0) + PANEL - 20} y={film - 22} anchor="end" size={14} text="$t$" width={20} />
  <Dim x1={x0(0) + PANEL - 14} y1={box} x2={x0(0) + PANEL - 14} y2={handle} />
  <Label x={x0(0) + PANEL - 20} y={(box + handle) / 2} anchor="end" size={14} text="$h$" width={20} />

  <!-- 2: the mask, its bar over the strip, and the light through the lens -->
  {#each [x0(1)] as x (x)}
    <line x1={x} y1={96} x2={x + PANEL} y2={96} class="stroke-base-content/35" stroke-width="2" />
    <rect x={x + mid - half} y={92} width={2 * half} height={8} class="fill-base-content/75" />
    <Label x={x + 2} y={84} anchor="start" size={12} text="mask" width={50} />
    {#each [18, 38, 98, 118] as dx (dx)}
      <line x1={x + dx} y1={40} x2={x + dx} y2={150} class="stroke-accent/70" stroke-width="1.4" />
      <path d="M{x + dx},{154} l-3.5,-8 h7 z" class="fill-accent/70" />
    {/each}
    <Label x={x + mid} y={52} size={14} text={String.raw`$\lambda$, NA`} width={80} class="text-accent" />
    <Label x={x + 2} y={resist - 8} anchor="start" size={11} text="resist" width={50} class="text-base-content/70" />
  {/each}

  <!-- 3: the developed line, w wide -->
  <Dim x1={x0(2) + mid - half} y1={resist - 14} x2={x0(2) + mid + half} y2={resist - 14} outside />
  <Label x={x0(2) + mid} y={resist - 32} size={14} text="$w$" width={30} />

  <!-- 4: the etch, stopping on the oxide -->
  {#each [x0(3) + 18, x0(3) + PANEL - 18] as ax (ax)}
    <line x1={ax} y1={120} x2={ax} y2={box - 6} class="stroke-error/70" stroke-width="1.4" />
    <path d="M{ax},{box - 2} l-3.5,-8 h7 z" class="fill-error/70" />
  {/each}
  <Label x={x0(3) + mid} y={112} size={12} text="stops on the oxide" width={PANEL} class="text-error/80" />

  <!-- 5: the strip, w by t, in its cladding -->
  <Label x={x0(4) + 4} y={142} anchor="start" size={12} text="cladding" width={80} />
  <Dim x1={x0(4) + mid - half} y1={film - 12} x2={x0(4) + mid + half} y2={film - 12} outside />
  <Label x={x0(4) + mid} y={film - 30} size={14} text="$w$" width={30} />
  <Label x={x0(4) + mid + half + 6} y={(film + box) / 2} anchor="start" size={14} text="$t$" width={20} />
</svg>
