// What the interface knows about the app, refreshed from the backend, and the rules for whether a
// source is ready to be asked.

import { api } from "./api.js";
import { assignSigla } from "./siglum.js";
import { clearLegacyHistory, legacyHistory } from "./history.js";
import type { HistorySummary, Locations, ProviderInfo, SettingInfo } from "./types.js";

export type TabId = "ask" | "history" | "sources" | "keys" | "usage";

export interface State {
  locations: Locations | null;
  providers: ProviderInfo[];
  settings: SettingInfo[];
  history: HistorySummary[];
  loaded: boolean;
  /** Set when the config exists but cannot be read; the message is the backend's. */
  configError: string | null;
}

export const state: State = { locations: null, providers: [], settings: [], history: [], loaded: false, configError: null };

const listeners = new Set<() => void>();

export function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** Reload everything from the backend and tell the views. */
export async function refresh(): Promise<void> {
  state.locations = await api.locations();
  try {
    state.providers = state.locations.config_exists ? await api.listProviders() : [];
    state.configError = null;
  } catch (e) {
    state.providers = [];
    state.configError = e instanceof Error ? e.message : String(e);
  }
  try {
    state.settings = await api.listSettings();
  } catch {
    state.settings = [];
  }
  try {
    // Sessions the interface used to keep in local storage move into the history file once.
    if (!state.loaded) {
      const old = legacyHistory();
      if (old.length) {
        await api.importHistory(old);
        clearLegacyHistory();
      }
    }
    state.history = await api.listHistory();
  } catch {
    state.history = [];
  }
  state.loaded = true;
  for (const fn of [...listeners]) fn();
}

export function siglaOf(providers: ProviderInfo[] = state.providers): Map<string, string> {
  return assignSigla(providers.map((p) => p.name));
}

export interface NotReady {
  /** Short, plain words for a list row. */
  text: string;
  /** Where the fix is. */
  fix: TabId;
  fixLabel: string;
}

/** Why a source cannot be asked right now, or null when it can. */
export function notReady(p: ProviderInfo, settings: SettingInfo[] = state.settings): NotReady | null {
  if (p.key_status === "missing") return { text: "needs a key", fix: "keys", fixLabel: "Add the key" };
  if (p.key_status === "error") return { text: "keychain unavailable", fix: "keys", fixLabel: "See keys" };
  const unset = p.placeholders.find((n) => settings.find((s) => s.name === n)?.source === "not_set");
  if (unset) return { text: `needs the setting ${unset}`, fix: "keys", fixLabel: "Set it" };
  return null;
}

export function limitReached(p: ProviderInfo): boolean {
  return p.daily_limit !== null && p.used >= p.daily_limit;
}

/** Sources the user can ask now (the daily limit can be overridden, so it does not count here). */
export function readyProviders(): ProviderInfo[] {
  return state.providers.filter((p) => notReady(p) === null);
}

/** Distinct key names with the sources that use each, and the status of the key. */
export interface KeyGroup {
  name: string;
  providers: string[];
  status: ProviderInfo["key_status"];
  error: string | null;
}

export function keyGroups(): KeyGroup[] {
  const groups = new Map<string, KeyGroup>();
  for (const p of state.providers) {
    if (!p.key) continue;
    const g = groups.get(p.key);
    if (g) g.providers.push(p.name);
    else groups.set(p.key, { name: p.key, providers: [p.name], status: p.key_status, error: p.key_error });
  }
  return [...groups.values()];
}
