/**
 * The Japanese dictionary, assembled.
 *
 * Each module file is already checked against its English counterpart, so by the time they
 * meet here completeness is settled. The annotation below is the second belt: it catches a
 * namespace added to English and forgotten here entirely, which the per-file checks cannot
 * see.
 */

import type { Dictionary } from "~/lib/i18n/locales/en";
import { common } from "~/lib/i18n/locales/ja/common";
import { editor } from "~/lib/i18n/locales/ja/editor";
import { items } from "~/lib/i18n/locales/ja/items";
import { library } from "~/lib/i18n/locales/ja/library";
import { projects } from "~/lib/i18n/locales/ja/projects";
import { settings } from "~/lib/i18n/locales/ja/settings";
import { shell } from "~/lib/i18n/locales/ja/shell";
import { system } from "~/lib/i18n/locales/ja/system";
import { vocabulary } from "~/lib/i18n/locales/ja/vocabulary";
import type { Translated } from "~/lib/i18n/translated";

export const dictionary: Translated<Dictionary> = {
  common,
  shell,
  library,
  items,
  editor,
  settings,
  vocabulary,
  projects,
  system,
};
