/**
 * Editor strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/editor.ts` is
 * type-checked against `Editor`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import type { Translated } from "~/lib/i18n/translated";

export const editor = {} as const;

export type Editor = Translated<typeof editor>;
