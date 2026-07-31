import { A } from "@solidjs/router";

/**
 * The client-side 404 (R-FE-2).
 *
 * Distinct from the server's: the server 404s `/api`, `/files`, `/p/` and build-asset paths
 * rather than serving the shell, so anything that reaches this component is a real
 * dashboard URL that does not exist — a typo or a stale link, not a routing failure.
 */
export function NotFound() {
  return (
    <section class="flex flex-col gap-3">
      <h1 class="text-xl font-semibold">Not found</h1>
      <p style={{ color: "var(--color-muted)" }}>
        <A href="/">Back to the library</A>
      </p>
    </section>
  );
}
