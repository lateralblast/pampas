//! The work behind a run, with no printing and no terminal: loading the config, building the
//! providers that can run, asking them all at once, merging the answers, and the tables of keys,
//! usage and settings. The command line and the GUI both call this; whatever is worth telling the
//! user comes back as data (warnings, rows) for the caller to show.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use futures::future::join_all;

use crate::keystore::{KeyStore, Source, resolve};
use crate::merge;
use crate::output;
use crate::provider::{
    AppleFm, OpenAiCompatible, Provider, ProviderConfig, ProviderEntry, parse_config, placeholders, select_enabled,
};
use crate::scrape::Scrape;
use crate::settings::Settings;
use crate::usage::{SharedUsage, Usage};

/// The reference config, for commands that are useful before the user has written their own.
pub const EXAMPLE_CONFIG: &str = include_str!("../examples/providers.json");

/// Read and parse a providers config.
pub fn load_config(path: &Path) -> anyhow::Result<Vec<ProviderEntry>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "{} does not exist: run `pampas-cli init` to create it from the bundled example, or pass --config",
                path.display()
            )
        }
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    parse_config(&text)
}

/// Write the bundled example config to `path`, creating its directory; never overwrite a file.
pub fn write_example_config(path: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(!path.exists(), "{} already exists; not overwriting it", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(path, EXAMPLE_CONFIG).with_context(|| format!("writing {}", path.display()))
}

/// The value of a `${NAME}` placeholder: the environment variable, else the stored setting. Empty counts as unset.
pub fn lookup_var(name: &str, settings: &Settings) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty()).or_else(|| settings.get(name))
}

/// The provider's key: its environment variable, else the keychain. `None` when the entry names no key.
pub fn find_key(cfg: &ProviderConfig, store: &dyn KeyStore) -> anyhow::Result<Option<(String, Source)>> {
    let Some(name) = &cfg.api_key_env else { return Ok(None) };
    let env = std::env::var(name).ok();
    resolve(name, env, store)
        .with_context(|| format!("{}: looking up {name}", cfg.name))?
        .with_context(|| format!("{}: no key: set {name} or run `pampas-cli key set {name}`", cfg.name))
        .map(Some)
}

/// Each distinct `api_key_env` in the config with the providers that use it, in config order.
pub fn key_types(entries: &[ProviderEntry]) -> Vec<(String, Vec<String>)> {
    let mut types: Vec<(String, Vec<String>)> = Vec::new();
    for entry in entries {
        let ProviderEntry::OpenAi(cfg) = entry else { continue };
        let Some(env) = &cfg.api_key_env else { continue };
        match types.iter_mut().find(|(name, _)| name == env) {
            Some((_, users)) => users.push(cfg.name.clone()),
            None => types.push((env.clone(), vec![cfg.name.clone()])),
        }
    }
    types
}

/// Where a provider's key stands.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyStatus {
    NotNeeded,
    Env,
    Keychain,
    Missing,
    /// The keychain itself failed; the text says why.
    Error(String),
}

/// One row of the key table: the provider, the name of its key (if it has one) and where it stands.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyRow {
    pub provider: String,
    pub key: Option<String>,
    pub status: KeyStatus,
}

/// A row for each configured provider.
pub fn key_rows(entries: &[ProviderEntry], store: &dyn KeyStore) -> Vec<KeyRow> {
    entries
        .iter()
        .map(|entry| match entry {
            ProviderEntry::OpenAi(cfg) => {
                let status = match find_key(cfg, store) {
                    Ok(None) => KeyStatus::NotNeeded,
                    Ok(Some((_, Source::Env))) => KeyStatus::Env,
                    Ok(Some((_, Source::Keychain))) => KeyStatus::Keychain,
                    // A missing key is the common case and the fix is `pampas-cli key set`; only say more
                    // when the keychain itself failed.
                    Err(e) if e.to_string().contains(": no key:") => KeyStatus::Missing,
                    Err(e) => KeyStatus::Error(e.root_cause().to_string()),
                };
                KeyRow { provider: cfg.name.clone(), key: cfg.api_key_env.clone(), status }
            }
            ProviderEntry::AppleFm(_) | ProviderEntry::Scrape(_) => {
                KeyRow { provider: entry.name().to_string(), key: None, status: KeyStatus::NotNeeded }
            }
        })
        .collect()
}

/// One row of the usage table.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageRow {
    pub provider: String,
    /// `None` for a provider that makes no counted requests (on-device).
    pub group: Option<String>,
    pub used: u32,
    pub limit: Option<u32>,
}

/// A row for each configured provider.
pub fn usage_rows(entries: &[ProviderEntry], usage: &Usage) -> Vec<UsageRow> {
    entries
        .iter()
        .map(|entry| match entry {
            ProviderEntry::OpenAi(cfg) => UsageRow {
                provider: cfg.name.clone(),
                group: Some(cfg.usage_group().to_string()),
                used: usage.used(cfg.usage_group()),
                limit: cfg.daily_limit,
            },
            ProviderEntry::AppleFm(cfg) => {
                UsageRow { provider: cfg.name.clone(), group: None, used: 0, limit: None }
            }
            ProviderEntry::Scrape(cfg) => UsageRow {
                provider: cfg.name.clone(),
                group: Some(cfg.usage_group().to_string()),
                used: usage.used(cfg.usage_group()),
                limit: cfg.daily_limit,
            },
        })
        .collect()
}

/// Where a `${NAME}` value comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingSource {
    Env,
    File,
    NotSet,
}

/// One row of the settings table.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingRow {
    pub name: String,
    pub value: Option<String>,
    pub source: SettingSource,
}

/// The placeholders the config uses plus anything stored, with each one's value and source.
pub fn setting_rows(entries: &[ProviderEntry], settings: &Settings) -> Vec<SettingRow> {
    let mut names: Vec<String> = entries
        .iter()
        .filter_map(|e| if let ProviderEntry::OpenAi(cfg) = e { Some(placeholders(&cfg.base_url)) } else { None })
        .flatten()
        .collect();
    names.extend(settings.names().cloned());
    names.sort();
    names.dedup();
    names
        .into_iter()
        .map(|name| match std::env::var(&name).ok().filter(|v| !v.is_empty()) {
            Some(v) => SettingRow { name, value: Some(v), source: SettingSource::Env },
            None => match settings.get(&name) {
                Some(v) => SettingRow { name, value: Some(v), source: SettingSource::File },
                None => SettingRow { name, value: None, source: SettingSource::NotSet },
            },
        })
        .collect()
}

/// What a run needs besides the config.
pub struct BuildOptions<'a> {
    /// Providers to turn on that their entry has `"enabled": false`.
    pub enable: &'a [String],
    /// Providers to leave out of this run.
    pub disable: &'a [String],
    /// Do not stop at `daily_limit` (requests are still counted).
    pub ignore_limits: bool,
    /// Allow providers that scrape web sites (`--scrape`); without it they are skipped with a warning.
    pub scrape: bool,
    /// Run browser scrape sources without a window for this run, whatever their entries say (`--headless`).
    /// Many sites refuse a browser with no window.
    pub headless: bool,
}

/// The `enable` and `disable` lists that make a run use only the provider called `name`: it is turned on
/// even if its entry says `"enabled": false`, and every other provider is left out. An unknown name is an error.
pub fn only_selection(entries: &[ProviderEntry], name: &str) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    if !entries.iter().any(|e| e.name() == name) {
        let known: Vec<&str> = entries.iter().map(|e| e.name()).collect();
        anyhow::bail!("no provider named `{name}` (configured: {})", known.join(", "));
    }
    let others = entries.iter().map(|e| e.name()).filter(|n| *n != name).map(str::to_string).collect();
    Ok((vec![name.to_string()], others))
}

/// The providers that can run, the names marked as preferred synthesizers, and why any others were left out.
pub struct Built {
    pub providers: Vec<Box<dyn Provider>>,
    pub designated: Vec<String>,
    /// One line per provider that was skipped (missing key, daily limit reached, Apple model unavailable),
    /// each starting `skipping provider:`.
    pub warnings: Vec<String>,
}

/// Choose the enabled entries and build a provider from each one that is usable. A provider that is not
/// usable is skipped with a warning rather than failing the run.
pub fn build_providers(
    entries: Vec<ProviderEntry>,
    settings: &Settings,
    store: &dyn KeyStore,
    usage: &SharedUsage,
    client: &reqwest::Client,
    options: &BuildOptions,
) -> anyhow::Result<Built> {
    let lookup = |name: &str| lookup_var(name, settings);
    let mut built = Built { providers: Vec::new(), designated: Vec::new(), warnings: Vec::new() };
    for entry in select_enabled(entries, options.enable, options.disable)? {
        if entry.synthesizer() {
            built.designated.push(entry.name().to_string());
        }
        match entry {
            ProviderEntry::OpenAi(mut cfg) => {
                if let Err(e) = cfg.expand_vars(&lookup) {
                    built.warnings.push(format!("skipping provider: {e:#}"));
                    continue;
                }
                match find_key(&cfg, store) {
                    Ok(found) => {
                        let key = found.map(|(key, _)| key);
                        let used = usage.lock().expect("usage lock").used(cfg.usage_group());
                        match cfg.daily_limit {
                            Some(limit) if used >= limit && !options.ignore_limits => built.warnings.push(format!(
                                "skipping provider: {}: daily limit of {limit} requests reached ({used} used today, UTC)",
                                cfg.name
                            )),
                            _ => built.providers.push(Box::new(OpenAiCompatible::new(
                                cfg,
                                key,
                                client.clone(),
                                usage.clone(),
                                !options.ignore_limits,
                            ))),
                        }
                    }
                    Err(e) => built.warnings.push(format!("skipping provider: {e:#}")),
                }
            }
            ProviderEntry::Scrape(cfg) => {
                let cfg = cfg.with_headless(options.headless);
                if !options.scrape {
                    built.warnings.push(format!(
                        "skipping provider: {}: it scrapes a web site, which is off unless you ask for it (--scrape)",
                        cfg.name
                    ));
                    continue;
                }
                let used = usage.lock().expect("usage lock").used(cfg.usage_group());
                match cfg.daily_limit {
                    Some(limit) if used >= limit && !options.ignore_limits => built.warnings.push(format!(
                        "skipping provider: {}: daily limit of {limit} requests reached ({used} used today, UTC)",
                        cfg.name
                    )),
                    _ => built.providers.push(Box::new(Scrape::new(
                        cfg,
                        client.clone(),
                        usage.clone(),
                        !options.ignore_limits,
                    ))),
                }
            }
            ProviderEntry::AppleFm(cfg) => match AppleFm::check(&cfg) {
                Ok(()) => built.providers.push(Box::new(AppleFm::new(cfg))),
                Err(e) => built.warnings.push(format!("skipping provider: {}: {e:#}", cfg.name)),
            },
        }
    }
    Ok(built)
}

/// Load the request counts for today from `path`, shareable between the providers running at once.
pub fn load_usage(path: std::path::PathBuf) -> SharedUsage {
    Arc::new(Mutex::new(Usage::load(path, crate::usage::today_utc())))
}

/// One provider's answer, or the reason it failed.
pub struct Reply {
    pub provider: String,
    pub result: anyhow::Result<String>,
}

/// Seconds a provider is given when neither `--timeout` nor its entry says.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// How long to wait for `provider`: the `--timeout` flag if given, else its entry's `timeout`, else the default.
pub fn effective_timeout(provider: &dyn Provider, flag: Option<u64>) -> Duration {
    Duration::from_secs(flag.or(provider.timeout()).unwrap_or(DEFAULT_TIMEOUT_SECS))
}

/// Send the prompt to every provider at once; a slow or failing one does not block the rest.
pub async fn fan_out(providers: &[Box<dyn Provider>], prompt: &str, flag: Option<u64>) -> Vec<Reply> {
    fan_out_each(providers, prompt, flag, &|_| {}).await
}

/// Like `fan_out`, and `on_done` is called with each reply the moment it arrives, so a caller can show
/// answers as they come in. The replies returned are in provider order.
pub async fn fan_out_each(
    providers: &[Box<dyn Provider>],
    prompt: &str,
    flag: Option<u64>,
    on_done: &(dyn Fn(&Reply) + Sync),
) -> Vec<Reply> {
    join_all(providers.iter().map(|p| async move {
        let timeout = effective_timeout(p.as_ref(), flag);
        let result = match tokio::time::timeout(timeout, p.ask(prompt)).await {
            Ok(r) => r,
            Err(_) => Err(anyhow::anyhow!("timed out after {}s", timeout.as_secs())),
        };
        let reply = Reply { provider: p.name().to_string(), result };
        on_done(&reply);
        reply
    }))
    .await
}

/// The replies as report entries, with each failure rendered to text.
pub fn entries_from(replies: Vec<Reply>) -> Vec<output::Entry> {
    replies
        .into_iter()
        .map(|r| output::Entry { provider: r.provider, outcome: r.result.map_err(|e| format!("{e:#}")) })
        .collect()
}

/// How `--merge` picks and runs its synthesizer.
pub struct Merge<'a> {
    pub designated: &'a [String],
    pub requested: Option<&'a str>,
    /// The `--timeout` flag, if given.
    pub timeout: Option<u64>,
}

/// Ask the synthesizer to consolidate the answers in `report`, recording the result in it. Returns
/// notices for the caller to show (nothing to merge, or the synthesizer failed).
pub async fn merge_answers(
    report: &mut output::Report,
    providers: &[Box<dyn Provider>],
    choice: Merge<'_>,
) -> anyhow::Result<Vec<String>> {
    let mut notices = Vec::new();
    let synthesis_prompt = {
        let answers: Vec<(&str, &str)> = report
            .entries
            .iter()
            .filter_map(|e| e.outcome.as_deref().ok().map(|text| (e.provider.as_str(), text)))
            .collect();
        match answers.len() {
            0 => anyhow::bail!("no provider answered, nothing to merge"),
            1 => {
                notices.push(format!("only {} answered; nothing to merge", answers[0].0));
                return Ok(notices);
            }
            _ => merge::synthesis_prompt(&report.prompt, &answers),
        }
    };

    // The synthesizer is chosen among providers that are running, not only those that answered.
    let running: Vec<String> = providers.iter().map(|p| p.name().to_string()).collect();
    let synth = merge::pick_synthesizer(&running, choice.designated, choice.requested)?;
    let synth_provider = providers.iter().find(|p| p.name() == synth).expect("picked from the running providers");
    let timeout = effective_timeout(synth_provider.as_ref(), choice.timeout);
    let merged = match tokio::time::timeout(timeout, synth_provider.ask(&synthesis_prompt)).await {
        Ok(r) => r,
        Err(_) => Err(anyhow::anyhow!("timed out after {}s", timeout.as_secs())),
    };
    match merged {
        Ok(text) => report.merged = Some((synth.to_string(), text)),
        Err(e) => {
            notices.push(format!("merging with {synth} failed: {e:#}; showing the separate answers"));
            report.merge_error = Some(format!("{synth}: {e:#}"));
        }
    }
    Ok(notices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keystore::KeyStore;

    #[test]
    fn timeout_flag_beats_entry_beats_default() {
        struct Fixed(Option<u64>);
        impl Provider for Fixed {
            fn name(&self) -> &str {
                "fixed"
            }
            fn ask<'a>(
                &'a self,
                _: &'a str,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send + 'a>> {
                Box::pin(async { Ok(String::new()) })
            }
            fn timeout(&self) -> Option<u64> {
                self.0
            }
        }
        assert_eq!(effective_timeout(&Fixed(None), None), Duration::from_secs(30));
        assert_eq!(effective_timeout(&Fixed(Some(180)), None), Duration::from_secs(180));
        assert_eq!(effective_timeout(&Fixed(Some(180)), Some(5)), Duration::from_secs(5));
        assert_eq!(effective_timeout(&Fixed(None), Some(5)), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn each_reply_is_reported_as_it_arrives_and_returned_in_order() {
        struct After(&'static str, u64);
        impl Provider for After {
            fn name(&self) -> &str {
                self.0
            }
            fn ask<'a>(
                &'a self,
                _: &'a str,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send + 'a>> {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(self.1)).await;
                    Ok(self.0.to_string())
                })
            }
        }
        let providers: Vec<Box<dyn Provider>> = vec![Box::new(After("slow", 60)), Box::new(After("fast", 1))];
        let seen = Mutex::new(Vec::new());
        let replies = fan_out_each(&providers, "p", None, &|r| seen.lock().unwrap().push(r.provider.clone())).await;
        assert_eq!(*seen.lock().unwrap(), vec!["fast", "slow"], "reported in completion order");
        let names: Vec<&str> = replies.iter().map(|r| r.provider.as_str()).collect();
        assert_eq!(names, vec!["slow", "fast"], "returned in provider order");
    }

    #[test]
    fn bundled_example_config_parses() {
        assert!(!key_types(&parse_config(EXAMPLE_CONFIG).unwrap()).is_empty());
    }

    #[test]
    fn key_types_group_providers_and_skip_keyless() {
        let entries = parse_config(
            r#"[
            {"name":"a","base_url":"u","model":"m","api_key_env":"K1"},
            {"name":"o","base_url":"u","model":"m"},
            {"name":"b","base_url":"u","model":"m","api_key_env":"K2"},
            {"name":"c","base_url":"u","model":"m","api_key_env":"K1"},
            {"name":"apple","kind":"apple_fm"}
        ]"#,
        )
        .unwrap();
        assert_eq!(
            key_types(&entries),
            vec![("K1".to_string(), vec!["a".to_string(), "c".to_string()]), ("K2".to_string(), vec!["b".to_string()])]
        );
    }

    /// A key store with fixed contents, so the tests never touch the real keychain.
    struct Fake(Option<String>);
    impl KeyStore for Fake {
        fn get(&self, _: &str) -> anyhow::Result<Option<String>> {
            Ok(self.0.clone())
        }
        fn set(&self, _: &str, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete(&self, _: &str) -> anyhow::Result<bool> {
            Ok(false)
        }
    }

    fn entries() -> Vec<ProviderEntry> {
        parse_config(
            r#"[
            {"name":"a","base_url":"https://x/${PAMPAS_ENGINE_TEST_ACCT}/v1","model":"m","api_key_env":"PAMPAS_ENGINE_TEST_KEY","daily_limit":2,"usage_group":"g"},
            {"name":"local","base_url":"http://localhost/v1","model":"m"},
            {"name":"off","base_url":"u","model":"m","enabled":false}
        ]"#,
        )
        .unwrap()
    }

    #[test]
    fn rows_describe_keys_usage_and_settings() {
        let es = entries();
        let rows = key_rows(&es, &Fake(None));
        assert_eq!(rows[0].status, KeyStatus::Missing);
        assert_eq!(rows[0].key.as_deref(), Some("PAMPAS_ENGINE_TEST_KEY"));
        assert_eq!(rows[1].status, KeyStatus::NotNeeded);
        assert_eq!(key_rows(&es, &Fake(Some("k".into())))[0].status, KeyStatus::Keychain);

        let usage = Usage::in_memory("2026-01-01");
        let urows = usage_rows(&es, &usage);
        assert_eq!((urows[0].group.as_deref(), urows[0].used, urows[0].limit), (Some("g"), 0, Some(2)));

        let settings = Settings::default();
        assert_eq!(
            setting_rows(&es, &settings),
            vec![SettingRow { name: "PAMPAS_ENGINE_TEST_ACCT".into(), value: None, source: SettingSource::NotSet }]
        );
    }

    #[test]
    fn building_skips_unusable_providers_with_a_warning_and_never_prints() {
        let usage: SharedUsage = Arc::new(Mutex::new(Usage::in_memory("2026-01-01")));
        let client = reqwest::Client::new();
        let opts = BuildOptions { enable: &[], disable: &[], ignore_limits: false, scrape: false, headless: false };
        // No account id and no key: "a" is skipped for the id first; "local" runs; "off" is disabled.
        let built = build_providers(entries(), &Settings::default(), &Fake(None), &usage, &client, &opts).unwrap();
        let names: Vec<&str> = built.providers.iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["local"]);
        assert_eq!(built.warnings.len(), 1);
        assert!(built.warnings[0].starts_with("skipping provider:") && built.warnings[0].contains("PAMPAS_ENGINE_TEST_ACCT"));
        // Turning on a disabled entry, and an unknown name is an error.
        let opts = BuildOptions { enable: &["off".to_string()], disable: &[], ignore_limits: false, scrape: false, headless: false };
        let built = build_providers(entries(), &Settings::default(), &Fake(None), &usage, &client, &opts).unwrap();
        assert_eq!(built.providers.len(), 2);
        let opts = BuildOptions { enable: &["nope".to_string()], disable: &[], ignore_limits: false, scrape: false, headless: false };
        assert!(build_providers(entries(), &Settings::default(), &Fake(None), &usage, &client, &opts).is_err());
    }

    #[test]
    fn only_selects_one_provider_even_if_it_is_switched_off() {
        let es = entries();
        let (enable, disable) = only_selection(&es, "off").unwrap();
        assert_eq!(enable, ["off"]);
        assert!(!disable.contains(&"off".to_string()) && disable.len() == es.len() - 1);
        let picked = select_enabled(entries(), &enable, &disable).unwrap();
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].name(), "off");
        assert!(only_selection(&es, "nope").unwrap_err().to_string().contains("no provider named `nope`"));
    }

    #[test]
    fn scrape_providers_run_only_with_scrape() {
        let cfg = r#"[{"name":"site","kind":"scrape","url":"https://x.test/?q={prompt}","selector":"p"}]"#;
        let usage: SharedUsage = Arc::new(Mutex::new(Usage::in_memory("2026-01-01")));
        let client = reqwest::Client::new();
        let build = |scrape| {
            let opts = BuildOptions { enable: &[], disable: &[], ignore_limits: false, scrape, headless: false };
            build_providers(parse_config(cfg).unwrap(), &Settings::default(), &Fake(None), &usage, &client, &opts).unwrap()
        };
        let off = build(false);
        assert!(off.providers.is_empty());
        assert!(off.warnings[0].contains("--scrape"));
        let on = build(true);
        assert_eq!(on.providers.len(), 1);
        assert!(on.warnings.is_empty());
    }
}
