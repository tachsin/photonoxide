import { defineConfig } from "vite";

// the studio's web view: served on a fixed port for `tauri dev`, built to dist/ for the app
export default defineConfig({
  clearScreen: false,
  server: { port: 1430, strictPort: true },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true, chunkSizeWarningLimit: 1500 },
});
