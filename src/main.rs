use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};

use pampas::engine::{
    self, BuildOptions, KeyStatus, Merge, SettingSource, build_providers, entries_from, fan_out, key_rows, key_types,
    load_config, load_usage, merge_answers, setting_rows, usage_rows, write_example_config,
};
use pampas::keystore::{KeyStore, Keychain};
use pampas::provider::parse_config;
use pampas::settings::{Settings, settings_path};
use pampas::usage::{Usage, today_utc};
use pampas::{output, paths, settings};

#[derive(Parser)]
#[command(
    name = "pampas-cli",
    version,
    about = "Parallel AI Multi-provider Prompt Aggregator & Synthesizer: query several free AI services in parallel and merge the results"
)]
struct Cli {
    /// The prompt to send to every provider
    prompt: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,

    /// Per-provider timeout in seconds, for every provider this run (default: the entry's `timeout`, else 30)
    #[arg(long)]
    timeout: Option<u64>,

    /// Turn on a provider that its config entry has `"enabled": false` (repeatable, or comma-separated)
    #[arg(long, value_name = "NAME", value_delimiter = ',')]
    enable: Vec<String>,

    /// Leave a provider out of this run (repeatable, or comma-separated)
    #[arg(long, value_name = "NAME", value_delimiter = ',')]
    disable: Vec<String>,

    /// Ask only this provider, even if its entry is switched off, and leave every other one out
    #[arg(long, value_name = "NAME", conflicts_with_all = ["enable", "disable"])]
    only: Option<String>,

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

    /// Also ask providers that scrape web sites with no API (`"kind": "scrape"` entries). Off by default:
    /// automated requests can get your IP address blocked or banned, and may break a site's terms
    #[arg(long)]
    scrape: bool,

    /// Run browser scrape sources without a window for this run (many sites block that); needs --scrape
    #[arg(long)]
    headless: bool,

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
    /// Write the bundled example providers config to the config path (see the README for the default per platform)
    Init,
    /// Move your files from the old macOS locations (~/.config/pampas, ~/.local/state/pampas) to the current ones
    Migrate,
    /// Manage the non-secret values behind `${NAME}` placeholders in provider `base_url`s
    Setting {
        #[command(subcommand)]
        action: SettingAction,
    },
    /// Add or list web sites that are scraped because they have no API (used with `--scrape`)
    Scrape {
        #[command(subcommand)]
        action: ScrapeAction,
    },
    /// Manage API keys in the OS keychain
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

#[derive(Subcommand)]
enum SettingAction {
    /// Store a value, e.g. `pampas-cli setting set CLOUDFLARE_ACCOUNT_ID abc123`; an environment variable of the same name wins
    Set {
        name: String,
        /// The value; asked for at a prompt if left out
        value: Option<String>,
    },
    /// Remove a stored value
    Delete { name: String },
    /// Show each placeholder the config uses, or that is stored, with its value and where it comes from
    List,
}

#[derive(Subcommand)]
enum ScrapeAction {
    /// Add a scrape source to the providers config. This is the only way to add one: it shows the risks
    /// and needs you to type "I understand" at a terminal (the app can show and switch these sources but not add them).
    /// With a known name and no other options (`pampas-cli scrape add deepseek`) the built-in setup for that site is used
    Add {
        /// Name shown in answers; `deepseek` alone uses the built-in setup
        name: String,
        /// Say you have read the risks instead of typing "I understand": they are still printed, followed by a
        /// line saying you acknowledged them, and no terminal is needed
        #[arg(long)]
        acknowledge: bool,
        /// Address to request; `{prompt}` is replaced by the prompt, percent-encoded
        #[arg(long)]
        url: Option<String>,
        /// CSS selector for the elements whose text is the answer (an HTML reply)
        #[arg(long, conflicts_with = "json_pointer")]
        selector: Option<String>,
        /// JSON pointer to the answer, e.g. /data/0/text (a JSON reply)
        #[arg(long)]
        json_pointer: Option<String>,
        /// Send a POST with this body instead of a GET; `{prompt}` is replaced by the prompt
        #[arg(long)]
        body: Option<String>,
        /// How the prompt is encoded in the body: form (default) or json
        #[arg(long, value_parser = ["form", "json"], requires = "body")]
        body_format: Option<String>,
        /// Extra request header as `Name: value`; repeatable
        #[arg(long = "header", value_name = "NAME: VALUE")]
        headers: Vec<String>,
        /// User-Agent header (default `pampas`)
        #[arg(long)]
        user_agent: Option<String>,
        /// Drive a real Chrome instead of sending one request, for pages that need JavaScript or a login;
        /// `--selector` is then the answer's selector and `--input-selector` is required
        #[arg(long)]
        browser: bool,
        /// Browser: CSS selector of the box the prompt is typed into
        #[arg(long)]
        input_selector: Option<String>,
        /// Browser: CSS selector of the send button (default: press Enter)
        #[arg(long)]
        submit_selector: Option<String>,
        /// Browser: CSS selector of something visible only while the answer is being written, e.g. a stop button
        #[arg(long)]
        busy_selector: Option<String>,
        /// Browser: CSS selector of an iframe on the same site that holds the answer; `--selector` and
        /// `--busy-selector` are then looked up inside it
        #[arg(long)]
        answer_frame: Option<String>,
        /// Browser: seconds the answer must stay unchanged to count as finished (default 4)
        #[arg(long)]
        settle_secs: Option<u64>,
        /// Browser: run with no window (many sites block this)
        #[arg(long)]
        headless: bool,
        /// Browser: path to Chrome or Chromium if it is not found on its own
        #[arg(long)]
        browser_path: Option<String>,
        /// Most requests per UTC day (strongly advised, to lower the chance of a block)
        #[arg(long)]
        daily_limit: Option<u32>,
        /// Seconds to wait for this site
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Open a browser source's page in a normal window on its own profile, so you can log in by hand once;
    /// close the window when done
    Login { name: String },
    /// Remove a scrape source from the providers config (a browser source's login stays in its profile folder)
    Remove { name: String },
    /// List the scrape sources in the providers config
    List,
}

#[derive(Subcommand)]
enum KeyAction {
    /// Store a key, read from a hidden prompt; NAME is the provider's `api_key_env`, e.g. GROQ_API_KEY
    Set {
        name: String,
        /// Read the key from standard input instead of a prompt, e.g. `pass show groq | pampas-cli key set GROQ_API_KEY --stdin`
        #[arg(long)]
        stdin: bool,
    },
    /// Remove a stored key
    Delete { name: String },
    /// Show where each configured provider's key comes from (never prints keys)
    List,
    /// List the key names the configured providers use, with the providers that share each
    Types,
}

/// The default providers config path. On macOS this may be the old location, with a hint about
/// `pampas-cli migrate`.
fn default_config_path() -> anyhow::Result<PathBuf> {
    let found = paths::config_file(paths::Os::current(), &paths::real_env, &|p: &Path| p.exists())?;
    if found.legacy {
        eprintln!("using {} (the old location); run `pampas-cli migrate` to move your files", found.path.display());
    }
    Ok(found.path)
}

/// The default usage file path; see `default_config_path`.
fn default_usage_path() -> anyhow::Result<PathBuf> {
    let found = paths::usage_file(paths::Os::current(), &paths::real_env, &|p: &Path| p.exists())?;
    if found.legacy {
        eprintln!("using {} (the old location); run `pampas-cli migrate` to move your files", found.path.display());
    }
    Ok(found.path)
}

/// Move files from the old macOS locations to the current ones.
fn run_migrate() -> anyhow::Result<()> {
    let plan = paths::migration_plan(paths::Os::current(), &paths::real_env, &|p: &Path| p.exists())?;
    if plan.is_empty() {
        println!("nothing to migrate");
        return Ok(());
    }
    for (from, to) in &plan {
        println!("{} -> {}", from.display(), to.display());
    }
    println!("moved {} file(s)", paths::migrate(&plan)?);
    Ok(())
}

fn run_setting(action: SettingAction, config: Option<PathBuf>) -> anyhow::Result<()> {
    let config_path = match config {
        Some(p) => p,
        None => default_config_path()?,
    };
    let path = settings_path(&config_path);
    let mut settings = Settings::load(&path)?;
    match action {
        SettingAction::Set { name, value } => {
            settings::check_name(&name)?;
            let value = match value {
                Some(v) => v,
                None => prompt_value(&name)?,
            };
            settings.set(&name, &value)?;
            settings.save(&path)?;
            println!("stored {name} in {}", path.display());
            if std::env::var(&name).is_ok_and(|v| !v.is_empty()) {
                println!("note: the environment variable {name} is set and takes precedence");
            }
        }
        SettingAction::Delete { name } => {
            if settings.delete(&name) {
                settings.save(&path)?;
                println!("deleted {name} from {}", path.display());
            } else {
                println!("{name} was not in {}", path.display());
            }
        }
        SettingAction::List => {
            // The placeholders the config uses (if it can be read) plus anything stored.
            let entries = load_config(&config_path).unwrap_or_default();
            let rows: Vec<Vec<String>> = setting_rows(&entries, &settings)
                .into_iter()
                .map(|r| {
                    let source = match r.source {
                        SettingSource::Env => "environment variable",
                        SettingSource::File => "settings file",
                        SettingSource::NotSet => "not set",
                    };
                    vec![r.name, r.value.unwrap_or_else(|| "-".into()), source.into()]
                })
                .collect();
            print_table(&["NAME", "VALUE", "SOURCE"], &rows);
        }
    }
    Ok(())
}

/// Print `name  group  used/limit` for each provider that makes counted requests.
fn run_usage(config: Option<PathBuf>, usage_file: Option<PathBuf>) -> anyhow::Result<()> {
    let path = config_path_or_offer(config)?;
    let usage = Usage::load(
        match usage_file {
            Some(p) => p,
            None => default_usage_path()?,
        },
        today_utc(),
    );
    let rows: Vec<Vec<String>> = usage_rows(&load_config(&path)?, &usage)
        .into_iter()
        .map(|r| match r.group {
            Some(group) => {
                let limit = r.limit.map_or("-".to_string(), |l| l.to_string());
                vec![r.provider, group, format!("{}/{}", r.used, limit)]
            }
            None => vec![r.provider, "-".into(), "local, not counted".into()],
        })
        .collect();
    print_table(&["PROVIDER", "GROUP", "USED/LIMIT"], &rows);
    Ok(())
}

/// Lay out rows under `headers`: aligned columns with a header line, or, for scripts, bare tab-separated lines.
fn format_table(headers: &[&str], rows: &[Vec<String>], aligned: bool) -> String {
    if !aligned {
        return rows.iter().map(|r| r.join("\t") + "\n").collect();
    }
    let widths: Vec<usize> = (0..headers.len())
        .map(|i| rows.iter().map(|r| r[i].chars().count()).chain([headers[i].chars().count()]).max().unwrap_or(0))
        .collect();
    let line = |cells: Vec<&str>| {
        let padded: Vec<String> = cells.iter().zip(&widths).map(|(c, w)| format!("{c:<w$}")).collect();
        padded.join("  ").trim_end().to_string() + "\n"
    };
    let mut out = line(headers.to_vec());
    for r in rows {
        out += &line(r.iter().map(String::as_str).collect());
    }
    out
}

/// Print a table to stdout, aligned when it is a terminal and tab-separated when piped.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    print!("{}", format_table(headers, rows, std::io::stdout().is_terminal()));
}

/// Write the example config to `config` (or the default path); never overwrite an existing file.
fn run_init(config: Option<PathBuf>) -> anyhow::Result<()> {
    let path = match config {
        Some(p) => p,
        None => default_config_path()?,
    };
    write_example_and_explain(&path)
}

/// `pampas-cli scrape add|remove|list|login`. Adding needs the risks acknowledged: typed at a terminal, or
/// with `--acknowledge`, which the user passes knowing what it says. The app can never add one.
async fn run_scrape(action: ScrapeAction, config: Option<PathBuf>) -> anyhow::Result<()> {
    let path = match config {
        Some(p) => p,
        None => default_config_path()?,
    };
    match action {
        ScrapeAction::Login { name } => {
            let cfg = load_config(&path)?
                .into_iter()
                .find_map(|e| match e {
                    pampas::provider::ProviderEntry::Scrape(c) if c.name == name => Some(c),
                    _ => None,
                })
                .with_context(|| format!("no scrape source named {name:?} in {}", path.display()))?;
            anyhow::ensure!(cfg.render == pampas::scrape::Render::Browser, "{name} is not a browser source, so it has nothing to log in to");
            println!("opening {} in a browser window for {name}: log in, then close the window", cfg.url);
            pampas::browser::login(&cfg).await
        }
        ScrapeAction::Remove { name } => {
            pampas::scrape::remove_entry(&path, &name)?;
            println!("removed {name} from {}; the previous file is kept as providers.json.bak", path.display());
            Ok(())
        }
        ScrapeAction::List => {
            let rows: Vec<Vec<String>> = load_config(&path)?
                .into_iter()
                .filter_map(|e| match e {
                    pampas::provider::ProviderEntry::Scrape(c) => Some(vec![
                        c.name.clone(),
                        c.url.clone(),
                        c.daily_limit.map_or("none".to_string(), |n| n.to_string()),
                        if c.enabled { "on" } else { "off" }.to_string(),
                    ]),
                    _ => None,
                })
                .collect();
            if rows.is_empty() {
                println!("no scrape sources in {}; add one with `pampas-cli scrape add`", path.display());
            } else {
                print_table(&["NAME", "URL", "DAILY LIMIT", "ENABLED"], &rows);
            }
            Ok(())
        }
        ScrapeAction::Add {
            name,
            acknowledge,
            url,
            selector,
            json_pointer,
            body,
            body_format,
            headers,
            user_agent,
            browser,
            input_selector,
            submit_selector,
            busy_selector,
            answer_frame,
            settle_secs,
            headless,
            browser_path,
            daily_limit,
            timeout,
        } => {
            use serde_json::{Map, Value, json};
            // A known site with no address given: fill in what is known about it.
            let (mut selector, mut browser, mut input_selector, mut settle_secs, mut daily_limit, mut busy_selector) =
                (selector, browser, input_selector, settle_secs, daily_limit, busy_selector);
            let mut answer_frame = answer_frame;
            let mut submit_selector = submit_selector;
            let mut site_warning = None;
            let mut first_run = None;
            let url = match url {
                Some(url) => url,
                None => {
                    let preset = pampas::scrape::preset(&name).with_context(|| {
                        format!(
                            "no address given: pass --url, or use the name of a built-in site ({})",
                            pampas::scrape::PRESETS.iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
                        )
                    })?;
                    anyhow::ensure!(
                        json_pointer.is_none() && body.is_none(),
                        "the built-in setup for {name} cannot be combined with --json-pointer or --body"
                    );
                    println!("using the built-in setup for {name} ({}); change any part of it with its option", preset.checked);
                    browser = true;
                    selector.get_or_insert_with(|| preset.selector.to_string());
                    input_selector.get_or_insert_with(|| preset.input_selector.to_string());
                    if let Some(busy) = preset.busy_selector {
                        busy_selector.get_or_insert_with(|| busy.to_string());
                    }
                    if let Some(button) = preset.submit_selector {
                        submit_selector.get_or_insert_with(|| button.to_string());
                    }
                    if let Some(frame) = preset.answer_frame {
                        answer_frame.get_or_insert_with(|| frame.to_string());
                    }
                    site_warning = preset.warning;
                    first_run = preset.first_run;
                    settle_secs.get_or_insert(preset.settle_secs);
                    daily_limit.get_or_insert(preset.daily_limit);
                    preset.url.to_string()
                }
            };
            let mut entry = Map::new();
            entry.insert("name".into(), json!(name));
            entry.insert("kind".into(), json!("scrape"));
            entry.insert("url".into(), json!(url));
            if let Some(v) = selector {
                entry.insert("selector".into(), json!(v));
            }
            if let Some(v) = json_pointer {
                entry.insert("json_pointer".into(), json!(v));
            }
            if browser {
                entry.insert("render".into(), json!("browser"));
            }
            for (key, value) in [
                ("input_selector", input_selector),
                ("submit_selector", submit_selector),
                ("busy_selector", busy_selector),
                ("answer_frame", answer_frame),
                ("browser_path", browser_path),
            ] {
                if let Some(v) = value {
                    entry.insert(key.into(), json!(v));
                }
            }
            if let Some(v) = settle_secs {
                entry.insert("settle_secs".into(), json!(v));
            }
            if headless {
                entry.insert("headless".into(), json!(true));
            }
            if let Some(v) = body {
                entry.insert("method".into(), json!("post"));
                entry.insert("body".into(), json!(v));
            }
            if let Some(v) = body_format {
                entry.insert("body_format".into(), json!(v));
            }
            if !headers.is_empty() {
                let mut map = Map::new();
                for h in &headers {
                    let (k, v) = h.split_once(':').with_context(|| format!("header {h:?} must look like `Name: value`"))?;
                    map.insert(k.trim().to_string(), json!(v.trim()));
                }
                entry.insert("headers".into(), Value::Object(map));
            }
            if let Some(v) = user_agent {
                entry.insert("user_agent".into(), json!(v));
            }
            if let Some(v) = daily_limit {
                entry.insert("daily_limit".into(), json!(v));
            }
            if let Some(v) = timeout {
                entry.insert("timeout".into(), json!(v));
            }
            let entry = Value::Object(entry);
            // Check the entry before asking anyone to acknowledge anything.
            pampas::provider::parse_config(&serde_json::to_string(&[&entry])?)?;
            anyhow::ensure!(
                acknowledge || (std::io::stdin().is_terminal() && std::io::stderr().is_terminal()),
                "adding a scrape source needs a terminal where you type \"{}\", or the --acknowledge flag, which says you have read the risks",
                pampas::scrape::ACK_PHRASE
            );
            eprintln!("{}\n", pampas::scrape::ADD_NOTICE);
            if browser {
                eprintln!("{}\n", pampas::scrape::ADD_BROWSER_NOTICE);
            }
            if let Some(warning) = site_warning {
                eprintln!("{warning}\n");
            }
            if acknowledge {
                eprintln!(
                    "Acknowledged with --acknowledge: by adding {name:?} you accept the risks above (a blocked or banned IP address or account, breaking the site's terms, a layout change breaking the source).\n"
                );
            } else {
                eprint!("Type \"{}\" to add {name:?} to {}: ", pampas::scrape::ACK_PHRASE, path.display());
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer).context("reading the answer")?;
                anyhow::ensure!(answer.trim() == pampas::scrape::ACK_PHRASE, "not added");
            }
            let cfg: pampas::scrape::ScrapeConfig = serde_json::from_value(entry.clone())?;
            pampas::scrape::add_entry(&path, entry)?;
            println!("added {name} to {}; ask it with `--scrape`, or tick scraping under More options in the app", path.display());
            if browser {
                if pampas::browser::has_profile(&cfg) {
                    println!("already logged in before; if it stops working, log in again with `pampas-cli scrape login {name}`");
                } else {
                    match first_run {
                        Some(what) => println!("a browser window opens now on {}: {what}", cfg.url),
                        None => println!("you need to log in to {} once: a browser window opens now; log in, then close the window", cfg.url),
                    }
                    pampas::browser::login(&cfg).await?;
                    println!("done; try `pampas-cli --scrape --only {name} \"hello\"`");
                }
            }
            Ok(())
        }
    }
}

/// Whether a typed answer means yes.
fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// The providers config path. When it is the default path, does not exist and there is a terminal to
/// ask on, offer to create it from the bundled example, so a first run does not just fail. A path given
/// with `--config` is never created: a typo should be reported, not turned into a new file.
fn config_path_or_offer(config: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = config {
        return Ok(p);
    }
    let path = default_config_path()?;
    if !path.exists() && std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        eprint!("{} does not exist. Create it from the bundled example? [y/N] ", path.display());
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).context("reading the answer")?;
        if is_yes(&answer) {
            write_example_and_explain(&path)?;
        }
    }
    Ok(path)
}

/// Write the bundled example config to `path` and say what to do next.
fn write_example_and_explain(path: &Path) -> anyhow::Result<()> {
    write_example_config(path)?;
    println!("wrote {}", path.display());
    println!("edit it: set CLOUDFLARE_ACCOUNT_ID (Cloudflare) and OLLAMA_REMOTE_HOST with `pampas-cli setting set <NAME> <value>` (or as environment variables), and remove providers you do not use");
    println!("then store keys with `pampas-cli key set <NAME>`; `pampas-cli key types` lists the names");
    Ok(())
}

/// Ask for a value on the terminal. The input is shown, since settings are not secrets.
fn prompt_value(name: &str) -> anyhow::Result<String> {
    anyhow::ensure!(
        std::io::stdin().is_terminal(),
        "no value given for {name}: pass it as an argument (`pampas-cli setting set {name} <value>`) or run this on a terminal to be asked"
    );
    eprint!("{name}: ");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).context("reading the value")?;
    Ok(line)
}

/// The key piped in on standard input: all of it, since a key is one line and may have a trailing newline.
fn read_key(mut input: impl std::io::Read) -> anyhow::Result<String> {
    let mut text = String::new();
    input.read_to_string(&mut text).context("reading the key from standard input")?;
    Ok(text)
}

fn run_key(action: KeyAction, config: Option<PathBuf>) -> anyhow::Result<()> {
    match action {
        KeyAction::Set { name, stdin } => {
            let key = if stdin { read_key(std::io::stdin().lock())? } else { rpassword::prompt_password(format!("{name}: "))? };
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
        KeyAction::Types => {
            let entries = match config {
                Some(p) => load_config(&p)?,
                None => {
                    let path = default_config_path()?;
                    if path.exists() {
                        load_config(&path)?
                    } else {
                        eprintln!("{} not found; showing the keys used by the bundled example config", path.display());
                        parse_config(engine::EXAMPLE_CONFIG)?
                    }
                }
            };
            let rows: Vec<Vec<String>> =
                key_types(&entries).into_iter().map(|(env, users)| vec![env, users.join(", ")]).collect();
            print_table(&["KEY", "PROVIDERS"], &rows);
        }
        KeyAction::List => {
            let path = config_path_or_offer(config)?;
            let rows: Vec<Vec<String>> = key_rows(&load_config(&path)?, &Keychain)
                .into_iter()
                .map(|r| {
                    let status = match r.status {
                        KeyStatus::NotNeeded => "no key needed".to_string(),
                        KeyStatus::Env => "environment variable".to_string(),
                        KeyStatus::Keychain => "OS keychain".to_string(),
                        KeyStatus::Missing => "missing".to_string(),
                        KeyStatus::Error(why) => format!("error: {why}"),
                    };
                    vec![r.provider, r.key.unwrap_or_else(|| "-".into()), status]
                })
                .collect();
            print_table(&["PROVIDER", "KEY", "SOURCE"], &rows);
        }
    }
    Ok(())
}

/// Rust ignores SIGPIPE, so printing into a pipe that has closed (`pampas-cli usage | head -1`) panics.
/// Restore the default so the process ends quietly, as other command-line tools do.
fn restore_sigpipe() {
    #[cfg(unix)]
    // SAFETY: setting a signal's disposition to the default is sound; nothing else touches it.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    restore_sigpipe();
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Key { action }) => return run_key(action, cli.config),
        Some(Command::Scrape { action }) => return run_scrape(action, cli.config).await,
        Some(Command::Setting { action }) => return run_setting(action, cli.config),
        Some(Command::Init) => return run_init(cli.config),
        Some(Command::Migrate) => return run_migrate(),
        Some(Command::Usage) => return run_usage(cli.config, cli.usage_file),
        None => {}
    }
    let prompt = cli.prompt.context("a prompt is required: pampas-cli \"your prompt\"")?;
    let path = config_path_or_offer(cli.config)?;
    let client = reqwest::Client::new();

    // A provider that is not usable (missing key, Apple model unavailable) is skipped with a warning
    // rather than aborting the run.
    let usage = load_usage(match cli.usage_file {
        Some(p) => p,
        None => default_usage_path()?,
    });
    let settings = Settings::load(&settings_path(&path))?;
    let entries = load_config(&path)?;
    let (enable, disable) = match &cli.only {
        Some(name) => pampas::engine::only_selection(&entries, name)?,
        None => (cli.enable.clone(), cli.disable.clone()),
    };
    let options = BuildOptions { enable: &enable, disable: &disable, ignore_limits: cli.ignore_limits, scrape: cli.scrape, headless: cli.headless };
    if cli.scrape {
        eprintln!("warning: {}", pampas::scrape::SCRAPE_WARNING);
    }
    let built = build_providers(entries, &settings, &Keychain, &usage, &client, &options)?;
    for warning in &built.warnings {
        eprintln!("{warning}");
    }
    anyhow::ensure!(!built.providers.is_empty(), "no usable providers in {}", path.display());

    let entries = entries_from(fan_out(&built.providers, &prompt, cli.timeout).await);
    for e in &entries {
        if let Err(msg) = &e.outcome {
            eprintln!("## {} failed: {msg}", e.provider);
        }
    }

    let mut report =
        output::Report { prompt, entries, merged: None, merge_error: None, show_answers: cli.show_answers };
    if cli.merge {
        let choice = Merge { designated: &built.designated, requested: cli.synthesizer.as_deref(), timeout: cli.timeout };
        for notice in merge_answers(&mut report, &built.providers, choice).await? {
            eprintln!("{notice}");
        }
    }
    emit(&report, cli.format, cli.output.as_deref())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_may_come_before_or_after_a_subcommand() {
        for args in [
            vec!["pampas-cli", "--config", "c.json", "key", "list"],
            vec!["pampas-cli", "key", "list", "--config", "c.json"],
            vec!["pampas-cli", "--usage-file", "u.json", "--config", "c.json", "usage"],
        ] {
            let cli = Cli::try_parse_from(&args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
            assert!(cli.command.is_some() && cli.prompt.is_none(), "{args:?}");
            assert_eq!(cli.config, Some(PathBuf::from("c.json")).filter(|_| args.contains(&"--config")));
        }
        let cli = Cli::try_parse_from(["pampas-cli", "--config", "c.json", "hello there"]).unwrap();
        assert_eq!(cli.prompt.as_deref(), Some("hello there"));
    }

    #[test]
    fn only_y_and_yes_mean_yes() {
        assert!(is_yes("y\n") && is_yes(" YES ") && is_yes("Yes"));
        assert!(!is_yes("") && !is_yes("\n") && !is_yes("n") && !is_yes("yep"));
    }

    #[test]
    fn read_key_takes_piped_input() {
        assert_eq!(read_key("sk-abc\n".as_bytes()).unwrap().trim(), "sk-abc");
    }

    #[test]
    fn table_aligns_when_asked_and_is_tab_separated_otherwise() {
        let rows = vec![vec!["a".to_string(), "x".to_string()], vec!["long".to_string(), "y".to_string()]];
        assert_eq!(format_table(&["NAME", "V"], &rows, true), "NAME  V\na     x\nlong  y\n");
        assert_eq!(format_table(&["NAME", "V"], &rows, false), "a\tx\nlong\ty\n");
    }
}
