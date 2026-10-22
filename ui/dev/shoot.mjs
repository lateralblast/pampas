// Dev tool: drive the mock page in headless Chrome and save screenshots.
//   [VIEWPORT_ONLY=1] node dev/shoot.mjs <outdir> <name> <width> <height> <light|dark> '<url query>' '<js to run>' [waitMs] [waitForJs]
// Needs the dev server (node dev/serve.mjs) and google-chrome. No dependencies.
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [outdir, name, width, height, scheme, query, script = "", waitMs = "800", waitFor = ""] = process.argv.slice(2);
mkdirSync(outdir, { recursive: true });
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(process.env.CHROME ?? "google-chrome", ["--headless=new", `--remote-debugging-port=${port}`, "--no-first-run", "--disable-gpu", "--hide-scrollbars", `--user-data-dir=${join(outdir, ".profile-" + port)}`, "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function targetUrl() {
  for (let i = 0; i < 50; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find((t) => t.type === "page");
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    await sleep(200);
  }
  throw new Error("Chrome did not start");
}

const ws = new WebSocket(await targetUrl());
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
const logs = [];
ws.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
  if (msg.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(msg.params.type)) logs.push(`${msg.params.type}: ${msg.params.args.map((a) => a.value ?? a.description).join(" ")}`);
  if (msg.method === "Runtime.exceptionThrown") logs.push("exception: " + (msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text));
};
const send = (method, params = {}) => new Promise((res) => { const i = ++id; pending.set(i, res); ws.send(JSON.stringify({ id: i, method, params })); });
const evalJs = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.result?.exceptionDetails) logs.push("eval error: " + (r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text));
  return r.result?.result?.value;
};

await send("Runtime.enable");
await send("Page.enable");
await send("Emulation.setDeviceMetricsOverride", { width: Number(width), height: Number(height), deviceScaleFactor: 1, mobile: false });
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: scheme }] });
await send("Page.navigate", { url: `http://127.0.0.1:8765/dev/index.html${query ? "?" + query : ""}` });
for (let i = 0; i < 60 && !(await evalJs("!!document.getElementById('prompt') && document.body.dataset.ready !== '0'")); i++) await sleep(100);
await sleep(300);
if (script) await evalJs(`(async()=>{ ${script} })()`);
if (waitFor) for (let i = 0; i < 100 && !(await evalJs(waitFor)); i++) await sleep(150);
await sleep(Number(waitMs));
const metrics = (await send("Page.getLayoutMetrics")).result;
const h = Math.min(Math.ceil(metrics.cssContentSize?.height ?? metrics.contentSize.height), 6000);
// VIEWPORT_ONLY=1 captures just the window at its top (for documentation); otherwise the whole page.
const viewportOnly = process.env.VIEWPORT_ONLY === "1";
if (viewportOnly) await evalJs("window.scrollTo(0, 0)");
const shot = await send("Page.captureScreenshot", viewportOnly
  ? { format: "png" }
  : { format: "png", captureBeyondViewport: true, clip: { x: 0, y: 0, width: Number(width), height: Math.max(Number(height), h), scale: 1 } });
const file = join(outdir, `${name}.png`);
writeFileSync(file, Buffer.from(shot.result.data, "base64"));
console.log(file, viewportOnly ? `${width}x${height}` : `${width}x${Math.max(Number(height), h)}`);
if (logs.length) console.log("CONSOLE:\n" + logs.join("\n"));
ws.close();
chrome.kill();
