# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack
Plain HTML, CSS and TypeScript, no UI framework (the user's decision, 2026-10-23), shown in a Tauri 2 webview on Linux (WebKitGTK), macOS (WKWebView) and Windows (WebView2). The TypeScript is compiled to `dist/` by `npm --prefix ui run build`. The Rust backend is the `pampas` library; its commands are in `src-tauri/src/main.rs`. Node and npm are build tools only.

## Users
Anyone who wants better answers than one AI chat gives, including people who have never edited a JSON file or seen an API key. Not only developers. The creator is the first and most demanding user. A typical session is longer research: ask a question, read several answers side by side, lean on the merged answer, re-ask or follow up, and come back to earlier questions later.

## Product Purpose
pampas sends one prompt to several free AI services at once and shows the answers together, optionally merged into one answer that says where the models agree and disagree. The GUI makes this usable without the command line: set up services and keys, ask, compare, merge, and see how much of each free tier has been used today. Success is a person trusting a consolidated answer more than any single model's, without spending money or reading documentation.

## Positioning
Free tiers only, many at once. It is not another chat client for one model, and it is not a paid aggregator: it queries services the user has their own free keys for, shows every answer including failures, and never routes through a middleman server. Keys stay on the user's machine (OS keychain), never in files.

## Operating Context
Desktop app on macOS, Windows and Linux, run alongside a browser and a text editor, often kept open all day. Answers arrive at very different speeds (seconds to over a minute for large local models) and some providers fail or hit daily limits, which is normal and must be shown calmly. Long answers contain markdown (headings, lists, code). Setup involves copying API keys from provider websites. The same config, keys, settings and usage files are shared with the command-line tool.

## Capabilities and Constraints
- Ask: prompt, choose which providers run, optional merge with a chosen synthesizer, optional timeout override and "ignore daily limits"; each provider's answer appears as it finishes (event per answer), then the merged answer.
- Providers: list with key status and usage; full editing of providers.json (add, edit, remove, reorder, switch on and off; unknown fields are preserved; invalid configs are never saved; the previous file is kept as `.bak`).
- Keys (secret, OS keychain) and settings (plain `${NAME}` values such as an account id) are set and deleted separately; a key that is set as an environment variable wins over the keychain.
- Usage: requests used against each daily limit, per UTC day.
- Providers fail often (missing key, daily limit reached, timeout, HTTP errors); failures are first-class results, not exceptions.
- Free tiers only; never add a paid service without asking. Keys never appear in files, logs or the UI after entry.
- The page is untrusted-output territory: model answers are text from third parties and must be rendered safely.
- Past questions and answers are saved automatically by the app (a History tab, `history.json` in the state directory, newest 200) and can be deleted singly or all at once, or saving can be switched off. The command line does not write it yet.

## Brand Commitments
Name: pampas (Parallel AI Multi-provider Prompt Aggregator & Synthesizer). The app icon is the user's tiger image (`pampas.png`, 512 px; attribution to Flaticon's tiger icon by Magnific in the README). Terse, plain, precise voice, as in the command line's messages; no marketing language.

## Evidence on Hand
Working backend with the real command set, a real example config of 17 providers (Groq, OpenRouter, Free.ai, NVIDIA, Cloudflare, Cohere, Gemini, Ollama local and remote, Apple on-device) and real status data. No screenshots, user research or testimonials exist; none may be invented.

## Product Principles
1. Show every answer and every failure; never hide a provider that did not answer, and say why in plain words.
2. The merged answer is a reading of the others, not a verdict: always keep the separate answers one step away and name who said what.
3. Setup is part of the product: a first run with no config and no keys must lead somewhere useful without documentation, and jargon (base URL, api_key_env, usage group) gets a plain explanation next to it.
4. Free means careful: make the cost of a request, and the limit it counts against, visible before and after asking.
5. Secrets are write-only: a key can be set, replaced or deleted, never displayed.

## Accessibility & Inclusion
Keyboard-first: every action reachable and operable without a mouse, visible focus, sensible tab order, Cmd/Ctrl+Enter to ask. Screen-reader usable: labelled controls, live regions for answers arriving and for errors, no information carried by colour alone. Follows the system light and dark theme and the reduced-motion setting. Text must stay readable at larger system font sizes and at narrow window widths down to about 640 px. Must look at home on macOS, Windows and Linux, using system fonts and honouring each platform's conventions rather than one fixed look.
