import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

/**
 * The dashboard's build.
 *
 * Output lands in `dist/`, which `curio-server` embeds into the binary via rust-embed —
 * one origin, one file to ship (R-FE-4, R-DEL-3).
 */
export default defineConfig({
  plugins: [solid(), tailwindcss()],

  // Relative asset URLs only. The SPA must never assume an origin: it is served from an
  // ephemeral port that changes every run (R-FE-4).
  base: "/",

  resolve: {
    // Mirrors the `~/*` path in tsconfig.json. Vite does not read tsconfig paths, so
    // without this the types resolve and the build does not — which fails late, at
    // bundling, rather than in the editor.
    alias: {
      "~": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  server: {
    // Fixed so the debug-mode proxy in curio-server has something deterministic to point
    // at when it lands in P3.
    port: 5173,
    strictPort: true,
  },

  build: {
    outDir: "dist",
    emptyOutDir: true,
    // Every asset is embedded in the binary, so inlining small files just moves bytes
    // from one place in the executable to another while making them uncacheable.
    assetsInlineLimit: 0,
    rollupOptions: {
      output: {
        manualChunks(id) {
          // R-FE-3: the editor stack must not load on any route but /prompts/:id. It was
          // the measured split point in the previous implementation, and TipTap plus
          // ProseMirror is the largest thing the dashboard imports — a regression here is
          // invisible except as a slower first paint on every other route.
          if (id.includes("@tiptap") || id.includes("prosemirror")) {
            return "editor";
          }
          return undefined;
        },
      },
    },
  },
});
