/**
 * The `/pair` handoff pickup.
 *
 * Injected only on the pairing page, and only for the fallback path — a normally installed
 * extension gets its token from the native-messaging host and this never runs (D11).
 *
 * **The handoff element is the authorization.** The page writes the token into it only
 * after an explicit user click, so this script watches for it appearing. Four gates apply,
 * carried over unchanged because each narrows what a hostile page could hand over
 * (R-EXT-8, R-SEC-4, Inventory §10.21):
 *
 * 1. the exact pathname `/pair`;
 * 2. the exact element id;
 * 3. at most 512 characters;
 * 4. printable ASCII only.
 *
 * Idempotent: picking up the same token twice is harmless, and re-clicking on the page is
 * expected.
 *
 * **P5.**
 */

const ELEMENT_ID = "curio-pairing-handoff";
const ATTRIBUTE = "data-curio-pairing-token";
const MAX_TOKEN_LENGTH = 512;

/**
 * Whether a value is shaped like a token we would accept.
 *
 * The gates are about limiting blast radius, not verifying the secret — only the server
 * can do that. What they prevent is a page stuffing a megabyte of markup, or control
 * characters, into something the extension will later put in an HTTP header.
 */
export function isAcceptableToken(value: string | null): value is string {
  if (!value) return false;
  if (value.length > MAX_TOKEN_LENGTH) return false;
  // Printable ASCII, no control characters, no whitespace.
  return /^[\x21-\x7e]+$/.test(value);
}

/** Whether this document is the pairing page. */
export function isPairPage(pathname: string): boolean {
  return pathname === "/pair";
}

function pickUp(): void {
  if (!isPairPage(window.location.pathname)) return;

  const element = document.getElementById(ELEMENT_ID);
  const token = element?.getAttribute(ATTRIBUTE) ?? null;
  if (!isAcceptableToken(token)) return;

  void chrome.runtime.sendMessage({ type: "pairing-token", token });
}

// The element appears on click, not on load, so a one-shot read would almost always miss
// it.
const observer = new MutationObserver(pickUp);
observer.observe(document.documentElement, {
  childList: true,
  subtree: true,
  attributes: true,
  attributeFilter: [ATTRIBUTE],
});

pickUp();
