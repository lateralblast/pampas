// Calls to the Rust backend. In a browser without Tauri (the mock page used for development) the
// same calls go to `window.__TAURI__` supplied by dev/mock.js.

import type {
  AskEvent,
  AskRequest,
  AskResponse,
  ConfigEntry,
  ExportFormat,
  ExportReport,
  HistorySummary,
  Locations,
  ProviderInfo,
  SettingInfo,
  UsageInfo,
} from "./types.js";

interface TauriGlobal {
  core: { invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> };
  event: { listen(name: string, cb: (e: { payload: unknown }) => void): Promise<() => void> };
}

declare global {
  interface Window {
    __TAURI__?: TauriGlobal;
  }
}

function tauri(): TauriGlobal {
  if (!window.__TAURI__) throw new Error("The app backend is not available.");
  return window.__TAURI__;
}

/** Backend errors arrive as text; make them Errors so the UI can show `.message`. */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return (await tauri().core.invoke(cmd, args)) as T;
  } catch (e) {
    throw e instanceof Error ? e : new Error(String(e));
  }
}

export const api = {
  locations: () => call<Locations>("locations"),
  listProviders: () => call<ProviderInfo[]>("list_providers"),
  getConfig: () => call<ConfigEntry[]>("get_config"),
  exampleConfig: () => call<ConfigEntry[]>("example_config"),
  validateConfig: (entries: ConfigEntry[]) => call<null>("validate_config", { entries }),
  saveConfig: (entries: ConfigEntry[]) => call<null>("save_config", { entries }),
  createConfig: () => call<null>("create_config"),
  setKey: (name: string, value: string) => call<null>("set_key", { name, value }),
  deleteKey: (name: string) => call<boolean>("delete_key", { name }),
  listSettings: () => call<SettingInfo[]>("list_settings"),
  setSetting: (name: string, value: string) => call<null>("set_setting", { name, value }),
  deleteSetting: (name: string) => call<boolean>("delete_setting", { name }),
  usage: () => call<UsageInfo[]>("usage"),
  listHistory: () => call<HistorySummary[]>("list_history"),
  getHistory: (id: string) => call<unknown>("get_history", { id }),
  saveHistory: (session: unknown) => call<null>("save_history", { session }),
  deleteHistory: (id: string) => call<boolean>("delete_history", { id }),
  clearHistory: () => call<number>("clear_history"),
  importHistory: (sessions: unknown[]) => call<number>("import_history", { sessions }),
  /** The edition as text in the command line's format, for the clipboard. */
  renderEdition: (report: ExportReport, format: ExportFormat) => call<string>("render_edition", { report, format }),
  /** Open the system's save dialog and write the edition there; the path, or null if cancelled. */
  saveEdition: (report: ExportReport, format: ExportFormat) => call<string | null>("save_edition", { report, format }),

  /** Ask the providers; `onEvent` hears the start and each answer as it arrives. */
  async ask(request: AskRequest, onEvent: (e: AskEvent) => void): Promise<AskResponse> {
    const stop = await tauri().event.listen("pampas-ask", (e) => onEvent(e.payload as AskEvent));
    try {
      return await call<AskResponse>("ask", { request });
    } finally {
      stop();
    }
  },
};
