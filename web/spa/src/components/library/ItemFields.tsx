import { Show } from "solid-js";
import { ExternalLink } from "~/components/icons";
import type { SaveState } from "~/components/library/autosave";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Item, ItemPatch } from "~/lib/types";

/**
 * The editable text of an item (FR-8).
 *
 * There is no Save button, and that is the design: edits go out 600 ms after the typing
 * stops. What replaces the button is an honest status line — "Saving…", "Saved", or the
 * reason it did not — because an autosaving form that says nothing is indistinguishable
 * from one that is losing work.
 */
export function ItemFields(props: {
  item: Item;
  state: SaveState;
  problem: string | null;
  onEdit: (patch: ItemPatch) => void;
}) {
  const blocked = () => (paused() ? t("library.item.fields.paused") : undefined);

  /**
   * The source URL, if it is one a browser should be sent to.
   *
   * `source_url` is whatever the extension captured or a user typed, and nothing validates
   * it on the way in — it can be a half-typed host, or a `javascript:` string. So the link
   * appears only for http and https, and the field is left to say the rest.
   */
  const openable = () => {
    const raw = props.item.source_url?.trim();
    if (!raw) return null;
    try {
      const url = new URL(raw);
      return url.protocol === "http:" || url.protocol === "https:" ? url.href : null;
    } catch {
      return null;
    }
  };

  return (
    <div class="flex flex-col gap-3">
      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">{t("library.item.fields.name")}</span>
        <input
          type="text"
          class="field field-block text-lg"
          value={props.item.name}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) => props.onEdit({ name: event.currentTarget.value })}
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">{t("library.item.fields.description")}</span>
        <textarea
          class="field field-block"
          rows="3"
          value={props.item.short_description}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) => props.onEdit({ short_description: event.currentTarget.value })}
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">{t("library.item.fields.sourceUrl")}</span>
        {/* The field stays a field — this is still the place the URL is edited. The link is
            an affordance laid into it, because the other thing anyone does with a source URL
            is go and look at it, and select-all-copy-new-tab-paste is four gestures for
            something the browser can do in one. It only appears once the value is a URL a
            browser would actually follow; see `openable`. */}
        <div class="relative">
          <input
            type="url"
            class="field field-block pr-10"
            placeholder="https://"
            value={props.item.source_url ?? ""}
            disabled={!!blocked()}
            title={blocked()}
            onInput={(event) =>
              props.onEdit({ source_url: event.currentTarget.value.trim() || null })
            }
          />
          <Show when={openable()}>
            {(href) => (
              <a
                href={href()}
                target="_blank"
                rel="noreferrer noopener"
                class="pill pill-icon absolute top-1/2 right-1 -translate-y-1/2"
                title={t("library.item.fields.openSource")}
              >
                <span class="sr-only">{t("library.item.fields.openSourceLabel")}</span>
                <ExternalLink />
              </a>
            )}
          </Show>
        </div>
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">{t("library.item.fields.imagePrompt")}</span>
        <textarea
          class="field field-block font-mono text-xs"
          rows="4"
          value={props.item.image_recipe ?? ""}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) =>
            props.onEdit({ image_recipe: event.currentTarget.value.trim() || null })
          }
        />
      </label>

      <p class="text-xs text-ink-faint" aria-live="polite">
        <Show when={props.problem} fallback={<Status state={props.state} item={props.item} />}>
          {(message) => <span class="text-caution">{message()}</span>}
        </Show>
      </p>
    </div>
  );
}

function Status(props: { state: SaveState; item: Item }) {
  return (
    <Show
      when={props.state === "idle"}
      fallback={props.state === "saving" ? t("common.saving") : t("library.item.fields.saved")}
    >
      {props.item.last_edited_by === "user"
        ? t("library.item.fields.byUser")
        : t("library.item.fields.byCurio")}
    </Show>
  );
}
