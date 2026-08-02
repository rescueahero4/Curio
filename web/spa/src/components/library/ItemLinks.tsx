import { createSignal, For, Show } from "solid-js";
import { type Option, OptionList } from "~/components/library/OptionList";
import { Popover } from "~/components/library/Popover";
import { familyOptions, nameOf, termOptions } from "~/components/library/vocab";
import { createTerm } from "~/lib/api";
import { ApiError } from "~/lib/http";
import { refreshVocabulary, vocabulary } from "~/lib/stores";
import type { Item, ItemPatch } from "~/lib/types";

/**
 * The item's families, design types and tags.
 *
 * Families are edited as a **whole set** (R-FE-14): the PATCH carries every family the item
 * should end up in, and the server keeps the score of the ones that were already there
 * while scoring the new ones 1.0 — a person is not 87 % sure. That is also why removing the
 * last family is a legitimate save rather than a no-op.
 *
 * All three sections use one picker (`VocabPicker`). What differs between them is not the
 * control but what "create" means underneath: a tag or a type is minted by the server the
 * first time a name is used, so adding it to the item is the whole of it, while a family is
 * a row of its own that must exist before anything can link to it.
 */
export function ItemLinks(props: { item: Item; onEdit: (patch: ItemPatch) => void }) {
  const [problem, setProblem] = createSignal<string | null>(null);

  const linked = () => props.item.families.map((family) => family.id);
  const setFamilies = (ids: string[]) => props.onEdit({ family_ids: ids });

  /**
   * Create a family and link it in one gesture.
   *
   * Two steps rather than one because the item→family link is by id, and a name that has
   * never been saved does not have one yet. The id is recovered by refreshing the
   * vocabulary and looking the name back up rather than by reading it out of the POST
   * response: the store has to be refreshed regardless, or the picker would not list the
   * family that was just made, and looking it up there keeps this from depending on a
   * response body's exact shape.
   */
  async function createFamily(name: string) {
    setProblem(null);
    try {
      await createTerm("families", { name });
      await refreshVocabulary();

      const created = vocabulary.families.find(
        (family) => family.name.toLowerCase() === name.toLowerCase(),
      );
      if (!created) {
        setProblem(`“${name}” was created but could not be linked. Reload and add it again.`);
        return;
      }
      setFamilies([...linked(), created.id]);
    } catch (error) {
      setProblem(error instanceof ApiError ? error.message : `Could not create “${name}”.`);
    }
  }

  /**
   * A family made here has a name and nothing else, and its description is the thing Curio
   * actually matches new captures against. Said only while it is true of a family on this
   * item, so it is a prompt to finish a specific job rather than standing advice.
   */
  const undescribed = () =>
    props.item.families.some((linkedFamily) =>
      vocabulary.families.some(
        (family) => family.id === linkedFamily.id && !family.description.trim(),
      ),
    );

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

          <VocabPicker
            noun="Aesthetic Family"
            title="Add a family"
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
            onCreate={(name) => void createFamily(name)}
          />
        </div>

        <Show when={props.item.families.some((family) => family.ai_proposed)}>
          <p class="text-xs text-ink-faint">
            Violet means Curio proposed this family rather than matching an existing one.
          </p>
        </Show>

        <Show when={undescribed()}>
          <p class="text-xs text-ink-faint">
            A family here has no description yet. Curio matches new captures against that
            description, so add one on the Vocabulary page for it to affect anything.
          </p>
        </Show>

        <Show when={problem()}>
          {(message) => <output class="banner tint-caution">{message()}</output>}
        </Show>
      </section>

      <NameSet
        title="Design types"
        noun="Design Type"
        values={props.item.design_types}
        options={termOptions("types")}
        empty="No design types yet."
        onChange={(next) => props.onEdit({ design_types: next })}
        toName={(id) => nameOf("types", id)}
      />

      <NameSet
        title="Tags"
        noun="Tag"
        values={props.item.tags}
        options={termOptions("tags")}
        empty="No tags yet."
        onChange={(next) => props.onEdit({ tags: next })}
        toName={(id) => nameOf("tags", id)}
      />
    </div>
  );
}

/**
 * A whole set of names: chips to remove, and one picker to add or create.
 *
 * Creating is just adding here. A tag or a design type is brought into existence by being
 * used, so a name that is not in the list yet takes the same path as one that is — which is
 * why this needs no request of its own and no error state.
 */
function NameSet(props: {
  title: string;
  noun: string;
  values: string[];
  options: Option[];
  empty: string;
  onChange: (next: string[]) => void;
  toName: (id: string) => string | undefined;
}) {
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

        <VocabPicker
          noun={props.noun}
          title={`Add to ${props.title.toLowerCase()}`}
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
          onCreate={add}
        />
      </div>
    </section>
  );
}

/**
 * The one picker, for all three vocabularies.
 *
 * It is deliberately thin: a trigger and a list whose single text box both filters and
 * creates. Everything that differs between a family, a design type and a tag is a prop —
 * the noun the offer is phrased with, and what `onCreate` actually does.
 */
function VocabPicker(props: {
  noun: string;
  title: string;
  options: Option[];
  selected: string[];
  empty: string;
  onToggle: (id: string) => void;
  onCreate: (name: string) => void;
}) {
  return (
    <Popover title={props.title} label="+">
      {() => (
        <OptionList
          options={props.options}
          selected={props.selected}
          empty={props.empty}
          onToggle={props.onToggle}
          create={{ noun: props.noun, onCreate: props.onCreate }}
        />
      )}
    </Popover>
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
