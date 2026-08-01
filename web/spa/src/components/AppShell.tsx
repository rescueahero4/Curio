import { useMatch } from "@solidjs/router";
import { createSignal, type ParentProps } from "solid-js";
import { AddItemDialog } from "~/components/AddItemDialog";
import { MissingKeyBanner } from "~/components/MissingKeyBanner";
import { PausedBanner } from "~/components/PausedBanner";
import { TopBar } from "~/components/TopBar";

/**
 * The frame every route renders inside.
 *
 * Both banners live here rather than on the pages they affect, and that placement is the
 * contract: paused (R-FE-8) and missing-key (FR-26) are conditions of the whole app, and a
 * per-page notice would let a user reach a page that never mentions why their edits are
 * being refused.
 *
 * The tray deliberately does not duplicate this navigation. Navigation belongs where it can
 * show the user what they are navigating to; the tray is a switch (D14).
 */
export function AppShell(props: ParentProps) {
  const [addingItem, setAddingItem] = createSignal(false);

  /**
   * The Prompt Helper is a document, not a view of a collection, and it is the one route the
   * container is dropped for. It paints its own desk edge to edge and hangs a rail off the
   * top bar, and neither can happen inside a padded 100rem column — a desk with 24px of page
   * ground either side of it is a rug.
   */
  const document = useMatch(() => "/prompts/:id");

  return (
    <div class="min-h-screen bg-ground text-ink">
      <TopBar onAddItem={() => setAddingItem(true)} />

      <div class="mx-auto w-full max-w-page px-6">
        <PausedBanner />
        <MissingKeyBanner />
      </div>

      <main classList={{ "mx-auto w-full max-w-page px-6 py-6": !document() }}>
        {props.children}
      </main>

      <AddItemDialog open={addingItem()} onClose={() => setAddingItem(false)} />
    </div>
  );
}
