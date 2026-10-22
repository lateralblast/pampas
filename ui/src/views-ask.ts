// The Ask page: the question, which sources to ask, and the edition that comes back.

import { api } from "./api.js";
import { announce, clear, h, icon, modKey } from "./dom.js";
import {
  applyAnswer,
  applyFailure,
  applyResponse,
  applyStarted,
  measureClamps,
  newEdition,
  renderEdition,
  tickElapsed,
  updateEdition,
  type EditionData,
} from "./edition.js";
import { clock, day, plural } from "./format.js";
import { loadPrefs, savePrefs } from "./history.js";
import { needsOnboarding, renderOnboarding } from "./onboarding.js";
import { siglaOf, limitReached, notReady, readyProviders, refresh, state, subscribe, type TabId } from "./store.js";
import type { ProviderInfo } from "./types.js";

const OVERRIDES = "pampas.sources.v1";

function loadOverrides(): Record<string, boolean> {
  try {
    return JSON.parse(localStorage.getItem(OVERRIDES) ?? "{}") as Record<string, boolean>;
  } catch {
    return {};
  }
}

function saveOverrides(o: Record<string, boolean>): void {
  try {
    localStorage.setItem(OVERRIDES, JSON.stringify(o));
  } catch {
    // a convenience only
  }
}

export interface AskHost {
  go(tab: TabId): void;
  error(message: string): void;
  notice(message: string): void;
}

export function mountAsk(root: HTMLElement, host: AskHost): void {
  let overrides = loadOverrides();
  let prefs = loadPrefs();
  let edition: EditionData | null = null;
  let saved = false;
  let running = false;
  const expanded = new Set<string>();
  let ticker: number | undefined;

  const banner = h("div", { class: "banner-slot" });
  const onboardingSlot = h("div", null);
  const prompt = h("textarea", {
    id: "prompt",
    rows: 3,
    placeholder: "Ask something that deserves more than one opinion",
    "aria-describedby": "prompt-hint",
    spellcheck: true,
  });
  const sourcesSlot = h("div", { class: "sources-slot" });
  const mergeBox = h("input", { type: "checkbox", id: "merge", checked: prefs.merge });
  const synth = h("select", { id: "synth", "aria-label": "Which source writes the reading" });
  const timeoutBox = h("input", { type: "number", id: "timeout", min: 1, max: 900, step: 1, inputMode: "numeric", placeholder: "each source's own" });
  const limitsBox = h("input", { type: "checkbox", id: "ignore-limits" });
  const askButton = h("button", { type: "submit", class: "primary", id: "ask-button" }, "Ask");
  const setBusy = (busy: boolean) => {
    // aria-disabled, not disabled: a disabled button drops keyboard focus in some web views.
    askButton.setAttribute("aria-disabled", String(busy));
    askButton.classList.toggle("busy", busy);
    askButton.textContent = busy ? "Asking…" : "Ask";
    if (!busy) markIdle();
  };
  /** With no source to ask, Ask is not the thing to do next: it stays focusable but looks inactive. */
  const markIdle = () => {
    const none = selectedNames().length === 0;
    askButton.setAttribute("aria-disabled", String(none));
    askButton.classList.toggle("idle", none);
    askButton.title = none ? "Choose a source to ask first" : "";
  };
  const slot = h("div", { class: "edition-slot", "aria-live": "off" });

  const hint = h("span", { id: "prompt-hint", class: "hint ask-hint" }, `${modKey()}+Enter to ask`);
  const advanced = h(
    "details",
    { class: "advanced" },
    h("summary", null, "More options"),
    h(
      "div",
      { class: "advanced-body" },
      h("div", { class: "field" }, h("label", { for: "timeout" }, "Wait at most (seconds) for each source"), timeoutBox, h("p", { class: "hint" }, "Leave empty to use each source's own limit (30 seconds unless set under Sources). Large models on your own computer may need several minutes.")),
      h("div", { class: "check" }, limitsBox, h("label", { for: "ignore-limits" }, "Ignore the daily limits for this question")),
    ),
  );

  const form = h(
    "form",
    { class: "composer", novalidate: true },
    h("label", { for: "prompt", class: "field-label" }, "Question"),
    prompt,
    sourcesSlot,
    h(
      "div",
      { class: "composer-row" },
      h("div", { class: "check" }, mergeBox, h("label", { for: "merge" }, "Merge the answers into one reading")),
      h("div", { class: "synth-wrap" }, h("label", { for: "synth", class: "visually-hidden" }, "Reading written by"), synth),
      h("div", { class: "ask-group" }, hint, askButton),
    ),
    h("div", { class: "quiet-row" }, advanced),
  );

  root.append(banner, onboardingSlot, form, slot);

  // ---- sources ----

  const isSelected = (p: ProviderInfo): boolean => {
    if (notReady(p)) return false;
    return overrides[p.name] ?? p.enabled;
  };

  const selectedNames = (): string[] => state.providers.filter(isSelected).map((p) => p.name);

  function renderSources(): void {
    const sigla = siglaOf();
    const names = selectedNames();
    const items = state.providers.map((p) => {
      const problem = notReady(p);
      const checkbox = h("input", {
        type: "checkbox",
        id: `src-${p.name}`,
        checked: isSelected(p),
        disabled: Boolean(problem),
        onchange: (ev: Event) => {
          overrides[p.name] = (ev.currentTarget as HTMLInputElement).checked;
          saveOverrides(overrides);
          renderSources();
          renderSynth();
        },
      });
      const limit = limitReached(p) ? h("span", { class: "tag bad" }, icon("warn"), "today's free requests used up") : null;
      return h(
        "li",
        { class: problem ? "source off" : "source" },
        h("div", { class: "check" }, checkbox, h("label", { for: `src-${p.name}` }, h("span", { class: "sig", "aria-hidden": "true" }, sigla.get(p.name) ?? "?"), h("span", { class: "sname" }, p.name), p.model ? h("span", { class: "smodel" }, p.model) : null)),
        problem
          ? h("span", { class: "tag bad" }, icon("warn"), problem.text, h("button", { type: "button", class: "linklike", onclick: () => host.go(problem.fix) }, problem.fixLabel))
          : limit,
      );
    });
    const summary = state.providers.length
      ? `${plural(names.length, "source")} will be asked`
      : "No sources yet";
    clear(sourcesSlot);
    if (!running) markIdle();
    sourcesSlot.append(
      h(
        "details",
        { class: "sources", open: names.length === 0 && state.providers.length > 0 },
        h(
          "summary",
          null,
          summary,
          names.length ? h("span", { class: "sig-run", "aria-hidden": "true" }, ...names.map((n) => h("span", { class: "sig" }, sigla.get(n) ?? "?"))) : null,
          names.length ? h("span", { class: "visually-hidden" }, `: ${names.join(", ")}`) : null,
        ),
        h(
          "div",
          { class: "sources-body" },
          h(
            "div",
            { class: "sources-tools" },
            h("button", { type: "button", class: "quiet", onclick: () => setAll(true) }, "Select all that are ready"),
            h("button", { type: "button", class: "quiet", onclick: () => setAll(false) }, "Select none"),
          ),
          h("ul", { class: "source-list" }, ...items),
        ),
      ),
    );
  }

  function setAll(on: boolean): void {
    overrides = {};
    for (const p of state.providers) overrides[p.name] = on;
    saveOverrides(overrides);
    renderSources();
    renderSynth();
  }

  function renderSynth(): void {
    const chosen = synth.value || prefs.synthesizer;
    clear(synth);
    synth.append(h("option", { value: "" }, "Reading written by: automatic"));
    for (const name of selectedNames()) synth.append(h("option", { value: name }, `Reading written by: ${name}`));
    synth.value = selectedNames().includes(chosen) ? chosen : "";
    synth.disabled = !mergeBox.checked;
  }

  mergeBox.addEventListener("change", () => {
    prefs = { ...prefs, merge: mergeBox.checked };
    savePrefs(prefs);
    synth.disabled = !mergeBox.checked;
  });
  synth.addEventListener("change", () => {
    prefs = { ...prefs, synthesizer: synth.value };
    savePrefs(prefs);
  });

  // ---- the edition ----

  function drawEdition(): void {
    if (!edition) {
      clear(slot);
      return;
    }
    const el = renderEdition(edition, {
      expanded,
      saved,
      onReveal: (name) => {
        expanded.add(name);
        drawEdition();
        requestAnimationFrame(() => {
          const target = slot.querySelector<HTMLElement>(`[data-key="${CSS.escape(name)}"] h3, [data-key="skip-${CSS.escape(name)}"] h3`);
          target?.scrollIntoView({ block: "center" });
          target?.focus();
        });
      },
      onToggle: (name) => {
        if (expanded.has(name)) expanded.delete(name);
        else expanded.add(name);
        drawEdition();
      },
      onExpandAll: (expand) => {
        expanded.clear();
        if (expand && edition) for (const n of edition.asked) if (edition.answers[n]?.ok) expanded.add(n);
        drawEdition();
      },
      onNavigate: host.go,
      onMessage: (kind, text) => (kind === "error" ? host.error(text) : host.notice(text)),
      onAskAgain: () => {
        saved = false;
        prompt.focus();
        void ask();
      },
    });
    updateEdition(slot, el);
    requestAnimationFrame(() => {
      measureClamps(slot);
      tickElapsed(slot);
    });
  }

  function startTicker(): void {
    window.clearInterval(ticker);
    ticker = window.setInterval(() => tickElapsed(slot), 1000);
  }

  async function ask(): Promise<void> {
    if (running) return;
    const question = prompt.value.trim();
    if (!question) {
      host.error("Type a question first.");
      prompt.focus();
      return;
    }
    const names = selectedNames();
    if (!names.length) {
      host.error("No source is selected. Open “sources will be asked” and tick at least one, or add a key.");
      return;
    }
    const timeout = Number(timeoutBox.value);
    const order = state.providers.map((p) => p.name);
    const models = Object.fromEntries(state.providers.map((p) => [p.name, p.model]));
    const request = {
      prompt: question,
      enable: state.providers.filter((p) => names.includes(p.name) && !p.enabled).map((p) => p.name),
      disable: state.providers.filter((p) => !names.includes(p.name) && p.enabled).map((p) => p.name),
      merge: mergeBox.checked,
      synthesizer: mergeBox.checked && synth.value ? synth.value : null,
      timeout: timeout > 0 ? Math.round(timeout) : null,
      ignore_limits: limitsBox.checked,
    };
    const current = newEdition(question, order, models, request.merge);
    edition = current;
    saved = false;
    expanded.clear();
    running = true;
    setBusy(true);
    startTicker();
    drawEdition();
    requestAnimationFrame(() => slot.scrollIntoView({ block: "start" }));
    try {
      const response = await api.ask(request, (event) => {
        if (event.event === "started") {
          applyStarted(current, event.providers, event.warnings);
          announce(`Asking ${plural(event.providers.length, "source")}.`);
        } else {
          applyAnswer(current, event);
          announce(event.ok ? `${event.provider} answered.` : `${event.provider} did not answer.`);
        }
        drawEdition();
      });
      applyResponse(current, response);
      announce(current.merged ? "The reading is ready." : "All answers are in.");
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      const lead = "no usable providers";
      if (message.startsWith(lead)) {
        // Every selected source was left out: show why, source by source.
        applyStarted(current, [], message.slice(lead.length).replace(/^[:\s]+/, "").split("; ").filter(Boolean));
        current.phase = "done";
        current.finishedAt = Date.now();
        if (!current.skipped.length) applyFailure(current, message);
        announce("None of the selected sources could be asked.", true);
      } else {
        applyFailure(current, message);
        announce(message, true);
      }
    } finally {
      running = false;
      window.clearInterval(ticker);
      setBusy(false);
      drawEdition();
      // Read the setting now: the History tab can change it while this page stays open.
      if (loadPrefs().keepHistory && (current.asked.length || current.skipped.length)) {
        try {
          await api.saveHistory(current);
        } catch (e) {
          host.error(`This question could not be saved to the history: ${e instanceof Error ? e.message : String(e)}`);
        }
      }
      renderOnboardingSlot();
      void refresh();
    }
  }

  // "Ask again" on a saved session puts its question here, ready to edit and send.
  window.addEventListener("pampas:prefill", (ev) => {
    prompt.value = (ev as CustomEvent<{ prompt: string }>).detail.prompt;
    window.scrollTo(0, 0);
    prompt.focus();
  });

  form.addEventListener("submit", (ev) => {
    ev.preventDefault();
    void ask();
  });
  prompt.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey)) {
      ev.preventDefault();
      void ask();
    }
  });

  // ---- first run and the legacy-location note ----

  function renderOnboardingSlot(): void {
    clear(onboardingSlot);
    if (needsOnboarding()) onboardingSlot.append(renderOnboarding(host.go, host.error));
    clear(banner);
    if (state.locations?.legacy) {
      banner.append(h("div", { class: "notice" }, h("p", null, "Your files are in the older location. Run `pampas migrate` in a terminal to move them to this system's usual place.")));
    }
    if (state.configError) {
      banner.append(h("div", { class: "notice error", role: "alert" }, h("p", null, "The list of sources could not be read."), h("pre", null, state.configError), h("button", { type: "button", class: "linklike", onclick: () => host.go("sources") }, "Open Sources")));
    }
  }

  subscribe(() => {
    renderOnboardingSlot();
    renderSources();
    renderSynth();
  });
  renderOnboardingSlot();
  renderSources();
  renderSynth();
  void readyProviders;
}
