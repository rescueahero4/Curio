import { createSignal } from "solid-js";

/** R-FE-9 names the key. It is part of the contract, not an implementation detail. */
const KEY = "curio.dense";

/**
 * The grid's density, remembered across sessions.
 *
 * `localStorage` can throw — private-mode Safari, a disabled-storage policy — and a
 * preference about card size is never worth failing a page render for, so both sides are
 * guarded and the default is simply "not dense".
 */
export function createDensity() {
  const [dense, setDense] = createSignal(read());

  return [
    dense,
    (next: boolean) => {
      setDense(next);
      try {
        localStorage.setItem(KEY, next ? "1" : "0");
      } catch {
        // Nothing to recover: the setting holds for this session and is re-chosen later.
      }
    },
  ] as const;
}

function read(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}
