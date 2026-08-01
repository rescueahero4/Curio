/**
 * Rasterise the brand mark to the PNG sizes the tray and the extension need.
 *
 * Run from the repo root:
 *
 *   node --experimental-strip-types assets/brand/rasterize.mjs
 *   # or simply: node assets/brand/rasterize.mjs
 *
 * Requires Playwright's Chromium, which the repo already uses for headed validation:
 *
 *   npm --prefix target/pw install playwright && npx playwright install chromium
 *
 * ## Why a browser and not an SVG library
 *
 * The mark is a single 500×500 path with a lot of curve detail, and the sizes that matter
 * most (16 px, 32 px) are where a naive rasteriser turns it to mush. Chromium's renderer is
 * the same one that will draw the SVG in the dashboard, so the raster and the vector agree
 * by construction rather than by luck.
 *
 * The output is committed. This script is the record of how those PNGs were produced, not
 * a build step — nothing in the gate runs it, and a contributor without Playwright can
 * still build everything.
 */

import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));

/** Sizes Chrome asks for, plus the 32 px the tray uses (R-EXT-1, PRD §5). */
const SIZES = [16, 32, 48, 128];

/**
 * The mark, padded.
 *
 * The source path occupies roughly x∈[25,476], y∈[45,455] of a 500 box — it is not
 * centred, and it very nearly touches three edges. Rendering it as-is gives an icon that
 * looks cropped at 16 px, so it is fitted into a padded viewBox here rather than by editing
 * the original file, which stays byte-identical to what the designer supplied.
 */
const VIEW_BOX = "10 30 480 440";

const source = fs.readFileSync(path.join(HERE, "curio-mark.svg"), "utf8");
const pathData = source.match(/ d="([^"]+)"/)?.[1];
if (!pathData) throw new Error("could not find the path data in curio-mark.svg");

const browser = await chromium.launch();
const page = await browser.newPage();

for (const size of SIZES) {
  // `ink` rather than pure black: the chrome's ink token is #1c1917, and an icon that is
  // blacker than every glyph beside it reads as a foreign object in a menu bar.
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}"
      viewBox="${VIEW_BOX}"><path d="${pathData}" fill="#1c1917"/></svg>`;

  await page.setViewportSize({ width: size, height: size });
  await page.setContent(
    `<html><body style="margin:0;background:transparent">${svg}</body></html>`,
  );
  await page.screenshot({
    path: path.join(HERE, `curio-mark-${size}.png`),
    omitBackground: true,
  });
  console.log(`wrote curio-mark-${size}.png`);
}

await browser.close();
