import { createSignal } from "solid-js";

/**
 * The pairing fallback (R-FE-19, D11).
 *
 * Normally unnecessary: a properly installed extension gets `{port, token, state}` from
 * the native-messaging host and never comes here. This page covers what native messaging
 * cannot — unpacked development installs with no registered NM manifest, a user who
 * declined the installer's registration step, or a future browser without NM support.
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
  const [error, setError] = createSignal<string | null>(null);

  async function authorize() {
    setError(null);
    try {
      const response = await fetch("/api/pair/authorize", { method: "POST" });
      if (!response.ok) {
        setError("Curio refused the request. Open the dashboard from the tray and retry.");
        return;
      }
      const body = (await response.json()) as { token?: string };
      if (body.token) setToken(body.token);
    } catch {
      setError("Could not reach Curio.");
    }
  }

  return (
    <section class="flex flex-col gap-4">
      <h1 class="text-xl font-semibold">Authorize this browser</h1>
      <p style={{ color: "var(--color-muted)" }}>
        The Curio extension normally connects on its own. Use this only if it cannot — typically a
        development install loaded unpacked.
      </p>

      <button
        type="button"
        onClick={authorize}
        class="w-fit rounded border px-4 py-2"
        style={{ "border-color": "var(--color-line)" }}
      >
        Authorize this browser
      </button>

      {error() && <p role="alert">{error()}</p>}

      {token() && (
        <>
          <p class="text-sm" style={{ color: "var(--color-muted)" }}>
            Authorized. The extension picks this up automatically — you can close this tab.
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
