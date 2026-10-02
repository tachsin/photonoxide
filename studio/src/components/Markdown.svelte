<script lang="ts">
  // Release notes and the changelog: headings, bullets, links, bold and code, nothing more.
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { text }: { text: string } = $props();

  const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

  function inline(s: string): string {
    return esc(s)
      .replace(/`([^`]+)`/g, '<code class="rounded bg-base-content/8 px-1 font-mono text-[0.9em]">$1</code>')
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a class="link link-primary" data-href="$2">$1</a>');
  }

  const html = $derived.by(() => {
    const out: string[] = [];
    let list = false;
    for (const raw of text.replace(/<!--.*?-->/g, "").split("\n")) {
      const line = raw.trimEnd();
      const bullet = /^\s*[-*] (.*)$/.exec(line);
      if (!bullet && list) {
        out.push("</ul>");
        list = false;
      }
      const h = /^(#{1,4}) (.*)$/.exec(line);
      if (h) {
        const size = ["", "text-lg", "text-base", "text-sm", "text-sm"][h[1].length];
        out.push(`<h${h[1].length + 1} class="${size} font-semibold mt-4 mb-1.5">${inline(h[2])}</h${h[1].length + 1}>`);
      } else if (bullet) {
        if (!list) out.push('<ul class="list-disc space-y-1 pl-5">');
        list = true;
        out.push(`<li>${inline(bullet[1])}</li>`);
      } else if (line.trim()) {
        out.push(`<p class="my-1.5">${inline(line)}</p>`);
      }
    }
    if (list) out.push("</ul>");
    return out.join("");
  });

  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href]");
    if (a) {
      e.preventDefault();
      openUrl(a.dataset.href!);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="selectable text-sm leading-relaxed" onclick={click}>{@html html}</div>
