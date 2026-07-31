import { createSignal, onCleanup, Show } from "solid-js";
import { serverIsUp } from "~/lib/session";

/**
 * What a visit with no session shows (R-FE-6a).
 *
 * This screen exists to kill a specific dead end. Under the previous design a bookmarked
 * dashboard tab, or one restored after a browser restart, would load and then fail every
 * request — presenting as a broken app rather than as a tab that simply needs relaunching
 * from the tray.
 *
 * So: never an error, and a quiet retry of `/health` (which is unauthenticated by
 * contract, R-SEC-11) so that a user who launches Curio while looking at this page sees it
 * notice. It cannot log them in — only the tray can mint a nonce — but it can stop looking
 * broken and tell them exactly what to do.
 */
export function NoSession(props: { reachable: boolean }) {
  const [up, setUp] = createSignal(props.reachable);

  const timer = setInterval(async () => {
    setUp(await serverIsUp());
  }, 3_000);
  onCleanup(() => clearInterval(timer));

  return (
    <main class="mx-auto flex min-h-screen max-w-md flex-col justify-center gap-4 px-6">
      <h1 class="text-xl font-semibold">Open Curio from the tray</h1>
      <p style={{ color: "var(--color-muted)" }}>
        This tab does not have a session. Curio's dashboard is opened from the tray icon, which
        hands the browser a one-time key — so a bookmarked link cannot get in on its own.
      </p>
      <p class="text-sm" style={{ color: "var(--color-muted)" }}>
        <Show when={up()} fallback="Waiting for Curio to start…">
          Curio is running. Choose <strong>Open Dashboard</strong> from its tray icon.
        </Show>
      </p>
    </main>
  );
}
