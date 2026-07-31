import type { Autosave } from "~/components/library/autosave";
import { familyName } from "~/components/library/vocab";
import type { Item, ItemPatch } from "~/lib/types";

/** What the server scores a family a person picked. Mirrored, not invented. */
const HUMAN_PICKED_SCORE = 1;

/**
 * The local echo of a patch.
 *
 * The one sanctioned optimistic write in the app (ARCH-03): a field the user is typing into
 * has to show what they typed, 600 ms before the server confirms it. It is deliberately a
 * pure function of the patch — nothing here invents a value the PATCH did not carry.
 *
 * A newly-linked family is scored 1.0 here because that is what the server will do with it,
 * and showing 0.00 until the response lands would be a number the user would try to explain.
 */
export function applyPatch(item: Item, patch: ItemPatch): Item {
  const next: Item = { ...item };

  if (patch.name !== undefined) next.name = patch.name;
  if (patch.short_description !== undefined) next.short_description = patch.short_description;
  if (patch.source_url !== undefined) next.source_url = patch.source_url;
  if (patch.image_recipe !== undefined) next.image_recipe = patch.image_recipe;
  if (patch.tags !== undefined) next.tags = patch.tags;
  if (patch.design_types !== undefined) next.design_types = patch.design_types;

  if (patch.family_ids !== undefined) {
    next.families = patch.family_ids.map(
      (id) =>
        item.families.find((family) => family.id === id) ?? {
          id,
          name: familyName(id),
          score: HUMAN_PICKED_SCORE,
          gray_zone: false,
          ai_proposed: false,
        },
    );
  }

  return next;
}

/**
 * Server truth, minus whatever the user is still typing (R-FE-13).
 *
 * The echo of our own PATCH is by definition older than the keystrokes that followed it, so
 * a field with an edit queued or in flight keeps the local value and everything else
 * follows the server. Written out field by field rather than looped, because the guard's
 * coverage is a contract and a loop over a key list hides which fields are in it.
 */
export function mergeServer(current: Item, next: Item, autosave: Autosave): Item {
  const merged: Item = { ...next };

  if (autosave.guarded("name")) merged.name = current.name;
  if (autosave.guarded("short_description")) merged.short_description = current.short_description;
  if (autosave.guarded("source_url")) merged.source_url = current.source_url;
  if (autosave.guarded("image_recipe")) merged.image_recipe = current.image_recipe;
  if (autosave.guarded("tags")) merged.tags = current.tags;
  if (autosave.guarded("design_types")) merged.design_types = current.design_types;
  if (autosave.guarded("family_ids")) merged.families = current.families;

  return merged;
}
