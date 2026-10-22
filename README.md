![Pampas cat](pampas.png)

# pampas

Version: 0.3.6

**pampas** is short for Parallel AI Multi-provider Prompt Aggregator & Synthesizer: a CLI that sends one prompt to several free AI services in parallel and merges the results.

## Contents

- [Quick start](#quick-start)
- [Providers](#providers)
- [Flags](#flags)
- [Usage limits](#usage-limits)
- [Output formats](#output-formats)
- [Merging answers](#merging-answers)
- [Apple Intelligence](#apple-intelligence)
- [Ollama](#ollama)
- [Cloudflare Workers AI](#cloudflare-workers-ai)
- [API keys](#api-keys)
- [Settings](#settings)
- [Environment variables](#environment-variables)

## Quick start

```
cargo run -- "your prompt"
```

Ready-to-copy entries for every working free service are in `examples/providers.json`: copy it to `~/.config/pampas/providers.json`, get a key for each service you want (see [Providers](#providers)) and provide it through its environment variable or `pampas key set` (see [API keys](#api-keys)). A provider with no key is skipped with a warning, so you can start with one.

Providers are listed in `~/.config/pampas/providers.json`, a JSON array. Any service with an OpenAI-compatible `/chat/completions` API works:

```json
[
  {
    "name": "groq",
    "base_url": "https://api.groq.com/openai/v1",
    "model": "<model id>",
    "api_key_env": "GROQ_API_KEY"
  }
]
```

`chat_path` is optional (default `/chat/completions`): the path appended to `base_url` for chat requests, for services that use a different one, such as Free.ai's `/chat/`. `fallback` is optional: `"free_models"` makes a provider whose `model` fails (an HTTP error such as `429` or `404`; a timeout does not trigger it) list the service's `/models` and try up to 3 free text models in listed order (a model counts as free when its prompt and completion prices are both `0` and its id ends in `:free`; safety classifiers and non-text models are skipped). When a fallback model answers, a notice naming it goes to stderr. The whole attempt, fallbacks included, shares the `--timeout`, so raise it (for example `--timeout 60`) for providers with a fallback. It needs a service that lists pricing in `/models`, as OpenRouter does, and the example `openrouter` entry uses it. `synthesizer` is optional (default `false`): with `--merge`, the first running entry marked `true` writes the consolidated answer (see [Merging answers](#merging-answers)). `daily_limit` is optional: the most requests to make per UTC day, after which the provider is skipped (see [Usage limits](#usage-limits)). `usage_group` is optional: the name requests are counted under, default the entry's own name; entries that share one account's quota should share a group. `enabled` is optional (default `true`): `false` leaves the provider out of runs unless `--enable <name>` is given; it works on `apple_fm` entries too. `base_url` may contain `${NAME}` placeholders, filled from the environment variable `NAME` (see [Environment variables](#environment-variables)); a provider whose variable is unset is skipped with a warning. `api_key_env` names the key: see [API keys](#api-keys); leave it out for a service that needs no key, such as Ollama (see [Ollama](#ollama)). A provider whose named key is missing is skipped with a warning. Each provider's answer is printed under a `## name` heading; failures go to stderr.

## Providers

Status as found while testing on 2026-10-22, each model with a real key. Free tiers change often: where a limit is marked "unconfirmed" it comes from a third-party listing or memory, not from the service, so check the service's own page before relying on it.

### Working free services

| Service | Entries in `examples/providers.json` | Models tested and working | Free-tier limits | Notes |
|---|---|---|---|---|
| **Groq** | `groq`, `groq-120b` | `openai/gpt-oss-20b`, `openai/gpt-oss-120b` | Not confirmed; they vary by model (a third-party list said 30 requests a minute, 14,400 a day, unverified). No `daily_limit` set. | Fast. The two entries share a `groq` usage group. The nearest free route to OpenAI models. |
| **OpenRouter** | `openrouter` | `minimax/minimax-m3.1-flash:free` | 50 requests a day and 20 a minute on `:free` models without credit (OpenRouter's own figure); `daily_limit` is 50. | The set of free models changes: of 18 listed free models, 11 answered, `google/gemma-4-31b-it:free` returned `429` (rate-limited upstream), `openai/gpt-oss-120b:free` was removed (`404`), and two returned `403`. The entry has `"fallback": "free_models"`, which tries other free models when its own fails. |
| **Free.ai** | `freeai` | `qwen7b` | 30,000 tokens a day and 1,000 API requests a month; `daily_limit` is 30 (about the monthly cap). | Chat endpoint is `/chat/` (`chat_path`). Once returned `503` "chat queue is currently full". |
| **NVIDIA** | `nvidia-deepseek`, `nvidia-glm`, `nvidia-nemotron` | `deepseek-ai/deepseek-v4.1-flash`, `z-ai/glm-5.3-flash`, `nvidia/nemotron-3-ultra-550b-a55b` | About 40 requests a minute reported (unconfirmed), meant for development; no `daily_limit`. | 83 models listed. `openai/gpt-oss-20b` and `google/gemma-4-31b-it` timed out after 90 s, `moonshotai/kimi-k2.6` returned `404` for the account. Entries share an `nvidia` group. |
| **Cloudflare Workers AI** | `cloudflare-gpt-oss`, `cloudflare-llama`, `cloudflare-gemma` | `@cf/openai/gpt-oss-120b`, `@cf/meta/llama-3.3-70b-instruct-fp8-fast`, `@cf/google/gemma-4-26b-a4b-it` | 10,000 "Neurons" a day (reported, unconfirmed), which `pampas` cannot count; no `daily_limit`. | Set `CLOUDFLARE_ACCOUNT_ID`; each `base_url` reads it. `@cf/zai-org/glm-5.3-flash` returns `403`: not on the free plan. Entries share a `cloudflare` group. |
| **Cohere** (trial key) | `cohere`, `cohere-r-plus` | `command-a-plus-05-2026`, `command-r-plus-08-2024` (also `command-a-03-2025`, `command-r7b-12-2024`) | About 1,000 calls a month (recalled, unconfirmed), non-commercial use only; `daily_limit` is 30. | OpenAI-compatible endpoint `https://api.cohere.ai/compatibility/v1`. Entries share a `cohere` group. |
| **Google Gemini** | `gemini`, `gemini-lite` | `gemini-flash-latest`, `gemini-flash-lite-latest` | Not confirmed; AI Studio shows them per model. No `daily_limit` set. | The `-latest` aliases keep working as models change; `gemini-2.5-flash` returned `404` (retired for new keys) and `gemma-4-31b-it` a `500`. The key must come from a new AI Studio project, or it can return `403 PERMISSION_DENIED`. Google may use free-tier prompts to improve its products. Entries share a `gemini` group. |
| **Ollama** (local or remote) | `ollama`, `ollama-remote` | `qwen2.5:3b` (local), `gemma4:12b` (remote) | Local and unlimited; set a `daily_limit` yourself if you want a cap. | No key. Both entries are `enabled: false` in the example, so use `--enable ollama`. See [Ollama](#ollama). |
| **Apple Intelligence** | `apple` | Apple's on-device model | Local and unlimited; not counted. | Needs macOS 26 or later on Apple silicon with Apple Intelligence on, and the Swift helper (see [Apple Intelligence](#apple-intelligence)). Skipped if unavailable. |

### Getting API keys

Store each key with `pampas key set <ENV VAR>` (typed at a hidden prompt) or export it; see [API keys](#api-keys).

| Service | Environment variable | Where to get it |
|---|---|---|
| Groq | `GROQ_API_KEY` | https://console.groq.com/keys: sign in, **Create API Key**. |
| OpenRouter | `OPENROUTER_API_KEY` | https://openrouter.ai/settings/keys: sign in, **Create Key**. No card needed for `:free` models. |
| Free.ai | `FREEAI_API_KEY` | Sign up at https://free.ai and create a key in your account (the exact page was not recorded; keys start `sk-free-`). |
| NVIDIA | `NVIDIA_API_KEY` | https://build.nvidia.com: sign in with an email, open any model and choose **Get API Key** (keys start `nvapi-`). |
| Cloudflare | `CLOUDFLARE_API_KEY` | https://dash.cloudflare.com/profile/api-tokens: **Create Token** with the **Workers AI** permission. Also copy your account id (the 32-character id in the dashboard URL, or on the Workers & Pages overview) and export it as `CLOUDFLARE_ACCOUNT_ID`; the entries' `base_url` reads it. |
| Cohere | `COHERE_API_KEY` | https://dashboard.cohere.com/api-keys: the default **Trial** key is enough. |
| Google Gemini | `GEMINI_API_KEY` | https://aistudio.google.com/apikey: **Create API key**, choosing **Create API key in new project**. Do not enable billing on the project. |
| Ollama | none | No key; see [Ollama](#ollama). |
| Apple Intelligence | none | No key; build the helper as described below. |

### Tried and not usable for free

| Service | Why not |
|---|---|
| **OpenAI API** | No free tier. ChatGPT's free tier is the web app only, with no API; scraping it breaks OpenAI's terms and is fragile. Open-weight `gpt-oss` models are available free through Groq, NVIDIA (`gpt-oss-20b`, slow) and Cloudflare instead. |
| **xAI (Grok)** | No free tier. |
| **Mistral** | No free tier. |
| **DeepSeek API** | Pay-as-you-go; third-party sources mention a one-off trial credit that expires, unconfirmed. DeepSeek V4.1 Flash is free through NVIDIA (`nvidia-deepseek`). |
| **Cerebras** | The no-card free tier ended; new accounts get a $5 trial credit that expires after 30 days and needs a payment method (reported). |
| **SambaNova** | A key with a zero balance returned `402 PAYMENT_METHOD_REQUIRED` for every model; a payment method is required. |
| **Hugging Face Inference Providers** | A free account had no credits (`402`, "subscribe to PRO to get monthly included credits"). |
| **GitHub Models** | One source says it was retired on 30 July 2026; not verified here. |

## Flags

- `<prompt>`: the prompt sent to every provider.
- `--timeout <secs>`: per-provider timeout in seconds (default 30).
- `--enable <name>`: run a provider whose config entry has `"enabled": false`. Repeatable, or comma-separated (`--enable a,b`).
- `--disable <name>`: leave a provider out of this run. Repeatable, or comma-separated. A name that matches no configured provider, or is both enabled and disabled, is an error.
- `--merge`: consolidate the answers into one, see [Merging answers](#merging-answers).
- `--synthesizer <name>`: with `--merge`, the provider that writes the consolidated answer.
- `--show-answers`: with `--merge`, also print each provider's own answer after the consolidated one.
- `-f, --format <text|markdown|md|json>`: output format, see [Output formats](#output-formats) (default `markdown`).
- `-o, --output <path>`: write the result to this file instead of stdout, overwriting it. A `wrote <path>` note goes to stderr. If the file cannot be written, the result is printed to stdout instead and `pampas` exits with an error.
- `--ignore-limits`: ignore every `daily_limit` for this run; requests are still counted.
- `--usage-file <path>`: where request counts are kept (default `~/.local/state/pampas/usage.json`, `$XDG_STATE_HOME` respected). Also accepted by `pampas usage`.
- `--config <path>`: providers config file (default `~/.config/pampas/providers.json`). Also accepted by `pampas key list`.
- `pampas init`: write the bundled example config to the config path (`--config`, default `~/.config/pampas/providers.json`); it never overwrites an existing file. Run it first, then edit the file.
- `pampas setting set|delete|list`: manage the values behind `${NAME}` placeholders in `base_url`, see [Settings](#settings).
- `pampas key set|delete|list|types`: manage stored keys, see [API keys](#api-keys). `pampas usage`: show today's request counts, see [Usage limits](#usage-limits). A subcommand must come before any flags (`pampas key list --config <path>`), and a prompt that is literally `key` or `usage` must be written `pampas -- key`.

## Usage limits

Free tiers cap requests per day, and going over can get a key throttled or blocked. `pampas` therefore counts every request it makes to an OpenAI-compatible provider, per UTC day (the day most services reset on), in the file given by `--usage-file`. Apple Intelligence entries are local and not counted.

- **What counts:** each request to the chat endpoint, counted before it is sent, so a failed request (a `429`, say) still counts. A `fallback` model attempt and the `--merge` synthesis call each count as one request. Listing `/models` for a fallback does not.
- **Limits:** set `"daily_limit": N` on an entry. Once the count reaches `N` the provider is skipped with a warning at startup, and a request that would exceed it mid-run (a fallback, or the synthesis call) is refused rather than sent. At 80% of the limit each request prints a `used of limit` note on stderr. `--ignore-limits` turns this off for a run.
- **Sharing:** entries that draw on one account's quota can share a counter with the same `"usage_group"`. Each entry still checks the shared count against its own `daily_limit`. The example config groups the two Groq entries.
- **Viewing:** `pampas usage` prints a table of each entry's name, usage group and `used/limit` (`-` if there is no limit). For example, after a few requests:

```
$ pampas usage
PROVIDER            GROUP          USED/LIMIT
groq                groq           12/-
groq-120b           groq           12/-
openrouter          openrouter     41/50
freeai              freeai         3/30
cohere              cohere         0/30
gemini              gemini         7/-
ollama              ollama         0/-
apple               -              local, not counted
```

  The two Groq entries show the same count because they share a usage group. The list is shortened here; it has one row per config entry.
- **Limits of the counter:** it only knows the requests `pampas` made on this machine, so use from other tools or machines is not seen, and the file is not locked, so two `pampas` runs at the same moment can lose a count. Limits are daily; a monthly cap such as Free.ai's 1,000 requests a month is approximated by its example `daily_limit` of 30. An unreadable counts file is moved to `usage.json.corrupt` with a warning and counting starts again from zero.
- **Example limits:** the example config sets OpenRouter to 50 a day (its limit for free models without credit) and Free.ai to 30. Groq has none set because its limits vary by model and were not confirmed; check your own limits on Groq's console and add `daily_limit` yourself.

## Output formats

`--format` chooses how the result is written, to stdout or to `--output <path>`. Failures and warnings always go to stderr, whatever the format.

- `markdown` (alias `md`, the default): each answer under a `## name` heading.
- `text`: each answer under a `name:` line, with no markup.
- `json`: one pretty-printed JSON object:

```json
{
  "prompt": "the prompt",
  "answers": [
    { "provider": "groq", "ok": true, "answer": "..." },
    { "provider": "openrouter", "ok": false, "error": "HTTP 429 ..." }
  ],
  "merged": { "provider": "groq-120b", "answer": "..." },
  "merge_error": null
}
```

`answers` lists every provider that ran, in config order, failures included (`ok` false, with `error`). `merged` is `null` unless `--merge` produced a consolidated answer; `merge_error` is a string when `--merge` was given and failed (then `merged` is `null`), else `null`. Key order is alphabetical. `json` always includes the separate answers, so `--show-answers` has no effect on it.

With `--merge`, markdown and text show the merged answer first (as `merged (by <name>)`) and the separate answers only with `--show-answers`, or instead of it if merging failed.

## Merging answers

By default each provider's answer is printed under its name. With `--merge`, `pampas` collects the answers and asks one provider, the synthesizer, to write a single answer: points all the answers agree on stated once, disagreements called out with the models that differ, and nothing added that no answer said. The output is a `## merged (by <name>)` section; add `--show-answers` to follow it with the separate answers.

The synthesizer is `--synthesizer <name>` if given, else the first running entry with `"synthesizer": true`, else the first running provider. It must be a running provider (enabled, with a key, or with its Apple model available), and it also answers the prompt like the others.

Merging costs one extra request to the synthesizer, which matters on free tiers, and the `--timeout` applies to it again. If only one provider answered, that answer is printed as is. If no provider answered, `pampas` exits with an error. If the synthesizer fails, the separate answers are printed instead, with a warning on stderr. A synthesis is a model's reading of the other answers, so check disagreements against the separate answers (`--show-answers`) when it matters.

## Apple Intelligence

On a Mac with Apple Intelligence (macOS 26 or later, Apple silicon), `pampas` can also ask Apple's on-device model. It is free, offline and needs no key. Apple only exposes it to Swift, so a small helper, `helpers/apple-fm.swift`, bridges it. Build it once and put it on your `PATH`:

```
swiftc -O helpers/apple-fm.swift -o ~/.local/bin/pampas-apple-fm
```

Then add an entry with `"kind": "apple_fm"` (the example config has one):

```json
{ "name": "apple", "kind": "apple_fm" }
```

`command` is optional (default `pampas-apple-fm`, looked up on `PATH`; a value with a slash is used as a path). Entries without `kind` are OpenAI-compatible as before. At startup `pampas` runs `pampas-apple-fm --check`; if the helper is missing or the model is unavailable (Apple Intelligence off, unsupported Mac, model still downloading), the entry is skipped with a warning and the other providers still run, so the same config works on any machine. `pampas key list` shows `no key needed` for these entries.

## Ollama

[Ollama](https://ollama.com) runs models on your own machine or on one you control. It serves an OpenAI-compatible API under `/v1`, so pampas needs only a config entry, with no key and no code. Leave `api_key_env` out and no `Authorization` header is sent.

### Local

1. Install Ollama and pull a model: `ollama pull qwen2.5:7b`. `ollama list` shows what you have.
2. Add an entry:

```json
{ "name": "ollama", "base_url": "http://localhost:11434/v1", "model": "qwen2.5:7b" }
```

3. Run it: `pampas "your prompt"`. If the entry has `"enabled": false`, add `--enable ollama`.

`model` must be exactly a name from `ollama list`, tag included (`qwen3:14b`, not `qwen3`).

### Remote

Put the other machine's address in the `base_url`, here through an environment variable so the config holds no addresses (`export OLLAMA_REMOTE_HOST=<address>`):

```json
{ "name": "ollama-remote", "base_url": "http://${OLLAMA_REMOTE_HOST}:11434/v1", "model": "gemma4:12b" }
```

By default Ollama listens only on `127.0.0.1`, so on the server start it with `OLLAMA_HOST=0.0.0.0 ollama serve`. If it runs as a systemd service, run `sudo systemctl edit ollama`, add `Environment="OLLAMA_HOST=0.0.0.0"` under `[Service]`, then `sudo systemctl restart ollama`. Open port 11434 in any firewall. Ollama has no authentication of its own, so expose it only on a trusted network. If a reverse proxy in front of it wants a bearer token, set `api_key_env` to the variable that holds it (see [API keys](#api-keys)).

Check that the server is reachable and see its models with `curl http://$OLLAMA_REMOTE_HOST:11434/api/tags`. A `Connection refused` means the server is not running or not listening beyond localhost.

### Notes

- Local models can be slow, especially on the first request while the model loads into memory. Raise `--timeout` (default 30 seconds) if an entry times out; a large model often needs `--timeout 180` or more.
- Several Ollama entries on one machine run at the same time and compete for it. A large model alongside others can be slow or run out of memory, so enable only the ones you need.
- Requests are counted like any other entry. With no `daily_limit` nothing is ever skipped; add one if you want a cap.
- `fallback: "free_models"` does not apply: Ollama's model list has no pricing.
- Tested against a local Ollama and a remote one on the LAN.

## Cloudflare Workers AI

The three `cloudflare-*` example entries need two things from your Cloudflare account: an API token (secret) and your account id (not secret, but part of the endpoint URL).

1. **Account id.** In the Cloudflare dashboard (`dash.cloudflare.com`) it is in any of these places; the layout changes now and then, so treat the exact labels as approximate:
   - the address bar after you log in: `https://dash.cloudflare.com/<32-character id>/home`;
   - **Account Home**, in the three-dot menu next to the account name: **Copy account ID**;
   - the **Workers & Pages** overview, in the right-hand sidebar;
   - **AI** > **Workers AI**, which shows the account id and a ready-made API URL (useful to confirm you picked the account that has Workers AI if you have several).
2. **API token.** Create one at https://dash.cloudflare.com/profile/api-tokens (**Create Token**) with the **Workers AI** permission.
3. **Store them.** The token is a secret, so it goes in the keychain; the account id is a setting:

```
pampas key set CLOUDFLARE_API_KEY
pampas setting set CLOUDFLARE_ACCOUNT_ID      # asks for the id; or add it: ... CLOUDFLARE_ACCOUNT_ID <id>
```

   Either can be an environment variable instead (`CLOUDFLARE_API_KEY`, `CLOUDFLARE_ACCOUNT_ID`); the environment wins. The entries' `base_url` is `https://api.cloudflare.com/client/v4/accounts/${CLOUDFLARE_ACCOUNT_ID}/ai/v1`, so they are skipped with a message until the account id is set (see [Settings](#settings)).
4. **Check.** `pampas key list` should show `CLOUDFLARE_API_KEY` found, and `pampas setting list` should show `CLOUDFLARE_ACCOUNT_ID` set.

Notes: the free plan is limited in Neurons per day, not requests, so `daily_limit` cannot express it (see `TODO.md`); the three entries share a `cloudflare` usage group. Models tested are listed under [Providers](#providers); `@cf/zai-org/glm-5.3-flash` returns `403` on the free plan. Not tested here: the account-id locations above come from memory of the dashboard, not from a fresh look.

## API keys

Each provider's `api_key_env` is the name of its key. pampas looks for the key in this order:

1. The environment variable of that name (an empty value counts as unset).
2. The OS keychain: macOS Keychain, or the Secret Service on Linux (GNOME Keyring, KWallet, or anything else that implements it). Entries are stored under the service `pampas`, named by `api_key_env`. Tested on macOS and on Ubuntu with GNOME Keyring (`secret-tool search service pampas` lists the stored keys); KWallet is untested.

Keys never go in the providers config file.

- `pampas key set <NAME> [--stdin]`: store a key, e.g. `pampas key set GROQ_API_KEY`. The key is typed at a hidden prompt, so it never appears on screen or in shell history. With `--stdin` the key is read from standard input instead, for scripts and password managers: `pass show groq | pampas key set GROQ_API_KEY --stdin`, or `pampas key set GROQ_API_KEY --stdin < keyfile`. There is deliberately no way to pass the key as an argument: it would sit in shell history, which a program cannot edit, and in `ps` output while the command runs. If you do type a key into the shell by mistake, delete the history line yourself (`history -d`/`fc`, or edit `~/.zsh_history`) and treat the key as exposed.

  ```
  $ pampas key set GROQ_API_KEY
  GROQ_API_KEY:                      <- typed at a hidden prompt, nothing is echoed
  stored GROQ_API_KEY in the OS keychain

  $ pass show groq | pampas key set GROQ_API_KEY --stdin
  stored GROQ_API_KEY in the OS keychain

  $ pampas key set GROQ_API_KEY --stdin < ~/groq.key
  stored GROQ_API_KEY in the OS keychain
  ```

  Trailing whitespace and the final newline are trimmed, and empty input is an error (`no key entered`). Delete a key file once it is stored.
- `pampas key delete <NAME>`: remove a stored key.
- `pampas key types [--config <path>]`: a table (`KEY`, `PROVIDERS`) of the key names the config uses (the names to give `key set`), with the providers that use it, comma-separated. Providers with no key are left out. With no config file at the default path it lists the keys of the bundled example config instead and says so on stderr.
- `pampas key list [--config <path>]`: a table (`PROVIDER`, `KEY`, `SOURCE`) with one row per configured provider: where its key comes from (`environment variable`, `OS keychain`, `missing`, `no key needed`, or `error: ...` if the keychain itself failed). Keys themselves are never printed. `key list`, `key types` and `usage` print an aligned table with a header row on a terminal, and bare tab-separated lines (no header) when piped, so scripts can parse them.

  ```
  $ pampas key list
  PROVIDER            KEY                 SOURCE
  groq                GROQ_API_KEY        environment variable
  openrouter          OPENROUTER_API_KEY  missing
  gemini              GEMINI_API_KEY      OS keychain
  ollama              -                   no key needed
  apple               -                   no key needed
  ```

  `missing` means neither the environment variable nor the keychain has the key: run `pampas key set <KEY>`. The list is shortened here; it has one row per config entry.

On a machine with no Secret Service (a headless Linux server, a container, CI), the keychain is unavailable: use environment variables there. A missing keychain only matters for providers whose environment variable is unset.

## Settings

Some values are not secrets but still differ per user: a Cloudflare account id, the address of a remote Ollama server. The example config refers to them as `${NAME}` placeholders in `base_url`, so the config itself holds nothing personal and can be shared. Each placeholder is filled from, in order:

1. The environment variable `NAME` (an empty value counts as unset).
2. The stored setting `NAME`, in `settings.json` in the same directory as the providers config (`~/.config/pampas/settings.json` by default).

A provider whose placeholder is set in neither place is skipped with a message naming it, like a missing key.

- `pampas setting set <NAME> [VALUE]`: store a value, e.g. `pampas setting set CLOUDFLARE_ACCOUNT_ID 0123456789abcdef0123456789abcdef`. Leave the value out and pampas asks for it at a prompt (`CLOUDFLARE_ACCOUNT_ID: `); the input is shown, since settings are not secrets. With no value and no terminal (piped input) it is an error. It refuses names ending in `_KEY`, `_TOKEN`, `_SECRET` or `_PASSWORD`: the file is plain text, so keep secrets in the keychain with `pampas key set`.
- `pampas setting delete <NAME>`: remove a stored value.
- `pampas setting list [--config <path>]`: a table of every placeholder the config uses plus anything stored, with its value and where it comes from.

```
$ pampas setting list
NAME                   VALUE                             SOURCE
CLOUDFLARE_ACCOUNT_ID  0123456789abcdef0123456789abcdef  settings file
OLLAMA_REMOTE_HOST     -                                 not set
```

Values are shown in full because they are not secret. `setting list` and the other `setting` commands use the `settings.json` beside the `--config` file you pass, so a scratch config gets its own settings. The same output rules as the other tables apply: aligned on a terminal, tab-separated when piped.

## Environment variables

| Variable | Used for | Description |
|---|---|---|
| `GROQ_API_KEY` | Groq | API key for the `groq` entries. |
| `OPENROUTER_API_KEY` | OpenRouter | API key for the `openrouter` entry. |
| `FREEAI_API_KEY` | Free.ai | API key for the `freeai` entry. |
| `NVIDIA_API_KEY` | NVIDIA | API key for the `nvidia-*` entries. |
| `CLOUDFLARE_API_KEY` | Cloudflare Workers AI | API token for the `cloudflare-*` entries. |
| `COHERE_API_KEY` | Cohere | Trial API key for the `cohere*` entries. |
| `GEMINI_API_KEY` | Google Gemini | API key for the `gemini*` entries. |
| `CLOUDFLARE_ACCOUNT_ID` | Cloudflare Workers AI | Your account id (not secret), used in the `base_url` of the `cloudflare-*` entries. |
| `OLLAMA_REMOTE_HOST` | Ollama | Address of the remote server, used in the `base_url` of `ollama-remote`. |
| `XDG_CONFIG_HOME` | pampas | Base directory of the default providers config, `$XDG_CONFIG_HOME/pampas/providers.json`; default `~/.config`. |
| `XDG_STATE_HOME` | pampas | Base directory of the default usage file, `$XDG_STATE_HOME/pampas/usage.json`; default `~/.local/state`. |
| `HOME` | pampas | Used to build the two default paths above when the `XDG_*` variables are unset. |

Variables used as `${NAME}` placeholders in a `base_url` are not secrets and are not looked up in the keychain; if the variable is unset, the stored value from `pampas setting` is used (see [Settings](#settings)). The seven `*_API_KEY` names are the `api_key_env` values in `examples/providers.json`; the name is whatever your config says, so a provider you add yourself can use any variable. Each is looked up in the environment first, then the OS keychain (see [API keys](#api-keys)). Ollama and Apple Intelligence entries need none. `OLLAMA_HOST` is read by the Ollama server, not by pampas.

## Attribution

<a href="https://www.flaticon.com/free-icons/tiger" title="tiger icons">Tiger icons created by Magnific - Flaticon</a>
