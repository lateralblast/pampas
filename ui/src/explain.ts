// Turn the backend's technical failures into plain sentences with a next step.

import type { TabId } from "./store.js";

export interface Plain {
  summary: string;
  fix?: { label: string; tab: TabId };
  /** The original text, kept for anyone who needs it. */
  detail: string;
}

/** Why a source that was asked did not answer. */
export function explainFailure(raw: string): Plain {
  const t = raw.toLowerCase();
  const timeout = /timed out after (\d+)s/.exec(t);
  if (timeout) {
    return {
      summary: `No answer within ${timeout[1]} seconds.`,
      fix: { label: "Allow more time", tab: "sources" },
      detail: raw,
    };
  }
  if (t.includes("blocking this address")) {
    return {
      summary: "The site refused the request and may be blocking this address. Stop asking it for a while.",
      detail: raw,
    };
  }
  if (t.includes("input box was not found")) {
    return {
      summary: "The chat page showed no place to type. You may need to log in again, or the site is showing a check or has changed.",
      detail: raw,
    };
  }
  if (t.includes("no chrome or chromium") || t.includes("browser did not start")) {
    return { summary: "The browser for this source could not be started.", detail: raw };
  }
  if (t.includes("selector matched no text") || t.includes("nothing at `")) {
    return {
      summary: "The page came back without an answer in the place pampas looks. Its layout may have changed, or it sent a block or login page.",
      fix: { label: "Check the source", tab: "sources" },
      detail: raw,
    };
  }
  const http = /http (\d{3})/.exec(t);
  const code = http ? Number(http[1]) : 0;
  if (code === 401 || code === 403) {
    return { summary: "The service did not accept the key.", fix: { label: "Check the key", tab: "keys" }, detail: raw };
  }
  if (code === 402) return { summary: "The service wants payment for this model.", detail: raw };
  if (code === 404) {
    return {
      summary: "The service does not know that model or address.",
      fix: { label: "Check the source", tab: "sources" },
      detail: raw,
    };
  }
  if (code === 429) {
    return { summary: "Out of free requests for now: the service asked us to slow down.", detail: raw };
  }
  if (code >= 500) return { summary: `The service had a problem (${code}). Try again shortly.`, detail: raw };
  if (t.includes("daily limit")) return { summary: "Today's free requests are used up.", detail: raw };
  if (/error sending request|connection refused|dns|connect/.test(t)) {
    return {
      summary: "Could not reach the service. Is it running, and is its address right?",
      fix: { label: "Check the source", tab: "sources" },
      detail: raw,
    };
  }
  if (t.includes("no choices") || t.includes("parsing")) {
    return { summary: "The service replied in a form pampas could not read.", detail: raw };
  }
  return { summary: "It did not answer.", detail: raw };
}

/** Why a source was left out before asking. Parses the backend's "skipping provider: name: reason". */
export function explainSkip(warning: string): { provider: string; plain: Plain } | null {
  const m = /^skipping provider:\s*([^:]+):\s*(.*)$/s.exec(warning);
  if (!m) return null;
  const provider = (m[1] ?? "").trim();
  const reason = m[2] ?? "";
  const r = reason.toLowerCase();
  let plain: Plain;
  if (r.startsWith("no key")) {
    plain = { summary: "No key yet.", fix: { label: "Add the key", tab: "keys" }, detail: reason };
  } else if (r.includes("daily limit")) {
    const n = /limit of (\d+)/.exec(r);
    plain = {
      summary: `Today's free requests are used up${n ? ` (${n[1]})` : ""}. They reset at 00:00 UTC.`,
      fix: { label: "See usage", tab: "usage" },
      detail: reason,
    };
  } else if (r.includes("--scrape")) {
    plain = {
      summary: "It scrapes a web site, and scraping is off. Turn it on under More options on the Ask page.",
      detail: reason,
    };
  } else if (r.includes("which is not set")) {
    const n = /\$\{([^}]+)\}/.exec(reason);
    plain = {
      summary: `Needs the setting ${n ? n[1] : "it refers to"}.`,
      fix: { label: "Set it", tab: "keys" },
      detail: reason,
    };
  } else if (r.includes("looking up")) {
    plain = { summary: "The system keychain could not be read.", fix: { label: "See keys", tab: "keys" }, detail: reason };
  } else {
    plain = { summary: "It was left out.", detail: reason };
  }
  return { provider, plain };
}
