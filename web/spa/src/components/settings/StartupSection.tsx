/**
 * Start at login.
 *
 * The OS is the authority here, not the config file: the stored value is what Curio asked
 * for and the toggle shows what the OS actually reports. Where the platform cannot honour
 * the request at all, this says why instead of offering a switch that would flip on screen
 * and change nothing.
 */

import { Show } from "solid-js";
import { type Commit, pausedReason } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { CheckField, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Settings } from "~/lib/types";

export function StartupSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);
  const unsupported = () => {
    const support = props.settings.launch_at_login_support;
    return support.supported ? null : support;
  };

  function toggle(next: boolean) {
    void saver.save({ launch_at_login: next }, { launch_at_login: !next });
  }

  return (
    <Section
      id="startup"
      title={t("settings.startup.title")}
      saver={unsupported() ? undefined : saver}
      blurb={t("settings.startup.blurb")}
    >
      {/* When the platform cannot honour this, the explanation is two whole sentences chosen
          by whether the server gave a reason — not one sentence with `: ${reason}` spliced
          into the middle of it. The English tolerated that because the explanation happens to
          belong right after the colon in English; in Japanese it belongs somewhere else, and
          no amount of punctuation in this file can move it there. `reason` itself arrives
          already written as a sentence, in whatever language the server writes in — so only
          the English renders it. The Japanese drops the placeholder and states the one
          platform fact the server can report; a second `AutostartSupport::reason` in Rust
          needs a key of its own rather than another branch here. */}
      <Show
        when={unsupported()}
        fallback={
          <CheckField
            label={t("settings.startup.toggle")}
            checked={props.settings.launch_at_login}
            disabled={paused()}
            reason={pausedReason()}
            onChange={toggle}
          />
        }
      >
        {(reported) => (
          <p class="max-w-prose text-sm text-ink-muted">
            {reported().reason
              ? t("settings.startup.unsupportedReason", { reason: reported().reason ?? "" })
              : t("settings.startup.unsupported")}
          </p>
        )}
      </Show>
    </Section>
  );
}
