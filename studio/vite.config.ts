import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import katex from "katex";
import { defineConfig, type Plugin } from "vite";

import { formulas } from "./src/lib/formulas.ts";

/**
 * Every formula of every Academy lesson (academy/*.md) rendered by KaTeX as the window renders
 * it, failing the build on the first that doesn't: CI's `pnpm build` then refuses a lesson whose
 * math is broken. The lessons' other checks are the program's tests (src-tauri/src/academy.rs).
 */
function lessonMath(): Plugin {
  return {
    name: "academy-math",
    buildStart() {
      const dir = fileURLToPath(new URL("../academy/", import.meta.url));
      const errors: string[] = [];
      let count = 0;
      for (const file of readdirSync(dir).filter((f) => f.endsWith(".md") && f !== "README.md")) {
        const text = readFileSync(dir + file, "utf8").replaceAll("\r\n", "\n");
        // the body: after the front matter, its line numbers kept
        const end = text.startsWith("---\n") ? text.indexOf("\n---\n", 4) + 5 : 0;
        const skipped = text.slice(0, end).split("\n").length - 1;
        for (const f of formulas(text.slice(end))) {
          count++;
          try {
            katex.renderToString(f.tex, { throwOnError: true, displayMode: f.display, strict: "error" });
          } catch (e) {
            errors.push(`academy/${file}:${f.line + skipped}: ${(e as Error).message}\n    ${f.tex}`);
          }
        }
      }
      if (errors.length) this.error(`${errors.length} of the lessons' ${count} formulas don't render:\n${errors.join("\n")}`);
    },
  };
}

// the studio's web view: served on a fixed port for `tauri dev`, built to dist/ for the app
export default defineConfig({
  plugins: [lessonMath(), tailwindcss(), svelte()],
  clearScreen: false,
  server: { port: 1430, strictPort: true },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    chunkSizeWarningLimit: 2500,
    // not the "[PLUGIN_TIMINGS]" warning on every build: compiling Svelte is JavaScript's work,
    // and most of the build is that whatever we do
    rolldownOptions: { checks: { bundlerTimings: false } },
  },
});
