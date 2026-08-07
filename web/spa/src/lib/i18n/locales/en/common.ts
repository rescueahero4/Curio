/**
 * The words that belong to no one screen, English.
 *
 * Deliberately short. A `common` namespace grows into a junk drawer the moment it starts
 * collecting strings that merely *look* alike — "Remove" as in "take this tag off the item"
 * and "Remove" as in "delete this file" are the same five letters in English and two
 * different words in Japanese (解除 and 削除), and a shared key would force the translator
 * to pick one and be wrong somewhere. So the bar for living here is not "several screens
 * use this string", it is "several screens use this string *meaning the same thing*".
 *
 * The language block is the exception that has to be here: it is read by someone who has
 * not yet chosen a language they can read.
 */

import type { Translated } from "~/lib/i18n/translated";

export const common = {
  /* -- verbs on buttons ------------------------------------------------------------- */
  save: "Save",
  cancel: "Cancel",
  delete: "Delete",
  close: "Close",
  dismiss: "Dismiss",
  apply: "Apply",
  edit: "Edit",
  copy: "Copy",
  copied: "Copied",
  retry: "Retry",

  /* -- states ----------------------------------------------------------------------- */
  saving: "Saving…",
  loading: "Loading…",
  applying: "Applying…",

  /* -- the language picker, and the one part of Settings that ships with the core ---- */
  language: {
    title: "Language",
    blurb: "Choose the language Curio's interface uses. This is stored in this browser only.",
    label: "Interface language",
    /**
     * Named for what it does, not for the language it names. "Match my browser" is a rule
     * the user is choosing; "English" would be a result they would have to re-derive every
     * time they moved to a machine set to something else.
     */
    system: "Match my browser",
  },
} as const;

export type Common = Translated<typeof common>;
