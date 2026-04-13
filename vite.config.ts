import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],
  // Vite options tailored for Tauri to prevent too much magic
  // and scope.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Using polling as fallback since fsEvents may not be available on all devices
      usePolling: true,
    },
  },
  build: {
    target: ["es2021"],
    minify: !process.env.TAURI_DEBUG,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
}));
