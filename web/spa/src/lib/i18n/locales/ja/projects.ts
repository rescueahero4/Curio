/**
 * Projects strings, Japanese.
 *
 * 開く is the verb throughout — for the page a project serves and for the folder on disk
 * alike. 起動 was tried and dropped: it means booting a program, where what this screen does
 * is open a served page in a tab, which `ja/library.ts` already calls 「新しいタブで開く」.
 * Timestamps are label-and-value (「検出: 昨日」) rather than a phrase, because
 * `Intl.RelativeTimeFormat` hands us 昨日 and 今 as often as 3 日前 and neither of those
 * takes に.
 */

import { template } from "@solid-primitives/i18n";
import type { Projects } from "~/lib/i18n/locales/en/projects";

export const projects: Projects = {
  title: "プロジェクト",

  subtitle: "AI ツールがプロジェクトルートに書き出したフォルダーです。自動的に検出されます。",

  paused: "Curio は一時停止中です。トレイアイコンから再開してください。",

  count: {
    one: template<{ count: number }>("プロジェクト {{ count }} 件"),
    other: template<{ count: number }>("プロジェクト {{ count }} 件"),
  },

  failed: "Curio はプロジェクトを読み込めませんでした。",
  retry: "もう一度試す",
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
    namePlaceholder: "任意。空欄の場合はフォルダー名を使います。",
    submit: "登録",
    saving: "登録中…",
    needPath: "先にフォルダーのパスを入力してください。",
    failed: "Curio はそのフォルダーを登録できませんでした。",
  },

  card: {
    launch: {
      label: "開く ↗",
      opening: "開いています…",
      title: "新しいタブで開きます。",
      missingLabel: "見つかりません",
      missingTitle: "フォルダーがないため、開くものがありません。",
      noPageLabel: "開けるページがありません",
      noPage:
        "ここにも v1/v2/… のサブフォルダーにも index.html がないため、開けるページがありません。フォルダーを開いて、ツールが実際に書き出した内容を確認してください。",
      blocked: template<{ url: string }>(
        "ブラウザーが新しいタブをブロックしました。プロジェクトは {{ url }} にあります。",
      ),
      failed: "Curio はそのプロジェクトを開けませんでした。",
    },

    reveal: template<{ path: string }>("{{ path }} をファイルマネージャーで開きます。"),
    revealAsked: template<{ path: string }>(
      "{{ path }} を開くようファイルマネージャーに要求しました。",
    ),
    revealFailed: "Curio はファイルマネージャーに開くよう要求できませんでした。",

    detected: template<{ when: string }>("検出: {{ when }}"),
    opened: template<{ when: string }>("最終アクセス: {{ when }}"),

    // Labels, not clauses: 「エージェントが追加」 would leave が without a predicate, since
    // 追加 on its own is a noun.
    origin: {
      mcp: "エージェントによる追加",
      manual: "手動での追加",
    },

    missing: {
      badge: "見つかりません",
      // The card's heading already names the project, so プロジェクト here would be a word
      // the reader has just read — and the banner wraps without it.
      title: "フォルダーが見つかりません",
      locate: "場所を開く",
      locateTitle: "実際に存在する最も近いフォルダーを開きます。",
      // 削除 rather than 解除: this ends the record, and the sentence beside it is what says
      // the folder itself is left alone.
      remove: "削除",
      removeTitle: "このプロジェクトを Curio から削除します。フォルダーはそのまま残ります。",
      removeFailed: "Curio はそのプロジェクトを削除できませんでした。",

      // Scoped to the record on purpose. 「完全に削除しますか」 beside a folder path reads as
      // *delete the files*, and a user who fears for their work declines a safe action.
      confirm: "このプロジェクトを Curio から削除しますか。",
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
    clear: "リンクを解除",
    failed: "Curio はプロンプトのリンクを変更できませんでした。",
  },
};
