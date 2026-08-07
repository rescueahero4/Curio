/**
 * Vocabulary strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/vocabulary.ts` is
 * type-checked against `Vocabulary`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import type { Translated } from "~/lib/i18n/translated";

export const vocabulary = {} as const;

export type Vocabulary = Translated<typeof vocabulary>;
