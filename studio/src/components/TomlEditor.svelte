<script lang="ts">
  // The job's text, editable: CodeMirror with TOML, JSON or YAML colouring that follows the theme.
  import { StreamLanguage, syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { json } from "@codemirror/legacy-modes/mode/javascript";
  import { toml } from "@codemirror/legacy-modes/mode/toml";
  import { yaml } from "@codemirror/legacy-modes/mode/yaml";
  import { Compartment, EditorState } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { tags } from "@lezer/highlight";
  import { basicSetup } from "codemirror";
  import { onMount } from "svelte";

  import type { JobFormat } from "../lib/api";

  let { value, onchange, format = "toml" }: { value: string; onchange: (text: string) => void; format?: JobFormat } =
    $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  const language = new Compartment();
  const modes = { toml, json, yaml };
  const languageOf = (f: JobFormat) => StreamLanguage.define(modes[f]);

  const look = HighlightStyle.define([
    { tag: tags.comment, color: "color-mix(in oklch, var(--color-base-content) 45%, transparent)", fontStyle: "italic" },
    { tag: [tags.string, tags.special(tags.string)], color: "var(--color-success)" },
    { tag: [tags.number, tags.bool, tags.atom], color: "var(--color-accent)" },
    { tag: [tags.propertyName, tags.variableName, tags.definition(tags.variableName)], color: "var(--color-primary)" },
    { tag: [tags.heading, tags.bracket, tags.squareBracket], color: "var(--color-secondary)" },
  ]);

  const theme = EditorView.theme({
    "&": { height: "100%", fontSize: "12.5px", backgroundColor: "transparent", color: "var(--color-base-content)" },
    ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.6" },
    ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "color-mix(in oklch, var(--color-base-content) 30%, transparent)" },
    ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "color-mix(in oklch, var(--color-base-content) 5%, transparent)" },
    ".cm-cursor": { borderLeftColor: "var(--color-primary)" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "color-mix(in oklch, var(--color-primary) 25%, transparent) !important" },
    "&.cm-focused": { outline: "none" },
  });

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          basicSetup,
          language.of(languageOf(format)),
          syntaxHighlighting(look),
          theme,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) onchange(u.state.doc.toString());
          }),
        ],
      }),
    });
    return () => view?.destroy();
  });

  // the format changed: colour the text as the new one
  $effect(() => {
    const f = format;
    view?.dispatch({ effects: language.reconfigure(languageOf(f)) });
  });

  // the form changed the text: show it, unless it is what the editor already holds
  $effect(() => {
    const text = value;
    if (view && text !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    }
  });
</script>

<div bind:this={host} class="h-full min-h-0 overflow-hidden"></div>
