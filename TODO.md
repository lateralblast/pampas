# TODO

Status: `[ ]` open, `[~]` in progress, `[x]` done.

## Open

- [ ] Run the Windows installers on a real Windows machine (the VM-built MSVC one and the MinGW one), and the app and `pampas.exe` in them: the keychain (Credential Manager), file locations under `%APPDATA%`, the save dialog, the WebView2 runtime on Windows 10. Nothing built for Windows has been run on Windows.
- [ ] Try the macOS build on a Mac: `./build.sh --target mac` (and `--arch arm64|x64|universal`), the ad hoc signature, the keychain, the local-network permission for a LAN Ollama server, and Gatekeeper on first open. Written from bobcat's process but never run.
- [ ] Try the Linux AppImage (`--package appimage` or `all`) and the arm64 container build (`--docker --arch arm64`), and install the built `.deb` and `.rpm` on clean machines.
- [ ] Windows VM: the first install stopped after the Rust step (as bobcat's did) and the later tools were installed by running `setup.cmd` through the watcher; the cause is unknown (the log shows nothing between Rust and the end). A fresh install now recovers on its first build, but find out why. Also `windows/oem/refresh.cmd` does not always restart the watcher, and the first request after a restart can take minutes to be noticed.
- [ ] Optional: release automation (tag, push and a GitHub release with the files in `dist/`), as bobcat's `build.sh --release` does; left out on purpose.
- [ ] History: the command line does not write to `history.json` yet; add saving CLI runs (opt in, since it records prompts and answers) and `pampas history list|show|delete`. Also: search inside the answers, not only the questions, and export a session (reuse **Save as…**) from the list.
- [ ] Save as…: the save dialog has not been opened in a test (it needs a real desktop), only the rendering, file names and the calls to the backend. Try it on Linux, macOS and Windows; check the suggested name and filter, cancelling, and saving over an existing file. Consider also a command-line `--copy` (clipboard) and choosing `--format` from the `--output` file extension.
- [ ] GUI polish: external links in answers are shown as text because a click would navigate the app window; add Tauri's opener plugin to open them in the browser. Also: cancel a running question, and test with a real screen reader (VoiceOver, NVDA, Orca) and on the macOS and Windows web views.
- [ ] GUI 6 (CI): the build matrix must run `npm --prefix ui ci`, `npm --prefix ui test` and `cargo test --workspace` on each platform before bundling.
- [ ] GUI 5, Apple Intelligence on macOS: bundle `pampas-apple-fm` as a Tauri sidecar (a per-target-triple binary) so the app needs no `PATH` setup; hide the provider on other platforms.
- [ ] GUI 6, packaging and CI: a GitHub Actions matrix on macOS, Windows and Ubuntu runners (no cross-building); decide on code signing (Apple developer account for macOS Gatekeeper and stable keychain access, Windows SmartScreen) and the WebView2 installer on Windows 10.
- [ ] Windows and macOS CLI audit (run on real machines; `src/paths.rs` has only been tested with fake environments): confirm `cargo build` and the tests pass on Windows (keychain via Credential Manager, `rpassword`, paths without `HOME`, the `cfg(unix)` `SIGPIPE` code), and note anything Unix-only in the README.
- [ ] Keychain: try KWallet (KDE) and a headless Linux session.
- [ ] Cloudflare Workers AI's free plan is 10,000 Neurons a day (confirmed), not requests, so `daily_limit` cannot express it; consider estimating Neurons from the response's token usage and the per-model rates on its pricing page.
- [ ] NVIDIA: about 40 requests a minute is only user-reported; check the limit shown on your build.nvidia.com dashboard. Also retry `openai/gpt-oss-20b` and `google/gemma-4-31b-it`, which timed out after 90s on 2026-10-22 (`moonshotai/kimi-k2.6` returned 404 for the account).
- [ ] Groq: confirm the free-plan limits in the Groq console (the docs show only the Developer plan: 30 a minute, 1,000 a day for `gpt-oss`) and set `daily_limit` on the Groq example entries.
- [ ] Usage counting: support monthly limits (Free.ai), and read providers' rate-limit response headers (Groq sends `x-ratelimit-remaining-requests`) instead of only counting locally.
- [ ] Merge: the synthesis prompt was only tried on one question; test it on prompts the models disagree on, and look at how `--format json` presents it.
- [ ] Apple Intelligence: the helper answers one prompt per process with no system instructions, streaming or session reuse, and its unavailable-reason text is untested on a Mac with Apple Intelligence off.
- [ ] Gemini: Google publishes no free-tier numbers, and third-party reports conflict; read your limits in AI Studio and set `daily_limit` on the `gemini*` example entries.
- [ ] Support non-OpenAI-compatible services if any chosen one needs it. Parked 2026-10-23: no chosen service needs it. When one does, prefer a generic `kind: "http"` adapter configured in JSON (URL, headers, request body template with `${prompt}`/`${model}`, response path to the text) over a Rust implementation per service, and build it against that real service. Free tiers only; ask before adding a paid one.

## Done

- [x] Installer builds (0.5.0): `build.sh` with Linux native and container, a Windows 11 VM and a MinGW container for Windows, and bobcat's macOS process copied; Linux `.deb`/`.rpm`, a container `.deb`, and Windows installers from both Windows routes were built and the VM route cold-start was verified.
- [x] History tab: finished questions saved automatically to `history.json` (library `pampas::history`), listed by day, opened in full, asked again, deleted singly or all at once, filtered by question, with a switch to stop saving; the old local-storage history is moved in once (0.4.9).
- [x] Copy buttons in the app confirm visibly ("Copied"), and **Save as…** saves an answer set as Markdown, plain text or JSON through the system save dialog, rendered by the command line's own formatter (0.4.8).
- [x] Screenshots of the GUI tabs (`docs/screenshots/`) in the README, taken from the interface running against the development mock backend (sample data); retake them when the interface changes.
- [x] GUI front end (`ui/`, plain HTML, CSS and TypeScript): Ask, Sources (full editor), Keys & settings and Usage pages in the "critical edition" design, with first-run steps, earlier questions, safe markdown rendering, keyboard and screen-reader support, light and dark themes. Checked in headless Chrome against a mock backend and in the real window on Linux; not yet seen in the macOS or Windows web views.
- [x] GUI backend (`src-tauri`, Tauri 2, in a Cargo workspace with the CLI): list providers, config get/validate/save/create (field order kept, `.bak` backup, atomic write), keys, settings, usage and `ask` with per-answer events; the command logic is in `src-tauri/src/logic.rs` and tested without a window. Run in a window on Linux (X11, under Xvfb) and reached through `./pampas --gui`; the page is a placeholder until GUI 4. Not yet run on macOS or Windows.
- [x] Library split: `src/lib.rs` and `src/engine.rs` hold the run logic and return warnings and rows as data; `src/main.rs` is the command line only. No behaviour change; the existing tests moved with their code. Provider-internal notices (a fallback model answered, 80% of the daily limit used) still go straight to stderr; give them a channel when the GUI needs them.
- [x] Platform file locations (`src/paths.rs`): native directories on macOS and Windows, XDG kept on Linux, `pampas migrate` and an old-location fallback for macOS. Unit-tested with fake environments for all three platforms; not yet run on macOS or Windows.
- [x] Per-entry `timeout` (seconds) for slow providers such as large local Ollama models; `--timeout` overrides it for one run, the default stays 30.
- [x] A missing default providers config is offered for creation from the bundled example on a terminal (`[y/N]`); `--config` paths and non-terminals keep the plain error.
- [x] Fixed the "Broken pipe" panic when output is piped into a command that exits early (`pampas usage | head -1`) by restoring the default `SIGPIPE` handling at startup.
- [x] Options before a subcommand (`pampas --config <path> key list`) now work: the cause was `args_conflicts_with_subcommands` on the top-level command, which clap applies even to global options.
- [x] Reworded the "No default store has been set" keychain error to say the keychain is unavailable and to use an environment variable instead.
- [x] Confirmed Cohere's trial-key limits (1,000 calls a month, 20 a minute per Chat model) from its docs; `daily_limit` of 30 stays. Checked Cloudflare (10,000 Neurons a day, 300 requests a minute), Groq, NVIDIA and Gemini; findings are in the README's Providers table.
- [x] `${NAME}` placeholders in `base_url` (environment first, then `pampas setting`), so the Cloudflare account id and the remote Ollama host are set once and the example config needs no editing.
- [x] `pampas key types` lists the key names the config uses and which providers share each.
- [x] Tried the keychain on Linux (Ubuntu, GNOME Keyring through the Secret Service): `pampas key set`, `key list` and `key delete` work, the key is stored under service `pampas` and visible to `secret-tool`.
- [x] Ollama support, local and remote, via the OpenAI-compatible endpoint and an optional `api_key_env`; tested against a local and a remote server.
- [x] Example config entries for Gemini (`gemini-flash-latest`, `gemini-flash-lite-latest`) with a key from a new AI Studio project, tested against the real API; the earlier `403` was the old key's project. Cerebras was dropped (its no-card free tier ended).
- [x] Example config entries for Cohere's trial key (Command A Plus, Command R Plus), tested against the real API.
- [x] Example config entries for Cloudflare Workers AI (`gpt-oss-120b`, Llama 3.3 70B, Gemma 4 26B), tested against the real API.
- [x] Example config entries for NVIDIA's hosted models (DeepSeek V4.1 Flash, GLM 5.3 Flash, Nemotron 3 Ultra), tested against the real API.
- [x] Daily request counting with `daily_limit` / `usage_group`, `pampas usage`, `--usage-file` and `--ignore-limits`; limits enforced at startup and per request.
- [x] `--format` (text, markdown/md, json; shape documented in the README) and `--output <path>`.
- [x] Merge: `--merge` has a chosen provider synthesize the answers (`--synthesizer`, `--show-answers`); side by side stays the default.
- [x] `--enable` / `--disable` flags and a per-entry `enabled` config field.
- [x] Apple Intelligence on-device model as an `apple_fm` provider via a Swift helper; tested on macOS 27.2 (arm64).
- [x] `fallback: "free_models"` provider option: when the configured model fails, try the service's free models; tested against OpenRouter.
- [x] Replaced the rate-limited Gemma model in the `openrouter` example entry with `minimax/minimax-m3.1-flash:free`, tested against the real API.
- [x] Tested the `groq-120b` example entry against the real Groq API.
- [x] Removed the `openrouter-gpt-oss` example entry: OpenRouter no longer offers `gpt-oss-120b` free (`404`).
- [x] Example entries for OpenAI's open-weight `gpt-oss-120b` on Groq and OpenRouter (OpenAI's own API has no free tier; untested, see Open).
- [x] Example config entry for Free.ai and a `chat_path` config option, tested against the real API.
- [x] Removed the Mistral example entry; Mistral is dropped.
- [x] Example config entry for Mistral, tested against the real API.
- [x] Secure local key store in the OS keychain, with `pampas key set|delete|list`.
- [x] Example config entry for OpenRouter, tested against the real API.
- [x] Tested the Groq entry against the real API.
- [x] Example config entry for Groq.
- [x] Generic OpenAI-compatible provider with a JSON config file; API keys read from environment variables.
- [x] Project skeleton with parallel fan-out.
