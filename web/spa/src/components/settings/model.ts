/** The two things every settings section shares: the write, and what to say when paused. */

import { t } from "~/lib/i18n";
import type { SettingsPatch } from "~/lib/types";

/** One PUT, landed in the page's copy of the settings. */
export type Commit = (patch: SettingsPatch) => Promise<void>;

/**
 * Every mutating control on this page is disabled while paused, and says this (R-FE-8).
 *
 * A function, not a constant. A module-level string is resolved once, when the module is
 * first imported — which is before the reader has had any chance to change the language, and
 * would leave ten controls across seven sections explaining the pause in whichever language
 * happened to be loaded at startup. Called from inside JSX it is read on every render, so it
 * follows a switch like everything else on the page.
 */
export const pausedReason = () => t("settings.paused.reason");
