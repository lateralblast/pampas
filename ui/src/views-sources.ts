// The Sources page: the list of providers, edited in place and saved as providers.json.

import { api } from "./api.js";
import { announce, append, breakable, clear, h, icon } from "./dom.js";
import { swapKeepingFocus } from "./edition.js";
import { plural } from "./format.js";
import { siglum } from "./siglum.js";
import { refresh, state, subscribe, type TabId } from "./store.js";
import type { ConfigEntry, ProviderInfo } from "./types.js";

export interface SourcesHost {
  go(tab: TabId): void;
  error(message: string): void;
  notice(message: string): void;
  /** Tells the shell whether there are unsaved changes, for the tab's marker. */
  dirty(dirty: boolean): void;
}

const str = (v: unknown): string => (typeof v === "string" ? v : "");
const num = (v: unknown): string => (typeof v === "number" ? String(v) : "");
const isApple = (e: ConfigEntry): boolean => e.kind === "apple_fm";
const isScrape = (e: ConfigEntry): boolean => e.kind === "scrape";

/** The host part of an address, for a quiet second line. */
function hostOf(url: string): string {
  const m = /^https?:\/\/([^/]+)/i.exec(url);
  return m?.[1] ?? url;
}

function blankEntry(kind: "openai" | "apple_fm"): ConfigEntry {
  return kind === "apple_fm" ? { name: "apple", kind: "apple_fm" } : { name: "", base_url: "", model: "" };
}

export function mountSources(root: HTMLElement, host: SourcesHost): void {
  let draft: ConfigEntry[] = [];
  let saved = "[]";
  let loaded = false;
  let editing: number | null = null;
  let adding = false;
  let examples: ConfigEntry[] = [];
  let jsonError = "";
  let saveError = "";
  let pendingRemove: number | null = null;

  const isDirty = () => JSON.stringify(draft) !== saved;
  const info = (name: string): ProviderInfo | undefined => state.providers.find((p) => p.name === name);

  async function load(): Promise<void> {
    if (!state.locations?.config_exists) {
      draft = [];
      saved = "[]";
      loaded = true;
      return;
    }
    try {
      draft = await api.getConfig();
      saved = JSON.stringify(draft);
      loaded = true;
      saveError = "";
    } catch (e) {
      loaded = true;
      saveError = e instanceof Error ? e.message : String(e);
    }
  }

  function changed(): void {
    host.dirty(isDirty());
    render();
  }

  async function save(): Promise<void> {
    saveError = "";
    try {
      await api.validateConfig(draft);
      await api.saveConfig(draft);
      saved = JSON.stringify(draft);
      editing = null;
      adding = false;
      host.dirty(false);
      announce("Saved.");
      host.notice("Saved. The previous list is kept as providers.json.bak.");
      await refresh();
    } catch (e) {
      saveError = e instanceof Error ? e.message : String(e);
      announce("Could not save: " + saveError, true);
      render();
    }
  }

  async function discard(): Promise<void> {
    await load();
    editing = null;
    adding = false;
    jsonError = "";
    host.dirty(false);
    render();
    announce("Changes discarded.");
  }

  function move(i: number, by: -1 | 1): void {
    const j = i + by;
    if (j < 0 || j >= draft.length) return;
    const a = draft[i];
    const b = draft[j];
    if (!a || !b) return;
    draft[i] = b;
    draft[j] = a;
    if (editing === i) editing = j;
    else if (editing === j) editing = i;
    changed();
    announce(`${str(a.name)} moved ${by < 0 ? "up" : "down"}.`);
  }

  function remove(i: number): void {
    const e = draft[i];
    if (pendingRemove !== i) {
      pendingRemove = i;
      render();
      window.setTimeout(() => {
        if (pendingRemove === i) {
          pendingRemove = null;
          render();
        }
      }, 5000);
      return;
    }
    draft.splice(i, 1);
    pendingRemove = null;
    editing = null;
    changed();
    announce(`${str(e?.name)} removed. Save to keep the change.`);
  }

  // ---- the editor for one source ----

  function editor(i: number): HTMLElement {
    const e = draft[i] ?? {};
    const apple = isApple(e);
    const scrape = isScrape(e);
    const field = (id: string, label: string, value: string, hint: string, opts: { type?: string; required?: boolean; placeholder?: string } = {}) =>
      h(
        "div",
        { class: "field" },
        h("label", { for: `f-${id}` }, label, opts.required ? h("span", { class: "req" }, " (required)") : null),
        h("input", { id: `f-${id}`, name: id, type: opts.type ?? "text", value, required: Boolean(opts.required), placeholder: opts.placeholder ?? "", autocomplete: "off", spellcheck: false, inputMode: opts.type === "number" ? "numeric" : undefined, "aria-describedby": `h-${id}` }),
        h("p", { id: `h-${id}`, class: "hint" }, hint),
      );
    const form = h("form", { class: "editor", novalidate: true });
    const err = h("p", { class: "field-error", role: "alert" });
    append(form, [
      h("h3", { class: "editor-title" }, apple ? "On-device source" : scrape ? "Web site source" : "Source details"),
      field("name", "Name", str(e.name), "A short name shown in answers, for example groq. Each source needs a different one.", { required: true }),
      apple
        ? field("command", "Helper program", str(e.command), "The program that talks to Apple Intelligence. Leave empty for the default, pampas-apple-fm.", { placeholder: "pampas-apple-fm" })
        : scrape
        ? h("p", { class: "hint" }, `This source scrapes ${hostOf(str(e.url)) || "a web site"}. It is only asked when scraping is switched on under More options on the Ask page. Scrape sources can only be added with “pampas-cli scrape add”, which asks you to acknowledge the risks; their address, request and selector cannot be changed here, only their timeout, daily limit and whether they are on.`)
        : h(
            "div",
            { class: "editor-grid" },
            field("base_url", "Service address", str(e.base_url), "The web address of the service's API, for example https://api.groq.com/openai/v1. Words in ${CAPITALS} are filled in from Settings.", { required: true, placeholder: "https://…" }),
            field("model", "Model", str(e.model), "The model's name as the service writes it.", { required: true }),
            field("api_key_env", "Key name", str(e.api_key_env), "Which stored key to use, for example GROQ_API_KEY. Leave empty if the service needs no key, as with Ollama on your own computer."),
          ),
      apple
        ? null
        : scrape
        ? h(
            "div",
            { class: "editor-grid" },
            field("timeout", "Wait at most (seconds)", num(e.timeout), "How long to wait for this site before giving up. The default is 30.", { type: "number" }),
            field("daily_limit", "Daily limit (requests)", num(e.daily_limit), "Stop asking this site after this many requests per day, to lower the chance of being blocked. Empty means no limit.", { type: "number" }),
          )
        : h(
            "details",
            { class: "advanced", open: ["chat_path", "fallback", "daily_limit", "usage_group", "timeout"].some((k) => e[k] !== undefined) },
            h("summary", null, "More options"),
            h(
              "div",
              { class: "advanced-body editor-grid" },
              field("timeout", "Wait at most (seconds)", num(e.timeout), "How long to wait for this source before giving up. The default is 30; a large model on your own computer may need more.", { type: "number" }),
              field("daily_limit", "Daily limit (requests)", num(e.daily_limit), "Stop asking this source after this many requests per day, to stay inside a free tier. Empty means no limit.", { type: "number" }),
              field("usage_group", "Shared limit group", str(e.usage_group), "Sources that use the same account can share one count by giving the same group name."),
              field("chat_path", "Chat path", str(e.chat_path), "The part of the address after the service address, if it is not /chat/completions."),
              h(
                "div",
                { class: "field" },
                h("label", { for: "f-fallback" }, "If the model fails"),
                h("select", { id: "f-fallback", name: "fallback", "aria-describedby": "h-fallback" }, h("option", { value: "", selected: !e.fallback }, "Report the failure"), h("option", { value: "free_models", selected: e.fallback === "free_models" }, "Try the service's free models")),
                h("p", { id: "h-fallback", class: "hint" }, "Only works with services that list prices, such as OpenRouter."),
              ),
            ),
          ),
      h(
        "div",
        { class: "check" },
        h("input", { type: "checkbox", id: "f-synthesizer", name: "synthesizer", checked: e.synthesizer === true }),
        h("label", { for: "f-synthesizer" }, "Prefer this source to write the merged reading"),
      ),
      err,
      h(
        "div",
        { class: "editor-actions" },
        h("button", { type: "submit", class: "primary" }, "Done"),
        h("button", { type: "button", onclick: () => { editing = null; if (!str(e.name) && !str(e.base_url) && !str(e.model)) draft.splice(i, 1); changed(); } }, "Cancel"),
      ),
    ]);
    if (scrape) (form.elements.namedItem("name") as HTMLInputElement).readOnly = true;
    form.addEventListener("submit", (ev) => {
      ev.preventDefault();
      const get = (n: string) => (form.elements.namedItem(n) as HTMLInputElement | HTMLSelectElement | null)?.value.trim() ?? "";
      const name = get("name");
      const problems: string[] = [];
      if (!name) problems.push("Give the source a name.");
      if (name && draft.some((o, j) => j !== i && str(o.name) === name)) problems.push(`Another source is already called ${name}.`);
      if (!apple && !scrape) {
        if (!get("base_url")) problems.push("Enter the service address.");
        if (!get("model")) problems.push("Enter the model.");
      }
      for (const n of ["timeout", "daily_limit"]) {
        const v = get(n);
        if (v && !(Number.isInteger(Number(v)) && Number(v) > 0)) problems.push(`${n === "timeout" ? "Wait at most" : "Daily limit"} must be a whole number above zero.`);
      }
      if (problems.length) {
        err.textContent = problems.join(" ");
        (form.querySelector("input:invalid, input") as HTMLElement | null)?.focus();
        return;
      }
      const next: ConfigEntry = { ...e, name };
      const setText = (k: string) => (get(k) ? (next[k] = get(k)) : delete next[k]);
      const setNum = (k: string) => (get(k) ? (next[k] = Number(get(k))) : delete next[k]);
      if (apple) setText("command");
      else if (scrape) {
        setNum("timeout");
        setNum("daily_limit");
      } else {
        for (const k of ["base_url", "model", "api_key_env", "usage_group", "chat_path"]) setText(k);
        setNum("timeout");
        setNum("daily_limit");
        if (get("fallback")) next.fallback = get("fallback");
        else delete next.fallback;
      }
      if ((form.elements.namedItem("synthesizer") as HTMLInputElement).checked) next.synthesizer = true;
      else delete next.synthesizer;
      draft[i] = next;
      editing = null;
      changed();
      announce(`${name} updated. Save to keep the change.`);
    });
    return form;
  }

  // ---- one row ----

  function rowFor(e: ConfigEntry, i: number): HTMLElement {
    const name = str(e.name) || "(unnamed)";
    const p = info(name);
    const on = e.enabled !== false;
    const apple = isApple(e);
    const scrape = isScrape(e);
    const tags: Node[] = [];
    if (p) {
      const t = p.key_status;
      if (t === "missing") tags.push(h("span", { class: "tag bad" }, icon("warn"), "needs key", h("button", { type: "button", class: "linklike", onclick: () => host.go("keys") }, "Add it")));
      else if (t === "error") tags.push(h("span", { class: "tag bad" }, icon("warn"), "keychain problem"));
      else if (t === "env" || t === "keychain") tags.push(h("span", { class: "tag ok" }, icon("check"), "key is set"));
      else tags.push(h("span", { class: "tag" }, "no key needed"));
      if (p.daily_limit !== null) tags.push(h("span", { class: "tag" }, `${p.used} of ${p.daily_limit} used today`));
    }
    if (e.synthesizer === true) tags.push(h("span", { class: "tag" }, "writes the reading"));
    const row = h(
      "li",
      { class: `source-row${on ? "" : " off"}`, dataset: { index: String(i) } },
      h(
        "div",
        { class: "row" },
        h("div", { class: "gutter" }, h("span", { class: "sig sig-lg", "aria-hidden": "true" }, siglum(i)), h("h3", { class: "wname" }, name), h("p", { class: "gmeta" }, apple ? "on this Mac" : breakable(hostOf(str(scrape ? e.url : e.base_url))))),
        h(
          "div",
          { class: "body" },
          h("p", { class: "smodel-line" }, apple ? "Apple Intelligence, on this device" : scrape ? "Scrapes a web site (only with scraping on)" : str(e.model) || "no model set"),
          h("div", { class: "tags" }, ...tags),
          h(
            "div",
            { class: "row-controls" },
            h("div", { class: "check switch" }, h("input", { type: "checkbox", id: `on-${i}`, checked: on, dataset: { focusKey: `on-${i}` }, onchange: (ev: Event) => { const c = ev.currentTarget as HTMLInputElement; if (c.checked) delete e.enabled; else e.enabled = false; changed(); } }), h("label", { for: `on-${i}` }, on ? "On" : "Off", h("span", { class: "visually-hidden" }, ` — ${name}`))),
            h("button", { type: "button", dataset: { focusKey: `edit-${i}` }, "aria-expanded": String(editing === i), onclick: () => { editing = editing === i ? null : i; render(); } }, icon("edit"), "Edit", h("span", { class: "visually-hidden" }, ` ${name}`)),
            h("button", { type: "button", class: "quiet icon-only", "aria-label": `Move ${name} up`, disabled: i === 0, dataset: { focusKey: `up-${i}` }, onclick: () => move(i, -1) }, icon("up")),
            h("button", { type: "button", class: "quiet icon-only", "aria-label": `Move ${name} down`, disabled: i === draft.length - 1, dataset: { focusKey: `down-${i}` }, onclick: () => move(i, 1) }, icon("down")),
            h("button", { type: "button", class: pendingRemove === i ? "danger" : "quiet", dataset: { focusKey: `rm-${i}` }, onclick: () => remove(i) }, icon("trash"), pendingRemove === i ? "Click again to remove" : "Remove", h("span", { class: "visually-hidden" }, ` ${name}`)),
          ),
        ),
      ),
      editing === i ? editor(i) : null,
    );
    return row;
  }

  // ---- adding ----

  function addPanel(): HTMLElement {
    const have = new Set(draft.map((e) => str(e.name)));
    const add = (entry: ConfigEntry, blank = false) => {
      draft.push(structuredClone(entry));
      editing = draft.length - 1;
      adding = false;
      changed();
      announce(blank ? "New source added. Fill in its details." : `${str(entry.name)} added. Save to keep it.`);
      root.querySelector(`[data-index="${draft.length - 1}"]`)?.scrollIntoView({ block: "center" });
    };
    const fresh = examples.filter((e) => !have.has(str(e.name)));
    return h(
      "section",
      { class: "add-panel", "aria-labelledby": "add-title" },
      h("h3", { id: "add-title" }, "Add a source"),
      h("p", { class: "hint" }, "Start from one of the built-in free services, or describe your own. Any service that speaks the OpenAI chat format works."),
      h(
        "div",
        { class: "add-choices" },
        h("button", { type: "button", onclick: () => add(blankEntry("openai"), true) }, icon("plus"), "A service of my own"),
        h("button", { type: "button", onclick: () => add(blankEntry("apple_fm"), true) }, icon("plus"), "Apple Intelligence on this Mac"),
      ),
      fresh.length
        ? h("ul", { class: "example-list" }, ...fresh.map((e) => h("li", null, h("span", { class: "sname" }, str(e.name)), h("span", { class: "smodel" }, isApple(e) ? "on this Mac" : isScrape(e) ? "scrapes a web site" : str(e.model)), h("span", { class: "muted" }, e.api_key_env ? ` needs ${str(e.api_key_env)}` : " needs no key"), h("button", { type: "button", class: "quiet", onclick: () => add(e) }, icon("plus"), "Add", h("span", { class: "visually-hidden" }, ` ${str(e.name)}`)))))
        : h("p", { class: "muted" }, "Every built-in source is already in your list."),
      h("button", { type: "button", class: "linklike", onclick: () => { adding = false; render(); } }, "Close"),
    );
  }

  // ---- the page ----

  function jsonEditor(): HTMLElement {
    const area = h("textarea", { id: "json-area", rows: 14, spellcheck: false, "aria-describedby": "json-hint", value: JSON.stringify(draft, null, 2) });
    return h(
      "details",
      { class: "advanced json" },
      h("summary", null, "Edit the file as text (advanced)"),
      h(
        "div",
        { class: "advanced-body" },
        h("p", { id: "json-hint", class: "hint" }, "This is the whole list as JSON, exactly as saved in providers.json. Fields this page does not know about are kept either way."),
        h("label", { for: "json-area", class: "visually-hidden" }, "The list of sources as JSON"),
        area,
        jsonError ? h("p", { class: "field-error", role: "alert" }, jsonError) : null,
        h(
          "button",
          {
            type: "button",
            onclick: async () => {
              try {
                const parsed: unknown = JSON.parse(area.value);
                if (!Array.isArray(parsed)) throw new Error("The list must start with [ and end with ].");
                await api.validateConfig(parsed as ConfigEntry[]);
                draft = parsed as ConfigEntry[];
                jsonError = "";
                changed();
                announce("The text was applied. Save to keep it.");
              } catch (e) {
                jsonError = e instanceof Error ? e.message : String(e);
                render();
              }
            },
          },
          "Apply the text",
        ),
      ),
    );
  }

  function render(): void {
    const page = h("div", { class: "page" });
    page.append(
      h("h2", { class: "page-title" }, "Sources"),
      h("p", { class: "lede" }, "The services pampas asks. The boxed letter on each is the mark used for it in answers. Changes are kept here until you save them."),
    );
    if (!loaded) {
      page.append(h("p", { class: "muted" }, "Loading…"));
    } else if (!state.locations?.config_exists && draft.length === 0) {
      page.append(
        h(
          "div",
          { class: "empty" },
          h("h3", null, "No list of sources yet"),
          h("p", null, "Start from the built-in list of free services; you can change it afterwards."),
          h("button", { type: "button", class: "primary", onclick: async () => { try { await api.createConfig(); await refresh(); } catch (e) { host.error(e instanceof Error ? e.message : String(e)); } } }, "Create the starter list"),
        ),
      );
    } else {
      if (isDirty()) {
        page.append(
          h(
            "div",
            { class: "savebar", role: "region", "aria-label": "Unsaved changes" },
            h("p", null, h("strong", null, "Unsaved changes."), " They are only here until you save."),
            h("div", { class: "savebar-actions" }, h("button", { type: "button", class: "primary", onclick: () => void save() }, "Save changes"), h("button", { type: "button", onclick: () => void discard() }, "Discard")),
          ),
        );
      }
      if (saveError) page.append(h("div", { class: "notice error", role: "alert" }, h("p", null, "Could not save."), h("pre", null, saveError)));
      page.append(h("p", { class: "count" }, plural(draft.length, "source")), h("ol", { class: "source-rows" }, ...draft.map((e, i) => rowFor(e, i))));
      page.append(adding ? addPanel() : h("button", { type: "button", dataset: { focusKey: "add" }, onclick: async () => { adding = true; if (!examples.length) { try { examples = await api.exampleConfig(); } catch { examples = []; } } render(); } }, icon("plus"), "Add a source"));
      page.append(jsonEditor());
    }
    swapKeepingFocus(root, page);
  }

  subscribe(() => {
    // Keep unsaved edits; only refresh the key and usage tags around them.
    if (!isDirty()) void load().then(render);
    else render();
  });
  void load().then(render);
  render();
  void clear;
}
