import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import os from "node:os";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Cache de deps fora do repo: dentro do Dropbox, o sync trava o rename do
// node_modules/.vite (EBUSY) e quebra a otimização de dependências.
const cacheDir = path.join(os.tmpdir(), "hicorder-vite");

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],
  cacheDir,

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
