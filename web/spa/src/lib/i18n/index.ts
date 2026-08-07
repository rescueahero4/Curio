/**
 * The dashboard's translator.
 *
 * `t` is a module-level singleton, reached by importing it, exactly like `paused()` in
 * `lib/http.ts` and the caches in `lib/stores.ts`. No provider, no context, no prop drilled
 * through six components — a string is available wherever a string is needed. It is a Solid
 * accessor underneath, so a component that calls `t(...)` in its JSX re-renders that text
 * on a language change and nothing else does.
 *
 * **English is bundled, Japanese is fetched.** English is the fallback and has to be there
 * before the first paint; Japanese is one `import()` away, and a reader who never switches
 * never pays for it (the same rule R-FE-3 applies to the editor stack). The Japanese
 * dictionary is *merged over* English rather than replacing it, so the one failure the type
 * system cannot see — a namespace that never got wired into the barrel — degrades to
 * English words rather than to blank space.
 *
 * Loading is awaited **before** the locale is committed. Committing first would paint one
 * frame of the old language under the new `lang` attribute.
 */

import * as i18n from "@solid-primitives/i18n";
import { createSignal } from "solid-js";
import {
  applyToDocument,
  commitPreference,
  fromBrowser,
  type Locale,
  type LocalePreference,
  locale,
} from "~/lib/i18n/locale";
import { type Dictionary, dictionary as en } from "~/lib/i18n/locales/en";
import type { Translated } from "~/lib/i18n/translated";

export type { Locale, LocalePreference } from "~/lib/i18n/locale";
export { LOCALE_NAMES, LOCALES, locale, preference } from "~/lib/i18n/locale";

/**
 * The flat key space, widened.
 *
 * Derived from `Translated<Dictionary>` rather than from `Dictionary` itself, so a value
 * types as `string` and not as the literal English word that happens to be in the source
 * file. `t("common.save") === "Save"` should not typecheck — at runtime it is 保存 half the
 * time, and a comparison like that is a bug the compiler is in a position to catch.
 */
type FlatDictionary = i18n.Flatten<Translated<Dictionary>>;

/** The widening assignment that makes the flattened English usable as either language. */
const ENGLISH: Translated<Dictionary> = en;

const ENGLISH_FLAT = i18n.flatten(ENGLISH);

const [dictionary, setDictionary] = createSignal<FlatDictionary>(ENGLISH_FLAT);

/**
 * Translate a key.
 *
 * `t("library.empty")` for a plain string; `t("library.count", { n: 4 })` where the English
 * carries `{{ n }}`. The arguments are typed off the template, so a call that forgets one —
 * or passes a name the string does not use — does not compile.
 */
export const t = i18n.translator(dictionary, i18n.resolveTemplate);

/**
 * Load the dictionary a locale needs.
 *
 * Repeat calls are cheap: the dynamic import is cached by the module system, so switching
 * back and forth costs one fetch in total, not one per switch.
 */
async function load(target: Locale): Promise<FlatDictionary> {
  if (target === "en") return ENGLISH_FLAT;

  const { dictionary: ja } = await import("~/lib/i18n/locales/ja");
  return { ...ENGLISH_FLAT, ...i18n.flatten(ja) };
}

/**
 * Switch languages: fetch, swap, then record.
 *
 * If the fetch fails — an offline reload against a stale service worker is the realistic
 * case — the preference is left alone and the user keeps the language they could read. A
 * half-applied switch is the one outcome worth ruling out.
 */
export async function setLocalePreference(next: LocalePreference): Promise<void> {
  const target: Locale = next === "system" ? fromBrowser() : next;
  try {
    setDictionary(await load(target));
  } catch {
    console.error("curio: could not load the dictionary for", target);
    return;
  }
  commitPreference(next);
}

/**
 * Prepare the detected locale before the app mounts. Called once, from `main.tsx`.
 *
 * English needs nothing and returns immediately, which is the common path and the reason
 * this does not gate the first paint for most readers.
 */
export async function startI18n(): Promise<void> {
  applyToDocument();
  const target = locale();
  if (target === "en") return;

  try {
    setDictionary(await load(target));
  } catch {
    // English is already in the signal, and English is readable. Falling back silently
    // beats an error screen for a dashboard whose actual job is still available.
    console.error("curio: could not load the dictionary for", target);
  }
}
