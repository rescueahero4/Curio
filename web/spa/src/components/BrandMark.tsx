/**
 * The magpie-and-gem mark.
 *
 * **Stand-in.** PRD §5 lists the mark among the brand assets reused as-is, with the
 * canonical vector living in the previous repo's `BrandMark.tsx`. That file is not
 * available here, so what follows is a clean silhouette drawn to the same description — a
 * perched magpie with a gem — and it is meant to be replaced wholesale by the original
 * path data. Nothing else in the app draws the mark, so that replacement is this file and
 * only this file.
 *
 * Single-colour on purpose: the chrome is monochrome and the accent is ink itself, so the
 * mark inherits `currentColor` and needs no palette of its own.
 */
export function BrandMark(props: { class?: string }) {
  return (
    <svg
      class={props.class}
      viewBox="0 0 24 24"
      fill="currentColor"
      width="24"
      height="24"
      role="img"
    >
      <title>Curio</title>
      {/* Body and tail: one continuous silhouette, tail tip at the lower left. */}
      <path d="M1.6 22.4 10.3 13.7c1.2-4 3.9-7.2 7.3-8 1.4-.3 2.6.4 3 1.6l2.6 1.2-2.8 1.2c-.4 3-2.6 5.4-5.6 6l-2.2.4Z" />
      {/* The gem, carried just ahead of the beak. */}
      <path d="M21.5 0.6 23.4 2.5 21.5 4.4 19.6 2.5Z" />
    </svg>
  );
}
