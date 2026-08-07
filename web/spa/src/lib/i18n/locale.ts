/**
 * Which language the dashboard is speaking, and how it decided.
 *
 * Two layers, and keeping them apart is the whole design. The **preference** is what the
 * user chose and what gets persisted — one of the shipped locales, or `"system"`, meaning
 * "keep following the browser". The **locale** is what that resolves to right now. Storing
 * only the resolved value would quietly freeze a user into whatever their browser happened
 * to say the first time they opened the app, with no way back short of clearing storage.
 *
 * The choice lives in `localStorage`, not on the server. Language is a property of the
 * person reading this screen, not of the library they are reading it against, and the same
 * Curio instance is legitimately read by one person in Japanese and another in English.
 */

import { createSignal } from "solid-js";

/** The locales the dashboard ships. Ordered as the picker lists them. */
export const LOCALES = ["en", "ja"] as const;

export type Locale = (typeof LOCALES)[number];

/** What the user picked. `"system"` defers to the browser, and keeps deferring. */
export type LocalePreference = Locale | "system";

/**
 * How each locale names *itself*.
 *
 * An endonym, always. A picker that offers "Japanese" is useless to the person who needs
 * it — they are looking for 日本語, and by definition cannot read the label that would tell
 * them where to find it.
 */
export const LOCALE_NAMES: Record<Locale, string> = {
  en: "English",
  ja: "日本語",
};

const STORAGE_KEY = "curio.locale";

const [preference, setPreference] = createSignal<LocalePreference>(readPreference());

/**
 * The resolved locale: the preference, or the browser's answer if it is `"system"`.
 *
 * A derived accessor rather than a `createMemo`, because a memo built at module scope has
 * no owner to dispose it and Solid says so, loudly, in development. There is nothing to
 * memoise anyway — this is a comparison and, at worst, a short loop over a list the browser
 * already holds.
 */
const locale = (): Locale => {
  const chosen = preference();
  return chosen === "system" ? fromBrowser() : chosen;
};

export { locale, preference };

/**
 * Record the user's choice and reflect it on the document.
 *
 * The dictionary swap is **not** here — `index.ts` owns that, because it can fail and this
 * cannot. A locale live in the signal while its dictionary is still in flight would paint
 * one half-translated frame, so the caller loads first and commits second.
 */
export function commitPreference(next: LocalePreference): void {
  setPreference(next);
  applyToDocument(locale());
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // A browser with storage denied still gets a working language for this tab; it just
    // does not remember. Refusing to switch would be a worse answer to the same problem.
  }
}

/**
 * Put the locale where things that are not Solid can see it.
 *
 * `lang` is read by screen readers to pick a voice, by the UA for line-breaking — which
 * Japanese needs and English does not — and by font fallback. Han characters render from a
 * Japanese face rather than a Chinese one only because this attribute says so.
 */
export function applyToDocument(next: Locale = locale()): void {
  document.documentElement.lang = next;
}

/**
 * The first shipped locale the browser asks for.
 *
 * `navigator.languages` is the full preference list, best first, and its entries are full
 * tags — "ja-JP" has to match the "ja" we ship, so the comparison is on the subtag.
 */
export function fromBrowser(): Locale {
  for (const tag of navigator.languages ?? [navigator.language]) {
    const base = tag.toLowerCase().split("-")[0];
    const hit = LOCALES.find((candidate) => candidate === base);
    if (hit) return hit;
  }
  return "en";
}

function readPreference(): LocalePreference {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "system") return "system";
    return LOCALES.find((candidate) => candidate === stored) ?? "system";
  } catch {
    return "system";
  }
}
