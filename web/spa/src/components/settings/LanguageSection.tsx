/**
 * The interface language.
 *
 * The one section on this page that does not talk to the server. Every other setting is a
 * property of the library — where files live, which model assesses them — and belongs to
 * whoever owns it. Language is a property of the person at the keyboard, so it is stored in
 * this browser and takes effect immediately. That is also why there is no save badge: there
 * is no round trip to report the end of, and a "Saved" that appeared after an instantaneous
 * change would be describing something the user watched happen.
 *
 * The options name themselves — English, 日本語 — because a reader hunting for their own
 * language cannot, by definition, read the label that would translate it for them.
 */

import { For } from "solid-js";
import { ChevronDown } from "~/components/icons";
import { Field, Section } from "~/components/settings/section";
import {
  LOCALE_NAMES,
  LOCALES,
  type LocalePreference,
  preference,
  setLocalePreference,
  t,
} from "~/lib/i18n";

export function LanguageSection() {
  /** `"system"` first: it is the default, and it is the one entry that is a rule. */
  const options = (): { value: LocalePreference; label: string }[] => [
    { value: "system", label: t("common.language.system") },
    ...LOCALES.map((code) => ({ value: code, label: LOCALE_NAMES[code] })),
  ];

  return (
    <Section id="language" title={t("common.language.title")} blurb={t("common.language.blurb")}>
      <Field label={t("common.language.label")}>
        {(id) => (
          <div class="field-wrap w-fit">
            <select
              id={id}
              class="field field-select w-auto"
              value={preference()}
              onChange={(event) => {
                void setLocalePreference(event.currentTarget.value as LocalePreference);
              }}
            >
              <For each={options()}>
                {(option) => <option value={option.value}>{option.label}</option>}
              </For>
            </select>
            {/* Drawn over the select, not by it — the same glyph at the same distance from
                the same edge as every other select in the app. */}
            <ChevronDown class="field-mark field-mark-end" />
          </div>
        )}
      </Field>
    </Section>
  );
}
