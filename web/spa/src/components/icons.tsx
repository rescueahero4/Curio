/**
 * The chrome's icons, drawn here rather than installed.
 *
 * An icon package would be the larger part of this app's JavaScript for the seven glyphs it
 * actually uses, and every one of those seven has to sit on a 16px grid next to 13px text
 * without looking bolted on. They are drawn on one grid, at one stroke weight, and inherit
 * `currentColor` — the same rule the brand mark follows, and the reason a disabled pill's
 * icon fades with its label instead of staying black.
 *
 * Every icon here is decoration: each one sits inside a control that carries its own name.
 * So they are `aria-hidden` without exception, and a caller that renders one alone owes the
 * accessible name to its button.
 */

interface IconProps {
  class?: string;
}

/** Shared geometry. 16px box, 1.5 stroke — the weight that matches 13px text. */
function Line(props: IconProps & { d: string }) {
  return (
    <svg
      class={props.class}
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d={props.d} />
    </svg>
  );
}

/** A control that opens a panel below itself. */
export function ChevronDown(props: IconProps) {
  return <Line class={props.class} d="m4 6.5 4 4 4-4" />;
}

/** A row that leads somewhere. */
export function ChevronRight(props: IconProps) {
  return <Line class={props.class} d="m6.5 4 4 4-4 4" />;
}

/**
 * The mark inside a search box.
 *
 * One `d` like the rest: the ring is two arcs rather than a `<circle>`, which keeps it on
 * the shared `Line` geometry instead of forking a second SVG shape for one glyph.
 */
export function Search(props: IconProps) {
  return <Line class={props.class} d="M6.5 2.5a4 4 0 1 0 0 8 4 4 0 0 0 0-8m2.9 6.9 4 4" />;
}

/** A link that leaves the app — the arrow points out of the frame it opens from. */
export function ExternalLink(props: IconProps) {
  return (
    <Line
      class={props.class}
      d="M7 3.5H4.25a.75.75 0 0 0-.75.75v7.5c0 .414.336.75.75.75h7.5a.75.75 0 0 0 .75-.75V9.5M9.5 3H13v3.5M12.75 3.25 7.75 8.25"
    />
  );
}

/**
 * The three view modes, told apart by density rather than by metaphor.
 *
 * Deliberately the same drawing at three rhythms: four large cells, nine small ones, three
 * full-width rows. A user choosing between them is choosing how much of the screen one
 * capture gets, and the icons say exactly that at a glance — where a "grid" icon and a
 * "list" icon would only say which is which after reading the tooltip.
 */
export function ViewComfortable(props: IconProps) {
  return (
    <svg
      class={props.class}
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="2" y="2" width="5.5" height="5.5" rx="1.25" />
      <rect x="8.5" y="2" width="5.5" height="5.5" rx="1.25" />
      <rect x="2" y="8.5" width="5.5" height="5.5" rx="1.25" />
      <rect x="8.5" y="8.5" width="5.5" height="5.5" rx="1.25" />
    </svg>
  );
}

export function ViewDense(props: IconProps) {
  return (
    <svg
      class={props.class}
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="2" y="2" width="3.2" height="3.2" rx="0.8" />
      <rect x="6.4" y="2" width="3.2" height="3.2" rx="0.8" />
      <rect x="10.8" y="2" width="3.2" height="3.2" rx="0.8" />
      <rect x="2" y="6.4" width="3.2" height="3.2" rx="0.8" />
      <rect x="6.4" y="6.4" width="3.2" height="3.2" rx="0.8" />
      <rect x="10.8" y="6.4" width="3.2" height="3.2" rx="0.8" />
      <rect x="2" y="10.8" width="3.2" height="3.2" rx="0.8" />
      <rect x="6.4" y="10.8" width="3.2" height="3.2" rx="0.8" />
      <rect x="10.8" y="10.8" width="3.2" height="3.2" rx="0.8" />
    </svg>
  );
}

/*
 * The Prompt Helper's rail.
 *
 * Drawn on the same 16px grid at the same stroke weight as the chrome above, because they
 * sit two rows apart on the same screen. Every glyph is one `d` — dots are zero-length
 * segments (`h0`) under the round linecap, which keeps them on the shared `Line` geometry
 * rather than forking a second SVG shape for three list bullets.
 */

export function Undo(props: IconProps) {
  return <Line class={props.class} d="M6 3.5 3 6.5l3 3M3 6.5h6.5a3.5 3.5 0 0 1 0 7H6" />;
}

export function Redo(props: IconProps) {
  return <Line class={props.class} d="m10 3.5 3 3-3 3M13 6.5H6.5a3.5 3.5 0 0 0 0 7H10" />;
}

export function Bold(props: IconProps) {
  return (
    <Line
      class={props.class}
      d="M5.5 3.5v9M5.5 3.5h3a2.25 2.25 0 0 1 0 4.5h-3M5.5 8h3.5a2.25 2.25 0 0 1 0 4.5H5.5"
    />
  );
}

export function Italic(props: IconProps) {
  return <Line class={props.class} d="M6.5 3.5h4M5.5 12.5h4M9.5 3.5l-3 9" />;
}

/** Inline code: the two chevrons, without the box the block version carries. */
export function Code(props: IconProps) {
  return <Line class={props.class} d="M6 5 3 8l3 3M10 5l3 3-3 3" />;
}

export function BulletList(props: IconProps) {
  return <Line class={props.class} d="M3 4.5h0M3 8h0M3 11.5h0M6 4.5h7M6 8h7M6 11.5h7" />;
}

export function OrderedList(props: IconProps) {
  return (
    <Line
      class={props.class}
      d="M6 4.5h7M6 8h7M6 11.5h7M2.6 3.4l.9-.6V6M2.5 10.1a.9.9 0 0 1 1.6.5c0 .9-1.6 1.3-1.6 2.4h1.8"
    />
  );
}

/** Quote: the bar a blockquote actually renders as, not a pair of curly marks. */
export function Quote(props: IconProps) {
  return <Line class={props.class} d="M3.5 3.5v9M6.5 5h6M6.5 8h6M6.5 11h4" />;
}

export function CodeBlock(props: IconProps) {
  return (
    <Line class={props.class} d="M2.5 3.5h11v9h-11zM6.5 6.5 5 8l1.5 1.5M9.5 6.5 11 8l-1.5 1.5" />
  );
}

export function Divider(props: IconProps) {
  return <Line class={props.class} d="M2.5 8h11" />;
}

export function Plus(props: IconProps) {
  return <Line class={props.class} d="M8 3.5v9M3.5 8h9" />;
}

/** Send this away. Drawn inside the 16px box like the rest, then scaled down by the caller. */
export function Cross(props: IconProps) {
  return <Line class={props.class} d="m4 4 8 8M12 4l-8 8" />;
}

/** The markdown badge: the enclosure, the M, and the arrow that means "as text". */
export function Markdown(props: IconProps) {
  return (
    <Line
      class={props.class}
      d="M2 4h12v8H2zM4.5 10V6l1.5 2 1.5-2v4M10.5 6v4M9 8.5l1.5 1.5L12 8.5"
    />
  );
}

/** Copy: the sheet behind the sheet. */
export function Copy(props: IconProps) {
  return (
    <Line
      class={props.class}
      d="M5.5 5.5V3.75a1.25 1.25 0 0 1 1.25-1.25h5.5A1.25 1.25 0 0 1 13.5 3.75v5.5a1.25 1.25 0 0 1-1.25 1.25H10.5M3.75 5.5h5.5a1.25 1.25 0 0 1 1.25 1.25v5.5a1.25 1.25 0 0 1-1.25 1.25h-5.5A1.25 1.25 0 0 1 2.5 12.25v-5.5A1.25 1.25 0 0 1 3.75 5.5Z"
    />
  );
}

/** Done — the tick a copy button turns into. */
export function Check(props: IconProps) {
  return <Line class={props.class} d="m3.5 8.5 3 3 6-7" />;
}

export function Trash(props: IconProps) {
  return (
    <Line
      class={props.class}
      d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8a1 1 0 0 0 1 .9h3.8a1 1 0 0 0 1-.9l.6-8"
    />
  );
}

/** A row: the thumbnail cell, then the line of text it labels. */
export function ViewList(props: IconProps) {
  return (
    <svg
      class={props.class}
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="2" y="2.5" width="3" height="3" rx="0.8" />
      <rect x="6.5" y="3.25" width="7.5" height="1.5" rx="0.75" />
      <rect x="2" y="6.5" width="3" height="3" rx="0.8" />
      <rect x="6.5" y="7.25" width="7.5" height="1.5" rx="0.75" />
      <rect x="2" y="10.5" width="3" height="3" rx="0.8" />
      <rect x="6.5" y="11.25" width="7.5" height="1.5" rx="0.75" />
    </svg>
  );
}
