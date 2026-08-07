/**
 * Library strings, Japanese.
 *
 * Two decisions run through the whole file.
 *
 * **The bulk bar's summaries are written as Japanese sentences, not assembled.** The English
 * builds them as verb + names + preposition + count, which puts the verb second; Japanese
 * puts it last, so every `done.*` key here is one complete sentence with the count in front
 * of the names. That is also why the `…One` / `…Other` pairs are identical: they are
 * English's plural, and 件 counts every number.
 *
 * **English's one "Remove" is three Japanese verbs, split by what is being removed.** Taking
 * a term off an item is 外す — the term itself survives, and a reader who saw 削除 there would
 * reasonably expect the tag to be gone from the library. Lifting a filter or a selection is
 * 解除, which is what Japanese interfaces put on those. Destroying the row is 削除. The same
 * user action always gets the same verb: the bulk bar's Remove button, its summary of what
 * that button did, and the × on an item's own chip are all 外す, because they are one action
 * reached from three places.
 *
 * **An injected name is wrapped in 「」, always.** These are arbitrary user data — a tag can
 * be `warm` or 暖色 — and the half-width space that a Latin run wants inside Japanese text is
 * a visible gap when the run is Japanese. Quoting sidesteps the rule instead of guessing at
 * it, and marks where the data starts and stops. Numbers are the exception and keep their
 * spaces: 「40 件」 is a Latin run under any reading.
 */

import { template } from "@solid-primitives/i18n";
import type { Library } from "~/lib/i18n/locales/en/library";

export const library: Library = {
  status: {
    processing: "評価待ち",
    ready: "説明済み",
    needsReview: "要確認",
    // A compound, not a clause: this sits in a badge beside 評価待ち and 要確認, and 評価に失敗
    // was the only one of the four carrying a particle. It still reads in the Status filter.
    failed: "評価失敗",
  },

  grid: {
    loadingMore: "さらに読み込んでいます…",
    errors: {
      load: "ライブラリを読み込めませんでした。",
      sessionExpired:
        "Curio が再起動しました。トレイアイコンからダッシュボードを開き直してください。",
      unreachable: "Curio が応答しません。まだ起動しているか確認してください。",
      retry: "もう一度読み込む",
    },
    empty: {
      loading: "ライブラリを読み込んでいます…",
      fresh: {
        title: "まだ何もありません。",
        blurb:
          "Curio 拡張機能でページをキャプチャするか、上部のバーの「+ アイテムを追加」を使ってください。届いたものから順に Curio が説明を書きます。",
      },
      filtered: {
        title: "条件に一致するものはありません。",
        blurb: "フィルターのピルを外すか、名前や説明に含まれる語で検索してみてください。",
      },
    },
  },

  filters: {
    type: {
      title: "デザインタイプ",
      empty: "デザインタイプはまだありません。キャプチャを説明する過程で Curio が追加します。",
    },
    family: {
      title: "ファミリー",
      empty: "ファミリーはまだありません。まとめられるものが集まると現れます。",
    },
    tag: { title: "タグ", empty: "タグはまだありません。" },
    status: { title: "ステータス", empty: "ステータスはありません。" },
    clear: "フィルターを解除",
    vocabulary: "語彙",
    view: {
      legend: "ライブラリの表示方法",
      comfortable: { label: "ゆったりグリッド", hint: "カードを大きく、1 行あたりは少なく" },
      dense: { label: "高密度グリッド", hint: "画面により多く表示" },
      list: { label: "リスト", hint: "1 行に 1 件、キャプチャ日付き" },
    },
  },

  options: {
    filter: "この一覧を絞り込む",
    // The English slash is doing the work of "or"; Japanese says it with 「か」 and reads as
    // one instruction rather than as two modes to choose between.
    filterOrCreate: "絞り込むか、新しく作成",
    // No space before を: `noun` is always one of the three below, which are always Japanese.
    // The half-width space rule is for Latin runs, and none of these is one.
    filterOrCreateLabel: template<{ noun: string }>("{{ noun }}を絞り込むか、新しく作成"),
    noMatch: "一致するものがありません。",
    create: template<{ noun: string; name: string }>("「{{ name }}」を新しい{{ noun }}として作成"),
    nouns: {
      family: "ファミリー",
      type: "デザインタイプ",
      tag: "タグ",
    },
  },

  selectToggle: {
    named: template<{ name: string }>("「{{ name }}」を選択"),
    unnamed: "このアイテムを選択",
  },

  card: {
    untitled: "無題",
    proposed: template<{ name: string }>("提案「{{ name }}」"),
    facets: {
      family: template<{ name: string }>("ファミリー「{{ name }}」"),
      type: template<{ name: string }>("デザインタイプ「{{ name }}」"),
    },
    failed: {
      reason: "Curio はこのアイテムを説明できませんでした。",
      paused: "Curio は一時停止中です。再評価するにはトレイアイコンから再開してください。",
      busy: "Curio が見直しています…",
      queueing: "登録中…",
      action: "再評価",
      problem: "再評価を登録できませんでした。少し時間をおいてお試しください。",
    },
  },

  bulk: {
    selected: template<{ count: number }>("{{ count }} 件を選択中"),
    allMatching: "このフィルターに一致するすべて",
    selectMatching: "一致するすべてを選択",
    clearSelection: "選択を解除",
    paused: "Curio は一時停止中です。一括編集するにはトレイアイコンから再開してください。",

    tags: {
      title: "タグ",
      empty: "タグはまだありません。下の欄に入力して作成できます。",
      fresh: "新しいタグ",
    },
    types: {
      title: "デザインタイプ",
      empty: "デザインタイプはまだありません。下の欄に入力して作成できます。",
      fresh: "新しいデザインタイプ",
    },
    families: {
      title: "ファミリー",
      empty: "ファミリーはまだありません。先に語彙ページで作成してください。",
    },

    panel: {
      add: "追加",
      // The term is taken off the selected items, not deleted from the library — 外す, the
      // same verb as the × on an item's own chip.
      remove: "外す",
      needOne: "まず 1 つ以上選んでください。",
    },

    delete: {
      askMatching: "このフィルターに一致するすべてを削除しますか。",
      askOne: template<{ count: number }>("{{ count }} 件を削除しますか。"),
      askOther: template<{ count: number }>("{{ count }} 件を削除しますか。"),
      // A pair of verbs rather than 「はい」/「いいえ」: the question is one line away and
      // the button should still say what pressing it does.
      yes: "削除する",
      no: "削除しない",
    },

    done: {
      // `values` is a 、-joined run of user-chosen names, so it is quoted like any other
      // injected name — one 「」 around the whole enumeration, not one per term.
      addedOne: template<{ values: string; count: number }>(
        "{{ count }} 件に「{{ values }}」を追加しました。",
      ),
      addedOther: template<{ values: string; count: number }>(
        "{{ count }} 件に「{{ values }}」を追加しました。",
      ),
      removedOne: template<{ values: string; count: number }>(
        "{{ count }} 件から「{{ values }}」を外しました。",
      ),
      removedOther: template<{ values: string; count: number }>(
        "{{ count }} 件から「{{ values }}」を外しました。",
      ),
      deletedOne: template<{ count: number }>("{{ count }} 件を削除しました。"),
      deletedOther: template<{ count: number }>("{{ count }} 件を削除しました。"),
    },

    errors: {
      overCap: template<{ matched: number; limit: number }>(
        "{{ matched }} 件が一致しますが、上限は {{ limit }} 件です。何も変更していません。フィルターを絞り込むか、選ぶ数を減らしてください。",
      ),
      paused: "Curio は一時停止中です。トレイアイコンから再開してください。",
      failed: "編集は反映されませんでした。",
    },
  },

  copyFailed: "クリップボードにアクセスできませんでした",

  item: {
    fields: {
      name: "名前",
      description: "短い説明",
      sourceUrl: "参照元 URL",
      openSource: "新しいタブで開く",
      openSourceLabel: "参照元 URL を新しいタブで開く",
      imagePrompt: "画像プロンプト（この見た目を再現するときに Curio が使う指示）",
      paused: "Curio は一時停止中です。編集は保存されません。",
      saved: "保存しました。",
      // "Edited by you" names the reader, which a Japanese interface does not. Both lines
      // say whose words are on screen and leave the person out of it.
      byUser: "手動で編集されています。再評価しても名前はそのまま残ります。",
      byCurio: "Curio が説明しました。ここでの変更はそのまま保持されます。",
      errors: {
        save: "編集を保存できませんでした。",
        paused:
          "Curio は一時停止中のため、編集は保存されませんでした。トレイアイコンから再開してください。",
        unreachable: "Curio が応答しないため、編集は保存されませんでした。",
      },
    },

    links: {
      families: "ファミリー",
      types: "デザインタイプ",
      tags: "タグ",
      addFamily: "ファミリーを追加",
      addType: "デザインタイプを追加",
      addTag: "タグを追加",
      remove: template<{ name: string }>("「{{ name }}」を外す"),
      emptyFamilies: "ファミリーはまだありません。",
      emptyTypes: "デザインタイプはまだありません。",
      emptyTags: "タグはまだありません。",
      proposed: "紫は、既存のファミリーに一致したのではなく Curio が提案したことを表します。",
      undescribed:
        "このアイテムのファミリーには、まだ説明がありません。Curio は新しいキャプチャをその説明と照らし合わせるため、語彙ページで説明を書いて初めて効果が出ます。",
      errors: {
        unlinked: template<{ name: string }>(
          "「{{ name }}」を作成しましたが、リンクできませんでした。再読み込みしてもう一度追加してください。",
        ),
        create: template<{ name: string }>("「{{ name }}」を作成できませんでした。"),
      },
    },

    grayZone: {
      title: "要確認です。",
      unplaced:
        "Curio はこのアイテムをどのファミリーにも置けませんでした。どこに属するか選んでください。",
      near: template<{ name: string; score: string }>(
        "Curio は判断しきれませんでした。「{{ name }}」のスコアは {{ score }} で、推測せずに確認する範囲に入っています。",
      ),
      keep: template<{ name: string }>("「{{ name }}」のままにする"),
      acceptProposal: template<{ name: string }>("Curio の提案「{{ name }}」を採用"),
      moveTo: "移動先",
      choose: "ファミリーを選択…",
      paused: "Curio は一時停止中です。判断するにはトレイアイコンから再開してください。",
      busy: "判断を保存しています…",
      problem: "判断を保存できませんでした。少し時間をおいてお試しください。",
    },

    toolbar: {
      copyFolder: "フォルダーのパスをコピー",
      copyBrief: "概要をコピー",
      reassess: "再評価",
      reassessHint: "再評価しても、手動で編集した名前はそのまま残ります。",
      queueing: "登録中…",
      askDelete: "このアイテムとフォルダーを削除しますか。スクリーンショットも一緒に削除されます。",
      yes: "削除する",
      no: "削除しない",
      deleting: "削除中…",
      paused: "Curio は一時停止中です。トレイアイコンから再開してください。",
      errors: {
        reassess: "再評価を登録できませんでした。",
        delete: "このアイテムを削除できませんでした。",
      },
    },

    handoff: {
      title: "エージェントに渡す",
      locating: "このアイテムの保存先を Curio に確認しています…",
      contents:
        "このフォルダーには screenshot.png と item.md が入っています。パスを Claude Code に貼り付けて、読み込むよう伝えてください。",
      imagePrompt: {
        title: "画像プロンプト",
        copy: "画像プロンプトをコピー",
        none: "このアイテムの画像プロンプトは、まだ書かれていません。",
      },
      noKey: {
        title: "API キーを待っています。",
        blurb:
          "スクリーンショットは保存済みで、このページはすべて編集できます。設定で API キーを登録すると、Curio が名前と説明とファミリーを書きます。待っている間に失われるものはありません。",
      },
    },
  },
};
