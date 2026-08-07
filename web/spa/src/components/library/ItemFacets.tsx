import { For, Show } from "solid-js";
import { listOf } from "~/components/library/vocab";
import { t } from "~/lib/i18n";
import type { Item } from "~/lib/types";

/**
 * The source host, which is the part of a URL a list can actually use.
 *
 * A full `https://www.example.com/some/deep/path?utm_source=…` is mostly noise at list
 * width: it truncates to the part that reads the same on every row. The host is what
 * identifies where a capture came from, which is the question the column exists to answer.
 */
export function hostOf(url: string | null): string | null {
  if (!url) return null;
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    // `source_url` is whatever the extension captured or the user typed, and nothing
    // validates it on the way in. Something unparseable still identifies the item to the
    // person who saved it, so it is shown as-is rather than dropped.
    return url;
  }
}

interface Facet {
  kind: "family" | "type";
  name: string;
}

/**
 * An item's vocabulary in a listing: aesthetic families first, then design types, capped.
 *
 * **Capped is the point.** An item can carry a dozen terms, and a card that renders all of
 * them stops being a fixed-height tile — the grid goes ragged and the row wraps to two
 * lines. So a listing shows the first few and says how many it is not showing; the item
 * page is where the complete set lives. The overflow marker carries the rest in its
 * tooltip, so nothing is hidden without a way to see it.
 *
 * Families come first because they are the taxonomy the product is organised around, and
 * they carry the heavier treatment — the strong line and full ink against the type's hairline
 * and muted ink. Both are bordered: see `.badge` for why a bare one does not work in a row.
 *
 * The AI's *proposed* family is excluded here. It is not a fact about the item yet, and the
 * card says so separately in the proposal tint.
 */
export function ItemFacets(props: { item: Item; limit: number }) {
  const facets = (): Facet[] => [
    ...props.item.families
      .filter((family) => !family.ai_proposed)
      .map((family) => ({ kind: "family" as const, name: family.name })),
    ...props.item.design_types.map((name) => ({ kind: "type" as const, name })),
  ];

  const shown = () => facets().slice(0, props.limit);
  const hidden = () => facets().slice(props.limit);
  const rest = () => listOf(hidden().map((facet) => facet.name));

  return (
    <>
      <For each={shown()}>
        {(facet) => (
          <span
            class="badge"
            classList={{ "badge-strong": facet.kind === "family" }}
            title={
              facet.kind === "family"
                ? t("library.card.facets.family", { name: facet.name })
                : t("library.card.facets.type", { name: facet.name })
            }
          >
            {facet.name}
          </span>
        )}
      </For>

      <Show when={hidden().length}>
        <span class="numeric text-2xs text-ink-faint" title={rest()}>
          +{hidden().length}
        </span>
      </Show>
    </>
  );
}
