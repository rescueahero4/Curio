/**
 * System strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/system.ts` is
 * type-checked against `System`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import type { Translated } from "~/lib/i18n/translated";

export const system = {} as const;

export type System = Translated<typeof system>;
