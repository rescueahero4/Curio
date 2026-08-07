/**
 * Items strings, Japanese.
 *
 * The add dialog's two prompts stay two prompts: the blurb explains what the dialog does
 * and is です・ます調, the drop zone is a label on a target and stops at the verb stem.
 * Rendering both as the same polite sentence — which is what a literal pass produces, since
 * the English is nearly the same sentence twice — makes the drop zone read as instructions
 * rather than as a thing you can drop on.
 */

import type { Items } from "~/lib/i18n/locales/en/items";

export const items: Items = {
  detail: {
    back: "ライブラリ",
    processing: "評価中",
    gone: {
      message: "このアイテムは削除されました。",
      back: "ライブラリに戻る",
    },
    errors: {
      open: "アイテムを開けませんでした。",
      notFound: "このアイテムはライブラリにありません。削除された可能性があります。",
    },
  },

  add: {
    title: "アイテムを追加",
    blurb:
      "画像をドロップするか、貼り付けるか、ファイルを選んでください。追加すると Curio が内容を説明します。",
    close: "追加せずに閉じる",
    dropzone: "ここに画像をドロップ、貼り付け、またはファイルを選択",
    sourceUrl: "参照元 URL（任意）",
    name: "タイトル（任意。未入力の場合は Curio が提案します）",
    blocked: {
      paused:
        "Curio は一時停止中です。アイテムを追加するにはトレイアイコンから「Resume」を選んでください。",
      // Instruction, not rule: the English carries both, and the disabled button beside
      // this hint already asserts the rule on its own.
      noFile: "まずスクリーンショットを追加してください。",
    },
    busy: "追加中…",
    submit: "追加",
    errors: {
      create: "アイテムを追加できませんでした。",
      paused:
        "Curio は一時停止中です。トレイアイコンから「Resume」を選び、もう一度お試しください。",
      tooLarge:
        "このスクリーンショットは大きすぎて追加できません。範囲を狭めて撮り直してください。",
    },
  },

  errors: {
    sessionExpired:
      "Curio が再起動しました。トレイアイコンからダッシュボードを開き直してください。",
    // The English question is a hint, not a question — it is telling the reader where to
    // look. Japanese states the check to make.
    unreachable: "Curio が応答しません。まだ起動しているか確認してください。",
  },
};
