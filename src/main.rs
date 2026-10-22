mod keystore;
mod merge;
mod output;
mod provider;
mod usage;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};
use futures::future::join_all;

use keystore::{KeyStore, Keychain, Source, resolve};
use provider::{AppleFm, OpenAiCompatible, Provider, ProviderConfig, ProviderEntry, parse_config, select_enabled};
use usage::{SharedUsage, Usage, today_utc};

#[derive(Parser)]
#[command(
    name = "pampas",
    version,
    about = "Parallel AI Multi-provider Prompt Aggregator & Synthesizer: query several free AI services in parallel and merge the results",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// The prompt to send to every provider
    prompt: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,

    /// Per-provider timeout in seconds
    #[arg(long, default_value_t = 30)]
    timeout: u64,

    /// Turn on a provider that its config entry has `"enabled": false` (repeatable, or comma-separated)
    #[arg(long, value_name = "NAME", value_delimiter = ',')]
    enable: Vec<String>,

    /// Leave a provider out of this run (repeatable, or comma-separated)
    #[arg(long, value_name = "NAME", value_delimiter = ',')]
    disable: Vec<String>,

    /// Consolidate the answers into one by asking a provider to synthesize them
    #[arg(long)]
    merge: bool,

    /// With --merge, the provider that writes the consolidated answer (default: the first entry marked
    /// `"synthesizer": true`, else the first running provider)
    #[arg(long, value_name = "NAME", requires = "merge")]
    synthesizer: Option<String>,

    /// With --merge, also print each provider's own answer after the consolidated one
    #[arg(long, requires = "merge")]
    show_answers: bool,

    /// Write the result to this file instead of stdout (overwrites it)
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Output format: text, markdown (alias md) or json
    #[arg(short, long, value_enum, default_value_t = output::Format::Markdown)]
    format: output::Format,

    /// Ignore `daily_limit` for this run (requests are still counted)
    #[arg(long)]
    ignore_limits: bool,

    /// Request-count file (default: ~/.local/state/pampas/usage.json)
    #[arg(long, global = true, value_name = "PATH")]
    usage_file: Option<PathBuf>,

    /// Providers config file (default: ~/.config/pampas/providers.json)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Show today's request count against each provider's daily limit (UTC day)
    Usage,
    /// Manage API keys in the OS keychain
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// Store a key, read from a hidden prompt; NAME is the provider's `api_key_env`, e.g. GROQ_API_KEY
    Set { name: String },
    /// Remove a stored key
    Delete { name: String },
    /// Show where each configured provider's key comes from (never prints keys)
    List,
}

/// One provider's answer, or the reason it failed.
struct Reply {
    provider: String,
    result: anyhow::Result<String>,
}

/// Send the prompt to every provider at once; a slow or failing one does not block the rest.
async fn fan_out(providers: &[Box<dyn Provider>], prompt: &str, timeout: Duration) -> Vec<Reply> {
    join_all(providers.iter().map(|p| async move {
        let result = match tokio::time::timeout(timeout, p.ask(prompt)).await {
            Ok(r) => r,
            Err(_) => Err(anyhow::anyhow!("timed out after {}s", timeout.as_secs())),
        };
        Reply { provider: p.name().to_string(), result }
    }))
    .await
}

fn default_config_path() -> anyhow::Result<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(".config"),
    };
    Ok(base.join("pampas/providers.json"))
}

fn default_usage_path() -> anyhow::Result<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(".local/state"),
    };
    Ok(base.join("pampas/usage.json"))
}

fn load_config(path: &Path) -> anyhow::Result<Vec<ProviderEntry>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_config(&text)
}

/// The provider's key: its environment variable, else the keychain. `None` when the entry names no key.
fn find_key(cfg: &ProviderConfig, store: &dyn KeyStore) -> anyhow::Result<Option<(String, Source)>> {
    let Some(name) = &cfg.api_key_env else { return Ok(None) };
    let env = std::env::var(name).ok();
    resolve(name, env, store)
        .with_context(|| format!("{}: looking up {name}", cfg.name))?
        .with_context(|| format!("{}: no key: set {name} or run `pampas key set {name}`", cfg.name))
        .map(Some)
}

/// Print `name  group  used/limit` for each provider that makes counted requests.
fn run_usage(config: Option<PathBuf>, usage_file: Option<PathBuf>) -> anyhow::Result<()> {
    let path = match config {
        Some(p) => p,
        None => default_config_path()?,
    };
    let usage = Usage::load(
        match usage_file {
            Some(p) => p,
            None => default_usage_path()?,
        },
        today_utc(),
    );
    for entry in load_config(&path)? {
        match entry {
            ProviderEntry::OpenAi(cfg) => {
                let limit = cfg.daily_limit.map_or("-".to_string(), |l| l.to_string());
                println!("{}\t{}\t{}/{}", cfg.name, cfg.usage_group(), usage.used(cfg.usage_group()), limit);
            }
            ProviderEntry::AppleFm(cfg) => println!("{}\t-\tlocal, not counted", cfg.name),
        }
    }
    Ok(())
}

fn run_key(action: KeyAction, config: Option<PathBuf>) -> anyhow::Result<()> {
    match action {
        KeyAction::Set { name } => {
            let key = rpassword::prompt_password(format!("{name}: "))?;
            let key = key.trim();
            anyhow::ensure!(!key.is_empty(), "no key entered");
            Keychain.set(&name, key)?;
            println!("stored {name} in the OS keychain");
        }
        KeyAction::Delete { name } => {
            if Keychain.delete(&name)? {
                println!("deleted {name} from the OS keychain");
            } else {
                println!("{name} was not in the OS keychain");
            }
        }
        KeyAction::List => {
            let path = match config {
                Some(p) => p,
                None => default_config_path()?,
            };
            for entry in load_config(&path)? {
                match entry {
                    ProviderEntry::OpenAi(cfg) => {
                        let status = match find_key(&cfg, &Keychain) {
                            Ok(None) => "no key needed".to_string(),
                            Ok(Some((_, Source::Env))) => "environment variable".to_string(),
                            Ok(Some((_, Source::Keychain))) => "OS keychain".to_string(),
                            Err(e) => format!("missing ({e:#})"),
                        };
                        println!("{}\t{}\t{}", cfg.name, cfg.api_key_env.as_deref().unwrap_or("-"), status);
                    }
                    ProviderEntry::AppleFm(cfg) => println!("{}\t-\tno key needed", cfg.name),
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Key { action }) => return run_key(action, cli.config),
        Some(Command::Usage) => return run_usage(cli.config, cli.usage_file),
        None => {}
    }
    let prompt = cli.prompt.context("a prompt is required: pampas \"your prompt\"")?;
    let path = match cli.config {
        Some(p) => p,
        None => default_config_path()?,
    };
    let client = reqwest::Client::new();

    // A provider that is not usable (missing key, Apple model unavailable) is skipped with a warning
    // rather than aborting the run.
    let usage: SharedUsage = Arc::new(Mutex::new(Usage::load(
        match cli.usage_file {
            Some(p) => p,
            None => default_usage_path()?,
        },
        today_utc(),
    )));
    let mut providers: Vec<Box<dyn Provider>> = Vec::new();
    let mut designated: Vec<String> = Vec::new();
    for entry in select_enabled(load_config(&path)?, &cli.enable, &cli.disable)? {
        if entry.synthesizer() {
            designated.push(entry.name().to_string());
        }
        match entry {
            ProviderEntry::OpenAi(cfg) => match find_key(&cfg, &Keychain) {
                Ok(found) => {
                    let key = found.map(|(key, _)| key);
                    let used = usage.lock().expect("usage lock").used(cfg.usage_group());
                    match cfg.daily_limit {
                        Some(limit) if used >= limit && !cli.ignore_limits => eprintln!(
                            "skipping provider: {}: daily limit of {limit} requests reached ({used} used today, UTC)",
                            cfg.name
                        ),
                        _ => providers.push(Box::new(OpenAiCompatible::new(
                            cfg,
                            key,
                            client.clone(),
                            usage.clone(),
                            !cli.ignore_limits,
                        ))),
                    }
                }
                Err(e) => eprintln!("skipping provider: {e:#}"),
            },
            ProviderEntry::AppleFm(cfg) => match AppleFm::check(&cfg) {
                Ok(()) => providers.push(Box::new(AppleFm::new(cfg))),
                Err(e) => eprintln!("skipping provider: {}: {e:#}", cfg.name),
            },
        }
    }
    anyhow::ensure!(!providers.is_empty(), "no usable providers in {}", path.display());

    let replies = fan_out(&providers, &prompt, Duration::from_secs(cli.timeout)).await;
    let entries: Vec<output::Entry> = replies
        .into_iter()
        .map(|r| output::Entry { provider: r.provider, outcome: r.result.map_err(|e| format!("{e:#}")) })
        .collect();
    for e in &entries {
        if let Err(msg) = &e.outcome {
            eprintln!("## {} failed: {msg}", e.provider);
        }
    }

    let mut report =
        output::Report { prompt, entries, merged: None, merge_error: None, show_answers: cli.show_answers };
    if cli.merge {
        let choice = Merge { designated: &designated, requested: cli.synthesizer.as_deref(), timeout: cli.timeout };
        merge_answers(&mut report, &providers, choice).await?;
    }
    emit(&report, cli.format, cli.output.as_deref())
}

/// How `--merge` picks and runs its synthesizer.
struct Merge<'a> {
    designated: &'a [String],
    requested: Option<&'a str>,
    timeout: u64,
}

/// Ask the synthesizer to consolidate the answers in `report`, recording the result in it.
async fn merge_answers(report: &mut output::Report, providers: &[Box<dyn Provider>], choice: Merge<'_>) -> anyhow::Result<()> {
    let synthesis_prompt = {
        let answers: Vec<(&str, &str)> = report
            .entries
            .iter()
            .filter_map(|e| e.outcome.as_deref().ok().map(|text| (e.provider.as_str(), text)))
            .collect();
        match answers.len() {
            0 => anyhow::bail!("no provider answered, nothing to merge"),
            1 => {
                eprintln!("only {} answered; nothing to merge", answers[0].0);
                return Ok(());
            }
            _ => merge::synthesis_prompt(&report.prompt, &answers),
        }
    };

    // The synthesizer is chosen among providers that are running, not only those that answered.
    let running: Vec<String> = providers.iter().map(|p| p.name().to_string()).collect();
    let synth = merge::pick_synthesizer(&running, choice.designated, choice.requested)?;
    let synth_provider = providers.iter().find(|p| p.name() == synth).expect("picked from the running providers");
    let timeout = Duration::from_secs(choice.timeout);
    let merged = match tokio::time::timeout(timeout, synth_provider.ask(&synthesis_prompt)).await {
        Ok(r) => r,
        Err(_) => Err(anyhow::anyhow!("timed out after {}s", timeout.as_secs())),
    };
    match merged {
        Ok(text) => report.merged = Some((synth.to_string(), text)),
        Err(e) => {
            eprintln!("merging with {synth} failed: {e:#}; showing the separate answers");
            report.merge_error = Some(format!("{synth}: {e:#}"));
        }
    }
    Ok(())
}

/// Print the rendered report, or write it to `path`. If the file cannot be written the result is
/// printed instead, so a slow run's answers are not lost.
fn emit(report: &output::Report, format: output::Format, path: Option<&Path>) -> anyhow::Result<()> {
    let text = output::render(report, format);
    let Some(path) = path else {
        print!("{text}");
        return Ok(());
    };
    match std::fs::write(path, &text) {
        Ok(()) => {
            eprintln!("wrote {}", path.display());
            Ok(())
        }
        Err(e) => {
            print!("{text}");
            Err(anyhow::Error::new(e).context(format!("writing {} (output printed instead)", path.display())))
        }
    }
}
