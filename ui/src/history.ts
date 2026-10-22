// Interface preferences, and the old local-storage history that is moved into the history file once.
// (Past sessions themselves are kept by the backend, in history.json beside the usage file.)

import type { EditionData } from "./edition.js";

const OLD_HISTORY = "pampas.history.v1";

/** Sessions the interface used to keep in the webview's local storage, for the one-time move. */
export function legacyHistory(): EditionData[] {
  try {
    const raw = localStorage.getItem(OLD_HISTORY);
    const list = raw ? (JSON.parse(raw) as EditionData[]) : [];
    return Array.isArray(list) ? list.filter((e) => e && typeof e.prompt === "string" && typeof e.id === "string") : [];
  } catch {
    return [];
  }
}

export function clearLegacyHistory(): void {
  try {
    localStorage.removeItem(OLD_HISTORY);
  } catch {
    // nothing to clear
  }
}

const PREFS = "pampas.prefs.v1";

export interface Prefs {
  merge: boolean;
  synthesizer: string;
  /** Save each finished question to the history. */
  keepHistory: boolean;
}

export function loadPrefs(): Prefs {
  try {
    const p = JSON.parse(localStorage.getItem(PREFS) ?? "{}") as Partial<Prefs>;
    return { merge: p.merge ?? true, synthesizer: p.synthesizer ?? "", keepHistory: p.keepHistory ?? true };
  } catch {
    return { merge: true, synthesizer: "", keepHistory: true };
  }
}

export function savePrefs(prefs: Prefs): void {
  try {
    localStorage.setItem(PREFS, JSON.stringify(prefs));
  } catch {
    // preferences are a convenience
  }
}
