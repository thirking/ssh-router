import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react],

  // Vite options tailored for Tauri development - only see this part
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
    // Tauri expects a fixed port; fail fast if 5173 is unavailable
  },
  // Env variables starting with TAURI_ are exposed to the client
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // Tauri supports ES2021
    target: ["es2021", "chrome100", "safari13"],
    // don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },
}));
