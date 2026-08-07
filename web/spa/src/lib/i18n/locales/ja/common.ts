/**
 * The words that belong to no one screen, Japanese.
 *
 * Button verbs are bare nouns — 保存, 削除, 適用 — which is what Japanese interfaces
 * actually put on buttons. The polite verb forms (保存します) belong in sentences, and a
 * button wearing one reads like the app narrating itself.
 */

import type { Common } from "~/lib/i18n/locales/en/common";

export const common: Common = {
  save: "保存",
  cancel: "キャンセル",
  delete: "削除",
  close: "閉じる",
  dismiss: "閉じる",
  apply: "適用",
  edit: "編集",
  copy: "コピー",
  // Past tense, because this label appears *after* the copy has happened.
  copied: "コピーしました",
  retry: "再試行",

  saving: "保存中…",
  loading: "読み込み中…",
  applying: "適用中…",

  language: {
    title: "言語",
    blurb: "Curio の表示言語を選びます。この設定はこのブラウザーにのみ保存されます。",
    label: "表示言語",
    system: "ブラウザーの設定に合わせる",
  },
};
