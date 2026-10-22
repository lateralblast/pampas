// A stand-in for the Tauri backend, so the interface can be tried in an ordinary browser.
// Query: ?fresh=1 (no config yet), ?speed=0.2 (faster answers), ?dark is the system's choice.
(() => {
  const q = new URLSearchParams(location.search);
  const speed = Number(q.get("speed") ?? 1);
  const fresh = q.get("fresh") === "1";
  const wait = (ms) => new Promise((r) => setTimeout(r, ms * speed));

  let configExists = !fresh;
  let config = [
    { name: "groq", base_url: "https://api.groq.com/openai/v1", model: "openai/gpt-oss-20b", api_key_env: "GROQ_API_KEY", usage_group: "groq" },
    { name: "groq-120b", base_url: "https://api.groq.com/openai/v1", model: "openai/gpt-oss-120b", api_key_env: "GROQ_API_KEY", synthesizer: true, usage_group: "groq" },
    { name: "openrouter", base_url: "https://openrouter.ai/api/v1", model: "minimax/minimax-m3.1-flash:free", api_key_env: "OPENROUTER_API_KEY", fallback: "free_models", daily_limit: 50 },
    { name: "freeai", base_url: "https://api.free.ai/v1", chat_path: "/chat/", model: "qwen7b", api_key_env: "FREEAI_API_KEY", daily_limit: 30 },
    { name: "cohere", base_url: "https://api.cohere.ai/compatibility/v1", model: "command-a-plus-05-2026", api_key_env: "COHERE_API_KEY", daily_limit: 30, usage_group: "cohere" },
    { name: "cloudflare-llama", base_url: "https://api.cloudflare.com/client/v4/accounts/${CLOUDFLARE_ACCOUNT_ID}/ai/v1", model: "@cf/meta/llama-3.3-70b-instruct-fp8-fast", api_key_env: "CLOUDFLARE_API_KEY", usage_group: "cloudflare" },
    { name: "gemini", base_url: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-flash-latest", api_key_env: "GEMINI_API_KEY", usage_group: "gemini" },
    { name: "ollama", base_url: "http://localhost:11434/v1", model: "qwen2.5:7b", timeout: 180, enabled: false },
    { name: "apple", kind: "apple_fm" },
  ];
  const examples = [
    ...config.map((e) => ({ ...e })),
    { name: "nvidia-glm", base_url: "https://integrate.api.nvidia.com/v1", model: "z-ai/glm-5.3-flash", api_key_env: "NVIDIA_API_KEY", usage_group: "nvidia" },
    { name: "ollama-remote", base_url: "http://${OLLAMA_REMOTE_HOST}:11434/v1", model: "gemma4:12b", timeout: 180, enabled: false },
  ];
  const keys = q.get("keys") === "all"
    ? { GROQ_API_KEY: "keychain", COHERE_API_KEY: "keychain", GEMINI_API_KEY: "env", OPENROUTER_API_KEY: "keychain", FREEAI_API_KEY: "keychain", CLOUDFLARE_API_KEY: "keychain" }
    : { GROQ_API_KEY: "keychain", COHERE_API_KEY: "keychain", GEMINI_API_KEY: "env" };
  const settings = { OLLAMA_REMOTE_HOST: "192.168.1.20" };
  const used = { groq: 12, openrouter: 41, freeai: 30, cohere: 3, gemini: 7 };

  const group = (e) => e.usage_group ?? e.name;
  const placeholders = (url) => [...(url ?? "").matchAll(/\$\{([^}]+)\}/g)].map((m) => m[1]);
  const keyStatus = (e) => (e.kind === "apple_fm" || !e.api_key_env ? "not_needed" : keys[e.api_key_env] ?? "missing");

  const info = (e) => ({
    name: e.name,
    kind: e.kind === "apple_fm" ? "apple_fm" : "openai",
    model: e.model ?? null,
    base_url: e.base_url ?? null,
    enabled: e.enabled !== false,
    synthesizer: e.synthesizer === true,
    timeout: e.timeout ?? null,
    fallback: Boolean(e.fallback),
    key: e.api_key_env ?? null,
    key_status: keyStatus(e),
    key_error: null,
    usage_group: e.kind === "apple_fm" ? null : group(e),
    used: e.kind === "apple_fm" ? 0 : used[group(e)] ?? 0,
    daily_limit: e.daily_limit ?? null,
    placeholders: placeholders(e.base_url),
  });

  const settingRows = () => {
    const names = new Set([...config.flatMap((e) => placeholders(e.base_url)), ...Object.keys(settings)]);
    return [...names].sort().map((name) => ({ name, value: settings[name] ?? null, source: settings[name] ? "file" : "not_set" }));
  };

  const listeners = new Map();
  const emit = (name, payload) => (listeners.get(name) ?? []).forEach((cb) => cb({ payload }));

  const answers = {
    groq: "A **read-through cache** with a short TTL is the safest start. Put it in front of the database, expire entries after about 60 seconds, and measure the hit rate before tuning.",
    "groq-120b": "# Caching\n\nStart with a read-through cache.\n\n- Keep the TTL short (30 to 120 seconds).\n- Invalidate on write for data that must be fresh.\n- Watch the hit rate: below 80% the cache is not earning its keep.\n\n```python\nvalue = cache.get(key)\nif value is None:\n    value = db.read(key)\n    cache.set(key, value, ttl=60)\n```\n\nSee [the Redis docs](https://redis.io/docs/) for eviction policies.",
    gemini: "Use a cache with a TTL, plus explicit invalidation when a record changes.\n\n| Approach | Freshness | Complexity |\n|---|---|---|\n| TTL only | stale up to TTL | low |\n| Invalidate on write | fresh | medium |\n| Write-through | fresh | high |\n\nFor most services the second row is the sweet spot. " + "A longer explanation follows so that this answer is taller than the collapsed row. ".repeat(14),
  };
  const merged = "**groq** and **gemini** agree on a read-through cache with a short TTL, and **groq-120b** adds the practical numbers: 30 to 120 seconds, measured by hit rate. The sources differ on invalidation: **gemini** wants explicit invalidation on write, while **groq** relies on the short TTL alone.\n\n- Agreed: read-through cache, short TTL.\n- Disputed: whether to invalidate on write.\n- Not heard: cohere (rate limited), freeai (daily limit).";

  // The same shapes as the Rust renderer (src-tauri/src/logic.rs), so the mock page shows real output.
  const render = ({ prompt, answers, merged, merge_error }, format) => {
    if (format === "json") {
      return JSON.stringify({ prompt, answers: answers.map((a) => (a.ok ? { provider: a.provider, ok: true, answer: a.text } : { provider: a.provider, ok: false, error: a.text })), merged: merged ? { provider: merged.provider, answer: merged.text } : null, merge_error }, null, 2) + "\n";
    }
    const head = (n) => (format === "markdown" ? `## ${n}` : `${n}:`);
    let out = format === "markdown" ? `# ${prompt.trim()}\n\n` : `${prompt.trim()}\n\n`;
    if (merged) out += `${head(`merged (by ${merged.provider})`)}\n${merged.text}\n\n`;
    for (const a of answers) if (a.ok) out += `${head(a.provider)}\n${a.text}\n\n`;
    return out;
  };

  // ---- history ----
  const now = Date.now();
  const mk = (id, ago, prompt, asked, merge) => {
    const answersFor = Object.fromEntries(asked.map((n, i) => [n, n === "cohere" ? { ok: false, text: "HTTP 429 Too Many Requests", ms: 2600 } : { ok: true, text: answers[n] ?? `${n} answers: ${prompt.slice(0, 40)}…`, ms: 900 + i * 700 }]));
    return { id, prompt, askedAt: now - ago, finishedAt: now - ago + 4200, order: config.map((e) => e.name), models: Object.fromEntries(config.map((e) => [e.name, e.model ?? null])), mergeRequested: merge, phase: "done", asked, answers: answersFor, skipped: [], notices: [], merged: merge ? { provider: "groq-120b", text: merged } : null, mergeError: null, fatal: null };
  };
  const hist = q.get("history") === "0" ? [] : [
    mk("h1", 25 * 60 * 1000, "How should I cache reads in a small web service, and when should the cache be invalidated?", ["groq", "groq-120b", "cohere", "gemini"], true),
    mk("h2", 3 * 3600 * 1000, "Is a tomato a fruit or a vegetable?", ["groq", "gemini", "apple"], true),
    mk("h3", 26 * 3600 * 1000, "What is the difference between a mutex and a semaphore?", ["groq", "groq-120b", "gemini"], false),
    mk("h4", 9 * 24 * 3600 * 1000, "Summarise the plot of Middlemarch in two sentences.", ["groq-120b", "gemini", "cohere"], true),
  ];
  const summary = (s) => ({ id: s.id, asked_at: s.askedAt, prompt: s.prompt, asked: s.asked.length, answered: s.asked.filter((n) => s.answers[n]?.ok).length, merged_by: s.merged?.provider ?? null });

  const handlers = {
    list_history: () => [...hist].sort((a, b) => b.askedAt - a.askedAt).map(summary),
    get_history: ({ id }) => { const s = hist.find((x) => x.id === id); if (!s) throw "that session is no longer in the history"; return JSON.parse(JSON.stringify(s)); },
    save_history: ({ session }) => { if (!session.id || !session.prompt) throw "a session needs an id and a prompt"; const i = hist.findIndex((x) => x.id === session.id); if (i >= 0) hist[i] = session; else hist.push(session); return null; },
    delete_history: ({ id }) => { const i = hist.findIndex((x) => x.id === id); if (i >= 0) hist.splice(i, 1); return i >= 0; },
    clear_history: () => { const n = hist.length; hist.length = 0; return n; },
    import_history: ({ sessions }) => { let n = 0; for (const s of sessions) if (!hist.some((x) => x.id === s.id)) { hist.push(s); n++; } return n; },
    render_edition: ({ report, format }) => render(report, format),
    save_edition: ({ report, format }) => {
      const ext = { markdown: "md", text: "txt", json: "json" }[format];
      const link = Object.assign(document.createElement("a"), { href: URL.createObjectURL(new Blob([render(report, format)], { type: "text/plain" })), download: `pampas.${ext}` });
      link.click();
      return `~/Downloads/pampas.${ext}`;
    },
    locations: () => ({ os: "linux", config: "/home/you/.config/pampas/providers.json", settings: "/home/you/.config/pampas/settings.json", usage: "/home/you/.local/state/pampas/usage.json", history: "/home/you/.local/state/pampas/history.json", config_exists: configExists, legacy: false }),
    list_providers: () => config.map(info),
    get_config: () => JSON.parse(JSON.stringify(config)),
    example_config: () => JSON.parse(JSON.stringify(examples)),
    validate_config: ({ entries }) => {
      const names = entries.map((e) => e.name);
      if (names.length !== new Set(names).size) throw "two providers are named the same; names must be unique";
      for (const e of entries) if (e.kind !== "apple_fm" && (!e.base_url || !e.model)) throw `provider ${e.name}: base_url and model are required`;
      return null;
    },
    save_config: ({ entries }) => { handlers.validate_config({ entries }); config = JSON.parse(JSON.stringify(entries)); return null; },
    create_config: () => { configExists = true; return null; },
    set_key: ({ name, value }) => { if (!value.trim()) throw "no key entered"; keys[name] = "keychain"; return null; },
    delete_key: ({ name }) => { const had = keys[name] === "keychain"; if (had) delete keys[name]; return had; },
    list_settings: () => settingRows(),
    set_setting: ({ name, value }) => { if (/_KEY$/.test(name)) throw `${name} looks like a secret; settings are stored in plain text. Use \`pampas key set ${name}\` instead`; settings[name] = value; return null; },
    delete_setting: ({ name }) => { const had = name in settings; delete settings[name]; return had; },
    usage: () => config.map((e) => ({ provider: e.name, group: e.kind === "apple_fm" ? null : group(e), used: e.kind === "apple_fm" ? 0 : used[group(e)] ?? 0, limit: e.daily_limit ?? null })),
    ask: async ({ request }) => {
      if (!request.prompt.trim()) throw "the prompt is empty";
      const warnings = [];
      const chosen = config.filter((e) => {
        const on = (e.enabled !== false || request.enable.includes(e.name)) && !request.disable.includes(e.name);
        if (!on) return false;
        if (keyStatus(e) === "missing") { warnings.push(`skipping provider: ${e.name}: no key: set ${e.api_key_env} or run \`pampas key set ${e.api_key_env}\``); return false; }
        if (e.daily_limit && (used[group(e)] ?? 0) >= e.daily_limit && !request.ignore_limits) { warnings.push(`skipping provider: ${e.name}: daily limit of ${e.daily_limit} requests reached (${used[group(e)]} used today, UTC)`); return false; }
        return true;
      });
      if (!chosen.length) throw `no usable providers: ${warnings.join("; ")}`;
      emit("pampas-ask", { event: "started", providers: chosen.map((e) => e.name), warnings });
      const plan = { groq: [1200, "ok"], "groq-120b": [2100, "ok"], gemini: [3800, "ok"], cohere: [2600, "429"], "cloudflare-llama": [5200, "timeout"], ollama: [4200, "ok"], apple: [900, "ok"], openrouter: [1500, "ok"], freeai: [1800, "ok"] };
      const got = await Promise.all(chosen.map(async (e) => {
        const [ms, kind] = plan[e.name] ?? [1500, "ok"];
        await wait(ms);
        const a = kind === "ok"
          ? { provider: e.name, ok: true, text: answers[e.name] ?? `${e.name} answers: use a cache with a short TTL.` }
          : { provider: e.name, ok: false, text: kind === "429" ? "HTTP 429 Too Many Requests: {\"error\":\"rate limit\"}" : "timed out after 30s" };
        emit("pampas-ask", { event: "answer", ...a });
        return a;
      }));
      used.groq = (used.groq ?? 0) + 2;
      let m = null;
      let mergeError = null;
      const good = got.filter((a) => a.ok);
      if (request.merge) {
        await wait(1800);
        if (good.length >= 2) m = { provider: request.synthesizer ?? "groq-120b", text: merged };
        else warnings.push(`only ${good[0]?.provider ?? "nobody"} answered; nothing to merge`);
      }
      return { answers: got, merged: m, merge_error: mergeError, warnings };
    },
  };

  window.__TAURI__ = {
    core: { invoke: async (cmd, args) => { await wait(60); const f = handlers[cmd]; if (!f) throw `unknown command ${cmd}`; return f(args ?? {}); } },
    event: { listen: async (name, cb) => { const list = listeners.get(name) ?? []; list.push(cb); listeners.set(name, list); return () => listeners.set(name, (listeners.get(name) ?? []).filter((x) => x !== cb)); } },
  };
})();
