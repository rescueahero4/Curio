/**
 * The shared entity caches: items, vocabulary, jobs (R-FE-5).
 *
 * **Store discipline (ARCH-03).** Server truth reaches these stores through exactly two
 * doors: a fetch the user asked for, and an SSE handler. Nothing else writes. In
 * particular there are no optimistic writes here — an edit goes out as a PATCH and comes
 * back as `item.updated`, so the question "who wrote this value" always has an answer.
 * (The one sanctioned exception is ItemDetail's pending-patch guard, R-FE-13, which owns
 * its own in-flight fields and does not live in this file.)
 *
 * The one piece of policy that *is* here is `filterActive`, and it is here reluctantly:
 * `item.created` must be ignored while any filter or search is on (Inventory §10.25), the
 * store is what receives the event, but only the grid knows whether a filter is on. So the
 * grid publishes that fact and the store honours it — rather than the grid subscribing to
 * SSE separately and re-deriving a decision the store has already made.
 */

import { createStore, produce } from "solid-js/store";
import { getVocabulary } from "~/lib/api";
import { events } from "~/lib/events";
import type { Item, JobUpdate, Vocabulary } from "~/lib/types";

interface ItemsState {
  list: Item[];
  /** Set by the grid. True whenever any filter or search narrows the view. */
  filterActive: boolean;
}

interface VocabularyState extends Vocabulary {
  loaded: boolean;
}

interface JobsState {
  byId: Record<string, JobUpdate>;
}

const [itemsState, setItemsState] = createStore<ItemsState>({ list: [], filterActive: false });
const [vocabularyState, setVocabularyState] = createStore<VocabularyState>({
  families: [],
  design_types: [],
  tags: [],
  loaded: false,
});
const [jobsState, setJobsState] = createStore<JobsState>({ byId: {} });

export const items = itemsState;
export const vocabulary = vocabularyState;
export const jobs = jobsState;

/* -- items -------------------------------------------------------------------------- */

/** Land a fetched page. `append` is the infinite-scroll path; `replace` is a new query. */
export function setItemsPage(page: Item[], mode: "replace" | "append"): void {
  if (mode === "replace") {
    setItemsState("list", page);
    return;
  }
  setItemsState(
    "list",
    produce((list: Item[]) => {
      const known = new Set(list.map((item) => item.id));
      for (const item of page) if (!known.has(item.id)) list.push(item);
    }),
  );
}

/** The grid tells the store whether the view is narrowed (Inventory §10.25). */
export function setFilterActive(active: boolean): void {
  setItemsState("filterActive", active);
}

/* -- vocabulary --------------------------------------------------------------------- */

/**
 * Refetch the vocabulary.
 *
 * `vocabulary.updated` carries an empty payload by design: a rename, a merge and a delete
 * each touch several collections and every affected item, so the event says *that*
 * something changed and this asks *what* — reconciling from a diff we were not sent would
 * be guesswork.
 */
export async function refreshVocabulary(): Promise<void> {
  try {
    const next = await getVocabulary();
    setVocabularyState({ ...next, loaded: true });
  } catch {
    // Keep the last good vocabulary on screen. A filter row that empties itself because
    // one request failed is worse than one that is briefly stale.
    console.error("curio: could not refresh the vocabulary");
  }
}

/* -- wiring ------------------------------------------------------------------------- */

/**
 * Subscribe the stores to the shared stream. Called once, from `main.tsx`.
 *
 * Returns a disposer so a test can tear the subscriptions down; the running app never
 * calls it, because these caches live exactly as long as the page does.
 */
export function connectStores(): () => void {
  const offs = [
    events.on("item.created", onItemCreated),
    events.on("item.updated", onItemUpdated),
    events.on("item.deleted", onItemDeleted),
    events.on("job.updated", onJobUpdated),
    events.on("vocabulary.updated", () => void refreshVocabulary()),
  ];
  return () => {
    for (const off of offs) off();
  };
}

function onItemCreated(payload: unknown): void {
  const item = payload as Item;
  if (!item?.id) return;

  // Inventory §10.25. Prepending a card that does not match the active filter is a lie
  // about what the user is looking at; the item is not lost, it appears when they clear
  // the filter and the list reloads.
  if (itemsState.filterActive) return;
  if (indexOf(item.id) >= 0) return;

  setItemsState("list", (list) => [item, ...list]);
}

function onItemUpdated(payload: unknown): void {
  const item = payload as Item;
  if (!item?.id) return;

  const index = indexOf(item.id);
  if (index < 0) return;
  setItemsState("list", index, item);
}

function onItemDeleted(payload: unknown): void {
  const { id } = (payload ?? {}) as { id?: string };
  if (!id) return;
  setItemsState("list", (list) => list.filter((item) => item.id !== id));
}

/**
 * `job.updated` is a full row on enqueue, start and finish — and a partial `{id, result}`
 * on every progress tick during a bulk run. Merging by id is the contract (§10.30);
 * assigning the payload wholesale would blank `kind` and `status` 24 times a minute.
 */
function onJobUpdated(payload: unknown): void {
  const update = payload as JobUpdate;
  if (!update?.id) return;

  setJobsState(
    produce((state: JobsState) => {
      state.byId[update.id] = { ...state.byId[update.id], ...update };
    }),
  );
}

function indexOf(id: string): number {
  return itemsState.list.findIndex((item) => item.id === id);
}
