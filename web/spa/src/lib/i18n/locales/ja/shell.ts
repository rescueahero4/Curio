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
    newProjectStarting: "作成中…",
    newProjectFailed: "新しいプロンプトを作成できませんでした。",
    pausedReason: "Curio は一時停止中です。トレイアイコンから「Resume」を選んでください。",
  },

  paused: {
    title: "Curio は一時停止中です。",
    // Three も rather than a と-joined pair and a comma: 「閲覧と検索、すでに…」 reads at
    // first as an apposition, and the reader has to back up to find the third item.
    body: "閲覧も検索も、すでにキャプチャしたものも、これまでどおり使えます。新しいキャプチャと編集は、トレイアイコンで「Resume」を選ぶまで受け付けません。",
  },

  missingKey: {
    // The English leads with the state and appends the cause; Japanese puts the cause first
    // and lands on the state, which is the same sentence in the order this language reads it.
    title: "API キーが未設定のため待機中です。",
    body: "キャプチャはこれまでどおり保存され、閲覧もできます。Curio は説明の生成を待っています。",
    // 順番待ち rather than a third 待機中 in as many lines — the English spends a different
    // word each time (queued, waits, waiting) and this has to do the same, by its own means.
    waiting: template<{ count: number }>("{{ count }} 件が順番待ちです。"),
    addKey: "設定で API キーを追加",
    drains: "追加すれば、あとは順番に処理されます。",
  },

  saved: {
    label: "変更を保存しました",
    undo: "元に戻す",
  },
};
