import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { resolve } from "node:path";

export default defineConfig({
  base: "/hidpp/",
  plugins: [wasm(), topLevelAwait()],
  resolve: {
    alias: {
      "hidpp-web": resolve(__dirname, "pkg/hidpp_web.js"),
    },
  },
  build: {
    target: "es2022",
    outDir: "dist",
    assetsInlineLimit: 0,
  },
});
