import { createSignal, For, Show } from "solid-js";
import { type Option, OptionList } from "~/components/library/OptionList";
import { Popover } from "~/components/library/Popover";
import { familyOptions, nameOf, termOptions } from "~/components/library/vocab";
import { vocabulary } from "~/lib/stores";
import type { Item, ItemPatch } from "~/lib/types";

/**
 * The item's families, design types and tags.
 *
 * Families are edited as a **whole set** (R-FE-14): the PATCH carries every family the item
 * should end up in, and the server keeps the score of the ones that were already there
 * while scoring the new ones 1.0 — a person is not 87 % sure. That is also why removing the
 * last family is a legitimate save rather than a no-op.
 *
 * Tags and types travel as names, because a name that does not exist yet is created by
 * using it; a family has a description behind it and must be created deliberately.
 */
export function ItemLinks(props: { item: Item; onEdit: (patch: ItemPatch) => void }) {
  const linked = () => props.item.families.map((family) => family.id);

  const setFamilies = (ids: string[]) => props.onEdit({ family_ids: ids });

  return (
    <div class="flex flex-col gap-3">
      <section class="flex flex-col gap-2">
        <h2 class="text-sm font-medium">Families</h2>
        <div class="flex flex-wrap items-center gap-1">
          <For each={props.item.families}>
            {(family) => (
              <span
                class="pill"
                classList={{
                  "tint-proposal": family.ai_proposed,
                  "tint-caution": family.gray_zone && !family.ai_proposed,
                  "pill-outline": !family.ai_proposed && !family.gray_zone,
                }}
              >
                {family.name}
                <span class="numeric text-2xs">{family.score.toFixed(2)}</span>
                <RemoveButton
                  label={`Remove ${family.name}`}
                  onRemove={() => setFamilies(linked().filter((id) => id !== family.id))}
                />
              </span>
            )}
          </For>

          <Popover
            title="Add a family"
            label="+ Family"
            blocked={
              vocabulary.families.length
                ? undefined
                : "No families exist yet. Create one on the Vocabulary page."
            }
          >
            {() => (
              <OptionList
                options={familyOptions()}
                selected={linked()}
                empty="No families yet."
                onToggle={(id) =>
                  setFamilies(
                    linked().includes(id)
                      ? linked().filter((value) => value !== id)
                      : [...linked(), id],
                  )
                }
              />
            )}
          </Popover>
        </div>
        <Show when={props.item.families.some((family) => family.ai_proposed)}>
          <p class="text-xs text-ink-faint">
            Violet means Curio proposed this family rather than matching an existing one.
          </p>
        </Show>
      </section>

      <NameSet
        title="Design types"
        values={props.item.design_types}
        options={termOptions("types")}
        empty="No design types yet — type one below."
        onChange={(next) => props.onEdit({ design_types: next })}
        toName={(id) => nameOf("types", id)}
      />

      <NameSet
        title="Tags"
        values={props.item.tags}
        options={termOptions("tags")}
        empty="No tags yet — type one below."
        onChange={(next) => props.onEdit({ tags: next })}
        toName={(id) => nameOf("tags", id)}
      />
    </div>
  );
}

/** A whole set of names: chips to remove, a picker and a text box to add. */
function NameSet(props: {
  title: string;
  values: string[];
  options: Option[];
  empty: string;
  onChange: (next: string[]) => void;
  toName: (id: string) => string | undefined;
}) {
  const [fresh, setFresh] = createSignal("");

  const add = (name: string) => {
    const trimmed = name.trim();
    if (!trimmed || props.values.includes(trimmed)) return;
    props.onChange([...props.values, trimmed]);
  };

  return (
    <section class="flex flex-col gap-2">
      <h2 class="text-sm font-medium">{props.title}</h2>
      <div class="flex flex-wrap items-center gap-1">
        <For each={props.values}>
          {(value) => (
            <span class="pill pill-outline">
              {value}
              <RemoveButton
                label={`Remove ${value}`}
                onRemove={() => props.onChange(props.values.filter((name) => name !== value))}
              />
            </span>
          )}
        </For>

        <Popover title={`Add to ${props.title.toLowerCase()}`} label="+">
          {(close) => (
            <>
              <OptionList
                options={props.options}
                selected={props.options
                  .filter((option) => props.values.includes(option.label))
                  .map((option) => option.id)}
                empty={props.empty}
                onToggle={(id) => {
                  const name = props.toName(id);
                  if (!name) return;
                  if (props.values.includes(name)) {
                    props.onChange(props.values.filter((value) => value !== name));
                    return;
                  }
                  add(name);
                }}
              />
              <form
                class="flex gap-2"
                onSubmit={(event) => {
                  event.preventDefault();
                  add(fresh());
                  setFresh("");
                  close();
                }}
              >
                <input
                  type="text"
                  class="field field-block"
                  placeholder="New name"
                  value={fresh()}
                  onInput={(event) => setFresh(event.currentTarget.value)}
                />
                <button type="submit" class="pill pill-ink">
                  Add
                </button>
              </form>
            </>
          )}
        </Popover>
      </div>
    </section>
  );
}

function RemoveButton(props: { label: string; onRemove: () => void }) {
  return (
    <button type="button" class="text-ink-faint hover:text-ink" onClick={props.onRemove}>
      <span class="sr-only">{props.label}</span>
      <span aria-hidden="true">×</span>
    </button>
  );
}
