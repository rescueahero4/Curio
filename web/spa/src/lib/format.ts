/**
 * Time, said the way a scanned list needs it.
 *
 * A grid is read at a glance, and "Aug 1, 2026, 5:48 PM" cannot be glanced at — it has to be
 * decoded against today's date before it means anything. "28 minutes ago" is the same fact
 * already reduced to the thing the reader wanted. The precise stamp is not thrown away: it
 * goes on the `datetime` attribute and the tooltip, where it costs no space.
 */

const RELATIVE = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

const ABSOLUTE = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

/** Each step's span in units of the one before it. */
const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["second", 60],
  ["minute", 60],
  ["hour", 24],
  ["day", 7],
  ["week", 4.345],
  ["month", 12],
];

/** "28 minutes ago", "yesterday", "3 months ago". Returns the input unchanged if unparseable. */
export function relativeTime(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;

  // Negative for the past, which is the direction Intl already expects.
  let delta = (at.getTime() - Date.now()) / 1000;
  for (const [unit, span] of STEPS) {
    if (Math.abs(delta) < span) return RELATIVE.format(Math.round(delta), unit);
    delta /= span;
  }
  return RELATIVE.format(Math.round(delta), "year");
}

/** The full stamp, for `title` and `datetime` where precision is free. */
export function absoluteTime(iso: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? iso : ABSOLUTE.format(at);
}
