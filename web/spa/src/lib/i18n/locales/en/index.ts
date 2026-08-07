/**
 * The English dictionary, assembled.
 *
 * One file per screen-group rather than one big object, for the same reason the components
 * are split that way: a string is easiest to keep honest next to the other strings from the
 * same surface. The namespace a key lands in is also its first line of documentation —
 * `library.bulk.selected` says where it appears before you have read the value.
 *
 * This barrel is the only place that knows the full set, and `Dictionary` — the type every
 * Japanese file is checked against — is derived from it.
 */

import { common } from "~/lib/i18n/locales/en/common";
import { editor } from "~/lib/i18n/locales/en/editor";
import { items } from "~/lib/i18n/locales/en/items";
import { library } from "~/lib/i18n/locales/en/library";
import { projects } from "~/lib/i18n/locales/en/projects";
import { settings } from "~/lib/i18n/locales/en/settings";
import { shell } from "~/lib/i18n/locales/en/shell";
import { system } from "~/lib/i18n/locales/en/system";
import { vocabulary } from "~/lib/i18n/locales/en/vocabulary";

export const dictionary = {
  common,
  shell,
  library,
  items,
  editor,
  settings,
  vocabulary,
  projects,
  system,
} as const;

export type Dictionary = typeof dictionary;
