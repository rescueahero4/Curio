/**
 * Vocabulary strings, Japanese.
 *
 * 語彙 for "vocabulary" throughout, including the page heading — it is the word the glossary
 * settled on and the one the Settings link points at.
 *
 * Merge is 統合 and its select is labelled 統合先, "the thing merged into". English can hang
 * "Merge into" over a dropdown and let the preposition point at it; Japanese marks the role
 * on the noun instead, which is why the label is a noun here and a verb phrase there.
 *
 * The destructive confirmations are deliberately flat — 削除しますか。 and 削除する, not
 * よろしいですか or a softened 削除してもよろしいでしょうか. A merge folds a word into
 * another one across every item that carries it and cannot be undone, and a question that
 * hedges about it is a question the reader skims.
 */

import { template } from "@solid-primitives/i18n";
import type { Vocabulary } from "~/lib/i18n/locales/en/vocabulary";

export const vocabulary: Vocabulary = {
  title: "語彙",
  blurb:
    "ここにあるのは、Curio がライブラリを説明するために使うすべての名前です。名前を変更すればすべてのアイテムに反映され、同じ意味だとわかった 2 つは統合できます。",
  loading: "語彙を読み込んでいます…",

  clearSelection: "選択を解除",

  confirmDelete: "削除する",

  tabs: {
    label: "語彙のコレクション",
    families: "ファミリー",
    types: "デザインタイプ",
    tags: "タグ",
  },

  // One word where English has two: 件 counts, so 1 件 and 30 件 take the same noun.
  kinds: {
    families: { one: "ファミリー", other: "ファミリー" },
    types: { one: "デザインタイプ", other: "デザインタイプ" },
    tags: { one: "タグ", other: "タグ" },
  },

  fields: {
    name: "名前",
    description: "説明",
  },

  origin: {
    label: "命名者",
    // "Anyone" is a filter that removes the filter, and すべて is what that reads as here.
    anyone: "すべて",
    ai: "Curio",
    // 自分 rather than あなた: an interface talking about the reader in the second person
    // reads as a translation almost every time.
    user: "自分",
  },

  table: {
    // Reversed against the English: Japanese states the whole before the part.
    shown: template<{ shown: number; total: number }>("{{ total }} 件中 {{ shown }} 件"),
    search: {
      placeholder: "検索",
      label: template<{ noun: string }>("{{ noun }} を検索"),
    },
    columns: {
      items: "アイテム数",
      actions: "操作",
    },
    selectAll: template<{ count: number }>("表示中の {{ count }} 件をすべて選択"),
    empty: {
      filtered: "この条件に一致するものはありません。",
      none: template<{ noun: string }>(
        "{{ noun }} はまだありません。Curio がキャプチャを説明しながら追加していきます。上の「追加」から自分で登録することもできます。",
      ),
    },
  },

  row: {
    noDescription: "説明なし",
    rename: "名前を変更",
    unchanged: "名前が変更されていません。",
    descriptionHint:
      "Curio はこのファミリーに何が当てはまるかを判断するときにこの説明を読みます。プロンプトではファミリーのチップがこの説明に展開されます。例を並べるよりも、雰囲気を言葉にするほうが役に立ちます。",
    saveDescription: "説明を保存",
    confirm: template<{ name: string; count: number }>(
      "{{ name }} を削除しますか。{{ count }} 件のアイテムはそのまま残り、この語が外れるだけです。",
    ),
    keep: "やめる",
  },

  merge: {
    into: "統合先",
    choose: "選択してください",
    empty: "統合先にできる語がほかにありません。",
    action: template<{ name: string; target: string }>("{{ name }} を {{ target }} に統合"),
    hint: template<{ name: string }>(
      "{{ name }} が付いているアイテムはそのまま残り、名前が統合先のものに変わります。",
    ),
  },

  bulk: {
    selected: template<{ count: number }>("{{ count }} 件を選択中"),
    progress: template<{ done: number; total: number }>("{{ total }} 件中 {{ done }} 件…"),
    mergeEmpty: "統合先にできる語が残っていません。",
    merge: template<{ count: number; target: string }>("{{ count }} 件を {{ target }} に統合"),
    // The same sentence at both keys. English needs "this word" and "these words"; Japanese
    // does not, and writing two variants anyway would invite them to drift.
    confirmOne: template<{ count: number; noun: string }>(
      "{{ noun }} {{ count }} 件を削除しますか。アイテムはそのまま残り、この語が外れるだけです。",
    ),
    confirmOther: template<{ count: number; noun: string }>(
      "{{ noun }} {{ count }} 件を削除しますか。アイテムはそのまま残り、この語が外れるだけです。",
    ),
    keep: "やめる",
    clear: "選択を解除",

    result: {
      deleted: template<{ count: number; noun: string }>(
        "{{ noun }} {{ count }} 件を削除しました。",
      ),
      merged: template<{ count: number; noun: string; target: string }>(
        "{{ noun }} {{ count }} 件を {{ target }} に統合しました。",
      ),
      // Named first, counted second: the reader wants to know which ones before how many.
      refused: template<{ count: number; names: string; why: string }>(
        "実行できなかったのは {{ names }} の {{ count }} 件です。{{ why }}",
      ),
      nothing: template<{ count: number; names: string; why: string }>(
        "何も変更されていません。実行できなかったのは {{ names }} の {{ count }} 件です。{{ why }}",
      ),
      separator: "、",
      // Nothing: 。 already closes the sentence and a space after it reads as a gap.
      spacer: "",
    },
  },

  add: {
    label: "追加",
    title: "語彙に追加",
    // 美的ファミリー adds a word without adding a meaning here — the menu has three items and
    // ファミリー is already the only one it could be.
    kinds: {
      families: "ファミリー",
      types: "デザインタイプ",
      tags: "タグ",
    },
    back: "コレクションの一覧に戻る",
    heading: template<{ noun: string }>("新しい {{ noun }}"),
    descriptionHint: "Curio が照合の基準にする説明であり、プロンプトでチップが展開される内容です。",
    submit: "追加",
    busy: "追加中…",
    needName: template<{ noun: string }>("{{ noun }} の名前を入力してください。"),
    paused: "Curio は一時停止中です。名前を追加するにはトレイアイコンから再開してください。",
    failed: "追加できませんでした。",
  },

  blocked: {
    paused: "Curio は一時停止中です。語彙を編集するにはトレイアイコンから再開してください。",
    busy: "処理中…",
  },

  errors: {
    generic: "変更は反映されませんでした。",
    paused: "Curio は一時停止中です。トレイアイコンから再開してください。",
  },
};
