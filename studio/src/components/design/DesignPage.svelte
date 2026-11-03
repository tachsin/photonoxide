<script lang="ts">
  // The Design page (#668): every component as its job file, a design alone (a `device` table, no
  // task) in YAML, TOML or JSON. On the left, the built-in components' files (jobs/components/,
  // each what its device's page builds at its defaults) and the workspace's own; on the right,
  // the file in the shared editor with the design in 3D beside it. A built-in file is read only:
  // "Edit a copy" makes it one of mine under another name. A component is named and saved in the
  // workspace's jobs/, where the rail lists it under "Your components" and it opens on its
  // device's page with every tab. A built-in component's name is refused, saying whose it is.
  import { ask } from "@tauri-apps/plugin-dialog";
  import { CopyPlus, FileCode2, FilePlus2, Lock, Save, SquareArrowOutUpRight } from "@lucide/svelte";

  import { api, type DesignFile, type DesignRead, type JobFormat } from "../../lib/api";
  import { app, toast } from "../../lib/app.svelte";
  import { builtInWith, FORMATS, nameProblem } from "../../lib/designs";
  import { designPage, loadDesignFiles, openInDesign } from "../../lib/designpage.svelte";
  import { devices, loadDevices, openDesign, openDeviceText } from "../../lib/device.svelte";
  import type { Scene } from "../../lib/events";
  import Device3D from "../device/Device3D.svelte";
  import JobEditor from "../job/JobEditor.svelte";

  loadDevices();
  loadDesignFiles();

  const files = $derived(designPage.files);
  const open = $derived(designPage.open);

  // the first built-in file, when nothing is open yet
  $effect(() => {
    if (!designPage.open && files?.built_in[0]) {
      const f = files.built_in[0];
      designPage.open = { text: f.text, format: f.format, path: f.path, builtIn: true, saved: f.text };
    }
  });

  // ---- the text, read: its name, description, design, and what the library says ----
  let read = $state<DesignRead | null>(null);
  let scene = $state<Scene | null>(null);
  $effect(() => {
    if (!open) return;
    const [t, f] = [open.text, open.format];
    let live = true;
    const timer = setTimeout(async () => {
      const r = await api.designRead(t, f).catch((e) => ({ name: "", about: "", spec: null, beyond: [], error: String(e) }) as DesignRead);
      if (!live) return;
      read = r;
      if (r.spec) {
        const g = await api.deviceGeometry(r.spec).catch(() => null);
        if (live && g?.scene) scene = g.scene;
      }
    }, 200);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  /** The name it is saved under: the text's, until another is typed. */
  let name = $state("");
  let named = $state<string | null>(null);
  $effect(() => {
    const n = read?.name ?? "";
    if (named === null || named === n) name = n;
  });
  const builtIn = $derived(files?.built_in ?? []);
  const clash = $derived(open && !open.builtIn ? builtInWith(builtIn, name) : undefined);
  const problem = $derived(open?.builtIn ? "" : nameProblem(name) || (clash ? `"${name}" is the built-in ${clash.kind}'s name: give your component another` : ""));
  const changed = $derived(!!open && open.text !== open.saved);

  function show(f: DesignFile) {
    named = null;
    openInDesign({ text: f.text, format: f.format, path: f.path, builtIn: f.built_in });
  }

  function edited(t: string) {
    if (!open || open.builtIn || t === open.text) return;
    open.text = t;
  }

  async function setFormat(f: JobFormat) {
    if (!open || f === open.format) return;
    try {
      const text = await api.convertJob(open.text, open.format, f);
      open.saved = open.saved === open.text ? text : open.saved;
      open.text = text;
      open.format = f;
    } catch (e) {
      toast(`It can't be written in ${f.toUpperCase()} as it is: ${e}`, "error");
    }
  }

  /** The file as one of mine, not saved yet: a built-in one's copy under another name, or a new design of `kind` at its defaults. */
  async function copyOf(text: string, format: JobFormat, newName: string) {
    const r = await api.designRead(text, format);
    if (!r.spec) return toast(`It isn't a design: ${r.error}`, "error");
    const copy = await api.designText(r.spec, newName, r.about, format);
    named = null;
    openInDesign({ text: copy, format, path: null, builtIn: false, saved: "" });
  }

  let picking = $state(false);
  async function newOf(id: string) {
    picking = false;
    const f = builtIn.find((b) => b.name === id);
    if (f) await copyOf(f.text, f.format, `my-${id}`);
  }

  async function save(replace = false) {
    if (!open || !read?.spec || problem) return;
    // the name typed, kept: the box shows the text's name again once it is saved
    const as = name.trim();
    try {
      // a name typed over the text's: the text written again under it
      const text = as === read.name ? open.text : await api.designText(read.spec, as, read.about, open.format);
      // its own file, the one it was opened from or saved to (in this format or another), is replaced unasked
      const stem = open.path?.split(/[\\/]/).pop()?.replace(/\.(ya?ml|toml|json)$/, "");
      const own = !!open.path && !open.builtIn && stem === as;
      const path = await api.saveComponent(text, open.format, replace || own);
      open.text = text;
      open.saved = text;
      open.path = path;
      named = null;
      app.workspaceVersion++;
      await loadDesignFiles();
      toast(`Saved ${as}: it is in the rail, under Your components, and opens on its page`, "success");
    } catch (e) {
      const m = String(e);
      if (m.startsWith("exists:")) {
        const there = m.slice("exists:".length).trim();
        if (await ask(`A component named ${as} exists (${there}). Replace it?`, { title: "Replace the component?", kind: "warning" })) await save(true);
        return;
      }
      toast(m, "error");
    }
  }

  /** The design on its device's page, with every tab. */
  async function onItsPage() {
    if (!open) return;
    if (open.path && !open.builtIn && !changed) return openDesign(open.path);
    const yaml = open.format === "yaml" ? open.text : await api.convertJob(open.text, open.format, "yaml").catch(() => null);
    if (yaml) await openDeviceText(yaml);
  }

  let height = $state(0);
  const ROW = "flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left text-[13px] transition-colors";
  const tone = (f: DesignFile) => (open?.path === f.path ? "bg-primary/12 text-primary" : "hover:bg-base-content/5");
</script>

<div class="flex h-full min-h-0">
  <!-- the files -->
  <nav class="flex w-60 shrink-0 flex-col border-r border-base-content/8" aria-label="Components' files">
    <div class="flex items-center gap-2 border-b border-base-content/8 px-3 py-2">
      <h2 class="flex-1 text-sm font-semibold">Design</h2>
      <div class="dropdown dropdown-end">
        <button class="btn btn-primary btn-xs gap-1" onclick={() => (picking = !picking)} title="A new component: a device's design at its defaults, to change, name and save"><FilePlus2 size={12} /> New</button>
        {#if picking}
          <ul class="dropdown-content menu z-30 mt-1 max-h-80 w-64 flex-nowrap overflow-y-auto rounded-box border border-base-content/10 bg-base-100 p-1 shadow-xl">
            {#each devices.list as d (d.id)}<li><button onclick={() => newOf(d.id)}>{d.kind}</button></li>{/each}
          </ul>
        {/if}
      </div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto p-2">
      {#if designPage.problem}<p class="px-2 text-xs text-error">{designPage.problem}</p>{/if}
      <h3 class="px-2 pt-1 pb-1 text-[11px] font-medium tracking-wide uppercase faint">Your components</h3>
      {#each files?.mine ?? [] as f (f.path)}
        <button class="{ROW} {tone(f)}" onclick={() => show(f)} title="{f.path}: a {f.kind}">
          <span class="min-w-0 flex-1 truncate">{f.name}</span><span class="text-[10.5px] uppercase faint">{f.format}</span>
        </button>
      {:else}
        <p class="px-2 pb-2 text-xs faint">None yet: start from New, or from a built-in one's copy.</p>
      {/each}
      <h3 class="px-2 pt-3 pb-1 text-[11px] font-medium tracking-wide uppercase faint">Built-in, in jobs/components/</h3>
      {#each builtIn as f (f.path)}
        <button class="{ROW} {tone(f)}" onclick={() => show(f)} title="{f.path}: the {f.kind} at its defaults">
          <Lock size={11} class="shrink-0 faint" /><span class="min-w-0 flex-1 truncate">{f.name}</span>
        </button>
      {/each}
    </div>
  </nav>

  <!-- the file -->
  <section class="flex min-h-0 min-w-0 flex-1 flex-col">
    {#if open}
      <div class="flex flex-wrap items-center gap-2 border-b border-base-content/8 px-4 py-2">
        <FileCode2 size={16} class="text-primary" />
        {#if open.builtIn}
          <span class="text-sm font-semibold num">{read?.name ?? ""}</span>
          <span class="badge badge-sm badge-ghost gap-1" title="A built-in component's file: read only"><Lock size={10} /> {open.path}</span>
        {:else}
          <label class="input input-sm w-56">
            <span class="text-xs faint">Name</span>
            <input class="num" bind:value={name} oninput={() => (named = name)} spellcheck="false" aria-label="The component's name" />
          </label>
          {#if open.path}<span class="badge badge-sm badge-ghost max-w-72 truncate" title={open.path}>{open.path.split(/[\\/]/).slice(-2).join("/")}</span>{:else}<span class="badge badge-sm badge-warning badge-soft">not saved</span>{/if}
          {#if changed && open.path}<span class="text-warning" title="Changed since it was saved">●</span>{/if}
        {/if}
        {#if read?.spec}<span class="text-xs muted">a {read.spec.kind}</span>{/if}
        <span class="flex-1"></span>
        <div class="join" role="group" aria-label="The format">
          {#each FORMATS as [f, label] (f)}
            <button class="btn btn-xs join-item {open.format === f ? 'btn-active' : 'btn-ghost'}" aria-pressed={open.format === f} onclick={() => setFormat(f)}>{label}</button>
          {/each}
        </div>
        <button class="btn btn-sm gap-1.5" onclick={onItsPage} disabled={!read?.spec} title="Open the design on its device's page, with every tab"><SquareArrowOutUpRight size={14} /> On its page</button>
        {#if open.builtIn}
          <button class="btn btn-primary btn-sm gap-1.5" onclick={() => open && copyOf(open.text, open.format, `${read?.name ?? "component"}-copy`)} title="The built-in one stays as it is: edit a copy, under another name, and save it in your workspace"><CopyPlus size={14} /> Edit a copy</button>
        {:else}
          <button class="btn btn-primary btn-sm gap-1.5" onclick={() => save()} disabled={!!problem || !read?.spec || !!read?.error || !!read?.beyond.length} title="Save it in the workspace's jobs/ as {name}.{open.format}"><Save size={14} /> Save</button>
        {/if}
      </div>
      {#if problem || read?.beyond.length}
        <p role="alert" class="border-b border-base-content/8 bg-warning/8 px-4 py-1.5 text-xs selectable">{problem || `A component is a design alone: ${read?.beyond.join(", ")} belong to a job that computes it.`}</p>
      {/if}
      <JobEditor text={open.text} format={open.format} onchange={edited} component={read?.name || "the component"} readonly={open.builtIn}>
        {#snippet toolbar()}
          {#if read?.about}<p class="mb-2 line-clamp-2 text-xs muted" title={read.about}>{read.about}</p>{/if}
        {/snippet}
        {#snippet preview()}
          <div class="relative min-h-0 flex-1 overflow-hidden" bind:clientHeight={height}>
            {#if scene}<Device3D {scene} height={Math.max(160, height - 8)} />{:else}<div class="skeleton m-3 h-[calc(100%-1.5rem)]"></div>{/if}
          </div>
        {/snippet}
      </JobEditor>
    {:else}
      <div class="grid flex-1 place-items-center text-sm faint">{files ? "Choose a component's file" : "Reading the components' files"}</div>
    {/if}
  </section>
</div>
