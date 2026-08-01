import { cp, mkdir, rm } from "node:fs/promises";
import * as esbuild from "esbuild";

/**
 * Bundle the extension into `dist/`, ready to load unpacked.
 *
 * esbuild rather than Vite: this produces three independent entry points with no shared
 * runtime, no HTML transformation, and no dev server — the parts of a bundler an extension
 * actually needs are the parts esbuild is.
 *
 * The manifest is **copied verbatim**. Its pinned `key` fixes the extension id that
 * `curio-server` allowlists, so a build step that rewrote or regenerated the manifest could
 * silently break capture (R-EXT-2, Inventory §10.1).
 */

const watch = process.argv.includes("--watch");

const entries = {
  "worker/service-worker": "src/worker/service-worker.ts",
  "popup/popup": "src/popup/popup.ts",
  "content/pair": "src/content/pair.ts",
};

await rm("dist", { recursive: true, force: true });
await mkdir("dist", { recursive: true });

const context = await esbuild.context({
  entryPoints: Object.fromEntries(Object.entries(entries).map(([out, input]) => [out, input])),
  outdir: "dist",
  bundle: true,
  format: "esm",
  // Chrome 116 is the floor: it is the first version where active WebSocket traffic resets
  // the MV3 service worker's idle timer, which is what makes the 20-second keepalive work
  // (R-EXT-1, R-EXT-10).
  target: "chrome116",
  sourcemap: watch ? "inline" : false,
  minify: !watch,
  logLevel: "info",
});

await context.rebuild();
await cp("manifest.json", "dist/manifest.json");
await cp("src/popup/popup.html", "dist/popup/popup.html");
// The mark, at the four sizes Chrome asks for. Generated from the one source SVG by
// `assets/brand/rasterize.mjs`; copied rather than bundled because esbuild has no reason
// to see them.
await cp("icons", "dist/icons", { recursive: true });

if (watch) {
  await context.watch();
  console.log("watching…");
} else {
  await context.dispose();
}
