# Manual end-to-end test guide

**Who this is for:** anyone. You do not need to be a developer, and you do not need to
understand the code. If you can copy a line of text into a black window and press Enter,
you can run this.

**What it does:** walks you through using Curio the way a real person would, and tells you
what you should see at each step. Where reality and this guide disagree, that is a bug
worth reporting.

**How long it takes:** about 45 minutes for the whole thing. Part 1 (Setup) plus any single
later part is a useful session on its own — you do not have to do it all at once.

**What it costs:** Part 4 spends real money on AI model calls. It is a few cents. Every
other part is free.

---

## Before you start

### What you need

- A computer running **Windows** or **macOS**
- **Google Chrome** (or Edge or Brave), version 116 or newer
- An **Anthropic API key** — only for Part 4. You can get one at
  [console.anthropic.com](https://console.anthropic.com). Skip Part 4 if you don't have one.

### Two words you'll see a lot

- **Terminal** — the black (or white) window where you type commands. On Windows, press the
  Start button, type `powershell`, and press Enter. On macOS, press `Cmd + Space`, type
  `terminal`, and press Enter.
- **The dashboard** — Curio's main screen, which opens in your web browser.

### How to read the instructions

When you see a box like this:

```sh
some command here
```

…it means: click into your terminal window, type or paste that line, and press Enter.

**Wait for each command to finish before typing the next one.** You'll know it's finished
when the terminal shows you a fresh prompt and stops printing new lines. Some commands take
a few minutes the first time — that is normal, and nothing is broken.

### Keep a note of what you find

For each check below, write down **PASS** or **FAIL**. If something fails, note what you saw
instead. That note is more useful than a description of what went wrong technically.

---

## Part 1 — Setup

*Everything else depends on this part. Do it first.*

### 1.1 Open a terminal in the project folder

Open your terminal, then move into the Curio folder. If Curio is in your Documents folder,
that looks like:

```sh
cd Documents/Curio
```

Adjust the path to wherever the folder actually is.

### 1.2 Install the pieces the browser parts need

Run these two lines, one at a time:

```sh
npm --prefix web/spa install
```

```sh
npm --prefix web/extension install
```

Each may take a minute or two and will print a lot of text. That's fine.

### 1.3 Build the dashboard

```sh
npm --prefix web/spa run build
```

> ✅ **Check 1** — the last lines mention `dist` and you see no red **error** text.

### 1.4 Start Curio

```sh
cargo run --bin curio
```

> ⚠️ The `--bin curio` part matters. Leaving it off gives you
> `error: cargo run could not determine which binary to run` — the project contains two
> programs and that tells it which one you want. It is not a sign anything is broken.

The first time, this takes **several minutes** — it is compiling the whole application.
Later runs take seconds. You'll see a lot of scrolling text; that's the compiler talking.

When it's ready, your web browser should open automatically at Curio's dashboard.

> ✅ **Check 2** — the dashboard opens by itself in your browser.
>
> ✅ **Check 3** — there is a **Curio icon in your system tray** (Windows: bottom-right,
> near the clock, possibly behind the `^` arrow. macOS: top-right menu bar).
>
> ✅ **Check 4** — the dashboard shows an **empty library** with wording that tells you how
> to add your first item, rather than a blank screen.

**Leave this terminal window alone from now on.** Curio is running inside it. Closing it, or
pressing `Ctrl+C` in it, stops Curio.

> 💡 **The terminal window staying open, printing lines like `listening on 127.0.0.1`, is
> correct — please don't report it.** You are running a developer build, and that window is
> where it writes its log. The version a user would install shows no terminal at all.
>
> If one of those lines says **`reclaiming a stale runtime.json from a previous run`**, that
> only means the previous Curio was killed rather than quit from the tray. It fixes itself;
> carry on.

### 1.5 Open a second terminal

You'll need one for the remaining commands. Open a new terminal window and `cd` into the
Curio folder again, exactly as in step 1.1.

---

## Part 2 — The browser extension

*This is how you actually capture designs. Validates Epic 8.*

### 2.1 Build the extension

In your **second** terminal:

```sh
npm --prefix web/extension run build
```

### 2.2 Let your browser know about Curio's helper

```sh
cargo run --bin curio-nmh -- --register
```

> ✅ **Check 5** — it prints something like `curio-nmh: registered for Chrome, Edge, Brave,
> Chromium`. Browsers you don't have installed may be listed as skipped; that's fine.

### 2.3 Install the extension in Chrome

1. Open Chrome.
2. In the address bar, type `chrome://extensions` and press Enter.
3. Turn on **Developer mode** — the switch is in the top-right corner.
4. Click **Load unpacked** (top-left).
5. Navigate to your Curio folder → `web` → `extension` → `dist`, and click **Select Folder**.

> ✅ **Check 6** — a card appears named **Curio Capture** with no red error text on it.

### 2.4 Check that the extension found Curio on its own

Click the extension's icon in Chrome's toolbar. You may need to click the puzzle-piece icon
first and then pin Curio.

> ✅ **Check 7** — the popup shows a **green dot** and the words **"Curio is running"**.
>
> **This is the important one.** You never typed a password, copied a code, or told the
> extension where Curio is. It found it by itself. If it says "Curio isn't running" while
> Curio *is* running, that's a real failure — note it.

### 2.5 Capture your first design

1. Go to any website you find visually interesting. A product landing page or a pricing page
   works well.
2. Click the Curio extension icon.
3. Click **Add website**.

> ✅ **Check 8** — the popup briefly says something like "Capturing the visible area…", then
> **"Added ✓"**, then closes itself.
>
> ✅ **Check 9** — **the web page you captured looks exactly as it did before.** You are at
> the same scroll position, the scrollbar is back, and any floating menu or header is
> visible again. Nothing about the page should look disturbed.
>
> ✅ **Check 10** — switch to the Curio dashboard tab and refresh it. Your capture is there
> as a card, showing the **top portion of the page** — not a squashed image of the whole
> page.

### 2.6 Capture a long page

1. Find a long page — one where you have to scroll a lot.
2. Click the Curio icon → **Add full-screen**.
3. **Don't switch tabs while it works.** It will scroll the page for you. This takes 10–30
   seconds.

> ✅ **Check 11** — when it finishes, the page is restored exactly as you left it, same as
> Check 9.
>
> ✅ **Check 12** — in the dashboard, open the new card and look at its image. It should be
> one tall, continuous picture of the page. Look carefully for:
> - **Tearing** — a horizontal line where the picture doesn't line up. *Should not happen.*
> - **Repeated menus** — the site's floating header appearing over and over down the image.
>   It should appear **once**, at the top.
> - **Duplicated or blank sections.** *Should not happen.*

---

## Part 3 — What happens without an API key

*Validates that Curio degrades gracefully. Do this **before** Part 4.*

If you have already added an API key, skip to Part 4 and come back to this later.

> ✅ **Check 13** — the dashboard shows a yellow banner reading roughly **"Queued — needs an
> API key. Captures still land and stay browsable."**
>
> ✅ **Check 14** — your captured cards say **"Waiting for assessment"**. They have **not**
> turned red or said "failed".
>
> ✅ **Check 15** — click a card. You can still see it, rename it, and browse it. The capture
> is not lost just because Curio can't describe it yet.

**Why this matters:** Curio should never lose your capture because a key is missing. It
should wait patiently and pick up where it left off.

---

## Part 4 — The AI assessment

*This part spends real money — a few cents. Validates Epic 7.*

### 4.1 Add your API key

1. In the Curio dashboard, click **Settings**.
2. Find the **API key** field.
3. Paste your Anthropic API key and save.

> ✅ **Check 16** — Curio confirms the key works. If you paste a deliberately wrong key
> (try `sk-ant-nonsense`), it should tell you it was rejected rather than silently accepting
> it. Put the real one back afterwards.

### 4.2 Watch the queue drain on its own

Go back to the Library. **Do not click anything.** Wait up to a minute.

> ✅ **Check 17** — the cards that said "Waiting for assessment" start describing themselves
> **without you asking**. Each one gains a name, a description, and one or more coloured
> **family** badges.
>
> ✅ **Check 18** — this takes well under 30 seconds per item once it starts.

### 4.3 Check the descriptions are actually good

Open two or three assessed cards.

> ✅ **Check 19** — the **name** describes what the page is (e.g. "Stripe pricing"), not what
> it looks like generically ("Blue website").
>
> ✅ **Check 20** — the **tags** are short words you'd actually filter by. If you see full
> sentences in the tag list, that's a failure.
>
> ✅ **Check 21** — the **family** assignment is plausible. If a minimalist page is called
> "Maximalist", note it.
>
> ✅ **Check 22** — look for near-duplicate tags across items, like `minimal` and
> `minimalist` both existing. A few are expected; dozens means Curio isn't reusing its own
> vocabulary properly.

### 4.4 Rename something and check Curio respects it

1. Open any assessed card and **rename it** to something distinctive, like `MY OWN NAME`.
2. Save.
3. Click **Re-assess**.

> ✅ **Check 23** — when the re-assessment finishes, **your name is still there.** Curio may
> update the description and tags, but it must not overwrite a name you chose. This is the
> single most important check in this section — an automated tool that renames your work
> feels like it's fighting you.

### 4.5 Change the thresholds

1. Go to **Settings** and find the two threshold sliders.
2. Move them noticeably — try lowering the upper one.
3. Go back to the Library.

> ✅ **Check 24** — some items change which families they belong to, or start showing a
> "needs review" marker, **immediately**.
>
> ✅ **Check 25** — this happens **instantly and free**. Watch your terminal: there should be
> no sign of new AI calls. Curio stored the scores and re-applies the rule itself.

---

## Part 5 — Pause and resume

*Validates that pausing stops writing without breaking reading.*

1. Click the **Curio tray icon** (system tray on Windows, menu bar on macOS).
2. Choose **Pause**.

> ✅ **Check 26** — the extension popup now shows an **amber/orange dot** and says Curio is
> **paused** — not that it isn't running. Those are different states and the popup must say
> which.
>
> ✅ **Check 27** — the capture buttons are greyed out with an explanation, rather than
> letting you click and then failing.
>
> ✅ **Check 28** — the dashboard still works. You can browse, search, and open items. Only
> *changing* things is refused.

3. Click the tray icon → **Resume**.

> ✅ **Check 29** — capture works again immediately. No restart needed.

---

## Part 6 — Prompts and projects

*Validates Epics 5 and 6 — the part that connects Curio to your AI coding tool.*

### 6.1 Write a prompt

1. In the dashboard, go to **Prompts** and create a new one.
2. Type a sentence describing something you want built.
3. Type `/` — a menu should appear.
4. Insert an **aesthetic family** and an **item** from your library.

> ✅ **Check 30** — the `/` menu appears and lets you search your own families and items.
>
> ✅ **Check 31** — the inserted references appear as distinct blocks (chips), not as plain
> typed text.

### 6.2 Send it to your AI tool

Click **Send to Claude**.

> ✅ **Check 32** — Curio says it copied the prompt and "asked to open" your tool. Note the
> careful wording: Curio cannot know whether the app really opened, and shouldn't claim it
> did.
>
> ✅ **Check 33** — paste into a text editor. The chips have expanded into **real folder
> paths** and instructions, not `[chip]` placeholders. An AI tool receiving this should be
> able to open those folders.

### 6.3 Watch a project appear

1. Find your **projects folder** — it's shown in Curio's Settings.
2. Create a new folder inside it, named anything.
3. Watch the Curio dashboard's **Projects** page.

> ✅ **Check 34** — the new folder appears as a project within about **5 seconds**, without
> you refreshing.
>
> ✅ **Check 35** — if you put an `index.html` file inside it, the **Launch** button opens
> it in your browser.
>
> ✅ **Check 36** — now **rename or move that folder away**. In Curio, it should be marked
> **missing** — it must **not** vanish. Curio never deletes your record because a folder
> moved.

---

## Part 7 — AI agent access (MCP)

*Validates Epic 9. Skip if you don't use AI coding agents.*

1. Go to **Settings → MCP** and turn it **on**.
2. Copy the connection details it shows you.
3. Connect an MCP client — MCP Inspector, Claude Desktop, or similar.

> ✅ **Check 37** — the client lists **seven tools**, all beginning `library_`, `prompt_` or
> `project_`.
>
> ✅ **Check 38** — ask the agent to search your library. It returns the items you captured.
>
> ✅ **Check 39** — with Curio **paused** (Part 5), the agent can still *search* but cannot
> *add* anything, and the refusal explains why.
>
> ✅ **Check 40** — turn MCP **off** in Settings. The next agent request is refused
> immediately, without restarting anything.

---

## Part 8 — Recovering from restarts

*Validates that Curio handles being closed and reopened. This is where things quietly break.*

### 8.1 Restart while the extension is connected

1. With Chrome open and the extension installed, quit Curio from the **tray icon → Quit**.
2. Start it again: in your first terminal, run `cargo run --bin curio` and wait for the dashboard.
3. Without touching the extension's settings, go to a website and capture it.

> ✅ **Check 41** — capture works. You should **not** have to reinstall the extension,
> re-pair, or copy any code. Curio issues a new key each time it starts, and the extension
> should pick that up silently.

### 8.2 Nothing is left behind

Quit Curio from the tray icon.

> ✅ **Check 42** — the extension popup now shows a **grey dot** and says Curio isn't
> running, and offers to let *you* launch it — it must **not** start Curio by itself.

### 8.3 Your library survived

Start Curio again.

> ✅ **Check 43** — every item, prompt, and project you created is still there, exactly as
> you left it.

---

## Part 9 — Bulk operations

*Validates Epic 7's bulk features. Needs about 10 items in your library.*

1. In the Library, select several items using the checkboxes.
2. Use the **AI re-tag** option, with an instruction such as "use British spelling".

> ✅ **Check 44** — Curio reports how many items it will process and starts working. For 8
> or more items it should mention batching.
>
> ✅ **Check 45** — progress updates as it goes; the screen doesn't just freeze.
>
> ✅ **Check 46** — cancelling actually stops it.

3. Find the **Consistency pass** (vocabulary dedupe) option and run it.

> ✅ **Check 47** — it suggests groups of tags that mean the same thing.
>
> ✅ **Check 48** — **nothing is merged until you approve it, one group at a time.** If it
> merges anything automatically, that's a failure — merges cannot be undone.

---

## Part 10 — Cleaning up

When you're finished testing:

```sh
cargo run --bin curio-nmh -- --unregister
```

Then remove the extension from `chrome://extensions`, and quit Curio from the tray icon.

Your library stays on disk — in a folder called `Curio` in your user folder — until you
delete it yourself.

---

## Reporting what you found

For anything that failed, the useful details are:

1. **Which check number** failed
2. **What you saw instead** of what the guide described
3. **What you were doing** immediately before
4. Whether it happens **every time** or just once
5. Your operating system and Chrome version

A screenshot is worth more than a description. If the first terminal window printed
anything that looks like an error at the same moment, copy that too.

---

## Quick summary sheet

| Part | What it proves | Needs an API key? |
|---|---|---|
| 1. Setup | The app builds, starts, and shows a dashboard | No |
| 2. Extension | Capture works and never disturbs the page | No |
| 3. No API key | Captures wait patiently instead of failing | No |
| 4. Assessment | The AI describes and classifies usefully | **Yes** |
| 5. Pause | Pausing stops writing without breaking reading | No |
| 6. Prompts & projects | Curio connects to your AI coding tool | No |
| 7. MCP | AI agents can read the library safely | No |
| 8. Restarts | Nothing breaks and nothing is lost | No |
| 9. Bulk | Bulk AI works and never merges without consent | **Yes** |

**If you only have 20 minutes:** do Part 1, Part 2, and Part 3. Those cover the path every
user takes on their first day.
