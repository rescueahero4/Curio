/**
 * The shape a translation has to match.
 *
 * English is declared `as const`, which gives every entry a *literal* type — `"Save"`, not
 * `string`. That is what makes `t("common.save")` return a known key rather than a guess,
 * but it also means a Japanese file typed as `typeof en` could only ever contain the
 * English words back again. `Translated` is the same tree with the leaves widened: same
 * keys, same nesting, same template arguments, any string as the value.
 *
 * The payoff is the whole reason the dictionaries are typed at all. Adding an English key
 * and forgetting the Japanese one is a **compile error**, not a blank label discovered by a
 * user six screens in. So is renaming a key on one side, and so is changing `{{ count }}`
 * to `{{ total }}` in one language and not the other — `Template` carries its arguments in
 * the type, so the two files cannot drift apart in the one way that would crash at runtime.
 */

import type { Template } from "@solid-primitives/i18n";

export type Translated<T> = {
  readonly [K in keyof T]: T[K] extends Template<infer Args>
    ? // Checked before the plain-string branch: a template *is* a string, so the order
      // here is what preserves its arguments instead of erasing them.
      Template<Args>
    : T[K] extends string
      ? string
      : Translated<T[K]>;
};
