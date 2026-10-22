// The Usage page: today's request counts against each source's daily limit.

import { api } from "./api.js";
import { clear, h, icon } from "./dom.js";
import { untilReset } from "./format.js";
import { siglaOf, state, subscribe } from "./store.js";
import type { UsageInfo } from "./types.js";

export function mountUsage(root: HTMLElement, onError: (message: string) => void): void {
  let rows: UsageInfo[] = [];

  async function load(): Promise<void> {
    try {
      rows = state.locations?.config_exists ? await api.usage() : [];
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    }
    render();
  }

  function render(): void {
    const sigla = siglaOf();
    clear(root);
    const table = h(
      "table",
      { class: "usage" },
      h("caption", { class: "visually-hidden" }, "Requests made today against each source's daily limit"),
      h("thead", null, h("tr", null, h("th", { scope: "col" }, "Source"), h("th", { scope: "col" }, "Counted with"), h("th", { scope: "col", class: "num" }, "Used today"), h("th", { scope: "col" }, "Of the daily limit"))),
      h(
        "tbody",
        null,
        ...rows.map((r) => {
          const limited = r.limit !== null;
          const pct = limited && r.limit ? Math.min(100, Math.round((r.used / r.limit) * 100)) : 0;
          const full = limited && r.limit !== null && r.used >= r.limit;
          return h(
            "tr",
            null,
            h("th", { scope: "row" }, h("span", { class: "sig", "aria-hidden": "true" }, sigla.get(r.provider) ?? "?"), " ", r.provider),
            h("td", { class: "muted" }, r.group === null ? "on this device" : r.group === r.provider ? "itself" : r.group),
            h("td", { class: "num" }, r.group === null ? "not counted" : String(r.used)),
            h(
              "td",
              null,
              r.group === null
                ? h("span", { class: "muted" }, "No limit: it runs on your own device")
                : limited
                  ? h("div", { class: "meter-wrap" }, h("div", { class: `meter${full ? " full" : ""}`, role: "img", "aria-label": `${r.used} of ${r.limit} used` }, h("div", { class: "meter-fill", style: `width:${pct}%` })), h("span", { class: full ? "bad-text" : "" }, full ? icon("warn") : null, `${r.used} of ${r.limit}`, full ? " — used up" : ""))
                  : h("span", { class: "muted" }, "No limit set"),
            ),
          );
        }),
      ),
    );
    root.append(
      h(
        "div",
        { class: "page" },
        h("h2", { class: "page-title" }, "Usage"),
        h("p", { class: "lede" }, `Free services allow only so many requests a day. pampas counts each one it makes, and stops asking a source whose limit is used up. Counts start again at 00:00 UTC, in ${untilReset()}.`),
        state.locations?.config_exists ? table : h("p", { class: "muted" }, "Make your list of sources first."),
        h("p", { class: "hint" }, "Only requests made through pampas are counted, on this computer. “No limit set” means pampas will not stop asking that source, not that the service has no limit of its own."),
        h("button", { type: "button", onclick: () => void load() }, "Refresh"),
      ),
    );
  }

  subscribe(() => void load());
  render();
}
