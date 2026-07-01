import { defineConfig } from "vite";
import preact from "@preact/preset-vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
// @ts-expect-error process is a nodejs global
const isWindows = process.env.TAURI_ENV_PLATFORM === "windows";
// @ts-expect-error process is a nodejs global
const isDebug = !!process.env.TAURI_ENV_DEBUG;

// Lightning CSS lowers native CSS nesting (and modules) to the webview engines
// Tauri actually ships against.
const cssTargets = isWindows ? { chrome: 105 << 16 } : { safari: 13 << 16 };

// https://vite.dev/config/
export default defineConfig({
  plugins: [preact()],
  clearScreen: false,
  // Tauri expects a fixed port and ignores the Rust sources.
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  css: {
    transformer: "lightningcss",
    lightningcss: { targets: cssTargets },
  },
  build: {
    target: isWindows ? "chrome105" : "safari13",
    minify: isDebug ? false : "esbuild",
    cssMinify: isDebug ? false : "lightningcss",
    sourcemap: isDebug,
    reportCompressedSize: false,
    // Single entry chunk loaded locally in one webview: no module-preload links
    // or polyfill, and keep CSS in one file.
    modulePreload: false,
    cssCodeSplit: false,
  },
  // Strip console/debugger from production bundles.
  esbuild: isDebug ? {} : { drop: ["console", "debugger"] },
});
