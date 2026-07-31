import { render } from "solid-js/web";
import { App } from "~/App";
import { events } from "~/lib/events";
import { bootstrapSession } from "~/lib/session";
import { connectStores } from "~/lib/stores";
import { NoSession } from "~/routes/NoSession";
import "~/styles.css";

/**
 * Entry point.
 *
 * The session is established **before** the app renders. Mounting first and authenticating
 * afterwards would flash a shell that immediately fills with 401s, which reads as breakage
 * rather than as "you need to open this from the tray".
 */
async function start() {
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing from index.html");

  const session = await bootstrapSession();

  if (session.kind === "authenticated") {
    // Subscribed before the stream opens, so the caches do not miss an event that arrives
    // between connecting and the first component mounting (R-FE-5, ARCH-03 store discipline).
    connectStores();
    events.connect();
    render(() => <App />, root);
    return;
  }

  // Never an error page (R-FE-6a). A bookmarked tab, or one restored after a restart, is
  // an ordinary situation with an ordinary answer: open Curio from the tray.
  render(() => <NoSession reachable={session.kind === "no-session"} />, root);
}

void start();
