use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, anyhow};
use serde::Deserialize;
use serde_json::json;

use crate::usage::{LimitReached, SharedUsage};

/// A free AI service that can answer a prompt.
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn ask<'a>(&'a self, prompt: &'a str) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>>;
    /// This provider's own timeout in seconds, if its config sets one; `--timeout` overrides it.
    fn timeout(&self) -> Option<u64> {
        None
    }
}

/// One entry in the providers config file.
#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    /// Base URL of an OpenAI-compatible API, e.g. `https://api.groq.com/openai/v1`.
    pub base_url: String,
    pub model: String,
    /// Name of the environment variable holding the API key; also the name its keychain entry is stored under.
    /// Leave it out for a service that needs no key, such as a local or LAN Ollama server.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Path appended to `base_url` for chat requests; defaults to `/chat/completions`.
    #[serde(default = "default_chat_path")]
    pub chat_path: String,
    /// What to do when `model` fails; unset means report the failure.
    #[serde(default)]
    pub fallback: Option<Fallback>,
    /// Set to false to leave the provider out of runs unless `--enable <name>` is given.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// With `--merge`, prefer this provider to write the consolidated answer.
    #[serde(default)]
    pub synthesizer: bool,
    /// Most requests to make per UTC day; the provider is skipped once it is used up.
    #[serde(default)]
    pub daily_limit: Option<u32>,
    /// Seconds to wait for this provider, instead of the default 30; the `--timeout` flag overrides it.
    /// Useful for large local models, which are slow to load and to answer.
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Name under which requests are counted, default the provider's own name. Entries that share a
    /// service account (and so its quota) can share a group.
    #[serde(default)]
    pub usage_group: Option<String>,
}

fn default_true() -> bool {
    true
}

/// A recovery strategy for a provider whose configured model fails.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fallback {
    /// List the service's `/models`, then try the free text models in listed order.
    FreeModels,
}

/// How many free models a fallback tries before giving up, to spare free-tier request quotas.
const FALLBACK_ATTEMPTS: usize = 3;

fn default_chat_path() -> String {
    "/chat/completions".to_string()
}

/// Replace each `${NAME}` in `text` with `lookup(NAME)`. Errors name the first variable that is unset
/// or empty, or say the `${` is not closed.
pub fn expand_vars(text: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find('}').ok_or_else(|| format!("unterminated `${{` in {text:?}"))?;
        let name = &after[..end];
        match lookup(name) {
            Some(value) if !value.is_empty() => out.push_str(&value),
            _ => return Err(name.to_string()),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The names of the `${NAME}` placeholders in `text`, in order.
pub fn placeholders(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else { break };
        names.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    names
}

impl ProviderConfig {
    /// Fill in `${NAME}` placeholders in `base_url`. The error says which variable to set.
    pub fn expand_vars(&mut self, lookup: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<()> {
        self.base_url = expand_vars(&self.base_url, lookup)
            .map_err(|name| anyhow!("{}: base_url uses ${{{name}}}, which is not set: set the environment variable {name} or run `pampas-cli setting set {name} <value>`", self.name))?;
        Ok(())
    }

    /// The name this provider's requests are counted under.
    pub fn usage_group(&self) -> &str {
        self.usage_group.as_deref().unwrap_or(&self.name)
    }

    /// URL of the service's model listing.
    fn models_url(&self) -> String {
        format!("{}/models", self.base_url.trim_end_matches('/'))
    }

    /// Full URL chat requests are posted to.
    fn chat_url(&self) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), self.chat_path.trim_start_matches('/'))
    }
}

/// Any service that speaks the OpenAI `/chat/completions` API.
pub struct OpenAiCompatible {
    config: ProviderConfig,
    api_key: Option<String>,
    client: reqwest::Client,
    usage: SharedUsage,
    /// Refuse requests once `daily_limit` is used up (false with `--ignore-limits`).
    enforce_limits: bool,
}

impl OpenAiCompatible {
    pub fn new(
        config: ProviderConfig,
        api_key: Option<String>,
        client: reqwest::Client,
        usage: SharedUsage,
        enforce_limits: bool,
    ) -> Self {
        Self { config, api_key, client, usage, enforce_limits }
    }

    /// Add the bearer token, if this provider has a key.
    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.bearer_auth(key),
            None => req,
        }
    }

    /// Ask one specific model. The request is counted before it is sent, so a failed one still
    /// counts against the daily limit.
    async fn ask_model(&self, model: &str, prompt: &str) -> anyhow::Result<String> {
        let (group, limit) = (self.config.usage_group(), self.config.daily_limit);
        let used = self.usage.lock().expect("usage lock").acquire(group, limit, self.enforce_limits)?;
        if let Some(limit) = limit
            && used * 5 >= limit * 4
        {
            eprintln!("{}: {used} of {limit} daily requests used", self.config.name);
        }
        let body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
        });
        let resp = self.authed(self.client.post(self.config.chat_url())).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("HTTP {status}: {}", text.chars().take(300).collect::<String>()));
        }
        parse_reply(&text)
    }

    /// Free text models the service lists, other than `exclude`.
    async fn free_models(&self, exclude: &str) -> anyhow::Result<Vec<String>> {
        let resp = self.authed(self.client.get(self.config.models_url())).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("listing models: HTTP {status}"));
        }
        parse_free_models(&text, exclude)
    }
}

impl Provider for OpenAiCompatible {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn timeout(&self) -> Option<u64> {
        self.config.timeout
    }

    fn ask<'a>(&'a self, prompt: &'a str) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let first = self.ask_model(&self.config.model, prompt).await;
            let err = match (first, self.config.fallback) {
                (Ok(text), _) => return Ok(text),
                (Err(e), None) => return Err(e),
                (Err(e), Some(Fallback::FreeModels)) => e,
            };
            if err.downcast_ref::<LimitReached>().is_some() {
                return Err(err);
            }
            let candidates = match self.free_models(&self.config.model).await {
                Ok(c) => c,
                Err(e) => return Err(err.context(format!("fallback unavailable ({e:#})"))),
            };
            let mut tried = Vec::new();
            for model in candidates.into_iter().take(FALLBACK_ATTEMPTS) {
                match self.ask_model(&model, prompt).await {
                    Ok(text) => {
                        eprintln!("{}: {} failed ({err:#}); answered by free model {model}", self.config.name, self.config.model);
                        return Ok(text);
                    }
                    Err(e) => tried.push(format!("{model}: {e:#}")),
                }
            }
            Err(err.context(format!("fallback failed too ({})", tried.join("; "))))
        })
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: Message,
}
#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

/// Extract the assistant text from a `/chat/completions` response body.
pub fn parse_reply(body: &str) -> anyhow::Result<String> {
    let parsed: ChatResponse = serde_json::from_str(body).context("unexpected response shape")?;
    parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| anyhow!("response had no message content"))
}

/// Ids of the free text models in a `/models` response, in listed order, minus `exclude`.
///
/// Free means both `pricing.prompt` and `pricing.completion` are `"0"` and the id ends in `:free`.
/// Models that do not output text, and safety classifiers, are skipped: they cannot answer a prompt.
pub fn parse_free_models(body: &str, exclude: &str) -> anyhow::Result<Vec<String>> {
    let parsed: serde_json::Value = serde_json::from_str(body).context("unexpected models response shape")?;
    let models = parsed["data"].as_array().context("models response had no `data` list")?;
    let is_zero = |v: &serde_json::Value| v.as_str() == Some("0");
    Ok(models
        .iter()
        .filter(|m| is_zero(&m["pricing"]["prompt"]) && is_zero(&m["pricing"]["completion"]))
        .filter(|m| {
            m["architecture"]["output_modalities"]
                .as_array()
                .is_none_or(|out| out.iter().any(|o| o.as_str() == Some("text")))
        })
        .filter_map(|m| m["id"].as_str())
        .filter(|id| id.ends_with(":free") && *id != exclude)
        .filter(|id| !id.contains("safety") && !id.contains("guard"))
        .map(str::to_string)
        .collect())
}

/// One entry of the providers config, by its `kind`.
#[derive(Debug)]
pub enum ProviderEntry {
    OpenAi(ProviderConfig),
    AppleFm(AppleFmConfig),
}

impl ProviderEntry {
    pub fn name(&self) -> &str {
        match self {
            Self::OpenAi(c) => &c.name,
            Self::AppleFm(c) => &c.name,
        }
    }

    pub fn synthesizer(&self) -> bool {
        match self {
            Self::OpenAi(c) => c.synthesizer,
            Self::AppleFm(c) => c.synthesizer,
        }
    }

    /// Whether the entry runs without `--enable`.
    pub fn enabled(&self) -> bool {
        match self {
            Self::OpenAi(c) => c.enabled,
            Self::AppleFm(c) => c.enabled,
        }
    }
}

/// The entries to run: those with `enabled` true, plus `enable`d names, minus `disable`d names.
///
/// A name that matches no entry, or is both enabled and disabled, is an error, so a typo cannot
/// silently leave a provider on or off.
pub fn select_enabled(
    entries: Vec<ProviderEntry>,
    enable: &[String],
    disable: &[String],
) -> anyhow::Result<Vec<ProviderEntry>> {
    for name in enable.iter().chain(disable) {
        if !entries.iter().any(|e| e.name() == name) {
            let known: Vec<&str> = entries.iter().map(|e| e.name()).collect();
            return Err(anyhow!("no provider named `{name}` (configured: {})", known.join(", ")));
        }
    }
    if let Some(name) = enable.iter().find(|n| disable.contains(n)) {
        return Err(anyhow!("`{name}` is both enabled and disabled"));
    }
    Ok(entries
        .into_iter()
        .filter(|e| !disable.iter().any(|n| n == e.name()) && (e.enabled() || enable.iter().any(|n| n == e.name())))
        .collect())
}

/// A config entry for Apple's on-device model, run through the `helpers/apple-fm.swift` helper.
#[derive(Debug, Deserialize)]
pub struct AppleFmConfig {
    pub name: String,
    /// The helper executable, looked up on `PATH` unless it contains a slash.
    #[serde(default = "default_apple_command")]
    pub command: String,
    /// Set to false to leave the provider out of runs unless `--enable <name>` is given.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// With `--merge`, prefer this provider to write the consolidated answer.
    #[serde(default)]
    pub synthesizer: bool,
}

fn default_apple_command() -> String {
    "pampas-apple-fm".to_string()
}

/// Apple's on-device model, via a helper that reads the prompt on stdin and prints the answer.
pub struct AppleFm {
    config: AppleFmConfig,
}

impl AppleFm {
    pub fn new(config: AppleFmConfig) -> Self {
        Self { config }
    }

    /// Ok if the helper runs and reports the model available, else why not.
    pub fn check(config: &AppleFmConfig) -> anyhow::Result<()> {
        let out = std::process::Command::new(&config.command)
            .arg("--check")
            .output()
            .with_context(|| format!("running `{}` (build it, see the README)", config.command))?;
        if out.status.success() {
            return Ok(());
        }
        Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

impl Provider for AppleFm {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn ask<'a>(&'a self, prompt: &'a str) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            use std::process::Stdio;
            use tokio::io::AsyncWriteExt;

            // kill_on_drop: the per-provider timeout drops this future, which must not leave the helper running.
            let mut child = tokio::process::Command::new(&self.config.command)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .with_context(|| format!("running `{}` (build it, see the README)", self.config.command))?;
            let mut stdin = child.stdin.take().context("no stdin")?;
            stdin.write_all(prompt.as_bytes()).await?;
            drop(stdin);
            let out = child.wait_with_output().await?;
            if !out.status.success() {
                return Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr).trim()));
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        })
    }
}

/// Parse the providers config file (a JSON array of entries). An entry's `"kind"` is `"openai"`
/// (the default, a [`ProviderConfig`]) or `"apple_fm"` (an [`AppleFmConfig`]).
pub fn parse_config(text: &str) -> anyhow::Result<Vec<ProviderEntry>> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(text).context("invalid providers config")?;
    entries
        .into_iter()
        .map(|v| {
            let kind = v["kind"].as_str().unwrap_or("openai").to_string();
            match kind.as_str() {
                "openai" => serde_json::from_value(v).map(ProviderEntry::OpenAi),
                "apple_fm" => serde_json::from_value(v).map(ProviderEntry::AppleFm),
                other => return Err(anyhow!("unknown provider kind `{other}` (expected `openai` or `apple_fm`)")),
            }
            .context("invalid providers config")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn openai(entry: &ProviderEntry) -> &ProviderConfig {
        match entry {
            ProviderEntry::OpenAi(c) => c,
            other => panic!("not an openai entry: {other:?}"),
        }
    }

    #[test]
    fn parses_reply() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#;
        assert_eq!(parse_reply(body).unwrap(), "hi");
    }

    #[test]
    fn empty_choices_is_an_error() {
        assert!(parse_reply(r#"{"choices":[]}"#).is_err());
    }

    #[test]
    fn timeout_is_optional() {
        let cfg = r#"[{"name":"a","base_url":"u","model":"m"},{"name":"b","base_url":"u","model":"m","timeout":180}]"#;
        let parsed = parse_config(cfg).unwrap();
        let timeouts: Vec<Option<u64>> =
            parsed.iter().map(|e| if let ProviderEntry::OpenAi(c) = e { c.timeout } else { None }).collect();
        assert_eq!(timeouts, vec![None, Some(180)]);
    }

    #[test]
    fn lists_placeholders() {
        assert_eq!(placeholders("https://x/${A}/y/${B}"), vec!["A", "B"]);
        assert!(placeholders("https://x/y").is_empty());
    }

    #[test]
    fn expands_placeholders_and_names_the_missing_one() {
        let lookup = |name: &str| match name {
            "ACCT" => Some("abc123".to_string()),
            "EMPTY" => Some(String::new()),
            _ => None,
        };
        assert_eq!(expand_vars("https://x/${ACCT}/ai/${ACCT}", &lookup).unwrap(), "https://x/abc123/ai/abc123");
        assert_eq!(expand_vars("no placeholders", &lookup).unwrap(), "no placeholders");
        assert_eq!(expand_vars("https://x/${NOPE}", &lookup).unwrap_err(), "NOPE");
        assert_eq!(expand_vars("https://x/${EMPTY}", &lookup).unwrap_err(), "EMPTY");
        assert!(expand_vars("https://x/${ACCT", &lookup).unwrap_err().contains("unterminated"));
    }

    #[test]
    fn api_key_env_is_optional() {
        let cfg = r#"[{"name":"ollama","base_url":"http://localhost:11434/v1","model":"m"}]"#;
        let parsed = parse_config(cfg).unwrap();
        assert_eq!(parsed[0].name(), "ollama");
    }

    #[test]
    fn parses_config() {
        let cfg = r#"[{"name":"groq","base_url":"https://api.groq.com/openai/v1","model":"m","api_key_env":"GROQ_API_KEY"}]"#;
        let parsed = parse_config(cfg).unwrap();
        assert_eq!(parsed[0].name(), "groq");
    }

    #[test]
    fn chat_path_defaults_and_can_be_overridden() {
        let cfg = r#"[
            {"name":"a","base_url":"https://x.test/v1/","model":"m","api_key_env":"K"},
            {"name":"b","base_url":"https://x.test/v1","model":"m","api_key_env":"K","chat_path":"/chat/"}
        ]"#;
        let parsed = parse_config(cfg).unwrap();
        assert_eq!(openai(&parsed[0]).chat_url(), "https://x.test/v1/chat/completions");
        assert_eq!(openai(&parsed[1]).chat_url(), "https://x.test/v1/chat/");
    }

    #[test]
    fn fallback_is_optional_and_parses() {
        let cfg = r#"[
            {"name":"a","base_url":"u","model":"m","api_key_env":"K"},
            {"name":"b","base_url":"u","model":"m","api_key_env":"K","fallback":"free_models"}
        ]"#;
        let parsed = parse_config(cfg).unwrap();
        assert_eq!(openai(&parsed[0]).fallback, None);
        assert_eq!(openai(&parsed[1]).fallback, Some(Fallback::FreeModels));
        assert!(parse_config(r#"[{"name":"a","base_url":"u","model":"m","api_key_env":"K","fallback":"bogus"}]"#).is_err());
    }

    #[test]
    fn free_models_are_filtered_and_ordered() {
        let body = r#"{"data":[
            {"id":"a/paid","pricing":{"prompt":"0.1","completion":"0.1"},"architecture":{"output_modalities":["text"]}},
            {"id":"b/image:free","pricing":{"prompt":"0","completion":"0"},"architecture":{"output_modalities":["image"]}},
            {"id":"c/guard-safety:free","pricing":{"prompt":"0","completion":"0"},"architecture":{"output_modalities":["text"]}},
            {"id":"d/current:free","pricing":{"prompt":"0","completion":"0"},"architecture":{"output_modalities":["text"]}},
            {"id":"e/first:free","pricing":{"prompt":"0","completion":"0"},"architecture":{"output_modalities":["text"]}},
            {"id":"f/router","pricing":{"prompt":"0","completion":"0"},"architecture":{"output_modalities":["text"]}},
            {"id":"g/second:free","pricing":{"prompt":"0","completion":"0"}}
        ]}"#;
        assert_eq!(parse_free_models(body, "d/current:free").unwrap(), ["e/first:free", "g/second:free"]);
    }

    #[test]
    fn apple_fm_entry_parses_and_needs_no_key() {
        let cfg = r#"[
            {"name":"apple","kind":"apple_fm"},
            {"name":"apple2","kind":"apple_fm","command":"/opt/bin/fm"}
        ]"#;
        let parsed = parse_config(cfg).unwrap();
        match (&parsed[0], &parsed[1]) {
            (ProviderEntry::AppleFm(a), ProviderEntry::AppleFm(b)) => {
                assert_eq!(a.command, "pampas-apple-fm");
                assert_eq!(b.command, "/opt/bin/fm");
            }
            other => panic!("expected apple_fm entries: {other:?}"),
        }
    }

    #[test]
    fn enable_and_disable_select_providers() {
        let cfg = r#"[
            {"name":"on","base_url":"u","model":"m","api_key_env":"K"},
            {"name":"off","base_url":"u","model":"m","api_key_env":"K","enabled":false},
            {"name":"apple","kind":"apple_fm","enabled":false}
        ]"#;
        let names = |enable: &[&str], disable: &[&str]| -> anyhow::Result<Vec<String>> {
            let own = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            let picked = select_enabled(parse_config(cfg)?, &own(enable), &own(disable))?;
            Ok(picked.iter().map(|e| e.name().to_string()).collect())
        };
        assert_eq!(names(&[], &[]).unwrap(), ["on"]);
        assert_eq!(names(&["off", "apple"], &[]).unwrap(), ["on", "off", "apple"]);
        assert_eq!(names(&[], &["on"]).unwrap(), Vec::<String>::new());
        assert!(format!("{:#}", names(&["nope"], &[]).unwrap_err()).contains("no provider named `nope`"));
        assert!(format!("{:#}", names(&["on"], &["on"]).unwrap_err()).contains("both enabled and disabled"));
    }

    #[test]
    fn limits_and_usage_groups_parse() {
        let cfg = r#"[
            {"name":"a","base_url":"u","model":"m","api_key_env":"K"},
            {"name":"b","base_url":"u","model":"m","api_key_env":"K","daily_limit":50,"usage_group":"acct"}
        ]"#;
        let parsed = parse_config(cfg).unwrap();
        assert_eq!((openai(&parsed[0]).daily_limit, openai(&parsed[0]).usage_group()), (None, "a"));
        assert_eq!((openai(&parsed[1]).daily_limit, openai(&parsed[1]).usage_group()), (Some(50), "acct"));
    }

    #[test]
    fn unknown_kind_is_an_error() {
        let err = parse_config(r#"[{"name":"x","kind":"bogus"}]"#).unwrap_err();
        assert!(format!("{err:#}").contains("unknown provider kind `bogus`"));
    }

    #[test]
    fn example_config_parses() {
        let parsed = parse_config(include_str!("../examples/providers.json")).unwrap();
        assert!(parsed.iter().any(|p| p.name() == "openrouter"));
    }
}
