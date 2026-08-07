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
 * The delete confirmations are deliberately flat — 削除しますか。 and 削除する, not
 * よろしいですか. A bulk delete takes a word off every item that carries it, and a question
 * that hedges about that is a question the reader skims. 削除する is a dictionary-form verb
 * where the README asks for bare nouns on buttons: the pair 削除する / 残す reads as two
 * answers to the question above them, where 削除 / 残す reads as one label beside one verb.
 *
 * Names that come from the data are wrapped in 「」, following `ja/library.ts`. A term is
 * free text and can be a word this sentence also uses — a tag literally named 削除 is legal —
 * so the quotes are what keep 「削除」を削除しますか readable. They are written into the
 * template for the single-name slots (`{{ name }}`, `{{ target }}`) and left out of
 * `{{ names }}`, which is not one name: `quotedList` has already bracketed each term in it,
 * and one pair of marks around the whole run would read as a single name containing a
 * separator. Either way the slot ends in a bracket, which is why none of them takes a space
 * before the particle that follows.
 */

import { template } from "@solid-primitives/i18n";
import type { Vocabulary } from "~/lib/i18n/locales/en/vocabulary";

export const vocabulary: Vocabulary = {
  title: "語彙",
  blurb:
    "ここにあるのは、Curio がライブラリを説明するために使うすべての名前です。名前を変更すればすべてのアイテムに反映され、同じ意味だとわかった 2 つは統合できます。",
  loading: "語彙を読み込んでいます…",
  // Not 再試行. This asks for the whole page again, not for one write to be re-attempted.
  retry: "もう一度試す",

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
      // No space before を: the slot holds a Japanese noun, not a Latin run. The resolver
      // only eats what is inside the braces, so a space written outside them is printed.
      label: template<{ noun: string }>("{{ noun }}を検索"),
    },
    columns: {
      items: "アイテム数",
      actions: "操作",
    },
    selectAll: template<{ count: number }>("表示中の {{ count }} 件をすべて選択"),
    clearSelection: "選択を解除",
    empty: {
      // フィルター is the glossary's word and the one on the control directly above this.
      filtered: "このフィルターに一致するものはありません。",
      none: template<{ noun: string }>(
        "{{ noun }}はまだありません。Curio がキャプチャを説明しながら追加していきます。上から自分で追加することもできます。",
      ),
    },
  },

  row: {
    noDescription: "説明なし",
    rename: "名前を変更",
    unchanged: "名前が変わっていません。",
    descriptionHint:
      "Curio はここに何が当てはまるかを判断するときにこの説明を読みます。プロンプトではファミリーのチップがこの説明に展開されます。例を並べるよりも、雰囲気を言葉にするほうが役に立ちます。",
    saveDescription: "説明を保存",
    confirm: template<{ name: string; count: number }>(
      "「{{ name }}」を削除しますか。{{ count }} 件のアイテムはそのまま残り、この語が外れるだけです。",
    ),
    // 残す, not やめる. The English chose "Keep it" over "Cancel" to say what happens to the
    // term; やめる would only say what happens to the click, which is キャンセル in other
    // letters and the reassurance the confirmation rests on would be gone.
    keep: "残す",
  },

  merge: {
    into: "統合先",
    choose: "選択…",
    empty: "統合先にできる語がほかにありません。",
    action: template<{ name: string; target: string }>("「{{ name }}」を「{{ target }}」に統合"),
    hint: template<{ name: string }>(
      "「{{ name }}」が付いているアイテムはそのまま残り、名前が統合先のものに変わります。",
    ),
  },

  bulk: {
    selected: template<{ count: number }>("{{ count }} 件を選択中"),
    progress: template<{ done: number; total: number }>("{{ total }} 件中 {{ done }} 件…"),
    mergeEmpty: "統合先にできる語が残っていません。",
    merge: template<{ count: number; target: string }>("{{ count }} 件を「{{ target }}」に統合"),
    // The same sentence at both keys. English needs "this word" and "these words"; Japanese
    // does not, and writing two variants anyway would invite them to drift.
    confirmOne: template<{ count: number; noun: string }>(
      "{{ noun }} {{ count }} 件を削除しますか。アイテムはそのまま残り、この語が外れるだけです。",
    ),
    confirmOther: template<{ count: number; noun: string }>(
      "{{ noun }} {{ count }} 件を削除しますか。アイテムはそのまま残り、この語が外れるだけです。",
    ),
    keep: "残す",
    // Short, because the pill has the rest of the bar around it to say what is being cleared.
    clear: "解除",

    result: {
      deleted: template<{ count: number; noun: string }>(
        "{{ noun }} {{ count }} 件を削除しました。",
      ),
      merged: template<{ count: number; noun: string; target: string }>(
        "{{ noun }} {{ count }} 件を「{{ target }}」に統合しました。",
      ),
      /*
       * Named first, counted second: the reader wants to know which ones before how many.
       *
       * No space on either side of the slot. `quotedList` delimits every name itself —
       * 「和モダン」「ミニマル」 — so the run already ends in a bracket and 「」の reads
       * correctly whether the names came out Japanese or Latin. A space would be the same
       * defect as 「タグ を検索」, and only visible on the half of the libraries whose terms
       * are not English.
       */
      refused: template<{ count: number; names: string }>(
        "実行できなかったのは{{ names }}の {{ count }} 件です。",
      ),
      nothing: template<{ count: number; names: string }>(
        "何も変更されていません。実行できなかったのは{{ names }}の {{ count }} 件です。",
      ),
    },
  },

  add: {
    label: "追加",
    title: "語彙に追加",
    // 美的ファミリー adds a word without adding a meaning here, and ja/library.ts already
    // calls this ファミリー everywhere the reader will have met it.
    kinds: {
      families: "ファミリー",
      types: "デザインタイプ",
      tags: "タグ",
    },
    back: "コレクションの一覧に戻る",
    heading: template<{ noun: string }>("新しい{{ noun }}"),
    descriptionHint: "Curio が照合に使う説明です。プロンプトではチップがこの内容に展開されます。",
    submit: "追加",
    busy: "追加中…",
    needName: template<{ noun: string }>("まず{{ noun }}の名前を入力してください。"),
    paused:
      "Curio は一時停止中です。名前を追加するにはトレイアイコンから「Resume」を選んでください。",
    failed: "追加できませんでした。",
    // No 統合してください here: this popover has no merge control to point at.
    taken: "その名前はすでに使われています。",
  },

  blocked: {
    paused:
      "Curio は一時停止中です。語彙を編集するにはトレイアイコンから「Resume」を選んでください。",
    busy: "処理中…",
  },

  errors: {
    // The app owns the failure rather than reporting it as weather.
    generic: "変更を保存できませんでした。",
    paused: "Curio は一時停止中です。トレイアイコンから「Resume」を選んでください。",
    taken: "その名前はすでに使われています。同じ意味なら統合してください。",
  },
};
