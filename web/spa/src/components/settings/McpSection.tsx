/** The MCP server: one toggle, two transports, two snippets. */

import { CopyBlock } from "~/components/settings/CopyBlock";
import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { CheckField, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import type { Settings } from "~/lib/types";

export function McpSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function toggle(next: boolean) {
    void saver.save({ mcp_enabled: next }, { mcp_enabled: !next });
  }

  const httpSnippet = () =>
    JSON.stringify(
      { mcpServers: { curio: { type: "http", url: props.settings.mcp_http_url } } },
      null,
      2,
    );

  const stdioSnippet = () =>
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
    <Section
      title="MCP server"
      saver={saver}
      blurb="Lets an AI agent search your library, read items, and register projects. The toggle gates both transports: turned off, the HTTP endpoint refuses and the stdio proxy — which forwards to the same endpoint — refuses with it. Off means off everywhere."
    >
      <CheckField
        label="Let agents reach this library over MCP"
        checked={props.settings.mcp_enabled}
        disabled={paused()}
        reason={PAUSED_REASON}
        onChange={toggle}
      />

      <div class="grid gap-4 lg:grid-cols-2">
        <CopyBlock
          label="Claude Code (HTTP)"
          hint="Paste into your Claude Code MCP config. The port is the one this run is listening on."
          snippet={httpSnippet()}
        />
        <CopyBlock
          label="Claude Desktop (stdio)"
          hint="Paste into claude_desktop_config.json. The proxy forwards to the same server, so the toggle above still applies."
          snippet={stdioSnippet()}
        />
      </div>
    </Section>
  );
}
