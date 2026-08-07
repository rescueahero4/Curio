/**
 * Editor strings, Japanese.
 *
 * The rail is nouns — 太字, 引用, 挿入 — because that is what sits on a Japanese formatting
 * control; the tooltips that explain a control are です・ます調, because they are sentences.
 *
 * `Markdown`, `Tab`, `Enter`, `Esc` and `Curio` stay Latin: they are a format name, three
 * key names and a product name, and a Japanese interface writes all five exactly as they
 * are. So does the `/` in the empty state — it is the key the reader has to press.
 */

import { template } from "@solid-primitives/i18n";
import type { Editor } from "~/lib/i18n/locales/en/editor";

export const editor: Editor = {
  list: {
    title: "プロンプト",
    new: "新規プロンプト",
    newHint: "テンプレートから書き始めます",
    loading: "プロンプトを読み込んでいます…",
    // Leading with 最終編集 rather than trailing に編集: the stamp is the variable part, and
    // a list row is scanned down its left edge.
    edited: template<{ when: string }>("最終編集 {{ when }}"),
    reuse: "再利用",
    deleteHint: "このプロンプトを削除します",

    empty: {
      title: "プロンプトはまだありません。",
      blurb:
        "新規プロンプトは Curio のお手本テンプレートから始まります。書き足すのも、要らない" +
        "ところを消すのも自由です。本文のどこかで / を入力すると、ライブラリからファミリー、" +
        "タグ、デザインタイプ、アイテムを引き出せます。",
    },

    failed: {
      delete: "このプロンプトを削除できませんでした。",
      create: "新しいプロンプトを作成できませんでした。",
    },
  },

  document: {
    opening: "プロンプトを開いています…",
    note:
      "挿入したチップは、プロンプトを書き出すときに展開されます。ファミリーは説明文ごと、" +
      "参照アイテムはフォルダーの絶対パスごと入るので、受け取ったエージェントはスクリーン" +
      "ショットとサイドカーをそのままディスクから読めます。",

    source: {
      reading: "Markdown を読み込んでいます…",
      empty: "このプロンプトは空です。",
    },

    failed: {
      save: "直前の編集を保存できませんでした。",
      source: "Markdown を読み込めませんでした。",
    },
  },

  toolbar: {
    label: "プロンプト本文",
    undo: "元に戻す",
    redo: "やり直す",

    block: {
      title: "ブロックの種類",
      // The template's section names are H2s, so this is the drop-down's "not a heading"
      // entry. 本文 rather than テキスト — it is the body of the document, not a text field.
      text: "本文",
      heading: template<{ level: number }>("見出し {{ level }}"),
    },

    bold: "太字",
    italic: "斜体",
    code: "インラインコード",
    bulletList: "箇条書き",
    orderedList: "番号付きリスト",
    quote: "引用",
    codeBlock: "コードブロック",
    divider: "区切り線",

    insert: {
      label: "挿入",
      title: "ライブラリから挿入",
    },

    source: {
      label: "Markdown",
      hint: "コピーしたときに出力されるテキストを表示します",
      blocked: "Markdown 表示は読み取り専用です。編集するには表示を戻してください",
    },
  },

  actions: {
    busy: "Curio が直前の操作を処理しています",

    copy: {
      label: "プロンプトをコピー",
      hint: "このプロンプトを書き出してクリップボードにコピーします",
      done: "クリップボードにコピーしました",
      refused: "クリップボードに書き込めませんでした。何もコピーされていません。",
    },

    delete: {
      label: "プロンプトを削除",
      hint: "このプロンプトを削除します",
      // 残す, not キャンセル: the English names what happens to the prompt, and so should
      // this. キャンセル would name what happens to the dialog.
      keep: "残す",
      confirm: "完全に削除",
    },

    failed: "問題が発生しました。",
  },

  slash: {
    label: "ライブラリから挿入",
    // No space before から: `kind` is Japanese here, not a Latin run.
    pick: template<{ kind: string }>("{{ kind }}から選択"),
    loading: "ライブラリを読み込んでいます…",
    none: "一致するものはまだありません。",

    hint: {
      tick: "Tab で選択",
      insert: "Enter で挿入",
      insertCount: template<{ count: number }>("Enter で {{ count }} 件を挿入"),
      close: "Esc で閉じる",
    },

    kinds: {
      aesthetic: "ファミリー",
      style: "タグ",
      type: "デザインタイプ",
      item: "アイテム",
    },

    hints: {
      aesthetic: "語彙のファミリー。説明文ごと展開されます",
      style: "タグ。語だけが入ります",
      type: "デザインタイプ。語だけが入ります",
      item: "参照アイテム。ツールが読めるフォルダーのパスごと入ります",
    },

    // Japanese does not inflect, so both forms are the same sentence with a counter.
    itemCount: {
      one: "1 件",
      other: template<{ count: number }>("{{ count }} 件"),
    },
  },

  chip: {
    role: {
      family: "ファミリー",
      tag: "タグ",
      type: "デザインタイプ",
      reference: "参照アイテム",
    },
  },
};
