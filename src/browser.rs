//! Drive a real Chrome (or Chromium) through its DevTools protocol, for a web site whose answer only
//! appears in a page that runs JavaScript, behind a login. Used by scrape sources with `"render": "browser"`.
//!
//! One question opens the browser on the source's own profile folder (so a login made with
//! `pampas-cli scrape login <name>` is remembered), loads the page, types the prompt into the input box,
//! submits it, waits for the answer to stop changing, reads it and closes the browser. Nothing here hides
//! automation from the site or gets around a login, a captcha or a block.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::paths;
use crate::scrape::ScrapeConfig;

/// Seconds the input box is waited for after the page loads.
const INPUT_WAIT_SECS: u64 = 30;

/// Where this source's browser profile (cookies, login) is kept.
pub fn profile_dir(cfg: &ScrapeConfig) -> anyhow::Result<PathBuf> {
    let name = cfg.profile.as_deref().unwrap_or(&cfg.name);
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    Ok(paths::state_dir(paths::Os::current(), &paths::real_env)?.join("browser").join(safe))
}

/// Whether the source's profile has been used before (a login window was opened on it), so a login is
/// probably done. It cannot tell an expired login from a good one.
pub fn has_profile(cfg: &ScrapeConfig) -> bool {
    profile_dir(cfg).map(|dir| dir.join("Default").is_dir()).unwrap_or(false)
}

/// The browser to run: `browser_path` if set, else the first of the usual names or install paths that exists.
pub fn find_browser(cfg: &ScrapeConfig) -> anyhow::Result<PathBuf> {
    if let Some(path) = &cfg.browser_path {
        return Ok(PathBuf::from(path));
    }
    let on_path = ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"];
    let installed = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];
    for path in installed.iter().map(Path::new).filter(|p| p.exists()) {
        return Ok(path.to_path_buf());
    }
    let dirs = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&dirs) {
        for name in on_path {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(anyhow!("no Chrome or Chromium found: install one, or set `browser_path` on the source"))
}

/// The browser command. `automated` opens the DevTools port pampas drives it through; a login by hand
/// does not need it, and sites such as Google refuse to sign in to a browser that has it open.
fn command(cfg: &ScrapeConfig, headless: bool, automated: bool) -> anyhow::Result<tokio::process::Command> {
    let profile = profile_dir(cfg)?;
    std::fs::create_dir_all(&profile).with_context(|| format!("creating {}", profile.display()))?;
    let mut cmd = tokio::process::Command::new(find_browser(cfg)?);
    if automated {
        cmd.arg("--remote-debugging-port=0");
    }
    cmd.arg(format!("--user-data-dir={}", profile.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // The per-provider timeout drops the future: the browser must not outlive it.
        .kill_on_drop(true);
    if headless {
        cmd.arg("--headless=new");
    }
    cmd.arg(&cfg.url);
    Ok(cmd)
}

/// Open the source's page in a normal window on its profile and wait until the window is closed, so the
/// user can log in (or solve a check) once by hand. It is an ordinary window with no automation port.
pub async fn login(cfg: &ScrapeConfig) -> anyhow::Result<()> {
    let mut child = command(cfg, false, false)?.spawn().context("starting the browser")?;
    child.wait().await?;
    Ok(())
}

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A connection to one page of a running browser.
struct Page {
    ws: Socket,
    next_id: u64,
}

impl Page {
    async fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.ws.send(Message::text(json!({"id": id, "method": method, "params": params}).to_string())).await?;
        while let Some(msg) = self.ws.next().await {
            let Message::Text(text) = msg? else { continue };
            let reply: Value = serde_json::from_str(text.as_str())?;
            if reply["id"].as_u64() == Some(id) {
                if let Some(err) = reply.get("error") {
                    return Err(anyhow!("browser: {method}: {}", err["message"].as_str().unwrap_or("failed")));
                }
                return Ok(reply["result"].clone());
            }
        }
        Err(anyhow!("the browser closed the connection"))
    }

    /// Evaluate `expression` in the page and return its value.
    async fn eval(&mut self, expression: &str) -> anyhow::Result<Value> {
        let r = self
            .call("Runtime.evaluate", json!({"expression": expression, "returnByValue": true, "awaitPromise": true}))
            .await?;
        if let Some(ex) = r.get("exceptionDetails") {
            let text = ex["exception"]["description"].as_str().or(ex["text"].as_str()).unwrap_or("script error");
            return Err(anyhow!("page script: {text}"));
        }
        Ok(r["result"]["value"].clone())
    }

    /// Click at a point of the page with mouse events, as a person would.
    async fn click(&mut self, x: f64, y: f64) -> anyhow::Result<()> {
        self.call("Input.dispatchMouseEvent", json!({"type": "mouseMoved", "x": x, "y": y})).await?;
        for kind in ["mousePressed", "mouseReleased"] {
            self.call("Input.dispatchMouseEvent", json!({"type": kind, "x": x, "y": y, "button": "left", "buttons": 1, "clickCount": 1})).await?;
        }
        Ok(())
    }

    /// Poll `expression` every 300 ms until it is truthy, or fail after `secs`.
    async fn wait_for(&mut self, expression: &str, secs: u64) -> anyhow::Result<bool> {
        let end = Instant::now() + Duration::from_secs(secs);
        loop {
            if self.eval(expression).await?.as_bool() == Some(true) {
                return Ok(true);
            }
            if Instant::now() >= end {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
}

/// Print a progress line to stderr when `PAMPAS_BROWSER_DEBUG` is set, to see where a source gets stuck.
fn trace(step: &str) {
    if std::env::var_os("PAMPAS_BROWSER_DEBUG").is_some() {
        eprintln!("browser: {step}");
    }
}

/// `text` as a JavaScript string literal.
fn js(text: &str) -> String {
    serde_json::to_string(text).expect("a string always serializes")
}

/// Read the debugging port Chrome wrote into the profile once it started.
async fn debugging_port(profile: &Path) -> anyhow::Result<u16> {
    let file = profile.join("DevToolsActivePort");
    for _ in 0..100 {
        if let Ok(text) = std::fs::read_to_string(&file)
            && let Some(port) = text.lines().next().and_then(|l| l.trim().parse().ok())
        {
            return Ok(port);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(anyhow!(
        "the browser did not start (is another window already using this source's profile? close it, or run `pampas-cli scrape login` again)"
    ))
}

/// The page target's WebSocket address.
async fn page_socket(port: u16) -> anyhow::Result<String> {
    for _ in 0..50 {
        if let Ok(resp) = reqwest::get(format!("http://127.0.0.1:{port}/json/list")).await
            && let Ok(list) = resp.json::<Vec<Value>>().await
            && let Some(url) = list.iter().find(|t| t["type"] == "page").and_then(|t| t["webSocketDebuggerUrl"].as_str())
        {
            return Ok(url.to_string());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(anyhow!("could not find the browser's page"))
}

/// Ask `prompt` on the source's page and return the answer's text.
pub async fn ask(cfg: &ScrapeConfig, prompt: &str) -> anyhow::Result<String> {
    let input = cfg.input_selector.as_deref().context("a browser source needs `input_selector`")?;
    let answer = cfg.selector.as_deref().context("a browser source needs `selector` for the answer")?;
    let profile = profile_dir(cfg)?;
    // A stale file from an earlier run would hand us a dead port.
    let _ = std::fs::remove_file(profile.join("DevToolsActivePort"));
    std::fs::create_dir_all(&profile)?;
    let mut child = command(cfg, cfg.headless, true)?.spawn().context("starting the browser")?;

    let result = drive(cfg, prompt, input, answer, &profile).await;

    // Closing through the browser lets it save the login; killing it is the fallback.
    if let Ok((mut page, _)) = reconnect(&profile).await {
        let _ = page.call("Browser.close", json!({})).await;
    }
    if tokio::time::timeout(Duration::from_secs(5), child.wait()).await.is_err() {
        let _ = child.kill().await;
    }
    result
}

async fn reconnect(profile: &Path) -> anyhow::Result<(Page, u16)> {
    let port = debugging_port(profile).await?;
    let (ws, _) = connect_async(page_socket(port).await?).await?;
    Ok((Page { ws, next_id: 0 }, port))
}

async fn drive(cfg: &ScrapeConfig, prompt: &str, input: &str, answer: &str, profile: &Path) -> anyhow::Result<String> {
    let (mut page, _) = reconnect(profile).await?;
    trace("connected");
    let (input_js, answer_js) = (js(input), js(answer));

    let found = page.wait_for(&format!("document.readyState === 'complete' && !!document.querySelector({input_js})"), INPUT_WAIT_SECS).await?;
    if !found {
        let at = page.eval("document.title + ' (' + location.href + ')'").await.ok().and_then(|v| v.as_str().map(str::to_string));
        return Err(anyhow!(
            "the input box was not found on {}: you may need to log in (`pampas-cli scrape login {}`), the site may be showing a check or a block, or its layout changed",
            at.unwrap_or_else(|| cfg.url.clone()),
            cfg.name
        ));
    }

    // Where the answer is looked for: the page, or the last matching same-site iframe in it.
    let root = match &cfg.answer_frame {
        Some(frame) => format!(
            "(() => {{ const f = document.querySelectorAll({}); const w = f[f.length - 1]; \
             try {{ return (w && w.contentDocument) || null; }} catch (e) {{ return null; }} }})()",
            js(frame)
        ),
        None => "document".to_string(),
    };
    let state = format!(
        "(() => {{ const root = {root}; if (!root) return {{ n: 0, t: '' }}; \
         const all = root.querySelectorAll({answer_js}); const last = all[all.length - 1]; \
         return {{ n: all.length, t: last ? last.innerText.trim() : '' }}; }})()"
    );
    trace("input found");
    let before = page.eval(&state).await?;

    page.eval(&format!("(() => {{ const el = document.querySelector({input_js}); el.focus(); document.execCommand('selectAll', false, null); return true; }})()")).await?;
    page.call("Input.insertText", json!({"text": prompt})).await?;
    trace("typed");
    tokio::time::sleep(Duration::from_millis(400)).await;
    match &cfg.submit_selector {
        Some(sel) => {
            // Find the button once it is there and enabled, then click it with real mouse events: sites
            // often ignore the untrusted click a script's `.click()` makes.
            let locate = format!(
                "(() => {{ const b = document.querySelector({}); \
                 if (!b || b.disabled || b.getAttribute('aria-disabled') === 'true' || !b.getClientRects().length) return null; \
                 b.scrollIntoView({{ block: 'center' }}); const r = b.getBoundingClientRect(); \
                 const x = r.left + r.width / 2, y = r.top + r.height / 2; \
                 const hit = document.elementFromPoint(x, y); \
                 if (!hit || !b.contains(hit)) return null; \
                 return {{ x, y }}; }})()",
                js(sel)
            );
            // The button can still be moving when it first appears (an input box that grows as text is
            // typed), so click only once two reads 200 ms apart agree, and the button itself is what is at that point (the page
            // can still be laying out, leaving something else there).
            let end = Instant::now() + Duration::from_secs(10);
            let mut last: Option<(f64, f64)> = None;
            let spot = loop {
                let at = page.eval(&locate).await?;
                if let (Some(x), Some(y)) = (at["x"].as_f64(), at["y"].as_f64()) {
                    if last.is_some_and(|(lx, ly)| (lx - x).abs() < 1.0 && (ly - y).abs() < 1.0) {
                        break (x, y);
                    }
                    last = Some((x, y));
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
                last = None;
                anyhow::ensure!(Instant::now() < end, "the submit button `{sel}` was not found or stayed disabled");
                tokio::time::sleep(Duration::from_millis(300)).await;
            };
            trace(&format!("clicking {sel} at {:.0},{:.0}", spot.0, spot.1));
            page.click(spot.0, spot.1).await?;
        }
        None => {
            for kind in ["keyDown", "keyUp"] {
                let mut key = json!({"type": kind, "key": "Enter", "code": "Enter", "windowsVirtualKeyCode": 13});
                if kind == "keyDown" {
                    key["text"] = json!("\r");
                }
                page.call("Input.dispatchKeyEvent", key).await?;
            }
        }
    }

    trace("submitted");
    // The answer is the last match once there is a new one (or the last one changed); it is done when
    // it has stopped changing for `settle_secs` and the busy marker, if there is one, is gone.
    let busy = cfg.busy_selector.as_deref().map_or("false".to_string(), |s| {
        format!(
            "(() => {{ const root = {root}; if (!root) return false; \
             return Array.from(root.querySelectorAll({})).some(e => e.getClientRects().length > 0); }})()",
            js(s)
        )
    });
    let settle = Duration::from_secs(cfg.settle_secs.unwrap_or(4));
    let mut last_text = String::new();
    let mut last_traced = String::new();
    let mut last_change = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        // A page that is reloading or navigating makes a call fail for a moment: try again.
        let (Ok(now), Ok(busy_now)) = (page.eval(&state).await, page.eval(&busy).await) else { continue };
        let busy_now = busy_now.as_bool().unwrap_or(false);
        let text = now["t"].as_str().unwrap_or_default().to_string();
        // Only a change is worth a line: the same state is seen every half second while it settles.
        let line = format!("state {now} busy {busy_now}");
        if line != last_traced {
            trace(&line);
            last_traced = line;
        }
        let is_new = now["n"] != before["n"] || now["t"] != before["t"];
        if !is_new || text.is_empty() {
            continue;
        }
        if text != last_text || busy_now {
            last_text = text;
            last_change = Instant::now();
        } else if last_change.elapsed() >= settle {
            return Ok(last_text);
        }
    }
}
