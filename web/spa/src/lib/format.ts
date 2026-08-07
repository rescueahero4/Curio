/**
 * Time, said the way a scanned list needs it.
 *
 * A grid is read at a glance, and "Aug 1, 2026, 5:48 PM" cannot be glanced at — it has to be
 * decoded against today's date before it means anything. "28 minutes ago" is the same fact
 * already reduced to the thing the reader wanted. The precise stamp is not thrown away: it
 * goes on the `datetime` attribute and the tooltip, where it costs no space.
 *
 * **Both formatters follow the interface language, not the browser's.** These used to pass
 * `undefined` for the locale, which means "whatever the browser is set to" — fine while
 * there was one language, wrong the moment there are two: a reader who picked Japanese on an
 * English machine would get 日本語 labels sitting next to "28 minutes ago". The formatters
 * are rebuilt when the language changes and memoised in between, because `Intl` constructors
 * are the expensive part and these are called once per row of a grid that scrolls.
 */

import { type Locale, locale } from "~/lib/i18n/locale";

const RELATIVE = perLocale((tag) => new Intl.RelativeTimeFormat(tag, { numeric: "auto" }));

const ABSOLUTE = perLocale(
  (tag) => new Intl.DateTimeFormat(tag, { dateStyle: "medium", timeStyle: "short" }),
);

/** Each step's span in units of the one before it. */
const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["second", 60],
  ["minute", 60],
  ["hour", 24],
  ["day", 7],
  ["week", 4.345],
  ["month", 12],
];

/** "28 minutes ago", "yesterday", "3 months ago" — or 「28 分前」「昨日」. */
export function relativeTime(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;

  // Negative for the past, which is the direction Intl already expects.
  let delta = (at.getTime() - Date.now()) / 1000;
  for (const [unit, span] of STEPS) {
    if (Math.abs(delta) < span) return RELATIVE().format(Math.round(delta), unit);
    delta /= span;
  }
  return RELATIVE().format(Math.round(delta), "year");
}

/** The full stamp, for `title` and `datetime` where precision is free. */
export function absoluteTime(iso: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? iso : ABSOLUTE().format(at);
}

/**
 * One `Intl` instance per language, built on first use and kept.
 *
 * A plain cache rather than a `createMemo`: a memo created at module scope has no owner to
 * dispose it, which Solid warns about in development, and the cache is the better fit
 * regardless — switching back to a language you have already used should not pay for the
 * constructor a second time. Reading `locale()` here is still a tracked read, so a grid
 * rendered inside a component re-formats its timestamps when the language changes.
 *
 * Exported for the callers that need an `Intl` this module does not wrap — `ItemRow` wants
 * a date with no time on it. Reach for this rather than constructing a formatter at module
 * scope: `new Intl.DateTimeFormat(undefined, …)` is the browser's locale, frozen at import,
 * which is the bug this file's own docblock is about.
 */
export function perLocale<T>(build: (tag: Locale) => T): () => T {
  const built = new Map<Locale, T>();
  return () => {
    const tag = locale();
    let instance = built.get(tag);
    if (!instance) {
      instance = build(tag);
      built.set(tag, instance);
    }
    return instance;
  };
}
