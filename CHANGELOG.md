# Changelog

All notable changes to NOVA are listed here. Versions follow [Semantic Versioning](https://semver.org).

## [0.5.0] — 2026-10-21

### Fixed
- Nova Control no longer freezes the user's mouse: macOS used to ignore the real mouse and keyboard for 0.25 s after each simulated event. After each click, the pointer now goes back to where the user left it.
- "Control" in a conversation can now use the built-in Nova Control brain on its own. "Nova Control + chat model" is a separate choice.
- The Nova Code conversation column scrolls again.

### Added
- **NOVA for iPhone** (`ios/`): a free SwiftUI app for chatting with NOVA, running Nova Control tasks and controlling the Mac's screen remotely (tap = click, long press = right click, drag = scroll or move, keyboard and shortcuts). Notifications let you approve, refuse or answer Nova Control from the lock screen.
- **Settings › Phone** in the desktop app: a built-in server (off by default, port 47860), advertised by Bonjour and paired with a 6-digit code. It needs no external service and also works through Tailscale.
- The macOS helper can now capture the whole screen with the cursor (`screen`), move the pointer (`move`) and drag (`drag`).
- **NOVA for iPhone works without a computer**: Nova 1 runs on the iPhone (a Swift port of the inference that gives the same results), together with NOVA's maths and unit tools and Apple Intelligence. Conversations made on the iPhone stay on the iPhone. When the Mac is reachable, NOVA uses its models; when it stops answering, the question is taken over on the iPhone.
- **"Control the Mac" sessions**: the Mac streams its screen only after you tap the button, and only while the iPhone shows it. While a session runs, the Mac shows "L'iPhone contrôle le Mac" in the notch. In the app, a "Vous contrôlez le Mac" banner grows out of the Dynamic Island; when you leave the app, the real Dynamic Island (Live Activity) takes over. A session ends with the Stop button, when you leave the Screen tab, or after 3 minutes without news from the phone.
- **New iPhone design**: animated aurora background, the NOVA orb, Liquid Glass cards, and redesigned chat, Nova Control, screen and settings.

## [0.4.8] — 2026-10-20

### Added
- **Detailed plans from the local AI**: the chat model on the Mac (Gemma 4, the Apple model…) now breaks a request into small, verifiable steps written in free-form language — not only a fixed list of actions. Implicit intents are understood (“put on some music” = any song, shuffled).
- **Free-form steps**: for a step with no built-in skill, Nova Control asks the local model for one action at a time (click an element, type, shortcut, menu, scroll…), looking at the interface and, with a vision model, at a screenshot — until the step is done (8 actions at most). Sensitive clicks and shortcuts still ask for approval.
- **Music**: “open Spotify and play some music”, pause, next, previous — directly in Spotify or Apple Music, then checks that a song is actually playing. “Play Drake on Spotify” searches and plays the first result.
- **Sees the screen when stuck**: when a button can't be found in the interface or by reading the screen (e.g. Discord's GIF button), the vision model looks at a screenshot, says where to click, and Nova Control checks that the screen reacted.

### Changed
- **Self-correction**: when a planned step fails, Nova Control looks at the screen and tries another way with the local model before moving on.

## [0.4.7] — 2026-10-20

### Added
- **Real French understanding**: when a chat model is installed (Gemma 4, Qwen, the Apple model…), Nova Control first asks it, on the Mac, to turn the request into precise steps — the application name is separated from the instruction (“open OneNote and structure my notes” → open OneNote, then restructure the open note), recipients, subjects, messages, GIF searches and “the same” are understood from free-form, misspelled French. Without a chat model, the Nova Control model and its rules are used as before.
- **Transform the open text**: structure, summarise, fix, translate, rephrase the document open in the active app (copy → rewrite by the local model → preview → replace). Replacing always asks for approval, whatever the mode.
- **GIFs in Discord**: opens the GIF section (found in the interface or by reading the screen), searches, and sends the first result after approval.

### Changed
- Websites open in a **new tab** instead of replacing the page you had open.
- “open X and <instruction>” is split correctly even without a chat model.

## [0.4.6] — 2026-10-20

### Changed
- **Finds apps like a person**: Spotlight search on file and displayed (translated) names with typo tolerance, then Spotlight on screen (⌘Space, type, Enter) as a last resort.
- **Typos** in app names are understood (“outlok”, “discrod”, “chrom”), and the Nova Control model was retrained with more typos.
- **Chrome profile page**: Nova Control detects “Who's using Chrome?”, picks the profile you named, otherwise the last one used (read locally from Chrome), then waits for the address bar.
- **Typing is checked everywhere**: after typing, the field is read back; apps that ignore simulated typing (Chrome's address bar…) get the text pasted instead, and the clipboard is restored. Opening a page is confirmed by the window title or the address.
- **Browsers like a user**: “go to Chrome and search youtube.com” types the address in Chrome's address bar (⌘L) instead of opening Safari in the background; a “search” for something that looks like a website opens the site.
- **NOVA never comes to the front by itself** anymore (the floating card no longer pulls the main window forward).
- **Mandatory notch animation** for 3 seconds when Nova Control takes control (not clickable).
- **Looks at the screen when stuck**: if a button is not in the app's interface list, Nova Control captures the screen, reads the text on-device and clicks the right word.
- **Subject and message** are read from the sentence (“with subject hi and message hello”), and **“the same on Discord to X”** reuses the previous message without asking.
- Tasks started from the chat are saved in the history.
- The floating card has a close button (×).

## [0.4.5] — 2026-10-20

### Changed
- **Every click is checked**: before clicking, Nova Control asks macOS what is exactly under the pointer and re-aims if the target moved; after clicking, it checks that the screen reacted, and clicks again another way if not.
- **No more questions about what you already said**: the Nova Control model was retrained on **371,000 sentences** with all kinds of names and pseudos (“First Pi”, “JSKabylie1946”, “@lea.m”…) and phrasings (“send hi to…”, “send a message to X and send hi”, “tell X that…”); recipient and message are also read with stricter rules. Without an app name, the last messaging app used is reused.
- **No more pop-ups of the NOVA window**: questions, approvals and the end of a task appear in a small floating card in the bottom-right corner.
- **Multitasking**: “send a message to First Pi, then go to Outlook and…, then…” runs every task in order with shared context; a failed task is reported and the next ones still run, with a summary at the end.
- Faster checks (no screenshot when the built-in brain does not need one).

## [0.4.4] — 2026-10-20

### Changed
- **Mandatory checks before sending** in chat apps: Nova Control confirms that the open conversation is the right person (window title, “Message @…” field, header). If it is not sure, it picks the matching search result itself, checks again, then asks for the exact name — and never sends to an unconfirmed conversation, whatever the permission mode. The approval card shows the proof (“Open conversation: …”).
- E-mails whose recipient, subject or body cannot be read back always ask for approval, even in “Accept all”.
- “Send a message to X and send him hi” is understood as one request: Nova Control no longer asks for something already said.
- The neon cursor now accompanies **every** action (typing, shortcuts, opening apps and websites, menus, scrolling) with a label.
- Apps built with web technologies (Discord, Slack, WhatsApp, Teams…) are now readable by Nova Control.

## [0.4.3] — 2026-10-20

### Changed
- **Nova Control works like a person, step by step**: it waits until the app is really ready, glides the cursor to each field, fills the recipient, subject and body, **checks** each part (and fixes what is missing), asks for approval, sends, then **checks that the message really left** (and retries another way if not).
- E-mails in Outlook, Spark, Thunderbird, Gmail… now include the message body (some apps ignore it in pre-filled e-mails: Nova Control types it itself).
- New: **messages** in Discord, WhatsApp, Messages, Slack, Telegram, Teams, Signal, Messenger and Instagram (web): open the conversation, write, check, send after approval.
- Better understanding of the recipient and the content (“tell Léa on Insta that…”, “from Outlook saying…”); the Nova Control model was retrained with more examples.
- The neon cursor is shown from the first move.
- Safety: **Allow** only reacts to a real mouse click (never to a key press).

## [0.4.2] — 2026-10-20

### Added
- Nova Control picks the right e-mail app and explains why: the one named in the request (“from Outlook”, “with Gmail”), the one just opened (“open Outlook and send an e-mail…”), the Mac's default mail app, then Apple Mail. Outlook, Spark, Thunderbird, Airmail, Mimestream, Canary and Gmail (web) write the e-mail in their own window; sending still needs your approval.
- Update notice: a small card slides in at the bottom right when NOVA starts and a new version is available, with the download progress inside it.

### Changed
- Automatic update checks happen only when NOVA starts (no more checks every 6 hours).

## [0.4.1] — 2026-10-20

### Added
- **Nova Control** (macOS): NOVA carries out tasks on your Mac — open apps and websites, search the web, create notes and folders, send e-mails with Apple Mail, save / export with a name, format and folder, pick menu commands, click buttons, type text, keyboard shortcuts, volume and music — including multi-step requests (“open Photoshop then save the image as PNG on the Desktop”).
  - Built-in brain: the **Nova Control** model, trained from scratch (no download), plus an executor that reads the screen through Accessibility and recovers from common errors (menu synonyms, shortcut fallbacks, scrolling, app name lookup, unexpected dialogs).
  - Advanced mode: an installed vision model (Gemma 4) can drive free-form tasks.
  - **Permission modes** like Claude: Auto (asks for sensitive actions), Manual (asks for every action), Accept all. Esc always stops; Nova Control never types into password fields.
  - A blue neon cursor shows every move and click; questions from Nova Control can be answered inline.
  - Available as a page and from the chat (**Control** toggle in the input bar).
- The notch island folds away while answering and has a stop button.
- Builds can be signed with a stable certificate so macOS keeps NOVA's permissions across updates.

## [0.4.0] — 2026-10-19

First public release on GitHub.

### Added
- **Nova Code** (optional, Settings → Nova Code): a separate “Code” workspace next to Chat. Open a project folder, browse its files with syntax highlighting, ask for explanations or changes, mention files with `@path`. Proposed files are shown as a **diff** and written only when you click **Apply**; the previous version is backed up first. Coding sessions have their own list in the sidebar.
- **Notch island** (macOS, optional): hover the MacBook notch to reveal Nova Code — project, live status, quick request field. It sits on the screen that has the notch, lets clicks through when folded, and folds back when the pointer leaves.
- **Interface languages**: follows the system language by default, changeable in Settings. English, French, Spanish, Portuguese, German, Italian, Dutch, plus Swahili, Lingala and Haitian Creole (marked for native review). Menus are translated too.
- **Reply language**: choose from ~140 languages (including Antillean, Haitian, Mauritian, Réunion and Guianese creoles, Lingala, Kikongo, Tshiluba, Wolof…) or type any other.

- **Nova 1**, NOVA's own model family, built and trained from scratch:
  - instant first layer for everyday requests (works whatever model is selected, can be turned off);
  - **Nova 1 Dessin**: simple vector drawings from a sentence, saved as PNG in one click;
  - **Nova 1 Résumé**: extractive summaries of PDFs and texts;
  - **Nova 1 Vision**: recognises 106 categories in images; on macOS it also reads videos (key frames + on-device transcript);
  - selectable as a model of its own in the model picker and on the Models page.
- The notch island now appears on every screen (a small pill on screens without a notch), lets you choose **Nova Chat or Nova Code** and the **model**, and shows the answer inside the island.
- Nova Code works without opening a folder.
- Answers to general-knowledge questions instead of refusing; questions about current facts (leaders, “current”…) are checked on Wikipedia automatically, with the source shown (can be turned off in Settings → Privacy).

### Known limitations
- Texts generated by built-in tools (maths steps, weather summaries) are still in French.

## [0.3.0] — 2026-10-19

Internal release (not published).

### Added
- **Automatic updates** from GitHub Releases, cryptographically signed; manual check in the NOVA menu and in Settings.
- **Native macOS menu** (About, updates, settings, new chat, sidebar, deep thinking, Wikipedia mode, help).
- **Menu-bar mini chat** (macOS): a floating panel that answers in place, with follow-ups and “Open in NOVA”.
- **Fast mode** by default (reasoning disabled): first word in ~0.2 s instead of ~14 s with Gemma 4; **Deep thinking** toggle.
- Collapsed reasoning display.
- Automatic switch to a **vision model** when an image is attached.
- Global **download panel** with progress, speed, ETA, cancel and **resume**.
- First-launch assistant recommending a model for the machine's RAM.
- Model catalog: Gemma 4 (E4B, 12B, 26B-A4B, 31B), Qwen 3.6 35B-A3B, Qwen 3.8 27B, gpt-oss 20B.
- Spoken maths operators (“12 times 7”) solved locally.

### Fixed
- Window could not be dragged reliably from the top bar.
- Orphaned llama-server processes after a forced quit; engine now restarts automatically after GPU errors.
- Engine used 4 context slots of 4,096 tokens; now a single 16,384-token slot.

### Privacy
- Release builds strip local file paths from binaries; the DMG contains no personal data.

## [0.2.0] — 2026-10-19
- Cross-platform rewrite with Tauri (Rust core + web UI), llama.cpp engine manager, weather, file search, confirmed actions, Wikipedia mode.

## [0.1.0] — 2026-10-19
- Native SwiftUI prototype for macOS.
