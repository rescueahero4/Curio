# Translating Curio's dashboard

Two languages ship: English (`en`) and Japanese (`ja`). English is the source of truth.

## Adding a string

1. Put the English in the namespace file for the screen it appears on —
   `locales/en/library.ts`, `locales/en/settings.ts`, and so on. Never in a component.
2. Put the Japanese at the same key in `locales/ja/<same-file>.ts`.
3. Use it: `import { t } from "~/lib/i18n"`, then `t("library.empty.title")`.

Step 2 is not optional and not on your honour — `locales/ja/*.ts` is type-checked against
its English counterpart, so a missing key fails `npm run typecheck`.

### Strings with values in them

```ts
// locales/en/library.ts
import { template } from "@solid-primitives/i18n";

export const library = {
  deleted: template<{ count: number }>("Deleted {{ count }} items."),
} as const;
```

```ts
// locales/ja/library.ts
deleted: template<{ count: number }>("{{ count }} 件のアイテムを削除しました。"),
```

```tsx
t("library.deleted", { count: 40 })
```

The argument names are carried in the type, so the two languages cannot drift apart in the
one way that would render `{{ cuont }}` to a user.

### Strings with markup in them

Don't build them by concatenation — word order differs between the two languages and a
sentence assembled from fragments will be wrong in one of them. Either keep the whole
sentence in one key and put the markup around it, or split it at a boundary that survives
translation (a heading and a body, not a subject and a predicate).

## Writing the Japanese

The bar is *what a Japanese product team would have shipped*, not *what this English says,
in Japanese*. Read the English, work out what the screen is telling the user, and write
that. If the English is a pun, an idiom, or a piece of voice — Curio's copy has plenty —
find the Japanese that does the same job, not the same words.

**Register.** Body copy, hints, and error text are です・ます調. Buttons and menu items are
bare nouns or verb stems — 保存, 削除, 元に戻す — because that is what Japanese interfaces
put on buttons; a button reading 保存します sounds like the app narrating itself.

**Drop the pronouns.** English says "your library", "you can still…". Japanese says
ライブラリ and そのまま…. 「あなた」 in an interface reads as a translation almost every
time.

**Count with counters.** 「3 件」, not 「3」. Items are 件. Japanese has no plural, so one
string covers every number — a `{{ count }} 件` needs no variants.

**Punctuation** is 。 and 、 — never `.` and `,`. Quotes are 「」. Ellipsis is …, as in
English. No space before 。

**Latin runs** — product names, numbers, model ids — stay Latin, with a half-width space on
each side when they sit inside Japanese text: `Curio は一時停止中です`. Never use full-width
alphanumerics.

**Don't translate what is not a word.** `Curio`, `MCP`, `Anthropic`, file paths, and the
model ids stay exactly as they are.

## Glossary

Consistency across screens matters more than the perfect word on any one of them. These are
settled; use them.

| English            | Japanese           | Note                                          |
| ------------------ | ------------------ | --------------------------------------------- |
| Library            | ライブラリ         |                                               |
| Item               | アイテム           | counter: 件                                   |
| Project            | プロジェクト       |                                               |
| Prompt             | プロンプト         |                                               |
| Vocabulary         | 語彙               | the controlled set of tags, types, families    |
| Tag                | タグ               |                                               |
| Design type        | デザインタイプ     |                                               |
| Family             | ファミリー         |                                               |
| Settings           | 設定               |                                               |
| Filter             | フィルター         |                                               |
| Search             | 検索               |                                               |
| Capture            | キャプチャ         |                                               |
| Assessment         | 評価               |                                               |
| Rubric             | 評価基準           |                                               |
| Confidence         | 信頼度             |                                               |
| Threshold          | しきい値           | kana, not 閾値                                 |
| Gray zone          | グレーゾーン       |                                               |
| Select / selection | 選択               |                                               |
| Bulk (edit)        | 一括               | 一括編集, 一括削除                             |
| Undo               | 元に戻す           |                                               |
| Paused             | 一時停止中         | `Curio は一時停止中です`                       |
| Tray icon          | トレイアイコン     |                                               |
| Session            | セッション         |                                               |
| Folder             | フォルダー         | long vowel mark, matching Windows and macOS ja |
| Browser            | ブラウザー         | likewise                                       |

## What is not translated

`lib/format.ts` handles dates and relative times through `Intl`, driven by the same locale
signal. Don't write date strings into a dictionary.
