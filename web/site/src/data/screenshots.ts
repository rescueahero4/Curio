import type { ImageMetadata } from "astro";

import itemDetail from "../assets/screenshots/item-detail.png";
import libraryGrid from "../assets/screenshots/library-grid.png";
import promptComposer from "../assets/screenshots/prompt-composer.png";

export interface Screenshot {
	/** Imported so Astro fingerprints and optimises it; never a raw path string. */
	image: ImageMetadata;
	/** Describes what the screen shows, for readers who cannot see it. */
	alt: string;
	/** Shown under the image in the full-screen view. */
	caption: string;
}

/**
 * The showcase grid's contents, in display order.
 *
 * Order is the design's, reading across the two columns then down: library, then prompt
 * composer, then item detail on the second row.
 *
 * Adding a screenshot is this list plus a file in `../assets/screenshots/` — the grid, the
 * counter, the arrows and the keyboard bounds all read their length from here, so nothing
 * else needs to change. An odd count simply leaves the last cell of the grid empty.
 */
export const screenshots: Screenshot[] = [
	{
		image: libraryGrid,
		alt: "The Curio library: a grid of captured design references, each with its source domain and the vocabulary terms Curio assigned to it, above filters for design type, family, tag and status.",
		caption: "The library — every capture, described and filed in your own vocabulary",
	},
	{
		image: promptComposer,
		alt: "The prompt composer, showing a structured design brief with headings for Brief, Intent, Guardrails, Design Direction and Output, where vocabulary terms are inserted as inline chips.",
		caption: "The prompt composer — briefs built from the vocabulary, ready for your AI tools",
	},
	{
		image: itemDetail,
		alt: "A single library item open for editing, showing its full-bleed screenshot beside a panel holding the name, Curio's written description, the source URL, a regeneration prompt, and editable family, design-type and tag chips.",
		caption: "One item — Curio's description and vocabulary, all of it yours to correct",
	},
];
