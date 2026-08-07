/**
 * The MCP server: one toggle, two transports, and the shortest way into each client.
 *
 * Claude Code is given a command rather than a config file because it has one — `claude mcp
 * add` writes the same registration the user would otherwise hand-edit, and a line they can
 * paste into a terminal has nowhere to go wrong. Claude Desktop has no such command, so it
 * still gets JSON.
 *
 * The stdio form leads, and that is a decision about *this* app rather than a preference for
 * stdio. Curio takes an ephemeral port unless one is pinned in config, so an HTTP
 * registration is only true until the next restart — it names a port that is gone. The proxy
 * asks the running instance where it is on every frame, which is the only form of this that
 * keeps working without the user learning why it stopped.
 */

import { Show } from "solid-js";
import { CopyBlock } from "~/components/settings/CopyBlock";
import { type Commit, pausedReason } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { CheckField, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Settings } from "~/lib/types";

/**
 * A path, safe to drop into a command line.
 *
 * `C:\Program Files\Curio\curio.exe` is two arguments unquoted, and the second one is not a
 * file. Double quotes rather than single because this line is as likely to be run in
 * PowerShell as in a POSIX shell, and only double quotes mean the same thing in both.
 */
function quote(value: string) {
  return /\s/.test(value) ? `"${value}"` : value;
}

export function McpSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function toggle(next: boolean) {
    void saver.save({ mcp_enabled: next }, { mcp_enabled: !next });
  }

  /**
   * `--` is load-bearing: everything after it is the command Claude Code should spawn, not
   * more flags for Claude Code. `--scope user` because a library is not a property of the
   * folder the user happened to be standing in when they ran this — the default scope would
   * register Curio for that one project and nowhere else.
   *
   * The command the server sends is an absolute path, so this line works without Curio being
   * on PATH — and has to survive the spaces that come with one on Windows.
   */
  const addCommand = () =>
    [
      "claude mcp add --scope user curio --",
      quote(props.settings.mcp_stdio_command),
      ...props.settings.mcp_stdio_args,
    ].join(" ");

  const addHttpCommand = () =>
    `claude mcp add --transport http --scope user curio ${props.settings.mcp_http_url}`;

  const desktopSnippet = () =>
    JSON.stringify(
      {
        mcpServers: {
          curio: {
            command: props.settings.mcp_stdio_command,
            args: props.settings.mcp_stdio_args,
          },
        },
      },
      null,
      2,
    );

  return (
    <Section id="mcp" title={t("settings.mcp.title")} saver={saver} blurb={t("settings.mcp.blurb")}>
      <CheckField
        label={t("settings.mcp.toggle")}
        checked={props.settings.mcp_enabled}
        disabled={paused()}
        reason={pausedReason()}
        onChange={toggle}
      />

      {/* Registering while the toggle is off is the one failure that looks like nothing: the
          command succeeds, the client lists the server, and every call comes back refused. */}
      <Show when={!props.settings.mcp_enabled}>
        <p class="max-w-prose text-xs text-caution">{t("settings.mcp.off")}</p>
      </Show>

      <CopyBlock
        label={t("settings.mcp.claudeCode.label")}
        hint={t("settings.mcp.claudeCode.hint")}
        snippet={addCommand()}
      />

      {/* Three registrations, one under another, at one width. They were two columns below
          the first, which said they were a pair of alternatives to it — they are not: they
          are two more clients, each with its own caveat, and the side-by-side arrangement
          also cut the JSON snippet to half the measure it needs and set two blocks of
          different heights against each other. */}
      <CopyBlock
        label={t("settings.mcp.claudeCodeHttp.label")}
        hint={t("settings.mcp.claudeCodeHttp.hint")}
        snippet={addHttpCommand()}
      />

      <CopyBlock
        label={t("settings.mcp.claudeDesktop.label")}
        hint={t("settings.mcp.claudeDesktop.hint")}
        snippet={desktopSnippet()}
      />
    </Section>
  );
}
