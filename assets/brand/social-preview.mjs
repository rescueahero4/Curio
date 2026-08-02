/**
 * Render the GitHub social preview card — the image shown when a link to the repo is
 * pasted into Slack, Discord, X, or any OpenGraph embed.
 *
 * Run from the repo root:
 *
 *   npm --prefix target/pw install playwright && npx playwright install chromium
 *   node assets/brand/social-preview.mjs
 *
 * `target/` is gitignored, so Playwright stays out of the repo and out of web/site — where
 * it would otherwise be downloaded by `npm ci` on every Pages deploy for a script that
 * deploy never runs.
 *
 * ## The output is not wired to anything
 *
 * GitHub stores the social preview itself: there is no path in the repo it reads, and no
 * REST endpoint, so `gh` cannot upload it either. The PNG this writes has to be attached by
 * hand, once, at **Settings → General → Social preview → Upload an image**. This script
 * exists so that "once" is reproducible — re-run it after a brand change instead of
 * reconstructing the card from memory in a design tool.
 *
 * ## Why a browser, again
 *
 * Same reason as rasterize.mjs: Chromium draws the mark and the real webfonts exactly as
 * the landing page will, so the card and the site agree by construction. The fonts are
 * inlined from web/site's @fontsource packages rather than assumed to be installed —
 * headless Chromium has neither Inria Serif nor IBM Plex Sans, and silently falling back to
 * a system serif would ship a card that misrepresents the brand.
 */

import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.join(HERE, "..", "..");

/**
 * Find Playwright wherever it happens to be installed.
 *
 * A bare `import "playwright"` cannot work here: Node resolves upward from this file —
 * assets/brand, assets, then the repo root — and the repo root is a cargo workspace with no
 * node_modules. Every place Playwright could plausibly live is off that path, so the
 * candidates are named explicitly and the failure says what to run.
 */
function loadPlaywright() {
	const candidates = [
		path.join(REPO_ROOT, "target", "pw"),
		path.join(REPO_ROOT, "web", "site"),
		REPO_ROOT,
	];
	for (const base of candidates) {
		try {
			return createRequire(path.join(base, "package.json"))("playwright");
		} catch {
			// Not here; try the next one.
		}
	}
	throw new Error(
		"playwright not found. From the repo root:\n" +
			"  npm --prefix target/pw install playwright && npx playwright install chromium",
	);
}

const { chromium } = loadPlaywright();

/** GitHub's documented size for the social preview. Anything else gets cropped or scaled. */
const WIDTH = 1280;
const HEIGHT = 640;

/**
 * Same padded box as rasterize.mjs — see the note there. The mark is off-centre in its
 * 500 box and nearly touches three edges.
 */
const VIEW_BOX = "10 30 480 440";

const source = fs.readFileSync(path.join(HERE, "curio-mark.svg"), "utf8");
const pathData = source.match(/ d="([^"]+)"/)?.[1];
if (!pathData) throw new Error("could not find the path data in curio-mark.svg");

/**
 * Inline a webfont as a data URI.
 *
 * Fails loudly. A missing font would otherwise render in a fallback and produce a card that
 * looks almost right — the worst possible outcome for an image that represents the project
 * everywhere it is linked.
 */
function fontFace(family, file, weight) {
	const full = path.join(REPO_ROOT, "web", "site", "node_modules", "@fontsource", file);
	if (!fs.existsSync(full)) {
		throw new Error(
			`missing font: ${full}\nRun \`npm --prefix web/site install\` first — the card is drawn with the same webfonts as the site.`,
		);
	}
	const data = fs.readFileSync(full).toString("base64");
	return `@font-face{font-family:"${family}";font-weight:${weight};font-style:normal;src:url(data:font/woff2;base64,${data}) format("woff2")}`;
}

const fonts = [
	fontFace("Inria Serif", "inria-serif/files/inria-serif-latin-400-normal.woff2", 400),
	fontFace("IBM Plex Sans", "ibm-plex-sans/files/ibm-plex-sans-latin-400-normal.woff2", 400),
].join("\n");

/**
 * The card.
 *
 * Laid out to survive being scaled down: Slack and X render this at a few hundred pixels
 * wide, so the wordmark carries the recognition and the tagline is sized to stay legible
 * rather than to fill the space. Content is kept well inside the edges because some
 * surfaces crop the card to a squarer aspect.
 */
const html = `<!doctype html>
<html><head><meta charset="utf-8"><style>
${fonts}
*{margin:0;padding:0;box-sizing:border-box}
body{
  width:${WIDTH}px;height:${HEIGHT}px;
  background:#fff;color:#000;
  font-family:"IBM Plex Sans",sans-serif;
  display:flex;flex-direction:column;justify-content:center;
  padding:0 104px;
  /* The landing page's hairline frame, so the card reads as the same surface. */
  border:1px solid #000;
}
.lockup{position:relative;display:flex;align-items:flex-end;gap:20px;margin-left:-14px}
/* No rotation. The landing page rotates its crow a half-turn because it draws the Figma
   export, whose path data is stored unrotated; curio-mark.svg is already upright, so
   copying that transform here would stand the bird on its head. */
.bird{position:absolute;top:0;left:0;width:104px;height:94px}
.word{font-family:"Inria Serif",serif;font-size:148px;line-height:1;padding-top:47px}
/* Wide enough for the tagline to sit on one line. At 940px it wrapped and left "library."
   alone on the second, which is the first thing the eye lands on after the wordmark. */
.tagline{font-size:34px;line-height:1.35;margin-top:26px;max-width:1040px}
.meta{display:flex;gap:12px;margin-top:40px}
.pill{
  border:1px solid #000;border-radius:100px;
  padding:9px 20px;font-size:22px;line-height:normal;white-space:nowrap;
}
.pill--solid{background:#3f3f3f;color:#fff}
</style></head>
<body>
  <div class="lockup">
    <svg class="bird" viewBox="${VIEW_BOX}" xmlns="http://www.w3.org/2000/svg"><path d="${pathData}" fill="#000"/></svg>
    <span class="word">Curio</span>
  </div>
  <p class="tagline">Free opensource personal local-first design inspiration library.</p>
  <div class="meta">
    <span class="pill pill--solid">Local-first</span>
    <span class="pill">No telemetry</span>
    <span class="pill">MIT</span>
    <span class="pill">Rust + SolidJS</span>
  </div>
</body></html>`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: WIDTH, height: HEIGHT }, deviceScaleFactor: 1 });
await page.setContent(html, { waitUntil: "load" });
// The inlined faces are data URIs, so this resolves immediately — but without it the
// screenshot can land before layout has reflowed to the real metrics.
await page.evaluate(() => document.fonts.ready);

const out = path.join(HERE, "social-preview.png");
await page.screenshot({ path: out });
await browser.close();

const kb = Math.round(fs.statSync(out).size / 1024);
console.log(`wrote social-preview.png (${WIDTH}x${HEIGHT}, ${kb}kB)`);
if (kb > 1024) console.warn("warning: GitHub rejects social previews over 1MB");
console.log("Upload it at Settings → General → Social preview; GitHub stores it, not the repo.");
