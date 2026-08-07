/**
 * Settings strings, Japanese.
 *
 * The English on this page was rewritten in plain language — it explains rather than
 * specifies, and it says what will go wrong before it goes wrong. The Japanese keeps that,
 * which mostly means です・ます調 with the officialese left out. 〜を行う, 依頼する and the
 * rest of the business-correspondence register are what the plain-language rewrite removed
 * from the English; putting them back in translation would undo it.
 *
 * Loanwords earn their place or they do not get one. 信頼度 and しきい値 are the glossary's
 * and stay. 「ビジョン」 for a vision model reads as 経営ビジョン, and 「ルート」 is root and
 * route at once, so both are written out — 「画像認識」 and 「保存先」/「置き場所」, the last
 * of which the section's own blurb was already using a line above.
 *
 * Field labels are noun phrases (「キーの設定」), not sentences — a label reading
 * 「キーを設定します」 narrates rather than names. Buttons are the bare stem. Only body copy
 * and hints take the polite ending.
 *
 * Model ids, file paths, `MCP`, `Anthropic`, `Curio`, `Claude Code`, and port numbers are
 * left exactly as they are, with a half-width space on each side where they sit inside
 * Japanese text.
 */

import { template } from "@solid-primitives/i18n";
import type { Settings } from "~/lib/i18n/locales/en/settings";

export const settings: Settings = {
  title: "設定",
  loading: "設定を読み込んでいます…",
  unreadable: {
    message: "Curio が設定を読み込めませんでした。",
    retry: "もう一度試す",
  },

  nav: {
    label: "設定のセクション",
    groups: {
      general: "一般",
      assessment: "評価",
      agents: "エージェント",
    },
    apiKey: "Anthropic キー",
    port: template<{ port: number }>("ポート {{ port }}"),
    ephemeralPort: "起動ごとに変わります",
  },

  paused: {
    reason: "Curio は一時停止中です。設定を変更するには、トレイアイコンから再開してください。",
    notSaved:
      "Curio は一時停止中のため、保存されませんでした。トレイアイコンから再開して、もう一度お試しください。",
  },

  save: {
    reverted: "元に戻しました",
    failed: "Curio が保存できませんでした。",
  },

  paths: {
    title: "パス",
    blurb: "ライブラリの保存先と、Curio が新しいプロジェクトを探すフォルダーです。",
    dataRoot: {
      label: "データの保存先",
      hint: "ライブラリ、ノート、プロンプトはすべてここに保存されます。アプリの中からは移動できません。",
    },
    projectsRoot: {
      /** 保存先 would be wrong here: Curio only ever reads this folder. */
      label: "プロジェクトの置き場所（監視対象）",
      hint: "Curio はこのフォルダーを監視します。中にフォルダーを作ると、数秒でプロジェクトになります。Enter キーで保存します。フォルダーはあらかじめ用意しておいてください。別のフォルダーの監視を始めるには、Curio の再起動が必要です。",
    },
  },

  startup: {
    title: "スタートアップ",
    blurb: "コンピューターの電源を入れたらすぐに Curio が動き出すようにします。",
    toggle: "ログイン時に Curio を起動する",
    unsupported:
      "お使いのシステムでは Curio 側から設定できません。コンピューターのスタートアップ項目に、手動で Curio を追加することはできます。",
    /**
     * `{{ reason }}` is deliberately unused. The argument is typed, not required, so this
     * still satisfies `Settings` — and the alternative is worse: the server writes its
     * reason as an English sentence, and dropping one into the middle of Japanese body copy
     * gives the reader a paragraph that changes language twice to say the same thing once.
     * There is exactly one reason the server can emit, so it is written out here instead.
     * A second one would have to arrive with a key of its own.
     */
    unsupportedReason: template<{ reason: string }>(
      "お使いのシステムでは Curio 側から設定できません。自動起動を登録できるのは Windows と macOS だけです。コンピューターのスタートアップ項目に、手動で Curio を追加することはできます。",
    ),
  },

  apiKey: {
    title: "Anthropic API キー",
    blurb:
      "キャプチャしたものを Curio が評価するために必要です。キーがなくても保存はできますが、評価はキーを追加するまで待機します。",
    /**
     * `set` is a noun label, not the 〜です predicate the English is. A finished statement
     * followed by a colon dangles a value it has already asserted — Japanese takes a colon
     * after a sentence only when the sentence points forward (「以下のとおりです：」), and
     * this one does not. The label form points forward by construction.
     *
     * Neither takes a closing 。: they are the two halves of one status line, and `set` is
     * followed by the masked key. A 。 on one and a separator on the other would alternate
     * in the same paragraph as the user sets and clears a key.
     */
    none: "キーは未設定です",
    set: "設定済みのキー:",
    replace: "キーの置き換え",
    add: "キーの設定",
    hint: "Enter キーで保存します。キーは安全に保管され、二度と表示されないため、置き換えると元に戻せません。",
    clear: "キーを消去",
    clearing: "消去中…",
    nothingToClear: "消去できるキーがありません。",
    cleared: "キーを消去しました。新しいキーを追加するまで、評価は待機します。",
    clearPaused:
      "Curio は一時停止中のため、キーは消去されませんでした。トレイアイコンから再開してください。",
    clearFailed: "Curio がキーを消去できませんでした。",
  },

  models: {
    title: "モデル",
    blurb:
      "Curio が使う AI モデルです。入力した名前はその場では確認されないため、打ち間違いは後から評価の失敗として現れます。",
    vision: {
      label: "画像認識",
      hint: "スクリーンショットを見て、評価を書きます。",
    },
    utility: {
      label: "ユーティリティ",
      hint: "バックグラウンドの細かい処理を担当します。",
    },
  },

  rubric: {
    title: "評価基準",
    blurb:
      "キャプチャしたすべてについて、Curio に何を見てほしいかを書くファイルです。自由に編集でき、書いた内容はアップデート後もそのまま残ります。",
    open: "評価基準を開く",
    /**
     * A bare 〜中 noun, like `apiKey.clearing` and `common.saving`, because this is a button
     * and buttons here do not take a polite progressive. 実行中 rather than a 〜中 form of
     * 伝える: the three states below already name the act, and a second name for it one
     * second earlier would read as two different things happening.
     */
    opening: "実行中…",
    /** 伝える, not 開く — Curio hands the file to the OS and never learns what happened to it. */
    asked: template<{ path: string }>("{{ path }} を開くよう、エディターに伝えました。"),
    paused:
      "Curio は一時停止中のため、エディターには何も伝えていません。トレイアイコンから再開してください。",
    failed: "Curio がエディターに伝えられませんでした。",
  },

  thresholds: {
    title: "信頼度のしきい値",
    blurb:
      "Curio が自分でアイテムを仕分けるために必要な信頼度です。これに届かないときは、代わりに確認を求めます。",
    lower: {
      label: "下限",
      hint: "これを下回ると、Curio は推測せずに新しいファミリーを提案します。",
    },
    upper: {
      label: "上限",
      hint: "これ以上であれば、Curio は確認せずにアイテムを仕分けます。",
    },
  },

  mcp: {
    title: "MCP サーバー",
    blurb:
      "Claude などの AI ツールが、ライブラリを検索して保存済みのアイテムを読めるようになります。オフにすると、外部からは一切接続できません。",
    /** 接続, matching the blurb above and `claudeCode.hint` below — one concept, one word. */
    toggle: "エージェントが MCP でこのライブラリに接続できるようにする",
    off: "スイッチがオフです。下の登録自体はできますが、オンにするまで何も動きません。",
    claudeCode: {
      label: "Claude Code",
      hint: "ターミナル、または Claude Code 自体に貼り付けてください。Claude が接続するときに Curio が起動している必要があります。Curio を移動したりインストールし直したりしたら、ここに戻ってもう一度実行してください。",
    },
    claudeCodeHttp: {
      /**
       * Half-width parentheses, so the whole label is one Latin run. `snippet.copy` puts a
       * half-width space after it, which is right after `)` and stray after `）`.
       */
      label: "Claude Code (HTTP)",
      hint: "同じ登録を、HTTP で直接つなぐ形にしたものです。固定ポートを設定している場合にだけ意味があります。そうでなければ、Curio を次に再起動した時点で使えなくなります。",
    },
    claudeDesktop: {
      label: "Claude Desktop",
      hint: "claude_desktop_config.json に貼り付けてください。Claude Desktop には、代わりに実行できるコマンドがありません。上のスイッチはこちらにも効きます。",
    },
  },

  snippet: {
    copy: template<{ label: string }>("{{ label }} の設定をコピー"),
    copied: template<{ label: string }>("{{ label }} の設定をコピーしました"),
    refused:
      "ブラウザーがクリップボードの使用を許可しませんでした。テキストを選択してコピーしてください。",
  },
};
