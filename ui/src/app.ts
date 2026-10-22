// The shell: tabs, the message area, keyboard shortcuts, and starting everything up.

import { announce, clear, h, modKey } from "./dom.js";
import { refresh, state, type TabId } from "./store.js";
import { mountAsk } from "./views-ask.js";
import { mountHistory } from "./views-history.js";
import { mountKeys } from "./views-keys.js";
import { mountSources } from "./views-sources.js";
import { mountUsage } from "./views-usage.js";

const TABS: { id: TabId; label: string }[] = [
  { id: "ask", label: "Ask" },
  { id: "history", label: "History" },
  { id: "sources", label: "Sources" },
  { id: "keys", label: "Keys & settings" },
  { id: "usage", label: "Usage" },
];

const tabButtons = new Map<TabId, HTMLButtonElement>();
const panels = new Map<TabId, HTMLElement>();
let current: TabId = "ask";
let sourcesDirty = false;

function select(id: TabId, focus = false): void {
  current = id;
  for (const [tab, button] of tabButtons) {
    const on = tab === id;
    button.setAttribute("aria-selected", String(on));
    button.tabIndex = on ? 0 : -1;
    panels.get(tab)?.toggleAttribute("hidden", !on);
  }
  if (focus) tabButtons.get(id)?.focus();
  try {
    history.replaceState(null, "", `#${id}`);
  } catch {
    // a file or custom-protocol page may refuse; the tab still works
  }
}

const message = document.getElementById("message") as HTMLElement;
let messageTimer: number | undefined;

function show(text: string, kind: "error" | "notice"): void {
  window.clearTimeout(messageTimer);
  clear(message);
  message.className = `message ${kind}`;
  message.setAttribute("role", kind === "error" ? "alert" : "status");
  message.append(
    h("p", null, text),
    h("button", { type: "button", class: "quiet", "aria-label": "Dismiss the message", onclick: () => { clear(message); message.className = "message"; } }, "Dismiss"),
  );
  if (kind === "notice") messageTimer = window.setTimeout(() => { clear(message); message.className = "message"; }, 6000);
  if (kind === "error") announce(text, true);
}

const host = {
  go: (tab: TabId) => select(tab, false),
  error: (text: string) => show(text, "error"),
  notice: (text: string) => show(text, "notice"),
  dirty: (dirty: boolean) => {
    sourcesDirty = dirty;
    const label = tabButtons.get("sources");
    if (label) label.dataset.dirty = String(dirty);
  },
};

function build(): void {
  const list = document.getElementById("tabs") as HTMLElement;
  for (const [i, tab] of TABS.entries()) {
    const button = h(
      "button",
      {
        type: "button",
        role: "tab",
        id: `tab-${tab.id}`,
        "aria-controls": `panel-${tab.id}`,
        "aria-selected": "false",
        title: `${tab.label} (${modKey()}+${i + 1})`,
        onclick: () => select(tab.id),
        onkeydown: (ev: KeyboardEvent) => {
          const at = TABS.findIndex((t) => t.id === current);
          const next = ev.key === "ArrowRight" ? at + 1 : ev.key === "ArrowLeft" ? at - 1 : ev.key === "Home" ? 0 : ev.key === "End" ? TABS.length - 1 : -1;
          if (next < 0) return;
          ev.preventDefault();
          select(TABS[(next + TABS.length) % TABS.length]?.id ?? "ask", true);
        },
      },
      tab.label,
      tab.id === "sources" ? h("span", { class: "dirty-dot", "aria-hidden": "true", title: "Unsaved changes" }) : null,
      tab.id === "sources" ? h("span", { class: "visually-hidden dirty-text" }, " (unsaved changes)") : null,
    );
    tabButtons.set(tab.id, button);
    list.append(button);
    panels.set(tab.id, document.getElementById(`panel-${tab.id}`) as HTMLElement);
  }

  mountAsk(panels.get("ask") as HTMLElement, host);
  mountHistory(panels.get("history") as HTMLElement, host);
  mountSources(panels.get("sources") as HTMLElement, host);
  mountKeys(panels.get("keys") as HTMLElement, host);
  mountUsage(panels.get("usage") as HTMLElement, host.error);

  document.addEventListener("keydown", (ev) => {
    if (!(ev.metaKey || ev.ctrlKey) || ev.altKey || ev.shiftKey) return;
    const n = Number(ev.key);
    if (n >= 1 && n <= TABS.length) {
      ev.preventDefault();
      select(TABS[n - 1]?.id ?? "ask", true);
    }
  });
  window.addEventListener("beforeunload", (ev) => {
    if (sourcesDirty) ev.preventDefault();
  });

  const fromHash = location.hash.slice(1) as TabId;
  select(TABS.some((t) => t.id === fromHash) ? fromHash : "ask");
}

async function start(): Promise<void> {
  build();
  try {
    await refresh();
  } catch (e) {
    show(e instanceof Error ? e.message : String(e), "error");
  }
  void state;
}

void start();
