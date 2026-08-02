/**
 * Pack the committed brand PNGs into `curio.ico`.
 *
 * Run from the repo root:
 *
 *   node packaging/windows/make-ico.mjs
 *
 * The output is committed, like the PNGs it is built from — this script is the record of
 * how the icon was produced, not a build step. Nothing in the gate runs it, and CI consumes
 * the committed `.ico` so a release never depends on regenerating an asset.
 *
 * ## Why hand-rolled and not a dependency
 *
 * An ICO is a 6-byte header, a 16-byte entry per image, and the images themselves. Since
 * Vista those images may be PNGs verbatim, so "convert PNG to ICO" is really "concatenate
 * PNGs with an index" — about forty lines. Adding an npm dependency to the packaging path
 * to avoid writing them would put a supply-chain surface on the one artifact users execute.
 *
 * To add sizes: extend SIZES in assets/brand/rasterize.mjs, re-run it, then re-run this.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const BRAND = path.join(HERE, "..", "..", "assets", "brand");
const OUT = path.join(HERE, "curio.ico");

/** Every size Windows asks for that we actually have on disk. */
const SIZES = [16, 32, 48, 128, 256];

const images = SIZES.map((size) => ({
  size,
  file: path.join(BRAND, `curio-mark-${size}.png`),
}))
  .filter(({ file }) => fs.existsSync(file))
  .map(({ size, file }) => ({ size, data: fs.readFileSync(file) }));

if (images.length === 0) {
  throw new Error(`no curio-mark-*.png found in ${BRAND}`);
}

const HEADER = 6;
const ENTRY = 16;

const header = Buffer.alloc(HEADER);
header.writeUInt16LE(0, 0); // reserved
header.writeUInt16LE(1, 2); // 1 = icon
header.writeUInt16LE(images.length, 4);

let offset = HEADER + ENTRY * images.length;
const entries = [];

for (const { size, data } of images) {
  const entry = Buffer.alloc(ENTRY);
  // 256 is stored as 0: the field is one byte, so 256 does not fit and the format spends
  // the wraparound on its largest legal size.
  entry.writeUInt8(size >= 256 ? 0 : size, 0);
  entry.writeUInt8(size >= 256 ? 0 : size, 1);
  entry.writeUInt8(0, 2); // palette size — 0 for truecolour
  entry.writeUInt8(0, 3); // reserved
  entry.writeUInt16LE(1, 4); // colour planes
  entry.writeUInt16LE(32, 6); // bits per pixel
  entry.writeUInt32LE(data.length, 8);
  entry.writeUInt32LE(offset, 12);
  entries.push(entry);
  offset += data.length;
}

fs.writeFileSync(
  OUT,
  Buffer.concat([header, ...entries, ...images.map((i) => i.data)]),
);

console.log(
  `wrote ${path.relative(process.cwd(), OUT)} — ${images.map((i) => i.size).join(", ")}px`,
);
