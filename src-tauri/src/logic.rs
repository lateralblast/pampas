//! What the GUI's commands do, with no Tauri types so it can be tested directly. Each function takes the
//! file locations and returns plain data that serializes to JSON for the front end. Everything here
//! goes through the `pampas` library, so the app and the command line behave the same.

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use pampas::history::{self, Store as HistoryStore};
use pampas::engine::{
    BuildOptions, KeyStatus, Merge, SettingSource, build_providers, entries_from, fan_out_each, key_rows, load_config,
    load_usage, merge_answers, setting_rows, usage_rows, write_example_config,
};
use pampas::keystore::{KeyStore, Keychain};
use pampas::output::{self, Entry, Report};
use pampas::paths;
use pampas::provider::{ProviderEntry, parse_config, placeholders};
use pampas::settings::{self, Settings, settings_path};
use pampas::usage::Usage;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Where the three files are, and what kind of system this is.
#[derive(Debug, Clone, Serialize)]
pub struct Locations {
    pub os: String,
    pub config: PathBuf,
    pub settings: PathBuf,
    pub usage: PathBuf,
    /// Where past sessions are kept.
    pub history: PathBuf,
    pub config_exists: bool,
    /// The old macOS locations are in use until `pampas-cli migrate` is run.
    pub legacy: bool,
}

impl Locations {
    /// The default locations for this system.
    pub fn locate() -> anyhow::Result<Self> {
        let os = paths::Os::current();
        let exists = |p: &Path| p.exists();
        let config = paths::config_file(os, &paths::real_env, &exists)?;
        let usage = paths::usage_file(os, &paths::real_env, &exists)?;
        Ok(Self::from_paths(os_name(os), config.path, usage.path, config.legacy || usage.legacy))
    }

    pub fn from_paths(os: &str, config: PathBuf, usage: PathBuf, legacy: bool) -> Self {
        Self {
            os: os.to_string(),
            settings: settings_path(&config),
            config_exists: config.exists(),
            config,
            history: usage.with_file_name("history.json"),
            usage,
            legacy,
        }
    }
}

fn os_name(os: paths::Os) -> &'static str {
    match os {
        paths::Os::Linux => "linux",
        paths::Os::MacOs => "macos",
        paths::Os::Windows => "windows",
    }
}

fn entries(loc: &Locations) -> anyhow::Result<Vec<ProviderEntry>> {
    load_config(&loc.config)
}

/// A provider as the UI shows it: its config, whether it is on, whether its key is there, and its usage.
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub name: String,
    /// `openai` or `apple_fm`.
    pub kind: &'static str,
    pub model: Option<String>,
    /// The URL as written in the file, `${NAME}` placeholders unexpanded.
    pub base_url: Option<String>,
    pub enabled: bool,
    pub synthesizer: bool,
    pub timeout: Option<u64>,
    pub fallback: bool,
    /// The name of the key it uses, if any.
    pub key: Option<String>,
    /// `not_needed`, `env`, `keychain`, `missing` or `error`.
    pub key_status: &'static str,
    pub key_error: Option<String>,
    pub usage_group: Option<String>,
    pub used: u32,
    pub daily_limit: Option<u32>,
    /// The `${NAME}` placeholders in `base_url`.
    pub placeholders: Vec<String>,
}

/// Every provider in the config, with key status and today's usage.
pub fn list_providers(loc: &Locations, store: &dyn KeyStore) -> anyhow::Result<Vec<ProviderInfo>> {
    let entries = entries(loc)?;
    let keys = key_rows(&entries, store);
    let usage = usage_rows(&entries, &Usage::load(loc.usage.clone(), pampas::usage::today_utc()));
    Ok(entries
        .iter()
        .zip(keys)
        .zip(usage)
        .map(|((entry, key), usage)| {
            let (key_status, key_error) = match &key.status {
                KeyStatus::NotNeeded => ("not_needed", None),
                KeyStatus::Env => ("env", None),
                KeyStatus::Keychain => ("keychain", None),
                KeyStatus::Missing => ("missing", None),
                KeyStatus::Error(why) => ("error", Some(why.clone())),
            };
            let mut info = ProviderInfo {
                name: entry.name().to_string(),
                kind: "apple_fm",
                model: None,
                base_url: None,
                enabled: entry.enabled(),
                synthesizer: entry.synthesizer(),
                timeout: None,
                fallback: false,
                key: key.key,
                key_status,
                key_error,
                usage_group: usage.group,
                used: usage.used,
                daily_limit: usage.limit,
                placeholders: Vec::new(),
            };
            if let ProviderEntry::OpenAi(cfg) = entry {
                info.kind = "openai";
                info.model = Some(cfg.model.clone());
                info.base_url = Some(cfg.base_url.clone());
                info.timeout = cfg.timeout;
                info.fallback = cfg.fallback.is_some();
                info.placeholders = placeholders(&cfg.base_url);
            }
            info
        })
        .collect())
}

/// The config as a list of JSON objects, exactly as written, for the editor.
pub fn get_config(loc: &Locations) -> anyhow::Result<Vec<Value>> {
    let text = std::fs::read_to_string(&loc.config)
        .with_context(|| format!("reading {} (use \"create\" to start from the bundled example)", loc.config.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", loc.config.display()))
}

/// Check `entries` as `pampas` would on load, without writing anything. The error says what is wrong.
pub fn validate_config(entries: &[Value]) -> anyhow::Result<()> {
    let text = serde_json::to_string(entries)?;
    parse_config(&text).map(|_| ())
}

/// Validate `entries`, keep the old file as `providers.json.bak`, and replace the config atomically.
/// Nothing is written when validation fails.
pub fn save_config(loc: &Locations, entries: &[Value]) -> anyhow::Result<()> {
    validate_config(entries)?;
    let mut seen = std::collections::HashSet::new();
    for e in entries {
        let name = e.get("name").and_then(Value::as_str).unwrap_or_default();
        anyhow::ensure!(seen.insert(name.to_string()), "two providers are named {name:?}; names must be unique");
    }
    let text = serde_json::to_string_pretty(entries)? + "\n";
    let path = &loc.config;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    if path.exists() {
        let backup = path.with_extension("json.bak");
        std::fs::copy(path, &backup).with_context(|| format!("backing up to {}", backup.display()))?;
    }
    // Write beside the target and rename, so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// The bundled example entries, for "add a source" to offer.
pub fn example_config() -> anyhow::Result<Vec<Value>> {
    serde_json::from_str(pampas::engine::EXAMPLE_CONFIG).context("parsing the bundled example config")
}

/// Create the config from the bundled example; never overwrites.
pub fn create_config(loc: &Locations) -> anyhow::Result<()> {
    write_example_config(&loc.config)
}

/// A key name must look like an environment variable name.
fn check_key_name(name: &str) -> anyhow::Result<()> {
    let ok = !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    anyhow::ensure!(ok, "{name:?} is not a valid key name: use letters, digits and underscores, e.g. GROQ_API_KEY");
    Ok(())
}

pub fn set_key(store: &dyn KeyStore, name: &str, value: &str) -> anyhow::Result<()> {
    check_key_name(name)?;
    let value = value.trim();
    anyhow::ensure!(!value.is_empty(), "no key entered");
    store.set(name, value)
}

/// Returns whether a key was there to delete.
pub fn delete_key(store: &dyn KeyStore, name: &str) -> anyhow::Result<bool> {
    check_key_name(name)?;
    store.delete(name)
}

/// One `${NAME}` value for the settings view.
#[derive(Debug, Serialize)]
pub struct SettingInfo {
    pub name: String,
    pub value: Option<String>,
    /// `env`, `file` or `not_set`.
    pub source: &'static str,
}

pub fn list_settings(loc: &Locations) -> anyhow::Result<Vec<SettingInfo>> {
    let settings = Settings::load(&loc.settings)?;
    // An unreadable config still lets stored settings be listed and fixed.
    let entries = entries(loc).unwrap_or_default();
    Ok(setting_rows(&entries, &settings)
        .into_iter()
        .map(|r| SettingInfo {
            name: r.name,
            value: r.value,
            source: match r.source {
                SettingSource::Env => "env",
                SettingSource::File => "file",
                SettingSource::NotSet => "not_set",
            },
        })
        .collect())
}

pub fn set_setting(loc: &Locations, name: &str, value: &str) -> anyhow::Result<()> {
    let mut settings = Settings::load(&loc.settings)?;
    settings.set(name, value)?;
    settings.save(&loc.settings)
}

/// Returns whether the setting was stored.
pub fn delete_setting(loc: &Locations, name: &str) -> anyhow::Result<bool> {
    settings::check_name(name)?;
    let mut settings = Settings::load(&loc.settings)?;
    let found = settings.delete(name);
    if found {
        settings.save(&loc.settings)?;
    }
    Ok(found)
}

/// One row of today's request counts.
#[derive(Debug, Serialize)]
pub struct UsageInfo {
    pub provider: String,
    /// `None` for on-device providers, which are not counted.
    pub group: Option<String>,
    pub used: u32,
    pub limit: Option<u32>,
}

pub fn usage(loc: &Locations) -> anyhow::Result<Vec<UsageInfo>> {
    let usage = Usage::load(loc.usage.clone(), pampas::usage::today_utc());
    Ok(usage_rows(&entries(loc)?, &usage)
        .into_iter()
        .map(|r| UsageInfo { provider: r.provider, group: r.group, used: r.used, limit: r.limit })
        .collect())
}

/// What a run is asked to do, from the front end.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AskRequest {
    pub prompt: String,
    /// Providers to turn on that their entry has switched off.
    pub enable: Vec<String>,
    /// Providers to leave out.
    pub disable: Vec<String>,
    pub merge: bool,
    pub synthesizer: Option<String>,
    /// Seconds for every provider, overriding the entries' own.
    pub timeout: Option<u64>,
    pub ignore_limits: bool,
}

/// One provider's answer or failure.
#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub provider: String,
    pub ok: bool,
    /// The answer when `ok`, else the reason it failed.
    pub text: String,
}

impl Answer {
    fn from_reply(provider: &str, outcome: &anyhow::Result<String>) -> Self {
        match outcome {
            Ok(text) => Answer { provider: provider.to_string(), ok: true, text: text.clone() },
            Err(e) => Answer { provider: provider.to_string(), ok: false, text: format!("{e:#}") },
        }
    }
}

/// Progress of a run, in the order it happens.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AskEvent {
    /// The providers about to be asked (so the UI can show them waiting), and any left out and why.
    Started { providers: Vec<String>, warnings: Vec<String> },
    /// One provider finished, in the order they finish.
    Answer(Answer),
}

#[derive(Debug, Serialize)]
pub struct Merged {
    pub provider: String,
    pub text: String,
}

/// The whole result of a run.
#[derive(Debug, Serialize)]
pub struct AskResponse {
    /// In provider order.
    pub answers: Vec<Answer>,
    pub merged: Option<Merged>,
    pub merge_error: Option<String>,
    /// Providers left out, and merge notices.
    pub warnings: Vec<String>,
}

/// Ask the providers. `on_event` hears of the start and of each answer as it arrives.
pub async fn ask(
    loc: &Locations,
    store: &dyn KeyStore,
    request: AskRequest,
    on_event: &(dyn Fn(AskEvent) + Sync),
) -> anyhow::Result<AskResponse> {
    anyhow::ensure!(!request.prompt.trim().is_empty(), "the prompt is empty");
    let settings = Settings::load(&loc.settings)?;
    let usage = load_usage(loc.usage.clone());
    let client = reqwest::Client::new();
    let options = BuildOptions { enable: &request.enable, disable: &request.disable, ignore_limits: request.ignore_limits };
    let built = build_providers(entries(loc)?, &settings, store, &usage, &client, &options)?;
    anyhow::ensure!(
        !built.providers.is_empty(),
        "no usable providers{}",
        if built.warnings.is_empty() { String::new() } else { format!(": {}", built.warnings.join("; ")) }
    );
    on_event(AskEvent::Started {
        providers: built.providers.iter().map(|p| p.name().to_string()).collect(),
        warnings: built.warnings.clone(),
    });

    let replies = fan_out_each(&built.providers, &request.prompt, request.timeout, &|r| {
        on_event(AskEvent::Answer(Answer::from_reply(&r.provider, &r.result)));
    })
    .await;
    let answers: Vec<Answer> = replies.iter().map(|r| Answer::from_reply(&r.provider, &r.result)).collect();
    let entries: Vec<Entry> = entries_from(replies);

    let mut report = Report {
        prompt: request.prompt,
        entries,
        merged: None,
        merge_error: None,
        show_answers: false,
    };
    let mut warnings = built.warnings;
    if request.merge {
        let choice = Merge {
            designated: &built.designated,
            requested: request.synthesizer.as_deref(),
            timeout: request.timeout,
        };
        warnings.extend(merge_answers(&mut report, &built.providers, choice).await.map_err(|e| anyhow!("{e:#}"))?);
    }
    Ok(AskResponse {
        answers,
        merged: report.merged.map(|(provider, text)| Merged { provider, text }),
        merge_error: report.merge_error,
        warnings,
    })
}

/// One source's answer in an edition being exported.
#[derive(Debug, Deserialize)]
pub struct ExportAnswer {
    pub provider: String,
    pub ok: bool,
    /// The answer when `ok`, else the reason it failed.
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportMerged {
    pub provider: String,
    pub text: String,
}

/// An edition as the interface holds it, to be copied or saved.
#[derive(Debug, Deserialize)]
pub struct ExportReport {
    pub prompt: String,
    pub answers: Vec<ExportAnswer>,
    pub merged: Option<ExportMerged>,
    pub merge_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Markdown,
    Text,
    Json,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Text => "txt",
            Self::Json => "json",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Text => "Plain text",
            Self::Json => "JSON",
        }
    }
}

/// The edition in one of the command line's formats (`--format`), with the merged answer first and the
/// separate answers after it. Markdown and text also start with the question, which the command line
/// leaves out; JSON already carries it.
pub fn render_export(report: ExportReport, format: ExportFormat) -> String {
    let entries = report
        .answers
        .into_iter()
        .map(|a| Entry { provider: a.provider, outcome: if a.ok { Ok(a.text) } else { Err(a.text) } })
        .collect();
    let question = report.prompt.trim().to_string();
    let rendered = output::render(
        &Report {
            prompt: report.prompt,
            entries,
            merged: report.merged.map(|m| (m.provider, m.text)),
            merge_error: report.merge_error,
            show_answers: true,
        },
        match format {
            ExportFormat::Markdown => output::Format::Markdown,
            ExportFormat::Text => output::Format::Text,
            ExportFormat::Json => output::Format::Json,
        },
    );
    match format {
        ExportFormat::Markdown => format!("# {question}\n\n{rendered}"),
        ExportFormat::Text => format!("{question}\n\n{rendered}"),
        ExportFormat::Json => rendered,
    }
}

/// A file name to suggest when saving: `pampas-` and the first words of the question, plain ASCII.
pub fn export_file_name(prompt: &str, format: ExportFormat) -> String {
    let mut slug = String::new();
    for word in prompt.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
        let next = if slug.is_empty() { word.to_ascii_lowercase() } else { format!("{slug}-{}", word.to_ascii_lowercase()) };
        if next.len() > 40 {
            break;
        }
        slug = next;
    }
    if slug.is_empty() {
        format!("pampas.{}", format.extension())
    } else {
        format!("pampas-{slug}.{}", format.extension())
    }
}

fn history_store(loc: &Locations) -> HistoryStore {
    HistoryStore::new(&loc.history)
}

/// The past sessions, newest first, one line each.
pub fn list_history(loc: &Locations) -> anyhow::Result<Vec<history::Summary>> {
    history_store(loc).summaries()
}

/// One past session in full, as the interface saved it.
pub fn get_history(loc: &Locations, id: &str) -> anyhow::Result<Value> {
    history_store(loc).get(id)
}

/// Keep a finished session (replacing one with the same id).
pub fn save_history(loc: &Locations, session: Value) -> anyhow::Result<()> {
    history_store(loc).save(session)
}

/// Returns whether the session was there.
pub fn delete_history(loc: &Locations, id: &str) -> anyhow::Result<bool> {
    history_store(loc).delete(id)
}

/// Forget every session; returns how many there were.
pub fn clear_history(loc: &Locations) -> anyhow::Result<usize> {
    history_store(loc).clear()
}

/// Bring in sessions kept elsewhere (the interface's old local storage); returns how many were new.
pub fn import_history(loc: &Locations, sessions: Vec<Value>) -> anyhow::Result<usize> {
    history_store(loc).import(sessions)
}

/// The real OS keychain.
pub fn keychain() -> Keychain {
    Keychain
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A key store held in memory, so tests never touch the real keychain.
    #[derive(Default)]
    struct Memory(Mutex<std::collections::HashMap<String, String>>);
    impl KeyStore for Memory {
        fn get(&self, name: &str) -> anyhow::Result<Option<String>> {
            Ok(self.0.lock().unwrap().get(name).cloned())
        }
        fn set(&self, name: &str, key: &str) -> anyhow::Result<()> {
            self.0.lock().unwrap().insert(name.into(), key.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> anyhow::Result<bool> {
            Ok(self.0.lock().unwrap().remove(name).is_some())
        }
    }

    fn scratch(tag: &str) -> (PathBuf, Locations) {
        let dir = std::env::temp_dir().join(format!("pampas-gui-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let loc = Locations::from_paths("linux", dir.join("providers.json"), dir.join("usage.json"), false);
        (dir, loc)
    }

    #[test]
    fn a_new_config_can_be_created_listed_edited_and_saved_without_losing_field_order() {
        let (dir, loc) = scratch("config");
        assert!(!loc.config_exists);
        assert!(get_config(&loc).is_err());
        create_config(&loc).unwrap();
        assert!(create_config(&loc).is_err(), "never overwrites");

        let store = Memory::default();
        let list = list_providers(&loc, &store).unwrap();
        assert!(list.iter().any(|p| p.name == "groq" && p.key_status == "missing" && p.key.as_deref() == Some("GROQ_API_KEY")));
        assert!(list.iter().any(|p| p.name == "ollama" && p.key_status == "not_needed" && !p.enabled));
        let cf = list.iter().find(|p| p.name == "cloudflare-llama").unwrap();
        assert_eq!(cf.placeholders, vec!["CLOUDFLARE_ACCOUNT_ID"]);
        assert!(cf.base_url.as_deref().unwrap().contains("${CLOUDFLARE_ACCOUNT_ID}"), "placeholders stay unexpanded");

        // Edit one field and save: the order of fields and entries is kept, a backup is made.
        let mut config = get_config(&loc).unwrap();
        let before: Vec<String> = config[0].as_object().unwrap().keys().cloned().collect();
        config[0]["model"] = Value::String("some/other-model".into());
        save_config(&loc, &config).unwrap();
        let again = get_config(&loc).unwrap();
        assert_eq!(again[0]["model"], "some/other-model");
        assert_eq!(again[0].as_object().unwrap().keys().cloned().collect::<Vec<_>>(), before);
        assert_eq!(again.len(), config.len());
        assert!(dir.join("providers.json.bak").exists());
        assert!(!dir.join("providers.json.tmp").exists());
        assert!(std::fs::read_to_string(&loc.config).unwrap().ends_with("}\n]\n"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn history_is_saved_listed_opened_and_forgotten_through_the_locations() {
        let (dir, loc) = scratch("history");
        assert_eq!(loc.history, dir.join("history.json"), "beside the usage file");
        assert!(list_history(&loc).unwrap().is_empty());
        let session = |id: &str, at: i64| serde_json::json!({"id": id, "prompt": format!("question {id}"), "askedAt": at, "asked": ["a"], "answers": {"a": {"ok": true, "text": "x", "ms": 5}}});
        save_history(&loc, session("older", 1)).unwrap();
        save_history(&loc, session("newer", 2)).unwrap();
        let list = list_history(&loc).unwrap();
        assert_eq!(list.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["newer", "older"]);
        assert_eq!((list[0].asked, list[0].answered), (1, 1));
        assert_eq!(get_history(&loc, "older").unwrap()["prompt"], "question older");
        assert!(delete_history(&loc, "older").unwrap());
        assert_eq!(import_history(&loc, vec![session("newer", 2), session("old2", 0)]).unwrap(), 1);
        assert_eq!(clear_history(&loc).unwrap(), 2);
        assert!(save_history(&loc, serde_json::json!({"prompt": "no id"})).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_example_entries_are_offered_and_valid() {
        let entries = example_config().unwrap();
        assert!(entries.len() > 10 && entries[0]["name"] == "groq");
        validate_config(&entries).unwrap();
    }

    fn export_report() -> ExportReport {
        ExportReport {
            prompt: "  Is a tomato a fruit?  ".into(),
            answers: vec![
                ExportAnswer { provider: "a".into(), ok: true, text: "yes".into() },
                ExportAnswer { provider: "b".into(), ok: false, text: "HTTP 429".into() },
            ],
            merged: Some(ExportMerged { provider: "a".into(), text: "botanically yes".into() }),
            merge_error: None,
        }
    }

    #[test]
    fn exports_match_the_command_line_formats_with_the_question_added() {
        let md = render_export(export_report(), ExportFormat::Markdown);
        assert_eq!(md, "# Is a tomato a fruit?\n\n## merged (by a)\nbotanically yes\n\n## a\nyes\n\n");
        let text = render_export(export_report(), ExportFormat::Text);
        assert_eq!(text, "Is a tomato a fruit?\n\nmerged (by a):\nbotanically yes\n\na:\nyes\n\n");
        let json: Value = serde_json::from_str(&render_export(export_report(), ExportFormat::Json)).unwrap();
        assert_eq!(json["prompt"], "  Is a tomato a fruit?  ");
        assert_eq!(json["answers"][1], serde_json::json!({"provider": "b", "ok": false, "error": "HTTP 429"}));
        assert_eq!(json["merged"], serde_json::json!({"provider": "a", "answer": "botanically yes"}));
    }

    #[test]
    fn suggested_file_names_are_short_plain_and_end_in_the_format() {
        assert_eq!(export_file_name("Is a tomato a fruit?", ExportFormat::Markdown), "pampas-is-a-tomato-a-fruit.md");
        assert_eq!(export_file_name("", ExportFormat::Json), "pampas.json");
        assert_eq!(export_file_name("???", ExportFormat::Text), "pampas.txt");
        assert_eq!(export_file_name("../../etc/passwd", ExportFormat::Text), "pampas-etc-passwd.txt", "no path characters survive");
        let long = export_file_name(&"word ".repeat(30), ExportFormat::Markdown);
        assert!(long.len() <= "pampas-".len() + 40 + ".md".len(), "{long}");
    }

    #[test]
    fn an_invalid_config_is_refused_and_nothing_is_written() {
        let (dir, loc) = scratch("invalid");
        create_config(&loc).unwrap();
        let original = std::fs::read_to_string(&loc.config).unwrap();
        let mut config = get_config(&loc).unwrap();
        config[0]["kind"] = Value::String("telepathy".into());
        assert!(save_config(&loc, &config).is_err());
        let mut config = get_config(&loc).unwrap();
        config[1]["name"] = config[0]["name"].clone();
        assert!(save_config(&loc, &config).unwrap_err().to_string().contains("unique"));
        assert!(validate_config(&[serde_json::json!({"name": "x"})]).is_err(), "missing base_url and model");
        assert_eq!(std::fs::read_to_string(&loc.config).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn keys_and_settings_are_stored_and_checked() {
        let (dir, loc) = scratch("keys");
        let store = Memory::default();
        set_key(&store, "GROQ_API_KEY", "  secret \n").unwrap();
        assert_eq!(store.get("GROQ_API_KEY").unwrap().as_deref(), Some("secret"));
        assert!(set_key(&store, "GROQ_API_KEY", "   ").is_err());
        assert!(set_key(&store, "bad name", "x").is_err());
        assert!(delete_key(&store, "GROQ_API_KEY").unwrap());
        assert!(!delete_key(&store, "GROQ_API_KEY").unwrap());

        create_config(&loc).unwrap();
        set_setting(&loc, "PAMPAS_GUI_TEST_ACCT", "abc123").unwrap();
        assert!(set_setting(&loc, "GROQ_API_KEY", "x").is_err(), "secret-looking names are refused");
        let rows = list_settings(&loc).unwrap();
        let mine = rows.iter().find(|r| r.name == "PAMPAS_GUI_TEST_ACCT").unwrap();
        assert_eq!((mine.value.as_deref(), mine.source), (Some("abc123"), "file"));
        assert!(rows.iter().any(|r| r.name == "CLOUDFLARE_ACCOUNT_ID" && r.source != "file"));
        assert!(delete_setting(&loc, "PAMPAS_GUI_TEST_ACCT").unwrap());
        assert!(!delete_setting(&loc, "PAMPAS_GUI_TEST_ACCT").unwrap());
        assert_eq!(usage(&loc).unwrap().iter().find(|u| u.provider == "openrouter").unwrap().limit, Some(50));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn ask_reports_each_answer_as_it_arrives() {
        // A tiny server standing in for an OpenAI-compatible endpoint.
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { break };
                std::thread::spawn(move || {
                    let mut buf = [0u8; 8192];
                    let _ = s.read(&mut buf);
                    let body = r#"{"choices":[{"message":{"role":"assistant","content":"pong"}}]}"#;
                    let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                });
            }
        });
        let (dir, loc) = scratch("ask");
        std::fs::write(
            &loc.config,
            format!(
                r#"[{{"name":"one","base_url":"http://127.0.0.1:{port}/v1","model":"m"}},
                    {{"name":"two","base_url":"http://127.0.0.1:{port}/v1","model":"m"}},
                    {{"name":"needs-key","base_url":"http://127.0.0.1:{port}/v1","model":"m","api_key_env":"PAMPAS_GUI_TEST_NO_SUCH_KEY"}}]"#
            ),
        )
        .unwrap();
        let events = Mutex::new(Vec::new());
        let request = AskRequest { prompt: "ping".into(), merge: true, ..Default::default() };
        let resp = ask(&loc, &Memory::default(), request, &|e| events.lock().unwrap().push(e)).await.unwrap();
        let events = events.into_inner().unwrap();
        assert!(matches!(&events[0], AskEvent::Started { providers, warnings } if providers.len() == 2 && warnings.len() == 1));
        assert_eq!(events.iter().filter(|e| matches!(e, AskEvent::Answer(a) if a.ok && a.text == "pong")).count(), 2);
        assert_eq!(resp.answers.len(), 2);
        // The merge ran with one of the providers as synthesizer and got the same canned reply.
        assert_eq!(resp.merged.as_ref().map(|m| m.text.as_str()), Some("pong"));
        assert!(resp.warnings.iter().any(|w| w.contains("needs-key")));

        let empty = AskRequest { prompt: "  ".into(), ..Default::default() };
        assert!(ask(&loc, &Memory::default(), empty, &|_| {}).await.is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
