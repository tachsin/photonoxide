<script lang="ts">
  // A feed row's picture: its journal's initials on the journal's own tint, and over them, when
  // Settings' images are on and the paper is an arXiv preprint, the preprint's first figure
  // (papers/images.rs), asked once the row is on screen. The box has its size from the start,
  // so nothing moves when a figure comes, and the initials stay when none does.
  import { app } from "../../lib/app.svelte";
  import { monogram, monogramHue, type Paper, type PaperSummary } from "../../lib/paperText";
  import { figures, figureUrl, wantFigure } from "../../lib/papers.svelte";

  let { paper, size = 40 }: { paper: PaperSummary | Paper; size?: number } = $props();

  const initials = $derived(monogram(paper.venue));
  const hue = $derived(monogramHue(paper.venue));
  const asks = $derived(!!app.state?.settings.paper_images && !!paper.arxiv);
  const name = $derived(asks ? figures[paper.id] : null);
  let broken = $state(false);
  let box: HTMLElement | undefined = $state();

  $effect(() => {
    if (!asks || !box) return;
    const id = paper.id;
    const seen = new IntersectionObserver((entries) => {
      for (const e of entries) wantFigure(id, e.isIntersecting);
    });
    seen.observe(box);
    return () => {
      seen.disconnect();
      wantFigure(id, false);
    };
  });
</script>

<span bind:this={box} class="paper-thumb relative grid shrink-0 place-items-center overflow-hidden rounded-lg font-semibold tracking-tight select-none {initials.length > 2 ? 'text-[10px]' : 'text-[11px]'}" style:--h={hue} style:width="{size}px" style:height="{size}px" aria-hidden="true">
  {initials}
  {#if name && !broken}
    <img src={figureUrl(name)} alt="" loading="lazy" decoding="async" width={size} height={size} class="absolute inset-0 h-full w-full bg-white object-cover" onerror={() => (broken = true)} />
  {/if}
</span>
