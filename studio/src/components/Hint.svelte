<script lang="ts">
  // An ⓘ with what something is in a tooltip beside it; the text may hold math in $…$, set by
  // KaTeX (a plain `title` or `data-tip` can't hold it).
  import { Info } from "@lucide/svelte";

  import MathText from "./MathText.svelte";

  let { text, side = "right" }: { text: string; side?: "right" | "left" | "top" | "bottom" } = $props();
  const SIDES = { right: "tooltip-right", left: "tooltip-left", top: "tooltip-top", bottom: "tooltip-bottom" };
</script>

<span class="tooltip {SIDES[side]} z-30 inline-flex shrink-0 align-middle font-normal">
  <span class="tooltip-content text-left text-[12px] leading-snug"><MathText {text} /></span>
  <Info size={12} class="faint" />
</span>
