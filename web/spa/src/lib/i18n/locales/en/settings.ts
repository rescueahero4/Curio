/**
 * Settings strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/settings.ts` is
 * type-checked against `Settings`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import type { Translated } from "~/lib/i18n/translated";

export const settings = {} as const;

export type Settings = Translated<typeof settings>;
