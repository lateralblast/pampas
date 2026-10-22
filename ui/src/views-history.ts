// The History tab: every finished question, saved automatically. Open one to read it again, ask it
// again, or delete it.

import { api } from "./api.js";
import { announce, clear, h, icon } from "./dom.js";
import { measureClamps, renderEdition, tickElapsed, updateEdition, type EditionData } from "./edition.js";
import { clock, day, plural } from "./format.js";
import { loadPrefs, savePrefs } from "./history.js";
import { refresh, state, subscribe, type TabId } from "./store.js";
import type { HistorySummary } from "./types.js";

export interface HistoryHost {
  go(tab: TabId): void;
  error(message: string): void;
  notice(message: string): void;
}

/** "Today", "Yesterday", or the date, for grouping the list. */
function dayLabel(epochMs: number): string {
  const d = new Date(epochMs);
  const label = day(epochMs);
  return label === "Today" || label === "Yesterday" ? label : d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long", year: d.getFullYear() === new Date().getFullYear() ? undefined : "numeric" });
}

export function mountHistory(root: HTMLElement, host: HistoryHost): void {
  let prefs = loadPrefs();
  let filter = "";
  let openId: string | null = null;
  let opened: EditionData | null = null;
  const expanded = new Set<string>();
  let armed: string | null = null; // the session whose Delete has been pressed once
  let armedAll = false;
  let armTimer: number | undefined;
  const fail = (e: unknown) => host.error(e instanceof Error ? e.message : String(e));

  const arm = (id: string | "all" | null) => {
    window.clearTimeout(armTimer);
    armed = id && id !== "all" ? id : null;
    armedAll = id === "all";
    if (id) armTimer = window.setTimeout(() => { armed = null; armedAll = false; render(); }, 5000);
  };

  async function remove(id: string): Promise<void> {
    try {
      await api.deleteHistory(id);
      arm(null);
      announce("The session was deleted.");
      if (openId === id) {
        openId = null;
        opened = null;
      }
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  async function open(id: string): Promise<void> {
    try {
      opened = (await api.getHistory(id)) as EditionData;
      openId = id;
      expanded.clear();
      arm(null);
      render();
      window.scrollTo(0, 0);
      root.querySelector<HTMLElement>("[data-focus-key=back]")?.focus();
    } catch (e) {
      fail(e);
      await refresh();
    }
  }

  function listView(): HTMLElement {
    const page = h("div", { class: "page" });
    page.append(
      h("h2", { class: "page-title" }, "History"),
      h("p", { class: "lede" }, "Each question you ask is saved here automatically, with everything that came back, on this computer. Open one to read it again, or delete the ones you do not want to keep."),
    );
    const keep = h("input", { type: "checkbox", id: "keep-history", checked: prefs.keepHistory, onchange: (ev: Event) => { prefs = { ...prefs, keepHistory: (ev.currentTarget as HTMLInputElement).checked }; savePrefs(prefs); announce(prefs.keepHistory ? "New questions will be saved." : "New questions will not be saved."); render(); } });
    page.append(h("div", { class: "check" }, keep, h("label", { for: "keep-history" }, "Save new questions to the history")));
    if (!prefs.keepHistory) page.append(h("p", { class: "hint" }, "History is off: questions you ask now are not being saved. The ones below stay until you delete them."));

    const all = state.history;
    if (!all.length) {
      page.append(
        h("div", { class: "empty" }, h("h3", null, "Nothing here yet"), h("p", null, "Questions you ask are saved here automatically, so you can come back to them."), h("button", { type: "button", class: "primary", onclick: () => host.go("ask") }, "Ask a question")),
      );
      return page;
    }

    const list = all.filter((s) => s.prompt.toLowerCase().includes(filter.trim().toLowerCase()));
    page.append(
      h(
        "div",
        { class: "history-tools" },
        h("div", { class: "field" }, h("label", { for: "history-filter" }, "Find a question"), h("input", { type: "search", id: "history-filter", value: filter, autocomplete: "off", placeholder: "Type part of a question", dataset: { focusKey: "filter" }, oninput: (ev: Event) => { filter = (ev.currentTarget as HTMLInputElement).value; render(); } })),
        h("p", { class: "count" }, filter ? `${plural(list.length, "session")} match` : plural(all.length, "session")),
        h("button", { type: "button", class: armedAll ? "danger" : "quiet", dataset: { focusKey: "clear-all" }, onclick: async () => {
          if (!armedAll) { arm("all"); render(); return; }
          try { const n = await api.clearHistory(); arm(null); announce(`${plural(n, "session")} deleted.`); host.notice(`Deleted ${plural(n, "session")} from the history.`); await refresh(); } catch (e) { fail(e); }
        } }, icon("trash"), armedAll ? "Click again to delete everything" : "Delete all history"),
      ),
    );
    if (!list.length) {
      page.append(h("p", { class: "muted" }, "No saved question matches that."));
      return page;
    }

    const groups = new Map<string, HistorySummary[]>();
    for (const s of list) {
      const key = dayLabel(s.asked_at);
      groups.set(key, [...(groups.get(key) ?? []), s]);
    }
    for (const [label, items] of groups) {
      page.append(
        h("h3", { class: "section-label" }, label),
        h(
          "ol",
          { class: "history-rows" },
          ...items.map((s) =>
            h(
              "li",
              { class: "row hist-row" },
              h("div", { class: "gutter" }, h("p", { class: "hist-time" }, clock(s.asked_at)), h("p", { class: "gmeta" }, `${s.answered} of ${plural(s.asked, "source")} answered`), s.merged_by ? h("p", { class: "gmeta" }, `merged by ${s.merged_by}`) : null),
              h(
                "div",
                { class: "body" },
                h("button", { type: "button", class: "hist-open", dataset: { focusKey: `open-${s.id}` }, onclick: () => void open(s.id) }, h("span", { class: "hist-q" }, s.prompt), h("span", { class: "visually-hidden" }, ` — open this session`)),
                h("div", { class: "row-actions" },
                  h("button", { type: "button", class: "quiet", onclick: () => void open(s.id) }, icon("chevron"), "Open"),
                  h("button", { type: "button", class: armed === s.id ? "danger" : "quiet", dataset: { focusKey: `del-${s.id}` }, onclick: () => { if (armed !== s.id) { arm(s.id); render(); } else void remove(s.id); } }, icon("trash"), armed === s.id ? "Click again to delete" : "Delete", h("span", { class: "visually-hidden" }, ` “${s.prompt.slice(0, 60)}”`)),
                ),
              ),
            ),
          ),
        ),
      );
    }
    return page;
  }

  function detailView(data: EditionData): HTMLElement {
    const slot = h("div", { class: "edition-slot" });
    const edition = renderEdition(data, {
      expanded,
      saved: true,
      onToggle: (name) => { if (expanded.has(name)) expanded.delete(name); else expanded.add(name); render(); },
      onReveal: (name) => {
        expanded.add(name);
        render();
        requestAnimationFrame(() => {
          const target = root.querySelector<HTMLElement>(`[data-key="${CSS.escape(name)}"] h3, [data-key="skip-${CSS.escape(name)}"] h3`);
          target?.scrollIntoView({ block: "center" });
          target?.focus();
        });
      },
      onExpandAll: (expand) => {
        expanded.clear();
        if (expand) for (const n of data.asked) if (data.answers[n]?.ok) expanded.add(n);
        render();
      },
      onNavigate: host.go,
      onMessage: (kind, text) => (kind === "error" ? host.error(text) : host.notice(text)),
      onAskAgain: () => {
        host.go("ask");
        window.setTimeout(() => window.dispatchEvent(new CustomEvent("pampas:prefill", { detail: { prompt: data.prompt } })), 0);
      },
    });
    updateEdition(slot, edition);
    return h(
      "div",
      { class: "page" },
      h("div", { class: "detail-bar" },
        h("button", { type: "button", dataset: { focusKey: "back" }, onclick: () => { openId = null; opened = null; arm(null); render(); window.setTimeout(() => root.querySelector<HTMLElement>("[data-focus-key^=open-]")?.focus(), 0); } }, h("span", { "aria-hidden": "true" }, "←"), "All sessions"),
        h("button", { type: "button", class: armed === data.id ? "danger" : "quiet", dataset: { focusKey: "delete-open" }, onclick: () => { if (armed !== data.id) { arm(data.id); render(); } else void remove(data.id); } }, icon("trash"), armed === data.id ? "Click again to delete this session" : "Delete this session"),
      ),
      slot,
    );
  }

  function render(): void {
    const active = document.activeElement as HTMLElement | null;
    const key = active && root.contains(active) ? active.dataset.focusKey : undefined;
    const view = openId && opened ? detailView(opened) : listView();
    clear(root);
    root.append(view);
    if (key) root.querySelector<HTMLElement>(`[data-focus-key="${CSS.escape(key)}"]`)?.focus();
    requestAnimationFrame(() => {
      measureClamps(root);
      tickElapsed(root);
    });
  }

  subscribe(() => {
    // A session deleted elsewhere (or the list changing) while one is open: go back to the list.
    if (openId && !state.history.some((s) => s.id === openId)) {
      openId = null;
      opened = null;
    }
    render();
  });
  render();
}
