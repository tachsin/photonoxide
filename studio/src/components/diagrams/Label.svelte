<script lang="ts">
  // A label in a diagram: text with TeX between dollars, set by KaTeX as the lesson's text is, at
  // a point of the drawing, and scaled with it.
  import MathText from "../MathText.svelte";

  let {
    x,
    y,
    text,
    anchor = "middle",
    size = 15,
    width = 260,
    class: klass = "text-base-content",
  }: {
    x: number;
    y: number;
    text: string;
    /** Which end of the text is at x, as SVG's text-anchor. */
    anchor?: "start" | "middle" | "end";
    /** Its size in the drawing's units. */
    size?: number;
    /** The room it has, in the drawing's units. */
    width?: number;
    class?: string;
  } = $props();

  const height = $derived(size * 2.4);
  const left = $derived(anchor === "start" ? x : anchor === "end" ? x - width : x - width / 2);
  const justify = $derived(anchor === "start" ? "justify-start" : anchor === "end" ? "justify-end" : "justify-center");
</script>

<foreignObject x={left} y={y - height / 2} {width} {height} class="pointer-events-none overflow-visible">
  <div class="flex h-full items-center whitespace-nowrap {justify} {klass}" style="font-size: {size}px; line-height: 1.2">
    <MathText {text} />
  </div>
</foreignObject>
