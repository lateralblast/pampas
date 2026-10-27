# Changelog

Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.9.9] - 2026-10-27

### Fixed

- `./build.sh --target mac --arch universal` (or `x64`) failed deep into the build with "can't find crate for `std`" when Homebrew's Rust came before rustup's on `PATH`: `rustup target add` had installed the target for rustup's toolchain, but `cargo` ran from Homebrew's, which has only the native standard library. The script now checks that the `rustc` it will use has the standard library for each requested architecture and, if not, stops at once and says to put `~/.cargo/bin` first on `PATH` (or remove the other Rust).

## [0.9.8] - 2026-10-27

### Changed

- The version test (`cargo test versions_agree`) now also checks `ui/package-lock.json`, so the lockfile can no longer fall behind the other version files unnoticed.

## [0.9.7] - 2026-10-27

### Fixed

- `ui/package-lock.json` still said version 0.4.5; it now matches the other version files.

## [0.9.6] - 2026-10-27

### Changed

- The README's status table and test list said all built-in sites were tried on Linux on 2026-10-23; they now say which were tried on Linux (the first six) and which on macOS (the eight added since), and that browser sources on macOS are no longer untested for those eight.

## [0.9.5] - 2026-10-27

### Changed

- The built-in `arena` setup has been run end to end through pampas and works; the README and the setup's note now say so.

## [0.9.4] - 2026-10-27

### Added

- `pampas-cli scrape add arena` adds Arena (formerly LMArena, `https://arena.ai/`) from a fourteenth built-in setup, with a warning that it answers with two anonymous models side by side, may ask for a reCAPTCHA (which pampas does not solve) and records questions. Selectors were checked on the live page: the question is sent with the Send message button and the answer is read from the answers' carousel, which leaves out your own question; with two models the last one is read. It has not yet been run through pampas, and no busy marker was found, so it relies on a settle time of 6 seconds.

## [0.9.3] - 2026-10-27

### Changed

- The built-in `grok` setup has been run end to end through pampas on a logged-in free account and works; the README and the setup's note now say so.

## [0.9.2] - 2026-10-27

### Added

- `pampas-cli scrape add grok` adds Grok's chat (`https://grok.com/`, a free account) from a thirteenth built-in setup, with a warning about account limits, the cookie dialog and upsell banners. Selectors were checked on the live page on a logged-in free account: the question is sent with the Submit button, the busy marker is the Stop model response button, and the answer is read from the assistant's row only (`items-start`), not from your own message, which uses the same markup. It has not yet been run through pampas.

## [0.9.1] - 2026-10-27

### Changed

- The built-in `mistral` setup has been run end to end through pampas on a logged-in free account and works; the README and the setup's note now say so.

## [0.9.0] - 2026-10-27

### Added

- `pampas-cli scrape add mistral` adds Mistral's chat (`https://chat.mistral.ai/work`, a free Mistral account) from a twelfth built-in setup, with a warning about account limits and pop-ups. Mistral now sends `chat.mistral.ai/chat` to this page, which it calls Vibe Work. Selectors were checked on the live page on a logged-in free account: the question is sent with the Send button, the busy marker is the Stop generation button, and the answer is the element marked `data-message-part-type="answer"`, which leaves out the thinking. It has not yet been run through pampas.

## [0.8.9] - 2026-10-27

### Changed

- The built-in `kimi` setup has been run end to end through pampas on a logged-in free account and works; the README and the setup's note now say so.

## [0.8.8] - 2026-10-27

### Added

- `pampas-cli scrape add kimi` adds Kimi's chat outside China (`https://www.kimi.ai/`, a free Kimi account) from an eleventh built-in setup, with a warning about account limits and pop-ups. Selectors were checked on the live page on a logged-in free account: the question is sent with the Send button, the busy marker is the Send button's `stop` state, and the answer is the last `.markdown` block of the assistant message (the one before it is the thinking). It has not yet been run through pampas.

## [0.8.7] - 2026-10-27

### Changed

- The built-in `zai` setup has been run end to end through pampas on a logged-in free account and works; the README and the setup's note now say so.

## [0.8.6] - 2026-10-27

### Added

- `pampas-cli scrape add zai` adds Z.ai's chat (`https://chat.z.ai/`, a free Z.ai account) from a tenth built-in setup, with a warning about account limits and pop-ups. Selectors were checked on the live page on a logged-in free account: the question is sent with the Send button, and the answer is read from `#response-content-container .markdown-prose`, which begins with a "Thought Process" line. There is no busy marker, so it relies on a settle time of 8 seconds. It has not yet been run through pampas.

## [0.8.5] - 2026-10-27

### Changed

- The built-in `qwen` setup has been run end to end through pampas on a logged-in free account and works; the README and the setup's note now say so.

### Fixed

- A browser source's Send button was clicked while still moving, so on Qwen (whose input box animates as text is typed) the click landed on the button's edge and the question was never sent. pampas now waits until the button has stopped moving, and is the element at the click point, before it clicks (the page can still be laying out, leaving another element at the spot where the button was first measured).

## [0.8.4] - 2026-10-27

### Fixed

- The built-in `qwen` setup typed the question but did not send it: Qwen ignores the Enter key pampas sends. It now clicks Qwen's Send button (`button.send-button`). Rebuild the source with `pampas-cli scrape remove qwen` and `pampas-cli scrape add qwen` to pick this up.

## [0.8.3] - 2026-10-27

### Added

- `pampas-cli scrape add qwen` adds Qwen Chat (`https://chat.qwen.ai/`, a free Qwen account) from a ninth built-in setup, with a warning about account limits. Selectors were checked on the live page on a logged-in free account: Enter sends, and the answer is read from `.custom-qwen-markdown`, which leaves out the "Thinking completed" header. It has not yet been run through pampas, and signed-out use was not checked.

## [0.8.2] - 2026-10-27

### Changed

- The built-in `deepai` setup has been run end to end through pampas (signed out) and works; the README and the setup's note now say so.

## [0.8.1] - 2026-10-27

### Changed

- The built-in `deepai` setup's selectors were checked on DeepAI's live page (Enter sends, the stop button appears while the answer is written, the answer is read from the last `.markdownContainer`); the README and the setup's note now say so. It has not yet been run through pampas.

## [0.8.0] - 2026-10-27

### Changed

- The built-in `perplexity` setup has been run end to end through pampas (signed out) and works; the README and the setup's note now say so.

## [0.7.9] - 2026-10-27

### Added

- `pampas-cli scrape add perplexity` adds Perplexity (`https://www.perplexity.ai/`, no account needed) from an eighth built-in setup. Selectors were checked on the live page in a signed-out browser: Enter sends, and the answer ends with a citation label such as `britannica`. Perplexity may show a Cloudflare or sign-in check, which pampas does not get past.

## [0.7.8] - 2026-10-27

### Fixed

- On macOS, `pampas-cli scrape add` (and `scrape login`) appeared to hang after the login window was closed, because closing Chrome's last window leaves Chrome running and pampas waits for it to exit. It now says to quit the browser with Cmd+Q, and the README says so.

## [0.7.7] - 2026-10-25

### Added

- `pampas-cli scrape add deepai` adds DeepAI's chat (`https://deepai.org/chat`, no account needed) from a seventh built-in setup, with a warning about its limits and terms. The selectors were read from the page's HTML and script and have **not** yet been tried in a browser, so treat it as untested until it has been run.

## [0.7.6] - 2026-10-23

### Added

- A headless switch for a run, off by default: `--headless` on the command line, and **Run the browser without a window (headless)** under the scraping option on the Ask tab (greyed out until scraping is ticked). It makes every browser scrape source run with no window for that question, whatever its entry says, and does nothing for sources that are not browser sources. Many sites detect or refuse a headless browser, so the default stays a visible window.

## [0.7.5] - 2026-10-23

### Changed

- On the Ask tab, each web site listed under *Also ask web sites that have no API (scraping)* now has its own checkbox (greyed out until scraping is ticked), with how many of its daily requests are used. None is ticked by default, so switching scraping on does not ask every site at once; the ticks are remembered, the scraping switch is not. Scrape sites no longer appear in the main "sources will be asked" list, and "Select all that are ready" does not select them.

## [0.7.4] - 2026-10-23

### Added

- `pampas-cli scrape add --acknowledge` adds a scrape source without typing "I understand": the risks (and any site-specific warning) are still printed in full, followed by a line saying you acknowledged them with the flag, and no terminal is needed. Without the flag the typed acknowledgement is unchanged. The app still cannot add scrape sources.

## [0.7.3] - 2026-10-23

### Added

- `pampas-cli scrape remove <name>` removes a scrape source from the providers config (the old file is kept as `providers.json.bak`). It refuses a source that is not a scrape source or does not exist. A browser source's login stays in its profile folder, so adding it again needs no new login.

## [0.7.2] - 2026-10-23

### Fixed

- A browser source's `submit_selector` is now clicked with real mouse events (the button is scrolled into view and clicked at its centre) instead of a script's `.click()`, which some sites ignore: Microsoft 365 Copilot's Send button was not being pressed. With `PAMPAS_BROWSER_DEBUG` set, a `clicking <selector> at x,y` line shows the click.

## [0.7.1] - 2026-10-23

### Fixed

- The built-in `copilot` setup now clicks Copilot's Send button (`button[type="submit"][aria-label="Send"]`): pressing Enter did not send in Microsoft 365 Copilot, so the question was typed and left waiting. If you added `copilot` before this version, remove it in the app's Sources tab (or delete its entry in `providers.json`) and run `pampas-cli scrape add copilot` again, or add `"submit_selector": "button[type=\"submit\"][aria-label=\"Send\"]"` to its entry. Built-in setups can now carry a `submit_selector`.

## [0.7.0] - 2026-10-23

### Added

- `pampas-cli scrape add bing` adds Bing's AI Mode (`https://www.bing.com/copilotsearch`) from a sixth built-in setup. Bing draws its answer in an iframe, so browser sources have a new option, `answer_frame` (`--answer-frame` on `scrape add`): a CSS selector for a same-site iframe in which `selector` and `busy_selector` are looked up. Tested with a local page whose answer is in an iframe; the Bing selectors were checked on one answer, not while one was being written.

## [0.6.9] - 2026-10-23

### Changed

- With `PAMPAS_BROWSER_DEBUG` set, a browser source prints a `state` line only when the page's state changes, not every half second while it waits for the answer to stop changing.

## [0.6.8] - 2026-10-23

### Added

- `pampas-cli scrape add google-ai-mode` adds Google's AI Mode (`https://www.google.com/search?udm=50`) from a fifth built-in setup, with a warning about Google's automation checks (CAPTCHA, IP block from Search, account flags) and terms. The answer selector (`[data-container-id="main-col"]`) was checked on a shared AI Mode page; the live page, the stop-button busy marker and a signed-out browser were not, so it is the least tested built-in setup.

## [0.6.7] - 2026-10-23

### Added

- `pampas-cli scrape add duckduckgo` adds DuckDuckGo's AI chat (`https://duck.ai/chat`, no account needed) from a fourth built-in setup. The window that opens after adding is for accepting DuckDuckGo's terms ("Continue") once. Input box (`textarea[name="user-prompt"]`) and answer selectors were checked on a real answer; the stop-button busy marker was not seen while an answer was being written.

### Changed

- A browser source keeps waiting if a call to the page fails for a moment (for example while the page navigates) instead of failing the question.

## [0.6.6] - 2026-10-23

### Added

- `pampas-cli scrape add copilot` adds Microsoft 365 Copilot chat (`https://m365.cloud.microsoft/chat`) from a third built-in setup, with a warning that a work or school account lets your organisation log the chats and that its sign-in policies may refuse the pampas browser profile. Input box (`#m365-chat-editor-target-element`) and answer (`[data-testid="markdown-reply"]`) selectors were checked on a signed-in chat; there is no busy selector.

## [0.6.5] - 2026-10-23

### Added

- `pampas-cli scrape add chatgpt` adds ChatGPT (`https://chatgpt.com/`) from a second built-in setup, in the same way as `deepseek`: it shows the risks, plus a warning that OpenAI's terms prohibit automated use and that the account can be limited or banned, needs `I understand`, and opens the login window if the profile is new. The input and answer selectors were checked on a logged-in chat; the "still writing" marker (`--busy-selector`) was not seen while an answer was being written. Built-in setups can now carry a busy selector and a site-specific warning.

## [0.6.4] - 2026-10-23

### Added

- `pampas-cli scrape add deepseek` with nothing else adds DeepSeek's chat using the built-in setup (address, input box and answer selectors, `--settle-secs 6`, `--daily-limit 20`). It still shows the risks and needs `I understand`, tells you that you must log in, and then opens the login window for you, unless that source's profile has been used before. Any option you give overrides the built-in value; giving `--url` turns it off. A name with no built-in setup and no `--url` is an error that lists the built-in names.

### Changed

- The browser options of `scrape add` (`--input-selector`, `--settle-secs`, `--headless` and the others) no longer need `--browser` to be accepted by the command line; a source that is not a browser source still refuses them with a clear message.

## [0.6.3] - 2026-10-23

### Added

- The Ask tab's *Also ask web sites that have no API (scraping)* option now lists the web sites added with `pampas-cli scrape add` (name, address, and whether each is selected above), or says how to add one when there are none.

## [0.6.2] - 2026-10-23

### Added

- `--only <name>`: ask just one provider (turned on even if its entry has `"enabled": false`) and leave every other one out, for example `pampas-cli --scrape --only deepseek "question"`. It cannot be combined with `--enable` or `--disable`, and an unknown name is an error that lists the configured ones.

## [0.6.1] - 2026-10-23

### Fixed

- `pampas-cli scrape login` opened Chrome with the automation (DevTools) port switched on, so Google's sign-in refused it with "This browser or app may not be secure". The login window is now an ordinary Chrome window on the source's profile with no automation port; the port is only opened when a question is asked, after you are logged in.

## [0.6.0] - 2026-10-23

### Added

- Browser scrape sources, for chat pages that need JavaScript and a login: `"render": "browser"` (with `input_selector`, `selector`, and optional `submit_selector`, `busy_selector`, `settle_secs`, `headless`, `browser_path`, `profile`). Each question starts Chrome or Chromium on a profile folder of the source's own, types the prompt, sends it, waits for the answer to stop changing, reads it and closes the browser. Still opt-in with `--scrape`, and added only with `pampas-cli scrape add --browser --input-selector ...`, whose prompt also warns that the account you log in with can be limited or suspended.
- `pampas-cli scrape login <name>` opens a browser source's page in a normal window on its profile so you can log in by hand once.
- A browser source's default timeout is 120 seconds. `PAMPAS_BROWSER_DEBUG` prints each step to stderr. The app explains a missing input box (log in again) and a browser that would not start.

## [0.5.9] - 2026-10-23

### Added

- `pampas-cli scrape add <name> --url ... --selector ...` (or `--json-pointer`, `--body`, `--header`, `--daily-limit` and others) and `pampas-cli scrape list`. `scrape add` prints the risks (IP block or ban, site terms, breakage) and needs you to type `I understand` at a terminal; it refuses when there is no terminal, so a script cannot do it for you.

### Changed

- Scrape sources can no longer be added in the app. They still appear in the Sources tab and can be switched on or off and given a timeout and daily limit, but saving a list that adds one, or changes a scrape source's address, request or name (including through the JSON list), is refused with a message pointing to `pampas-cli scrape add`.
- The bundled example config no longer has a `scrape-example` entry.

## [0.5.8] - 2026-10-23

### Added

- `--scrape`: also ask web sites that have no API. A new provider kind, `"kind": "scrape"`, sends the prompt (`{prompt}` in `url` or `body`) as a `get` or `post` request and takes the answer from the page (`selector`, CSS) or a JSON reply (`json_pointer`). Scraping is off by default: without `--scrape` those entries are skipped with a warning, and with it a warning says your IP address can be blocked or banned and that it may break a site's terms. No sites are built in; `examples/providers.json` has a disabled `scrape-example` entry. See "Scraping web sites" in the README.
- The Ask tab's *More options* has **Also ask web sites that have no API (scraping)**, with the same warning, and the Sources tab shows and edits the name, timeout and daily limit of scrape sources (the rest in the JSON list).
- Plain-words explanations in the app for a site that blocks the request, a selector that finds nothing, and a scrape source skipped because scraping is off.

## [0.5.7] - 2026-10-23

### Fixed

- The README's Cloudflare account id instructions now say where **Workers & Pages** is in the dashboard: under **Compute**.

## [0.5.6] - 2026-10-23

### Added

- `./build.sh --release <version> --commit <ref>` tags the commit, branch or tag you name for that release, for a version that no commit's `Cargo.toml` says (for example one built only from the working tree, then committed with a later version). The commit must be on GitHub. Example: `./build.sh --release 0.5.3 --commit HEAD`.

## [0.5.5] - 2026-10-23

### Added

- `./build.sh --release <version>` (for example `--release 0.5.3`) releases a specific version instead of the current one, using the files in `dist/` for it. It tags the version's existing tag, else the newest commit whose `Cargo.toml` says that version, and stops if that commit is not on GitHub or no commit has that version. It cannot be combined with `--target`.

## [0.5.4] - 2026-10-23

### Added

- `./build.sh --release [--target <t>] [--draft] [--yes] [--dry-run]` tags the commit `v<version>`, pushes the tag and creates a GitHub release holding the files in `dist/` for that version, with that version's `CHANGELOG.md` section as the notes (needs `gh`, logged in). If the release exists the files are added to it, so each platform can be built and added from its own machine. It never commits: the tree must be committed and pushed, and it stops otherwise. Without `--target` it releases what is already in `dist/`; `--dry-run` shows what it would do and changes nothing.
- `./build.sh --delete-release [<version>] [--keep-tag] [--yes] [--dry-run]` deletes that GitHub release and its files, and the tag on GitHub and locally unless `--keep-tag` is given. It asks first. Copied from bobcat's `build.sh`.

## [0.5.3] - 2026-10-23

### Fixed

- `./build.sh --target mac` no longer stops after the installer with "the command-line binary was not found at target/release/pampas": the script defined `build_target` twice, and the second, older copy (still looking for a binary called `pampas`) was the one that ran, so the `pampas-cli-<version>-mac-<arch>.tar.gz` archive was never made. The older copy is removed.
- The macOS build has now been run on a Mac (Apple silicon): the `.dmg` and the command-line archive build, the app is signed ad hoc with identifier `au.com.lateralblast.pampas`, and `Info.plist` has the local-network note and minimum system version 11.0.

## [0.5.2] - 2026-10-23

### Changed

- The command line is now called `pampas-cli` (it was `pampas`): `pampas-cli "your prompt"`, `pampas-cli key set GROQ_API_KEY`, `pampas-cli init`, and so on. The usage text, the error messages that tell you what to type, and the README all use the new name. Unchanged: the app, the package and library (`pampas`), the config and state folders (`pampas`), the keychain service (`pampas`), and the `./pampas` wrapper script in a checkout. Anything that calls `pampas` on a PATH needs to call `pampas-cli`.
- `./build.sh` packages the command line apart from the app's installer: `pampas-cli-<version>-linux.tar.gz` (`-linux-arm64` for the arm64 container), `pampas-cli-<version>-mac-<arm64|x64|universal>.tar.gz`, `pampas-cli-<version>-windows.zip`. Each holds a folder with just the binary (`pampas-cli`, `pampas-cli.exe` on Windows). It used to copy a bare binary named `pampas-<version>-<system>` into `dist/`.

## [0.5.1] - 2026-10-23

### Fixed

- `./build.sh --target windows` (the VM route) and `--target windows --docker` no longer copy installers from earlier versions into `dist/`: the VM and the container keep their build folder between builds, so an old `pampas_<older>_x64-setup.exe` was copied out beside the new one (and in the container route the old one could replace the new one). Each build now clears the old installers first.

## [0.5.0] - 2026-10-23

### Added

- `build.sh`: builds the app installer and the `pampas` command-line binary into `dist/` (not committed). `--target linux` (deb, rpm or AppImage, natively on Linux), `--target windows`, `--target mac`, with `--docker`, `--arch`, `--package` and `--no-cli`. It builds the interface first and never commits, tags or publishes. Adapted from the build process of bobcat, the author's other Tauri app.
- A Windows 11 virtual machine for building the Windows installer on a Linux host: `windows/vm.py` (`up`, `status`, `build [--manage]`, `down`), `windows/compose.yml` (the `dockurr/windows` image under QEMU/KVM, viewer on port 8008, disk in `~/.local/share/pampas/windows`) and the scripts that run inside Windows (`windows/oem/`, kept with CRLF line endings by `.gitattributes`). `./build.sh --target windows` starts the VM, builds, copies the installer and `pampas.exe` out and stops the VM again.
- `docker/linux` (Linux packages in an Ubuntu 24.04 container, the default on a Mac) and `docker/windows` (Windows installer cross-built with MinGW in a container, the default on a Mac).
- macOS build settings copied from bobcat's working process: ad hoc signing, a minimum macOS version, and `src-tauri/Info.plist` with the local-network usage note (so macOS lets the app reach an Ollama server on the LAN). Not yet run on a Mac.
- Package details in `src-tauri/tauri.conf.json`: category, description and publisher.
- A "Building installers" section in the README, with what has and has not been tested.

### Fixed

- (in the new Windows scripts, before release) the build watcher stopped seeing requests after a build script changed a variable the watcher shared; each request and `setup.cmd` now run in their own environment. The VM's install script registers the watcher first, and `build.cmd` installs any build tool that is missing, because the install script stopped after the Rust step on the first VM.

## [0.4.9] - 2026-10-23

### Added

- A **History** tab in the app. Each finished question is saved automatically with its question, merged reading and every answer, and the tab lists the sessions by day, newest first. Open one to read it again exactly as it was, **Ask again** to put the question back in the Ask tab, delete one (a second press confirms) or **Delete all history**, find a question by typing part of it, and switch saving off with **Save new questions to the history**. `Ctrl`/`⌘` + `5` opens it.
- `history.json` in the state directory (beside `usage.json`) holds the sessions, at most the newest 200, written atomically; an unreadable file is moved to `history.json.corrupt` so nothing is lost. The code is `pampas::history` in the library, so the command line can use it later.
- App backend commands `list_history`, `get_history`, `save_history`, `delete_history`, `clear_history` and `import_history`.

### Changed

- The "Earlier questions" list on the Ask tab is gone: it was kept in the webview's local storage. Sessions kept there are moved into `history.json` once, the first time this version starts, and the old copy is removed.
- Turning history off in the History tab takes effect at once; `Ctrl`/`⌘` + `1` to `5` switch tabs (were `1` to `4`).

## [0.4.8] - 2026-10-23

### Added

- **Save as…** in the app: a menu on each answer set that saves it as Markdown (`.md`), plain text (`.txt`) or JSON (`.json`) through the system's save dialog. The files use the command line's own renderer (the same as `pampas -f markdown|text|json -o file`), with the question added at the top of Markdown and text files (JSON already carries it): the merged reading first, then each answer; sources that did not answer are left out of Markdown and text and marked `"ok": false` in JSON. The save dialog is opened by the app's backend, not by the page, so the page never supplies a file path. The file name suggested is `pampas-` and the first words of the question.
- The app uses the `tauri-plugin-dialog` crate for the save dialog.

### Changed

- The copy buttons in the app (`Copy everything`, `Copy the reading`, and each source's `Copy`) now say "Copied" on the button for a moment (or "Could not copy"); before, only screen readers were told. `Copy everything` copies the same Markdown as **Save as… › Markdown**.

## [0.4.7] - 2026-10-23

### Changed

- The app's window icon is now the 128 px version of `pampas.png`, so the tiger is sharp in task switchers and docks (it was the 32 px one). The bundle icons for macOS (`icon.icns`), Windows (`icon.ico`) and Linux (`icon.png`) were already generated from `pampas.png`; regenerate them all with `cargo tauri icon pampas.png -o src-tauri/icons` (then delete the `android` and `ios` folders it adds, which this app does not use). Putting the 512 px file first in `bundle.icon` made Tauri set no window icon at all, so keep `icons/128x128.png` first.

## [0.4.6] - 2026-10-23

### Added

- The desktop app's interface (`ui/`, plain HTML, CSS and TypeScript, no framework), built as a critical edition of the answers: the question, then the merged "reading" with boxed sigla (a, b, c) marking each source in the text, then each source as a witness row, then a "Not heard" list that says in plain words why a source failed or was left out before asking, and where to fix it. Answers arrive row by row as each source finishes (rows already shown are left untouched, so an opened "Technical detail" or a text selection survives; the one that just arrived settles in). The boxed letters in the reading are buttons that jump to that source's answer. Pages: Ask (question, sources to ask, merge, earlier questions kept in the app's local storage, first-run steps), Sources (full editor for `providers.json` with a plain-language form, reorder, remove, add from the built-in list, save bar, JSON text view), Keys & settings (write-only keys with where to get each, settings), and Usage (today's counts against each limit).
- It follows the system light and dark theme and the reduced-motion setting, works with the keyboard (`Ctrl`/`⌘`+`Enter` asks, `Ctrl`/`⌘`+`1` to `4` switch pages, arrow keys move between tabs), announces answers and errors to screen readers, and reflows down to a narrow window. Model answers are rendered as safe markdown: HTML is escaped and links are shown as text with their address, never as clickable links.
- `ui/dev/` (development only): a mock backend and a small server, so the interface can be tried in an ordinary browser (`node ui/dev/serve.mjs`, then `http://127.0.0.1:8765/dev/`), and a screenshot script.
- `example_config` command for the GUI's "add a source" list.
- `npm --prefix ui test` tests the markdown renderer, the siglum logic and the failure explanations.

### Changed

- `./pampas --gui` installs the front end's tools on first use and builds the interface before starting the app (run `npm --prefix ui run build` yourself before a bare `cargo tauri dev`); `cargo tauri build` builds it first on its own.
- The app's security policy (`csp`) is set instead of left open.
- `tests/version_sync.rs` also checks `ui/package.json`.

## [0.4.5] - 2026-10-23

### Added

- The GUI's backend, in `src-tauri` (a Tauri 2 app, `pampas-gui`, in a Cargo workspace with the command line): commands to list the providers with their key status and usage, read, validate and save `providers.json` (the field order is kept, the old file is kept as `providers.json.bak`, and nothing is written if validation fails), create the config from the bundled example, set and delete keys and settings, show usage, and ask the providers, with an event as each one answers. The window itself is a placeholder page until the interface is built. `./pampas --gui` starts it.
- `engine::fan_out_each`, which reports each provider's reply as it arrives.

### Changed

- `KeyStore` requires `Send + Sync`, so a key store can be used from the app's threads.
- `tests/version_sync.rs` also checks `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.

## [0.4.4] - 2026-10-23

### Changed

- The logic behind a run now lives in a library (`src/lib.rs`, with `src/engine.rs` for loading the config, building the providers, asking them, merging, and the key, usage and settings tables) and `src/main.rs` is a thin command line over it, ready for the GUI to use the same code. Warnings and notices are returned as data instead of printed. No change to the command-line behaviour: the tables, messages and flags are as before.

## [0.4.3] - 2026-10-23

### Added

- Platform file locations in one place (`src/paths.rs`): Linux is unchanged; macOS now uses `~/Library/Application Support/pampas`; Windows uses `%APPDATA%\pampas` for config and settings and `%LOCALAPPDATA%\pampas` for usage, and no longer needs `HOME`. An `XDG_*` variable still overrides the default on Linux and macOS.
- `pampas migrate` moves files from the old macOS locations (`~/.config/pampas`, `~/.local/state/pampas`) to the new ones, never overwriting. Until it is run, old files are still used when nothing exists at the new location, with a note on stderr.
- `README.md`: a File locations section.

### Changed

- The `README.md` environment variable table lists the Windows variables and says the `XDG_*` variables apply to Linux and macOS.

## [0.4.2] - 2026-10-23

### Added

- A `timeout` field on a provider entry gives that provider its own wait, in seconds, instead of the default 30. The `ollama` and `ollama-remote` example entries use 180.

### Changed

- `--timeout` no longer has a default of its own: without it a provider uses its entry's `timeout`, else 30. When given, it applies to every provider for that run and overrides entries, as before. The `--merge` call uses the synthesizer's timeout in the same way.

## [0.4.1] - 2026-10-23

### Added

- When `~/.config/pampas/providers.json` does not exist and `pampas usage`, `pampas key list` or a prompt run is started on a terminal, pampas asks `Create it from the bundled example? [y/N]` and writes the file on `y`. With no terminal, or with a `--config` path (which is never created, so a typo is reported), it fails with the usual message pointing at `pampas init`.

## [0.4.0] - 2026-10-23

### Fixed

- Output piped into a command that closes early (`pampas usage | head -1`) no longer panics with "failed printing to stdout: Broken pipe"; the process ends quietly, as other command-line tools do. The new dependency is `libc`, on Unix only.

## [0.3.9] - 2026-10-23

### Fixed

- Options can now come before a subcommand: `pampas --config <path> key list` and `pampas --usage-file <path> usage` work, as `pampas key list --config <path>` already did. Before, clap reported "the subcommand ... cannot be used with --config".

## [0.3.8] - 2026-10-23

### Changed

- When the OS keychain is unavailable (no Secret Service running, access refused), the error now says so and tells you to set the key as an environment variable (`export GROQ_API_KEY=<key>`), instead of "No default store has been set". It applies to `key set`, `key list`, `key delete` and normal runs.

## [0.3.7] - 2026-10-23

### Changed

- `README.md` Providers table: Cohere's trial-key limits are confirmed from its docs (1,000 calls a month, 20 a minute per Chat model), and Cloudflare's 10,000 Neurons a day and 300 requests a minute are confirmed, with a rough requests-a-day estimate. Groq and NVIDIA now give the figures found and how reliable each is; Gemini notes that Google publishes none and that third-party reports conflict. No `daily_limit` changed.

## [0.3.6] - 2026-10-23

### Changed

- `pampas setting set <NAME>` with no value now asks for it at a prompt (shown as typed, since settings are not secrets). Piped with no value, it is an error that says so, rather than waiting on input.

## [0.3.5] - 2026-10-23

### Added

- `pampas setting set <NAME> <VALUE>`, `setting delete <NAME>` and `setting list` manage the values behind `${NAME}` placeholders in a provider's `base_url`, in `settings.json` beside the providers config. An environment variable of the same name takes precedence. `setting list` shows each placeholder the config uses with its value and source (`environment variable`, `settings file`, `not set`).
- `setting set` refuses names that end in `_KEY`, `_TOKEN`, `_SECRET` or `_PASSWORD` and points at `pampas key set`, since the file is plain text.

### Changed

- The "not set" message for a `${NAME}` placeholder now also names `pampas setting set NAME <value>`.

## [0.3.4] - 2026-10-23

### Added

- `${NAME}` placeholders in an entry's `base_url` are filled from the environment variable `NAME`. A provider whose variable is unset or empty is skipped with a message naming it, like a missing key.

### Changed

- `examples/providers.json` uses `${CLOUDFLARE_ACCOUNT_ID}` in the three Cloudflare entries and `${OLLAMA_REMOTE_HOST}` in `ollama-remote`, replacing the `YOUR_ACCOUNT_ID` and `YOUR_OLLAMA_HOST` placeholders you had to edit by hand. `pampas init` now tells you to set the variables.

## [0.3.3] - 2026-10-23

### Changed

- `pampas key list`, `key types` and `usage` print an aligned table with a header row when stdout is a terminal, and the same tab-separated lines as before (no header) when piped.
- `pampas key list` shows just `missing` for a provider with no key, instead of repeating the full "no key: set ..." message on every row.

## [0.3.2] - 2026-10-23

### Added

- `pampas init` writes the bundled example providers config to `~/.config/pampas/providers.json` (or the `--config` path), creating the directory, and refuses to overwrite an existing file.

### Changed

- A missing providers config now says `<path> does not exist: run `pampas init` ...` instead of "No such file or directory".

## [0.3.1] - 2026-10-23

### Added

- `pampas key set <NAME> --stdin` reads the key from standard input (`pass show groq | pampas key set GROQ_API_KEY --stdin`), for scripts and password managers. The key never appears as an argument, so it is not in shell history or `ps`.

## [0.3.0] - 2026-10-23

### Fixed

- `pampas key types` failed with "No such file or directory" when `~/.config/pampas/providers.json` did not exist. It now shows the keys used by the bundled `examples/providers.json` and says so on stderr. An explicit `--config` path that is missing is still an error.

## [0.2.9] - 2026-10-23

### Added

- `pampas key types [--config <path>]` lists each key name (`api_key_env`) the configured providers use, with the providers that share it. Entries with no key, such as Ollama and Apple, are left out.

## [0.2.8] - 2026-10-23

### Changed

- Tested the OS keychain on Linux (Ubuntu with GNOME Keyring): `pampas key set`, `key list` and `key delete` work against the Secret Service, and a stored key is used when the environment variable is unset. `README.md` now says so.

## [0.2.7] - 2026-10-23

### Added

- Ollama support, local and remote, through its OpenAI-compatible endpoint: set `base_url` to `http://localhost:11434/v1` (or `http://HOST:11434/v1`) and `model` to a pulled model. `examples/providers.json` has `ollama` and `ollama-remote` entries, both `enabled: false`; turn one on with `--enable ollama`.

### Changed

- `api_key_env` is now optional. An entry without it sends no `Authorization` header, and `pampas key list` shows `no key needed`. Set it for a remote Ollama behind a proxy that wants a token.

## [0.2.6] - 2026-10-22

### Added

- `README.md`: a Providers section with tables of each working free service (entries, tested models, free-tier limits, notes), where to get each API key, and the services tried and found unusable. The long provider paragraph moved there from the quick start.

## [0.2.5] - 2026-10-22

### Added

- `examples/providers.json`: Gemini entries `gemini` (`gemini-flash-latest`) and `gemini-lite` (`gemini-flash-lite-latest`) on Google's OpenAI-compatible endpoint, using `GEMINI_API_KEY`, tested against the real API. They share a `gemini` usage group; no `daily_limit` is set because the free-tier limits were not confirmed.

## [0.2.4] - 2026-10-22

### Added

- `examples/providers.json`: Cohere entries `cohere` (`command-a-plus-05-2026`) and `cohere-r-plus` (`command-r-plus-08-2024`) on `https://api.cohere.ai/compatibility/v1`, using `COHERE_API_KEY` (a free trial key), tested against the real API. They share a `cohere` usage group with a `daily_limit` of 30.

## [0.2.3] - 2026-10-22

### Added

- `examples/providers.json`: Cloudflare Workers AI entries `cloudflare-gpt-oss` (`@cf/openai/gpt-oss-120b`), `cloudflare-llama` (`@cf/meta/llama-3.3-70b-instruct-fp8-fast`) and `cloudflare-gemma` (`@cf/google/gemma-4-26b-a4b-it`), using `CLOUDFLARE_API_KEY`, tested against the real API. Replace `YOUR_ACCOUNT_ID` in their `base_url` with your Cloudflare account id.

## [0.2.2] - 2026-10-22

### Added

- `examples/providers.json`: NVIDIA entries `nvidia-deepseek` (`deepseek-ai/deepseek-v4.1-flash`), `nvidia-glm` (`z-ai/glm-5.3-flash`) and `nvidia-nemotron` (`nvidia/nemotron-3-ultra-550b-a55b`) on `https://integrate.api.nvidia.com/v1`, using `NVIDIA_API_KEY`, tested against the real API.

## [0.2.1] - 2026-10-22

### Added

- Daily request counting, to stay inside free-tier limits. Every request to an OpenAI-compatible provider is counted per UTC day in `~/.local/state/pampas/usage.json` (`--usage-file <path>` to change it). An entry's optional `daily_limit` skips the provider once used up and refuses a mid-run request (fallback or `--merge` synthesis) that would exceed it, with a note at 80%; optional `usage_group` shares one counter between entries on the same account.
- `pampas usage` shows today's count against each limit; `--ignore-limits` overrides the limits for one run.
- `examples/providers.json`: OpenRouter limited to 50 requests a day, Free.ai to 30 (about its 1,000 a month), and the two Groq entries share a `groq` group.

## [0.2.0] - 2026-10-22

### Added

- `--format <text|markdown|md|json>` (`-f`) sets the output format; `markdown` is the default and matches the previous output. `json` prints every answer, failures included, plus the merged answer, in the shape documented in the README.
- `--output <path>` (`-o`) writes the result to a file instead of stdout. If the file cannot be written the result is printed instead and `pampas` exits with an error.

## [0.1.9] - 2026-10-22

### Added

- `--merge` consolidates the providers' answers into one: a provider (the synthesizer) is asked to state agreement once, call out disagreements by model, and add nothing new. `--synthesizer <name>` picks it, else the first entry with `"synthesizer": true`, else the first running provider; `--show-answers` also prints each separate answer. If the synthesizer fails, or only one provider answered, the separate answers are printed. Without `--merge` output is unchanged.
- Optional `"synthesizer": true` in a config entry; the example config marks `groq-120b`.

## [0.1.8] - 2026-10-22

### Added

- `--enable <name>` and `--disable <name>` flags (repeatable or comma-separated) to turn providers on or off for a run, and an optional `"enabled": false` in a config entry to keep a provider off by default. An unknown provider name, or one both enabled and disabled, is an error.

## [0.1.7] - 2026-10-22

### Added

- Apple Intelligence support: a provider entry with `"kind": "apple_fm"` asks Apple's on-device model through the new `helpers/apple-fm.swift` helper (build it with `swiftc`, see the README). It needs no key; if the helper or the model is unavailable the entry is skipped with a warning. The example config has an `apple` entry.
- Optional `command` in an `apple_fm` entry to point at the helper, default `pampas-apple-fm` on `PATH`.

## [0.1.6] - 2026-10-22

### Added

- Optional `fallback` in a provider's config entry. `"free_models"` makes a provider whose model fails (for example OpenRouter's `429` or `404` on a `:free` model) list the service's `/models` and try up to 3 free text models in order, noting the one that answered on stderr. The `openrouter` entry in `examples/providers.json` now uses it.

## [0.1.5] - 2026-10-22

### Changed

- `examples/providers.json`: the `openrouter` entry now uses `minimax/minimax-m3.1-flash:free`, tested against the real API. The previous `google/gemma-4-31b-it:free` kept returning `429` (rate-limited upstream by Google).

## [0.1.4] - 2026-10-22

### Changed

- The `groq-120b` example entry (`openai/gpt-oss-120b`) was tested against the real Groq API and works.

## [0.1.3] - 2026-10-22

### Removed

- `examples/providers.json`: the `openrouter-gpt-oss` entry. OpenRouter no longer offers `gpt-oss-120b` for free and returns `404`; no `gpt-oss` model is free there now.

## [0.1.2] - 2026-10-22

### Added

- `examples/providers.json`: `groq-120b` (`openai/gpt-oss-120b` on Groq) and `openrouter-gpt-oss` (`openai/gpt-oss-120b:free` on OpenRouter), reusing `GROQ_API_KEY` and `OPENROUTER_API_KEY`. Not yet tested against the real APIs.

## [0.1.1] - 2026-10-22

### Changed

- Gave the project a name expansion: pampas is short for Parallel AI Multi-provider Prompt Aggregator & Synthesizer. It appears in the `README.md` intro, the `pampas --help` text and the `Cargo.toml` description.

## [0.1.0] - 2026-10-22

### Changed

- Renamed the project and binary from `munchkin` to `pampas`. The default config is now `~/.config/pampas/providers.json` and keys are stored in the OS keychain under the service `pampas`; move the config directory and re-run `pampas key set <NAME>` for each key stored under the old name.

## [0.0.9] - 2026-10-21

### Added

- Optional `chat_path` in a provider's config entry (default `/chat/completions`) for services whose chat endpoint is elsewhere.
- `examples/providers.json`: a Free.ai entry (`FREEAI_API_KEY`, model `qwen7b`, `chat_path` `/chat/`), tested against the real API.

## [0.0.8] - 2026-10-21

### Removed

- The Mistral entry from `examples/providers.json`; Mistral is no longer a planned provider.

## [0.0.7] - 2026-10-21

### Added

- `examples/providers.json`: a Mistral entry (`MISTRAL_API_KEY`, model `open-mistral-nemo`), tested against the real API.

## [0.0.6] - 2026-10-21

### Added

- A secure local key store: API keys can be kept in the OS keychain (macOS Keychain, or the Secret Service on Linux) instead of the environment. `munchkin key set <NAME>` stores a key from a hidden prompt, `munchkin key delete <NAME>` removes it, and `munchkin key list` shows where each configured provider's key comes from without printing it.

### Changed

- A provider's key is now looked up in its `api_key_env` environment variable first, then in the OS keychain. Existing environment-variable setups are unaffected. The warning for a provider with no key now names the `munchkin key set` command.
- The prompt is no longer required when a subcommand is given; `munchkin -- key` sends the prompt `key`.

## [0.0.5] - 2026-10-21

### Added

- `examples/providers.json`: an OpenRouter entry (`OPENROUTER_API_KEY`, free model `google/gemma-4-31b-it:free`), tested against the real API.
- A test that `examples/providers.json` parses.

## [0.0.4] - 2026-10-21

### Added

- `examples/providers.json`: a ready-to-copy config entry for Groq (`GROQ_API_KEY`, model `openai/gpt-oss-20b`).

## [0.0.3] - 2026-10-21

### Added

- A generic OpenAI-compatible provider: any service with a `/chat/completions` API can be queried by adding it to the providers config.
- `--config <path>` flag, and a default config at `~/.config/munchkin/providers.json` (`$XDG_CONFIG_HOME` respected). API keys are read from the environment variable each entry names; a provider with a missing key is skipped with a warning.

## [0.0.2] - 2026-10-21

### Changed

- Renamed the project and binary from `mainecoon` to `munchkin`.

## [0.0.1] - 2026-10-21

### Added

- Rust project skeleton: `mainecoon <prompt>` with `--timeout`, and a parallel fan-out over a `Provider` trait (no providers yet).
- `tests/version_sync.rs` checks `Cargo.toml`, `README.md` and `CHANGELOG.md` agree on the version.
