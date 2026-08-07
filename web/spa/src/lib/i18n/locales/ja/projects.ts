/**
 * Projects strings, Japanese.
 *
 * 起動 is the verb for opening a project's page, on the tile and in the timestamps alike —
 * 開く is what the *folder* does, and a screen that used one word for both would leave the
 * reader guessing which of the two a control was about.
 *
 * The counter on projects is 件, like items: 「3 件のプロジェクト」. `count.one` and
 * `count.other` are the same sentence because Japanese does not inflect for number; the two
 * keys exist for English's sake and collapsing them is the correct Japanese, not a
 * copy-paste slip.
 */

import { template } from "@solid-primitives/i18n";
import type { Projects } from "~/lib/i18n/locales/en/projects";

export const projects: Projects = {
  title: "プロジェクト",

  subtitle: "AI ツールがプロジェクトルートに書き出したフォルダーです。自動的に検出されます。",

  paused: "Curio は一時停止中です。トレイアイコンから再開してください。",

  count: {
    one: template<{ count: number }>("{{ count }} 件のプロジェクト"),
    other: template<{ count: number }>("{{ count }} 件のプロジェクト"),
  },

  failed: "Curio はプロジェクトを読み込めませんでした。",
  loading: "プロジェクトを探しています…",

  empty: {
    title: "プロジェクトはまだありません。",
    // 「取り込み操作は必要ありません」 carries the English "there is no import step" — the
    // point is not that an import is unnecessary but that there is nothing to do at all.
    body: "プロンプトを作成して AI ツールに貼り付け、書き出し先をプロジェクトルートに指定します。書き出されたフォルダーは 5 秒ほどでここに表示されます。取り込み操作は必要ありません。どのフォルダーを監視しているかは設定で確認できます。",
    settings: "設定を開く",
  },

  register: {
    lead: "プロジェクトがこのマシンの別の場所にありますか。",
    open: "フォルダーを手動で登録",

    title: "フォルダーを登録",
    blurb:
      "このマシン上のどこにあるフォルダーでも指定できます。ただし、フォルダーは既に存在している必要があります。Curio はパスを確認するだけで、フォルダーを作成することはありません。この方法で追加したフォルダーは、後で名前を変更しても追従しません。名前の変更に耐えるマーカーファイルを持つのは、Curio が自動で見つけたフォルダーだけです。",
    path: "フォルダーのパス",
    name: "名前",
    namePlaceholder: "任意。空欄の場合はフォルダー名を使います",
    submit: "登録",
    saving: "登録中…",
    needPath: "先にフォルダーのパスを入力してください。",
    failed: "Curio はそのフォルダーを登録できませんでした。",
  },

  card: {
    launch: {
      label: "起動 ↗",
      opening: "起動中…",
      title: "新しいタブで起動します",
      missingLabel: "フォルダーがありません",
      missingTitle: "フォルダーがないため、表示できるものがありません。",
      noPageLabel: "起動できるページがありません",
      noPage:
        "ここにも v1/v2/… のサブフォルダーにも index.html がないため、起動できるページがありません。フォルダーを開いて、ツールが実際に書き出した内容を確認してください。",
      blocked: template<{ url: string }>(
        "ブラウザーが新しいタブをブロックしました。プロジェクトは {{ url }} にあります。",
      ),
      failed: "Curio はそのプロジェクトを開けませんでした。",
    },

    badge: "見つかりません",

    reveal: template<{ path: string }>("{{ path }} をファイルマネージャーで開きます"),
    revealFailed: "Curio はファイルマネージャーの起動を要求できませんでした。",

    // 「検出 3 日前」 — the same shape Japanese dashboards use for 「更新 3 分前」, and short
    // enough to sit on one line with the origin note beside it.
    detected: template<{ when: string }>("検出 {{ when }}"),
    opened: template<{ when: string }>("起動 {{ when }}"),

    origin: {
      mcp: "エージェントが追加",
      manual: "手動で追加",
    },

    missing: {
      title: "プロジェクトフォルダーが見つかりません",
      locate: "場所を開く",
      locateTitle: "実際に存在する最も近いフォルダーを開きます",
      // 削除 rather than 解除: this ends the record, and the title beside it is what says the
      // folder itself is left alone.
      remove: "削除",
      removeTitle: "このプロジェクトを Curio から削除します。フォルダーはそのまま残ります。",
      removeFailed: "Curio はそのプロジェクトを削除できませんでした。",

      confirm: "このプロジェクトを完全に削除しますか。",
      cost: "プロンプトとのリンクも失われます。",
      removing: "削除中…",
      keep: "残す",
    },
  },

  prompt: {
    from: template<{ title: string }>("「{{ title }}」から作成"),
    untitled: "無題のプロンプト",
    unlink: "プロンプトのリンクを解除",
    deleted: "プロンプトは削除済み",
    clear: "解除",
    failed: "Curio はプロンプトのリンクを変更できませんでした。",
  },
};
