import { render } from "solid-js/web";
import { App } from "~/App";
import { events } from "~/lib/events";
import { startI18n } from "~/lib/i18n";
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
 *
 * The dictionary is settled before that, and for the same reason: the sign-in-less screen
 * this may fall through to has words on it too, and a frame of English under a Japanese
 * `lang` attribute is a worse first impression than the few milliseconds it costs to wait.
 * For English readers it costs nothing — that dictionary is already in the bundle.
 */
async function start() {
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing from index.html");

  await startI18n();

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
