<script lang="ts">
  // A length's unit, µm or nm, as a button: clicking it shows every length in the other unit,
  // app-wide, and the choice is saved with the settings. It takes the colour and size of the
  // text it sits in.
  import { lengthUnit, toggleLengthUnit, unitText } from "../lib/units";

  let { tip = "top", class: extra = "" }: { tip?: "top" | "bottom" | "left" | "right"; class?: string } = $props();

  const other = $derived(lengthUnit() === "um" ? "nm" : "µm");
  const placement = { top: "tooltip-top", bottom: "tooltip-bottom", left: "tooltip-left", right: "tooltip-right" };

  function flip(e: MouseEvent) {
    // inside a label it would otherwise also focus the label's input
    e.preventDefault();
    e.stopPropagation();
    toggleLengthUnit();
  }
</script>

<button type="button" class="unit-chip tooltip {placement[tip]} {extra}" data-tip="Show lengths in {other}" aria-label="Lengths in {unitText()}: show them in {other}" onclick={flip}>{unitText()}</button>
