<script lang="ts">
  // A lithography machine in 2D, cut through its middle: where the light (or the electrons) comes
  // from, each part it passes, the mask, and the wafer. The electron beam is driven by the
  // writer's path over the design's shapes (lib/litho.ts): on while it writes a line, blanked onto
  // the aperture while it moves. The contact aligner and the imprint press go through their
  // cycles; the light flows as dashes. A schematic, not to scale.
  import { collector, contactCycle, EUV, euvPath, type Beam, type Kind, type Segment, type Tool, type XY } from "../../lib/litho";

  let {
    tool,
    exposeShapes,
    segments,
    beam,
    span,
    time,
    running,
  }: {
    tool: Tool;
    /** Whether the light (or beam) falls on the shapes themselves. */
    exposeShapes: boolean;
    segments: Segment[];
    beam: Beam;
    /** The design's extent along x, µm: where the beam lands across the field. */
    span: [number, number];
    /** Seconds, for the cycles. */
    time: number;
    running: boolean;
  } = $props();

  const kind: Kind = $derived(tool.kind);
  const light = $derived(tool.colour);

  // ---- helpers ----

  const unit = (v: XY): XY => {
    const l = Math.hypot(v[0], v[1]) || 1;
    return [v[0] / l, v[1] / l];
  };
  const f1 = (v: number) => v.toFixed(1);
  /** A mirror at `p` that turns light from `a` to `b`: a short concave arc facing them. */
  function mirror(a: XY, p: XY, b: XY, half = 14): string {
    const u = unit([a[0] - p[0], a[1] - p[1]]);
    const v = unit([b[0] - p[0], b[1] - p[1]]);
    const n = unit([u[0] + v[0], u[1] + v[1]]);
    const t: XY = [-n[1], n[0]];
    const e1: XY = [p[0] + t[0] * half + n[0] * 3, p[1] + t[1] * half + n[1] * 3];
    const e2: XY = [p[0] - t[0] * half + n[0] * 3, p[1] - t[1] * half + n[1] * 3];
    return `M${f1(e1[0])},${f1(e1[1])}Q${f1(p[0] - n[0] * 3)},${f1(p[1] - n[1] * 3)} ${f1(e2[0])},${f1(e2[1])}`;
  }
  const line = (pts: XY[]) => "M" + pts.map((p) => `${f1(p[0])},${f1(p[1])}`).join("L");

  /** Mask blocks across [0, 1] for the cuts: lines (the shapes covered) or narrow openings (the shapes open). */
  const blocks = $derived(
    exposeShapes
      ? [
          [0, 0.3],
          [0.42, 0.72],
          [0.84, 1],
        ]
      : [
          [0.14, 0.26],
          [0.52, 0.64],
        ],
  );
  const gaps = $derived.by(() => {
    const out: [number, number][] = [];
    let at = 0;
    for (const [a, b] of blocks) {
      if (a > at) out.push([at, a]);
      at = b;
    }
    if (at < 1) out.push([at, 1]);
    return out;
  });

  // ---- EUV: the 3D model's x and z, drawn ----

  const E = (p: XY): XY => [330 + 33 * p[0], 345 - 34 * p[1]];
  const euv = euvPath().map(E);
  const col = collector();
  /** The collector's two arcs and its rim, drawn. */
  const coll = (() => {
    const perp: XY = [-col.axis[1], col.axis[0]];
    const at = (r: number, s: number): XY => E([col.centre[0] + col.axis[0] * s + perp[0] * r, col.centre[1] + col.axis[1] * s + perp[1] * r]);
    const up = col.profile.map(([r, s]) => at(r, s));
    const down = col.profile.map(([r, s]) => at(-r, s));
    return { up, down, rim: [up[up.length - 1], down[down.length - 1]] as [XY, XY] };
  })();
  const P = E(EUV.plasma);
  const IF = E(EUV.focus);
  const laserFrom = E([EUV.plasma[0] - 2.6 * col.axis[0], EUV.plasma[1] - 2.6 * col.axis[1]]);
  const W = E(EUV.wafer);
  const RET = E(EUV.reticle);
  /** The droplets cross in front of the collector, across its axis, one every fifth of a second; each flashes as it passes the laser's focus. */
  const across: XY = [-col.axis[1], col.axis[0]];
  const NOZZLE = E([EUV.plasma[0] + across[0] * 1.6, EUV.plasma[1] + across[1] * 1.6]);
  const CATCHER = E([EUV.plasma[0] - across[0] * 1.4, EUV.plasma[1] - across[1] * 1.4]);
  const DROPS = 6;
  const drops = $derived(
    Array.from({ length: DROPS }, (_, k): XY | null => {
      const u = (((time / 1.2 + k / DROPS) % 1) + 1) % 1;
      if (u > 0.53 && u < 0.6) return null;
      return [NOZZLE[0] + (CATCHER[0] - NOZZLE[0]) * u, NOZZLE[1] + (CATCHER[1] - NOZZLE[1]) * u];
    }),
  );
  const flash = $derived(Math.exp(-(((((time / 1.2) * DROPS) % 1) - 0.2) ** 2) / 0.004));
  /** Rays from the plasma off the collector to the focus: the ellipse sends each there. */
  const rimRays: XY[] = [coll.up[5], coll.up[16], coll.up[27], coll.down[5], coll.down[16], coll.down[27]];

  // ---- the electron beam ----

  /** Where the beam lands across the field, px from the axis. */
  const land = $derived.by(() => {
    const [x0, x1] = span;
    const m = (x0 + x1) / 2;
    const half = (x1 - x0) / 2 || 1;
    return (x: number) => ((x - m) / half) * 52;
  });
  const dx = $derived(land(beam.at[0]));
  /** The written lines' footprint on the resist, as the side view sees it. */
  const footprint = $derived.by(() => {
    let d = "";
    const n = Math.min(beam.done, segments.length);
    for (let k = 0; k < n; k++) {
      const s = segments[k];
      d += `M${f1(250 + land(s.a[0]))},311L${f1(250 + land(s.b[0]))},311`;
    }
    if (beam.on && beam.done < segments.length) d += `M${f1(250 + land(segments[beam.done].a[0]))},311L${f1(250 + dx)},311`;
    return d;
  });
  /** The beam's outline down to the blanker: (y, half-width) pairs, through two crossovers. */
  const ENVELOPE: [number, number][] = [
    [32, 0.6],
    [46, 2.6],
    [56, 0.9],
    [77, 6],
    [100, 1.6],
    [122, 1.6],
  ];
  const envelope = (pts: [number, number][]) => "M" + pts.map(([y, h]) => `${250 - h},${y}`).join("L") + "L" + [...pts].reverse().map(([y, h]) => `${250 + h},${y}`).join("L") + "Z";
  /** A magnetic lens, both sides: its iron yoke and its coil, `gap` from the axis. */
  function lens(y: number, h: number, w: number, gap: number) {
    return [
      { x: 250 - gap - w, y, w, h },
      { x: 250 + gap, y, w, h },
    ];
  }

  // ---- the cycles ----

  /** The contact aligner: the microscope in and out, the shutter open and shut (lib/litho.ts). */
  const contact = $derived(contactCycle(time));
  /** How far each objective has moved out of the light, px. */
  const scopeOut = $derived((1 - contact.scope) * 66);
  /** Where the shutter's edge is: the light passes left of it. */
  const shutterEdge = $derived(156 + 188 * contact.shutter);
  /** The imprint press: down, pressed and heated, up, the resist keeping the relief. */
  const pressPhase = $derived((time % 7) / 7);
  const pressDrop = $derived(pressPhase < 0.2 ? (pressPhase / 0.2) * 60 : pressPhase < 0.5 ? 60 : pressPhase < 0.68 ? 60 * (1 - (pressPhase - 0.5) / 0.18) : 0);
  const heated = $derived(pressPhase > 0.2 && pressPhase < 0.5);
  const relief = $derived(pressPhase > 0.2);
</script>

<svg viewBox="0 0 560 {kind === 'imprint' ? 280 : 362}" class="litho block h-auto w-full {running ? '' : 'paused'}" style="--light: {light}" role="img" aria-label="The {tool.label} tool in 2D">
  <defs>
    <radialGradient id="litho-glow">
      <stop offset="0" stop-color={light} stop-opacity="0.95" />
      <stop offset="1" stop-color={light} stop-opacity="0" />
    </radialGradient>
    <radialGradient id="litho-hot">
      <stop offset="0" stop-color="#fff7ed" stop-opacity="1" />
      <stop offset="0.35" stop-color={light} stop-opacity="0.9" />
      <stop offset="1" stop-color={light} stop-opacity="0" />
    </radialGradient>
    <pattern id="litho-coil" width="4" height="4" patternUnits="userSpaceOnUse">
      <rect width="4" height="4" fill="#d97706" fill-opacity="0.35" />
      <circle cx="2" cy="2" r="1.1" fill="#b45309" />
    </pattern>
  </defs>

  {#if kind === "euv"}
    <!-- the vacuum vessel, the laser outside it -->
    <rect x="78" y="24" width="474" height="334" rx="14" class="vacuum" />
    <text x="88" y="40" class="label">vacuum: air absorbs 13.4 nm</text>
    <rect x="6" y="326" width="56" height="22" rx="4" class="box" />
    <text x="34" y="340" class="label" text-anchor="middle">laser</text>
    <rect x="75" y="327" width="6" height="12" class="window" />
    <path d={line([laserFrom, P])} class="pump" />

    <!-- tin droplets past the plasma point, the collector, the cone to the intermediate focus -->
    <circle cx={NOZZLE[0]} cy={NOZZLE[1]} r="6" class="metal" />
    <text x={NOZZLE[0] - 9} y={NOZZLE[1] + 3} class="label" text-anchor="end">tin droplets</text>
    {#each drops as d, k (k)}
      {#if d}<circle cx={d[0]} cy={d[1]} r="2.4" class="tin" />{/if}
    {/each}
    <circle cx={CATCHER[0]} cy={CATCHER[1]} r="7" class="metal" />
    <polygon points={[...coll.up, IF, ...coll.down.slice().reverse()].map((p) => `${f1(p[0])},${f1(p[1])}`).join(" ")} class="beam" />
    {#each rimRays as q, i (i)}
      <path d={line([P, q, IF])} class="ray thin" />
    {/each}
    <path d={line(coll.up)} class="mirror thick" />
    <path d={line(coll.down)} class="mirror thick" />
    <text x="106" y="296" class="label" text-anchor="end">collector</text>
    <text x="106" y="307" class="label" text-anchor="end">(ellipsoid)</text>
    <circle cx={P[0]} cy={P[1]} r={6 + 16 * flash} fill="url(#litho-hot)" />
    <text x={P[0] + 26} y={P[1] + 26} class="label">plasma</text>
    <circle cx={IF[0]} cy={IF[1]} r="2.6" fill={light} />
    <text x={IF[0]} y={IF[1] - 10} class="label" text-anchor="middle">intermediate focus</text>

    <!-- the illuminator, the reticle, the projection optics, the wafer -->
    <rect x="398" y="84" width="146" height="232" rx="12" class="column" />
    <path d={line(euv)} class="beam-line" />
    <path d={line(euv)} class="ray" />
    <path d={mirror(euv[0], euv[1], euv[2], 18)} class="mirror facets" />
    <text x={euv[1][0] + 4} y={euv[1][1] + 24} class="label" text-anchor="end">field facet mirror</text>
    <path d={mirror(euv[1], euv[2], euv[3], 16)} class="mirror facets" />
    <text x={euv[2][0] - 10} y={euv[2][1] + 4} class="label" text-anchor="end">pupil facet mirror</text>
    {#each EUV.mirrors as _, i (i)}
      <path d={mirror(euv[3 + i], euv[4 + i], euv[5 + i], 13)} class="mirror" />
      <text x={euv[4 + i][0] + (euv[4 + i][0] > 480 ? 10 : -10)} y={euv[4 + i][1] + 3} class="label tiny" text-anchor={euv[4 + i][0] > 480 ? "start" : "end"}>M{i + 1}</text>
    {/each}
    <text x="404" y="300" class="label">projection optics:</text>
    <text x="404" y="311" class="label">6 mirrors, 4× smaller</text>
    <g class="scan-mask">
      <rect x={RET[0] - 62} y={RET[1] - 22} width="124" height="9" rx="2" class="metal" />
      <rect x={RET[0] - 55} y={RET[1] - 13} width="110" height="10" class="ml-body" />
      {#each Array.from({ length: 4 }, (_, k) => k) as k (k)}
        <rect x={RET[0] - 55} y={RET[1] - 3 + k * 0.8} width="110" height="0.5" class="mo" />
      {/each}
      {#each [0, 1, 2, 3, 4, 5] as k (k)}
        <rect x={RET[0] - 50 + k * 18} y={RET[1]} width="9" height="3" class="absorber" />
      {/each}
    </g>
    <text x={RET[0] - 68} y={RET[1] - 14} class="label" text-anchor="end">reticle stage ⇆</text>
    <text x={RET[0] - 68} y={RET[1] - 2} class="label" text-anchor="end">reticle (mask), face down</text>
    <g class="scan-wafer">
      <rect x={W[0] - 60} y={W[1] + 7} width="120" height="12" rx="3" class="metal" />
      <rect x={W[0] - 52} y={W[1] + 1} width="104" height="6" class="wafer" />
      <rect x={W[0] - 52} y={W[1] - 1} width="104" height="2.2" class="resist" />
    </g>
    <rect x={W[0] - 9} y={W[1] - 2} width="18" height="3" fill={light} class="pulse" />
    <text x={W[0] - 66} y={W[1] + 16} class="label" text-anchor="end">wafer stage ⇆</text>
  {:else if kind === "projection" || kind === "immersion"}
    <!-- the source and the beam delivery -->
    <rect x="16" y="12" width="140" height="30" rx="6" class="box" />
    <text x="86" y="31" class="label" text-anchor="middle">{tool.source}</text>
    <path d="M156,27 H250" class="beam-line" />
    <path d="M156,27 H250" class="ray" />
    <path d="M240,17 L262,39" class="mirror" />
    <text x="270" y="22" class="label">fold mirror</text>
    <!-- the illuminator: fly's-eye arrays, condenser, blades -->
    <rect x="214" y="30" width="72" height="78" class="beam" />
    {#each [228, 241, 254, 267, 280] as x (x)}
      <path d="M{x - 4},30 V108" class="ray thin" />
    {/each}
    {#each [48, 60] as y (y)}
      {#each [0, 1, 2, 3, 4, 5] as k (k)}
        <ellipse cx={220 + k * 12} cy={y} rx="5.4" ry="3.6" class="lens" />
      {/each}
    {/each}
    <text x="200" y="57" class="label" text-anchor="end">fly's-eye arrays: even light</text>
    <ellipse cx="250" cy="80" rx="52" ry="6" class="lens" />
    <text x="190" y="83" class="label" text-anchor="end">condenser</text>
    <rect x="196" y="94" width="26" height="4" class="metal" />
    <rect x="278" y="94" width="26" height="4" class="metal" />
    <text x="190" y="99" class="label" text-anchor="end">blades: frame the slit</text>
    <!-- the reticle on its stage -->
    <g class="scan-mask">
      <rect x="166" y="104" width="22" height="7" rx="1.5" class="metal" />
      <rect x="312" y="104" width="22" height="7" rx="1.5" class="metal" />
      <rect x="172" y="111" width="156" height="8" rx="1" class="quartz" />
      {#each blocks as [a, b], i (i)}
        <rect x={182 + a * 136} y="119" width={(b - a) * 136} height="2.6" class="chrome" />
      {/each}
    </g>
    <text x="344" y="114" class="label">reticle (mask): quartz,</text>
    <text x="344" y="125" class="label">chrome below ⇆</text>
    <!-- the light the reticle lets through, to the field on the wafer -->
    <polygon points="196,122 304,122 {kind === 'immersion' ? '272,313 228,313' : '268,313 232,313'}" class="beam" />
    {#each gaps as [a, b], i (i)}
      {@const x = 182 + ((a + b) / 2) * 136}
      <path d="M{f1(x)},122 L{f1(232 + ((a + b) / 2) * 36)},313" class="ray" />
    {/each}
    <!-- the projection lens -->
    <rect x="192" y="130" width="116" height="168" rx="10" class="column" />
    {#each [[142, 50], [158, 56], [174, 46], [190, 36], [206, 30], [222, 38], [238, 50], [254, 52], [270, 42], [286, 28]] as [y, rx] (y)}
      <ellipse cx="250" cy={y} {rx} ry="5.5" class="lens" />
    {/each}
    <text x="320" y="196" class="label">projection lens: many</text>
    <text x="320" y="207" class="label">elements, 4× smaller</text>
    {#if kind === "immersion"}
      <rect x="214" y="294" width="18" height="10" rx="2" class="metal" />
      <rect x="268" y="294" width="18" height="10" rx="2" class="metal" />
      <rect x="230" y="298" width="40" height="15" rx="3" class="water" />
      <text x="320" y="300" class="label">hood holding water</text>
      <text x="320" y="311" class="label">between lens and wafer</text>
    {/if}
    <!-- the alignment sensor -->
    <rect x="326" y="262" width="34" height="18" rx="3" class="box" />
    <path d="M334,280 L300,313" class="align" />
    <text x="366" y="274" class="label">alignment sensor</text>
    <!-- the wafer on its stage, measured by interferometer -->
    <g class="scan-wafer">
      <rect x="166" y="324" width="168" height="16" rx="3" class="metal" />
      <rect x="334" y="322" width="4" height="20" class="stage-mirror" />
      <rect x="176" y="316" width="148" height="8" class="wafer" />
      <rect x="176" y="314" width="148" height="2.4" class="resist" />
    </g>
    <circle cx="250" cy="314" r="10" fill="url(#litho-glow)" class="pulse" />
    <rect x="410" y="326" width="40" height="14" rx="3" class="box" />
    <path d="M410,333 H340" class="laser-beam" />
    <text x="456" y="337" class="label">interferometer</text>
    <text x="250" y="356" class="label" text-anchor="middle">wafer stage ⇆</text>
  {:else if kind === "contact"}
    <!-- the lamp house: an ellipsoidal mirror round the lamp, a lens making the light parallel -->
    <rect x="186" y="6" width="128" height="58" rx="8" class="box" />
    <path d="M206,16 Q250,78 294,16" class="mirror thick" />
    <circle cx="250" cy="32" r="12" fill="url(#litho-hot)" class="pulse" />
    <text x="320" y="22" class="label">mercury lamp,</text>
    <text x="320" y="33" class="label">ellipsoidal mirror</text>
    <!-- the lamp stays on; the shutter under the lens lets the light down to the mask -->
    <polygon points="250,40 160,96 340,96" class="beam" />
    <rect x="156" y="104" width="188" height="8" class="beam" />
    <rect x="156" y="114" width={shutterEdge - 156} height="112" class="beam" />
    {#each [172, 196, 220, 244, 268, 292, 316] as x (x)}
      <path d="M{x},104 V{x < shutterEdge ? 226 : 112}" class="ray" />
    {/each}
    <rect x={shutterEdge} y="111" width="188" height="4" rx="1" class="metal" />
    <text x="150" y="116" class="label" text-anchor="end">shutter</text>
    <ellipse cx="250" cy="100" rx="96" ry="7" class="lens" />
    <text x="352" y="104" class="label">lens: parallel light</text>
    <!-- the alignment microscope, over the marks while aligning, away while exposing -->
    {#each [178, 322] as x0 (x0)}
      {@const x = x0 + (x0 < 250 ? -scopeOut : scopeOut)}
      <g>
        <rect x={x - 9} y="150" width="18" height="50" rx="3" class="box" />
        <rect x={x - 6} y="200" width="12" height="12" rx="2" class="metal" />
        {#if contact.aligning}<path d="M{x},212 V228" class="align" />{/if}
      </g>
    {/each}
    <text x="80" y="160" class="label" text-anchor="middle">alignment</text>
    <text x="80" y="171" class="label" text-anchor="middle">microscope</text>
    <text x="250" y="306" class="label now" text-anchor="middle">{contact.doing}</text>
    <!-- mask on resist on wafer on chuck -->
    <rect x="140" y="228" width="220" height="10" rx="1.5" class="quartz" />
    {#each blocks as [a, b], i (i)}
      <rect x={150 + a * 200} y="238" width={(b - a) * 200} height="3" class="chrome" />
    {/each}
    <path d="M150,234 l6,0 m-3,-3 l0,6 M344,234 l6,0 m-3,-3 l0,6" class="mark" />
    <text x="368" y="236" class="label">mask on the resist,</text>
    <text x="368" y="247" class="label">the same size</text>
    <rect x="150" y="241" width="200" height="3" class="resist" />
    <rect x="150" y="244" width="200" height="10" class="wafer" />
    <rect x="130" y="254" width="240" height="18" rx="3" class="metal" />
    <text x="250" y="290" class="label" text-anchor="middle">wafer on its chuck</text>
  {:else if kind === "ebeam"}
    <rect x="176" y="6" width="148" height="294" rx="14" class="vacuum" />
    <text x="334" y="20" class="label">vacuum column</text>
    <!-- the gun -->
    <rect x="240" y="11" width="20" height="9" rx="2" class="metal" />
    <path d="M244,20 L250,32 L256,20 Z" class="metal" />
    <path d="M228,14 V37 H245 M272,14 V37 H255" class="yoke-line" />
    <text x="168" y="22" class="label" text-anchor="end">electron gun:</text>
    <text x="168" y="33" class="label" text-anchor="end">cathode, Wehnelt</text>
    <rect x="196" y="46" width="49" height="4" class="metal" />
    <rect x="255" y="46" width="49" height="4" class="metal" />
    <text x="168" y="51" class="label" text-anchor="end">anode</text>
    <!-- lenses: iron yokes round copper coils -->
    {#each [[66, 22, 46, 6], [150, 22, 42, 6], [220, 30, 50, 14]] as [y, h, w, gap] (y)}
      {#each lens(y, h, w, gap) as r, i (i)}
        <rect x={r.x} y={r.y} width={r.w} height={r.h} rx="3" class="yoke" />
        <rect x={r.x + 6} y={r.y + 5} width={r.w - 12} height={r.h - 10} fill="url(#litho-coil)" />
      {/each}
    {/each}
    <text x="168" y="80" class="label" text-anchor="end">condenser lens</text>
    <text x="168" y="164" class="label" text-anchor="end">second lens</text>
    <text x="168" y="238" class="label" text-anchor="end">objective lens</text>
    <!-- blanker, aperture, deflector, detector -->
    <path d="M243,100 V122 M257,100 V122" class="plate {beam.on ? '' : 'live'}" />
    <text x="168" y="114" class="label" text-anchor="end">blanker</text>
    <rect x="196" y="136" width="51" height="4" class="metal" />
    <rect x="253" y="136" width="51" height="4" class="metal" />
    <text x="168" y="141" class="label" text-anchor="end">aperture</text>
    <path d="M243,184 V208 M257,184 V208" class="plate live" />
    <text x="168" y="199" class="label" text-anchor="end">deflector</text>
    <rect x="212" y="264" width="30" height="3" class="metal" />
    <rect x="258" y="264" width="30" height="3" class="metal" />
    <text x="334" y="268" class="label">electron detector</text>
    <!-- the beam: to the blanker always; then down to the resist, or bent onto the aperture -->
    <path d={envelope(ENVELOPE)} class="e-beam" />
    {#if beam.on}
      <path d={envelope([[122, 1.6], [138, 1.2], [161, 4.5], [184, 1.4], [197, 1.4]])} class="e-beam" />
      <path d="M250,197 L{f1(250 + dx)},310" class="e-line" />
      <path d="M250,32 V197 L{f1(250 + dx)},310" class="ray" />
      <circle cx={250 + dx} cy="310" r="8" fill="url(#litho-glow)" />
    {:else}
      <path d="M250,104 Q251,122 266,136" class="e-line" />
      <circle cx="266" cy="136" r="6" fill="url(#litho-glow)" />
    {/if}
    <text x="334" y="104" class="label">{beam.on ? "beam on: writing a line" : "beam blanked: onto the aperture"}</text>
    <text x="334" y="116" class="label">{beam.on ? "" : "while the spot moves"}</text>
    <!-- the sample, what is written on it, the stage and its interferometer -->
    <rect x="150" y="309" width="200" height="4" class="resist" />
    <path d={footprint} class="written-line" />
    <rect x="150" y="313" width="200" height="9" class="wafer" />
    <rect x="140" y="322" width="220" height="16" rx="3" class="metal" />
    <rect x="360" y="320" width="4" height="20" class="stage-mirror" />
    <rect x="410" y="324" width="40" height="14" rx="3" class="box" />
    <path d="M410,331 H366" class="laser-beam" />
    <text x="456" y="335" class="label">interferometer</text>
    <text x="250" y="354" class="label" text-anchor="middle">resist on the wafer, its stage</text>
  {:else}
    <!-- the press: mould down, pressed and heated, lifted -->
    <g style="transform: translateY({pressDrop}px)">
      <rect x="160" y="30" width="200" height="40" rx="6" class="box" />
      <text x="260" y="54" class="label" text-anchor="middle">press head</text>
      <rect x="176" y="70" width="168" height="26" class="quartz" />
      {#each gaps as [a, b], i (i)}
        <rect x={182 + a * 156} y="96" width={(b - a) * 156} height="12" class="quartz" />
      {/each}
    </g>
    <text x="370" y="92" class="label">mould: the pattern in relief</text>
    <rect x="170" y="168" width="180" height="4" class="resist" />
    {#if relief}
      {#each blocks as [a, b], i (i)}
        <rect x={182 + a * 156} y="160" width={(b - a) * 156} height="8" class="resist" />
      {/each}
    {/if}
    <rect x="170" y="172" width="180" height="12" class="wafer" />
    <rect x="160" y="184" width="200" height="18" rx="3" class={heated ? "heater" : "metal"} />
    <text x="370" y="172" class="label">resist on the wafer</text>
    <text x="370" y="198" class="label">{heated ? "heated: the resist flows" : "chuck"}</text>
    <text x="260" y="236" class="label" text-anchor="middle">{pressPhase < 0.2 ? "the mould comes down" : pressPhase < 0.5 ? "pressed in, the resist softened by heat" : pressPhase < 0.68 ? "lifted" : "the resist keeps the mould's relief"}</text>
  {/if}
</svg>

<style>
  .label {
    font-size: 10px;
    fill: currentColor;
    fill-opacity: 0.68;
  }
  .label.tiny {
    font-size: 8.5px;
  }
  .vacuum {
    fill: color-mix(in oklab, currentColor 2.5%, transparent);
    stroke: currentColor;
    stroke-opacity: 0.25;
    stroke-dasharray: 5 5;
  }
  .box {
    fill: var(--color-base-100);
    stroke: currentColor;
    stroke-opacity: 0.35;
  }
  .window {
    fill: #7dd3fc;
    fill-opacity: 0.6;
  }
  .metal {
    fill: color-mix(in oklab, currentColor 24%, transparent);
  }
  .yoke {
    fill: color-mix(in oklab, currentColor 30%, transparent);
  }
  .yoke-line {
    fill: none;
    stroke: color-mix(in oklab, currentColor 45%, transparent);
    stroke-width: 3;
  }
  .column {
    fill: color-mix(in oklab, currentColor 4%, transparent);
    stroke: currentColor;
    stroke-opacity: 0.2;
  }
  .mirror {
    fill: none;
    stroke: color-mix(in oklab, currentColor 65%, #94a3b8);
    stroke-width: 3.5;
    stroke-linecap: round;
  }
  .mirror.thick {
    stroke-width: 4;
  }
  .mirror.facets {
    stroke-dasharray: 3.2 1.2;
    stroke-linecap: butt;
    stroke-width: 4.5;
  }
  .lens {
    fill: color-mix(in oklab, #7dd3fc 30%, transparent);
    stroke: #38bdf8;
    stroke-opacity: 0.7;
  }
  .plate {
    stroke: currentColor;
    stroke-opacity: 0.55;
    stroke-width: 2.5;
  }
  .plate.live {
    stroke: #f59e0b;
    stroke-opacity: 0.9;
  }
  .quartz {
    fill: color-mix(in oklab, #bae6fd 45%, transparent);
    stroke: #7dd3fc;
    stroke-opacity: 0.6;
  }
  .chrome,
  .absorber {
    fill: color-mix(in oklab, currentColor 75%, transparent);
  }
  .ml-body {
    fill: color-mix(in oklab, currentColor 18%, transparent);
  }
  .mo {
    fill: #a78bfa;
  }
  .wafer {
    fill: #94a3b8;
  }
  .resist {
    fill: #f472b6;
  }
  .water {
    fill: color-mix(in oklab, #38bdf8 35%, transparent);
    stroke: #0ea5e9;
    stroke-opacity: 0.6;
  }
  .heater {
    fill: #f97316;
    fill-opacity: 0.55;
  }
  .stage-mirror {
    fill: #cbd5e1;
    stroke: currentColor;
    stroke-opacity: 0.3;
  }
  .laser-beam {
    stroke: #ef4444;
    stroke-width: 1.4;
    stroke-dasharray: 4 3;
    animation: flow 0.6s linear infinite;
  }
  .align {
    stroke: #22c55e;
    stroke-width: 1.4;
    stroke-dasharray: 3 3;
  }
  .mark {
    stroke: #22c55e;
    stroke-width: 1.2;
    fill: none;
  }
  .beam {
    fill: var(--light);
    fill-opacity: 0.14;
  }
  .beam-line {
    fill: none;
    stroke: var(--light);
    stroke-opacity: 0.32;
    stroke-width: 5;
    stroke-linejoin: round;
  }
  .e-beam {
    fill: var(--light);
    fill-opacity: 0.45;
  }
  .e-line {
    fill: none;
    stroke: var(--light);
    stroke-width: 2.4;
    stroke-opacity: 0.7;
  }
  .written-line {
    stroke: #0ea5e9;
    stroke-width: 4;
  }
  .pump {
    stroke: #ef4444;
    stroke-width: 2;
    stroke-dasharray: 6 6;
    animation: flow 0.6s linear infinite;
  }
  .ray {
    fill: none;
    stroke: var(--light);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-dasharray: 3 11;
    animation: flow 0.9s linear infinite;
  }
  .ray.thin {
    stroke-width: 1.1;
    stroke-opacity: 0.7;
  }
  .tin {
    fill: #a3a3a3;
  }
  .pulse {
    animation: pulse 0.35s ease-in-out infinite alternate;
    transform-box: fill-box;
    transform-origin: center;
  }
  .scan-mask {
    animation: scan 3s ease-in-out infinite alternate;
  }
  .scan-wafer {
    animation: scan-back 3s ease-in-out infinite alternate;
  }
  .paused * {
    animation-play-state: paused !important;
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -14;
    }
  }
  @keyframes fall {
    from {
      transform: translateY(0);
      opacity: 1;
    }
    to {
      transform: translateY(86px);
      opacity: 0.3;
    }
  }
  @keyframes pulse {
    from {
      transform: scale(0.75);
    }
    to {
      transform: scale(1.15);
    }
  }
  /* the mask moves 4 times as far as the wafer, the other way */
  @keyframes scan {
    from {
      transform: translateX(-16px);
    }
    to {
      transform: translateX(16px);
    }
  }
  @keyframes scan-back {
    from {
      transform: translateX(4px);
    }
    to {
      transform: translateX(-4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .litho * {
      animation: none !important;
    }
  }
</style>
