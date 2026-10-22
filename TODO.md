# TODO

Status: `[ ]` open, `[~]` in progress, `[x]` done.

## Open

- [ ] Confirm Cohere's trial-key limits (calls per month, requests per minute) and adjust the `cohere` entries' `daily_limit`, now an assumed 30 a day from a recalled 1,000 a month.
- [ ] Cloudflare Workers AI's free plan is limited in Neurons per day, not requests, so `daily_limit` cannot express it; confirm the limit and consider estimating Neurons or reading its response headers.
- [ ] Confirm NVIDIA's free-tier limits and set a `daily_limit` / `usage_group` accordingly. Also retry `openai/gpt-oss-20b` and `google/gemma-4-31b-it` on NVIDIA, which timed out after 90s on 2026-10-22 (`moonshotai/kimi-k2.6` returned 404 for the account).
- [ ] Find Groq's real free-tier limits (they vary by model; see the limits page in the Groq console) and set `daily_limit` on the Groq example entries.
- [ ] Usage counting: support monthly limits (Free.ai), and read providers' rate-limit response headers (Groq sends `x-ratelimit-remaining-requests`) instead of only counting locally.
- [ ] Subcommands only work before any flags (`pampas key list --config <path>`, not `pampas --config <path> key list`); find out why clap treats the subcommand as the prompt there and fix it.
- [ ] Merge: the synthesis prompt was only tried on one question; test it on prompts the models disagree on, and look at how `--format json` presents it.
- [ ] Apple Intelligence: the helper answers one prompt per process with no system instructions, streaming or session reuse, and its unavailable-reason text is untested on a Mac with Apple Intelligence off.
- [ ] Try the keychain on a Linux machine with a running Secret Service (written from the crate's documented behaviour; only macOS has been run).
- [ ] Find Gemini's real free-tier limits (AI Studio shows them per model) and set `daily_limit` on the `gemini*` example entries.
- [ ] Support non-OpenAI-compatible services if any chosen one needs it.

## Done

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
