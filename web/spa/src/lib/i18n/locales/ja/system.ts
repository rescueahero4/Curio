/**
 * System strings, Japanese.
 *
 * These screens are read by someone who is already confused, so the Japanese is plainer
 * than the English rather than more formal. No 恐れ入りますが, no お手数ですが: the reader
 * wants to know what happened and what to press, and padding pushes both further down the
 * paragraph.
 *
 * `Open Dashboard` stays Latin because the tray menu it names is English on every locale.
 */

import { template } from "@solid-primitives/i18n";
import type { System } from "~/lib/i18n/locales/en/system";

export const system: System = {
  noSession: {
    // A statement, not a request. The body already says ください once and `running` says it
    // again; a heading that asks as well makes the screen instruct three times.
    title: "トレイから Curio を開く",
    body: "このタブにはセッションがありません。ダッシュボードはトレイアイコンから開きます。そのときにブラウザーに一度だけ使える鍵が渡されるため、ブックマークしたリンクからは開けません。",
    waiting: "Curio の起動を待っています…",
    running: "Curio は実行中です。トレイアイコンのメニューから「Open Dashboard」を選んでください。",
  },

  notFound: {
    title: "ページが見つかりません",
    back: "ライブラリに戻る",
  },

  pair: {
    // The heading is the noun phrase and the button is the verb, which is the inverse of
    // the shape the English lands on. Both are full length: this page has one action, and
    // a 「許可」 button beside a paragraph of explanation reads as dismissing the page.
    title: "このブラウザーの許可",
    blurb:
      "Curio の拡張機能は通常、自動で接続します。接続できない場合にだけこのページを使ってください。たいていは、パッケージ化されていない拡張機能として読み込んだ開発版で必要になります。",
    authorize: "このブラウザーを許可",
    done: "許可しました。拡張機能が自動で受け取ります。このタブは閉じてかまいません。",
    refused:
      "Curio がリクエストを拒否しました。トレイからダッシュボードを開き直して、もう一度試してください。",
    unreachable: "Curio に接続できませんでした。",
  },

  request: {
    unreachable: "Curio が応答していません。",
    paused: "Curio は一時停止中です。",
    failed: template<{ status: number }>("リクエストに失敗しました (HTTP {{ status }})。"),
  },
};
