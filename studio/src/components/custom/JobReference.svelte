<script lang="ts">
  // The Reference tab beside the job's text: the schema's docs for the whole job, of its kind,
  // a table at a time. Each key with its type, unit, default, range and docs, the needed ones
  // marked; a table's keys open under it. A key the text has is a link to it there; a filter
  // narrows the keys shown to those whose name or docs have the words.
  import { ChevronRight } from "@lucide/svelte";

  import { fieldsAt, type Field, type Path, type Schema } from "../../lib/jobschema";

  let { schema, model, onjump }: { schema: Schema | null; model: unknown; onjump: (p: Path) => void } = $props();

  let filter = $state("");
  /** The tables opened, by their paths. */
  let opened = $state<Set<string>>(new Set([JSON.stringify(["task"])]));
  const key = (p: Path) => JSON.stringify(p);
  function toggle(p: Path) {
    const k = key(p);
    const next = new Set(opened);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    opened = next;
  }

  /** Where a key's own keys are: a table's, or its first item's for a list of tables. */
  const inner = (p: Path, f: Field): Path | null => {
    const t = f.doc.type;
    if (t.startsWith("list of table")) return [...p, f.name, 0];
    return /\btable\b/.test(t) && !t.startsWith("list") ? [...p, f.name] : null;
  };
  const at = (p: Path) => p.reduce<unknown>((v, k) => (v && typeof v === "object" ? (v as Record<string | number, unknown>)[k] : undefined), model);
  const words = $derived(filter.toLowerCase().split(/\s+/).filter(Boolean));
  const shows = (f: Field) => words.every((w) => f.name.toLowerCase().includes(w) || f.doc.description.toLowerCase().includes(w));
  const meta = (f: Field) =>
    [f.doc.type, f.doc.unit && `unit: ${f.doc.unit}`, f.doc.default && `default ${f.doc.default}`, f.doc.range && `from ${f.doc.range}`].filter(Boolean).join(" · ");
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="shrink-0 px-3 pt-2 pb-2">
    <input class="input input-sm w-full text-xs" placeholder="Filter the keys by name or docs" bind:value={filter} spellcheck="false" />
  </div>
  <div class="min-h-0 flex-1 overflow-y-auto px-3 pb-3 text-xs">
    {#if !schema}
      <p class="faint">The schema is loading.</p>
    {:else}
      {@render table([], 0)}
    {/if}
  </div>
</div>

{#snippet table(p: Path, depth: number)}
  {@const fields = schema ? fieldsAt(schema, model, p) : []}
  <ul class="space-y-1.5 {depth ? 'mt-1.5 ml-2 border-l border-base-content/10 pl-2.5' : ''}">
    {#each fields as f (f.name)}
      {@const sub = depth < 5 ? inner(p, f) : null}
      {@const here = at([...p, f.name]) !== undefined}
      {@const open = !!sub && (opened.has(key(sub)) || words.length > 0)}
      {#if shows(f) || (sub && words.length > 0)}
        <li>
          <div class="flex items-baseline gap-1.5">
            {#if sub}
              <button class="btn btn-ghost btn-xs btn-square -ml-1 h-4 min-h-0 w-4" aria-label={open ? "Close" : "Open"} onclick={() => toggle(sub)}>
                <ChevronRight size={12} class="transition-transform {open ? 'rotate-90' : ''}" />
              </button>
            {/if}
            {#if here}
              <button class="link link-hover font-mono font-medium text-primary" title="Show it in the text" onclick={() => onjump([...p, f.name])}>{f.name}</button>
            {:else}
              <span class="font-mono font-medium text-base-content/80">{f.name}</span>
            {/if}
            {#if f.required}<span class="badge badge-xs badge-soft badge-warning">needed</span>{/if}
            <span class="truncate text-[10.5px] faint" title={meta(f)}>{meta(f)}</span>
          </div>
          {#if f.doc.description && shows(f)}<p class="mt-0.5 leading-snug faint">{f.doc.description.replace(/`/g, "")}</p>{/if}
          {#if f.doc.values.length && shows(f)}<p class="mt-0.5 font-mono text-[10.5px] faint">one of: {f.doc.values.join(", ")}</p>{/if}
          {#if open && sub}{@render table(sub, depth + 1)}{/if}
        </li>
      {/if}
    {/each}
  </ul>
{/snippet}
