<script lang="ts">
  // The process as photons: each photon one wave packet, its wiggle as long as its wavelength
  // (so a bluer, more energetic photon wiggles tighter) and its colour the colour it is (a deep
  // red past the visible, a violet before it, both dashed as the eye doesn't see them). The
  // photons one event takes in meet at a point in the crystal and the photons it gives out leave
  // from there; under them, each photon's energy and the balance. The nonlinear processes and the
  // pair sources both draw it, each giving its photons and what they meet in.
  import { electronVolts, type Photon } from "../../lib/nonlinear";
  import MathText from "../MathText.svelte";

  let {
    inputs,
    outputs,
    medium,
    poled = false,
  }: {
    /** The photons one event takes in, and gives out. */
    inputs: Photon[];
    outputs: Photon[];
    /** What they meet in, as it is labelled under it ("linbo3", "ring"). */
    medium: string;
    /** Whether to draw its domains flipped. */
    poled?: boolean;
  } = $props();

  const p = $derived({ inputs, outputs });

  const W = 660;
  const H = 190;
  // room on each side for a label as wide as "1064 nm · 1.165 eV"
  const LABEL = 168;
  const box = { x: (W - 100) / 2, y: 30, w: 100, h: 120 };
  const at = { x: box.x + box.w / 2, y: box.y + box.h / 2 };

  /** A wavelength's colour: its hue in the visible, a deep red past it and a violet before it. */
  function colour(l: number | null): string {
    if (!l) return "var(--color-base-content)";
    if (l > 0.75) return "#b4233c";
    if (l < 0.39) return "#6d3fd8";
    return `hsl(${Math.round(((0.75 - l) / (0.75 - 0.39)) * 270)} 85% 48%)`;
  }
  const seen = (l: number | null) => !!l && l >= 0.39 && l <= 0.75;

  /** A photon from (x0, y0) to (x1, y1): a wave packet, its period from its wavelength. */
  function packet(x0: number, y0: number, x1: number, y1: number, l: number | null): string {
    const [dx, dy] = [x1 - x0, y1 - y0];
    const length = Math.hypot(dx, dy);
    const [ux, uy] = [dx / length, dy / length];
    const period = 8 + 13 * Math.min(l ?? 1, 4);
    const steps = Math.ceil(length / 2);
    let d = "";
    for (let i = 0; i <= steps; i++) {
      const s = (i / steps) * length;
      // tapered at both ends: a packet, not an endless wave
      const amp = 6 * Math.sin((Math.PI * s) / length);
      const off = amp * Math.sin((2 * Math.PI * s) / period);
      d += `${i ? "L" : "M"}${(x0 + ux * s - uy * off).toFixed(1)} ${(y0 + uy * s + ux * off).toFixed(1)}`;
    }
    return d;
  }

  /** Where each photon starts (in) or ends (out), spread along the side. */
  const spread = (n: number, k: number) => 34 + ((k + 0.5) * (H - 68)) / n;

  /** What each photon is labelled with: its wavelength, its energy or its frequency. */
  let unit = $state<"nm" | "eV" | "Hz">("nm");
  const UNITS: [typeof unit, string][] = [
    ["nm", "Each photon's wavelength in vacuum"],
    ["eV", "Each photon's energy"],
    ["Hz", "Each photon's frequency"],
  ];

  /** The SI prefixes, largest first. */
  const PREFIXES: [number, string][] = [[12, "T"], [9, "G"], [6, "M"], [3, "k"], [0, ""], [-3, "m"], [-6, "µ"], [-9, "n"], [-12, "p"]];
  /** A value as the number in front of its prefix (1 to 999, four figures) and the prefix. */
  function prefixed(v: number): [number, string] {
    const [e, prefix] = PREFIXES.find(([e]) => Math.abs(v) >= 10 ** e) ?? PREFIXES[PREFIXES.length - 1];
    return [Number((v / 10 ** e).toPrecision(4)), prefix];
  }
  /** A value in its base unit with its prefix: 1.064 µm, 362 meV, 281.8 THz. */
  function si(v: number, base: string): string {
    const [n, prefix] = prefixed(v);
    return `${n} ${prefix}${base}`;
  }
  /** A length in metres as TeX, with its prefix: 1.064\,\mu\text{m}. */
  function lengthTex(metres: number): string {
    const [n, prefix] = prefixed(metres);
    return `${n}\\,${prefix === "µ" ? "\\mu\\text{m}" : `\\text{${prefix}m}`}`;
  }
  /** The photons' inverse wavelengths added up, in and out, as TeX: what is conserved, as λ isn't. */
  function inverseBalance(inputs: Photon[], outputs: Photon[]): string {
    const all = [...inputs, ...outputs];
    if (!all.every((x) => x.wavelength_um)) return "the balance needs every wavelength";
    const each = (x: Photon) => `\\frac{1}{${lengthTex((x.wavelength_um as number) * 1e-6)}}`;
    const whole = (list: Photon[]) => `\\frac{1}{${lengthTex(1e-6 / list.reduce((a, x) => a + 1 / (x.wavelength_um as number), 0))}}`;
    return `$${inputs.map(each).join(" + ")} = ${whole(inputs)}$ in → $${whole(outputs)}$ out`;
  }
  /** What the balance says, in each unit. */
  const MEANS: Record<typeof unit, string> = {
    nm: String.raw`Each wiggle is one photon, as long as its wavelength $\lambda$: what adds up is $1/\lambda$, not $\lambda$.`,
    eV: String.raw`Each wiggle is one photon, of energy $E = hc/\lambda$: the energy going in is the energy coming out.`,
    Hz: String.raw`Each wiggle is one photon, of frequency $\nu = c/\lambda$: the frequencies add up, as $E = h\nu$ is conserved.`,
  };
  const C = 299_792_458;
  /** A photon's energy (eV) or frequency (Hz), from its wavelength in µm: what adds up in the balance. */
  const amount = (l: number) => (unit === "Hz" ? C / (l * 1e-6) : electronVolts(l));
  /** A photon's label in the unit chosen. */
  const label = (l: number | null) => (!l ? "?" : unit === "nm" ? si(l * 1e-6, "m") : si(amount(l), unit));
  /** The balance in energy or frequency (wavelengths don't add up: eV when "nm" is chosen). */
  const sum = (list: Photon[]) => (list.every((x) => x.wavelength_um) ? si(list.reduce((a, x) => a + amount(x.wavelength_um as number), 0), unit === "Hz" ? "Hz" : "eV") : "?");
  const term = (l: number | null) => (!l ? "?" : si(amount(l), unit === "Hz" ? "Hz" : "eV"));
</script>

<div class="flex justify-end">
  <div class="join" role="group" aria-label="Label the photons by">
    {#each UNITS as [u, about] (u)}
      <button class="btn join-item btn-xs {unit === u ? 'btn-active' : ''}" title={about} aria-pressed={unit === u} onclick={() => (unit = u)}>{u}</button>
    {/each}
  </div>
</div>
<svg viewBox="0 0 {W} {H}" class="w-full" role="img" aria-label="The photons one event takes in and gives out">
  <defs>
    <radialGradient id="nl-vertex"><stop offset="0" stop-color="#f7b955" stop-opacity="0.9" /><stop offset="1" stop-color="#f7b955" stop-opacity="0" /></radialGradient>
  </defs>

  <!-- the crystal, its domains flipped every half period when poled, and the point the photons meet -->
  <rect x={box.x} y={box.y} width={box.w} height={box.h} rx="8" class="fill-base-200 stroke-base-content/20" stroke-width="1.2" />
  {#if poled}
    {#each Array.from({ length: 8 }, (_, i) => i) as i (i)}
      {#if i % 2}<rect x={box.x + (i * box.w) / 8} y={box.y + 1} width={box.w / 8} height={box.h - 2} class="fill-base-content/7" />{/if}
    {/each}
  {/if}
  <circle cx={at.x} cy={at.y} r="16" fill="url(#nl-vertex)" />
  <circle cx={at.x} cy={at.y} r="3.5" class="fill-base-content/70" />
  <text x={at.x} y={box.y + box.h + 16} text-anchor="middle" class="fill-base-content/55 text-[11px]">
    {medium}{poled ? ", poled" : ""}
  </text>

  {#each p.inputs as ph, k (k)}
    {@const y = spread(p.inputs.length, k)}
    <path d={packet(LABEL + 8, y, at.x - 10, at.y + (y - at.y) * 0.12, ph.wavelength_um)} fill="none" stroke={colour(ph.wavelength_um)} stroke-width="2.4" stroke-linecap="round" stroke-dasharray={seen(ph.wavelength_um) ? undefined : "5 3"} />
    <text x={LABEL} y={y - 3} text-anchor="end" class="fill-base-content/85 text-[11.5px]">{ph.role}</text>
    <text x={LABEL} y={y + 11} text-anchor="end" class="fill-base-content/55 text-[10.5px] num">{label(ph.wavelength_um)}</text>
  {/each}

  {#each p.outputs as ph, k (k)}
    {@const y = spread(p.outputs.length, k)}
    <path d={packet(at.x + 10, at.y + (y - at.y) * 0.12, W - LABEL - 8, y, ph.wavelength_um)} fill="none" stroke={colour(ph.wavelength_um)} stroke-width="2.4" stroke-linecap="round" stroke-dasharray={seen(ph.wavelength_um) ? undefined : "5 3"} />
    <path d="M{W - LABEL - 8} {y} l-7 -4 v8 z" fill={colour(ph.wavelength_um)} />
    <text x={W - LABEL} y={y - 3} class="fill-base-content/85 text-[11.5px]">{ph.role}</text>
    <text x={W - LABEL} y={y + 11} class="fill-base-content/55 text-[10.5px] num">{label(ph.wavelength_um)}</text>
  {/each}
</svg>
<!-- the balance and what it means, in the unit chosen -->
{#if unit === "nm"}
  <p class="mt-1 text-center text-xs muted"><MathText text={inverseBalance(p.inputs, p.outputs)} /></p>
{:else}
  <p class="mt-1 text-center text-xs num muted">{p.inputs.map((x) => term(x.wavelength_um)).join(" + ")} = {sum(p.inputs)} in → {sum(p.outputs)} out</p>
{/if}
<p class="mt-0.5 text-center text-[11px] faint"><MathText text={MEANS[unit]} /></p>
