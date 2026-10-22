# Changelog

Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
