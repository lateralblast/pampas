// The Keys & settings page. Keys are secrets and write-only; settings are plain values.

import { api } from "./api.js";
import { announce, copyText, h, icon } from "./dom.js";
import { swapKeepingFocus } from "./edition.js";
import { keyGroups, refresh, siglaOf, state, subscribe } from "./store.js";
import type { SettingInfo } from "./types.js";

/** Where to get a free key for the services in the starter list. */
const KEY_HELP: Record<string, { where: string; note?: string }> = {
  GROQ_API_KEY: { where: "https://console.groq.com/keys" },
  OPENROUTER_API_KEY: { where: "https://openrouter.ai/settings/keys", note: "No card is needed for the free models." },
  FREEAI_API_KEY: { where: "https://free.ai", note: "Create a key in your account." },
  NVIDIA_API_KEY: { where: "https://build.nvidia.com", note: "Open any model and choose Get API Key." },
  CLOUDFLARE_API_KEY: { where: "https://dash.cloudflare.com/profile/api-tokens", note: "Create a token with the Workers AI permission. Cloudflare also needs your account id under Settings." },
  COHERE_API_KEY: { where: "https://dashboard.cohere.com/api-keys", note: "The free Trial key is enough." },
  GEMINI_API_KEY: { where: "https://aistudio.google.com/apikey", note: "Create the key in a new project and do not turn on billing." },
};

export interface KeysHost {
  error(message: string): void;
  notice(message: string): void;
}

export function mountKeys(root: HTMLElement, host: KeysHost): void {
  const drafts: Record<string, string> = {};
  const fail = (e: unknown) => host.error(e instanceof Error ? e.message : String(e));

  function keyRow(g: ReturnType<typeof keyGroups>[number]): HTMLElement {
    const sigla = siglaOf();
    const help = KEY_HELP[g.name];
    const input = h("input", {
      type: "password",
      id: `key-${g.name}`,
      autocomplete: "off",
      spellcheck: false,
      autocapitalize: "off",
      placeholder: g.status === "keychain" ? "Paste a new key to replace it" : "Paste the key here",
      "aria-describedby": `key-help-${g.name}`,
      dataset: { focusKey: `key-${g.name}` },
      oninput: (ev: Event) => { drafts[g.name] = (ev.currentTarget as HTMLInputElement).value; },
      value: drafts[g.name] ?? "",
    });
    const status =
      g.status === "keychain"
        ? h("p", { class: "wstate ok" }, icon("check"), "Stored in your system's keychain")
        : g.status === "env"
          ? h("p", { class: "wstate ok" }, icon("check"), "Taken from the environment variable of the same name")
          : g.status === "error"
            ? h("p", { class: "wstate bad" }, icon("warn"), `The keychain could not be used: ${g.error ?? "unknown problem"}`)
            : g.status === "not_needed"
              ? h("p", { class: "wstate" }, "Not needed")
              : h("p", { class: "wstate" }, icon("key"), "Not set yet");
    const save = async () => {
      const value = (drafts[g.name] ?? "").trim();
      if (!value) {
        host.error("Paste the key first.");
        input.focus();
        return;
      }
      try {
        await api.setKey(g.name, value);
        delete drafts[g.name];
        announce(`The key ${g.name} was saved.`);
        host.notice(`Saved ${g.name} in the system keychain.`);
        await refresh();
      } catch (e) {
        fail(e);
      }
    };
    return h(
      "li",
      { class: "key-row" },
      h(
        "div",
        { class: "row" },
        h("div", { class: "gutter" }, h("h3", { class: "mono-name" }, g.name), status),
        h(
          "div",
          { class: "body" },
          h("p", { class: "used-by" }, "Used by ", ...g.providers.flatMap((n, i) => [i ? ", " : "", h("span", { class: "nowrap" }, n, " ", h("span", { class: "sig", "aria-hidden": "true" }, sigla.get(n) ?? "?"))])),
          help
            ? h(
                "div",
                { id: `key-help-${g.name}` },
                h("p", { class: "hint" }, "Get a free key at ", h("span", { class: "selectable" }, help.where), " ", h("button", { type: "button", class: "linklike", onclick: async () => announce((await copyText(help.where)) ? "Address copied." : "Could not copy.") }, "Copy address")),
                help.note ? h("p", { class: "hint" }, help.note) : null,
              )
            : h("p", { id: `key-help-${g.name}`, class: "hint" }, "Get the key from the service's website."),
          g.status === "env" ? h("p", { class: "hint" }, "A key you store here is ignored while that environment variable is set.") : null,
          h(
            "form",
            { class: "inline-form", onsubmit: (ev: Event) => { ev.preventDefault(); void save(); } },
            h("label", { for: `key-${g.name}`, class: "visually-hidden" }, `Key for ${g.name}`),
            input,
            h("button", { type: "submit", class: "primary" }, g.status === "keychain" ? "Replace key" : "Save key"),
            g.status === "keychain"
              ? h("button", { type: "button", class: "quiet", onclick: async () => { try { await api.deleteKey(g.name); announce(`The key ${g.name} was removed.`); host.notice(`Removed ${g.name} from the keychain.`); await refresh(); } catch (e) { fail(e); } } }, icon("trash"), "Remove", h("span", { class: "visually-hidden" }, ` ${g.name}`))
              : h("span", { "aria-hidden": "true" }),
          ),
        ),
      ),
    );
  }

  function settingRow(s: SettingInfo): HTMLElement {
    const users = state.providers.filter((p) => p.placeholders.includes(s.name)).map((p) => p.name);
    const input = h("input", { type: "text", id: `set-${s.name}`, value: drafts[`set:${s.name}`] ?? s.value ?? "", autocomplete: "off", spellcheck: false, dataset: { focusKey: `set-${s.name}` }, oninput: (ev: Event) => { drafts[`set:${s.name}`] = (ev.currentTarget as HTMLInputElement).value; } });
    const status =
      s.source === "env"
        ? h("p", { class: "wstate ok" }, icon("check"), "From an environment variable")
        : s.source === "file"
          ? h("p", { class: "wstate ok" }, icon("check"), "Stored")
          : h("p", { class: "wstate" }, icon("key"), "Not set yet");
    const save = async () => {
      try {
        await api.setSetting(s.name, input.value);
        delete drafts[`set:${s.name}`];
        announce(`${s.name} was saved.`);
        host.notice(`Saved ${s.name}.`);
        await refresh();
      } catch (e) {
        fail(e);
      }
    };
    return h(
      "li",
      { class: "key-row" },
      h(
        "div",
        { class: "row" },
        h("div", { class: "gutter" }, h("h3", { class: "mono-name" }, s.name), status),
        h(
          "div",
          { class: "body" },
          users.length ? h("p", { class: "used-by" }, "Needed by ", users.join(", ")) : h("p", { class: "used-by muted" }, "No source uses this right now."),
          s.source === "env" ? h("p", { class: "hint" }, "The environment variable wins over a stored value.") : null,
          h("form", { class: "inline-form", onsubmit: (ev: Event) => { ev.preventDefault(); void save(); } }, h("label", { for: `set-${s.name}`, class: "visually-hidden" }, `Value of ${s.name}`), input, h("button", { type: "submit", class: "primary" }, "Save"), s.source === "file" ? h("button", { type: "button", class: "quiet", onclick: async () => { try { await api.deleteSetting(s.name); announce(`${s.name} was removed.`); await refresh(); } catch (e) { fail(e); } } }, icon("trash"), "Remove", h("span", { class: "visually-hidden" }, ` ${s.name}`)) : h("span", { "aria-hidden": "true" })),
        ),
      ),
    );
  }

  function addSetting(): HTMLElement {
    const name = h("input", { type: "text", id: "new-set-name", placeholder: "CLOUDFLARE_ACCOUNT_ID", autocomplete: "off", spellcheck: false, autocapitalize: "characters", value: drafts["new:name"] ?? "", oninput: (e: Event) => { drafts["new:name"] = (e.currentTarget as HTMLInputElement).value; } });
    const value = h("input", { type: "text", id: "new-set-value", autocomplete: "off", spellcheck: false, value: drafts["new:value"] ?? "", oninput: (e: Event) => { drafts["new:value"] = (e.currentTarget as HTMLInputElement).value; } });
    return h(
      "form",
      {
        class: "add-setting",
        onsubmit: async (ev: Event) => {
          ev.preventDefault();
          try {
            await api.setSetting(name.value.trim(), value.value);
            delete drafts["new:name"];
            delete drafts["new:value"];
            announce("Setting added.");
            await refresh();
          } catch (e) {
            fail(e);
          }
        },
      },
      h("h3", { class: "subhead" }, "Add a setting"),
      h("div", { class: "field" }, h("label", { for: "new-set-name" }, "Name"), name, h("p", { class: "hint" }, "Capital letters, digits and underscores, as written inside ${ } in a source's address.")),
      h("div", { class: "field" }, h("label", { for: "new-set-value" }, "Value"), value),
      h("button", { type: "submit" }, icon("plus"), "Add setting"),
    );
  }

  function render(): void {
    const keys = keyGroups();
    const page = h("div", { class: "page" });
    page.append(
      h("h2", { class: "page-title" }, "Keys & settings"),
      h("h3", { class: "section-title" }, "Secret keys"),
      h("p", { class: "lede" }, "A key is like a password a service gives you for free. pampas keeps each one in your system's keychain and never shows it again after you save it. You can replace or remove it."),
    );
    if (!state.locations?.config_exists) {
      page.append(h("p", { class: "muted" }, "Make your list of sources first, then the keys it needs appear here."));
    } else if (!keys.length) {
      page.append(h("p", { class: "muted" }, "None of your sources need a key right now."));
    } else {
      page.append(h("ul", { class: "key-list" }, ...keys.map(keyRow)));
    }
    page.append(
      h("h3", { class: "section-title" }, "Plain settings"),
      h("p", { class: "lede" }, "Some sources need a plain value that is not secret, such as your Cloudflare account id. They are written as ${NAME} in the source's address and filled in from here."),
      state.settings.length ? h("ul", { class: "key-list" }, ...state.settings.map(settingRow)) : h("p", { class: "muted" }, "No settings are needed right now."),
      addSetting(),
    );
    swapKeepingFocus(root, page);
  }

  subscribe(render);
  render();
}
