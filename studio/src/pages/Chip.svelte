<script lang="ts">
  // The chip view: components placed on a canvas and wired port to port, external ports named,
  // the library checking the netlist as it changes, and the circuit simulated by the library's
  // sparse solve. Saved to the workspace's circuits/ as a chip file.
  import {
    ArrowRightLeft,
    ChevronDown,
    CircleAlert,
    CirclePlay,
    FileDown,
    FilePlus2,
    FileUp,
    FlipVertical2,
    Maximize,
    Redo2,
    RotateCw,
    Save,
    Trash2,
    Undo2,
    X,
    ZoomIn,
    ZoomOut,
  } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  import Glyph from "../components/Glyph.svelte";
  import SpectrumPlot from "../components/SpectrumPlot.svelte";
  import Tip from "../components/Tip.svelte";
  import UnitChip from "../components/UnitChip.svelte";
  import { ago, api, type Chip, type CircuitExample, type CircuitItem, type KindInfo, type Problem } from "../lib/api";
  import { app, go, toast } from "../lib/app.svelte";
  import {
    addInstance,
    change,
    dirty,
    allParts,
    editor,
    exportTouchstone,
    importTouchstone,
    kindOf,
    library,
    loadLibrary,
    openChipFile,
    partKey,
    redo,
    remember,
    saveChip,
    showChip,
    simulate,
    undo,
  } from "../lib/chip.svelte";
  import {
    bounds,
    emptyChip,
    externalPin,
    freshPortName,
    nameProblem,
    pinsOf,
    removeInstance,
    renameInstance,
    snap,
    sweepProblem,
    tag,
    TAG,
    tagLabel,
    usedPorts,
    wirePath,
    type Quantity,
    type WorldPin,
  } from "../lib/circuit";
  import { label } from "../lib/plot";
  import { isMicrometres, len, lenUnit, showIn, shown, storeIn, stored, unitOf } from "../lib/units";

  loadLibrary();

  let examples = $state<CircuitExample[]>([]);
  let files = $state<CircuitItem[]>([]);
  api
    .circuitExamples()
    .then((e) => (examples = e))
    .catch((e) => toast(String(e), "error"));
  $effect(() => {
    void app.workspaceVersion;
    api.circuits().then((c) => (files = c)).catch(() => {});
  });

  const chip = $derived(editor.chip);
  /** Every part by its key; an instance's is `partKey(inst)`. */
  const kinds = $derived(allParts());
  const categories = $derived([...new Set(library.kinds.map((k) => k.category))]);

  // ---- the canvas's geometry ----

  let W = $state(800);
  let H = $state(600);
  let svg: SVGSVGElement | undefined = $state();
  const view = $derived(editor.view);

  /** Every pin on the canvas by its ref: instances' "inst.port" and external ports' "#k". */
  const pins = $derived.by(() => {
    const all = new Map<string, WorldPin>();
    for (const inst of chip.instance) for (const p of pinsOf(inst, kinds.get(partKey(inst)))) all.set(p.ref, p);
    chip.port.forEach((e, k) => all.set(`#${k}`, externalPin(e, k)));
    return all;
  });
  const used = $derived(usedPorts(chip));

  const toWorld = (sx: number, sy: number): [number, number] => [view.cx + (sx - W / 2) / view.k, view.cy + (sy - H / 2) / view.k];

  function local(e: { clientX: number; clientY: number }): [number, number] {
    const r = svg!.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  /** The pin nearest the world point (x, y), within reach, other than `except`. */
  function pinAt(x: number, y: number, except?: string): WorldPin | null {
    let best: WorldPin | null = null;
    let reach = Math.max(7, 12 / view.k);
    for (const p of pins.values()) {
      if (p.ref === except) continue;
      const d = Math.hypot(p.x - x, p.y - y);
      if (d < reach) {
        reach = d;
        best = p;
      }
    }
    return best;
  }

  function fit() {
    const b = bounds(chip, kinds);
    if (!b) {
      editor.view = { cx: 0, cy: 0, k: 1.6 };
      return;
    }
    const [x0, y0, x1, y1] = b;
    const k = Math.min((W - 80) / Math.max(x1 - x0, 1), (H - 80) / Math.max(y1 - y0, 1));
    editor.view = { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2, k: Math.min(3, Math.max(0.3, k)) };
  }

  let fitted = -1;
  $effect(() => {
    // fit once the canvas has its size, whenever a chip is opened
    if (editor.fit !== fitted && W > 100 && H > 100 && library.loaded) {
      fitted = editor.fit;
      fit();
    }
  });

  function zoom(factor: number, sx = W / 2, sy = H / 2) {
    const [wx, wy] = toWorld(sx, sy);
    const k = Math.min(6, Math.max(0.2, view.k * factor));
    editor.view = { cx: wx - (sx - W / 2) / k, cy: wy - (sy - H / 2) / k, k };
  }

  // ---- the library's checks, as the chip changes ----

  $effect(() => {
    const snapshot = JSON.parse(JSON.stringify(chip)) as Chip;
    const timer = setTimeout(() => {
      api
        .circuitCheck(snapshot)
        .then((p) => {
          editor.problems = p;
        })
        .catch(() => {});
    }, 90);
    return () => clearTimeout(timer);
  });

  const mistakes = $derived(editor.problems.filter((p) => !p.dangling));
  const dangling = $derived(editor.problems.filter((p) => p.dangling));
  const badInstances = $derived(new Set(mistakes.map((p) => p.instance).filter(Boolean)));
  const badWires = $derived(new Set(mistakes.map((p) => p.connection).filter((c) => c !== undefined)));
  const badPorts = $derived(new Set(mistakes.map((p) => p.external).filter((c) => c !== undefined)));

  // the results follow the chip: simulated again a moment after it changes, while shown
  const stale = $derived(!!editor.sim && editor.simulated !== JSON.stringify(chip));
  $effect(() => {
    if (!stale || editor.simulating || editor.problems.length || sweepProblem(chip.sweep)) return;
    const timer = setTimeout(simulate, 350);
    return () => clearTimeout(timer);
  });

  // ---- pointer gestures ----

  type Drag =
    | { kind: "pan"; sx: number; sy: number; cx: number; cy: number }
    | { kind: "move"; what: "instance" | "port"; id: string | number; dx: number; dy: number; sx: number; sy: number; moved: boolean }
    | { kind: "wire"; from: WorldPin; x: number; y: number };

  let drag = $state<Drag | null>(null);
  /** A part being dragged in from the palette: a library id, or "#port" for an external port. */
  let placing = $state<{ part: string; sx: number; sy: number; x: number; y: number; over: boolean; moved: boolean } | null>(null);
  /** A connection just refused, drawn red for a moment. */
  let refused = $state<{ a: WorldPin; b: { x: number; y: number } } | null>(null);
  let hoverPin = $state<string | null>(null);

  function down(e: PointerEvent) {
    if (e.button === 2) return;
    (document.activeElement as HTMLElement | null)?.blur?.();
    const [sx, sy] = local(e);
    const [x, y] = toWorld(sx, sy);
    const target = e.target as Element;
    if (e.button === 1) {
      drag = { kind: "pan", sx, sy, cx: view.cx, cy: view.cy };
    } else {
      const pin = pinAt(x, y);
      const body = target.closest("[data-instance], [data-port], [data-wire], [data-exposure]") as HTMLElement | null;
      if (pin && (!body || body.dataset.wire === undefined)) {
        drag = { kind: "wire", from: pin, x, y };
      } else if (body?.dataset.instance) {
        const inst = chip.instance.find((i) => i.name === body.dataset.instance)!;
        editor.selection = { kind: "instance", name: inst.name };
        drag = { kind: "move", what: "instance", id: inst.name, dx: inst.x - x, dy: inst.y - y, sx, sy, moved: false };
      } else if (body?.dataset.port) {
        const k = Number(body.dataset.port);
        const p = chip.port[k];
        editor.selection = { kind: "port", index: k };
        drag = { kind: "move", what: "port", id: k, dx: p.x - x, dy: p.y - y, sx, sy, moved: false };
      } else if (body?.dataset.wire) {
        editor.selection = { kind: "wire", index: Number(body.dataset.wire) };
      } else if (body?.dataset.exposure) {
        editor.selection = { kind: "exposure", index: Number(body.dataset.exposure) };
      } else {
        editor.selection = null;
        drag = { kind: "pan", sx, sy, cx: view.cx, cy: view.cy };
      }
    }
    if (drag) svg?.setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    const [sx, sy] = local(e);
    const [x, y] = toWorld(sx, sy);
    if (!drag) {
      hoverPin = pinAt(x, y)?.ref ?? null;
      return;
    }
    if (drag.kind === "pan") {
      editor.view = { ...view, cx: drag.cx - (sx - drag.sx) / view.k, cy: drag.cy - (sy - drag.sy) / view.k };
    } else if (drag.kind === "move") {
      if (!drag.moved) {
        if (Math.hypot(sx - drag.sx, sy - drag.sy) < 3) return;
        remember();
        drag.moved = true;
      }
      const nx = snap(x + drag.dx);
      const ny = snap(y + drag.dy);
      if (drag.what === "instance") {
        const id = drag.id;
        const inst = chip.instance.find((i) => i.name === id);
        if (inst && (inst.x !== nx || inst.y !== ny)) Object.assign(inst, { x: nx, y: ny });
      } else {
        const p = chip.port[drag.id as number];
        if (p && (p.x !== nx || p.y !== ny)) Object.assign(p, { x: nx, y: ny });
      }
    } else {
      drag.x = x;
      drag.y = y;
      hoverPin = pinAt(x, y, drag.from.ref)?.ref ?? null;
    }
  }

  function up(e: PointerEvent) {
    const d = drag;
    drag = null;
    if (svg?.hasPointerCapture(e.pointerId)) svg.releasePointerCapture(e.pointerId);
    if (d?.kind !== "wire") return;
    const [x, y] = toWorld(...local(e));
    const to = pinAt(x, y, d.from.ref);
    if (to) {
      connect(d.from, to);
    } else if (!d.from.ref.startsWith("#") && Math.hypot(x - d.from.x, y - d.from.y) > 16) {
      // dropped on the canvas: an external port there, exposing the pin
      if (used.has(d.from.ref)) {
        refuse(d.from, { x, y }, `${d.from.ref} is already wired: delete its wire first`);
        return;
      }
      const left = x < d.from.x;
      change((c) => {
        const name = freshPortName(c, left);
        const px = snap(x + (left ? TAG : -TAG));
        c.port.push({ name, at: d.from.ref, x: px, y: snap(y), rotation: left ? 0 : 180 });
        editor.selection = { kind: "port", index: c.port.length - 1 };
      });
    }
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const [sx, sy] = local(e);
    zoom(Math.exp(-e.deltaY * 0.0015), sx, sy);
  }

  function refuse(a: WorldPin, b: { x: number; y: number }, reason: string) {
    refused = { a, b };
    setTimeout(() => (refused = null), 1400);
    toast(`Not connected: ${reason}`, "warning");
  }

  /** Wires two pins, if the library allows it; says why not otherwise. */
  async function connect(a: WorldPin, b: WorldPin) {
    const ea = a.ref.startsWith("#");
    const eb = b.ref.startsWith("#");
    if (ea && eb) return refuse(a, b, "two external ports can't be wired to each other: each exposes a component's port");
    const candidate = JSON.parse(JSON.stringify(chip)) as Chip;
    let blame: (p: Problem) => boolean;
    let apply: (c: Chip) => void;
    if (ea || eb) {
      const k = Number((ea ? a : b).ref.slice(1));
      const port = (ea ? b : a).ref;
      const ext = candidate.port[k];
      if (ext.at) return refuse(a, b, `${ext.name} already exposes ${ext.at}: delete that wire first`);
      ext.at = port;
      blame = (p) => p.external === k;
      apply = (c) => (c.port[k].at = port);
    } else {
      candidate.connections.push([a.ref, b.ref]);
      const index = candidate.connections.length - 1;
      blame = (p) => p.connection === index;
      apply = (c) => c.connections.push([a.ref, b.ref]);
    }
    try {
      const problem = (await api.circuitCheck(candidate)).find((p) => !p.dangling && blame(p));
      if (problem) return refuse(a, b, problem.message);
    } catch (e) {
      return refuse(a, b, String(e));
    }
    change(apply);
  }

  // ---- placing parts from the palette ----

  function grab(e: PointerEvent, part: string) {
    if (e.button !== 0) return;
    e.preventDefault();
    placing = { part, sx: e.clientX, sy: e.clientY, x: 0, y: 0, over: false, moved: false };
  }

  function placeMove(e: PointerEvent) {
    if (!placing || !svg) return;
    if (!placing.moved && Math.hypot(e.clientX - placing.sx, e.clientY - placing.sy) > 4) placing.moved = true;
    const r = svg.getBoundingClientRect();
    placing.over = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    const [x, y] = toWorld(e.clientX - r.left, e.clientY - r.top);
    placing.x = snap(x);
    placing.y = snap(y);
  }

  function placeUp() {
    const p = placing;
    placing = null;
    if (!p || (p.moved && !p.over)) return;
    const at = p.moved ? { x: p.x, y: p.y } : { x: snap(view.cx), y: snap(view.cy) };
    if (p.part === "#port") {
      change((c) => {
        const input = at.x <= view.cx;
        c.port.push({ name: freshPortName(c, input), at: "", x: at.x, y: at.y, rotation: input ? 0 : 180 });
        editor.selection = { kind: "port", index: c.port.length - 1 };
      });
    } else {
      addInstance(p.part, at.x, at.y);
    }
  }

  // ---- editing ----

  function deleteSelection() {
    const s = editor.selection;
    if (!s) return;
    change((c) => {
      if (s.kind === "instance") removeInstance(c, s.name);
      else if (s.kind === "port") c.port.splice(s.index, 1);
      else if (s.kind === "wire") c.connections.splice(s.index, 1);
      else c.port[s.index].at = "";
    });
    editor.selection = null;
  }

  function rotateSelection(by: number) {
    const s = editor.selection;
    if (s?.kind === "instance") {
      change((c) => {
        const inst = c.instance.find((i) => i.name === s.name)!;
        inst.rotation = ((((inst.rotation ?? 0) + by) % 360) + 360) % 360;
      });
    } else if (s?.kind === "port") {
      change((c) => (c.port[s.index].rotation = ((((c.port[s.index].rotation ?? 0) + by) % 360) + 360) % 360));
    }
  }

  function mirrorSelection() {
    const s = editor.selection;
    if (s?.kind !== "instance") return;
    change((c) => {
      const inst = c.instance.find((i) => i.name === s.name)!;
      inst.mirror = !inst.mirror;
    });
  }

  function open(c: Chip, path: string | null) {
    (document.activeElement as HTMLElement | null)?.blur?.();
    showChip(c, path);
  }

  const openFile = openChipFile;

  async function remove(item: CircuitItem) {
    if (!(await ask(`Delete circuits/${item.name}.toml?`, { title: "Delete the circuit?", kind: "warning" }))) return;
    try {
      await api.deleteCircuit(item.path);
      if (editor.path === item.path) editor.path = null;
      app.workspaceVersion++;
      toast(`Deleted ${item.name}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function keys(e: KeyboardEvent) {
    const typing = (e.target as HTMLElement).closest?.("input, textarea, select, [contenteditable]");
    const mod = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (mod && key === "s") {
      e.preventDefault();
      saveChip();
    } else if (mod && e.key === "Enter") {
      e.preventDefault();
      simulate();
    } else if (typing || app.palette) {
      return;
    } else if (mod && key === "z" && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (mod && (key === "y" || (key === "z" && e.shiftKey))) {
      e.preventDefault();
      redo();
    } else if (mod) {
      return;
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      deleteSelection();
    } else if (key === "r") {
      rotateSelection(e.shiftKey ? -90 : 90);
    } else if (key === "m") {
      mirrorSelection();
    } else if (key === "f") {
      fit();
    } else if (e.key === "Escape") {
      drag = null;
      placing = null;
      editor.selection = null;
    }
  }

  onMount(() => {
    // an external wheel listener, so the page doesn't scroll as the canvas zooms
    const el = svg;
    el?.addEventListener("wheel", wheel, { passive: false });
    return () => el?.removeEventListener("wheel", wheel);
  });

  // ---- the inspector ----

  const selectedInstance = $derived(editor.selection?.kind === "instance" ? chip.instance.find((i) => i.name === (editor.selection as { name: string }).name) : undefined);
  const selectedKind = $derived(selectedInstance ? kinds.get(partKey(selectedInstance)) : undefined);
  const selectedPort = $derived(editor.selection?.kind === "port" ? chip.port[editor.selection.index] : undefined);
  let renaming = $state("");
  $effect(() => {
    renaming = selectedInstance?.name ?? selectedPort?.name ?? "";
  });

  function rename() {
    const to = renaming.trim();
    if (selectedInstance && to !== selectedInstance.name) {
      const bad = nameProblem(to, chip.instance.map((i) => i.name));
      if (bad) return toast(bad, "warning");
      const from = selectedInstance.name;
      change((c) => renameInstance(c, from, to));
      editor.selection = { kind: "instance", name: to };
    } else if (selectedPort && to !== selectedPort.name) {
      const bad = nameProblem(to, chip.port.map((p) => p.name));
      if (bad) return toast(bad, "warning");
      const k = (editor.selection as { index: number }).index;
      change((c) => (c.port[k].name = to));
    }
  }

  /** A measured component's wavelengths, "1.5 to 1.6 µm", in the unit shown. */
  const range = (v: [number, number] | null | undefined) => (v ? `${len(v[0])} to ${lenUnit(v[1])}` : "?");

  function setValue(name: string, k: KindInfo, i: number, v: number, record: boolean) {
    if (!Number.isFinite(v)) return;
    const p = k.parameters[i];
    const edit = (c: Chip) => {
      const inst = c.instance.find((x) => x.name === name);
      if (inst) inst.values = { ...inst.values, [p.name]: Math.min(p.max, Math.max(p.min, v)) };
    };
    if (record) change(edit);
    else edit(chip);
  }

  function sliderMax(p: KindInfo["parameters"][number]): number {
    if (p.max - p.min <= 100) return p.max;
    return Math.min(p.max, Math.max(p.min + 1, 4 * Math.abs(p.default) || 1));
  }

  /** What a port of the selected instance is wired to. */
  function partner(ref: string): string | null {
    for (const [a, b] of chip.connections) {
      if (a === ref) return b;
      if (b === ref) return a;
    }
    const e = chip.port.find((p) => p.at === ref);
    return e ? `external port ${e.name}` : null;
  }

  function point(p: Problem) {
    if (p.connection !== undefined) editor.selection = { kind: "wire", index: p.connection };
    else if (p.external !== undefined) editor.selection = { kind: "port", index: p.external };
    else if (p.instance && chip.instance.some((i) => i.name === p.instance)) editor.selection = { kind: "instance", name: p.instance };
  }

  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

  let input = $state(0);
  let quantity = $state<Quantity>("power");
  let resultsOpen = $state(true);

  function runSimulation() {
    resultsOpen = true;
    simulate();
  }

  const wires = $derived(
    chip.connections.map(([a, b], i) => ({ i, a: pins.get(a), b: pins.get(b) })).filter((w) => w.a && w.b) as { i: number; a: WorldPin; b: WorldPin }[],
  );
  const exposures = $derived(
    chip.port.map((p, i) => ({ i, a: pins.get(`#${i}`), b: p.at ? pins.get(p.at) : undefined })).filter((w) => w.a && w.b) as { i: number; a: WorldPin; b: WorldPin }[],
  );
  const minor = $derived(view.k >= 0.9);
</script>

<svelte:window onkeydown={keys} onpointermove={placeMove} onpointerup={placeUp} />

<div class="grid h-full grid-cols-[16rem_minmax(0,1fr)_19rem] max-xl:grid-cols-[13rem_minmax(0,1fr)_16rem]">
  <!-- the parts and the circuits -->
  <aside class="flex min-h-0 flex-col border-r border-base-content/8 bg-base-100/40">
    <div class="flex gap-2 p-3">
      <div class="dropdown flex-1">
        <div tabindex="0" role="button" class="btn btn-primary btn-sm w-full gap-1.5"><FilePlus2 size={15} /> New <ChevronDown size={14} /></div>
        <ul tabindex="-1" class="dropdown-content menu z-30 mt-1 w-72 rounded-box border border-base-content/10 bg-base-100 p-2 shadow-xl">
          <li><button onclick={() => open(emptyChip(), null)}><span class="font-medium">An empty chip</span></button></li>
          <li class="menu-title pt-2">From an example</li>
          {#each examples as ex (ex.file)}
            <li>
              <button class="flex flex-col items-start gap-0" onclick={() => open(ex.chip, null)}>
                <span class="font-medium">{ex.chip.name}</span><span class="text-xs faint">{ex.chip.about}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
      <p class="px-2 pt-1 pb-1 text-[10.5px] font-semibold tracking-wider uppercase faint">Parts: drag onto the chip</p>
      {#each categories as category (category)}
        <p class="px-2 pt-2 text-[11px] faint">{category}</p>
        {#each library.kinds.filter((k) => k.category === category) as k (k.id)}
          <button
            class="flex w-full cursor-grab items-center gap-2 rounded-lg px-2 py-1 text-left text-sm hover:bg-base-content/5 active:cursor-grabbing"
            onpointerdown={(e) => grab(e, k.id)}
            title="{k.title}{k.variant ? ` (${k.variant})` : ''}: drag it onto the chip, or click to place it in the middle"
          >
            <svg width="36" height="22" viewBox="{-k.symbol.width / 2 - 4} {-k.symbol.height / 2 - 4} {k.symbol.width + 8} {k.symbol.height + 8}" class="shrink-0">
              <Glyph symbol={k.symbol} />
            </svg>
            <span class="min-w-0 truncate">{k.title}{#if k.variant && library.kinds.some((o) => o !== k && o.title === k.title)}<span class="ml-1 text-xs faint">{k.variant}</span>{/if}</span>
          </button>
        {/each}
      {/each}
      <p class="px-2 pt-2 text-[11px] faint">Measured</p>
      {#each library.measured as k (partKey(k))}
        <button
          class="flex w-full cursor-grab items-center gap-2 rounded-lg px-2 py-1 text-left text-sm hover:bg-base-content/5 active:cursor-grabbing"
          onpointerdown={(e) => grab(e, partKey(k))}
          title="{k.title}: {k.ports.length} ports, measured from {range(k.provenance.validity)}. Drag it onto the chip."
        >
          <svg width="36" height="22" viewBox="{-k.symbol.width / 2 - 4} {-k.symbol.height / 2 - 4} {k.symbol.width + 8} {k.symbol.height + 8}" class="shrink-0">
            <Glyph symbol={k.symbol} />
          </svg>
          <span class="min-w-0 truncate">{k.title}</span>
        </button>
      {/each}
      <button class="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left text-sm text-primary hover:bg-base-content/5" onclick={importTouchstone} title="Read S-parameters from a Touchstone file (.sNp) as a measured component">
        <FileUp size={15} class="mx-2.5 shrink-0" /> Import Touchstone…
      </button>
      <p class="px-2 pt-2 text-[11px] faint">The circuit's own</p>
      <button
        class="flex w-full cursor-grab items-center gap-2 rounded-lg px-2 py-1 text-left text-sm hover:bg-base-content/5"
        onpointerdown={(e) => grab(e, "#port")}
        title="An external port: drag it onto the chip and wire it to a component's port. Or drag a wire from a port to empty space."
      >
        <svg width="36" height="22" viewBox="-24 -12 48 24" class="shrink-0"><path d="M -20 -8 L 10 -8 L 19 0 L 10 8 L -20 8 Z" class="fill-accent/20 stroke-accent" stroke-width="1.4" /></svg>
        <span>External port</span>
      </button>

      <p class="px-2 pt-4 pb-1 text-[10.5px] font-semibold tracking-wider uppercase faint">In the workspace</p>
      {#each files as f (f.path)}
        <div class="group flex items-center rounded-lg {editor.path === f.path ? 'bg-primary/12' : 'hover:bg-base-content/4'}">
          <button class="min-w-0 flex-1 px-2 py-1.5 text-left" onclick={() => openFile(f.path)} title={f.about}>
            <span class="block truncate text-sm font-medium">{f.name}</span>
            <span class="block truncate text-xs faint">{plural(f.instances, "part")} · {ago(f.modified)}</span>
          </button>
          <div class="tooltip tooltip-left" data-tip="Delete">
            <button class="btn btn-ghost btn-xs btn-square opacity-0 group-hover:opacity-100" aria-label="Delete {f.name}" onclick={() => remove(f)}><Trash2 size={13} /></button>
          </div>
        </div>
      {:else}
        <p class="px-2 text-xs faint">None yet: Ctrl+S saves this one to circuits/.</p>
      {/each}
    </div>
  </aside>

  <!-- the canvas -->
  <div class="flex min-h-0 min-w-0 flex-col">
    <!-- Save and Simulate stay in view; too narrow for the rest, the tools beside them scroll
         sideways (so their hints are the window's own: a drawn one would be clipped) -->
    <div class="flex items-center gap-1 border-b border-base-content/8 bg-base-100/60 px-3 py-1.5 whitespace-nowrap">
      <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto [scrollbar-width:thin] [&>*]:shrink-0">
      <div title="Undo (Ctrl+Z)">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Undo" disabled={!editor.undoable} onclick={undo}><Undo2 size={16} /></button>
      </div>
      <div title="Redo (Ctrl+Y)">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Redo" disabled={!editor.redoable} onclick={redo}><Redo2 size={16} /></button>
      </div>
      <span class="mx-1 h-5 border-l border-base-content/10"></span>
      <div title="Rotate (R; Shift+R the other way)">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Rotate" disabled={editor.selection?.kind !== "instance" && editor.selection?.kind !== "port"} onclick={() => rotateSelection(90)}><RotateCw size={16} /></button>
      </div>
      <div title="Mirror top to bottom (M)">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Mirror" disabled={editor.selection?.kind !== "instance"} onclick={mirrorSelection}><FlipVertical2 size={16} /></button>
      </div>
      <div title="Delete (Del)">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Delete" disabled={!editor.selection} onclick={deleteSelection}><Trash2 size={16} /></button>
      </div>
      <span class="mx-1 h-5 border-l border-base-content/10"></span>
      <div title="Zoom out">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Zoom out" onclick={() => zoom(1 / 1.25)}><ZoomOut size={16} /></button>
      </div>
      <span class="w-12 text-center text-xs faint num">{Math.round(view.k * 62.5)}%</span>
      <div title="Zoom in">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Zoom in" onclick={() => zoom(1.25)}><ZoomIn size={16} /></button>
      </div>
      <div title="Fit the chip in view (F)">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Fit" onclick={fit}><Maximize size={16} /></button>
      </div>
      <span class="min-w-2 flex-1"></span>
      {#if mistakes.length}
        <span class="badge badge-sm badge-error badge-soft gap-1" title={mistakes.map((p) => p.message).join("\n")}><CircleAlert size={12} /> {mistakes.length} {mistakes.length === 1 ? "problem" : "problems"}</span>
      {/if}
      {#if dangling.length}
        <span class="badge badge-sm badge-warning badge-soft gap-1" title={dangling.map((p) => p.port).join(", ")}>{dangling.length} open {dangling.length === 1 ? "port" : "ports"}</span>
      {/if}
      {#if !editor.problems.length && chip.instance.length}
        <span class="badge badge-sm badge-success badge-soft">complete</span>
      {/if}
      </div>
      <span class="mx-1 h-5 shrink-0 border-l border-base-content/10"></span>
      <div class="shrink-0" title="Save to circuits/{chip.name}.toml (Ctrl+S)">
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={saveChip}><Save size={15} /> Save{#if dirty()}<span class="status status-warning"></span>{/if}</button>
      </div>
      <div class="shrink-0" title="The circuit's S-parameters over its wavelengths (Ctrl+Enter)">
        <button class="btn btn-primary btn-sm gap-1.5" disabled={editor.simulating} onclick={runSimulation}>
          {#if editor.simulating}<span class="loading loading-spinner loading-xs"></span>{:else}<CirclePlay size={15} />{/if} Simulate
        </button>
      </div>
    </div>

    <div class="relative min-h-0 flex-1 overflow-hidden bg-base-200" bind:clientWidth={W} bind:clientHeight={H}>
      <svg
        bind:this={svg}
        width={W}
        height={H}
        class="block outline-none select-none {drag?.kind === 'pan' ? 'cursor-grabbing' : hoverPin ? 'cursor-crosshair' : ''}"
        role="application"
        aria-label="The chip: drag parts here, and wires from port to port"
        onpointerdown={down}
        onpointermove={move}
        onpointerup={up}
        onpointerleave={() => (hoverPin = null)}
        oncontextmenu={(e) => e.preventDefault()}
      >
        <defs>
          <pattern id="chip-minor" width="10" height="10" x="-5" y="-5" patternUnits="userSpaceOnUse">
            <circle cx="5" cy="5" r={0.9 / Math.max(view.k, 0.5)} class="fill-base-content/20" />
          </pattern>
          <pattern id="chip-major" width="50" height="50" x="-25" y="-25" patternUnits="userSpaceOnUse">
            <circle cx="25" cy="25" r={1.6 / Math.max(view.k, 0.4)} class="fill-base-content/30" />
          </pattern>
        </defs>
        <g transform="translate({W / 2} {H / 2}) scale({view.k}) translate({-view.cx} {-view.cy})">
          {#each [minor ? "chip-minor" : null, "chip-major"].filter(Boolean) as id (id)}
            <rect x={view.cx - W / 2 / view.k - 50} y={view.cy - H / 2 / view.k - 50} width={W / view.k + 100} height={H / view.k + 100} fill="url(#{id})" />
          {/each}

          <!-- wires -->
          {#each wires as w (w.i)}
            {@const on = editor.selection?.kind === "wire" && editor.selection.index === w.i}
            <g data-wire={w.i} class="cursor-pointer">
              <path d={wirePath(w.a, w.b)} class="fill-none stroke-transparent" stroke-width={10 / view.k + 2} />
              <path
                d={wirePath(w.a, w.b)}
                class="fill-none {badWires.has(w.i) ? 'stroke-error' : on ? 'stroke-primary' : 'stroke-base-content/55'}"
                stroke-width={on ? 2.6 : 1.8}
                stroke-linecap="round"
              />
            </g>
          {/each}
          {#each exposures as w (w.i)}
            {@const on = editor.selection?.kind === "exposure" && editor.selection.index === w.i}
            <g data-exposure={w.i} class="cursor-pointer">
              <path d={wirePath(w.a, w.b)} class="fill-none stroke-transparent" stroke-width={10 / view.k + 2} />
              <path d={wirePath(w.a, w.b)} class="fill-none {badPorts.has(w.i) ? 'stroke-error' : on ? 'stroke-primary' : 'stroke-accent/80'}" stroke-width={on ? 2.6 : 1.8} stroke-linecap="round" />
            </g>
          {/each}

          <!-- instances -->
          {#each chip.instance as inst (inst.name)}
            {@const k = kinds.get(partKey(inst))}
            {@const on = editor.selection?.kind === "instance" && editor.selection.name === inst.name}
            {@const turned = ((inst.rotation ?? 0) / 90) % 2 === 1}
            {@const hh = k ? (turned ? k.symbol.width : k.symbol.height) : 30}
            <g data-instance={inst.name} class="cursor-move">
              {#if k}
                <g transform="translate({inst.x} {inst.y}) rotate({inst.rotation ?? 0}) scale(1 {inst.mirror ? -1 : 1})">
                  <Glyph symbol={k.symbol} selected={on} />
                  {#if badInstances.has(inst.name)}
                    <rect x={-k.symbol.width / 2 - 3} y={-k.symbol.height / 2 - 3} width={k.symbol.width + 6} height={k.symbol.height + 6} rx="7" class="fill-none stroke-error" stroke-width="1.4" stroke-dasharray="4 3" />
                  {/if}
                </g>
              {:else}
                <rect x={inst.x - 20} y={inst.y - 15} width="40" height="30" rx="5" class="fill-error/10 stroke-error" stroke-dasharray="4 3" />
                <text x={inst.x} y={inst.y} text-anchor="middle" dominant-baseline="central" class="fill-error text-[9px]">?</text>
              {/if}
              <text x={inst.x} y={inst.y - hh / 2 - 6} text-anchor="middle" class="text-[8px] font-medium {on ? 'fill-primary' : 'fill-base-content/75'}">{inst.name}</text>
            </g>
          {/each}

          <!-- external ports -->
          {#each chip.port as p, i (i)}
            {@const on = editor.selection?.kind === "port" && editor.selection.index === i}
            {@const rot = p.rotation ?? 0}
            <g data-port={i} class="cursor-move">
              <g transform="translate({p.x} {p.y}) rotate({rot})">
                <path
                  d={tag(p.name).d}
                  class="{on ? 'fill-primary/20 stroke-primary' : badPorts.has(i) || !p.at ? 'fill-warning/15 stroke-warning' : 'fill-accent/15 stroke-accent'}"
                  stroke-width={on ? 1.6 : 1.2}
                />
              </g>
              <text x={tagLabel(p)[0]} y={tagLabel(p)[1]} text-anchor="middle" dominant-baseline="central" class="text-[8px] font-semibold {on ? 'fill-primary' : 'fill-base-content/85'} num">{p.name}</text>
            </g>
          {/each}

          <!-- pins: open ones stand out -->
          {#each [...pins.values()] as pin (pin.ref)}
            {@const open = pin.ref.startsWith("#") ? !chip.port[Number(pin.ref.slice(1))]?.at : !used.has(pin.ref)}
            {@const hot = hoverPin === pin.ref}
            {#if open}
              <circle cx={pin.x} cy={pin.y} r={hot ? 4.6 : 3.4} class="fill-base-100 stroke-warning pointer-events-none" stroke-width="1.4" data-pin={pin.ref} />
              <circle cx={pin.x} cy={pin.y} r="1.2" class="fill-warning pointer-events-none" />
            {:else}
              <circle cx={pin.x} cy={pin.y} r={hot ? 3.6 : 2} class="fill-base-content/60 pointer-events-none" data-pin={pin.ref} />
            {/if}
            {#if hot}<circle cx={pin.x} cy={pin.y} r="7" class="fill-primary/15 stroke-primary pointer-events-none" stroke-width="1" />{/if}
          {/each}

          <!-- a wire being drawn, or one just refused -->
          {#if drag?.kind === "wire"}
            {@const to = hoverPin ? pins.get(hoverPin) : undefined}
            <path d={wirePath(drag.from, to ?? { x: drag.x, y: drag.y })} class="fill-none stroke-primary pointer-events-none" stroke-width="1.8" stroke-dasharray="5 4" />
          {/if}
          {#if refused}
            <path d={wirePath(refused.a, refused.b)} class="fill-none stroke-error pointer-events-none" stroke-width="2.2" stroke-dasharray="3 3" />
          {/if}

          <!-- a part being dragged in -->
          {#if placing?.moved && placing.over}
            {#if placing.part === "#port"}
              <path d="M -20 -8 L 10 -8 L 20 0 L 10 8 L -20 8 Z" transform="translate({placing.x} {placing.y})" class="fill-accent/20 stroke-accent pointer-events-none opacity-70" />
            {:else if kindOf(placing.part)}
              <g transform="translate({placing.x} {placing.y})" class="pointer-events-none opacity-60"><Glyph symbol={kindOf(placing.part)!.symbol} selected /></g>
            {/if}
          {/if}
        </g>
      </svg>

      {#if !chip.instance.length && !chip.port.length}
        <div class="pointer-events-none absolute inset-0 grid place-items-center">
          <div class="max-w-sm text-center">
            <p class="text-sm font-medium muted">An empty chip</p>
            <p class="mt-1 text-xs faint">Drag parts from the left onto it, then drag from a port to another to wire them. Drag from a port into empty space for an external port.</p>
          </div>
        </div>
      {/if}
      <div class="pointer-events-none absolute bottom-2 left-3 text-[11px] faint">
        Drag the background to pan, scroll to zoom · R rotates, M mirrors, Del deletes
      </div>
    </div>

    <!-- the results -->
    {#if resultsOpen && (editor.sim || editor.simProblem || editor.simulating)}
      <div class="h-[22rem] shrink-0 overflow-y-auto border-t border-base-content/8 bg-base-100 px-5 py-3">
        <div class="flex items-center gap-2">
          <h3 class="panel-title whitespace-nowrap">The circuit's S-parameters</h3>
          {#if editor.sim}
            <span class="truncate text-xs faint">
              {plural(editor.sim.instances, "component")}, {editor.sim.spectrum.wavelength_um.length} wavelengths, solved in {editor.sim.seconds < 1 ? `${(editor.sim.seconds * 1000).toFixed(0)} ms` : `${editor.sim.seconds.toFixed(2)} s`}
            </span>
            {#if stale}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
          {/if}
          <span class="flex-1"></span>
          {#if editor.sim}
            {@const sim = editor.sim}
            <div class="tooltip tooltip-left" data-tip="Save the S-parameters as a Touchstone file (.sNp)">
              <button
                class="btn btn-ghost btn-xs gap-1"
                onclick={() => exportTouchstone(chip.name, sim.spectrum.ports.length, (path) => api.circuitTouchstone($state.snapshot(chip) as Chip, path))}><FileDown size={13} /> Touchstone</button
              >
            </div>
          {/if}
          <div class="tooltip tooltip-left" data-tip="Hide the results">
            <button class="btn btn-ghost btn-xs btn-square" aria-label="Hide the results" onclick={() => (resultsOpen = false)}><X size={14} /></button>
          </div>
        </div>
        <div class="mt-1 flex flex-wrap items-center gap-2">
          {#if editor.sim}
            {@const c = editor.sim.checks}
            <span
              class="badge badge-sm {c.reciprocity < 1e-9 ? 'badge-success' : c.reciprocal ? 'badge-error' : 'badge-ghost'} badge-soft num"
              title="Reciprocity: the largest |S_qp − S_pq| over the sweep. Round-off for a circuit of reciprocal components."
              >max |S − Sᵀ| {label(c.reciprocity)}</span
            >
            <span
              class="badge badge-sm {c.largest_singular_value <= 1 + 1e-9 ? 'badge-success' : 'badge-error'} badge-soft num"
              title="Passivity: the largest singular value of S over the sweep. At most 1: the circuit makes no power."
              >σ<sub>max</sub> {c.largest_singular_value.toFixed(9)}</span
            >
            <span class="badge badge-sm badge-ghost num" title="Unitarity: the largest entry of |SᴴS − I| over the sweep. Zero for a lossless circuit; a lossy one shows its loss here."
              >max |SᴴS − I| {label(c.unitarity)}</span
            >
          {/if}
        </div>
        {#if editor.simProblem}
          <p class="mt-6 text-center text-sm text-warning">Can't simulate: {editor.simProblem}</p>
        {:else if editor.sim}
          <div class="mt-2"><SpectrumPlot data={editor.sim.spectrum} bind:input bind:quantity name={chip.name} height={250} /></div>
        {:else}
          <div class="grid h-56 place-items-center"><span class="loading loading-ring text-primary"></span></div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- the inspector -->
  <aside class="min-h-0 overflow-y-auto border-l border-base-content/8 bg-base-100/40 p-4">
    {#if selectedInstance}
      {@const inst = selectedInstance}
      <p class="panel-title">Instance</p>
      <input class="input input-sm mt-2 w-full num" bind:value={renaming} onchange={rename} onkeydown={(e) => e.key === "Enter" && rename()} aria-label="Its name" />
      {#if selectedKind}
        {@const k = selectedKind}
        <div class="mt-2 flex items-center gap-2 text-sm">
          <span class="font-medium">{k.title}</span>
          <span class="badge badge-xs badge-secondary badge-soft">{k.provenance.fidelity}</span>
          <span class="flex-1"></span>
          <button
            class="btn btn-link btn-xs h-auto min-h-0 p-0"
            onclick={() => {
              app.component = partKey(k);
              go("components");
            }}>In the library</button
          >
        </div>
        <p class="mt-1 text-xs faint">{k.about}</p>
        {#if k.file}
          <p class="mt-1 truncate text-xs num muted" title={k.provenance.source}>{k.file}</p>
          <p class="text-xs faint">{k.convention === "engineering" ? "e^(+jωt), as RF tools write" : "e^(−iωt), as photonoxide writes"}; measured from {range(k.provenance.validity)}</p>
        {/if}
        <div class="mt-3 flex items-center gap-1 text-xs muted">
          <span class="num">({inst.x}, {inst.y})</span>
          <span>· {inst.rotation ?? 0}°{inst.mirror ? ", mirrored" : ""}</span>
          <span class="flex-1"></span>
          <div class="tooltip" data-tip="Rotate (R)"><button class="btn btn-ghost btn-xs btn-square" aria-label="Rotate" onclick={() => rotateSelection(90)}><RotateCw size={13} /></button></div>
          <div class="tooltip" data-tip="Mirror (M)"><button class="btn btn-ghost btn-xs btn-square" aria-label="Mirror" onclick={mirrorSelection}><FlipVertical2 size={13} /></button></div>
          <div class="tooltip tooltip-left" data-tip="Delete (Del)"><button class="btn btn-ghost btn-xs btn-square" aria-label="Delete" onclick={deleteSelection}><Trash2 size={13} /></button></div>
        </div>

        <p class="panel-title mt-5">Parameters</p>
        {#each k.parameters as p, i (p.name)}
          {@const v = inst.values?.[p.name] ?? p.default}
          {@const wrong = mistakes.find((m) => m.instance === inst.name && m.parameter === p.name)}
          <div class="mt-3">
            <div class="flex items-baseline gap-1.5 text-xs">
              <span class="font-medium num">{p.name}</span>
              {#if isMicrometres(p.unit)}<span class="faint"><UnitChip /></span>{:else if p.unit}<span class="faint">{p.unit}</span>{/if}
              <span class="flex-1"></span>
              <span class="faint num" title="Its range; default {label(showIn(p.unit, p.default))}">{label(showIn(p.unit, p.min))} – {label(showIn(p.unit, p.max))}</span>
            </div>
            <div class="mt-1 flex items-center gap-2">
              <input
                class="range range-xs range-primary flex-1"
                type="range"
                min={p.min}
                max={sliderMax(p)}
                step={(sliderMax(p) - p.min) / 1000}
                value={v}
                onpointerdown={remember}
                oninput={(e) => setValue(inst.name, k, i, Number(e.currentTarget.value), false)}
                aria-label={p.name}
              />
              <input
                class="input input-xs w-24 num"
                type="number"
                step="any"
                min={showIn(p.unit, p.min)}
                max={showIn(p.unit, p.max)}
                value={showIn(p.unit, v)}
                onchange={(e) => setValue(inst.name, k, i, storeIn(p.unit, Number(e.currentTarget.value)), true)}
                aria-label="{p.name} in {unitOf(p.unit) || 'its units'}"
              />
            </div>
            {#if wrong}<p class="mt-1 text-xs text-error">{wrong.message}</p>{/if}
          </div>
        {:else}
          <p class="mt-2 text-xs faint">None.</p>
        {/each}

        <p class="panel-title mt-5">Ports</p>
        <ul class="mt-2 space-y-1 text-xs">
          {#each k.ports as port (port.name)}
            {@const ref = `${inst.name}.${port.name}`}
            {@const to = partner(ref)}
            <li class="flex items-center gap-2">
              <span class="w-10 font-medium num">{port.name}</span>
              {#if to}
                <ArrowRightLeft size={11} class="faint" /><span class="truncate num muted">{to}</span>
              {:else}
                <span class="text-warning">open: wire it or expose it</span>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="mt-3 text-sm text-error">No component {inst.kind} in the library.</p>
      {/if}
      {#each mistakes.filter((m) => m.instance === inst.name && !m.parameter) as m (m.message)}
        <p class="mt-3 text-xs text-error">{m.message}</p>
      {/each}
    {:else if selectedPort && editor.selection?.kind === "port"}
      {@const index = editor.selection.index}
      <p class="panel-title">External port</p>
      <input class="input input-sm mt-2 w-full num" bind:value={renaming} onchange={rename} onkeydown={(e) => e.key === "Enter" && rename()} aria-label="Its name" />
      <p class="mt-3 text-sm">
        {#if selectedPort.at}Exposes <span class="font-medium num">{selectedPort.at}</span>{:else}<span class="text-warning">Not wired: drag from its tip to a component's port.</span>{/if}
      </p>
      <p class="mt-1 text-xs faint">A port of the circuit: light goes in and comes out here, and the spectrum is between these ports, in their order.</p>
      <div class="mt-3 flex gap-1">
        <button class="btn btn-ghost btn-xs gap-1" onclick={() => rotateSelection(90)} title="Rotate (R)"><RotateCw size={13} /> Rotate</button>
        {#if selectedPort.at}<button class="btn btn-ghost btn-xs gap-1" onclick={() => change((c) => (c.port[index].at = ""))}>Unwire</button>{/if}
        <button class="btn btn-ghost btn-xs gap-1" onclick={deleteSelection} title="Delete (Del)"><Trash2 size={13} /> Delete</button>
      </div>
      {#each mistakes.filter((m) => m.external === index) as m (m.message)}
        <p class="mt-3 text-xs text-error">{m.message}</p>
      {/each}
    {:else if editor.selection?.kind === "wire" && chip.connections[editor.selection.index]}
      {@const [a, b] = chip.connections[editor.selection.index]}
      {@const index = editor.selection.index}
      <p class="panel-title">Connection</p>
      <p class="mt-2 text-sm num">{a} <span class="faint">↔</span> {b}</p>
      <p class="mt-1 text-xs faint">What leaves one port enters the other, at the same reference plane: a connection has no length and no loss.</p>
      <button class="btn btn-ghost btn-xs mt-3 gap-1" onclick={deleteSelection}><Trash2 size={13} /> Delete (Del)</button>
      {#each mistakes.filter((m) => m.connection === index) as m (m.message)}
        <p class="mt-3 text-xs text-error">{m.message}</p>
      {/each}
    {:else if editor.selection?.kind === "exposure" && chip.port[editor.selection.index]}
      {@const p = chip.port[editor.selection.index]}
      <p class="panel-title">Exposure</p>
      <p class="mt-2 text-sm num">{p.name} <span class="faint">→</span> {p.at}</p>
      <button class="btn btn-ghost btn-xs mt-3 gap-1" onclick={deleteSelection}><Trash2 size={13} /> Unwire (Del)</button>
    {:else}
      <p class="panel-title">Circuit</p>
      <label class="mt-2 flex flex-col gap-1 text-xs">
        <span class="muted">Name: the file is circuits/{chip.name}.toml</span>
        <input
          class="input input-sm w-full num"
          value={chip.name}
          onchange={(e) => {
            const v = e.currentTarget.value.trim();
            if (/^[A-Za-z0-9_-]+$/.test(v)) change((c) => (c.name = v));
            else {
              toast("A circuit's name names its file: letters, digits, - and _", "warning");
              e.currentTarget.value = chip.name;
            }
          }}
        />
      </label>
      <label class="mt-3 flex flex-col gap-1 text-xs">
        <span class="muted">About</span>
        <textarea class="textarea textarea-sm w-full" rows="3" value={chip.about ?? ""} onchange={(e) => change((c) => (c.about = e.currentTarget.value))}></textarea>
      </label>
      <p class="panel-title mt-5">Wavelengths</p>
      <div class="mt-2 grid grid-cols-3 gap-2 text-xs">
        <label class="flex flex-col gap-1"><span class="muted">From (<UnitChip />)</span><input class="input input-xs num" type="number" step={shown(0.01)} value={shown(chip.sweep.from_um)} onchange={(e) => change((c) => (c.sweep.from_um = stored(Number(e.currentTarget.value))))} /></label>
        <label class="flex flex-col gap-1"><span class="muted">To (<UnitChip />)</span><input class="input input-xs num" type="number" step={shown(0.01)} value={shown(chip.sweep.to_um)} onchange={(e) => change((c) => (c.sweep.to_um = stored(Number(e.currentTarget.value))))} /></label>
        <label class="flex flex-col gap-1"><span class="muted">Points</span><input class="input input-xs num" type="number" step="100" value={chip.sweep.points} onchange={(e) => change((c) => (c.sweep.points = Math.round(Number(e.currentTarget.value))))} /></label>
      </div>
      {#if sweepProblem(chip.sweep)}<p class="mt-1 text-xs text-warning">{sweepProblem(chip.sweep)}</p>{/if}
      <p class="mt-4 text-xs muted">
        {plural(chip.instance.length, "component")}, {plural(chip.connections.length, "connection")}, {plural(chip.port.length, "external port")}
      </p>
      {#if editor.path}<p class="mt-1 truncate text-xs faint" title={editor.path}>{editor.path}</p>{/if}

      {#if editor.problems.length}
        <p class="panel-title mt-5">To finish</p>
        <ul class="mt-2 space-y-1">
          {#each editor.problems as p, i (i)}
            <li>
              <button class="w-full rounded-md px-2 py-1 text-left text-xs hover:bg-base-content/5 {p.dangling ? 'text-warning' : 'text-error'}" onclick={() => point(p)}>{p.message}</button>
            </li>
          {/each}
        </ul>
      {/if}

      <div class="mt-5">
        <Tip id="chip-intro" title="Wiring a chip">
          Drag parts from the left. Drag from a port (the dots) to another port to connect them; the library refuses what it can't build and says why.
          Drag from a port into empty space to make it an external port. Open ports are marked: every port must be connected or exposed before the circuit simulates.
        </Tip>
      </div>
    {/if}
  </aside>
</div>
