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

  return (
    <div class="min-h-screen bg-ground text-ink">
      <TopBar onAddItem={() => setAddingItem(true)} />

      <div class="mx-auto w-full max-w-7xl px-6">
        <PausedBanner />
        <MissingKeyBanner />
      </div>

      <main class="mx-auto w-full max-w-7xl px-6 py-6">{props.children}</main>

      <AddItemDialog open={addingItem()} onClose={() => setAddingItem(false)} />
    </div>
  );
}
