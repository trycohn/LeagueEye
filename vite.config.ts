import { defineConfig } from "vite";
import { resolve } from "path";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
        goldOverlay: resolve(__dirname, "gold-overlay.html"),
        objectiveOverlay: resolve(__dirname, "objective-overlay.html"),
      },
    },
  },
  server: {
    // 127.0.0.1 avoids VPN/DNS issues where localhost resolves or routes oddly on Windows
    host: host || "127.0.0.1",
    port: 5173,
    strictPort: true,
    hmr: host ? { protocol: "ws", host, port: 5174 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
