<script lang="ts">
  // Text with inline TeX math ($…$), the math set by KaTeX in the text's own colour. KaTeX's
  // stylesheet comes in through Vite with its fonts, so the math renders offline.
  import "katex/dist/katex.min.css";

  import { mathHtml } from "../lib/math";

  let { text, class: klass = "" }: { text: string; class?: string } = $props();

  const html = $derived(mathHtml(text));
</script>

<span class="math-text {klass}">{@html html}</span>

<style>
  /* KaTeX sets math at 1.21em of the text; in the studio's small UI type that towers over it. */
  .math-text :global(.katex) {
    font-size: 1.08em;
  }
</style>
