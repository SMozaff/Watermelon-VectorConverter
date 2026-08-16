import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: { port: 5174, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // Tauri ships with a modern WebView; avoid legacy downlevel transforms
    // that are incompatible with current Svelte SSR output.
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  ssr: {
    target: "node",
  },
});
