import { defineConfig } from "astro/config";

/**
 * The Curio landing page.
 *
 * A single page, and deliberately so: the documentation site that used to live here was
 * removed, and `docs/` is now read on GitHub rather than republished. That is why there is
 * no content collection, no docs framework and no search index in this config — if you are
 * looking for where the architecture pages went, they never left `docs/`.
 *
 * Nothing built here is embedded in the binary. `web/spa` is the app's UI and is a client
 * for a loopback server; it is not served from this site.
 */
export default defineConfig({
  // GitHub Pages serves a project site from a subpath. Both halves matter: `site` builds
  // absolute URLs, `base` prefixes every asset and internal link. Drop `base` and the CSS
  // 404s on the deployed site while working perfectly in `astro dev`.
  site: "https://rescueahero4.github.io",
  base: "/Curio",

  // Pages has no server-side redirect for `/foo` → `/foo/index.html`, so directory-style
  // output with a trailing slash is the only form that resolves.
  trailingSlash: "always",
});
