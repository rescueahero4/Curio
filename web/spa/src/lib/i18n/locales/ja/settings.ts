/**
 * Settings strings, Japanese.
 *
 * The English on this page was rewritten in plain language — it explains rather than
 * specifies, and it says what will go wrong before it goes wrong. The Japanese keeps that,
 * which mostly means です・ます調 with the jargon left out: 「監視」 and 「しきい値」 stay
 * because they are what the screen is about, but nothing gets a katakana loanword where a
 * plain word exists.
 *
 * Field labels are noun phrases (「キーの設定」), not sentences — a label reading
 * 「キーを設定します」 narrates rather than names. Buttons are the verb stem 「消去」,
 * 「開く」. Only body copy and hints take the polite ending.
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
      label: "データルート",
      hint: "ライブラリ、ノート、プロンプトはすべてここに保存されます。アプリの中からは移動できません。",
    },
    projectsRoot: {
      label: "プロジェクトルート（監視対象）",
      hint: "Curio はこのフォルダーを監視します。中にフォルダーを作ると、数秒でプロジェクトになります。Enter キーで保存します。フォルダーはあらかじめ用意しておく必要があり、別のフォルダーの監視を始めるには Curio の再起動が必要です。",
    },
  },

  startup: {
    title: "スタートアップ",
    blurb: "コンピューターの電源を入れたらすぐに Curio が動き出すようにします。",
    toggle: "ログイン時に Curio を起動する",
    unsupported:
      "お使いのシステムでは、Curio 側からこの設定を行えません。コンピューターのスタートアップ項目に、手動で Curio を追加することはできます。",
    unsupportedReason: template<{ reason: string }>(
      "お使いのシステムでは、Curio 側からこの設定を行えません。{{ reason }} コンピューターのスタートアップ項目に、手動で Curio を追加することはできます。",
    ),
  },

  apiKey: {
    title: "Anthropic API キー",
    blurb:
      "キャプチャしたものを Curio が評価するために必要です。キーがなくても保存はできますが、評価はキーを追加するまで待機します。",
    none: "キーは未設定です。",
    set: template<{ key: string }>("キーは設定済みです（{{ key }}）"),
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
      "Curio が使う AI モデルです。入力した名前はその場では確認されないため、打ち間違いは後から評価の失敗として表れます。",
    vision: {
      label: "ビジョン",
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
      "キャプチャしたもののどこを見てほしいかを、Curio に伝えるファイルです。自由に編集でき、書いた内容はアップデート後もそのまま残ります。",
    open: "評価基準を開く",
    opening: "依頼中…",
    paused:
      "Curio は一時停止中のため、エディターを開くよう依頼しませんでした。トレイアイコンから再開してください。",
    failed: "Curio がエディターを呼び出せませんでした。",
  },

  thresholds: {
    title: "信頼度のしきい値",
    blurb:
      "Curio が自分で仕分けるために、どれだけ確信を持てばよいかを決めます。確信が足りないときは、代わりに確認を求めます。",
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
      "Claude などの AI ツールが、ライブラリを検索して保存済みのアイテムを読めるようになります。オフにすれば完全にオフで、どこからも接続できません。",
    toggle: "MCP 経由でエージェントからこのライブラリに接続できるようにする",
    off: "スイッチがオフです。下の登録自体はできますが、オンにするまで何も動きません。",
    claudeCode: {
      label: "Claude Code",
      hint: "ターミナル、または Claude Code 自体に貼り付けてください。Claude が接続するときに Curio が起動している必要があります。Curio を移動したり入れ直したりしたら、ここに戻ってもう一度実行してください。",
    },
    claudeCodeHttp: {
      label: "Claude Code（HTTP 接続）",
      hint: "同じものを直接つなぐ方法です。固定ポートを設定している場合にだけ意味があります。そうでなければ、Curio を次に再起動した時点で使えなくなります。",
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
