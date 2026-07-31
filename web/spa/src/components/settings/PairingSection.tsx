/** The fallback pairing path for the browser extension (R-FE-19, D11). */

import { Section } from "~/components/settings/section";

export function PairingSection() {
  return (
    <Section
      title="Browser extension"
      blurb="A normally installed extension connects on its own and never needs this. Use it when the extension cannot — a development install loaded unpacked, or an installer step that was declined."
    >
      {/*
        A full document load, not a client-side route change. The extension's content script
        injects at document load only, so a soft navigation would land on /pair with nothing
        watching for the handoff element and the authorize click would appear to do nothing.
        `rel="external"` is how @solidjs/router is told to leave an anchor alone — it has no
        `reloadDocument` prop of its own.
      */}
      <a href="/pair" rel="external" class="pill pill-outline w-fit">
        Authorize this browser
      </a>
      <p class="max-w-prose text-xs text-ink-faint">
        Opens as a full page load, which is what lets the extension see the page.
      </p>
    </Section>
  );
}
