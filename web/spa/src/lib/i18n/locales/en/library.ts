/**
 * Library strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/library.ts` is
 * type-checked against `Library`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import type { Translated } from "~/lib/i18n/translated";

export const library = {} as const;

export type Library = Translated<typeof library>;
