import { createSignal, Show } from "solid-js";
import { t } from "~/lib/i18n";

/**
 * Which way the authorization failed.
 *
 * The signal holds the *kind* rather than the sentence. Storing `t("system.pair.refused")`
 * would freeze the wording at the moment of the failure, so a reader who switched language
 * afterwards would be looking at an alert in the language they just left. `http.ts` accepts
 * that trade because `Error.message` has to be a string; nothing here forces it.
 */
type Problem = "refused" | "unreachable";

/**
 * The pairing fallback (R-FE-19, D11).
 *
 * Normally unnecessary: a properly installed extension gets `{port, token, state}` from
 * the native-messaging host and never comes here. This page covers what native messaging
 * cannot — unpacked development installs with no registered NM manifest, a user who
 * declined the installer's registration step, or a future browser without NM support.
 *
 * ## Reachable by URL only, and that is deliberate
 *
 * Settings used to carry a "Browser extension" section linking here. It was removed: the
 * state this page resolves cannot be reached without a **pinned port** (`CURIO_PORT`, or
 * `port` hand-edited into `config.json`), and neither is settable from the dashboard. That
 * made the link a developer control sitting on an end-user surface — visible to everyone,
 * actionable by almost no one, and misleading in the two states a user can actually reach:
 *
 * * **native messaging registered** — the extension re-handshakes on its own and this page
 *   is never needed (`worker/connection.ts`, R-EXT-18a);
 * * **ephemeral port, no NM** — the extension cannot find Curio at all, and the handoff
 *   below fails too, because `acceptPairingToken` has no port to attach the token to.
 *
 * So the page stays routed and the entry point does not. Navigate to `/pair` directly. The
 * four conditions that make it necessary, and the single command that avoids all of them,
 * are written up in `web/extension/README.md` under "Development installs".
 *
 * This is a stopgap, not the destination: once the tray registers the native-messaging host
 * on first launch (the open P6 gap in `packaging/README.md`), NM covers every real install
 * and this page, its content script, and `POST /api/pair/authorize` can all be deleted
 * together.
 *
 * ## The security contract, unchanged from the previous implementation
 *
 * **The handoff element *is* the authorization.** So the secret must be absent from the
 * DOM until the user explicitly clicks, and only then may it appear, only in the known
 * element (Inventory §6, §10.21, R-SEC-4). A page that renders the token on load would
 * hand it to anything that could read this document, which is the entire attack this gate
 * prevents. Re-clicking is harmless.
 *
 * What changed is only what is inside: the per-run runtime token, fetched by the click
 * from `POST /api/pair/authorize`, rather than a long-lived pairing token.
 */
export function Pair() {
  const [token, setToken] = createSignal<string | null>(null);
  const [problem, setProblem] = createSignal<Problem | null>(null);

  async function authorize() {
    setProblem(null);
    try {
      const response = await fetch("/api/pair/authorize", { method: "POST" });
      if (!response.ok) {
        setProblem("refused");
        return;
      }
      const body = (await response.json()) as { token?: string };
      if (body.token) setToken(body.token);
    } catch {
      setProblem("unreachable");
    }
  }

  return (
    <section class="flex flex-col gap-4">
      <h1 class="text-xl font-semibold">{t("system.pair.title")}</h1>
      <p style={{ color: "var(--color-muted)" }}>{t("system.pair.blurb")}</p>

      <button
        type="button"
        onClick={authorize}
        class="w-fit rounded border px-4 py-2"
        style={{ "border-color": "var(--color-line)" }}
      >
        {t("system.pair.authorize")}
      </button>

      <Show when={problem()}>{(kind) => <p role="alert">{t(`system.pair.${kind()}`)}</p>}</Show>

      {token() && (
        <>
          <p class="text-sm" style={{ color: "var(--color-muted)" }}>
            {t("system.pair.done")}
          </p>
          {/*
            The handoff element. It appears ONLY after the click above; the extension's
            content script watches for it with a MutationObserver and applies its own
            gates: exact pathname, this element id, ≤512 characters, printable ASCII
            (R-SEC-4, R-EXT-8).
          */}
          <div id="curio-pairing-handoff" data-curio-pairing-token={token()} hidden />
        </>
      )}
    </section>
  );
}
