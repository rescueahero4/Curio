/**
 * Projects strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/projects.ts` is
 * type-checked against `Projects`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import type { Translated } from "~/lib/i18n/translated";

export const projects = {} as const;

export type Projects = Translated<typeof projects>;
