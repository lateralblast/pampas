// First-run steps, shown on the Ask page until there is something to ask.

import { api } from "./api.js";
import { announce, h, icon } from "./dom.js";
import { plural } from "./format.js";
import { refresh, state, type TabId } from "./store.js";

export function needsOnboarding(): boolean {
  return !state.loaded || !state.locations?.config_exists || !state.providers.some((p) => p.enabled && p.key_status !== "missing" && p.key_status !== "error");
}

export function renderOnboarding(go: (tab: TabId) => void, onError: (message: string) => void): HTMLElement {
  const hasConfig = Boolean(state.locations?.config_exists);
  const usable = state.providers.filter((p) => p.enabled && p.key_status !== "missing" && p.key_status !== "error");
  const asked = state.history.length > 0;

  const step = (done: boolean, title: string, text: string, action: Node | null) =>
    h(
      "li",
      { class: done ? "step done" : "step" },
      h("span", { class: "step-mark", "aria-hidden": "true" }, done ? icon("check") : ""),
      h(
        "div",
        { class: "step-body" },
        h("h3", null, title, h("span", { class: "visually-hidden" }, done ? " (done)" : " (to do)")),
        h("p", null, text),
        action,
      ),
    );

  const create = h(
    "button",
    {
      type: "button",
      class: "primary",
      onclick: async (ev: Event) => {
        const btn = ev.currentTarget as HTMLButtonElement;
        btn.disabled = true;
        try {
          await api.createConfig();
          await refresh();
          announce("The starter list was created.");
        } catch (e) {
          onError(e instanceof Error ? e.message : String(e));
          btn.disabled = false;
        }
      },
    },
    "Create the starter list",
  );

  return h(
    "section",
    { class: "onboarding", "aria-labelledby": "onb-title" },
    h("h2", { id: "onb-title" }, "Ask several free AI services at once"),
    h("p", { class: "lede" }, "pampas sends one question to every service you set up, shows each answer, and can merge them into one reading that says where they agree and differ. Three steps to begin:"),
    h(
      "ol",
      { class: "steps" },
      step(
        hasConfig,
        "Make your list of sources",
        hasConfig
          ? `Your list has ${plural(state.providers.length, "source")}. Change it any time under Sources.`
          : "Start from the built-in list of free services. You can add, remove and change them later.",
        hasConfig ? null : create,
      ),
      step(
        usable.length > 0,
        "Add a key for at least one source",
        usable.length > 0
          ? `${plural(usable.length, "source")} can be asked.`
          : "Most services need a free key from their website. Or switch on one that runs on your own computer, such as Ollama, under Sources.",
        usable.length > 0 || !hasConfig ? null : h("button", { type: "button", class: "primary", onclick: () => go("keys") }, "Add keys"),
      ),
      step(asked, "Ask your first question", "Type it in the box below and press Ask.", null),
    ),
  );
}
