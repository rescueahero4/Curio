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

**If translating makes you want to change what the English says, change the English — or
leave both alone. Never only the Japanese.** This is the rule that is easiest to break
without noticing. Translating a screen is the most careful anyone ever reads its copy, so
you *will* find things: a redundant possessive, a button repeating its own heading, an
instruction that says "from its tray icon" when you choose from the menu. Fixing them
quietly on the Japanese side forks the two languages — the Japanese then carries an edit no
English reviewer can see, and when the judgement is wrong the damage lands only on the
readers who cannot check it against the source. A translator who improves one side has
stopped translating.

This is about judgement, not grammar. Japanese routinely has to make explicit what English
leaves implicit — 「ページが見つかりません」 for "Not found", because 見つかりません alone has
no subject. Supplying what the grammar requires is translating. Supplying what you think the
writer should have said is not.

**Separately: the Japanese must not say less than the English.** The clause you drop is
never a whole sentence — it is small enough to feel like tidying. One screen here lost the
possessive from 「Curio のダッシュボード」, six characters, on the reasoning that the heading
above already said Curio. But English names a thing and marks it as given in one move, where
Japanese splits those jobs: without the introduction, the 「は」 that follows presupposes a
referent the reader was never given. Six characters, on the one screen whose entire job is
orienting someone who has forgotten what Curio is.

And if the Japanese would say *more* than the English, that is not a licence either — it is
a sign the English is missing something, and the fix goes in the English.

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

## Reusing `common`

Reuse a `common` key when your screen means **exactly** the same thing by it. The bar is not
"the same English word appears" — `Remove` as in "take this tag off" and `Remove` as in
"delete this file" are five identical letters and two different Japanese words (解除 and
削除).

The one that has caught several people already:

| Situation | Key | English | Japanese |
| --- | --- | --- | --- |
| Re-attempt one failed write | `common.retry` | Retry | 再試行 |
| Re-request a whole page that failed to load | your own namespace | Try again | もう一度試す |

They are different offers and they read differently. Three namespaces reached for
`common.retry` on a page-level failure banner and shipped "Retry" beside another screen's
"Try again"; if you are writing the button under *"Curio could not read your …"*, it is the
second row.

## What is not translated

`lib/format.ts` handles dates and relative times through `Intl`, driven by the same locale
signal. Don't write date strings into a dictionary.
