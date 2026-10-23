// The shapes the Rust backend returns (see src-tauri/src/logic.rs).

export interface Locations {
  os: string;
  config: string;
  settings: string;
  usage: string;
  history: string;
  config_exists: boolean;
  legacy: boolean;
}

export type KeyStatus = "not_needed" | "env" | "keychain" | "missing" | "error";

/** One line of the history list. */
export interface HistorySummary {
  id: string;
  asked_at: number;
  prompt: string;
  asked: number;
  answered: number;
  merged_by: string | null;
}

export interface ProviderInfo {
  name: string;
  kind: "openai" | "apple_fm" | "scrape";
  model: string | null;
  base_url: string | null;
  enabled: boolean;
  synthesizer: boolean;
  timeout: number | null;
  fallback: boolean;
  key: string | null;
  key_status: KeyStatus;
  key_error: string | null;
  usage_group: string | null;
  used: number;
  daily_limit: number | null;
  placeholders: string[];
}

export interface SettingInfo {
  name: string;
  value: string | null;
  source: "env" | "file" | "not_set";
}

export interface UsageInfo {
  provider: string;
  group: string | null;
  used: number;
  limit: number | null;
}

export interface AskRequest {
  prompt: string;
  enable: string[];
  disable: string[];
  merge: boolean;
  synthesizer: string | null;
  timeout: number | null;
  ignore_limits: boolean;
  scrape: boolean;
  headless: boolean;
}

export interface Answer {
  provider: string;
  ok: boolean;
  text: string;
}

export type AskEvent =
  | { event: "started"; providers: string[]; warnings: string[] }
  | ({ event: "answer" } & Answer);

export interface AskResponse {
  answers: Answer[];
  merged: { provider: string; text: string } | null;
  merge_error: string | null;
  warnings: string[];
}

export type ExportFormat = "markdown" | "text" | "json";

/** An edition as the backend renders it for copying or saving. */
export interface ExportReport {
  prompt: string;
  answers: Answer[];
  merged: { provider: string; text: string } | null;
  merge_error: string | null;
}

/** One entry of providers.json, as the editor sees it: any field may be present. */
export type ConfigEntry = Record<string, unknown>;
