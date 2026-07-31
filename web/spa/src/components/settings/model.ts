/** The two things every settings section shares: the write, and what to say when paused. */

import type { SettingsPatch } from "~/lib/types";

/** One PUT, landed in the page's copy of the settings. */
export type Commit = (patch: SettingsPatch) => Promise<void>;

/** Every mutating control on this page is disabled while paused, and says this (R-FE-8). */
export const PAUSED_REASON = "Curio is paused. Resume from the tray icon to change settings.";
