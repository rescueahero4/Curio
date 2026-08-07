/**
 * Shell strings, Japanese.
 *
 * The two banners are the app admitting a limitation, so they are plain です・ます with no
 * hedging: 一時停止中です, 待機中です. The buttons beside them are bare nouns — 設定,
 * 新規プロジェクト — which is what a Japanese toolbar puts on a button.
 */

import { template } from "@solid-primitives/i18n";
import type { Shell } from "~/lib/i18n/locales/en/shell";

export const shell: Shell = {
  brand: {
    home: "Curio ライブラリ",
  },

  nav: {
    // The landmark's name, not a translation of the word "Sections". A screen reader reads
    // this next to 「設定のセクション」 from the Settings page, so it has to say which nav
    // it is rather than that it is one.
    label: "メインナビゲーション",
    library: "ライブラリ",
    projects: "プロジェクト",
    prompts: "プロンプト",
  },

  search: {
    label: "ライブラリを検索",
  },

  actions: {
    settings: "設定",
    addItem: "+ アイテムを追加",
    newProject: "新規プロジェクト",
    // 開始中 would describe the button; 作成中 describes the prompt, which is what the
    // user is waiting for. The failure below stays on the same verb.
    newProjectStarting: "作成中…",
    newProjectFailed: "新しいプロンプトを作成できませんでした。",
    pausedReason: "Curio は一時停止中です。トレイアイコンから再開してください。",
  },

  paused: {
    title: "Curio は一時停止中です。",
    // Three も rather than a と-joined pair and a comma: 「閲覧と検索、すでに…」 reads at
    // first as an apposition, and the reader has to back up to find the third item.
    body: "閲覧も検索も、すでにキャプチャしたものも、これまでどおり使えます。新しいキャプチャと編集は、トレイアイコンから「再開」を選ぶまで受け付けません。",
  },

  missingKey: {
    // The English leads with the state and appends the cause; Japanese puts the cause first
    // and lands on the state, which is the same sentence in the order this language reads it.
    title: "API キーが未設定のため待機中です。",
    body: "キャプチャはこれまでどおり保存され、閲覧もできます。待っているのは説明の生成だけです。",
    // 順番待ち rather than a third 待機中 in as many sentences — the English varies its own
    // wording across these lines for the same reason, and 順番 picks up the queue below.
    waiting: template<{ count: number }>("{{ count }} 件が順番待ちです。"),
    addKey: "設定で API キーを追加",
    // 登録すれば, not 設定すれば: the line above already spends 設定 on the screen's name, and
    // the same word arriving as a verb one sentence later reads as a stutter — the same fault
    // 順番待ち was introduced to fix, one line further on.
    drains: "登録すれば、あとは順番に処理されます。",
  },

  saved: {
    label: "変更を保存しました",
    undo: "元に戻す",
  },
};
