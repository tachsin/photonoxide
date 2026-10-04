import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// the studio's web view: served on a fixed port for `tauri dev`, built to dist/ for the app
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
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
