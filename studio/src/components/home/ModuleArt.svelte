<script module lang="ts">
  export type Art = "components" | "chip" | "fabrication" | "academy" | "materials" | "validation";
</script>

<script lang="ts">
  // A module's picture on Home's cards: a few shapes of what the module shows (a ring beside its
  // bus, a chip's wired parts, a cross-section's layers, a mode's profile, a dispersion curve,
  // points on a curve they agree with), drawn in the theme's colours so it follows light and
  // dark. Inline SVG: nothing to load, nothing to draw on a canvas, no WebGL context.
  let { art }: { art: Art } = $props();
</script>

<svg viewBox="0 0 200 96" class="h-full w-full" aria-hidden="true" preserveAspectRatio="xMidYMid meet">
  {#if art === "components"}
    <!-- an all-pass ring beside its bus, its field brightest in the ring -->
    <defs>
      <radialGradient id="ma-ring" cx="0.5" cy="0.5" r="0.5"><stop offset="0.55" class="[stop-color:var(--color-primary)]" stop-opacity="0" /><stop offset="0.8" class="[stop-color:var(--color-primary)]" stop-opacity="0.35" /><stop offset="1" class="[stop-color:var(--color-primary)]" stop-opacity="0" /></radialGradient>
    </defs>
    <circle cx="100" cy="42" r="34" fill="url(#ma-ring)" />
    <circle cx="100" cy="42" r="26" fill="none" class="stroke-primary" stroke-width="5" />
    <rect x="18" y="74" width="164" height="5" rx="2.5" class="fill-base-content/30" />
    <rect x="18" y="74" width="60" height="5" rx="2.5" class="fill-secondary" />
    <circle cx="100" cy="42" r="26" fill="none" class="stroke-base-100" stroke-width="1" stroke-dasharray="3 7" opacity="0.6" />
  {:else if art === "chip"}
    <!-- parts on a die, wired port to port -->
    <rect x="16" y="10" width="168" height="76" rx="8" class="fill-base-content/5 stroke-base-content/15" stroke-width="1.5" />
    <path d="M44 34 H 76 M 108 34 C 124 34 124 62 140 62 M 108 34 H 140 M 44 62 H 76 Q 92 62 92 48" fill="none" class="stroke-secondary" stroke-width="3" stroke-linecap="round" />
    <rect x="28" y="26" width="18" height="16" rx="3" class="fill-primary" />
    <rect x="76" y="24" width="32" height="20" rx="4" class="fill-primary/70" />
    <circle cx="154" cy="34" r="10" fill="none" class="stroke-primary" stroke-width="3" />
    <rect x="140" y="54" width="28" height="16" rx="3" class="fill-accent/80" />
    <rect x="28" y="54" width="18" height="16" rx="3" class="fill-primary" />
  {:else if art === "fabrication"}
    <!-- a cross-section's layers, the strip etched into the top one -->
    <rect x="20" y="66" width="160" height="20" rx="3" class="fill-base-content/25" />
    <rect x="20" y="48" width="160" height="18" class="fill-info/35" />
    <rect x="82" y="34" width="36" height="14" rx="1.5" class="fill-primary" />
    <path d="M20 48 V 30 Q 20 22 30 22 H 170 Q 180 22 180 30 V 48" class="fill-secondary/15 stroke-secondary/50" stroke-width="1.5" stroke-dasharray="4 4" />
    <path d="M100 4 V 16 M94 11 L100 17 L106 11" fill="none" class="stroke-accent" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
  {:else if art === "academy"}
    <!-- a mode's profile in its core, and the wave it carries -->
    <rect x="76" y="14" width="48" height="70" rx="3" class="fill-primary/12" />
    <path d="M20 84 C 70 84 76 18 100 18 C 124 18 130 84 180 84" fill="none" class="stroke-primary" stroke-width="3.5" stroke-linecap="round" />
    <path d="M20 50 q 10 -14 20 0 t 20 0 t 20 0 t 20 0 t 20 0 t 20 0 t 20 0 t 20 0" fill="none" class="stroke-secondary/70" stroke-width="2" />
  {:else if art === "materials"}
    <!-- an index falling with the wavelength: Sellmeier's curve and measured points on it -->
    <path d="M24 12 V 84 H 184" fill="none" class="stroke-base-content/25" stroke-width="1.5" />
    <path d="M30 18 C 50 52 80 64 120 70 S 170 76 180 77" fill="none" class="stroke-primary" stroke-width="3.5" stroke-linecap="round" />
    {#each [[42, 37], [64, 53], [92, 63], [128, 71], [160, 75]] as [x, y] (x)}
      <circle cx={x} cy={y} r="4" class="fill-accent stroke-base-100" stroke-width="1.5" />
    {/each}
  {:else}
    <!-- a solver's numbers on the exact curve, each checked -->
    <path d="M24 12 V 84 H 184" fill="none" class="stroke-base-content/25" stroke-width="1.5" />
    <path d="M28 76 C 70 74 90 30 120 24 S 170 20 182 20" fill="none" class="stroke-success/70" stroke-width="3" stroke-linecap="round" />
    {#each [[46, 74], [80, 56], [104, 33], [140, 22], [170, 20]] as [x, y] (x)}
      <path d="M{x} {y - 9} V {y + 9}" class="stroke-base-content/35" stroke-width="1.5" />
      <circle cx={x} cy={y} r="4.5" class="fill-success stroke-base-100" stroke-width="1.5" />
    {/each}
  {/if}
</svg>
