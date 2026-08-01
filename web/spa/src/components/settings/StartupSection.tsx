/**
 * Start at login.
 *
 * The OS is the authority here, not the config file: the stored value is what Curio asked
 * for and the toggle shows what the OS actually reports. Where the platform cannot honour
 * the request at all, this says why instead of offering a switch that would flip on screen
 * and change nothing.
 */

import { Show } from "solid-js";
import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { CheckField, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
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
      title="Startup"
      saver={unsupported() ? undefined : saver}
      blurb="Curio can start with your machine so captures and the watcher are running before you go looking for them."
    >
      <Show
        when={unsupported()}
        fallback={
          <CheckField
            label="Start Curio when I log in"
            checked={props.settings.launch_at_login}
            disabled={paused()}
            reason={PAUSED_REASON}
            onChange={toggle}
          />
        }
      >
        {(reported) => (
          <p class="max-w-prose text-sm text-ink-muted">
            This platform cannot register Curio to start at login
            {reported().reason ? `: ${reported().reason}` : "."} There is no toggle here because
            there is nothing behind it — add Curio to your session's startup items instead.
          </p>
        )}
      </Show>
    </Section>
  );
}
