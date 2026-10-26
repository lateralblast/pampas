//! A provider for a web site that has no API: send the prompt in an ordinary HTTP request and pull the
//! answer out of the page (a CSS selector) or out of a JSON reply (a JSON pointer). Only runs with
//! `--scrape`, because automated requests to a site can get the caller's IP address blocked or banned and
//! may break the site's terms.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, anyhow};
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::provider::Provider;
use crate::usage::SharedUsage;

/// Printed (and shown in the app) whenever scraping is switched on.
pub const SCRAPE_WARNING: &str = "scraping is on: automated requests to web sites can get your IP address blocked or banned, and may break a site's terms of use";

/// What must be typed to add a scrape source from the command line.
pub const ACK_PHRASE: &str = "I understand";

/// Printed before a scrape source is added, so adding one is a deliberate act.
pub const ADD_NOTICE: &str = "Scraping a web site with no API is at your own risk.\n\
- The site can block or ban your IP address (and sometimes the network in front of it).\n\
- It may break the site's terms of use.\n\
- The page layout can change at any time and break the source.\n\
- pampas does not hide itself or get around blocks, logins or captchas.";

/// Extra notice for a browser source, which uses your own logged-in account.
pub const ADD_BROWSER_NOTICE: &str = "This source drives a real Chrome window with a login of yours, which is kept in a pampas folder.\n\
- The account, not only the IP address, can be flagged, limited or suspended for automated use.\n\
- A browser window opens for each question unless you chose --headless (which many sites block).\n\
- You will need to log in to the site yourself, once: after you add it, a browser window opens for that.";

/// What is known about a site that can be added by name alone (`pampas-cli scrape add deepseek`).
pub struct Preset {
    pub name: &'static str,
    pub url: &'static str,
    pub input_selector: &'static str,
    pub selector: &'static str,
    /// The send button, for a site where Enter does not send. `None` presses Enter.
    pub submit_selector: Option<&'static str>,
    /// Something visible only while the answer is being written, if one is known.
    pub busy_selector: Option<&'static str>,
    /// An iframe the answer is inside, if it is not in the page itself.
    pub answer_frame: Option<&'static str>,
    pub settle_secs: u64,
    pub daily_limit: u32,
    /// When the selectors were last seen to work.
    pub checked: &'static str,
    /// Shown with the risks before the user acknowledges them, for what is particular to this site.
    pub warning: Option<&'static str>,
    /// What to do in the window that opens the first time, when it is not simply logging in.
    pub first_run: Option<&'static str>,
}

/// The built-in sites. Every one is a browser source and still needs the acknowledgement and a login.
pub const PRESETS: &[Preset] = &[Preset {
    name: "deepseek",
    url: "https://chat.deepseek.com/",
    input_selector: "textarea[placeholder=\"Message DeepSeek\"]",
    selector: ".ds-markdown.ds-assistant-message-main-content",
    submit_selector: None,
    busy_selector: None,
    answer_frame: None,
    settle_secs: 6,
    daily_limit: 20,
    checked: "selectors checked 2026-10-23",
    warning: None,
    first_run: None,
},
Preset {
    name: "chatgpt",
    url: "https://chatgpt.com/",
    input_selector: "div[contenteditable=\"true\"][aria-label=\"Ask ChatGPT\"]",
    selector: "[data-markdown-text-style=\"assistant-message\"]",
    submit_selector: None,
    busy_selector: Some("[data-talvt-turn-state]:not([data-talvt-turn-state=\"complete\"])"),
    answer_frame: None,
    settle_secs: 5,
    daily_limit: 20,
    checked: "selectors checked 2026-10-23; the busy marker was not seen while an answer was being written",
    warning: Some("OpenAI's terms of use prohibit automated use of ChatGPT, and it checks for automation more strictly than most sites: your ChatGPT account can be limited or banned, and a free account has its own message limits."),
    first_run: None,
},
Preset {
    name: "copilot",
    url: "https://m365.cloud.microsoft/chat",
    input_selector: "#m365-chat-editor-target-element",
    selector: "[data-testid=\"markdown-reply\"]",
    submit_selector: Some("button[type=\"submit\"][aria-label=\"Send\"]"),
    busy_selector: None,
    answer_frame: None,
    settle_secs: 6,
    daily_limit: 20,
    checked: "selectors checked 2026-10-23 on a Microsoft 365 account",
    warning: Some("This uses a Microsoft 365 account. If it is a work or school account, your organisation can log and review the chats, and its policies may refuse sign-in from the separate pampas browser profile or ask for extra verification. Microsoft's terms apply to automated use too."),
    first_run: None,
},
Preset {
    name: "duckduckgo",
    url: "https://duck.ai/chat",
    input_selector: "textarea[name=\"user-prompt\"]",
    selector: "[id*=\"-assistant-message-\"] .space-y-4",
    submit_selector: None,
    busy_selector: Some("button[aria-label=\"Stop generating\"]:not([disabled])"),
    answer_frame: None,
    settle_secs: 4,
    daily_limit: 20,
    checked: "selectors checked 2026-10-23; the busy marker was not seen while an answer was being written",
    warning: Some("DuckDuckGo's AI chat needs no account, but it is rate limited per address and may ask for a check if it sees automated use, and its terms apply to automated use too."),
    first_run: Some("no login is needed: the first time, DuckDuckGo asks you to click Continue to accept its terms of service. Click it, then close the window."),
},
Preset {
    name: "deepai",
    url: "https://deepai.org/chat",
    input_selector: "textarea#persistentChatbox",
    selector: ".outputBox .markdownContainer",
    submit_selector: Some("#chatSubmitButton"),
    busy_selector: Some("#chatStopButton[style*=\"display: block\"]"),
    answer_frame: None,
    settle_secs: 5,
    daily_limit: 20,
    checked: "selectors read from the page's HTML and script on 2026-10-25, not yet tried in a browser",
    warning: Some("DeepAI's chat works without an account but limits free use, and some models need DeepAI Pro. Its terms apply to automated use too, and it may ask for a check or limit your address if it sees automated use."),
    first_run: Some("no login is needed: accept or dismiss any banner, make sure you can see the \"Message AI Chat...\" box, then close the window."),
},
Preset {
    name: "google-ai-mode",
    url: "https://www.google.com/search?udm=50",
    input_selector: "textarea[aria-label=\"Ask anything\"]",
    selector: "[data-container-id=\"main-col\"]",
    submit_selector: None,
    busy_selector: Some("button[aria-label=\"Stop\"]"),
    answer_frame: None,
    settle_secs: 5,
    daily_limit: 20,
    checked: "selectors checked 2026-10-23 on a shared AI Mode page; the live page and the busy marker were not seen while an answer was being written",
    warning: Some("Google detects automated use more strictly than most sites: it may show an \"unusual traffic\" page or a CAPTCHA (pampas will not solve it), block your IP address from Google Search for a while, or flag the account you are signed in with. Its terms prohibit automated queries."),
    first_run: Some("Google may show a consent page, ask you to sign in, or not offer AI Mode to a signed-out browser in your region. Do what you want for that, as in any browser (signing in is optional if AI Mode works without), make sure you can see the \"Ask anything\" box, then close the window."),
},
Preset {
    name: "bing",
    url: "https://www.bing.com/copilotsearch",
    input_selector: "textarea[id^=\"b_copilot_composer\"]",
    selector: ".gs_temp_ra",
    submit_selector: None,
    busy_selector: None,
    answer_frame: Some(".frame_cont iframe"),
    settle_secs: 5,
    daily_limit: 20,
    checked: "selectors checked 2026-10-23 on one answer; the busy marker and a signed-out browser were not checked",
    warning: Some("Bing may show a verification page for automated use (pampas will not solve it), and Microsoft's terms apply to automated queries. If you sign in, the question is recorded in that Microsoft account's activity."),
    first_run: Some("a Microsoft sign-in is optional if AI Mode works without it in your region; accept any consent page, make sure you can see the \"Ask anything\" box, then close the window."),
}];

/// The built-in setup for `name`, if there is one.
pub fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// Keys of a scrape entry that the app may change; everything else (the address, the request, the
/// selector, the name) can only be set from the command line.
const APP_EDITABLE: [&str; 5] = ["enabled", "synthesizer", "timeout", "daily_limit", "usage_group"];

fn is_scrape(entry: &serde_json::Value) -> bool {
    entry["kind"].as_str() == Some("scrape")
}

/// Refuse a list of entries from the app that adds a scrape source or changes how one makes its request.
/// `old` is the list now on disk. Removing a scrape source, and changing only [`APP_EDITABLE`] fields, is fine.
pub fn check_app_edit(old: &[serde_json::Value], new: &[serde_json::Value]) -> anyhow::Result<()> {
    let fixed = |e: &serde_json::Value| {
        let mut e = e.clone();
        if let Some(map) = e.as_object_mut() {
            for k in APP_EDITABLE {
                map.remove(k);
            }
        }
        e
    };
    for entry in new.iter().filter(|e| is_scrape(e)) {
        let name = entry["name"].as_str().unwrap_or_default();
        let known = old.iter().find(|o| is_scrape(o) && o["name"].as_str() == Some(name));
        match known {
            None => {
                return Err(anyhow!(
                    "{name:?}: scrape sources can only be added from the command line (`pampas-cli scrape add`), where the risks are acknowledged"
                ));
            }
            Some(o) if fixed(o) != fixed(entry) => {
                return Err(anyhow!(
                    "{name:?}: a scrape source's address and request can only be changed from the command line (`pampas-cli scrape add`, after removing it here)"
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// Append `entry` to the providers config at `path` (a JSON array, created if missing), keeping the
/// existing entries exactly as written. The entry must be a valid scrape entry with an unused name.
pub fn add_entry(path: &std::path::Path, entry: serde_json::Value) -> anyhow::Result<()> {
    anyhow::ensure!(is_scrape(&entry), "not a scrape entry");
    let mut entries: Vec<serde_json::Value> = match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let name = entry["name"].as_str().unwrap_or_default().to_string();
    anyhow::ensure!(!name.is_empty(), "the source needs a name");
    anyhow::ensure!(
        !entries.iter().any(|e| e["name"].as_str() == Some(name.as_str())),
        "{} already has a provider named {name:?}",
        path.display()
    );
    entries.push(entry);
    crate::provider::parse_config(&serde_json::to_string(&entries)?)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    if path.exists() {
        let backup = path.with_extension("json.bak");
        std::fs::copy(path, &backup).with_context(|| format!("backing up to {}", backup.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&entries)? + "\n")?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// Remove the scrape source called `name` from the providers config at `path`, keeping the other entries
/// exactly as written and the old file as `providers.json.bak`. Only scrape sources can be removed this way.
pub fn remove_entry(path: &std::path::Path, name: &str) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut entries: Vec<serde_json::Value> =
        serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    let at = entries
        .iter()
        .position(|e| e["name"].as_str() == Some(name))
        .with_context(|| format!("{} has no provider named {name:?}", path.display()))?;
    anyhow::ensure!(is_scrape(&entries[at]), "{name:?} is not a scrape source; remove other sources in the app or by editing the file");
    entries.remove(at);
    let backup = path.with_extension("json.bak");
    std::fs::copy(path, &backup).with_context(|| format!("backing up to {}", backup.display()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&entries)? + "\n")?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// One config entry with `"kind": "scrape"`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrapeConfig {
    pub name: String,
    #[serde(default = "scrape_kind")]
    #[allow(dead_code)]
    kind: String,
    /// Address to request. `{prompt}` is replaced by the prompt, percent-encoded.
    pub url: String,
    /// `get` (default) or `post`.
    #[serde(default)]
    pub method: Method,
    /// Request body for `post`. `{prompt}` is replaced by the prompt, encoded to suit `body_format`.
    #[serde(default)]
    pub body: Option<String>,
    /// `form` (default, `application/x-www-form-urlencoded`) or `json`.
    #[serde(default)]
    pub body_format: BodyFormat,
    /// Extra request headers, such as `Cookie` or `Accept-Language`. `{prompt}` is not replaced here.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Value of the `User-Agent` header; default `pampas`.
    #[serde(default)]
    pub user_agent: Option<String>,
    /// CSS selector for the elements whose text is the answer (an HTML reply).
    #[serde(default)]
    pub selector: Option<String>,
    /// JSON pointer (such as `/data/0/text`) to the answer in a JSON reply.
    #[serde(default)]
    pub json_pointer: Option<String>,
    /// `http` (default: one plain request) or `browser` (drive a real Chrome, for pages that need JavaScript
    /// or a login; then `selector` is the answer's selector and `input_selector` is required).
    #[serde(default)]
    pub render: Render,
    /// Browser only: CSS selector of the box the prompt is typed into.
    #[serde(default)]
    pub input_selector: Option<String>,
    /// Browser only: CSS selector of the send button; without it Enter is pressed.
    #[serde(default)]
    pub submit_selector: Option<String>,
    /// Browser only: CSS selector of something visible only while the answer is still being written (such as a stop button).
    #[serde(default)]
    pub busy_selector: Option<String>,
    /// Browser only: CSS selector of an iframe (same site as the page) that holds the answer; `selector`
    /// and `busy_selector` are then looked up inside the last matching frame instead of the page.
    #[serde(default)]
    pub answer_frame: Option<String>,
    /// Browser only: seconds the answer must stay unchanged to count as finished (default 4).
    #[serde(default)]
    pub settle_secs: Option<u64>,
    /// Browser only: run with no window. Many sites block this; the default is a visible window.
    #[serde(default)]
    pub headless: bool,
    /// Browser only: path to Chrome or Chromium, if it is not found on its own.
    #[serde(default)]
    pub browser_path: Option<String>,
    /// Browser only: name of the profile folder holding the login; default the source's name.
    #[serde(default)]
    pub profile: Option<String>,
    /// Set to false to leave the provider out of runs unless `--enable <name>` is given.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// With `--merge`, prefer this provider to write the consolidated answer.
    #[serde(default)]
    pub synthesizer: bool,
    /// Most requests to make per UTC day.
    #[serde(default)]
    pub daily_limit: Option<u32>,
    /// Seconds to wait for this provider, instead of the default 30.
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Name under which requests are counted, default the provider's own name.
    #[serde(default)]
    pub usage_group: Option<String>,
}

fn scrape_kind() -> String {
    "scrape".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    #[default]
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Render {
    #[default]
    Http,
    Browser,
}

/// Seconds a browser source is given when its entry and `--timeout` say nothing: starting Chrome,
/// loading the page and waiting for a long answer take much longer than one request.
const BROWSER_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyFormat {
    #[default]
    Form,
    Json,
}

impl ScrapeConfig {
    /// Reject an entry that could never work, so it fails when the config is loaded or saved.
    pub fn check(&self) -> anyhow::Result<()> {
        let browser_only = [
            ("input_selector", self.input_selector.is_some()),
            ("submit_selector", self.submit_selector.is_some()),
            ("busy_selector", self.busy_selector.is_some()),
            ("answer_frame", self.answer_frame.is_some()),
            ("settle_secs", self.settle_secs.is_some()),
            ("headless", self.headless),
            ("browser_path", self.browser_path.is_some()),
            ("profile", self.profile.is_some()),
        ];
        if self.render == Render::Browser {
            anyhow::ensure!(self.input_selector.is_some(), "{}: a browser source needs `input_selector`", self.name);
            anyhow::ensure!(self.selector.is_some(), "{}: a browser source needs `selector` (the answer)", self.name);
            anyhow::ensure!(self.json_pointer.is_none(), "{}: a browser source reads the page, not JSON", self.name);
            anyhow::ensure!(
                self.method == Method::Get && self.body.is_none(),
                "{}: a browser source loads `url` and types the prompt; it takes no `method` or `body`",
                self.name
            );
            for (field, value) in [("input_selector", &self.input_selector), ("submit_selector", &self.submit_selector), ("busy_selector", &self.busy_selector), ("answer_frame", &self.answer_frame)] {
                if let Some(sel) = value {
                    Selector::parse(sel).map_err(|e| anyhow!("{}: invalid {field} {sel:?}: {e}", self.name))?;
                }
            }
        } else if let Some((field, _)) = browser_only.iter().find(|(_, set)| *set) {
            return Err(anyhow!("{}: `{field}` is only for `\"render\": \"browser\"`", self.name));
        }
        match (&self.selector, &self.json_pointer) {
            (Some(_), Some(_)) => return Err(anyhow!("{}: give `selector` or `json_pointer`, not both", self.name)),
            (None, None) => {
                return Err(anyhow!("{}: a scrape entry needs `selector` (HTML) or `json_pointer` (JSON)", self.name));
            }
            (Some(sel), None) => {
                Selector::parse(sel).map_err(|e| anyhow!("{}: invalid selector {sel:?}: {e}", self.name))?;
            }
            _ => {}
        }
        if self.method == Method::Post && self.body.is_none() {
            return Err(anyhow!("{}: `method` post needs a `body`", self.name));
        }
        Ok(())
    }

    /// With `on`, a browser source runs without a window whatever its entry says (a per-run switch);
    /// without it, and for a source that is not a browser source, the entry is unchanged.
    pub fn with_headless(mut self, on: bool) -> Self {
        if on && self.render == Render::Browser {
            self.headless = true;
        }
        self
    }

    /// The entry's own timeout, or the longer default for a browser source.
    fn timeout_for(&self) -> Option<u64> {
        self.timeout.or((self.render == Render::Browser).then_some(BROWSER_TIMEOUT_SECS))
    }

    pub fn usage_group(&self) -> &str {
        self.usage_group.as_deref().unwrap_or(&self.name)
    }
}

/// Percent-encode `text` for a URL or a form body.
fn percent_encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `prompt` as the inside of a JSON string.
fn json_escape(text: &str) -> String {
    let quoted = serde_json::to_string(text).expect("a string always serializes");
    quoted[1..quoted.len() - 1].to_string()
}

/// The text of every element `selector` matches in `html`, one paragraph each, whitespace tidied.
pub fn extract_html(html: &str, selector: &str) -> anyhow::Result<String> {
    let selector = Selector::parse(selector).map_err(|e| anyhow!("invalid selector {selector:?}: {e}"))?;
    let doc = Html::parse_document(html);
    let parts: Vec<String> = doc
        .select(&selector)
        .map(|el| el.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(anyhow!(
            "the selector matched no text: the page layout may have changed, or the site returned a block or login page instead of an answer"
        ));
    }
    Ok(parts.join("\n\n"))
}

/// The string at `pointer` in a JSON reply.
pub fn extract_json(body: &str, pointer: &str) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::from_str(body).context("the reply was not JSON")?;
    match value.pointer(pointer) {
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        Some(serde_json::Value::String(_)) | None => Err(anyhow!("nothing at `{pointer}` in the reply")),
        Some(other) => Ok(other.to_string()),
    }
}

/// A web site queried and scraped; see the module comment.
pub struct Scrape {
    config: ScrapeConfig,
    client: reqwest::Client,
    usage: SharedUsage,
    enforce_limits: bool,
}

impl Scrape {
    pub fn new(config: ScrapeConfig, client: reqwest::Client, usage: SharedUsage, enforce_limits: bool) -> Self {
        Self { config, client, usage, enforce_limits }
    }

    fn request(&self, prompt: &str) -> reqwest::RequestBuilder {
        let c = &self.config;
        let url = c.url.replace("{prompt}", &percent_encode(prompt));
        let mut req = match c.method {
            Method::Get => self.client.get(url),
            Method::Post => {
                let (body, content_type) = match c.body_format {
                    BodyFormat::Form => (
                        c.body.as_deref().unwrap_or("").replace("{prompt}", &percent_encode(prompt)),
                        "application/x-www-form-urlencoded",
                    ),
                    BodyFormat::Json => {
                        (c.body.as_deref().unwrap_or("").replace("{prompt}", &json_escape(prompt)), "application/json")
                    }
                };
                self.client.post(url).header("Content-Type", content_type).body(body)
            }
        };
        req = req.header("User-Agent", c.user_agent.as_deref().unwrap_or("pampas"));
        for (name, value) in &c.headers {
            req = req.header(name, value);
        }
        req
    }
}

impl Provider for Scrape {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn timeout(&self) -> Option<u64> {
        ScrapeConfig::timeout_for(&self.config)
    }

    fn ask<'a>(&'a self, prompt: &'a str) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let c = &self.config;
            self.usage.lock().expect("usage lock").acquire(c.usage_group(), c.daily_limit, self.enforce_limits)?;
            if c.render == Render::Browser {
                return crate::browser::ask(c, prompt).await;
            }
            let resp = self.request(prompt).send().await?;
            let status = resp.status();
            let text = resp.text().await?;
            if !status.is_success() {
                let hint = if matches!(status.as_u16(), 403 | 429 | 503) {
                    " (the site may be rate limiting or blocking this address: stop asking it for a while)"
                } else {
                    ""
                };
                return Err(anyhow!("HTTP {status}{hint}"));
            }
            match (&c.selector, &c.json_pointer) {
                (Some(sel), _) => extract_html(&text, sel),
                (None, Some(ptr)) => extract_json(&text, ptr),
                (None, None) => Err(anyhow!("no `selector` or `json_pointer`")),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> anyhow::Result<ScrapeConfig> {
        let c: ScrapeConfig = serde_json::from_str(json)?;
        c.check()?;
        Ok(c)
    }

    #[test]
    fn encodes_prompt_for_urls_and_json() {
        assert_eq!(percent_encode("a b&c=é"), "a%20b%26c%3D%C3%A9");
        assert_eq!(json_escape("say \"hi\"\nnow"), "say \\\"hi\\\"\\nnow");
    }

    #[test]
    fn extracts_text_from_html() {
        let html = "<html><body><div class=a>  One\n <b>two</b></div><p>skip</p><div class=a>Three</div></body></html>";
        assert_eq!(extract_html(html, "div.a").unwrap(), "One two\n\nThree");
        assert!(extract_html(html, "div.none").unwrap_err().to_string().contains("matched no text"));
    }

    #[test]
    fn extracts_from_json() {
        let body = r#"{"data":[{"text":" hello "}],"n":3}"#;
        assert_eq!(extract_json(body, "/data/0/text").unwrap(), "hello");
        assert_eq!(extract_json(body, "/n").unwrap(), "3");
        assert!(extract_json(body, "/nope").is_err());
        assert!(extract_json("<html>", "/x").is_err());
    }

    #[test]
    fn remove_entry_takes_out_only_a_named_scrape_source() {
        use serde_json::json;
        let dir = std::env::temp_dir().join(format!("pampas-scrape-rm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("providers.json");
        std::fs::write(&path, r#"[{"name":"o","base_url":"u","model":"m"}]"#).unwrap();
        add_entry(&path, json!({"name":"s","kind":"scrape","url":"https://x/?q={prompt}","selector":"p"})).unwrap();
        assert!(remove_entry(&path, "o").unwrap_err().to_string().contains("not a scrape source"));
        assert!(remove_entry(&path, "nope").unwrap_err().to_string().contains("no provider named"));
        remove_entry(&path, "s").unwrap();
        let left: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0]["name"], "o");
        assert!(path.with_extension("json.bak").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_app_cannot_add_or_rewrite_scrape_sources() {
        use serde_json::json;
        let site = json!({"name":"s","kind":"scrape","url":"https://x/?q={prompt}","selector":"p","daily_limit":5});
        let openai = json!({"name":"o","base_url":"u","model":"m"});
        // Adding one is refused; so is adding one beside an existing source of the same kind.
        assert!(check_app_edit(&[openai.clone()], &[openai.clone(), site.clone()]).is_err());
        // Unchanged, removed, or with only the allowed fields changed, is fine.
        assert!(check_app_edit(&[site.clone()], &[site.clone()]).is_ok());
        assert!(check_app_edit(&[site.clone()], &[]).is_ok());
        let tweaked = json!({"name":"s","kind":"scrape","url":"https://x/?q={prompt}","selector":"p","enabled":false,"timeout":60});
        assert!(check_app_edit(&[site.clone()], &[tweaked]).is_ok());
        // A different address or selector, or a rename, is refused.
        let moved = json!({"name":"s","kind":"scrape","url":"https://evil/?q={prompt}","selector":"p","daily_limit":5});
        assert!(check_app_edit(&[site.clone()], &[moved]).is_err());
        let renamed = json!({"name":"t","kind":"scrape","url":"https://x/?q={prompt}","selector":"p","daily_limit":5});
        assert!(check_app_edit(&[site], &[renamed]).is_err());
    }

    #[test]
    fn add_entry_appends_validates_and_keeps_a_backup() {
        use serde_json::json;
        let dir = std::env::temp_dir().join(format!("pampas-scrape-add-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("providers.json");
        std::fs::write(&path, r#"[{"name":"o","base_url":"u","model":"m"}]"#).unwrap();
        let site = json!({"name":"s","kind":"scrape","url":"https://x/?q={prompt}","selector":"p"});
        add_entry(&path, site.clone()).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"o\"") && text.contains("\"s\""));
        assert!(path.with_extension("json.bak").exists());
        assert!(add_entry(&path, site).unwrap_err().to_string().contains("already has"));
        let bad = json!({"name":"b","kind":"scrape","url":"u"});
        assert!(add_entry(&path, bad).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "a refused entry leaves the file alone");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn headless_can_be_forced_for_a_run_on_browser_sources_only() {
        let browser = r#"{"name":"b","render":"browser","url":"u","input_selector":"t","selector":".a"}"#;
        let plain = r#"{"name":"p","url":"u","selector":"p"}"#;
        let cfg = |json: &str| serde_json::from_str::<ScrapeConfig>(json).unwrap();
        assert!(!cfg(browser).headless);
        assert!(!cfg(browser).with_headless(false).headless);
        assert!(cfg(browser).with_headless(true).headless);
        // A plain request has no window to hide, and forcing it would make its entry invalid.
        let forced = cfg(plain).with_headless(true);
        assert!(!forced.headless);
        forced.check().unwrap();
    }

    #[test]
    fn browser_entries_are_checked() {
        let ok = r#"{"name":"b","render":"browser","url":"https://x/","input_selector":"textarea","selector":".a"}"#;
        assert!(parse(ok).is_ok());
        let timeout = |json: &str| ScrapeConfig::timeout_for(&serde_json::from_str(json).unwrap());
        assert_eq!(timeout(ok), Some(120));
        assert_eq!(timeout(r#"{"name":"b","url":"u","selector":"p"}"#), None);
        assert!(parse(r#"{"name":"b","render":"browser","url":"u","selector":".a"}"#).unwrap_err().to_string().contains("input_selector"));
        assert!(parse(r#"{"name":"b","render":"browser","url":"u","input_selector":"t"}"#).is_err());
        assert!(parse(r#"{"name":"b","render":"browser","url":"u","input_selector":"t","selector":".a","method":"post","body":"x"}"#).is_err());
        assert!(parse(r#"{"name":"b","render":"browser","url":"u","input_selector":"t","selector":"a[","busy_selector":"x"}"#).is_err());
        // Browser-only fields are refused on a plain request.
        assert!(parse(r#"{"name":"b","url":"u","selector":"p","headless":true}"#).unwrap_err().to_string().contains("only for"));
    }

    #[test]
    fn presets_make_valid_browser_entries() {
        assert!(preset("nope").is_none());
        for p in PRESETS {
            let json = serde_json::json!({
                "name": p.name, "kind": "scrape", "render": "browser", "url": p.url,
                "input_selector": p.input_selector, "selector": p.selector,
                "settle_secs": p.settle_secs, "daily_limit": p.daily_limit,
                "busy_selector": p.busy_selector, "answer_frame": p.answer_frame,
                "submit_selector": p.submit_selector,
            });
            let cfg: ScrapeConfig = serde_json::from_value(json).unwrap();
            cfg.check().unwrap();
        }
        assert_eq!(preset("deepseek").unwrap().url, "https://chat.deepseek.com/");
        assert_eq!(preset("chatgpt").unwrap().url, "https://chatgpt.com/");
        assert_eq!(preset("copilot").unwrap().url, "https://m365.cloud.microsoft/chat");
        assert_eq!(preset("duckduckgo").unwrap().url, "https://duck.ai/chat");
        assert_eq!(preset("deepai").unwrap().url, "https://deepai.org/chat");
        assert!(preset("deepai").unwrap().submit_selector.is_some());
        assert!(preset("google-ai-mode").unwrap().url.contains("udm=50"));
        assert!(preset("copilot").unwrap().submit_selector.is_some());
        assert!(preset("deepseek").unwrap().submit_selector.is_none());
        assert_eq!(preset("bing").unwrap().answer_frame, Some(".frame_cont iframe"));
    }

    #[test]
    fn entries_are_checked() {
        assert!(parse(r#"{"name":"a","kind":"scrape","url":"https://x/?q={prompt}","selector":"p"}"#).is_ok());
        assert!(parse(r#"{"name":"a","url":"u"}"#).unwrap_err().to_string().contains("needs `selector`"));
        assert!(parse(r#"{"name":"a","url":"u","selector":"p","json_pointer":"/a"}"#).is_err());
        assert!(parse(r#"{"name":"a","url":"u","selector":"p[","method":"get"}"#).is_err());
        assert!(parse(r#"{"name":"a","url":"u","selector":"p","method":"post"}"#).unwrap_err().to_string().contains("needs a `body`"));
        assert!(parse(r#"{"name":"a","url":"u","selector":"p","bogus":1}"#).is_err());
    }
}
