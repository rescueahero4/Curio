/*
 * The bar Curio overlays on a project it is serving, so the versions an agent generated can
 * be told apart and moved between.
 *
 * This runs inside a page Curio did not write, next to CSS and JavaScript it has never seen —
 * the three example prototypes each define their own `.pill`, `.glass`, `.nav`, `.card` and
 * `.btn`. So every line here is written to the same rule: **leave no trace on the document**.
 * A closed shadow root on a hyphenated element, `all: initial` on the host, one capture-phase
 * listener that only ever consumes the chord it owns, no globals, and the whole thing inside a
 * try/catch. If this script fails, the user's design must still be exactly their design.
 *
 * It is served from `/__curio/variant-switcher.js` and injected by `routes/files.rs` — never
 * written to disk, so a project generated before any of this existed still gets it.
 */

(() => {
  "use strict";

  const TAG = "curio-variant-switcher";
  const SHOWN_KEY = "curio.variant-switcher.shown";
  /* Alt, because a prototype's own handlers are Ctrl/Cmd chords or bare letters. */
  const CHORD_LABEL = navigator.platform.startsWith("Mac") ? "⌥V" : "Alt+V";

  /* ---------------------------------------------------------------- the tokens
   *
   * Copied by value from web/spa/src/styles.css. The served page has no Curio custom
   * properties to var() against, so this is the one place in the codebase that restates
   * them — and `styles_and_switcher_agree_on_the_palette` in routes/files.rs fails the
   * build if the two ever drift.
   */
  const CSS = `
    :host {
      /* Inherited properties still cross a shadow boundary, so every one the page could
         have set has to be stated rather than assumed. */
      font-family: system-ui, -apple-system, "Segoe UI", "Helvetica Neue", sans-serif;
      font-size: 0.75rem;
      font-weight: 400;
      font-style: normal;
      line-height: 1.125rem;
      letter-spacing: normal;
      text-transform: none;
      text-align: left;
      color: #1c1917;
    }
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    .bar {
      align-items: center;
      background: color-mix(in srgb, #fafaf9 80%, transparent);
      backdrop-filter: blur(24px) saturate(180%);
      border-bottom: 1px solid #e7e5e4;
      box-shadow: 0 1px 2px rgb(28 25 23 / 0.05), 0 10px 28px -8px rgb(28 25 23 / 0.18);
      display: flex;
      gap: 0.75rem;
      padding: 0.375rem 0.75rem;
      width: 100%;
    }

    /* Where blur is unsupported the fill has to go opaque, or the text stops being legible
       over a dense page — the same rule styles.css keeps for .glass. */
    @supports not (backdrop-filter: blur(1px)) {
      .bar { background: #fafaf9; }
    }

    .mark {
      color: #1c1917;
      flex: none;
      font-size: 0.8125rem;
      font-weight: 600;
      letter-spacing: -0.01em;
    }

    .list {
      align-items: center;
      display: flex;
      flex: 1 1 auto;
      gap: 0.25rem;
      list-style: none;
      /* Never a second row: the bar's height must be predictable over someone's hero. */
      overflow-x: auto;
      scrollbar-width: thin;
    }

    .variant {
      align-items: center;
      border: 1px solid transparent;
      border-radius: 9999px;
      color: #57534e;
      display: inline-flex;
      gap: 0.375rem;
      padding: 0.3125rem 0.75rem;
      text-decoration: none;
      transition: background-color 120ms cubic-bezier(0.4, 0, 0.2, 1),
                  border-color 120ms cubic-bezier(0.4, 0, 0.2, 1),
                  color 120ms cubic-bezier(0.4, 0, 0.2, 1);
      white-space: nowrap;
    }
    .variant:hover { background: #e7e5e4; color: #1c1917; }
    .variant[aria-current="page"] {
      background: #ffffff;
      border-color: #d6d3d1;
      color: #1c1917;
    }

    .name { font-size: 0.8125rem; }

    .chips { align-items: center; display: inline-flex; gap: 0.25rem; }
    .chip {
      align-items: center;
      background: #ffffff;
      border: 1px solid #e7e5e4;
      border-radius: 9999px;
      color: #57534e;
      display: inline-flex;
      font-size: 0.6875rem;
      line-height: 1rem;
      padding: 0.0625rem 0.375rem;
      white-space: nowrap;
    }
    .chip-strong { border-color: #d6d3d1; color: #1c1917; }

    .hide {
      align-items: center;
      background: transparent;
      border: 1px solid #d6d3d1;
      border-radius: 9999px;
      color: #57534e;
      cursor: pointer;
      display: inline-flex;
      flex: none;
      font: inherit;
      font-size: 0.6875rem;
      gap: 0.25rem;
      padding: 0.1875rem 0.5rem;
    }
    .hide:hover { background: #e7e5e4; color: #1c1917; }

    .note { color: #92400e; flex: none; font-size: 0.6875rem; }

    /* ---- hidden ----
     * A three-pixel rule at the very edge of the viewport. Small enough to screenshot
     * around, present enough that the feature is not gone; hovering it says what to press.
     */
    .edge {
      background: linear-gradient(90deg, #d6d3d1, #78716c);
      cursor: pointer;
      height: 3px;
      width: 100%;
    }
    .edge-hint {
      background: color-mix(in srgb, #fafaf9 92%, transparent);
      border: 1px solid #e7e5e4;
      border-radius: 9999px;
      border-top: none;
      color: #57534e;
      font-size: 0.6875rem;
      left: 0.75rem;
      opacity: 0;
      padding: 0.125rem 0.5rem;
      pointer-events: none;
      position: absolute;
      top: 3px;
      transition: opacity 120ms cubic-bezier(0.4, 0, 0.2, 1);
    }
    .edge-wrap:hover .edge-hint, .edge:focus-visible + .edge-hint { opacity: 1; }
    .edge-wrap { position: relative; }

    :focus-visible { outline: 2px solid #1c1917; outline-offset: 2px; }

    @media (prefers-reduced-motion: reduce) {
      .variant, .edge-hint { transition: none; }
    }
  `;

  /* ------------------------------------------------------------------ helpers */

  function remembered() {
    try {
      /* No stored answer means this is the first project ever served. Show the bar, so the
         feature is met once rather than waiting behind a chord nobody was told about. */
      return window.localStorage.getItem(SHOWN_KEY) !== "0";
    } catch {
      return true;
    }
  }

  function remember(shown) {
    try {
      window.localStorage.setItem(SHOWN_KEY, shown ? "1" : "0");
    } catch {
      /* Private modes throw on write. A preference that cannot be saved is not a reason to
         take the bar down. */
    }
  }

  function chipsFor(variant) {
    const chips = [];
    if (variant.family) chips.push({ text: variant.family, strong: true });
    if (variant.design_type) chips.push({ text: variant.design_type, strong: false });
    for (const tag of (variant.tags || []).slice(0, 2)) {
      chips.push({ text: tag, strong: false });
    }
    const hidden = (variant.tags || []).length - 2;
    if (hidden > 0) chips.push({ text: `+${hidden}`, strong: false });
    return chips;
  }

  /* ------------------------------------------------------------------- render */

  function mount(projectId, entry, data) {
    const variants = data.variants || [];
    /* Nothing to switch between. Removing itself is the whole of the right behaviour here:
       a disabled bar reading "1 of 1" is pure noise on somebody's design. */
    if (variants.length < 2) return;

    const host = document.createElement(TAG);
    /* `position` after `all: initial`, which resets it too. This defeats the `* { }` rules
       and the inherited typography every stylesheet on the page is entitled to set. */
    host.style.cssText =
      "all: initial; position: fixed; top: 0; left: 0; right: 0; z-index: 2147483000;";

    const root = host.attachShadow({ mode: "closed" });
    const style = document.createElement("style");
    style.textContent = CSS;
    root.append(style);

    const current = entry.split("/")[0];
    const index = Math.max(
      variants.findIndex((variant) => variant.slug === current || variant.entry === entry),
      0,
    );

    let shown = remembered();

    function draw() {
      for (const child of Array.from(root.children)) {
        if (child !== style) child.remove();
      }
      root.append(shown ? bar() : edge());
    }

    function bar() {
      const wrap = document.createElement("div");
      wrap.className = "bar";
      wrap.setAttribute("role", "region");
      wrap.setAttribute("aria-label", "Curio version switcher");

      const mark = document.createElement("span");
      mark.className = "mark";
      mark.textContent = "Curio";
      wrap.append(mark);

      const list = document.createElement("ul");
      list.className = "list";
      variants.forEach((variant, at) => {
        const item = document.createElement("li");
        /* A real link, not a click handler: middle-click and Cmd-click then open a version
           in a new tab, which is exactly what comparing three designs wants. */
        const link = document.createElement("a");
        link.className = "variant";
        link.href = variant.url;
        if (at === index) link.setAttribute("aria-current", "page");
        if (variant.summary) link.title = variant.summary;

        const name = document.createElement("span");
        name.className = "name";
        name.textContent = variant.name || variant.slug;
        link.append(name);

        const chips = chipsFor(variant);
        if (chips.length) {
          const holder = document.createElement("span");
          holder.className = "chips";
          for (const chip of chips) {
            const el = document.createElement("span");
            el.className = chip.strong ? "chip chip-strong" : "chip";
            el.textContent = chip.text;
            holder.append(el);
          }
          link.append(holder);
        }

        item.append(link);
        list.append(item);
      });
      wrap.append(list);

      if (data.manifest_status === "malformed") {
        /* Said out loud, because it is the user's file and only they can fix it. */
        const note = document.createElement("span");
        note.className = "note";
        note.textContent = "curio-variants.json could not be read";
        note.title = data.manifest_error || "";
        wrap.append(note);
      }

      const hide = document.createElement("button");
      hide.type = "button";
      hide.className = "hide";
      hide.textContent = `Hide ${CHORD_LABEL}`;
      hide.addEventListener("click", () => toggle(false));
      wrap.append(hide);

      return wrap;
    }

    function edge() {
      const wrap = document.createElement("div");
      wrap.className = "edge-wrap";

      const rule = document.createElement("button");
      rule.type = "button";
      rule.className = "edge";
      rule.setAttribute("aria-label", `Show the Curio version switcher (${CHORD_LABEL})`);
      rule.addEventListener("click", () => toggle(true));
      wrap.append(rule);

      const hint = document.createElement("span");
      hint.className = "edge-hint";
      hint.textContent = `Curio · ${index + 1} of ${variants.length} · ${CHORD_LABEL}`;
      wrap.append(hint);

      return wrap;
    }

    function toggle(next) {
      shown = next;
      remember(shown);
      draw();
    }

    window.addEventListener(
      "keydown",
      (event) => {
        if (event.defaultPrevented || event.isComposing) return;

        const target = event.target;
        /* Never take a key from a form on the page being previewed. */
        if (
          target instanceof HTMLElement &&
          (target.isContentEditable ||
            target.tagName === "INPUT" ||
            target.tagName === "TEXTAREA" ||
            target.tagName === "SELECT")
        ) {
          return;
        }

        /* `code`, not `key`: Option+V on macOS produces `√`. */
        const chord =
          event.altKey && !event.ctrlKey && !event.metaKey && event.code === "KeyV";
        if (chord) {
          event.preventDefault();
          /* Only for the chord we consumed. Every other key the page still sees. */
          event.stopPropagation();
          toggle(!shown);
          return;
        }

        if (event.key === "Escape" && shown) {
          /* Deliberately no preventDefault: Escape with our bar open may also mean something
             to the page, and we are the newer, less important layer. */
          toggle(false);
        }
      },
      { capture: true },
    );

    draw();
    /* First child, so a stylesheet's `body > :last-child` or a footer rule is untouched.
       Fixed positioning means DOM order costs nothing visually. */
    document.body.prepend(host);
  }

  /* --------------------------------------------------------------------- boot */

  function start() {
    const tag = document.querySelector("script[data-curio-project]");
    if (!tag) return;

    const projectId = tag.getAttribute("data-curio-project");
    const entry = tag.getAttribute("data-curio-entry") || "";
    if (!projectId) return;

    fetch(`/api/projects/${encodeURIComponent(projectId)}/variants`, {
      credentials: "same-origin",
    })
      .then((response) => (response.ok ? response.json() : null))
      .then((data) => {
        if (data) mount(projectId, entry, data);
      })
      .catch(() => {
        /* Curio paused, restarted, or the project moved. The page is the point; a missing
           toolbar is not worth an error in the user's console. */
      });
  }

  try {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", start, { once: true });
    } else {
      start();
    }
  } catch {
    /* Whatever went wrong here belongs to Curio, not to the page. */
  }
})();
