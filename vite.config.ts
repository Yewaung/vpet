import { resolve } from "node:path";
import { defineConfig } from "vite";

// Tauri expects a fixed dev port and serves the built files from dist/.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        pet: resolve(__dirname, "index.html"),
        talk: resolve(__dirname, "talk.html"),
        mind: resolve(__dirname, "mind.html"),
        status: resolve(__dirname, "status.html"),
        menu: resolve(__dirname, "menu.html"),
        eggs: resolve(__dirname, "eggs.html"),
        share: resolve(__dirname, "share.html"),
      },
    },
  },
});
